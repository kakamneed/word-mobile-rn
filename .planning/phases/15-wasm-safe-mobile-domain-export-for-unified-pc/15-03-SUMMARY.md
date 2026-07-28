---
phase: 15-wasm-safe-mobile-domain-export-for-unified-pc
plan: 03
subsystem: domain
tags: [rust, wasm32, study, projections, deterministic-fixtures]

# Dependency graph
requires:
  - phase: 15-01
    provides: Native semantic fixtures, source-lock promotion, and parity evidence tooling
  - phase: 15-02
    provides: WASM-safe canonical serde models and native compatibility re-exports
provides:
  - Deterministic WASM-safe study, progress, summary, resume, wrong-word, and report rules
  - Thin study-core and app-core compatibility adapters over one pure implementation
  - Reviewed Wave 2 fixture evidence and promoted source lock
affects: [15-04, 15-05, word-net, learning-flow]

# Tech tracking
tech-stack:
  added: [word-domain-core]
  patterns: [explicit deterministic context, typed pure projections, native adapter ownership, compatibility facade]

key-files:
  created:
    - crates/domain-core/src/study.rs
    - crates/domain-core/src/progress.rs
    - crates/domain-core/src/projections.rs
    - crates/domain-core/tests/study_fixtures.rs
    - crates/domain-core/tests/projection_fixtures.rs
  modified:
    - crates/study-core/src/question_builder.rs
    - crates/study-core/src/answer_evaluator.rs
    - crates/app-core/src/services/reports_service.rs
    - crates/app-core/src/services/wrong_words_service.rs
    - crates/app-core/src/facade/study_facade.rs
    - docs/features/learning.md
    - fixtures/domain/v1/source-lock.json

key-decisions:
  - "Use fixed-width u64 deterministic hashing so native and wasm32 ordering cannot diverge by pointer width."
  - "Keep Review eligible for all four question types while selecting only one question per entry; only NewWord enters the four-round type-major loop."
  - "Resolve feedback-only translations from the retained source entry after submission because StudyQuestion intentionally redacts them before submission."
  - "Keep JSON conversion, SQLite access, and current local-day acquisition in native adapters; domain-core receives typed inputs and an explicit DomainContext."

patterns-established:
  - "Pure domain boundary: exported calculations receive all time, day, session, and ordering inputs explicitly."
  - "Compatibility facade: existing study-core and app-core public surfaces delegate to domain-core instead of copying rule branches."

requirements-completed: [ARCH-05, ARCH-07]

# Metrics
duration: 46min
completed: 2026-07-28
---

# Phase 15 Plan 03: Deterministic Domain Rules Summary

**WASM-safe Rust study and projection rules now power the existing native facades with fixture-proven NewWord, Review, feedback, identity, and local-day parity.**

## Performance

- **Duration:** 46 min
- **Started:** 2026-07-28T08:48:12Z
- **Completed:** 2026-07-28T09:34:01Z
- **Tasks:** 3
- **Files modified:** 29

## Accomplishments

- Extracted deterministic study generation, answer evaluation, question-unit progress, summaries, and resume transitions into `word-domain-core`, which compiles for `wasm32-unknown-unknown`.
- Extracted typed wrong-word and report projections while leaving SQLite, JSON compatibility, and platform time acquisition in native app-core adapters.
- Converted study-core into compatibility glue and preserved the locked NewWord four-question type-major loop, one-question Review behavior, and feedback-only example translations.
- Captured seven accepted native fixture groups, promoted the reviewed Wave 2 source lock, and reverified the promoted digest `7b576b77e145b9291a870ac4ec3957c50969c9405c53ba308fe9a39034f6c06e`.

## Task Commits

Each task was committed atomically using TDD red/green commits:

1. **Task 1 RED: deterministic study fixtures** - `b25c34f` (test)
2. **Task 1 GREEN: WASM-safe study core** - `5a45d2e` (feat)
3. **Task 2 RED: typed projection fixtures** - `1fb9343` (test)
4. **Task 2 GREEN: report and wrong-word projections** - `4deb335` (feat)
5. **Task 3 RED: study compatibility assertions** - `830c1dc` (test)
6. **Task 3 GREEN: study-core domain delegation** - `fd01901` (feat)
7. **Task 3 evidence: reviewed Wave 2 promotion** - `a17147a` (docs)
8. **Task 3 fix: promoted fixture manifest digest** - `80c07ac` (fix)

## Files Created/Modified

- `crates/domain-core/src/context.rs` - Explicit time, local-day, session, and ordering context.
- `crates/domain-core/src/study.rs` - Deterministic question generation, answer evaluation, mode rules, and session definitions.
- `crates/domain-core/src/progress.rs` - Question-unit progress, transition, and summary calculations.
- `crates/domain-core/src/projections.rs` - Typed wrong-word and report projection inputs/outputs.
- `crates/domain-core/tests/study_fixtures.rs` - Canonical NewWord, Review, visibility, progress, and resume assertions.
- `crates/domain-core/tests/projection_fixtures.rs` - Identity and explicit-local-day projection assertions.
- `crates/study-core/src/*.rs` - Compatibility re-exports and thin wrappers over domain-core.
- `crates/app-core/src/services/{reports_service,wrong_words_service}.rs` - Native repository/JSON adapters around pure projections.
- `crates/app-core/src/facade/study_facade.rs` - Native session facade using shared repair and domain rules.
- `fixtures/domain/v1/evidence/wave-2*.json` - Reviewed diff and successful parity evidence.
- `fixtures/domain/v1/{source-lock.json,source-lock-promotions.json,manifest.json}` - Promoted Wave 2 digest and fixture manifest alignment.
- `docs/features/{learning.md,_reconciliation.md}` - Learning-capability implementation, pitfalls, verification, and path reconciliation.

