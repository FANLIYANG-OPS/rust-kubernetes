mod firecracker;
mod qemu;
mod cgroup;
use myutil::{err::*, *};
use std::fs;

#[inline(always)]
pub(in crate::linux) fn init() -> Result<()> {
    fs::create_dir_all(firecracker::LOG_DIR)
        .c(d!())
        .and_then(|_| qemu::init().c(d!()))
}
