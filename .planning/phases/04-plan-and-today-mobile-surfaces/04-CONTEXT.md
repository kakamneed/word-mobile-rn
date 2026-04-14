# Phase 4: Plan and Today Mobile Surfaces - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers the first true mobile-facing product surfaces for onboarding, today-home guidance, and active-plan access. It turns the runtime proved in Phase 3 into a usable mobile landing experience centered on today's tasks and a mobile-appropriate plan surface. It does not deliver the full study-session interaction flow or complete all desktop-equivalent configuration depth.

</domain>

<decisions>
## Implementation Decisions

### Today-home information density
- **D-01:** The mobile Today home should use a hybrid layout: the top of the screen focuses on the next action and today's summary, while deeper task details live below or in expandable sections.
- **D-02:** The mobile home must not replicate the desktop page's full information density on first view.
- **D-03:** Users should be able to understand what to do next immediately, then inspect breakdown and carry-over detail without leaving the page.

### Onboarding scope
- **D-04:** Phase 4 should include a full onboarding wizard rather than a minimal setup or no-wizard flow.
- **D-05:** The mobile onboarding should still be adapted for mobile pacing and touch interaction, but it must preserve the full setup arc: welcome, vocabulary selection, plan setup, and completion.
- **D-06:** The onboarding path should connect cleanly into the Today home rather than ending in a dead-end completion screen.

### Plan editing depth
- **D-07:** Phase 4 should support plan reading plus lightweight mobile editing rather than a full desktop-equivalent editor.
- **D-08:** The initial mobile plan surface should allow core target adjustments and practical wordbook control, while more complex editing depth can remain for later phases.
- **D-09:** The mobile plan surface must be sufficient to support the Today and onboarding flows without turning Phase 4 into a full configuration port.

### Mobile navigation structure
- **D-10:** Today should be the default mobile home screen after onboarding and app launch.
- **D-11:** Plan should exist as a secondary destination rather than a primary peer to Today in this phase.
- **D-12:** Navigation should optimize for the "what should I do now?" mobile question first, with plan access discoverable from Today and onboarding completion paths.

### the agent's Discretion
- Exact visual pattern for expanding or revealing deeper Today details
- Exact onboarding step visuals and pacing within the full wizard scope
- Exact quick-edit fields shown on the mobile plan surface
- Exact navigation mechanism used to move from Today into the plan screen

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile-first direction and touch-first experience constraints
- `.planning/REQUIREMENTS.md` - Phase 4 must support `PLAN-01`, `PLAN-03`, and prepare the way for `STUD-01`
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 4
- `.planning/STATE.md` - current planning status and prior phase decisions
- `.planning/phases/03-react-native-bootstrap-and-mobile-runtime/03-CONTEXT.md` - runtime prerequisites and real-bridge assumptions that Phase 4 builds on

### Desktop reference implementation
- `../word-desktop-tauri/apps/desktop/src/features/today/today-home-page.tsx` - desktop Today page reference for task-state composition
- `../word-desktop-tauri/apps/desktop/src/features/today/next-action-card.tsx` - focused next-action pattern worth preserving on mobile
- `../word-desktop-tauri/apps/desktop/src/features/plans/plan-editor-page.tsx` - desktop plan-edit reference to simplify for mobile
- `../word-desktop-tauri/apps/desktop/src/components/onboarding/onboarding-wizard.tsx` - full onboarding flow reference the mobile wizard should preserve conceptually

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src/features/today/today-home-page.tsx`: provides the current composition of today summary, next action, carry-over, and breakdown sections
- `../word-desktop-tauri/apps/desktop/src/features/today/next-action-card.tsx`: isolates the most mobile-valuable part of the Today experience
- `../word-desktop-tauri/apps/desktop/src/components/onboarding/onboarding-wizard.tsx`: already describes the complete first-run setup arc that the user wants preserved on mobile
- `../word-desktop-tauri/apps/desktop/src/features/plans/plan-editor-page.tsx`: shows which plan settings exist today and therefore which subset must be chosen for lightweight mobile editing

### Established Patterns
- The product already treats the Today screen as the primary task-orchestration surface
- Desktop onboarding is complete but likely too dense to port one-to-one onto mobile
- Plan editing currently includes more depth than Phase 4 mobile should expose
- The current user flow is strongest when next action is explicit rather than hidden behind static numbers

### Integration Points
- Mobile Today should consume the shared today-home contract already stabilized in Phase 2 and proven in Phase 3 runtime work
- Mobile onboarding should write through the shared plan and wordbook contracts rather than inventing client-only draft logic
- Mobile plan editing should reuse the same underlying plan DTOs while exposing only a reduced field set
- The end of onboarding and the default app launch destination should both land on Today

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants a hybrid Today screen: immediately actionable on top, richer breakdown below.
- The user wants the full onboarding journey to remain part of the mobile product instead of collapsing it into a minimal or implicit setup.
- The user wants plan editing in this phase to stay intentionally lighter than the desktop plan page.
- The user wants Today, not Plan, to be the default mobile home.

</specifics>

<deferred>
## Deferred Ideas

- Full mobile study-session answer flow - belongs to Phase 5
- Full-depth mobile plan editor parity with desktop - later phase
- Bottom-tab-first information architecture with Today and Plan as peer primary tabs - deferred unless later surfaces make it necessary
- Broader secondary surfaces such as reports or wrong words as first-class mobile navigation items - later phases

</deferred>

---

*Phase: 04-plan-and-today-mobile-surfaces*
*Context gathered: 2026-04-09*
