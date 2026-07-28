---
phase: 15-wasm-safe-mobile-domain-export-for-unified-pc
plan: 02
subsystem: domain-export
tags: [rust, serde, wasm, dependency-inversion, golden-fixtures]
requires:
  - phase: 15-01
    provides: Evidence-gated source lock and canonical native fixture corpus
provides:
  - Serde-only canonical study and projection DTO crate
  - Storage facade compatibility re-exports for native and mobile callers
  - WASM-clean study model dependency graph without SQLite
  - Reviewed Wave 1 parity evidence and promoted source lock
affects: [15-03, 15-04, 15-05, word-net]
tech-stack:
  added: [word-domain-models]
  patterns: [canonical-dto-ownership, compatibility-reexport, evidence-gated-promotion]
key-files:
  created:
    - crates/domain-models/Cargo.toml
    - crates/domain-models/src/study.rs
    - crates/domain-models/src/projections.rs
    - fixtures/domain/v1/evidence/wave-1.json
    - fixtures/domain/v1/evidence/wave-1-reviewed-diff.json
  modified:
    - crates/storage-core/src/models/mod.rs
    - crates/study-core/Cargo.toml
    - scripts/domain-export/capture-native-fixtures.ps1
    - fixtures/domain/v1/source-lock.json
    - docs/features/learning.md
key-decisions:
  - "Keep storage facade paths source-compatible while canonical type identity comes from word-domain-models."
  - "Resolve the preserved study-core import name to the pure domain package so dirty user-owned question_builder.rs remains untouched."
  - "Permit evidence-gated append-only promotion within the already accepted Wave 1 label while still rejecting older waves."
patterns-established:
  - "Canonical DTOs live in a serde-only crate; persistence crates re-export rather than duplicate them."
  - "Parity evidence references the accepted pre-promotion digest and updated learning-ledger hash."
requirements-completed: [ARCH-04, ARCH-07]
duration: 24 min
completed: 2026-07-28
---

# Phase 15 Plan 02: WASM-Safe Domain Model Ownership Summary

**Serde-only canonical study/projection DTOs with storage compatibility re-exports and reviewed native fixture parity**

## Performance

- **Duration:** 24 min
- **Started:** 2026-07-28T08:15:00Z
- **Completed:** 2026-07-28T08:39:34Z
- **Tasks:** 2
- **Files modified:** 24

## Accomplishments

- Created `word-domain-models` with fixture-backed serde round trips and a dependency graph that compiles for `wasm32-unknown-unknown` without persistence or platform runtimes.
- Replaced storage-owned study, Today/progress, and wrong-word definitions with canonical compatibility re-exports while preserving app-core, platform-mobile, and Flutter JSON contracts.
- Removed the real `word-storage-core`/`rusqlite` dependency from the study-core graph without modifying the dirty user-owned question builder.
- Verified all seven accepted native fixture groups, promoted reviewed Wave 1 evidence, and passed the post-promotion lock and ordinary fixture checks.

## Task Commits

Each TDD task has separate RED and GREEN commits:

1. **Task 1: Establish serde-only model ownership and compatibility tests**
   - `8978821` - failing canonical fixture compatibility tests
   - `6f309ac` - serde-only canonical study and projection models
2. **Task 2: Re-export canonical models without changing mobile call paths**
   - `89c3f87` - failing storage/canonical type-identity assertion
   - `68fd7fa` - native compatibility re-exports and Wave 1 evidence promotion

## Files Created/Modified

- `crates/domain-models/src/study.rs` - Canonical study DTOs, enums, summaries, and vocabulary payload types.
- `crates/domain-models/src/projections.rs` - Canonical Today, plan/progress, and wrong-word projection DTOs.
- `crates/storage-core/src/models/*.rs` - Compatibility re-exports retaining existing native import paths.
- `crates/study-core/Cargo.toml` - Resolves the existing source import name to the pure model package instead of storage-core.
- `scripts/domain-export/capture-native-fixtures.ps1` - Verification-only machine-readable parity evidence output.
- `fixtures/domain/v1/evidence/wave-1*.json` - Reviewed ownership diff and successful native parity evidence.
- `docs/features/learning.md` - Shared contract route, verification, and fixture-promotion lesson.

## Decisions Made

