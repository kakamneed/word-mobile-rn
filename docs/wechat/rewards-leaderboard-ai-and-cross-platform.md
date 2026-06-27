# Mini Program Rewards, Leaderboard, AI, And Cross-Platform Account Notes

Date: 2026-05-20
Status: Draft
Plan: `.vico/plans/active/2026-05-20-wechat-miniprogram-migration.md`

## Purpose

This document captures the migration decisions for the higher-risk or
post-foundation Mini Program surfaces:

- daily rewards,
- lightweight leaderboards,
- reward image community features,
- AI feature readiness,
- future App-side WeChat login and account unification.

## Rewards

### MVP Scope

Ship only deterministic daily reward draw in the first Mini Program release.

MVP APIs:

- `GET /v1/rewards/today`
- `POST /v1/rewards/today/draw`

Rules:

- One reward per `internal_user_id` per local product date.
- Guest rewards may be cached locally, but signed-in rewards are server-owned.
- Reward asset metadata should reference CDN/lazy package URLs.
- Do not put all reward image assets in the Mini Program main package.

Recommended response:

```typescript
interface TodayRewardState {
  todayDate: string;
  rewardId?: string;
  claimedAt?: string;
  asset?: {
    rewardId: string;
    imageUrl: string;
    width?: number;
    height?: number;
    mimeType?: string;
  };
}
```

### Deferred Reward Image Upload

Do not include public user-uploaded reward images in MVP unless the moderation
pipeline is ready.

Required before public upload/vote:

- upload entitlement rules,
- image size/type validation,
- object storage policy,
- moderation status: `pending`, `approved`, `rejected`, `withdrawn`,
- moderation reason,
- user withdrawal path,
- report/complaint path,
- admin review tooling,
- audit logs.

## Leaderboard

### MVP Scope

Ship a cloud-only leaderboard after reports are stable. Do not reproduce local
SQLite fallback behavior in the Mini Program.

APIs:

- `POST /v1/leaderboard/summary`
- `GET /v1/leaderboard`

Metrics:

- `totalQuestions`
- `accuracy`
- `mixedAccuracy`
- `currentStreak`

Periods:

- `weekly`
- `monthly`
- `all_time`

Rules:

- Leaderboard summaries should be derived from server-side report/study data
  whenever possible.
- If a client submits report-derived counters, backend must validate bounds and
  idempotency.
- Public rows must expose only approved display fields.
- Avatar/display-name use must follow WeChat authorization and privacy rules.

## AI Readiness

AI must stay out of the first public Mini Program release unless all compliance
and safety gates are complete.

Feature candidates:

- generated passage from today's wrong words,
- AI wrong-word import,
- AI hint suggestions,
- chat/workbench.

Required before implementation:

- Mini Program service category alignment for the AI capability.
- Regulatory filing/registration review for model/provider usage.
- Public model/provider disclosure where required.
- AI-generated-content labeling in the UI.
- Privacy policy update for user text, files, images, and generated output.
- Input moderation.
- Output moderation.
- Rate limits and abuse prevention.
- Audit logs for generation requests.
- User-visible failure states.

MVP rule:

- Do not show AI tabs, shortcuts, or hidden entry points in the public first
  release.
- Wrong-word hint suggestions should be deterministic or empty until AI is
  explicitly enabled.

## Cross-Platform Account Unification

The Mini Program account design should support future App-side WeChat login.

Rules:

- `internal_user_id` remains the learning data owner.
- Mini Program openid and App openid are separate provider subjects.
- Use unionid for cross-application matching when available.
- Never assume Mini Program openid equals App openid.
- Email login remains a valid App credential during migration.

Future App-side options:

1. WeChat Open Platform App login.
2. QR-based binding/confirmation through Mini Program or web.

Recommended rollout:

1. Mini Program: WeChat login first.
2. Mini Program: email binding for old App users.
3. Backend: identity table supports `wechat_mp`, `wechat_app`, and `email`.
4. Flutter App: add "Bind WeChat" in account settings.
5. Flutter App: optionally add direct WeChat login once Open Platform setup is
   approved.

Conflict handling:

- email-first user binds WeChat: attach WeChat identity to existing account.
- WeChat-first user binds empty email: attach email identity to WeChat account.
- WeChat-first user binds populated email: show merge preview and require
  explicit confirmation when both sides contain meaningful learning data.

## Verification

Before these surfaces are called implemented:

- daily reward draw is idempotent per user/date,
- leaderboard rankings match known report histories,
- public leaderboard fields exclude private identifiers,
- reward image upload is disabled or moderation-gated,
- AI entry points are absent from MVP builds,
- identity records can represent email plus Mini Program plus future App WeChat
  providers for one `internal_user_id`.
