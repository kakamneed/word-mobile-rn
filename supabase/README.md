# Supabase Local Contract

This directory contains the first migration-ready Supabase contract for the
Flutter + Rust + Supabase architecture migration.

## Local Setup

Install the Supabase CLI, then run from the repository root:

```powershell
supabase start
supabase db reset
```

The Flutter app should receive the local API URL and anon key via `dart-define`:

```powershell
flutter run `
  --dart-define=SUPABASE_URL=http://127.0.0.1:54321 `
  --dart-define=SUPABASE_ANON_KEY=<local-anon-key>
```

Do not commit service-role keys, provider secrets, access tokens, or refresh
tokens. Flutter feature code consumes typed account/sync state only.

## Included In The First Migration

- Auth-owned `profiles`
- Account-scoped `devices`
- Owner-scoped plan and wordbook preference tables
- Append-only `study_events`
- Sync cursor and dead-letter infrastructure
- Wrong-word, report, and AI projection tables
- RLS policies for owner-only client access
- RPC helpers for device registration and revocation
- Private `ai-passages` bucket policy scaffold

## Verification Targets

- Unauthenticated clients cannot read protected tables.
- User A cannot select, insert, or update User B rows.
- Authenticated users can create/update their own profile, device, plan, and
  wordbook preference rows.
- `register_device` upserts only for `auth.uid()`.
- `sync_dead_letters` remains function/service-owned by default.
