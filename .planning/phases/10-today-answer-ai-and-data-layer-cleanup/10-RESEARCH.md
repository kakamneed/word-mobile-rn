# Phase 10 Research

## Scope confirmation

Phase 10 should clean only the Flutter implementation and the Rust/data layers actively used by Flutter. React Native under `apps/mobile` is legacy context only and should not receive active implementation cleanup unless stale references are misleading Flutter work.

The cleanup must preserve current user-facing behavior and effects while removing obsolete remnants, duplicate truth, empty adapters, demo/mock production paths, stale docs/tests, and misleading fallback logic. The added scope is real and must be planned: sidebar entries, leaderboard, image voting leaderboard mode, and image extraction/upload paths are active Flutter surfaces.

No `CLAUDE.md` was found in the repo root. Relevant repo-local skills read: `skill/word-mobile-study-flow-map/SKILL.md`, `skill/flutter-today-study-target-consistency/SKILL.md`, and `skill/flutter-android-release-wireless-deploy/SKILL.md`.

## Active layer map: Flutter screens/routes, Flutter SDK/bridge, native adapters, Rust platform-mobile/app-core/study-core/storage-core, SQLite/cloud-adjacent ownership

Canonical layer order:

1. Flutter app lifecycle: `apps/flutter_mobile/lib/main.dart`, `apps/flutter_mobile/lib/state/app_state.dart`.
2. Flutter navigation/screens: `mobile_root_shell.dart`, `today_shell_screen.dart`, `study_screen.dart`, `ai_screen.dart`, `wrong_words_screen.dart`, `reports_screen.dart`, `plan_screen.dart`, `leaderboard_screen.dart`, `account_drawer.dart`, profile/settings/onboarding/Croc BTI adjacent entries.
3. Flutter typed SDK: `sdk.dart`, `today_client.dart`, `plan_client.dart`, `study_client.dart`, `ai_client.dart`, `wrong_words_client.dart`, `reports_client.dart`, `sync_client.dart`, `local_data_owner_client.dart`, `reward_client.dart`, `reward_image_client.dart`.
4. Flutter bridge boundary: `bridge/rust_bridge.dart`, `bridge/bridge_codec.dart`, `bridge/bridge_error.dart`. Screens should not call native primitives directly.
5. Native adapters: `apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java`, `android/.../MainActivity.kt`, `ios/Runner/AppDelegate.swift`.
6. Rust mobile hub: `crates/platform-mobile/src/bridge.rs`, with platform exports in `android.rs`, `ios.rs`, path ownership in `paths.rs`, runtime setup in `runtime.rs`.
7. Rust domain/facade: `crates/app-core/src/facade/study_facade.rs`, `today_facade.rs`, `services/reports_service.rs`, `services/wrong_words_service.rs`; `crates/study-core/src/question_builder.rs`, `answer_evaluator.rs`, `session_definition.rs`.
8. Storage truth: `crates/storage-core/src/models/*`, `persistence/schema.rs`, `study_repo.rs`, `mastered_entry_repo.rs`, `word_hint_repo.rs`, `sync_repo.rs`.
9. Cloud-adjacent side effects: `apps/flutter_mobile/lib/supabase/*`, especially `auth_session_manager.dart`, `leaderboard_service.dart`, `supabase_auth_service.dart`, plus `SyncClient`. These must not become the local study correctness source.

SQLite ownership found in `schema.rs`: `app_settings`, `study_results`, `imported_wrong_words`, `mastered_entries`, `reward_image_upload_entitlements`, `reward_images`, `leaderboard_image_tags`, `reward_image_votes`, `reward_image_weekly_winners`, `reward_draw_pool_images`, and `local_leaderboard_summaries`.

## Page-specific cleanup risks: Today, Study, AI, WrongWords, Reports, Sidebar, Leaderboard, Image voting/image upload

