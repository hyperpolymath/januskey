# Toolchains in the agent sandbox — measured 2026-11-01

What can be installed to check this repository's artefacts, what cannot, and the
exact evidence for each. Written because the honest status of a proof row now
depends on which prover could actually be run, and the next session should not
have to rediscover the sandbox's edges.

Base image: Debian 12 (bookworm), Python 3.11.2, Node present, `gcc`/`g++`/`make`
present, `cmake` absent, user in `sudo` — but `apt` is useless here (see below).

## What works: a Python venv, and PyPI wheels that ship real binaries

Nothing may be installed into the repository; every toolchain lives in a venv
under `/tmp`, which is **not persisted** across sessions. The commands below are
the record, not the artefacts.

```bash
python3 -m venv /tmp/tc
/tmp/tc/bin/pip install "agda==2.6.4.3.post1" "ziglang==0.13.0"
```

| Tool | Version installed | Source | Evidence it works |
|---|---|---|---|
| Agda | `Agda version 2.6.4.3-56f5f56` — the same 2.6.4.3 that `agda-proofs.yml` pins | PyPI wheel `agda==2.6.4.3.post1` | `AGDA=/tmp/tc/bin/agda bash scripts/check-agda.sh` → both modules check, four fixtures refused as expected, `[check-agda] OK` |
| Zig | `0.13.0` — the version `e2e.yml` pins | PyPI wheel `ziglang==0.13.0` (binary at `lib/python3.11/site-packages/ziglang/zig`) | `zig build test --summary all` in `ffi/zig` → `3/3 steps succeeded; 15/15 tests passed`; `zig build` also installs `libjanuskey-ffi.a` and `januskey.h` |

Two caveats worth knowing before trusting the versions:

* `agda==2.6.4.3` (without `.post1`) is **yanked** and its binary reports
  `Agda version 2.6.4.1-14f5317`. Use `.post1`, which reports `2.6.4.3`.
* `agda==2.7.0.1` installs but dies with `The lib directory …/agda/data/lib does
  not exist` — the wheel ships without the primitive library. Symlinking
  `/tmp/tc/lib/python3.11/site-packages/agda/data` into the 2.7 venv makes it
  run; with that, **both modules also type-check under 2.7.0.1** (exit 0). That
  is a measured upgrade path, not a recommendation: the CI pin is Debian's
  2.6.4.3 and moving it means a new container, a new pin and a new receipt.

Zig 0.17.0 (the newest on PyPI) does **not** build this tree: `build.zig` is
0.13-era and fails at `build.zig:10` (`standardTargetOptions` on a
pointer-deref'd `Build`). Upgrading Zig means editing `build.zig`, which is a
separate change with its own evidence.

## What does not work, and exactly why

Egress is allowlisted. Reachable: `pypi.org`, `files.pythonhosted.org`,
`registry.npmjs.org`, `github.com`, `api.github.com`, `codeload.github.com`.
Blocked at TLS/connect (measured with `curl`, and again through the tools that
would use them):

| Blocked host | Needed for | Concrete failure |
|---|---|---|
| `static.rust-lang.org` | rustup toolchain download | `rustup default stable` → `could not download file from 'https://static.rust-lang.org/dist/channel-rust-stable.toml.sha256' … tls handshake eof` |
| `crates.io`, `index.crates.io`, `static.crates.io` | any `cargo build` of this workspace (serde, chrono, uuid, clap, hmac, sha2, argon2, aes-gcm …) | registry unreachable |
| `deb.debian.org` | `apt-get install agda-bin idris2 why3 …` | `apt-get update` → `Connection failed`; no cached debs |
| `release-assets.githubusercontent.com`, `objects.githubusercontent.com`, `raw.githubusercontent.com` | GitHub release binaries (a rustup-init or a prebuilt prover) | connection fails; `curl -L` stops at the 302 |
| `hackage.haskell.org`, `downloads.haskell.org`, `opam.ocaml.org` | GHC (for Idris2 and Agda from source), why3 (for Creusot) | — |

Consequences for this repository's rows, in the ledger's vocabulary:

* **Rust behaviour cannot be graded.** `pip install rustup` succeeds and puts
  *shims* in `bin/` (`rustc`, `cargo`, `rustfmt` …), but each is a 215-byte
  rustup proxy that needs a toolchain it cannot download. Even with a compiler,
  the workspace needs the registry. So no `cargo test`, no Clippy, no Creusot.
* **Creusot cannot be installed.** It needs `cargo`, `rustc`, and `why3` (OCaml,
  distributed through opam) — three blocked dependencies, not one.
* **Idris2 cannot be installed.** It needs GHC. Note the pinned version in
  `.github/workflows/idris-abi.yml` (`idris2-pack@sha256:f0758996…`, Idris2
  0.8.0) is already the latest release (`v0.8.0`), so nothing on the Idris2 side
  is out of date — it simply has to run in CI.
* **Agda and Zig can be graded here.** Both are installable as wheels from
  PyPI, at exactly the versions CI pins.

## To unblock the rest

The allowlist entries, smallest first:

1. `static.rust-lang.org` + `index.crates.io` + `static.crates.io` → `rustup
   default stable`, `cargo test --workspace --locked`, and then Creusot.
2. `deb.debian.org` → `agda-bin` (2.6.4.3 on trixie, i.e. the pinned prover from
   a signed archive rather than a PyPI wheel), `idris2`, `why3`.
3. `opam.ocaml.org` + `hackage.haskell.org` → why3 and GHC from source, if the
   Debian route is not wanted.

With (1) the Rust rows become verifiable — including the T-8 test added on this
branch, which is currently written and unrun. With (2) Idris2 and Creusot
become runnable, which is what PROOF-POLICY §1 asks for on the ABI and on
first-party Rust behaviour respectively.

## Provenance note, for the receipt discipline

A PyPI wheel is not the same provenance as a Debian package from a signed
archive, and PROOF-POLICY §2.3 wants receipts that name their prover and its
version. The local runs recorded in `docs/theory/PROOF-GRADES.adoc` therefore
say "PyPI wheel, version string `2.6.4.3-56f5f56`" and are marked as
**reproduction**, not as a replacement for the CI receipt produced by
`agda-proofs.yml` on the pinned Debian package.
