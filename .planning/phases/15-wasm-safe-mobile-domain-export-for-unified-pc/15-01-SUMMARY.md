---
phase: 15-wasm-safe-mobile-domain-export-for-unified-pc
plan: 01
subsystem: domain-export
tags: [rust, powershell, golden-fixtures, source-lock, wasm]
requires: []
provides:
  - Evidence-gated source and learning-ledger hash lock
  - Seven deterministic native mobile behavior fixture groups
  - Semantic JSON comparison preserving array order and values
affects: [15-02, 15-03, 15-04, 15-05, word-net]
tech-stack:
  added: []
  patterns: [golden-master-first, explicit-fixture-acceptance, append-only-source-promotion]
key-files:
  created:
    - scripts/domain-export/capture-native-fixtures.ps1
    - scripts/domain-export/compare-semantic-json.mjs
    - fixtures/domain/v1/manifest.json
    - fixtures/domain/v1/source-lock.json
    - fixtures/domain/v1/source-lock-promotions.json
    - fixtures/domain/v1/requests/study-newword.json
    - fixtures/domain/v1/expected/study-newword.json
  modified:
    - scripts/domain-export/check-source-lock.ps1
    - crates/app-core/tests/baseline_runner.rs
    - docs/features/learning.md
key-decisions:
  - "Hash authoritative mobile product inputs separately from generated Phase 15 fixture infrastructure."
  - "Require reviewed diff, successful parity checks, and an updated learning-ledger digest before source-lock promotion."
  - "Capture pre-submit and post-submit study projections separately so translations remain feedback-only."
requirements-completed: [ARCH-05, ARCH-06, ARCH-07]
duration: 30 min
completed: 2026-07-28
---

# Phase 15 Plan 01: Mobile Domain Golden Baseline Summary

**Evidence-gated mobile source identity plus seven deterministic native golden groups for study, progress, resume, wrong-word, and report behavior**

## Performance

- **Duration:** 30 min
- **Started:** 2026-07-28T07:39:45Z
- **Completed:** 2026-07-28T08:09:37Z
- **Tasks:** 2
- **Files modified:** 22

## Accomplishments

- Captured SHA-256 identity for the authoritative question/session/report/wrong-word sources, mobile bridge, Flutter display test, and learning ledger while excluding generated export artifacts.
- Added fail-closed Wave checks and append-only promotion history with embedded reviewed-diff and parity evidence.
- Captured seven deterministic native fixture groups covering NewWord type-major rounds, all non-NewWord modes, feedback-only translations, progress/carry-over, resume, wrong-word identity, and report local day.
- Added explicit fixture acceptance and semantic JSON comparison; the serial app-core baseline passes 14/14.

## Task Commits

Each task followed RED then GREEN TDD commits:

1. **Task 1: Create the mobile source and ledger hard gate**
   - `ede5ed2` - failing source-lock contract test
   - `aa15e74` - source lock, promotion gate, and ledger contract
2. **Task 2: Capture the canonical native fixture corpus**
   - `4bf6fb2` - failing canonical fixture corpus
   - `6a5c5c2` - accepted native outputs, capture/comparator tooling, and baseline alignment

## Files Created/Modified

- `scripts/domain-export/check-source-lock.ps1` - Wave capture/check/promotion with explicit drift lists and embedded promotion evidence.
- `scripts/domain-export/capture-native-fixtures.ps1` - Refuses incidental overwrite and verifies all manifest groups against the accepted lock.
- `scripts/domain-export/compare-semantic-json.mjs` - Ignores object-key order while preserving arrays and scalar values.
- `fixtures/domain/v1/manifest.json` - Names seven behavior groups and pins accepted source digest `8cf2ae28e079d1a4cee6cb3fe406c5ddc139a16ea984990baff859d3fbbd6507`.
- `fixtures/domain/v1/requests/*.json` - Fixed clock/day/session/order inputs and production-shaped payloads.
- `fixtures/domain/v1/expected/*.json` - Accepted native outputs captured from current mobile-backed Rust behavior.
- `crates/app-core/tests/baseline_runner.rs` - Native fixture dispatcher/invariants plus current final-submit and empty-request resume assertions.
- `docs/features/learning.md` - Shared route, pitfalls, failures, and actual verification evidence.

