pub(crate) use real::*;

#[cfg(feature = "nft")]
pub(crate) mod real {

    use crate::{async_sleep, Vm, THREAD_POOL};
    use lazy_static::lazy_static;
    use myutil::{err::*, *};
    use parking_lot::Mutex;
    use std::{collections::HashSet, mem, process, sync::Arc};

    const TABLE_PROTO: &str = "ip";
    const TABLE_NAME: &str = "tt-core";

    lazy_static! {
        static ref RULE_SET: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vct![]));
        static ref RULE_SET_ALLOW_FAIL: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vct![]));
    }

    #[inline(always)]
    pub(crate) fn allow_outgoing(vm_set: &[&Vm]) -> Result<()> {
        let ip_set = vm_set
            .iter()
            .map(|vm| vm.ip.to_string())
            .collect::<Vec<_>>();
        if ip_set.is_empty() {
            return Ok(());
        }
        let args = format!(
            " delete element {proto} {table} BLACK_LIST {{{ip_set}}}; ",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            ip_set = ip_set.join(","),
        );
        RULE_SET_ALLOW_FAIL.lock().push(args);
        Ok(())
    }

    pub(crate) fn deny_outgoing(vm_set: &[&Vm]) -> Result<()> {
        let ip_set = vm_set
            .iter()
            .map(|vm| vm.ip.to_string())
            .collect::<Vec<_>>();
        if ip_set.is_empty() {
            return Ok(());
        }
        let args = format!(
            "add element {proto} {table} BLACK_LIST {{ {ip_set} }}",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            ip_set = ip_set.join(","),
        );
        RULE_SET.lock().push(args);
        Ok(())
    }

    pub(in crate::linux) fn init(server_ip: &str) -> Result<()> {
        set_rule_cron();
        let args = format!("
            add table {proto} {table};
            delete table {proto} {table};
            add table {proto} {table};
            add set {proto} {table} BLACK_LIST {{type ipv4_addr;}};
            add chain {proto} {table} FWD_CHAIN {{type filter hook forward priority 0; policy accept;}};
            add rule {proto} {table} FWD_CHAIN ct state established,related accept;
            add rule {proto} {table} FWD_CHAIN {proto} saddr @BLACK_LIST drop;
            add map {proto} {table} PORT_TO_PORT {{type inet_service: inet_service;}};
            add map {proto} {table} PORT_TO_IPV4 {{ type inet_service: ipv4_addr;}};
            add chain {proto} {table} DNAT_CHAIN {{type nat hook prerouting priority -100;}};
            add chain {proto} {table} SNAT_CHAIN {{ type nat hook postrouting priority 100 ;}};
            add rule {proto} {table} DNAT_CHAIN dnat tcp dport map @PORT_TO_IPV4: tcp dport map @PORT_TO_PORT;
            add rule {proto} {table} DNAT_CHAIN dnat udp dport map @PORT_TO_TPV4: udp dport map @PORT_TO_PORT;
            add rule {proto} {table} SNAT_CHAIN ip saddr 10.0.0.0/8 ip daddr != 10.0.0.0/8 snat to {pubip};",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            pubip=server_ip
        );
        nft_exec(&args).c(d!())
    }

    pub(crate) fn set_rule(vm: &Vm) -> Result<()> {
        if vm.port_map.is_empty() {
            return Ok(());
        }

        let mut port_to_ipv4: Vec<String> = vct![];
        let mut port_to_port: Vec<String> = vct![];

        vm.port_map.iter().for_each(|(vm_port, pub_port)| {
            port_to_ipv4.push(format!("{}:{}", pub_port, vm.ip.as_str()));
            port_to_port.push(format!("{}:{}", pub_port, vm_port));
        });

        let args = format!(
            "
            add element {proto} {table} PORT_TO_IPV4 {{ {ptoip} }};
            add element {proto} {table} PORT_TO_PORT {{ {ptop} }};
        ",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            ptoip = port_to_ipv4.join(","),
            ptop = port_to_port.join(","),
        );

        RULE_SET.lock().push(args);
        Ok(())
    }

    pub(crate) fn clean_rule(vm_set: &[&Vm]) -> Result<()> {
        let port_set = vm_set
            .iter()
            .map(|vm| vm.port_map.values())
            .flatten()
            .collect::<HashSet<_>>();
        if port_set.is_empty() {
            return Ok(());
        }
        let args = format!(
            "
            delete element {proto} {table} PORT_TO_IPV4 {{{pub_port}}};
            delete element {proto} {table} PORT_TO_PORT {{{pub_port}}};
            ",
            proto = TABLE_PROTO,
            table = TABLE_NAME,
            pub_port = port_set
                .iter()
                .map(|p| p.to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
        RULE_SET.lock().push(args);
        omit!(allow_outgoing(vm_set));
        Ok(())
    }

    fn set_rule_cron() {
        THREAD_POOL.spawn_ok(async {
            loop {
                async_sleep(2).await;
                let args = mem::take(&mut *RULE_SET.lock());
                if !args.is_empty() {
                    THREAD_POOL.spawn_ok(async move {
                        async_sleep(1).await;
                        info_omit!(nft_exec(dbg!(&args.join(""))));
                    })
                }
                let args_allow_fail = mem::take(&mut *RULE_SET_ALLOW_FAIL.lock());
                if !args_allow_fail.is_empty() {
                    THREAD_POOL.spawn_ok(async move {
                        async_sleep(1).await;
                        args_allow_fail.iter().for_each(|arg| {
                            info_omit!(nft_exec(dbg!(&arg)));
                        });
                    })
                }
            }
        });
    }

    fn nft_exec(args: &str) -> Result<()> {
        let args = format!("nft '{}'", args);
        let res = process::Command::new("sh")
            .args(&["-c", &args])
            .output()
            .c(d!())?;
        if res.status.success() {
            Ok(())
        } else {
            Err(eg!(String::from_utf8_lossy(&res.stderr)))
        }
    }
}
