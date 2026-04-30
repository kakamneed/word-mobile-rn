# Reports UI Vs Aggregate Boundary

Status: Draft
Owner: Flutter shell + Rust shared core
Phase: Slice 7 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document makes explicit which parts of the reports experience belong to:

- Flutter UI
- Rust-owned aggregate truth

## Slice 7 first-pass decisions

- report values are domain truth
- chart rendering is presentation
- Flutter may derive display-only artifacts, never authoritative aggregate values
- stale or failed report reads must remain visibly stale or failed, not silently upgraded to truth

## Principle

Report values are domain truth.

Chart rendering is presentation.

## Rust-owned aggregate truth

- total study days
- total words learned
- total questions answered
- overall accuracy
- streak info
- per-mode breakdown values
- daily series values

## Flutter-owned UI behavior

- chart layout and scaling
- color and typography choices
- scroll and gesture behavior
- comparative visual emphasis
- empty/loading/error presentation
- selected visible range for rendering

## Allowed UI transformation

- changing visual grouping
- changing chart style
- adding tooltips or legends
- formatting numbers and dates for display
- visible-range slicing for presentation only

## Forbidden aggregate behavior

- recomputing total accuracy from partial local subsets
- deriving streak truth independently from UI state
- recomputing per-mode totals from visible list data only
- substituting placeholder or cached visual values as if they were fresh truth

## Safe UI derivations

These are acceptable because they remain presentation-only:

- axis tick labels
- chart point spacing
- visible-range slicing for rendering
- percentage formatting
- trend highlighting that does not invent new aggregate values

## Unsafe UI derivations

These are not acceptable unless Rust explicitly defines them:

- local aggregate recomputation
- inferred carryover or completion stats
- fallback summary totals when reports read fails
- derived streak or accuracy from partial in-memory subsets

## Error boundary

When reports read fails:

- show reports-level error state
- allow retry
- do not fabricate updated aggregate values

## Review question for feature work

When someone proposes a reports change, ask:

- is this changing how truth is shown
- or is this changing what truth is

If the answer is "what truth is", the change belongs on the Rust/domain side.

## Verification focus

- same report payload can be rendered with different chart layouts and still communicate the same truth
- UI changes cannot change aggregate values
- stale or missing data is never presented as authoritative current truth

## Exit criteria

- Report feature work has a clear answer for "is this UI or aggregate logic?"
- Flutter charts can evolve without threatening domain correctness.
