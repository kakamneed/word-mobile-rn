---
name: flutter-today-study-target-consistency
description: Diagnose and fix Flutter mobile Today task breakdown versus study-session question-count mismatches. Use when Today shows an updated plan or growth-adjusted target, but entering a study task still uses an old or base question count, such as Today showing 8/32 while new-word study opens as 8/24.
---

# Today Study Target Consistency

Use this skill for `apps/flutter_mobile` and `crates/platform-mobile` bugs where Today, Plan, and the concrete study session disagree about task totals.

## Core Rule

Treat Today task breakdown and study-session generation as two separate consumers that must share one target source.

- Today cards read `todaySnapshot` targets built from `today_target_seed_from_plan_value`.
- Study sessions read unit counts through `study_mode_target_count`, then question count is derived by `word_study_core::QuestionBuilder`.
- New word and review modes multiply words by 4 questions per word. Test modes and root/affix use one question per unit.

If Today is correct but task entry is wrong, do not stop at fixing `todaySnapshot`. Check `study_mode_target_count` and active session restore logic.

## Debug Checklist

1. Confirm the expected target in Today:
   - `newWordsTarget`, `reviewWordsTarget`, `mixedTestTarget`, `wrongWordTestTarget`, `rootAffixTarget`.
   - Example: Plan base `newWordsPerDay = 6` plus growth `+2` means Today should show `8/32`.

2. Confirm the concrete session target:
   - Start the affected mode and inspect `StartSessionResponse.progress.total`.
   - For new-word/review, `progress.total` should equal grown unit count times 4.

3. Check both target paths in `crates/platform-mobile/src/bridge.rs`:
   - `today_target_seed_from_plan_value_for_date`
   - `study_mode_target_count`
   - Both must use `grown_plan_unit_count` for growth-adjusted plans.

4. Check stale active session reuse:
   - `apply_active_session_restore_shape`
   - `word_app_core::start_study_session`
   - `active_study_session_*` rows in `app_settings`
   - In-memory sessions via `word_app_core::clear_all_active_sessions`

5. When applying a saved plan to today, clear stale active sessions:
   - Delete `active_study_session_%` settings.
   - Clear in-memory active sessions.
   - Do not delete `study_sessions` or `study_results`; Today progress should keep already answered questions.

## Regression Tests To Add

Add focused Rust tests in `crates/platform-mobile/src/bridge.rs`:

- Applying a plan to Today clears stale `active_study_session_*` snapshots.
- `today_target_seed_from_plan_value_for_date` reflects changed plan targets.
- Growth rules increase Today targets after elapsed intervals.
- `hydrate_start_session_request` uses growth-adjusted target counts, e.g. base 6 new words plus growth +2 hydrates 8 entry payloads and produces 32 questions.

Run:

```powershell
cargo fmt
cargo test -p word-platform-mobile today_target_seed --lib
cargo test -p word-platform-mobile growth --lib
cargo test -p word-platform-mobile new_word_session_uses_growth_adjusted_today_target_count --lib
cargo check -p word-platform-mobile
flutter analyze --no-pub
```
