# Aliyun API Contract Draft

Date: 2026-05-05

This is the first mobile/admin API contract for the Aliyun replacement backend.
It intentionally replaces client-direct Supabase table/RPC access with backend
routes that own auth, authorization, validation, persistence, and audit logs.

## Common conventions

- Base mobile API: `/v1`
- Base admin API: `/admin`
- Request and response bodies are JSON unless stated otherwise.
- Mobile authenticated routes use `Authorization: Bearer <access_token>`.
- Admin authenticated routes use a separate admin access token.
- User-owned resources are scoped by the authenticated `user_id`; clients do
  not send authoritative `user_id` values except where explicitly noted for
  restore/debug payloads.
- Error responses use:

```json
{
  "error": {
    "code": "string_code",
    "message": "Human-readable safe message",
    "requestId": "optional-request-id"
  }
}
```

## Mobile auth

### `POST /v1/auth/signup`

Creates a first-party account, profile row, and initial session.

Request:

```json
{
  "email": "user@example.com",
  "password": "plain-text-over-https",
  "displayName": "optional",
  "locale": "zh-CN"
}
```

Response:

```json
{
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "displayName": "optional"
  },
  "session": {
    "accessToken": "jwt",
    "refreshToken": "opaque-token",
    "expiresAt": "iso-8601"
  }
}
```

### `POST /v1/auth/login`

Authenticates by email/password and creates a new session.

### `POST /v1/auth/refresh`

Rotates or refreshes a session from a refresh token.

### `POST /v1/auth/logout`

Revokes the current refresh/session token. Access token expiry remains short.

### `GET /v1/me`

Returns the current user, profile, and minimum cloud-readiness state required
by the Flutter startup auth checks.

## Profile and devices

### `PUT /v1/profile`

Updates display name and locale. Replaces direct `profiles.upsert` plus
Supabase user metadata writes.

### `POST /v1/devices/register`

Replaces `register_device` RPC.

Request:

```json
{
  "deviceId": "uuid",
  "platform": "android",
  "deviceLabel": "optional",
  "appVersion": "1.0.0"
}
```

### `POST /v1/devices/{deviceId}/revoke`

Replaces `revoke_device` RPC.

## Announcements

### `GET /v1/announcements/active`

Returns visible announcements for the current platform/version. Anonymous
access is allowed, but the response must include only public announcement
fields.

Query:

- `platform`: `android`, `ios`, or `all`
- `appVersion`: optional semantic app version
- `limit`: default `10`, max `50`

## Leaderboard

### `POST /v1/leaderboard/summary`

Authenticated route replacing `refresh_leaderboard_summary`.

Request:

```json
{
  "displayName": "optional",
  "period": "weekly",
  "periodStart": "2026-05-04",
  "metricSource": "reports",
  "totalQuestions": 120,
  "correctCount": 96,
  "mixedTestTotalQuestions": 20,
  "mixedTestCorrectCount": 18,
  "currentStreakDays": 7,
  "summaryKey": "reports-weekly-2026-05-04",
  "summaryAt": "iso-8601"
}
```

Validation must reject negative counters and correct counts greater than totals.

### `GET /v1/leaderboard`

Returns ranked leaderboard entries.

Query:

- `metric`: `totalQuestions`, `accuracy`, `mixedAccuracy`, or `currentStreak`
- `period`: `weekly`, `monthly`, or `all_time`
- `periodStart`: date, ignored for `all_time`
- `limit`: default `50`, max `100`

Ranking must match the current Supabase SQL intent:

- score desc,
- tie question count desc,
- `updated_at` asc,
- `user_id` asc.

## Sync

The first Aliyun backend should not expose direct table upserts. It should
accept domain payloads and perform server-side validation and user scoping.

### `POST /v1/sync/flush`

Uploads one or more pending local outbox items.

Request:

```json
{
  "items": [
    {
      "localItemId": 123,
      "domain": "plan_config",
      "idempotencyKey": "optional-stable-key",
      "payload": {}
    }
  ]
}
```

Supported domains:

- `plan_config`
- `wordbook_preferences`
- `report_snapshot`
- `study_word_points`
- `wrong_word_entries`
- `ai_passages`

Response maps each local item to success/failure so Flutter can call
`recordSyncResult`.

### `GET /v1/sync/snapshot`

Returns the cloud snapshot used by `restoreCloudDataToLocal`.

Response sections:

- `planConfig`
- `wordbookPreferences`
- `studyWordPoints`
- `reportSnapshots`
- `wrongWordEntries`

`ai_passages` restore is not part of the current Flutter restore flow.

## Account lifecycle

### `POST /v1/account/delete`

Starts or performs account deletion according to the product retention policy.
The route should require a recent authenticated session, revoke active sessions,
and remove or anonymize user-owned cloud data according to the final policy.

## Admin auth

### `POST /admin/auth/login`

Dedicated admin login. Admin identity must not be a normal user flag.

### `POST /admin/auth/logout`

Revokes the current admin session.

## Admin operations

Every admin mutation must write `admin_audit_logs`.

### `GET /admin/users`

Search and list users with pagination.

### `GET /admin/users/{userId}`

Returns user profile, account state, session summary, and cloud data counts.

### `POST /admin/users/{userId}/disable`

Disables a user account and optionally revokes sessions.

### `POST /admin/users/{userId}/revoke-sessions`

Revokes all active sessions for a user.

### `GET /admin/announcements`

Lists announcements, including inactive and archived rows.

### `POST /admin/announcements`

Creates an announcement and records admin audit metadata.

### `PUT /admin/announcements/{id}`

Updates an announcement and records before/after audit summary.

### `POST /admin/announcements/{id}/archive`

Archives or hides an announcement without deleting historical data.

### `GET /admin/leaderboard`

Read-only leaderboard inspection for support/debug.

### `GET /admin/users/{userId}/sync-debug`

Read-only projection summary for sync/report/wrong-word support.

### `GET /admin/audit-logs`

Searches admin audit logs by actor, action, target, and date range.

## Provider-neutral data routes

Migration tooling, not the mobile app, should own neutral export/import.
Expose these only as operator scripts or locked-down internal admin jobs:

- export core data to JSONL/CSV plus checksums,
- import neutral data into PostgreSQL,
- restore object inventory if object storage is introduced.
