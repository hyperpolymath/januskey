// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//
// Crypto-shredding experiment for ADR-0002 (proposed). Nothing here is used by
// `jk`: it measures the real code where it exists and a prototype where it
// does not, so the design is argued from numbers.
//
//   store:  ContentStore::store with compression off and with it on (the
//           `jk` default), against the proposed path: gzip, then an
//           AES-256-GCM seal under a fresh per-blob key, then the same
//           temp/fsync/rename write for the blob, plus one synced append of
//           its key to the key journal (the layout ADR-0002 §2 chooses).
//           `seal_only` is the cipher with no I/O.
//   erase:  obliterate_file on a blob (3-pass overwrite, unlink: the current
//           path) against crypto-shredding, in its two stages:
//             pending: drop the blob's key file (through the same
//                      obliterate_file); copies retained elsewhere are still
//                      wrapped under the live epoch key;
//             final:   drop the entry and re-seal every live key under a
//                      fresh epoch key, either as one table (one synced
//                      write) or as one file per key (one synced write each).
//
// Payloads are random, so incompressible: the worst case for gzip, which then
// costs time and saves nothing. Each is made unique per iteration, so dedupe
// never short-circuits a write. fsync cost depends on the device: numbers
// from a container's virtual disk say nothing about an SSD's flash
// translation layer.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use criterion::{
    black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput,
};
use flate2::{write::GzEncoder, Compression};
use januskey::obliteration::obliterate_file;
use januskey::{ContentHash, ContentStore};
use rand::RngCore;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const SIZES: [usize; 4] = [4 << 10, 64 << 10, 1 << 20, 16 << 20];
/// A key-table entry: 32-byte content address, then the blob key wrapped
/// under the epoch key (12-byte nonce, 32-byte key, 16-byte tag).
const ENTRY: usize = 32 + 12 + 32 + 16;

/// `size` random bytes: incompressible, the worst case for gzip.
fn random_payload(size: usize) -> Vec<u8> {
    let mut data = vec![0u8; size];
    rand::rng().fill_bytes(&mut data);
    data
}

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

/// gzip at the store's level, as `ContentStore::write_blob` does.
fn gzip(data: &[u8]) -> Vec<u8> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).unwrap();
    enc.finish().unwrap()
}

fn bench_store(c: &mut Criterion) {
    let mut g = c.benchmark_group("crypto_shred/store");
    g.sample_size(10).measurement_time(Duration::from_secs(3));
    for size in SIZES {
        g.throughput(Throughput::Bytes(size as u64));
        for (name, compression) in [("plaintext_raw", false), ("plaintext_default", true)] {
            g.bench_with_input(BenchmarkId::new(name, size), &size, |b, &size| {
                let dir = tempfile::tempdir().unwrap();
                let store = ContentStore::new(dir.path().to_path_buf(), compression).unwrap();
                let (mut data, mut n) = (random_payload(size), 0u64);
                b.iter(|| {
                    make_unique(&mut data, &mut n);
                    black_box(store.store(&data).unwrap());
                });
            });
        }
        g.bench_with_input(
            BenchmarkId::new("sealed_default", size),
            &size,
            |b, &size| {
                let dir = tempfile::tempdir().unwrap();
                let mut journal = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.path().join("keys.journal"))
                    .unwrap();
                let (mut data, mut n) = (random_payload(size), 0u64);
                b.iter(|| {
                    make_unique(&mut data, &mut n);
                    let hash = ContentHash::from_bytes(&data);
                    // Compress before sealing: ciphertext does not compress.
                    let (key, sealed) = seal(&gzip(&data));
                    write_synced(dir.path(), hash.raw_hash(), &sealed);
                    // The key must be durable too. A store appends one entry
                    // to the key journal and syncs it; the sealed table is
                    // rewritten only by a shred, which rewrites it anyway.
                    // Wrapping the entry adds microseconds (E5).
                    let mut entry = [0u8; ENTRY];
                    entry[..32].copy_from_slice(&key);
                    journal.write_all(&entry).unwrap();
                    journal.sync_all().unwrap();
                });
            },
        );
        g.bench_with_input(BenchmarkId::new("seal_only", size), &size, |b, &size| {
            let (mut data, mut n) = (random_payload(size), 0u64);
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
                let data = random_payload(size);
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

    // Pending shred: drop one key file, the same overwrite on 32 bytes.
    g.bench_function("pending_drop_key_file", |b| {
        let dir = tempfile::tempdir().unwrap();
        b.iter_batched(
            || {
                let p = dir.path().join("key");
                fs::write(&p, random_payload(32)).unwrap();
                p
            },
            |p| black_box(obliterate_file(&p).unwrap()),
            BatchSize::PerIteration,
        );
    });

    // Final shred, one table: drop the entry, re-seal the whole table under a
    // fresh epoch key, one synced write. (Destroying the old epoch secret is
    // not measured: where it lives is the open question of ADR §4.)
    for entries in [1_000usize, 10_000, 100_000] {
        g.bench_with_input(
            BenchmarkId::new("final_reseal_table", entries),
            &entries,
            |b, &entries| {
                let dir = tempfile::tempdir().unwrap();
                let table = random_payload(entries * ENTRY);
                let mut victim = 0usize;
                b.iter(|| {
                    victim = (victim + 1) % entries;
                    let mut next = Vec::with_capacity(table.len() - ENTRY);
                    next.extend_from_slice(&table[..victim * ENTRY]);
                    next.extend_from_slice(&table[(victim + 1) * ENTRY..]);
                    let (epoch_key, sealed) = seal(&next);
                    write_synced(dir.path(), "keys", &sealed);
                    black_box(epoch_key);
                });
            },
        );
    }

    // Final shred, one file per key: rewrap every live key file under a fresh
    // epoch key, one synced write each.
    let entries = 1_000usize;
    g.bench_with_input(
        BenchmarkId::new("final_rewrap_files", entries),
        &entries,
        |b, &entries| {
            let dir = tempfile::tempdir().unwrap();
            let names: Vec<String> = (0..entries).map(|i| format!("{i:08}.key")).collect();
            b.iter(|| {
                let mut epoch_key = [0u8; 32];
                rand::rng().fill_bytes(&mut epoch_key);
                let cipher = Aes256Gcm::new_from_slice(&epoch_key).unwrap();
                for name in &names {
                    let mut nonce = [0u8; 12];
                    rand::rng().fill_bytes(&mut nonce);
                    let blob_key = random_payload(32);
                    let wrapped = cipher
                        .encrypt(Nonce::from_slice(&nonce), blob_key.as_slice())
                        .unwrap();
                    write_synced(dir.path(), name, &wrapped);
                }
                black_box(epoch_key);
            });
        },
    );
    g.finish();
}

criterion_group!(benches, bench_store, bench_erase);
criterion_main!(benches);
