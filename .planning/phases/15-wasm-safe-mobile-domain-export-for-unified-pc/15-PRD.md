# Phase 15 PRD: WASM-safe Mobile Domain Export for Unified PC

## Objective

Expose the current mobile Rust domain behavior as a deterministic, versioned, WASM-safe package that Word Net can consume for both browser and Tauri delivery.

## Locked Decisions

- The current Flutter-backed Rust implementation is authoritative when older desktop or Word Net behavior differs.
- This phase extracts and isolates existing rules; it does not rewrite product behavior.
- Do not attempt to compile `app-core` or storage-heavy crates wholesale to WASM. Create or refine a pure domain boundary with no `rusqlite`, filesystem, network, Flutter, or Tauri dependency.
- SQLite repositories and application lifecycle remain native adapter concerns.
- The boundary uses additive versioned JSON request/result envelopes and structured errors.
- Word Net must pin both the protocol version and source commit/hash of the generated artifact.
- Completion requires native/WASM fixture equivalence and real browser evidence, not only a successful Rust build.

## Required Behavior Fixtures

- NewWord uses type-major rounds and exactly four questions per selected word.
- Review and other non-NewWord modes do not inherit the four-question loop.
- Plans, Today progress, carry-over, and session summaries use question counts consistently.
- A word has one canonical accepted-meaning set across its NewWord questions.
- Question payloads preserve type-specific visibility, phonetics, examples, translations, and per-word distractors without answer leakage.
- Active-session resume preserves answered/current state and does not restart from question one.
- Wrong-word and report projections retain real word identity and local-day semantics.

## Deliverables

- A WASM-safe crate boundary and native/WASM runners.
- Versioned request/result/error schema and compatibility checks.
- Canonical JSON fixture corpus derived from current mobile production behavior.
- Reproducible WASM artifact generation with source identity metadata.
- Focused mobile regression gates and Chromium/Firefox/WebKit consumption evidence.

## Out of Scope

- Word Net UI or persistence implementation.
- Flutter visual redesign.
- Moving SQLite into WASM.
- Replacing mobile bridges or changing product rules beyond compatibility-preserving extraction.
