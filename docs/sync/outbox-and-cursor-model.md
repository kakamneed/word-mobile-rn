# Outbox And Cursor Model

Status: Draft
Owner: Rust sync layer
Phase: Slice 8 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the local sync state model required for deferred push/pull synchronization.

The primary goal is to ensure:

- local writes succeed first
- sync can resume after restart
- cloud transport failures do not erase local pending work

## Slice 8 first-pass decisions

- local persistence happens before cloud delivery
- sync queue state is Rust-owned local infrastructure, not Flutter UI state
- outbox rows survive restart
- cursor advances only after successful local apply
- poisoned records move to dead letter instead of infinite retry

## Core concepts

### Outbox

Definition:

- local queue of pending cloud-bound writes

Responsibilities:

- survive app restarts
- track per-record status
- support retry and dead-letter routing

### Cursor

Definition:

- durable marker for what remote changes have already been observed/applied

Responsibilities:

- support incremental pull
- only advance after successful apply

### Dead letter

Definition:

- local store for sync records that should not remain in infinite retry

Responsibilities:

- preserve debugging and supportability
- isolate poisoned records from healthy sync flow

## Suggested local structures

### `sync_outbox`

Suggested fields:

- `id`
- `domain`
- `payload_json`
- `idempotency_key`
- `created_at`
- `attempt_count`
- `last_attempt_at`
- `status`
- `lease_owner` optional
- `lease_expires_at` optional

Notes:

- `domain` keeps routing and failure isolation explicit
- lease fields prevent duplicate workers from pushing the same row at once

### `sync_cursor_state`

Suggested fields:

- `user_id`
- `device_id`
- `last_pushed_at`
- `last_pulled_cursor`
- `updated_at`

Notes:

- treat pushed and pulled progress as different dimensions
- one stale cursor must not imply local data loss

### `sync_dead_letter`

Suggested fields:

- `id`
- `domain`
- `payload_json`
- `idempotency_key`
- `failure_code`
- `failure_message`
- `created_at`
- `last_attempt_at`

## Status vocabulary

Recommended outbox statuses:

- `pending`
- `in_flight`
- `succeeded`
- `retryable_failure`
- `dead_lettered`

## Queue ownership

Rust sync layer owns:

- enqueue
- claim / lease
- push attempt
- retry scheduling
- dead-letter routing
- cursor advancement

Flutter may read:

- pending count
- high-level sync status
- retry affordance

Flutter may not own:

- raw queue mutation
- cursor mutation
- merge application

## Lifecycle

### Push lifecycle

1. local mutation succeeds
2. outbox row created
3. push worker claims row
4. cloud write attempted
5. result marks success, retryable failure, or dead letter

### Pull lifecycle

1. current cursor loaded
2. remote changes fetched after cursor
3. merge layer applies changes locally
4. cursor advances only after local apply succeeds

## Retry posture

- retry only retryable failures automatically
- preserve the same `idempotency_key` across retries
- use backoff policy in the sync layer, not per-feature ad hoc retry loops
- move repeated poison records to dead letter instead of permanent churn

## Failure isolation

- one failed outbox row must not block local study flow
- one failed domain must not automatically block unrelated healthy domains
- auth failure pauses protected sync but does not destroy queue state
- protocol drift should surface as explicit support state, not silent drop

## Anti-patterns

- volatile memory-only outbox
- cursor advance before merge success
- deleting failed outbox entries without audit trail
- one giant monolithic queue without domain metadata
- letting Flutter page code manage retry counters

## Exit criteria

- Outbox, cursor, and dead-letter concepts are explicit enough to drive implementation.
- Queue ownership is clear.
- Restart-safe sync continuity is explicit.
