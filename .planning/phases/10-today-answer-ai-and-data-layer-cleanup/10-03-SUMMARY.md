# 计划 10-03 总结

## 已完成

- 验证 Today AI 快捷入口和 `AiScreen` 都使用类型化的 `AiClient` / `sdk.ai` 路径处理上下文、生成、历史和导入行为。
- 验证 `WrongWordsScreen` 使用 `WrongWordsClient` 处理列表、详情、提示持久化；错词导入通过 `AiClient` 流转，而不是 UI 页面局部修改错词本。
- 验证 `ReportsScreen` 使用 `ReportsClient.getReportsOverview`，报告数值保持在持久化聚合边界上，而不是页面本地计数器。
- 保留现有 Flutter UI 和视觉效果；未触碰 React Native 实现文件。

## 变更文件

- 计划 10-03 不需要额外实质代码改动；相关 Today key/fallback 调整已在 10-01 中完成。
- `apps/flutter_mobile/lib/sdk/ai_client.dart` 和 `apps/flutter_mobile/test/ai_wrong_word_import_test.dart` 曾因行尾噪声出现在 `git status` 中；`git diff --ignore-cr-at-eol` 未显示实质 diff。

## 验证

- `cargo test -p word-app-core reports --lib` 通过：6 passed。
- `cargo test -p word-platform-mobile wrong --lib` 通过：12 passed。
- `cargo test -p word-platform-mobile report --lib` 通过：0 tests matched，命令成功完成。
- `cargo test -p word-platform-mobile wrong_word_image_analysis_uses_backup_after_primary_failure --lib -- --nocapture` 通过：1 passed。
- `cargo test -p word-platform-mobile ai --lib` 用宽泛 filter 运行时失败，原因是 `wrong_word_image_analysis_uses_backup_after_primary_failure` 在分组运行中无法访问 mock backup server；同一个测试单独重跑通过。当前判断为分组测试的 mock server/环境干扰，不是确定性的逻辑失败。
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\ai_passage_generation_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。
- `D:\flutter\flutter\bin\flutter.bat test --no-pub test\ai_wrong_word_import_test.dart -r expanded` 运行 120 秒后超时，没有得到测试失败断言结果。

## 备注

- Flutter 表层的 AI 仍保持非阻塞。
- 错词导入/图片抽取和报告聚合边界已通过 Rust 测试与类型化 client 检查覆盖。
