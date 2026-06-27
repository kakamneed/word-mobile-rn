# Croc BTI Mini Program Port

Date: 2026-05-20
Status: Draft
Plan: `.vico/plans/active/2026-05-20-wechat-miniprogram-migration.md`

## Purpose

This document defines how to port the Flutter Croc BTI learning personality flow
to the WeChat Mini Program without depending on Flutter widgets, shared
preferences, or the Rust bridge.

Croc BTI is suitable for an early Mini Program release because it is a
deterministic questionnaire and plan-personalization feature, not an AI or
user-generated-content feature.

## Flutter Reference

Reference files:

- `apps/flutter_mobile/lib/features/croc_bti_screen.dart`
- `apps/flutter_mobile/lib/features/croc_bti_model.dart`
- `apps/flutter_mobile/lib/sdk/croc_bti_client.dart`
- `apps/flutter_mobile/lib/sdk/plan_client.dart`

Reference behavior:

- Load saved Croc BTI answers by user scope.
- Load saved daily learning minutes.
- Restore an existing server/Rust profile when available.
- Let the user answer all Croc BTI questions.
- Evaluate answers into a result code, title, summary, advice, axis scores,
  learning weights, plan input, and question-type weights.
- Let the user adjust daily minutes and generated plan/question-type weights.
- Save profile.
- Save plan.
- Apply saved plan to today.
- Flush pending cloud sync when available.

## Mini Program Ownership

The Mini Program should own:

- questionnaire UI,
- answer draft state,
- local guest cache,
- result preview,
- client-side deterministic evaluation,
- user edits before applying.

Backend should own:

- authenticated profile persistence,
- plan persistence,
- apply-to-today side effects,
- cross-device restore,
- sync/audit fields.

## Data Model

### Local Draft

Store locally under a user-scoped key:

```typescript
interface CrocBtiLocalDraft {
  scope: 'guest' | string; // internalUserId when signed in
  answers: Record<string, number>;
  dailyLearningMinutes: number;
  updatedAt: string;
}
```

Local storage key:

```text
croc_bti_saved_answers_v1.<scope>
croc_bti_daily_minutes_v1.<scope>
```

This mirrors the Flutter scoped-key behavior and allows a guest user to take the
test before login.

### Saved Profile

```typescript
interface CrocBtiProfile {
  resultCode: string;
  title: string;
  summary: string;
  advice: string;
  answers: Record<string, number>;
  axisScores: Record<string, {
    score: number;
    selectedTrait: string;
    strength: string;
  }>;
  weights: Record<string, number>;
  planInput: Record<string, number>;
  questionTypeWeightsByMode: Record<string, Record<string, number>>;
  dailyLearningMinutes: number;
  growthRuleEnabled: boolean;
  source: 'croc_bti';
  version: number;
  evaluatedAt: string;
}
```

## SDK Methods

Extend `MiniProgramSdk` with:

```typescript
interface CrocBtiClient {
  getProfile(): Promise<CrocBtiProfile | null>;
  saveProfile(input: CrocBtiProfile): Promise<CrocBtiProfile>;
}
```

Backend endpoints:

- `GET /v1/croc-bti/profile`
- `PUT /v1/croc-bti/profile`

Plan application still uses existing plan endpoints:

- `PUT /v1/plan/active`
- `POST /v1/plan/apply-to-today`

## Evaluation Port

Port these Flutter model concepts to TypeScript:

- `crocBtiQuestions`
- axis trait mapping
- result profile mapping
- `evaluateCrocBti(answers)`
- `hasCompleteCrocBtiAnswers(answers)`
- `crocBtiPlanInputForDailyMinutes(weights, minutes)`
- `crocBtiPlanInputFor(plan, result, overrides)`
- question-type weight rebalance logic

Porting rules:

- Keep result codes stable.
- Keep question ids stable.
- Keep output field names stable.
- Add golden tests with known answer sets before changing wording or weights.

## User Flow

### First Open

1. Load local draft by current scope.
2. Load remote profile if signed in.
3. Prefer local draft when it has answers; otherwise restore profile answers.
4. Load active plan for default growth rule and existing plan values.

### Answer Questionnaire

1. User answers questions.
2. Save local draft after each answer or page transition.
3. Enable result only when all questions are answered.

### Preview Result

1. Evaluate result locally.
2. Show title, summary, axis scores, advice, and generated plan.
3. Let user adjust daily minutes.
4. Let user adjust generated plan values and question-type weights.
5. Let user toggle growth rule.

### Apply Result

1. Save local draft.
2. Save Croc BTI profile to backend.
3. Save plan through `plan.savePlan`.
4. Apply saved plan to today.
5. Refresh Today data.

## Verification

Required tests:

- Known complete answer set returns the same result code as Flutter.
- Daily minutes clamp to 10-240.
- Plan input generation changes when daily minutes change.
- Question-type weights remain bounded 0-100 and rebalance consistently.
- Guest draft and signed-in draft use separate storage scopes.
- Apply flow calls save profile, save plan, and apply-to-today in order.

Manual smoke:

1. Open Croc BTI as guest.
2. Answer partial test.
3. Close and reopen; answers restore.
4. Complete test and preview result.
5. Sign in; verify guest draft can still be applied or migrated by policy.
6. Apply result; Today reflects updated plan.

## MVP Decision

Recommended: include Croc BTI in the first Mini Program release if Phase 2 core
learning MVP is stable. It is review-safe compared with AI/community features
and gives the Mini Program a distinct onboarding hook.
