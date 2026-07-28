# Feature: Learning Flow

> Slug: `learning`
> Status: `mobile_in_progress`
> Updated: `2026-07-16`

## Product Intent

The Learning Flow is the user-facing study surface launched from Today. It covers new-word study, review, mixed test, wrong-word reinforcement, roots/affixes, example-based questions, input questions, and word skeleton completion.

The durable intent from the conversation history is simple: every mode must use the real plan/wordbook source, render trustworthy questions and answers, and settle progress so Today never shows stale incomplete work after a mode has actually finished.

## UX Contract

- Study is a vertical feed or single-question progression where answered questions can be reviewed but not resubmitted.
- After answer submission, the answered page must remain stable. The next question may be prepared as a pending feed item, but it must not be auto-jumped into view or followed by a forced jump back to the old current question.
- Choice questions are single-select. Tapping one option must not visually select all four options.
- Choices must be stable, unique, non-empty, and tied to one authoritative correct answer. The UI must never show empty `/` choices.
- Choice correctness must use the unique correct label/text for the question, not a broad meaning list that can point at a distractor.
- Input questions must show the correct answer after a wrong response. If `StudyResult.correctAnswer` is empty, mobile must fall back to `StudyQuestion.acceptedMeanings`.
- Word skeleton questions must not reveal the full word in the hero or prompt. The user enters only the missing letters, and those letters fill the underscores in place.
- Skeleton blanks should be visually separated, for example `f_ _dge`, so the user knows how many letters to enter.
- AI hint affordances must reflect availability: disabled/default color when no saved hint is available, theme color and clickable only when a hint exists.
- Study progress must display `current question / session total`. It must not use visible feed count, which caused states like `17/17` followed by `17/18`.
- Completing one mode and moving to another must leave Today showing the completed mode as complete, not stale values like `19/20` or `0/4`.
- Lazy question generation may treat future unanswered questions as a black box, but after app exit, restart, or process kill, it must preserve answered content and the current active question exactly. New-word study is excluded from this relaxed future-question contract until its fixed four-type learning pattern is handled separately.

## Shared Domain/Data Contract

The shared Rust/domain layer is authoritative for session state and must be reused by mobile and desktop.

