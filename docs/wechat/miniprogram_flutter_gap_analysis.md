# Mini Program vs Flutter Function-Level Gap Analysis

Updated: 2026-05-22

This document is the execution reference for mini-program parity work. It compares Flutter/Rust source functions against the Taro mini-program implementation at function level. Screenshots are useful for visual QA, but implementation decisions must be traced to these source chains.

## Required Feedback Format

Every implementation report after this document must include rows like this:

| Area | Flutter/Rust source | Mini-program source | Difference | Change made |
| --- | --- | --- | --- | --- |
| Study choice submit | `apps/flutter_mobile/lib/features/study_screen.dart:2173` `_buildChoiceOptions` | `apps/wechat_miniprogram/src/sdk/studyInteraction.ts` | Gesture semantics differ | Port single-tap select and double-tap submit |

Line numbers must be refreshed after edits.

## Study Session And Answering

| Area | Flutter/Rust source | Mini-program source | Difference | Required fix |
| --- | --- | --- | --- | --- |
| Start session | `apps/flutter_mobile/lib/features/study_screen.dart` `_startWithBestAvailableSeed` | `apps/wechat_miniprogram/src/pages/study/index.tsx` session bootstrap | Flutter loads active plan, passes mode and question-type weights into SDK. Mini does similar but can still fall back to local fixture. | Keep page thin. Prefer HTTP SDK/source payload. Mock fixture must be explicit demo-only. |
| Backend session start | `crates/app-core/src/facade/study_facade.rs` `start_study_session` | `apps/wechat_miniprogram/src/sdk/client.ts` `study.startSession` | Rust owns candidate selection and generated question sequence. Mini mock slices `questionBank`. | Real mode must call backend. Mock mode must be labelled and tested as fixture only. |
| Question builder | `crates/study-core/src/question_builder.rs` `build_questions` | `apps/wechat_miniprogram/src/sdk/mockData.ts` `questionBank` | Rust builds by mode/type; mini fixture hard-codes words and choices. | Do not treat `mockData` as product truth. Add source-integrity checks for fixture-only use. |
| Submit boundary | `apps/flutter_mobile/lib/features/study_screen.dart:141` `_submit` and `crates/app-core/src/facade/study_facade.rs` `submit_study_answer` | `apps/wechat_miniprogram/src/sdk/client.ts` `submitAnswer` | Flutter/Rust submit is the answer judgment boundary. Mini mock evaluates locally. | In HTTP mode never locally judge correctness; render only returned `StudyResult`. |
| Choice display | `apps/flutter_mobile/lib/features/study_screen.dart` `_choiceDisplay` | `apps/wechat_miniprogram/src/sdk/studyChoice.ts` `choiceDisplay`/`sanitizeChoices` | Flutter tolerates multiple payload shapes but does not silently collapse valid source contracts. Mini dedupe can reduce four choices to three. | Frontend may normalize display but must surface contract errors when a choice question cannot render four choices. |
| Choice state | `apps/flutter_mobile/lib/features/study_screen.dart` `_choiceState`, `_resolvedCorrectChoiceTextToken`, `_choiceUserAnswerTokens` | `apps/wechat_miniprogram/src/sdk/studyChoice.ts` `choiceState` | Mini needs to match returned label/text/token mapping exactly. | Keep row-level correct/wrong state; no separate choice-summary card. |
| Choice gesture | `apps/flutter_mobile/lib/features/study_screen.dart:2173` `_buildChoiceOptions` | `apps/wechat_miniprogram/src/sdk/studyInteraction.ts` | Flutter single tap selects; double tap submits. Mini must not increment progress on first tap. | Keep `questionId + label + time-window + submitting=false` guard. |
| Feed composition | `apps/flutter_mobile/lib/features/study_screen.dart` `_feedItems`, `_buildFeedScaffold` | `apps/wechat_miniprogram/src/sdk/studyFeed.ts` | Flutter feed is derived from session state. Mini merges latest result and current question locally. | Ensure submit response replaces current question and completion page; no stale current question after complete. |
| Completion summary | `apps/flutter_mobile/lib/features/study_screen.dart` `_CompletionPage`, `_AccuracyPanel`, `_summarySubtitle` | `apps/wechat_miniprogram/src/pages/study/index.tsx` completion rendering | Flutter summary text and next action are mode-aware. Mini summary must not be local-count-only. | Drive completion from `SubmitAnswerResponse.isComplete`, `summary`, and `nextAction` for every mode. |

