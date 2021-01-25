use crate::linux::vm::util;
use core_def::VmId;
use myutil::{err::*, *};
use std::{fs, path::PathBuf};

const CGROUP_ROOT_PATH: &str = "/tmp/.ttcgroup";
const CGROUP_ADMIN_PATH: &str = "/tmp/.ttcgroup/ttadmin";

pub(in crate::linux) fn init() -> Result<()> {
    fs::create_dir_all(CGROUP_ROOT_PATH)
        .c(d!())
        .and_then(|_| util::mount_cgroup2(CGROUP_ROOT_PATH).c(d!()))
        .and_then(|_| fs::create_dir_all(CGROUP_ADMIN_PATH).c(d!()))
}

fn get_mnt_point(id: VmId) -> Result<PathBuf> {
    let mut path = PathBuf::from(CGROUP_ROOT_PATH);
    path.push(id.to_string());
    if path.exists() && path.is_dir() {
        Ok(path)
    } else {
        Err(eg!("not exists"))
    }
}
