use std::{collections::HashMap, num::ParseIntError, process::Command};

#[derive(Debug)]
pub struct PortProcess {
    pub command: String,
    pub pid: u32,
    pub user: String,
    pub ports: Vec<u16>,
    pub running_since: String, // @TODO format correctly
    pub cpu: f32,              // @TODO adjust type
    pub memory: f32,           // @TODO convert to MB
    pub path: String,
}

impl PortProcess {
    fn from_network_processes(network_processes: &[NetworkProcess]) -> Vec<PortProcess> {
        network_processes
            .into_iter()
            .fold(
                HashMap::<u32, Vec<&NetworkProcess>>::new(),
                |mut grouped, p| {
                    grouped.entry(p.pid).or_default().push(p);
                    grouped
                },
            )
            .into_values()
            .map(|procs| {
                let first = procs.first().expect("empty group");
                PortProcess {
                    command: first.command.clone(),
                    pid: first.pid,
                    user: first.user.clone(),
                    ports: procs.iter().map(|p| p.port).collect(),
                    running_since: first.running_since.clone(),
                    cpu: first.cpu,
                    memory: first.memory,
                    path: first.path.clone(),
                }
            })
            .collect::<Vec<_>>()
    }
}

pub fn get_port_processes() -> Vec<PortProcess> {
    let output = Command::new("lsof")
        .args(["-i", "-P", "-n"])
        .output()
        .expect("Error ocurred running lsof");

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        panic!("Error: {error}")
    }

    let content = String::from_utf8(output.stdout).expect("Unable to parse lsof response");

    let network_processes = content
        .lines()
        .skip(1)
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();

            let name = *fields.get(8)?;
            let command = fields.first()?.to_string();
            let pid: u32 = fields
                .get(1)?
                .parse()
                .inspect_err(|e| eprintln!("bad pid {:?}: {e}", fields.get(1)))
                .ok()?;
            let user = fields.get(2)?.to_string();
            let port = NetworkProcess::parse_local_port(name)
                .inspect_err(|e| eprintln!("bad port {:?}: {e}", name))
                .ok()?;

            if !NetworkProcess::is_port_holder(name) {
                return None;
            }

            let output = Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "etime=,%cpu=,rss=,args="])
                .output()
                .expect("Error ocurred running ps");

            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                panic!("Error: {error}")
            }

            let content = String::from_utf8(output.stdout).expect("Unable to parse ps response");

            let line = content.lines().next()?;

            let fields = NetworkProcess::parse_ps_line(line)?;
            dbg!(&fields);

            let running_since = fields.0.to_string();
            let cpu: f32 = fields
                .1
                .parse()
                .inspect_err(|e| eprintln!("bad cpu {:?}: {e}", fields.1))
                .ok()?;
            let memory_kb: f32 = fields
                .2
                .parse()
                .inspect_err(|e| eprintln!("bad memory {:?}: {e}", fields.2))
                .ok()?;
            let path = fields.3.to_string();

            Some(NetworkProcess {
                command,
                pid,
                user,
                port,
                running_since,
                cpu,
                memory: memory_kb / 1024.0, // TODO BUILD A PARSER FOR GB TOO
                path,
            })
        })
        .collect::<Vec<NetworkProcess>>();

    PortProcess::from_network_processes(&network_processes)
}

pub fn kill_port_process(process: &PortProcess) {
    println!("Going to kill process {}", process.pid);

    let output = Command::new("kill")
        .args(["-9", &process.pid.to_string()])
        .output()
        .expect("Error ocurred killing process");

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        panic!("Error: {error}")
    }
}

#[derive(Debug)]
struct NetworkProcess {
    command: String,
    pid: u32,
    user: String,
    port: u16,
    running_since: String, // @TODO format correctly
    cpu: f32,              // @TODO adjust type
    memory: f32,           // @TODO convert to MB
    path: String,
}

impl NetworkProcess {
    fn parse_local_port(name: &str) -> Result<u16, ParseIntError> {
        let addr = name.split_whitespace().next().unwrap();
        addr.rsplit(':').next().unwrap().parse()
    }

    fn is_port_holder(name: &str) -> bool {
        if name.contains("->") {
            // Outbound connection
            return false;
        }
        let Some(addr) = name.split_whitespace().next() else {
            return false;
        };
        let Some(port) = addr.rsplit(':').next() else {
            return false;
        };

        port != "*" && !port.is_empty()
    }

    fn parse_ps_line(line: &str) -> Option<(&str, &str, &str, &str)> {
        let line = line.trim_start();
        let (etime, rest) = line.split_once(char::is_whitespace)?;
        let rest = rest.trim_start(); // eats the run of spaces before %cpu
        let (cpu, rest) = rest.split_once(char::is_whitespace)?;
        let rest = rest.trim_start(); // eats the run before %mem
        let (mem, args) = rest.split_once(char::is_whitespace)?;
        Some((etime, cpu, mem, args.trim_start())) // eats the run before args
    }
}
