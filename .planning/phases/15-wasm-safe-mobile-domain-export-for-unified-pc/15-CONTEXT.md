# Phase 15 Context

## Phase Boundary

Phase 15 makes current mobile Rust learning behavior consumable by the unified PC product through a WASM-safe contract. It owns domain extraction, protocol/version identity, canonical fixtures, artifact generation, and cross-target equivalence. Word Net integration and PC UI/storage remain outside this phase.

## Implementation Decisions

- Treat Flutter-backed mobile behavior and its current Rust tests as truth.
- Extract the smallest coherent pure-domain surface instead of forcing storage-backed application crates to compile for WASM.
- Keep native repositories, SQLite transactions, paths, clocks requiring platform access, sync, and lifecycle in adapters.
- Use deterministic inputs for clocks/randomness where fixture equivalence depends on them.
- Publish structured JSON envelopes with explicit protocol and source versions.
- Preserve all existing mobile call paths during extraction; compatibility adapters are acceptable when they keep behavior unchanged.

## Verification Decisions

- Lock native canonical outputs first, then require byte-normalized or semantically identical WASM outputs.
- Cover the NewWord/Review distinction, question units, meanings, question rendering data, resume, wrong-word identity, and report/local-day contracts.
- Run focused Rust and Flutter regression tests for every moved rule boundary.
- Require actual Chromium, Firefox, and WebKit loading/execution checks plus size/startup/serialization measurements before declaring the artifact browser-ready.

## Deferred Ideas

- Sharing storage engines across browser and native PC targets.
- Cloud synchronization through the WASM package.
- UI component reuse between Flutter and React.
