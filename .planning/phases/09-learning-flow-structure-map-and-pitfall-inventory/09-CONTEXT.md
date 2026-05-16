# Phase 09: Learning flow structure map and pitfall inventory - Context

**Gathered:** 2026-05-13
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 09 maps the Flutter mobile learning flow only. It should produce an evidence-backed structure map and pitfall inventory for the current Flutter implementation before Phase 10 performs cleanup.

The active scope includes `apps/flutter_mobile`, Flutter bridge/SDK clients, Android/iOS native bridge adapters needed by Flutter, Rust mobile bridge/core/storage layers, local SQLite persistence, Flutter-related Supabase/cloud boundaries, and existing Flutter project-local skills/docs.

React Native code under `apps/mobile` is not an implementation target for this phase. It may be mentioned only as legacy residue or decommission context when that helps Phase 10 avoid stale paths.
</domain>

<decisions>
## Implementation Decisions

### Flutter-Only Scope
- **D-01:** Phase 09 must ignore React Native as an active implementation path and focus on the Flutter app as the current product surface.
- **D-02:** `apps/mobile` references from older 07.x phases should be treated as historical context only. New canonical references must point to Flutter paths unless a Rust/shared layer is actively used by Flutter.
- **D-03:** Phase 09 should explicitly mark legacy RN docs/skills/code as out of active scope when they could confuse future cleanup work.

### Canonical Layer Hierarchy
- **D-04:** The canonical Flutter learning-flow hierarchy is: app lifecycle state -> Flutter feature screens -> typed Flutter SDK clients -> Flutter MethodChannel bridge -> native Android/iOS adapters -> Rust `platform-mobile` bridge -> Rust `app-core` facades -> `study-core`/domain services -> `storage-core` models and SQLite persistence -> optional cloud/Supabase sync and AI transport.
- **D-05:** Flutter feature screens must consume `WordSdk` clients and should not call native bridge APIs directly.
- **D-06:** The bridge codec/error layer is the only Flutter-side protocol boundary; pages should consume typed models and typed bridge errors instead of raw JSON/native strings.
- **D-07:** Rust and SQLite remain the source of learning truth. Flutter can own display state, selected option UI state, local feed paging, loading/error presentation, and navigation handoff, but not answer correctness or persisted progress truth.

### Structure Map Coverage
- **D-08:** The Phase 09 structure map must cover Today, Plan handoff, study answering, answer feedback, resume/progress, wrong words, reports, AI passage/history/import, local data owner/auth startup, sync/cloud side effects, and release verification entry points where they touch the learning loop.
- **D-09:** The map should identify the canonical file path for each active layer and list stale or duplicate paths separately with a keep/delete/replace recommendation.
- **D-10:** The map should distinguish core learning loop dependencies from adjacent product features such as rewards, leaderboard, comments, account drawer, and Croc BTI so Phase 10 can clean learning-flow code without accidentally deleting unrelated active features.

### Historical Pitfall Inventory
- **D-11:** The selected-wrong-option highlighting bug must be recorded as a cross-layer pitfall: backend result payloads can be stale or label/text shaped differently than Flutter expects, so Flutter feedback must match by label, value, and text while still respecting authoritative correct choice data.
- **D-12:** The "all correct answers point to A" bug must be recorded as an authority bug: stale or fallback `correctChoiceLabel` must never override actual question content when it is known to be stale, and regression tests must prove A/B/C/D positions are preserved.
- **D-13:** Phase 09 should include earlier pitfalls from 07.x: stub truth, hardcoded launch payloads, wrong mode resume, Today progress mismatch, root/affix answer leakage, mixed-test distractor direction, AI mock text/history, and stale active session snapshots.
- **D-14:** Each pitfall entry should name the earliest wrong source to inspect first: Flutter UI state, SDK DTO decode, bridge method payload, Rust bridge hydration, app-core session state, question builder/evaluator, or persistence.

