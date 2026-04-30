# study-mixed-incorrect-to-wrongword

## Why this sample matters

Validates that an incorrect answer in mixedTest mode produces the correct result outcome,
and that the session summary reflects the error. This is the primary sample for catching
regressions where Flutter might display "wrong" but not persist the incorrect state.

- Verifies `outcome=incorrect` when response does not match any accepted meaning.
- Verifies summary incorrectCount is at least 1.
- Verifies mixedTest generates 1 question per word (not 4 like newWord/review).

## Known sensitivities

- Requires deterministic question generation.
- The "WRONG_VALUE" response must not match any accepted meaning.
