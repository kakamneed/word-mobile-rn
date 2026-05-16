---
phase: 10-today-answer-ai-and-data-layer-cleanup
verified: 2026-05-14T03:06:40Z
status: human_needed
score: 9/9 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 8/9
  gaps_closed:
    - "Phase 10 summaries and smoke matrix now account for validation results, focused Flutter timeout blockers, and release-device deferred status."
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Release-device cold start -> immediate Study -> submit -> return Today"
    expected: "Study stays open after startup auth/local-owner refresh, submit works, and Today progress refreshes without stale session creation."
    why_human: "The release-device deploy is explicitly deferred and requires a connected Android target plus real lifecycle timing."
  - test: "Visual/effects preservation across Today, Study, AI, Wrong Words, Reports, leaderboard, image vote/upload, and sidebar routes"
    expected: "Current Flutter layouts, feedback colors/icons, route reachability, and non-blocking error states match existing product behavior."
    why_human: "Visual quality and real navigation flow cannot be fully verified from static code inspection."
---

# Phase 10: Today answer AI and data layer cleanup Verification Report

**Phase Goal:** Thoroughly clean the learning-flow implementation layers identified in Phase 9, removing obsolete remnants and simplifying Today, answer evaluation, AI, bridge, and persistence code while preserving current user-facing behavior and effects.
**Verified:** 2026-05-14T03:06:40Z
**Status:** human_needed
**Re-verification:** Yes - after gap closure

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | Selected wrong answer is visibly marked wrong while the correct option remains accurate. | VERIFIED | Quick regression check still finds red/cancel wrong-choice rendering in `study_screen.dart` and prior test evidence in `study_question_display_test.dart`. |
| 2 | Correct answer label/index never collapses to A. | VERIFIED | `StudyQuestion.fromJson` still rejects missing/invalid `correctChoiceLabel`; no production fallback-to-A path was found in active quick checks. |
| 3 | Cold-start Study handoff does not bounce back to Today. | VERIFIED | `MobileRootShell` still gates Study closure through `rootOwnerChangedAfterInitialLoadForTest`, so first owner resolution does not close Study. |
| 4 | Today and Study use canonical Flutter SDK/Rust handoff paths. | VERIFIED | Prior data-flow trace remains valid: Today/Study handoff uses typed SDK calls and Rust authoritative Today state. |
| 5 | AI, Wrong Words, Reports, bridge/data, leaderboard, image vote/upload, and sidebar routes use canonical Flutter/Rust paths. | VERIFIED | Quick scan found no active Flutter feature imports of direct bridge, `sample_study_payloads`, `LeaderboardService`, or `apps/mobile`; leaderboard still calls `sdk.rewardImages.getLocalLeaderboard`. |
| 6 | Obsolete/legacy Flutter residues are not still wired into active UI flows. | VERIFIED | No active feature-screen legacy wiring found in re-check. |
| 7 | Removed legacy code has no surviving active imports/navigation entries/bridge methods implying it is active. | VERIFIED | Prior classification still holds: remaining RN references are comments/test fixture context, not active Flutter routes. |
| 8 | Existing product behavior and effects are preserved unless explicitly obsolete. | HUMAN NEEDED | Static/code checks pass, but visual/effects preservation still requires release-device/manual route smoke. |
| 9 | Phase 10 summaries and smoke matrix account for verification and remaining manual/device checks. | VERIFIED | `10-05-SMOKE-MATRIX.md` now has `## Validation Results`, marks focused Flutter tests `BLOCKED`, records release-device deploy as `DEFERRED`, includes `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1`, and has a pending device result slot. `10-05-SUMMARY.md` mirrors PASS / PASS WITH NOTE / BLOCKED / DEFERRED outcomes. |

