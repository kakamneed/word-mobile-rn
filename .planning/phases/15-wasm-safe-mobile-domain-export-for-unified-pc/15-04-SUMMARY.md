---
phase: 15-wasm-safe-mobile-domain-export-for-unified-pc
plan: 04
subsystem: domain
tags: [rust, wasm-bindgen, json-protocol, wasm32, cross-target-parity]

# Dependency graph
requires:
  - phase: 15-03
    provides: Deterministic WASM-safe domain rules and accepted Wave 2 fixture behavior
provides:
  - Additive typed JSON protocol v1 with stable structured success and error envelopes
  - Native CLI and wasm-bindgen adapters over the identical execute_v1 dispatcher
  - Node-hosted native/WASM equivalence for all seven canonical fixture groups
  - Reviewed Wave 3 evidence, numeric v1 manifest, and promoted source lock
affects: [15-05, word-net, learning-flow, wasm-export]

# Tech tracking
tech-stack:
  added: [word-domain-protocol, word-domain-wasm, wasm-bindgen, wasm-bindgen-test, wasm-pack]
  patterns: [typed JSON boundary, shared pure dispatcher, input-only legacy adapters, live wasm equivalence]

key-files:
  created:
    - crates/domain-protocol/src/v1.rs
    - crates/domain-protocol/tests/protocol_v1.rs
    - crates/domain-wasm/src/lib.rs
    - crates/domain-wasm/tests/fixture_equivalence.rs
    - fixtures/domain/v1/evidence/wave-3.json
    - fixtures/domain/v1/evidence/wave-3-reviewed-diff.json
  modified:
    - crates/study-domain-runner/src/main.rs
    - crates/domain-core/src/study.rs
    - fixtures/domain/v1/manifest.json
    - fixtures/domain/v1/source-lock.json
    - docs/features/learning.md

key-decisions:
  - "Parse envelope metadata first, then immediately deserialize each command payload into a typed Rust structure before domain execution."
  - "Keep legacy native CLI names as input-only mappings while full v1 requests and WASM calls use the same execute_v1 dispatcher unchanged."
  - "Require real Node-hosted wasm32 execution because compile-only target checks cannot expose pointer-width-dependent ordering."

patterns-established:
  - "Protocol envelope: every response identifies protocol, request, and source and contains exactly one of result or structured error."
  - "Thin adapters: native and WASM boundaries contain transport conversion only; command execution lives in word-domain-protocol/domain-core."

requirements-completed: [ARCH-05, ARCH-06, ARCH-07]

# Metrics
duration: 36min
completed: 2026-07-28
---

# Phase 15 Plan 04: Versioned Native/WASM Protocol Summary

**Typed additive protocol v1 now drives identical native and Node-hosted WASM results through one shared dispatcher, with stable errors and seven-fixture parity.**

## Performance

- **Duration:** 36 min
- **Started:** 2026-07-28T09:42:30Z
- **Completed:** 2026-07-28T10:19:22Z
- **Tasks:** 2
- **Files modified:** 19

## Accomplishments

- Published typed v1 request/result/error envelopes with source identity, additive unknown-field tolerance, stable error codes, and no raw parser/runtime exception leakage.
- Replaced native rule ownership with a full-v1 passthrough plus three legacy input mappings, and added a one-function `wasm-bindgen` adapter over the identical dispatcher.
- Proved all seven canonical results under real Node-hosted wasm32 execution, not only host tests or wasm32 compilation.
- Captured reviewed Wave 3 parity evidence, promoted final source-lock digest `f9a8537c359f7b478c08fed5bef873b5b4cdabf4e430f6f691e69d623f14c668`, and published manifest protocol version `1`.

## Task Commits

1. **Task 1 RED: protocol v1 contract** - `00cef65` (test)
2. **Task 1 GREEN: typed dispatcher** - `6f7ae8c` (feat)
3. **Task 1 REFACTOR: formatted contract sources** - `d9d9bfc` (refactor)
4. **Task 2 RED: WASM fixture equivalence** - `b32e0e6` (test)
5. **Task 2 GREEN: native/WASM adapters and fixed-width seeds** - `9cf4013` (feat)
6. **Task 2 evidence: Wave 3 promotion and ledger** - `43544e4` (docs)

## Files Created/Modified

- `crates/domain-protocol/src/v1.rs` - Versioned command enum, typed payload decoding, dispatcher, response envelope, and stable errors.
- `crates/domain-protocol/tests/protocol_v1.rs` - Boundary compatibility/error tests plus canonical result coverage.
- `crates/domain-wasm/src/lib.rs` - Minimal `wasm-bindgen` passthrough to shared `execute_v1`.
- `crates/domain-wasm/tests/fixture_equivalence.rs` - Host and Node-hosted wasm32 semantic comparator for all canonical fixtures.
- `crates/study-domain-runner/src/main.rs` - Native v1 passthrough and legacy input-only command mappings.
- `crates/domain-core/src/study.rs` - Fixed-width ordering and distractor seeds with fixed-value regressions.
- `fixtures/domain/v1/{manifest.json,source-lock.json,source-lock-promotions.json}` - Numeric protocol version and accepted Wave 3 state.
- `fixtures/domain/v1/evidence/wave-3*.json` - Successful parity evidence and reviewed implementation diff.
- `docs/features/{learning.md,_reconciliation.md}` - Durable protocol/export contract, live-WASM lesson, verification, and capability mapping.

