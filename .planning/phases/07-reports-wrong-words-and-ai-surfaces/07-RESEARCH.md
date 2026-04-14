# Phase 7: Reports, Wrong Words, and AI Surfaces - Research

**Date:** 2026-04-09
**Status:** Complete

## Goal

Determine how to complete the mobile post-study review loop by combining reports, wrong-word review, and AI passage reading into one coherent review-center experience.

## Key Findings

### 1. Desktop already proves that reports and wrong words are distinct but related review concepts

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/reports/reports-page.tsx`
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/wrong-word-page.tsx`

Implication:
- Mobile should group them under one review center without collapsing them into the same screen.
- Reports and wrong-word review can stay separate internal destinations while sharing a common review entry point.

### 2. Reports should not start mode-first on mobile

Evidence:
- Desktop reports page uses mode tabs as a primary control surface
- User locked overview-first report structure in `07-CONTEXT.md`

Implication:
- Mobile reports should first answer "How am I doing overall?" and only then offer per-mode drill-down.
- The mode switch becomes a secondary deeper control, not the first thing users see.

### 3. Wrong-word review is best kept at list-plus-detail depth in this phase

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/wrong-word-page.tsx`

Implication:
- A solid list-plus-detail mobile pattern is enough to deliver review value without overloading Phase 7 with heavy batch operations.
- Detail depth matters more than adding admin-style bulk controls.

### 4. AI passage already has strong renderer/history primitives and can stand alone

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/ai-passage-reader.tsx`
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/ai-passage-history-panel.tsx`

Implication:
- A dedicated AI page is viable without requiring AI to stay nested under wrong-word review.
- The mobile AI page should still feel like part of the review loop through shared entry and navigation, but it does not need to live inside another screen.

### 5. Phase 7 should feel like one review center, not three unrelated utilities

Evidence:
- User explicitly chose a unified review-center architecture
- Earlier phases already established Today-first learning and summary-driven completion flows

Implication:
- The mobile app should now have a review domain users can enter after study, from Today, or through navigation.
- Internal segmentation can separate reports, wrong words, and AI while preserving one coherent review mental model.

## Recommended Planning Shape

Phase 7 should be split into three executable plans:

1. Review center shell and overview-first reports
2. Wrong-word list-plus-detail review flow inside the review center
3. Dedicated AI page and AI history/reader integration inside the review center

## Risks To Plan Around

- If reports start too dense, the mobile review experience will feel desktop-ported.
- If wrong-word detail is too thin, the review center will feel shallow.
- If AI is made too separate, it may stop feeling like part of review; if too embedded, it will contradict the user's decision.

## Planning Guidance

- Use one review-center information architecture with clear internal destinations.
- Keep reports overview-first and mode-drill-down second.
- Prioritize a strong wrong-word detail experience over batch tooling.
- Give AI a dedicated page while still routing it through the review center.
