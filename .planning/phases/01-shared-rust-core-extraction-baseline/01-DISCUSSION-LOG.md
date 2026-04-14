# Phase 1: Shared Rust Core Extraction Baseline - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md; this log preserves the alternatives considered.

**Date:** 2026-04-09
**Phase:** 01-Shared Rust Core Extraction Baseline
**Areas discussed:** shared core repository strategy, initial crate split, platform abstraction approach, migration safety and rollout

---

## Shared Core Repository Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| A | Extract shared crates inside the current desktop repository first, then let mobile reuse them | |
| B | Move shared crates directly into the mobile repository and let desktop depend on that repo | |
| C | Create a third dedicated shared-core repository used by both desktop and mobile | X |

**User's choice:** C
**Notes:** The user explicitly chose a dedicated shared-core repository even though it is more complex operationally than using the desktop repo as the first extraction home.

---

## Initial Crate Split

| Option | Description | Selected |
|--------|-------------|----------|
| A | Medium-granularity split with roughly `app-core`, `study-core`, `content-core`, and `storage-core` | X |
| B | Fine-granularity split with many domain crates from the start | |
| C | Very coarse split with one or two large crates first | |

**User's choice:** A
**Notes:** The user prefers a balanced extraction that avoids over-design in the first phase while still preventing a single monolithic shared crate.

---

## Platform Abstraction Approach

| Option | Description | Selected |
|--------|-------------|----------|
| A | Start with a coarse runtime boundary such as `PlatformServices` or `AppRuntime` | X |
| B | Start with many fine-grained traits for each platform capability | |
| C | Delay a unified abstraction and only extract pure business logic first | |

**User's choice:** A
**Notes:** The user prefers a practical first abstraction layer that can absorb current Tauri-specific assumptions before any later refinement.

---

## Migration Safety and Rollout

| Option | Description | Selected |
|--------|-------------|----------|
| A | Keep the desktop app compilable and key flows runnable throughout the migration | X |
| B | Permit a large refactor first and stabilize desktop afterward | |
| C | Run new and old implementations side by side for an extended period | |

**User's choice:** A
**Notes:** The user wants a conservative migration where desktop remains the active regression anchor during the extraction.

---

## the agent's Discretion

- Exact crate names within the chosen medium-granularity structure
- Exact temporary adapter placement during extraction
- Exact shape of the first coarse platform abstraction

## Deferred Ideas

- React Native bridge design and mobile native-module structure
- Final mobile app bootstrap strategy
- Later fine-grained crate decomposition
- Later fine-grained platform trait decomposition
