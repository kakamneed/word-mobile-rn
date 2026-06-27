# Word Mini Program API

Status: scaffold for scheme B.

This service is the first-party API layer between the Taro Mini Program and the
existing Supabase-backed product data. The Mini Program must not call Supabase
directly.

## Boundary

```text
Taro Mini Program
  -> server/miniprogram-api /v1/*
  -> Supabase service-role adapter
  -> existing product tables and shared domain semantics
```

## Rules

- Keep Supabase URL, service role keys, WeChat app secret, and token signing
  secrets server-side only.
- Resolve every request to `internal_user_id` before touching user-owned data.
- Map `internal_user_id` to the current Supabase owner id while the existing
  Flutter cloud schema remains `auth.users.id` based.
- Preserve Flutter behavior by adding this API layer and migrations only; do
  not mutate Flutter SDK or Rust bridge contracts for Mini Program needs.
- Exclude AI and public upload endpoints from the first public release API.

## Current Scaffold

- `src/routes.js` defines the route catalog consumed by contract checks.
- `src/auth/contracts.js` defines account/session DTO examples.
- `src/supabase/adapter.js` defines the adapter interface that will wrap
  Supabase service-role calls.
- `src/auth/service.js` implements the first testable account/session service
  for WeChat login, refresh, logout, `me`, and email binding decisions.
- `scripts/check-all.mjs` verifies route contracts, WeChat login/session
  lifecycle, and email binding/merge-preview behavior without external
  dependencies.

## Environment

Runtime implementation will require:

```text
SUPABASE_URL=
SUPABASE_SERVICE_ROLE_KEY=
WECHAT_MP_APP_ID=
WECHAT_MP_APP_SECRET=
MINIPROGRAM_ACCESS_TOKEN_SECRET=
MINIPROGRAM_REFRESH_TOKEN_SECRET=
```

Do not place these values in the Mini Program app.
