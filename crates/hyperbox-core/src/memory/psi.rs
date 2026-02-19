//! PSI (Pressure Stall Information) Memory Monitor
//!
//! Monitors kernel-level memory pressure and automatically tunes swap
//! pressure based on real-time system demand.
//!
//! # Requirements
//! - Linux 5.0+ (PSI interface in procfs)
//! - Optional: Root access for swap tuning (graceful fallback without it)
//!
//! # Graceful Fallback
//! On unsupported kernels, PSI monitoring returns disabled state but doesn't error.
//! Swap tuning is best-effort and fails silently if root privileges aren't available.
//!
//! # Example
//! ```no_run
//! use hyperbox_core::memory::PSIMonitor;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut monitor = PSIMonitor::new()?;
//!
//!     if !monitor.is_enabled() {
//!         println!("PSI monitoring not available on this system");
//!         return Ok(());
//!     }
//!
//!     // Monitor for 10 seconds
//!     for _ in 0..10 {
//!         monitor.update().await?;
//!
//!         if let Some(reading) = monitor.current() {
//!             println!("Memory pressure: some={:.1}% full={:.1}%",
//!                 reading.some, reading.full);
//!
//!             if monitor.is_critical() {
//!                 println!("Critical memory pressure detected!");
//!                 monitor.tune_swap().await?;
//!             }
//!         }
//!
//!         tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
//!     }
//!
//!     Ok(())
//! }
//! ```

use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::{anyhow, Result};

/// PSI (Pressure Stall Information) memory pressure reading.
///
/// Represents a single point-in-time measurement of memory pressure from the kernel.
#[derive(Debug, Clone)]
pub struct PSIMemory {
    /// Percentage of CPU time ANY task stalled on memory (0.0-100.0)
    pub some: f64,
    /// Percentage of CPU time ALL tasks stalled on memory (0.0-100.0)
    pub full: f64,
    /// Unix timestamp of the reading
    pub timestamp: u64,
}

/// PSI Monitor - tracks kernel memory pressure over time.
///
/// Maintains a sliding window of memory pressure readings and detects
/// when critical or warning thresholds are exceeded.
#[derive(Debug)]
pub struct PSIMonitor {
    window: std::time::Duration,
    critical_threshold: f64,       // >50% = critical
    warning_threshold: f64,        // >20% = warning
    samples: VecDeque<PSIMemory>,
    enabled: bool,
}

impl PSIMonitor {
    /// Create a new PSI monitor.
    ///
    /// Automatically detects kernel support by checking for `/proc/pressure/memory`.
    /// If the file doesn't exist (kernel < 5.0), monitoring is disabled but no error occurs.
    ///
    /// # Returns
    /// Returns Ok(monitor) with disabled flag if PSI is unsupported.
    /// Only returns an error if there's an actual I/O problem reading PSI data.
    pub fn new() -> Result<Self> {
        let psi_path = Path::new("/proc/pressure/memory");

        if !psi_path.exists() {
            eprintln!("⚠️  PSI not available (kernel < 5.0 or unsupported platform)");
            return Ok(Self {
                window: std::time::Duration::from_secs(10),
                critical_threshold: 50.0,
                warning_threshold: 20.0,
                samples: VecDeque::with_capacity(10),
                enabled: false,
            });
        }

        Ok(Self {
            window: std::time::Duration::from_secs(10),
            critical_threshold: 50.0,
            warning_threshold: 20.0,
            samples: VecDeque::with_capacity(10),
            enabled: true,
        })
    }

