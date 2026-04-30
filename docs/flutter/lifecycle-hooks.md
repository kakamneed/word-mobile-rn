# Flutter Lifecycle Hooks

Status: Draft
Owner: Mobile platform layer + Flutter shell layer
Phase: Slice 4 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how Flutter lifecycle events should interact with the bridge and Rust runtime without moving domain truth into Flutter memory.

## Principle

Flutter lifecycle signals are hints, not truth.

That means:

- Flutter may notify the system that the app entered background or foreground.
- Rust and persisted state remain responsible for session continuity and restoration truth.

## Slice 4 first-pass decisions

- lifecycle callbacks enter Flutter shell first, then flow through SDK/bridge
- Flutter does not rebuild domain truth from widget memory on resume or restart
- startup always re-enters through runtime/bootstrap truth
- active study session continuity remains Rust-owned even if Flutter process memory is lost

## Lifecycle points to handle

### Cold start

Required behavior:

- initialize runtime prerequisites
- read bootstrap state
- decide onboarding vs app shell
- avoid rendering a fake ready state before bootstrap truth is known

### Foreground entry

Required behavior:

- refresh app-level shell state if needed
- check whether resumable study state exists
- avoid recomputing domain truth in Flutter

### Background entry

Required behavior:

- flush or persist any bridge-local transient state if needed
- do not cancel study sessions implicitly
- do not assume Flutter memory will survive

### Process restart

Required behavior:

- bootstrap from persisted truth
- discover resumable session state via Rust/runtime path
- restore app shell from domain-owned state instead of Flutter cache

## Hook ownership

Flutter shell owns:

- listening to app lifecycle callbacks
- routing those callbacks to the SDK/bridge layer
- deciding UI transitions based on typed state returned from SDK

Native adapter owns:

- platform-specific lifecycle integration details
- secure storage/path/runtime availability checks

Rust owns:

- resumable session truth
- persisted progress truth
- restart recovery truth

## Suggested lifecycle pipeline

```text
platform lifecycle callback
  -> Flutter shell observer
    -> Flutter SDK / bridge entry
      -> native adapter if needed
        -> Rust/runtime authoritative read
```

## Lifecycle checks by phase

### Startup check

- runtime initialized
- bootstrap DTO decoded
- onboarding/app routing determined

### Foreground check

- app shell still valid
- resumable session indicator available
- optional lightweight refresh for visible feature surfaces

### Study resume check

- session id
- progress
- current question identity
- question-engine compatibility status

### Background check

- no implicit session cancel
- no local Dart-only persistence pretending to be session truth

## Event-specific rules

### App resume after possible drift

- prefer authoritative re-read
- do not trust stale in-memory Today or Study state
- route by typed runtime/session state, not by last visible screen alone

### Restart during active study

- Flutter shell restarts from bootstrap/runtime truth
- session resume hint comes from Rust/runtime persistence
- progress and current question must come from persisted session truth

### Resume after startup error

- retry startup path
- do not bypass bootstrap just because Flutter remembers a last route

## Anti-patterns

- Reconstructing a study session from Flutter widget state after restart
- Treating foreground entry as permission to regenerate today snapshot in Dart
- Cancelling active sessions on background just because Flutter loses view state
- Using page memory as the only source of answer-progress truth

## Verification scenarios

- cold start with healthy runtime
- cold start with runtime/storage failure
- background -> foreground during active study
- process kill -> restart during active study
- restart after engine version change
- foreground re-entry after potential state drift

## Downstream handoff

- Slice 5 can attach auth/runtime state restoration to this lifecycle shape
- Slice 6 can implement Today/Plan/Study resume semantics without inventing new lifecycle ownership
- Slice 8 can later plug sync triggers into foreground entry without changing domain ownership

## Exit criteria

- Lifecycle event handling does not require Flutter to own domain state.
- Session resume behavior is defined before Flutter feature implementation.
- Restart recovery is driven by Rust/runtime truth, not page reconstruction.
- Foreground re-entry rules are explicit enough for Slice 6 implementation.
