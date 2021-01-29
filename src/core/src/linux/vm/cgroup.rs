use crate::FUCK;
use crate::{common, linux::vm::util};
use common::{async_sleep, THREAD_POOL};
use core_def::VmId;
use myutil::{err::*, *};
use nix::{
    sys::signal::{self, kill},
    unistd::Pid,
};
use std::{fs, io::Write, path::PathBuf, process};

const CGROUP_ROOT_PATH: &str = "/tmp/.ttcgroup";
const CGROUP_ADMIN_PATH: &str = "/tmp/.ttcgroup/ttadmin";
const CGROUP_PROCS: &str = "cgroup.procs";

fn cgroup_reset_admin() -> Result<()> {
    let group_procs_info: String = format!("{}/{}", CGROUP_ADMIN_PATH, CGROUP_PROCS);
    fs::OpenOptions::new()
        .append(true)
        .open(group_procs_info.as_str())
        .c(d!())
        .unwrap()
        .write(process::id().to_string().as_bytes())
        .c(d!())
        .map(|_| ())
}

pub(crate) fn kill_vm(id: VmId) -> Result<()> {
    get_proc_meta_path(id)
        .c(d!())
        .and_then(|p| kill_group(p).c(d!()))
}

fn kill_group(cgroup_path: PathBuf) -> Result<()> {
    fs::read(&cgroup_path)
        .c(d!())
        .and_then(|b| String::from_utf8(b).c(d!()))
        .and_then(|s| {
            let mut failed_list = vct![];
            s.lines().for_each(|pid| {
                let pid = pnk!(pid.parse::<u32>());
                if process::id() == pid {
                    info_omit!(cgroup_reset_admin());
                    return;
                }
                kill(Pid::from_raw(pid as libc::pid_t), signal::SIGTERM)
                    .c(d!())
                    .unwrap_or_else(|e| failed_list.push((pid, e)))
            });
            alt!(
                failed_list.is_empty(),
                Ok(()),
                Err(eg!(format!("{:#?}", failed_list)))
            )
        })
        .and_then(|_| cgroup_path.parent().ok_or(eg!(FUCK)))
        .map(|dir| {
            let dir = dir.to_owned();
            THREAD_POOL.spawn_ok(async move {
                async_sleep(5).await;
                info_omit!(fs::remove_dir(&dir))
            })
        })
}

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
