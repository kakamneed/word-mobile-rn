# 计划 10-01 总结

## 已完成

- 稳定了根页面的 auth/local-owner 刷新处理：冷启动时第一次解析 owner 不再强制关闭已经请求打开的 Study 页面。
- 移除了 Today bundle 加载时的副作用：当 Today snapshot 为空时，不再自动调用 `applySavedPlanToToday()`；现在 active plan 只作为展示 fallback，直到用户显式同步/应用计划。
- 在 `mobile_root_shell.dart` 增加了可测试的根路由/侧边栏清单钩子，并为 Today 的主要学习入口和 AI 快捷入口增加稳定 key，方便后续 smoke 验证。
- 增加了聚焦回归 helper/test，覆盖首次 owner 解析不会把 Study 弹回 Today，以及 Today plan fallback 只展示不写入。

## 变更文件

- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`
- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `apps/flutter_mobile/test/auth_session_manager_test.dart`
- `apps/flutter_mobile/test/today_task_breakdown_test.dart`

## 验证

- `D:\flutter\flutter\bin\flutter.bat analyze --no-pub` 完成，只有 4 个既有/info 级 analyzer 提示：
  - `profile_settings_screen.dart` 中 deprecated `withOpacity` 用法
  - `reward_image_client.dart` 中 null-aware element 风格建议
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\auth_session_manager_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\today_task_breakdown_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。

## 备注

- 未修改 React Native 实现文件。
- 冷启动修复刻意没有采用延迟、忽略首次返回之类的 UI hack；只有在已建立初始 owner 之后又发生真正 owner 变化时，才会清理 Study 路由。
