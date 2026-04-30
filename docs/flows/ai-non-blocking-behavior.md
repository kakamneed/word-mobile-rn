# AI Non-Blocking Behavior

Status: Draft
Owner: Flutter shell + Rust shared core + cloud integration layer
Phase: Slice 7 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines how AI passage features should behave so they remain optional enhancements and never block the primary study loop.

## Slice 7 first-pass decisions

- AI is additive, not foundational
- AI surfaces may fail independently without poisoning Today or Study
- provider secrets never belong in Flutter feature code
- generated passages and history are persisted artifacts, not only page memory
- AI context / generation / history / passage-read are separate contract paths

## Core principle

AI is additive, not foundational.

That means:

- today must remain usable without AI
- study must remain completable without AI
- AI failure must not downgrade the rest of the app into a broken state

## Contract families

- `getTodayAiPassageContext(...)`
- `generateAiPassage(...)`
- `getAiPassageHistory(...)`
- `getAiPassage(...)`
- `saveAiPassage(...)`

Rules:

- Flutter consumes AI through Rust-backed contracts or adapter-controlled paths
- Flutter must not own provider secret handling
- Flutter must not infer validation truth locally

## Entry points

- user opens AI surface intentionally
- today may show AI-related CTA or preview
- user requests passage generation
- user opens AI history or a saved passage

## Non-blocking rules

### Today surface

- AI context load may be deferred
- today task counts and next action do not wait for AI success
- missing AI config must not block today readiness

### Study surface

- study completion never waits for AI generation
- wrong-word updates and reports persist independently of AI availability
- AI-related CTA may be hidden/disabled without affecting study loop correctness

### AI surface itself

- generation may fail visibly
- history read may fail visibly
- a failure on this surface should not poison unrelated app state

## State model

Suggested high-level AI states:

- `idle`
- `context_loading`
- `ready_to_generate`
- `generating`
- `generated`
- `history_loading`
- `error_non_blocking`

## Failure classes to distinguish

- provider configuration missing
- network unavailable
- provider request failed
- validation failed
- history load failed
- passage read failed

## Required UX behavior

- failures are visible
- retry remains possible when sensible
- user can leave AI and continue normal app use immediately
- failed AI actions do not overwrite valid saved passage history

## Persistence rules

- successful generated passages can be saved and revisited
- history is a persisted artifact, not just temporary UI memory
- validation status belongs to Rust/domain output, not Flutter-only UI inference

## Ownership boundary

### Flutter may own

- CTA placement
- loading/pending states
- history list layout
- passage reading layout
- retry affordance

### Flutter may not own

- provider secret storage
- generation truth
- validation truth
- "best available fallback" semantics that silently mutate domain expectations

## Main smoke flows

### Flow 1: Today -> AI generate success

1. user opens AI surface
2. today AI context loads if available
3. generation succeeds
4. passage saves and appears in history

### Flow 2: Today -> AI generate failure

1. AI request fails
2. failure is visible
3. user can leave AI surface and continue normal study loop

### Flow 3: AI history reopen

1. previously saved passage exists
2. user opens history
3. user reopens saved passage
4. Flutter reads persisted artifact instead of assuming page memory

## Anti-patterns

- gating today readiness on AI context success
- triggering generation during main bootstrap path
- blocking session summary or study completion on AI persistence
- letting Flutter call provider secrets directly
- treating generated-but-unsaved in-memory passage text as the only history truth

## Verification focus

- missing config still allows normal app entry
- generation failure leaves today/study intact
- generation success writes passage/history as expected
- saved passage can be reopened later
- AI surface failure never blocks return to Today or Study

## Exit criteria

- AI feature paths are clearly optional.
- Failure of AI paths does not break core study flows.
- Slice 8 can later sync AI artifacts without redefining Slice 7 non-blocking rules.
