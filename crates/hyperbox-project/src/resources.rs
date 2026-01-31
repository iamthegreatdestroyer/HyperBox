//! Resource pool management for projects.
//!
//! Manages shared resources (CPU, memory, disk) across all projects.

use crate::error::{ProjectError, Result};
use crate::ProjectId;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info};

/// Default CPU quota (in millicores, 1000 = 1 CPU).
const DEFAULT_CPU_QUOTA: u64 = 2000; // 2 CPUs
/// Default memory quota (in bytes).
const DEFAULT_MEMORY_QUOTA: u64 = 4 * 1024 * 1024 * 1024; // 4 GB
/// Default disk quota (in bytes).
const DEFAULT_DISK_QUOTA: u64 = 50 * 1024 * 1024 * 1024; // 50 GB

/// Resource pool for managing shared resources.
pub struct ResourcePool {
    /// Total CPU available (millicores).
    total_cpu: AtomicU64,
    /// Total memory available (bytes).
    total_memory: AtomicU64,
    /// Total disk available (bytes).
    total_disk: AtomicU64,
    /// Allocated CPU per project.
    cpu_allocations: DashMap<ProjectId, u64>,
    /// Allocated memory per project.
    memory_allocations: DashMap<ProjectId, u64>,
    /// Allocated disk per project.
    disk_allocations: DashMap<ProjectId, u64>,
}

/// Resource allocation for a project.
#[derive(Debug, Clone, Default)]
pub struct ResourceAllocation {
    /// CPU in millicores (1000 = 1 CPU).
    pub cpu: u64,
    /// Memory in bytes.
    pub memory: u64,
    /// Disk in bytes.
    pub disk: u64,
}

/// Resource usage statistics.
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// Total CPU available.
    pub total_cpu: u64,
    /// Used CPU.
    pub used_cpu: u64,
    /// Total memory available.
    pub total_memory: u64,
    /// Used memory.
    pub used_memory: u64,
    /// Total disk available.
    pub total_disk: u64,
    /// Used disk.
    pub used_disk: u64,
}

impl ResourcePool {
    /// Create a new resource pool with default quotas.
    pub fn new() -> Self {
        Self::with_quotas(DEFAULT_CPU_QUOTA, DEFAULT_MEMORY_QUOTA, DEFAULT_DISK_QUOTA)
    }

    /// Create a resource pool with custom quotas.
    pub fn with_quotas(cpu: u64, memory: u64, disk: u64) -> Self {
        Self {
            total_cpu: AtomicU64::new(cpu),
            total_memory: AtomicU64::new(memory),
            total_disk: AtomicU64::new(disk),
            cpu_allocations: DashMap::new(),
            memory_allocations: DashMap::new(),
            disk_allocations: DashMap::new(),
        }
    }

