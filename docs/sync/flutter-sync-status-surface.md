# Flutter Sync Status Surface

Status: Draft
Owner: Flutter shell + Rust sync layer
Phase: Slice 8 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the small, read-only sync status surface Flutter may consume.

## Slice 8 first-pass decisions

- Flutter may present sync state but does not own sync mechanics
- sync status must remain non-blocking for local-first study
- debug/support surfaces may be richer than everyday user-facing surfaces

## Flutter must not own

- sync queue state mutation
- merge logic
- cursor advancement

## Allowed Flutter-visible sync fields

- `syncEnabled`
- `accountSyncState`
- `pendingUploadCount`
- `lastSyncSucceededAt`
- `lastSyncErrorCode`

## Disallowed Flutter-visible internals

- raw outbox payloads
- merge resolution details
- service-role or maintenance details
- cloud SQL error internals

## UI use cases

- subtle sync indicator
- manual retry affordance
- diagnostics summary for settings/support page

## UX rules

- sync status should never block main study entry
- failed sync should be visible but not catastrophic to local-first flows
- pending sync count may be shown, but not as a hard gate for learning

## Suggested surfaces

### Everyday app surface

- tiny sync indicator
- pending upload badge if useful
- high-level last error state only when relevant

### Settings / diagnostics surface

- last successful sync time
- latest high-level sync error code
- pending upload count
- whether account sync is enabled or paused

## Retry ownership

Flutter may:

- expose a retry button
- route that action into the Rust sync layer

Flutter may not:

- implement queue retry logic itself
- mutate outbox rows directly

## Exit criteria

- Flutter has enough information to present sync state without becoming sync owner.
- Support surfaces can distinguish sync degradation without exposing sensitive internals.
