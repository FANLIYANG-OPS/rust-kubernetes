use crate::img_root_register;
use myutil::{err::*, *};
use nix::sched::{clone, CloneFlags};
pub(crate) mod vm;

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

    // todo
    // let ops = || -> isize {
    //     info!(Ok(())).and(Ok(0)).or
    // };
}
