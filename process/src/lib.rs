use anyhow::Result;
use nix::unistd::{sysconf, SysconfVar};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fs;

#[cfg(target_os = "linux")]
fn get_all_pids() -> Result<Vec<String>> {
    let mut pids_list: Vec<String> = Vec::new();
    let proc_file_path = fs::read_dir("/proc")?;
    for file in proc_file_path {
        if let Some(file_name) = file?.file_name().to_str() {
            if let Some(pid) = file_name.chars().next() {
                if pid.is_numeric() {
                    pids_list.push(String::from(file_name));
                }
            }
        }
    }
    Ok(pids_list)
}

#[cfg(target_os = "linux")]
fn get_proc_name(pid: u32) -> Result<String> {
    let mut proc_name = String::new();
    let buffer = fs::read_to_string(format!("/proc/{}/status", pid))?;
    for line in buffer.lines() {
        if line.contains("Name:") {
            proc_name = line
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string();
            break;
        }
    }
    Ok(proc_name)
}
#[cfg(target_os = "linux")]
fn get_proc_cpu_usage(pid: u32) -> Result<f32> {
    let system_clock_tick = sysconf(SysconfVar::CLK_TCK)?.unwrap_or(100) as f32;
    let buffer = fs::read_to_string(format!("/proc/{}/stat", pid))?;
    let proc_utime = buffer
        .split_whitespace()
        .nth(13)
        .unwrap_or("0")
        .parse::<f32>()?;
    let proc_stime = buffer
        .split_whitespace()
        .nth(14)
        .unwrap_or("0")
        .parse::<f32>()?;
    let proc_starttime = buffer
        .split_whitespace()
        .nth(21)
        .unwrap_or("0")
        .parse::<f32>()?;
    let system_uptime = fs::read_to_string("/proc/uptime")?
        .split_whitespace()
        .next()
        .unwrap()
        .parse::<f32>()?;
    let total_time = proc_utime + proc_stime;
    let seconds = system_uptime - (proc_starttime / system_clock_tick);
    let num_of_cpus = fs::read_to_string("/proc/cpuinfo")?
        .lines()
        .filter(|line| line.contains("processor"))
        .count() as f32;
    Ok(100f32 * ((total_time / system_clock_tick) / seconds) / num_of_cpus)
}
#[cfg(target_os = "linux")]
fn get_proc_mem_usage(pid: u32) -> Result<f32> {
    let mut rss = 0f32; // in MB
    let mut buffer = fs::read_to_string(format!("/proc/{}/status", pid))?;
    for line in buffer.lines() {
        if line.contains("VmRSS:") {
            rss = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse::<f32>()?;
            break;
        }
    }
    buffer = fs::read_to_string("/proc/meminfo")?;
    let mut total_mem = 0f32;
    for line in buffer.lines() {
        if line.contains("MemTotal:") {
            total_mem = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse::<f32>()?;
            break;
        }
    }
    if total_mem != 0f32 {
        return Ok(rss / total_mem * 100f32);
    }
    Ok(0f32)
}
#[cfg(target_os = "linux")]
fn get_proc_path(pid: u32) -> Result<String> {
    let buffer = fs::read_link(format!("/proc/{}/exe", pid));
    if let Ok(proc_path) = buffer {
        return Ok(proc_path.to_str().unwrap_or("").to_string());
    }
    Ok("".to_string())
}
#[cfg(target_os = "linux")]
fn get_proc_user(pid: u32) -> Result<String> {
    let mut proc_user_uid = String::new();
    let mut buffer = fs::read_to_string(format!("/proc/{}/status", pid))?;
    for line in buffer.lines() {
        if line.contains("Uid:") {
            proc_user_uid = line
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .to_string();
            break;
        }
    }
    buffer = fs::read_to_string("/etc/passwd")?;
    for line in buffer.lines() {
        if let Some(proc_user_line_uid) = line.split(':').nth(2) {
            if proc_user_line_uid == proc_user_uid {
                return Ok(line.split(':').next().unwrap().to_string());
            }
        }
    }
    Ok("".to_string())
}
#[cfg(target_os = "linux")]
fn get_proc_ppid(pid: u32) -> Result<u32> {
    let mut proc_ppid = 0u32;
    let buffer = fs::read_to_string(format!("/proc/{}/status", pid))?;
    for line in buffer.lines() {
        if line.contains("PPid:") {
            proc_ppid = line
                .split_whitespace()
                .last()
                .unwrap_or_default()
                .parse::<u32>()?;
            break;
        }
    }
    Ok(proc_ppid)
}
#[cfg(target_os = "linux")]
fn get_total_cpu_usage() -> Result<f32> {
    let mut total_cpu_usage = 0f32;
    let buffer = fs::read_to_string("/proc/stat")?;
    if let Some(cpu_metrics_line) = buffer.lines().next() {
        let mut idle_time = 0f32;
        let mut index = 0usize;
        let mut cpu_metrics_splitter = cpu_metrics_line.split_whitespace();
        while index < 10 {
            let cpu_metric = cpu_metrics_splitter.next().unwrap_or("0");
            if index == 0 {
                index += 1;
                continue;
            }
            if index == 4 {
                idle_time = cpu_metric.parse::<f32>()?;
            }
            total_cpu_usage += cpu_metric.parse::<f32>()?;
            index += 1;
        }
        total_cpu_usage = 100f32 - (idle_time * 100f32) / total_cpu_usage;
    }
    Ok(total_cpu_usage)
}
#[cfg(target_os = "linux")]
fn get_total_mem_usage() -> Result<f32> {
    let buffer = fs::read_to_string("/proc/meminfo")?;
    let mut free_mem = 0f32;
    let mut total_mem = 0f32;
    for line in buffer.lines() {
        if line.contains("MemFree:") || line.contains("Buffers") || line.contains("Cached") {
            free_mem += line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse::<f32>()?;
        }
        if line.contains("MemTotal:") {
            total_mem = line
                .split_whitespace()
                .nth(1)
                .unwrap_or("0")
                .parse::<f32>()?;
        }
    }
    Ok(100f32 - (free_mem * 100f32) / total_mem)
}

