# Phase 6: Vocabulary Management and Plan Editing - Research

**Date:** 2026-04-09
**Status:** Complete

## Goal

Determine how to bring vocabulary management and a meaningfully deeper plan editor onto mobile without violating the existing fallback, last-known-good, and study-readiness rules.

## Key Findings

### 1. Vocabulary management is best modeled as state plus action, not list plus action alone

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/library/library-page.tsx`
- `../word-desktop-tauri/apps/desktop/src/lib/vocabulary-client.ts`

Implication:
- Mobile should lead with import/runtime health and current source state, then present the wordbook list underneath.
- This aligns better with the user's locked "status-overview-first" choice and the underlying system truth.

### 2. The update model is centralized already, so the mobile UI should not fake per-book updates

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/library/update-action.tsx`
- `../word-desktop-tauri/apps/desktop/src/lib/vocabulary-client.ts`

Implication:
- A single primary update button plus a strong status card matches the actual backend pipeline.
- Per-book update controls would create a misleading mental model unless the backend changes later.

### 3. Limited-corpus and fallback semantics are part of product trust, not incidental edge states

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/plans/active-wordbook-selector.tsx`
- `../word-desktop-tauri/apps/desktop/src/lib/vocabulary-client.ts`
- Prior runtime and seed decisions from earlier mobile phases

Implication:
- Mobile must clearly explain when updates failed but last-known-good data is still usable.
- Mobile should also preserve limited-corpus warnings where they influence study readiness or plan editing decisions.

### 4. The desktop plan editor defines the depth boundary that mobile is now intentionally approaching

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/plans/plan-editor-page.tsx`

Implication:
- Phase 6 should not remain at simple quick-edit depth.
- Mobile can now move closer to the desktop capability set by bringing in more target fields, growth-rule configuration, and stronger wordbook selection flow.
- However, it still needs mobile-appropriate grouping and stepwise layout to avoid becoming unreadable.

### 5. Vocabulary management and plan editing should reinforce each other

Evidence:
- Desktop wordbook selection and limited-corpus logic already affect plan readiness
- Earlier mobile phases made Today and Study depend on plan and runtime state

Implication:
- The mobile user should be able to understand from the vocabulary screen whether the current data is healthy enough for study.
- The plan editor should reflect the consequences of wordbook availability, limited corpus, and update/fallback state rather than acting as a disconnected form.

## Recommended Planning Shape

Phase 6 should be split into three executable plans:

1. Status-first vocabulary management screen and centralized update action
2. Fallback/failure messaging, retry flow, and wordbook list management
3. Deeper mobile plan editor approaching desktop capability while preserving mobile readability

## Risks To Plan Around

- If the vocabulary page becomes too list-centric, users may miss whether study is still safe after a failed update.
- If the plan editor jumps straight to full desktop parity, mobile readability will suffer.
- If fallback explanation is too weak, users may wrongly assume vocabulary is broken and stop studying.

## Planning Guidance

- Lead vocabulary management with trust and state, not just toggles.
- Keep update as a single primary action.
- Move the plan editor materially closer to desktop capability, but use mobile grouping rather than a direct layout port.
- Make fallback states explicit and reassuring when study can continue.
