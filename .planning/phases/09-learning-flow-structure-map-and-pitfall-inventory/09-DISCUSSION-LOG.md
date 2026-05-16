# Phase 09: Learning flow structure map and pitfall inventory - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md; this log preserves the alternatives considered.

**Date:** 2026-05-13
**Phase:** 09-learning-flow-structure-map-and-pitfall-inventory
**Areas discussed:** Phase scope, implementation target, historical pitfalls, cleanup handoff

---

## Phase Scope

| Option | Description | Selected |
|--------|-------------|----------|
| Whole repo learning flow | Map Flutter, legacy React Native, Rust, database, AI, and all historical paths as active implementation surfaces | |
| Flutter-only learning flow | Treat Flutter as the active mobile product surface; include Rust/data layers only where Flutter uses them | yes |
| Existing 07.4 continuation | Fold this work back into the previous parity/polish stage | |

**User's choice:** 只关注flutter版

**Notes:** The initial request asked for Today, answering, AI, database, and skills across the learning page cleanup. After new phases were created and Phase 09 discussion began, the user explicitly narrowed the active implementation target to Flutter only. React Native should be treated as legacy residue or historical context only.

---

## Historical Pitfalls

| Pitfall | Description | Required Treatment |
|---------|-------------|--------------------|
| Selected wrong option not red | User selected an incorrect multiple-choice option but UI did not clearly mark that selected option as wrong | Record as a cross-layer UI/result-shape pitfall and require regression coverage |
| Correct answer collapses to A | No matter which option is correct, final answer path can drift toward A due to stale label fallback or wrong authority | Record as an authority/index pitfall and require A/B/C/D preservation tests |
| Stub or stale truth | Old mobile/RN/stub paths can look valid after many version changes | Classify every active vs stale path before cleanup |
| Empty adapters | Bridge/client layers may exist without being true current owners | Identify as keep/delete/replace before Phase 10 |

**User's choice:** Capture all known pitfalls and clean old residue in later phases.

**Notes:** Existing Flutter test surface already includes selected wrong option and stale correct-label cases, but Phase 09 should map the full chain from Flutter display state back through SDK, bridge, Rust question builder/evaluator, and persistence.

---

## Cleanup Handoff

| Option | Description | Selected |
|--------|-------------|----------|
| Map first, clean later | Phase 09 produces structure map and cleanup acceptance gates; Phase 10 performs code cleanup | yes |
| Clean while mapping | Delete stale paths during the mapping phase | |
| Document only | Produce docs without setting up Phase 10 cleanup constraints | |

**User's choice:** Use multiple new phases, with Phase 09 as the structure/pitfall phase.

**Notes:** This keeps destructive cleanup out of the discussion/context phase and gives Phase 10 a safer plan surface.

---

## Canonical Decision Summary

- Flutter is the only active app surface for this work.
- Rust bridge/core/storage remain in scope because Flutter depends on them for learning truth.
- React Native paths are legacy context, not active implementation targets.
- Phase 09 should prepare cleanup and skillization rather than directly modifying implementation code.