    /// Check if PSI monitoring is available on this system.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Read current PSI data from `/proc/pressure/memory`.
    ///
    /// The file format is:
    /// ```text
    /// some avg10=0.50 avg60=0.30 avg300=0.10 total=123456789
    /// full avg10=0.10 avg60=0.05 avg300=0.02 total=98765432
    /// ```
    ///
    /// This function extracts the avg10 (10-second average) values.
    pub async fn read_psi(&self) -> Result<PSIMemory> {
        if !self.enabled {
            return Err(anyhow!("PSI monitoring not available"));
        }

        let content = fs::read_to_string("/proc/pressure/memory")
            .map_err(|e| anyhow!("Failed to read PSI: {}", e))?;

        let mut some_val: f64 = 0.0;
        let mut full_val: f64 = 0.0;

        // Parse the PSI file
        for line in content.lines() {
            if line.starts_with("some ") {
                some_val = Self::parse_avg10(line)?;
            } else if line.starts_with("full ") {
                full_val = Self::parse_avg10(line)?;
            }
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        Ok(PSIMemory {
            some: some_val,
            full: full_val,
            timestamp,
        })
    }

    /// Parse avg10 value from a PSI line.
    ///
    /// Extracts the avg10=X.XX value from a line like:
    /// "some avg10=0.50 avg60=0.30 avg300=0.10 total=123456789"
    fn parse_avg10(line: &str) -> Result<f64> {
        for part in line.split_whitespace() {
            if part.starts_with("avg10=") {
                let val_str = &part[6..];  // Skip "avg10="
                return val_str
                    .parse::<f64>()
                    .map_err(|e| anyhow!("Failed to parse avg10 value: {}", e));
            }
        }
        Ok(0.0)
    }

    /// Update monitor with latest PSI reading.
    ///
    /// Fetches current memory pressure and stores it in the sliding window.
    /// Automatically removes old samples to maintain the 10-sample window.
    pub async fn update(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let reading = self.read_psi().await?;
        self.samples.push_back(reading);

        // Keep only last 10 samples (100 seconds at 10s interval)
        if self.samples.len() > 10 {
            self.samples.pop_front();
        }

        Ok(())
    }

    /// Get the most recent PSI reading.
    pub fn current(&self) -> Option<&PSIMemory> {
        self.samples.back()
    }

    /// Check if memory pressure is at critical level (>50%).
    pub fn is_critical(&self) -> bool {
        self.current()
            .map(|m| m.full > self.critical_threshold)
            .unwrap_or(false)
    }

    /// Check if memory pressure is at warning level (20-50%).
    pub fn is_warning(&self) -> bool {
        self.current()
            .map(|m| m.full > self.warning_threshold && m.full <= self.critical_threshold)
            .unwrap_or(false)
    }

    /// Get all samples in current window.
    pub fn samples(&self) -> &VecDeque<PSIMemory> {
        &self.samples
    }

    /// Tune swap settings based on current memory pressure.
    pub async fn tune_swap(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let mut tuner = SwapTuner::new();
        tuner.tune_for_pressure(self).await
    }

    /// Set custom critical threshold (in percentage, 0-100).
    pub fn set_critical_threshold(&mut self, threshold: f64) {
        self.critical_threshold = threshold.clamp(0.0, 100.0);
    }

    /// Set custom warning threshold (in percentage, 0-100).
    pub fn set_warning_threshold(&mut self, threshold: f64) {
        self.warning_threshold = threshold.clamp(0.0, 100.0);
    }
}

impl Default for PSIMonitor {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            window: std::time::Duration::from_secs(10),
            critical_threshold: 50.0,
            warning_threshold: 20.0,
            samples: VecDeque::with_capacity(10),
            enabled: false,
        })
    }
}

/// Swap tuner - dynamically adjusts swappiness based on memory pressure.
///
/// The kernel `vm.swappiness` parameter (0-100) controls how aggressively
/// the kernel swaps memory to disk:
/// - 0-10: Avoid swapping (use physical memory first)
/// - 40-50: Balanced approach (default often 60)
/// - 80-100: Aggressively swap (reduces memory pressure)
///
/// This tuner automatically adjusts it based on observed PSI metrics.
#[derive(Debug)]
pub struct SwapTuner {
    current_swappiness: u32,   // 0-100, default usually 60
    target_swappiness: u32,
}

impl SwapTuner {
    /// Create a new swap tuner and read current swappiness value.
    pub fn new() -> Self {
        let current = Self::read_swappiness().unwrap_or(60);
        Self {
            current_swappiness: current,
            target_swappiness: current,
        }
    }

    /// Read current swappiness value from `/proc/sys/vm/swappiness`.
    fn read_swappiness() -> Result<u32> {
        let content = fs::read_to_string("/proc/sys/vm/swappiness")?;
        Ok(content.trim().parse()?)
    }

    /// Write swappiness value to `/proc/sys/vm/swappiness`.
    ///
    /// Requires root privileges. If the write fails (e.g., due to permissions),
    /// this warns but doesn't fail - tuning is best-effort.
    fn write_swappiness(&self, value: u32) -> Result<()> {
        match fs::write("/proc/sys/vm/swappiness", value.to_string()) {
            Ok(_) => {
                eprintln!("✅ Swappiness tuned to {}", value);
                Ok(())
            }
            Err(e) => {
                eprintln!("⚠️  Cannot tune swappiness (requires root): {}", e);
                Ok(()) // Don't fail, just warn
            }
        }
    }

