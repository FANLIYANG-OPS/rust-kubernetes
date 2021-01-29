//!
//! # vm basic types definition
//!

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
};

/// C
pub const CPU_DEFAULT: i32 = 2;
/// MB
pub const MEMORY_DEFAULT: i32 = 1024;
/// MB
pub const DISK_DEFAULT: i32 = 40 * 1024;

/// client id
pub type CliId = String;
/// &client id
pub type CliIdRef = str;
/// env/container id
pub type EnvId = String;
/// &env/container id
pub type EnvIdRef = str;

/// the mac address last two paragraphs , max value is 255 x 255
pub type VmId = i32;
/// process id
pub type Pid = u32;

/// the vm default open port
pub const SSH_PORT: u16 = 22;
/// the vm open port for exec
pub const EXEC_PORT: u16 = 38888;

/// eg: 9211
pub type Port = u16;
/// inside port
pub type VmPort = Port;
/// out port , nat
pub type PubPort = Port;

/// ip
/// eg: 127.0.0.1
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct Ipv4 {
    addr: String,
}

impl Ipv4 {
    // create a new ip
    pub fn new(addr: String) -> Ipv4 {
        Ipv4 { addr }
    }
    // ipv4 convert to string
    pub fn as_str(&self) -> &str {
        self.addr.as_str()
    }
}

/// print value is ipv4.addr
impl fmt::Display for Ipv4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.addr)
    }
}

/// current support container type

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum VmKind {
    Qemu,
    Bhyve,
    FireCracker,
    Docker,
    Unknown,
}

/// print container type
impl fmt::Display for VmKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VmKind::Qemu => write!(f, "qemu"),
            VmKind::Bhyve => write!(f, "bhyve"),
            VmKind::FireCracker => write!(f, "fireCracker"),
            VmKind::Docker => write!(f, "docker"),
            VmKind::Unknown => write!(f, "unKnow"),
        }
    }
}

#[cfg(target_os = "linux")]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Docker
    }
}

#[cfg(target_os = "freebsd")]
impl Default for VmKind {
    fn default() -> VmKind {
        VmKind::Bhyve
    }
}

#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
impl Default for VmKind {
    fn default() -> Self {
        VmKind::Unknown
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VmInfo {
    pub os: String,
    pub cpu_num: i32,
    pub memory_size: i32,
    pub disk_size: i32,
    pub ip: Ipv4,
    pub port_map: HashMap<VmPort, PubPort>,
}

/// # env container message
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvMeta {
    /// env id
    pub id: EnvId,
    /// env create time
    pub start_timestamp: u64,
    /// env stop time
    pub end_timestamp: u64,
    /// inside container
    pub vm_cnt: usize,
    /// is stop
    pub is_stopped: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EnvInfo {
    pub id: EnvId,
    pub start_timestamp: u64,
    pub end_timestamp: u64,
    pub vm: BTreeMap<VmId, VmInfo>,
    pub is_stopped: bool,
}

#[derive(Clone, Debug)]
pub struct VmCfg {
    pub image_tag: String,
    pub port_list: Vec<VmPort>,
    pub kind: VmKind,
    pub cpu_num: Option<i32>,
    pub memory_size: Option<i32>,
    pub disk_szie: Option<i32>,
    pub rand_uuid: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VmCfgProxy {
    pub image_tag: String,
    pub port_list: Vec<VmPort>,
    pub cpu_num: Option<i32>,
    pub memoy_size: Option<i32>,
    pub disk_size: Option<i32>,
    pub rand_uuid: bool,
}
