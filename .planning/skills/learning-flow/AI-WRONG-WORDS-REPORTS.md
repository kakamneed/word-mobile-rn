# AI, Wrong Words, And Reports

## Purpose

Use this guide when changing AI passage generation/history/import, Today AI shortcut, AI wrong-word image/source import, Wrong Words, report summaries, or non-blocking AI error behavior.

## Canonical Files

Flutter:

- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `apps/flutter_mobile/lib/features/ai_screen.dart`
- `apps/flutter_mobile/lib/features/wrong_words_screen.dart`
- `apps/flutter_mobile/lib/features/reports_screen.dart`
- `apps/flutter_mobile/lib/sdk/ai_client.dart`
- `apps/flutter_mobile/lib/sdk/wrong_words_client.dart`
- `apps/flutter_mobile/lib/sdk/reports_client.dart`
- `apps/flutter_mobile/test/ai_passage_generation_test.dart`
- `apps/flutter_mobile/test/ai_wrong_word_import_test.dart`

Rust/data:

- `crates/platform-mobile/src/bridge.rs`
- `crates/app-core/src/services/reports_service.rs`
- `crates/app-core/src/services/wrong_words_service.rs`
- `crates/storage-core/src/persistence/schema.rs`

## Workflow

1. Keep AI optional and non-blocking. Today and Study must remain usable when AI provider calls fail.
2. Route Today AI shortcut and `AiScreen` through `sdk.ai` / `AiClient`.
3. Let Flutter own source selection, review UI, loading state, and visible error display.
4. Keep extraction truth, import commit, wrong-word persistence, report aggregation, and AI history persistence on typed client/Rust paths.
5. Verify Wrong Words and Reports after Study completion by persisted state, not temporary UI counters.

## Invariants

- AI failure is visible and recoverable, but never blocks the core study loop.
- Wrong-word import commits must appear in Wrong Words through `WrongWordsClient` state.
- Reports must use `ReportsClient.getReportsOverview`.
- Wrong-word hints/mastered/trash actions must persist after reload.
- AI context should not resurrect deleted/trash/mastered entries as active wrong words.

## Verification

Run or record:

```powershell
cargo test -p word-app-core reports --lib
cargo test -p word-platform-mobile wrong --lib
cargo test -p word-platform-mobile report --lib
cargo test -p word-platform-mobile wrong_word_image_analysis_uses_backup_after_primary_failure --lib -- --nocapture
D:\flutter\flutter\bin\flutter.bat test --no-pub test\ai_passage_generation_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\ai_wrong_word_import_test.dart -r expanded
```

For grouped Rust AI filters, mock-server interference can occur. If a broad grouped filter fails, rerun the specific failing test alone and document both outcomes.

## Stale Paths To Avoid

- UI-local mock AI history as active truth.
- Page-local report counters as persisted report truth.
- Wrong-word notebook mutation that bypasses `WrongWordsClient`/Rust persistence.
- Blocking Today/Study navigation on AI sync or provider availability.
