---
phase: 15-wasm-safe-mobile-domain-export-for-unified-pc
verified: 2026-07-29T07:05:01Z
status: passed
score: 5/5 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 3/5
  gaps_closed:
    - "Canonical native/WASM fixtures are now locked to the accepted Wave 4 source identity and both guarded fixture verification and the serial baseline pass."
    - "The reproducible artifact identity is consistent across manifest, UAT, source lock, and validator; pinned clean ancestor commit 7609406 validates after later metadata commits."
  gaps_remaining: []
  regressions: []
---

# Phase 15: WASM-safe Mobile Domain Export Verification Report

**Phase Goal:** Export the current Flutter-backed Rust domain behavior as a WASM-safe canonical package for Word Net without moving SQLite or platform lifecycle concerns into the shared core.
**Verified:** 2026-07-29T07:05:01Z
**Status:** passed
**Re-verification:** Yes - after gap closure

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | Exported model/rule/protocol/WASM crates compile for wasm32 without persistence or platform dependencies. | VERIFIED | Fresh wasm32 check passed for all four crates; `cargo tree -p word-domain-wasm` contained none of `rusqlite`, `reqwest`, `jni`, `objc`, `tauri`, or `platform-mobile`. |
| 2 | Native and WASM runners return equivalent canonical JSON for fixtures locked to accepted mobile truth. | VERIFIED | Fixture manifest and source lock both identify `127a0a37...`; guarded verification passed all seven fixtures and the serial baseline passed 14/14. |
| 3 | Existing Flutter/mobile Rust production wiring remains behaviorally preserved after extraction. | VERIFIED | Previous wiring inspection and 50/50 platform-mobile regression pass remain current; domain/protocol regressions passed again. User accepted all six named release-device flows. |
| 4 | Word Net can pin and validate a reproducible versioned package with structured errors. | VERIFIED | Manifest validation passed for clean ancestor commit `7609406`; UAT, fixture manifest, and source lock agree on the final `127a0a37...` ledger identity, with the artifact rebuilt from the corresponding clean commit; JS/WASM hashes match. |
| 5 | Chromium, Firefox, and WebKit execution plus size/startup/serialization evidence is recorded. | VERIFIED | Final evidence records all three engines executing the unchanged generated package, seven fixtures, and a structured error within locked budgets. |

