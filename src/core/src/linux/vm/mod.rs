use myutil::{err::*, *};
use nix::mount::{self, MsFlags};

pub(in crate::linux) fn mount_make_private() {}

/// mount file
fn mount_fs(
    from: Option<&str>,
    to: Option<&str>,
    fs_type: Option<&str>,
    flags: MsFlags,
    data: Option<&str>,
) -> Result<()> {
    mount::mount(from, to.unwrap(), fs_type, flags, data);
    Ok(())
}
