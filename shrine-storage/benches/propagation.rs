//! Version propagation overhead benchmarks for shrine-storage.
//!
//! Tests:
//! - Y-propagation (folder-level) overhead
//! - Z-propagation (subtree-level) overhead
//! - Impact of sibling count on Y-propagation
//! - Impact of hierarchy depth on Z-propagation

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
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
// Y-Propagation (Folder-level) Overhead
// =============================================================================

/// Benchmark Y-propagation with varying number of siblings.
/// When you write to a path, the parent's Y-lock must be updated,
/// which requires iterating over all children to build the snapshot.
fn bench_y_propagation_sibling_count(c: &mut Criterion) {
    let mut group = c.benchmark_group("y_propagation_siblings");

    let data = vec![0xABu8; 256];

    for num_siblings in [1, 10, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_siblings", num_siblings)),
            num_siblings,
            |b, &num_siblings| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent
                storage
                    .batch_write(&[Binding::new("/parent", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create siblings in batches
                let batch_size = 100.min(num_siblings);
                for batch_start in (0..num_siblings).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(num_siblings);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            Binding::new(
                                format!("/parent/sibling_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                // Benchmark updating one sibling (triggers Y-propagation to parent)
                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/parent/sibling_0",
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

/// Benchmark Y-propagation overhead when creating new children.
/// Creating a new child changes the parent's Y-shape, which is more expensive.
fn bench_y_propagation_create_child(c: &mut Criterion) {
    let mut group = c.benchmark_group("y_propagation_create");

    let data = vec![0xABu8; 256];

    for existing_siblings in [0, 10, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_existing", existing_siblings)),
            existing_siblings,
            |b, &existing_siblings| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent
                storage
                    .batch_write(&[Binding::new("/parent", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create existing siblings
                if existing_siblings > 0 {
                    let batch_size = 100.min(existing_siblings);
                    for batch_start in (0..existing_siblings).step_by(batch_size) {
                        let batch_end = (batch_start + batch_size).min(existing_siblings);
                        let bindings: Vec<_> = (batch_start..batch_end)
                            .map(|i| {
                                Binding::new(
                                    format!("/parent/existing_{}", i),
                                    Lock::new(1, 1),
                                    make_slots(&data),
                                )
                            })
                            .collect();
                        storage.batch_write(&bindings).unwrap();
                    }
                }

                let mut new_child_idx = 0u64;
                b.iter(|| {
                    // Create a new child (shape change, more expensive)
                    storage
                        .batch_write(&[Binding::new(
                            format!("/parent/new_{}", new_child_idx),
                            Lock::new(1, 1),
                            make_slots(&data),
                        )])
                        .unwrap();
                    new_child_idx += 1;
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Z-Propagation (Subtree-level) Overhead
// =============================================================================

/// Benchmark Z-propagation with varying hierarchy depth.
/// Writing at a deep path requires updating Z-locks for all ancestors.
fn bench_z_propagation_depth(c: &mut Criterion) {
    let mut group = c.benchmark_group("z_propagation_depth");

    let data = vec![0xABu8; 256];

    for depth in [1, 5, 10, 20, 30].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("depth_{}", depth)),
            depth,
            |b, &depth| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create all ancestor paths first so they have Z-locks
                let mut path = String::new();
                for i in 0..depth {
                    path.push_str(&format!("/level{}", i));
                    storage
                        .batch_write(&[Binding::new(&path, Lock::new(1, 1), make_slots(&data))])
                        .unwrap();
                }

                // The leaf path
                let leaf_path = path.clone();

                // Get current version of leaf
                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            &leaf_path,
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

/// Benchmark Z-propagation with varying descendant count at each level.
/// More descendants means larger Z-snapshots to build.
fn bench_z_propagation_descendants(c: &mut Criterion) {
    let mut group = c.benchmark_group("z_propagation_descendants");

    let data = vec![0xABu8; 256];

    for total_descendants in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_descendants", total_descendants)),
            total_descendants,
            |b, &total_descendants| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create root
                storage
                    .batch_write(&[Binding::new("/root", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                // Create descendants at mixed depths
                let batch_size = 50.min(total_descendants);
                for batch_start in (0..total_descendants).step_by(batch_size) {
                    let batch_end = (batch_start + batch_size).min(total_descendants);
                    let bindings: Vec<_> = (batch_start..batch_end)
                        .map(|i| {
                            let depth = (i % 3) + 1;
                            let path = match depth {
                                1 => format!("/root/d_{}", i),
                                2 => format!("/root/a/d_{}", i),
                                _ => format!("/root/a/b/d_{}", i),
                            };
                            Binding::new(path, Lock::new(1, 1), make_slots(&data))
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                }

                // Benchmark updating one leaf (triggers Z-propagation up to root)
                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/root/d_0",
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

/// Benchmark combined Y and Z propagation overhead.
/// This measures the total overhead when both propagations occur.
fn bench_combined_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("combined_propagation");

    let data = vec![0xABu8; 256];

    // Scenario: deep path with wide siblings at each level
    group.bench_function("deep_and_wide", |b| {
        let mut storage = Storage::open_in_memory().unwrap();

        let depth = 5;
        let siblings_per_level = 10;

        // Create structure: each level has `siblings_per_level` children
        for level in 0..depth {
            let parent = if level == 0 {
                "/root".to_string()
            } else {
                let mut p = "/root".to_string();
                for l in 0..level {
                    p.push_str(&format!("/child_0_level{}", l));
                }
                p
            };

            // Create parent if it doesn't exist
            if level == 0 {
                storage
                    .batch_write(&[Binding::new(&parent, Lock::new(1, 1), make_slots(&data))])
                    .unwrap();
            }

            // Create siblings at this level
            let bindings: Vec<_> = (0..siblings_per_level)
                .map(|i| {
                    Binding::new(
                        format!("{}/child_{}_level{}", parent, i, level),
                        Lock::new(1, 1),
                        make_slots(&data),
                    )
                })
                .collect();
            storage.batch_write(&bindings).unwrap();
        }

        // Benchmark updating a deep leaf
        let mut version = 2u64;
        let leaf_path = format!(
            "/root/child_0_level0/child_0_level1/child_0_level2/child_0_level3/child_0_level4"
        );

        // First ensure leaf exists
        storage.batch_write(&[Binding::new(&leaf_path, Lock::new(1, 1), make_slots(&data))]);
        // .unwrap();

        b.iter(|| {
            storage
                .batch_write(&[Binding::new(
                    &leaf_path,
                    Lock::new(version, 1),
                    make_slots(&data),
                )])
                .unwrap();
            version += 1;
        });
    });

    group.finish();
}

/// Benchmark batch writes effect on propagation.
/// Multiple paths in same batch may share ancestors, potentially optimizing propagation.
fn bench_batch_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_propagation");

    let data = vec![0xABu8; 256];

    // Benchmark: write N siblings individually vs in batch
    for num_paths in [2, 5, 10].iter() {
        // Individual writes
        group.bench_with_input(
            BenchmarkId::new("individual", num_paths),
            num_paths,
            |b, &num_paths| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent
                storage
                    .batch_write(&[Binding::new(
                        "/batch_parent",
                        Lock::new(1, 1),
                        make_slots(&data),
                    )])
                    .unwrap();

                let mut batch_num = 0u64;
                b.iter(|| {
                    for i in 0..num_paths {
                        storage
                            .batch_write(&[Binding::new(
                                format!("/batch_parent/path_{}_batch{}", i, batch_num),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )])
                            .unwrap();
                    }
                    batch_num += 1;
                });
            },
        );

        // Batch write
        group.bench_with_input(
            BenchmarkId::new("batched", num_paths),
            num_paths,
            |b, &num_paths| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Create parent
                storage
                    .batch_write(&[Binding::new(
                        "/batch_parent2",
                        Lock::new(1, 1),
                        make_slots(&data),
                    )])
                    .unwrap();

                let mut batch_num = 0u64;
                b.iter(|| {
                    let bindings: Vec<_> = (0..num_paths)
                        .map(|i| {
                            Binding::new(
                                format!("/batch_parent2/path_{}_batch{}", i, batch_num),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                    batch_num += 1;
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_y_propagation_sibling_count,
    bench_y_propagation_create_child,
    bench_z_propagation_depth,
    bench_z_propagation_descendants,
    bench_combined_propagation,
    bench_batch_propagation,
);
criterion_main!(benches);
