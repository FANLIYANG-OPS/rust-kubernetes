use crate::linux::vm::util;
use crate::FUCK;
use core_def::{Pid, VmId};
use myutil::{err::*, *};
use std::{fs, io::Write, path::PathBuf};

const CGROUP_ROOT_PATH: &str = "/tmp/.ttcgroup";
const CGROUP_ADMIN_PATH: &str = "/tmp/.ttcgroup/ttadmin";
const CGROUP_PROCS: &str = "cgroup.procs";

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

pub(in crate::linux) fn add_vm(id: VmId, pid: Pid) -> Result<()> {
    add_proc(id, pid).c(d!())
}

fn add_proc(id: VmId, pid: Pid) -> Result<()> {
    get_proc_meta_path(id)
        .c(d!())
        .and_then(|meta| fs::OpenOptions::new().append(true).open(meta).c(d!()))
        .and_then(|mut f| f.write(pid.to_string().as_bytes()).c(d!()).map(|_| ()))
}

#[inline(always)]
fn get_proc_meta_path(id: VmId) -> Result<PathBuf> {
    let mut mount = get_mnt_point(id).c(d!()).unwrap();
    mount.push(CGROUP_PROCS);
    alt!(mount.is_file(), Ok(mount), Err(eg!()))
}

fn cgroup_ready() -> bool {
    let mut path = PathBuf::from(CGROUP_ROOT_PATH);
    path.push(CGROUP_PROCS);
    path.is_file()
}

pub(in crate::linux) fn alloc_mnt_point(id: VmId) -> Result<PathBuf> {
    if !cgroup_ready() {
        return Err(eg!(FUCK));
    }
    let mut path = PathBuf::from(CGROUP_ROOT_PATH);
    path.push(id.to_string());
    if !path.exists() {
        fs::create_dir(&path).c(d!()).map(|_| path)
    } else if 0
        == get_proc_meta_path(id)
            .c(d!())
            .unwrap()
            .metadata()
            .c(d!())
            .unwrap()
            .len()
    {
        Ok(path)
    } else {
        Err(eg!(FUCK))
    }
}
