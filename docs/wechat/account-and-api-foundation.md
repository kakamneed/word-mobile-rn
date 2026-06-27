# WeChat Mini Program Account And API Foundation

Date: 2026-05-20
Status: Draft
Plan: `.vico/plans/active/2026-05-20-wechat-miniprogram-migration.md`

## Purpose

This document defines the first account, login, binding, and API security
contract for a WeChat Mini Program version of Word Mobile.

The Mini Program should feel native to WeChat users while still preserving the
current product rule: learning data belongs to one durable product identity, not
to whichever login method was used first.

## Scope

In scope:

- WeChat-first login for Mini Program users.
- Email account binding for existing Flutter/App users.
- A durable internal user identity that can later support App-side WeChat login.
- Guest/local-first semantics translated from the current Flutter auth manager.
- API security rules for a Mini Program client.

Out of scope for this slice:

- Concrete Mini Program UI implementation.
- Full study/session API payload definitions.
- AI feature approval, content filtering, or generation APIs.
- Public reward image upload and moderation implementation.

## Existing Reference Behavior

The Flutter app currently uses email/password plus OTP through the cloud auth
gateway. After a valid session exists, `AuthSessionManager` does more than mark
the user signed in:

- verifies the cloud data surface is accessible,
- reconciles the local data owner with the authenticated user,
- restores cloud data to local storage when local data is empty or reset,
- backfills local learning data to cloud when local data already exists,
- preserves local study when auth or network checks fail.

The Mini Program auth design must preserve those product semantics even though
the login credential changes from email-first to WeChat-first.

## Identity Model

Use `internal_user_id` as the durable product owner for all learning data.
Login methods attach to that user; they are not separate data owners.

Recommended tables or equivalent records:

### `users`

| Field | Purpose |
|---|---|
| `internal_user_id` | Primary product identity. |
| `created_at` | First account creation time. |
| `status` | `active`, `disabled`, `deleted_pending`, or `deleted`. |
| `primary_identity` | Display hint only, for example `wechat_mp` or `email`. |
| `last_login_at` | Last successful auth time. |

### `user_identities`

| Field | Purpose |
|---|---|
| `identity_id` | Stable row id. |
| `internal_user_id` | Owner. |
| `provider` | `wechat_mp`, `wechat_app`, or `email`. |
| `provider_subject` | Openid, unionid-scoped key, or email auth user id. |
| `provider_subject_secondary` | Optional unionid/openid pair when available. |
| `is_verified` | Whether the provider proof was completed. |
| `bound_at` | Time identity was attached. |
| `last_seen_at` | Last successful login/bind through this identity. |

Uniqueness rules:

- `(provider, provider_subject)` must be unique.
- A verified `email` identity can attach to only one `internal_user_id`.
- A verified `wechat_mp` openid can attach to only one `internal_user_id`.
- A verified `wechat_unionid` value should be unique across WeChat providers
  when available, but absence of unionid must not block Mini Program login.

Suggested provider subject mapping:

| Provider | `provider_subject` | Secondary data |
|---|---|---|
| `wechat_mp` | Mini Program `openid` | `unionid` when available. |
| `wechat_app` | App Open Platform `openid` | `unionid` when available. |
| `email` | First-party or Supabase-compatible user id | Normalized email as metadata. |

## Session Model

The Mini Program must not use broad cloud service credentials directly.

Recommended session shape:

- Mini Program calls backend auth endpoints.
- Backend issues a short-lived access token and a refresh token or refresh
  session handle.
- All user-owned API calls use `Authorization: Bearer <accessToken>`.
- Refresh tokens are stored in WeChat storage with the same caution as any
  bearer credential.
- Logout revokes the refresh token/session handle server-side.

Access token claims should include:

- `sub`: `internal_user_id`
- `sid`: session id
- `aud`: `word-mobile-miniprogram`
- `iat`, `exp`
- optional `identity_provider`: `wechat_mp` or `email`

Do not include openid, email, or session_key in client-readable claims unless a
feature explicitly needs it.

## WeChat Login Flow

Endpoint:

`POST /v1/auth/wechat/mp/login`

Client steps:

