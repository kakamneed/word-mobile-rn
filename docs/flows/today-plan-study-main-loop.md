# Today Plan Study Main Loop

Status: Draft
Owner: Flutter shell + Rust shared core
Phase: Slice 6 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document describes the primary mobile learning loop that must survive the migration to Flutter without changing domain truth.

The loop is:

```text
bootstrap -> today -> plan(optional) -> today -> study -> summary -> today
```

## Slice 6 first-pass decisions

- Flutter owns shell flow, routing, and interaction feel.
- Rust owns today truth, plan semantics, study correctness, summary truth, and persisted side effects.
- SQLite remains the immediate persisted truth for runtime continuity.
- Today refresh always comes from authoritative re-read rules, not local recomputation.
- `savePlan` and `applySavedPlanToToday` are distinct user actions with distinct consequences.
- Study completion and study cancellation are distinct domain paths.

## Guiding rules

- Flutter owns presentation and interaction flow.
- Rust owns today truth, plan semantics, study correctness, and completion side effects.
- SQLite remains the immediate persisted truth for runtime continuity.

## Shell states Slice 6 must support

### Bootstrap / app shell

Flutter should be able to represent:

- bootstrap loading
- onboarding route
- app-ready route
- startup error route

Rust must still own:

- bootstrap truth
- app readiness semantics
- resumable-session discovery

### Today shell

Flutter should be able to represent:

- today payload loading
- targets/progress display
- next recommended action
- resumable session hint
- entry CTAs for plan and study

Rust must still own:

- target counts
- carryover semantics
- recommendation logic
- resumable-session truth

### Plan shell

Flutter should be able to represent:

- active plan read
- local editable draft
- save pending
- apply-to-today pending/result
- recoverable save/apply error

Rust must still own:

- growth rule semantics
- persisted plan truth
- apply-to-today semantics
- same-day today snapshot behavior

### Study shell

Flutter should be able to represent:

- session loading
- current question display
- local answer input state
- submit pending
- summary ready
- cancel confirmation

Rust must still own:

- session lifecycle
- correctness
- progress truth
- summary truth
- wrong-word and report side effects

## Main loop slices

### 1. Bootstrap to Today

Goal:

- user reaches a truthful today shell after app startup

Required steps:

1. runtime paths and SQLite become available
2. bootstrap state is read
3. onboarding or app shell route is chosen
4. today payload is loaded from Rust

Must not happen:

- today task counts guessed by Flutter before Rust returns
- app entering a fake ready state from UI defaults alone

### 2. Today to Plan

Goal:

- user can inspect and modify the active plan safely

Required steps:

1. read active plan
2. render local editable draft
3. save plan
4. optionally apply saved plan to today

Must not happen:

- same-day today snapshot silently recomputed just because draft values changed
- Flutter reinterpreting growth-rule semantics

### 3. Today to Study

Goal:

- user can start a study session from today and complete a real answer loop

Required steps:

1. choose study mode from today
2. call Rust session start
3. render current question
4. submit answer
5. render next question or summary
6. complete or cancel session
7. return to today through authoritative refresh policy

Must not happen:

- Flutter guessing answer correctness
- Flutter locally calculating the only source of summary truth

## Contract boundaries

### Today-facing contract family

- `getBootstrapState`
- `getTodayHomeState`
- later optional resumable-session hint if contract expands

### Plan-facing contract family

- `getActivePlan`
- `savePlan`
- `applySavedPlanToToday`

### Study-facing contract family

- `startStudySession`
- `submitStudyAnswer`
- `completeStudySession`
- `cancelStudySession`

## Loop invariants

- Today targets and progress come from Rust truth.
- Plan saves and apply-to-today are distinct actions.
- Study session lifecycle remains Rust-driven.
- Completion side effects update wrong-word and report truth through Rust persistence.
- Navigation is never the only source of domain state.

## Primary UX checkpoints

- On opening the app, the user can understand today goal quickly.
- Plan edits feel local and responsive, but commit through Rust.
- Study cards remain touch-friendly and answer-safe.
- After study completion, today reflects the updated persisted truth.

## Failure boundaries

### Bootstrap failure

- startup error route is shown
- app does not fake readiness

### Plan save failure

- draft stays in UI
- persisted truth remains unchanged

### Apply-to-today failure

- active plan may still be valid
- today snapshot must not be guessed locally
- user gets recoverable apply failure feedback

### Study submit failure

- current question is not advanced locally
- UI shows recoverable error state

### Session completion failure

- completion is not inferred from UI alone
- app re-checks authoritative state before final routing

## Main smoke flows

### Flow 1: Bootstrap -> Today

1. app starts
2. bootstrap succeeds
3. today payload loads
4. user sees targets and next action

### Flow 2: Today -> Plan -> Save

1. user enters plan
2. reads active plan
3. edits targets / growth rules
4. saves plan
5. same-day today snapshot remains stable unless apply flow is used

### Flow 3: Today -> Study -> Complete -> Today

1. user starts mode from today
2. question renders
3. answer submitted
4. summary returned
5. complete session acknowledged
6. today refreshes from Rust truth

### Flow 4: Today -> Study -> Cancel

1. user starts session
2. user cancels
3. local and UI state honor cancel semantics
4. no fake completion summary shown

### Flow 5: Restart -> Resume -> Complete

1. session in progress
2. app restart
3. Flutter bootstrap + runtime restore
4. session resumes from persisted truth
5. session completes with no duplicated results

## Verification checkpoints

- bootstrap -> today happy path
- bootstrap startup error path
- plan save without same-day snapshot mutation
- apply-to-today distinct from save-only
- study correct/incorrect/skipped loop
- study cancel path
- restart -> resume -> complete path

## Exit criteria

- The full main loop can be explained without relying on hidden UI assumptions.
- Each transition has one clear source of truth owner.
- Slice 7 can extend the loop without redefining Today / Plan / Study ownership.
