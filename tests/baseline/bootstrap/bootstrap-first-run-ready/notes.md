# bootstrap-first-run-ready

## Why this sample matters

Captures the exact state a brand new user sees on first launch.
This is the gate that determines whether the app enters onboarding or main flow.

- Verifies that `firstRunRequired=true` when onboarding metadata is absent.
- Verifies that `appReady=false` when bundled snapshot is missing (test environment has no bundled resources).
- Protects against Flutter migration accidentally skipping onboarding or misreading bootstrap state.

## Known sensitivities

- Depends on `app_metadata` table having no `onboarding_completed` entry.
- Depends on `bundled_resource_path` returning a missing file path in test environment.
