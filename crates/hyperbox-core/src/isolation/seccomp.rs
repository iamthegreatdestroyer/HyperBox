//! Seccomp profile management.

use serde::{Deserialize, Serialize};

/// Seccomp action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeccompAction {
    /// Allow the syscall
    Allow,
    /// Return errno
    Errno,
    /// Kill the thread
    Kill,
    /// Kill the process
    KillProcess,
    /// Send SIGSYS
    Trap,
    /// Log the syscall
    Log,
    /// Trace the syscall
    Trace,
}

/// Seccomp operator for argument comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SeccompOperator {
    /// Not equal
    NotEqual,
    /// Less than
    LessThan,
    /// Less than or equal
    LessOrEqual,
    /// Equal to
    EqualTo,
    /// Greater than or equal
    GreaterOrEqual,
    /// Greater than
    GreaterThan,
    /// Masked equal
    MaskedEqual,
}

/// Seccomp argument filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompArg {
    /// Argument index (0-5)
    pub index: u32,
    /// Value to compare
    pub value: u64,
    /// Second value for masked comparison
    pub value_two: Option<u64>,
    /// Comparison operator
    pub op: SeccompOperator,
}

/// Seccomp syscall rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompSyscall {
    /// Syscall names
    pub names: Vec<String>,
    /// Action to take
    pub action: SeccompAction,
    /// Errno to return (if action is Errno)
    pub errno_ret: Option<u32>,
    /// Argument filters
    pub args: Vec<SeccompArg>,
}

/// Seccomp profile for container security.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeccompProfile {
    /// Default action for unmatched syscalls
    pub default_action: SeccompAction,
    /// Architecture filters
    pub architectures: Vec<String>,
    /// Syscall rules
    pub syscalls: Vec<SeccompSyscall>,
}

impl SeccompProfile {
    /// Create a default restrictive profile.
    ///
    /// This profile blocks dangerous syscalls while allowing common operations.
    #[must_use]
    pub fn default_profile() -> Self {
        Self {
            default_action: SeccompAction::Errno,
            architectures: vec![
                "SCMP_ARCH_X86_64".to_string(),
                "SCMP_ARCH_X86".to_string(),
                "SCMP_ARCH_AARCH64".to_string(),
            ],
            syscalls: vec![
                // Allow common syscalls
                SeccompSyscall {
                    names: Self::allowed_syscalls(),
                    action: SeccompAction::Allow,
                    errno_ret: None,
                    args: Vec::new(),
                },
            ],
        }
    }

    /// Create an unconfined profile (allow all).
    #[must_use]
    pub fn unconfined() -> Self {
        Self {
            default_action: SeccompAction::Allow,
            architectures: Vec::new(),
            syscalls: Vec::new(),
        }
    }

    /// List of commonly allowed syscalls for containers.
    fn allowed_syscalls() -> Vec<String> {
        vec![
            // Process management
            "execve", "execveat", "clone", "clone3", "fork", "vfork",
            "exit", "exit_group", "wait4", "waitid",
            // File operations
            "read", "write", "open", "openat", "openat2", "close",
            "stat", "fstat", "lstat", "newfstatat", "statx",
            "lseek", "pread64", "pwrite64", "readv", "writev",
            "access", "faccessat", "faccessat2",
            "dup", "dup2", "dup3", "fcntl",
            "flock", "fsync", "fdatasync",
            "truncate", "ftruncate",
            "getdents", "getdents64",
            "getcwd", "chdir", "fchdir",
            "rename", "renameat", "renameat2",
            "mkdir", "mkdirat", "rmdir",
            "link", "linkat", "symlink", "symlinkat",
            "unlink", "unlinkat",
            "readlink", "readlinkat",
            "chmod", "fchmod", "fchmodat",
            "chown", "fchown", "lchown", "fchownat",
            "umask",
            // Memory
            "brk", "mmap", "munmap", "mprotect", "mremap",
            "madvise", "mlock", "munlock",
            // Signals
            "rt_sigaction", "rt_sigprocmask", "rt_sigreturn",
            "rt_sigsuspend", "sigaltstack",
            // Time
            "nanosleep", "clock_nanosleep", "clock_gettime",
            "clock_getres", "gettimeofday", "time",
            // IPC
            "pipe", "pipe2", "socket", "socketpair",
            "connect", "accept", "accept4",
            "bind", "listen", "shutdown",
            "sendto", "recvfrom", "sendmsg", "recvmsg",
            "setsockopt", "getsockopt",
            "getpeername", "getsockname",
            "poll", "ppoll", "select", "pselect6",
            "epoll_create", "epoll_create1", "epoll_ctl", "epoll_wait", "epoll_pwait",
            // User/Group
            "getuid", "geteuid", "getgid", "getegid",
            "getgroups", "setgroups",
            "getpid", "getppid", "gettid", "getpgid", "getpgrp",
            "setsid", "setpgid",
            // Misc
            "uname", "sysinfo", "getrlimit", "setrlimit", "prlimit64",
            "getrusage", "times",
            "futex", "set_robust_list", "get_robust_list",
            "arch_prctl", "prctl",
            "set_tid_address",
            "ioctl",
            "eventfd", "eventfd2",
            "timerfd_create", "timerfd_settime", "timerfd_gettime",
            "signalfd", "signalfd4",
            "getrandom",
        ]
        .into_iter()
        .map(String::from)
        .collect()
    }

    /// Add a syscall rule.
    pub fn add_rule(&mut self, syscall: SeccompSyscall) {
        self.syscalls.push(syscall);
    }

    /// Allow a specific syscall.
    pub fn allow(&mut self, name: impl Into<String>) {
        self.syscalls.push(SeccompSyscall {
            names: vec![name.into()],
            action: SeccompAction::Allow,
            errno_ret: None,
            args: Vec::new(),
        });
    }

    /// Block a specific syscall.
    pub fn block(&mut self, name: impl Into<String>) {
        self.syscalls.push(SeccompSyscall {
            names: vec![name.into()],
            action: SeccompAction::Errno,
            errno_ret: Some(1), // EPERM
            args: Vec::new(),
        });
    }

    /// Convert to OCI-compatible JSON.
    pub fn to_oci_json(&self) -> serde_json::Value {
        serde_json::json!({
            "defaultAction": format!("SCMP_ACT_{:?}", self.default_action).to_uppercase(),
            "architectures": self.architectures,
            "syscalls": self.syscalls.iter().map(|s| {
                serde_json::json!({
                    "names": s.names,
                    "action": format!("SCMP_ACT_{:?}", s.action).to_uppercase(),
                    "args": s.args.iter().map(|a| {
                        serde_json::json!({
                            "index": a.index,
                            "value": a.value,
                            "valueTwo": a.value_two.unwrap_or(0),
                            "op": format!("SCMP_CMP_{:?}", a.op).to_uppercase()
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        })
    }
}

impl Default for SeccompProfile {
    fn default() -> Self {
        Self::default_profile()
    }
}
