# Phase 13: WeChat Mini Program Flutter Behavior and Interaction Parity - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning
**Source:** User request plus Flutter source decomposition

<domain>
## Phase Boundary

Phase 12 restored the first visual pass of the WeChat Mini Program shell and major pages, but several surfaces still behave like static mini-program placeholders rather than Flutter-derived product flows. Phase 13 restores the missing behavior and interaction model:

- button logic and card/page proportions across the mini program pages
- Reports line chart behavior, mode-card expansion, and backend data sourcing
- Croc BTI question set, result calculation, plan counts, question-type weights, and mode-weight tuning
- Plan editor save/apply/wordbook/growth-rule behavior
- Study page TikTok-style vertical answering flow

AI-related UI and AI generation flows stay out of scope for the first mini-program release.
</domain>

<decisions>
## Implementation Decisions

### Flutter Is The Product Source
- Flutter files under `apps/flutter_mobile/lib/features/` are the reference for behavior, not only screenshots.
- The mini program must reuse backend/business contracts where possible. Frontend code should adapt Taro/WeChat rendering and event handling around those contracts.
- Where Flutter currently calls SDK methods, the mini program must expose equivalent methods before building UI on top of mocks.

### Reports
- `ReportsScreen` uses `sdk.reports.getReportsOverview()` as the data source.
- `dailySeries` drives the main daily accuracy chart and the selected-day detail card.
- `modeBreakdown` drives mode cards.
- `modeSeries[mode]` drives the compact line chart inside each selected mode card.
- The mini program must add `modeSeries` to `ReportsOverview`; fake chart bars without mode-series data are not enough.

### Croc BTI
- Flutter loads saved local answers, saved daily minutes, and `sdk.crocBti.getProfile()` before showing the question/result state.
- Croc BTI answers are complete only when every canonical `crocBtiQuestions` id has an answer.
- Result application must save the profile, save the active plan, apply the plan to today, and flush pending cloud sync when available.
- Question-type weights apply to review, mixed test, and wrong-word reinforcement modes; new-word and root-affix modes are excluded from question-type customization.

### Plan
- Plan editing must preserve Flutter's dirty-state semantics: name, all count fields, growth-rule toggles, growth-rule mode, per-mode growth rules, and wordbook selection all affect whether there are unsaved changes.
- `savePlan` and `applySavedPlanToToday` are distinct backend actions.
- Wordbook selection is persisted with `toggleWordbook` before saving the plan.
- `questionTypeWeightsByMode` must be preserved when saving the plan, so Croc BTI tuning is not dropped by later plan edits.

### Study
- Study starts by reading the active plan and passing `questionTypeWeightsByMode[mode]` into `study.startSession` unless the session is a resume.
- The visible study UI is a vertical feed: answered questions remain as previous pages, the current question is appended, and the completion page is appended at the end.
- Submit, reveal-answer, dispute, comments, hint, and mastered actions must be wired to SDK behavior where supported.
- The mini program should use WeChat-compatible vertical paging, but the data model must follow Flutter's `_feedItems` behavior.

### Mini Program Constraints
- Keep custom navigation and the four bottom tabs from Phase 12: Today, Plan, Wrong, Reports.
- Drawer-only surfaces remain drawer-only: Croc BTI, leaderboard, account/profile/settings.
- Do not reintroduce an AI tab, AI page, AI summary card, or AI generation button in this phase.
- Keep the splitChunks runtime guard from Phase 12 unless a new verified bundling approach is introduced.
</decisions>

<canonical_refs>
## Canonical References

### Flutter Reference
- `apps/flutter_mobile/lib/features/reports_screen.dart` - Reports load flow, daily chart, mode cards, selected detail behavior.
- `apps/flutter_mobile/lib/features/croc_bti_screen.dart` - Croc BTI page state, saved answers, result editing, apply side effects.
- `apps/flutter_mobile/lib/features/croc_bti_model.dart` - Canonical Croc BTI questions, scoring, plan counts, and question-type weights.
- `apps/flutter_mobile/lib/features/plan_screen.dart` - Plan dirty state, save/apply, wordbook, growth-rule behavior.
- `apps/flutter_mobile/lib/features/study_screen.dart` - Study session start, vertical feed model, submit/result/completion flow.

### Mini Program Implementation
- `apps/wechat_miniprogram/src/components/Screen.tsx` - custom shell, bottom nav, drawer entry points.
- `apps/wechat_miniprogram/src/pages/today/index.tsx` - Today page cards and entry buttons.
- `apps/wechat_miniprogram/src/pages/plan/index.tsx` - current plan UI to replace with real save/apply behavior.
- `apps/wechat_miniprogram/src/pages/reports/index.tsx` - current Reports UI and chart placeholder.
- `apps/wechat_miniprogram/src/pages/croc-bti/index.tsx` - current Croc BTI simplified flow.
- `apps/wechat_miniprogram/src/pages/study/index.tsx` - current non-feed answering flow.
- `apps/wechat_miniprogram/src/sdk/types.ts` - mini program DTOs that need parity fields.
- `apps/wechat_miniprogram/src/sdk/client.ts` - mock SDK currently missing several plan, report, and Croc BTI methods.
- `apps/wechat_miniprogram/src/sdk/httpClient.ts` - real backend client boundary to extend without changing Flutter-side contracts.
- `apps/wechat_miniprogram/config/index.ts` - WeChat runtime bundling guard.
</canonical_refs>

<specifics>
## Specific Ideas

- Introduce a small mini-program chart model helper that mirrors Flutter's chart math: `stepX = 56`, `leftGutter = 34`, `rightGutter = 18`, `topGutter = 12`, `bottomGutter = 22`, `chartHeight = 140` or compact `104`.
- Use WeChat `Swiper` with `vertical` for the Study feed if it can preserve input and answered-card state; otherwise use a page-snapping scroll view with explicit current-index state.
- Create pure TS helpers for Croc BTI scoring, plan count calculation, question-type normalization, report chart points, and study feed item construction so tests can verify behavior without DevTools.
- Keep source data names close to Flutter: `dailySeries`, `modeBreakdown`, `modeSeries`, `questionTypeWeightsByMode`, `growthRuleMode`, `growthRulesByMode`, `sharedGrowthRule`.
</specifics>

<deferred>
## Deferred Ideas

- AI summary/generation UI and AI tab.
- WeChat payment or advanced account capabilities.
- Full leaderboard backend implementation if ranking endpoints are not yet ready; drawer entry can stay as a reserved linked surface.
</deferred>

---

*Phase: 13-wechat-miniprogram-flutter-behavior-and-interaction-parity*
*Context gathered: 2026-05-21 via source decomposition*
