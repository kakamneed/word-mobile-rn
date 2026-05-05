# Supabase Local Contract

This directory contains the first migration-ready Supabase contract for the
Flutter + Rust + Supabase architecture migration.

## Local Setup

Install the Supabase CLI, then run from the repository root:

```powershell
supabase start
supabase db reset
```

This workspace also supports a repo-local CLI install under `.tools/` so tool
files and npm cache can stay on the D: drive:

```powershell
npm.cmd install --prefix D:\projects\word-mobile-rn\.tools\supabase-cli --cache D:\projects\word-mobile-rn\.npm-cache supabase@2.95.6
scripts\supabase-local.cmd start
scripts\supabase-local.cmd db reset
scripts\supabase-local.cmd status
```

Local Supabase still requires Docker Desktop to be installed and running. If
you want Docker data on D:, configure Docker Desktop's disk image / data
location before the first `start`.

The Flutter app should receive the local API URL and anon key via `dart-define`:

```powershell
flutter run `
  --dart-define=SUPABASE_URL=http://127.0.0.1:54321 `
  --dart-define=SUPABASE_ANON_KEY=<local-anon-key>
```

## Cloud Setup Without Docker

For cloud projects, Docker is not required. Create or open a Supabase project,
then either push migrations with an authenticated CLI session:

```powershell
scripts\supabase-local.cmd login
scripts\supabase-local.cmd link --project-ref <project-ref>
scripts\supabase-local.cmd db push
```

Or run the SQL manually in Dashboard SQL Editor:

```text
supabase/cloud-setup.sql
```

The Flutter cloud smoke script reads `.env.supabase.local`, which is ignored by
git:

```powershell
scripts\flutter-run-supabase-cloud.cmd
```

Required local env keys:

```text
SUPABASE_URL=https://<project-ref>.supabase.co
SUPABASE_ANON_KEY=<publishable-or-anon-key>
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
