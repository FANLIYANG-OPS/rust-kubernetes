use crate::{pause, resume, vm};
use core_def::{CliId, CliIdRef, EnvId, EnvIdRef, Ipv4, PubPort, VmId, VmKind, VmPort};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_LIFE_TIME: u64 = 6 * 3600;
const MIN_START_STOP_ITV: u64 = 20;
const VM_PRESET_ID: i32 = -1;
const FUCK: &str = "THE FUCKING WORLD IS OVER !!!";

pub type OsName = String;
pub type ImageTag = String;

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Vm {
    pub image_path: PathBuf,
    pub kind: VmKind,
    pub cpu_num: i32,
    pub memory_size: i32,
    pub disk_size: i32,
    #[serde(skip)]
    serv_belong_to: Weak<Serv>,
    pub id: VmId,
    pub ip: Ipv4,
    pub port_map: HashMap<VmPort, PubPort>,
    pub during_stop: bool,
    pub image_cached: bool,
    pub rand_uuid: bool,
}

impl Vm {
    pub fn get_id(&self) -> VmId {
        self.id
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Env {
    pub id: EnvId,
    start_timetamp: u64,
    end_timestamp: u64,
    is_stopped: bool,
    pub outgoing_denied: bool,
    last_mgmt_ts: u64,
    pub vm: HashMap<VmId, Vm>,
    #[serde(skip)]
    serv_belong_to: Weak<Serv>,
    pub cli_belong_to: Option<CliId>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Resource {
    pub vm_active: i32,
    pub cpu_total: i32,
    pub cpu_used: i32,
    pub memory_total: i32,
    pub memory_used: i32,
    pub disk_total: i32,
    pub disk_used: i32,
}

impl Resource {
    #[inline(always)]
    pub fn new(cpu_total: i32, memory_total: i32, disk_total: i32) -> Resource {
        let mut res = Resource::default();
        res.cpu_total = cpu_total;
        res.memory_total = memory_total;
        res.disk_total = disk_total;
        res
    }
}

#[derive(Debug, Default)]
pub struct CfgDB {
    path: PathBuf,
}

impl CfgDB {
    #[inline(always)]
    pub fn new(path: &str) -> CfgDB {
        let p = Path::new(path);
        assert!(p.is_dir());
        CfgDB {
            path: p.to_path_buf(),
        }
    }
    /// write config to disk
    pub fn write(&self, cli_id: &CliIdRef, env: &Env) -> Result<()> {
        serde_json::to_string_pretty(env).c(d!()).and_then(|cfg| {
            let mut cfgpath = self.path.clone();
            cfgpath.push(base64::encode(cli_id));
            fs::create_dir_all(&cfgpath).c(d!())?;
            cfgpath.push(format!("{}.json", env.id));
            fs::write(cfgpath, cfg).c(d!())
        })
    }
}

#[derive(Debug, Default)]
pub struct Serv {
    cli: Arc<RwLock<HashMap<CliId, HashMap<EnvId, Env>>>>,
    env_id_inuse: Arc<Mutex<HashSet<EnvId>>>,
    vm_id_inuse: Arc<Mutex<HashSet<VmId>>>,
    pub_port_inuse: Arc<Mutex<HashSet<PubPort>>>,
    resource: Arc<RwLock<Resource>>,
    pub cfg_db: Arc<CfgDB>,
}

impl Serv {
    /// start env
    pub fn start_env(&self, cli_id: &CliIdRef, env_id: &EnvIdRef) -> Result<()> {
        if let Some(env_set) = self.cli.write().get_mut(env_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                let timestamp = ts!();
                if env.last_mgmt_ts + MIN_START_STOP_ITV > timestamp {
                    Err(eg!(format!(
                        "wait {} seconds , and try again!",
                        MIN_START_STOP_ITV
                    )))
                }
                env.last_mgmt_ts = timestamp;
                env.vm.values_mut().for_each(|vm| {
                    resume(vm).c(d!()).map(|_| {
                        let mut rsc = self.resource.write();
                        rsc.vm_active += 1;
                        rsc.cpu_used += vm.cpu_num;
                        rsc.memory_used += vm.memory_size;
                        rsc.disk_used += vm.disk_size;
                        vm.during_stop = false;
                    })?;
                });
                env.is_stopped = false;
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub fn new(cfg_path: &str) -> Serv {
        let mut s = Serv::default();
        s.cfg_db = Arc::new(CfgDB::new(cfg_path));
        s
    }
    #[inline(always)]
    pub fn set_resource(&self, rsc: Resource) {
        *self.resource.write() = Resource::new(rsc.cpu_total, rsc.memory_total, rsc.disk_total)
    }

    #[inline(always)]
    pub fn get_resource(&self) -> Resource {
        *self.resource.read()
    }

    pub fn clean_expired_env(&self) {
        let ts = ts!();
        let cli = self.cli.read();
        let expired = cli
            .iter()
            .map(|(cli_id, env)| {
                env.iter()
                    .filter(|(_, v)| v.end_timestamp < ts)
                    .map(move |(k, _)| (cli_id.clone(), k.clone()))
            })
            .flatten()
            .collect::<Vec<_>>();
        if !expired.is_empty() {
            drop(cli);
            let mut cli = self.cli.write();
            expired.iter().for_each(|(cli_id, key)| {
                cli.get_mut(cli_id.as_str()).map(|env| env.remove(key));
            })
        }
        vm::zobmie_clean();
    }

    /// add new client
    #[inline(always)]
    pub fn add_client(&self, id: CliId) -> Result<()> {
        let mut cli = self.cli.write();
        if cli.get(&id).is_some() {
            Err(eg!("Client already exists!"))
        } else {
            cli.insert(id, map! {});
            Ok(())
        }
    }

    /// delete client
    #[inline(always)]
    pub fn del_client(&self, id: &CliIdRef) {
        self.cli.write().remove(id);
    }
    /// if env is null , create it
    pub fn register_env(&self, id: CliId, mut env: Env) -> Result<()> {
        let cli_id = id.clone();
        let mut cli = self.cli.write();
        let env_set = cli.entry(id).or_insert(map!());
        if env_set.get(&env.id).is_some() {
            Err(eg!("env already exists!"))
        } else {
            env.vm.values_mut().for_each(|vm| vm.image_cached = true);
            self.cfg_db.write(&cli_id, &env).c(d!()).map(|_| {
                env.cli_belong_to = Some(cli_id);
                env_set.insert(env.id.clone(), env);
            })
        }
    }

    /// delete env
    pub fn del_env(&self, cli_id: &CliIdRef, env_id: &EnvIdRef) {
        if let Some(env_set) = self.cli.write().get_mut(cli_id) {
            if let Some(mut env) = env_set.remove(env_id) {
                env.vm.values_mut().for_each(|v| v.image_cached = false);
            }
        }
    }

    pub fn stop_env(&self, cli_id: &CliIdRef, env_id: &EnvIdRef) -> Result<()> {
        if let Some(env_set) = self.cli.write().get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                let timestamp = ts!();
                if env.last_mgmt_ts + MIN_START_STOP_ITV > timestamp {
                    Err(eg!(format!(
                        "wait {} seconds , and try again !",
                        MIN_START_STOP_ITV
                    )))
                }
                env.last_mgmt_ts = ts;
                env.vm.values_mut().for_each(|vm| {
                    pause(vm.get_id()).c(d!()).map(|_| {
                        let mut rsc = self.resource.write();
                        rsc.vm_active -= 1;
                        rsc.cpu_used -= vm.cpu_num;
                        rsc.memory_used -= vm.memory_used;
                        rsc.disk_used -= vm.disk_size;
                        vm.during_stop = true;
                    })?;
                });
                env.is_stopped = true;
            }
        }
        Ok(())
    }
}
