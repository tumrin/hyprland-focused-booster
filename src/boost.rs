use std::fmt::Display;
use std::{
    fs, io, process,
    sync::{LazyLock, Mutex},
    thread,
    time::Duration,
};

use systemd::sd_journal_log;

use crate::config::CONFIG;
#[cfg(debug_assertions)]
use crate::debug::{Target, debug_print};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Boost,
    Revert,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Op::Boost => "boosting",
                Op::Revert => "reverting",
            }
        )
    }
}

pub struct DmemValueStrings {
    pub boost: Vec<String>,
    pub revert: Vec<String>,
}
pub struct MemValueStrings {
    pub boost: String,
    pub revert: String,
}
pub struct CpuValueStrings {
    pub boost: String,
    pub revert: String,
}

const VRAM_CGROUP_NAMES: [&str; 2] = ["vram", "vidmem"];
const RETRY_COUNT: i32 = 5;
const RETRY_DURATION: Duration = Duration::from_millis(500);

static DMEM_VALUES: LazyLock<DmemValueStrings> = LazyLock::new(|| {
    let (boost, revert): (Vec<String>, Vec<String>) =
        fs::read_to_string("/sys/fs/cgroup/dmem.capacity")
            .expect("Could not read GPU devices")
            .lines()
            .filter_map(|line| {
                if !VRAM_CGROUP_NAMES
                    .into_iter()
                    .any(|name| line.contains(name))
                {
                    return None;
                }

                let [gpu, capacity] = line.split_whitespace().collect::<Vec<&str>>()[..] else {
                    return None;
                };
                let capacity = capacity.parse::<u64>();
                if let Ok(capacity) = capacity {
                    Some((
                        format!(
                            "{gpu} {}\n",
                            (capacity as f64 * CONFIG.vram_boost_value as f64 * 0.01) as u64
                        ),
                        format!("{gpu} 0\n"),
                    ))
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
    DmemValueStrings { boost, revert }
});

static MEM_VALUES: LazyLock<MemValueStrings> = LazyLock::new(|| {
    let mem_file = fs::read_to_string("/proc/meminfo").expect("Could not read meminfo");
    let mem_capacity = mem_file
        .lines()
        .find(|line| line.contains("MemTotal"))
        .and_then(|line| line.split_whitespace().nth(1));

    if let Some(capacity) = mem_capacity {
        MemValueStrings {
            revert: "0".to_string(),
            boost: ((capacity
                .parse::<f64>()
                .expect("Failed to parse memory capacity")
                * 1024.0
                * CONFIG.ram_boost_value as f64
                * 0.01)
                .ceil() as u64)
                .to_string(),
        }
    } else {
        sd_journal_log!(3, "Could not read memory capacity");
        process::exit(1);
    }
});

static CPU_VALUES: LazyLock<CpuValueStrings> = LazyLock::new(|| CpuValueStrings {
    boost: CONFIG.cpu_boost_value.to_string(),
    revert: "100".to_string(),
});

type WriteFunctions = Vec<fn(&str, Op)>;
static WRITE_OPERATIONS: LazyLock<WriteFunctions> = LazyLock::new(|| {
    let mut ops: WriteFunctions = vec![];
    if CONFIG.vram_boost {
        ops.push(write_cgroup_dmem);
    }
    if CONFIG.cpu_boost {
        ops.push(write_cgroup_cpu);
    }
    if CONFIG.ram_boost {
        ops.push(write_cgroup_mem)
    }
    ops
});

pub static PREVIOUS_PID: LazyLock<Mutex<Option<i32>>> = LazyLock::new(|| Mutex::new(None));

pub fn write_cgroup_cpu(path: &str, op: Op) {
    let path = format!("{path}/cpu.weight");
    let res = fs::write(
        &path,
        match op {
            Op::Boost => &CPU_VALUES.boost,
            Op::Revert => &CPU_VALUES.revert,
        },
    );
    // There are some cases where cpu.weight does not exist for some applications
    if let Err(err) = res
        && err.kind() != io::ErrorKind::NotFound
    {
        sd_journal_log!(3, "Error {op} CPU: {err} for path: {path}.");
    }
    #[cfg(debug_assertions)]
    debug_print(&op, &Target::Cpu, &path);
}

pub fn write_cgroup_mem(path: &str, op: Op) {
    let path = format!("{path}/memory.low");
    let res = fs::write(
        &path,
        match op {
            Op::Boost => &MEM_VALUES.boost,
            Op::Revert => &MEM_VALUES.revert,
        },
    );
    // There are some cases where memory.low does not exist for some applications
    if let Err(err) = res
        && err.kind() != io::ErrorKind::NotFound
    {
        sd_journal_log!(3, "Error {op} MEM: {err} for path: {path}.");
    }
    #[cfg(debug_assertions)]
    debug_print(&op, &Target::Mem, &path);
}

pub fn write_cgroup_dmem(path: &str, op: Op) {
    let path = format!("{path}/dmem.low");
    let values: &Vec<String> = match op {
        Op::Boost => &DMEM_VALUES.boost,
        Op::Revert => &DMEM_VALUES.revert,
    };

    let mut retry = 0;
    let res = loop {
        let write = values.iter().try_for_each(|value| fs::write(&path, value));

        // dmemcg-booster might not be ready yet
        let retryable = op == Op::Boost
            && write.as_ref().is_err_and(|error| {
                error.kind() == io::ErrorKind::NotFound
                    || error.kind() == io::ErrorKind::PermissionDenied
            });

        retry += 1;

        if !retryable || retry >= RETRY_COUNT {
            break write;
        }

        sd_journal_log!(
            5,
            "Could not write to dmem.low. Retrying {retry} / {RETRY_COUNT}."
        );
        thread::sleep(RETRY_DURATION);
    };

    if let Err(err) = res {
        sd_journal_log!(3, "Error {op} VRAM: {err} for path: {path}.");
    }
    #[cfg(debug_assertions)]
    debug_print(&op, &Target::Dmem, &path);
}

pub fn write_cgroup(pid: i32, op: Op) {
    match systemd::login::get_cgroup(Some(pid)) {
        Ok(service) => {
            let path = format!("/sys/fs/cgroup{}", service);
            if path.contains("app.slice") {
                WRITE_OPERATIONS.iter().for_each(|wo| {
                    wo(&path, op);
                });
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
