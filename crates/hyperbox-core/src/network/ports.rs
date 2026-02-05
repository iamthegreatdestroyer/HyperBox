//! Port allocation and management.

use crate::error::{CoreError, Result};
use dashmap::DashSet;
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicU16, Ordering};
use tracing::debug;

/// Minimum ephemeral port.
pub const MIN_PORT: u16 = 32768;
/// Maximum ephemeral port.
pub const MAX_PORT: u16 = 60999;

/// Port allocator for container port mappings.
///
/// Provides automatic port allocation with conflict detection
/// and project-based port reservation.
pub struct PortAllocator {
    /// Allocated ports
    allocated: DashSet<u16>,
    /// Project port reservations (project_id -> ports)
    project_ports: dashmap::DashMap<String, Vec<u16>>,
    /// Next port to try
    next_port: AtomicU16,
}

impl PortAllocator {
    /// Create a new port allocator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            allocated: DashSet::new(),
            project_ports: dashmap::DashMap::new(),
            next_port: AtomicU16::new(MIN_PORT),
        }
    }

    /// Allocate a port.
    ///
    /// If `preferred` is Some and available, returns that port.
    /// Otherwise allocates the next available port.
    pub fn allocate(&self, preferred: Option<u16>) -> Result<u16> {
        // Try preferred port first
        if let Some(port) = preferred {
            if self.try_allocate(port)? {
                return Ok(port);
            }
        }

        // Find next available port
        let start = self.next_port.load(Ordering::Relaxed);
        let mut port = start;

        loop {
            if self.try_allocate(port)? {
                self.next_port.store(port + 1, Ordering::Relaxed);
                return Ok(port);
            }

            port = if port >= MAX_PORT { MIN_PORT } else { port + 1 };

            if port == start {
                return Err(CoreError::PortAllocationFailed {
                    port: preferred.unwrap_or(0),
                });
            }
        }
    }

    /// Try to allocate a specific port.
    fn try_allocate(&self, port: u16) -> Result<bool> {
        // Check if already allocated by us
        if self.allocated.contains(&port) {
            return Ok(false);
        }

        // Check if port is actually available on the system
        if !self.is_port_available(port) {
            return Ok(false);
        }

        // Allocate it
        self.allocated.insert(port);
        debug!("Allocated port {}", port);
        Ok(true)
    }

    /// Check if a port is available on the system.
    fn is_port_available(&self, port: u16) -> bool {
        // Try to bind to check availability
        let addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
        TcpListener::bind(addr).is_ok()
    }

    /// Release a port.
    pub fn release(&self, port: u16) {
        self.allocated.remove(&port);
        debug!("Released port {}", port);
    }

    /// Allocate a range of ports for a project.
    pub fn allocate_for_project(&self, project_id: &str, count: usize) -> Result<Vec<u16>> {
        let mut ports = Vec::with_capacity(count);

        for _ in 0..count {
            let port = self.allocate(None)?;
            ports.push(port);
        }

        self.project_ports
            .insert(project_id.to_string(), ports.clone());
        Ok(ports)
    }

    /// Get ports allocated to a project.
    pub fn get_project_ports(&self, project_id: &str) -> Vec<u16> {
        self.project_ports
            .get(project_id)
            .map(|p| p.value().clone())
            .unwrap_or_default()
    }

    /// Release all ports for a project.
    pub fn release_project(&self, project_id: &str) {
        if let Some((_, ports)) = self.project_ports.remove(project_id) {
            for port in ports {
                self.release(port);
            }
        }
    }

    /// Check if a port is allocated.
    #[must_use]
    pub fn is_allocated(&self, port: u16) -> bool {
        self.allocated.contains(&port)
    }

    /// Get count of allocated ports.
    #[must_use]
    pub fn allocated_count(&self) -> usize {
        self.allocated.len()
    }

    /// List all allocated ports.
    #[must_use]
    pub fn list_allocated(&self) -> Vec<u16> {
        self.allocated.iter().map(|p| *p).collect()
    }
}

impl Default for PortAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_allocator() {
        let allocator = PortAllocator::new();

        let port1 = allocator.allocate(None).unwrap();
        assert!(port1 >= MIN_PORT && port1 <= MAX_PORT);
        assert!(allocator.is_allocated(port1));

        let port2 = allocator.allocate(None).unwrap();
        assert_ne!(port1, port2);

        allocator.release(port1);
        assert!(!allocator.is_allocated(port1));
    }

    #[test]
    fn test_preferred_port() {
        let allocator = PortAllocator::new();

        // Allocate preferred port if available
        let port = allocator.allocate(Some(45678)).unwrap();
        assert_eq!(port, 45678);
    }
}
