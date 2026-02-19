//! eBPF-based automatic system tracing.
//!
//! Provides zero-instrumentation tracing of:
//! - Syscalls (entry/exit, duration, return value)
//! - Network I/O (src/dst IP, port, bytes)
//! - Process creation/termination
//!
//! Requirements: Linux kernel 5.1+ with CONFIG_BPF
//! Automatic fallback: On unsupported kernels, graceful disable
//!
//! Performance Impact: <2% CPU overhead
//! Collection: Rolling 10-minute window

pub mod ebpf;

pub use ebpf::{
    eBPFTracer, NetworkTrace, SyscallTrace, format_ipv4, parse_ipv4, syscall_id_to_name,
};
