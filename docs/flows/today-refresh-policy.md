# Today Refresh Policy

Status: Draft
Owner: Flutter shell + Rust shared core
Phase: Slice 6 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines when the Flutter app should re-read authoritative today state instead of inferring updates from UI actions.

## Principle

Flutter should refresh today from Rust truth whenever an action may have changed persisted today-facing state.

Flutter should not recompute today snapshot locally.

## Slice 6 first-pass decisions

- cold start always enters today through an authoritative read
- `savePlan` does not imply same-day today snapshot mutation
- `applySavedPlanToToday` does imply today-facing mutation
- study completion almost always requires authoritative today refresh
- resume after possible drift prefers re-read over trusting in-memory state

## Refresh triggers

### Always refresh

- after cold start bootstrap succeeds
- after `applySavedPlanToToday`
- after successful study session completion
- after app resume where runtime drift may have occurred
- after restart recovery

### Conditional refresh

- after session cancel
  - refresh if cancel semantics affect resumable-session or today CTA state
- after plan save without apply
  - refresh active plan if needed
  - do not assume same-day snapshot changed

### Usually no full today refresh needed

- while editing plan draft before save
- while typing or previewing study response
- during local loading-state transitions that do not commit persisted truth

## Why this policy exists

Without this policy, Flutter may:

- decrement targets optimistically
- recompute progress from partial UI state
- mutate same-day snapshot semantics
- drift from Rust-side carryover and recommendation logic

## Trigger matrix

| Event | Full today re-read | Reason |
|---|---|---|
| bootstrap success | yes | initial authoritative entry |
| plan draft change | no | local UI only |
| `savePlan` only | usually no | active plan may change, today snapshot should remain stable same day |
| `applySavedPlanToToday` | yes | explicit today-facing mutation |
| study answer submit mid-session | no | session still in progress |
| study complete | yes | progress/targets/recommendation may change |
| study cancel | conditional | depends on resumable/cancel semantics |
| app foreground after possible drift | yes | local truth may have changed while UI was inactive |
| restart after crash/process death | yes | reconstruct from persisted truth |

## Related refresh classes

The app should distinguish:

- today payload refresh
- active plan-only refresh
- study-state refresh

These should not all be collapsed into one page-local helper.

## Implementation notes

- Refresh should happen through the SDK/client layer, not by page-specific ad hoc calls.
- The app should distinguish:
  - today payload refresh
  - active plan-only refresh
  - study state refresh
- "return to today" is not itself a refresh rule; refresh depends on whether persisted today-facing truth changed.

## Anti-patterns

- locally adjusting `completedTasks` before reading Rust
- locally recomputing carryover
- assuming plan save equals same-day today snapshot mutation
- treating the last visible Today widget state as durable truth after resume

## Exit criteria

- Every major today-facing mutation has an explicit refresh rule.
- Flutter never needs to guess same-day snapshot semantics.
- Slice 6 implementation can decide refresh timing without reopening ownership debates.
