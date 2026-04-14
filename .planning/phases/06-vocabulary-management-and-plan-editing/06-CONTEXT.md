# Phase 6: Vocabulary Management and Plan Editing - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase brings the remaining user-controlled setup surfaces of the learning loop onto mobile: vocabulary management, vocabulary update state, and a deeper but still mobile-appropriate plan editing experience. It must preserve the existing last-known-good and fallback guarantees while making wordbook status and update outcomes understandable on phone. It does not cover reports, wrong-word review, or AI surfaces.

</domain>

<decisions>
## Implementation Decisions

### Vocabulary management information structure
- **D-01:** The mobile vocabulary management screen should be status-overview-first rather than list-first.
- **D-02:** Users should understand overall library health, update state, and study readiness before drilling into the individual wordbook list.
- **D-03:** The wordbook list remains important, but it should sit under the status summary rather than replacing it as the primary first impression.

### Update interaction model
- **D-04:** Vocabulary updates should be triggered through a single primary update action paired with a page-level status card.
- **D-05:** The mobile UI should not fragment update behavior into per-book update controls in this phase.
- **D-06:** Update progress, fallback state, and retry opportunities should all be legible from the same management surface rather than requiring a separate task-flow page.

### Plan editing depth in this phase
- **D-07:** Phase 6 should expand plan editing closer to the desktop plan editor rather than staying at the lighter Phase 4 depth.
- **D-08:** The mobile plan surface should now support substantially more complete configuration of targets, growth rules, and wordbook selection while still remaining mobile-appropriate in layout.
- **D-09:** This phase should move the plan surface beyond "quick edit" and toward "serious mobile configuration", but not at the expense of making the flow unreadable on phone.

### Failure and fallback presentation
- **D-10:** When an update fails but the last-known-good local corpus remains usable, the mobile UI must say that explicitly.
- **D-11:** Fallback states must explain both the failure and the fact that learning can continue with the current local dataset.
- **D-12:** Failure messaging should prioritize clarity and trust over minimalism, but it should not become a blocking full-screen interruption when study can still continue.

### the agent's Discretion
- Exact hierarchy and wording of the status overview card
- Exact balance between "near-desktop" plan depth and mobile readability
- Exact placement of retry and fallback explanation actions
- Exact visual differentiation between healthy, updating, failed-with-fallback, and degraded states

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile-first and offline-first constraints
- `.planning/REQUIREMENTS.md` - Phase 6 must satisfy `LIB-01`, `LIB-02`, `LIB-03`, and continue supporting plan configuration needs
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 6
- `.planning/STATE.md` - current planning status and prior locked decisions
- `.planning/phases/04-plan-and-today-mobile-surfaces/04-CONTEXT.md` - prior mobile plan-surface and Today-first navigation decisions
- `.planning/phases/05-study-session-mvp/05-CONTEXT.md` - prior study-loop assumptions that vocabulary and plan editing must continue to support

### Desktop reference implementation
- `../word-desktop-tauri/apps/desktop/src/features/library/library-page.tsx` - desktop vocabulary management structure and page composition
- `../word-desktop-tauri/apps/desktop/src/features/library/update-action.tsx` - desktop manual update action pattern
- `../word-desktop-tauri/apps/desktop/src/features/plans/active-wordbook-selector.tsx` - desktop wordbook selection behavior and limited-corpus signaling
- `../word-desktop-tauri/apps/desktop/src/lib/vocabulary-client.ts` - current vocabulary management contract surface
- `../word-desktop-tauri/apps/desktop/src/features/plans/plan-editor-page.tsx` - desktop plan depth reference for the larger mobile editing step in this phase

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src/lib/vocabulary-client.ts`: already defines the current wordbook list, import status, toggle, and start-import contract surface
- `../word-desktop-tauri/apps/desktop/src/features/library/library-page.tsx`: shows how status summary and wordbook list currently compose on desktop
- `../word-desktop-tauri/apps/desktop/src/features/library/update-action.tsx`: demonstrates the single manual update action pattern
- `../word-desktop-tauri/apps/desktop/src/features/plans/active-wordbook-selector.tsx`: already expresses limited-corpus and study-readiness logic for wordbook selection
- `../word-desktop-tauri/apps/desktop/src/features/plans/plan-editor-page.tsx`: provides the full current plan-edit depth that mobile will partially approach in this phase

### Established Patterns
- Vocabulary update is currently modeled as a managed import/status flow rather than per-book independent updating
- The product already distinguishes formal corpus, fallback behavior, and limited-corpus warning states
- Wordbook choice and plan depth are tightly linked to study readiness
- The desktop app already uses a combined "status + list + action" model for library management

### Integration Points
- Mobile vocabulary management should consume the shared vocabulary status and wordbook contracts stabilized in earlier phases
- The vocabulary screen should remain non-blocking when fallback keeps study available
- The deeper plan surface in this phase must still feed the same shared plan contracts used by onboarding, Today, and study
- Wordbook selection warnings and limited-corpus states should inform plan editing rather than live in a disconnected screen

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants vocabulary management to open with a status overview rather than a raw list.
- The user wants update behavior to stay centered on one primary action plus page-level status explanation.
- The user wants Phase 6 plan editing to move significantly closer to desktop depth, not remain in lightweight quick-edit mode.
- The user wants fallback failures explained clearly in a way that preserves trust by saying study can still continue.

</specifics>

<deferred>
## Deferred Ideas

- Reports, wrong-word notebook, and AI surfaces - later phases
- Per-book independent update mechanics - deferred unless the backend update model changes
- Full desktop-equal plan editing parity if it would compromise mobile readability
- More complex background-sync or scheduled update behaviors - later phase

</deferred>

---

*Phase: 06-vocabulary-management-and-plan-editing*
*Context gathered: 2026-04-09*
