# Auth And Device Lifecycle

Status: Draft
Owner: Cloud integration layer + client auth layer
Phase: Slice 3 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the intended lifecycle for accounts and devices once Supabase Auth is introduced.

The main goal is to avoid ambiguous behavior around:

- guest-to-account upgrade
- multiple devices on one account
- logout semantics
- account deletion

## Core states

### Guest local-only user

Characteristics:

- no cloud identity required
- local SQLite truth only
- can study offline

### Signed-in user with empty cloud state

Characteristics:

- valid Supabase identity
- no meaningful synced product data yet
- local data may be first source to bind

### Signed-in user with existing cloud state

Characteristics:

- valid Supabase identity
- cloud plan/preferences/history already exist
- local and cloud may need merge or explicit conflict handling

### Revoked device

Characteristics:

- Supabase user may still exist
- device row is marked unusable for protected cloud writes
- local SQLite may still be readable until the app resolves state

### Deleted account or hard-invalid identity

Characteristics:

- previous cloud identity is no longer usable
- local runtime must not panic or hard-crash
- app must transition into an explicit degraded or local-retained state

## Lifecycle state model

| Runtime state | Auth identity | Device row | Local SQLite | Expected cloud behavior |
|---|---|---|---|---|
| `guest_local_only` | none | optional local pre-registration marker only | readable/writable | no cloud writes |
| `signed_in_needs_bind` | valid | created or pending refresh | readable/writable | merge decision required before broad sync |
| `signed_in_active` | valid | active | readable/writable | normal sync allowed |
| `signed_in_revoked_device` | valid | revoked | readable/writable | block protected writes; prompt recovery |
| `signed_out_retained_local` | none or cleared | stale historical row allowed | readable/writable by product policy | no protected writes |
| `account_deleted_or_invalid` | invalid | irrelevant or stale | readable at minimum | do not attempt ordinary sync |

## Key transitions

### Signup

Expected outcome:

- auth identity created
- profile row created
- initial device row registered
- local runtime remains authoritative for in-progress or same-device local truth

### Login

Expected outcome:

- auth session available
- device recognized or registered
- local app decides whether bind/merge is needed

Decision after login:

- if local syncable state exists and cloud is empty, enter `signed_in_needs_bind`
- if cloud state exists and local is empty, pull into local and continue
- if both sides contain meaningful state, run merge strategy per domain before full sync

### Guest bind to account

Expected outcome:

- local product data is evaluated for merge/upload
- cloud ownership established for future sync

Must define:

- whether bind is automatic or user-confirmed
- what wins if cloud state already exists

Recommended first-pass decision:

- automatic bind when cloud is empty
- explicit confirmation only when both local and cloud already contain meaningful user-authored state
- never discard local study history silently during bind

### Logout

Expected outcome:

- auth session removed from client
- local learning data may remain
- cloud-bound sync identity is detached until next login

Must define:

- what local data remains
- whether local cached cloud projections remain readable

Recommended first-pass policy:

- clear tokens and active auth session metadata
- keep local SQLite, reports, wrong words, plans, and resumable study truth unless a later product decision says otherwise
- transition into `signed_out_retained_local`
- let Slice 5 UX build on [logout-and-retention-policy.md](/d:/projects/word-mobile-rn/docs/auth/logout-and-retention-policy.md)

### Device revoke

Expected outcome:

- revoked device can no longer push protected cloud writes
- historical rows remain attributable
- local runtime can still open, but account state becomes recovery-gated

### Account deletion

Expected outcome:

- cloud account and user-owned cloud rows enter deletion flow
- local device should not silently crash if now orphaned

Recommended first-pass policy:

- surface an explicit account-deleted state
- stop protected cloud sync immediately
- preserve local readability long enough for recovery/export decisions
- require an explicit cleanup path before destructive local deletion

## Device model rules

- each signed-in device gets a stable `device_id`
- `device_id` survives ordinary app restarts
- reinstall behavior must be explicitly defined
- device revocation should not require rewriting historical study event ownership

## Reinstall and rotation rules

- ordinary restart keeps the same `device_id`
- reinstall should default to a new logical `device_id` unless secure recovery is explicitly supported later
- old device rows may remain historical even after new registration
- sync logic must not assume one account maps to one active device

## Failure handling rules

- auth refresh failure does not imply local runtime corruption
- revoked device does not imply immediate local database deletion
- deleted account does not imply Rust runtime bootstrap failure
- unsupported merge/bind state should stop cloud sync, not local study entry

## Open decisions

- whether guest bind auto-uploads local history immediately
- whether logout leaves a readable local archive
- whether reinstall creates a new logical device id or attempts recovery
- whether revoked devices may still read previously synced cloud data already cached locally

## Exit criteria

- Guest, signed-in, revoked-device, and deleted-account paths are all explicitly described.
- Slice 5 can implement auth UX without guessing lifecycle semantics.