- `StartSessionRequest` and `StartSessionEntryPayload` must be populated from real plan/wordbook entries, not test fixtures or stale restore data.
- `StartSessionEntryPayload` carries `cn_choice_distractors` and `en_choice_distractors` for richer distractor selection.
- `StudyQuestion` must carry renderable state: `questionId`, `questionType`, `entrySourceId`, `word`, `prompt`, `acceptedMeanings`, `choices`, `correctChoiceLabel`, `questionIndex`, `totalQuestions`, `hasHint`, and `userHint` when relevant.
- `SubmitAnswerResponse` must return `result`, `isComplete`, `currentQuestion`, `summary`, `nextAction`, `progress`, and `answeredQuestions`.
- `StudyResult.correctAnswer` should be displayable. Choice questions should resolve to the unique correct option; input questions should expose the expected text answer.
- Active session snapshots represent unfinished sessions only. A final submit must persist a completed session and clear the active snapshot.
- `save_completed_session` and `clear_persisted_session` are part of the final-submit path, not only explicit `completeSession`.
- A lazy-generation snapshot must store enough deterministic session-plan state to rebuild answered questions and the current question exactly after process death. Future unanswered questions do not need to be pre-materialized or byte-identical before the user reaches them, except for new-word study until a separate new-word contract is defined.
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
- Today plan targets are the authoritative task target shown on Today/Plan and must not be silently rewritten by an active or completed Study session. If generation can only produce fewer valid renderable questions than the plan target, the Study/session layer must repair, downgrade, or explicitly surface that shortage instead of mutating Today from `34` to `33`.
- Study session totals and completion summaries must use one session denominator. Reports must dedupe duplicate result rows and never summarize retry rows as states such as `34/33`.

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
- `2026-07-15` - Baseline reconciliation: The dirty worktree contains a V2 active-session snapshot contract that persists entry payloads, distractors, progress, and a question-plan signature; invalid, stale, previous-day, placeholder, or undecodable snapshots are cleared and surfaced through recent study diagnostics.
- `2026-07-15` - Baseline reconciliation: Study questions are validated before use, duplicate answer submission is idempotent, the answered feed is windowed with an older-history API, and mode-specific question-type weights no longer leak across new-word, review, and root-affix sessions.
- `2026-07-15` - Problems encountered: Historical changes indicate stale/corrupt snapshots, mismatched choice labels, duplicate submissions, and cross-mode progress leakage were all active failure modes. Exact authorship and prior test status cannot be inferred from the worktree, so current verification is recorded separately.
- `2026-07-15` - Problems encountered: Running all 28 `study_facade` tests in parallel produced `NoActiveSession` in two tests because the suite shares process-global active-session state; both failing scenarios passed when rerun individually. Test isolation remains a reliability gap even though the focused product behaviors pass.
- `2026-07-15` - Route decision: Future lazy question generation may defer all unanswered future questions, but it must persist or deterministically rebuild every answered question and the current question across app restart and killed-background-process resume. New-word study remains a separate design slice because its fixed learning-pattern sequence has stricter expectations.
- `2026-07-15` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now blocks `_jumpToCurrentFeedPage` while `_pendingNextQuestion` exists, so submit does not trigger an automatic page jump or later pull the user back to the answered page. `apps/flutter_mobile/test/study_question_display_test.dart` covers the pending-next no-auto-jump guard.
- `2026-07-16` - Diagnosis: The recurring Today/Study/report mismatch could be caused by three different denominators: Today targets from the plan seed, Study totals from the actual generated question list, and summaries from `study_results.len()`. Duplicate/stale result rows could therefore produce states such as a 34-plan target, a 33-question session, and a `34/33` report.
- `2026-07-16` - Modification points: `crates/storage-core/src/models/study_result.rs` now supports summary generation with an explicit question-plan total, dedupes results by `question_id`, and caps counted results to that total. `crates/study-core/src/session_summary.rs` exposes `build_summary_with_total`, and `crates/app-core/src/facade/study_facade.rs` uses the active session question count for final submit, mastered completion, and explicit completion summaries.
- `2026-07-16` - Superseded approach: `crates/platform-mobile/src/bridge.rs` briefly aligned Today targets to active snapshot `questions.len()` or completed-session distinct question counts. Later testing showed this caused Today/Plan target drift such as `34 -> 33`, so that target-rewrite behavior was removed while result-row dedupe was retained.
- `2026-07-16` - Diagnosis: A Study screen showing `a.` as both hero word and example prompt came from data/domain validation, not Flutter rendering. `KaoYan_3.json` contains abbreviation-like headwords such as `a.` and `vs.`, and the domain validator previously accepted any alphanumeric token, so those entries could become normal study targets.
- `2026-07-16` - Modification points: `crates/app-core/src/facade/study_facade.rs` now requires study target words to look like real word tokens: at least two alphabetic characters and no dotted abbreviation marker. The same guard applies to entry payload filtering, generated question validation, V1 snapshot rejection, and V2 snapshot rebuild filtering.
- `2026-07-16` - Diagnosis: The latest `34 -> 33` Today/Study mismatch showed that aligning Today targets to generated/completed session question counts was the wrong repair layer. It masked generation shortages by changing the user's plan-visible task total after entering or leaving Study.
- `2026-07-16` - Modification points: `crates/platform-mobile/src/bridge.rs` no longer lets active or completed study sessions rewrite Today target totals; completion/progress still dedupes result rows, but the plan target remains stable.
- `2026-07-16` - Modification points: `crates/app-core/src/facade/study_facade.rs` now repairs incomplete choice questions before validation. Choice questions require exactly four unique options with a valid correct label; incomplete choice questions are downgraded to input questions so the UI does not show a missing D option and the session count is not shrunk.
- `2026-07-16` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now renders the completion page progress pill from the completion summary total, so a completed 28-question session displays `28/28` instead of the last visible feed index such as `21/28`.
- `2026-07-17` - Diagnosis: The remaining auto-slide bug became stable from the 21st question because the visible feed keeps a roughly 20-item answered window. When the answered list window shifted after submit, `PageView.onPageChanged` could observe the same page index now pointing at the pending next question and falsely treat that rebuild as a user swipe.
- `2026-07-17` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now tracks real user drag start/end through `ScrollNotification`. `onPageChanged` only updates visible page/focus state; pending next-question activation happens only after a user-driven scroll settles on the pending page. `apps/flutter_mobile/test/study_question_display_test.dart` covers the 20-answered-window boundary.
- `2026-07-17` - Diagnosis: Today/Plan totals could still exceed Study entry totals by one in non-new-word modes because platform hydrate selected exactly the target number of entry IDs, then app-core filtered invalid payloads such as dotted abbreviation headwords before question generation.
- `2026-07-17` - Modification points: `crates/platform-mobile/src/bridge.rs` now overfetches study candidates, applies the same bridge-side validity guards for source ID, study word, and meanings, and truncates the valid payload list back to the Today target before starting Study. `apps/flutter_mobile/lib/features/study_screen.dart` also corrects the visible PageView item back to the just-submitted question when a pending next question is appended, preventing feed-window shifts from landing on the next page automatically. `apps/flutter_mobile/test/study_question_display_test.dart` and `crates/platform-mobile/src/bridge.rs` cover these regressions.

