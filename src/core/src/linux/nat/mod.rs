// pub(in crate::linux) fn init(serv_ip: &str) -> Result<()> {
//     set_rule_cron();

//     let arg = format!("
//         add table {proto} {table};
//         delete table {proto} {table};
//         add table {proto} {table};
//         add set {proto} {table} BLACK_LIST {{ type ipv4_addr; }};
//         add chain {proto} {table} FWD_CHAIN {{ type filter hook forward priority 0; policy accept; }};
//         add rule {proto} {table} FWD_CHAIN ct state established,related accept;
//         add rule {proto} {table} FWD_CHAIN {proto} saddr @BLACK_LIST drop;
//         add map {proto} {table} PORT_TO_PORT {{ type inet_service: inet_service; }};
//         add map {proto} {table} PORT_TO_IPV4 {{ type inet_service: ipv4_addr; }};
//         add chain {proto} {table} DNAT_CHAIN {{ type nat hook prerouting priority -100; }};
//         add chain {proto} {table} SNAT_CHAIN {{ type nat hook postrouting priority 100; }};
//         add rule {proto} {table} DNAT_CHAIN dnat tcp dport map @PORT_TO_IPV4: tcp dport map @PORT_TO_PORT;
//         add rule {proto} {table} DNAT_CHAIN dnat udp dport map @PORT_TO_IPV4: udp dport map @PORT_TO_PORT;
//         add rule {proto} {table} SNAT_CHAIN ip saddr 10.0.0.0/8 ip daddr != 10.0.0.0/8 snat to {pubip};
//         ",
//         proto=TABLE_PROTO,
//         table=TABLE_NAME,
//         pubip=serv_ip,
//     );

//     nft_exec(&arg).c(d!())
// }

#[cfg(feature = "nft")]
pub(crate) mod real {

    use crate::{async_sleep, Vm, THREAD_POOL};
    use lazy_static::lazy_static;
    use myutil::{err::*, *};
    use parking_lot::Mutex;
    use std::{mem, process, sync::Arc};

    const TABLE_PROTO: &str = "ip";
    const TABLE_NAME: &str = "tt-core";

    lazy_static! {
        static ref RULE_SET: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vct![]));
        static ref RUST_SET_ALLOW_FAIL: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vct![]));
    }

    // pub(in crate::linux) fn init(serv_ip: &str) -> Result<()> {
    //     set_rule_cron();

    //     let arg = format!("

    //         add chain {proto} {table} FWD_CHAIN {{ type filter hook forward priority 0; policy accept; }};
    //         add rule {proto} {table} FWD_CHAIN ct state established,related accept;
    //         add rule {proto} {table} FWD_CHAIN {proto} saddr @BLACK_LIST drop;

    //         add map {proto} {table} PORT_TO_PORT {{ type inet_service: inet_service; }};
    //         add map {proto} {table} PORT_TO_IPV4 {{ type inet_service: ipv4_addr; }};
    //         add chain {proto} {table} DNAT_CHAIN {{ type nat hook prerouting priority -100; }};
    //         add chain {proto} {table} SNAT_CHAIN {{ type nat hook postrouting priority 100; }};
    //         add rule {proto} {table} DNAT_CHAIN dnat tcp dport map @PORT_TO_IPV4: tcp dport map @PORT_TO_PORT;
    //         add rule {proto} {table} DNAT_CHAIN dnat udp dport map @PORT_TO_IPV4: udp dport map @PORT_TO_PORT;
    //         add rule {proto} {table} SNAT_CHAIN ip saddr 10.0.0.0/8 ip daddr != 10.0.0.0/8 snat to {pubip};
    //         ",
    //         proto=TABLE_PROTO,
    //         table=TABLE_NAME,
    //         pubip=serv_ip,
    //     );

    //     nft_exec(&arg).c(d!())
    // }

    pub(in crate::linux) fn init(server_ip: &str) -> Result<()> {
        set_rule_cron();
        let args = format!(
            "
            add table {proto} {table};
            delete table {proto} {table};
            add table {proto} {table};
            add set {proto} {table} BLACK_LIST {{type ipv4_addr;}};
            add chain {proto} {table} FWD_CHAIN {{type filter hook forward priority 0; policy accept;}};
        ",
            proto = TABLE_PROTO,
            table = TABLE_NAME
        );
        nft_exec(&args).c(d!())
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
                let args_allow_fail = mem::take(&mut *RUST_SET_ALLOW_FAIL.lock());
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
