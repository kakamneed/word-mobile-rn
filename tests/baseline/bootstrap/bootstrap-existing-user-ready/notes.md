# bootstrap-existing-user-ready

## Why this sample matters

Captures the state when an existing user reopens the app.
This should not redirect to onboarding and should show appReady with settings available.

- Verifies that `firstRunRequired=false` after onboarding completion.
- Verifies that `settingsEntryAvailable=true` for initialized databases.
- Protects against Flutter migration re-showing onboarding to existing users.

## Known sensitivities

- Requires `app_metadata` table to have `onboarding_completed=true`.
- Requires database to be fully initialized with schema.
