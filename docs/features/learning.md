# Feature: Learning Flow

> Slug: `learning`
> Status: `mobile_in_progress`
> Updated: `2026-07-26`

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

## Unified PC Consumption Route

- `2026-07-28`: Word Net is the surviving PC repository and will deliver both browser PWA and Tauri desktop targets from one React UI codebase.
- The current Flutter-backed Rust learning behavior is authoritative. Word Net must consume a pinned, versioned WASM export rather than recreate study rules in TypeScript.
- Only serde-friendly domain models and deterministic rules belong in the WASM package. SQLite, paths, network access, app lifecycle, and desktop/mobile bridge ownership remain in platform adapters.
- Native and WASM canonical fixtures must cover NewWord type-major four-question rounds, Review single-question behavior, question-unit progress, stable accepted meanings, active-session resume, wrong words, and reports.
- The existing `word-desktop-tauri` implementation remains a donor of verified behavior, migration fixtures, and release evidence until Word Net passes equivalent Web and Tauri gates; it is not a third long-lived product implementation.

## Sync And Storage

- Local `study_sessions`, `study_results`, and active session snapshots drive Today progress.
- Active snapshots must only survive for unfinished sessions. Completed modes must write a completed row and delete `active_study_session_*`.
- Supabase/cloud sync for account, wrong words, accepted disputed meanings, hints, and study results can be eventually consistent, but local state should become correct immediately.
- Today progress should be derived from authoritative completed records plus active-session seeds, never from stale snapshots alone.
- Today plan targets are the authoritative task target shown on Today/Plan and must not be silently rewritten by an active or completed Study session. If generation can only produce fewer valid renderable questions than the plan target, the Study/session layer must repair, downgrade, or explicitly surface that shortage instead of mutating Today from `34` to `33`.
- Study session totals and completion summaries must use one session denominator. Reports must dedupe duplicate result rows and never summarize retry rows as states such as `34/33`.
- Wrong-word reinforcement draws from historical `study_results` and shares the wrong-word page scoring model. The score includes cumulative errors, error rate, wrong recency, total correct answers, and correct answers since the last wrong answer; recovered words must decay instead of occupying the front indefinitely.
- Study-side secondary actions such as accepting a disputed meaning must not turn recoverable action failures into full-screen Study errors. The current session snapshot and feed state should remain intact while the user gets a non-blocking failure message.

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
- `2026-07-18` - Diagnosis: The previous PageView correction still allowed a visible flash because the pending next question remained a real PageView item; when a window shift or controller pixel drift landed on that item, Flutter could render it for a frame before the correction jump pulled back.
- `2026-07-18` - Diagnosis: Wrong-word reinforcement repetition is explained by current source and ordering logic rather than snapshots alone. `crates/platform-mobile/src/bridge.rs` loads the wrong-word pool from historical incorrect/skipped `study_results`, `crates/app-core/src/services/wrong_words_service.rs` scores words as `errorCount * 2.5 + recency * 0.2` capped at 10, and Study generation consumes the ordered front slice without shuffling.
- `2026-07-18` - Modification points: `crates/app-core/src/services/wrong_words_service.rs` now computes a finer wrong-word priority score from error volume, error rate, wrong recency, total correct answers, and correct answers since the latest wrong answer. `crates/platform-mobile/src/bridge.rs` now supplies `correctCount`, `lastCorrectAt`, and `correctSinceLastWrong` when loading wrong-word entries, so the same decayed score drives both the wrong-word page and wrong-word reinforcement.
- `2026-07-18` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` no longer inserts the pending next question into `_feedItems` at all. The answered current card keeps the "swipe to continue" hint and advances through an intentional vertical-drag handler, so there is no next-question PageView item to flash to. `apps/flutter_mobile/test/study_question_display_test.dart` now verifies the drag threshold and that pending has no feed index.
- `2026-07-18` - Diagnosis: The same PageView pre-insertion problem affected the final question: adding the completion page immediately after final submit could move the viewport straight to the summary and leave no reviewable final answered card.
- `2026-07-18` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now keeps completion data hidden from `_feedItems` until the user intentionally drags on the final answered card. The completion page is inserted only after that gesture, so users can reach the summary deliberately and still pull back to the final answer page. `apps/flutter_mobile/test/study_question_display_test.dart` covers the completion-visible gate.
- `2026-07-19` - Diagnosis: The recurring "after 20 questions" Study auto-slide is tied to `ANSWERED_FEED_WINDOW = 20` in `crates/app-core/src/facade/study_facade.rs`, which returns only the most recent answered questions to the mobile feed and exposes older history through a separate API. The value appears to be a UI/payload tradeoff for a short reviewable feed, not a product rule. The side effect is that on the 21st answer the first feed item is dropped, so a rebuilt vertical `PageView` can keep the same numeric page/pixel while the item at that position has changed, causing visible snap, pullback, or progress drift if Flutter navigation is keyed by index instead of question identity and explicit user intent.
- `2026-07-19` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now decouples the 20-item answered history window from next-question navigation. Pending next questions remain absent from `_feedItems` after submit, become a temporary PageView item only after the answered-card swipe gesture, animate into view, and are promoted to `currentQuestion` without calling `jumpToPage`. Non-current pending pages cannot submit, reveal, select choices, or mark mastered during the transition. `apps/flutter_mobile/test/study_question_display_test.dart` covers the pending-visible gate and auto-jump guard.
- `2026-07-20` - Superseded approach: A short input-submit gesture cooldown and a 20-second submit timeout were tried for input auto-advance and long selected-state stalls, but both were removed because they added non-root control flow and could interrupt requests that would otherwise submit correctly.
- `2026-07-20` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now treats stale/out-of-range visible PageView indexes as a display-only glitch: the progress pill falls back to the current or last-submitted question's real `questionIndex`, preventing window-trim flashes such as `30/34 -> 20/34`. The pending-next advance gesture now requires both enough vertical velocity and enough accumulated drag distance in the same gesture, so keyboard/input/scroll settlement cannot trigger next-question navigation from a single drag-end velocity event. `apps/flutter_mobile/test/study_question_display_test.dart` covers stale-index progress fallback and the stricter advance gesture gate.
- `2026-07-26` - Diagnosis: Dispute acceptance failures were routed through `_error`, and the error page Retry called `_start`. That turned a recoverable action failure such as `Failed to accept disputed meaning: No active session` into a full Study restart, which could clear the local feed/snapshot view while Today still reflected persisted answered progress.
- `2026-07-26` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now keeps dispute failures inside the current Study flow with a snackbar instead of setting `_error`. A successful dispute accept updates `_session` and last-submitted state from the native response and shows that the dispute joined the sync queue. Flutter no longer writes `word_disputed_meanings` directly. `crates/platform-mobile/src/bridge.rs` keeps the lightweight `word_disputed_meaning` outbox event but no longer rebuilds rolling study-point or full wrong-word sync snapshots during dispute acceptance. `apps/flutter_mobile/lib/sdk/sync_client.dart` now uploads `word_disputed_meaning` outbox rows to Supabase, and `apps/flutter_mobile/test/sync_client_test.dart` covers the payload-to-row mapping.
- `2026-07-26` - Diagnosis: Input-question submit still felt blocked during keyboard dismissal because `_submit` waited for the native bridge response before unfocusing the answer field, then cleared/rebuilt the feed while Android was also closing the keyboard. The input field also toggled `readOnly` during submit, which can cause Android text-input connection churn.
- `2026-07-26` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now marks the active question and unfocuses input questions before calling `submitAnswer`, so keyboard dismissal starts while the current card is still stable. The input `TextField` keeps a stable `readOnly: false` configuration and uses `IgnorePointer` during submit to block extra touches without reconnecting the keyboard. `apps/flutter_mobile/test/study_question_display_test.dart` covers the pre-bridge settle rule and stable input configuration.
- `2026-07-27` - Exam-to-Study priority contract: AI section-summary causal candidates are persisted on their matching exam occurrences as `user_mark='wrong'`. `crates/platform-mobile/src/bridge.rs` now sorts unlearned NewWord candidates that have this persisted exercise evidence ahead of the ordinary daily-stable wordbook rank or global frequency fallback. WrongWordReinforcement already consumes the same wrong evidence through the shared wrong-word priority path.
- `2026-07-27` - Keyboard latency diagnosis: input-question jank was traced to keyboard animation still affecting the Study feed. `_KeyboardInsetSpacer` previously used `MediaQuery.viewInsets.bottom`, causing the vertical PageView page content to relayout on every keyboard frame, and `_focusAnswerInputForCurrentQuestion` automatically requested focus after input-question activation, allowing keyboard open/close animation to overlap with feed transitions and feedback rendering.
- `2026-07-27` - Modification points: `apps/flutter_mobile/lib/features/study_screen.dart` now keeps the keyboard spacer at its fixed base height and disables automatic keyboard opening for Study input questions; users tap the answer field to open the keyboard. `apps/flutter_mobile/test/study_question_display_test.dart` covers the fixed spacer, stable input configuration, and no-auto-keyboard policy.
- `2026-07-27` - Submit-path performance contract: `crates/platform-mobile/src/bridge.rs` routes Study session actions through `open_study_session_action_database`, which opens the already-initialized database instead of applying schema and seed-meaning repair on every submit/mastered/dispute/complete/cancel action.
- `2026-07-27` - Choice-position diagnosis: `crates/study-core/src/question_builder.rs` placed correct answers with `question_index % choice_count` for Chinese choices and `(question_index + 1) % choice_count` for English choices. This made correct labels deterministic by ordinal position and could make adjacent visible choice questions reuse the same correct option in some mode/type sequences.
- `2026-07-27` - Modification points: `QuestionBuilder` now chooses the correct option position from a stable session/word/type/index/correct-text seed and, when possible, deterministically rotates away from the previous choice question's correct label. The previous label is tracked across interleaved input questions so the next visible choice question also avoids repeating the same correct option. `word-study-core` tests cover NewWord choice label ownership and MixedTest choice-label non-repetition across interleaved inputs.
- `2026-07-28` - Phase 15 domain-export modification points: `scripts/domain-export/check-source-lock.ps1` establishes a hash gate over the current Flutter-backed Rust question/session/report/wrong-word behavior, the mobile bridge, the focused Flutter display contract, and this ledger. Wave 0 capture is the initial mobile truth; later accepted digests require a reviewed machine-readable diff, successful parity evidence tied to the old digest, and a changed learning-ledger digest. Generated Phase 15 fixtures and scripts are deliberately excluded from the authoritative product-source set.
- `2026-07-28` - Phase 15 canonical-fixture modification points: `crates/app-core/tests/baseline_runner.rs` now derives seven deterministic native outputs from explicit `nowUtc`, `localDay`, `sessionId`, and ordering-seed requests under `fixtures/domain/v1/`. The corpus locks NewWord's four type-major rounds, every non-NewWord mode's one-question-per-entry rule, pre-submit translation hiding and post-submit feedback, progress/summary carry-over, resume history, real wrong-word identity, and report local-day/mode filtering. `scripts/domain-export/capture-native-fixtures.ps1` requires `-AcceptCurrentMobileTruth` before overwriting expected JSON; `scripts/domain-export/compare-semantic-json.mjs` ignores object-key order while preserving array order and values.
- `2026-07-28` - Phase 15 baseline repair: the serial app-core suite exposed older runner assertions that explicitly completed sessions after final submit and resumed with a newly supplied source set. The owned baseline runner now asserts final-submit completion/persistence and resumes the saved deterministic input-question plan with an empty request; product implementation code was not changed.
- `2026-07-28` - Phase 15 model-ownership modification points: `crates/domain-models` now owns the serde-only study, plan/progress, resume, wrong-word, and Today projection DTOs. `crates/storage-core/src/models/mod.rs` and the former storage model modules re-export those canonical types so app-core, platform-mobile, and Flutter-facing JSON paths retain their existing contracts. `crates/study-core/Cargo.toml` resolves its preserved source-level compatibility import to the pure `word-domain-models` package, so the study engine no longer brings storage or SQLite into its graph; the user-owned dirty `question_builder.rs` was not rewritten.
- `2026-07-28` - Phase 15 evidence-tool repair: the planned Wave 1 command exposed that `capture-native-fixtures.ps1` lacked its specified `-WriteEvidence` option. The verification-only option now writes promotion evidence tied to the accepted pre-promotion digest and current learning-ledger hash after the canonical fixture test passes; ordinary `-Verify` and acceptance capture remain fail-closed on source drift.

- `2026-07-28` - Phase 15 shared-engine modification points: `crates/domain-core` now owns typed deterministic question generation, answer evaluation, question-unit progress and summaries, resume transitions, wrong-word scoring, and report projections. `crates/study-core` is compatibility glue over `word-domain-core`; app-core keeps JSON, SQLite, snapshots, and local-clock acquisition in native adapters. NewWord remains four type-major questions per word, while Review selects one question per entry from the full four-type pool.
- `2026-07-28` - Phase 15 compatibility repair: V2 resume reconstruction now applies the same incomplete-choice repair as fresh session start before validation. The canonical post-submit fixture adapter reads translation feedback from the retained source entry after evaluation, while pre-submit `StudyQuestion.exampleTranslation` stays null.
- `2026-07-28` - Baseline reconciliation provenance: the Phase 15 source lock froze pre-existing worktree changes in `crates/study-core/src/question_builder.rs` and `crates/app-core/src/services/wrong_words_service.rs`. This extraction preserved their current behavior as authoritative input, but the dirty tree alone does not establish authorship, exact timing, intent, or prior verification; only the checks recorded below were run in this execution.

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
- The no-flash mobile route is stronger than page anchoring: do not put a pending next question into PageView until it becomes the current question. Trigger advancement from the answered card gesture instead.
- If a visible transition to the next question is needed, insert pending as a gesture-scoped temporary page and promote it after the animation; do not let answered-history window trimming or numeric PageView indexes trigger that transition.
- Input-question focus, keyboard dismissal, and submit settlement can produce different touch timing from choice questions. Do not accept pending-next gestures immediately after an input submit, and do not let stale PageView indexes drive progress while the feed window is being rebuilt.
- Apply the same rule to summaries: final completion data can exist in state, but the summary PageView item should not exist until the user asks to view it.
- Hint action availability is saved-hint availability, not AI suggestion availability. A question with only generated suggestions must keep the right-side hint button disabled/default-colored until the user actually saves a hint.
- Study answer submission is on the critical touch path. It may persist local answer progress and the active session snapshot, but heavy sync snapshots must not be rebuilt after every non-final answer.
- Do not use the full-screen Study error Retry for recoverable per-question actions such as dispute acceptance. Retry runs the Study start path and can desynchronize local feed state from already-persisted progress.
- Do not make cloud dispute recording part of the local accept transaction. Study should call the native accept API once, update from that response, and let the sync outbox own cloud delivery.
- For input questions, close the keyboard before bridge submit work and keep the text-input configuration stable while submitting. Use parent-level touch blocking rather than toggling `TextField.readOnly` during the critical path.
- For Study input questions, keyboard animation must not drive feed layout or page activation. Keep bottom spacing independent of `MediaQuery.viewInsets`, and do not auto-open the keyboard when a question becomes current; let the user tap the input.
- Do not run schema application or seed-repair maintenance from per-answer Study bridge calls. Startup/session-start may initialize; submit/mastered/dispute/complete/cancel must stay on the lightweight session-action path.
- Correct-choice position must not be derived directly from `question_index` modulo option count. Use a stable per-question seed and avoid repeating the previous visible choice label when there is more than one possible position.

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

- Do not use transient report highlighting as a Study-priority signal. Exam causal words must resolve to persisted occurrence `entry_id` evidence; otherwise a red word can look prioritized in the report but remain unavailable to the study selector.
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
- Do not keep pending next questions as hidden/preloaded PageView pages; even if guarded, a real page can flash during viewport correction.
- Do not append completion summaries as real PageView pages immediately after the last answer; that can skip the final answered card and prevent users from reviewing it.
- Do not let cumulative historical wrong counts permanently dominate wrong-word reinforcement. Correct answers after the latest wrong answer must lower priority, and any future rotation/cooldown should build on the shared wrong-word scoring service instead of adding a separate mode-only sort.
- Do not treat the 20-item answered-feed window as a business invariant. It is only a transport/UI history window and it must not drive progress, session totals, swipe behavior, or current-question advancement.
- Do not let right-side action buttons paint active backgrounds when their callbacks are disabled; visual availability must match tap availability.
- Do not enqueue rolling study-point or full wrong-word sync snapshots from every partial answer. These scans can block the first daily new-word submit flow; defer them until session completion or an explicit non-interactive sync boundary.
- Do not use the full-screen Study error Retry for recoverable per-question actions such as dispute acceptance; Retry re-enters the Study start path and can make local feed state look reset even when persisted Today progress is correct.
- Do not enqueue rolling study-point or full wrong-word sync snapshots from dispute acceptance. The lightweight disputed-meaning outbox event is enough for the action path; heavy snapshots belong at completion or an explicit sync boundary.
- Do not leave a new native outbox domain without a Flutter `SyncClient` uploader in the non-WordAdmin path. Otherwise the queue item will be marked `unsupported_domain` and the UI will misleadingly claim it is queued.
- Do not put `MediaQuery.viewInsets` into the Study answer feed content. Android keyboard animation can otherwise resize/rebuild the feed during submit and reintroduce visible blank-background flashes or accidental vertical movement.
- Do not auto-request focus for the next input question during PageView/feed transitions. Keyboard opening must be a user action, not a side effect of current-question activation.
- Do not treat "stable" as "ordinal." Stable choice generation still needs enough entropy from session, word, question type, index, and correct text; otherwise users can learn the answer slot pattern instead of the vocabulary.
- Do not update the Phase 15 source lock by recapturing after Wave 0. Product-source drift must fail closed, and only evidence-gated promotion may advance the accepted digest; promotion is parity bookkeeping, not permission to copy stale desktop rules or correct product behavior inside the export phase.
- Native report fixtures must use the persisted history shape (`summary.totalQuestions`, `summary.correctCount`, `summary.totalTimeMs`) and serialized enum text. Flattened counters silently filter to zero and do not exercise report mode normalization.
- A baseline restart test must not supply a fresh entry set when it intends to prove snapshot restoration. Resume uses an empty request so the persisted question plan, answered history, current index, and total remain authoritative.
- Do not regenerate accepted fixtures merely because DTO ownership moves. Verify the existing outputs against the accepted digest, review every locked source diff, and promote only the source lock; model extraction is not permission to change NewWord, Review, or bridge JSON behavior.

- Do not regenerate V2 resume questions and validate them through a different path from fresh-start questions. Apply the same incomplete-choice repair first, or valid input fallbacks can disappear and make an otherwise resumable session look empty.
- A feedback-only field cannot be recovered from a deliberately redacted pre-submit question. Retain the typed source entry through submission and construct feedback from that source after evaluation.
- Do not fabricate distractors to satisfy a choice shape. When a complete four-option pool is unavailable, preserve the existing deterministic downgrade to input; tests that need choice semantics must supply a complete explicit distractor pool.

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
  - `cargo test -p word-domain-core study_fixture -- --nocapture` passed 2/2 canonical study/progress fixtures on 2026-07-28.
  - `cargo test -p word-domain-core projection_fixture -- --nocapture` passed 2/2 typed wrong-word/report projection fixtures on 2026-07-28.
  - `cargo test -p word-domain-core` passed 26/26 unit and fixture tests on 2026-07-28.
  - `cargo check -p word-domain-core --target wasm32-unknown-unknown` passed on 2026-07-28 with fixed-width deterministic hashing and no platform/persistence dependency.
  - `cargo test -p word-study-core` passed 3/3 direct-domain compatibility assertions on 2026-07-28.
  - `cargo test -p word-app-core study_facade::tests:: -- --test-threads=1` passed 31/31 serial facade tests on 2026-07-28.
  - `cargo test -p word-app-core --test baseline_runner -- --test-threads=1` passed 14/14 baseline and canonical corpus tests on 2026-07-28.
  - `cargo test -p word-app-core reports_service -- --test-threads=1` passed 2/2 native/direct report adapter tests on 2026-07-28.
  - `cargo test -p word-app-core wrong_words_service -- --test-threads=1` passed 8/8 native/direct wrong-word adapter tests on 2026-07-28.
  - `cargo test -p word-platform-mobile --lib` passed 48/48 on 2026-07-27, including `new_word_selection_prioritizes_ai_causal_vocabulary` and `causal_words_are_persisted_as_wrong_answer_priority_evidence`; `cargo check -p word-platform-mobile` passed with four pre-existing dead-code warnings.
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
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 23 study display/submit/progress/navigation cases on 2026-07-18, including the pending-not-in-PageView and vertical-drag threshold regressions.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 24 study display/submit/progress/navigation cases on 2026-07-18, including the completion-visible PageView gate.
  - No tests were run for the 2026-07-18 wrong-word reinforcement repetition diagnosis; it was based on code inspection of selection/scoring/generation paths only.
  - `cargo test -p word-app-core wrong_words_service -- --nocapture` passed 7 wrong-word scoring/service tests on 2026-07-18, including consecutive-correct priority decay.
  - `cargo test -p word-platform-mobile wrong_word_reinforcement_demotes_words_recovered_by_recent_correct_answers -- --nocapture` passed on 2026-07-18, proving the platform reinforcement entry list demotes a recovered high-history wrong word behind an unrecovered wrong word.
  - `cargo check -p word-platform-mobile` passed on 2026-07-18 with existing dead-code warnings in `bridge.rs`.
  - `git diff --check -- crates\app-core\src\services\wrong_words_service.rs crates\platform-mobile\src\bridge.rs docs\features\learning.md` passed on 2026-07-18, with only Git CRLF conversion warnings.
  - No automated tests were run for the 2026-07-19 answered-feed-window explanation; it was a code/documentation diagnosis only.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 24 study display/submit/progress/navigation cases on 2026-07-19, including the pending-visible transition gate.
  - `D:\flutter\flutter\bin\flutter.bat analyze lib\features\study_screen.dart test\study_question_display_test.dart` passed with no issues on 2026-07-19 after resolving local Flutter dependencies.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 25 study display/submit/progress/navigation cases on 2026-07-20, including stale-window-index progress fallback and the stricter pending-next drag-distance gate.
  - `D:\flutter\flutter\bin\flutter.bat analyze lib\features\study_screen.dart test\study_question_display_test.dart` passed with no issues on 2026-07-20.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 26 study display/submit/progress/navigation cases on 2026-07-20, including saved-hint-only hint action availability and disabled active-surface rendering.
  - `D:\flutter\flutter\bin\flutter.bat analyze lib\features\study_screen.dart test\study_question_display_test.dart` passed with no issues on 2026-07-20 after the hint-action fix.
  - `cargo test -p word-platform-mobile study_submit_sync_snapshots_are_deferred_until_session_complete -- --nocapture` passed on 2026-07-23, guarding that partial submit does not trigger heavy sync snapshots.
  - `cargo check -p word-platform-mobile` passed on 2026-07-23 with existing dead-code warnings in `bridge.rs`.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 27 study display/submit/progress/navigation/action cases on 2026-07-26, including the dispute-failure current-flow guard.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart test\sync_client_test.dart --no-pub --no-test-assets` passed all 30 focused Study/Sync cases on 2026-07-26, including native-queued dispute success text and `word_disputed_meaning` payload mapping.
  - `cargo test -p word-platform-mobile study_dispute_accept_does_not_enqueue_heavy_sync_snapshots -- --nocapture` passed on 2026-07-26.
  - `cargo test -p word-platform-mobile study_submit_sync_snapshots_are_deferred_until_session_complete -- --nocapture` passed on 2026-07-26.
  - `cargo test -p word-platform-mobile study_session_actions_skip_schema_application_on_the_submit_path -- --nocapture` passed on 2026-07-27.
  - `cargo test -p word-platform-mobile study_submit_sync_snapshots_are_deferred_until_session_complete -- --nocapture` passed on 2026-07-27.
  - `cargo check -p word-platform-mobile` passed on 2026-07-27 with existing dead-code warnings in `bridge.rs`.
  - `git diff --check -- apps\flutter_mobile\lib\features\study_screen.dart apps\flutter_mobile\test\study_question_display_test.dart crates\platform-mobile\src\bridge.rs docs\features\learning.md` passed on 2026-07-27, with only Git CRLF conversion warnings.
  - `cargo test -p word-study-core choice_labels -- --nocapture` passed on 2026-07-27, covering NewWord generated correct-label ownership and MixedTest non-repetition.
  - `cargo test -p word-study-core mixed_test_choice_labels_do_not_repeat_across_interleaved_inputs -- --nocapture` passed on 2026-07-27.
  - `cargo test -p word-study-core -- --nocapture` passed 35/35 tests on 2026-07-27.
  - `cargo test -p word-app-core start_session_downgrades_incomplete_choice_question_without_shrinking_session -- --nocapture` passed on 2026-07-27.
  - `cargo check -p word-study-core` passed on 2026-07-27.
  - `D:\flutter\flutter\bin\flutter.bat analyze lib\features\study_screen.dart lib\sdk\sync_client.dart test\study_question_display_test.dart test\sync_client_test.dart` passed with no issues on 2026-07-26.
  - `cargo check -p word-platform-mobile` passed on 2026-07-26 with existing dead-code warnings in `bridge.rs`.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 29 focused Study display/submit/progress/navigation/action cases on 2026-07-26, including the input pre-bridge keyboard-settle rule.
  - `D:\flutter\flutter\bin\flutter.bat analyze lib\features\study_screen.dart test\study_question_display_test.dart` passed with no issues on 2026-07-26 after rerunning with a longer timeout; the first analyze attempt timed out before reporting diagnostics.
  - `D:\flutter\flutter\bin\flutter.bat test test\study_question_display_test.dart --no-pub --no-test-assets` passed all 29 focused Study display/submit/progress/navigation/action cases on 2026-07-27, including the fixed keyboard spacer and no-auto-keyboard policy.
  - `D:\flutter\flutter\bin\flutter.bat analyze lib\features\study_screen.dart test\study_question_display_test.dart` passed with no issues on 2026-07-27.
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
