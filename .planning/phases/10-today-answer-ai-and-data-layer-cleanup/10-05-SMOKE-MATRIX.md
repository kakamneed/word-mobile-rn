# Phase 10 Smoke Matrix

## Validation Results

| Area | Command Or Check | Result | Status | Notes |
|---|---|---|---|---|
| Phase plan completion | `node C:\Users\clf20\.codex\get-shit-done\bin\gsd-tools.cjs phase-plan-index 10` | All five plans have summaries; `incomplete` is empty. | PASS | Confirms 10-01 through 10-05 execution artifacts exist. |
| Flutter static analysis | `D:\flutter\flutter\bin\flutter.bat analyze --no-pub` from `apps/flutter_mobile` | Completed with four info-only existing findings. | PASS WITH KNOWN INFO | Existing info findings are `withOpacity` in `profile_settings_screen.dart` and null-aware element suggestions in `reward_image_client.dart`. |
| Rust study facade | `cargo test -p word-app-core study --lib` | Passed after stale persisted session fallback cleanup. | PASS | Covers answer label preservation and stale resume deletion. |
| Rust study core | `cargo test -p word-study-core --lib` | Passed. | PASS | Covers study question/evaluation core. |
| Rust reports | `cargo test -p word-app-core reports --lib` | Passed. | PASS | Confirms reports aggregate path remains valid. |
| Rust platform wrong words | `cargo test -p word-platform-mobile wrong --lib` | Passed. | PASS | Confirms wrong-word bridge path remains valid. |
| Rust platform reward images | `cargo test -p word-platform-mobile reward --lib` | Passed. | PASS | Confirms reward image bridge/client backing path. |
| Rust platform leaderboard | `cargo test -p word-platform-mobile leaderboard --lib` | Passed. | PASS | Confirms local leaderboard bridge path. |
| Rust platform image | `cargo test -p word-platform-mobile image --lib` | Passed. | PASS | Confirms image extraction/upload-adjacent bridge path. |
| Rust AI grouped filter | `cargo test -p word-platform-mobile ai --lib` | One grouped run hit mock backup-server reachability interference; isolated backup test passed. | PASS WITH NOTE | `wrong_word_image_analysis_uses_backup_after_primary_failure` passed when run alone with `--nocapture`. |
| Flutter focused tests | `flutter test test\auth_session_manager_test.dart`, `test\today_task_breakdown_test.dart`, `test\study_question_display_test.dart`, `test\study_client_test.dart`, `test\ai_passage_generation_test.dart`, `test\ai_wrong_word_import_test.dart`, `test\sidebar_smoke_test.dart`, `test\leaderboard_screen_test.dart` | Each focused run timed out at 120 seconds in this environment without a failure assertion result. | BLOCKED | Kept as explicit blocked checks rather than claiming pass. Static analysis and Rust gates still passed. |
| Release device deploy | `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1` | Not run in this execution turn. | DEFERRED | Requires connected Android release target and lifecycle observation. Device result: pending manual run. |

## Release Device Status

| Field | Value |
|---|---|
| Deploy command | `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1` |
| Execution status | Deferred |
| Reason | No connected release-device smoke run was performed during this phase execution. |
| Device result | Pending manual validation |
| Required cold-start check | Kill app, launch fresh, wait for Today, immediately enter Study, submit one answer, return Today. |
| Required sidebar/image check | Open account drawer, leaderboard, image vote mode, upload failure path, Wrong Words, Reports, AI, onboarding replay, Croc BTI, profile, and settings as applicable. |

## Cold Start And Study Handoff

| Case | Steps | Expected Result |
|---|---|---|
| Cold start immediate Study | Kill the app process, launch app, wait until Today content is visible, immediately tap the primary Study action. | Study screen stays open after auth/local-owner refresh. It must not bounce back to Today once. |
| Submit then return Today | From the cold-start Study screen, answer one question, then close/return to Today. | Today refreshes progress, Wrong Words/Reports/AI reload seeds are allowed to refresh, and no stale session is created. |
| Empty Today snapshot with active plan | Clear or simulate missing Today snapshot while an active saved plan exists, then open Today. | Today displays plan-derived fallback rows but does not auto-apply the plan or invalidate Study until the user explicitly syncs/applies. |

