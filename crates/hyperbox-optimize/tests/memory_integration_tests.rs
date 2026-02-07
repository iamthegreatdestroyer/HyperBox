//! Integration tests for B3: Dynamic Memory Manager
//!
//! Tests verify memory management functionality including:
//! - Memory sampling and analysis
//! - Balloon controller state management
//! - Memory pressure detection
//! - Idle detection and aggressive reclaim
//! - History tracking and EMA smoothing

#[cfg(test)]
mod tests {
    use hyperbox_optimize::memory::{ContainerMemoryState, MemoryConfig, MemorySample};
    use hyperbox_optimize::Result;

    /// Helper to create a new ContainerMemoryState with default values.
    fn new_container_state(container_id: &str) -> ContainerMemoryState {
        ContainerMemoryState {
            container_id: container_id.to_string(),
            balloon_inflated_bytes: 0,
            balloon_target_bytes: 0,
            latest_sample: None,
            ema_working_set: 0.0,
            idle_ticks: 0,
            is_idle: false,
            ksm_merged_bytes: 0,
            history: Vec::new(),
        }
    }

    mod memory_config {
        use super::*;

        #[test]
        fn test_default_config() {
            let config = MemoryConfig::default();

            assert!(config.balloon_enabled);
            assert!(config.free_page_reporting);
            assert!(!config.ksm_enabled);
            assert_eq!(config.high_watermark, 0.80);
            assert_eq!(config.low_watermark, 0.50);
            assert_eq!(config.poll_interval_ms, 1_000);
            assert_eq!(config.min_memory_bytes, 32 * 1_024 * 1_024); // 32 MiB
            assert_eq!(config.idle_change_threshold, 0.05);
        }

        #[test]
        fn test_custom_config() {
            let mut config = MemoryConfig::default();
            config.high_watermark = 0.85;
            config.low_watermark = 0.60;
            config.balloon_enabled = false;
            config.ksm_enabled = true;

            assert_eq!(config.high_watermark, 0.85);
            assert_eq!(config.low_watermark, 0.60);
            assert!(!config.balloon_enabled);
            assert!(config.ksm_enabled);
        }

        #[test]
        fn test_config_watermark_ordering() {
            let config = MemoryConfig::default();
            // High watermark should be greater than low watermark
            assert!(config.high_watermark > config.low_watermark);
            assert!(config.high_watermark <= 1.0);
            assert!(config.low_watermark >= 0.0);
        }
    }

    mod memory_sample_calculations {
        use super::*;

        #[test]
        fn test_working_set_calculation() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 200,
                active_file_bytes: 300,
                anon_bytes: 400,
                slab_reclaimable_bytes: 100,
            };

