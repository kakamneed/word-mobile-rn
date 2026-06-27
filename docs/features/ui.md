# Feature: UI System

> Slug: `ui`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

建立移动端稳定的信息架构和视觉层级，让学习任务、错词、AI、报告等页面从调试壳回到产品态。

## UX Contract

- 主导航稳定：今日、计划、错词、报告、AI。
- 调试诊断不占据主屏优先级。
- 卡片和面板只在承载真实内容时使用；AI 工作台已转向对话流。
- 文案必须可读，避免 mojibake 和转义 Unicode 出现在 UI。

## Shared Domain/Data Contract

UI 不拥有领域真相。Flutter 页面通过 SDK/bridge 消费 Rust 和 Supabase 合同。

## Flutter Mobile Route

- Owner screen/widget: `MobileRootShell` 和各 feature screen。
- SDK/bridge calls: 分页面隔离，Shell 负责缓存和 reload seed。
- Loading/cache/reload behavior: 使用 ShellPageDataCache 和 stale-while-revalidate，避免空白等待。
- Orientation/gesture constraints: 普通页面竖屏，图谱可横屏。
- First implementation slice: RN parity 主壳、Today 产品态、AI workbench。
- Current status: mobile in progress.

## Tauri Desktop Route

- Owner view/window: 桌面主窗口与侧栏/多面板布局。
- Shared APIs to reuse: Flutter 已验证的 Rust bridge 语义应转成 Tauri commands。
- Desktop-specific layout: 左侧导航/顶部工具栏/可调整面板，不复制手机底栏。
- Mobile assumptions to avoid: 小屏卡片堆叠、底部 composer 固定、手机键盘避让。
- First parity slice: 复刻主信息架构和页面数据流，不复刻像素布局。
- Current status: planned.

## Sync And Storage

UI 层缓存只做呈现优化，不能成为跨端同步真相。

## AI Or Provider Implications

AI 页 UI 要展示 provider 错误但不能泄露调试入口给普通用户。

## Implementation Log

- `2026-04-24`: Flutter Today 从调试壳改为 RN 风格任务主屏，诊断信息折叠。
- `2026-04-24`: Flutter 主壳加入五 tab 导航。
- `2026-05-09`: AI 页由卡片堆叠改为 workbench。
- `2026-06-25`: 建立 UI 总记录。

## Mobile Lessons Learned

- 调试信息需要保留，但不能成为 Today 首屏。
- AI 页旧卡片结构不适合继续扩展，功能选择+对话流更好。
- 字符串乱码会直接破坏信任，修改中文文案时要局部验证。

## Desktop Follow-Up Notes

桌面端应继承信息架构和数据流，而不是继承手机端底部导航。AI workbench 可改为中间对话流+侧边工具栏。

## Route Changes

- `2026-06-25`: UI 作为跨页面基础能力单独记录。

## Known Pitfalls

- 不要把原始 map/json 显示给用户。
- 不要复制移动端的屏幕限制到桌面端。
- 不要做无归属的通用组件重构。

## Verification

- Mobile: Flutter analyze 历史记录多次通过；部分超时需按功能记录补证据。
- Desktop: pending.
- Shared/domain: UI 不改变领域测试。