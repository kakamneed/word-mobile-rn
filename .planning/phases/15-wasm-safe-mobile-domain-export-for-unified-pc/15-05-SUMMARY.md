---
phase: 15-wasm-safe-mobile-domain-export-for-unified-pc
plan: 05
subsystem: domain
tags: [rust, wasm, playwright, reproducible-build, flutter, device-uat]

# Dependency graph
requires:
  - phase: 15-04
    provides: Typed protocol v1 with equivalent native and WASM runners
provides:
  - Reproducible pin-ready generated WASM package with clean source identity
  - Chromium, Firefox, and WebKit execution evidence with locked size and timing budgets
  - Selected-device Flutter acceptance for NewWord, Review, resume, Today, Wrong Words, and Reports
  - Evidence-gated Wave 4 source lock aligned with the final learning ledger
affects: [word-net, learning-flow, wasm-export, mobile-release]

# Tech tracking
tech-stack:
  added: [playwright-browser-gates, generated-wasm-package]
  patterns: [two-clean-worktree reproducibility, manifest-pinned browser import, evidence-gated ledger promotion]

key-files:
  created:
    - artifacts/domain-wasm/manifest.json
    - artifacts/domain-wasm/package/word_domain_wasm_bg.wasm
    - tests/domain-browser/domain-wasm.spec.ts
    - fixtures/domain/v1/evidence/wave-4.json
    - fixtures/domain/v1/evidence/wave-4-uat.json
  modified:
    - scripts/domain-export/build-package.ps1
    - scripts/domain-export/run-browser-gates.ps1
    - fixtures/domain/v1/source-lock.json
    - docs/features/learning.md
    - .planning/phases/15-wasm-safe-mobile-domain-export-for-unified-pc/15-UAT.md

key-decisions:
  - "Release-device acceptance is PASS WITH KNOWN ISSUES, not an unqualified pass; the two longstanding performance issues remain unresolved and are not Phase 15 regressions."
  - "Because the learning ledger is source-lock input, post-build UAT evidence requires an evidence-gated ledger-only promotion and a new exact-commit reproducibility build rather than relabeling the old manifest."
  - "A pin-ready build may use clean detached worktrees even when the primary worktree contains unrelated user changes."

patterns-established:
  - "Pin-ready identity: manifest records protocol/schema, exact clean commit, accepted source-lock digest, normalized Cargo.lock hash, artifact hashes, tool versions, and measured budgets."
  - "Browser proof: import the generated package itself and execute all canonical requests plus structured errors in Chromium, Firefox, and WebKit."

requirements-completed: [ARCH-04, ARCH-05, ARCH-06, ARCH-07]

# Metrics
duration: 1h 30m active across checkpoint
completed: 2026-07-29
---

# Phase 15 Plan 05: Pin-Ready Browser and Mobile Compatibility Summary

**Reproducible protocol-v1 WASM package pinned to clean commit `ccef6b8`, proven in three browsers, and accepted on Android release `1.0.3+4` with two disclosed longstanding performance issues.**

## Performance

- **Duration:** 1h 30m active across checkpoint
- **Started:** 2026-07-28T11:23:28Z
- **Completed:** 2026-07-29T06:33:16Z
- **Tasks:** 3
- **Files modified:** 24

## Accomplishments

- Built a reproducible optimized package whose manifest records `dirty=false`, exact commit `ccef6b83a9de50b131c74a8d7b2898b4333b68c7`, source-lock digest `2b5522895e7f0983b7d67f0a3bbe1c92d03eedef17b991d44eb620a94cb8e538`, Cargo lock identity, stable JS/WASM hashes, and measured budgets.
- Executed all seven canonical requests plus a structured error through the generated package in Chromium, Firefox, and Playwright WebKit.
- Preserved mobile production behavior through Rust/Flutter regression gates and user acceptance of NewWord, Review, resume, Today, Wrong Words, and Reports on selected Android release `1.0.3+4`.
- Recorded keyboard hide/show jank and the first daily-entry double-submit stall as longstanding unresolved risks, neither introduced nor fixed by Phase 15.

## Task Commits

1. **Task 1: Prove candidate and promote reviewed Wave 4 source** - `e1c3352` (feat)
2. **Task 2: Normalize Cargo lock package identity** - `03ada86` (fix)
3. **Task 2: Publish reproducible domain WASM artifact** - `4d39cb0` (feat)
4. **Task 3: Accept release device with known issues and promote UAT ledger** - `ccef6b8` (docs)
5. **Task 3 contract repair: Repin artifact after device acceptance** - `631f26d` (fix)

## Files Created/Modified

- `artifacts/domain-wasm/manifest.json` - Pin-ready clean commit, promoted source digest, build identity, hashes, sizes, budgets, and reproducibility proof.
- `artifacts/domain-wasm/package/` - Generated JavaScript/WASM package consumed directly by browser tests.
- `tests/domain-browser/domain-wasm.spec.ts` - Canonical package execution, structured error, and measurement assertions.
- `scripts/domain-export/build-package.ps1` - Candidate and two-clean-worktree reproducible package builds.
- `scripts/domain-export/run-browser-gates.ps1` - Chromium/Firefox/WebKit gates and machine-readable evidence.
- `fixtures/domain/v1/evidence/wave-4*.json` - Candidate review/parity history plus distinct UAT closure evidence.
- `fixtures/domain/v1/source-lock*.json` - Evidence-gated Wave 4 promotions, including the ledger-only UAT closure.
- `.planning/phases/15-wasm-safe-mobile-domain-export-for-unified-pc/15-UAT.md` - Automated, artifact, deployment, selected-device, and known-issue acceptance record.
- `docs/features/learning.md` - Durable export, browser, release-device, source-identity, and residual-risk record.
- `docs/features/_reconciliation.md` - Phase 15 generated-package and browser-evidence capability mapping.

