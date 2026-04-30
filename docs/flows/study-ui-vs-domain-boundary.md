# Study UI Vs Domain Boundary

Status: Draft
Owner: Flutter shell + Rust shared core
Phase: Slice 6 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document makes the boundary explicit between what the Flutter study UI may own and what must remain Rust-owned domain truth.

## Principle

The study screen may feel smart, but it must not become a second study engine.

## Slice 6 first-pass decisions

- Flutter renders the current study experience but does not own question truth.
- Rust owns correctness, progress, summary, cancel/complete semantics, and persisted side effects.
- Local optimistic behavior is allowed only for obvious UI affordances, not domain conclusions.
- Restart and resume always rebuild from persisted Rust/runtime truth.

## Flutter UI owns

- rendering current question
- collecting user input
- controlling pending/loading states
- presenting feedback returned from Rust
- local form affordances
- button enabled/disabled rules tied to obvious UI conditions
- post-success navigation choice

## Rust/domain owns

- question generation
- correctness evaluation
- session state transitions
- progress truth
- summary generation
- wrong-word updates
- report side effects

## Specific boundaries

### Question rendering

Flutter may:

- display prompt, choices, phonetics, examples
- decide layout and accessibility behavior

Flutter may not:

- generate substitute questions
- choose alternate accepted meanings as truth

### Answer submission

Flutter may:

- block duplicate taps
- show submitting state
- preserve user input until response returns

Flutter may not:

- pre-score the answer
- advance to next question without Rust response

### Progress

Flutter may:

- display progress returned by Rust

Flutter may not:

- maintain the only authoritative `current/total`
- infer completion from local counters alone

### Summary

Flutter may:

- present summary beautifully
- choose post-summary navigation

Flutter may not:

- compute final correct/incorrect/skipped totals as the only truth

### Cancel vs complete

Flutter may:

- ask for confirmation before cancel
- present different UX for cancel and complete

Flutter may not:

- treat cancel as a silent complete
- assume completion side effects happened before Rust confirms them

## Allowed optimistic behavior

- temporary button disabled state
- local spinner/progress indicator while bridge call is pending
- draft answer text retention before submit returns
- temporary save/submit loading affordances

## Forbidden optimistic behavior

- pre-marking answer as correct
- pre-advancing to next card
- mutating today totals before authoritative completion flow
- showing a synthetic final summary before Rust completion result
- pretending cancel succeeded before Rust confirms cancel semantics

## Restart and resume rule

If Flutter loses process memory:

- the study UI must rebuild from persisted Rust/runtime truth
- not from cached widget tree assumptions

Required recovered truths:

- session id
- current question identity
- progress
- compatibility/resume status

## Error handling boundary

Flutter may:

- show retry UI
- show recoverable error state
- keep input visible when appropriate

Flutter may not:

- invent fallback grading
- silently continue the session if submit failed
- synthesize a "best guess" summary after completion failure

## Verification focus

- submit correct/incorrect/skipped
- cancel vs complete distinction
- restart/resume integrity
- summary alignment with persisted side effects
- no progress regression after restart

## Exit criteria

- The study UI can be implemented without re-owning domain rules.
- Feature code can point to this document to decide what belongs in Flutter vs Rust.
- Slice 7 can reuse these boundaries when wrong-word/report side effects become visible surfaces.
