use crate::{nat, pause, resume, vm};
use core_def::{
    CliId, CliIdRef, EnvId, EnvIdRef, EnvInfo, EnvMeta, Ipv4, PubPort, VmCfg, VmId, VmInfo, VmKind,
    VmPort, CPU_DEFAULT, DISK_DEFAULT, EXEC_PORT, MEMORY_DEFAULT, SSH_PORT,
};
use lazy_static::lazy_static;
use myutil::{err::*, *};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicI32, AtomicU16, Ordering},
};
#[cfg(not(feature = "testmock"))]
use std::{thread, time};

const MAX_LIFE_TIME: u64 = 6 * 3600;
const MIN_START_STOP_ITV: u64 = 20;
const VM_PRESET_ID: i32 = -1;
pub const FUCK: &str = "THE FUCKING WORLD IS OVER !!!";
const VM_ID_LIMIT: i32 = 0xffff;
pub type OsName = String;
pub type ImageTag = String;
const PUB_PORT_LIMIT: u16 = 20000;
const PUB_PORT_BASE: u16 = 40000;

lazy_static! {
    static ref VM_ID: AtomicI32 = AtomicI32::new(0);
    static ref PUB_PORT: AtomicU16 = AtomicU16::new(PUB_PORT_BASE);
}

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

impl Drop for Vm {
    fn drop(&mut self) {
        if let Some(server) = self.serv_belong_to.upgrade() {
            server.vm_id_inuse.lock().remove(&self.id);
            if !self.during_stop {
                let mut rsc = server.resource.write();
                rsc.vm_active -= -1;
                rsc.cpu_used -= self.cpu_num;
                rsc.memory_used -= self.memory_size;
                rsc.disk_used -= self.disk_size;
            }
            if !self.port_map.is_empty() {
                let mut port_pub = vct![];
                let mut inuse = server.pub_port_inuse.lock();
                self.port_map.values().for_each(|port| {
                    inuse.remove(port);
                    port_pub.push(*port);
                });
                info_omit!(nat::clean_rule(&[self]))
            }
        }
        vm::post_clean(self)
    }
}

impl Vm {
    fn start_vm(&self) -> Result<()> {
        vm::start(self).c(d!())
    }

    /// gen ipv4
    fn gen_ip(vm_id: VmId) -> Ipv4 {
        Ipv4::new(format!("10.10.{}.{}", vm_id / 256, vm_id % 256))
    }

    /// alloc vm id
    fn alloc_id(&mut self, server: &Arc<Serv>) -> Result<VmId> {
        let mut vm_id_inuse = server.vm_id_inuse.lock();
        // create new id
        let vm_id = if VM_PRESET_ID == self.id {
            let mut cnter = 0;
            loop {
                let id = VM_ID.fetch_add(1, Ordering::Relaxed) % VM_ID_LIMIT;
                if vm_id_inuse.get(&id).is_none() {
                    vm_id_inuse.insert(id);
                    self.id = id;
                    break id;
                }
                cnter += 1;
                if VM_ID_LIMIT < cnter {
                    Err(eg!(FUCK))
                }
            }
        } else if vm_id_inuse.get(&self.id).is_none() {
            vm_id_inuse.insert(self.id);
            self.id
        } else {
            Err(eg!(FUCK))
        };
        Ok(vm_id)
    }

    fn pre_start(&self) -> Result<()> {
        vm::get_pre_starter(self)?(self).c(d!())
    }

    fn alloc_resource(&mut self, server: &Arc<Serv>) -> Result<()> {
        let mut res = server.resource.write();
        res.vm_active += 1;
        res.cpu_used += self.cpu_num;
        res.memory_used += self.memory_size;
        res.disk_used += self.disk_size;
        self.alloc_id(&server)
            .c(d!())
            .map(|id| self.ip = Self::gen_ip(id))
            .and_then(|_| self.alloc_pub_port(&server).c(d!()))
            .and_then(|_| nat::set_rule(self).c(d!()))
            .and_then(|_| self.pre_start().c(d!()))
    }

    /// create and cache vm meta message
    pub(crate) fn create_mate_from_cache(server: &Weak<Serv>, mut vm: Vm) -> Result<Vm> {
        if let Some(s) = server.upgrade() {
            vm.serv_belong_to = Weak::clone(server);
            vm.alloc_resource(&s).c(d!()).map(|_| vm)
        } else {
            Err(eg!(FUCK))
        }
    }

