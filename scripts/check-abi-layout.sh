#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
# Idris2 ABI <-> C header <-> Zig agreement, mechanically, without a compiler:
#   - struct sizes: Layout.idr StructSize vs ffi/zig/src/main.zig @sizeOf asserts
#   - status codes: Foreign.idr statusCode table vs januskey.h JK_* #defines
#   - op kinds / algorithms: Foreign.idr code tables vs januskey.h enum values
# Any drift between the three fails the gate.
set -euo pipefail
cd "$(dirname "$0")/.."
F=src/abi/JanusKey/ABI/Foreign.idr
L=src/abi/JanusKey/ABI/Layout.idr
H=ffi/zig/include/januskey.h
Z=ffi/zig/src/main.zig
fail=0
say() { echo "[check-abi-layout] $*"; }
same() { # name, expected-list, actual-list
  if [ "$2" = "$3" ]; then say "OK   $1: $2"; else say "FAIL $1: Idris [$2] vs C/Zig [$3]"; fail=1; fi
}

# 1. Struct sizes (name order: content hash, key id, obliteration proof)
idris_sizes=$(grep -oE '^(contentHashSize|keyIdSize|oblitProofSize) : StructSize [0-9]+' "$L" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
zig_sizes=$(grep -oE '@sizeOf\((ContentHash|KeyId|OblitProof)\) == [0-9]+' "$Z" | sort -u | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
same "struct sizes (hash, keyid, oblit)" "$idris_sizes" "$zig_sizes"
hdr_oblit=$(grep -oE 'jk_oblit_proof_t;  /\* [0-9]+ bytes' "$H" | grep -oE '[0-9]+' | head -1)
same "oblit proof size vs header comment" "$(echo "$idris_sizes" | awk '{print $3}')" "$hdr_oblit"

# 2. Status codes, in table order
idris_status=$(grep -oE '^statusCode [A-Za-z]+ += [0-9]+' "$F" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
hdr_status=$(grep -oE '^#define JK_(OK|ERR_[A-Z_]+) +[0-9]+' "$H" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
same "status codes" "$idris_status" "$hdr_status"
n_idris=$(grep -cE '^  (Ok|Err[A-Za-z]+) +: Status' "$F"); n_hdr=$(grep -cE '^#define JK_(OK|ERR_)' "$H")
same "status count" "$n_idris" "$n_hdr"

# 3. Op kinds and algorithms, in enum order
idris_ops=$(grep -oE '^opKindCode [A-Za-z]+ += [0-9]+' "$F" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
hdr_ops=$(grep -oE 'JK_OP_[A-Z_]+ += [0-9]+' "$H" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
same "op kinds" "$idris_ops" "$hdr_ops"
idris_alg=$(grep -oE '^algorithmCode [A-Za-z0-9]+ += [0-9]+' "$F" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
hdr_alg=$(grep -oE 'JK_ALGO_[A-Z0-9_]+ += [0-9]+' "$H" | awk '{print $NF}' | tr '\n' ' ' | sed 's/ $//')
same "algorithms" "$idris_alg" "$hdr_alg"

# 4. Every exported jk_* symbol in the header has a %foreign declaration, and vice versa
hdr_syms=$(grep -oE '\bjk_[a-z_]+\(' "$H" | tr -d '(' | sort -u | tr '\n' ' ' | sed 's/ $//')
idris_syms=$(grep -oE '%foreign "C:jk_[a-z_]+' "$F" | sed 's/%foreign "C://' | sort -u | tr '\n' ' ' | sed 's/ $//')
same "exported symbols" "$idris_syms" "$hdr_syms"
zig_syms=$(grep -oE 'pub export fn jk_[a-z_]+' "$Z" | awk '{print $NF}' | sort -u | tr '\n' ' ' | sed 's/ $//')
same "zig exports" "$idris_syms" "$zig_syms"

[ "$fail" -eq 0 ] && say "OK: Idris2 ABI, januskey.h and ffi/zig agree" || { say "FAIL"; exit 1; }
