# Phase 14 Research: Flutter Source Decomposition For Mini-Program Correction

## Research Question

What exactly must be copied from Flutter source so the Taro mini program stops approximating the UI and begins reproducing the real learning flow?

## Study Decomposition

### Flutter Module Responsibilities

`apps/flutter_mobile/lib/features/study_screen.dart`

- `StudyScreen` owns page-local state: `_session`, `_latestResponse`, `_lastSubmittedQuestion`, `_lastSubmittedResult`, `_lastSubmittedResponse`, `_completion`, `_selectedChoice`, `_visiblePageIndex`, and response timing.
- `_startWithBestAvailableSeed()` loads the active plan, then calls `widget.sdk.study.startSession()` with:
  - `mode`
  - `entrySourceIds: []`
  - `questionTypeWeights: null` when resuming
  - otherwise `_questionTypeWeightsForMode(plan?.questionTypeWeightsByMode, widget.mode)`
- `_submit()` is the answer boundary. It calls `sdk.study.submitAnswer(questionId, response, responseTimeMs)` and then updates the session from the returned `SubmitAnswerResponse`.
- `_feedItems` constructs the vertical feed by merging answered questions, the last submitted result, the current unanswered question, and a completion page.
- `_buildFeedScaffold()` renders a vertical `PageView.builder`, top overlay, right close button, and one `_StudyFeedPage` per feed item.
- `_StudyFeedPage` lays out the question header, answer composer, completion hint, and right-side round action buttons.
- `_buildChoiceOptions()` renders choice rows and implements the correct interaction:
  - `onTap`: select only, and disabled when already selected or answered.
  - `onDoubleTap`: select and submit.
  - No normal second tap submission.
- `_choiceDisplay()` accepts multiple backend shapes (`label`, `value`, `text`, `meaning`, `word`) and falls back to A/B/C/D.
- `_choiceState()` determines correct/wrong row states using label, value, text, returned `StudyResult.correctAnswer`, `StudyQuestion.correctChoiceLabel`, and response tokens.
- `_resolvedCorrectChoiceTextToken()` maps returned answer data back to the displayed choice text.

### Flutter Answer Feedback

Choice feedback is not a separate red/green card for choice questions. It is rendered on the option rows:

- Before submit:
  - selected row background `#E8F5E9`
  - selected text `#1B5E20`
  - other rows background `#F2F3F5`
- After submit:
  - correct row background `#E8F5E9`, text/label `#2E7D32`, check-circle icon.
  - user's wrong row background `#FFEBEE`, text/label `#C62828`, cancel icon.
  - other rows background `#F2F3F5`, text/label `#999999`.

Input-answer questions do have `_buildInputFeedback()`, which displays the typed response and correct answer text. That feedback pattern must not be applied to choice questions.

### Mini Program Deviations

`apps/wechat_miniprogram/src/sdk/client.ts`

- `buildQuestionsForSession()` cycles `questionBank[mode]` with `bank[index % bank.length]`.
- `mockData.ts` currently has only one `newWord` item, so a 20-question new-word session is twenty variants of `herald`.
- `applyStudySideEffects()` increments `today.dailyProgress.completedTasks` per answer.
- `submitAnswer()` locally calls `isCorrect(question,response)` and sets `correctAnswer` to `question.acceptedMeanings[0]`, which is not enough for choice-label correctness.

`apps/wechat_miniprogram/src/sdk/studyInteraction.ts`

- `shouldSubmitChoiceTap()` returns true when `selected === choice`, causing slow second taps to submit. Flutter does not do this.

`apps/wechat_miniprogram/src/pages/study/index.tsx`

- Renders a separate `.result` card for answered choice questions.
- Determines row correctness with `value === result.correctAnswer`, missing Flutter's label/text/token resolution.
- Side actions are text glyphs, not Flutter-like icon buttons.

### Required Study Fix Shape

- Add a TS helper mirroring Flutter:
  - `choiceDisplay(choice,index)`
  - `normalizeChoiceToken(value)`
  - `choiceUserAnswerTokens(result,responseOverride)`
  - `resolvedCorrectChoiceTextToken(question,result)`
  - `choiceState(question,result,display,correctTextToken,responseOverride)`
  - `sanitizeChoices(question.choices)` with no empty entries and stable de-duplication.
- Change tap helper to select on first normal tap and submit only on detected double tap.
- Remove choice-result summary card and apply row-level feedback.
- Replace repeated mock sessions with:
  1. real backend/http `startStudySession` when configured, and
  2. a deterministic varied fixture/imported payload fallback with enough unique entries and distractors for local WeChat DevTools.