- `2026-07-28` - Phase 15 domain-export modification points: `scripts/domain-export/check-source-lock.ps1` establishes a hash gate over the current Flutter-backed Rust question/session/report/wrong-word behavior, the mobile bridge, the focused Flutter display contract, and this ledger. Wave 0 capture is the initial mobile truth; later accepted digests require a reviewed machine-readable diff, successful parity evidence tied to the old digest, and a changed learning-ledger digest. Generated Phase 15 fixtures and scripts are deliberately excluded from the authoritative product-source set.

- `2026-07-28` - Phase 15 canonical-fixture modification points: `crates/app-core/tests/baseline_runner.rs` now derives seven deterministic native outputs from explicit `nowUtc`, `localDay`, `sessionId`, and ordering-seed requests under `fixtures/domain/v1/`. The corpus locks NewWord's four type-major rounds, every non-NewWord mode's one-question-per-entry rule, pre-submit translation hiding and post-submit feedback, progress/summary carry-over, resume history, real wrong-word identity, and report local-day/mode filtering. `scripts/domain-export/capture-native-fixtures.ps1` requires `-AcceptCurrentMobileTruth` before overwriting expected JSON; `scripts/domain-export/compare-semantic-json.mjs` ignores object-key order while preserving array order and values.
- `2026-07-28` - Phase 15 baseline repair: the serial app-core suite exposed older runner assertions that explicitly completed sessions after final submit and resumed with a newly supplied source set. The owned baseline runner now asserts final-submit completion/persistence and resumes the saved deterministic input-question plan with an empty request; product implementation code was not changed.
- `2026-07-28` - Phase 15 model-ownership modification points: `crates/domain-models` now owns the serde-only study, plan/progress, resume, wrong-word, and Today projection DTOs. `crates/storage-core/src/models/mod.rs` and the former storage model modules re-export those canonical types so app-core, platform-mobile, and Flutter-facing JSON paths retain their existing contracts. `crates/study-core/Cargo.toml` resolves its preserved source-level compatibility import to the pure `word-domain-models` package, so the study engine no longer brings storage or SQLite into its graph; the user-owned dirty `question_builder.rs` was not rewritten.
- `2026-07-28` - Phase 15 evidence-tool repair: the planned Wave 1 command exposed that `capture-native-fixtures.ps1` lacked its specified `-WriteEvidence` option. The verification-only option now writes promotion evidence tied to the accepted pre-promotion digest and current learning-ledger hash after the canonical fixture test passes; ordinary `-Verify` and acceptance capture remain fail-closed on source drift.

## Mobile Lessons Learned