## Root/Affix Source Chain

| Area | Flutter/Rust source | Mini-program source | Difference | Required fix |
| --- | --- | --- | --- | --- |
| Mode definition | `crates/study-core/src/session_definition.rs` `SessionMode::RootAffix` | `apps/wechat_miniprogram/src/sdk/types.ts` `StudyMode` | Both expose `rootAffix`. | Keep mode in plan/today/study reports. |
| Question generation | `crates/study-core/src/question_builder.rs:131` `build_root_affix_questions`; `:442` `build_root_affix_question` | `apps/wechat_miniprogram/src/sdk/mockData.ts` `questionBank.rootAffix` | Rust generates `RootToGlossInput` and `GlossToRootInput`. Mini currently uses hand-written `re-`, `pre-`, `sub-`, `trans-`. | Trace exact root/affix library source. If unavailable locally, mark mock as demo-only and do not claim Flutter parity. |
| Root/affix display | `apps/flutter_mobile/lib/features/study_screen.dart:1981`, `:2032`, `:2744` `_RootAffixRelatedWords` | `apps/wechat_miniprogram/src/pages/study/index.tsx` root/input rendering | Flutter has dedicated root/affix display helpers. Mini mostly falls through generic input rendering. | Port root/affix prompt, related words, and input feedback behavior after source is traced. |
| Summary | `apps/flutter_mobile/lib/features/study_screen.dart:3236` `_summarySubtitle` | `apps/wechat_miniprogram/src/pages/study/index.tsx` completion copy | Flutter labels root/affix summary as root-focused. | Add root/affix-specific summary copy and tests. |

Current finding: active mini-program fixture root/affix data is not proven to be Flutter/Rust sourced. It must be replaced or explicitly quarantined as a demo fixture.

## Today State, Progress, And Refresh

| Area | Flutter/Rust source | Mini-program source | Difference | Required fix |
| --- | --- | --- | --- | --- |
| Load boundary | `apps/flutter_mobile/lib/features/today_shell_screen.dart` `_load` | `apps/wechat_miniprogram/src/pages/today/index.tsx` `refreshToday` | Flutter reloads home-state as the display truth. Mini only mounted once and pull-down refreshed. | Refresh on page show, pull-down, and study return. |
| Partial bundle | `apps/flutter_mobile/lib/features/today_shell_screen.dart` `_partialBundleFromToday` | `apps/wechat_miniprogram/src/sdk/client.ts` `syncActiveSessionResumeHint` | Flutter carries partial progress through Today bundle/snapshot. Mini mock writes `modeProgress` on submit but page may not reload. | Use refreshed Today state to drive total and row progress. |
| Per-mode rows | `apps/flutter_mobile/lib/features/today_shell_screen.dart` task row helpers | `apps/wechat_miniprogram/src/pages/today/index.tsx` `taskProgress` | Mini has per-mode field support, but stale page state makes rows lag. | Add `useDidShow` refresh and test partial exit. |
| Completion | `apps/flutter_mobile/lib/features/today_shell_screen.dart` `todayCompletionForTest` | `apps/wechat_miniprogram/src/pages/today/index.tsx` `calculateTodayCompletion` | Both compute from daily progress. Mini mock must not advance daily completion per first answer. | Only complete mode on session summary/complete path. |
| Pull refresh visual | `CrocodileRefreshIndicator` and Flutter loading progress indicators | `apps/wechat_miniprogram/src/pages/today/index.tsx` `usePullDownRefresh` | Mini had trigger but no visible in-page refresh state. | Add `refreshing` state and visible progress strip/spinner. |

## Reports Chart

