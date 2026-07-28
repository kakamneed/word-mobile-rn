# Phase 15 UAT: WASM-safe Mobile Domain Export

## Status

**Automation:** PASS WITH NOTE
**Release-device acceptance:** HUMAN NEEDED
**Phase completion:** BLOCKED until the selected-device checks below are observed and recorded.

Automated browser, artifact, parity, Rust, and focused Flutter evidence does not substitute for release-device interaction acceptance.

## Pin-Ready Artifact

| Field | Value |
| --- | --- |
| Exact clean build commit | `03ada863d9a96de92c975db32528c12870607679` |
| Git dirty | `false` |
| Protocol / manifest schema | `1` / `1` |
| Promoted source-lock digest | `2677acb20546179d5092d54d491e9ee2f4ffdcb3e0d3277abd7799cf4be2f1c1` |
| Cargo.lock SHA-256 | `60be5d0c73e1275387682d7342f7825a570b3f299cc71e8ae28cedf8c57ebb0a` (LF-normalized text) |
| WASM SHA-256 | `9a7f58f55f8596d89d3f485f5a8b9c10161ad0bfb9daba4ebb42e146bebdbe2b` |
| WASM size | 546507 raw / 187753 gzip bytes |
| JavaScript SHA-256 | `0102c84b618c67fe3e8e4ea4e42a880c0b28c7fffca35835fb307b5f0e02a399` |
| JavaScript size | 6648 bytes |
| Reproducibility | Two clean detached worktrees produced identical normalized manifests and optimized WASM hashes; temporary worktrees were removed. |

## Automated Evidence

| Gate | Status | Evidence |
| --- | --- | --- |
| Wave 4 source lock | PASS | Promoted and post-checked at digest `2677acb2...`. |
| Candidate browser package | PASS | Chromium, Firefox, and Playwright WebKit imported the generated package and executed all seven fixtures plus a structured error. Candidate measurements are in `fixtures/domain/v1/evidence/wave-4.json`. |
| Clean final manifest | PASS | `node scripts/domain-export/validate-manifest.mjs artifacts/domain-wasm/manifest.json`. |
| Final browser artifact | PASS | `run-browser-gates.ps1 -FinalArtifact` passed Chromium, Firefox, and Playwright WebKit against the clean-commit package and enforced all locked budgets. |
| Full Rust workspace | PASS WITH NOTE | `$env:RUST_TEST_THREADS='1'; cargo test --workspace` passed the complete workspace. The exact default-parallel command was run twice and failed because parallel app-core tests clear shared global session/diagnostic registries; that command is not claimed as passed. |
| Focused Flutter mobile | PASS | From `apps/flutter_mobile`, the named Study, Today, Wrong Words, and Reports suites passed 36/36. The first root-level plan command was invalid because the repository root has no `pubspec.yaml`; the corrected package-root command is recorded in `docs/features/learning.md`. |
| Wrong Words prerequisite | PASS WITH NOTE | Three stale red/yellow expectations were aligned to the already-rendered four-level contract. The focused Wrong Words and Reports tests each passed 1/1; the user-owned repair remains outside Phase 15 commits and is recorded in `docs/features/wrong-words-page.md`. |

## Release Build And Device

| Field | Status | Value |
| --- | --- | --- |
| Platform | PENDING | Android or iOS selected-device release flow |
| Device serial / identifier | PENDING | Must identify one explicit usable transport when ADB lists more than one |
| Fresh build identity | PENDING | Record APK/IPA path, timestamp, size, and SHA-256 |
| Install | PENDING | Record selected-device install result |
| Launch | PENDING | Record selected-device launch result |

## Human Acceptance Checklist

| Behavior | Status | Evidence to record |
| --- | --- | --- |
| NewWord with at least two words | PENDING | Four type-major questions per word; stable accepted meaning; phonetics/examples present; translation hidden before submit. |
| Review | PENDING | One question per entry, not the NewWord four-round loop. |
| Resume | PENDING | Exit after submitting, re-enter, and confirm the session does not restart at question one. |
| Today | PENDING | Progress and totals use question units. |
| Wrong Words | PENDING | Real word identity is shown after the exercised flow. |
| Reports | PENDING | Current local day and persisted result identity are shown. |

## Acceptance Record

Awaiting selected-device observation. Do not change this section to PASS from automated checks alone.