### Cleanup Readiness For Phase 10
- **D-15:** Phase 09 must not perform destructive cleanup. Its output is the deletion/cleanup map and acceptance gate for Phase 10.
- **D-16:** Cleanup recommendations should be conservative: preserve user-facing behavior and visual effects unless the structure map proves the path is obsolete or harmful.
- **D-17:** Before Phase 10 deletes or rewires code, Phase 09 should require a focused test/checklist target for each high-risk layer, especially selected option state, correct answer index, Today progress, resume, AI non-blocking behavior, and SQLite persistence.

### Skillization Prep
- **D-18:** Phase 09 should identify which parts of the existing `skill/word-mobile-study-flow-map` and `skill/flutter-today-study-target-consistency` are still valid and which need replacement after Flutter-only cleanup.
- **D-19:** Phase 11 should split final skill docs by capability: Flutter Today/Plan handoff, Flutter study answering, AI passage/history/import, bridge/data persistence, Supabase/sync learning boundaries, and Flutter release validation.

### the agent's Discretion
- Exact document filenames produced by Phase 09, as long as they are canonical refs for Phase 10 and Phase 11.
- Exact format of stale-path classification, as long as each item has owner layer, risk, and keep/delete/replace recommendation.
- Exact test inventory format, as long as the two named hard bugs are covered explicitly.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 09 Scope And History
- `.planning/ROADMAP.md` - Phase 09-11 scope and success criteria.
- `.planning/STATE.md` - current project status and Phase 09 resume point.
- `.planning/phases/07.1-mobile-parity-and-study-correctness-fixes/07.1-CONTEXT.md` - historical correctness and parity issues.
- `.planning/phases/07.2-mobile-desktop-parity-alignment-and-session-state-fidelity/07.2-CONTEXT.md` - session state, Today truthfulness, and parity decisions.
- `.planning/phases/07.3-mobile-runtime-truth-alignment-ai-passage-and-data-continuity/07.3-CONTEXT.md` - stub truth, AI, and data continuity history.
- `.planning/phases/07.4-mobile-study-ux-polish-and-desktop-parity-closure/07.4-CONTEXT.md` - study UX, answer-choice fairness, root/affix, and AI semantic fixes.

