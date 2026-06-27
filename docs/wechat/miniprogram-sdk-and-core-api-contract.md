# WeChat Mini Program SDK And Core API Contract

Date: 2026-05-20
Status: Draft
Plan: `.vico/plans/active/2026-05-20-wechat-miniprogram-migration.md`

## Purpose

This document maps the Flutter `WordSdk` surface into a Mini Program friendly
TypeScript SDK and backend API contract for the core learning MVP.

The Mini Program must not call the Rust bridge. It should call a first-party API
through a small `MiniProgramSdk` that preserves product semantics from the
Flutter app.

## Design Rules

- Mini Program feature pages call `MiniProgramSdk`, not raw `wx.request`.
- `MiniProgramSdk` returns typed models and normalized errors.
- Backend APIs own auth, user scoping, validation, and domain side effects.
- The client may cache display data, but it must not become the source of study
  correctness, wrong-word scoring, or report aggregation.
- AI endpoints are excluded from the MVP SDK.

## SDK Layout

Recommended package layout under the future Mini Program app:

```text
apps/wechat_miniprogram/
  miniprogram/
    app.ts
    app.json
    pages/
      today/
      study/
      wrong-words/
      reports/
      plan/
      account/
    sdk/
      client.ts
      errors.ts
      auth.ts
      today.ts
      plan.ts
      study.ts
      wrong-words.ts
      reports.ts
      rewards.ts
      sync.ts
      types.ts
```

## Common Transport

`MiniProgramSdk` wraps `wx.request`.

Request rules:

- Base URL comes from environment/config.
- Authenticated calls send `Authorization: Bearer <accessToken>`.
- JSON request/response only for MVP.
- All responses are parsed and validated before reaching pages.
- 401 triggers refresh once; repeated 401 returns `AUTH_REQUIRED`.

Common error shape:

```typescript
interface ApiErrorBody {
  error: {
    code: string;
    message: string;
    requestId?: string;
    details?: unknown;
  };
}
```

SDK error classes:

```typescript
type SdkErrorKind =
  | 'auth'
  | 'validation'
  | 'domain'
  | 'network'
  | 'protocol'
  | 'server';
```

## Client Families

### `auth`

Backed by `docs/wechat/account-and-api-foundation.md`.

Methods:

- `loginWithWechatCode(input): Promise<AuthSessionState>`
- `refreshSession(): Promise<AuthSession>`
- `logout(): Promise<void>`
- `getMe(): Promise<MeState>`
- `startEmailBind(email): Promise<EmailBindChallenge>`
- `verifyEmailBind(input): Promise<EmailBindResult>`
- `getMergePreview(mergeDecisionId): Promise<MergePreview>`
- `confirmMerge(input): Promise<MergeResult>`

### `today`

Flutter reference: `TodayClient.getTodayHomeState`.

Methods:

- `getTodayHomeState(): Promise<TodayHomeState>`

Endpoint:

`GET /v1/today`

Response:

```typescript
interface TodayHomeState {
  todayDate: string;
  activePlan?: PlanSummary;
  todaySnapshot?: Record<string, unknown>;
  wordbooks: WordbookSummary[];
  dailyProgress: Record<string, unknown>;
  resumeHint?: ResumeSessionHint;
  syncStatus?: SyncStatus;
}
```

Rules:

- Today targets and progress come from backend truth.
- Client must not guess target counts before the API returns.
- AI context/history must not be included in MVP `today`.

### `plan`

Flutter reference: `PlanClient`.

Methods:

- `getActivePlan(): Promise<PlanSummary | null>`
- `savePlan(input): Promise<PlanSummary>`
- `applySavedPlanToToday(): Promise<PlanSummary>`
- `getWordbooks(): Promise<WordbookSummary[]>`
- `setWordbookActive(input): Promise<void>`

Endpoints:

- `GET /v1/plan/active`
- `PUT /v1/plan/active`
- `POST /v1/plan/apply-to-today`
- `GET /v1/wordbooks`
- `PUT /v1/wordbooks/{wordbookId}`

Rules:

- Saving plan and applying it to today remain distinct actions.
- Backend owns growth rule semantics.
- Client editable drafts are not persisted truth.

### `study`

Flutter reference: `StudyClient`.

Methods:

- `startSession(input): Promise<StartSessionResponse>`
- `getResumeSessionHint(): Promise<ResumeSessionHint>`
- `submitAnswer(input): Promise<SubmitAnswerResponse>`
- `markEntryMastered(input): Promise<MarkStudyEntryMasteredResponse>`
- `completeSession(sessionId): Promise<CompleteSessionResponse>`
- `cancelSession(sessionId): Promise<void>`

Endpoints:

- `POST /v1/study/sessions`
- `GET /v1/study/resume-hint`
- `POST /v1/study/answers`
- `POST /v1/study/entries/{entrySourceId}/mastered`
- `POST /v1/study/sessions/{sessionId}/complete`
- `POST /v1/study/sessions/{sessionId}/cancel`

