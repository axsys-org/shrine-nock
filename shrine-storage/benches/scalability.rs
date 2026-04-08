//! Scalability benchmarks for shrine-storage.
//!
//! Tests:
//! - Deep hierarchy performance (paths up to 50 levels deep)
//! - Wide hierarchy performance (single parent with many children)
//! - Total path count scaling (1K to 100K paths)
//! - Database size impact on operations

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use shrine_storage::{Binding, Lock, Pail, Storage};
use std::collections::HashMap;
use tempfile::TempDir;

/// Helper to create slots with a single data payload
fn make_slots(data: &[u8]) -> HashMap<String, Pail> {
    HashMap::from([(
        "data".to_string(),
        Pail::new("/types/binary", data.to_vec()),
    )])
}

// =============================================================================
// Deep Hierarchy Performance
// =============================================================================

/// Benchmark operations at extreme hierarchy depths.
fn bench_deep_hierarchy_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("deep_hierarchy_write");

    let data = vec![0xABu8; 256];

    for depth in [5, 10, 20, 30, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("depth_{}", depth)),
            depth,
            |b, &depth| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Build the deep path
                let mut path = String::new();
                for i in 0..depth {
                    path.push_str(&format!("/l{}", i));
                }

                // Initialize all ancestor paths
                let mut ancestor = String::new();
                for i in 0..depth {
                    ancestor.push_str(&format!("/l{}", i));
                    storage
                        .batch_write(&[Binding::new(&ancestor, Lock::new(1, 1), make_slots(&data))])
                        .unwrap();
                }

                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            &path,
                            Lock::new(version, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

/// Benchmark read operations at extreme depths
fn bench_deep_hierarchy_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("deep_hierarchy_read");

    let data = vec![0xABu8; 256];

    for depth in [5, 10, 20, 30, 50].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("depth_{}", depth)),
            depth,
            |b, &depth| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Build and create the deep path
                let mut path = String::new();
                for i in 0..depth {
                    path.push_str(&format!("/l{}", i));
                    storage
                        .batch_write(&[Binding::new(&path, Lock::new(1, 1), make_slots(&data))])
                        .unwrap();
                }

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
// Wide Hierarchy Performance
// =============================================================================

