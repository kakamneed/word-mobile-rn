# Aliyun Target Architecture

Date: 2026-05-05

This document defines the first target architecture for replacing Supabase with
an Aliyun-hosted backend while keeping the design portable enough to move to a
different cloud later.

## Deployment shape

Production should use:

- ECS for the independent `D:\projects\word-admin` application server and admin web deployment.
- RDS PostgreSQL for the primary database.
- OSS only for object data that is actually needed by shipped features.
- HTTPS on `api.<domain>` for mobile API traffic.
- HTTPS on `admin.<domain>` for the admin console.
- Optional `app.<domain>` or `www.<domain>` for public pages, privacy policy,
  user agreement, and ICP/APP filing support.

Local development should not require buying cloud resources:

- Backend API runs locally from `D:\projects\word-admin`.
- Admin web runs locally from `D:\projects\word-admin`.
- PostgreSQL runs locally or in Docker.
- Provider adapters use local filesystem/null implementations where practical.

Staging should mirror production decisions closely enough to rehearse:

- Aliyun ECS plus RDS PostgreSQL when cloud rehearsal starts.
- Same migration scripts as production.
- Same backup and restore procedure as production.
- Separate domains and secrets from production.

## Service boundaries

The first implementation can be a monolith, but the boundaries must stay clear:

- Mobile API routes handle app auth, profile, sync, leaderboard, announcements,
  and account operations.
- Admin API routes handle operator login, user lookup, announcements,
  leaderboard inspection, support/debug views, and audited admin mutations.
- Domain services own business rules and must not import Aliyun SDKs directly.
- Provider adapters wrap object storage, SMS/email, logs, metrics, and future
  cloud-specific services.

The mobile app must never connect directly to PostgreSQL or OSS with broad
credentials.

## Database

Use PostgreSQL-compatible schema and migrations.

Initial database modules:

- `users`
- `user_sessions`
- `refresh_tokens` or equivalent session table
- `admin_users`
- `admin_sessions`
- `admin_audit_logs`
- `profiles`
- `devices`
- `plan_configs`
- `wordbook_preferences`
- `study_word_points`
- `wrong_word_entries`
- `ai_passages`
- `report_snapshots`
- `leaderboard_stats`
- `announcements`

Tables from the Supabase schema that may be deferred or reshaped:

- `study_events`
- `sync_cursors`
- `sync_dead_letters`

These should be explicitly accepted or deferred during API design rather than
silently dropped.

## Auth and authorization

Normal users:

- Email/password auth in v1.
- Password hashes stored with a modern adaptive hashing algorithm.
- Short-lived access tokens.
- Refresh token/session rotation or revocation support.
- Account disable and session revocation support for admin operations.

Admins:

- Dedicated admin identity, not ordinary user accounts with a flag.
- Admin sessions separate from mobile user sessions.
- Role checks before admin routes.
- All admin mutations write audit logs.

Authorization:

- Replace Supabase RLS with backend-side checks.
- All user-owned reads and writes must scope by authenticated `user_id`.
- Public surfaces such as announcements and leaderboard reads must return only
  fields intended for public exposure.

## API surfaces

Minimum mobile API:

- `POST /v1/auth/signup`
- `POST /v1/auth/login`
- `POST /v1/auth/refresh`
- `POST /v1/auth/logout`
- `GET /v1/me`
- `PUT /v1/profile`
- `POST /v1/devices/register`
- `POST /v1/devices/{deviceId}/revoke`
- `GET /v1/announcements/active`
- `POST /v1/leaderboard/summary`
- `GET /v1/leaderboard`
- `POST /v1/sync/flush` or domain-specific upsert endpoints
- `GET /v1/sync/snapshot`
- `POST /v1/account/delete` or equivalent foundation

Minimum admin API:

- `POST /admin/auth/login`
- `POST /admin/auth/logout`
- `GET /admin/users`
- `GET /admin/users/{userId}`
- `POST /admin/users/{userId}/disable`
- `POST /admin/users/{userId}/revoke-sessions`
- `GET /admin/announcements`
- `POST /admin/announcements`
- `PUT /admin/announcements/{id}`
- `POST /admin/announcements/{id}/archive`
- `GET /admin/leaderboard`
- `GET /admin/users/{userId}/sync-debug`
- `GET /admin/audit-logs`

## Admin console v1

The admin UI should stay small but extensible:

- Login screen.
- User search/list.
- User detail with profile, account status, session status, and cloud data
  counts.
- Announcement list and editor.
- Leaderboard inspection page.
- Read-only support/debug page for sync/report/wrong-word projections.
- Audit log page.

No raw SQL console or arbitrary database editor should ship in the admin UI.

## Provider adapters

Keep these behind interfaces or narrow modules:

- Object storage: Aliyun OSS now, S3-compatible or filesystem later.
- Email/SMS: Aliyun or another provider later.
- Logging: local structured logs now, Aliyun SLS or another sink later.
- Metrics: local/no-op now, cloud metrics later.
- Deployment config: environment variables and migration scripts, not hardcoded
  cloud resource names.

Core services should depend on these adapters, not directly on Aliyun SDKs.

## Backup, restore, and neutral export

Production readiness requires:

- Automated PostgreSQL backups.
- Manual on-demand backup before migration/cutover.
- Restore rehearsal into staging.
- Row counts by table after restore.
- Foreign-key/key relationship checks.
- Selected checksums for user-owned tables.
- Neutral export files for core entities.
- Object inventory export if OSS is introduced.

Neutral export should be able to seed:

- Aliyun PostgreSQL.
- A clean local PostgreSQL database.
- Another cloud-hosted PostgreSQL database.

## Cutover implications

The final migration should not depend on a hidden proxy or dual-write bridge.

Cutover needs:

- A frozen or read-only Supabase write window.
- Supabase export.
- Import into Aliyun PostgreSQL.
- Validation report.
- Aliyun-targeted mobile release.
- Monitoring of auth, sync, leaderboard, announcements, and crashes.
- Rollback decision window while Supabase remains available for comparison.

## Open implementation choice

The repository does not yet contain a backend application. A future execution
slice should choose and scaffold the backend stack. Reasonable options include:

- TypeScript backend plus TypeScript admin web, reusing `packages/contracts`
  patterns.
- Rust backend if sharing Rust domain logic server-side becomes more valuable.

For the next slice, the important constraint is not the language choice; it is
that the backend owns auth, permissions, migrations, backup/restore, admin
auditability, and provider adapters.