## Study Answer Regression

| Case | Steps | Expected Result |
|---|---|---|
| Selected wrong option | Start an A/B/C/D choice card, select an incorrect option, submit. | Selected wrong option is red/cancel; correct option is green/check. |
| Non-A correct option | Use a question where the correct label is B, C, or D, submit the correct option. | Result remains correct and the UI does not display A as the final answer. |
| Resume answered history | Answer a non-A choice, leave Study, resume the session. | Answered history preserves the real correct label and submitted response. |

## AI, Wrong Words, Reports

| Case | Steps | Expected Result |
|---|---|---|
| Today AI shortcut failure | Disconnect AI provider or force provider error, tap Today AI generation. | Error is visible; Today and Study remain usable. |
| AI page history | Generate or restore a passage from AI page. | History reloads through `AiClient`; no page-local mock history appears. |
| Wrong-word image/source import | Start AI wrong-word import from image/source input. | Flutter handles selection/review; extraction and commit go through `AiClient`; committed words appear in Wrong Words. |
| Wrong-word hint/mastered/trash | Open Wrong Words, save a hint, mark mastered/trash if available. | Changes persist after page reload and are reflected in AI wrong-word context. |
| Reports after completion | Complete or partially complete a Study session, open Reports. | Reports display persisted aggregate values from `ReportsClient`, not temporary UI counters. |

## Sidebar And Root Entries

| Entry | Signed Out | Signed In | Expected Result |
|---|---|---|---|
| Today tab | Yes | Yes | Opens Today. |
| Plan tab | Yes | Yes | Opens Plan without losing unsaved-change guard. |
| Wrong Words tab | Yes | Yes | Opens Wrong Words empty/data states. |
| Reports tab | Yes | Yes | Opens persisted Reports empty/data states. |
| AI tab | Yes | Yes | Opens AI page without blocking Study/Today. |
| Account drawer | Yes | Yes | Drawer opens from avatar. |
| Onboarding replay | Yes | Yes | Opens onboarding replay. |
| Croc BTI | Yes | Yes | Opens Croc BTI screen. |
| Sign in / Sign up | Yes | No | Opens auth flows only while signed out. |
| Profile | No | Yes | Opens profile settings only while signed in. |
| Leaderboard | No | Yes | Opens leaderboard only while signed in. |
| Settings | No | Yes | Opens settings only while signed in. |

## Leaderboard And Image Vote

| Case | Steps | Expected Result |
|---|---|---|
| Empty leaderboard | Open Leaderboard with no local summaries. | Empty local leaderboard message appears; no Supabase RPC is required. |
| Local leaderboard rows | Seed or create local summary, open Leaderboard. | Ranked rows render from `RewardImageClient.getLocalLeaderboard`. |
| Image vote list | Open Leaderboard, select image vote mode. | Approved local images appear sorted by vote count; missing image files show broken-image fallback. |
| Vote success | Tap image vote action. | Vote records through `RewardImageClient.vote`; list refreshes and stays on page. |
| Vote failure | Force duplicate/network/domain error and tap vote. | Failure snackbar appears; image vote page remains usable. |
| Upload entitlement failure | Force no entitlement and call upload flow/client. | Error is surfaced as a bridge/domain error, not hidden as success. |
| Create upload failure | Force missing file/upload failure. | Error is visible; no phantom image appears in ranking. |

## Release Device Smoke

| Case | Steps | Expected Result |
|---|---|---|
| Analyze/build gate | Run Flutter analyze and Rust focused tests before device install. | No new errors; known info warnings are documented. |
| Device cold start | Install release build, kill app, launch fresh. | Today loads; immediate Study does not bounce back. |
| Full loop | Study one question, return Today, open Wrong Words, Reports, AI, Leaderboard. | All pages open, use persisted/typed clients, and preserve current visual behavior. |
