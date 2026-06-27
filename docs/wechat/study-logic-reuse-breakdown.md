# Study Logic Reuse Breakdown

Date: 2026-05-20
Status: Implementation guide
Scope: WeChat Mini Program scheme B backend

## Goal

Replace the current Mini Program backend study placeholder with the same
business semantics used by the Flutter app, without changing Flutter SDK,
Rust bridge, or local SQLite behavior.

The Mini Program migration is a light-frontend migration. Except for platform
additions such as WeChat login/session handling, Study behavior should not be
rewritten in Taro or Node. Taro should imitate the necessary Flutter UI/SDK
wrapper shape, while the backend should call the same Rust domain source used
by Flutter.

The Mini Program should call first-party HTTP endpoints. Those endpoints should
reuse Rust/domain study logic server-side wherever correctness lives today.

## Existing Flutter Path

```text
StudyScreen
  -> WordSdk.study
  -> StudyClient DTO validation
  -> RustBridge method
  -> app-core study_facade
  -> study-core QuestionBuilder / AnswerEvaluator / SessionSummaryService
  -> storage-core persistence side effects
```

## Unit Breakdown

| Unit | Existing owner | Mini Program reuse decision | Notes |
|---|---|---|---|
| Page orchestration | `StudyScreen` | Reimplement in Taro | UI state, focus, dialogs, and navigation are platform-specific. |
| DTO contract | `StudyClient` | Reuse shape exactly | HTTP responses must preserve required fields and choice validation shape. |
| Session mode rules | `study-core/session_definition.rs` | Reuse Rust | Defines newWord/review/mixedTest/wrongWord/rootAffix semantics. |
| Question generation | `study-core/question_builder.rs` | Reuse Rust | Contains weighted question types, all-four loop, distractors, root/affix, choice labels. |
| Answer grading | `study-core/answer_evaluator.rs` | Reuse Rust | Handles choice label authority, fuzzy input, word skeleton, skipped answers. |
| Session summary | `study-core/session_summary.rs` | Reuse Rust | Avoid reimplementing accuracy, counts, wrong word summary. |
| Active session state | `app-core/facade/study_facade.rs` | Adapt server-side | Flutter uses process memory plus local persisted snapshot. Server needs per-user/session store. |
| Session resume | `study_facade` persisted snapshot | Adapt server-side | Use DB/Redis-like storage keyed by `internal_user_id`, mode, session id. |
| Study event persistence | `storage-core/persistence/study_repo` | Adapt to Supabase | Existing Flutter SQLite persistence cannot be reused directly; preserve event and DTO semantics. |
| Wrong-word projection | storage projection tables | Adapt to Supabase | Use `wrong_word_entries` projection or rebuild from study events. |
| Disputed meaning | `accept_disputed_meaning` + cloud service | Defer or admin-safe | Needs user accepted meaning persistence and moderation posture. |
| Mark mastered | `mark_study_entry_mastered` | Phase after MVP | Needs mastered-entry storage in cloud before parity. |
| Complete/cancel | `completeStudySession`, `cancelStudySession` | Add after start/submit parity | Required for polish and resume cleanup. |
| Hint prompt | Flutter wrong-word hint logic | MVP deterministic only | AI suggestions remain gated. |

## Required HTTP Endpoint Parity

### Start Session

Endpoint: `POST /v1/study/sessions`

Must accept:

- `mode`
- optional `wordbookId`
- optional `entrySourceIds`
- optional `entryPayloads`
- optional `distractorPayloads`
- optional `questionTypeWeights`

Must return Flutter-compatible `StartSessionResponse`:

- `session`
- `currentQuestion`
- `progress`
- `answeredQuestions`

### Submit Answer

Endpoint: `POST /v1/study/answers`

Must accept:

- `questionId`
- `response`
- `responseTimeMs`

Must return Flutter-compatible `SubmitAnswerResponse`:

