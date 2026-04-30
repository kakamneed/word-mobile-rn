# Supabase Schema Outline

Status: Draft
Owner: Cloud integration layer
Phase: Slice 3 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document sketches the first-pass Supabase schema for identity, syncable product data, and derived projections.

It is intentionally a schema outline, not a migration-ready SQL file.

## Design principles

- Prefer user-scoped normalized tables over mirroring SQLite wholesale.
- Separate event tables from projection tables.
- Attach `user_id` to all user-owned cloud rows.
- Attach `device_id` to multi-device write paths.
- Introduce explicit idempotency keys for replayable writes.

## Proposed table groups

### Identity tables

#### `profiles`

Suggested columns:

- `user_id uuid primary key`
- `display_name text`
- `locale text`
- `created_at timestamptz`
- `updated_at timestamptz`

Notes:

- `user_id` should align with `auth.users.id`
- minimal profile only; avoid mixing app runtime data here

#### `devices`

Suggested columns:

- `device_id uuid primary key`
- `user_id uuid not null`
- `platform text`
- `device_label text null`
- `app_version text`
- `last_seen_at timestamptz`
- `revoked_at timestamptz null`
- `created_at timestamptz`

Notes:

- supports multiple active devices
- revocation should not require deleting historical event ownership

### Product preference tables

#### `plan_configs`

Suggested columns:

- `plan_id uuid primary key`
- `user_id uuid not null`
- `name text`
- `new_words_per_day int`
- `review_words_per_day int`
- `mixed_test_per_day int`
- `wrong_word_test_per_day int`
- `root_affix_per_day int null`
- `growth_rule_mode text null`
- `shared_growth_rule jsonb null`
- `growth_rules_by_mode jsonb null`
- `version bigint`
- `created_at timestamptz`
- `updated_at timestamptz`

Notes:

- keeps shared plan semantics
- do not store same-day today snapshot here

#### `wordbook_preferences`

Suggested columns:

- `user_id uuid not null`
- `wordbook_id bigint not null`
- `is_active boolean not null`
- `updated_at timestamptz`
- primary key `(user_id, wordbook_id)`

### Event tables

#### `study_events`

Suggested columns:

- `event_id uuid primary key`
- `user_id uuid not null`
- `device_id uuid not null`
- `session_id text not null`
- `event_type text not null`
- `payload_json jsonb not null`
- `occurred_at timestamptz not null`
- `ingested_at timestamptz not null`
- `idempotency_key text not null unique`

Notes:

- event types might later include:
  - `session_started`
  - `answer_submitted`
  - `session_completed`
  - `session_cancelled`

#### `sync_cursors`

Suggested columns:

- `user_id uuid not null`
- `device_id uuid not null`
- `last_pushed_event_at timestamptz null`
- `last_pulled_server_cursor text null`
- `updated_at timestamptz not null`
- primary key `(user_id, device_id)`

#### `sync_dead_letters`

Suggested columns:

- `dead_letter_id uuid primary key`
- `user_id uuid not null`
- `device_id uuid not null`
- `idempotency_key text not null`
- `payload_json jsonb not null`
- `error_code text not null`
- `created_at timestamptz not null`

### Projection tables

#### `wrong_word_entries`

Suggested columns:

- `user_id uuid not null`
- `entry_id bigint not null`
- `error_count int not null`
- `last_wrong_at timestamptz not null`
- `priority_score numeric not null`
- `projection_version bigint not null`
- `updated_at timestamptz not null`
- primary key `(user_id, entry_id)`

Notes:

- projection table, not the only source of truth

#### `ai_passages`

Suggested columns:

- `passage_id uuid primary key`
- `user_id uuid not null`
- `title text`
- `payload_json jsonb not null`
- `validation_status text not null`
- `generated_at timestamptz not null`
- `updated_at timestamptz not null`

#### `report_snapshots` optional

Suggested columns:

- `user_id uuid not null`
- `snapshot_date date not null`
- `payload_json jsonb not null`
- `updated_at timestamptz not null`
- primary key `(user_id, snapshot_date)`

Notes:

- only keep if cloud-side cached projections are useful
- do not rely on this as sole historical truth

## Storage buckets

### `ai-passages` optional private bucket

Intended use:

- optional attachment storage for AI passage artifacts that outgrow practical row size
- private user-scoped exports such as rendered passage assets or future audio attachments

Rules:

- keep bucket private by default
- metadata truth stays in `ai_passages`
- bucket objects are referenced by stable object keys owned by `user_id`
- do not make bucket presence required for ordinary study flow

Not for:

- primary study/session truth
- local runtime backups
- arbitrary client log upload

## Privileged functions

The first schema pass should assume a small allowlist of function-mediated operations instead of letting clients write every table shape directly.

### Candidate RPC / Edge function surfaces

#### `register_device`

Purpose:

- create or refresh a `devices` row for the current authenticated user
- normalize server-side device ownership checks

#### `revoke_device`

Purpose:

- mark a device row revoked without destructive history rewrite
- optionally require re-auth or recent session verification

#### `request_account_cleanup`

Purpose:

- orchestrate user-owned row cleanup for account deletion flows
- keep broad delete permissions out of the client

#### `rebuild_wrong_word_projection`

Purpose:

- rebuild `wrong_word_entries` from source events when repair is needed
- keep projection repair on the privileged side

#### `refresh_report_snapshot`

Purpose:

- rebuild derived `report_snapshots` if that cache exists
- avoid exposing aggregate repair primitives to ordinary clients

## Conventions for later SQL work

- prefer Postgres functions for transactional row updates close to the tables
- use Edge Functions only when external network calls, service-role fan-out, or admin orchestration is required
- never require privileged functions for offline-first local study execution
- every privileged surface should document:
  - caller
  - required auth context
  - touched tables
  - idempotency behavior
  - failure contract

## Constraints and conventions

- prefer `timestamptz` for all server-time fields
- use explicit unique keys for replayable writes
- avoid nullable ownership fields on user-scoped tables
- use text enum-like fields initially only if migration friction matters; otherwise prefer stricter enum strategy later

## Not included on purpose

- full mirror of every SQLite table
- local active-session persistence tables
- UI state or navigation state
- platform filesystem paths

## Open decisions

- whether `plan_id` should be UUID or carry a stable imported numeric identity
- whether wrong-word projections need a separate event table
- whether report projections should live in Postgres at all
- whether the optional `ai-passages` bucket ships in v1 or only after text-row limits appear

## Exit criteria

- Table families are stable enough for RLS design.
- Sync and auth documents can reference this outline without redefining tables.
