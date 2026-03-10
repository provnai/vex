use crate::traits::{self, ConnectionInfo};
#[allow(unused_imports)]
use anyhow::{anyhow, Result};
#[allow(unused_imports)]
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Factory for blocking scraper
pub fn create_network_scraper() -> Box<dyn traits::NetworkWatchman> {
    #[cfg(windows)]
    return Box::new(windows_impl::Win32Watchman);
    #[cfg(not(windows))]
    return Box::new(stub_impl::ProcFsWatchman);
}

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::ptr;
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_INSUFFICIENT_BUFFER, INVALID_HANDLE_VALUE, NO_ERROR,
    };
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID, MIB_TCPROW_OWNER_PID,
        MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    };
    use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
    };

    pub struct Win32Watchman;

    impl traits::NetworkWatchman for Win32Watchman {
        fn get_process_connections(&self, pid: u32) -> Result<Vec<ConnectionInfo>> {
            let mut connections = Vec::new();
            let family = Self::get_process_family(pid);

            // 1. Get IPv4 TCP Table (Once for all)
            let ipv4_rows = unsafe { Self::get_tcp_table_v4() }.unwrap_or_default();
            // 2. Get IPv6 TCP Table
            let ipv6_rows = unsafe { Self::get_tcp_table_v6() }.unwrap_or_default();

            for member_pid in family {
                // IPv4
                for row in &ipv4_rows {
                    if row.dwOwningPid == member_pid {
                        connections.push(ConnectionInfo {
                            local_ip: Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string(),
                            local_port: u16::from_be(row.dwLocalPort as u16),
                            remote_ip: Ipv4Addr::from(u32::from_be(row.dwRemoteAddr)).to_string(),
                            remote_port: u16::from_be(row.dwRemotePort as u16),
                            pid: member_pid,
                            process_name: "win32".to_string(),
                        });
                    }
                }
                // IPv6
                for row in &ipv6_rows {
                    if row.dwOwningPid == member_pid {
                        connections.push(ConnectionInfo {
                            local_ip: Ipv6Addr::from(row.ucLocalAddr).to_string(),
                            local_port: u16::from_be(row.dwLocalPort as u16),
                            remote_ip: Ipv6Addr::from(row.ucRemoteAddr).to_string(),
                            remote_port: u16::from_be(row.dwRemotePort as u16),
                            pid: member_pid,
                            process_name: "win32".to_string(),
                        });
                    }
                }
            }

            Ok(connections)
        }
    }

    impl Win32Watchman {
        // ... helper methods from original SocketScraper ...
        unsafe fn get_tcp_table_v4() -> Result<Vec<MIB_TCPROW_OWNER_PID>, u32> {
            let mut size = 0;
            let mut result = GetExtendedTcpTable(
                ptr::null_mut(),
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if result != ERROR_INSUFFICIENT_BUFFER {
                return Err(result);
            }
            let mut buffer = vec![0u8; size as usize];
            result = GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if result != NO_ERROR {
                return Err(result);
            }
            let table = &*(buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID);
            let rows =
                std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
            Ok(rows.to_vec())
        }

        unsafe fn get_tcp_table_v6() -> Result<Vec<MIB_TCP6ROW_OWNER_PID>, u32> {
            let mut size = 0;
            let mut result = GetExtendedTcpTable(
                ptr::null_mut(),
                &mut size,
                0,
                AF_INET6 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if result != ERROR_INSUFFICIENT_BUFFER {
                return Err(result);
            }
            let mut buffer = vec![0u8; size as usize];
            result = GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut _,
                &mut size,
                0,
                AF_INET6 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            );
            if result != NO_ERROR {
                return Err(result);
            }
            let table = &*(buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID);
            let rows =
                std::slice::from_raw_parts(table.table.as_ptr(), table.dwNumEntries as usize);
            Ok(rows.to_vec())
        }

        unsafe fn snapshot_processes() -> Result<Vec<(u32, u32)>, u32> {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return Err(1);
            }
            let mut processes = Vec::new();
            let mut entry: PROCESSENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;
            if Process32First(snapshot, &mut entry) != 0 {
                loop {
                    processes.push((entry.th32ProcessID, entry.th32ParentProcessID));
                    if Process32Next(snapshot, &mut entry) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
            Ok(processes)
        }

        pub fn get_process_family(root_pid: u32) -> Vec<u32> {
            let mut family = vec![root_pid];
            let mut children = vec![root_pid];
            while !children.is_empty() {
                let mut next_generation = Vec::new();
                if let Ok(all_processes) = unsafe { Self::snapshot_processes() } {
                    for parent_pid in &children {
                        for (pid, p_parent_pid) in &all_processes {
                            if p_parent_pid == parent_pid && !family.contains(pid) {
                                family.push(*pid);
                                next_generation.push(*pid);
                            }
                        }
                    }
                }
                children = next_generation;
                if family.len() > 100 {
                    break;
                }
            }
            family
        }
    }
}

#[cfg(not(windows))]
mod stub_impl {
    use super::*;
    use std::fs;

    pub struct ProcFsWatchman;

    impl traits::NetworkWatchman for ProcFsWatchman {
        fn get_process_connections(&self, target_pid: u32) -> Result<Vec<ConnectionInfo>> {
            let mut connections = Vec::new();

            // 1. Get all active TCP connections from procfs
            let tcp_entries = self.parse_proc_net_tcp("/proc/net/tcp")?;
            let tcp6_entries = self
                .parse_proc_net_tcp("/proc/net/tcp6")
                .unwrap_or_default();

            // 2. Map Inodes to PIDs
            // For Linux, we often need to check /proc/[pid]/fd to find which process owns which socket inode.
            let family = self.get_process_family(target_pid);

            for pid in family {
                let pid_inodes = self.get_inodes_for_pid(pid).unwrap_or_default();

                for entry in tcp_entries.iter().chain(tcp6_entries.iter()) {
                    if pid_inodes.contains(&entry.inode) {
                        connections.push(ConnectionInfo {
                            local_ip: entry.local_ip.clone(),
                            local_port: entry.local_port,
                            remote_ip: entry.remote_ip.clone(),
                            remote_port: entry.remote_port,
                            pid,
                            process_name: "linux_proc".to_string(),
                        });
                    }
                }
            }

            Ok(connections)
        }
    }

    struct ProcNetEntry {
        local_ip: String,
        local_port: u16,
        remote_ip: String,
        remote_port: u16,
        inode: u64,
    }

    impl ProcFsWatchman {
        fn parse_proc_net_tcp(&self, path: &str) -> Result<Vec<ProcNetEntry>> {
            let content = match fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => return Ok(Vec::new()),
            };

            let mut entries = Vec::new();
            for line in content.lines().skip(1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 10 {
                    continue;
                }

                // Expected format: sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrmt   uid  timeout inode
                // local_address is HEX_IP:HEX_PORT
                let local = self.parse_hex_addr(parts[1])?;
                let remote = self.parse_hex_addr(parts[2])?;
                let inode = parts[9].parse::<u64>().unwrap_or(0);

                entries.push(ProcNetEntry {
                    local_ip: local.0,
                    local_port: local.1,
                    remote_ip: remote.0,
                    remote_port: remote.1,
                    inode,
                });
            }
            Ok(entries)
        }

        fn parse_hex_addr(&self, addr: &str) -> Result<(String, u16)> {
            let mut split = addr.split(':');
            let hex_ip = split.next().ok_or_else(|| anyhow!("Invalid addr format"))?;
            let hex_port = split.next().ok_or_else(|| anyhow!("Invalid addr format"))?;

            let port = u16::from_str_radix(hex_port, 16)?;

            // IP is in little-endian hex (for IPv4)
            if hex_ip.len() == 8 {
                let ip_val = u32::from_str_radix(hex_ip, 16)?;
                let ip = std::net::Ipv4Addr::from(u32::from_be(ip_val));
                Ok((ip.to_string(), port))
            } else if hex_ip.len() == 32 {
                // IPv6 from /proc/net/tcp6 (4 words of 32 bits, host-endian)
                let mut bytes = [0u8; 16];
                for i in 0..4 {
                    let chunk = &hex_ip[i * 8..(i + 1) * 8];
                    let word = u32::from_str_radix(chunk, 16)?;
                    bytes[i * 4..(i + 1) * 4].copy_from_slice(&word.to_ne_bytes());
                }
                let ip = std::net::Ipv6Addr::from(bytes);
                Ok((ip.to_string(), port))
            } else {
                Err(anyhow!("Unsupported hex IP format: {}", hex_ip))
            }
        }

        fn get_inodes_for_pid(&self, pid: u32) -> Result<Vec<u64>> {
            let mut inodes = Vec::new();
            let fd_path = format!("/proc/{}/fd", pid);
            if let Ok(entries) = fs::read_dir(fd_path) {
                for entry in entries.flatten() {
                    if let Ok(link) = fs::read_link(entry.path()) {
                        let link_str = link.to_string_lossy();
                        if link_str.starts_with("socket:[") {
                            let inode_str = &link_str[8..link_str.len() - 1];
                            if let Ok(inode) = inode_str.parse::<u64>() {
                                inodes.push(inode);
                            }
                        }
                    }
                }
            }
            Ok(inodes)
        }

        fn get_process_family(&self, root_pid: u32) -> Vec<u32> {
            let mut family = vec![root_pid];
            let mut queue = vec![root_pid];

            while let Some(pid) = queue.pop() {
                // Linux 3.5+ facilitates this via /proc/[pid]/task/[tid]/children
                // We'll check the main thread's children first
                let children_path = format!("/proc/{}/task/{}/children", pid, pid);
                if let Ok(content) = fs::read_to_string(children_path) {
                    for child_str in content.split_whitespace() {
                        if let Ok(child_pid) = child_str.parse::<u32>() {
                            if !family.contains(&child_pid) {
                                family.push(child_pid);
                                queue.push(child_pid);
                            }
                        }
                    }
                }
            }
            family
        }
    }
}

