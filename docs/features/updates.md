# Feature: Updates

> Slug: `updates`
> Status: `mobile_in_progress`
> Updated: `2026-06-25`

## Product Intent

让用户知道是否有新版本、如何更新，并保证移动端包身份和本地数据连续性。

## UX Contract

- 更新提示不能阻塞学习主流程，除非未来标记为强制。
- Flutter 替换 RN 时应保持应用身份，避免本地数据丢失。
- 版本检查和提示需可解释。

## Shared Domain/Data Contract

更新服务主要在客户端；本地数据连续性依赖 package id / bundle id / Rust SQLite 路径和加法迁移规则。

## Flutter Mobile Route

- Owner screen/widget: sidebar/settings/update prompt。
- SDK/bridge calls: app update service、runtime paths if needed。
- Loading/cache/reload behavior: best-effort，不阻塞 Today。
- Orientation/gesture constraints: 手机弹窗/设置页。
- First implementation slice: app update service tests 历史存在。
- Current status: mobile in progress。

## Tauri Desktop Route

- Owner view/window: desktop updater/settings。
- Shared APIs to reuse: 版本/公告云配置可共享，但桌面更新机制不同。
- Desktop-specific layout: Tauri updater 或自定义更新检查。
- Mobile assumptions to avoid: Android APK 安装路径和 package id 不适用。
- First parity slice: 显示版本和更新可用状态。
- Current status: planned.

## Sync And Storage

升级安装不能破坏本地 SQLite。卸载重装不算数据连续性方案。

## AI Or Provider Implications

无。

## Implementation Log

- `2026-04-22`: Flutter 替换 RN 计划明确应用标识和数据连续性规则。
- `2026-05-15`: app_update_service_test 历史通过。
- `2026-06-25`: 建立更新记录。

## Mobile Lessons Learned

- 应用显示名可改，applicationId/bundle id 不应随意改。
- 本地数据连续性依赖升级安装和加法迁移。

## Desktop Follow-Up Notes

桌面端应单独设计 Tauri updater，但保留版本提示和非阻塞原则。

## Route Changes

- `2026-06-25`: 更新作为独立记录维护。

## Known Pitfalls

- 不要把卸载重装当迁移路径。
- 不要把更新检查失败当启动失败。

## Verification

- Mobile: app_update_service_test 历史通过。
- Desktop: pending.
- Shared/domain: migration/data-continuity smoke。