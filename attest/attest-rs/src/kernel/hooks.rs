pub struct ConnectContext {
    pub ip: String,
    pub port: u16,
}

// Eventually, we would map this to the C-repr expected by eBPF
#[repr(C)]
pub struct BpfSockAddr {
    pub user_family: u32,
    pub user_ip4: u32,
    pub user_ip6: [u32; 4],
    pub user_port: u32,
    // ... other fields matching uapi/linux/bpf.h
}
