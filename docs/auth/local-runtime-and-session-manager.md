# Local Runtime And Session Manager

Status: Draft
Owner: Mobile auth/runtime layer
Phase: Slice 5 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how the mobile app should manage:

- local runtime bootstrap
- auth session restoration
- secure storage access
- account state resolution
- fallback into local-first execution

It exists to keep account support from breaking local study continuity.

## Core principle

Local study execution must remain possible even when:

- the user is not signed in
- auth refresh fails
- the device is offline
- cloud sync is temporarily unavailable

## Slice 5 default decisions

- SQLite remains the authoritative local runtime for study continuity.
- auth token and refresh token live only in secure storage, never in SQLite
- Flutter feature code must not read raw token values directly
- local identity, cloud identity, and device identity are different concepts
- logout transitions to `signed_out_retained_local` by default
- revoked or deleted account state blocks protected cloud actions, not local bootstrap
- reinstall defaults to a new logical `device_id` unless a later secure recovery path is added

## Runtime layers

### SQLite runtime

Owns:

- today state
- plan state
- active/resumable study state
- wrong-word state
- reports state
- persisted AI artifacts
- local account/runtime metadata that is not secret

Must not own:

- access token
- refresh token
- raw provider secrets

### Secure storage runtime

Owns:

- access token
- refresh token
- secure device-bound session material
- any secret binding material needed for future device/account proof

Must not own:

- full study history
- report truth
- local session truth

### Lightweight app preferences

Owns:

- non-sensitive flags
- UX-only toggles
- optional onboarding markers if not already modeled elsewhere

## Ownership matrix

| Concern | Owner |
|---|---|
| study continuity | Rust + SQLite runtime |
| account session existence | secure storage + auth session manager |
| account UI route selection | Flutter shell from typed state |
| token refresh scheduling | auth session manager |
| today snapshot truth | Rust runtime |
| logout clearing rules | auth session manager |
| sync eligibility | typed account/runtime state, not arbitrary feature logic |

## Local identity model

Slice 5 should keep these identity concepts explicit:

| Identity | Meaning | Storage |
|---|---|---|
| `local_profile_id` | device-local logical owner id that also works in guest mode | SQLite |
| `user_id` | authenticated cloud account id from Supabase | secure session + local metadata mirror when needed |
| `device_id` | stable logical device identity for account-scoped writes | SQLite metadata and/or secure bootstrap metadata |

Rules:

- `local_profile_id` may exist before any login
- `user_id` may be null while still allowing normal study
- `device_id` survives ordinary app restart
- reinstall should create a new `device_id` by default
- no feature should assume `local_profile_id == user_id`

## Suggested state model

| State | Meaning | Local study | Cloud reads/writes |
|---|---|---|---|
| `guest_local_only` | no active cloud identity | allowed | disabled |
| `signed_in_active` | valid session and normal account mode | allowed | allowed |
| `signed_in_expired` | previous sign-in exists but refresh/session is invalid | allowed | blocked or degraded |
| `signed_out_retained_local` | user explicitly logged out but local data remains | allowed by default policy | disabled |
| `account_deleted_or_revoked` | account removed or device lost protected access | allowed in retained-local mode only | blocked |

## State transitions

- first launch with no session -> `guest_local_only`
- valid session restored on startup -> `signed_in_active`
- refresh failure or token invalidation -> `signed_in_expired`
- explicit logout -> `signed_out_retained_local`
- deleted account / revoked device -> `account_deleted_or_revoked`
- explicit re-login from retained local state -> `signed_in_active` or bind flow

## Session manager responsibilities

The session manager should own:

- startup session read
- auth state resolution
- refresh attempts
- session invalidation
- logout clearing rules
- signed-in vs guest mode reporting
- typed account-state publication to the Flutter shell

It should not own:

- study session truth
- today snapshot generation
- report aggregation
- wrong-word or report merge semantics

## Startup flow

Suggested order:

1. prepare filesystem/runtime paths
2. open or migrate SQLite
3. initialize or verify local runtime metadata such as `local_profile_id` and `device_id`
4. read secure storage auth session
5. initialize Rust runtime/bootstrap
6. resolve auth/account state
7. route app shell

Important rules:

- secure storage failure must not automatically make SQLite unreadable
- Rust bootstrap may succeed even when auth state is degraded
- route selection must come from typed runtime/auth state, not scattered feature checks

## Account resolution rules

### Guest startup

- no valid secure session
- local runtime still boots normally
- app enters `guest_local_only` or `signed_out_retained_local` depending on last explicit action

### Signed-in startup

- valid secure session exists
- session manager restores account context
- if local and cloud need attachment, enter bind flow rather than guessing

### Expired session startup

- local runtime still boots
- session manager marks state as `signed_in_expired`
- cloud writes pause until recovery succeeds

### Deleted or revoked account/device

- clear protected cloud eligibility
- keep local runtime readable by default
- show a typed recovery-safe state instead of a crash

## Recovery behavior

### Offline startup

- local runtime should still bootstrap
- cloud session may remain stale until network returns
- study execution remains available if product policy allows

### Expired token

- app should not silently destroy local data
- cloud-facing actions should pause or degrade
- user should transition into a typed auth state, not a generic crash

### Restart during study

- study continuity should still come from Rust/runtime persisted truth
- auth session manager should only affect account state, not session correctness

### Secure storage unavailable

- treat as auth/session degradation, not database corruption
- allow local runtime bootstrap where possible
- surface a supportable typed error path for re-auth or reinstall decisions

## Session refresh policy

- refresh is owned centrally by the auth session manager
- Flutter feature code should receive typed auth state, not raw `401` handling responsibility
- foreground return may trigger refresh, but refresh failure must not wipe local study continuity
- refresh retry policy belongs to the auth/session layer, not individual feature pages

## Interfaces to define later

- `AuthSessionManager`
- `RuntimeBootstrapCoordinator`
- `SecureStorageAdapter`
- `AccountStateResolver`
- `LocalIdentityProvider`

## Verification focus

- guest cold start
- signed-in restart with valid session
- expired token restart
- logout keep-local
- account-deleted-or-revoked recovery state
- active study session across auth-state change boundary

## Exit criteria

- Startup ordering is explicit.
- Account/session handling does not own learning truth.
- Token boundary is explicit.
- Fallback local mode remains clearly supported.
