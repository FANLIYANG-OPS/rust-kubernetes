use linux::vm::cmd_exec;
use myutil::{err::*, *};
use std::fs;

use crate::linux;
pub(super) const BRIDGE: &str = "k8s-bridge";

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
