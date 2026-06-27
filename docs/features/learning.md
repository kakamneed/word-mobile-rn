# Feature: Learning Flow

> Slug: `learning`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

The Learning Flow is the user-facing study surface launched from Today. It covers new-word study, review, mixed test, wrong-word reinforcement, roots/affixes, example-based questions, input questions, and word skeleton completion.

The durable intent from the conversation history is simple: every mode must use the real plan/wordbook source, render trustworthy questions and answers, and settle progress so Today never shows stale incomplete work after a mode has actually finished.

## UX Contract

- Study is a vertical feed or single-question progression where answered questions can be reviewed but not resubmitted.
- Choice questions are single-select. Tapping one option must not visually select all four options.
- Choices must be stable, unique, non-empty, and tied to one authoritative correct answer. The UI must never show empty `/` choices.
- Choice correctness must use the unique correct label/text for the question, not a broad meaning list that can point at a distractor.
- Input questions must show the correct answer after a wrong response. If `StudyResult.correctAnswer` is empty, mobile must fall back to `StudyQuestion.acceptedMeanings`.
- Word skeleton questions must not reveal the full word in the hero or prompt. The user enters only the missing letters, and those letters fill the underscores in place.
- Skeleton blanks should be visually separated, for example `f_ _dge`, so the user knows how many letters to enter.
- AI hint affordances must reflect availability: disabled/default color when no saved hint is available, theme color and clickable only when a hint exists.
- Study progress must display `current question / session total`. It must not use visible feed count, which caused states like `17/17` followed by `17/18`.
- Completing one mode and moving to another must leave Today showing the completed mode as complete, not stale values like `19/20` or `0/4`.

## Shared Domain/Data Contract

The shared Rust/domain layer is authoritative for session state and must be reused by mobile and desktop.

- `StartSessionRequest` and `StartSessionEntryPayload` must be populated from real plan/wordbook entries, not test fixtures or stale restore data.
- `StartSessionEntryPayload` carries `cn_choice_distractors` and `en_choice_distractors` for richer distractor selection.
- `StudyQuestion` must carry renderable state: `questionId`, `questionType`, `entrySourceId`, `word`, `prompt`, `acceptedMeanings`, `choices`, `correctChoiceLabel`, `questionIndex`, `totalQuestions`, `hasHint`, and `userHint` when relevant.
- `SubmitAnswerResponse` must return `result`, `isComplete`, `currentQuestion`, `summary`, `nextAction`, `progress`, and `answeredQuestions`.
- `StudyResult.correctAnswer` should be displayable. Choice questions should resolve to the unique correct option; input questions should expose the expected text answer.
- Active session snapshots represent unfinished sessions only. A final submit must persist a completed session and clear the active snapshot.
- `save_completed_session` and `clear_persisted_session` are part of the final-submit path, not only explicit `completeSession`.
- Mastered, skipped, disputed meaning, and accepted meaning changes can affect the current session, future question pools, Today progress, wrong words, and AI inputs.

## Flutter Mobile Route

- Owner screen/widget: `apps/flutter_mobile/lib/features/study_screen.dart` / `StudyScreen`.
- SDK/bridge calls: start/resume study session, submit answer, complete/cancel, mark mastered, accept disputed meaning, hint, comments, reveal answer.
- Loading/cache/reload behavior: when Study closes or completes, `MobileRootShell` must invalidate Today/Wrong/Reports/AI-related caches so Today does not render stale active snapshots.
- Orientation/gesture constraints: portrait-first mobile layout; avoid overlapping question content, options, bottom actions, captions/overlays, and keyboard.
- Current status: mobile in progress. Core session and feedback issues have been patched, but real-device smoke across all modes is still required.

Focused mobile checks from the conversation history:

- New-word study must not keep using the test-pool/front words such as `surgeon`, `cancel/concel`, and `explosive` across days/builds.
- Roots/affixes must not keep returning the same first items such as `ab` and `abl`, and must never show internal keys like `active_session_restore_0`.
- Verify each active type: CN-to-EN choice, EN-to-CN choice, example-to-CN choice, pure-example choice, CN input, word skeleton, and roots/affixes input.
- Selecting one choice should select only that option and enable submit.
- Wrong input feedback must include the correct answer.
- Skeleton example: `f_ _dge` accepts `ri` and renders `fridge`.
- After finishing a mode and entering another mode, returning to Today must still show the first mode complete.

## Tauri Desktop Route

- Owner view/window: desktop study session view inside the desktop learning workspace.
- Shared APIs to reuse: Rust session facade, question builder, answer evaluator, study repository, mastered/disputed meaning repositories.
- Desktop-specific layout: prefer a left question/history rail and right active-question/detail panel. Use keyboard-first operations instead of copying the phone feed exactly.
- Mobile assumptions to avoid: no mobile bottom-button layout, no small-screen lyric/overlay constraints, no forced swipe interaction.
- First parity slice: start/resume session, submit answer, completed-session persistence, answered history, correct-answer feedback, Today progress refresh.
- Current status: planned.

## Sync And Storage

- Local `study_sessions`, `study_results`, and active session snapshots drive Today progress.
- Active snapshots must only survive for unfinished sessions. Completed modes must write a completed row and delete `active_study_session_*`.
- Supabase/cloud sync for account, wrong words, accepted disputed meanings, hints, and study results can be eventually consistent, but local state should become correct immediately.
- Today progress should be derived from authoritative completed records plus active-session seeds, never from stale snapshots alone.

