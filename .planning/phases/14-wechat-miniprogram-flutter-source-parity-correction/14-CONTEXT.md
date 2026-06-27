# Phase 14: WeChat Mini Program Flutter Source Parity Correction - Context

**Gathered:** 2026-05-21
**Status:** Ready for planning
**Source:** User bug report plus direct Flutter and mini-program source inspection

<domain>
## Phase Boundary

Phase 13 added broad behavior-parity scaffolding, but live WeChat DevTools testing shows the implementation still behaves like a shallow mock in the most important learning surfaces:

- Croc BTI result layout collapses code, title, image, and text instead of reproducing Flutter's result card structure.
- Study repeats the same `herald` question, increments progress on selection/submission incorrectly, and does not move through a real varied session.
- Study choice feedback is implemented as a separate result card, while Flutter marks the actual choice rows.
- Side buttons use text glyphs instead of Flutter-equivalent icon buttons and several actions are not faithful to Flutter semantics.
- Today progress mutates from the mini mock SDK per answered question instead of following session summary / daily progress truth.
- Reports charts are still too large and not fully source-equivalent in point selection and compact mode chart behavior.

Phase 14 is a correction phase. It must re-read and copy Flutter source semantics before modifying the Taro implementation. The acceptance reference is the Flutter source and SDK contract, not screenshots alone.
</domain>

<decisions>
## Implementation Decisions

### Source Hierarchy
- Flutter source under `apps/flutter_mobile/lib/features/` and `apps/flutter_mobile/lib/sdk/` is the page and contract source of truth.
- Rust/backend or imported payload sources are the business logic source of truth where the mini program can call them.
- Taro code may adapt layout and events for WeChat, but it must not invent study progression, answer judgment, plan counts, or report aggregates when a Flutter/backend path already exists.

### Study Must Be Repaired Before Cosmetic Polish
- `apps/wechat_miniprogram/src/sdk/client.ts` currently creates a session by cycling `questionBank[mode]`, so `newWord` repeats one `herald` item for all 20 planned questions.
- `applyStudySideEffects()` currently increments `today.dailyProgress.completedTasks` on every answer, which explains false Today completion/progress.
- `apps/wechat_miniprogram/src/sdk/studyInteraction.ts` submits when the already-selected item is tapped again; Flutter disables second normal tap and submits only on double tap.
- Choice feedback must be row-level: selected row light green before submit, correct row `#E8F5E9/#2E7D32` with check icon after submit, wrong user row `#FFEBEE/#C62828` with cancel icon, other rows `#F2F3F5/#999999`.
- The mini program must port Flutter's `_choiceDisplay`, `_choiceState`, `_resolvedCorrectChoiceTextToken`, `_choiceUserAnswerTokens`, and duplicate/empty choice guards into tested TypeScript helpers.

### Study Session And Backend Semantics
- `StudyScreen._startWithBestAvailableSeed()` gets the active plan first, then calls `study.startSession` with `mode`, empty `entrySourceIds`, and `questionTypeWeights` for the active mode unless resuming.
- `StudyClient.startSession()` accepts `entryPayloads`, `distractorPayloads`, and `questionTypeWeights`; the mini client must expose the same request surface.
- `submitAnswer()` must be the only normal answer-judgment path. The page must not locally decide correctness except for rendering helper comparisons using returned `StudyResult`.
- Completion must come from `SubmitAnswerResponse.isComplete` plus `summary`, not from local question-count shortcuts.

### Croc BTI
- Croc BTI answers start empty unless saved answers/profile restore them. There must be no default selected answer.
- Result can be shown only when `hasCompleteCrocBtiAnswers()` is true for every canonical Flutter question id.
- Flutter result card is vertical: 180px image, code line, title line, centered summary. The mini result must not render code and title inline.
- Apply result must follow Flutter order: save answers, save daily minutes, build plan input, normalize question-type weights, save Croc profile, save plan, apply saved plan to today, flush sync.
- Question-type weight rebalance must normalize the changed mode to 100 and exclude `newWord` and `rootAffix`.

### Reports
- `ReportsScreen._load()` defaults selected day to the last `dailySeries` item and selected mode to the first `modeBreakdown` item.
- `_DailyLineChart` dimensions are `chartHeight = 140` and compact `104`; point hitboxes are 24x24 with 10/12px visual nodes.
- Daily chart point tap updates selected-day detail. Mode card tap toggles expanded state and, when selected, renders advice and a compact chart from `modeSeries[mode]`.

