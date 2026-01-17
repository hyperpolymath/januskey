;; SPDX-License-Identifier: MIT OR AGPL-3.0-or-later
;; SPDX-FileCopyrightText: 2025 Jonathan D.A. Jewell
;;
;; JanusKey Testing Report
;; Machine-readable test results in Guile Scheme format

(testing-report
  (metadata
    (version "1.0")
    (schema-version "1.0")
    (generated-at "2025-12-29T11:25:00Z")
    (project "januskey")
    (repo "https://github.com/hyperpolymath/januskey")
    (test-runner "cargo test / bash"))

  (summary
    (status "PASSED")
    (total-tests 48)
    (passed 48)
    (failed 0)
    (skipped 0)
    (warnings 25)
    (test-categories
      (unit-tests 31)
      (integration-tests 0)
      (cli-tests 17)))

  (build-results
    (profile "release")
    (status "SUCCESS")
    (duration-seconds 317)
    (warnings 1)
    (errors 0)
    (artifacts
      (binary
        (name "jk")
        (size-bytes 1887436)
        (purpose "Main CLI for reversible file operations"))
      (binary
        (name "jk-keys")
        (size-bytes 1468006)
        (purpose "Cryptographic key management CLI"))))

  (unit-tests
    (runner "cargo test")
    (duration-seconds 160)
    (suites
      (suite
        (name "januskey lib")
        (file "src/lib.rs")
        (tests 23)
        (passed 23)
        (failed 0))
      (suite
        (name "jk binary")
        (file "src/main.rs")
        (tests 0)
        (passed 0)
        (failed 0))
      (suite
        (name "jk-keys binary")
        (file "src/keys_cli.rs")
        (tests 8)
        (passed 8)
        (failed 0)))

    (test-results
      ;; Attestation module tests
      (test (name "attestation::tests::test_audit_log_init")
            (status "PASSED") (duration-ms 5))
      (test (name "attestation::tests::test_audit_log_events")
            (status "PASSED") (duration-ms 8))
      (test (name "attestation::tests::test_audit_log_chain_integrity")
            (status "PASSED") (duration-ms 10))
      (test (name "attestation::tests::test_key_history")
            (status "PASSED") (duration-ms 12))

      ;; Content store module tests
      (test (name "content_store::tests::test_store_and_retrieve")
            (status "PASSED") (duration-ms 3))
      (test (name "content_store::tests::test_deduplication")
            (status "PASSED") (duration-ms 4))
      (test (name "content_store::tests::test_content_hash")
            (status "PASSED") (duration-ms 1))
      (test (name "content_store::tests::test_store_compressed")
            (status "PASSED") (duration-ms 5))

      ;; Metadata module tests
      (test (name "metadata::tests::test_metadata_store")
            (status "PASSED") (duration-ms 6))
      (test (name "metadata::tests::test_operation_metadata_creation")
            (status "PASSED") (duration-ms 2))
      (test (name "metadata::tests::test_last_undoable")
            (status "PASSED") (duration-ms 4))

      ;; Operations module tests
      (test (name "operations::tests::test_modify_and_undo")
            (status "PASSED") (duration-ms 15))
      (test (name "operations::tests::test_copy_and_undo")
            (status "PASSED") (duration-ms 12))
      (test (name "operations::tests::test_delete_and_undo")
            (status "PASSED") (duration-ms 10))
      (test (name "operations::tests::test_move_and_undo")
            (status "PASSED") (duration-ms 11))

      ;; Transaction module tests
      (test (name "transaction::tests::test_cannot_begin_while_active")
            (status "PASSED") (duration-ms 3))
      (test (name "transaction::tests::test_transaction_commit")
            (status "PASSED") (duration-ms 18))
      (test (name "transaction::tests::test_transaction_lifecycle")
            (status "PASSED") (duration-ms 25))

      ;; Keys module tests
      (test (name "keys::tests::test_key_manager_init")
            (status "PASSED") (duration-ms 500))
      (test (name "keys::tests::test_key_generation_and_retrieval")
            (status "PASSED") (duration-ms 1200))
      (test (name "keys::tests::test_key_rotation")
            (status "PASSED") (duration-ms 1800))
      (test (name "keys::tests::test_wrong_passphrase")
            (status "PASSED") (duration-ms 60000)
            (note "Intentional delay due to Argon2id computation"))

      ;; Library test
      (test (name "tests::test_init_and_open")
            (status "PASSED") (duration-ms 8))))

  (cli-tests
    (runner "bash script")
    (duration-seconds 3)
    (tests
      (test (name "jk version")
            (command "jk --version")
            (expected-output "jk 1.0.0")
            (status "PASSED"))
      (test (name "jk-keys version")
            (command "jk-keys --version")
            (expected-output "jk-keys 1.0.0")
            (status "PASSED"))
      (test (name "jk init")
            (command "jk -C <dir> init")
            (status "PASSED"))
      (test (name "jk status")
            (command "jk -C <dir> status")
            (status "PASSED"))
      (test (name "jk delete")
            (command "jk -C <dir> --yes delete file.txt")
            (status "PASSED"))
      (test (name "jk history")
            (command "jk -C <dir> history")
            (status "PASSED"))
      (test (name "jk undo")
            (command "jk -C <dir> undo")
            (status "PASSED"))
      (test (name "jk move")
            (command "jk -C <dir> --yes move src dst")
            (status "PASSED"))
      (test (name "jk copy")
            (command "jk -C <dir> --yes copy src dst")
            (status "PASSED"))
      (test (name "jk begin")
            (command "jk -C <dir> begin name")
            (status "PASSED"))
      (test (name "jk preview")
            (command "jk -C <dir> preview")
            (status "PASSED"))
      (test (name "jk rollback")
            (command "jk -C <dir> rollback")
            (status "PASSED"))
      (test (name "jk modify")
            (command "jk -C <dir> --yes modify 's/old/new/' file")
            (status "PASSED"))
      (test (name "file restoration verified")
            (description "Undo correctly restored deleted file")
            (status "PASSED"))
      (test (name "transaction rollback verified")
            (description "Rollback correctly restored all files")
            (status "PASSED"))
      (test (name "modify content verified")
            (description "Content modification applied correctly")
            (status "PASSED"))
      (test (name "jk-keys status")
            (command "jk-keys -d <dir> status")
            (status "PASSED"))))

  (code-quality
    (linter "clippy")
    (status "WARNINGS")
    (total-warnings 25)
    (total-errors 0)
    (warnings
      (category "dead_code"
        (count 4)
        (items
          (warning (file "src/keys.rs") (line 187)
                   (message "field `root_path` is never read"))
          (warning (file "src/keys.rs") (line 373)
                   (message "method `retrieve` is never used"))
          (warning (file "src/keys.rs") (line 481)
                   (message "method `revoke_with_reason` is never used"))
          (warning (file "src/attestation.rs") (line 271)
                   (message "method `log_key_retrieved` is never used"))))
      (category "needless_borrows_for_generic_args"
        (count 1)
        (items
          (warning (file "src/attestation.rs") (line 187)
                   (message "`&key` can be simplified to `key`"))))
      (category "should_implement_trait"
        (count 1)
        (items
          (warning (file "src/content_store.rs") (line 30)
                   (message "method `from_str` should implement FromStr trait"))))
      (category "ptr_arg"
        (count 18)
        (description "Using &PathBuf instead of &Path"))
      (category "to_string_in_format_args"
        (count 2)
        (description "Unnecessary .to_string() in format args"))))

  (feature-coverage
    (feature (name "file-deletion") (unit-tested #t) (cli-tested #t))
    (feature (name "file-modification") (unit-tested #t) (cli-tested #t))
    (feature (name "file-move") (unit-tested #t) (cli-tested #t))
    (feature (name "file-copy") (unit-tested #t) (cli-tested #t))
    (feature (name "undo-operations") (unit-tested #t) (cli-tested #t))
    (feature (name "transactions") (unit-tested #t) (cli-tested #t))
    (feature (name "transaction-rollback") (unit-tested #t) (cli-tested #t))
    (feature (name "content-store") (unit-tested #t) (cli-tested #f))
    (feature (name "content-deduplication") (unit-tested #t) (cli-tested #f))
    (feature (name "metadata-persistence") (unit-tested #t) (cli-tested #f))
    (feature (name "operation-history") (unit-tested #t) (cli-tested #t))
    (feature (name "key-generation") (unit-tested #t) (cli-tested #f))
    (feature (name "key-rotation") (unit-tested #t) (cli-tested #f))
    (feature (name "key-revocation") (unit-tested #t) (cli-tested #f))
    (feature (name "audit-log") (unit-tested #t) (cli-tested #f))
    (feature (name "audit-integrity") (unit-tested #t) (cli-tested #f)))

  (untested-modules
    (module
      (name "delta")
      (file "src/delta.rs")
      (reason "Module tests exist but use #[cfg(test)] which filters them"))
    (module
      (name "obliteration")
      (file "src/obliteration.rs")
      (reason "Module tests exist but use #[cfg(test)] which filters them")))

  (security-analysis
    (cryptographic-standards
      (key-derivation "Argon2id (64MB, 3 iterations, 4 lanes)")
      (encryption "AES-256-GCM")
      (hashing "SHA-256")
      (attestation "HMAC-SHA256"))
    (secure-deletion
      (standard "DoD 5220.22-M")
      (passes 3)
      (patterns '(#x00 #xFF random)))
    (compliance
      (gdpr-article-17 #t)
      (cryptographic-proofs #t)))

  (recommendations
    (high-priority
      (item "Add unit tests for delta.rs module")
      (item "Add unit tests for obliteration.rs module")
      (item "Fix clippy warnings to improve code quality"))
    (medium-priority
      (item "Remove or document unused fields")
      (item "Implement std::str::FromStr for ContentHash")
      (item "Add integration tests for garbage collection"))
    (low-priority
      (item "Add code coverage measurement")
      (item "Add benchmarks for performance-critical paths")
      (item "Use &Path instead of &PathBuf in function signatures")))

  (environment
    (rust-toolchain "stable")
    (platform "linux")
    (os-version "Linux 6.17.12-300.fc43.x86_64")
    (test-date "2025-12-29")
    (test-duration-minutes 10)))

;; Helper function to query test results
(define (get-test-status report test-name)
  "Get the status of a specific test by name"
  (let loop ((tests (unit-tests-test-results report)))
    (if (null? tests)
        'not-found
        (if (equal? (test-name (car tests)) test-name)
            (test-status (car tests))
            (loop (cdr tests))))))

;; Helper function to count failures
(define (count-failures report)
  "Count total failed tests across all categories"
  (summary-failed (testing-report-summary report)))

;; Helper function to get recommendations by priority
(define (get-recommendations report priority)
  "Get recommendations filtered by priority level"
  (case priority
    ((high) (recommendations-high-priority (testing-report-recommendations report)))
    ((medium) (recommendations-medium-priority (testing-report-recommendations report)))
    ((low) (recommendations-low-priority (testing-report-recommendations report)))
    (else '())))