#[derive(Clone)]
pub struct Process {
    pid: u32,
    name: String,
    cpu_used: f32,
    mem_used: f32,
    path: String,
    user: String,
    ppid: u32,
}

#[derive(Clone)]
pub struct ProcessTreeNode {
    pub proc_info: Process,
    pub children: Vec<ProcessTreeNode>,
}

#[derive(Clone)]
pub struct ProcessTree {
    pub root: ProcessTreeNode,
}

impl ProcessTreeNode {
    fn new(proc_info: &Process) -> ProcessTreeNode {
        ProcessTreeNode {
            proc_info: (*proc_info).clone(),
            children: Vec::new(),
        }
    }
}

pub fn build_process_tree(system: &System) -> ProcessTree {
    let mut proc_tree = ProcessTree {
        root: ProcessTreeNode::new(&Process {
            pid: 0,
            name: "System Hierarchy".to_string(),
            cpu_used: 0f32,
            mem_used: 0f32,
            path: "".to_string(),
            user: "".to_string(),
            ppid: 0,
        }),
    };
    let root = &mut proc_tree.root;
    build_process_tree_data(root, system);
    proc_tree
}

fn build_process_tree_data(proc_node: &mut ProcessTreeNode, system: &System) {
    let mut ppid_map: HashMap<u32, Vec<u32>> = HashMap::new();
    let pids_str_list = get_all_pids().unwrap_or_default();
    for pid_str in pids_str_list {
        if let Ok(pid_u32) = pid_str.parse::<u32>() {
            if let Some(value) = system.procs.get(&pid_u32) {
                match ppid_map.entry(value.get_ppid()) {
                    Vacant(entry) => {
                        entry.insert(vec![pid_u32]);
                    }
                    Occupied(mut entry) => {
                        entry.get_mut().push(pid_u32);
                    }
                }
            }
        }
    }
    build_process_tree_relations(proc_node, &system.procs, &ppid_map)
}

fn build_process_tree_relations(
    proc_node: &mut ProcessTreeNode,
    procs: &HashMap<u32, Process>,
    ppid_map: &HashMap<u32, Vec<u32>>,
) {
    let pid = proc_node.proc_info.pid;
    if let Some(children) = ppid_map.get(&pid) {
        proc_node.children.extend(children.iter().map(|child_pid| {
            let proc = &procs[child_pid];
            let mut child = ProcessTreeNode::new(proc);
            build_process_tree_relations(&mut child, procs, ppid_map);
            child
        }));
    }
}

impl Process {
    fn new() -> Self {
        Self {
            pid: 0u32,
            name: String::new(),
            cpu_used: 0f32,
            mem_used: 0f32,
            path: String::new(),
            user: String::new(),
            ppid: 0u32,
        }
    }
    pub fn get_name(&self) -> &String {
        &self.name
    }
    pub fn get_cpu_used(&self) -> f32 {
        self.cpu_used
    }
    pub fn get_mem_used(&self) -> f32 {
        self.mem_used
    }
    pub fn get_path(&self) -> &String {
        &self.path
    }
    pub fn get_user(&self) -> &String {
        &self.user
    }
    pub fn get_ppid(&self) -> u32 {
        self.ppid
    }
    pub fn get_pid(&self) -> u32 {
        self.pid
    }
}
pub struct System {
    procs: HashMap<u32, Process>,
    cpu_used: f32,
    mem_used: f32,
}

impl System {
    pub fn new() -> Self {
        Self {
            procs: HashMap::new(),
            cpu_used: 0f32,
            mem_used: 0f32,
        }
    }
    pub fn refresh_system_info(&mut self) -> Result<()> {
        self.procs.clear();
        let pid_str_list = get_all_pids()?;
        for pid_str in pid_str_list {
            let mut process_info = Process::new();
            let pid = pid_str.parse::<u32>()?;
            process_info.pid = pid;
            process_info.name = get_proc_name(pid)?;
            process_info.cpu_used = get_proc_cpu_usage(pid)?;
            process_info.mem_used = get_proc_mem_usage(pid)?;
            process_info.path = get_proc_path(pid)?;
            process_info.user = get_proc_user(pid)?;
            process_info.ppid = get_proc_ppid(pid)?;
            self.procs.insert(pid, process_info);
        }
        self.cpu_used = get_total_cpu_usage()?;
        self.mem_used = get_total_mem_usage()?;
        Ok(())
    }
    pub fn get_proc_info(&self, pid: &u32) -> Option<&Process> {
        self.procs.get(pid)
    }
    pub fn get_total_cpu_usage(&self) -> f32 {
        self.cpu_used
    }
    pub fn get_total_mem_usage(&self) -> f32 {
        self.mem_used
    }
    pub fn get_procs_as_list(&mut self) -> Vec<(u32, Process)> {
        let proc_list: Vec<(u32, Process)> = self.procs.drain().collect();
        proc_list
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

pub fn print_proc_info(proc: &Process) {
    println!("Name:        {}", proc.name);
    println!("CPU used:    {}", proc.cpu_used);
    println!("Memory used: {}", proc.mem_used);
    println!("Path:        {}", proc.path);
    println!("User:        {}", proc.user);
    println!("Ppid:        {}", proc.ppid);
}
