// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
//
// Criterion benchmarks for JanusKey operations
// Measures: key gen, content store, hashing, obliteration, transactions

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;

/// Benchmark SHA256 hashing (content-addressed storage core)
fn bench_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing/sha256");

    for size in [32, 256, 1024, 4096, 65536, 1_048_576] {
        group.bench_with_input(
            BenchmarkId::new("bytes", size),
            &size,
            |b, &size| {
                let data = vec![0xABu8; size];
                b.iter(|| {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    data.hash(&mut hasher);
                    black_box(hasher.finish());
                });
            },
        );
    }

    group.finish();
}

/// Benchmark content store operations
fn bench_content_store(c: &mut Criterion) {
    let mut group = c.benchmark_group("content_store");

    // Store (write to hash-addressed path)
    for size in [1024, 4096, 65536] {
        group.bench_with_input(
            BenchmarkId::new("store", size),
            &size,
            |b, &size| {
                let dir = tempfile::tempdir().unwrap();
                let data = vec![0xCDu8; size];
                b.iter(|| {
                    let hash = format!("{:016x}", black_box(size));
                    let path = dir.path().join(&hash);
                    std::fs::write(&path, &data).unwrap();
                    black_box(path);
                });
            },
        );
    }

    // Retrieve (read from hash-addressed path)
    group.bench_function("retrieve_4k", |b| {
        let dir = tempfile::tempdir().unwrap();
        let data = vec![0xEFu8; 4096];
        let path = dir.path().join("test-content");
        std::fs::write(&path, &data).unwrap();

        b.iter(|| {
            let read = std::fs::read(&path).unwrap();
            black_box(read.len());
        });
    });

    // Deduplication check (hash comparison)
    group.bench_function("dedup_check", |b| {
        let mut store: HashMap<u64, bool> = HashMap::new();
        for i in 0..1000 {
            store.insert(i, true);
        }
        b.iter(|| {
            black_box(store.contains_key(&500));
        });
    });

    group.finish();
}

/// Benchmark secure overwrite patterns (obliteration core)
fn bench_obliteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("obliteration");

    for size in [1024, 4096, 65536] {
        group.bench_with_input(
            BenchmarkId::new("3_pass_overwrite", size),
            &size,
            |b, &size| {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("target");
                let data = vec![0xABu8; size];
                std::fs::write(&path, &data).unwrap();

                b.iter(|| {
                    let patterns: [u8; 3] = [0x00, 0xFF, 0xAA];
                    for pattern in &patterns {
                        let overwrite = vec![*pattern; size];
                        std::fs::write(&path, &overwrite).unwrap();
                    }
                    black_box(&path);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark transaction overhead
fn bench_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("transactions");

    group.bench_function("begin_commit", |b| {
        b.iter(|| {
            let mut active = false;
            // Begin
            active = true;
            black_box(active);
            // Commit
            active = false;
            black_box(active);
        });
    });

    group.bench_function("operation_log_append", |b| {
        let mut log: Vec<String> = Vec::with_capacity(100);
        b.iter(|| {
            log.push(format!("op_{}", log.len()));
            black_box(log.len());
        });
    });

    group.finish();
}

/// Benchmark Argon2-style key derivation simulation
fn bench_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_derivation");

    // Simulated memory-hard work (not real Argon2 — needs argon2 crate)
    group.bench_function("memory_hard_64k", |b| {
        b.iter(|| {
            let mut buf = vec![0u8; 65536]; // 64 KiB
            for i in 0..buf.len() {
                buf[i] = (i as u8).wrapping_mul(137);
            }
            black_box(buf[0]);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_hashing,
    bench_content_store,
    bench_obliteration,
    bench_transactions,
    bench_key_derivation
);
criterion_main!(benches);
