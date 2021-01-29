use crate::{async_sleep, common::CLONE_MARK, vm, THREAD_POOL, ZFS_ROOT};
use crate::{common::vm_img_path, linux, Vm};
use lazy_static::lazy_static;
use linux::vm::cmd_exec;
use myutil::{err::*, *};
use nix::{sys::wait, unistd};
use std::fs;
#[cfg(feature = "nft")]
pub(super) const BRIDGE: &str = "ttcore-bridge";

lazy_static! {
    static ref IOMMU: &'static str = {
        pnk!(fs::read("/proc/cpuinfo")
            .c(d!())
            .and_then(|cpu_info| String::from_utf8(cpu_info).c(d!()))
            .and_then(|cpu_info| {
                if cpu_info.contains(" svm ") {
                    Ok("amd-iommu")
                } else if cpu_info.contains(" vmx ") {
                    Ok("intel-iommu")
                } else {
                    Err(eg!("Unsupported platfrom!"))
                }
            }))
    };
}

#[cfg(feature = "zfs")]
#[inline(always)]
pub(super) fn remove_image(vm: &Vm) -> Result<()> {
    let args = format!(
        "zfs destory {root}/{clone_mark}/{id}",
        root = *ZFS_ROOT,
        clone_mark = CLONE_MARK,
        id = vm.id
    );
    THREAD_POOL.spawn_ok(async move {
        async_sleep(5).await;
        info_omit!(cmd_exec("sh", &["-c", &args]));
    });
    Ok(())
}

#[cfg(feature = "zfs")]
fn gen_disk_info(vm: &Vm) -> (String, String) {
    let disk = format!(
        "file={img},if=none,format=raw,cache=none,id=DISK_{id}",
        img = vm_img_path(vm).to_string_lossy(),
        id = vm.id
    );
    let disk_drive = format!("virtio-blk-pci,drive=DISK_{}", vm.id);
    (disk, disk_drive)
}

fn gen_vm_uuid() -> Result<String> {
    fs::read("/proc/sys/kernel/random/uuid")
        .c(d!())
        .and_then(|mut uuid| {
            uuid.pop();
            String::from_utf8(uuid).c(d!())
        })
}

pub(in crate::linux) fn wait_pid() {
    while let Ok(wait_status) =
        wait::waitpid(unistd::Pid::from_raw(-1), Some(wait::WaitPidFlag::WNOHANG))
    {
        if wait_status == wait::WaitStatus::StillAlive {
            break;
        }
    }
}

#[cfg(feature = "nft")]
#[inline(always)]
pub(super) fn set_tap(vm: &Vm) -> Result<()> {
    let tap = format!("TAP-{}", vm.id);
    cmd_exec("ip", &["link", "set", &tap, "master", &BRIDGE])
        .c(d!())
        .and_then(|_| cmd_exec("ip", &["link", "set", &tap, "up"]).c(d!()))
}

/// start vm
#[cfg(feature = "nft")]
pub(super) fn start(vm: &Vm) -> Result<()> {
    let cpu = vm.cpu_num.to_string();
    let memory = vm.memory_size.to_string();
    let net_dev = format!(
        "tap,ifname=TAP-{0},script=no,downscript=no,id=NET_{0}",
        vm.id
    );
    const WIDTH: usize = 2;
    let net_dev_device = format!(
        "virtio-net-pci,mac=52:54:00:11:{:>0width$x}:{:>width$x},netdev=NET_{}",
        vm.id / 256,
        vm.id % 256,
        vm.id,
        width = WIDTH
    );
    let (disk, disk_drive) = gen_disk_info(vm);
    let uuid = if vm.rand_uuid {
        gen_vm_uuid().c(d!())?
    } else {
        "5ce41b72-0e2e-48f9-8422-7647b557aba8".to_owned()
    };
    let args = &[
        "-enable-kvm",
        "-machine",
        "q35,accel=kvm",
        "-device",
        &IOMMU,
        "-cpu",
        "host",
        "-smp",
        cpu.as_str(),
        "-netdev",
        net_dev.as_str(),
        "-device",
        net_dev_device.as_str(),
        "-drive",
        disk.as_str(),
        "-device",
        disk_drive.as_str(),
        "-boot",
        "order=cd",
        "-vnc",
        &format!(":{}", vm.id),
        "-uuid",
        &uuid,
        "-daemonize",
    ];
    cmd_exec("qemu-system-x86_64", dbg!(args))
        .map(|_| {
            wait_pid();
        })
        .c(d!())
        .and_then(|_| set_tap(vm).c(d!()))
}

/// init network driver
#[cfg(feature = "nft")]
pub(super) fn init() -> Result<()> {
    fs::write("/proc/sys/net/ipv4/ip_forward", "1")
        .c(d!())
        .and_then(|_| cmd_exec("modprobe", &["tun"]).c(d!()))
        .and_then(|_| {
            cmd_exec("ip", &["addr", "flush", "dev", BRIDGE])
                .c(d!())
                .or_else(|e| cmd_exec("ip", &["link", "add", BRIDGE, "type", "bridge"]).c(d!()))
        })
        .and_then(|_| cmd_exec("ip", &["addr", "add", "10.0.0.1/8", "dev", BRIDGE]).c(d!()))
        .map(|_| {
            (0..1000).for_each(|n| {
                omit!(cmd_exec("ip", &["link", "del", &format!("TAP-{}", n)]));
            })
        })
}

#[inline(always)]
pub(crate) fn pre_start(vm: &Vm) -> Result<()> {
    if !vm.image_cached {
        create_image(vm).c(d!())?;
    }
    #[cfg(feature = "nft")]
    create_tap(&format!("TAP-{}", vm.id)).c(d!()).unwrap();
    Ok(())
}

#[cfg(feature = "zfs")]
pub(crate) fn create_image(vm: &Vm) -> Result<()> {
    let args = format!(
        "zfs clone -o volmode=dev {root}/{os}@base {root}/{clone_mark}{id}",
        root = *ZFS_ROOT,
        os = vm
            .image_path
            .file_name()
            .ok_or(eg!())
            .unwrap()
            .to_str()
            .ok_or(eg!())
            .unwrap(),
        clone_mark = CLONE_MARK,
        id = vm.id,
    );
    cmd_exec("sh", &["-c", &args]).c(d!())
}

#[cfg(feature = "nft")]
#[inline(always)]
fn create_tap(tap: &str) -> Result<()> {
    cmd_exec("ip", &["tuntap", "add", &tap, "mode", "tap"]).c(d!())
}

#[cfg(feature = "nft")]
#[inline(always)]
pub(super) fn remove_tap(vm: &Vm) -> Result<()> {
    let tap = format!("TAP-{}", vm.id);
    THREAD_POOL.spawn_ok(async move {
        async_sleep(5).await;
        info_omit!(cmd_exec("ip", &["tuntap", "del", &tap, "mode", "tap"]).c(d!()))
    });
    Ok(())
}

