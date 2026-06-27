# Summary 14-07: Function-level gap map and targeted parity fixes

## Completed

- Replaced `docs/wechat/miniprogram_flutter_gap_analysis.md` with a UTF-8 function-level comparison matrix.
- Added `docs/wechat/miniprogram_flutter_change_feedback_template.md` so future implementation reports must cite Flutter/Rust and mini-program functions/lines.
- Added Today page refresh-on-show and visible pull-down refresh indicator.
- Aligned Reports chart touch box, visual dot center, and line segment center with Flutter chart point math.
- Replaced the mini-program Reports CSS-rotated line segments with one SVG path generated from the exact same point coordinates as the dots.
- Rewrote the mini-program Wrong Words page Chinese labels in UTF-8, restoring Flutter's `错词本` / `筛选` / `强化入口` / `错词列表` copy.
- Added an explicit mini-program `study.preserveProgress()` SDK method and wired Study home/keep-exit to await it before returning to Today with `refresh=1`.
- Consolidated duplicated `ResumeSessionHint` type declarations so Today and Study share one resume/progress contract.
- Replaced the active root/affix mini-program fixture source with a structured `rootAffixFallbackPayloads` generator that mirrors Flutter's fallback payload shape and Rust's `build_root_affix_question` path.
- Added regression checks for:
  - report chart point/hitbox/visual center alignment
  - SVG chart path endpoints matching report chart points
  - Today refresh-on-show and refresh indicator presence
  - unfinished-session `preserveProgress()` preserving `resumeHint` and per-mode `modeProgress`
  - root/affix session source generated from `rootAffixFallbackPayloads`
  - gap-analysis required sections

## Verification

- `npm.cmd run test:reports` passed.
- `npm.cmd run test:study-flow` passed.
- `npm.cmd run test:ui-contract` passed.
- `npm.cmd run test:source-integrity` passed.
- `npm.cmd run typecheck` passed.
- `npm.cmd run build:weapp` passed.

## Warnings

- `build:weapp` still reports large Croc BTI PNG asset warnings. This is not introduced by 14-07, but it remains release debt.
- Root/affix is improved from hand-written active questions to Flutter-shaped fallback payload generation, but full production parity still depends on using backend/Rust source payloads in HTTP mode.

## Files Changed

- `docs/wechat/miniprogram_flutter_gap_analysis.md`
- `docs/wechat/miniprogram_flutter_change_feedback_template.md`
- `apps/wechat_miniprogram/src/pages/today/index.tsx`
- `apps/wechat_miniprogram/src/pages/today/index.scss`
- `apps/wechat_miniprogram/src/sdk/reportChart.ts`
- `apps/wechat_miniprogram/src/pages/reports/index.tsx`
- `apps/wechat_miniprogram/src/pages/reports/index.scss`
- `apps/wechat_miniprogram/src/pages/study/index.tsx`
- `apps/wechat_miniprogram/src/pages/wrong-words/index.tsx`
- `apps/wechat_miniprogram/src/sdk/client.ts`
- `apps/wechat_miniprogram/src/sdk/httpClient.ts`
- `apps/wechat_miniprogram/src/sdk/types.ts`
- `apps/wechat_miniprogram/src/sdk/mockData.ts`
- `apps/wechat_miniprogram/scripts/reports-golden.ts`
- `apps/wechat_miniprogram/scripts/study-flow-golden.ts`
- `apps/wechat_miniprogram/scripts/ui-contract.ts`
- `apps/wechat_miniprogram/scripts/source-integrity-check.ts`
