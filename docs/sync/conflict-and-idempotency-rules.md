# Conflict And Idempotency Rules

Status: Draft
Owner: Rust sync layer
Phase: Slice 8 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the minimum rules for:

- duplicate write protection
- conflict handling
- replay safety

## Slice 8 first-pass decisions

- idempotency is mandatory for replayable cloud writes
- conflicts are domain-specific
- append-only history is never resolved by overwrite
- stale or poisoned payloads must not spin forever invisibly

## Idempotency rules

### Study events

- every pushed event must carry an `idempotency_key`
- retries must reuse the same key
- cloud ingest must not double-count on retry

### Config writes

- plan and preference writes should use stable row identity plus version/update policy

### AI artifacts

- repeated save of the same artifact should not create silent duplicates when identity is known

## Conflict rules by domain

### Study events

- never solve by overwrite
- append + deduplicate

### Plan configs

- version-aware conflict policy

### Wordbook preferences

- simpler overwrite policy acceptable if source attribution is preserved

### Wrong-word projections

- do not blindly overwrite if source events are available

### Reports

- treat as derived; rebuild if needed instead of resolving aggregate conflicts as primary truth

## Failure classes

- duplicate delivery
- stale pull cursor
- partial batch success
- auth expiry mid-sync
- schema/protocol drift

## Operational posture

- duplicate delivery should be harmless under a reused `idempotency_key`
- stale cursor should trigger controlled recovery, not silent local reset
- partial batch success should preserve successful records and isolate failing ones
- auth expiry should pause protected cloud work without destroying queue state
- schema/protocol drift should route to explicit supportable failure handling

## Anti-patterns

- using only `updated_at` to resolve historical event conflicts
- silent duplication on retry
- infinite retry loops for poisoned payloads
- treating aggregate projections as interchangeable with source events

## Exit criteria

- Sync implementation can point here for conflict/idempotency policy instead of inventing rules ad hoc.
- Failure classes are explicit enough for retry and dead-letter behavior.
