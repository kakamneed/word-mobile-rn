# RLS Matrix

Status: Draft
Owner: Cloud integration layer
Phase: Slice 3 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines row-level security expectations for the initial Supabase table set.

It is meant to answer:

- who can read
- who can write
- who must be denied
- when service-role or function-mediated access is required

## Policy principles

- Default user-owned access should be scoped by `auth.uid() = user_id`.
- Service-role access must never be exposed to the client.
- Any elevated maintenance path should go through controlled functions or server-side operations.
- Projection/cache tables may intentionally have stricter write paths than source-event tables.

## Policy shape notes

- For ordinary tables, start from `deny all` and add explicit owner policies.
- Owner policies should validate both row visibility and inserted ownership, not only select access.
- `WITH CHECK (auth.uid() = user_id)` matters just as much as `USING (auth.uid() = user_id)`.
- Client tokens must never be able to impersonate a different `user_id` through payload fields alone.
- Tables that exist for repair, dead-letter, or cache rebuild work should default to function/service-only writes.

## Matrix

| Table | Select | Insert | Update | Delete | Elevated access notes |
|---|---|---|---|---|---|
| `profiles` | owner only | owner only on first creation | owner only | owner limited / usually function-mediated | service role may handle admin cleanup |
| `devices` | owner only | owner only | owner only for owned device rows | owner or function-mediated revoke | admin/service role may revoke |
| `plan_configs` | owner only | owner only | owner only | owner only | none by default |
| `wordbook_preferences` | owner only | owner only | owner only | owner only | none by default |
| `study_events` | owner only | owner only | usually no client update after insert | no client delete by default | service role may support repair/replay |
| `sync_cursors` | owner only | owner only | owner only | no ordinary delete | service role for maintenance okay |
| `sync_dead_letters` | owner limited or hidden from client | service/function only | service/function only | service/function only | likely not directly client-readable |
| `wrong_word_entries` | owner only | owner only or function-owned projection write | owner only or function-owned projection write | no ordinary delete by default | if projection is server-updated, use function/service role |
| `ai_passages` | owner only | owner only | owner only | owner only | if generated server-side, elevated create path may exist |
| `report_snapshots` | owner only | service/function preferred if derived | service/function preferred if derived | no ordinary delete by default | derived cache policy |
| `storage.objects` in `ai-passages` bucket | owner only via bucket policy | owner only or function-mediated upload | owner only for owned objects | owner only or cleanup function | bucket stays private; service role only for repair/admin flows |

## Example policy intent

### Owner-scoped table

Expected shape:

- `select`: `auth.uid() = user_id`
- `insert`: `auth.uid() = user_id`
- `update`: `auth.uid() = user_id`

### Service/function-mediated table

Expected shape:

- ordinary client has no direct insert/update/delete policy
- privileged function performs controlled writes
- any client-readable view is intentionally narrow

## Denial scenarios to test

- user A selecting rows owned by user B
- user A inserting rows with `user_id = user B`
- user A updating device rows belonging to user B
- user A writing fake study events for another user
- unauthenticated client reading protected tables

## Tables likely to need function-mediated access

- `sync_dead_letters`
- projection-maintained `wrong_word_entries` if written by server rebuilds
- derived `report_snapshots`
- account-deletion cleanup flows
- device revoke and account cleanup flows
- optional bucket repair / attachment cleanup flows

## Recommended test cases

### Positive cases

- owner reads own profile
- owner updates own plan config
- owner inserts own study event
- owner reads own AI passage

### Negative cases

- cross-user read
- cross-user insert
- cross-user update
- unauthenticated read

### Elevated cases

- service role rebuilds projection row
- deletion function clears user-owned rows
- device revoke path updates target device row
- service function removes owned bucket objects during account cleanup

## Open decisions

- whether projection tables are client-written or only function-written
- whether some operational tables should be hidden entirely from normal clients
- whether bucket uploads go directly through signed URLs or only through server-issued upload flows

## Exit criteria

- Every first-pass schema table has an intended RLS story.
- Negative and elevated test paths are listed before SQL policies are written.