- `result`
- `progress`
- `answeredQuestions`
- `isComplete`
- optional `currentQuestion`
- optional `summary`
- optional `nextAction`
- optional deterministic `hintPrompt`

### Wrong Words

Endpoints:

- `GET /v1/wrong-words`
- `GET /v1/wrong-words/:entryId`

Must derive from the same post-answer truth as Flutter:

- incorrect/skipped attempts affect wrong-word state
- correct/fuzzy correct should not inflate wrong counts
- reports and leaderboard derive from server-side accepted study events

## Reuse Architecture

Recommended bridge:

```text
server/miniprogram-api
  -> StudyDomainAdapter
  -> Rust domain runner or Rust domain service
  -> app-core facade-compatible operations
  -> word-study-core + word-storage-core models
  -> Supabase adapter for persistence/projections
```

The server should not call Flutter or mobile RustBridge. It should call a
server-owned adapter that exposes the same domain operations:

- `startStudySession(input)`
- `submitStudyAnswer(input)`
- `completeStudySession(sessionId)`
- `cancelStudySession(sessionId)`
- `markStudyEntryMastered(input)`
- `acceptDisputedMeaning(input)`

## Migration Slices

1. **DTO golden parity**
   - Create representative JSON fixtures for Flutter `StartSessionResponse`
     and `SubmitAnswerResponse`.
   - Validate Mini Program server responses include required fields.

2. **Answer evaluator parity**
   - Port tests around choice label authority, fuzzy input, skipped input, and
     word skeleton.
   - Prefer calling Rust `AnswerEvaluator` instead of translating to JS.

3. **Question builder parity**
   - Start with server-fed `entryPayloads` and `distractorPayloads`.
   - Verify newWord all-four loop, mixedTest weighted pool, wrong-word pool,
     rootAffix fixed root-to-gloss behavior.

4. **Server session store**
   - Store active session by `internal_user_id`, `session_id`, and mode.
   - Persist question list, current index, results, and question type weights.

5. **Supabase side effects**
   - Write `study_events`.
   - Update or rebuild report snapshots and wrong-word projections.
   - Keep Flutter SQLite behavior untouched.

6. **Deferred parity**
   - Mastered entry pruning.
   - Accepted disputed meaning.
   - Complete/cancel cleanup.
   - AI hint suggestions.

## Current Gap In Mini Program Backend

`server/miniprogram-api` now has the Flutter StudyClient method surface exposed
over HTTP:

- `startStudySession`
- `getResumeSessionHint`
- `submitStudyAnswer`
- `markStudyEntryMastered`
- `acceptDisputedMeaning`
- `completeStudySession`
- `cancelStudySession`

The TypeScript Mini Program SDK has been adjusted to imitate Flutter DTOs,
including choice labels/text, `correctChoiceLabel`, phonetics, normalized
responses, summaries, hint prompts, mastered, disputed meaning, complete, and
cancel wrappers.

A Rust runner crate, `word-study-domain-runner`, exists for server-side reuse of
`word-study-core` and `word-storage-core` models. It currently exposes:

- `build-session`: calls Rust `QuestionBuilder`
- `evaluate-answer`: calls Rust `AnswerEvaluator`
- `complete-session`: calls Rust `SessionSummaryService`

`RustRunnerStudyDomain` now stores the runner-built question list and advances
payload-backed sessions through Rust answer evaluation and Rust summaries.
When Taro sends only mode/wordbook/source ids, the backend asks its adapter for
`entryPayloads` and `distractorPayloads` before invoking Rust. The memory
adapter provides fixture payloads for local checks; the real Supabase adapter
maps rows from `study_entry_payloads`.

Rust session start, answers, and completion now emit `study_events` through the
adapter. Rust answer events are also projected into report snapshots and
wrong-word entries; leaderboard reads derive from report snapshots. The
remaining production work is content import into `study_entry_payloads` and
manual Mini Program DevTools smoke.

See `docs/wechat/study-cloud-content-import.md` for the exact schema push,
payload export/import, and smoke commands.
