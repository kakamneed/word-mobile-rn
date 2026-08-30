# Feature: Reports Page

> Slug: `reports-page`
> Status: `mobile_in_progress`
> Updated: `2026-07-17`

## Product Intent

Give learners a trustworthy progress report across study days and study modes. The page should answer:

- what has been answered
- how accurate each mode is
- which days actually had study activity
- whether mode-specific trends are improving or exposing weak spots

Reports are a read-only reflection of persisted study side effects. Flutter renders the story; Rust owns the aggregate truth.

## UX Contract

- The Reports tab shows overall study metrics, streak context, daily accuracy trend, and per-mode cards.
- Per-mode cards show total questions, correct count, wrong count, accuracy, advice, and an optional mode trend chart.
- Trend charts must start from the first day with actual answered questions for that series.
- Days with no answered questions must not be drawn as 0% points.
- Dates must follow the user's local study day. UTC timestamps around midnight must not appear under the previous calendar day in China time.
- Per-mode card totals and per-mode chart points must use the same aggregation source.
- Empty modes can show summary cards with zero totals, but their charts should not invent zero-answer history.
- The page has `单词学习 / 模拟练习` modes. Practice mode first selects an exam family, then shows one whole-paper accuracy line and separate listening, cloze, reading, and new-type lines across attempted papers.
- Translation and writing are subjective and must never enter practice accuracy totals or type lines.

## Shared Domain/Data Contract

Primary read contract:

- Flutter SDK: `ReportsClient.getReportsOverview()`
- Flutter SDK: `ExamPracticeClient.getPracticeReport()`
- Bridge method: `getReportsOverview`
- Rust domain aggregator: `word_app_core::services::reports_service::build_reports_overview`
- Mobile bridge history loader: `load_reports_history`

Payload shape:

```json
{
  "totalStudyDays": 1,
  "totalWordsLearned": 52,
  "totalQuestionsAnswered": 93,
  "overallAccuracy": 64.5,
  "streakInfo": {
    "currentStreak": 1,
    "longestStreak": 1,
    "lastStudyDate": "2026-05-01"
  },
  "modeBreakdown": [
    {
      "mode": "review",
      "totalQuestions": 24,
      "correctCount": 16,
      "accuracyPercent": 66.6667
    }
  ],
  "dailySeries": [
    {
      "date": "2026-05-01",
      "totalQuestions": 38,
      "correctCount": 24,
      "accuracyPercent": 63.1579,
      "studyTimeMs": 402000
    }
  ],
  "modeSeries": {
    "review": [
      {
        "date": "2026-05-01",
        "totalQuestions": 24,
        "correctCount": 16,
        "accuracyPercent": 66.6667,
        "studyTimeMs": 9000
      }
    ]
  }
}
```

Contract rules:

- `load_reports_history` should aggregate from `study_results.answered_at`, not only from completed sessions.
- `answered_at` must be converted to local date before grouping.
- Report rows with `totalQuestions == 0` are ignored by aggregate series and study-day counts.
- Stored enum strings may be JSON-encoded, for example `"\"mixedTest\""`, and must be normalized before aggregation.
- Flutter must not recompute authoritative totals from partial visible chart data.
- `getExamPracticeReport` aggregates current `exercise_attempts` rows with non-null correctness. Whole-paper points include all auto-graded objective attempts; type points use normalized section classification and omit absent types instead of drawing 0% points.

## Flutter Mobile Route

- Owner screen/widget: `apps/flutter_mobile/lib/features/reports_screen.dart`
- SDK/bridge calls: `WordSdk.reports.getReportsOverview()`
- Loading/cache/reload behavior: load on tab init, refresh through `RefreshIndicator`, show recoverable error state on bridge failure.
- Orientation/gesture constraints: vertical phone layout with horizontally scrollable charts; chart points are touch targets; volume overlays or narrow screens can occlude rightmost points, so trend content must remain horizontally scrollable.
- First implementation slice: overall cards, daily chart, per-mode cards with expandable compact charts.
- Current status: mobile implemented and actively corrected for report-history accuracy.
- Practice reports reuse the existing horizontally scrollable accuracy chart, but paper identity is stable `paperId`; the axis label is derived from the paper year/set title.

Mobile presentation rules:

- `_DailyLineChart` receives already-authoritative series data.
- Selected day detail is presentation state only.
- Compact charts should render no points when series is empty instead of filling dates.
- Mode labels/colors live in Flutter, but mode identity and metrics come from Rust.

## Tauri Desktop Route

- Owner view/window: future desktop Reports view.
- Shared APIs to reuse: call the same Rust report overview API or the shared app-core service through the desktop bridge.
- Desktop-specific layout: use a dashboard layout with a daily trend panel, mode table/cards, and an inspector panel for selected day or mode.
- Mobile assumptions to avoid: do not copy mobile horizontal scroll as the main chart navigation; desktop can show more points and hover tooltips.
- First parity slice: render the same `ReportsOverview` DTO, verify local-date grouping, and show daily plus per-mode charts with no zero-answer filler points.
- Current status: not started.

Desktop must reuse the same aggregation logic. It should not rebuild mode totals from desktop UI state.

## Sync And Storage

Local source of truth:

- `study_sessions`
- `study_results`
- learned count derived from distinct correct or fuzzy-correct entries

