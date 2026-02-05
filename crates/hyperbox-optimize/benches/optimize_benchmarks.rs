//! Benchmarks for HyperBox optimization components
//!
//! Run with: cargo bench -p hyperbox-optimize

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::Duration;

/// Benchmark CRIU checkpoint preparation (mocked)
fn bench_criu_checkpoint_prep(c: &mut Criterion) {
    c.bench_function("criu_checkpoint_prep", |b| {
        b.iter(|| {
            // Simulate checkpoint preparation overhead
            let container_id = black_box("container_12345");
            let checkpoint_path = black_box("/tmp/checkpoint");

            // Mock: Validate container state
            let _state = container_id.len() + checkpoint_path.len();
        });
    });
}

/// Benchmark lazy layer lookup (hash-based)
fn bench_lazy_layer_lookup(c: &mut Criterion) {
    use std::collections::HashMap;

    // Pre-build a layer index
    let mut layer_index: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..10000 {
        layer_index.insert(
            format!("sha256:{:064x}", i),
            vec![0u8; 1024], // 1KB mock data
        );
    }

    c.bench_function("lazy_layer_lookup", |b| {
        b.iter(|| {
            let key = black_box(
                "sha256:0000000000000000000000000000000000000000000000000000000000001234",
            );
            layer_index.get(key)
        });
    });
}

/// Benchmark prediction model update
fn bench_prediction_update(c: &mut Criterion) {
    c.bench_function("prediction_update", |b| {
        b.iter(|| {
            let mut access_counts: Vec<(String, u64)> = (0..1000)
                .map(|i| (format!("container_{}", i), i as u64))
                .collect();

            // Simulate updating prediction weights
            for (_, count) in access_counts.iter_mut() {
                *count = (*count as f64 * 0.95 + 1.0) as u64;
            }

            // Sort by access count for prediction
            access_counts.sort_by(|a, b| b.1.cmp(&a.1));
            let top_10: Vec<_> = access_counts.into_iter().take(10).collect();
            black_box(top_10)
        });
    });
}

/// Benchmark pre-warm candidate selection
fn bench_prewarm_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("prewarm_selection");

    for size in [100, 1000, 10000].iter() {
        let candidates: Vec<(String, f64)> = (0..*size)
            .map(|i| (format!("container_{}", i), i as f64 / *size as f64))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Select top 10 candidates
                let mut sorted = candidates.clone();
                sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
                let top_10: Vec<_> = sorted.into_iter().take(10).collect();
                black_box(top_10)
            });
        });
    }

    group.finish();
}

/// Benchmark compression ratio estimation
fn bench_compression_estimation(c: &mut Criterion) {
    let data: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();

    c.bench_function("compression_estimation", |b| {
        b.iter(|| {
            // Estimate compressibility by sampling
            let sample_size = 1000;
            let sample: Vec<u8> = data
                .iter()
                .step_by(data.len() / sample_size)
                .copied()
                .collect();

            // Simple entropy estimation
            let mut counts = [0u32; 256];
            for byte in &sample {
                counts[*byte as usize] += 1;
            }

            let entropy: f64 = counts
                .iter()
                .filter(|&&c| c > 0)
                .map(|&c| {
                    let p = c as f64 / sample.len() as f64;
                    -p * p.log2()
                })
                .sum();

            black_box(entropy)
        });
    });
}

/// Benchmark layer deduplication lookup
fn bench_dedup_lookup(c: &mut Criterion) {
    use std::collections::HashSet;

    // Simulate a dedup index
    let dedup_index: HashSet<[u8; 32]> = (0u64..100_000)
        .map(|i| {
            let mut hash = [0u8; 32];
            hash[0..8].copy_from_slice(&i.to_le_bytes());
            hash
        })
        .collect();

    c.bench_function("dedup_lookup", |b| {
        let test_hash = {
            let mut hash = [0u8; 32];
            hash[0..8].copy_from_slice(&50000u64.to_le_bytes());
            hash
        };

        b.iter(|| black_box(dedup_index.contains(&test_hash)));
    });
}

criterion_group!(
    benches,
    bench_criu_checkpoint_prep,
    bench_lazy_layer_lookup,
    bench_prediction_update,
    bench_prewarm_selection,
    bench_compression_estimation,
    bench_dedup_lookup,
);

criterion_main!(benches);