## Decisions Made

- Deterministic ordering hashes use `u64`, avoiding native/wasm pointer-width differences and debug overflow.
- Review retains access to the complete question-type set but selects one type per entry; its mode rule cannot enter NewWord's four-round loop.
- Pre-submit questions keep example translations absent. Post-submit feedback looks up the retained source entry rather than weakening that redaction boundary.
- Projection APIs are typed and pure. Existing JSON payload shape, repository access, and acquisition of the actual local day remain adapter responsibilities.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed pointer-width-dependent deterministic hashing**
- **Found during:** Task 1 (deterministic study contracts)
- **Issue:** FNV constants expressed as `usize` overflowed on wasm32 and could make ordering target-dependent.
- **Fix:** Performed hashing in fixed-width `u64` and converted only the final bounded result.
- **Files modified:** `crates/domain-core/src/study.rs`
- **Verification:** Domain study fixtures and `cargo check -p word-domain-core --target wasm32-unknown-unknown` passed.
- **Committed in:** `5a45d2e`

**2. [Rule 1 - Bug] Routed V2 resume rebuild and feedback through the shared compatibility behavior**
- **Found during:** Task 3 (study-core compatibility facade)
- **Issue:** V2 snapshot rebuild skipped question repair, and the baseline feedback harness tried to read a translation from the intentionally redacted question.
- **Fix:** Applied the existing repair path before validation and resolved post-submit translations from the retained source request entry; stale tests were aligned with safe choice downgrade and deterministic labels.
- **Files modified:** `crates/app-core/src/facade/study_facade.rs`, `crates/app-core/tests/baseline_runner.rs`
- **Verification:** 31 study-facade tests and 14 serial baseline-runner tests passed.
- **Committed in:** `fd01901`

**3. [Rule 3 - Blocking] Aligned the fixture manifest after source-lock promotion**
- **Found during:** Task 3 post-promotion verification
- **Issue:** Promotion updated the source lock but left the fixture manifest pinned to the previous accepted digest, so a fresh baseline run rejected the otherwise valid Wave 2 state.
- **Fix:** Updated the manifest digest to the promoted Wave 2 digest and reran all final lock/fixture checks.
- **Files modified:** `fixtures/domain/v1/manifest.json`
- **Verification:** 14 baseline tests, all seven fixture captures, and Wave 2 source-lock check passed after the update.
- **Committed in:** `80c07ac`

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking issue)
**Impact on plan:** Each fix was necessary for deterministic cross-target behavior or for the promoted evidence state to remain reproducible; no feature scope was added.

## Issues Encountered

- The planned intermediate `capture-native-fixtures.ps1 -Verify` correctly reported source-lock drift after extraction and before reviewed promotion. The promoted Wave 2 state subsequently passed the same verification.
- `question_builder.rs` and `wrong_words_service.rs` contained authoritative dirty input when execution began. Their current contents were frozen and incorporated, but authorship, exact timing, intent, and prior verification are not inferred from that worktree state.
- `docs/features/learning.md` and `_reconciliation.md` also contained existing user changes. Those changes were preserved; this plan's ledger additions record only the extraction points, observed pitfalls, and checks actually run.
- One pre-existing dead-code warning for `has_snapshot` remains in the baseline runner and is outside this plan's behavior surface.

## Verification

- `cargo test -p word-domain-core` - 26 tests passed.
- `cargo check -p word-domain-core --target wasm32-unknown-unknown` - passed.
- `cargo test -p word-study-core` - 3 tests passed.
- `cargo test -p word-app-core reports_service -- --test-threads=1` - 2 tests passed.
- `cargo test -p word-app-core wrong_words_service -- --test-threads=1` - 8 tests passed.
- `cargo test -p word-app-core study_facade::tests:: -- --test-threads=1` - 31 tests passed.
- `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` - 14 tests passed.
- `capture-native-fixtures.ps1 -Verify -WriteEvidence fixtures/domain/v1/evidence/wave-2.json` - all 7 accepted fixture groups passed.
- `check-source-lock.ps1 -Check -Wave 2` - promoted digest passed.

## Known Stubs

- `fixtures/domain/v1/manifest.json:3` retains `protocolVersion: "v1-placeholder"`. This is an intentional handoff to Plan 15-04's versioned export contract and does not affect Plan 15-03's native/domain semantic parity.

## Deferred Issues

- None affecting this plan's goal. The pre-existing baseline-runner warning remains out of scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 15-04 can expose the deterministic typed core through a versioned WASM contract without duplicating mobile study rules.
- Plan 15-05 can consume the same accepted fixture corpus for artifact and browser parity gates.
- No implementation blocker remains; live mobile UX was not changed or claimed by this extraction plan.

## Self-Check: PASSED

- All declared domain-core, fixture evidence, and summary files exist.
- All eight task/evidence commits are present in git history.

---
*Phase: 15-wasm-safe-mobile-domain-export-for-unified-pc*
*Completed: 2026-07-28*
