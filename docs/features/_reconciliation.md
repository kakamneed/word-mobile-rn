# Feature Ledger Reconciliation

> Status: `baseline_reconciled`
> Updated: `2026-07-19`

## Purpose

Track which pre-existing worktree changes have been reconciled into feature ledgers. This is an operational index, not a product capability record and not proof that the mapped implementation is complete or verified.

## Reconciled Baseline

### Learning Flow

Ledger: `docs/features/learning.md`

- `apps/flutter_mobile/lib/features/study_screen.dart`
- `apps/flutter_mobile/lib/sdk/study_client.dart`
- `apps/flutter_mobile/test/study_client_test.dart`
- `apps/flutter_mobile/test/study_question_display_test.dart`
- `crates/app-core/src/facade.rs`
- `crates/app-core/src/facade/study_facade.rs`
- `crates/app-core/src/services/reports_service.rs`
- `crates/app-core/src/services/wrong_words_service.rs`
- `crates/app-core/src/lib.rs`
- `crates/domain-core/**`
- `crates/domain-protocol/**`
- `crates/domain-wasm/**`
- `crates/study-domain-runner/**`
- `scripts/domain-export/build-package.ps1`
- `scripts/domain-export/run-browser-gates.ps1`
- `tests/domain-browser/**`
- `fixtures/domain/v1/**`
- `artifacts/domain-wasm/**`
- `crates/platform-mobile/src/bridge.rs`
- `crates/storage-core/src/models/study_result.rs`
- `crates/study-core/src/session_summary.rs`
- `crates/study-core/src/answer_evaluator.rs`
- `crates/study-core/src/question_builder.rs`
- `crates/study-core/src/session_definition.rs`
- `crates/study-core/src/state_transition.rs`

### Wrong Word Graph

Ledger: `docs/features/wrong-word-graph.md`

- `apps/flutter_mobile/lib/features/wrong_word_graph_screen.dart`
- `apps/flutter_mobile/test/wrong_word_graph_screen_test.dart`

### Wrong Words Page

Ledger: `docs/features/wrong-words-page.md`

- `apps/flutter_mobile/lib/features/wrong_words_screen.dart`
- `apps/flutter_mobile/lib/features/learning_content_mode_selector.dart`
- `apps/flutter_mobile/test/wrong_words_screen_test.dart`
- `apps/flutter_mobile/test/exam_practice_wrong_words_test.dart`

### Reports Page

Ledger: `docs/features/reports-page.md`

- `apps/flutter_mobile/lib/features/reports_screen.dart`
- `apps/flutter_mobile/test/reports_screen_test.dart`
- `apps/flutter_mobile/lib/features/learning_content_mode_selector.dart`

### Exam Paper Import

Ledger: `docs/features/exam-paper-import.md`

- `apps/flutter_mobile/assets/exam-papers/**`
- `apps/flutter_mobile/assets/exam-dictionary/**`
- `apps/flutter_mobile/pubspec.yaml`
- `scripts/import-exam-papers.mjs`
- `scripts/import-exam-papers.test.mjs`
- `scripts/repair-exam-question-stems.mjs`
- `scripts/check-exam-paper-import.mjs`
- `scripts/apply-exam-paper-paragraph-translations.mjs`
- `scripts/exam-paper-paragraph-translations.json`
- `docs/exam-paper-import-report.json`

### Exam Practice And Vocabulary Intelligence

Ledger: `docs/features/exam-practice-vocab-intelligence.md`