**Score:** 5/5 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `crates/domain-models` | Serde-only canonical DTOs | VERIFIED | Substantive, additive-field compatible, wasm32-safe. |
| `crates/domain-core` | Deterministic typed study/projection rules | VERIFIED | Fresh unit and fixture suites passed. |
| `crates/domain-protocol/src/v1.rs` | Versioned typed dispatcher and structured errors | VERIFIED | Five protocol tests passed; typed payload dispatch remains shared by both adapters. |
| `crates/domain-wasm/src/lib.rs` | Thin WASM adapter | VERIFIED | Direct passthrough to shared `execute_v1`; equivalence test passed. |
| `fixtures/domain/v1/manifest.json` | Canonical corpus bound to accepted source | VERIFIED | `sourceLockDigest` equals accepted `127a0a37...`; guarded verification passed. |
| `artifacts/domain-wasm/manifest.json` and package | Pin-ready reproducible artifact | VERIFIED | Clean source commit `7609406`, two-build reproducibility, source/Cargo lock identity, hashes, sizes, and budgets are recorded and validated. |
| `15-UAT.md` | Final artifact and device acceptance evidence | VERIFIED | Pin-ready table matches the final artifact; six named mobile flows are accepted with known issues disclosed. |

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| storage model facade | domain-models | public re-exports | WIRED | Native import paths retain canonical DTO identity. |
| study-core | domain-core | compatibility wrappers/re-exports | WIRED | Study behavior delegates to the pure implementation. |
| app-core reports/wrong words | domain-core projections | typed adapter after repository reads | WIRED | SQLite and platform time remain outside shared core. |
| domain-wasm and native runner | domain-protocol | identical `execute_v1` dispatcher | WIRED | Both adapters remain transport-only. |
| fixture manifest | accepted source lock | `sourceLockDigest` | WIRED | Both identify `127a0a376fdcaa1bebde0e277562d93098d9c09e25fa145ca7d95aab65306aef`. |
| browser spec | generated package/manifest | manifest-pinned dynamic import | WIRED | Recorded final gates execute the actual generated package. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| domain protocol | typed command payload | versioned JSON -> typed Rust inputs -> domain-core | Yes | FLOWING |
| mobile compatibility path | study/report/wrong-word DTOs | native repositories/platform context -> app-core -> domain-core | Yes | FLOWING |
| canonical fixture gate | request/expected corpus | accepted source lock -> fixture manifest -> native/protocol/WASM execution | Yes | FLOWING |
| generated browser package | fixture request JSON | JS loader -> generated WASM -> shared `execute_v1` | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Wave 4 source identity | `check-source-lock.ps1 -Check -Wave 4` | Passed at `127a0a37...` | PASS |
| Canonical native fixture gate | `capture-native-fixtures.ps1 -Verify` | All seven fixture groups passed | PASS |
| Serial mobile baseline | `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` | 14/14 passed | PASS |
| Manifest validation | `node scripts/domain-export/validate-manifest.mjs artifacts/domain-wasm/manifest.json` | Valid for pinned commit `7609406`; WASM hash matched | PASS |
| Validator ancestor contract | `node --test scripts/domain-export/validate-manifest.test.mjs` | 1/1 passed | PASS |
| Pure domain/protocol/adapters | `cargo test -p word-domain-models -p word-domain-core -p word-domain-protocol -p word-domain-wasm -p word-study-domain-runner` | All suites passed | PASS |
| wasm32 compilation | `cargo check -p word-domain-models -p word-domain-core -p word-domain-protocol -p word-domain-wasm --target wasm32-unknown-unknown` | Passed | PASS |
| Dependency and artifact identity | `cargo tree -p word-domain-wasm` plus SHA-256/ancestor checks | No forbidden dependencies; source ancestor and JS/WASM hashes verified | PASS |
| Three-browser generated artifact | Recorded final `run-browser-gates.ps1 -FinalArtifact` evidence | Chromium, Firefox, and WebKit passed together | PASS |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| ARCH-04 | 15-02, 15-05 | wasm32-safe models without forbidden runtime dependencies | SATISFIED | Fresh wasm32 build and forbidden-dependency scan passed. |
| ARCH-05 | 15-01, 15-03, 15-04, 15-05 | Equivalent native/WASM canonical results | SATISFIED | Source-locked fixture gate, 14-test native baseline, protocol, and WASM equivalence tests passed. |
| ARCH-06 | 15-01, 15-04, 15-05 | Additive versioned protocol, structured errors, reproducible identity | SATISFIED | Protocol tests, ancestor-aware validator, aligned UAT/manifest identity, and reproducible artifact evidence pass. |
| ARCH-07 | 15-01 through 15-05 | Preserve Flutter production behavior and keep SQLite/platform concerns outside shared package | SATISFIED | Adapter boundaries, dependency scan, regression evidence, and accepted release-device UAT preserve the mobile contract. |

No Phase 15 requirements are orphaned from plan frontmatter.

### Anti-Patterns Found

No blocker anti-patterns remain. The former stale fixture digest, HEAD-equality validator rule, and superseded UAT identity are repaired.

The existing baseline-runner `has_snapshot` dead-code warning is non-blocking and unrelated to Phase 15 goal achievement.

### Human Verification Required

None outstanding for the Phase 15 functional scope. The user already accepted NewWord, Review, resume, Today, Wrong Words, and Reports on Android release `1.0.3+4`.

The longstanding keyboard hide/show jank and one first-daily-entry double-submit stall remain open residual performance risks. They predate Phase 15 by user attribution, were neither introduced nor fixed by the extraction, and are not Phase 15 regressions.

### Gaps Summary

Both prior identity-chain gaps are closed. The canonical corpus is locked to the final accepted source digest, its guarded native gate passes, and the reproducible package can be validated from its clean source commit even after later metadata commits. No regressions or remaining Phase 15 goal gaps were found.

---

_Verified: 2026-07-29T07:05:01Z_
_Verifier: Codex (gsd-verifier)_
