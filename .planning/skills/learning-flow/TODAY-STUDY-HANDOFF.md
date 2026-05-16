# Today / Plan / Study Handoff

## Purpose

Use this guide when changing Today, active plan fallback, root navigation, Study launch, resume hints, startup auth/local-owner refresh, or Today progress truth.

## Canonical Files

Flutter:

- `apps/flutter_mobile/lib/state/app_state.dart`
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`
- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `apps/flutter_mobile/lib/features/plan_screen.dart`
- `apps/flutter_mobile/lib/sdk/sdk.dart`
- `apps/flutter_mobile/lib/sdk/today_client.dart`
- `apps/flutter_mobile/lib/sdk/plan_client.dart`
- `apps/flutter_mobile/lib/sdk/study_client.dart`

Rust/data:

- `crates/platform-mobile/src/bridge.rs`
- `crates/app-core/src/facade/today_facade.rs`
- `crates/app-core/src/facade/study_facade.rs`
- `crates/storage-core/src/models/today_home_state.rs`
- `crates/storage-core/src/persistence/schema.rs`

Tests:

- `apps/flutter_mobile/test/auth_session_manager_test.dart`
- `apps/flutter_mobile/test/today_task_breakdown_test.dart`

## Workflow

1. Identify whether the bug is display-only Today state, active plan state, Study start/resume, persisted results, or startup owner/auth refresh.
2. Inspect `TodayShellScreen._loadHomeBundle` and `_openStudy` before changing Rust.
3. Inspect `MobileRootShell._handleAppStateChanged` if Study closes, reloads, or bounces to Today.
4. Inspect Rust Today builders only after confirming Flutter is calling the right typed SDK path.
5. Preserve Today fallback as display-only unless the user explicitly applies/syncs the plan.

## Invariants

- First cold-start owner resolution must not close a user-requested Study route.
- A true owner change after an initial owner is established may clear Study and reload user-owned surfaces.
- Today snapshot fallback can display active plan rows but must not implicitly apply the plan.
- Today progress must come from persisted completions/active session truth, not current page index.
- Study launch must respect the user-selected mode and only use resume hints as hints.

## Verification

Run or record:

```powershell
D:\flutter\flutter\bin\flutter.bat analyze --no-pub
D:\flutter\flutter\bin\flutter.bat test --no-pub test\auth_session_manager_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\today_task_breakdown_test.dart -r expanded
```

If Flutter tests time out in this environment, mark them `BLOCKED`, not passed.

Manual release smoke:

1. Kill app.
2. Launch fresh.
3. Wait for Today content.
4. Immediately enter Study.
5. Confirm Study does not bounce back to Today.
6. Submit one answer and return Today.
7. Confirm progress refreshes without stale session creation.

## Stale Paths To Avoid

- `apps/mobile/**` React Native route logic.
- UI delays, timers, or one-frame ignore hacks for startup state races.
- Page-local Today progress counters as persisted truth.
