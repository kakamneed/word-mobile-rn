# Phase 15 UAT: WASM-safe Mobile Domain Export

## Status

**Automation:** PASS WITH NOTE
**Release-device acceptance:** PASS WITH KNOWN ISSUES
**Phase completion:** PASS WITH KNOWN ISSUES

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
| Wave 4 source lock | PASS | Rechecked before device UAT at digest `2677acb2...`. |
| Candidate browser package | PASS | Chromium, Firefox, and Playwright WebKit imported the generated package and executed all seven fixtures plus a structured error. Candidate measurements are in `fixtures/domain/v1/evidence/wave-4.json`. |
| Clean final manifest | PASS | `node scripts/domain-export/validate-manifest.mjs artifacts/domain-wasm/manifest.json`. |
| Final browser artifact | PASS | `run-browser-gates.ps1 -FinalArtifact` passed Chromium, Firefox, and Playwright WebKit against the clean-commit package and enforced all locked budgets. |
| Full Rust workspace | PASS WITH NOTE | `$env:RUST_TEST_THREADS='1'; cargo test --workspace` passed the complete workspace. The exact default-parallel command was run twice and failed because parallel app-core tests clear shared global session/diagnostic registries; that command is not claimed as passed. |
| Focused Flutter mobile | PASS | From `apps/flutter_mobile`, the named Study, Today, Wrong Words, and Reports suites passed 36/36. The first root-level plan command was invalid because the repository root has no `pubspec.yaml`; the corrected package-root command is recorded in `docs/features/learning.md`. |
| Wrong Words prerequisite | PASS WITH NOTE | Three stale red/yellow expectations were aligned to the already-rendered four-level contract. The focused Wrong Words and Reports tests each passed 1/1; the user-owned repair remains outside Phase 15 commits and is recorded in `docs/features/wrong-words-page.md`. |
| Mobile release bridge | PASS | `cargo test -p word-platform-mobile` passed 50/50 before device UAT; existing unused-code warnings remain. |
| Checkpoint continuation rerun | PASS WITH NOTE | On 2026-07-29, `cargo test -p word-platform-mobile` passed 50/50 again. A fresh source-lock check exposed the expected post-promotion `docs/features/learning.md` drift after release-device evidence was appended. Two restricted browser attempts passed Chromium but failed while Firefox created a page; the same unchanged package then passed Chromium, Firefox, and WebKit together with browser subprocess permission, producing distinct `wave-4-uat.json` evidence. The ledger-only digest is re-promoted and the pin-ready artifact rebuilt from that exact clean commit so the final manifest does not retain a stale source-lock identity. |

## Release Build And Device

| Field | Status | Value |
| --- | --- | --- |
| Platform | PASS | Android release build installed and functionally accepted on `REA-AN00`. |
| Device serial / identifier | PASS | `adb-A2WDVB3526005032-plj2yB._adb-tls-connect._tcp`; `adb get-state` returned `device`. |
| Fresh build identity | PASS | `com.wordmobile` `1.0.3+4`; `apps/flutter_mobile/build/app/outputs/flutter-apk/app-release.apk`; 2026-07-28 21:15:19 +08:00; 114753646 bytes; SHA-256 `982097cab23462c2aa3762136abc9e21f3c74001d460f33823430fc21a938571`. The canonical script confirmed Supabase dart-defines and `lib/arm64-v8a/libword_platform_mobile.so`. |
| Install | PASS | Serial-pinned `adb install -r` returned `Success`; device package state reports `versionName=1.0.3`, `versionCode=4`, and `lastUpdateTime=2026-07-28 21:20:44`. |
| Launch | PASS | Serial-pinned launcher command injected one event for `com.wordmobile`; PID `1484` was running and `com.wordmobile/.MainActivity` was reported as the device top app. |

## Human Acceptance Checklist

| Behavior | Status | Evidence to record |
| --- | --- | --- |
| NewWord with at least two words | PASS | User reported the named release-device functionality normal on `1.0.3+4`. |
| Review | PASS | User reported the named release-device functionality normal on `1.0.3+4`. |
| Resume | PASS | User reported the named release-device functionality normal on `1.0.3+4`. |
| Today | PASS | User reported the named release-device functionality normal on `1.0.3+4`. |
| Wrong Words | PASS | User reported the named release-device functionality normal on `1.0.3+4`. |
| Reports | PASS | User reported the named release-device functionality normal on `1.0.3+4`. |

## Acceptance Record

On the fresh selected-device release `1.0.3+4`, the user reported all six named functional checks normal: NewWord, Review, resume, Today, Wrong Words, and Reports. Release-device acceptance is therefore **PASS WITH KNOWN ISSUES**, based on user observation rather than automated inference.

Two longstanding unresolved performance issues were also observed on this release: "键盘收起弹出卡顿" and "每日刚进入固定会有一次双击提交长卡顿". Based on the user's attribution, these issues predate Phase 15; Phase 15 neither introduced nor fixed them. They remain open residual risks and are not classified as Phase 15 regressions.
