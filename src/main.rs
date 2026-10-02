use std::process;

use hyprland::{data::Client, event_listener, shared::HyprDataActiveOptional};
use systemd::sd_journal_log;

use crate::boost::{Op, PREVIOUS_PID, write_cgroup};
#[cfg(debug_assertions)]
use crate::config::CONFIG;

mod boost;
mod config;
#[cfg(debug_assertions)]
mod debug;

fn main() {
    #[cfg(debug_assertions)]
    dbg!(&CONFIG);

    ctrlc::set_handler(move || {
        if let Ok(prev_lock) = PREVIOUS_PID.lock() {
            if let Some(previous) = *prev_lock {
                write_cgroup(previous, Op::Revert);
            } else {
                sd_journal_log!(3, "Failed to revert previous pid boost");
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
        if let Ok(mut prev_lock) = PREVIOUS_PID.lock() {
            match *prev_lock {
                Some(previous) => {
                    if let Ok(Some(current)) = Client::get_active() {
                        if previous != current.pid {
                            write_cgroup(previous, Op::Revert);
                            write_cgroup(current.pid, Op::Boost);
                            *prev_lock = Some(current.pid);
                        }
                    } else {
                        write_cgroup(previous, Op::Revert);
                        *prev_lock = None;
                    }
                }
                None => {
                    if let Ok(Some(data)) = Client::get_active() {
                        write_cgroup(data.pid, Op::Boost);
                        *prev_lock = Some(data.pid);
                    }
                }
            }
        } else {
            sd_journal_log!(4, "Failed to acquire lock on previous boosted pid");
        }
    });
    el.start_listener()
        .inspect_err(|e| sd_journal_log!(3, "{e}"))
        .expect("Failed to start event listener");

    if let Ok(Some(prev)) = PREVIOUS_PID.lock().map(|p| *p) {
        write_cgroup(prev, Op::Revert);
        process::exit(0);
    }
}
