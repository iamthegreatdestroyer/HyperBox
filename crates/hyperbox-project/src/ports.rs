//! Project port allocation and management.
//!
//! Provides automatic port allocation per project to avoid conflicts.

use crate::ProjectId;
use crate::error::{ProjectError, Result};
use dashmap::{DashMap, DashSet};
use std::net::TcpListener;
use std::sync::atomic::{AtomicU16, Ordering};
use tracing::{debug, warn};

/// Minimum ephemeral port.
const MIN_PORT: u16 = 32768;
/// Maximum ephemeral port.
const MAX_PORT: u16 = 60999;
/// Default number of ports per project.
const PORTS_PER_PROJECT: usize = 100;

/// Project port manager.
pub struct ProjectPortManager {
    /// Allocated ports by project
    project_ports: DashMap<ProjectId, Vec<u16>>,
    /// All allocated ports
    allocated: DashSet<u16>,
    /// Next port to try
    next_port: AtomicU16,
    /// Port ranges reserved per project
    reserved_ranges: DashMap<ProjectId, (u16, u16)>,
}

impl ProjectPortManager {
    /// Create a new port manager.
    pub fn new() -> Self {
        Self {
            project_ports: DashMap::new(),
            allocated: DashSet::new(),
            next_port: AtomicU16::new(MIN_PORT),
            reserved_ranges: DashMap::new(),
        }
    }

    /// Allocate a port for a project.
    pub fn allocate(&self, project_id: ProjectId, preferred: Option<u16>) -> Result<u16> {
        // Try preferred port first
        if let Some(port) = preferred {
            if self.try_allocate_port(port) {
                self.project_ports.entry(project_id).or_default().push(port);
                return Ok(port);
            }
        }

        // Try to find within project's reserved range
        if let Some(range) = self.reserved_ranges.get(&project_id) {
            for port in range.0..=range.1 {
                if self.try_allocate_port(port) {
                    self.project_ports.entry(project_id).or_default().push(port);
                    return Ok(port);
                }
            }
        }

        // Find any available port
        let port = self.find_available_port()?;
        self.project_ports.entry(project_id).or_default().push(port);

        Ok(port)
    }

    /// Try to allocate a specific port.
    fn try_allocate_port(&self, port: u16) -> bool {
        if self.allocated.contains(&port) {
            return false;
        }

        // Check if port is actually available on the system
        if !Self::is_port_available(port) {
            return false;
        }

        self.allocated.insert(port);
        debug!("Allocated port {}", port);
        true
    }

    /// Check if a port is available on the system.
    fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    /// Find an available port.
    fn find_available_port(&self) -> Result<u16> {
        let start = self.next_port.load(Ordering::Relaxed);
        let mut port = start;

        loop {
            if self.try_allocate_port(port) {
                self.next_port
                    .store(if port >= MAX_PORT { MIN_PORT } else { port + 1 }, Ordering::Relaxed);
                return Ok(port);
            }

            port = if port >= MAX_PORT { MIN_PORT } else { port + 1 };

            if port == start {
                return Err(ProjectError::PortAllocation {
                    reason: "No available ports".to_string(),
                });
            }
        }
    }

    /// Reserve a range of ports for a project.
    pub fn reserve_range(&self, project_id: ProjectId) -> Result<(u16, u16)> {
        if self.reserved_ranges.contains_key(&project_id) {
            return Ok(*self.reserved_ranges.get(&project_id).unwrap().value());
        }

        let start = self
            .next_port
            .fetch_add(PORTS_PER_PROJECT as u16, Ordering::Relaxed);

        let end = start + PORTS_PER_PROJECT as u16 - 1;
        let range = (start.min(MAX_PORT), end.min(MAX_PORT));

        self.reserved_ranges.insert(project_id, range);
        debug!("Reserved port range {:?} for project {}", range, project_id);

        Ok(range)
    }

    /// Release a port.
    pub fn release(&self, port: u16) {
        self.allocated.remove(&port);
        debug!("Released port {}", port);
    }

    /// Release all ports for a project.
    pub fn release_project(&self, project_id: ProjectId) {
        if let Some((_, ports)) = self.project_ports.remove(&project_id) {
            for port in ports {
                self.allocated.remove(&port);
            }
        }
        self.reserved_ranges.remove(&project_id);
        debug!("Released all ports for project {}", project_id);
    }

    /// Get all ports for a project.
    pub fn get_project_ports(&self, project_id: ProjectId) -> Vec<u16> {
        self.project_ports
            .get(&project_id)
            .map(|p| p.value().clone())
            .unwrap_or_default()
    }

    /// Get total allocated ports count.
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }

    /// Check if a port is allocated.
    pub fn is_allocated(&self, port: u16) -> bool {
        self.allocated.contains(&port)
    }
}

impl Default for ProjectPortManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_port_allocation() {
        let manager = ProjectPortManager::new();
        let project_id = Uuid::new_v4();

        let port = manager.allocate(project_id, None).unwrap();
        assert!(port >= MIN_PORT && port <= MAX_PORT);
        assert!(manager.is_allocated(port));
    }

    #[test]
    fn test_preferred_port() {
        let manager = ProjectPortManager::new();
        let project_id = Uuid::new_v4();

        // Find an available port for testing
        let preferred = 45678;
        if ProjectPortManager::is_port_available(preferred) {
            let port = manager.allocate(project_id, Some(preferred)).unwrap();
            assert_eq!(port, preferred);
        }
    }

    #[test]
    fn test_release_port() {
        let manager = ProjectPortManager::new();
        let project_id = Uuid::new_v4();

        let port = manager.allocate(project_id, None).unwrap();
        assert!(manager.is_allocated(port));

        manager.release(port);
        assert!(!manager.is_allocated(port));
    }
}