### Plan And Today State
- Plan save preserves `questionTypeWeightsByMode`, `growthRuleMode`, `sharedGrowthRule`, `growthRulesByMode`, selected wordbook, and count clamps.
- `savePlan` and `applySavedPlanToToday` are distinct actions. Mini must not collapse them into one uncontrolled local spread.
- Today daily progress should be read from `sdk.today.getHomeState()` or backend-backed state after a session change; it must not be manually incremented per answer inside the page mock.

### AI Omission
- AI-related UI remains out of scope for mini-program v1. Do not add AI tab, AI page, AI card, or AI generation control during this correction.
</decisions>

<canonical_refs>
## Canonical References

### Flutter Study
- `apps/flutter_mobile/lib/features/study_screen.dart` - `_startWithBestAvailableSeed`, `_submit`, `_feedItems`, `_buildFeedScaffold`, `_buildChoiceOptions`, `_choiceDisplay`, `_choiceState`, `_resolvedCorrectChoiceTextToken`, side action buttons.
- `apps/flutter_mobile/lib/sdk/study_client.dart` - `StartSessionEntryPayload`, `StudyQuestion`, `StudyResult`, `StartSessionResponse`, `SubmitAnswerResponse`, `startSession`, `submitAnswer`, `markEntryMastered`, `acceptDisputedMeaning`.

### Flutter Croc BTI
- `apps/flutter_mobile/lib/features/croc_bti_screen.dart` - initial restore, completion gating, result layout, editable plan state, apply sequence, rebalance helper.
- `apps/flutter_mobile/lib/features/croc_bti_model.dart` - canonical questions, scoring, profile titles/assets/flavor, plan input calculation, question-type weights, storage helpers.

### Flutter Reports And Plan
- `apps/flutter_mobile/lib/features/reports_screen.dart` - selected day/mode state, chart constants, tappable points, compact mode chart.
- `apps/flutter_mobile/lib/features/plan_screen.dart` - dirty state, save/apply, wordbook toggle, growth rules, count clamps.
- `apps/flutter_mobile/lib/sdk/plan_client.dart` - `PlanSummary` fields and preservation rules.
- `apps/flutter_mobile/lib/sdk/reports_client.dart` - `ReportsOverview` shape including `modeSeries`.

### Mini Program Files To Correct
- `apps/wechat_miniprogram/src/sdk/client.ts` - mock SDK session generation, answer submission, Today/report side effects, plan save/apply behavior.
- `apps/wechat_miniprogram/src/sdk/mockData.ts` - current one-question banks and seed state.
- `apps/wechat_miniprogram/src/sdk/types.ts` - DTO parity fields for study, plan, reports, Croc BTI.
- `apps/wechat_miniprogram/src/sdk/httpClient.ts` - real backend client parity surface.
- `apps/wechat_miniprogram/src/sdk/studyInteraction.ts` - tap/double-tap behavior.
- `apps/wechat_miniprogram/src/sdk/studyFeed.ts` - feed item construction.
- `apps/wechat_miniprogram/src/pages/study/index.tsx` and `.scss` - Study feed UI, side actions, choice rendering, completion.
- `apps/wechat_miniprogram/src/pages/croc-bti/index.tsx` and `.scss` - Croc BTI result, completion gating, apply flow.
- `apps/wechat_miniprogram/src/pages/reports/index.tsx` and `.scss` - daily/mode chart interaction and sizing.
- `apps/wechat_miniprogram/src/pages/plan/index.tsx` and `.scss` - plan persistence and question-type weights preservation.
- `apps/wechat_miniprogram/src/pages/today/index.tsx` and `.scss` - refresh from true progress after session/apply.
- `apps/wechat_miniprogram/src/assets/croc_bti/*` - copied Flutter Croc BTI result images; use them instead of placeholders.
</canonical_refs>

<specifics>
## Concrete Bugs To Prove Fixed

- Croc BTI: a fresh test page must have zero selected answers; answering all questions enables result; result card title/code/image no longer overlap.
- Study: starting a `newWord` session with target 20 must not produce twenty identical `herald` pages.
- Study: single tap selects only; double tap submits; repeated slow tap on selected option does not submit.
- Study: a choice question renders four choices when the source question has four valid choices.
- Study: wrong choice after submit shows the selected wrong row red and the correct row green; no separate choice-result card is shown.
- Study: top progress follows visible question/session progress, not local tap count.
- Today: answering one question does not mark all daily tasks complete.
- Reports: tapping a daily point changes the selected detail; tapping a mode card toggles its compact chart.
</specifics>

<deferred>
## Deferred Ideas

- AI mini-program UI and AI generation.
- Full production leaderboard backend if endpoint coverage is not yet ready.
- Payment or advanced WeChat account capabilities.
</deferred>

---

*Phase: 14-wechat-miniprogram-flutter-source-parity-correction*
*Context gathered: 2026-05-21 via source inspection*
