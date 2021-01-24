use myutil::{err::*, *};
use nix::mount::{self, MsFlags};

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
    mount::mount(from, to.unwrap(), fs_type, flags, data);
}
