# Mobile Feature Record Index

> Updated: `2026-06-27`

本索引用于把移动端功能记录固定到稳定分类中。每个功能记录都应写清楚：移动端已知问题、解决方法、后续路线、桌面端 Tauri 如何同步，以及验证状态。

全局 skill: `C:\Users\clf20\.codex\skills\cross-platform-feature-ledger\SKILL.md`

## Categories

- 动画: `docs/features/animation.md`
- UI: `docs/features/ui.md`
- 学习: `docs/features/learning.md`
- 计划页: `docs/features/plan-page.md`
- 侧边栏功能: `docs/features/sidebar-features.md`
- 公告和推送: `docs/features/announcements-push.md`
- 鳄 BTI: `docs/features/croc-bti.md`
- Today 页: `docs/features/today-page.md`
- AI 短文: `docs/features/ai-passage.md`
- AI 页面工作台: `docs/features/ai-page-chat-workbench.md`
- 知识图谱: `docs/features/knowledge-graph.md`
- 错词图谱: `docs/features/wrong-word-graph.md`
- 错词本: `docs/features/wrong-words-page.md`
- 词库: `docs/features/word-library.md`
- 更新: `docs/features/updates.md`
- Supabase: `docs/features/supabase.md`
- 云服务: `docs/features/cloud-services.md`

## Cross-Feature Links

- 错词图谱依赖错词本的 `getWrongWords` / `getWrongWordDetail` 语义，并复用 `entryKind`、错误次数、root/affix 分类和导入错词规则。
- 错词图谱依赖学习流的 `study_results`，其中红色 `coOccurrence` 边来自同一学习 session 内共同出错或跳过的词。
- 错词图谱未来可读取 AI 短文历史，把同一篇短文里被覆盖的错词补充为红色上下文边。
- 知识图谱是产品层级分类，错词图谱是当前第一个落地子功能；记忆宫殿应作为后续独立模式，不与本次 2.5D 图谱混在一起。

## Current Verification Notes

- `cargo check -p word-platform-mobile` passed on 2026-06-27 for the graph bridge and relation layers.
- Flutter graph file formatting passed on 2026-06-27.
- `flutter test test\study_question_display_test.dart` is currently blocked by unrelated existing mojibake syntax errors in `apps/flutter_mobile/lib/features/study_screen.dart` and a missing/renamed `WordHintSuggestion` test symbol. Do not treat that as a wrong-word graph runtime failure without first repairing the learning-flow file.
- Dart analyzer for the graph file timed out locally after 120 seconds; keep this as a local tooling risk until the broader Flutter source tree is parse-clean again.

## Maintenance Rule

当移动端某个分类新增问题、修复、策略调整或验证结果时，先更新对应分类记录；如果该变更会影响桌面端实现，再同步更新 `D:\projects\word-desktop-tauri\docs\features\` 中对应或相关记录。
