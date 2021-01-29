pub use core_def::*;
use fmt::write;
use myutil::{err::*, *};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const OPS_ID_LEN: usize = 4;
pub const DEFAULT_REQ_ID: u64 = std::u64::MAX;
pub type UUID = u64;
pub type ServerAddress = String;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Request<T: Serialize> {
    pub uuid: UUID,
    pub cli_id: CliId,
    pub msg: T,
}

impl<T: Serialize> Request<T> {
    pub fn new(uuid: UUID, cli_id: CliId, msg: T) -> Self {
        Request { uuid, cli_id, msg }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub enum ResultStatus {
    Fail,
    Success,
}

impl fmt::Display for ResultStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ResultStatus::Fail => "Fail",
            ResultStatus::Success => "Success",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    pub uuid: UUID,
    pub status: ResultStatus,
    pub msg: Vec<u8>,
}
impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "status: {} , msg: {}",
            self.status,
            String::from_utf8_lossy(&self.msg)
        )
    }
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct ResponseGetServInfo {
    pub vm_total: i32,
    pub cpu_total: i32,
    pub cpu_used: i32,
    pub memory_total: i32,
    pub memory_used: i32,
    pub disk_total: i32,
    pub disk_used: i32,
    pub supported_list: Vec<String>,
}

pub type ResponseGetEnvList = Vec<EnvMeta>;

#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RequestGetEnvInfo {
    pub env_set: Vec<EnvId>,
}

pub type ResponseGetEnvInfo = Vec<EnvInfo>;

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestAddressEnv {
    pub env_id: EnvId,
    pub life_time: Option<u64>,
    pub dup_each: Option<u32>,
    pub deny_outgoing: bool,
    pub vm_cfg: Option<Vec<VmCfgProxy>>,
    pub os_prefix: Vec<String>,
    pub cpu_num: Option<i32>,
    pub memory_size: Option<i32>,
    pub disk_size: Option<i32>,
    pub port_set: Vec<Port>,
    pub rand_bool: bool,
}

impl RequestAddressEnv {
    pub fn set_ssh_port(&mut self) {
        let set = |data: &mut Vec<VmPort>| {
            data.push(SSH_PORT);
            data.push(EXEC_PORT);
            data.sort_unstable();
            data.dedup();
        };
        if let Some(vc) = self.vm_cfg.as_mut() {
            vc.iter_mut().for_each(|cfg| {
                set(&mut cfg.port_list);
            });
        } else {
            set(&mut self.port_set);
        }
    }
    pub fn set_os_lower(&mut self) {
        self.os_prefix
            .iter_mut()
            .for_each(|os| *os = os.to_lowercase());
    }
    pub fn check_dup(&self) -> Result<u32> {
        const DUP_MAX: u32 = 500;
        let dup_each = self.dup_each.unwrap_or(0);
        if DUP_MAX < dup_each {
            Err(eg!(format!(
                "the number of 'dup' too large: {}(max {})",
                dup_each, DUP_MAX
            )))
        } else {
            Ok(dup_each)
        }
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RequestStopEnv {
    pub env_id: EnvId,
}

pub type RequestStartEnv = RequestStopEnv;

#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RequestUpdateEnvLife {
    pub env_id: EnvId,
    pub life_time: u64,
    pub is_fucker: bool,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RequestUpdateEnvResource {
    pub env_id: EnvId,
    pub cpu_num: Option<i32>,
    pub memory_size: Option<i32>,
    pub disk_size: Option<i32>,
    pub vm_port: Vec<u16>,
    pub deny_outgoing: Option<bool>,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RequestDeleteEnv {
    pub env_id: EnvId,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RequestUpdateEnvKickVm {
    pub env_id: EnvId,
    pub vm_id: Vec<VmId>,
    pub os_prefix: Vec<String>,
}
