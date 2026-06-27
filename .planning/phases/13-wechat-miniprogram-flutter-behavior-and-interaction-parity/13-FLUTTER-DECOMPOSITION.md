# Phase 13 Flutter Decomposition

This document captures the Flutter behavior that the Taro mini program must reproduce. The goal is to prevent implementation from guessing from screenshots alone.

## Reports

Reference: `apps/flutter_mobile/lib/features/reports_screen.dart`

### Data Load
- `ReportsScreen._load()` calls `widget.sdk.reports.getReportsOverview()`.
- It normalizes `reports.dailySeries` and `reports.modeBreakdown` into `List<Map<String, dynamic>>`.
- `_selectedDay` defaults to the last item in `dailySeries`.
- `_selectedMode` defaults to the first mode in `modeBreakdown`.

### Daily Chart
- `_DailyLineChart` accepts `data`, `selectedDate`, `lineColor`, `onSelect`, and `compact`.
- Empty data renders an empty-state text.
- Chart constants:
  - `stepX = 56`
  - `edgeInset = 28`
  - `leftGutter = 34`
  - `rightGutter = 18`
  - `topGutter = 12`
  - `bottomGutter = 22`
  - `chartHeight = 140`, compact `104`
- Each point uses `accuracyPercent`, falling back to `accuracy`, clamped `0..100`.
- X is `leftGutter + edgeInset + index * stepX`.
- Y is `topGutter + plotHeight - (accuracy / 100) * plotHeight`.
- Points are tappable. Tap calls `onSelect(item)`.
- Point colors:
  - `< 50`: red `#FF3B30`
  - `< 85`: orange `#FF9500`
  - otherwise green `#34C759`
- Selected point has a white border.
- Non-compact chart shows a selected-day summary above the chart.

### Daily Detail
- `_DailyDetailCard` reads:
  - `date`
  - `totalQuestions` or `questionsAnswered`
  - `accuracyPercent` or `accuracy`
  - `studyTimeMs` or `totalTimeMs`
- Mini must not invent daily detail fields locally if the backend already provides them.

### Mode Breakdown
- `_ModeBreakdownCard` is tappable and expands/collapses based on `_selectedMode`.
- Mode colors:
  - `newWord`: `#34C759`
  - `review`: `#007AFF`
  - `mixedTest`: `#FF9500`
  - `wrongWordReinforcement`: `#FF3B30`
  - `rootAffix`: `#8E44AD`
- Each card reads:
  - `accuracyPercent` or `accuracy`
  - `totalQuestions` or `questionsAnswered`
  - `correctCount`
  - `missed = total - correct`
- The progress bar value is `accuracy / 100`.
- When selected, the card shows advice text and a compact `_DailyLineChart` with `reports.modeSeries[mode]`.
- Mini gap: current `ReportsOverview` lacks `modeSeries`, so chart fidelity cannot be achieved until the type, mock client, and HTTP client all carry it.

## Croc BTI

References:
- `apps/flutter_mobile/lib/features/croc_bti_screen.dart`
- `apps/flutter_mobile/lib/features/croc_bti_model.dart`

### Initial State
- `_loadInitialState()` loads saved answers scoped by `userId`.
- It loads saved daily minutes scoped by `userId`.
- It calls `widget.sdk.crocBti.getProfile()` and can restore answers/minutes from the cloud profile.
- `_dailyLearningMinutes` is clamped to `10..240`.
- `_showResult` becomes true only when `hasCompleteCrocBtiAnswers(_answers)` is true.
- It then loads the active plan with `widget.sdk.plan.getActivePlan()`.

### Completion And Result
- `hasCompleteCrocBtiAnswers()` requires every canonical `crocBtiQuestions` id to exist in answers.
- `evaluateCrocBti()` scores four axes, creates a four-letter code, reads a profile, calculates mode weights, creates plan input, and calculates question-type weights.
- Result contains:
  - `code`
  - `title`
  - `summary`
  - `advice`
  - `assetPath`
  - `axisScores`
  - `weights`
  - `planInput`
  - `questionTypeWeightsByMode`

### Editable Result State
- `_ensureEditableState(result)` initializes:
  - `_editedPlanInput = crocBtiPlanInputForDailyMinutes(result.weights, _dailyLearningMinutes)`
  - `_editedQuestionTypeWeights` from `result.questionTypeWeightsByMode`
  - `_growthRuleEnabled` from the active plan
- `_updateDailyLearningMinutes()` recalculates plan input from weights.
- `_updatePlanInput()` clamps each plan input field to `0..240`.
- `_updateQuestionTypeWeight()` rebalances one mode so weights still normalize to `100`.

### Plan Count Algorithm
- `crocBtiPlanInputForDailyMinutes(weights, dailyMinutes)`:
  - clamps minutes to `10..240`
  - uses `totalQuestions = minutes * 4`
  - combines `activeRecall` into mixed test and wrong-word review
  - maps counts to:
    - `newWordsPerDay`
    - `reviewWordsPerDay`
    - `mixedTestPerDay`
    - `wrongWordTestPerDay`
    - `rootAffixPerDay`
  - rounds `newWordsPerDay` down to a multiple of four
  - distributes any remaining difference into the largest non-new-word bucket

