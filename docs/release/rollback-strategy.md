# Rollback Strategy

Status: Draft
Owner: Release management + mobile platform team
Phase: Slice 9 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how to halt or reverse a Flutter rollout without corrupting local user data.

## Core principle

Rollback must be a planned product path, not an improvisation.

## Trigger conditions

Examples:

- severe bootstrap failures
- resumable-session corruption
- migration failures after upgrade
- auth/session restore failure spike
- sync duplication or corruption spike

## Rollback requirements

- local SQLite truth must remain readable or intentionally migratable
- secure storage behavior must remain safe
- rollback must not depend on hand-editing user databases
- rollback decision must be observable from release metrics

## Rollback modes

### Halt rollout

Used when:

- staged rollout has not yet fully expanded

Action:

- stop further promotion
- continue investigation

### Roll back to prior stable client

Used when:

- a shipped Flutter build causes unacceptable regression

Action:

- move users back to prior stable channel/build path according to store/release mechanics

### Feature-path disablement

Used when:

- a narrow non-core path is unstable but full rollback is unnecessary

Action:

- only when the affected feature can be safely disabled without violating product truth

## Data-safety checklist

- no one-way destructive schema change without tested fallback
- no token/session cleanup that destroys unrelated local study data
- rollback path validated for:
  - guest users
  - signed-in users
  - users with resumable local sessions

## Validation scenarios

- RN -> Flutter upgrade -> rollback dry run
- signed-in Flutter user -> rollback
- sync queue present -> rollback
- expired session + rollback

## Anti-patterns

- assuming app-store rollback magically preserves runtime compatibility
- treating rollback as only a code branch concern
- deleting local caches/databases to simplify rollback

## Exit criteria

- Rollback triggers, modes, and data-safety expectations are explicit.
- Rollback remains product-operable, not developer-ritual-only.