Key request shapes:

```typescript
interface StartSessionRequest {
  mode: StudyMode;
  entrySourceIds?: string[];
  entryPayloads?: StartSessionEntryPayload[];
  distractorPayloads?: StartSessionEntryPayload[];
  questionTypeWeights?: Record<string, number>;
}

interface SubmitAnswerRequest {
  questionId: string;
  response: string;
  responseTimeMs: number;
}
```

Rules:

- Backend owns answer correctness and side effects.
- Client records response time but backend validates the session/question.
- Completion side effects update wrong words and reports through backend domain
  logic.
- In-progress session truth is local/server session state, not navigation state.

### `wrongWords`

Flutter reference: `WrongWordsClient`.

Methods:

- `list(filter?: WrongWordFilter): Promise<WrongWordEntry[]>`
- `getDetail(entryId): Promise<WrongWordDetail>`
- `saveHint(input): Promise<WordHintState>`
- `getHintSuggestions(entryId): Promise<WordHintSuggestion[]>`

Endpoints:

- `GET /v1/wrong-words?filter=all`
- `GET /v1/wrong-words/{entryId}`
- `PUT /v1/wrong-words/{entryId}/hint`
- `GET /v1/wrong-words/{entryId}/hint-suggestions`

MVP rule:

- `hint-suggestions` returns deterministic/non-AI suggestions or an empty list.
  AI suggestions are a later compliance-gated feature.

### `reports`

Flutter reference: `ReportsClient`.

Methods:

- `getOverview(): Promise<ReportsOverview>`

Endpoint:

`GET /v1/reports/overview`

Response mirrors Flutter:

```typescript
interface ReportsOverview {
  totalStudyDays: number;
  totalWordsLearned: number;
  totalQuestionsAnswered: number;
  overallAccuracy: number;
  streakInfo: Record<string, unknown>;
  modeBreakdown: unknown[];
  last7Days: unknown[];
  dailySeries: unknown[];
  modeSeries: Record<string, unknown>;
}
```

Rules:

- Backend owns aggregate calculation.
- Client chart rendering must not change aggregate meaning.

### `rewards`

Flutter reference: `RewardClient`.

MVP methods:

- `getTodayRewardState(): Promise<TodayRewardState>`
- `drawTodayReward(input): Promise<TodayRewardState>`

Endpoints:

- `GET /v1/rewards/today`
- `POST /v1/rewards/today/draw`

Rules:

- One reward per user/date.
- Reward asset metadata should point to CDN/lazy-loaded assets.
- Save-to-gallery and user-uploaded images are not core MVP.

### `sync`

MVP methods:

- `getSyncStatus(): Promise<SyncStatus>`
- `flushPending(): Promise<SyncFlushResult>`
- `restoreSnapshot(): Promise<SyncSnapshot>`

Endpoints:

- `GET /v1/sync/status`
- `POST /v1/sync/flush`
- `GET /v1/sync/snapshot`

Rules:

- Mini Program can use lightweight local cache, but backend remains the durable
  cross-device truth.
- Flush payloads must be domain-scoped and idempotent.

## Page To SDK Map

| Page | Required clients | MVP notes |
|---|---|---|
| Today | `today`, `plan`, `study`, `rewards`, `sync` | No AI shortcut in public MVP. |
| Study | `study`, `plan` | Backend evaluates answers. |
| Wrong Words | `wrongWords`, `study` | Reinforcement starts study mode. |
| Reports | `reports` | Client renders charts only. |
| Plan | `plan` | Save/apply remain separate. |
| Account | `auth`, `sync` | WeChat login and email bind. |

## MVP Navigation

Recommended first tab shape:

- `today`
- `wrong-words`
- `reports`
- `account`

Secondary routes:

- `study`
- `plan`
- `email-bind`
- `merge-preview`

This keeps the first release focused on the learning loop and avoids surfacing
AI/community features before compliance work is complete.

## Verification Contract

The first implementation should prove these flows:

1. WeChat login -> `GET /v1/me` -> Today renders.
2. Today -> Study -> submit answer -> completion summary -> Today refresh.
3. Study incorrect/skipped -> Wrong Words list/detail reflects persisted truth.
4. Study completion -> Reports overview totals change.
5. Plan save -> apply to today -> Today shows updated targets.
6. Account -> email bind -> merge preview or bound state.

## Deferred From MVP SDK

- AI passage generation/history/import.
- Public reward image upload, moderation, and voting.
- App-side WeChat login.
- Admin review APIs.
- Payment or subscription surfaces.

## Open Implementation Choices

- Mini Program framework: native, Taro, or uni-app.
- Backend implementation language: TypeScript service, Rust service, or hybrid.
- Whether contract DTOs should be generated from shared schemas or handwritten
  with tests against Flutter/Rust examples.