    /// Detect system resources and set quotas accordingly.
    pub fn detect_system_resources(&self) {
        let num_cpus = num_cpus::get() as u64;
        let cpu_millicores = num_cpus * 1000;

        // Leave some CPU for the host system
        let available_cpu = (cpu_millicores as f64 * 0.8) as u64;
        self.total_cpu.store(available_cpu, Ordering::Relaxed);

        // Detect system memory (if available)
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemTotal:") {
                        if let Some(kb) = line.split_whitespace().nth(1) {
                            if let Ok(kb_value) = kb.parse::<u64>() {
                                // Leave 20% for the host
                                let available = (kb_value * 1024 * 80) / 100;
                                self.total_memory.store(available, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }
        }

        info!(
            "Detected system resources: {} CPU millicores, {} MB memory",
            self.total_cpu.load(Ordering::Relaxed),
            self.total_memory.load(Ordering::Relaxed) / (1024 * 1024)
        );
    }

    /// Allocate resources for a project.
    pub fn allocate(
        &self,
        project_id: ProjectId,
        allocation: ResourceAllocation,
    ) -> Result<()> {
        // Check CPU availability
        let used_cpu = self.used_cpu();
        let total_cpu = self.total_cpu.load(Ordering::Relaxed);
        if used_cpu + allocation.cpu > total_cpu {
            return Err(ProjectError::ResourceLimitExceeded {
                resource: "CPU".to_string(),
                limit: total_cpu,
                requested: allocation.cpu,
            });
        }

        // Check memory availability
        let used_memory = self.used_memory();
        let total_memory = self.total_memory.load(Ordering::Relaxed);
        if used_memory + allocation.memory > total_memory {
            return Err(ProjectError::ResourceLimitExceeded {
                resource: "memory".to_string(),
                limit: total_memory,
                requested: allocation.memory,
            });
        }

        // Check disk availability
        let used_disk = self.used_disk();
        let total_disk = self.total_disk.load(Ordering::Relaxed);
        if used_disk + allocation.disk > total_disk {
            return Err(ProjectError::ResourceLimitExceeded {
                resource: "disk".to_string(),
                limit: total_disk,
                requested: allocation.disk,
            });
        }

        // Record allocations
        self.cpu_allocations.insert(project_id, allocation.cpu);
        self.memory_allocations.insert(project_id, allocation.memory);
        self.disk_allocations.insert(project_id, allocation.disk);

        debug!(
            "Allocated resources for project {}: {} millicores, {} MB, {} GB disk",
            project_id,
            allocation.cpu,
            allocation.memory / (1024 * 1024),
            allocation.disk / (1024 * 1024 * 1024)
        );

        Ok(())
    }

    /// Release resources for a project.
    pub fn release(&self, project_id: ProjectId) {
        self.cpu_allocations.remove(&project_id);
        self.memory_allocations.remove(&project_id);
        self.disk_allocations.remove(&project_id);
        debug!("Released resources for project {}", project_id);
    }

    /// Get allocation for a project.
    pub fn get_allocation(&self, project_id: ProjectId) -> ResourceAllocation {
        ResourceAllocation {
            cpu: self.cpu_allocations.get(&project_id).map(|v| *v).unwrap_or(0),
            memory: self.memory_allocations.get(&project_id).map(|v| *v).unwrap_or(0),
            disk: self.disk_allocations.get(&project_id).map(|v| *v).unwrap_or(0),
        }
    }

    /// Get used CPU (millicores).
    pub fn used_cpu(&self) -> u64 {
        self.cpu_allocations.iter().map(|r| *r.value()).sum()
    }

    /// Get used memory (bytes).
    pub fn used_memory(&self) -> u64 {
        self.memory_allocations.iter().map(|r| *r.value()).sum()
    }

    /// Get used disk (bytes).
    pub fn used_disk(&self) -> u64 {
        self.disk_allocations.iter().map(|r| *r.value()).sum()
    }

    /// Get available CPU (millicores).
    pub fn available_cpu(&self) -> u64 {
        self.total_cpu.load(Ordering::Relaxed).saturating_sub(self.used_cpu())
    }

    /// Get available memory (bytes).
    pub fn available_memory(&self) -> u64 {
        self.total_memory.load(Ordering::Relaxed).saturating_sub(self.used_memory())
    }

    /// Get available disk (bytes).
    pub fn available_disk(&self) -> u64 {
        self.total_disk.load(Ordering::Relaxed).saturating_sub(self.used_disk())
    }

    /// Get resource statistics.
    pub fn stats(&self) -> ResourceStats {
        ResourceStats {
            total_cpu: self.total_cpu.load(Ordering::Relaxed),
            used_cpu: self.used_cpu(),
            total_memory: self.total_memory.load(Ordering::Relaxed),
            used_memory: self.used_memory(),
            total_disk: self.total_disk.load(Ordering::Relaxed),
            used_disk: self.used_disk(),
        }
    }

    /// Set total CPU quota.
    pub fn set_cpu_quota(&self, millicores: u64) {
        self.total_cpu.store(millicores, Ordering::Relaxed);
    }

    /// Set total memory quota.
    pub fn set_memory_quota(&self, bytes: u64) {
        self.total_memory.store(bytes, Ordering::Relaxed);
    }

    /// Set total disk quota.
    pub fn set_disk_quota(&self, bytes: u64) {
        self.total_disk.store(bytes, Ordering::Relaxed);
    }
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self::new()
    }
}
