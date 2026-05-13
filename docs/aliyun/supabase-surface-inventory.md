# Supabase Surface Inventory for Aliyun Replacement

Date: 2026-05-05

This inventory captures the current Supabase-backed surface that the Aliyun
backend must replace. It is intentionally implementation-facing: schema
conversion, API design, admin console scope, migration tooling, and release
cutover should all trace back to these surfaces.

## Current Supabase contract

Source files:

- `supabase/cloud-setup.sql`
- `supabase/migrations/202604230001_initial_mobile_cloud_schema.sql`
- `supabase/migrations/202604300001_lightweight_leaderboard.sql`
- `supabase/migrations/202605010001_daily_leaderboard_periods.sql`
- `supabase/migrations/202605020001_plan_config_user_upsert.sql`
- `supabase/migrations/202605020002_report_snapshot_write_policies.sql`
- `supabase/migrations/202605020003_study_word_points.sql`
- `supabase/migrations/202605050001_wrong_word_hint_fields.sql`
- `supabase/migrations/202605050002_cloud_announcements.sql`
- `supabase/announcements-admin.sql`

Supabase currently provides:

- PostgreSQL tables and functions.
- Supabase Auth identity, sessions, and user metadata.
- RLS via `auth.uid() = user_id`.
- RPC calls for device and leaderboard flows.
- A private `ai-passages` storage bucket scaffold.
- Dashboard/manual SQL operator path for announcements.

## Tables to replace

| Table | Current role | Owner model | Aliyun replacement notes |
| --- | --- | --- | --- |
| `auth.users` | Supabase-owned identity source | Supabase Auth | Replace with first-party `users` plus password/session tables. Preserve stable UUIDs where possible during migration. |
| `public.profiles` | User profile, display name, locale | User-owned | Reference first-party `users(id)`. Used by auth verification, leaderboard display, and profile settings. |
| `public.devices` | Account device registry | User-owned | Replace `register_device`/`revoke_device` RPC with authenticated API endpoints. |
| `public.plan_configs` | Cloud copy of learning plan settings | User-owned, one current row per user | Mobile sync upserts by `user_id`; restore reads latest row for user. |
| `public.wordbook_preferences` | Cloud copy of active wordbook selection | User-owned | Mobile sync upserts many rows by `(user_id, wordbook_id)`. |
| `public.study_events` | Append-only sync event model | User-owned with active device check | Present in schema but not currently used by Flutter direct sync client. Keep in target schema only if retained as future event pipeline. |
| `public.study_word_points` | Daily per-word learning aggregates | User-owned | Mobile sync upserts by `(user_id, point_date, entry_id, mode, question_type)` and restore reads ordered by date. |
| `public.sync_cursors` | Device sync cursor state | User-owned | Present in schema but not currently used by Flutter direct sync client. Keep or defer based on final sync API design. |
| `public.sync_dead_letters` | Failed sync payload capture | Service/function-owned by intent | Should become backend-only diagnostics, visible read-only in admin if needed. |
| `public.wrong_word_entries` | Wrong-word projection and AI hint fields | User-owned | Mobile sync upserts by `(user_id, entry_id)`. Includes `hint_text`, `hint_source`, `hint_updated_at`, `projection_version`. |
| `public.ai_passages` | AI passage projection rows | User-owned | Mobile sync upserts by stable `passage_id`; optional storage bucket may later hold large attachments. |
| `public.report_snapshots` | Daily report snapshots | User-owned | Mobile sync upserts by `(user_id, snapshot_date)` and restore reads ordered by date. |
| `public.leaderboard_stats` | Public leaderboard projection | Public read, owner write | Current primary key is `(user_id, period, period_start)`. Replace RPC with API endpoints or backend-owned SQL. |
| `public.announcements` | Public active app announcements | Public read, operator/admin write | Replace manual SQL path with admin console controls and audited admin API. |

## Functions and RPC

| Function/RPC | Current behavior | Replacement |
| --- | --- | --- |
| `public.set_updated_at()` | Trigger helper for `updated_at` columns | Keep equivalent trigger or update timestamps in backend writes. |
| `public.handle_new_user_profile()` | Creates profile after Supabase Auth user insert | Replace with profile creation inside signup/import flow. |
| `public.register_device(...)` | Authenticated device upsert scoped to `auth.uid()` | `POST /v1/devices/register` with JWT user identity. |
| `public.revoke_device(p_device_id)` | Authenticated revoke scoped to `auth.uid()` | `POST /v1/devices/{deviceId}/revoke` or `DELETE /v1/devices/{deviceId}`. |
| `public.refresh_leaderboard_summary(...)` | Authenticated leaderboard upsert for period and metrics | `POST /v1/leaderboard/summary` with server-side validation and authenticated user identity. |
| `public.get_leaderboard(...)` | Public/current-user leaderboard ranking by metric/period | `GET /v1/leaderboard?metric=&period=&periodStart=&limit=`. |

## Storage

| Bucket | Current status | Replacement |
| --- | --- | --- |
| `ai-passages` | Private bucket policy scaffold, path owner based on first folder segment. Current Flutter sync writes `ai_passages` rows, not bucket objects. | Defer OSS object migration unless shipped flows start writing bucket objects. Keep a provider-neutral object manifest format if OSS is introduced. |

## Flutter client call sites

### Auth and session

Files:

