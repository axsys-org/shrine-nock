//! Write throughput benchmarks for shrine-storage.
//!
//! Tests:
//! - Single-path sequential writes with varying payload sizes
//! - Batch write performance with varying batch sizes
//! - Comparison of batch_write() vs apply() APIs
//! - Large blob writes (blob store performance)

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use shrine_storage::{Binding, Lock, Note, Pail, Storage};
use std::collections::HashMap;
use tempfile::TempDir;

/// Helper to create slots with a single data payload
fn make_slots(data: &[u8]) -> HashMap<String, Pail> {
    HashMap::from([(
        "data".to_string(),
        Pail::new("/types/binary", data.to_vec()),
    )])
}

/// Helper to create a typed tale for apply() API
fn make_tale(data: &[u8]) -> HashMap<String, Pail> {
    HashMap::from([(
        "data".to_string(),
        Pail::new("/types/binary", data.to_vec()),
    )])
}

// =============================================================================
// Single-path Sequential Writes
// =============================================================================

/// Benchmark sequential writes to a single path with varying payload sizes.
/// Each iteration writes a new version.
fn bench_single_path_sequential_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_path_sequential");

    // Test payload sizes: 1KB, 10KB, 100KB (blob threshold), 500KB (uses blob store)
    for size in [1024, 10 * 1024, 100 * 1024, 500 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size / 1024)),
            size,
            |b, &size| {
                let mut storage = Storage::open_in_memory().unwrap();
                let data = vec![0xABu8; size];

                // Initialize the path
                storage
                    .batch_write(&[Binding::new("/bench", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/bench",
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

/// Benchmark writes with file-backed storage (includes blob store for large payloads)
fn bench_single_path_file_backed(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_path_file_backed");

    for size in [1024, 100 * 1024, 500 * 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}KB", size / 1024)),
            size,
            |b, &size| {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("bench.db");
                let mut storage = Storage::open(&db_path).unwrap();
                let data = vec![0xABu8; size];

                // Initialize the path
                storage
                    .batch_write(&[Binding::new("/bench", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/bench",
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
// Batch Write Performance
// =============================================================================

/// Benchmark batch writes with varying batch sizes.
/// All paths in a batch are siblings under /batch/.
fn bench_batch_write_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_write_sizes");

    let data = vec![0xABu8; 1024]; // 1KB payload

    for batch_size in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                let mut storage = Storage::open_in_memory().unwrap();

                // Pre-create all paths so we're measuring update performance
                let bindings: Vec<_> = (0..batch_size)
                    .map(|i| {
                        Binding::new(
                            format!("/batch/path_{}", i),
                            Lock::new(1, 1),
                            make_slots(&data),
                        )
                    })
                    .collect();
                storage.batch_write(&bindings).unwrap();

                let mut version = 2u64;
                b.iter(|| {
                    let bindings: Vec<_> = (0..batch_size)
                        .map(|i| {
                            Binding::new(
                                format!("/batch/path_{}", i),
                                Lock::new(version, 1),
                                make_slots(&data),
                            )
                        })
                        .collect();
                    storage.batch_write(&bindings).unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

/// Benchmark batch writes creating new paths each time (shape changes).
fn bench_batch_write_creates(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_write_creates");

    let data = vec![0xABu8; 1024];

    for batch_size in [1, 10, 100].iter() {
        group.throughput(Throughput::Elements(*batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            batch_size,
            |b, &batch_size| {
                let mut storage = Storage::open_in_memory().unwrap();
                let mut batch_num = 0u64;

                b.iter(|| {
                    let bindings: Vec<_> = (0..batch_size)
                        .map(|i| {
                            Binding::new(
                                format!("/create/batch_{}/path_{}", batch_num, i),
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

// =============================================================================
// batch_write() vs apply() API Comparison
// =============================================================================

/// Compare batch_write() (explicit versions) vs apply() (intent-based) APIs
fn bench_api_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_comparison");

    let data = vec![0xABu8; 1024];

    // batch_write API - updates
    group.bench_function("batch_write_update", |b| {
        let mut storage = Storage::open_in_memory().unwrap();
        storage
            .batch_write(&[Binding::new(
                "/api/test",
                Lock::new(1, 1),
                make_slots(&data),
            )])
            .unwrap();

        let mut version = 2u64;
        b.iter(|| {
            storage
                .batch_write(&[Binding::new(
                    "/api/test",
                    Lock::new(version, 1),
                    make_slots(&data),
                )])
                .unwrap();
            version += 1;
        });
    });

    // apply API with Poke (update, no shape change)
    group.bench_function("apply_poke", |b| {
        let mut storage = Storage::open_in_memory().unwrap();
        storage
            .apply(vec![(
                "/api/test".to_string(),
                Note::Make(make_tale(&data)),
            )])
            .unwrap();

        b.iter(|| {
            storage
                .apply(vec![(
                    "/api/test".to_string(),
                    Note::Poke(make_tale(&data)),
                )])
                .unwrap();
        });
    });

    // apply API with Make (shape change)
    group.bench_function("apply_make", |b| {
        let mut storage = Storage::open_in_memory().unwrap();
        let mut path_num = 0u64;

        b.iter(|| {
            storage
                .apply(vec![(
                    format!("/api/make_{}", path_num),
                    Note::Make(make_tale(&data)),
                )])
                .unwrap();
            path_num += 1;
        });
    });

    group.finish();
}

// =============================================================================
// Blob Store Performance (large payloads)
// =============================================================================

/// Benchmark large payload writes that trigger blob store
fn bench_large_payloads(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_payloads");
    group.sample_size(20); // Fewer samples for large payloads

    // Test sizes above blob threshold (100KB)
    for size_mb in [1, 10].iter() {
        let size = size_mb * 1024 * 1024;
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", size_mb)),
            &size,
            |b, &size| {
                let temp_dir = TempDir::new().unwrap();
                let db_path = temp_dir.path().join("bench.db");
                let mut storage = Storage::open(&db_path).unwrap();
                let data = vec![0xABu8; size];

                storage
                    .batch_write(&[Binding::new("/large", Lock::new(1, 1), make_slots(&data))])
                    .unwrap();

                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/large",
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

/// Benchmark blob deduplication (writing same content multiple times)
fn bench_blob_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("blob_deduplication");
    group.sample_size(20);

    let size = 1024 * 1024; // 1MB
    let data = vec![0xABu8; size]; // Same content each time

    group.bench_function("deduplicated_1MB", |b| {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("bench.db");
        let mut storage = Storage::open(&db_path).unwrap();

        let mut path_num = 0u64;
        b.iter(|| {
            // Write same content to different paths - should deduplicate in blob store
            storage
                .batch_write(&[Binding::new(
                    format!("/dedup/path_{}", path_num),
                    Lock::new(1, 1),
                    make_slots(&data),
                )])
                .unwrap();
            path_num += 1;
        });
    });

    group.finish();
}

// =============================================================================
// Multi-slot Writes
// =============================================================================

/// Benchmark writes with multiple slots per binding
fn bench_multi_slot_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_slot_writes");

    let slot_data = vec![0xABu8; 1024]; // 1KB per slot

    for num_slots in [1, 5, 10, 20].iter() {
        group.throughput(Throughput::Bytes((*num_slots * 1024) as u64));
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
                    .batch_write(&[Binding::new("/multi", Lock::new(1, 1), slots.clone())])
                    .unwrap();

                let mut version = 2u64;
                b.iter(|| {
                    storage
                        .batch_write(&[Binding::new(
                            "/multi",
                            Lock::new(version, 1),
                            slots.clone(),
                        )])
                        .unwrap();
                    version += 1;
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_single_path_sequential_writes,
    bench_single_path_file_backed,
    bench_batch_write_sizes,
    bench_batch_write_creates,
    bench_api_comparison,
    bench_large_payloads,
    bench_blob_deduplication,
    bench_multi_slot_writes,
);
criterion_main!(benches);
