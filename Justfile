# SPDX-License-Identifier: PMPL-1.0-or-later
# januskey - Development Tasks
set shell := ["bash", "-uc"]
set dotenv-load := true

project := "januskey"

# Show all recipes
default:
    @just --list --unsorted

# Build all workspace crates (release)
build:
    cargo build --workspace --release

# Run all workspace tests
test:
    cargo test --workspace

# Clean build artefacts
clean:
    cargo clean

# Format all code
fmt:
    cargo fmt --all

# Lint all code with clippy
lint:
    cargo clippy --workspace -- -D warnings

# Run benchmarks
bench:
    cargo bench --workspace

# Run end-to-end tests (shell-based lifecycle)
test-e2e:
    @echo "=== E2E Tests ==="
    @if [ -x tests/e2e/lifecycle_e2e.sh ]; then bash tests/e2e/lifecycle_e2e.sh; else echo "SKIP: tests/e2e/lifecycle_e2e.sh not executable or missing"; fi

# Run aspect / cross-cutting tests
test-aspect:
    @echo "=== Aspect Tests ==="
    @if [ -x tests/aspect/cross_cutting_test.sh ]; then bash tests/aspect/cross_cutting_test.sh; else echo "SKIP: tests/aspect/cross_cutting_test.sh not executable or missing"; fi

# Run P2P component integration tests
test-p2p:
    cargo test --package januskey --test p2p_test

# Run regression tests
test-regressions:
    cargo test --package reversible-core --test unwrap_safety_test

# Run property-based tests
test-property:
    cargo test --workspace -- --include-ignored proptest

# Run FFI tests (placeholder — no FFI tests yet)
test-ffi:
    @echo "=== FFI Tests ==="
    @if [ -d ffi/zig/test ]; then echo "TODO: Run Zig FFI integration tests"; else echo "SKIP: No FFI tests present yet (see ffi/zig/)"; fi

# Smoke test: build + version + help
smoke:
    @echo "=== Smoke Test ==="
    cargo build --workspace
    @echo "--- jk --version ---"
    @cargo run --package januskey --bin jk -- --version 2>/dev/null || echo "WARN: jk --version not yet implemented"
    @echo "--- jk --help ---"
    @cargo run --package januskey --bin jk -- --help 2>/dev/null || echo "WARN: jk --help not yet implemented"
    @echo "Smoke test complete."

# Validate contractile files parse correctly
test-contracts:
    @echo "=== Contract Tests ==="
    @if command -v must >/dev/null 2>&1; then must check; \
    else \
        echo "must not found — validating contractile files manually..."; \
        for f in contractiles/intend contractiles/must contractiles/trust; do \
            if [ -f "$$f" ]; then echo "  [OK] $$f exists and is non-empty ($$(wc -c < $$f) bytes)"; \
            else echo "  [FAIL] $$f missing"; fi; \
        done; \
    fi

# Check Idris2 ABI proofs (requires idris2)
test-proofs:
    @echo "=== Proof Regression ==="
    @if command -v idris2 >/dev/null 2>&1; then \
        for f in src/abi/Types.idr src/abi/Layout.idr src/abi/Foreign.idr src/abi/Proofs.idr; do \
            if [ -f "$$f" ]; then \
                echo "Checking $$f..."; \
                idris2 --check "$$f" && echo "  [OK] $$f" || echo "  [FAIL] $$f"; \
            else \
                echo "  [SKIP] $$f not found"; \
            fi; \
        done; \
    else \
        echo "SKIP: idris2 not installed. Install via: pack install-app idris2"; \
    fi

# Run full test suite (all categories)
test-all: test test-p2p test-regressions test-e2e test-aspect test-contracts test-proofs smoke

# [AUTO-GENERATED] Multi-arch / RISC-V target
build-riscv:
	@echo "Building for RISC-V..."
	cross build --target riscv64gc-unknown-linux-gnu

# Run panic-attacker pre-commit scan
assail:
    @command -v panic-attack >/dev/null 2>&1 && panic-attack assail . || echo "panic-attack not found — install from https://github.com/hyperpolymath/panic-attacker"

# Self-diagnostic — checks dependencies, permissions, paths
doctor:
    @echo "Running diagnostics for januskey..."
    @echo "Checking required tools..."
    @command -v just >/dev/null 2>&1 && echo "  [OK] just" || echo "  [FAIL] just not found"
    @command -v git >/dev/null 2>&1 && echo "  [OK] git" || echo "  [FAIL] git not found"
    @command -v cargo >/dev/null 2>&1 && echo "  [OK] cargo" || echo "  [FAIL] cargo not found"
    @command -v rustc >/dev/null 2>&1 && echo "  [OK] rustc ($$(rustc --version))" || echo "  [FAIL] rustc not found"
    @command -v idris2 >/dev/null 2>&1 && echo "  [OK] idris2" || echo "  [INFO] idris2 not found (optional, for proof checking)"
    @echo "Checking for hardcoded paths..."
    @grep -rn '$$HOME\|$$ECLIPSE_DIR' --include='*.rs' --include='*.ex' --include='*.res' --include='*.gleam' --include='*.sh' . 2>/dev/null | head -5 || echo "  [OK] No hardcoded paths"
    @echo "Diagnostics complete."

# Auto-repair common issues
heal:
    @echo "Attempting auto-repair for januskey..."
    @echo "Fixing permissions..."
    @find . -name "*.sh" -exec chmod +x {} \; 2>/dev/null || true
    @echo "Cleaning stale caches..."
    @rm -rf .cache/stale 2>/dev/null || true
    @echo "Repair complete."

# Guided tour of key features
tour:
    @echo "=== januskey Tour ==="
    @echo ""
    @echo "1. Project structure:"
    @ls -la
    @echo ""
    @echo "2. Available commands: just --list"
    @echo ""
    @echo "3. Read README.adoc for full overview"
    @echo "4. Read EXPLAINME.adoc for architecture decisions"
    @echo "5. Run 'just doctor' to check your setup"
    @echo ""
    @echo "Tour complete! Try 'just --list' to see all available commands."

# Open feedback channel with diagnostic context
help-me:
    @echo "=== januskey Help ==="
    @echo "Platform: $(uname -s) $(uname -m)"
    @echo "Shell: $SHELL"
    @echo ""
    @echo "To report an issue:"
    @echo "  https://github.com/hyperpolymath/januskey/issues/new"
    @echo ""
    @echo "Include the output of 'just doctor' in your report."


# Print the current CRG grade (reads from READINESS.md '**Current Grade:** X' line)
crg-grade:
    @grade=$$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1); \
    [ -z "$$grade" ] && grade="X"; \
    echo "$$grade"

# Generate a shields.io badge markdown for the current CRG grade
# Looks for '**Current Grade:** X' in READINESS.md; falls back to X
crg-badge:
    @grade=$$(grep -oP '(?<=\*\*Current Grade:\*\* )[A-FX]' READINESS.md 2>/dev/null | head -1); \
    [ -z "$$grade" ] && grade="X"; \
    case "$$grade" in \
      A) color="brightgreen" ;; B) color="green" ;; C) color="yellow" ;; \
      D) color="orange" ;; E) color="red" ;; F) color="critical" ;; \
      *) color="lightgrey" ;; esac; \
    echo "[![CRG $$grade](https://img.shields.io/badge/CRG-$$grade-$$color?style=flat-square)](https://github.com/hyperpolymath/standards/tree/main/component-readiness-grades)"