## Decisions Made

- Current Flutter-backed Rust behavior remains authoritative; promotion records parity evidence but does not authorize product-rule correction or copying stale desktop behavior.
- Expected fixture JSON can only change under `-AcceptCurrentMobileTruth`; `-Verify` never rewrites accepted output.
- Final-submit persistence and empty-request snapshot resume are the current facade contracts captured by the baseline runner.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected PowerShell status interpolation**
- **Found during:** Task 2 guarded capture
- **Issue:** `$mode:` parsed as an invalid scoped variable and stopped the command before capture.
- **Fix:** Delimited the variable as `${mode}`.
- **Files modified:** `scripts/domain-export/capture-native-fixtures.ps1`
- **Verification:** Explicit capture and verify modes both passed.
- **Commit:** `6a5c5c2`

**2. [Rule 1 - Bug] Corrected report fixture persistence shape**
- **Found during:** Task 2 first accepted capture
- **Issue:** Flattened report counters were filtered out because production history reads nested `summary` values and serialized enum text.
- **Fix:** Used persisted history shape and asserted total questions, study days, and normalized modes.
- **Files modified:** `fixtures/domain/v1/requests/report-local-day.json`, `crates/app-core/tests/baseline_runner.rs`
- **Verification:** Report fixture and full baseline suite passed.
- **Commit:** `6a5c5c2`

**3. [Rule 3 - Blocking] Aligned stale baseline-only completion and resume assertions**
- **Found during:** Task 2 full serial baseline verification
- **Issue:** Five old assertions called explicit completion after final submit; resume supplied fresh sources instead of restoring the persisted plan.
- **Fix:** Asserted final-submit summaries and resumed a deterministic input-question snapshot with an empty request. Product code was unchanged.
- **Files modified:** `crates/app-core/tests/baseline_runner.rs`
- **Verification:** Serial baseline passed 14/14.
- **Commit:** `6a5c5c2`

**4. [Rule 1 - Bug] Permitted evidence-gated ledger-only promotion**
- **Found during:** Task 2 ledger checkpoint
- **Issue:** The initial promotion rule required a product-source change, making mandatory verification-only ledger updates impossible to accept without recapture.
- **Fix:** Require any authoritative or ledger change, while still requiring reviewed coverage for every product-source change and successful parity evidence.
- **Files modified:** `scripts/domain-export/check-source-lock.ps1`, `fixtures/domain/v1/source-lock-promotions.json`
- **Verification:** Unpromoted ledger drift failed with an explicit file list; Wave 1 promotion and post-check passed.
- **Commit:** `6a5c5c2`

**Total deviations:** 4 auto-fixed (3 bugs, 1 blocking issue).
**Impact:** All fixes were limited to plan-owned fixture/gate infrastructure and baseline assertions; no production behavior changed.

## Known Stubs

- `fixtures/domain/v1/manifest.json:3` uses intentional `v1-placeholder` protocol identity. Plan 15-04 replaces it when the additive JSON protocol and shared dispatcher exist; native behavior claims and source digest are already concrete.

## Issues Encountered

- The first full serial baseline run failed 5/14 because older baseline-only assertions predated current final-submit persistence and snapshot resume rules. The aligned suite now passes 14/14.
- Existing `MockPlatformRuntime.has_snapshot` dead-code warning remains pre-existing and non-blocking.

## Verification

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/domain-export/check-source-lock.ps1 -Check -Wave 1` passed.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/domain-export/capture-native-fixtures.ps1 -Verify` passed all seven fixture groups.
- `node scripts/domain-export/compare-semantic-json.mjs --self-test` passed.
- `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` passed 14/14.
- Manifest inspection confirmed every required behavior group is named.
- `git diff --check` passed for all plan-owned implementation and ledger files, with CRLF conversion warnings only.

## Next Phase Readiness

- Ready for Plan 15-02 dependency inversion into serde-only domain models.
- Every later wave can run the accepted source lock and native fixture corpus before moving behavior.

## Self-Check: PASSED

- All key created/modified files exist.
- All four Task 1/Task 2 RED/GREEN commits exist in repository history.
