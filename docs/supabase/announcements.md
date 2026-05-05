# Cloud Announcements

Status: Implemented
Owner: Cloud integration layer + Flutter shell

## Purpose

Announcements let the operator publish short in-app messages without shipping a
new mobile build. The app reads active rows from Supabase and shows the highest
priority visible announcement in the Today screen.

## Cloud Contract

Source table: `public.announcements`

Key fields:

- `title` and `body`: user-facing message content
- `level`: `info`, `success`, `warning`, or `critical`
- `priority`: higher values sort first
- `is_active`: operator-controlled visibility switch
- `published_at`, `starts_at`, `ends_at`: active time window
- `target_platform`: `all`, `android`, or `ios`
- `min_app_version`, `max_app_version`: reserved for future version targeting

## Access Model

The mobile app uses the anon Supabase client. RLS allows selecting only rows
that are active, published, and inside their time window. There are no ordinary
client insert, update, or delete policies.

Operator writes should happen through Supabase Dashboard SQL Editor, the CLI, or
another trusted service-role controlled path.

## Publishing Flow

1. Ensure `supabase/cloud-setup.sql` or
   `supabase/migrations/202605050002_cloud_announcements.sql` has run in the
   target Supabase project.
2. Use `supabase/announcements-admin.sql` in SQL Editor to insert or hide
   announcements.
3. Open or refresh the app. The Today screen fetches visible announcements
   best-effort and continues normally when Supabase is unavailable.

## Non-Goals

- OS push notifications
- Cross-device read receipts
- Per-user targeting
- Admin UI inside the mobile app
