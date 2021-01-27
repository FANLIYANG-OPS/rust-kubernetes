pub(crate) mod cgroup;
pub(crate) mod engine;
pub(crate) mod util;

use std::process;

use myutil::{err::*, *};

use crate::Vm;

#[inline(always)]
pub(crate) fn start(vm: &Vm) -> Result<()> {
    cgroup::alloc_mnt_point(vm.id)
        .c(d!())
        .and_then(|_| cgroup::add_vm(vm.id, process::id()).c(d!()))
        .and_then(|_| engine::start(vm).c(d!()))
}

fn cmd_exec(cmd: &str, args: &[&str]) -> Result<()> {
    let res = process::Command::new(cmd).args(args).output().c(d!())?;
    if res.status.success() {
        Ok(())
    } else {
        Err(eg!(String::from_utf8_lossy(&res.stderr)))
    }
}