    pub(crate) fn create_mate(server: &Weak<Serv>, cfg: VmCfg) -> Result<Vm> {
        let cpu = cfg.cpu_num.unwrap_or(CPU_DEFAULT);
        let memory = cfg.memory_size.unwrap_or(MEMORY_DEFAULT);
        let disk = cfg.disk_szie.unwrap_or(DISK_DEFAULT);
        let mut vm = Vm {
            image_path: PathBuf::from(cfg.image_tag),
            kind: cfg.kind,
            cpu_num: cpu,
            memory_size: memory,
            disk_size: disk,
            serv_belong_to: Weak::clone(server),
            id: VM_PRESET_ID,
            ip: Ipv4::default(),
            port_map: cfg
                .port_list
                .into_iter()
                .fold(HashMap::new(), |mut acc, port| {
                    acc.insert(port, 0);
                    acc
                }),
            during_stop: false,
            image_cached: false,
            rand_uuid: cfg.rand_uuid,
        };
        if let Some(s) = server.upgrade() {
            vm.alloc_resource(&s).c(d!()).map(|_| vm)
        } else {
            Err(eg!(FUCK))
        }
    }

    fn alloc_pub_port(&mut self, server: &Arc<Serv>) -> Result<()> {
        let mut center = 0;
        let mut center_length = self.port_map.len();
        let mut buf = vct![];
        while 0 < center_length {
            let mut port_inuse = server.pub_port_inuse.lock();
            let port = PUB_PORT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                    Some(PUB_PORT_BASE + (1 + x) % PUB_PORT_LIMIT)
                })
                .map_err(|_| eg!(d!(FUCK)))
                .c(d!())?;
            if port_inuse.get(&port).is_none() {
                port_inuse.insert(port);
                buf.push(port);
                center_length -= 1;
            }
            center += 1;
            if PUB_PORT_LIMIT < center {
                Err(eg!(FUCK))
            }
        }
        self.port_map
            .values_mut()
            .zip(buf.into_iter())
            .for_each(|(p, port)| *p = port);
        Ok(())
    }
    pub fn get_id(&self) -> VmId {
        self.id
    }
    pub(crate) fn as_info(&self) -> VmInfo {
        VmInfo {
            os: self
                .image_path
                .file_name()
                .map(|f| f.to_str())
                .flatten()
                .unwrap_or("Unknown")
                .to_owned(),
            cpu_num: self.cpu_num,
            memory_size: self.memory_size,
            disk_size: self.disk_size,
            ip: self.ip.clone(),
            port_map: self.port_map.clone(),
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct Env {
    pub id: EnvId,
    start_timestamp: u64,
    end_timestamp: u64,
    is_stopped: bool,
    pub outgoing_denied: bool,
    last_mgmt_ts: u64,
    pub vm: HashMap<VmId, Vm>,
    #[serde(skip)]
    serv_belong_to: Weak<Serv>,
    #[serde(skip)]
    pub cli_belong_to: Option<CliId>,
}

impl Drop for Env {
    fn drop(&mut self) {
        if let Some(server) = self.serv_belong_to.upgrade() {
            server.env_id_inuse.lock().remove(&self.id);
            info_omit!(server.cfg_db.del(self))
        }
    }
}

impl Env {
    pub fn add_vm_set(&mut self, cfg: Vec<VmCfg>) -> Result<()> {
        self.add_vm_set_complex(cfg, vct!(), false).c(d!())
    }

    // patch create/recover/complex vm
    pub fn add_vm_set_complex(
        &mut self,
        cfg_set: Vec<VmCfg>,
        vm_set: Vec<Vm>,
        preload: bool,
    ) -> Result<()> {
        let mut vm: Vec<Vm> = vec![];
        self.check_resource(&cfg_set).c(d!())?;
        if preload {
            vm_set
                .into_iter()
                .for_each(|v| vm.push(Vm::create_mate_from_cache(&self.serv_belong_to, v)?));
        } else {
            cfg_set
                .into_iter()
                .for_each(|cfg| vm.push(Vm::create_mate(&self.serv_belong_to, cfg)?));
        }

        Self::check_image(&vm).c(d!());
        vm.into_iter()
            .filter(|v| !v.during_stop)
            .for_each(|v| v.start_vm().c(d!())?);
        vm.into_iter().for_each(|v| self.vm.insert(v.get_id(), v)?);
        if self.outgoing_denied {
            self.update_hardware(None, None, None, &[], Some(true))
                .c(d!())?
        }
        Ok(())
    }

    fn check_image(vm: &[Vm]) -> Result<()> {
        let mut cnter = 0;
        let path_set: Vec<_> = vm.iter().map(|v| vm_img_path(v)).collect();
        let mut time_out = (path_set.len() * 100) as u64;
        alt!(2000 > time_out, time_out = 2000);
        let time_out_unit = 200;
        let limit = time_out / time_out_unit;
        while path_set
            .iter()
            .map(|p| p.canonicalize())
            .any(|p| p.is_err())
        {
            if limit < cnter {
                Err(
                    eg!(@path_set.into_iter().filter( |p| p.canonicalize().is_err()).collect::<Vec<_>>()),
                )
            }
            cnter += 1;
            thread::sleep(time::Duration::from_millis(time_out_unit));
        }
        Ok(())
    }

    #[cfg(feature = "testmock")]
    fn check_image(_vm: &[Vm]) -> Result<()> {
        Ok(())
    }

    fn check_resource(&self, cfg_set: &[VmCfg]) -> Result<()> {
        if let Some(server) = self.serv_belong_to.upgrade() {
            let res = *server.resource.read();
            let (cpu, memory, disk) =
                cfg_set
                    .iter()
                    .fold((Some(0i32), Some(0i32), Some(0i32)), |end, vm| {
                        (
                            end.0
                                .map(|i| i.checked_add(vm.cpu_num.unwrap_or(CPU_DEFAULT)))
                                .flatten(),
                            end.1
                                .map(|i| i.checked_add(vm.memory_size.unwrap_or(MEMORY_DEFAULT)))
                                .flatten(),
                            end.2
                                .map(|i| i.checked_add(vm.disk_szie.unwrap_or(DISK_DEFAULT)))
                                .flatten(),
                        )
                    });
            let (cpu, memory, disk) = if let (Some(c), Some(m), Some(d)) = (cpu, memory, disk) {
                (c, m, d)
            } else {
                Err(eg!(FUCK))
            };

            if res.cpu_used.checked_add(cpu).ok_or(eg!(FUCK))? > res.cpu_total {
                Err(eg!(format!(
                    "cpu resource busy: total {}, used {} , need {}",
                    res.cpu_total, res.cpu_used, cpu
                )))
            }
            if res.memory_used.checked_add(memory).ok_or(eg!(FUCK))? > res.memory_total {
                Err(eg!(format!(
                    "memory resource busy: total {}, used {} , need {}",
                    res.memory_total, res.memory_used, memory
                )))
            }
            if res.disk_used.checked_add(disk).ok_or(eg!(FUCK))? > res.disk_total {
                Err(eg!(format!(
                    "disk resource busy: total {}, used {} , need {}",
                    res.disk_total, res.disk_used, disk
                )))
            }
        } else {
            Err(eg!(FUCK))
        }
        Ok(())
    }

    /// new env
    pub fn new(server: &Arc<Serv>, env_id: &EnvIdRef) -> Result<Env> {
        let mut inuse = server.env_id_inuse.lock();
        if inuse.get(env_id).is_none() {
            inuse.insert(env_id.to_owned());
            drop(inuse);
        } else {
            Err(eg!(FUCK))
        }
        Ok(Env {
            id: env_id.to_owned(),
            start_timestamp: ts!(),
            end_timestamp: 3600 + ts!(),
            is_stopped: false,
            outgoing_denied: false,
            last_mgmt_ts: 0,
            vm: HashMap::new(),
            serv_belong_to: Arc::downgrade(server),
            cli_belong_to: None,
        })
    }

    /// load env
    pub fn load(mut self, server: &Arc<Serv>) -> Result<Env> {
        let mut inuse = server.env_id_inuse.lock();
        if inuse.get(&self.id).is_none() {
            inuse.insert(self.id.clone());
            drop(inuse);
        }
        Ok(self)
    }

    /// check and update resource
    fn check_resource_and_set(&self, hardware: (i32, i32, i32)) -> Result<()> {
        if let Some(server) = self.serv_belong_to.upgrade() {
            let res = *server.resource.read();
            let vm_num = self.vm.len() as i32;
            let (cpu, memory, disk) = self.vm.values().fold((0, 0, 0), |mut before, vm| {
                before.0 += vm.cpu_num;
                before.1 += vm.memory_size;
                before.2 += vm.disk_size;
                before
            });
            let (cpu_new, memory_new, disk_new) = if let (Some(CPU), Some(MEMORY), Some(DISK)) = (
                hardware.0.checked_mul(vm_num),
                hardware.1.checked_mul(vm_num),
                hardware.2.checked_mul(vm_num),
            ) {
                (CPU, MEMORY, DISK)
            } else {
                Err(eg!(FUCK))
            };

            if cpu_new > cpu
                && res
                    .cpu_used
                    .checked_add(cpu_new)
                    .map(|i| i.checked_sub(cpu))
                    .flatten()
                    .ok_or(eg!(FUCK))?
                    > res.cpu_total
            {
                Err(eg!(format!(
                    "cpu resource busy: total {}MB , used {}MB , need {}MB",
                    res.cpu_total, res.cpu_used, cpu_new,
                )))
            }

            if memory_new > memory
                && res
                    .memory_used
                    .checked_add(memory_new)
                    .map(|i| i.checked_sub(memory))
                    .flatten()
                    .ok_or(eg!(FUCK))?
                    > res.memory_total
            {
                Err(eg!(format!(
                    "memory resource busy: total {}MB , used {}MB, need {}MB",
                    res.memory_total, res.memory_used, memory_new
                )))
            }

            if disk_new > disk
                && res
                    .disk_used
                    .checked_add(disk_new)
                    .map(|i| i.checked_sub(disk))
                    .flatten()
                    .ok_or(eg!(FUCK))?
                    > res.disk_total
            {
                Err(eg!(format!(
                    "disk resource busy: total {}MB , used {}MB , need {}MB ",
                    res.disk_total, res.disk_used, disk_new
                )))
            }
            let mut result = server.resource.write();
            result.cpu_used = result.cpu_used + (cpu_new / vm_num) - (cpu / vm_num);
            result.memory_used = result.memory_used + (memory_new / vm_num) - (memory / vm_num);
            result.disk_used = result.disk_used + (disk_new / vm_num) - (disk / vm_num);
        } else {
            Err(eg!(FUCK))
        }
        Ok(())
    }

    /// update hardware
    pub fn update_hardware(
        &mut self,
        cpu: Option<i32>,
        memory: Option<i32>,
        disk: Option<i32>,
        vm_port: &[VmPort],
        out_going: Option<bool>,
    ) -> Result<()> {
        if [&cpu, &memory, &disk].iter().any(|i| i.is_some()) {
            if !self.is_stopped {
                Err(eg!("env must be stopped before update it's hardware[s]."))
            }
            let (cpu_new, memory_new, disk_new) = if let Some(vm) = self.vm.values().next() {
                (
                    cpu.unwrap_or(vm.cpu_num),
                    memory.unwrap_or(vm.memory_size),
                    disk.unwrap_or(vm.disk_size),
                )
            } else {
                Ok(())
            };
            self.check_resource_and_set((cpu_new, memory_new, disk_new))
                .c(d!())?;
            self.vm.values_mut().for_each(|vm| {
                vm.cpu_num = cpu_new;
                vm.memory_size = memory_new;
                vm.disk_size = disk_new;
            });
        }
        if !vm_port.is_empty() {
            let mut port_vec = vm_port.to_vec();
            if let Some(server) = self.serv_belong_to.upgrade() {
                {
                    let mut lock = server.pub_port_inuse.lock();
                    let vm_set = self.vm.values().fold(vct![], |mut base, vm| {
                        vm.port_map.values().for_each(|port| {
                            lock.remove(port);
                            base.push(vm);
                        });
                        base
                    });
                    nat::clean_rule(vm_set.as_slice()).c(d!())
                }
                port_vec.push(SSH_PORT);
                port_vec.push(EXEC_PORT);
                port_vec.sort_unstable();
                port_vec.dedup();
                self.vm.values_mut().for_each(|vm| {
                    vm.port_map = port_vec.iter().map(|p| (*p, 0u16)).collect();
                    vm.alloc_pub_port(&server)
                        .c(d!())
                        .and_then(|_| nat::set_rule(vm).c(d!()))?;
                });
            } else {
                Err(eg!(FUCK))
            }
            if let Some(going) = out_going {
                let vm_set = self.vm.values().collect::<Vec<_>>();
                if going && !self.outgoing_denied {
                    nat::deny_outgoing(vm_set.as_slice()).c(d!())?;
                    self.outgoing_denied = true;
                } else if !going && self.outgoing_denied {
                    nat::allow_outgoing(vm_set.as_slice()).c(d!())?;
                    self.outgoing_denied = false;
                }
            }
        }
        Ok(())
    }

    /// update env lift time
    pub fn update_life(&mut self, s: u64, is_fucker: bool) -> Result<()> {
        if MAX_LIFE_TIME < s && !is_fucker {
            Err(eg!("life time so long!!!"))
        } else {
            self.end_timestamp = self.start_timestamp + s;
        }
    }

    /// convert env meta message
    fn as_meta(&self) -> EnvMeta {
        EnvMeta {
            id: self.id.clone(),
            start_timestamp: self.start_timetamp,
            end_timestamp: self.end_timestamp,
            vm_cnt: self.vm.len(),
            is_stopped: self.is_stopped,
        }
    }

    /// convert env info
    fn as_info(&self) -> EnvInfo {
        EnvInfo {
            id: self.id.clone(),
            start_timestamp: self.start_timestamp,
            end_timestamp: self.end_timestamp,
            vm: self.vm.iter().map(|(&k, v)| (k, v.as_info())).collect(),
            is_stopped: self.is_stopped,
        }
    }
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
    pub fn del(&self, env: &Env) -> Result<()> {
        let mut cfg_path = self.path.clone();
        let client = env
            .cli_belong_to
            .as_ref()
            .ok_or(eg!(FUCK))
            .c(d!())
            .map(base64::encode)
            .map(|cli| String::from_utf8_lossy(cli.as_bytes()).into_owned())?;
        cfg_path.push(client);
        cfg_path.push(format!("{}.json", env.id));
        fs::remove_file(cfg_path).c(d!())
    }

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
    /// update env cpu/memory/disk...
    pub fn update_env_hardware(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        cpu_memory_disk: (Option<i32>, Option<i32>, Option<i32>),
        vm_port: &[port],
        out_going: Option<bool>,
    ) -> Result<()> {
        let mut cli = self.cli.write();
        if let Some(env_set) = cli.get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                let (cpu, memory, disk) = cpu_memory_disk;
                env.update_hardware(cpu, memory, disk, vm_port, out_going)
                    .c(d!())
            } else {
                Err(eg!("env not exists"))
            }
        } else {
            Err(eg!("cli not exists"))
        }
    }

    /// patch delete vm
    pub fn update_env_del_vm(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        vm_id_set: &[VmId],
    ) -> Result<()> {
        let mut cli = self.cli.write();
        if let Some(env_set) = cli.get_mut(cli_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                vm_id_set.iter().for_each(|vm_id| {
                    env.vm.remove(vm_id);
                });
                self.cfg_db.write(cli_id, &env).c(d!())
            } else {
                Err(eg!("env not exists!!!"))
            }
        } else {
            Err(eg!("client not exists!!!"))
        }
    }

    /// update env life time
    pub fn update_env_life(
        &self,
        cli_id: &CliIdRef,
        env_id: &EnvIdRef,
        life_time: u64,
        is_fucker: bool,
    ) -> Result<()> {
        let mut cli = self.cli.write();
        if let Some(env_set) = cli.get_mut(env_id) {
            if let Some(env) = env_set.get_mut(env_id) {
                env.update_life(life_time, is_fucker)
                    .c(d!())
                    .and_then(|_| self.cfg_db.write(cli_id, &env).c(d!()))
            } else {
                Err(eg!("env not exists"))
            }
        } else {
            Err(eg!("client NOT exists"))
        }
    }

    /// get env details by dev_id
    pub fn get_env_details(&self, cli_id: &CliIdRef, env_set: Vec<EnvId>) -> Vec<EnvInfo> {
        let get_env_details = |env: &HashMap<EnvId, Env>| {
            env.values()
                .filter(|&v| env_set.iter().any(|vid| vid == v.id))
                .map(|env| env.as_info())
                .collect::<Vec<_>>()
        };
        self.cli
            .read()
            .get(cli_id)
            .map(get_env_details)
            .unwrap_or_default()
    }

    /// get env all meta message
    pub fn get_env_meta_all(&self) -> Vec<EnvMeta> {
        self.cli
            .read()
            .values()
            .map(|env| env.values().map(|i| i.as_meta()))
            .flatten()
            .collect::<Vec<_>>()
    }
    /// get env meta message
    pub fn get_env_meta(&self, env_id: &EnvIdRef) -> Vec<EnvMeta> {
        let get_env_meta =
            |env: &HashMap<EnvId, Env>| env.values().map(|env| env.as_meta()).collect::<Vec<_>>();
        self.cli
            .read()
            .get(env_id)
            .map(get_env_meta)
            .unwrap_or_default()
    }

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
