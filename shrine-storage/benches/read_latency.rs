//! Read latency benchmarks for shrine-storage.
//!
//! Tests:
//! - Current version reads for read_x(), read_y(), read_z()
//! - Historical version reads
//! - Reads with varying hierarchy width and depth
//! - Mixed read/write workloads

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use rand::prelude::*;
use shrine_storage::{Binding, Lock, Pail, Storage};
use std::collections::HashMap;

/// Helper to create slots with a single data payload
fn make_slots(data: &[u8]) -> HashMap<String, Pail> {
    HashMap::from([(
        "data".to_string(),
        Pail::new("/types/binary", data.to_vec()),
    )])
}

// =============================================================================
// Current Version Reads - Basic Operations
// =============================================================================

/// Benchmark read_x() for current version
fn bench_read_x_current(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_x_current");

    // Vary payload sizes
    for size in [1024, 10 * 1024, 100 * 1024].iter() {
        let payload = vec![0xABu8; *size];
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size / 1024)),
            size,
            |b, _size| {
                let mut storage = Storage::open_in_memory().unwrap();
                storage
                    .batch_write(&[Binding::new(
                        "/read/test",
                        Lock::new(1, 1),
                        make_slots(&payload),
                    )])
                    .unwrap();

                b.iter(|| {
                    let result = storage.read_x("/read/test", None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark read_y() for current version with varying number of children
fn bench_read_y_current(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_y_current");

    let data = vec![0xABu8; 256];

    for num_children in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_children as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_children", num_children)),
            num_children,
            |b, &num_children| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent first so it has y/z locks
                storage
                    .batch_write(&[Binding::new("/parent", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create children in batches
                let batch_size = 100.min(num_children);
                for batch_start in (0..num_children).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(num_children);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            Binding::new(
                                format!("/parent/child_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                b.iter(|| {
                    let result = storage.read_y("/parent", None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark read_z() for current version with varying number of descendants
fn bench_read_z_current(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_z_current");

    let data = vec![0xABu8; 256];

    for num_descendants in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*num_descendants as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_descendants", num_descendants)),
            num_descendants,
            |b, &num_descendants| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create root first
                storage
                    .batch_write(&[Binding::new("/root", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create descendants at various depths
                let batch_size = 100.min(num_descendants);
                for batch_start in (0..num_descendants).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(num_descendants);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            // Vary depth: some at /root/x, some at /root/a/x, etc.
                            let depth = (i % 3) + 1;
                            let path = match depth {
                                1 => format!("/root/d_{}", i),
                                2 => format!("/root/level2/d_{}", i),
                                _ => format!("/root/level2/level3/d_{}", i),
                            };
                            Binding::new(path, Lock::new(1, 1), make_slots(&data))
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                b.iter(|| {
                    let result = storage.read_z("/root", None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Historical Version Reads
// =============================================================================

/// Benchmark reading historical versions (comparing recent vs old)
fn bench_read_x_historical(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_x_historical");

    let data = vec![0xABu8; 1024];
    let num_versions = 1000;

    // Setup: create path with many versions
    let mut storage = Storage::open_in_memory().unwrap();
    for v in 1..=num_versions {
        storage
            .batch_write(&[Binding::new("/history", Lock::new(v, 1), make_slots(&data))])
            .unwrap();
    }

    // Benchmark reading current version
    group.bench_function("current", |b| {
        b.iter(|| {
            let result = storage.read_x("/history", None).unwrap();
            black_box(result);
        });
    });

    // Benchmark reading recent version (version 999)
    group.bench_function("recent_v999", |b| {
        b.iter(|| {
            let result = storage.read_x("/history", Some(999)).unwrap();
            black_box(result);
        });
    });

    // Benchmark reading middle version (version 500)
    group.bench_function("middle_v500", |b| {
        b.iter(|| {
            let result = storage.read_x("/history", Some(500)).unwrap();
            black_box(result);
        });
    });

    // Benchmark reading oldest version (version 1)
    group.bench_function("oldest_v1", |b| {
        b.iter(|| {
            let result = storage.read_x("/history", Some(1)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark historical read_y() versions
fn bench_read_y_historical(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_y_historical");

    let data = vec![0xABu8; 256];

    // Setup: create parent with children, then update children multiple times
    let mut storage = Storage::open_in_memory().unwrap();

    // Create parent
    storage
        .batch_write(&[Binding::new(
            "/yhistory",
            Lock::new(1, 1),
            make_slots(&data),
        )])
        .unwrap();

    // Create 10 children
    for i in 0..10 {
        storage
            .batch_write(&[Binding::new(
                format!("/yhistory/child_{}", i),
                Lock::new(1, 1),
                make_slots(&data),
            )])
            .unwrap();
    }

    // Update children multiple times to create y-version history
    for v in 2..=100 {
        storage
            .batch_write(&[Binding::new(
                "/yhistory/child_0",
                Lock::new(v, 1),
                make_slots(&data),
            )])
            .unwrap();
    }

    // Benchmark reading various y versions
    group.bench_function("current", |b| {
        b.iter(|| {
            let result = storage.read_y("/yhistory", None).unwrap();
            black_box(result);
        });
    });

    group.bench_function("historical_v50", |b| {
        b.iter(|| {
            let result = storage.read_y("/yhistory", Some(50)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("historical_v1", |b| {
        b.iter(|| {
            let result = storage.read_y("/yhistory", Some(1)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// =============================================================================
// Read with Varying Hierarchy Depth
// =============================================================================

/// Benchmark reads at different hierarchy depths
fn bench_read_by_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_by_depth");

    let data = vec![0xABu8; 1024];

    for depth in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("depth_{}", depth)),
            depth,
            |b, &depth| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create path at specified depth: /a/b/c/d/...
                let mut path = String::new();
                for i in 0..depth {
                    path.push_str(&format!("/level{}", i));
                }

                storage
                    .batch_write(&[Binding::new(&path, Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                b.iter(|| {
                    let result = storage.read_x(&path, None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Mixed Read/Write Workloads
// =============================================================================

/// Simulate 80/20 read/write workload
fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");

    let data = vec![0xABu8; 1024];
    let num_paths = 100;

    group.bench_function("80_read_20_write", |b| {
        let mut storage = Storage::open_in_memory().unwrap();
        let mut rng = rand::thread_rng();

        // Initialize paths
        for i in 0..num_paths {
            storage
                .batch_write(&[Binding::new(
                    format!("/mixed/path_{}", i),
                    Lock::new(1, 1),
                    make_slots(&data),
                )])
                .unwrap();
        }

        // Track versions for each path
        let mut versions: Vec<u64> = vec![1; num_paths];
        let mut op_count = 0u64;

        b.iter(|| {
            let path_idx = rng.gen_range(0..num_paths);
            let path = format!("/mixed/path_{}", path_idx);

            // 80% reads, 20% writes
            if rng.gen_ratio(80, 100) {
                let result = storage.read_x(&path, None).unwrap();
                black_box(result);
            } else {
                versions[path_idx] += 1;
                storage
                    .batch_write(&[Binding::new(
                        &path,
                        Lock::new(versions[path_idx], 1),
                        make_slots(&data),
                    )])
                    .unwrap();
            }
            op_count += 1;
        });
    });

    group.bench_function("50_read_50_write", |b| {
        let mut storage = Storage::open_in_memory().unwrap();
        let mut rng = rand::thread_rng();

        // Initialize paths
        for i in 0..num_paths {
            storage
                .batch_write(&[Binding::new(
                    format!("/mixed50/path_{}", i),
                    Lock::new(1, 1),
                    make_slots(&data),
                )])
                .unwrap();
        }

        let mut versions: Vec<u64> = vec![1; num_paths];

        b.iter(|| {
            let path_idx = rng.gen_range(0..num_paths);
            let path = format!("/mixed50/path_{}", path_idx);

            if rng.gen_bool(0.5) {
                let result = storage.read_x(&path, None).unwrap();
                black_box(result);
            } else {
                versions[path_idx] += 1;
                storage
                    .batch_write(&[Binding::new(
                        &path,
                        Lock::new(versions[path_idx], 1),
                        make_slots(&data),
                    )])
                    .unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark hot-spot access pattern (most reads go to a few paths)
fn bench_hotspot_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("hotspot_access");

    let data = vec![0xABu8; 1024];
    let num_paths = 1000;
    let hot_paths = 10; // 10 paths get 80% of reads

    group.bench_function("80pct_to_1pct_paths", |b| {
        let mut storage = Storage::open_in_memory().unwrap();
        let mut rng = rand::thread_rng();

        // Initialize paths
        for i in 0..num_paths {
            storage
                .batch_write(&[Binding::new(
                    format!("/hotspot/path_{}", i),
                    Lock::new(1, 1),
                    make_slots(&data),
                )])
                .unwrap();
        }

        b.iter(|| {
            // 80% chance of reading from hot paths
            let path_idx = if rng.gen_ratio(80, 100) {
                rng.gen_range(0..hot_paths)
            } else {
                rng.gen_range(hot_paths..num_paths)
            };
            let path = format!("/hotspot/path_{}", path_idx);
            let result = storage.read_x(&path, None).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// =============================================================================
// Multi-slot Reads
// =============================================================================

/// Benchmark reading bindings with multiple slots
fn bench_read_multi_slot(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_multi_slot");

    let slot_data = vec![0xABu8; 1024];

    for num_slots in [1, 5, 10, 20].iter() {
        group.throughput(Throughput::Elements(*num_slots as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_slots", num_slots)),
            num_slots,
            |b, &num_slots| {
                let mut storage = Storage::open_in_memory().unwrap();

                let slots: HashMap<String, Pail> = (0..num_slots)
                    .map(|i| {
                        (
                            format!("slot_{}", i),
                            Pail::new("/types/binary", slot_data.clone()),
                        )
                    })
                    .collect();

                storage
                    .batch_write(&[Binding::new("/multislot", Lock::new(1, 1), slots)])
                    .unwrap();

                b.iter(|| {
                    let result = storage.read_x("/multislot", None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_read_x_current,
    bench_read_y_current,
    bench_read_z_current,
    bench_read_x_historical,
    bench_read_y_historical,
    bench_read_by_depth,
    bench_mixed_workload,
    bench_hotspot_access,
    bench_read_multi_slot,
);
criterion_main!(benches);