// -------------------------------------------------------------
// Public Platform-Agnostic Watchman (Background Task Manager)
// -------------------------------------------------------------

pub struct NetworkWatchman {
    pid: u32,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl NetworkWatchman {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn start(
        &self,
    ) -> tokio::task::JoinHandle<Vec<crate::runtime::network::NetworkConnection>> {
        let pid = self.pid;
        let stop = self.stop.clone();

        // We need to move the scraper into the task, but wait, scraper is Box<dyn ...> which is Send...
        // But `self.scraper` is owned by `self`.
        // To move scraper into async task, we need to Clone it? Trait object doesn't imply Clone.
        // OR we can create a NEW scraper inside the task? `create_network_scraper()` is cheap.
        // Let's create it inside the task.

        let unique_conns = Arc::new(Mutex::new(std::collections::HashMap::new()));
        let unique_clone = unique_conns.clone();

        tokio::spawn(async move {
            let scraper = create_network_scraper(); // Create fresh scraper in task

            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                if let Ok(current) = scraper.get_process_connections(pid) {
                    if !current.is_empty() {
                        let mut conns = unique_clone.lock().await;
                        for conn in current {
                            // Map ConnectionInfo (trait) to NetworkConnection (legacy struct)
                            // Or update legacy logic to use ConnectionInfo?
                            // Assuming NetworkConnection struct is needed for backward compat.
                            let legacy_conn = NetworkConnection {
                                local_addr: format!("{}:{}", conn.local_ip, conn.local_port)
                                    .parse()
                                    .unwrap_or("0.0.0.0:0".parse().unwrap()),
                                remote_addr: format!("{}:{}", conn.remote_ip, conn.remote_port)
                                    .parse()
                                    .unwrap_or("0.0.0.0:0".parse().unwrap()),
                                protocol: "TCP".to_string(),
                                state: "ESTAB".to_string(), // Simplified state
                            };

                            let key =
                                format!("{}-{}", legacy_conn.remote_addr, legacy_conn.protocol);
                            conns.entry(key).or_insert(legacy_conn);
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            let conns = unique_clone.lock().await;
            conns.values().cloned().collect()
        })
    }

    pub fn stop(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// Legacy struct for compatibility
#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkConnection {
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub protocol: String,
    pub state: String,
}
