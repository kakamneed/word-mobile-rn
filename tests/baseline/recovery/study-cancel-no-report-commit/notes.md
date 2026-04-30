# study-cancel-no-report-commit

## Why this sample matters

Validates that cancelling a study session mid-stream does NOT write any completed session
or results to the database. This prevents partial sessions from polluting reports and wrong words.

- Verifies cancel operation succeeds.
- Verifies no study_sessions row with completed_at is created.
- Verifies no study_results rows are persisted.
- Protects against Flutter migration where cancel might not fully clean up.

## Known sensitivities

- The global ACTIVE_SESSIONS state must be cleared by cancel.
- Any persisted session snapshot must also be removed.