1. Call `wx.login()` and get `code`.
2. Send `code`, Mini Program app version, and device/client metadata to backend.
3. Store returned app session tokens.
4. Call `GET /v1/me` to hydrate profile and account readiness.

Request:

```json
{
  "code": "wx-login-code",
  "appVersion": "1.0.0",
  "client": {
    "platform": "wechat_mp",
    "sdkVersion": "optional",
    "language": "zh-CN"
  }
}
```

Backend steps:

1. Validate the request shape and rate limit by IP/session fingerprint.
2. Exchange `code` with WeChat server using backend-held app secret.
3. Validate WeChat response shape before using it.
4. Find existing `wechat_mp` identity by openid.
5. If found, issue a session for its `internal_user_id`.
6. If not found, create `users` and `user_identities` records.
7. Persist session metadata and return app tokens.

Response:

```json
{
  "user": {
    "internalUserId": "uuid",
    "primaryIdentity": "wechat_mp",
    "hasEmailBinding": false,
    "hasWechatBinding": true
  },
  "session": {
    "accessToken": "jwt-or-opaque-access-token",
    "refreshToken": "opaque-refresh-token",
    "expiresAt": "2026-05-20T12:00:00Z"
  },
  "accountState": {
    "phase": "signed_in_active",
    "needsBindDecision": false,
    "cloudDataState": "empty_or_ready"
  }
}
```

Error codes:

| Code | Meaning |
|---|---|
| `WECHAT_CODE_INVALID` | Code exchange failed or code expired. |
| `WECHAT_RESPONSE_INVALID` | WeChat response was missing required fields. |
| `AUTH_RATE_LIMITED` | Too many login attempts. |
| `ACCOUNT_DISABLED` | User exists but cannot sign in. |
| `AUTH_INTERNAL_ERROR` | Safe generic server failure. |

## Refresh And Logout

### `POST /v1/auth/refresh`

Request:

```json
{
  "refreshToken": "opaque-refresh-token"
}
```

Response:

```json
{
  "session": {
    "accessToken": "new-access-token",
    "refreshToken": "new-refresh-token",
    "expiresAt": "2026-05-20T13:00:00Z"
  }
}
```

Refresh behavior:

- Rotate refresh tokens when possible.
- Reject revoked, expired, or reused refresh tokens.
- Preserve local Mini Program study cache when refresh fails; do not delete
  learning data as a side effect of auth failure.

### `POST /v1/auth/logout`

Authenticated route. Revokes the current refresh/session handle.

Logout policy:

- Server session is revoked.
- Client auth tokens are cleared.
- Local learning cache is retained unless the user explicitly requests data
  deletion.

## Email Binding Flow

Email is not the default Mini Program login. It is the bridge for existing
App/Flutter users and future cross-platform account recovery.

### `POST /v1/account/email-bind/start`

Authenticated as a WeChat Mini Program user.

Request:

```json
{
  "email": "user@example.com"
}
```

Behavior:

- Normalize and validate email.
- Locate or create a pending email binding challenge.
- Send OTP using the existing email provider.
- Do not attach the email identity yet.

Response:

```json
{
  "challengeId": "uuid",
  "expiresAt": "2026-05-20T12:10:00Z",
  "delivery": "email"
}
```

### `POST /v1/account/email-bind/verify`

Authenticated as the current WeChat user.

Request:

```json
{
  "challengeId": "uuid",
  "token": "123456"
}
```

Backend behavior:

1. Validate OTP challenge.
2. Resolve the verified email identity.
3. If the email identity is unbound, attach it to the current
   `internal_user_id`.
4. If the email identity is already bound to this user, return success.
5. If the email identity belongs to a different `internal_user_id`, enter merge
   evaluation rather than silently moving identities.

Response when no conflict exists:

```json
{
  "bindingState": "bound",
  "user": {
    "internalUserId": "uuid",
    "hasEmailBinding": true,
    "hasWechatBinding": true
  }
}
```

Response when both sides have meaningful data:

```json
{
  "bindingState": "merge_decision_required",
  "mergePreview": {
    "currentWechatUser": {
      "hasStudyEvents": true,
      "hasPlanConfig": true
    },
    "emailUser": {
      "hasStudyEvents": true,
      "hasPlanConfig": true
    },
    "recommendedAction": "merge_with_confirmation"
  }
}
```

