use myutil::{err::*, *};
use nix::{
    mount::{self, MsFlags},
    sys::wait,
    unistd,
};

#[cfg(not(feature = "testmock"))]
pub(in crate::linux) fn mount_cgroup2(path: &str) -> Result<()> {
    mount_fs(None, Some(path), Some("cgroup2"), MsFlags::empty(), None).c(d!())
}

/// mount proc dir
pub(in crate::linux) fn mount_dyn_fs_proc() -> Result<()> {
    let mut flags = MsFlags::empty();
    flags.insert(MsFlags::MS_NODEV);
    flags.insert(MsFlags::MS_NOEXEC);
    flags.insert(MsFlags::MS_NOSUID);
    flags.insert(MsFlags::MS_RELATIME);

    mount_fs(None, Some("/proc"), Some("proc"), flags, None).c(d!())
}

/// mount tmp dir
pub(in crate::linux) fn mount_tmp_fs() -> Result<()> {
    let mut flags = MsFlags::empty();
    flags.insert(MsFlags::MS_RELATIME);
    mount_fs(None, Some("/tmp"), Some("tmpfs"), flags, None).c(d!())
}

/// mount private dir
pub(in crate::linux) fn mount_make_private() -> Result<()> {
    mount_fs(
        None,
        Some("/"),
        None,
        pnk!(MsFlags::from_bits(
            MsFlags::MS_REC.bits() | MsFlags::MS_PRIVATE.bits()
        )),
        None,
    )
    .c(d!())
}

/// mount file
fn mount_fs(
    from: Option<&str>,
    to: Option<&str>,
    fs_type: Option<&str>,
    flags: MsFlags,
    data: Option<&str>,
) -> Result<()> {
    mount::mount(from, to.unwrap(), fs_type, flags, data).c(d!())
}

pub(in crate::linux) fn wait_pid() -> () {
    while let Ok(st) = wait::waitpid(unistd::Pid::from_raw(-1), Some(wait::WaitPidFlag::WNOHANG)) {
        if st == wait::WaitStatus::StillAlive {
            break;
        }
    }
}
