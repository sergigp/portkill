use std::{collections::HashMap, num::ParseIntError, os::unix::process, process::Command};

#[derive(Debug)]
struct NetworkProcess {
    command: String,
    pid: u32,
    user: String,
    protocol: String, // tcp/udp
    port: u16,
}

#[derive(Debug)]
struct KillableProcess {
    pid: u32,
    command: String,
    ports: Vec<u16>,
}

fn main() {
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

            if !is_port_holder(name) {
                return None;
            }
            Some(NetworkProcess {
                command: fields.first()?.to_string(),
                pid: fields
                    .get(1)?
                    .parse()
                    .inspect_err(|e| eprintln!("bad pid {:?}: {e}", fields.get(1)))
                    .ok()?,
                user: fields.get(2)?.to_string(),
                protocol: fields.get(7)?.to_string(),
                port: parse_local_port(name)
                    .inspect_err(|e| eprintln!("bad port {:?}: {e}", name))
                    .ok()?,
            })
        })
        .collect::<Vec<NetworkProcess>>();

    let killable_processes = network_processes
        .into_iter()
        .fold(
            HashMap::<u32, Vec<NetworkProcess>>::new(),
            |mut grouped, p| {
                grouped.entry(p.pid).or_default().push(p);
                grouped
            },
        )
        .into_values()
        .map(|procs| {
            let first = procs.first().expect("empty group");
            KillableProcess {
                pid: first.pid,
                command: first.command.clone(),
                ports: procs.iter().map(|p| p.port).collect(),
            }
        })
        .collect::<Vec<_>>();

    dbg!(killable_processes);
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

fn parse_local_port(name: &str) -> Result<u16, ParseIntError> {
    let addr = name.split_whitespace().next().unwrap();
    addr.rsplit(':').next().unwrap().parse()
}
