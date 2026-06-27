# Feature: Knowledge Graph

> Slug: `knowledge-graph`
> Status: `planned`
> Updated: `2026-06-25`

## Product Intent

这是“知识图谱”总分类记录。当前第一项具体功能是错词图谱，详见 `docs/features/wrong-word-graph.md`。

## UX Contract

- 当前预期不是记忆宫殿，而是 Obsidian-like 错词关系图。
- 点击节点后高亮关联节点，出现关系轮盘。
- 关系层：白色形近词、绿色近义词、红色同场错词、紫色同根词。
- 移动端可进入横屏，右侧小栏放错词，支持拖入空间。

## Shared Domain/Data Contract

知识图谱应复用 wrong-word graph 的 nodes/edges/position 合同，并为未来记忆宫殿保留独立模式边界。

## Flutter Mobile Route

- Owner screen/widget: planned graph route。
- SDK/bridge calls: planned graph seed/position APIs。
- Loading/cache/reload behavior: 图谱 seed 可缓存，位置保存要乐观更新。
- Orientation/gesture constraints: 横屏优先，拖拽、缩放、选择、轮盘。
- First implementation slice: wrong-word graph shell。
- Current status: planned。

## Tauri Desktop Route

- Owner view/window: desktop graph workspace。
- Shared APIs to reuse: graph seed/position/relation APIs。
- Desktop-specific layout: 大画布、侧栏、筛选器、hover inspector。
- Mobile assumptions to avoid: 不强制横屏，不复制手机右栏比例。
- First parity slice: render same graph contract。
- Current status: planned.

## Sync And Storage

用户摆放位置是用户数据；派生边可重新计算。

## AI Or Provider Implications

AI 可辅助聚类、解释关系、后续记忆宫殿生图，但核心图谱不依赖 AI。

## Implementation Log

- `2026-06-25`: 从用户 Obsidian 参考图和轮盘关系设定建立记录。

## Mobile Lessons Learned

- 第一版应 2.5D，不要真 3D 起步。
- 记忆宫殿应独立成后续模式。

## Desktop Follow-Up Notes

桌面端从 wrong-word graph 合同开始，不要另起一套关系计算。

## Route Changes

- `2026-06-25`: `knowledge-graph` 作为总分类，`wrong-word-graph` 作为首个具体功能。

## Known Pitfalls

- 不要把图谱做成泛用数据库浏览器。
- 不要让颜色成为唯一信息载体。

## Verification

- Mobile: pending。
- Desktop: pending。
- Shared/domain: pending。