## AI Or Provider Implications

- AI hint UI is not a blanket affordance. It should only be active when a saved/displayable hint exists.
- High-error words may generate hint suggestions, but the UI must distinguish suggested/unavailable from saved/clickable.
- AI and wrong-word inputs must filter mastered/excluded entries so already-mastered words do not keep entering AI study material.

## Implementation Log

- `2026-05-09`: Added `answeredQuestions` to study responses so mobile can review answered feed items.
- `2026-05-09`: Added mastered persistence/API/current-session pruning/future filtering.
- `2026-04 to 2026-06`: Repeatedly investigated new-word study returning fixed test/front words instead of the real wordbook/plan source.
- `2026-04 to 2026-06`: Investigated roots/affixes stale snapshot pollution, including internal values like `active_session_restore_0`.
- `2026-05`: Adding example-based and skeleton question types regressed old modes: all options selected, submit disabled, answer mismatch, and weak distractor randomness.
- `2026-05`: Distractor generation was tightened so the app does not cycle through only four or five option sets and does not emit empty `/` options.
- `2026-05`: Choice correctness was corrected toward unique option labels/text after cases like `condemn` showing the correct meaning in choices but grading a distractor as correct.
- `2026-05`: Skeleton questions changed from full-word input to missing-letter input, with masked hero/prompt display.
- `2026-05`: Study progress was corrected from visible feed count to session total to avoid `17/17 -> 17/18`.
- `2026-06-25`: Final submit path was fixed to save completed session, clear active snapshot, and remove in-memory active session.
- `2026-06-25`: Input-question feedback now falls back from empty `correctAnswer` to `acceptedMeanings`.
- `2026-06-25`: Mojibake in Study UI/tests was cleaned and future Chinese literals should be written through UTF-8-safe paths or Unicode escapes.

## Mobile Lessons Learned

- Seeing questions on Study is not enough; always verify the source is the real wordbook/plan, not a test pool or fixed sorted prefix.
- If a rebuilt package still shows the same first words, inspect active snapshots and restore paths before only changing question builder logic.
- Today progress and Study progress are two views of the same session state. Submit-final and explicit-complete must share completion semantics.
- New question types are high-risk for old interactions: selection state, submit enablement, answer display, and choice uniqueness need regression coverage.
- Distractor randomness needs a real candidate pool and dedupe rules; small hard-coded-looking sets are visible to users quickly.
- Mobile feedback must not assume backend `correctAnswer` is always populated.
- Skeleton answer semantics are missing letters, not full words.
- On Windows PowerShell, writing non-ASCII literals can corrupt files; prefer UTF-8-safe tooling or ASCII Unicode escapes for source edits.

## Desktop Follow-Up Notes

- Desktop should start from the corrected shared session contract, not from the older mobile bugs.
- Reuse final-submit completion, active-snapshot semantics, input feedback fallback, and skeleton missing-letter behavior exactly.
- Desktop can expose `answeredQuestions` as a persistent side history for review and correction.
- Keyboard-first defaults: number keys for choices, Enter submit, Esc exit, Tab movement.
- Do not copy the mobile AI hint confusion. Desktop can present hints/comments/dispute actions in a right-side tool area with explicit disabled states.

## Route Changes

- `2026-05-08`: Learning flow moved from card-style study to feed-style reviewable study.
- `2026-05`: Question types expanded from basic bilingual choices to examples, pure examples, inputs, and skeleton completion.
- `2026-05`: Skeleton completion moved from full-word answer to missing-letter answer.
- `2026-06`: Completion route expanded so submitting the final question completes and clears the active snapshot.

## Known Pitfalls

- Do not let mock/test payloads or old active snapshots override the real wordbook.
- Do not leave active snapshots after completed sessions.
- Do not use feed item count as session total.
- Do not emit `/`, empty strings, or duplicate meanings as choices.
- Do not use broad accepted meanings as the only source for choice correctness.
- Do not reveal full words in skeleton heroes/prompts.
- Do not show clickable AI hints for words without available hints.
- Do not write Chinese source/docs through Windows default encoding.

## Verification

- Mobile:
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --plain-name "input feedback correct answer falls back"` was blocked by missing local Pub cache dependencies: `async`, `vector_math`, `shared_preferences`, `supabase_flutter`, and others.
  - `D:\flutter\flutter\bin\dart.bat format ...` was blocked by missing `flutter_lints-6.0.0/lib/flutter.yaml`.
  - Still required: real-device smoke for all modes, single-choice state, input feedback, skeleton missing letters, and Today progress after mode completion.
- Desktop:
  - Pending; desktop learning page is not implemented yet.
- Shared/domain:
  - `cargo fmt --check` passed on 2026-06-25.
  - `cargo test -p word-app-core final_submit_persists_completed_session_and_clears_resume_snapshot -- --nocapture` passed on 2026-06-25.
  - `git diff --check -- apps/flutter_mobile/lib/features/study_screen.dart apps/flutter_mobile/test/study_question_display_test.dart crates/app-core/src/facade/study_facade.rs crates/app-core/tests/baseline_runner.rs` passed on 2026-06-25.
