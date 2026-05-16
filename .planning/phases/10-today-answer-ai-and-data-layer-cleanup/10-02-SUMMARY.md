# 计划 10-02 总结

## 已完成

- 确认 Flutter 多选题展示路径在已有答题结果后，会把用户选错的选项标成红色/cancel，并把正确选项标成绿色/check。
- 确认 `StudyClient` 会拒绝缺失或非法 `correctChoiceLabel` 的选择题，避免静默 fallback 到 A。
- 保留并验证了 Rust 侧对非 A 正确答案、陈旧 A 标签场景的正确性保护。
- 修复 `study_facade` 里的空 resume 陈旧 snapshot 边界：当空 resume 请求删除了陈旧持久化 snapshot 后，现在返回 `NotEnoughWords`，不会继续落到无关的内存 active session。

## 变更文件

- `crates/app-core/src/facade/study_facade.rs`
- `crates/platform-mobile/src/bridge.rs`
- `crates/study-core/src/question_builder.rs`

说明：若干 Flutter study 文件曾在 `git status` 中出现，原因是行尾噪声；`git diff --ignore-cr-at-eol` 未显示本计划带来的实质 Flutter 变更。

## 验证

- `cargo test -p word-app-core study --lib` 通过：13 passed。
- `cargo test -p word-study-core --lib` 通过：28 passed。
- `D:\flutter\flutter\bin\flutter.bat analyze --no-pub` 完成，仍是计划 10-01 中同样 4 个 info 级提示：
  - `profile_settings_screen.dart` 中 deprecated `withOpacity` 用法
  - `reward_image_client.dart` 中 null-aware element 风格建议
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_question_display_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\study_client_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。

## 备注

- 未修改 React Native 实现文件。
- 两个关键历史回归已有 Flutter helper test 和 Rust test 覆盖：
  - 选错项标红，同时正确项保持绿色
  - 非 A 正确答案在 start、submit、history、resume、evaluator、builder 路径中保持真实标签