/// Benchmark operations with many siblings (wide hierarchy).
fn bench_wide_hierarchy_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_hierarchy_write");
    group.sample_size(50);

    let data = vec![0xABu8; 256];

    for num_children in [10, 100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_children", num_children)),
            num_children,
            |b, &num_children| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent
                storage
                    .batch_write(&[Binding::new("/wide", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create all children
                let batch_size = 500.min(num_children);
                for batch_start in (0..num_children).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(num_children);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            Binding::new(
                                format!("/wide/child_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                // Benchmark: write to one child (triggers Y-propagation across all siblings)
                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/wide/child_0",
                            Lock::new(version, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

/// Benchmark read_y with many children
fn bench_wide_hierarchy_read_y(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_hierarchy_read_y");
    group.sample_size(50);

    let data = vec![0xABu8; 256];

    for num_children in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*num_children as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_children", num_children)),
            num_children,
            |b, &num_children| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent
                storage
                    .batch_write(&[Binding::new("/wide_y", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create all children
                let batch_size = 500.min(num_children);
                for batch_start in (0..num_children).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(num_children);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            Binding::new(
                                format!("/wide_y/child_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                b.iter(|| {
                    let result = storage.read_y("/wide_y", None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark read_z with many descendants
fn bench_wide_hierarchy_read_z(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide_hierarchy_read_z");
    group.sample_size(50);

    let data = vec![0xABu8; 256];

    for num_descendants in [10, 100, 1000, 10000].iter() {
        group.throughput(Throughput::Elements(*num_descendants as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_descendants", num_descendants)),
            num_descendants,
            |b, &num_descendants| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create root
                storage
                    .batch_write(&[Binding::new("/wide_z", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create descendants at various depths
                let batch_size = 500.min(num_descendants);
                for batch_start in (0..num_descendants).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(num_descendants);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            let depth = (i % 3) + 1;
                            let path = match depth {
                                1 => format!("/wide_z/d_{}", i),
                                2 => format!("/wide_z/a/d_{}", i),
                                _ => format!("/wide_z/a/b/d_{}", i),
                            };
                            Binding::new(path, Lock::new(1, 1), make_slots(&data))
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                b.iter(|| {
                    let result = storage.read_z("/wide_z", None).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Total Path Count Scaling
// =============================================================================

/// Benchmark operations as database grows in total path count.
fn bench_path_count_scaling_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_count_scaling_write");
    group.sample_size(30);

    let data = vec![0xABu8; 256];

    for total_paths in [1000, 10_000, 50_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_paths", total_paths)),
            total_paths,
            |b, &total_paths| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create many paths spread across hierarchy
                let paths_per_batch = 500;
                for batch_start in (0..total_paths).step_by(paths_per_batch) {
                    let batch_end = (batch_start + paths_per_batch).min(total_paths);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            // Distribute across multiple parent directories
                            let parent_idx = i % 100;
                            Binding::new(
                                format!("/scale/parent_{}/child_{}", parent_idx, i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                // Benchmark: write to a specific path
                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/scale/parent_0/child_0",
                            Lock::new(version, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

/// Benchmark read operations as database grows
fn bench_path_count_scaling_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_count_scaling_read");
    group.sample_size(30);

    let data = vec![0xABu8; 256];

    for total_paths in [1000, 10_000, 50_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_paths", total_paths)),
            total_paths,
            |b, &total_paths| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create many paths
                let paths_per_batch = 500;
                for batch_start in (0..total_paths).step_by(paths_per_batch) {
                    let batch_end = (batch_start + paths_per_batch).min(total_paths);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            let parent_idx = i % 100;
                            Binding::new(
                                format!("/scale_read/parent_{}/child_{}", parent_idx, i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                b.iter(|| {
                    let result = storage
                        .read_x("/scale_read/parent_50/child_50", None)
                        .unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// File-backed Storage Scaling
// =============================================================================

/// Benchmark file-backed storage performance as database grows
fn bench_file_backed_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_backed_scaling");
    group.sample_size(20);

    let data = vec![0xABu8; 1024];

    for total_paths in [1000, 5000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_paths", total_paths)),
            total_paths,
            |b, &total_paths| {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("scale.db");
                let mut storage = Storage::open(&db_path).unwrap();

                // Create paths
                let paths_per_batch = 500;
                for batch_start in (0..total_paths).step_by(paths_per_batch) {
                    let batch_end = (batch_start + paths_per_batch).min(total_paths);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            let parent_idx = i % 100;
                            Binding::new(
                                format!("/file/parent_{}/child_{}", parent_idx, i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/file/parent_0/child_0",
                            Lock::new(version, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Version History Scaling
// =============================================================================

/// Benchmark operations as version history grows for a single path
fn bench_version_history_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_history_scaling");
    group.sample_size(30);

    let data = vec![0xABu8; 1024];

    for num_versions in [100, 1000, 5000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_versions", num_versions)),
            num_versions,
            |b, &num_versions| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create path with many versions
                for v in 1..=num_versions {
                    storage
                        .batch_write(&[Binding::new(
                            "/versioned",
                            Lock::new(v as u64, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                }

                // Benchmark: add one more version
                let mut version = (num_versions + 1) as u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/versioned",
                            Lock::new(version, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

/// Benchmark historical reads as version history grows
fn bench_version_history_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_history_read");

    let data = vec![0xABu8; 1024];

    for num_versions in [100, 1000, 5000].iter() {
        // Setup: create path with many versions
        let mut storage = Storage::open_in_memory().unwrap();
        for v in 1..=*num_versions {
            storage
                .batch_write(&[Binding::new(
                    "/history_read",
                    Lock::new(v as u64, 1),
                    make_slots(&data),
                )])
                .unwrap();
        }

        // Read current
        group.bench_with_input(
            BenchmarkId::new("current", format!("{}_versions", num_versions)),
            num_versions,
            |b, _| {
                b.iter(|| {
                    let result = storage.read_x("/history_read", None).unwrap();
                    black_box(result);
                });
            },
        );

        // Read historical (middle)
        group.bench_with_input(
            BenchmarkId::new("historical_mid", format!("{}_versions", num_versions)),
            num_versions,
            |b, num_versions| {
                let mid_version = (*num_versions / 2) as u64;
                b.iter(|| {
                    let result = storage.read_x("/history_read", Some(mid_version)).unwrap();
                    black_box(result);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_deep_hierarchy_write,
    bench_deep_hierarchy_read,
    bench_wide_hierarchy_write,
    bench_wide_hierarchy_read_y,
    bench_wide_hierarchy_read_z,
    bench_path_count_scaling_write,
    bench_path_count_scaling_read,
    bench_file_backed_scaling,
    bench_version_history_scaling,
    bench_version_history_read,
);
criterion_main!(benches);
