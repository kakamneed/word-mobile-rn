# Regression Guardrails

## Guardrail Matrix

| Risk | Required Check | Automated Surface | Manual Surface |
|---|---|---|---|
| Selected wrong option not red | Wrong selected option red/cancel and correct option green/check | `study_question_display_test.dart`; `choiceStateForTest` | Submit wrong choice on device |
| Correct answer drifts to A | Non-A correct label preserved through DTO and result | `study_client_test.dart`; `cargo test -p word-app-core study --lib`; `cargo test -p word-study-core --lib` | Use B/C/D correct cards |
| Cold-start Study bounce | First owner resolution does not close Study | `auth_session_manager_test.dart`; `rootOwnerChangedAfterInitialLoadForTest` | Kill app -> launch -> immediately enter Study |
| Today fallback mutates truth | Active plan fallback is display-only | `today_task_breakdown_test.dart`; `todaySnapshotOrPlanFallbackForTest` | Clear Today snapshot with active plan and open Today |
| AI blocks main loop | AI errors visible and non-blocking | `ai_passage_generation_test.dart`; Rust AI/wrong tests | Force AI error, then open Today/Study |
| Wrong Words persistence | Hint/mastered/trash persists and reloads | `cargo test -p word-platform-mobile wrong --lib` | Save hint, reload Wrong Words |
| Reports persistence | Reports read persisted aggregate | `cargo test -p word-app-core reports --lib` | Complete session, open Reports |
| Sidebar route drift | Signed-in/out drawer inventory correct | `sidebar_smoke_test.dart`; route inventory helper | Traverse drawer entries |
| Leaderboard/image vote drift | Local leaderboard and image vote use `RewardImageClient` | `leaderboard_screen_test.dart`; Rust reward/leaderboard/image tests | Vote, duplicate/failure, missing image fallback |
| Release packaging/lifecycle | Release APK includes bridge and Supabase defines | Flutter release deploy script | Full cold-start device smoke |

## Command Set

Reliable gates in this environment:

```powershell
D:\flutter\flutter\bin\flutter.bat analyze --no-pub
cargo test -p word-app-core study --lib
cargo test -p word-study-core --lib
cargo test -p word-app-core reports --lib
cargo test -p word-platform-mobile wrong --lib
cargo test -p word-platform-mobile reward --lib
cargo test -p word-platform-mobile leaderboard --lib
cargo test -p word-platform-mobile image --lib
```

Focused Flutter tests to run when feasible:

```powershell
D:\flutter\flutter\bin\flutter.bat test --no-pub test\auth_session_manager_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\today_task_breakdown_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_question_display_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_client_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\ai_passage_generation_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\ai_wrong_word_import_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\sidebar_smoke_test.dart -r expanded
D:\flutter\flutter\bin\flutter.bat test --no-pub test\leaderboard_screen_test.dart -r expanded
```

If these time out, record `BLOCKED` with duration and no failure output.

## Release Device Required

Use:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1
```

Record result in the relevant phase smoke matrix or UAT file.