- Seeing questions on Study is not enough; always verify the source is the real wordbook/plan, not a test pool or fixed sorted prefix.
- If a rebuilt package still shows the same first words, inspect active snapshots and restore paths before only changing question builder logic.
- Today progress and Study progress are two views of the same session state. Submit-final and explicit-complete must share completion semantics.
- Do not let Study repair Today by changing plan-visible targets. A generated session with fewer valid questions is a Study/session quality issue, not permission to rewrite Today/Plan totals.
- Once a valid session exists, use one session denominator for Study progress and completion summary; dedupe retry rows before counting completion.
- Runtime learning validation must be stricter than generic display validation. A token like `a.` has alphanumeric content but is not a valid study target.
- New question types are high-risk for old interactions: selection state, submit enablement, answer display, and choice uniqueness need regression coverage.
- Distractor randomness needs a real candidate pool and dedupe rules; small hard-coded-looking sets are visible to users quickly.
- Mobile feedback must not assume backend `correctAnswer` is always populated.
- Skeleton answer semantics are missing letters, not full words.
- On Windows PowerShell, writing non-ASCII literals can corrupt files; prefer UTF-8-safe tooling or ASCII Unicode escapes for source edits.
- Persist enough source payload in resumable sessions to rebuild the same valid question plan; persisting rendered questions alone makes engine upgrades and corruption recovery fragile.
- Treat duplicate mobile submissions as retries of the same question, not as an automatic out-of-order failure.
- For lazy generation, the user-visible invariant is resume stability for answered feed and current question. Optimizing future-question generation must not trade away that resume invariant.
- Keep duplicate-submit protection and next-question navigation separate. Preventing double-submit stalls must not produce automatic next-page motion or button-only next-question flow.
- PageView `onPageChanged` is not proof of user intent when the item list is rebuilt or windowed. Use scroll-start drag evidence before treating a pending page as a deliberate next-question swipe.
- Study hydrate must not pass exactly N raw candidate IDs into app-core when app-core may reject invalid payloads. Overfetch/filter/truncate before session start so Today/Plan totals remain stable without rewriting the user's targets.
- When pending-next feed items are appended after submit, keep the viewport anchored on the submitted question; otherwise a stable numeric page index can visually become the next question after the answered-history window shifts.

## Desktop Follow-Up Notes

- Desktop should start from the corrected shared session contract, not from the older mobile bugs.
- Reuse final-submit completion, active-snapshot semantics, input feedback fallback, and skeleton missing-letter behavior exactly.
- Desktop can expose `answeredQuestions` as a persistent side history for review and correction.
- Desktop must reuse snapshot schema/version checks, idempotent submission, and paged answer-history semantics rather than implementing a separate resume model.
- Keyboard-first defaults: number keys for choices, Enter submit, Esc exit, Tab movement.
- Do not copy the mobile AI hint confusion. Desktop can present hints/comments/dispute actions in a right-side tool area with explicit disabled states.

## Route Changes

- `2026-05-08`: Learning flow moved from card-style study to feed-style reviewable study.
- `2026-05`: Question types expanded from basic bilingual choices to examples, pure examples, inputs, and skeleton completion.
- `2026-05`: Skeleton completion moved from full-word answer to missing-letter answer.
- `2026-06`: Completion route expanded so submitting the final question completes and clears the active snapshot.
- `2026-07-15`: Resume routing now prefers a validated V2 source-payload snapshot and self-repairs invalid persisted state instead of restoring display placeholders or stale question plans.
- `2026-07-15`: Pending next-question navigation changed to user-controlled swipe only; auto page jumps are disabled while a pending next question is waiting.

## Known Pitfalls