## Merge And Bind Rules

Reuse the domain posture from `docs/auth/guest-bind-flow.md` and
`docs/supabase/merge-strategy.md`.

First-pass rules:

| Scenario | Outcome |
|---|---|
| WeChat user has no meaningful data, email user has data | Attach WeChat identity to email user's `internal_user_id`. |
| WeChat user has data, email user has no meaningful data | Attach email identity to WeChat user's `internal_user_id`. |
| Both users have meaningful data | Require explicit merge confirmation. |
| Email identity already belongs to current user | Return idempotent success. |
| Email identity belongs to disabled/deleted account | Block and surface support-safe error. |

Domain merge posture:

| Domain | Rule |
|---|---|
| Plan configs | Explicit winner if both changed meaningfully. |
| Wordbook preferences | Last-write-wins with device/source attribution. |
| Study events | Append with idempotency keys. |
| Wrong-word state | Rebuild from merged study events where possible. |
| Reports | Rebuild from merged study truth. |
| Reward daily state | Keep one reward per user/date; if both exist, preserve the earlier claim and audit the conflict. |
| AI passages | Not part of MVP merge; merge by stable passage id later. |

Merge confirmation should summarize affected domains, not expose raw table rows.

## Guest And Local-First Semantics

Mini Program local cache is smaller than the Flutter SQLite runtime, but the
product rule remains:

- A user can open the Mini Program and begin lightweight local interaction
  before full cloud sync is healthy.
- Auth failure must not delete local cached study progress.
- In-progress study session truth is local until submitted/completed.
- Bind or merge interruption returns to a recoverable state.

Recommended local markers:

- `guestClientId`
- `lastInternalUserId`
- `pendingBindChallengeId`
- `pendingMergeDecisionId`
- `localStudyDrafts`
- `lastSuccessfulSnapshotAt`

Local markers are coordination metadata only. Server-side study events remain
the durable source for cross-device truth.

## API Security Rules

Rules for all Mini Program APIs:

- Use HTTPS only.
- Validate every request body at the backend boundary.
- Validate WeChat API responses before trusting fields.
- Never send WeChat `session_key`, app secret, service role key, or broad cloud
  credentials to the Mini Program.
- Scope every user-owned read/write by authenticated `internal_user_id`.
- Do not accept authoritative `internalUserId` from client request bodies.
- Use structured errors:

```json
{
  "error": {
    "code": "machine_readable_code",
    "message": "Safe human-readable message",
    "requestId": "optional-request-id"
  }
}
```

- Rate limit auth, binding, OTP verification, and upload routes.
- Log auth/bind/merge decisions with request id and actor, but do not log OTPs,
  session tokens, WeChat session keys, or raw secrets.

## Minimum API Surface For Phase 1

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/auth/wechat/mp/login` | Exchange WeChat code for product session. |
| `POST` | `/v1/auth/refresh` | Refresh app session. |
| `POST` | `/v1/auth/logout` | Revoke app session. |
| `GET` | `/v1/me` | Return profile, identities, and readiness state. |
| `POST` | `/v1/account/email-bind/start` | Send email binding OTP. |
| `POST` | `/v1/account/email-bind/verify` | Verify email and bind or request merge decision. |
| `GET` | `/v1/account/merge-preview/{mergeDecisionId}` | Inspect pending merge summary. |
| `POST` | `/v1/account/merge-confirm` | Confirm a merge decision. |

## Future App WeChat Login Compatibility

Leave room for App-side WeChat login from the start:

- Store Mini Program openid separately from App openid.
- Prefer unionid for cross-app matching when available.
- Never assume Mini Program openid equals App openid.
- Add `wechat_app` as a provider without changing learning data ownership.
- Let App email users bind WeChat later and land on the same
  `internal_user_id`.

## Phase 1 Exit Criteria

Phase 1 is ready for implementation when:

- The identity model uses `internal_user_id` as the data owner.
- WeChat login flow is specified from `wx.login` through backend session issue.
- Email binding and conflict behavior are explicit.
- Guest/local-first behavior preserves the current Flutter auth semantics.
- API security rules prevent direct client-side privileged cloud access.
