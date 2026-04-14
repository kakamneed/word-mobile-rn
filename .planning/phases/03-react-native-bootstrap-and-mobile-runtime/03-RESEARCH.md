# Phase 3: React Native Bootstrap and Mobile Runtime - Research

**Date:** 2026-04-09
**Status:** Complete

## Goal

Determine the safest and most direct way to prove mobile runtime viability using the existing desktop runtime logic as the behavior reference.

## Key Findings

### 1. Bootstrap should stay blocking on hard runtime failures

Evidence:
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/bootstrap_service.rs`
- The desktop app already treats runtime-storage prep, database path resolution, database init, and missing required bundled snapshot as startup blockers.

Implication:
- Mobile bootstrap should preserve this contract instead of inventing a permissive degraded shell.
- Phase 3 needs an explicit mobile-ready bootstrap state model that can drive a visible blocking error screen.

### 2. Runtime path logic must move behind a mobile platform adapter

Evidence:
- `../word-desktop-tauri/apps/desktop/src-tauri/src/persistence/app_paths.rs`
- The current implementation depends directly on Tauri app path helpers and desktop-oriented runtime roots.

Implication:
- A `platform-mobile` adapter needs to resolve sandbox-safe paths for database, config, logs, vocab cache, and bundled resources.
- Shared core should consume path/resource capabilities from the coarse runtime boundary locked in Phase 1.

### 3. Formal bundled seed data is already the right behavioral baseline

Evidence:
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/local_seed_service.rs`
- The desktop app already differentiates formal local baseline data from repair seed behavior and treats limited corpus states as insufficient.

Implication:
- Mobile should bundle formal seed resources directly rather than reintroducing a tiny repair-only baseline.
- Phase 3 should include installation or first-use exposure of those resources inside the mobile sandbox.

### 4. Settings and bootstrap are good first bridge surfaces, but not sufficient alone

Evidence:
- `../word-desktop-tauri/apps/desktop/src-tauri/src/services/settings_service.rs`
- Settings read/write is relatively self-contained and contract-stable.
- Bootstrap is also a compact facade candidate.

Implication:
- These are ideal first RN bridge entrypoints.
- However, the user explicitly wants stronger validation than simple bootstrap/status reads, so one real study-launch path must also be included before Phase 3 closes.

### 5. Study launch is the minimum runtime-proof path for the learning core

Evidence:
- `../word-desktop-tauri/apps/desktop/src-tauri/src/commands/study.rs`
- The current study-launch flow touches runtime preparation, database open, local seed readiness, snapshot reads, and session generation.

Implication:
- A successful mobile-side study launch is a stronger proof of runtime viability than bootstrap alone.
- Phase 3 can stop at session launch and does not need the full answer-submit-next mobile UX yet.

## Recommended Planning Shape

Phase 3 should be split into three executable plans:

1. RN shell plus real mobile native bridge skeleton
2. Mobile sandbox storage, SQLite init/migration, and formal bundled seed installation
3. Blocking bootstrap UX plus runtime smoke validation through bootstrap/settings/today-home and one real study launch

## Risks To Plan Around

- Mobile resource packaging may diverge between iOS and Android if asset paths are not normalized early.
- SQLite and seed install order must be deterministic; otherwise cold start can become flaky.
- A real native bridge in Phase 3 raises build complexity, so plan scope must stay tight and avoid delivering extra UI.

## Planning Guidance

- Keep mobile runtime validation centered on real native integration, not mocks.
- Preserve desktop startup severity rules on mobile.
- Treat formal bundled seed data as mandatory runtime infrastructure.
- Require a single real study-launch smoke path before calling the phase complete.
