# Feature: Plan Page

> Slug: `plan-page`
> Status: `mobile_in_progress`
> Updated: `2026-08-06`

## Product Intent

Let users configure the daily learning rhythm in one place: new words, review, mixed tests, wrong-word reinforcement, root/affix practice, growth rules, and the active wordbook. The plan must drive Today task breakdowns and Study sessions consistently, while still letting users decide whether a change starts tomorrow or refreshes today.

## UX Contract

- The Plan page is the single editor for daily targets, growth rules, and wordbook selection.
- Only one wordbook can be selected at a time.
- Selecting a wordbook or editing targets is local until the user taps either save or sync/apply to today.
- Save Plan persists the new plan for future days; it does not silently rewrite today's task snapshot.
- Apply/Sync To Today persists the current edits, refreshes today's plan snapshot, invalidates study sessions, and updates Today/Study views immediately.
- If the user leaves Plan with unapplied edits, the shell must warn that the plan has not been applied.
- The Plan page must not jump scroll position when selecting a wordbook, especially when switching from Medical English to another book.
- Today and Study must show the same target counts. For word-based modes, one plan unit equals four questions.

## Shared Domain/Data Contract

- Flutter calls `PlanClient.getActivePlan`, `savePlan`, `applySavedPlanToToday`, `getWordbooks`, and `toggleWordbook`.
- Rust bridge stores future defaults in `saved_plan_json` and `saved_wordbooks_json`.
- Rust bridge stores today's frozen execution state in `today_plan_json`, `today_wordbooks_json`, `today_review_wordbooks_json`, and `today_snapshot_seed_json`.
- `apply_saved_plan_to_today` copies saved plan/wordbook state into today state, persists target seeds, and clears active study session snapshots.
- Review may preserve `today_review_wordbooks_json` when switching to a new wordbook that has no review candidates today.
- Growth fields include `growthRuleEnabled`, `growthRuleMode`, `sharedGrowthRule`, `growthRulesByMode`, `growthRuleStartDate`, and `growthRuleStartDatesByMode`.
- Today targets are derived from Rust, not Flutter. Flutter only displays returned counts.
- Review candidates include prior answered words except skipped answers. Incorrect prior answers still count as learned enough to review.

## Flutter Mobile Route

- Owner screen/widget: `apps/flutter_mobile/lib/features/plan_screen.dart`, hosted in `MobileRootShell`.
- SDK/bridge calls: `PlanClient` methods in `apps/flutter_mobile/lib/sdk/plan_client.dart`.
- Loading/cache/reload behavior: Plan loads active plan and wordbooks, keeps pending wordbook selection locally, reports dirty state to the shell, and uses `onTodayPlanApplied` to refresh Today, Study, Reports, Wrong, and AI seeds.
- Orientation/gesture constraints: phone portrait, long scroll form, bottom navigation, pull-to-refresh, stepper controls, radio wordbook selection, and small-screen scroll preservation.
- First implementation slice: Flutter Plan editor with RN-parity targets, single wordbook selection, growth rules, save/apply semantics, and Today/Study invalidation.
- Current status: mobile in progress; main behavior is implemented and regression-tested, but text encoding cleanup and visual polish remain.

## Tauri Desktop Route

- Owner view/window: desktop plan/settings view under the main learning workspace.
- Shared APIs to reuse: the same Rust plan bridge/domain APIs used by Flutter, especially saved-vs-today snapshot separation and review wordbook preservation.
- Desktop-specific layout: multi-column settings form with a live Today preview panel, a wordbook table/list, and explicit actions for Save For Future Days and Apply To Today.
- Mobile assumptions to avoid: do not copy the mobile long-card scroll layout, bottom navigation dirty guard, or radio focus workaround. Desktop should use stable panes and keyboard-friendly form controls.
- First parity slice: load active plan and wordbooks, edit daily targets, select exactly one wordbook, save future plan, apply to today, and show Today target preview.
- Current status: planned.

## Sync And Storage

Plan and wordbook preferences are user configuration and should sync as saved defaults. Today snapshots are local execution state for the current day and must be treated as frozen once applied, except when the user explicitly applies a new plan to today.

`saved_wordbooks_json` is the future/default wordbook preference. `today_wordbooks_json` is the new-word/mixed/root-affix wordbook for today. `today_review_wordbooks_json` can intentionally differ so old review tasks are not lost when the new wordbook has no review pool yet.

Applying a plan to today must clear active study session snapshots and in-memory active sessions so Study cannot keep serving stale questions from the previous plan or wordbook.

## AI Or Provider Implications

No direct provider calls. AI passage eligibility and content depend indirectly on Today task completion and wrong-word/target-word context after plan application.

## Implementation Log

