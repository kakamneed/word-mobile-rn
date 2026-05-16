# Release Validation And Human UAT

## Purpose

Use this guide before closing any Flutter learning-flow change that touches UI, Study, Today, bridge/Rust, SQLite persistence, AI, sync/auth/Supabase, leaderboard, reward images, or packaged assets.

## Automated Gates

Always run or explicitly record why skipped:

```powershell
D:\flutter\flutter\bin\flutter.bat analyze --no-pub
cargo test -p word-app-core study --lib
cargo test -p word-study-core --lib
cargo test -p word-platform-mobile reward --lib
cargo test -p word-platform-mobile leaderboard --lib
cargo test -p word-platform-mobile image --lib
```

Run focused Flutter tests for touched surfaces. If the runner times out in this environment, mark the check `BLOCKED` and keep analyzer/Rust gates separate.

## Release Device Smoke

Preferred deploy command:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1
```

Use the project-local `skill/flutter-android-release-wireless-deploy` workflow for details. Do not use the legacy React Native wireless deploy skill for Flutter.

## Human UAT Checklist

1. Kill app process.
2. Launch fresh release build.
3. Wait until Today content is visible.
4. Immediately tap the primary Study action.
5. Confirm Study stays open and does not bounce back to Today.
6. Submit one answer.
7. Return to Today and confirm progress refreshes.
8. Open AI, Wrong Words, Reports, leaderboard, image vote mode, account drawer entries, onboarding replay, Croc BTI, profile, and settings as applicable.
9. Confirm visible errors are non-blocking and current visual behavior/effects remain acceptable.

## Result Accounting

Use these statuses:

- `PASS`: command or smoke ran and passed.
- `PASS WITH NOTE`: ran and passed with documented environmental caveat.
- `BLOCKED`: could not complete due environment/tool timeout; do not count as pass.
- `DEFERRED`: intentionally not run yet, with exact command and reason.
- `HUMAN NEEDED`: requires connected device or visual/manual judgment.

Every phase verification must distinguish automated proof from deferred human/device work.
