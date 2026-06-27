# Phase 13 Execution Notes

**Executed:** 2026-05-21

## Implemented

- Added shared Flutter-style tokens and reusable CSS utility classes in the mini program app stylesheet.
- Rebuilt the custom shell labels and drawer routes so Croc BTI and leaderboard remain drawer-only.
- Extended mini SDK contracts for plan growth rules, wordbooks, Croc BTI profile, report `modeSeries`, and study start `questionTypeWeights`.
- Added pure helpers for Croc BTI scoring/normalization, report chart point math, plan payload construction, and Study feed construction.
- Replaced Reports fake bars with data-backed tappable line charts and expandable mode cards.
- Replaced Croc BTI placeholder flow with saved answers/minutes, full scoring, editable plan input, question-type weights, profile save, plan save, apply-to-today, and sync flush.
- Replaced Plan placeholder controls with hydrated draft state, dirty state, wordbook selection, save, and apply-to-today.
- Replaced Study single-card flow with vertical `Swiper` feed items for answered/current/completion pages and plan-derived question-type weights at session start.
- Added report, plan-flow, Croc BTI, study-flow, and UI contract checks.

## Verification

- `npm.cmd run typecheck` - passed.
- `npm.cmd run test:ui-contract` - passed.
- `npm.cmd run test:croc-bti` - passed.
- `npm.cmd run test:reports` - passed.
- `npm.cmd run test:plan-flow` - passed.
- `npm.cmd run test:study-flow` - passed.
- `npm.cmd run build:weapp` - passed.

## DevTools Smoke Checklist

Manual WeChat DevTools checks still need to be performed after reopening the built `dist`:

- Today task buttons enter Study with the intended mode.
- Plan save/apply button states change.
- Reports daily chart point tap changes selected day.
- Reports mode card tap expands compact chart.
- Croc BTI can answer all questions, edit result weights, and apply plan.
- Study can answer vertically, retain answered pages, and show completion.
- Console does not show `TypeError: n[e] is not a function`.
- Console does not show `Error: timeout`.

Status for known previous runtime errors:

- `TypeError: n[e] is not a function` - no build-time recurrence; requires DevTools runtime smoke check.
- `Error: timeout` - no build-time recurrence; requires DevTools runtime smoke check.
