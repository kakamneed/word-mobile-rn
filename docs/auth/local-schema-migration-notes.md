# Local Schema Migration Notes

Status: Draft
Owner: Mobile runtime layer + storage layer
Phase: Slice 5 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document tracks the local schema and runtime migration concerns introduced when account support is added to a previously local-only app.

## Goals

- Preserve existing local study data
- Introduce account-aware runtime metadata safely
- Avoid mixing auth migration with unrelated domain migrations

## Slice 5 default decisions

- existing pre-account SQLite databases must remain readable
- tokens stay out of SQLite even after account support is added
- secure storage initialization failure must not automatically force a destructive reset
- account-aware metadata should be additive and migration-safe
- reinstall should be treated separately from ordinary schema migration

## Migration concerns

### Existing pre-account SQLite databases

Need to answer:

- what minimum new metadata must be added
- whether local rows need owner scoping
- whether any sync markers are introduced

Recommended first-pass approach:

- add runtime/account metadata tables or columns additively
- avoid rewriting historical study rows purely for account support
- do not require every legacy row to be backfilled with cloud identity before app startup can continue

### Secure storage initialization

Need to answer:

- how token storage is created on first account-capable launch
- how startup behaves when secure storage is unavailable

Recommended first-pass approach:

- initialize secure storage lazily but early in startup
- if secure storage cannot initialize, keep local runtime readable where possible
- surface an auth/runtime-specific issue instead of a generic database corruption error

### Runtime compatibility

Need to answer:

- how auth/runtime migration interacts with question-engine version migration
- how migration failures are surfaced distinctly from bootstrap domain failures

## Minimum local metadata to introduce

Exact names may change, but Slice 5 should assume these metadata families exist locally:

- local runtime owner marker such as `local_profile_id`
- logical `device_id`
- nullable last-known `user_id`
- account state / bind marker
- future sync marker placeholders where needed

Rules:

- these fields are runtime coordination metadata, not replacements for domain truth
- adding them must not make old study data unreadable
- auth token material still does not belong in SQLite

## Suggested categories of migration

- storage schema migration
- secure storage bootstrapping
- account-state metadata migration
- runtime/bootstrap compatibility migration

These should be tracked separately, even if they happen in the same release window.

## Startup order under migration

Suggested order:

1. open legacy SQLite
2. run additive schema migration
3. ensure local runtime metadata exists
4. initialize secure storage
5. read or create session metadata
6. continue Rust bootstrap

Important rule:

- a failure in step 4 must not be mislabeled as step 1 or step 2 corruption

## Compatibility rules

- old local databases must remain readable without login
- existing study/session/report/wrong-word truth must not be dropped as a shortcut
- account metadata migration must not silently rewrite study semantics
- migration for account support must not be bundled with unrelated destructive cleanup

## Reinstall posture

- reinstall is not the same as schema migration
- reinstall should usually produce a new secure storage context
- reinstall may produce a new logical `device_id`
- old cloud device rows can remain historical rather than forcing local rewrite

## Failure surfacing

Failure categories should be distinguishable:

- SQLite schema migration failure
- secure storage initialization failure
- account-state metadata migration failure
- runtime/bootstrap compatibility failure

Do not hide these behind:

- generic auth failed
- generic startup failed
- forced onboarding reset

## Risks

- new account metadata makes old local databases unreadable
- secure storage initialization failure blocks unrelated local startup
- account-scoping fields are added in a way that forces destructive reset
- device/account metadata is mixed into core study tables too aggressively

## Smoke scenarios

- legacy local-only user upgrades and can still read today/study state
- legacy local-only user upgrades while offline
- secure storage unavailable but SQLite still boots
- signed-in upgrade with existing local database
- reinstall without secure storage recovery creates a safe new device identity posture

## Anti-patterns

- deleting the old database as a shortcut to account support
- forcing login before old local data can be opened
- hiding migration failures behind generic auth errors
- storing access token or refresh token in SQLite migration tables

## Exit criteria

- Old local databases remain readable or deliberately migratable.
- Migration categories are separated.
- Token boundary is preserved.
- Account support does not require destructive reset of prior local study data.
