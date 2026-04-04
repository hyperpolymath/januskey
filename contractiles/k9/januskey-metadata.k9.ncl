K9!
# SPDX-License-Identifier: PMPL-1.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
# K9 Kennel-level guard: januskey project metadata and security invariants
# Security Level: Kennel (data-only, no execution)
# No signature required

{
  pedigree = {
    schema_version = "1.0.0",
    component_type = "security-tool-metadata",
    security = {
      leash = 'Kennel,
      trust_level = "data-only",
      allow_network = false,
      allow_filesystem_write = false,
      allow_subprocess = false,
    },
    metadata = {
      name = "januskey-metadata",
      version = "0.2.0",
      description = "Januskey cryptographic key management — security invariants and project constraints",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },
  },

  # Project configuration
  config = {
    # Cryptographic algorithm policy
    crypto = {
      # Algorithms that MUST NOT appear in production code
      banned_algorithms = [
        "MD5",
        "SHA1",
        "DES",
        "RC4",
        "RSA-PKCS1-v1.5",
      ],
      # Required algorithm standards
      required_algorithms = [
        "SHA-256",
        "SHA-3",
        "AES-256-GCM",
        "X25519",
        "Ed25519",
      ],
    },

    # Safety invariants
    safety = {
      # No unsafe blocks in cryptographic code
      forbidden_in_crypto = [
        "unsafe",
        "unwrap()",
        "expect()",
        "todo!()",
        "unimplemented!()",
      ],
      # Required proof annotations
      proofs_required = true,
    },

    # Test requirements
    tests = {
      min_unit_tests = 50,
      min_integration_tests = 5,
      property_tests_required = true,
      fuzz_targets_required = true,
    },

    # Build configuration
    build = {
      edition = "2021",
      msrv = "1.75",
    },

    # Language policy
    banned_files = ["Makefile", "Dockerfile"],
    banned_languages = ["python", "typescript", "javascript"],
  },

  # RSR compliance
  rsr = {
    explainme = true,
    ai_manifest = true,
    machine_readable = true,
    spdx_headers_required = true,
    security_md_required = true,
    idris2_abi = true,
    zig_ffi = true,
  },
}
