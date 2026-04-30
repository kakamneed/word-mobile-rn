# Logout And Retention Policy

Status: Draft
Owner: Mobile auth layer
Phase: Slice 5 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines what happens when a user logs out, loses auth session validity, or deletes their account.

The main goal is to prevent destructive ambiguity around local learning data.

## Policy principles

- Logging out should not silently delete learning history by default.
- Token clearing and data deletion are separate actions.
- Local-first execution should not be broken by auth state changes unless product policy explicitly requires it.

## Slice 5 default decisions

- explicit logout does not delete SQLite by default
- explicit logout clears secure auth material
- explicit logout keeps local plan, reports, wrong words, and study continuity by default
- AI local artifacts may remain readable if they are not secret-bearing; provider credentials must not remain
- token expiry is not equivalent to logout
- account deletion is not equivalent to logout

## Logout actions

### Must clear

- access token
- refresh token
- active cloud auth session metadata
- secure provider credentials or secrets that are account-bound

### Must not be implicitly cleared by default

- SQLite learning history
- reports
- wrong-word state
- plan data
- resumable local study state, unless product explicitly decides otherwise
- local AI passage history that is already persisted as non-secret user content

### Must transition

- app state enters `signed_out_retained_local` by default
- cloud sync eligibility becomes disabled
- future cloud-protected actions require re-auth

## Retained-local mode

Recommended post-logout mode:

- `signed_out_retained_local`

Characteristics:

- no cloud writes
- local runtime remains readable
- app may continue in guest/local mode depending on product decision
- support can still inspect runtime-level local continuity issues

## Explicit destructive actions

These are separate from logout and must never happen implicitly:

- erase local database
- erase retained AI history
- erase local resumable study state
- erase local cache after account deletion

If product later adds "remove local data from this device", it should be a second explicit action after logout, not part of logout itself.

## Token expiry behavior

When token refresh fails:

- do not wipe local data
- transition into explicit auth state
- block or degrade only cloud-protected actions
- do not pretend the user requested logout

Recommended state:

- `signed_in_expired` or equivalent typed state

## Device revoked behavior

When the server marks the device revoked:

- protected cloud writes stop
- local runtime remains readable by default
- app should surface a recoverable account/device state
- do not silently clear study history just because device access changed

## Account deletion behavior

Account deletion should be treated separately from logout.

Recommended first-pass policy:

- clear secure auth material
- stop all protected cloud operations
- preserve local cache as a read-safe retained archive until an explicit cleanup decision is taken
- surface a typed `account_deleted_or_revoked` state instead of crashing into generic auth failure

Questions that Slice 5 should answer explicitly:

- whether local cache remains as an orphaned archive
- whether the app re-enters guest mode automatically
- whether the user is prompted before removing cloud-bound metadata from device

Recommended default:

- keep local archive readable
- do not auto-delete SQLite
- allow explicit later cleanup if product wants it

## Local data retention matrix

| Data family | Logout | Token expired | Account deleted |
|---|---|---|---|
| auth tokens | clear | invalid / refresh failed | clear |
| SQLite study history | keep | keep | keep by default |
| active study session | keep by default | keep | keep by default unless product later says otherwise |
| reports | keep | keep | keep by default |
| wrong words | keep | keep | keep by default |
| AI local passages | keep by default | keep | keep by default |
| provider secrets | clear if account-bound | invalidate | clear |

## UI and support expectations

- logout should never masquerade as data loss
- expired token should not look like local runtime corruption
- account deletion should have a distinct supportable message
- any "remove local data" affordance must clearly state destructive consequences

## Anti-patterns

- tying `logout` to full local database reset
- leaving stale tokens in debug logs or preferences
- treating expired token as equivalent to corrupted local runtime
- coupling account deletion cleanup to invisible client-side destructive behavior

## Exit criteria

- Logout semantics are explicit.
- Data retention after logout is explicit.
- Token invalidation does not imply local history destruction.
- Deleted-account and revoked-device handling are distinct from ordinary logout.
