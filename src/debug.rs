#[cfg(debug_assertions)]
use std::fmt::Display;

#[cfg(debug_assertions)]
use systemd::sd_journal_log;

#[cfg(debug_assertions)]
use crate::boost::Op;

#[cfg(debug_assertions)]
pub enum Target {
    Cpu,
    Dmem,
    Mem,
}
#[cfg(debug_assertions)]
impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Target::Cpu => "CPU",
                Target::Dmem => "DMEM",
                Target::Mem => "MEM",
            }
        )
    }
}
#[cfg(debug_assertions)]
pub fn debug_print(op: &Op, target: &Target, path: &str) {
    sd_journal_log!(5, "{op} {target} for {path}");
    println!("{op} {target} for {path}");
}