## Decisions Made

- User-observed functionality closes the six named device checks, while two user-attributed old performance issues keep the release status at PASS WITH KNOWN ISSUES.
- Release APK/build output remains outside Git; only source-identifiable WASM package artifacts and durable evidence are committed.
- The learning ledger remains part of the source lock. Its post-build UAT update was promoted with distinct evidence, then the artifact was rebuilt twice from that exact clean commit so manifest identity stayed truthful.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Normalized Cargo.lock package identity**
- **Found during:** Task 2 reproducibility validation
- **Issue:** Cargo lock identity depended on platform line endings rather than normalized lockfile text.
- **Fix:** Hash LF-normalized Cargo.lock text in the package manifest.
- **Files modified:** `scripts/domain-export/build-package.ps1`
- **Verification:** Two clean builds produced identical normalized manifests and optimized WASM hashes.
- **Committed in:** `03ada86`

**2. [Rule 3 - Blocking] Reconciled post-UAT ledger and artifact source identity**
- **Found during:** Task 3 checkpoint continuation
- **Issue:** Adding mandatory release-device evidence to `docs/features/learning.md` changed an authoritative source-lock input, so leaving the already-published manifest unchanged would make its accepted digest stale.
- **Fix:** Preserved the original Wave 4 evidence, added distinct `wave-4-uat` parity/review evidence after one complete three-browser pass, performed a ledger-only equal-Wave-4 promotion, committed UAT and accepted lock atomically, then rebuilt twice from exact clean commit `ccef6b8` and repinned the manifest.
- **Files modified:** `15-UAT.md`, `docs/features/learning.md`, `fixtures/domain/v1/source-lock*.json`, `fixtures/domain/v1/evidence/wave-4-uat*.json`, `artifacts/domain-wasm/manifest.json`
- **Verification:** Source lock passed at `2b552289...`; manifest validation matched that digest and `ccef6b8`; final Chromium, Firefox, and WebKit passed together.
- **Committed in:** `ccef6b8`, `631f26d`

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking contract issue)
**Impact on plan:** Both corrections preserve reproducible source identity without changing protocol behavior or generated JS/WASM hashes.

## Issues Encountered

- Default-parallel `cargo test --workspace` failed because existing app-core tests clear shared global registries; the complete workspace passed with `RUST_TEST_THREADS=1`, and the default command is not claimed as passed.
- The root-level focused Flutter command was invalid because the repository root has no `pubspec.yaml`; the corrected package-root suites passed 36/36.
- Two restricted browser reruns passed Chromium but failed while Firefox created a page. With browser subprocess permission, both the distinct UAT evidence run and final regenerated-artifact run passed Chromium, Firefox, and WebKit together.
- Three stale Wrong Words color expectations were aligned to the already-rendered four-level contract outside Phase 15 commits; focused Wrong Words and Reports tests passed.
- Device acceptance retained "键盘收起弹出卡顿" and "每日刚进入固定会有一次双击提交长卡顿" as pre-existing unresolved performance issues.

## Verification

- `check-source-lock.ps1 -Check -Wave 4` - passed at accepted digest `2b5522895e7f0983b7d67f0a3bbe1c92d03eedef17b991d44eb620a94cb8e538` after the ledger-only UAT promotion.
- `build-package.ps1 -CleanCommittedWorktree -ReproducibilityCheck` - two clean builds from `ccef6b8` produced identical normalized manifests and optimized WASM hash `9a7f58f...`.
- `validate-manifest.mjs artifacts/domain-wasm/manifest.json` - passed with `dirty=false`, exact commit `ccef6b8`, and the accepted source-lock digest.
- `run-browser-gates.ps1 -FinalArtifact` - final regenerated artifact passed Chromium, Firefox, and WebKit together.
- `cargo test --workspace` with `RUST_TEST_THREADS=1` - complete workspace passed; default parallel execution remains explicitly not passed.
- Focused Flutter Study, Today, Wrong Words, and Reports suites - passed 36/36 from `apps/flutter_mobile`.
- `cargo test -p word-platform-mobile` - passed 50/50 before device UAT and again during checkpoint continuation; existing unused-code warnings remain.
- Android deployment - fresh `com.wordmobile` `1.0.3+4`, SHA-256 `982097ca...`, installed and launched on explicitly selected `REA-AN00` transport.
- Device UAT - user reported NewWord, Review, resume, Today, Wrong Words, and Reports functioning normally; acceptance is PASS WITH KNOWN ISSUES.
- `git diff --check` on owned completion files - passed.

## Known Stubs

None - the generated package, manifest identity, browser imports, and device acceptance paths are fully wired. Historical placeholder references in the learning ledger describe resolved or rejected states, not active stubs.

## Deferred Issues

- Keyboard hide/show jank remains unresolved.
- The first daily entry still has one long double-submit stall.
- Both are longstanding by user attribution and are not Phase 15 regressions or fixes.

## User Setup Required

None - no external service configuration remains for this plan.

## Next Phase Readiness

- Word Net can pin protocol version `1`, clean source commit `ccef6b8`, accepted source-lock digest `2b552289...`, and the published package hashes.
- Phase 15 success criteria are satisfied with explicit known-issue disclosure; no SQLite, filesystem, network, Flutter, Tauri, or platform lifecycle concern entered the WASM package.
- The two mobile performance issues remain suitable follow-up work outside the Phase 15 export boundary.

## Self-Check: PASSED

- All declared summary, UAT, artifact, evidence, and ledger files exist.
- All five Task 1-3 and contract-repair commits are present in Git history.

---
*Phase: 15-wasm-safe-mobile-domain-export-for-unified-pc*
*Completed: 2026-07-29*
