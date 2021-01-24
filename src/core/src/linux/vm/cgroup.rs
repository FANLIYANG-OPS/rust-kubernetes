
use crate::linux::vm::util;
use myutil::{err::*, *};
use std::fs;

const CGROUP_ROOT_PATH: &str = "/tmp/.ttcgroup";
const CGROUP_ADMIN_PATH: &str = "/tmp/.ttcgroup/ttadmin";

pub(in crate::linux) fn init() -> Result<()> {
    fs::create_dir_all(CGROUP_ROOT_PATH)
        .c(d!())
        .and_then(|_| util::mount_cgroup2(CGROUP_ROOT_PATH).c(d!()))
        .and_then(|_| fs::create_dir_all(CGROUP_ADMIN_PATH).c(d!()))
}