| Area | Flutter/Rust source | Mini-program source | Difference | Required fix |
| --- | --- | --- | --- | --- |
| Load defaults | `apps/flutter_mobile/lib/features/reports_screen.dart` `_load` | `apps/wechat_miniprogram/src/pages/reports/index.tsx` `useEffect` | Both choose last daily item and first mode. | Keep source-backed overview and avoid stale mock in HTTP mode. |
| Point math | `apps/flutter_mobile/lib/features/reports_screen.dart` `_DailyLineChart` | `apps/wechat_miniprogram/src/sdk/reportChart.ts` `buildReportChartPoints` | Constants now mirror Flutter, but CSS dot box was larger than `REPORT_CHART_TOUCH_SIZE`, causing visual offset. | Touch box and style box must share the exported touch size; segment center must start at point center. |
| Painter/segments | `apps/flutter_mobile/lib/features/reports_screen.dart` `_LineChartPainter` | `apps/wechat_miniprogram/src/pages/reports/index.tsx` `Chart` | Flutter draws a path through point centers. Mini CSS segment top-left can offset line by half its thickness. | Position segment at `previous.y - lineThickness/2`. |
| Hit testing | Flutter `_ChartPoint` hitboxes | Mini `.line-chart__dot` | Flutter hitbox and visual node are centered on same point. | Assert visual center and hitbox center equal chart point. |
| Mode compact chart | Flutter `_ModeBreakdownCard` compact `_DailyLineChart` | Mini `Chart compact` | Same structure, but needs same point alignment fix. | Reuse corrected chart math for compact chart. |

## Plan And Croc BTI

| Area | Flutter/Rust source | Mini-program source | Difference | Required fix |
| --- | --- | --- | --- | --- |
| Plan hydration | `apps/flutter_mobile/lib/features/plan_screen.dart` root/affix controller hydration | `apps/wechat_miniprogram/src/pages/plan/index.tsx` | Mini supports `rootAffixPerDay` but text/encoding and save/apply need verification. | Preserve root/affix through save/apply and Today rows. |
| Save vs apply | `apps/flutter_mobile/lib/features/plan_screen.dart` `_persistPlanAndWordbook`, `_applyToToday` | `apps/wechat_miniprogram/src/sdk/planFlow.ts`, page handlers | Flutter separates saving plan template from applying to today. | Keep distinct actions and tests. |
| Croc questions/profile | `apps/flutter_mobile/lib/features/croc_bti_model.dart` canonical questions/profiles/assets | `apps/wechat_miniprogram/src/sdk/crocBti.ts` | Mini must be checked for encoding, scale, and asset parity. | Regenerate constants from Flutter or verify one-to-one. |
| Croc apply | `apps/flutter_mobile/lib/features/croc_bti_screen.dart` `_applyResult` | `apps/wechat_miniprogram/src/pages/croc-bti/index.tsx` | Apply sequence must save profile, save plan, apply today, flush sync. | Keep order in tests and preserve root/affix. |

## Active Bugs From Latest QA

| Bug | Source-level cause | Required correction |
| --- | --- | --- |
| Root/affix asks suspicious fake questions | `mockData.ts` hand-authored root list, not proven source data | Trace source and quarantine mock |
| Chart points still offset | CSS touch box width `36px` while exported hitbox constant is `24px`; segment top uses center as top edge | Align CSS box and segment center |
| Today total updates but rows lag | Today page did not refresh on show after returning from Study | Refresh on `useDidShow` and use `modeProgress` rows |
| Pull-down has no visible animation | Only `usePullDownRefresh` trigger existed | Add page-level `refreshing` indicator |
| Completion summary not compared for all modes | Existing tests focus flow more than per-mode summary contract | Add per-mode summary assertions |

## Execution Order

1. Fix visible/stale state issues that are already source-confirmed: Today `useDidShow`, refresh indicator, report chart CSS alignment.
2. Add tests for those fixes.
3. Quarantine root/affix fixture and document that it is not source parity until exact library path/import is found.
4. Continue deeper root/affix source tracing before claiming parity.