- Do not let mock/test payloads or old active snapshots override the real wordbook.
- Do not treat abbreviation-like headwords such as `a.` or `vs.` as normal study entries even if the seed book contains meanings for them.
- Do not leave active snapshots after completed sessions.
- Do not use feed item count as session total.
- Do not use `results.len()` as the completed-session total; duplicate submit/retry rows can outnumber the actual question plan.
- Do not rewrite Today/Plan task targets from active snapshots or completed sessions. Today target drift is more confusing than a generation shortage and can hide the real defect.
- Do not let incomplete choice questions reach Flutter as 3-option multiple choice. Repair or downgrade them before validation.
- Do not emit `/`, empty strings, or duplicate meanings as choices.
- Do not use broad accepted meanings as the only source for choice correctness.
- Do not reveal full words in skeleton heroes/prompts.
- Do not show clickable AI hints for words without available hints.
- Do not write Chinese source/docs through Windows default encoding.
- Do not trust a persisted session only because it deserializes; validate schema, engine version, study date, mode, source IDs, question weights, prompts, choices, and correct labels.
- Do not reject a network/UI retry after its question was already answered; return the existing result idempotently.
- Do not assume `study_facade` tests are parallel-safe while they mutate the same global active-session registry; serialize them or isolate registry state before treating the grouped suite as a stable gate.
- Do not implement lazy generation by only storing `current_index`; without either persisted rendered answered/current questions or deterministic per-index reconstruction, killed-process resume can change what the user already saw.
- Do not let delayed `jumpToPage` callbacks run while a pending next question exists; that can create visible auto-slide/pullback and progress-count mismatch.
- Do not activate pending next questions directly inside `PageView.onPageChanged`; list-window shifts around the 20 answered item boundary can make rebuilds look like page changes.
- Do not fix a Today/Study total mismatch by mutating Today or Plan counts. The generation layer should fill valid entries up to the target, and only then report a smaller session if the real valid pool is insufficient.
- Do not rely on app-core filtering alone for user-visible target counts; platform hydrate needs to skip invalid wordbook rows before constructing the start-session payload.

- Do not update the Phase 15 source lock by recapturing after Wave 0. Product-source drift must fail closed, and only evidence-gated promotion may advance the accepted digest; promotion is parity bookkeeping, not permission to copy stale desktop rules or correct product behavior inside the export phase.

- Native report fixtures must use the persisted history shape (`summary.totalQuestions`, `summary.correctCount`, `summary.totalTimeMs`) and serialized enum text. Flattened counters silently filter to zero and do not exercise report mode normalization.
- A baseline restart test must not supply a fresh entry set when it intends to prove snapshot restoration. Resume uses an empty request so the persisted question plan, answered history, current index, and total remain authoritative.
- Do not regenerate accepted fixtures merely because DTO ownership moves. Verify the existing outputs against the accepted digest, review every locked source diff, and promote only the source lock; model extraction is not permission to change NewWord, Review, or bridge JSON behavior.

## Verification