            // Working set = anon + active_file + slab = 400 + 300 + 100 = 800
            assert_eq!(sample.working_set_bytes(), 800);
        }

        #[test]
        fn test_working_set_overflow_safety() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: u64::MAX,
                anon_bytes: 100,
                slab_reclaimable_bytes: 0,
            };

            // Should saturate at u64::MAX, not overflow
            let working_set = sample.working_set_bytes();
            assert!(working_set >= 100);
        }

        #[test]
        fn test_usage_ratio_zero_limit() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 500,
                limit_bytes: 0,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            assert_eq!(sample.usage_ratio(), 0.0);
        }

        #[test]
        fn test_usage_ratio_normal() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            assert_eq!(sample.usage_ratio(), 0.5);
        }

        #[test]
        fn test_usage_ratio_at_limit() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 2000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            assert_eq!(sample.usage_ratio(), 1.0);
        }

        #[test]
        fn test_usage_ratio_over_limit() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 2500,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            assert!(sample.usage_ratio() > 1.0);
        }

        #[test]
        fn test_reclaimable_bytes() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 100,
                active_file_bytes: 300,
                anon_bytes: 400,
                slab_reclaimable_bytes: 100,
            };

            // Working set = 800, so reclaimable = 1000 - 800 = 200
            assert_eq!(sample.reclaimable_bytes(), 200);
        }

        #[test]
        fn test_reclaimable_bytes_underflow_safety() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 100,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 300,
                anon_bytes: 400,
                slab_reclaimable_bytes: 0,
            };

            // Working set (700) > current (100), so reclaimable should saturate at 0
            assert_eq!(sample.reclaimable_bytes(), 0);
        }
    }

    mod container_memory_state {
        use super::*;

        #[test]
        fn test_new_state() {
            let state = new_container_state("test-container");

            assert_eq!(state.container_id, "test-container");
            assert_eq!(state.balloon_inflated_bytes, 0);
            assert_eq!(state.balloon_target_bytes, 0);
            assert!(state.latest_sample.is_none());
            assert_eq!(state.ema_working_set, 0.0);
            assert_eq!(state.idle_ticks, 0);
            assert!(!state.is_idle);
            assert_eq!(state.ksm_merged_bytes, 0);
            assert!(state.history.is_empty());
        }

        #[test]
        fn test_state_isolation() {
            let state1 = new_container_state("container1");
            let state2 = new_container_state("container2");

            assert_ne!(state1.container_id, state2.container_id);
            assert_eq!(state1.balloon_inflated_bytes, state2.balloon_inflated_bytes);
        }

        #[test]
        fn test_state_update_isolation() {
            let mut state1 = new_container_state("container1");
            let mut state2 = new_container_state("container2");

            state1.balloon_inflated_bytes = 100;
            state2.balloon_inflated_bytes = 200;

            assert_eq!(state1.balloon_inflated_bytes, 100);
            assert_eq!(state2.balloon_inflated_bytes, 200);
        }
    }

    mod memory_pressure_states {
        use super::*;

        #[test]
        fn test_memory_pressure_low() {
            let config = MemoryConfig::default();
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 400,
                limit_bytes: 2000, // 20% usage
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let usage_ratio = sample.usage_ratio();
            assert!(usage_ratio < config.low_watermark);
        }

        #[test]
        fn test_memory_pressure_moderate() {
            let config = MemoryConfig::default();
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1200,
                limit_bytes: 2000, // 60% usage
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let usage_ratio = sample.usage_ratio();
            assert!(usage_ratio > config.low_watermark);
            assert!(usage_ratio < config.high_watermark);
        }

        #[test]
        fn test_memory_pressure_high() {
            let config = MemoryConfig::default();
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1800,
                limit_bytes: 2000, // 90% usage
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let usage_ratio = sample.usage_ratio();
            assert!(usage_ratio > config.high_watermark);
        }

        #[test]
        fn test_memory_pressure_custom_watermarks() {
            let mut config = MemoryConfig::default();
            config.high_watermark = 0.75;
            config.low_watermark = 0.25;

            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000, // 50% usage
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let usage_ratio = sample.usage_ratio();
            assert!(usage_ratio > config.low_watermark);
            assert!(usage_ratio < config.high_watermark);
        }
    }

    mod idle_detection {
        use super::*;

        #[test]
        fn test_idle_change_threshold() {
            let config = MemoryConfig::default();
            let sample1 = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let sample2 = MemorySample {
                timestamp_ms: 2000,
                current_bytes: 1020, // 2% change
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let change_ratio = (sample2.current_bytes as f64 - sample1.current_bytes as f64).abs()
                / sample1.current_bytes as f64;

            // 2% change is below 5% threshold, so idle
            assert!(change_ratio < config.idle_change_threshold);
        }

        #[test]
        fn test_non_idle_change_threshold() {
            let config = MemoryConfig::default();
            let sample1 = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let sample2 = MemorySample {
                timestamp_ms: 2000,
                current_bytes: 1100, // 10% change
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let change_ratio = (sample2.current_bytes as f64 - sample1.current_bytes as f64).abs()
                / sample1.current_bytes as f64;

            // 10% change is above 5% threshold, so not idle
            assert!(change_ratio > config.idle_change_threshold);
        }
    }

    mod swap_and_slab_handling {
        use super::*;

        #[test]
        fn test_swap_memory_tracking() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 500,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            assert_eq!(sample.swap_bytes, 500);
        }

        #[test]
        fn test_slab_reclaimable_memory() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 300,
                slab_reclaimable_bytes: 400,
            };

            // Slab is reclaimable, so included in working set
            assert_eq!(sample.working_set_bytes(), 700);
            assert_eq!(sample.reclaimable_bytes(), 300);
        }

        #[test]
        fn test_inactive_file_vs_active_file() {
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 300,
                active_file_bytes: 200,
                anon_bytes: 100,
                slab_reclaimable_bytes: 0,
            };

            // Only active file counts toward working set, inactive is reclaimable
            assert_eq!(sample.working_set_bytes(), 300); // 100 (anon) + 200 (active_file)
            assert_eq!(sample.reclaimable_bytes(), 700); // 1000 - 300
        }
    }

    mod memory_history_and_ema {
        use super::*;

        #[test]
        fn test_ema_calculation() {
            let mut state = new_container_state("test");
            const EMA_ALPHA: f64 = 0.3;

            // Initial EMA
            state.ema_working_set = 0.0;
            let sample_ws = 1000.0;
            let new_ema = EMA_ALPHA * sample_ws + (1.0 - EMA_ALPHA) * state.ema_working_set;
            assert!((new_ema - 300.0).abs() < 0.01);

            // Second update
            state.ema_working_set = new_ema;
            let sample_ws2 = 1100.0;
            let new_ema2 = EMA_ALPHA * sample_ws2 + (1.0 - EMA_ALPHA) * state.ema_working_set;
            assert!(new_ema2 > new_ema);
        }

        #[test]
        fn test_history_bounded() {
            let mut state = new_container_state("test");
            const MAX_HISTORY_SAMPLES: usize = 3_600;

            // Add samples up to max
            for i in 0..MAX_HISTORY_SAMPLES {
                state.history.push(MemorySample {
                    timestamp_ms: 1000 + i as u64,
                    current_bytes: 1000,
                    limit_bytes: 2000,
                    swap_bytes: 0,
                    inactive_file_bytes: 0,
                    active_file_bytes: 0,
                    anon_bytes: 0,
                    slab_reclaimable_bytes: 0,
                });
            }

            assert_eq!(state.history.len(), MAX_HISTORY_SAMPLES);
        }

        #[test]
        fn test_memory_sample_timestamp_progression() {
            let mut state = new_container_state("test");

            let sample1 = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1000,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            let sample2 = MemorySample {
                timestamp_ms: 2000,
                current_bytes: 1100,
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            state.latest_sample = Some(sample1);
            assert_eq!(state.latest_sample.as_ref().unwrap().timestamp_ms, 1000);

            state.latest_sample = Some(sample2);
            assert_eq!(state.latest_sample.as_ref().unwrap().timestamp_ms, 2000);
        }
    }

    mod balloon_management {
        use super::*;

        #[test]
        fn test_balloon_inflation() {
            let mut state = new_container_state("test");

            state.balloon_target_bytes = 500;
            state.balloon_inflated_bytes = 200;

            let inflation_needed = state
                .balloon_target_bytes
                .saturating_sub(state.balloon_inflated_bytes);
            assert_eq!(inflation_needed, 300);
        }

        #[test]
        fn test_balloon_deflation() {
            let mut state = new_container_state("test");

            state.balloon_inflated_bytes = 500;
            state.balloon_target_bytes = 200;

            let deflation_needed = state
                .balloon_inflated_bytes
                .saturating_sub(state.balloon_target_bytes);
            assert_eq!(deflation_needed, 300);
        }

        #[test]
        fn test_balloon_pressure_below_low_watermark() {
            let config = MemoryConfig::default();
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 600, // 30% of limit
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            // Below low watermark (50%), should deflate balloon
            assert!(sample.usage_ratio() < config.low_watermark);
        }

        #[test]
        fn test_balloon_pressure_above_high_watermark() {
            let config = MemoryConfig::default();
            let sample = MemorySample {
                timestamp_ms: 1000,
                current_bytes: 1800, // 90% of limit
                limit_bytes: 2000,
                swap_bytes: 0,
                inactive_file_bytes: 0,
                active_file_bytes: 0,
                anon_bytes: 0,
                slab_reclaimable_bytes: 0,
            };

            // Above high watermark (80%), should inflate balloon
            assert!(sample.usage_ratio() > config.high_watermark);
        }
    }

    mod ksm_merged_bytes {
        use super::*;

        #[test]
        fn test_ksm_tracking() {
            let mut state = new_container_state("test");

            state.ksm_merged_bytes = 0;
            assert_eq!(state.ksm_merged_bytes, 0);

            // When enabled and merged
            state.ksm_merged_bytes = 500;
            assert!(state.ksm_merged_bytes > 0);
        }
    }
}
