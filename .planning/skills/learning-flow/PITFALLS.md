# Learning Flow Pitfalls And Recipes

## Quick Rule

When a bug crosses Today, Study, AI, reports, wrong words, leaderboard, or persistence, locate the earliest wrong source before patching the visible symptom.

## Pitfall: Selected Wrong Option Is Not Red

Symptoms:

- User selects a wrong A/B/C/D option.
- Correct option may turn green, but selected wrong option stays neutral.

Inspect first:

1. `apps/flutter_mobile/lib/features/study_screen.dart`
2. `StudyResult.userResponse`
3. `StudyQuestion.choices`
4. `StudyQuestion.correctChoiceLabel`
5. `apps/flutter_mobile/test/study_question_display_test.dart`

Recipe:

- Match user response against label, value, and display text tokens.
- Keep correct matching separate from user-wrong matching.
- Add or update tests where wrong A is red while correct B/C/D is green.

Never:

- Infer wrong selected state only from `result.isCorrect == false`.
- Compare only display text if stored result may be label-shaped.

## Pitfall: Correct Answer Drifts To A

Symptoms:

- The UI or result eventually points to A no matter which option is correct.

Inspect first:

1. `crates/study-core/src/question_builder.rs`
2. `crates/study-core/src/answer_evaluator.rs`
3. `crates/app-core/src/facade/study_facade.rs`
4. `crates/platform-mobile/src/bridge.rs`
5. `apps/flutter_mobile/lib/sdk/study_client.dart`
6. `apps/flutter_mobile/lib/features/study_screen.dart`

Recipe:

- Require valid `correctChoiceLabel` for choice questions.
- Reject missing/invalid correctness payloads with a protocol error.
- Test non-A correct labels through start, submit, history, resume, Dart decode, and UI feedback.

Never:

- Use `correctChoiceLabel ?? 'A'`.
- Treat stale persisted A labels as more authoritative than actual question content.

## Pitfall: Cold-Start Study Bounces Back To Today

Symptoms:

- App launches.
- User enters Study quickly.
- Auth/local-owner refresh closes Study and returns to Today once.

Inspect first:

1. `AppState.initialize` and `refreshAuthState`.
2. `AuthSessionManager.resolveStartupState`.
3. `MobileRootShell._handleAppStateChanged`.
4. `TodayShellScreen._loadHomeBundle`.
5. `StudyScreen.didUpdateWidget`.

Recipe:

- Treat null -> resolved owner during first startup as initial load, not a post-initial owner change.
- Only clear Study when owner changes after an initial owner has already been established.
- Avoid automatic Today plan apply during display fallback.

Never:

- Add timer delays.
- Ignore the first back event.
- Hide the bounce with navigation animation hacks.

## Pitfall: Today Fallback Mutates Plan/Today Truth

Symptoms:

- Today snapshot missing.
- Active plan fallback displays.
- The app silently applies plan to Today or invalidates Study sessions during load.

Recipe:

- Keep fallback display-only.
- Require explicit user action for apply/sync.
- If applying plan, clear only stale active session snapshots that are invalidated by the change; preserve historical results.

## Pitfall: AI Blocks The Study Loop

Symptoms:

- AI provider error prevents Today/Study from continuing.
- AI history/import failure traps the user.

Recipe:

- Route through `AiClient`.
- Catch errors into visible messages.
- Keep Today/Study navigation independent of AI success.
- Flush sync opportunistically, not as a blocking prerequisite.

## Pitfall: Reports Or Wrong Words Use Page-Local Truth

Symptoms:

- Reports differ after reload.
- Wrong-word changes disappear or conflict with mastered/trash.

Recipe:

- Reports must read `ReportsClient.getReportsOverview`.
- Wrong Words must read/write through `WrongWordsClient`.
- Mastered/trash exclusion must win over wrong-word display.

## Pitfall: Leaderboard Or Image Vote Pulls Remote Truth Into Local Flow

Symptoms:

- Local leaderboard fails unless Supabase RPC works.
- Image vote mode succeeds visually without Rust/SQLite state.

Recipe:

- Keep `LeaderboardScreen` local-first via `RewardImageClient`.
- Treat `LeaderboardService` as future-only unless explicitly activated.
- Verify empty state, local rows, vote success/failure, entitlement failure, and upload failure.

## Pitfall: Validation Accounting Overclaims

Symptoms:

- Summary says done, but tests timed out.
- Release-device smoke was never run.

Recipe:

- Use `PASS`, `PASS WITH NOTE`, `BLOCKED`, `DEFERRED`, and `HUMAN NEEDED`.
- Keep release-device result slots explicit.
- Never count a timeout as pass.