### Question-Type Weights
- `normalizeCrocBtiQuestionTypeWeightsByMode()` excludes `rootAffix` and `newWord`.
- `calculateCrocBtiQuestionTypeWeights()` returns modes:
  - `review`
  - `mixedTest`
  - `wrongWordReinforcement`
- Canonical question types include:
  - `enToCnInput`
  - `exampleToCnChoiceNoTranslation`
  - `enToCnChoice`
  - `cnToEnChoice`
  - `wordSkeletonInput`
  - `exampleToCnChoice`
- Mini gap: current mini `crocBti.ts` uses simplified English questions and contains `cnToEnInput`, which is not part of the current mini `StudyQuestion.questionType` union. This must be corrected during parity work.

### Apply Side Effects
- `_applyResult()` performs this sequence:
  1. save local answers
  2. save local daily minutes
  3. build `planInput`
  4. build normalized `questionTypeWeightsByMode`
  5. `widget.sdk.crocBti.saveProfile(profile)`
  6. `widget.sdk.plan.savePlan(planId, input: crocBtiPlanInputFor(...))`
  7. `widget.sdk.plan.applySavedPlanToToday()`
  8. `widget.sdk.sync.flushPendingToCloud()`
  9. call `onApplied`, show confirmation, and return to the previous page

Mini must implement this as a real flow, not just a local result screen.

## Plan

Reference: `apps/flutter_mobile/lib/features/plan_screen.dart`

### Load And Hydration
- `_load()` fetches active plan and wordbooks in parallel via cache or SDK.
- `_hydrateControllers(plan)` fills all fields from the active plan.
- Wordbook active state is derived by the first `WordbookSummary.isActive`.

### Dirty State
`_hasUnsavedChanges` returns true if any of these differ from the loaded plan:
- plan name
- new words per day
- review words per day
- mixed test per day
- wrong-word test per day
- root-affix per day
- growth rule enabled
- growth rule mode
- wordbook selection
- shared growth interval
- shared growth increment
- per-mode growth interval
- per-mode growth increment

### Save Payload
`_buildPlanInput(plan)` returns:
- `name`
- `newWordsPerDay`, rounded down to a multiple of four and clamped `0..180`
- `reviewWordsPerDay`, clamped `0..200`
- `mixedTestPerDay`, clamped `0..50`
- `wrongWordTestPerDay`, clamped `0..30`
- `rootAffixPerDay`, clamped `0..50`
- `growthRuleEnabled`
- `growthRuleMode`
- `growthIntervalDays`
- `growthIncrement`
- `sharedGrowthRule`
- `growthRulesByMode`
- existing `questionTypeWeightsByMode`

### Persist And Apply
- `_persistPlanAndWordbook(plan)` toggles the selected wordbook if it changed, then calls `sdk.plan.savePlan(planId, input)`.
- `_savePlan()` persists and reloads wordbooks.
- `_applyToToday()` persists, calls `sdk.plan.applySavedPlanToToday()`, reloads wordbooks, and refreshes local state.

Mini gap: current mini UI is mostly visual and the mock SDK does not expose all of these plan methods.

## Study

Reference: `apps/flutter_mobile/lib/features/study_screen.dart`

### Session Start
- `_startWithBestAvailableSeed()` gets the active plan first.
- It calls `widget.sdk.study.startSession()` with:
  - `mode`
  - `entrySourceIds: []`
  - `questionTypeWeights: null` when resuming
  - otherwise `_questionTypeWeightsForMode(plan?.questionTypeWeightsByMode, widget.mode)`

Mini gap: current mini `startSession(mode)` does not pass question-type weights.

### Submit Flow
- `_submit()` chooses response text from explicit choice, selected choice, or input controller.
- Empty response is rejected unless `allowEmpty` is true.
- It calls `sdk.study.submitAnswer(questionId, response, responseTimeMs)`.
- The latest answered question is merged into the session with `mergeLatestAnsweredQuestionForTest()`.
- If the response is complete, it appends a completion state. Otherwise it restarts the timer and focuses the next question.

### Vertical Feed Model
- `_feedItems` builds the visible feed:
  1. map `session.answeredQuestions` into answered feed items
  2. merge the last submitted question/result if needed
  3. append the current unanswered question if it is not already answered
  4. append a completion item if `_completion` exists
- `_jumpToCurrentFeedPage()` jumps to the last feed page after frame.
- `_buildFeedScaffold()` renders `PageView.builder(scrollDirection: Axis.vertical)`.
- `onPageChanged` updates `_visiblePageIndex`, restarts timer for the current unanswered question, and settles input for old pages.

### Feed Page Layout
- `_StudyFeedPage` is a full-screen page with:
  - pale background
  - safe area
  - top padding equal to status bar plus 56
  - question header
  - question composer
  - completion-swipe hint when the next page is completion
  - right-side vertical round action buttons: hint, comments, show answer, dispute, mastered

Mini gap: current mini Study page is a single static card and needs a `Swiper` or page-snapping equivalent that preserves the same item model.
