# Baseline Runner Specification

## How to run

```bash
# All baseline tests (serial execution required due to global session state)
cargo test --package word-app-core --test baseline_runner -- --test-threads=1

# Specific category
cargo test --package word-app-core --test baseline_runner baseline_bootstrap
cargo test --package word-app-core --test baseline_runner baseline_today
cargo test --package word-app-core --test baseline_runner baseline_study
```

## Serial execution requirement

Study tests share a global `ACTIVE_SESSIONS` in-memory map. Tests that use the same
session mode (e.g., two MixedTest tests) will collide when run in parallel.
Use `--test-threads=1` until the facade boundary is refactored in Slice 2.

## Test categories

| Category | Tests | What it validates |
|----------|-------|-------------------|
| bootstrap | 2 | First-run vs existing user bootstrap state |
| today | 1 | Plan-to-snapshot derivation, progress, wordbooks |
| study | 3 | Full study lifecycle: newWord all-correct, mixedTest incorrect, cancel-no-persist |
| invariant | 2 | Progress monotonicity, isComplete/summary mutex |

## Golden file structure

Each sample directory contains:
- `input.json` - Preconditions and step sequence
- `expected-output.json` - Structural assertions
- `expected-db-delta.json` - Database change expectations
- `notes.md` - Why this sample matters

## Assertion types

The runner supports these assertion kinds:
- `equals` - Exact value match
- `notNull` - Value must exist and not be null
- `greaterThan` / `greaterThanOrEqual` - Numeric comparison
- `type` - Type check ("array", "object", etc.)
- `minLength` - Array/string minimum length

## Current coverage matrix

| Sample ID | Category | Test function | Status |
|-----------|----------|---------------|--------|
| bootstrap-first-run-ready | bootstrap | baseline_bootstrap_first_run_ready | PASS |
| bootstrap-existing-user-ready | bootstrap | baseline_bootstrap_existing_user_ready | PASS |
| today-plan-stable-baseline | today | baseline_today_plan_stable | PASS |
| today-after-plan-edit-same-day | today | baseline_today_snapshot_not_polluted_by_plan_edit | PASS |
| study-newword-all-correct | study | baseline_study_newword_all_correct | PASS |
| study-review-all-correct | study | baseline_study_review_mode | PASS |
| study-mixed-incorrect-to-wrongword | study | baseline_study_mixed_incorrect | PASS |
| study-fuzzy-correct | study | baseline_study_fuzzy_correct | PASS |
| study-skip-answer | study | baseline_study_skip_answer | PASS |
| study-cancel-no-report-commit | recovery | baseline_study_cancel_no_persist | PASS |
| study-resume-after-restart | recovery | baseline_session_persistence_across_restart | PASS |
| progress-monotonic | invariant | baseline_progress_monotonic_in_session | PASS |
| is-complete-summary-mutex | invariant | baseline_is_complete_and_summary_mutex | PASS |

## Not yet covered (needs additional samples)

| Sample ID | Category | Why needed |
|-----------|----------|------------|
| today-carryover-baseline | today | Carryover tasks from previous days |
| study-wrongword-reinforcement | study | Wrong-word reinforcement mode |
| study-root-affix-bidirectional | study | Root/affix question types |
| study-engine-version-fail-open | recovery | Old snapshot upgrade behavior |
| wrongword-detail-risk-breakdown | wrong-words | Risk breakdown and error history |
| reports-daily-series-after-session | reports | Report aggregation after completion |
| ai-missing-config-non-blocking | ai | AI failure doesn't block learning |
| ai-generation-failure-non-blocking | ai | AI request failure is non-blocking |
