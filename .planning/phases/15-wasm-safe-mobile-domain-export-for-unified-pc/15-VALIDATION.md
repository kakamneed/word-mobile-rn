---
phase: 15
slug: wasm-safe-mobile-domain-export-for-unified-pc
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-07-28
---

# Phase 15 Validation Strategy

## Test Infrastructure

| Property | Value |
|----------|-------|
| Native | Rust built-in test harness |
| WASM | `wasm-bindgen-test` / `wasm-pack test` |
| Browser | Playwright Chromium, Firefox, WebKit projects |
| Mobile compatibility | Existing focused Flutter plus Rust bridge/app tests |
| Quick run | `cargo test -p word-domain-core -p word-domain-protocol` |
| Full run | `cargo test --workspace` plus canonical native/WASM diff and three-browser gate |

## Sampling Rate

- After every task: moved-module unit tests and the relevant canonical fixture subset.
- After every wave: pure-domain/protocol tests and semantic native/WASM fixture comparison.
- Before phase verification: full workspace, focused Flutter compatibility, optimized artifact/manifest validation, and all three browser projects.
- Maximum feedback latency for task-local checks: 120 seconds; split suites when needed.

## Per-Requirement Verification Map

| Requirement | Behavior | Automated verification | Status |
|-------------|----------|------------------------|--------|
| ARCH-04 | WASM crate excludes storage/platform dependencies and compiles | `cargo check -p word-domain-wasm --target wasm32-unknown-unknown` plus forbidden dependency scan | Wave 0 |
| ARCH-05 | Native/WASM canonical JSON agrees | native runner + WASM runner + semantic JSON comparator | Wave 0 |
| ARCH-05 | NewWord/Review, meanings, question data, units | `cargo test -p word-domain-core fixture_` | Partial infrastructure |
| ARCH-05 | Resume, wrong identity, reports/local day | focused pure state/projection fixtures | Wave 0 |
| ARCH-06 | Versioning, additive fields, errors, manifest | `cargo test -p word-domain-protocol` plus manifest validator | Wave 0 |
| ARCH-06 | Artifact executes in three browsers | `npm.cmd run test:domain-wasm -- --project=chromium --project=firefox --project=webkit` | Wave 0 |
| ARCH-07 | Existing mobile behavior survives extraction | focused `app-core`/`platform-mobile` tests and named Flutter study/today/report/wrong suites | Existing + extensions |

## Wave 0 Requirements

- [ ] Pin/install `wasm32-unknown-unknown`, `wasm-pack`, wasm-bindgen test tooling, and repo-local Playwright browsers.
- [ ] Add forbidden-dependency scan for the WASM graph.
- [ ] Capture current native canonical requests/expected results before extraction.
- [ ] Add semantic JSON comparator preserving array order.
- [ ] Add protocol, structured-error, and artifact-manifest tests.
- [ ] Add browser artifact harness and size/startup/serialization measurement scripts.
- [ ] Record initial measurements and lock final non-regression budgets before the last wave.

## Manual-Only Verifications

| Behavior | Requirement | Why manual | Instructions |
|----------|-------------|------------|--------------|
| Flutter production flow remains visually/interaction equivalent | ARCH-07 | Automated contracts cannot prove device interaction and rendering | Run focused release-device NewWord, Review, resume, Today, Wrong Words, and Reports smoke after automated gates pass. |

## Validation Sign-Off

- [x] Every requirement has an automated gate or Wave 0 dependency.
- [x] No three consecutive planned tasks may omit automated verification.
- [x] Watch-mode commands are forbidden.
- [x] Browser readiness requires actual Chromium, Firefox, and Playwright WebKit execution.
- [ ] Wave 0 dependencies installed and baseline fixtures captured.

**Approval:** approved for planning 2026-07-28
