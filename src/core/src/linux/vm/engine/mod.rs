mod firecracker;
mod qemu;

use crate::{Vm, FUCK};
use core_def::VmKind;
use myutil::{err::*, *};
use std::fs;

pub(super) fn get_pre_starter(vm: &Vm) -> Result<fn(&Vm) -> Result<()>> {
    match vm.kind {
        VmKind::Qemu => Ok(qemu::pre_start),
        _ => Err(eg!("Unsupported VmKind")),
    }
}

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

#[inline(always)]
pub(super) fn remove_image(vm: &Vm) -> Result<()> {
    match vm.kind {
        VmKind::Qemu => qemu::remove_image(vm).c(d!()),
        _ => Err(eg!(FUCK)),
    }
}

#[cfg(feature = "nft")]
#[inline(always)]
pub(super) fn remove_tap(vm: &Vm) -> Result<()> {
    match vm.kind {
        VmKind::Qemu => qemu::remove_tap(vm).c(d!()),
        _ => Err(eg!(FUCK)),
    }
}