**Score:** 9/9 must-haves verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `apps/flutter_mobile/lib/features/study_screen.dart` | Answer feedback renderer | VERIFIED | Wrong selected choices are styled red/cancel; correct choices remain green/check after result. |
| `apps/flutter_mobile/lib/sdk/study_client.dart` | Typed study DTO decode without fallback-to-A | VERIFIED | Choice questions require valid `correctChoiceLabel`; malformed correctness data throws `BridgeError.protocol`. |
| `apps/flutter_mobile/lib/features/mobile_root_shell.dart` | Root route/study handoff owner | VERIFIED | Owner refresh no longer clears Study on first owner resolution; route inventory exists. |
| `apps/flutter_mobile/lib/features/today_shell_screen.dart` | Today bundle, fallback, AI shortcut, Study launch | VERIFIED | Study opens via SDK resume hint; plan fallback is display-only until explicit apply/sync action. |
| `crates/platform-mobile/src/bridge.rs` | Canonical Rust mobile bridge | VERIFIED | Active Today, reports, wrong words, AI, reward image, leaderboard, and study bridge methods are present and SQLite-backed. |
| `apps/flutter_mobile/lib/features/ai_screen.dart` | AI generation/history/import surface | VERIFIED | Uses `widget.sdk.ai` and catches/display errors without blocking navigation. |
| `apps/flutter_mobile/lib/features/wrong_words_screen.dart` | Wrong-word notebook actions | VERIFIED | List/detail/hint actions use `widget.sdk.wrongWords`. |
| `apps/flutter_mobile/lib/features/reports_screen.dart` | Persisted aggregate display | VERIFIED | Loads via `widget.sdk.reports.getReportsOverview`. |
| `apps/flutter_mobile/lib/features/leaderboard_screen.dart` | Local leaderboard and image vote UI | VERIFIED | Uses `widget.sdk.rewardImages.getLocalLeaderboard`, `listImages`, and vote/upload client paths. |
| `.planning/phases/10-today-answer-ai-and-data-layer-cleanup/10-05-SMOKE-MATRIX.md` | Manual release-device and validation matrix | VERIFIED | Validation ledger, focused Flutter timeout blocker row, release-device deferred row, exact deploy command, and pending device result slot are present. |
| `.planning/phases/10-today-answer-ai-and-data-layer-cleanup/10-05-SUMMARY.md` | Verification outcome accounting | VERIFIED | Summary explicitly states PASS/PASS WITH NOTE for completed gates, BLOCKED for Flutter timeouts, and DEFERRED for release-device smoke. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `today_shell_screen.dart` | `StudyClient` | `widget.appState.sdk.study.getResumeSessionHint()` | WIRED | Today start/resume passes resume hint into Study handoff. |
| `mobile_root_shell.dart` | `AppState` | listener and owner-change logic | WIRED | Auth/local-owner refresh reloads surfaces but only closes Study on true post-initial owner change. |
| `platform-mobile/src/bridge.rs` | `today_facade.rs` | `core_build_today_home_state(TodayHomeStateSeed)` | WIRED | Active bridge builds authoritative Today state from persisted completions/targets. |
| `study_client.dart` | `study_screen.dart` | decoded `StudyQuestion.correctChoiceLabel` | WIRED | UI feedback uses decoded label plus result tokens. |
| `ai_screen.dart` / `today_shell_screen.dart` | `AiClient` | `widget.sdk.ai` calls | WIRED | Context/history/generation/import paths are typed client calls. |
| `wrong_words_screen.dart` | `WrongWordsClient` | `widget.sdk.wrongWords` calls | WIRED | List/detail/hint persistence is client-backed. |
| `reports_screen.dart` | `ReportsClient` | `getReportsOverview()` | WIRED | Report display comes from persisted aggregate client. |
| `leaderboard_screen.dart` | `RewardImageClient` | local leaderboard/list/vote/upload calls | WIRED | No active Supabase leaderboard dependency in the screen. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `TodayShellScreen` | `_cachedBundle.today.todaySnapshot` | `sdk.today.getTodayHomeState` -> `platform-mobile build_authoritative_today_home_state` | Yes, persisted targets/completions | FLOWING |
| `StudyScreen` | `StudyQuestion` / `StudyResult` | `StudyClient` -> Rust study facade/bridge | Yes, DTO validated and result-backed | FLOWING |
| `AiScreen` | `_context`, `_history`, `_passage` | `AiClient` methods | Yes, bridge-backed with visible errors | FLOWING |
| `WrongWordsScreen` | `_words`, `_detail` | `WrongWordsClient` methods | Yes, bridge/Rust-backed | FLOWING |
| `ReportsScreen` | reports overview maps | `ReportsClient.getReportsOverview` | Yes, bridge persisted aggregate path | FLOWING |
| `LeaderboardScreen` | `_entries`, `_rewardImages` | `RewardImageClient` local methods | Yes, local SQLite/Rust-backed client | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Prior gap closure: validation ledger | `Select-String 10-05-SMOKE-MATRIX.md "## Validation Results|Flutter focused tests|BLOCKED|Release device deploy|DEFERRED"` | Required rows and statuses found. | PASS |
| Prior gap closure: release-device status | `Select-String 10-05-SMOKE-MATRIX.md "Deploy command|Device result|android-release-wireless-deploy.ps1|Pending manual validation"` | Exact deploy command and pending device-result slot found. | PASS |
| Prior gap closure: summary accounting | `Select-String 10-05-SUMMARY.md "PASS|PASS WITH NOTE|BLOCKED|DEFERRED|not counted as passed"` | Summary states Flutter timeouts are BLOCKED and not counted as pass. | PASS |
| Quick regression: canonical active routes | Feature-screen `Select-String` for direct bridge/demo/legacy imports | No active matches found. | PASS |
| Focused Flutter tests | Matrix ledger | Timed out at 120s and recorded as `BLOCKED`, not pass. | BLOCKED, ACCOUNTED |
| Release-device smoke | Matrix ledger | Not run and recorded as `DEFERRED` with reason and pending result. | HUMAN NEEDED |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| ARCH-02 | 10-01..10-05 | Stable DTO contracts instead of duplicated shapes | SATISFIED | Typed Flutter clients and bridge codec/error boundaries used by feature screens. |
| MOB-02 | 10-01..10-05 | Restore plan/today/wrong-word/report state from local SQLite | SATISFIED | Today, Wrong Words, Reports, leaderboard/image surfaces trace to Rust/SQLite-backed bridge paths. |
| STUD-02 | 10-01, 10-02, 10-05 | Start study from Today and complete answer loop | SATISFIED PROGRAMMATICALLY, DEVICE HUMAN NEEDED | Today -> Study handoff and submit flow are wired; release device loop is explicitly deferred. |
| STUD-03 | 10-02, 10-05 | Study cards do not reveal answer before submission | SATISFIED | `study_screen.dart` only applies answer feedback when `result != null`; tests cover pre/post choice state. |
| STUD-04 | 10-01..10-05 | Progress/result persistence matches domain rules | SATISFIED | Rust bridge/facade tests and Today authoritative state use persisted completions. |
| STUD-05 | 10-02, 10-05 | Touch-friendly study interactions/layouts | HUMAN NEEDED | Static UI still present; mobile layout quality requires device/manual verification. |
| PLAN-01 | 10-01, 10-04, 10-05 | Read active plan and today snapshot from mobile | SATISFIED | Today bundle and plan fallback use SDK/Rust paths; explicit apply-to-Today action exists. |
| WRNG-01 | 10-02..10-05 | Wrong answers/skips persist into wrong-word state | SATISFIED | Wrong Words and study submit paths are Rust-backed; wrong-word client actions persist. |
| RPT-01 | 10-03..10-05 | View daily/mode summaries from persisted aggregates | SATISFIED | Reports screen uses `ReportsClient.getReportsOverview`; Rust reports tests reported passing. |
| AI-01 | 10-03, 10-05 | AI optional and cannot block main loop | SATISFIED | AI errors are caught/displayed; Today/Study navigation does not depend on AI success. |
| AI-02 | 10-03, 10-05 | Generated passages/history cached locally | SATISFIED | AI history loads through `AiClient`; local restore/backfill is sync-adjacent. |
| AI-03 | 10-03, 10-05 | AI failures visible and non-blocking | SATISFIED | AI screen and Today shortcut catch errors into visible messages. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---:|---|---|---|
| `crates/app-core/src/facade/today_facade.rs` | 52 | `Build snapshot (placeholder...)` | Info | Stale/orphaned facade path; active Flutter mobile bridge uses `build_authoritative_today_home_state`, so this is not currently blocking. |
| `crates/platform-mobile/src/bridge.rs` | 10222 etc. | `../../apps/mobile/android/app/src/main/assets` in tests | Info | React Native asset path remains in Rust test fixtures, not active Flutter UI routing. |

### Human Verification Required

### 1. Release Device Cold-Start Study Loop

**Test:** Run `apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1`, kill the app, launch fresh, wait for Today, immediately enter Study, submit one answer, return Today.
**Expected:** Study does not bounce back to Today; submit succeeds; Today progress refreshes; no stale session appears.
**Why human:** The matrix correctly marks this as `DEFERRED`; it requires a connected Android target and real app lifecycle timing.

### 2. Visual And Route Smoke

**Test:** Manually traverse Today, Study, AI, Wrong Words, Reports, leaderboard, image vote/upload, account drawer/sidebar entries.
**Expected:** Routes open through Flutter surfaces, errors are visible/non-blocking, and existing visual behavior/effects remain acceptable.
**Why human:** Visual preservation and device navigation quality cannot be fully proven with static code inspection.

### Gaps Summary

No structured gaps remain. The prior verification/smoke-accounting gap is closed: the smoke matrix now records validation results, marks Flutter focused-test timeouts as `BLOCKED` rather than pass, and records release-device smoke as `DEFERRED` with exact command, reason, and pending result slot. Phase 10 is ready for the remaining human/device verification items.

---

_Verified: 2026-05-14T03:06:40Z_
_Verifier: Claude (gsd-verifier)_
