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

Feature migrations added after the first setup can also be run directly from
`supabase/migrations/`. For operator-authored announcements, use:

```text
supabase/announcements-admin.sql
```

The reward image voting prototype needs this migration before ordinary users can
upload to the `reward-images` bucket or vote on cloud images:

```text
supabase/migrations/202605140001_reward_image_voting.sql
```

Cloud profile avatars need this migration before ordinary users can upload
their account avatar to the `avatars` bucket and before `get_leaderboard`
returns `avatar_url` for the normal learning leaderboard:

```text
supabase/migrations/202605140003_profile_avatars.sql
```

After running it in the Dashboard SQL Editor, run or keep the included
`select pg_notify('pgrst', 'reload schema');` statement so the new
`get_reward_image_vote_leaderboard` and `vote_reward_image` RPCs are visible to
PostgREST.

User-uploaded reward images are compressed by the Flutter client before cloud
upload. The target is a 540px WebP at quality 60, with retry qualities 50 and 40
when needed; JPEG at the same dimensions/qualities is used only if WebP
compression is unavailable on the device. The `reward-images` bucket migration
keeps a 512KB file-size ceiling as a server-side guard, so ordinary user uploads
should stay in the same order of magnitude as `assets/rewards/webp_q60_540/`.

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
- Public active announcements for in-app operator messages
- RLS policies for owner-only client access
- RLS policy for public read-only active announcements
- RPC helpers for device registration and revocation
- Private `ai-passages` bucket policy scaffold

## Verification Targets

- Unauthenticated clients cannot read protected tables.
- User A cannot select, insert, or update User B rows.
- Authenticated users can create/update their own profile, device, plan, and
  wordbook preference rows.
- `register_device` upserts only for `auth.uid()`.
- `sync_dead_letters` remains function/service-owned by default.
- Active announcements can be read by ordinary clients, but ordinary clients
  cannot insert, update, or delete announcements.
