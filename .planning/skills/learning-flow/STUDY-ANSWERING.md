# Study Answering And Feedback

## Purpose

Use this guide when changing Study cards, selected option display, input submit, answer evaluation, question builder, resume history, wrong-answer persistence, or the two historical hard bugs.

## Canonical Files

Flutter:

- `apps/flutter_mobile/lib/features/study_screen.dart`
- `apps/flutter_mobile/lib/sdk/study_client.dart`
- `apps/flutter_mobile/lib/bridge/bridge_error.dart`
- `apps/flutter_mobile/test/study_question_display_test.dart`
- `apps/flutter_mobile/test/study_client_test.dart`

Rust:

- `crates/platform-mobile/src/bridge.rs`
- `crates/app-core/src/facade/study_facade.rs`
- `crates/study-core/src/question_builder.rs`
- `crates/study-core/src/answer_evaluator.rs`
- `crates/study-core/src/session_definition.rs`
- `crates/storage-core/src/models/study_question.rs`
- `crates/storage-core/src/models/study_requests.rs`

## Workflow

1. Determine whether the issue is UI rendering, Dart DTO decoding, bridge payload, Rust session state, question generation, or answer evaluation.
2. For choice bugs, inspect `StudyQuestion.correctChoiceLabel`, `StudyQuestion.choices`, `StudyResult.userResponse`, and `StudyResult.correctAnswer` together.
3. Keep answer correctness in Rust/domain logic. Flutter may reconcile display tokens for old results but must not invent correctness.
4. Preserve the result-driven feedback rhythm: no correctness styling before submission; after result, correct option green/check and selected wrong option red/cancel.
5. When clearing stale persisted snapshots, ensure empty resume requests cannot fall through into unrelated in-memory sessions.

## Historical Hard Bugs

### Selected wrong option not red

Required behavior:

- If the user selects the wrong A/B/C/D option and submits, that selected option is red/cancel.
- The actual correct option is green/check.
- Matching must tolerate label/value/text shaped historical `userResponse` values.

### Correct answer drifts to A

Required behavior:

- Choice questions must carry a valid `correctChoiceLabel`.
- Missing or invalid `correctChoiceLabel` is a protocol error, not a fallback to A.
- Non-A correct labels must survive question builder, bridge DTO, Dart decode, submit, answered history, and resume.

## Verification

Run or record:

```powershell
cargo test -p word-app-core study --lib
cargo test -p word-study-core --lib
D:\flutter\flutter\bin\flutter.bat analyze --no-pub
D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_question_display_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_client_test.dart -r expanded
```

If Flutter tests time out, record them as `BLOCKED`.

## Stale Paths To Avoid

- `correctChoiceLabel ?? 'A'` or equivalent default-to-A behavior.
- Demo/sample payloads as production answer truth.
- UI-only fixes that hide stale Rust/session payloads.
- Deleting historical `study_results` to fix current-session mismatch.