**Today:** `TodayShellScreen` loads Today, active plan, AI context/history, reward state, announcements, and sync status in one bundle. Cleanup should split core study truth from adjacent cards without visual redesign. High-risk logic: `_snapshotOrPlanFallback`, `_loadHomeBundle`, automatic `applySavedPlanToToday`, `_openStudy`, `_generateAiPassage`, reward card, and sync diagnostic expansion. Rust also implicitly applies saved plan when Today snapshot is missing in `build_authoritative_today_home_state`.

**Study:** `StudyScreen` owns local UI state only: selected choice, text input, page/feed position, loading/submitting, last submitted display merge. Correctness, final progress, wrong-word side effects, reports, and persistence belong to Rust. High-risk functions: `_start`, `didUpdateWidget`, `_startWithBestAvailableSeed`, `_submit`, `_feedItems`, `_choiceState`, `_isCorrectChoice`, `_resolvedCorrectChoiceTextToken`, `_markCurrentEntryMastered`.

**AI:** `AiScreen` and Today AI shortcut both call `sdk.ai`. Cleanup should keep one typed AI client path and one shared generation/history/import semantic model. Today and AI page currently duplicate generation gating and sync flushing. AI image import enters through `AiClient.pickWrongWordImportSource`, `analyzeWrongWordImport`, `commitWrongWordImport`; Flutter should own selection/review only, not extraction truth or persistence truth.

**WrongWords:** `WrongWordsScreen` is first-class scope. It uses `WrongWordsClient` for list/detail/hint save, starts `wrongWordReinforcement`, and displays normal words plus root/affix entries. Cleanup risks: filters as display only vs persisted wrong-word truth, imported wrong words with negative/detail IDs, hint unlock/save, mastered/trash exclusion, AI suggestion source, and duplicate wrong-word notebooks.

**Reports:** `ReportsScreen` must stay backed by `ReportsClient.getReportsOverview`, not UI-local counters. Cleanup risks: session summary vs persisted reports, daily series vs mode breakdown, restored cloud report overview fallback in Rust, and leaderboard summary derivation from reports.

**Sidebar:** `MobileRootShell` and `AccountDrawer` expose leaderboard, settings, onboarding replay, Croc BTI, account/auth, and profile settings. Phase 10 verification must smoke every drawer entry for reachability, no stale RN/placeholder routes, no duplicate hidden routes, back navigation, cold-start reentry, empty/data/error states.

**Leaderboard:** `LeaderboardScreen` currently uses local SQLite leaderboard through `RewardImageClient.getLocalLeaderboard`, while `LeaderboardService` still contains Supabase RPC-based leaderboard code. This is a cleanup boundary: decide whether Supabase leaderboard service is live, stale, or future-only. Do not let remote leaderboard refresh become required for local Today/Study/Reports.

**Image voting/image upload:** Image vote mode in `LeaderboardScreen` lists `RewardImageClient.listImages(publicOnly: true)`, sorts by `voteCount`, and votes via `voteRewardImage`. Upload/entitlement APIs exist in `RewardImageClient` and Rust but need caller audit: `getUploadEntitlement`, `refreshUploadEntitlement`, `createUpload`, `moderateImage`, `selectLeaderboardTag`. Persistence is local SQLite, not page state.

## Historical pitfall analysis: selected wrong option red, correct option drift to A

Selected wrong option red: Flutter must match user response across label/value/text forms. `StudyScreen.choiceStateForTest` already supports response override and token matching; preserve and expand tests in `apps/flutter_mobile/test/study_question_display_test.dart`.

Correct option drift to A: stale `correctChoiceLabel = A` must not override authoritative question content when actual choice text or correct answer is known. The first source to inspect is Rust `study-core/question_builder.rs` and `answer_evaluator.rs`, then `storage-core/models/study_question.rs`, `study_client.dart` decoding, and finally Flutter display helpers.

Acceptance gate: a wrong selected option renders red/X while correct option remains green/check; non-A correct options stay non-A through Rust builder/evaluator, bridge DTO, Dart decode, and Flutter feedback rendering.