Storage implications:

- Reports should work for completed and in-progress sessions because `study_results` are the actual activity log.
- Sync should eventually upload answer records or server-side equivalents, not just completed-session summaries.
- Conflict handling should avoid double-counting duplicate answer records; stable result IDs or idempotency keys may be needed before cloud aggregation.

Offline behavior:

- Offline local answers should appear in local reports immediately after they are persisted.
- Cloud sync lag must not block local report truth.

## AI Or Provider Implications

None for the core report aggregate. AI may later explain trends or suggest study actions, but it must consume report truth rather than generate it.

## Implementation Log

- `2026-04-29`: Fixed report mode data not appearing by normalizing JSON-encoded stored modes in the Rust report path.
- `2026-05-02`: User reported inconsistent per-mode progress, missing `2026-05-01`, and charts containing no-answer 0% points.
- `2026-05-02`: Changed report history loading to aggregate from `study_results.answered_at` by local answer date and mode, including unfinished sessions.
- `2026-05-02`: Changed report aggregation to skip rows with `totalQuestions == 0`, so charts begin at actual answer days.
- `2026-05-02`: Added regression tests for unfinished-session answers and zero-answer series filtering.
- `2026-06-25`: Ledger created from conversation history using `cross-platform-feature-ledger`.
- `2026-07-17`: Added `getExamPracticeReport` across Dart, Rust, Android JNI/Java/Kotlin, and iOS C/Swift. The Rust aggregate groups submitted objective attempts by exam, paper, and section, then classifies listening, cloze, reading, and new-type sections.
- `2026-07-17`: Added the report content selector, exam-family dropdown, whole-paper line, and four fixed type panels. Empty type series say `暂无趋势数据` and do not fabricate failures.
- `2026-07-18`: Removed the Reports-level content selector. Reports consumes the single learning-content mode owned by `MobileRootShell` and changed only from the Today selector.

## Mobile Lessons Learned

- Completed-session date is not a reliable report date. Users expect the chart to reflect the day they answered questions.
- UTC string slicing causes visible off-by-one-day bugs in China time. Always convert `answered_at` to local date before grouping.
- Filling visual series with zero rows makes charts look like the user performed badly on days they did not study.
- Per-mode card totals and compact trend charts must share the same domain aggregation, otherwise they visibly disagree.
- Flutter chart fixes should not patch aggregate meaning; the fix belongs in Rust when the truth is wrong.
- A paper trend is not a time-series of repeated sessions; the current contract exposes the latest persisted per-question state for each paper. Stable paper identity prevents same-year CET sets from collapsing into one point.
- Shared bottom-navigation destinations must consume the Today-selected content mode instead of maintaining independent defaults.

## Desktop Follow-Up Notes

- Desktop should reuse the `ReportsOverview` contract unchanged for the first parity slice.
- Add hover details for points instead of mobile tap-only selected cards.
- Use a table or segmented control for mode comparison, but keep the no-zero-answer-point invariant.
- Include a debug/dev view for raw series data if desktop becomes the reporting QA surface.

## Route Changes

- `2026-05-02`: Report date semantics changed from `study_sessions.completed_at` to local date derived from `study_results.answered_at`.
- `2026-05-02`: Series semantics changed from possible zero-answer filler rows to answered-day-only rows for `dailySeries` and `modeSeries`.
- `2026-07-17`: Reports gained a second aggregate route for exam practice; study-day reports remain unchanged in word mode.
- `2026-07-18`: Reports no longer exposes its own mode route control; the root shell selects which existing aggregate view is visible.

## Known Pitfalls

- Do not group reports by `completed_at` only.
- Do not use `answered_at[0..10]` for local study day when timestamps can be UTC.
- Do not draw 0% points for days with no answers.
- Do not let Flutter recompute authoritative totals from chart subsets.
- Do not forget JSON-encoded enum normalization for modes, question types, and outcomes.
- Do not consider an empty mode chart a report failure; it may simply have no answered questions.
- Do not include translation/writing reference answers in accuracy. Only attempts with persisted non-null correctness are reportable.
- Do not use year alone as paper identity; multiple CET sets can share a year and month.
- Do not reintroduce a Reports-local practice/word toggle; it creates mode drift from Today and Wrong Words.

## Verification

- Mobile:
  - `flutter analyze --no-pub`
  - Manual expectation: `2026-05-01` answers appear in reports, and mode charts omit no-answer dates.
- Desktop:
  - Not implemented.
- Shared/domain:
  - `cargo test -p word-platform-mobile reports_history`
  - `cargo test -p word-app-core reports_service`
  - Regression: unfinished session with `2026-04-30T16:01:00Z` answers groups as local `2026-05-01`.
  - Regression: zero-answer history rows do not create chart points or study days.
- `2026-07-17`: `flutter test --no-pub test\reports_screen_test.dart` passed exam selection and paper/type trend rendering; `cargo test -p word-platform-mobile exam_report_classifies_supported_objective_sections --lib`, `cargo check -p word-platform-mobile`, and targeted Flutter analysis passed. Native route presence was verified across Dart plus Android/iOS surfaces; an APK/device smoke was not run.
- `2026-07-18`: The focused report regression passed with practice mode supplied by the parent and asserted no local segmented selector; targeted Flutter analysis reported no issues.
