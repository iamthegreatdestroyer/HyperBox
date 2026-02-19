//! Profile export to multiple formats
//!
//! Exports profiling data to JSON, libseccomp, and eBPF bytecode formats
//! for consumption by security policies, container runtimes, and eBPF programs.

use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};

/// Profile format for export
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileFormat {
    /// JSON format for human readability and tooling
    JSON,
    /// libseccomp filter format
    Libseccomp,
    /// eBPF bytecode format
    EBPFBytecode,
}

impl ProfileFormat {
    /// Get format name
    pub fn name(&self) -> &'static str {
        match self {
            ProfileFormat::JSON => "json",
            ProfileFormat::Libseccomp => "libseccomp",
            ProfileFormat::EBPFBytecode => "ebpf_bytecode",
        }
    }
}

/// Syscall action in security policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyscallAction {
    /// Allow syscall without restrictions
    Allow,
    /// Log the syscall
    Log,
    /// Deny the syscall
    Deny,
    /// Kill the process
    Kill,
}

impl SyscallAction {
    /// Get action name
    pub fn name(&self) -> &'static str {
        match self {
            SyscallAction::Allow => "allow",
            SyscallAction::Log => "log",
            SyscallAction::Deny => "deny",
            SyscallAction::Kill => "kill",
        }
    }
}

/// CPU architecture for profile
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Architecture {
    /// x86_64 architecture
    X86_64,
    /// ARM64 architecture
    Arm64,
    /// ARM (32-bit) architecture
    Arm,
}

impl Architecture {
    /// Get architecture name
    pub fn name(&self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::Arm64 => "arm64",
            Architecture::Arm => "arm",
        }
    }
}

/// Syscall profile entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallProfile {
    /// Syscall name
    pub name: String,
    /// Syscall number(s) per architecture
    pub numbers: HashMap<String, u32>,
    /// Action for this syscall
    pub action: SyscallAction,
    /// Optional whitelist of argument values
    pub arg_whitelist: Option<Vec<Vec<u64>>>,
}

impl SyscallProfile {
    /// Create new syscall profile
    pub fn new(name: String, action: SyscallAction) -> Self {
        Self {
            name,
            numbers: HashMap::new(),
            action,
            arg_whitelist: None,
        }
    }

    /// Add architecture-specific syscall number
    pub fn add_number(&mut self, arch: Architecture, number: u32) {
        self.numbers.insert(arch.name().to_string(), number);
    }

    /// Set argument whitelist
    pub fn set_arg_whitelist(&mut self, whitelist: Vec<Vec<u64>>) {
        self.arg_whitelist = Some(whitelist);
    }
}

/// Security profile for a container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityProfile {
    /// Profile name
    pub name: String,
    /// Profile version
    pub version: String,
    /// Description
    pub description: String,
    /// Default action for unlisted syscalls
    pub default_action: SyscallAction,
    /// Syscall profiles
    pub syscalls: Vec<SyscallProfile>,
    /// Supported architectures
    pub architectures: HashSet<String>,
}

impl SecurityProfile {
    /// Create new security profile
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            description: String::new(),
            default_action: SyscallAction::Deny,
            syscalls: vec![],
            architectures: HashSet::new(),
        }
    }

    /// Add syscall to profile
    pub fn add_syscall(&mut self, profile: SyscallProfile) {
        self.syscalls.push(profile);
    }

    /// Add supported architecture
    pub fn add_architecture(&mut self, arch: Architecture) {
        self.architectures.insert(arch.name().to_string());
    }

    /// Get syscall by name
    pub fn get_syscall(&self, name: &str) -> Option<&SyscallProfile> {
        self.syscalls.iter().find(|s| s.name == name)
    }

    /// Get allowed syscalls
    pub fn allowed_syscalls(&self) -> Vec<&SyscallProfile> {
        self.syscalls
            .iter()
            .filter(|s| s.action == SyscallAction::Allow)
            .collect()
    }

    /// Get denied syscalls
    pub fn denied_syscalls(&self) -> Vec<&SyscallProfile> {
        self.syscalls
            .iter()
            .filter(|s| s.action == SyscallAction::Deny)
            .collect()
    }
}

/// Profile exporter to multiple formats
pub struct ProfileExporter;

impl ProfileExporter {
    /// Export profile to JSON
    pub fn export_json(profile: &SecurityProfile) -> Result<String> {
        serde_json::to_string_pretty(profile)
            .map_err(|e| anyhow!("Failed to serialize profile to JSON: {}", e))
    }

