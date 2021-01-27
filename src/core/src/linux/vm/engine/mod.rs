mod firecracker;
mod qemu;

use crate::Vm;
use core_def::VmKind;
use myutil::{err::*, *};
use std::fs;

#[inline(always)]
pub(in crate::linux) fn init() -> Result<()> {
    fs::create_dir_all(firecracker::LOG_DIR)
        .c(d!())
        .and_then(|_| qemu::init().c(d!()))
}

pub(super) fn start(vm: &Vm) -> Result<()> {
    match vm.kind {
        VmKind::Qemu => qemu::start(vm).c(d!()),
        // VmKind::FireCracker => firecracker::
        // todo
        _ => Err(eg!("Unsupported Vm Kind")),
    }
}
