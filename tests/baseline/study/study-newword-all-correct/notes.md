# study-newword-all-correct

## Why this sample matters

This is the simplest happy-path study cycle: new word mode, all correct answers.
It validates the core question generation, answer evaluation, progress tracking,
session completion, and summary generation pipeline end to end.

- Verifies newWord mode generates 4 questions per word (enToCnChoice, exampleToCnChoice, cnToEnChoice, enToCnInput).
- Verifies progress advances monotonically.
- Verifies isComplete only becomes true on the last question.
- Verifies summary counts are correct after all-correct answers.
- Verifies accuracy is 100%.

## Known sensitivities

- Requires exactly 5 entry payloads with known words and meanings.
- Question generation is deterministic given the same words and mode.
- answer_outcome=correct requires response to match acceptedMeanings.
