# Mini Program Technology Selection And Backend Reuse

Date: 2026-05-20
Status: Accepted
Plan: `.vico/plans/active/2026-05-20-wechat-miniprogram-migration.md`

## Decisions

- Mini Program frontend: Taro + React + TypeScript.
- Backend strategy: reuse existing backend/domain logic as much as practical.
- Rewrite strategy: any rewritten logic must live behind additive service or
  adapter boundaries and must not change Flutter-facing behavior.
- Flutter safety rule: existing Flutter app APIs, Rust bridge contracts, local
  SQLite behavior, and Supabase/cloud sync semantics must remain compatible
  unless a separate Flutter migration plan explicitly changes them.

## Rationale

Taro + TypeScript is the best fit for this migration because the Mini Program
needs a new frontend shell with complex business UI, typed SDK contracts, and
possible future H5/multi-mini-program reuse. It offers a more maintainable
component and state model than raw WXML/WXSS while avoiding a new App runtime
direction, because the native mobile App path remains Flutter.

Backend reuse is preferred because the current Flutter app already depends on
carefully shaped domain behavior:

- study session lifecycle,
- answer evaluation,
- plan save/apply semantics,
- wrong-word side effects,
- report aggregation,
- guest bind and cloud restore/backfill behavior.

The Mini Program should consume these semantics through HTTP APIs, not fork them
silently in client-side TypeScript.

## Implementation Boundaries

### Allowed

- Add `apps/wechat_miniprogram` for the Taro app.
- Add a Mini Program-specific TypeScript SDK under that app.
- Add backend routes that call existing domain services or Rust core wrappers.
- Add DTO mappers for Mini Program HTTP contracts.
- Add contract tests comparing Mini Program API responses against Flutter SDK
  reference shapes.
- Add new service code when no reusable backend/domain boundary exists yet.

### Not Allowed Without A Separate Plan

- Breaking `apps/flutter_mobile/lib/sdk/*` public shapes.
- Changing Rust bridge method names or response semantics for Flutter.
- Changing Flutter local data owner, sync restore, or backfill behavior.
- Moving AI, reward image upload, or leaderboard moderation into Flutter-only
  assumptions.
- Replacing shared domain behavior with Mini Program-only business rules.

## Backend Reuse Priority

1. Reuse existing Rust/domain logic server-side when it owns correctness.
2. If direct Rust reuse is not available, expose an additive backend service
   that preserves existing contract semantics.
3. If TypeScript rewrite is unavoidable, write golden/contract tests first
   against Flutter/Rust reference payloads.
4. Keep rewritten logic server-side unless it is purely deterministic UI helper
   logic, such as Croc BTI client-side preview.

## First Implementation Target

The next execution slice should scaffold:

```text
apps/wechat_miniprogram/
  config/
  src/
    app.config.ts
    app.ts
    pages/
    sdk/
```

with Taro + React + TypeScript, plus mock SDK fixtures for:

- auth/me,
- today,
- plan,
- study,
- wrong words,
- reports.

The first shell should not require live backend availability. It should make
the page contracts concrete while backend reuse work proceeds behind the SDK.

## Verification

- Taro project builds or typechecks locally.
- Mock SDK can drive Today, Study, Wrong Words, Reports, Plan, and Account
  placeholder screens.
- No existing Flutter/Rust files are changed for scaffolding.
- If backend routes are added, Flutter tests/analyzer remain unaffected.
