# Word Mobile RN

React Native mobile client for the Word learning product, built around a shared Rust core extracted from the desktop app.

## Goal

Migrate the current desktop-first vocabulary learning app to mobile without rewriting the core study rules, offline storage model, or vocabulary pipeline.

## Architecture Direction

- Mobile shell: React Native
- Shared domain core: Rust crates
- Local persistence: SQLite on device
- AI boundary: Rust-side service layer, non-blocking to main study flow
- Product rule: offline-first, local-first, study-loop-first

## Planning Docs

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `PROJECT_PLAN.md`

## Recommended Next Step

Start with the Phase 1 bootstrap work from `.planning/ROADMAP.md`, then create a spike branch to validate:

1. Rust core crate extraction
2. React Native to Rust bridge strategy
3. Mobile SQLite runtime path and migration behavior
