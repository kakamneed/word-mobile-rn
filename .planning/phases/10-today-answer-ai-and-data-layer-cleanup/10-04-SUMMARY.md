# 计划 10-04 总结

## 已完成

- 明确 Flutter 排行榜渲染是 local-first，并通过 `RewardImageClient` 走 Rust/SQLite 后端，不依赖 Supabase `LeaderboardService`。
- 从 `LeaderboardScreen` 移除了 active `LeaderboardService` 依赖；远程排行榜 service 现在被明确标注为 future-only，并与本地 Today/Study/Reports/WrongWords/AI/leaderboard 正确性隔离。
- 为 account drawer / sidebar 各入口增加稳定 key，供计划 10-05 建立路由 smoke matrix。
- 保留图片投票/上传的所有权边界：仍通过 Rust/SQLite reward-image API 和既有 UI 行为完成。
- 将 bridge error 的用户提示更新为清晰的中文用户可见文案。

## 变更文件

- `apps/flutter_mobile/lib/features/account_drawer.dart`
- `apps/flutter_mobile/lib/features/leaderboard_screen.dart`
- `apps/flutter_mobile/lib/supabase/leaderboard_service.dart`
- `apps/flutter_mobile/lib/bridge/bridge_error.dart`
- `crates/platform-mobile/src/bridge.rs`（来自此前 Rust formatting 的格式噪声）

## 验证

- `cargo test -p word-platform-mobile reward --lib` 通过：3 passed。
- `cargo test -p word-platform-mobile leaderboard --lib` 通过：1 passed。
- `cargo test -p word-platform-mobile image --lib` 通过：5 passed。
- `D:\flutter\flutter\bin\flutter.bat analyze --no-pub` 完成，仍是此前同样 4 个 info 级提示：
  - `profile_settings_screen.dart` 中 deprecated `withOpacity` 用法
  - `reward_image_client.dart` 中 null-aware element 风格建议

## 备注

- 未修改 React Native 实现文件。
- Cloud/Supabase 排行榜代码仍保留，但只作为未来远程集成边界。本地排行榜、图片投票、reward-image 上传资格仍由 Rust/SQLite 支撑。
