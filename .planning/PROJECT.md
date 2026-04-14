# Word Mobile RN

## Project Overview

Word Mobile RN is the mobile-first companion product for the existing Word Desktop app.

It is not intended to reimplement the learning engine in JavaScript. Instead, it will reuse the existing Rust-centered domain model and move the mobile user experience into a dedicated React Native shell.

## Core Value

Users can complete a stable daily vocabulary-learning workflow on phone, offline when needed, with reliable local persistence and without AI or sync features blocking the main learning loop.

## Product Direction

- Mobile shell: React Native
- Shared domain core: Rust crates
- Local persistence: SQLite
- Vocabulary source: imported and validated into local storage before runtime use
- AI: optional enhancement through Rust-side service boundary
- Sync: later capability, not part of MVP critical path

## Architecture Constraints

- Frontend does not own learning truth
- Rust core owns study scheduling, daily snapshot generation, reporting aggregation, vocabulary import rules, and AI boundaries
- SQLite remains the on-device source of truth
- AI must never block study completion
- Imported vocabulary content must be validated before becoming runtime data
- Mobile lifecycle interruptions must be explicitly handled

## Recommended Repository Shape

```text
word-mobile-rn/
  apps/
    mobile/
  packages/
    contracts/
    mobile-ui/
  crates/
    app-core/
    study-core/
    plan-core/
    vocab-core/
    report-core/
    ai-core/
    storage-core/
    platform-mobile/
  docs/
  .planning/
```

## Working Model

- Use the current desktop project as the source of truth for domain behavior during extraction
- Refactor toward shared crates before feature expansion
- Build mobile UI for touch-first flows rather than copying desktop information density
- Keep release quality focused on offline startup, persistence safety, and study-loop continuity

## Near-Term Success Definition

The mobile MVP is successful when a user can:

1. Install the app
2. Launch offline
3. Recover local state after restart
4. See today tasks
5. Complete at least one full study session
6. Persist results into reports and wrong-word state

## Reference Source

Primary source system:
- Existing desktop repository at `D:\projects\word-desktop-tauri`

This project should treat the desktop app as the behavior reference while it extracts the reusable core.
