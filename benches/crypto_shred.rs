// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//
// Crypto-shredding experiment for ADR-0002 (proposed). Nothing here is used by
// `jk`: it measures the real code where it exists and a prototype where it
// does not, so the design is argued from numbers.
//
//   store:  ContentStore::store (plaintext, temp/fsync/rename) against an
//           AES-256-GCM seal under a fresh per-blob key followed by the same
//           temp/fsync/rename write for the blob and again for its key file;
//           `seal_only` is the cipher with no I/O.
//   erase:  obliterate_file on a blob (3-pass overwrite, unlink: the current
//           path) against dropping the blob's key, either as one small key
//           file (through the same obliterate_file) or as one entry removed
//           from a key table that is rewritten atomically.
//
// Every stored payload is made unique per iteration, so dedupe never short-
// circuits a write. fsync cost depends on the device: numbers from a
// container's virtual disk say nothing about an SSD's flash translation layer.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use januskey::obliteration::obliterate_file;
use januskey::{ContentHash, ContentStore};
use rand::RngCore;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const SIZES: [usize; 4] = [4 << 10, 64 << 10, 1 << 20, 16 << 20];
/// A table entry: 32-byte content address and 32-byte blob key.
const ENTRY: usize = 64;

/// Make `data` differ from every earlier payload, so the store cannot dedupe.
fn make_unique(data: &mut [u8], n: &mut u64) {
    *n += 1;
    data[..8].copy_from_slice(&n.to_le_bytes());
}

/// Write `bytes` to `dir/name` the way the store does: temporary file,
/// fsync, rename, then fsync the directory.
fn write_synced(dir: &Path, name: &str, bytes: &[u8]) {
    let tmp = dir.join(format!(".tmp-{name}"));
    let mut f = fs::File::create(&tmp).unwrap();
    f.write_all(bytes).unwrap();
    f.sync_all().unwrap();
    fs::rename(&tmp, dir.join(name)).unwrap();
    fs::File::open(dir).unwrap().sync_all().unwrap();
}

/// Seal `data` under a fresh random key; returns (key, nonce || ciphertext).
fn seal(data: &[u8]) -> ([u8; 32], Vec<u8>) {
    let mut key = [0u8; 32];
    let mut nonce = [0u8; 12];
    rand::rng().fill_bytes(&mut key);
    rand::rng().fill_bytes(&mut nonce);
    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
    let ct = cipher.encrypt(Nonce::from_slice(&nonce), data).unwrap();
    let mut out = Vec::with_capacity(12 + ct.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    (key, out)
}

fn bench_store(c: &mut Criterion) {
    let mut g = c.benchmark_group("crypto_shred/store");
    g.sample_size(10).measurement_time(Duration::from_secs(3));
    for size in SIZES {
        g.throughput(Throughput::Bytes(size as u64));
        g.bench_with_input(BenchmarkId::new("plaintext", size), &size, |b, &size| {
            let dir = tempfile::tempdir().unwrap();
            let store = ContentStore::new(dir.path().to_path_buf(), false).unwrap();
            let (mut data, mut n) = (vec![0xCDu8; size], 0u64);
            b.iter(|| {
                make_unique(&mut data, &mut n);
                black_box(store.store(&data).unwrap());
            });
        });
        g.bench_with_input(BenchmarkId::new("sealed", size), &size, |b, &size| {
            let dir = tempfile::tempdir().unwrap();
            let (mut data, mut n) = (vec![0xCDu8; size], 0u64);
            b.iter(|| {
                make_unique(&mut data, &mut n);
                let hash = ContentHash::from_bytes(&data);
                let (key, sealed) = seal(&data);
                write_synced(dir.path(), hash.raw_hash(), &sealed);
                // The key must be durable too: one more synced write. It is
                // stored unwrapped here; wrapping adds microseconds (E5).
                write_synced(dir.path(), &format!("{}.key", hash.raw_hash()), &key);
            });
        });
        g.bench_with_input(BenchmarkId::new("seal_only", size), &size, |b, &size| {
            let (mut data, mut n) = (vec![0xCDu8; size], 0u64);
            b.iter(|| {
                make_unique(&mut data, &mut n);
                black_box(seal(&data));
            });
        });
    }
    g.finish();
}

fn bench_erase(c: &mut Criterion) {
    let mut g = c.benchmark_group("crypto_shred/erase");
    g.sample_size(10).measurement_time(Duration::from_secs(3));

    // The current path: overwrite the blob itself, whatever its size.
    for size in SIZES {
        g.bench_with_input(
            BenchmarkId::new("overwrite_blob", size),
            &size,
            |b, &size| {
                let dir = tempfile::tempdir().unwrap();
                let data = vec![0xEFu8; size];
                b.iter_batched(
                    || {
                        let p = dir.path().join("blob");
                        fs::write(&p, &data).unwrap();
                        p
                    },
                    |p| black_box(obliterate_file(&p).unwrap()),
                    BatchSize::PerIteration,
                );
            },
        );
    }

    // Crypto-shred, one key file per blob: the same overwrite, on 32 bytes.
    g.bench_function("drop_key_file", |b| {
        let dir = tempfile::tempdir().unwrap();
        b.iter_batched(
            || {
                let p = dir.path().join("key");
                let mut key = [0u8; 32];
                rand::rng().fill_bytes(&mut key);
                fs::write(&p, key).unwrap();
                p
            },
            |p| black_box(obliterate_file(&p).unwrap()),
            BatchSize::PerIteration,
        );
    });

    // Crypto-shred, one key table: remove an entry, rewrite the table.
    for entries in [1_000usize, 10_000, 100_000] {
        g.bench_with_input(
            BenchmarkId::new("drop_key_table", entries),
            &entries,
            |b, &entries| {
                let dir = tempfile::tempdir().unwrap();
                let mut table = vec![0u8; entries * ENTRY];
                rand::rng().fill_bytes(&mut table);
                let mut victim = 0usize;
                b.iter(|| {
                    victim = (victim + 1) % entries;
                    let mut next = Vec::with_capacity(table.len() - ENTRY);
                    next.extend_from_slice(&table[..victim * ENTRY]);
                    next.extend_from_slice(&table[(victim + 1) * ENTRY..]);
                    write_synced(dir.path(), "keys", &next);
                    black_box(next.len());
                });
            },
        );
    }
    g.finish();
}

criterion_group!(benches, bench_store, bench_erase);
criterion_main!(benches);
