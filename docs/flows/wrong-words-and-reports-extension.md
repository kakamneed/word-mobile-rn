# Wrong Words And Reports Extension

Status: Draft
Owner: Flutter shell + Rust shared core
Phase: Slice 7 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document describes how the main learning loop extends into:

- wrong-word notebook surfaces
- report overview surfaces

without moving wrong-word or report truth into Flutter.

## Slice 7 first-pass decisions

- wrong-word and report values remain Rust-owned persisted truth
- Flutter may extend reading and presentation, not truth generation
- study side effects must be observable later through Rust-backed reads
- opening wrong words or reports must not require Flutter recomputation
- wrong-word list/detail and reports overview are separate contract surfaces

## Principle

Wrong-word state and report aggregates remain Rust-owned persisted truth.

Flutter may improve:

- layout
- filtering
- chart rendering
- detail presentation

Flutter may not become:

- the source of wrong-word scoring truth
- the source of report aggregation truth

## Contract families

### Wrong words

- `getWrongWords(...)`
- `getWrongWordDetail(...)`

### Reports

- `getReportsOverview(...)`

Rules:

- Flutter reads these contracts as Rust-owned DTOs
- Flutter does not synthesize list/detail/report rows when reads fail
- later sync or AI work must not change these ownership rules

## Wrong-word flow

### Entry points

- user finishes a session and later opens wrong words
- user opens wrong words directly from app navigation

### Required behavior

1. Flutter reads wrong-word list from Rust-backed contract
2. Flutter renders filter and list UI
3. user selects an entry
4. Flutter reads wrong-word detail from Rust-backed contract
5. detail renders risk/history/related information

### Rust-owned truth

- which entries are active
- error counts
- last wrong timestamp
- risk breakdown
- derived priority score
- error history

### Flutter-owned concerns

- list layout
- filter chips
- sort/display affordances
- detail expansion and visual grouping
- loading/error/empty states

### Wrong-word UI state model

- `list_loading`
- `list_ready`
- `list_empty`
- `list_error`
- `detail_loading`
- `detail_ready`
- `detail_error`

## Reports flow

### Entry points

- user opens reports from navigation
- user finishes study and later inspects progress

### Required behavior

1. Flutter reads reports overview from Rust-backed contract
2. Flutter renders KPI summary
3. Flutter renders chart(s) and mode breakdown
4. user may scroll, switch range, inspect breakdown

### Rust-owned truth

- overall accuracy
- total questions answered
- streak info
- daily series values
- mode breakdown values

### Flutter-owned concerns

- chart composition
- scroll behavior
- labels and visual density
- comparative emphasis and section ordering
- loading/error/empty states

### Reports UI state model

- `overview_loading`
- `overview_ready`
- `overview_empty`
- `overview_error`

## Shared invariants

- wrong-word list/detail must reflect persisted study side effects
- report overview must reflect persisted aggregate truth
- opening either surface must not require recomputation in Flutter
- study completion may change these surfaces, but the user sees that change only through authoritative reads

## Study side-effect extension

Slice 7 depends on this chain remaining true:

```text
study complete / incorrect / skipped
  -> Rust persists side effects
    -> wrong-word read reflects new persisted truth
    -> report read reflects new aggregate truth
```

Flutter must not replace this with:

```text
study screen local memory
  -> guessed wrong-word/report updates
```

## Failure boundaries

### Wrong-word list failure

- Flutter shows recoverable list error
- no local fallback scoring logic is invented

### Wrong-word detail failure

- detail stays unavailable
- list remains usable

### Reports read failure

- Flutter shows recoverable reports error
- does not fabricate stale aggregate values as fresh truth

## Main smoke flows

### Flow 1: Study -> Wrong Words

1. user completes a session with incorrect/skipped answers
2. wrong-word list reflects updated persisted truth
3. wrong-word detail shows matching risk/history

### Flow 2: Study -> Reports

1. user completes a session
2. reports overview updates
3. chart and aggregate values align with Rust truth

### Flow 3: Direct open from navigation

1. user opens wrong words or reports from app navigation
2. surface reads from Rust-backed contract
3. no study-screen-local assumptions are required

## Verification focus

- study with incorrect/skipped answers leads to wrong-word visibility
- wrong-word detail risk breakdown matches Rust output
- report aggregate values align with baseline after study completion
- chart layout changes do not change aggregate meaning
- direct navigation entry works without hidden study-memory dependency

## Exit criteria

- Wrong-word and reports surfaces can be implemented with no Flutter-side aggregate recalculation.
- All data shown in these flows has one clear Rust-owned truth source.
- Slice 8 sync planning can treat these surfaces as readers of persisted truth, not independent truth generators.