- Stop mutating Today completion per answer. Session answers may update report/wrong-word fixtures for local smoke, but Today completion must refresh from session summary/home state.

## Croc BTI Decomposition

### Flutter Module Responsibilities

`apps/flutter_mobile/lib/features/croc_bti_screen.dart`

- `_answers` starts as `{}`.
- `_loadInitialState()` loads saved local answers/minutes and `sdk.crocBti.getProfile()`, then sets `_showResult = hasCompleteCrocBtiAnswers(_answers)`.
- `allAnswered` is `_answers.length == crocBtiQuestions.length`.
- AppBar and bottom result buttons are disabled until all questions are answered.
- `_QuestionCard` uses `ChoiceChip` over `{-2,-1,0,1,2}` labels.
- `_ResultView` is a `ListView` with:
  - result card: 180px asset image, code, title, centered summary
  - recommendation weights card
  - plan input card
  - question type weights card
  - apply button and retake button
- `_applyResult()` saves local answers/minutes, saves profile, saves plan, applies to today, flushes sync, then returns.
- `_rebalanceQuestionTypeWeights()` keeps a changed mode normalized to 100.

`apps/flutter_mobile/lib/features/croc_bti_model.dart`

- `crocBtiQuestions` is the canonical 24-question source.
- `_profiles` and `_profileVisuals` are canonical names, copy, images, and flavor text.
- `evaluateCrocBti()` computes four axis scores, code, profile, visuals, weights, plan input, and question-type weights.
- `normalizeCrocBtiQuestionTypeWeightsByMode()` excludes `rootAffix` and `newWord`.

### Mini Program Deviations

- Result layout currently compresses image/code/title/copy into a horizontal-ish card and the title/code run together.
- Fresh Croc BTI must not default-select answer value `3` or any converted value; all answers should be absent until tapped.
- The current button behavior must avoid "view result does nothing"; disabled state must match Flutter, and enabled state must switch to result.

## Reports Decomposition

`apps/flutter_mobile/lib/features/reports_screen.dart`

- `ReportsScreen._load()` gets `sdk.reports.getReportsOverview()`, defaults `_selectedDay` to last `dailySeries`, `_selectedMode` to first `modeBreakdown`.
- `_DailyLineChart` constants:
  - `stepX = 56`
  - `edgeInset = 28`
  - `leftGutter = 34`
  - `rightGutter = 18`
  - `topGutter = 12`
  - `bottomGutter = 22`
  - `chartHeight = 140`, compact `104`
- Tappable point hitbox is 24x24. Visual node is 12px or 10px compact.
- Point colors are red `<50`, orange `<85`, green otherwise.
- Selected point has a white border.
- `_ModeBreakdownCard` toggles selected state and renders compact `_DailyLineChart` from `modeSeries[mode]`.

Mini must keep chart smaller and selectable; rendering oversized static charts fails this source contract.

## Plan And Today Decomposition

`apps/flutter_mobile/lib/features/plan_screen.dart`

- Plan dirty state includes name, counts, growth toggles, growth mode, wordbook, shared/per-mode growth rules.
- Save payload clamps counts and preserves `questionTypeWeightsByMode`.
- `_persistPlanAndWordbook()` toggles wordbook first, then saves plan.
- `_applyToToday()` saves, calls `applySavedPlanToToday()`, reloads wordbooks, and refreshes local state.

`apps/flutter_mobile/lib/sdk/today_client.dart`

- Today state is a DTO returned from `getHomeState()`, including `dailyProgress`.
- Today should not be locally "completed" by clicking answer options.

## Validation Architecture

Phase 14 needs source-parity tests before and after UI changes:

- Study flow tests:
  - varied questions for a 20-question `newWord` session
  - four choices preserved
  - single tap select, double tap submit
  - choice row state resolver marks correct/wrong rows like Flutter
  - Today progress is not incremented per question
- Croc BTI tests:
  - no default answers
  - result requires all canonical ids
  - profile text/assets copied from Flutter keys
  - rebalance totals equal 100
  - apply calls save profile, save plan, apply today, sync flush in order
- Reports tests:
  - chart point math matches Flutter constants
  - daily point selection updates selected detail
  - mode selection toggles compact chart source from `modeSeries`
- Build checks:
  - typecheck
  - focused tests
  - WeChat build

## Research Complete

This phase can now be planned. The main risk is continuing to patch presentation while mock/session contracts remain wrong. Execute data/session fixes before UI polish.
