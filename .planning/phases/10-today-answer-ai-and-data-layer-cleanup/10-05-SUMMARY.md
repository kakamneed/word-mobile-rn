# 计划 10-05 总结

## 已完成

- 在 `sidebar_smoke_test.dart` 中增加侧边栏/root smoke 覆盖，验证 Phase 10 根路由清单以及登录/未登录 drawer 入口。
- 在 `leaderboard_screen_test.dart` 中增加排行榜 smoke 覆盖，包含空本地 summary、本地数据行、图片投票成功、图片投票失败、reward image 上传资格失败和上传失败。
- 创建 `10-05-SMOKE-MATRIX.md`，记录手动与 release-device smoke 场景，覆盖冷启动 -> Study -> submit -> Today 刷新、侧边栏入口、AI、Wrong Words、Reports、leaderboard、image voting 和 image upload failure。

## 变更文件

- `apps/flutter_mobile/test/sidebar_smoke_test.dart`
- `apps/flutter_mobile/test/leaderboard_screen_test.dart`
- `.planning/phases/10-today-answer-ai-and-data-layer-cleanup/10-05-SMOKE-MATRIX.md`

## 验证

- `D:\flutter\flutter\bin\flutter.bat analyze --no-pub` 完成，仍是 Phase 10 全程相同的 4 个 info 级提示：
  - `profile_settings_screen.dart` 中 deprecated `withOpacity` 用法
  - `reward_image_client.dart` 中 null-aware element 风格建议
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\sidebar_smoke_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\leaderboard_screen_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。
- `10-05-SMOKE-MATRIX.md` 现在记录了 Phase 10 validation-results ledger：
  - Rust 聚焦 gate 和 Flutter static analysis 标为 `PASS` 或 `PASS WITH NOTE`。
  - 超时的 Flutter focused test 标为 `BLOCKED`，不计为通过。
  - Release-device deploy/smoke 标为 `DEFERRED`，并写明精确 deploy 命令和待填写 device result。

## 备注

- 未修改 React Native 实现文件。
- 当前环境里 Flutter test runner 的超时在多个 focused Flutter test 中一致出现；analyzer 接受了新增测试，且没有发现新增错误。
- Release-device smoke 仍需要连接 Android 目标设备，用于验证冷启动生命周期 timing 和视觉/路由保真。
