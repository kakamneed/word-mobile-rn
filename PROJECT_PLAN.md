# Word Mobile RN Project Plan

## 1. Project Intent

This project is the mobile evolution of Word Desktop Tauri.

The chosen direction is:

- Keep the proven study, planning, reporting, vocabulary, and AI domain logic in Rust
- Extract that logic into shared crates
- Build a dedicated React Native mobile app as the new mobile shell
- Keep offline-first behavior and local SQLite as first-class constraints

This is not a simple UI port. It is a platform split:

- Desktop remains a Tauri shell
- Mobile becomes a React Native shell
- Both consume the same Rust core through platform-specific adapters

## 2. Why Scheme B

Scheme B is the best fit because it balances long-term product quality and asset reuse:

- The current project already concentrates the most important product rules in Rust services
- React Native is better suited than a webview-style mobile shell for navigation, gestures, lifecycle, notifications, and app-store distribution
- The team can still reuse TypeScript product thinking and DTO contracts
- Shared Rust crates protect the most valuable logic from divergence across desktop and mobile

## 3. Migration Principle

The migration sequence should be:

1. Extract platform-neutral Rust core
2. Define stable cross-platform contracts
3. Refactor desktop to consume the new core
4. Build the mobile shell on top of the same core
5. Only then expand into sync, background refresh, and mobile-native enhancements

This keeps the desktop product working while the mobile product is being built.

## 4. Target Architecture

```text
word-mobile-rn/
  apps/
    mobile/
      src/
      android/
      ios/
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
    architecture.md
    migration-map.md
    bridge-notes.md
```

Recommended shared shape across repositories:

- Rust core owns study rules, daily snapshot generation, reporting aggregation, vocabulary normalization/import, AI prompt validation, and storage schema
- Mobile TypeScript owns navigation, rendering, transient page state, optimistic UI, and device-specific interaction handling
- Platform adapters own filesystem paths, SQLite opening, background tasks, secure storage, and network/lifecycle hooks

## 5. Main Workstreams

### Workstream A: Core extraction

- Move desktop Rust services into domain-focused crates
- Separate business logic from Tauri command handlers
- Replace Tauri-specific path and runtime assumptions with platform traits

### Workstream B: Contract stabilization

- Define DTOs and command contracts shared by desktop and mobile
- Make serialization stable and versioned
- Introduce compatibility tests for cross-platform request/response shapes

### Workstream C: Mobile shell

- Build React Native app shell
- Implement onboarding, today page, study session flow, plan editor, wrong-word notebook, and reports
- Optimize interaction flow for mobile rather than mirroring desktop layout

### Workstream D: Mobile storage/runtime

- Establish app sandbox paths on iOS and Android
- Open and migrate SQLite safely on-device
- Package offline seed data and vocabulary fallback assets for first launch

### Workstream E: AI and sync boundary

- Preserve AI as optional enhancement
- Cache outputs locally
- Treat sync as a later phase, not a prerequisite for the mobile MVP

## 6. Delivery Strategy

### Milestone 1: Shared core foundation

Goal:
- Shared Rust crates exist and desktop can still compile against them

Exit criteria:
- Desktop uses the extracted core with no behavior regression in study, plans, and reports

### Milestone 2: Mobile runtime viability

Goal:
- React Native app can launch, bootstrap local storage, load seed vocabulary, and read today state from shared Rust core

Exit criteria:
- Device build runs offline and survives app restart with persisted state

### Milestone 3: Mobile study MVP

Goal:
- Users can complete the main daily study loop on phone

Exit criteria:
- Today page, study session, plan reading, and result persistence are usable end to end

### Milestone 4: Mobile feature completion

Goal:
- Wrong-word notebook, reports, settings, vocabulary management, and AI passage features land on mobile

Exit criteria:
- Mobile has parity for the core learning loop and post-study review surfaces

### Milestone 5: Hardening and release

Goal:
- Production-grade mobile quality for app lifecycle, packaging, error recovery, and store submission

Exit criteria:
- Smoke tests, migration tests, offline cold start tests, and release build validation all pass

## 7. Key Risks

- Rust bridge complexity can slow early progress if the API surface is not narrowed first
- Mobile lifecycle interruptions can expose assumptions that were safe on desktop
- Vocabulary import and asset packaging may become too heavy for first-launch mobile UX if not staged carefully
- Desktop and mobile may drift if shared contracts are not versioned and tested

## 8. Immediate Recommendation

Start the new project with planning and architecture documentation first, then run a technical spike for:

1. Rust crate extraction from the current desktop repository
2. React Native bridge prototype for bootstrap and today-home reads
3. On-device SQLite open/migrate/read test with seed data
