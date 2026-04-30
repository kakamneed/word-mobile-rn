# Cloud Data Classification

Status: Draft
Owner: Rust shared core + cloud integration layer
Phase: Slice 3 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document classifies product data before any Supabase schema is finalized.

The goal is to prevent a common architectural mistake:

- treating all existing local data as cloud truth
- or letting identity/storage concerns pull core learning logic out of Rust

## Guiding rules

- SQLite remains the on-device source of truth for offline execution.
- Supabase owns identity, cloud persistence, and sync landing zones.
- Rust owns learning semantics.
- Rebuildable projections should not be treated as irreplaceable truth unless there is a strong operational reason.

## Classification layers

### 1. Local-only truth

Definition:

- data that must exist and remain usable offline
- data whose immediate correctness cannot depend on cloud availability

Examples:

- active study session snapshot
- current question and progress
- persisted today snapshot
- local runtime migration state
- local engine/version compatibility markers

Cloud policy:

- do not make cloud the primary owner
- may later sync supporting events or backups, but execution truth stays local-first

### 2. Cloud-backed identity truth

Definition:

- data whose authority comes from account identity, not from local device state

Examples:

- user id
- auth identity records
- profile basics
- account lifecycle status
- device registration ownership

Cloud policy:

- owned by Supabase Auth and user-scoped profile/device tables

### 3. Syncable product data

Definition:

- product data that benefits from cross-device portability and can be synchronized safely

Examples:

- plan configurations
- wordbook activation preferences
- study event stream
- AI passage history
- wrong-word projection or wrong-word source events
- sync checkpoints

Cloud policy:

- store in Postgres tables
- synchronize via Rust-owned sync rules

### 4. Rebuildable projections

Definition:

- data that can be reconstructed from events and persisted truth

Examples:

- report aggregates
- streak summaries
- recommendation hints
- derived wrong-word ranking

Cloud policy:

- avoid treating these as primary truth unless needed for performance or ops
- if stored, mark them as derived/cache-like

## Data-by-domain classification

### Identity

| Domain data | Classification | Notes |
|---|---|---|
| `user_id` | Cloud-backed identity truth | Supabase authority |
| auth session state | Cloud-backed identity truth | Client consumes, cloud owns |
| device registration | Cloud-backed identity truth | Device belongs to account scope |

### Plan

| Domain data | Classification | Notes |
|---|---|---|
| active plan config | Syncable product data | Can sync across devices |
| same-day today snapshot | Local-only truth | Execution hint for the day; not cloud primary truth |
| growth rule config | Syncable product data | Stored with plan config |

### Study

| Domain data | Classification | Notes |
|---|---|---|
| active in-progress session | Local-only truth | Must survive offline and restart locally |
| answer results / study events | Syncable product data | Prefer event stream upload |
| session summary | Rebuildable projection | Can be derived from results/events |

### Wrong words

| Domain data | Classification | Notes |
|---|---|---|
| wrong-word source events | Syncable product data | Safest long-term source |
| wrong-word current projection | Syncable product data or rebuildable projection | Decide based on UX/perf needs |
| risk score | Rebuildable projection | Avoid making this the only source of truth |

### Reports

| Domain data | Classification | Notes |
|---|---|---|
| daily report aggregates | Rebuildable projection | Can be re-aggregated |
| streak info | Rebuildable projection | Depends on historical study truth |

### AI

| Domain data | Classification | Notes |
|---|---|---|
| provider credentials | Cloud-backed identity/admin truth or secure local config | Never client-public |
| AI passage history | Syncable product data | Useful across devices |
| AI passage validation status | Syncable product data | Part of persisted artifact metadata |

## Anti-patterns

- uploading the whole SQLite file as the first sync strategy
- making report aggregates the only cloud study truth
- storing platform-specific runtime state in shared cloud tables
- letting account login become required for ordinary local learning execution

## Open decisions

- whether wrong-word state should sync as projection rows, event stream, or both
- whether report aggregates need cloud caching for product reasons
- whether AI artifacts should include bucket-backed attachments later

## Exit criteria

- Every major data family is classified.
- The schema design phase can proceed without confusing local truth and cloud truth.
