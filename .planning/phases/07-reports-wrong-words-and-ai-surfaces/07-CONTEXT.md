# Phase 7: Reports, Wrong Words, and AI Surfaces - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase completes the mobile post-study review loop by adding report views, wrong-word review surfaces, and AI passage surfaces. It should turn the study data already produced in earlier phases into a coherent mobile review experience without blocking the core learning flow. It does not revisit the study-session interaction model itself or the broader release-hardening concerns of later phases.

</domain>

<decisions>
## Implementation Decisions

### Report home structure
- **D-01:** Mobile reports should be structured as an overview-first review experience with mode-specific drill-down beneath it.
- **D-02:** Users should first see an overall learning picture, then drill into specific study modes rather than landing immediately on mode tabs as the primary screen.
- **D-03:** Report design should remain mobile-readable and should not copy the desktop report density directly.

### Wrong-word interaction depth
- **D-04:** Phase 7 wrong-word review should focus on a strong list-plus-detail flow rather than expanding into heavy batch operations.
- **D-05:** Filtering can remain practical and useful, but the phase should prioritize high-quality review and inspection over bulk management mechanics.
- **D-06:** Wrong-word detail should be rich enough to support actual review rather than forcing everything into one long list screen.

### AI passage placement
- **D-07:** AI passages should live on a dedicated independent mobile page rather than being hidden inside wrong-word or report pages.
- **D-08:** The AI surface should still feel like part of the review loop, but the user wants it to have its own destination and not be subordinated to another page.
- **D-09:** AI remains a non-blocking enhancement and should not displace the central role of reports and wrong-word review.

### Overall information architecture
- **D-10:** Phase 7 should be organized as a single "review center" domain that contains reports, wrong words, and AI as coordinated internal destinations or segments.
- **D-11:** The mobile information architecture should unify these review surfaces rather than scattering them as unrelated top-level pages.
- **D-12:** The review center should still preserve clear boundaries between the three surfaces so users can understand where they are and what each area is for.

### the agent's Discretion
- Exact internal review-center navigation pattern
- Exact shape of the overview-first report landing screen
- Exact wrong-word detail depth and section ordering
- Exact entry path into the dedicated AI page inside the review center

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile-first and offline-first constraints
- `.planning/REQUIREMENTS.md` - Phase 7 must satisfy `WRNG-02`, `RPT-01`, `AI-01`, `AI-02`, and `AI-03`
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 7
- `.planning/STATE.md` - current planning status and prior phase decisions
- `.planning/phases/05-study-session-mvp/05-CONTEXT.md` - prior summary, next-round, and study completion decisions that feed into review surfaces
- `.planning/phases/06-vocabulary-management-and-plan-editing/06-CONTEXT.md` - prior state and trust decisions that continue to shape mobile review UX

### Desktop reference implementation
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/wrong-word-page.tsx` - desktop wrong-word notebook composition
- `../word-desktop-tauri/apps/desktop/src/features/reports/reports-page.tsx` - desktop reports composition and mode-switch structure
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/ai-passage-reader.tsx` - structured AI passage rendering reference
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/ai-passage-history-panel.tsx` - AI history browsing reference

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/wrong-word-page.tsx`: already defines the baseline list-plus-detail review shape for wrong words
- `../word-desktop-tauri/apps/desktop/src/features/reports/reports-page.tsx`: shows the current report composition and per-mode analysis model
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/ai-passage-reader.tsx`: already expresses a structured AI passage renderer that can inform mobile reading layout
- `../word-desktop-tauri/apps/desktop/src/features/wrong-words/ai-passage-history-panel.tsx`: provides a history model for prior generated passages

### Established Patterns
- Reports and wrong-word review are already distinct concepts in the product, even if both belong to the post-study review loop
- AI passages are optional and tied to review, but the user now wants them surfaced as their own destination rather than buried inside another screen
- The product already has the data contracts needed to support overview metrics, wrong-word detail, and AI history independently

### Integration Points
- The review center should receive users from Today, post-study summary, or secondary navigation
- Reports should consume persisted aggregates rather than temporary client counters
- Wrong-word detail should consume the existing notebook and detail contracts
- The dedicated AI page should still connect conceptually to the broader review center rather than becoming a disconnected side feature

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants reports to start with an overview and then allow mode drill-down.
- The user wants wrong-word review to stay at list-plus-detail depth in this phase.
- The user explicitly wants AI passages on their own independent page.
- The user wants all three surfaces grouped under one review-center information architecture rather than split into unrelated top-level destinations.

</specifics>

<deferred>
## Deferred Ideas

- Heavy batch wrong-word operations
- Directly desktop-equivalent report density on mobile
- AI being embedded only as a subsection of another review page
- Release hardening and analytics instrumentation beyond the review UI itself

</deferred>

---

*Phase: 07-reports-wrong-words-and-ai-surfaces*
*Context gathered: 2026-04-09*
