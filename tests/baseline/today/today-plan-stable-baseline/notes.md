# today-plan-stable-baseline

## Why this sample matters

This is the most fundamental today state: active plan exists, no study completed yet today.
It validates that the today snapshot is correctly derived from the plan configuration.

- Verifies that plan fields map correctly to snapshot targets.
- Verifies that completed counts are 0 at start of day.
- Verifies that daily progress shows correct total tasks and next recommended action.
- Verifies that wordbooks are returned correctly.

## Known sensitivities

- Requires a seeded plan with known configuration values.
- todayDate is derived from system clock at call time, which is non-deterministic.
  Tests should use the clock fixture or mock the date.