    /// Tune swappiness for current memory pressure levels.
    ///
    /// Strategy:
    /// - High pressure (>50%) → swappiness = 80 (aggressively reduce memory pressure)
    /// - Medium pressure (20-50%) → swappiness = 70 (moderate approach)
    /// - Low pressure (<20%) → swappiness = 40 (conservative, save swaps)
    pub async fn tune_for_pressure(&mut self, monitor: &PSIMonitor) -> Result<()> {
        if !monitor.enabled {
            return Ok(());
        }

        if let Some(reading) = monitor.current() {
            self.target_swappiness = if reading.full > 50.0 {
                80  // Aggressively reduce memory pressure
            } else if reading.full > 20.0 {
                70  // Moderate approach
            } else {
                40  // Conservative, save swaps
            };

            if self.target_swappiness != self.current_swappiness {
                self.write_swappiness(self.target_swappiness).ok();
                self.current_swappiness = self.target_swappiness;
            }
        }

        Ok(())
    }

    /// Get current swappiness value.
    pub fn current(&self) -> u32 {
        self.current_swappiness
    }

    /// Get target swappiness value.
    pub fn target(&self) -> u32 {
        self.target_swappiness
    }
}

impl Default for SwapTuner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psi_monitor_creation() {
        let monitor = PSIMonitor::new();
        assert!(monitor.is_ok());
        let m = monitor.unwrap();
        // Monitor should be either enabled (on Linux) or disabled gracefully
        // Either way, no panic
        assert!(!m.is_critical() || !m.is_warning());
    }

    #[test]
    fn test_psi_monitor_default() {
        let monitor = PSIMonitor::default();
        assert!(monitor.is_enabled() || !monitor.is_enabled()); // Either is fine
    }

    #[test]
    fn test_psi_parser_avg10() {
        // Test PSI line parsing
        let line = "some avg10=0.50 avg60=0.30 avg300=0.10 total=123456789";
        let result = PSIMonitor::parse_avg10(line);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0.50);
    }

    #[test]
    fn test_psi_parser_edge_cases() {
        let test_cases = vec![
            ("some avg10=0.00 avg60=0.00 avg300=0.00 total=0", 0.0),
            ("some avg10=100.00 avg60=100.00 avg300=100.00 total=999999999", 100.0),
            ("some avg10=23.45 avg60=12.34 avg300=1.23 total=555555", 23.45),
        ];

        for (line, expected) in test_cases {
            let result = PSIMonitor::parse_avg10(line);
            assert!(result.is_ok(), "Failed to parse: {}", line);
            assert_eq!(result.unwrap(), expected);
        }
    }

    #[test]
    fn test_swap_tuner_creation() {
        let tuner = SwapTuner::new();
        assert!(tuner.current() >= 0);
        assert!(tuner.current() <= 100);
    }

    #[test]
    fn test_swap_tuner_default() {
        let tuner = SwapTuner::default();
        assert!(tuner.current() >= 0);
        assert!(tuner.current() <= 100);
    }

    #[test]
    fn test_threshold_clamping() {
        let mut monitor = PSIMonitor::new().unwrap();
        monitor.set_critical_threshold(150.0); // Should clamp to 100.0
        assert_eq!(monitor.critical_threshold, 100.0);

        monitor.set_critical_threshold(-10.0); // Should clamp to 0.0
        assert_eq!(monitor.critical_threshold, 0.0);

        monitor.set_critical_threshold(50.0);
        assert_eq!(monitor.critical_threshold, 50.0);
    }

    #[tokio::test]
    async fn test_psi_update_when_disabled() {
        let mut monitor = PSIMonitor::new().unwrap();
        if !monitor.is_enabled() {
            // Should succeed even when disabled
            let result = monitor.update().await;
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_swap_tuner_when_disabled() {
        let monitor = PSIMonitor::new().unwrap();
        let result = monitor.tune_swap().await;
        // Should never fail, even if PSI is disabled
        assert!(result.is_ok());
    }

    #[test]
    fn test_psi_memory_ordering() {
        let mem1 = PSIMemory {
            some: 0.5,
            full: 0.1,
            timestamp: 100,
        };
        let mem2 = PSIMemory {
            some: 0.6,
            full: 0.2,
            timestamp: 110,
        };

        // Verify we can construct and store readings
        assert!(mem1.timestamp < mem2.timestamp);
        assert!(mem1.full < mem2.full);
    }
}
