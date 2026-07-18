use std::{
    fs, process,
    sync::{LazyLock, Mutex},
    thread,
    time::Duration,
};

use hyprland::{data::Client, event_listener, shared::HyprDataActiveOptional};
use systemd::sd_journal_log;

enum OP {
    Boost,
    Revert,
}

struct ValueString {
    boost: String,
    revert: String,
}

const SLEEP_DURATION: Duration = Duration::from_millis(500);
const RETRY_COUNT: i32 = 5;

static PREVIOUS: LazyLock<Mutex<Option<i32>>> = LazyLock::new(|| Mutex::new(None));
static VALUES: LazyLock<ValueString> = LazyLock::new(|| {
    let (boost, revert): (String, String) = fs::read_to_string("/sys/fs/cgroup/dmem.capacity")
        .expect("Could not read GPU devices")
        .lines()
        .filter_map(|line| {
            if !line.contains("vram") {
                return None;
            }
            let [gpu, capacity] = line.split_whitespace().collect::<Vec<&str>>()[..] else {
                return None;
            };
            let capacity = capacity.parse::<u64>();
            if let Ok(capacity) = capacity {
                Some((format!("{gpu} {capacity}\n"), format!("{gpu} 0\n")))
            } else {
                sd_journal_log!(4, "Failed to get capacity for device: {gpu}");
                None
            }
        })
        .collect();

    if boost.is_empty() || revert.is_empty() {
        sd_journal_log!(3, "Could not find any GPUs");
        process::exit(1);
    }
    ValueString { boost, revert }
});

fn main() {
    ctrlc::set_handler(|| {
        if let Ok(prev_lock) = PREVIOUS.lock() {
            if let Some(previous) = *prev_lock {
                write_cgroup_dmem(previous, OP::Revert);
            } else {
                sd_journal_log!(3, "Failed to revert previous PID dmem.low value");
            }
            process::exit(0);
        }
    })
    .inspect_err(|e| {
        sd_journal_log!(3, "{e}");
    })
    .expect("Failed to register interrupt handler");

    let mut el = event_listener::EventListener::new();

    el.add_active_window_changed_handler(move |_w| {
        if let Ok(mut prev_lock) = PREVIOUS.lock() {
            if let Some(previous) = *prev_lock {
                write_cgroup_dmem(previous, OP::Revert);
            }
            if let Ok(Some(data)) = Client::get_active() {
                write_cgroup_dmem(data.pid, OP::Boost);
                *prev_lock = Some(data.pid);
            } else {
                *prev_lock = None;
            }
        } else {
            sd_journal_log!(4, "Failed to acquire lock on previous boosted pid");
        }
    });

    el.start_listener()
        .inspect_err(|e| sd_journal_log!(3, "{e}"))
        .expect("Failed to start event listener");

    if let Ok(Some(prev)) = PREVIOUS.lock().map(|p| *p) {
        write_cgroup_dmem(prev, OP::Revert);
    }
}

fn write_cgroup_dmem(pid: i32, op: OP) {
    match systemd::login::get_cgroup(Some(pid)) {
        Ok(service) => {
            let path = format!("/sys/fs/cgroup{}/dmem.low", service);
            if path.contains("app.slice") {
                // dmemcg-booster might not be ready yet so retry few times before logging an error
                let mut retry = 0;
                let res = loop {
                    let write = fs::write(
                        &path,
                        match op {
                            OP::Boost => &VALUES.boost,
                            OP::Revert => &VALUES.revert,
                        },
                    );
                    retry += 1;
                    if retry >= RETRY_COUNT || write.is_ok() {
                        break write;
                    }
                    sd_journal_log!(
                        5,
                        "Could not write to dmem.low. Retrying {retry} / {RETRY_COUNT}."
                    );
                    thread::sleep(SLEEP_DURATION);
                };
                if let Err(err) = res {
                    sd_journal_log!(3, "Error: {err} for path: {path}.");
                };
            }
        }
        Err(e) => {
            // Skip writing to journal when previous process no longer exists.
            if !e.to_string().contains("No such process") {
                sd_journal_log!(
                    4,
                    "Error when trying to find cgroup by pid: {pid} with error: {e}"
                );
            }
        }
    }
}
