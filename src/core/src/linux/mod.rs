#[cfg(feature = "zfs")]
use crate::{img_root_register, ImageTag, OsName, Vm, CLONE_MARK};
use core_def::VmKind;
use myutil::{err::*, *};
use nix::sched::{clone, CloneFlags};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
pub(crate) mod nat;
pub(crate) mod vm;

/// system entrance
#[cfg(feature = "zfs")]
#[inline(always)]
pub fn exec(img_path: &str, func: fn() -> Result<()>, server_ip: &str) -> Result<()> {
    img_root_register(Some(img_path));
    do_exec(func, server_ip).c(d!())
}

fn do_exec(func: fn() -> Result<()>, server_ip: &str) -> Result<()> {
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
                .and_then(|_| vm::util::mount_dyn_fs_proc().c(d!()))
                .and_then(|_| vm::util::mount_tmp_fs().c(d!()))
                .and_then(|_| vm::engine::init().c(d!()))
                .and_then(|_| vm::cgroup::init().c(d!()))
                .and_then(|_| nat::init(server_ip).c(d!())) // todo
        )
        .and(Ok(0))
        .or::<Result<i32>>(Ok(-1))
        .unwrap()
    };
    clone(
        Box::new(ops),
        stack.as_mut_slice(),
        flags,
        Some(libc::SIGCHLD),
    )
    .c(d!())
    .map(|_| ())
}

fn get_image_path(img_path: &str) -> Result<Vec<PathBuf>> {
    let mut res: Vec<PathBuf> = vct![];
    let dir = Path::new(img_path);
    if dir.is_dir() {
        for entry in fs::read_dir(dir).c(d!())? {
            let entry = entry.c(d!())?;
            let path = entry.path();
            if let Some(p) = path.to_str() {
                res.push(PathBuf::from(p));
            }
        }
    }
    Ok(res)
}

/// get vm engine by image prefix
pub fn vm_kind(os: &str) -> Result<VmKind> {
    let os = os.to_lowercase();
    for (prefix, kind) in &[("qemu", VmKind::Qemu), ("fire", VmKind::FireCracker)] {
        if os.starts_with(prefix) {
            return Ok(*kind);
        }
    }
    Err(eg!("Invaild os name"))
}

#[cfg(feature = "zfs")]
pub fn get_os_info(image_path: &str) -> Result<HashMap<OsName, ImageTag>> {
    get_image_path(image_path).c(d!()).map(|path| {
        path.iter()
            .filter_map(|i| {
                i.file_name()
                    .map(|j| j.to_str())
                    .flatten()
                    .map(|os| (os, i))
            })
            .filter(|(os, _)| {
                vm_kind(os).is_ok()
                    && !(os.contains('@') || os.contains("-part") || os.starts_with(CLONE_MARK))
            })
            .map(|(os, i)| (os.to_lowercase(), i.to_string_lossy().into_owned()))
            .collect()
    })
}

#[ignore = "always"]
pub fn pause() -> Result<()> {
    vm::cgroup::init().c(d!())
}

#[inline(always)]
pub fn resume(vm: &Vm) -> Result<()> {
    vm::start(vm).c(d!())
}

fn gen_walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut res: Vec<PathBuf> = vct![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir).c(d!()).unwrap() {
            let entry = entry.c(d!()).unwrap();
            let path = entry.path();
            if path.is_dir() {
                res.append(&mut gen_walk(&path).c(d!()).unwrap());
            } else if let Some(p) = path.to_str() {
                res.push(PathBuf::from(p));
            }
        }
    }
    Ok(res)
}