- Canonical type ownership moves without changing serialized names, defaults, bridge symbols, or frozen NewWord/Review behavior.
- Storage remains the compatibility facade during migration, but its study/projection modules contain re-exports rather than duplicate DTO definitions.
- Cargo package aliasing preserves the dirty user-owned `question_builder.rs` source while proving study-core resolves only `word-domain-models` and not SQLite.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Installed the missing WASM Rust target**
- **Found during:** Task 1 WASM verification
- **Issue:** `cargo check --target wasm32-unknown-unknown` failed because the standard library target was not installed.
- **Fix:** Installed `wasm32-unknown-unknown` with `rustup target add` and reran the exact check.
- **Files modified:** None in the repository.
- **Verification:** WASM check passed.
- **Commit:** N/A (toolchain-only)

**2. [Rule 3 - Blocking] Added the plan-specified parity evidence output**
- **Found during:** Task 2 fixture verification
- **Issue:** The Wave 0 script rejected the planned `-WriteEvidence` parameter, preventing evidence-gated promotion.
- **Fix:** Added verification-only evidence writing tied to the accepted digest and current ledger hash; ordinary verify/capture behavior remains fail-closed.
- **Files modified:** `scripts/domain-export/capture-native-fixtures.ps1`
- **Verification:** The exact `-Verify -WriteEvidence fixtures/domain/v1/evidence/wave-1.json` command passed and produced valid promotion evidence.
- **Commit:** `68fd7fa`

**3. [Rule 1 - Bug] Allowed append-only promotion within the existing Wave 1 label**
- **Found during:** Task 2 source-lock promotion
- **Issue:** Plan 15-01 had already promoted a ledger-only entry as Wave 1, so the required Plan 15-02 `-Promote -Wave 1` command was rejected even with new reviewed evidence.
- **Fix:** Reject only promotions older than the accepted wave; equal-wave promotions still require source/ledger changes, reviewed coverage, and passing parity evidence.
- **Files modified:** `scripts/domain-export/check-source-lock.ps1`
- **Verification:** Contract self-test, exact promotion command, and post-promotion Wave 1 check passed.
- **Commit:** `68fd7fa`

**4. [Rule 2 - Missing Critical] Aligned the fixture manifest with the promoted digest**
- **Found during:** Task 2 post-promotion verification
- **Issue:** Promotion updated the accepted lock but not `manifest.json`, which would make all later ordinary fixture verification fail on digest mismatch.
- **Fix:** Updated the manifest pointer to promoted digest `1f9bb5b7060d0ee19cebd68bcbd3862884cbc471156e0df672b8d9b726438bd9`.
- **Files modified:** `fixtures/domain/v1/manifest.json`
- **Verification:** Ordinary `capture-native-fixtures.ps1 -Verify` and the final source-lock check both passed.
- **Commit:** `68fd7fa`

**Total deviations:** 4 auto-fixed (1 bug, 1 missing critical, 2 blocking issues).
**Impact:** Fixes were limited to required local toolchain support and Phase 15 evidence infrastructure; product behavior and JSON contracts did not change.

## Known Stubs

None.

## Issues Encountered

- The initial evidence command failed on a missing parameter and the initial promotion failed because Wave 1 was already accepted. Both prerequisite inconsistencies were fixed and their exact commands now pass.
- Existing platform-mobile dead-code warnings and the baseline runner's unused `has_snapshot` warning remain pre-existing and non-blocking.

## Verification

- `cargo test -p word-domain-models` passed 2/2.
- Domain dependency scan found none of `rusqlite`, `reqwest`, `jni`, `objc`, or `tauri`.
- `cargo check -p word-domain-models --target wasm32-unknown-unknown` passed.
- `cargo check -p word-storage-core -p word-study-core -p word-app-core -p word-platform-mobile` passed.
- `cargo tree -p word-study-core` contained `word-domain-models` and neither `word-storage-core` nor `rusqlite`.
- `cargo test -p word-study-core` passed 35/35.
- `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` passed 14/14.
- Evidence-writing native fixture verification passed all seven groups.
- Source-lock contract self-test, evidence-gated Wave 1 promotion, post-check, and ordinary post-promotion fixture verification passed.
- `git diff --check` passed for the plan-owned implementation/evidence paths, with CRLF conversion warnings only.

## Next Phase Readiness

- Ready for Plan 15-03 to move pure study behavior onto the WASM-safe model boundary.
- No unresolved blocker; current dirty user product work remains preserved and uncommitted.

## Self-Check: PASSED

- All listed created artifacts exist.
- All four RED/GREEN task commits exist in repository history.