- Mobile:
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub` passed all 21 study display/submit/progress/navigation cases on 2026-07-15 after adding the pending-next no-auto-jump guard.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart test\wrong_word_graph_screen_test.dart --no-pub` passed all 23 tests on 2026-07-15, including 20 study display/submit/progress cases.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --plain-name "input feedback correct answer falls back"` was blocked by missing local Pub cache dependencies: `async`, `vector_math`, `shared_preferences`, `supabase_flutter`, and others.
  - `D:\flutter\flutter\bin\dart.bat format ...` was blocked by missing `flutter_lints-6.0.0/lib/flutter.yaml`.
  - Still required: real-device smoke for all modes, single-choice state, input feedback, skeleton missing letters, and Today progress after mode completion.
- Desktop:
  - Pending; desktop learning page is not implemented yet.
- Shared/domain:
  - No new verification was run for the 2026-07-15 lazy-generation contract clarification; this was a design constraint update only.
  - `cargo test -p word-app-core study_facade::tests::` ran 28 focused tests on 2026-07-15: 26 passed and 2 failed with `NoActiveSession` under parallel execution; each failed test passed when rerun alone, identifying shared global test-state interference.
  - `cargo test -p word-app-core study_facade::tests:: -- --test-threads=1` passed all 28 tests on 2026-07-15; serial execution is the current reliable suite gate.
  - `answered_feed_is_windowed_and_older_history_can_be_loaded` and `completed_new_word_does_not_leak_feed_or_progress_into_review` each passed individually on 2026-07-15.
  - `cargo fmt --check` passed on 2026-06-25.
  - `cargo test -p word-app-core final_submit_persists_completed_session_and_clears_resume_snapshot -- --nocapture` passed on 2026-06-25.
  - `cargo test -p word-platform-mobile today_active_targets_use_actual_snapshot_question_count -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-platform-mobile today_completed_targets_and_completions_dedupe_question_results -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-storage-core summary_uses_question_plan_total_and_dedupes_extra_results -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-app-core final_submit_persists_completed_session_and_clears_resume_snapshot -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-app-core start_session_filters_invalid_entry_payloads_before_generation -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-app-core start_session_filters_abbreviation_like_entry_payloads_before_generation -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-app-core study_question_validation_rejects_abbreviation_like_word -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-app-core start_session_downgrades_incomplete_choice_question_without_shrinking_session -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-platform-mobile today_active_session_completion_does_not_rewrite_plan_target -- --nocapture` passed on 2026-07-16.
  - `cargo test -p word-platform-mobile today_completed_completions_dedupe_without_rewriting_plan_target -- --nocapture` passed on 2026-07-16.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub` passed all 21 study display/submit/progress/navigation cases on 2026-07-16.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub` was blocked on 2026-07-17 because Flutter tried to download `win32_windows_x64.dll` from GitHub and timed out.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 22 study display/submit/progress/navigation cases on 2026-07-17, including the user-driven pending-scroll boundary test.
  - `cargo test -p word-platform-mobile hydrate_start_session_overfetches_invalid_study_payloads_to_match_target -- --nocapture` passed on 2026-07-17, proving Study hydrate fills a two-item Mixed Test target when one raw candidate is invalid.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 23 study display/submit/progress/navigation cases on 2026-07-17, including the submitted-current-question feed-anchor regression.
  - `cargo check -p word-platform-mobile` passed on 2026-07-17 with existing dead-code warnings in `bridge.rs`.
  - `git diff --check -- crates\platform-mobile\src\bridge.rs apps\flutter_mobile\lib\features\study_screen.dart apps\flutter_mobile\test\study_question_display_test.dart` passed on 2026-07-17, with only Git CRLF conversion warnings.
  - `cargo check -p word-platform-mobile` passed on 2026-07-16 with existing dead-code warnings in `bridge.rs` and seed maintenance helpers.
  - `cargo fmt` passed on 2026-07-16.
  - `git diff --check -- crates\platform-mobile\src\bridge.rs crates\storage-core\src\models\study_result.rs crates\study-core\src\session_summary.rs crates\app-core\src\facade\study_facade.rs apps\flutter_mobile\lib\features\study_screen.dart apps\flutter_mobile\test\study_question_display_test.dart docs\features\learning.md docs\features\_reconciliation.md` passed on 2026-07-16, with only Git CRLF conversion warnings.
  - `git diff --check -- apps/flutter_mobile/lib/features/study_screen.dart apps/flutter_mobile/test/study_question_display_test.dart crates/app-core/src/facade/study_facade.rs crates/app-core/tests/baseline_runner.rs` passed on 2026-06-25.
- Phase 15 Wave 0 source-lock contract self-test passed on 2026-07-28; Wave 0 capture and check accepted digest `9774425edd021402959f2e620cef4fc2334def7eae51adb703f13ea1df0c786e` against the current dirty authoritative mobile inputs and learning ledger.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/domain-export/capture-native-fixtures.ps1 -AcceptCurrentMobileTruth` passed on 2026-07-28 and explicitly captured then re-verified all seven canonical native fixtures.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/domain-export/capture-native-fixtures.ps1 -Verify` passed on 2026-07-28 without acceptance mode; the focused canonical fixture test passed 1/1.
- `node scripts/domain-export/compare-semantic-json.mjs --self-test` passed on 2026-07-28, proving object-key order is ignored while array order and scalar values remain significant.
- `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` passed 14/14 on 2026-07-28 after aligning stale baseline-only completion and resume setup with current facade behavior. The first full-suite run failed 5 tests on those stale assertions; no product implementation was changed.
  - `cargo test -p word-domain-models` passed 2/2 fixture-backed serde compatibility tests on 2026-07-28; `cargo tree -p word-domain-models` contained none of `rusqlite`, `reqwest`, `jni`, `objc`, or `tauri`; and `cargo check -p word-domain-models --target wasm32-unknown-unknown` passed after installing the missing Rust target.
  - `cargo check -p word-storage-core -p word-study-core -p word-app-core -p word-platform-mobile` passed on 2026-07-28 with four pre-existing platform-mobile dead-code warnings. `cargo tree -p word-study-core` resolved `word-domain-models` and contained neither `word-storage-core` nor `rusqlite`.
  - `cargo test -p word-study-core` passed 35/35 and `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` passed 14/14 on 2026-07-28 after canonical DTO extraction.
