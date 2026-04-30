# Domain Sync Strategy

Status: Draft
Owner: Rust sync layer + cloud integration layer
Phase: Slice 8 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the first-pass sync strategy for each major product domain.

Different domains must not be forced into one generic merge or write model.

## Slice 8 first-pass decisions

- sync is domain-specific, not one generic row copier
- study history prefers append-only event sync
- projections are secondary to source truth
- local-first correctness wins over cloud immediacy
- Flutter reads sync status but does not become sync owner

## Domain strategies

### Plan configs

Recommended approach:

- sync as versioned config rows
- resolve conflicts by explicit version policy, not event replay

Why:

- plans are authored configuration, not event history

### Wordbook preferences

Recommended approach:

- sync as lightweight preference rows
- conflict handling can be simpler than study history

Why:

- low-risk preference toggles do not justify heavy event treatment

### Study events

Recommended approach:

- append-only event sync
- deduplicate via idempotency key

Reason:

- this is the safest source of cross-device study truth

### Wrong-word state

Recommended approach:

- prefer source-event-driven rebuild where possible
- projection sync is secondary

Reason:

- ranking and priority can drift if projection rows are treated as sole truth

### Reports

Recommended approach:

- do not treat aggregate snapshots as primary sync truth
- rebuild from event history or use clearly derived caches

Reason:

- aggregates are derived and should not overwrite historical source truth

### AI passages

Recommended approach:

- sync as user-owned persisted artifacts
- keep stable ids and validation metadata

Reason:

- passages behave more like owned artifacts than ephemeral UI output

## Domain summary matrix

| Domain | Primary sync shape | Conflict posture |
|---|---|---|
| plan configs | versioned rows | explicit version policy |
| wordbook preferences | simple rows | lightweight overwrite policy |
| study events | append-only events | append + idempotent dedupe |
| wrong words | source-event or rebuild-driven | avoid trusting projection overwrite |
| reports | derived cache / rebuild | rebuild rather than aggregate conflict resolution |
| AI passages | artifact rows | stable-id merge |

## General rules

- local-first execution remains primary
- sync transport never decides learning correctness
- projection tables are not interchangeable with source-event tables
- queue failures never justify mutating local study truth

## Exit criteria

- Every major sync domain has an explicit first-pass strategy.
- Slice 8 implementation can route by domain without inventing rules ad hoc.