### Flutter App Surfaces
- `apps/flutter_mobile/lib/main.dart` - Flutter app bootstrap entry.
- `apps/flutter_mobile/lib/state/app_state.dart` - app lifecycle, bootstrap, and auth state owner.
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart` - tab/root navigation and study handoff container.
- `apps/flutter_mobile/lib/features/today_shell_screen.dart` - Today home, progress, task breakdown, AI shortcut, reward and sync summaries.
- `apps/flutter_mobile/lib/features/plan_screen.dart` - active plan, wordbook, and apply-to-today surface.
- `apps/flutter_mobile/lib/features/study_screen.dart` - active study feed, answer selection, feedback rendering, resume, skip/show answer, completion, and mastered behavior.
- `apps/flutter_mobile/lib/features/ai_screen.dart` - AI passage/history and wrong-word import chat surface.
- `apps/flutter_mobile/lib/features/wrong_words_screen.dart` - wrong-word review surface.
- `apps/flutter_mobile/lib/features/reports_screen.dart` - persisted report surface.

### Flutter SDK And Bridge
- `apps/flutter_mobile/lib/sdk/sdk.dart` - central typed SDK composition.
- `apps/flutter_mobile/lib/sdk/today_client.dart` - Flutter Today contract client.
- `apps/flutter_mobile/lib/sdk/plan_client.dart` - Flutter plan/wordbook contract client.
- `apps/flutter_mobile/lib/sdk/study_client.dart` - Flutter study session DTOs and operations.
- `apps/flutter_mobile/lib/sdk/ai_client.dart` - Flutter AI passage/history/import contract client.
- `apps/flutter_mobile/lib/sdk/reports_client.dart` - Flutter reports client.
- `apps/flutter_mobile/lib/sdk/wrong_words_client.dart` - Flutter wrong-word client.
- `apps/flutter_mobile/lib/sdk/sync_client.dart` - Flutter sync/cloud restore client.
- `apps/flutter_mobile/lib/bridge/rust_bridge.dart` - only Flutter file that talks to native code.
- `apps/flutter_mobile/lib/bridge/bridge_codec.dart` - JSON serialization and protocol decoding boundary.
- `apps/flutter_mobile/lib/bridge/bridge_error.dart` - typed Flutter bridge error model.

### Native And Rust Learning Truth
- `apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java` - Android native adapter for Flutter MethodChannel to Rust.
- `apps/flutter_mobile/android/app/src/main/kotlin/com/wordmobile/flutter_mobile/MainActivity.kt` - Flutter Android entry and plugin wiring.
- `apps/flutter_mobile/ios/Runner/AppDelegate.swift` - iOS Flutter/native bridge wiring.
- `crates/platform-mobile/src/bridge.rs` - Rust mobile bridge methods, Today building, session hydration, AI/report/wrong-word/mobile persistence logic.
- `crates/platform-mobile/src/android.rs` - Android FFI/native export layer.
- `crates/platform-mobile/src/ios.rs` - iOS FFI/native export layer.
- `crates/platform-mobile/src/paths.rs` - mobile sandbox path contract.
- `crates/app-core/src/facade/study_facade.rs` - study session state, answer submission, active snapshot, and persistence coordination.
- `crates/app-core/src/facade/today_facade.rs` - Today facade boundary.
- `crates/study-core/src/question_builder.rs` - question generation, choices, and correct-option construction.
- `crates/study-core/src/answer_evaluator.rs` - answer correctness authority.
- `crates/study-core/src/session_definition.rs` - mode/question count definition.
- `crates/storage-core/src/models/study_question.rs` - persisted/bridged study question model.
- `crates/storage-core/src/models/study_requests.rs` - study request model.
- `crates/storage-core/src/models/today_home_state.rs` - Today state DTO.
- `crates/storage-core/src/persistence/schema.rs` - SQLite schema.
- `crates/storage-core/src/persistence/mod.rs` - database initialization and persistence modules.
- `crates/storage-core/src/persistence/mastered_entry_repo.rs` - mastered/trash exclusion persistence.

### Existing Flutter Docs And Skills
- `docs/flutter/flutter-current-behavior-review.md` - existing Flutter behavior map, but text appears encoding-corrupted and should be refreshed or replaced in Phase 09.
- `docs/flutter/flutter-sdk-surface.md` - Flutter SDK surface reference.
- `docs/flutter/bridge-architecture.md` - Flutter bridge architecture reference.
- `docs/flutter/bridge-error-model.md` - Flutter bridge error model reference.
- `docs/flows/today-plan-study-main-loop.md` - cross-flow Today/Plan/Study reference.
- `docs/flows/study-ui-vs-domain-boundary.md` - UI/domain responsibility boundary.
- `docs/flows/ai-non-blocking-behavior.md` - AI non-blocking behavior reference.
- `docs/flows/reports-ui-vs-aggregate-boundary.md` - report aggregate boundary.
- `skill/word-mobile-study-flow-map/SKILL.md` - existing study-flow debugging skill; keep as source material for Phase 11.
- `skill/flutter-today-study-target-consistency/SKILL.md` - existing Today/study target consistency skill; keep as source material for Phase 11.
- `skill/flutter-android-release-wireless-deploy/SKILL.md` - Flutter release-device validation workflow.

### Tests And Verification Surfaces
- `apps/flutter_mobile/test/study_question_display_test.dart` - existing Flutter tests for hero display, selected wrong option, correct choice state, and stale answered-list merging.
- `crates/platform-mobile/src/bridge.rs` tests - existing bridge-level tests for Today progress, hydration, active sessions, and AI/report/sync behavior.
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `WordSdk` already centralizes typed Flutter clients over the Rust bridge; Phase 10 should route through this rather than adding direct bridge calls from pages.
- `StudyScreen` exposes test helpers such as `choiceDisplayForTest`, `choiceStateForTest`, `studyHeroDisplayForTest`, `wordSkeletonDisplayForTest`, and `mergeLatestAnsweredQuestionForTest`.
- Existing Flutter tests already cover the two user-named hard bugs at the UI state level, but Phase 09 should still trace their backend and bridge causes.
- `RustBridge`, `BridgeCodec`, and `BridgeError` already form a clean protocol boundary that can be kept and documented.
- `platform-mobile/src/bridge.rs` contains the current Rust-side mobile coordination surface for Today, study, AI, reports, wrong words, sync, and persistence.

### Established Patterns
- Flutter screens call typed clients from `WordSdk`.
- Typed Flutter clients encode request maps, call a named bridge method, decode JSON, and return typed Dart DTOs.
- Rust `platform-mobile` hydrates Flutter requests into real local data and delegates study/session truth to `app-core` and `study-core`.
- SQLite is the local source of truth; Supabase/cloud work is adjacent sync/restore/remote feature work, not the source of local answer truth.
- AI is optional and non-blocking, but Flutter currently exposes AI from both Today shortcut and dedicated AI page.

### Integration Points
- Today -> Study: `TodayShellScreen._openStudy` and `StudyScreen._startWithBestAvailableSeed` call `sdk.study.startSession`.
- Study answer -> feedback: `StudyScreen._submit` calls `sdk.study.submitAnswer`, then merges the submitted question/result into local feed state for display.
- Correct/wrong option display: `StudyScreen` derives selected/correct state from `StudyQuestion`, `StudyResult`, and local response override.
- Today progress: `TodayShellScreen` reads `sdk.today.getTodayHomeState`, active plan, AI context/history, reward state, announcements, and sync status.
- AI: `TodayShellScreen` and `AiScreen` both call `sdk.ai` methods and may flush sync after generation.
- Sync/cloud: `AppState`, `AuthSessionManager`, `SyncClient`, and Supabase services participate in startup/restore/flush, but should not become required for local study correctness.

### Stale Or Risky Areas To Classify In Phase 09
- React Native paths and skills are now legacy for this task and should not be used as active canonical refs.
- `docs/flutter/flutter-current-behavior-review.md` appears mojibake-corrupted and should be refreshed before it becomes canonical.
- Flutter pages contain some mojibake user-facing strings; Phase 09 should classify whether these are display-copy cleanup, encoding repair, or unrelated to learning-flow structure.
- `sample_study_payloads.dart` and any fallback/demo payloads should be audited as possible stale truth.
- AI, rewards, leaderboard, comments, Croc BTI, and account features are adjacent to Today but should be separated from core learning cleanup unless they affect learning-flow state.
</code_context>

<specifics>
## Specific Ideas

- User explicitly corrected scope during discussion: "只关注flutter版".
- The two named historical hard bugs are selected wrong option not being marked red and all final correct answers pointing to A.
- Phase 09 should be a mapping and inventory phase, not a cleanup phase.
- Phase 10 should use Phase 09 outputs to remove obsolete Flutter-era residue thoroughly while preserving existing functions and effects.
- Phase 11 should produce skill-standard docs so future changes start from the cleaned Flutter structure.
</specifics>

<deferred>
## Deferred Ideas

- React Native active cleanup and decommission work belongs outside this Phase 09 Flutter-only context, except where legacy paths must be labeled stale to prevent confusion.
- New product features unrelated to Today, study, AI, reports, wrong words, persistence, or learning-flow verification are out of scope.
- Mandatory account-backed sync remains outside the local-first learning-flow cleanup unless a specific Flutter local/cloud boundary is blocking correctness.
</deferred>

---

*Phase: 09-learning-flow-structure-map-and-pitfall-inventory*
*Context gathered: 2026-05-13*