- `crates/storage-core/src/models/exercise_vocab.rs`
- `crates/storage-core/src/models/mod.rs`
- `crates/storage-core/src/persistence/exercise_vocab_repo.rs`
- `crates/storage-core/src/persistence/mod.rs`
- `crates/storage-core/src/persistence/schema.rs`
- `crates/app-core/src/services/exam_practice_service.rs`
- `crates/app-core/src/services/mod.rs`
- `crates/platform-mobile/src/bridge.rs`
- `crates/platform-mobile/src/ai_agent.rs`
- `crates/platform-mobile/src/android.rs`
- `crates/platform-mobile/src/ios.rs`
- `crates/platform-mobile/include/word_platform_mobile_ios.h`
- `apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java`
- `apps/flutter_mobile/android/app/src/main/kotlin/com/wordmobile/flutter_mobile/MainActivity.kt`
- `apps/flutter_mobile/ios/Runner/AppDelegate.swift`
- `apps/flutter_mobile/lib/sdk/exam_practice_client.dart`
- `apps/flutter_mobile/lib/sdk/sdk.dart`
- `apps/flutter_mobile/lib/features/exam_practice_screen.dart`
- `apps/flutter_mobile/lib/features/exam_analysis_task_notifications.dart`
- `apps/flutter_mobile/lib/features/exam_paper_import_dialog.dart`
- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`
- `apps/flutter_mobile/lib/features/ai_screen.dart`
- `apps/flutter_mobile/test/exam_practice_client_test.dart`
- `apps/flutter_mobile/test/exam_practice_screen_test.dart`
- `apps/flutter_mobile/test/exam_paper_import_dialog_test.dart`
- `apps/flutter_mobile/lib/features/learning_content_mode_selector.dart`
- `apps/flutter_mobile/test/reports_screen_test.dart`
- `apps/flutter_mobile/test/wrong_words_screen_test.dart`
- `apps/flutter_mobile/test/exam_practice_wrong_words_test.dart`
- `scripts/check-exam-paper-import.test.mjs`
- `scripts/build-exam-corpus-dictionary.mjs`
- `scripts/build-exam-corpus-dictionary.test.mjs`
- `scripts/check-exam-corpus-dictionary.mjs`
- `scripts/android-exam-dictionary-packaging.test.mjs`
- `docs/exam-corpus-dictionary-report.json`

These entries include the current implementation stream. Files that also serve Today, AI workbench, graph, or import are intentionally documented in those capability ledgers as well; this index maps ownership, not exclusive authorship.

### Word Library Question Data Quality

Ledger: `docs/features/word-library.md`

- `apps/mobile/android/app/src/main/assets/seed-vocab/book/CET4_3.json`
- `apps/mobile/android/app/src/main/assets/seed-vocab/book/CET6_3.json`
- `apps/mobile/android/app/src/main/assets/seed-vocab/book/KaoYan_3.json`
- `apps/mobile/android/app/src/main/assets/seed-vocab/book/MEDICAL_RESP.json`
- `scripts/audit-seed-vocab-choice-conflicts.mjs`
- `scripts/repair-seed-vocab-choice-conflicts.mjs`
- `scripts/repair-seed-vocab-choice-conflicts-strict.mjs`
- `scripts/enrich-seed-vocab-real-exam-examples.mjs`
- `scripts/check-seed-vocab-real-exam-examples.mjs`
- `scripts/repair-seed-vocab-real-exam-meanings.mjs`
- `docs/seed-vocab-choice-conflicts-report.json`
- `docs/seed-vocab-choice-conflict-repair-report.json`
- `docs/seed-vocab-real-exam-examples-report.json`
- `docs/seed-vocab-real-exam-meaning-repair-report.json`
- `scripts/list-seed-vocab-choice-conflicts.mjs`
- `scripts/list-seed-vocab-choice-conflicts.test.mjs`
- `docs/seed-vocab-choice-conflict-inventory-apk-runtime.json`
- `docs/seed-vocab-choice-conflict-inventory-apk-runtime.md`
- `docs/seed-vocab-choice-conflict-inventory-baseline.json`
- `docs/seed-vocab-choice-conflict-inventory-baseline.md`
- `docs/seed-vocab-choice-conflict-inventory-current.json`
- `docs/seed-vocab-choice-conflict-inventory-current.md`
- `docs/seed-vocab-choice-conflict-inventory-worktree-runtime.json`
- `docs/seed-vocab-choice-conflict-inventory-worktree-runtime.md`

## Deliberate Exclusions

- `.codex-small.patch` and `.codex-today.patch`: temporary patch artifacts, not product contracts.
- `flutter_today_screen.png`: local screenshot evidence without a stable feature contract.
- `releases/*.apk`: release artifacts, tracked only by release/deployment workflows.
- `AGENTS.md`, `docs/features/_template.md`, and this file: ledger workflow infrastructure rather than user-facing features.
- `apps/flutter_mobile/build/**`: generated build output, including the verified release APK, is excluded from capability ledgers.

## Provenance Limits

This baseline was reconstructed from the working tree on `2026-07-15`. It does not establish who made each change, the exact change time, whether every file belongs to one atomic implementation, or whether tests passed before this reconciliation. Current verification belongs in each mapped feature record.

## Reconciliation Verification

- `python C:\Users\clf20\.codex\skills\.system\skill-creator\scripts\quick_validate.py C:\Users\clf20\.codex\skills\cross-platform-feature-ledger` passed.
- `git diff --check` passed for the ledger workflow and reconciled feature records.
- Current product checks and their failures are recorded in the mapped feature ledgers rather than treated as baseline proof.

## Desktop Consumption Workflow

- `2026-07-15` - Modification points: Added the manually invoked desktop skill `D:\projects\word-desktop-tauri\.agents\skills\consume-mobile-feature-ledger`. It reads the live mobile reconciliation index and one feature ledger, compares desktop code/planning, prioritizes mobile-leading verified behavior, converts mobile lessons and pitfalls into desktop prevention gates, and writes only a hash-pinned desktop consumption assessment.
- `2026-07-15` - Priority rule: Desktop conversion must first implement capability already leading on mobile and prevent known mobile failures. Desktop-specific layout enhancements come only after parity, shared-contract reuse, and pitfall prevention.
- `2026-07-15` - Problems encountered: The initial Node-based freshness inspector could not be verified because the local Node executable repeatedly timed out even for `node --version`. The skill now uses a PowerShell-native inspector, removing that runtime dependency for this Windows desktop repository.
- `2026-07-15` - Verification: The desktop skill passed `quick_validate.py`. A live `wrong-word-graph` inspection detected mobile status `mobile_in_progress`, source updated `2026-07-15`, and a missing desktop consumption record; a temporary hash-matched record then passed `-Check` with `consumptionState: current`.
- `2026-07-28` - Route change: Word Net becomes the single maintained PC codebase for browser PWA and Tauri delivery. Mobile remains the source of truth for shared learning behavior, exposed through a pinned WASM-safe Rust boundary.
- `2026-07-28` - Preservation rule: `word-desktop-tauri` stays available as a hash-pinned donor and verification source until Word Net reproduces the accepted behavior and release gates. It must not evolve into a competing third implementation.
- `2026-07-28` - Phase 15 Wave 4 mapping: the generated-package builder, three-browser harness, final manifest/package, and their evidence belong to the Learning Flow ledger because they preserve the shared mobile learning contract for Word Net consumption. Ignored candidate output, Playwright reports, browser binaries, temporary clean worktrees, and release APKs remain deliberate build/runtime exclusions.