## Cold-start study bounce-back hypotheses and files to inspect during execution

Do not fix with delays or "ignore first return" UI hacks. Likely root causes to inspect:

1. `AppState.initialize()` calls `refreshAuthState()` after app ready. `refreshAuthState()` notifies once as `checking`, then again with resolved state.
2. `MobileRootShell._handleAppStateChanged()` compares `authState.userId` to `_loadedProfileUserId` and forcibly sets `_showingStudy = false`, clears `_studyResumeHint`, and increments reload seeds. On cold start, `_loadedProfileUserId` is initially null, so auth resolution/local owner restore can close an already opened study screen.
3. `AuthSessionManager.resolveStartupState()` may reconcile local owner and restore/backfill cloud data, causing a second state refresh while Study is starting.
4. `TodayShellScreen._loadHomeBundle()` may auto-apply saved plan when Today snapshot is missing, then reload Today state and possibly invalidate active sessions.
5. `StudyScreen` key includes reload seed, mode, resume hint current, and word. Any root-shell seed/auth change can rebuild Study; `StudyScreen.didUpdateWidget()` restarts if mode/resume signature changes.
6. Rust side: inspect `getResumeSessionHint`, `startStudySession`, `try_resume_empty_start_request`, `apply_active_session_restore_shape`, `active_study_session_%` settings, `apply_saved_plan_to_today`, and cloud restore/backfill code in `platform-mobile/src/bridge.rs`.

Execution should instrument earliest source: auth/user owner transition, route state transition, study start error, Today refresh side effect, or Rust session invalidation.

## Recommended plan decomposition, preferably 5 plan files matching ROADMAP while including the added sidebar/leaderboard/image scope

1. `10-01 - Clean Today/root navigation/cold-start handoff`
   Cover `AppState`, `MobileRootShell`, `TodayShellScreen`, plan apply-to-today, resume hint, startup/auth refresh, and the cold-start bounce. Include sidebar entry inventory but defer page-specific cleanup to later plans.

2. `10-02 - Clean Study answering and historical answer regressions`
   Cover `StudyScreen`, `StudyClient`, bridge DTOs, `platform-mobile` start/submit/resume, `study_facade`, `question_builder`, `answer_evaluator`, and tests for selected wrong red plus non-A correct identity.

3. `10-03 - Clean AI, wrong-word import, WrongWords, and Reports surfaces`
   Cover `AiScreen`, Today AI shortcut, `AiClient`, `WrongWordsScreen`, `WrongWordsClient`, `ReportsScreen`, `ReportsClient`, Rust wrong-word/report/AI history code, imported wrong words, hints, mastered/trash exclusion, and non-blocking AI failure behavior.

4. `10-04 - Clean bridge/data/persistence plus leaderboard/image-vote ownership`
   Cover `BridgeCodec/Error`, Android/iOS native adapters, `platform-mobile/src/bridge.rs` extraction boundaries, SQLite schema/repositories, sync/cloud-adjacent code, `LeaderboardScreen`, `RewardImageClient`, `LeaderboardService`, reward-image tables, image voting/upload entitlement APIs, and demo seed cleanup.

5. `10-05 - Regression validation and sidebar smoke closure`
   Run focused Flutter/Rust tests, analyze/build checks, release-device smoke, and manual sidebar route matrix. Include Today -> Study -> Submit -> Complete -> Today, AI failures, wrong words/reports refresh, leaderboard empty/data/vote states, image missing/upload failure cases, and cold-start immediate study entry.

## Verification strategy: Flutter tests, Rust tests, analyze/build/device smoke suggestions

Flutter focused tests:

