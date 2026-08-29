use netstat2::{AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, get_sockets_info};
use std::{
    collections::{HashMap, HashSet},
    process::Command,
};
use sysinfo::{Pid, ProcessesToUpdate, System, Users};

#[derive(Debug)]
pub struct PortProcess {
    pub command: String,
    pub pid: u32,
    pub user: String,
    pub ports: HashSet<u16>,
    pub running_since: String, // @TODO format correctly
    pub cpu: f32,
    pub memory: u64,
    pub path: String,
}

impl PortProcess {
    pub fn sort_key(&self) -> (&str, u32) {
        (&self.command, self.pid)
    }
}

impl PortProcess {
    pub fn without_sysinfo(pid: u32, ports: HashSet<u16>) -> PortProcess {
        PortProcess {
            command: "".to_owned(),
            pid,
            user: "".to_owned(),
            ports,
            running_since: "".to_owned(),
            cpu: 0.0,
            memory: 0,
            path: "".to_owned(),
        }
    }
}

pub fn get_port_processes(sys: &mut System) -> Vec<PortProcess> {
    let sockets = get_sockets_info(
        AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6,
        ProtocolFlags::TCP,
    )
    .expect("Error ocurred retrieving sockets info");

    let pids_with_ports: HashMap<u32, HashSet<u16>> = sockets
        .iter()
        .filter(|s| 
            matches!(&s.protocol_socket_info, ProtocolSocketInfo::Tcp(tcp) if tcp.state == netstat2::TcpState::Listen)
        )
        .fold(HashMap::new(), |mut acc, s| {
            let pids = &s.associated_pids;
            let port: u16 = s.local_port();
            pids.iter().for_each(|pid| {
                acc.entry(*pid).or_insert_with(HashSet::new).insert(port);
            });
            acc
        });

    let pids: Vec<Pid> = pids_with_ports.keys().map(|p| Pid::from_u32(*p)).collect();

    sys.refresh_processes(ProcessesToUpdate::Some(&pids), true);

    let processes = sys.processes();
    let users = Users::new_with_refreshed_list();

    let mut port_processes: Vec<PortProcess> = pids_with_ports
        .into_iter()
        .map(|(pid, ports)| {
            match processes.get(&Pid::from_u32(pid)) {
                Some(process) => {
                    let user_name = process
                        .user_id()
                        .and_then(|uid| users.get_user_by_id(uid))
                        .map(|u| u.name().to_owned())
                        .unwrap_or_else(|| "?".to_owned());

                    PortProcess {
                        command: process.name().to_string_lossy().into_owned(),
                        pid,
                        user: user_name,
                        ports,
                        running_since: "?".to_owned(),
                        cpu: process.cpu_usage(), // first iteration will be 0
                        memory: process.memory(),
                        path: process
                            .exe()
                            .map_or("?".to_owned(), |v| v.to_string_lossy().into_owned()),
                    }
                }
                None => PortProcess::without_sysinfo(pid, ports),
            }
        })
        .collect();

    port_processes.sort_by(|a, b| a.sort_key().cmp(&b.sort_key()));
    port_processes
}

pub fn kill_port_process(process: &PortProcess) {
    let output = Command::new("kill")
        .args(["-9", &process.pid.to_string()])
        .output()
        .expect("Error ocurred killing process");

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        panic!("Error: {error}")
    }
}
