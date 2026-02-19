//! eBPF tracing backend
//!
//! Provides kernel-level tracing using eBPF programs for syscall and network monitoring.

/// eBPF tracer for system-level tracing
#[derive(Debug, Clone)]
pub struct eBPFTracer;

/// Syscall trace event
#[derive(Debug, Clone)]
pub struct SyscallTrace {
    /// Syscall number
    pub syscall_id: u32,
    /// Syscall name
    pub syscall_name: String,
    /// Return value
    pub return_value: i64,
    /// Duration in microseconds
    pub duration_us: u64,
}

/// Network trace event
#[derive(Debug, Clone)]
pub struct NetworkTrace {
    /// Source IP
    pub source_ip: String,
    /// Destination IP
    pub dest_ip: String,
    /// Source port
    pub source_port: u16,
    /// Destination port
    pub dest_port: u16,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
}

/// Convert syscall ID to name
pub fn syscall_id_to_name(id: u32) -> String {
    match id {
        0 => "read".to_string(),
        1 => "write".to_string(),
        2 => "open".to_string(),
        3 => "close".to_string(),
        _ => format!("syscall_{}", id),
    }
}

/// Parse IPv4 address string
pub fn parse_ipv4(ip_str: &str) -> Option<u32> {
    let parts: Vec<&str> = ip_str.split('.').collect();
    if parts.len() != 4 {
        return None;
    }
    let mut ip = 0u32;
    for part in parts {
        if let Ok(byte) = part.parse::<u32>() {
            if byte > 255 {
                return None;
            }
            ip = (ip << 8) | byte;
        } else {
            return None;
        }
    }
    Some(ip)
}

/// Format IPv4 address from u32
pub fn format_ipv4(ip: u32) -> String {
    format!("{}.{}.{}.{}", (ip >> 24) & 0xFF, (ip >> 16) & 0xFF, (ip >> 8) & 0xFF, ip & 0xFF)
}