## Decisions Made

- Envelope fields are inspected before full deserialization so unsupported versions, unknown commands, and missing required fields receive distinct stable codes while every response retains request/source identity.
- Canonical and legacy command payloads become typed Rust values immediately at the boundary; adapters do not select question/report/wrong-word rules.
- Parsed JSON equality preserves array order while ignoring object-key serialization order, matching the established semantic comparator contract.
- `usize` is prohibited for deterministic seeds that cross native/WASM targets; only a bounded modulo result may convert to an index.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed pointer-width-dependent ordering and choice seeds**
- **Found during:** Task 2 live Node-hosted WASM equivalence
- **Issue:** `stable_seed` accumulated in `usize`, and `stable_distractor_seed` folded in `u64` but cast the full hash back to `usize`. Native tests and wasm32 compilation passed while real wasm32 execution changed NewWord order and answer positions.
- **Fix:** Kept both algorithms in `u64`, converted only bounded modulo results to indexes, and added fixed-value regressions.
- **Files modified:** `crates/domain-core/src/study.rs`
- **Verification:** 27 domain tests and the real Node-hosted seven-fixture comparator passed.
- **Committed in:** `9cf4013`

**2. [Rule 3 - Blocking] Published and aligned the fixture manifest after promotion**
- **Found during:** Task 2 Wave 3 promotion
- **Issue:** Plan 15-03 intentionally left `protocolVersion: "v1-placeholder"`, and source-lock promotion does not update the manifest digest, which would make ordinary fixture verification fail after promotion.
- **Fix:** Published numeric protocol version `1`, aligned the manifest to the final accepted digest, and reran ordinary fixture verification and the post-promotion source-lock check.
- **Files modified:** `fixtures/domain/v1/manifest.json`
- **Verification:** All seven accepted native fixtures and Wave 3 source lock passed at final digest `f9a8537c...`.
- **Committed in:** `43544e4`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking issue)
**Impact on plan:** Both fixes were necessary to make the promised cross-target equivalence and reproducible promoted state true; accepted fixture behavior did not change.

## Issues Encountered

- `wasm-pack` was absent and was installed as required. Its first test attempt hit a Windows Schannel download error for a target-only crate; an authorized unrestricted retry completed dependency acquisition.
- A sandboxed `wasm-pack` retry could not use its user temp/cache directories; the same command passed with the required toolchain access.
- The baseline runner retains one pre-existing `has_snapshot` dead-code warning outside this plan's behavior surface.

## Verification

- `cargo test -p word-domain-core -p word-domain-protocol -p word-domain-wasm -p word-study-domain-runner` - passed: 27 domain tests, 5 protocol tests, 1 host adapter comparator, and 2 native runner tests.
- `cargo check -p word-domain-protocol -p word-domain-wasm --target wasm32-unknown-unknown` - passed.
- `cargo tree -p word-domain-wasm` forbidden scan - no `rusqlite`, `reqwest`, `jni`, `objc`, `tauri`, or `platform-mobile` matches.
- `wasm-pack test --node crates/domain-wasm` - passed 1/1 real wasm32 canonical comparator after exposing and repairing the seed bug.
- `capture-native-fixtures.ps1 -Verify -WriteEvidence fixtures/domain/v1/evidence/wave-3.json` - all seven accepted fixtures passed and evidence was written.
- `check-source-lock.ps1 -Promote -Wave 3 ...` - evidence-gated promotion passed.
- `capture-native-fixtures.ps1 -Verify` - ordinary post-promotion verification passed all seven fixtures.
- `check-source-lock.ps1 -Check -Wave 3` - passed at final digest `f9a8537c359f7b478c08fed5bef873b5b4cdabf4e430f6f691e69d623f14c668`.

## Known Stubs

None - the previous manifest protocol placeholder was resolved and all created/modified runtime paths are wired to real domain inputs.

## Deferred Issues

None affecting this plan's goal.

## User Setup Required

None - no external service configuration required. `wasm-pack` is installed in the local Cargo toolchain.

## Next Phase Readiness

- Plan 15-05 can package and consume the numeric v1 WASM export against the accepted Wave 3 corpus.
- Word Net can rely on stable envelope/error semantics and a proven native/WASM dispatcher identity.
- No implementation or evidence blocker remains.

## Self-Check: PASSED

- All declared protocol, adapter, evidence, and summary files exist.
- All six Task 1/Task 2 commits are present in git history.

---
*Phase: 15-wasm-safe-mobile-domain-export-for-unified-pc*
*Completed: 2026-07-28*
