pub(crate) mod cgroup;
pub(crate) mod engine;
pub(crate) mod util;

use std::{os::unix::prelude::CommandExt, process};

use cgroup::kill_vm;
use engine::remove_image;
use myutil::{err::*, *};
use nix::unistd::{fork, ForkResult};

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

#[inline(always)]
pub(crate) fn zobmie_clean() {
    util::wait_pid()
}

// 必须后台执行
#[inline(always)]
#[cfg(all(feature = "nft", any(feature = "cow", feature = "zfs")))]
fn cmd_exec_daemonize(cmd: &str, args: &[&str]) -> Result<()> {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => pnk!(Err(eg!(process::Command::new(cmd)
            .stdin(process::Stdio::null())
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .args(args)
            .exec()))),
        Ok(_) => Ok(()),
        Err(e) => Err(e).c(d!()),
    }
}

#[inline(always)]
pub(crate) fn get_pre_starter(vm: &Vm) -> Result<fn(&Vm) -> Result<()>> {
    engine::get_pre_starter(vm).c(d!())
}

#[inline(always)]
pub(crate) fn post_clean(vm: &Vm) {
    info_omit!(cgroup::kill_vm(vm.id));
    if !vm.image_cached {
        info_omit!(engine::remove_image(vm));
    }
    #[cfg(feature = "nft")]
    info_omit!(engine::remove_tap(vm));
}
