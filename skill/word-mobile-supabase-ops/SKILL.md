---
name: word-mobile-supabase-ops
description: Operate this project's existing Supabase cloud schema, migrations, PostgREST reloads, and direct remote SQL. Use for Word Mobile Supabase schema changes, cloud setup, RLS, comments, leaderboard, auth/cloud sync, or when remote Supabase execution is needed.
---

# Word Mobile Supabase Ops

Use this project-local skill only in `D:\projects\word-mobile-rn`.

## Project Facts

- Supabase project ref: `pmevdtgogsudnfgxjgiz`
- Cloud URL host: `pmevdtgogsudnfgxjgiz.supabase.co`
- Local env file: `.env.supabase.local`
- Never print keys, service-role tokens, access tokens, refresh tokens, or database passwords.
- If an access token was pasted into chat, recommend revoking it after the operation.

## Existing Local Layout

- Main schema setup: `supabase/cloud-setup.sql`
- CLI migrations: `supabase/migrations/*.sql`
- Supabase config: `supabase/config.toml`
- Project-local CLI wrapper: `scripts/supabase-local.cmd`
- CLI binary expected at `.tools/supabase-cli/node_modules/.bin/supabase.cmd`
- Flutter dart-define build scripts read `.env.supabase.local`.

## Before Cloud Work

1. Confirm `.env.supabase.local` exists and contains at least:
   - `SUPABASE_URL`
   - `SUPABASE_ANON_KEY`
   - usually `SUPABASE_SERVICE_ROLE_KEY`
2. Confirm `.env.supabase.local` is gitignored.
3. Confirm `scripts\supabase-local.cmd --version` works.
4. If remote SQL or migrations are needed, ask the user for a Supabase Access Token for this session:
   - Do not ask them to paste service role as a substitute for CLI migration work.
   - Set it only as a process env var for the command:
     ```powershell
     $env:SUPABASE_ACCESS_TOKEN='...'
     ```
   - Do not store it in files.

## Remote Direct Execution

Prefer linked remote `db query` for one-off DDL checks or applying a single migration file:

```powershell
$env:SUPABASE_ACCESS_TOKEN='...'
scripts\supabase-local.cmd link --project-ref pmevdtgogsudnfgxjgiz
scripts\supabase-local.cmd db query --linked "select table_name from information_schema.tables where table_schema = 'public' order by table_name;"
scripts\supabase-local.cmd db query --linked -f supabase\migrations\YYYYMMDDNNNN_name.sql
```

Use `db push` only when migration history is known to be aligned:

```powershell
$env:SUPABASE_ACCESS_TOKEN='...'
scripts\supabase-local.cmd migration list --linked
scripts\supabase-local.cmd db push --yes
```

If the sandbox blocks remote sockets, rerun the same command with escalated permissions.

## Migration History Trap

This project may have remote schema deployed manually while CLI migration history is empty. Do not blindly run `db push` if the migration list shows Local versions but empty Remote versions.

First query remote tables:

```powershell
$env:SUPABASE_ACCESS_TOKEN='...'
scripts\supabase-local.cmd db query --linked "select table_name from information_schema.tables where table_schema = 'public' order by table_name;"
```

If old tables already exist but migration history is empty:

1. Apply only the new migration SQL with `db query --linked -f`.
2. Verify the new table/policies.
3. Repair history so future `db push` is safe:
   ```powershell
   scripts\supabase-local.cmd migration repair --status applied <versions...>
   ```

Use `migration repair` only after verifying the remote schema matches the intended migrations closely enough. Record why repair was safe.

## PostgREST Verification

Every cloud schema change should end with:

```sql
notify pgrst, 'reload schema';
```

Then verify through REST without printing secrets:

```powershell
$envLines = Get-Content .env.supabase.local
$url = ($envLines | Where-Object { $_ -like 'SUPABASE_URL=*' } | Select-Object -First 1).Split('=',2)[1]
$key = ($envLines | Where-Object { $_ -like 'SUPABASE_SERVICE_ROLE_KEY=*' } | Select-Object -First 1).Split('=',2)[1]
$headers = @{apikey=$key; Authorization="Bearer $key"}
Invoke-WebRequest -Uri "$url/rest/v1/<table>?select=*&limit=1" -Headers $headers -Method Get -UseBasicParsing
```

Report only status codes and table names.

## RLS Pattern

For user-owned tables:

- `user_id uuid not null references public.profiles(user_id) on delete cascade` when embedding profile display names.
- `for select using (true)` only when product behavior is intentionally public-read, such as shared word comments.
- `insert/update/delete` should generally use `auth.uid() = user_id`.

After adding tables used by Flutter:

- Add migration under `supabase/migrations`.
- Also update `supabase/cloud-setup.sql` if full cloud bootstrap must include it.
- Reload PostgREST.
- Probe via REST for `200`, not `404`.

## Final Report Checklist

Include:

- Which migration/table was applied.
- Whether migration history was repaired.
- REST verification status.
- Any remaining manual action, especially token revocation.
