use crate::img_root_register;
use myutil::{err::*, *};
use nix::sched::{clone, CloneFlags};
pub(crate) mod vm;
pub(crate) mod nat;


#[cfg(feature = "zfs")]
pub fn exec(img_path: &str, func: fn() -> Result<()>, server_ip: &str) -> Result<()> {
    img_root_register(Some(img_path));
    Ok(())
}

fn do_exec(func: fn() -> Result<()>, server_ip: &str) {
    const STACK_SIZE: usize = 1024 * 1024;
    let mut stack = Vec::with_capacity(STACK_SIZE);

    unsafe {
        stack.set_len(STACK_SIZE);
    }

    let mut flags = CloneFlags::empty();
    flags.insert(CloneFlags::CLONE_NEWNS);
    flags.insert(CloneFlags::CLONE_NEWPID);

    let ops = || -> isize {
        info!(
            vm::util::mount_make_private()
                .c(d!())
                // todo
                .and_then(|_| vm::util::mount_dyn_fs_proc().c(d!()))
                .and_then(|_| vm::util::mount_tmp_fs().c(d!())) 
                .and_then(|_| vm::engine::init().c(d!()))
                .and_then(|_| vm::cgroup::init().c(d!()))
                // todo
        )
    };
}