- `apps/flutter_mobile/lib/supabase/supabase_config.dart`
- `apps/flutter_mobile/lib/supabase/supabase_auth_service.dart`
- `apps/flutter_mobile/lib/supabase/auth_session_manager.dart`
- `apps/flutter_mobile/lib/features/auth_screen.dart`
- `apps/flutter_mobile/lib/features/account_drawer.dart`
- `apps/flutter_mobile/lib/features/mobile_root_shell.dart`
- `apps/flutter_mobile/lib/features/onboarding_flow.dart`

Current behavior:

- `SUPABASE_URL` and `SUPABASE_ANON_KEY` are compiled through Dart defines.
- `Supabase.initialize` boots the client.
- Email signup, email/password login, refresh, restore session, and sign out use Supabase Auth.
- Startup verification reads `profiles` and `plan_configs`.
- UI text and error mapping still names Supabase in some paths.

Replacement needs:

- Introduce an Aliyun API config, token storage, refresh flow, and session restore.
- Preserve the existing `AuthSessionManager` behavior shape for UI continuity.
- Replace Supabase-specific error text before production cutover.

### Profile settings

File:

- `apps/flutter_mobile/lib/features/profile_settings_screen.dart`

Current behavior:

- Local profile data is saved in `SharedPreferences`.
- If signed in, cloud profile display name is upserted into `profiles`.
- Supabase Auth user metadata is also updated with `display_name`.
- Avatar image remains local file storage only.

Replacement needs:

- `PUT /v1/profile` for display name.
- Decide whether auth user metadata remains separate or profile table is the only display-name source.
- Keep avatar image local unless a future cloud avatar feature is explicitly added.

### Announcements

Files:

- `apps/flutter_mobile/lib/supabase/announcement_service.dart`
- `apps/flutter_mobile/lib/features/today_shell_screen.dart`
- `supabase/announcements-admin.sql`

Current behavior:

- Client reads active announcements from `announcements`.
- Client filters time window, target platform, and dismissed IDs.
- Operator writes are manual SQL snippets.

Replacement needs:

- `GET /v1/announcements/active`.
- Admin console create/edit/archive controls.
- Admin audit logs for announcement mutations.

### Leaderboard

Files:

- `apps/flutter_mobile/lib/supabase/leaderboard_service.dart`
- `apps/flutter_mobile/lib/features/leaderboard_screen.dart`

Current behavior:

- Client calls `refresh_leaderboard_summary` with totals derived from reports.
- Client calls `get_leaderboard` by metric, period, period start, and limit.
- Metrics: `totalQuestions`, `accuracy`, `mixedAccuracy`, `currentStreak`.
- Periods: `weekly`, `monthly`, `all_time`.
- Current row model includes total questions, correct count, mixed-test totals,
  accuracy percentages, current streak, display name, and `is_current_user`.

Replacement needs:

- `POST /v1/leaderboard/summary`.
- `GET /v1/leaderboard`.
- Server-side ranking must match current SQL ordering:
  score desc, tie question count desc, `updated_at` asc, `user_id` asc.
- Admin console should allow read-only leaderboard inspection and safe refresh/debug visibility.

### Cloud sync

File:

- `apps/flutter_mobile/lib/sdk/sync_client.dart`

Current direct-write domains:

- `plan_config` -> upsert `plan_configs` on `user_id`.
- `wordbook_preferences` -> upsert `wordbook_preferences` on `(user_id, wordbook_id)`.
- `report_snapshot` -> upsert `report_snapshots` on `(user_id, snapshot_date)`.
- `study_word_points` -> upsert `study_word_points` on `(user_id, point_date, entry_id, mode, question_type)`.
- `wrong_word_entries` -> upsert `wrong_word_entries` on `(user_id, entry_id)`.
- `ai_passages` -> upsert `ai_passages` on `passage_id`.

Current restore reads:

- `plan_configs`
- `wordbook_preferences`
- `study_word_points`
- `report_snapshots`
- `wrong_word_entries`

Replacement needs:

- Replace direct table writes with an authenticated sync API.
- Keep payload idempotency and conflict rules explicit; do not rely on direct database `upsert` from the client.
- Add read-only admin support views for sync status and selected user-owned projections.

## Admin console initial scope

The first admin console should cover only operationally useful surfaces:

- Admin login with role checks separate from normal users.
- User lookup and user detail.
- Profile and account status inspection.
- Session revocation/account disable foundations.
- Announcement create/edit/archive.
- Leaderboard inspection.
- Read-only sync/report/wrong-word inspection for support.
- Audit log search for admin mutations.

Do not expose broad raw database editing in the admin UI.

## Portability and data safety requirements

Aliyun is the first target, not a permanent lock-in assumption.

- Core data should remain in PostgreSQL-compatible schema and migrations.
- Object storage, SMS/email, logging, metrics, and deployment configuration should use adapters or narrow provider wrappers.
- Migration tooling must support:
  - Supabase export to neutral files.
  - Neutral files to Aliyun PostgreSQL import.
  - Neutral files to a clean non-Aliyun PostgreSQL environment.
- Backups must include restore drills and integrity checks:
  - row counts by table,
  - key relationship checks,
  - selected checksums for user-owned tables,
  - object inventory comparison if OSS is introduced.

## Open decisions

- Whether `study_events`, `sync_cursors`, and `sync_dead_letters` ship in the first Aliyun backend or remain planned schema for a future event sync pipeline.
- How to handle existing Supabase Auth password hashes. If they cannot be migrated, the cutover needs a password reset or re-authentication path.
- Whether the first backend is a monolith containing mobile API plus admin API, or separate services behind one deployment.
- Whether production starts with Aliyun RDS immediately or with ECS-hosted PostgreSQL for a rehearsal-only environment.
