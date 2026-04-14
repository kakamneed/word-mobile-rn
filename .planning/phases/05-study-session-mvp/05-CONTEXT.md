# Phase 5: Study Session MVP - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase delivers the first real mobile study-session experience from question rendering through answer submission, feedback, completion summary, and interruption recovery. It must use the shared study contracts and persistence rules already established in earlier phases, and it must be good enough to complete the main phone study loop from Today into a persisted result. It does not include the broader reports or wrong-word product surfaces beyond what is required to preserve session correctness.

</domain>

<decisions>
## Implementation Decisions

### Answer interaction rhythm
- **D-01:** Mobile study sessions should preserve the two-step interaction rhythm: answer, then explicit submit, then explicit next-question progression.
- **D-02:** Mobile should adapt the controls and layout for touch, but it should not collapse the learning loop into auto-submit or auto-advance behavior.
- **D-03:** Feedback must remain visible between submit and next-question progression rather than being skipped by automatic transitions.

### Question layout and progressive disclosure
- **D-04:** Mobile question layout should use progressive disclosure instead of rendering all possible detail at once.
- **D-05:** The minimum always-visible question information must include the prompt plus the phonetic information where applicable.
- **D-06:** Examples, hints, and richer supporting context can expand progressively based on state, question type, or user action.
- **D-07:** Progressive disclosure must not hide the phonetic information that the user explicitly wants in the minimum visible payload.

### Interruption recovery
- **D-08:** If a study session is interrupted, the app should offer an explicit recovery choice rather than forcing continuation or silently discarding progress.
- **D-09:** The recovery prompt should let users continue the active session or abandon it and return to Today.
- **D-10:** Recovery behavior must preserve the underlying persisted truth and should not reset session state silently.

### Completion and exit behavior
- **D-11:** Session completion should land on a summary screen before the user leaves the session.
- **D-12:** The summary screen should offer two explicit next steps: return to Today and continue into the next round when that path is available.
- **D-13:** Completion should not immediately dump the user back to Today without visible closure.

### the agent's Discretion
- Exact control placement for submit and next buttons on mobile
- Exact trigger pattern for expanding hints/examples
- Exact wording and structure of the interruption-recovery prompt
- Exact visual treatment of the summary and "continue next round" branch

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile-first and offline-first constraints
- `.planning/REQUIREMENTS.md` - Phase 5 must satisfy `STUD-01`, `STUD-02`, `STUD-03`, `STUD-04`, and `STUD-05`
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 5
- `.planning/STATE.md` - current planning status and prior phase decisions
- `.planning/phases/04-plan-and-today-mobile-surfaces/04-CONTEXT.md` - Today-first entry assumptions that feed users into study

### Desktop study reference implementation
- `../word-desktop-tauri/apps/desktop/src/features/study/study-session-page.tsx` - current study-session interaction pattern and feedback flow
- `../word-desktop-tauri/apps/desktop/src/lib/study-client.ts` - desktop-side study DTO and contract surface
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs` - study-launch, submit, advance, complete, and persistence behavior reference

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src/lib/study-client.ts`: already defines the current study contract surface for launch, submit, advance, completion, and summary
- `../word-desktop-tauri/apps/desktop/src/features/study/study-session-page.tsx`: already contains the clearest current representation of the answer, submit, feedback, next, and summary rhythm
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs`: already defines the true backend sequencing and persistence behavior that mobile must respect

### Established Patterns
- The current study flow already separates answering, submit, and next-question progression
- Feedback is an explicit session phase, not an invisible transient state
- Completion already produces a summary and next-action signal from the backend
- The product's learning correctness depends on preserving the current shared Rust-side sequencing and persistence semantics

### Integration Points
- Mobile study will be entered from the Today-first navigation decided in Phase 4
- Mobile UI must consume the shared study launch, submit, advance, and complete contracts rather than inventing client-side sequencing
- Interruption recovery must connect the mobile lifecycle back into the session state exposed by the runtime and shared study facades
- Completion should hand users back to Today or into next round using backend-provided next-action semantics

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants mobile to keep the two-step submit/next rhythm rather than becoming an auto-advance quiz.
- The user explicitly requires phonetic information to remain part of the minimum visible question payload.
- The user wants interruption recovery to be a user-visible decision, not a silent reset or forced resume.
- The user wants completion to end in a summary screen with both "return to Today" and "continue next round" branches.

</specifics>

<deferred>
## Deferred Ideas

- Full reports and wrong-word product surfaces - later phases
- Broader study analytics inside the session UI - later phases
- More experimental one-handed or gesture-only study interaction models - deferred until the core MVP loop is stable
- Any attempt to collapse submit/feedback/next into a faster but less explicit study rhythm - deferred unless later user testing justifies it

</deferred>

---

*Phase: 05-study-session-mvp*
*Context gathered: 2026-04-09*
