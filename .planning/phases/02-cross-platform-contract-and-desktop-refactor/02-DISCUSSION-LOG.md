# Phase 2: Cross-Platform Contract and Desktop Refactor - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md; this log preserves the alternatives considered.

**Date:** 2026-04-09
**Phase:** 02-Cross-Platform Contract and Desktop Refactor
**Areas discussed:** contract source of truth, API exposure shape, contract versioning strategy, desktop refactor acceptance scope

---

## Contract Source of Truth

| Option | Description | Selected |
|--------|-------------|----------|
| A | Rust DTOs and Rust-side shared-core models are the source of truth for platform contracts | X |
| B | A separate independent contract layer is the source of truth for both Rust and TypeScript | |
| C | Continue with handwritten mirrored contracts on both sides and rely on tests to catch drift | |

**User's choice:** A
**Notes:** The user wants contract truth to stay aligned with the Rust-owned core rather than introducing a neutral schema layer in this phase.

---

## API Exposure Shape

| Option | Description | Selected |
|--------|-------------|----------|
| A | Keep the API close to current command granularity | |
| B | Expose only a small number of coarse facade entrypoints | |
| C | Use two layers: platform-facing facades on top, finer-grained internal operations below | X |

**User's choice:** C
**Notes:** The user wants a cleaner platform boundary without losing the flexibility needed for testing and gradual migration.

---

## Contract Versioning Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| A | Use strong synchronization for now and do not introduce explicit multi-version compatibility yet | X |
| B | Introduce explicit contract versioning and compatibility rules immediately | |
| C | Version only selected high-risk DTOs and keep the rest strongly synchronized | |

**User's choice:** A
**Notes:** The user prefers speed and clarity during architecture migration over early compatibility machinery.

---

## Desktop Refactor Acceptance Scope

| Option | Description | Selected |
|--------|-------------|----------|
| A | Minimum loop only: bootstrap + today + study | |
| B | Main learning loop: bootstrap + today + study + wrong words + reports | X |
| C | All current desktop command surfaces must migrate in this phase | |

**User's choice:** B
**Notes:** The user wants the refactor validated against the actual learning loop rather than a narrow smoke path, but does not require every peripheral command to migrate in the same phase.

---

## the agent's Discretion

- Exact contract generation workflow
- Exact facade grouping and naming
- Exact temporary desktop adapter shape
- Exact migration ordering within the required acceptance scope

## Deferred Ideas

- Explicit long-lived versioned compatibility policy
- React Native bridge details
- Full migration of non-core desktop command surfaces
- Independent neutral schema layer outside Rust
