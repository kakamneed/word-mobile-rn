# Merge Strategy

Status: Draft
Owner: Rust sync layer + cloud integration layer
Phase: Slice 3 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how local and cloud data should meet when an account is introduced or multiple devices converge.

The goal is to avoid accidental data loss and avoid pretending that all domains can use the same merge rule.

## Principles

- Different domains need different merge strategies.
- Event streams should prefer append + idempotent ingest.
- Projection rows should not be blindly merged if they can be rebuilt.
- latest-timestamp-wins is not sufficient for all domains.
- local-only runtime truth must not be overwritten by cloud during active execution.

## Guardrails before any merge

- Never treat the whole SQLite database as one merge unit.
- Separate local-only runtime state from syncable product state first.
- Do not merge an in-progress local study session into cloud as if it were a finalized report.
- Rebuildable projections should be repaired from source truth, not reconciled row-by-row when avoidable.

## Domain strategies

### Plan configs

Candidate strategy:

- explicit versioned replace
- if both local and cloud changed, resolve by latest version or explicit bind decision

Why:

- plans are user-authored configs, not event streams
- same-day today snapshots remain outside the cloud merge target

### Wordbook preferences

Candidate strategy:

- last-write-wins with device attribution

Why:

- preference toggles are low-risk compared with answer event streams

### Study events

Candidate strategy:

- append-only ingest
- enforce `idempotency_key`

Why:

- events are historical truth and should not be overwritten by newer rows
- this is the preferred source for rebuilding wrong-word/report projections

### Wrong-word state

Candidate strategy:

- prefer rebuild from synced source events where possible
- do not use blind row overwrite as primary merge rule

Why:

- wrong-word score/ranking is derived and can drift if merged naively

### Reports

Candidate strategy:

- rebuild from study events, not from conflicting aggregate rows

Why:

- aggregate drift compounds quickly and is hard to audit

### AI passages

Candidate strategy:

- merge by stable `passage_id`
- treat passage record as owned artifact row

Why:

- passage history behaves more like document history than event projection

## Domains that do not merge directly

These stay local-first and should not participate in first-bind row reconciliation:

- active study session snapshot
- current question/progress cursor
- today snapshot cache
- transient runtime/bootstrap flags

## First-bind scenarios

### Local data exists, cloud empty

Preferred outcome:

- local becomes initial sync source

Notes:

- upload syncable product data only
- do not upload local runtime-only tables as primary truth

### Local empty, cloud exists

Preferred outcome:

- pull cloud state to local

### Local and cloud both populated

Preferred outcome:

- run domain-specific merge rules
- surface explicit user confirmation only where necessary

## First-bind decision matrix

| Domain | Local has data | Cloud has data | Recommended action |
|---|---|---|---|
| plan configs | yes | no | upload local |
| plan configs | no | yes | pull cloud |
| plan configs | yes | yes | compare version and choose explicit winner if conflict is meaningful |
| wordbook preferences | yes | yes | last-write-wins with device attribution |
| study events | yes | yes | append both sides with idempotency keys |
| wrong-word projection | yes | yes | rebuild from merged source events, avoid direct overwrite |
| reports snapshot/cache | yes | yes | rebuild from merged study truth |
| AI passages | yes | yes | merge by stable `passage_id`, resolve duplicate ids idempotently |

## Conflict handling posture

- Prefer deterministic automatic resolution for low-risk preference domains.
- Prefer append-plus-rebuild for event/projection domains.
- Prefer explicit user choice only for rare cases where both sides contain meaningful authored configuration and no safe automatic rule exists.
- If merge confidence is low, pause cloud sync and preserve both sides rather than guessing destructively.

## Anti-patterns

- global latest-updated-at wins for all domains
- overwriting study history with aggregate snapshots
- making wrong-word projections the only merge source
- replacing local in-progress session truth from cloud during app startup

## Exit criteria

- Each major syncable domain has an intended first-pass merge rule.
- Slice 8 can build sync mechanics without redefining merge semantics.
