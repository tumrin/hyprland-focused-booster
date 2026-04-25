use std::{
    collections::HashMap,
    fs, process,
    sync::{LazyLock, Mutex},
};

use hyprland::{data::Client, event_listener, shared::HyprDataActiveOptional};
use systemd::sd_journal_log;

enum OP {
    Boost,
    Revert,
}

static PREVIOUS: LazyLock<Mutex<Option<i32>>> = LazyLock::new(|| Mutex::new(None));
static GPUS: LazyLock<HashMap<String, u64>> = LazyLock::new(|| {
    let capacities: HashMap<String, u64> = fs::read_to_string("/sys/fs/cgroup/dmem.capacity")
        .expect("Could not read GPU devices")
        .lines()
        .filter_map(|line| {
            if !line.contains("vram") {
                return None;
            }
            let [gpu, capacity] = line.trim().split(' ').collect::<Vec<&str>>()[..] else {
                return None;
            };
            let capacity = capacity.parse::<u64>();
            if let Ok(capacity) = capacity {
                Some((gpu.to_string(), capacity))
            } else {
                sd_journal_log!(4, "Failed to get capacity for device: {gpu}");
                None
            }
        })
        .collect();

    if capacities.is_empty() {
        sd_journal_log!(3, "Could not find any GPUs");
    };
    capacities
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
                *prev_lock = Some(data.pid)
            } else {
                *prev_lock = None;
                sd_journal_log!(4, "Failed to get active client");
            }
        } else {
            sd_journal_log!(
                4,
                "Failed to acquire lock on previous cgroup dmem.low value"
            );
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
    if let Ok(service) = systemd::login::get_cgroup(Some(pid)) {
        let path = format!("/sys/fs/cgroup{}/dmem.low", service);
        let value = GPUS
            .iter()
            .map(|(gpu, value)| {
                format!(
                    "{} {}\n",
                    gpu,
                    match op {
                        OP::Boost => value,
                        OP::Revert => &0,
                    }
                )
            })
            .collect::<String>();
        let res = fs::write(&path, value);
        if let Err(err) = res {
            sd_journal_log!(
                4,
                "Error: {err} for path: {path}. This may be caused by dmemcg-booster not yet having enabled dmem controls in which case this is safe to ignore."
            );
        };
    } else {
        sd_journal_log!(4, "Failed to get cgroup for pid: {pid}");
    }
}
