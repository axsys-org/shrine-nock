//! Concurrency and contention benchmarks for shrine-storage.
//!
//! Tests:
//! - Reader/writer contention
//! - Multi-instance coordination (multiple Storage instances on same DB)
//! - WAL checkpoint overhead
//! - Concurrent read throughput

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use shrine_storage::{Binding, Lock, Pail, Storage};
use std::collections::HashMap;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

/// Helper to create slots with a single data payload
fn make_slots(data: &[u8]) -> HashMap<String, Pail> {
    HashMap::from([(
        "data".to_string(),
        Pail::new("/types/binary", data.to_vec()),
    )])
}

// =============================================================================
// Multi-instance Coordination
// =============================================================================

/// Benchmark multiple Storage instances writing to the same database.
/// This tests SQLite's WAL mode coordination.
fn bench_multi_instance_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_instance_writes");
    group.sample_size(20);

    let data = vec![0xABu8; 1024];

    for num_instances in [1, 2, 4].iter() {
        group.throughput(Throughput::Elements(*num_instances as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_instances", num_instances)),
            num_instances,
            |b, &num_instances| {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("concurrent.db");

                // Create initial storage and paths
                {
                    let mut storage = Storage::open(&db_path).unwrap();
                    for i in 0..num_instances {
                        storage
                            .batch_write(&[Binding::new(
                                format!("/instance_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )])
                            .unwrap();
                    }
                }

                b.iter(|| {
                    let barrier = Arc::new(Barrier::new(num_instances));
                    let handles: Vec<_> = (0..num_instances)
                        .map(|i| {
                            let db_path = db_path.clone();
                            let data = data.clone();
                            let barrier = Arc::clone(&barrier);

                            thread::spawn(move || {
                                let mut storage = Storage::open(&db_path).unwrap();

                                // Get current version
                                let current_lock = storage
                                    .get_current_lock(
                                        &format!("/instance_{}", i),
                                        shrine_storage::Care::X,
                                    )
                                    .unwrap()
                                    .unwrap();

                                // Wait for all threads to be ready
                                barrier.wait();

                                // Write
                                storage
                                    .batch_write(&[Binding::new(
                                        format!("/instance_{}", i),
                                        Lock::new(current_lock.data + 1, current_lock.shape),
                                        make_slots(&data),
                                    )])
                                    .unwrap();
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

/// Benchmark concurrent reads from multiple threads
fn bench_multi_instance_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_instance_reads");
    group.sample_size(20);

    let data = vec![0xABu8; 1024];
    let num_paths = 100;

    for num_readers in [1, 2, 4, 8].iter() {
        group.throughput(Throughput::Elements((*num_readers * 100) as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_readers", num_readers)),
            num_readers,
            |b, &num_readers| {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("read_concurrent.db");

                // Create initial storage and paths
                {
                    let mut storage = Storage::open(&db_path).unwrap();
                    for i in 0..num_paths {
                        storage
                            .batch_write(&[Binding::new(
                                format!("/read_path_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )])
                            .unwrap();
                    }
                }

                b.iter(|| {
                    let barrier = Arc::new(Barrier::new(num_readers));
                    let handles: Vec<_> = (0..num_readers)
                        .map(|reader_id| {
                            let db_path = db_path.clone();
                            let barrier = Arc::clone(&barrier);

                            thread::spawn(move || {
                                let storage = Storage::open(&db_path).unwrap();

                                // Wait for all threads to be ready
                                barrier.wait();

                                // Each reader reads 100 paths
                                for i in 0..100 {
                                    let path_idx = (reader_id * 10 + i) % num_paths;
                                    let result = storage
                                        .read_x(&format!("/read_path_{}", path_idx), None)
                                        .unwrap();
                                    black_box(result);
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Reader/Writer Contention
// =============================================================================

/// Benchmark read latency while writes are occurring
fn bench_reader_writer_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("reader_writer_contention");
    group.sample_size(20);

    let data = vec![0xABu8; 1024];

    // Scenario: N readers, 1 continuous writer
    for num_readers in [1, 2, 4].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_readers_1_writer", num_readers)),
            num_readers,
            |b, &num_readers| {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("contention.db");

                // Create initial data
                {
                    let mut storage = Storage::open(&db_path).unwrap();
                    storage
                        .batch_write(&[Binding::new(
                            "/contended",
                            Lock::new(1, 1),
                            make_slots(&data),
                        )])
                        .unwrap();

                    // Create paths for readers to read
                    for i in 0..100 {
                        storage
                            .batch_write(&[Binding::new(
                                format!("/read_{}", i),
                                Lock::new(1, 1),
                                make_slots(&data),
                            )])
                            .unwrap();
                    }
                }

                b.iter(|| {
                    use std::sync::atomic::{AtomicBool, Ordering};

                    let stop_flag = Arc::new(AtomicBool::new(false));
                    let barrier = Arc::new(Barrier::new(num_readers + 1)); // +1 for writer

                    // Spawn writer thread
                    let writer_handle = {
                        let db_path = db_path.clone();
                        let data = data.clone();
                        let stop_flag = Arc::clone(&stop_flag);
                        let barrier = Arc::clone(&barrier);

                        thread::spawn(move || {
                            let mut storage = Storage::open(&db_path).unwrap();
                            let mut version = 2u64;

                            barrier.wait();

                            while !stop_flag.load(Ordering::Relaxed) {
                                storage
                                    .batch_write(&[Binding::new(
                                        "/contended",
                                        Lock::new(version, 1),
                                        make_slots(&data),
                                    )])
                                    .unwrap();
                                version += 1;

                                // Small pause to not completely saturate
                                if version % 10 == 0 {
                                    thread::yield_now();
                                }
                            }
                        })
                    };

                    // Spawn reader threads
                    let reader_handles: Vec<_> = (0..num_readers)
                        .map(|reader_id| {
                            let db_path = db_path.clone();
                            let barrier = Arc::clone(&barrier);

                            thread::spawn(move || {
                                let storage = Storage::open(&db_path).unwrap();

                                barrier.wait();

                                // Each reader does 50 reads
                                for i in 0..50 {
                                    let path_idx = (reader_id * 7 + i) % 100;
                                    let result = storage
                                        .read_x(&format!("/read_{}", path_idx), None)
                                        .unwrap();
                                    black_box(result);
                                }
                            })
                        })
                        .collect();

                    // Wait for readers to finish
                    for handle in reader_handles {
                        handle.join().unwrap();
                    }

                    // Stop the writer
                    stop_flag.store(true, Ordering::Relaxed);
                    writer_handle.join().unwrap();
                });
            },
        );
    }
    group.finish();
}

// =============================================================================
// Single Instance, Sequential Access (Baseline)
// =============================================================================

/// Baseline: single instance sequential writes for comparison
fn bench_single_instance_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_instance_baseline");

    let data = vec![0xABu8; 1024];

    group.bench_function("file_backed_write", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("baseline.db");
        let mut storage = Storage::open(&db_path).unwrap();

        storage
            .batch_write(&[Binding::new(
                "/baseline",
                Lock::new(1, 1),
                make_slots(&data),
            )])
            .unwrap();

        let mut version = 2u64;
        b.iter(|| {
            storage
                .batch_write(&[Binding::new(
                    "/baseline",
                    Lock::new(version, 1),
                    make_slots(&data),
                )])
                .unwrap();
            version += 1;
        });
    });

    group.bench_function("file_backed_read", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("baseline_read.db");
        let mut storage = Storage::open(&db_path).unwrap();

        storage
            .batch_write(&[Binding::new(
                "/baseline_read",
                Lock::new(1, 1),
                make_slots(&data),
            )])
            .unwrap();

        b.iter(|| {
            let result = storage.read_x("/baseline_read", None).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

// =============================================================================
// Storage Instance Creation Overhead
// =============================================================================

/// Benchmark the cost of opening a Storage instance
fn bench_storage_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_open");

    // In-memory (fresh)
    group.bench_function("in_memory_fresh", |b| {
        b.iter(|| {
            let storage = Storage::open_in_memory().unwrap();
            black_box(storage);
        });
    });

    // File-backed (fresh)
    group.bench_function("file_backed_fresh", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("open_test.db");
            let storage = Storage::open(&db_path).unwrap();
            black_box(storage);
        });
    });

    // File-backed (existing small DB)
    group.bench_function("file_backed_existing_small", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("existing_small.db");

        // Create DB with some data
        {
            let mut storage = Storage::open(&db_path).unwrap();
            let data = vec![0xABu8; 1024];
            for i in 0..100 {
                storage
                    .batch_write(&[Binding::new(
                        format!("/path_{}", i),
                        Lock::new(1, 1),
                        make_slots(&data),
                    )])
                    .unwrap();
            }
        }

        b.iter(|| {
            let storage = Storage::open(&db_path).unwrap();
            black_box(storage);
        });
    });

    // File-backed (existing larger DB)
    group.bench_function("file_backed_existing_large", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("existing_large.db");

        // Create DB with more data
        {
            let mut storage = Storage::open(&db_path).unwrap();
            let data = vec![0xABu8; 1024];
            let batch_size = 500;
            for batch_start in (0..5000).step_by(batch_size) {
                let batch_end = batch_start + batch_size;
                let bindings: Vec<_> = (batch_start..batch_end)
                    .map(|i| {
                        Binding::new(
                            format!("/large/path_{}", i),
                            Lock::new(1, 1),
                            make_slots(&data),
                        )
                    })
                    .collect();
                storage.batch_write(&bindings).unwrap();
            }
        }

        b.iter(|| {
            let storage = Storage::open(&db_path).unwrap();
            black_box(storage);
        });
    });

    group.finish();
}

// =============================================================================
// Throughput Under Contention
// =============================================================================

/// Measure total throughput (reads + writes) under various contention scenarios
fn bench_throughput_under_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_under_contention");
    group.sample_size(10);

    let data = vec![0xABu8; 1024];
    let operations_per_thread = 100;

    // All readers
    group.bench_function("4_threads_all_readers", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("throughput.db");

        // Setup
        {
            let mut storage = Storage::open(&db_path).unwrap();
            for i in 0..100 {
                storage
                    .batch_write(&[Binding::new(
                        format!("/tp_{}", i),
                        Lock::new(1, 1),
                        make_slots(&data),
                    )])
                    .unwrap();
            }
        }

        b.iter(|| {
            let barrier = Arc::new(Barrier::new(4));
            let handles: Vec<_> = (0..4)
                .map(|thread_id| {
                    let db_path = db_path.clone();
                    let barrier = Arc::clone(&barrier);

                    thread::spawn(move || {
                        let storage = Storage::open(&db_path).unwrap();
                        barrier.wait();

                        for i in 0..operations_per_thread {
                            let path_idx = (thread_id * 17 + i) % 100;
                            let result =
                                storage.read_x(&format!("/tp_{}", path_idx), None).unwrap();
                            black_box(result);
                        }
                    })
                })
                .collect();

            for h in handles {
                h.join().unwrap();
            }
        });
    });

    // 3 readers, 1 writer (each to separate paths)
    group.bench_function("3_readers_1_writer", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("throughput2.db");

        // Setup
        {
            let mut storage = Storage::open(&db_path).unwrap();
            for i in 0..100 {
                storage
                    .batch_write(&[Binding::new(
                        format!("/tp2_{}", i),
                        Lock::new(1, 1),
                        make_slots(&data),
                    )])
                    .unwrap();
            }
            // Writer's path
            storage
                .batch_write(&[Binding::new(
                    "/writer_path",
                    Lock::new(1, 1),
                    make_slots(&data),
                )])
                .unwrap();
        }

        b.iter(|| {
            let barrier = Arc::new(Barrier::new(4));

            // 3 reader threads
            let reader_handles: Vec<_> = (0..3)
                .map(|thread_id| {
                    let db_path = db_path.clone();
                    let barrier = Arc::clone(&barrier);

                    thread::spawn(move || {
                        let storage = Storage::open(&db_path).unwrap();
                        barrier.wait();

                        for i in 0..operations_per_thread {
                            let path_idx = (thread_id * 17 + i) % 100;
                            let result =
                                storage.read_x(&format!("/tp2_{}", path_idx), None).unwrap();
                            black_box(result);
                        }
                    })
                })
                .collect();

            // 1 writer thread
            let writer_handle = {
                let db_path = db_path.clone();
                let data = data.clone();
                let barrier = Arc::clone(&barrier);

                thread::spawn(move || {
                    let mut storage = Storage::open(&db_path).unwrap();
                    let current = storage
                        .get_current_lock("/writer_path", shrine_storage::Care::X)
                        .unwrap()
                        .unwrap();
                    let mut version = current.data + 1;

                    barrier.wait();

                    for _ in 0..operations_per_thread {
                        storage
                            .batch_write(&[Binding::new(
                                "/writer_path",
                                Lock::new(version, current.shape),
                                make_slots(&data),
                            )])
                            .unwrap();
                        version += 1;
                    }
                })
            };

            for h in reader_handles {
                h.join().unwrap();
            }
            writer_handle.join().unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_multi_instance_writes,
    bench_multi_instance_reads,
    bench_reader_writer_contention,
    bench_single_instance_baseline,
    bench_storage_open,
    bench_throughput_under_contention,
);
criterion_main!(benches);