- `2026-08-07`: Phase 6 producer promotion captures the canonical six question-unit targets, fixed New Word multiplication, configurable Review/Mixed/Wrong ratios, and explicit saved-plan versus apply-to-today boundary in the reviewed Wave 6 fixture set. The promotion proves the producer contract and browser artifact parity, not mobile or Word Net UI acceptance.
- `2026-08-06`: Added `highFrequencyPerDay` to saved/today plan JSON, growth rules, Rust Today targets/completions, Plan editing, Study target hydration, Today task cards, and completion calculation. New backend plans default to 10 questions; older decoded/constructed plans remain backward-compatible at zero when the field is absent outside the backend default payload.
- `2026-04-24`: Flutter main learning flow began recovering Plan/Study behavior from RN parity.
- `2026-05-19`: Plan active plan and wordbooks moved toward shell-level preload/cache expectations.
- `2026-06-25`: Rebuilt this feature ledger as the durable cross-platform Plan page record.
- `2026-06-25`: Mobile Plan changed wordbook selection from immediate backend toggle to pending local edit controlled by Save Plan or Apply To Today.
- `2026-06-25`: Added dirty-state warning when leaving Plan with unapplied changes.
- `2026-06-25`: Fixed Apply To Today refresh propagation so Today and Study no longer require app restart/re-entry.
- `2026-06-25`: Fixed Study session reuse so changed wordbook/source requests rebuild active sessions.
- `2026-06-25`: Fixed review targets dropping from 28 to 20 when review candidates included incorrect prior answers and review bucket de-duplication underfilled the target.
- `2026-06-25`: Fixed missing `today_review_wordbooks_json` fallback to use `today_wordbooks_json` instead of default wordbook selection.
- `2026-06-25`: Preserved Plan scroll offset and cleared focus when changing wordbooks to avoid automatic jumps in the wordbook management section.

## Mobile Lessons Learned

- Do not let a wordbook radio tap write directly to backend state. It makes the "tomorrow vs today" decision invisible and causes stale Today/Study behavior.
- `IndexedStack` keeps Study alive. A plan/wordbook application must bump a study reload key or the old Study state can remain visible.
- Rust core active sessions must compare current request sources before reusing an in-memory session.
- Today task targets and Study target counts must come from the same Rust plan snapshot path.
- Review target alignment must not count only correct answers; incorrect prior answers are part of the review pool.
- Review bucket de-duplication must skip already selected words before counting quota, or targets can underfill by one or two words.
- Dirty banners or focus changes inside a long scroll form can shift visible content. Preserve scroll position around wordbook selection changes.

## Desktop Follow-Up Notes

Desktop should expose saved-vs-today state explicitly, ideally with a preview panel showing what will change before applying. It should preserve the mobile contract but not copy the mobile workaround for scroll/focus jumps. Desktop can show review wordbook divergence as a small explanation when today's review remains on an older wordbook.

Desktop should add focused parity checks for:

- Save future plan does not mutate today's snapshot.
- Apply to today updates Today preview and Study source pool.
- Changing wordbook with no review candidates keeps today's review pool.
- Review target count remains plan units times four when enough previously answered words exist.

## Route Changes

- `2026-06-25`: Route clarified from "Plan edits immediately saved" to "Plan edits are pending until Save or Apply".
- `2026-06-25`: Wordbook selection changed from multi-toggle semantics to single-selection semantics.
- `2026-06-25`: Today review wordbook can intentionally diverge from today active wordbook to preserve same-day review continuity.
- `2026-06-25`: Study session cache invalidation became part of the Plan apply contract.

## Known Pitfalls

- Do not infer a high-frequency session size from the 100-word learning pool; use `highFrequencyPerDay` from the frozen Today plan.
- Do not compute or clamp Today/Study target counts in Flutter.
- Do not use `saved_wordbooks_json` as today's source after a day snapshot exists.
- Do not default missing review wordbook snapshot to the global default wordbook; fall back to `today_wordbooks_json`.
- Do not reuse active study sessions when the requested entry source IDs changed.
- Do not count `take(n)` before de-duplicating review candidates.
- Do not make wordbook selection immediately persist on every radio tap.
- Do not allow save/apply snackbar or dirty banner changes to move the user away from the wordbook list.

## Verification

- Mobile (`2026-08-06`): `flutter test test\croc_bti_model_test.dart test\today_task_breakdown_test.dart` passed all 17 cases, including unchanged legacy Plan/Today behavior while the new backend field is additive.
- Mobile (`2026-08-06`): targeted `flutter analyze --no-pub` over the eight changed Plan/Croc/Study/Today/Reports Dart files passed with no issues.
- Shared/domain (`2026-08-06`): `cargo check --workspace` passed with existing platform dead-code warnings.
- Mobile:
  - `flutter analyze --no-pub`
  - Manual expected flows: edit target, save for future, apply to today, leave dirty Plan, switch wordbook from Medical English, pull-refresh Today, enter Study.
- Desktop:
  - Pending. First parity slice should reuse the shared Rust checks before UI work is considered complete.
- Shared/domain:
  - `cargo check -p word-platform-mobile`
  - `cargo test -p word-platform-mobile review_targets_include_incorrect_prior_answers_as_learned_words --lib`
  - `cargo test -p word-platform-mobile review_wordbook_defaults_to_today_wordbook_when_review_snapshot_is_missing --lib`
  - `cargo test -p word-platform-mobile applying_new_wordbook_keeps_today_review_on_old_wordbook_without_candidates --lib`
  - `cargo test -p word-platform-mobile saved_wordbook_change_does_not_change_today_wordbook_until_applied --lib`
  - `cargo test -p word-platform-mobile review_selection_spreads_across_prior_learning_age_buckets --lib`
  - `cargo test -p word-app-core start_session_rebuilds_in_memory_session_when_sources_changed --lib`
  - `cargo test -p word-app-core start_session_discards_same_day_snapshot_when_sources_changed --lib`
