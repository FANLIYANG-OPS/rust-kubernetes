pub(crate) mod cgroup;
pub(crate) mod engine;
pub(crate) mod util;

use std::process;

use myutil::{err::*, *};

fn cmd_exec(cmd: &str, args: &[&str]) -> Result<()> {
    let res = process::Command::new(cmd).args(args).output().c(d!())?;
    if res.status.success() {
        Ok(())
    } else {
        Err(eg!(String::from_utf8_lossy(&res.stderr)))
    }
}