    /// Export profile to libseccomp filter rules
    pub fn export_libseccomp(profile: &SecurityProfile) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!("# libseccomp filter for: {}\n", profile.name));
        output.push_str(&format!("# Version: {}\n", profile.version));
        output.push_str(&format!("# {}\n\n", profile.description));

        // Header
        output.push_str("SCMP_ARCH_NATIVE DEFAULT SCMP_ACT_DENY\n\n");

        // Add all architectures
        for arch in &profile.architectures {
            match arch.as_str() {
                "x86_64" => output.push_str("SCMP_ARCH_X86_64\n"),
                "arm64" => output.push_str("SCMP_ARCH_ARM64\n"),
                "arm" => output.push_str("SCMP_ARCH_ARM\n"),
                _ => {}
            }
        }
        output.push('\n');

        // Add syscall rules
        for syscall in &profile.syscalls {
            let action = match syscall.action {
                SyscallAction::Allow => "SCMP_ACT_ALLOW",
                SyscallAction::Log => "SCMP_ACT_LOG",
                SyscallAction::Deny => "SCMP_ACT_ERRNO(EPERM)",
                SyscallAction::Kill => "SCMP_ACT_KILL",
            };

            // Use x86_64 syscall number for output
            if let Some(num) = syscall.numbers.get("x86_64") {
                if let Some(whitelist) = &syscall.arg_whitelist {
                    // With argument restrictions
                    for (arg_set_idx, args) in whitelist.iter().enumerate() {
                        if args.is_empty() {
                            output.push_str(&format!("{} {} {}\n", action, syscall.name, num));
                        } else {
                            output.push_str(&format!(
                                "# {} {} {} (args: {:?})\n",
                                action, syscall.name, num, args
                            ));
                        }
                    }
                } else {
                    // Without argument restrictions
                    output.push_str(&format!("{} {} {}\n", action, syscall.name, num));
                }
            }
        }

        Ok(output)
    }

    /// Export profile to eBPF bytecode format (pseudo-code representation)
    pub fn export_ebpf_bytecode(profile: &SecurityProfile) -> Result<String> {
        let mut output = String::new();
        output.push_str("// eBPF bytecode representation for filter\n");
        output.push_str(&format!("// Profile: {} (v{})\n", profile.name, profile.version));
        output.push_str(&format!("// {}\n\n", profile.description));

        // BPF filter structure
        output.push_str("struct filter_state {\n");
        output.push_str("    u32 syscall_id;\n");
        output.push_str("    u64 args[6];\n");
        output.push_str("    u32 action;\n");
        output.push_str("};\n\n");

        // Generate decision tree
        output.push_str("// Decision tree for syscall filtering\n");
        output.push_str("int filter_syscall(struct filter_state *ctx) {\n");
        output.push_str("    switch (ctx->syscall_id) {\n");

        for syscall in &profile.syscalls {
            if let Some(num) = syscall.numbers.get("x86_64") {
                let action_code = match syscall.action {
                    SyscallAction::Allow => "0", // ALLOW
                    SyscallAction::Log => "1",   // LOG
                    SyscallAction::Deny => "2",  // DENY
                    SyscallAction::Kill => "3",  // KILL
                };

                output.push_str(&format!("        case {}:  // {}\n", num, syscall.name));
                output.push_str(&format!("            return {};\n", action_code));
            }
        }

        // Default action
        let default_code = match profile.default_action {
            SyscallAction::Allow => "0",
            SyscallAction::Log => "1",
            SyscallAction::Deny => "2",
            SyscallAction::Kill => "3",
        };
        output.push_str(&format!("        default:\n"));
        output.push_str(&format!("            return {};\n", default_code));

        output.push_str("    }\n");
        output.push_str("}\n");

        Ok(output)
    }

    /// Export profile in multiple formats
    pub fn export_all(profile: &SecurityProfile) -> Result<HashMap<String, String>> {
        let mut exports = HashMap::new();

        exports.insert(
            ProfileFormat::JSON.name().to_string(),
            Self::export_json(profile)?,
        );
        exports.insert(
            ProfileFormat::Libseccomp.name().to_string(),
            Self::export_libseccomp(profile)?,
        );
        exports.insert(
            ProfileFormat::EBPFBytecode.name().to_string(),
            Self::export_ebpf_bytecode(profile)?,
        );

        Ok(exports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_format_names() {
        assert_eq!(ProfileFormat::JSON.name(), "json");
        assert_eq!(ProfileFormat::Libseccomp.name(), "libseccomp");
        assert_eq!(ProfileFormat::EBPFBytecode.name(), "ebpf_bytecode");
    }

    #[test]
    fn test_syscall_action_names() {
        assert_eq!(SyscallAction::Allow.name(), "allow");
        assert_eq!(SyscallAction::Log.name(), "log");
        assert_eq!(SyscallAction::Deny.name(), "deny");
        assert_eq!(SyscallAction::Kill.name(), "kill");
    }

    #[test]
    fn test_architecture_names() {
        assert_eq!(Architecture::X86_64.name(), "x86_64");
        assert_eq!(Architecture::Arm64.name(), "arm64");
        assert_eq!(Architecture::Arm.name(), "arm");
    }

    #[test]
    fn test_syscall_profile_creation() {
        let profile = SyscallProfile::new("open".to_string(), SyscallAction::Allow);
        assert_eq!(profile.name, "open");
        assert_eq!(profile.action, SyscallAction::Allow);
        assert!(profile.numbers.is_empty());
    }

    #[test]
    fn test_syscall_profile_add_number() {
        let mut profile = SyscallProfile::new("open".to_string(), SyscallAction::Allow);
        profile.add_number(Architecture::X86_64, 2);
        profile.add_number(Architecture::Arm64, 56);

        assert_eq!(profile.numbers.get("x86_64"), Some(&2));
        assert_eq!(profile.numbers.get("arm64"), Some(&56));
    }

    #[test]
    fn test_security_profile_creation() {
        let profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        assert_eq!(profile.name, "test");
        assert_eq!(profile.version, "1.0");
        assert_eq!(profile.default_action, SyscallAction::Deny);
        assert!(profile.syscalls.is_empty());
    }

    #[test]
    fn test_security_profile_add_syscall() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        let syscall = SyscallProfile::new("read".to_string(), SyscallAction::Allow);
        profile.add_syscall(syscall);

        assert_eq!(profile.syscalls.len(), 1);
        assert!(profile.get_syscall("read").is_some());
    }

    #[test]
    fn test_security_profile_add_architecture() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.add_architecture(Architecture::X86_64);
        profile.add_architecture(Architecture::Arm64);

        assert_eq!(profile.architectures.len(), 2);
        assert!(profile.architectures.contains("x86_64"));
    }

    #[test]
    fn test_allowed_syscalls() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.add_syscall(SyscallProfile::new("read".to_string(), SyscallAction::Allow));
        profile.add_syscall(SyscallProfile::new("write".to_string(), SyscallAction::Allow));
        profile.add_syscall(SyscallProfile::new("open".to_string(), SyscallAction::Deny));

        let allowed = profile.allowed_syscalls();
        assert_eq!(allowed.len(), 2);
    }

    #[test]
    fn test_denied_syscalls() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.add_syscall(SyscallProfile::new("read".to_string(), SyscallAction::Allow));
        profile.add_syscall(SyscallProfile::new("open".to_string(), SyscallAction::Deny));
        profile.add_syscall(SyscallProfile::new("chmod".to_string(), SyscallAction::Deny));

        let denied = profile.denied_syscalls();
        assert_eq!(denied.len(), 2);
    }

    #[test]
    fn test_export_json() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.add_syscall(SyscallProfile::new("read".to_string(), SyscallAction::Allow));

        let json = ProfileExporter::export_json(&profile).unwrap();
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"version\": \"1.0\""));
        assert!(json.contains("read"));
    }

    #[test]
    fn test_export_libseccomp() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.add_architecture(Architecture::X86_64);

        let mut syscall = SyscallProfile::new("read".to_string(), SyscallAction::Allow);
        syscall.add_number(Architecture::X86_64, 0);
        profile.add_syscall(syscall);

        let libseccomp = ProfileExporter::export_libseccomp(&profile).unwrap();
        assert!(libseccomp.contains("libseccomp filter"));
        assert!(libseccomp.contains("SCMP_ACT_ALLOW"));
        assert!(libseccomp.contains("read"));
    }

    #[test]
    fn test_export_ebpf_bytecode() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());

        let mut syscall = SyscallProfile::new("read".to_string(), SyscallAction::Allow);
        syscall.add_number(Architecture::X86_64, 0);
        profile.add_syscall(syscall);

        let ebpf = ProfileExporter::export_ebpf_bytecode(&profile).unwrap();
        assert!(ebpf.contains("eBPF bytecode"));
        assert!(ebpf.contains("filter_syscall"));
        assert!(ebpf.contains("read"));
    }

    #[test]
    fn test_export_all_formats() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.add_architecture(Architecture::X86_64);
        profile.add_syscall(SyscallProfile::new("read".to_string(), SyscallAction::Allow));

        let exports = ProfileExporter::export_all(&profile).unwrap();
        assert_eq!(exports.len(), 3);
        assert!(exports.contains_key("json"));
        assert!(exports.contains_key("libseccomp"));
        assert!(exports.contains_key("ebpf_bytecode"));
    }

    #[test]
    fn test_syscall_profile_arg_whitelist() {
        let mut profile = SyscallProfile::new("open".to_string(), SyscallAction::Allow);
        let whitelist = vec![vec![1, 2], vec![3, 4]];
        profile.set_arg_whitelist(whitelist);

        assert!(profile.arg_whitelist.is_some());
        assert_eq!(profile.arg_whitelist.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_security_profile_with_description() {
        let mut profile = SecurityProfile::new("test".to_string(), "1.0".to_string());
        profile.description = "Test security profile".to_string();

        assert_eq!(profile.description, "Test security profile");
    }
}
