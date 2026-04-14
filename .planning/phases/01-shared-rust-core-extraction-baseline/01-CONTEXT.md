# Phase 1: Shared Rust Core Extraction Baseline - Context

**Gathered:** 2026-04-09
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase extracts the reusable Rust business core out of the current desktop Tauri app so the desktop shell and future React Native mobile shell can share the same domain logic. The phase covers crate boundaries, platform abstraction shape, and migration safety strategy. It does not add new mobile features or redesign the product UI.

</domain>

<decisions>
## Implementation Decisions

### Shared core repository strategy
- **D-01:** The shared Rust core will live in a separate dedicated repository rather than staying inside the desktop repository or moving directly into the mobile repository.
- **D-02:** During the extraction period, the existing desktop repository remains the behavior reference and source of truth for domain correctness, but the long-term ownership target for shared crates is the dedicated shared-core repo.

### Initial crate split
- **D-03:** Phase 1 uses a medium-granularity crate layout rather than a highly fragmented layout.
- **D-04:** The first extraction target should group code into approximately four buckets: `app-core`, `study-core`, `content-core`, and `storage-core`.
- **D-05:** Fine-grained crates such as separate `plan`, `report`, `ai`, or `platform-*` crates can be introduced later if the medium split proves too coarse.

### Platform abstraction approach
- **D-06:** Platform dependencies should first be abstracted behind a coarse runtime boundary such as `PlatformServices` or `AppRuntime`.
- **D-07:** The first runtime boundary should cover path resolution, bundled resource access, storage bootstrap, clock/time access, network-adjacent helpers, and external-open behavior instead of introducing many separate traits immediately.
- **D-08:** Fine-grained trait decomposition is explicitly deferred until the mobile bridge and runtime integration reveal real pressure points.

### Migration safety and rollout
- **D-09:** The desktop app must remain compilable throughout the extraction work, and key behavior chains must remain runnable as the migration progresses.
- **D-10:** The migration may use temporary adapter bridges where necessary, but it must not rely on a single large "break everything then repair later" refactor.
- **D-11:** The desktop app remains the regression anchor until the shared core has passed through desktop integration successfully.

### the agent's Discretion
- Exact crate names and final crate count within the medium-granularity strategy
- The precise shape of the coarse runtime abstraction
- Whether a temporary compatibility layer should sit at command-handler level or service-entry level

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Mobile migration planning docs
- `.planning/PROJECT.md` - product direction, constraints, and success definition for the mobile project
- `.planning/REQUIREMENTS.md` - architecture requirements that Phase 1 must satisfy, especially `ARCH-01`, `ARCH-02`, and `ARCH-03`
- `.planning/ROADMAP.md` - official scope and success criteria for Phase 1
- `.planning/STATE.md` - active assumptions and current project status
- `PROJECT_PLAN.md` - repository-shape target and migration sequencing rationale

### Desktop reference implementation
- `../word-desktop-tauri/apps/desktop/src-tauri/src/main.rs` - current Tauri shell entry point and command registration surface
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/bootstrap.rs` - example of a thin command wrapper over service logic
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs` - command-heavy integration surface that currently mixes runtime prep, persistence access, session orchestration, and domain updates
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/mod.rs` - current service-module inventory showing likely extraction boundaries
- `../word-desktop-tauri/apps/desktop/src-tauri/src/persistence/app_paths.rs` - current Tauri-specific path and bundled-resource assumptions that must move behind a platform boundary
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/runtime_storage_service.rs` - runtime-root preparation and migration logic that depends on platform-specific storage behavior
- `../word-desktop-tauri/apps/desktop/src/lib/tauri.ts` - current frontend invoke boundary used by the desktop shell

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/mod.rs`: already groups the business logic by domain area, which makes it the best starting map for crate extraction
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/bootstrap.rs`: demonstrates that some command handlers are already thin adapters and can stay at shell level
- `../word-desktop-tauri/apps/desktop/src/lib/tauri.ts`: provides a single frontend invocation entry point, which is useful later when aligning desktop and mobile contracts

### Established Patterns
- The desktop app currently uses Tauri command handlers as the only frontend-to-native boundary
- Business logic is mostly service-oriented, but some command modules still mix orchestration, persistence access, and runtime preparation in one file
- The persistence layer and runtime path resolution still assume direct Tauri `AppHandle` access
- The desktop app treats startup, study session execution, and vocabulary readiness as critical product chains that must not drift during migration

### Integration Points
- The first extraction seam should sit between `commands/*` and `services/*`
- Tauri-specific runtime dependencies currently flow through `persistence/app_paths.rs` and services such as `runtime_storage_service.rs`
- `commands/study.rs` is a high-value integration target because it touches session orchestration, persistence updates, snapshot reads, and runtime setup all in one place
- The future desktop shell should call shared-core entrypoints through a thin adapter instead of directly owning business logic

</code_context>

<specifics>
## Specific Ideas

- The user explicitly chose a dedicated third repository for the shared Rust core even though it increases repository-management complexity.
- The user prefers a medium-granularity extraction rather than either a giant domain-core crate or a fully fragmented crate graph.
- The user wants a conservative migration strategy where desktop remains the behavior anchor throughout the extraction.

</specifics>

<deferred>
## Deferred Ideas

- Exact React Native bridge technology and mobile-side native module structure - belongs to Phase 3
- Final mobile repository bootstrapping and app shell setup - belongs to Phase 3
- Fine-grained crate decomposition beyond the initial medium split - deferred until Phase 1 or Phase 2 exposes pressure points
- Fine-grained platform trait decomposition - deferred until post-extraction runtime needs are clearer

</deferred>

---

*Phase: 01-shared-rust-core-extraction-baseline*
*Context gathered: 2026-04-09*
