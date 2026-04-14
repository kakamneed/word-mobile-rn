# Phase 2: Cross-Platform Contract and Desktop Refactor - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase defines the shared cross-platform contracts that desktop and mobile will consume, refactors the desktop app to use the extracted shared core through adapter layers, and locks the validation standard for the desktop-side refactor. The phase is about contract ownership, API surface shape, synchronization policy, and desktop refactor acceptance. It does not cover React Native bridge implementation or mobile UI delivery.

</domain>

<decisions>
## Implementation Decisions

### Contract source of truth
- **D-01:** Rust DTOs and Rust-side shared-core models are the source of truth for cross-platform contracts.
- **D-02:** TypeScript contract types should be generated from, or mechanically derived from, the Rust-owned contract layer rather than maintained as an independent truth source.
- **D-03:** Handwritten mirrored TypeScript interfaces should be reduced over time, especially where the desktop app currently duplicates Rust response shapes.

### API exposure shape
- **D-04:** Phase 2 will use a two-layer boundary: a small number of outer facades or use-case entrypoints for platform consumers, with finer-grained internal operations still available below them.
- **D-05:** Desktop and future mobile integrations should primarily target the outer facade layer rather than directly consuming many low-level operations.
- **D-06:** The inner fine-grained layer may remain available to support testing, orchestration, and gradual desktop migration without forcing all current command behavior into oversized facade methods.

### Contract versioning strategy
- **D-07:** Contracts will use strong synchronization during this phase rather than introducing a full explicit version-compatibility system.
- **D-08:** Desktop and mobile are expected to move in lockstep while contract stabilization is still in progress.
- **D-09:** Explicit multi-version compatibility and long-lived backward-compatibility rules are deferred until later phases introduce real multi-client release pressure.

### Desktop refactor acceptance scope
- **D-10:** Phase 2 is not considered complete unless the desktop refactor passes the main learning-loop acceptance scope rather than only a minimal bootstrap path.
- **D-11:** The required refactor acceptance surface is: bootstrap, today home, study flow, wrong-word notebook, and reports.
- **D-12:** Settings, legacy compatibility tools, and AI surfaces may be refactored in a later pass if the main learning loop is already safely running on shared-core contracts.

### the agent's Discretion
- Exact generation workflow for Rust-to-TypeScript contract types
- Exact facade names and grouping boundaries
- Which internal low-level operations remain directly callable for tests or transition adapters
- Which desktop commands are temporarily allowed to stay on older internals while the required acceptance surface is stabilized

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - mobile product constraints and architecture direction
- `.planning/REQUIREMENTS.md` - Phase 2 must satisfy `ARCH-01` and especially `ARCH-02`
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 2
- `.planning/STATE.md` - current project status and prior discussion outcomes
- `.planning/phases/01-shared-rust-core-extraction-baseline/01-CONTEXT.md` - prior locked decisions on repository strategy, crate granularity, platform boundary shape, and incremental migration safety

### Desktop contract examples
- `../word-desktop-tauri/apps/desktop/src/lib/tauri.ts` - current single invoke entry point from frontend to native layer
- `../word-desktop-tauri/apps/desktop/src/lib/today-home-client.ts` - handwritten TypeScript contract layer for Today home
- `../word-desktop-tauri/apps/desktop/src/lib/study-client.ts` - handwritten TypeScript contract layer for study session APIs
- `../word-desktop-tauri/apps/desktop/src-tauri/src/models/today_home_state.rs` - Rust DTO for Today home state with serialization contract hints
- `../word-desktop-tauri/apps/desktop/src-tauri/src/models/study_session.rs` - Rust DTO and enum surface for study session contract design
- `../word-desktop-tauri/apps/desktop/src-tauri/src/main.rs` - current command registration surface that desktop refactor will need to adapt

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src/lib/tauri.ts`: already centralizes frontend invocation, making it the natural place to reroute desktop through new shared-core-backed facades
- `../word-desktop-tauri/apps/desktop/src/lib/today-home-client.ts`: shows a clear frontend-friendly contract module that can later be generated or replaced by generated types
- `../word-desktop-tauri/apps/desktop/src/lib/study-client.ts`: provides the richest current example of desktop-side handwritten DTO mirroring and therefore the best candidate for contract stabilization work
- `../word-desktop-tauri/apps/desktop/src-tauri/src/models/today_home_state.rs`: already uses serde naming rules and tests to express JSON contract behavior

### Established Patterns
- Desktop frontend contract types are currently handwritten in TypeScript and mirror Rust DTOs manually
- Rust models already carry serialization details such as `camelCase` mapping and field renames, which supports Rust-owned contract truth
- The desktop app still exposes many command-level entrypoints from the Tauri shell, but those command groupings already suggest potential facade group boundaries such as bootstrap, study, today, reports, wrong words, settings, and vocabulary
- The current product's true regression anchor is the learning loop, not every peripheral command surface equally

### Integration Points
- The contract stabilization seam sits between shared Rust DTOs and desktop TypeScript client modules
- The desktop refactor seam sits between Tauri command handlers and the new shared-core facade layer
- The highest-value contract migration targets are Today home and Study because they already have explicit frontend TS clients and Rust DTO counterparts
- The required desktop refactor acceptance path must cover bootstrap, today, study, wrong words, and reports before Phase 2 can be closed

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants Rust to own the truth of contract definitions rather than creating a neutral shared-schema layer first.
- The user wants a two-layer API boundary so platform consumers see a cleaner facade, while implementation and test code can still use more granular operations below it.
- The user deliberately chose strong synchronization over early versioned compatibility because the project is still in active architectural migration.
- The user wants Phase 2 acceptance to prove the real learning loop, not just a minimal smoke test.

</specifics>

<deferred>
## Deferred Ideas

- Explicit long-lived contract versioning and backward-compatibility policy - deferred until multiple independently released clients create real pressure
- React Native bridge binding shape - belongs to Phase 3
- Full desktop parity across every non-core command surface - can be deferred beyond the main learning-loop acceptance scope if needed
- Neutral standalone schema system that is independent from Rust models - deferred unless the generated-contract pipeline proves insufficient

</deferred>

---

*Phase: 02-cross-platform-contract-and-desktop-refactor*
*Context gathered: 2026-04-09*
