---
name: word-mobile-study-flow-map
description: Diagnose and modify this project's Flutter Today page, study session, wordbook, progress, resume, mastered-word, and choice-answer logic. Use for Word Mobile bugs involving Today progress, study modes, question generation, option grading, continuation, wrong answers, wordbooks, and progress inconsistency.
---

# Word Mobile Study Flow Map

Use this project-local skill only in `D:\projects\word-mobile-rn`.

## Current Canonical Reference

Before changing the Flutter learning flow, read `.planning/skills/learning-flow/INDEX.md`.
That Phase 11 index is the current Flutter-only canonical entry point for Today,
Study answering, AI, Wrong Words, Reports, bridge/data persistence, leaderboard,
image vote/upload, release validation, known pitfalls, and regression guardrails.

This skill remains useful as supporting study-flow detail, but React Native paths
are legacy context only unless the user explicitly asks for React Native.

## Mental Model

Treat the app as five coupled layers:

1. **Plan and Today targets** decide what should be studied today.
2. **Wordbook and candidate hydration** choose concrete entries.
3. **Study session state** owns question order, progress, resume, and answered history.
4. **Question builder and answer evaluator** own options, correct answers, and grading.
5. **Flutter study UI** displays feed pages, selected state, feedback, and navigation.

Never fix one layer without checking the adjacent layer that consumes it.

## Key Files

Flutter:

- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`
- `apps/flutter_mobile/lib/features/study_screen.dart`
- `apps/flutter_mobile/lib/sdk/study_client.dart`
- `apps/flutter_mobile/test/study_question_display_test.dart`

Rust app/core:

- `crates/platform-mobile/src/bridge.rs`
- `crates/app-core/src/facade/study_facade.rs`
- `crates/study-core/src/question_builder.rs`
- `crates/study-core/src/answer_evaluator.rs`
- `crates/study-core/src/session_definition.rs`

Storage:

- `crates/storage-core/src/models/study_question.rs`
- `crates/storage-core/src/models/study_requests.rs`
- `crates/storage-core/src/persistence/schema.rs`
- `crates/storage-core/src/persistence/mastered_entry_repo.rs`

## Study Modes and Counts

- `newWord`: one word becomes four fixed question types.
- `review`: similar multi-question groups; do not let partial word groups linger after mastered/deleted.
- `mixedTest`, `wrongWordReinforcement`, `rootAffix`: usually one unit maps closer to one question.
- Today progress is not the same thing as current feed page index.
- Study page progress should reflect the visible page when swiping answered history.

## Resume and Progress Invariants

Keep these invariants true:

- Entering a specific mode must start or resume that mode, not whichever unfinished mode exists first.
- `getResumeSessionHint` can suggest a mode, but UI entry points must not overwrite the user-requested mode.
- `try_resume_empty_start_request` must resume only if an active snapshot exists for the requested mode.
- Answering questions should not double-count Today progress when re-entering a partially completed session.
- `answeredQuestions` should contain stable question/result pairs for already answered feed pages.
- Returning to Today should use persisted `study_results` and active session snapshots consistently.

When a bug says "Today shows 4/20 but study opens 1/20" inspect:

1. `_openStudy` in Flutter entry points.
2. `StudyScreen._startWithBestAvailableSeed`.
3. `try_resume_empty_start_request` in `platform-mobile`.
4. `word_app_core::start_study_session` snapshot matching.
5. `active_study_session_*` app settings.

## Choice Options and Grading

Correctness must be derived from authoritative question content, not stale labels.

Backend rules:

- `AnswerEvaluator` should resolve the correct choice from:
  - `question.word` for `cnToEnChoice`.
  - exact normalized `accepted_meanings` vs choice text for Chinese choice questions.
  - `correct_choice_label` only as fallback.
- Do not accept arbitrary text as a choice response unless the UI intentionally sends text.
- Add regression tests when stale `correctChoiceLabel = A` would otherwise mark A correct.

Frontend rules:

- The UI may need to display old results whose `userResponse` is a label, text, or empty.
- Mark correct option by matching option text against accepted meanings or correct answer.
- Mark wrong selected option by matching both option label and option text against `result.userResponse`.
- For wrong answers, show correct option green/check and selected wrong option red/X.
- For skipped or "show answer", still display the correct answer.

Useful test surface:

- `choiceDisplayForTest`
- `choiceStateForTest`
- `studyHeroDisplayForTest`
- `wordSkeletonDisplayForTest`

## Mastered / Trash Behavior

Mastered entries must be excluded everywhere:

- New word pool.
- Review pool.
- Wrong word book.
- AI/wrong-word derived contexts.
- Active session remaining questions for that entry.

If a word is mastered mid-session:

- Cancel the remaining unanswered question types for that same entry.
- Persist the active session shape after removal.
- Prefer trash/mastered over wrong-word records.

Check:

- `mastered_entries` storage.
- `mark_study_entry_mastered`.
- wrong-word loaders in `platform-mobile`.
- question candidate hydration filters.

## Wordbook and Candidate Hydration

Wordbooks affect which entries can be selected, but Today progress and active session state may outlive a wordbook selection change.

When changing wordbook logic:

- Check selected wordbook settings.
- Check `today_wordbooks_json` and active wordbook settings.
- Verify hydrate request entry IDs are concrete real entries, not restore placeholders.
- Clear stale active snapshots only when a plan/wordbook change invalidates the current active session.
- Do not delete historical `study_results` to fix a current-session mismatch.

## UI Feed Flow

The Douyin-like study screen should follow these rules:

- Swipe vertically between answered pages and the current unanswered page.
- Do not auto-advance after answer submission.
- Answered pages are read-only and show answer state.
- The visible progress pill follows the swiped page index.
- Choice: tap selects, double-tap submits.
- Input: Enter submits.
- "Show answer" submits an empty/skipped answer and reveals the answer on the current page.
- Side buttons should not cover question or options.

## Debug Procedure

1. Reproduce in the smallest mode and entry count.
2. Identify whether the mismatch is UI-only, active snapshot, persisted results, candidate hydration, or grading.
3. Inspect concrete payloads:
   - `StartSessionResponse.progress`
   - `answeredQuestions`
   - `currentQuestion`
   - `StudyResult.userResponse`
   - `StudyResult.correctAnswer`
   - `StudyQuestion.choices`
   - `StudyQuestion.acceptedMeanings`
4. Fix the earliest wrong source.
5. Add a narrow regression test.
6. Run focused tests before broader checks.

## Verification Commands

Prefer focused commands:

```powershell
cargo test -p word-study-core choice_uses_actual_correct_text_when_saved_label_is_stale
cargo test -p word-study-core
cargo test -p word-platform-mobile empty_start_request
D:\flutter\flutter\bin\flutter.bat analyze --no-pub
D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_question_display_test.dart -r expanded
git diff --check
```

If `flutter test` hangs with no output but `flutter analyze` succeeds, report it as unverified rather than silently treating it as passed.

## Final Response Checklist

For study-flow work, summarize:

- Which invariant was broken.
- Which layer was fixed.
- Which tests/checks ran.
- Any remaining unverified runtime behavior, especially if Flutter test runner timed out.
