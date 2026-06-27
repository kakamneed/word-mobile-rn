# 14-06 Summary - WeChat Mini Program source parity correction

## Scope

This pass corrected the specific regressions found after comparing the mini program with the Flutter implementation:

- Text icons in Study and the global shell were replaced with CSS-drawn icons.
- Mojibake in Study, Reports, Plan, and drawer navigation was removed.
- Study answer feedback now stays on the answered question and shows correct/wrong markers instead of auto-jumping forward.
- Unfinished study progress now writes a resume hint to Today and resumes the same in-memory session instead of restarting.
- Today now supports native pull-down refresh and re-fetches home/reward state from the SDK.
- Today task rows now read per-mode progress instead of showing hard-coded `0/total`.
- Root-affix mode is present in Today and has a four-question input chain.
- Study golden now completes all five modes and validates each completion summary.
- Reports chart axes, guides, dots, and line segments now share the same chart geometry source.
- Source-integrity checks now block common mojibake and literal icon text regressions.

## Changed Files

- `apps/wechat_miniprogram/src/pages/study/index.tsx`
- `apps/wechat_miniprogram/src/pages/study/index.scss`
- `apps/wechat_miniprogram/src/components/Screen.tsx`
- `apps/wechat_miniprogram/src/components/Screen.scss`
- `apps/wechat_miniprogram/src/pages/reports/index.tsx`
- `apps/wechat_miniprogram/src/pages/reports/index.scss`
- `apps/wechat_miniprogram/src/pages/plan/index.tsx`
- `apps/wechat_miniprogram/src/pages/today/index.tsx`
- `apps/wechat_miniprogram/src/pages/today/index.scss`
- `apps/wechat_miniprogram/src/app.config.ts`
- `apps/wechat_miniprogram/src/sdk/client.ts`
- `apps/wechat_miniprogram/src/sdk/types.ts`
- `apps/wechat_miniprogram/src/sdk/mockData.ts`
- `apps/wechat_miniprogram/scripts/source-integrity-check.ts`
- `apps/wechat_miniprogram/scripts/study-flow-golden.ts`
- `apps/wechat_miniprogram/scripts/reports-golden.ts`

## Verification

Passed:

- `npm.cmd run test:source-integrity`
- `npm.cmd run typecheck`
- `npm.cmd run test:study-flow`
- `npm.cmd run test:reports`
- `npm.cmd run test:croc-bti`
- `npm.cmd run test:plan-flow`
- `npm.cmd run test:ui-contract`
- `npm.cmd run test:api-contract`
- `npm.cmd run build:weapp` compiled successfully; the wrapper command timed out after Webpack success while waiting for process exit.

Build warnings remain for oversized Croc BTI PNG assets. They do not block compilation, but should be optimized before upload if package size becomes an issue.