- Existing: `apps/flutter_mobile/test/study_question_display_test.dart`, `today_task_breakdown_test.dart`, `study_client_test.dart`, `ai_passage_generation_test.dart`, `ai_wrong_word_import_test.dart`, `auth_session_manager_test.dart`.
- Add/expand: root-shell cold-start auth refresh does not close Study; Today auto-apply does not stale-start session; AI page and Today shortcut share generation/history behavior; WrongWords mastered/trash/hint behavior; Reports after completion; Leaderboard local summary/image vote behavior.

Rust focused tests:

- `cargo test -p word-study-core` for `question_builder` and `answer_evaluator`.
- `cargo test -p word-platform-mobile --lib` for Today targets, active session restore, stale snapshot invalidation, reports/wrong words/AI persistence, reward image entitlement, public image list, unique vote per week, local leaderboard demo.
- `cargo test -p word-app-core` and `cargo test -p word-storage-core` where facade/repository changes occur.

Static/build/device:

- `D:\flutter\flutter\bin\flutter.bat analyze --no-pub` from `apps/flutter_mobile`.
- Focused `flutter test --no-pub test\study_question_display_test.dart -r expanded`; if runner hangs, report unverified.
- `cargo fmt`, `cargo check -p word-platform-mobile`, `git diff --check`.
- Release-device smoke via `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1` when implementation changes affect Flutter UI, Rust bridge, Supabase/auth/cloud, or packaged assets.

Manual smoke must include kill process/cold start -> Today ready -> immediately enter Study -> no automatic return to Today -> submit one answer -> return Today and see refreshed progress.

## Concrete files likely to change vs files to avoid

Likely to change:

- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`
- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `apps/flutter_mobile/lib/features/study_screen.dart`
- `apps/flutter_mobile/lib/features/ai_screen.dart`
- `apps/flutter_mobile/lib/features/wrong_words_screen.dart`
- `apps/flutter_mobile/lib/features/reports_screen.dart`
- `apps/flutter_mobile/lib/features/leaderboard_screen.dart`
- `apps/flutter_mobile/lib/features/account_drawer.dart`
- `apps/flutter_mobile/lib/features/sample_study_payloads.dart`
- `apps/flutter_mobile/lib/state/app_state.dart`
- `apps/flutter_mobile/lib/sdk/*.dart`, especially `study_client.dart`, `ai_client.dart`, `wrong_words_client.dart`, `reports_client.dart`, `reward_image_client.dart`, `sync_client.dart`
- `apps/flutter_mobile/lib/bridge/*.dart`
- `apps/flutter_mobile/lib/supabase/auth_session_manager.dart`, `leaderboard_service.dart` if caller audit proves stale/live cleanup need
- `apps/flutter_mobile/android/app/src/main/java/com/wordmobile/RustBridge.java`
- `apps/flutter_mobile/ios/Runner/AppDelegate.swift`
- `crates/platform-mobile/src/bridge.rs`, possibly `android.rs`, `ios.rs`, `paths.rs`
- `crates/app-core/src/facade/study_facade.rs`, `today_facade.rs`, `services/reports_service.rs`, `services/wrong_words_service.rs`
- `crates/study-core/src/question_builder.rs`, `answer_evaluator.rs`, `session_definition.rs`
- `crates/storage-core/src/models/*`, `persistence/schema.rs`, `study_repo.rs`, `mastered_entry_repo.rs`, `word_hint_repo.rs`, `sync_repo.rs`
- Tests under `apps/flutter_mobile/test/` and Rust module tests in touched crates
- Flow docs/skills only after code cleanup changes canonical boundaries

Files/areas to avoid or treat as legacy:

- `apps/mobile/**` React Native code, except to label as legacy if referenced by Flutter docs/plans.
- Legacy RN skills such as `skill/legacy-react-native-android-release-wireless-deploy`.
- Broad visual redesign of Flutter screens.
- Destructive deletion of historical SQLite state such as `study_results`, active result history, wrong-word state, report history, AI passage history, reward image/vote rows.
- Full rewrite of `crates/platform-mobile/src/bridge.rs` unless a narrow extraction reduces real coupling and is covered by tests.

RESEARCH COMPLETE
