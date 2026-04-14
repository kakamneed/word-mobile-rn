# Phase 4: Plan and Today Mobile Surfaces - Research

**Date:** 2026-04-09
**Status:** Complete

## Goal

Determine how to turn the proven mobile runtime into the first usable mobile product experience for onboarding, today-home guidance, and lightweight plan access.

## Key Findings

### 1. The desktop Today page already reveals the right content buckets, but not the right mobile density

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/today/today-home-page.tsx`
- `../word-desktop-tauri/apps/desktop/src/features/today/next-action-card.tsx`

Implication:
- Mobile should keep the same semantic building blocks: next action, progress summary, breakdown, carry-over, and plan context.
- It should not copy the desktop screen's density one-to-one.
- A hybrid structure is the best fit: next action and summary at the top, richer details further down or behind progressive disclosure.

### 2. The next-action pattern is the most important mobile carry-forward

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/today/next-action-card.tsx`

Implication:
- Mobile Today should anchor around "what should I do now?" rather than statistics-first design.
- Today should remain the default home because it naturally orients the user toward the next study action.

### 3. Desktop onboarding is complete enough to preserve conceptually, but too heavy to port directly

Evidence:
- `../word-desktop-tauri/apps/desktop/src/components/onboarding/onboarding-wizard.tsx`

Implication:
- The mobile app can keep the full onboarding arc: welcome, vocabulary selection, plan setup, completion.
- Each step should be simplified and touch-first, but the flow should remain explicit rather than hidden behind in-page prompts.

### 4. Full desktop plan editing is too deep for this phase

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/plans/plan-editor-page.tsx`

Implication:
- Mobile Phase 4 should support reading and lightweight editing only.
- The core fields to preserve are active wordbooks and primary per-day targets.
- Complex growth-rule detail can remain out of scope until later.

### 5. Phase 4 should form a coherent pre-study product loop

Evidence:
- Roadmap Phase 4 goal and success criteria
- Prior phase decisions: runtime already proved, study interaction deferred to Phase 5

Implication:
- The mobile app should be able to go from first launch -> onboarding -> Today -> plan check/adjust -> readiness for study.
- It does not need to complete the full study loop yet.

## Recommended Planning Shape

Phase 4 should be split into three executable plans:

1. Mobile navigation and onboarding flow
2. Hybrid Today home surface with next action and expandable deeper detail
3. Lightweight plan surface and Today-centered navigation integration

## Risks To Plan Around

- If onboarding and Today are designed independently, the handoff will feel disjointed.
- If the plan surface becomes too deep, it will consume Phase 4 and delay Phase 5.
- If Today lacks enough detail below the fold, users may not trust the daily task picture.

## Planning Guidance

- Keep Today as the default post-onboarding landing screen.
- Preserve the full onboarding arc, but simplify each step for mobile.
- Favor a quick-entry Today surface with deeper information progressively revealed.
- Keep plan editing intentionally smaller than the desktop editor.
