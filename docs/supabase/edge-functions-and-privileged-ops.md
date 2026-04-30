# Edge Functions And Privileged Operations

Status: Draft
Owner: Cloud integration layer
Phase: Slice 3 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines which cloud operations stay client-direct, which require Postgres functions, and which justify Supabase Edge Functions or service-role execution.

The goal is to keep three boundaries clear:

- the mobile app never holds privileged credentials
- Supabase does not become the owner of learning truth
- Slice 4 and Slice 8 can integrate cloud operations without inventing new privilege rules

## Principles

- The client may use the user session token for ordinary owner-scoped reads and writes only.
- The client must never receive or embed a service-role key.
- Use Postgres functions when the work is close to the data and needs transactional privilege.
- Use Edge Functions only when orchestration, external APIs, or privileged multi-step flows justify them.
- Privileged operations must remain non-blocking for local-first study execution whenever possible.

## Client-direct operations

These may be performed with normal authenticated client access under RLS:

- read and update `profiles`
- read and update `plan_configs`
- read and update `wordbook_preferences`
- insert owner-scoped `study_events`
- read owner-scoped `wrong_word_entries`
- read and write owner-scoped `ai_passages` rows when no elevated generation path is needed
- read owner-scoped `sync_cursors`

## Function-mediated operations

These should prefer SQL functions or RPC-style server entrypoints:

### Device registration and revoke

Why:

- centralize ownership validation
- avoid broad direct update/delete permissions on `devices`

Likely surfaces:

- `register_device`
- `revoke_device`

### Projection repair

Why:

- wrong-word and report projections are derived and may need controlled rebuilds
- ordinary clients should not hold broad repair rights

Likely surfaces:

- `rebuild_wrong_word_projection`
- `refresh_report_snapshot`

### Account cleanup

Why:

- account deletion touches multiple tables and optional storage objects
- this is exactly the kind of operation that should not be modeled as open-ended client deletes

Likely surfaces:

- `request_account_cleanup`
- `finalize_account_cleanup`

## Edge Function candidates

Edge Functions are justified only when plain RLS + SQL functions are not enough.

### Account deletion orchestration

Use when:

- cleanup spans tables, buckets, and auth admin APIs
- a server-controlled long-running or auditable flow is needed

### AI generation or validation with protected provider credentials

Use when:

- external model/provider credentials must stay server-side
- generated artifacts need normalization before persisting

### Signed upload issuance for optional bucket objects

Use when:

- the app needs a time-limited upload token for private bucket objects
- direct bucket writes should stay tightly scoped

## Service-role policy

- Service-role credentials live only in trusted server environments.
- Service-role use must be limited to:
  - admin cleanup
  - projection repair
  - bucket maintenance
  - external provider fan-out
- Service-role execution must never be routed through Flutter, React Native, or Rust mobile runtime code.

## Storage bucket policy

If `ai-passages` or a future private bucket is introduced:

- keep the bucket private
- scope object ownership to `user_id`
- store durable metadata in Postgres rows first
- use signed URLs or server-issued upload flows for client object transfer
- never store study/session truth only in bucket objects

## Do not use privileged cloud code for

- routine today reads
- routine study progression
- local session recovery
- wrong-word/report truth generation during offline execution
- anything that would make login required for core study flow

## Downstream handoff

- Slice 4 can treat cloud privilege as an adapter concern, not a Flutter UI concern.
- Slice 5 can build auth UX without inventing service-role semantics.
- Slice 8 can design sync flows knowing which repair and maintenance paths are privileged.

## Exit criteria

- privileged operations have an allowlist
- client-direct vs privileged responsibilities are explicit
- service-role exposure risks are addressed before implementation
