# Phase 5: Study Session MVP - Research

**Date:** 2026-04-09
**Status:** Complete

## Goal

Determine how to turn the existing study-session domain logic into a mobile-first interaction flow that preserves correctness while fitting touch interaction and interruption recovery needs.

## Key Findings

### 1. The existing backend sequencing already matches the desired mobile correctness model

Evidence:
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs`
- `../word-desktop-tauri/apps/desktop/src/lib/study-client.ts`

Implication:
- Mobile should preserve launch -> answer -> submit -> feedback -> advance -> complete instead of inventing a new client-side session machine.
- The safest implementation is to keep the shared Rust-side sequencing authoritative and let mobile focus on presentation and lifecycle handling.

### 2. The desktop study page already encodes the right high-level interaction rhythm

Evidence:
- `../word-desktop-tauri/apps/desktop/src/features/study/study-session-page.tsx`

Implication:
- The two-step submit/next pattern and explicit feedback phase should be preserved.
- Mobile should simplify layout and controls, not the correctness rules.

### 3. Progressive disclosure is required for mobile density, but phonetics must stay visible

Evidence:
- User decision captured in `05-CONTEXT.md`
- Desktop page already conditionally reveals examples, hints, and some feedback states

Implication:
- The mobile question card should always show prompt plus phonetic information where applicable.
- Examples, hints, and extended support content can expand progressively to reduce clutter.

### 4. Interruption recovery belongs in the mobile shell, not just the study screen

Evidence:
- Phase 3 locked a real RN runtime and blocking startup behavior
- Study-session state currently lives in backend-owned active session logic

Implication:
- Mobile needs an app-level recovery handshake when a live session exists.
- Recovery should not silently destroy or forcibly resume state; the user chose an explicit continue-or-abandon decision.

### 5. Completion should use backend summary semantics, but mobile can present stronger next-step affordances

Evidence:
- Desktop flow already returns summary plus `nextAction`
- User decision explicitly chose summary plus dual exits: Today and next round

Implication:
- Mobile should consume backend summary and next-action fields, then present a more explicit post-session branching UI.
- This keeps behavior consistent while improving mobile completion clarity.

## Recommended Planning Shape

Phase 5 should be split into three executable plans:

1. Study-session shell, question rendering, and two-step answer flow
2. Progressive disclosure, phonetic-first presentation, and interruption recovery
3. Completion summary, next-round branching, and persistence validation

## Risks To Plan Around

- If mobile adds too much client-side session logic, it may drift from the shared Rust truth.
- If phonetics are treated as optional detail, the implementation will violate a locked user decision.
- If interruption recovery is buried inside one screen only, app-level session resumption may feel brittle.

## Planning Guidance

- Keep the backend session machine authoritative.
- Make phonetics part of the default question payload.
- Build interruption recovery as a first-class mobile flow, not an afterthought.
- Use the summary screen as the explicit bridge back to Today or into the next round.
