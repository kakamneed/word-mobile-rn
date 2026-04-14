# Requirements: Word Mobile RN

**Defined:** 2026-04-09

## v1 Requirements

### Core architecture

- [ ] **ARCH-01**: The project must extract study, plan, report, vocabulary, AI validation, and storage logic into reusable Rust crates that are independent from Tauri command handlers.
- [ ] **ARCH-02**: Desktop and mobile must consume stable, documented DTO contracts instead of duplicating request/response shapes informally.
- [ ] **ARCH-03**: Platform-specific concerns such as paths, app lifecycle, background jobs, secure key storage, and update behavior must live in platform adapter layers rather than shared core crates.

### Mobile runtime

- [ ] **MOB-01**: The mobile app must cold start into a visible shell without requiring network access.
- [ ] **MOB-02**: The mobile app must restore plan state, today progress, wrong-word state, and reports after app restart from local SQLite.
- [ ] **MOB-03**: The mobile app must survive background/foreground interruptions without corrupting the current study session state.
- [ ] **MOB-04**: The mobile app must provide offline seed vocabulary so first launch does not depend on remote download success.
- [ ] **MOB-05**: The mobile app must keep runtime files inside approved iOS/Android sandbox paths.

### Study workflow

- [ ] **STUD-01**: Users can open the app and understand today's learning goal within 5 seconds.
- [ ] **STUD-02**: Users can start a study session from the today page and complete a full answer-submit-next loop on mobile.
- [ ] **STUD-03**: Study cards must not reveal the answer before submission.
- [ ] **STUD-04**: Study progress and result persistence must match the desktop domain rules for correctness, wrong-word updates, and session summaries.
- [ ] **STUD-05**: The app must support touch-friendly study interactions and mobile-safe layouts for long content.

### Plans and snapshots

- [ ] **PLAN-01**: Users can read the active plan and today snapshot from mobile.
- [ ] **PLAN-02**: Users can edit plan targets and growth rules from mobile in a simplified mobile-appropriate editor.
- [ ] **PLAN-03**: Today snapshots remain stable for the current day even if the user edits the plan afterward.

### Vocabulary and import

- [ ] **LIB-01**: Users can view available wordbooks and their enabled status on mobile.
- [ ] **LIB-02**: The system must validate imported or updated vocabulary data before promoting it to runtime use.
- [ ] **LIB-03**: Vocabulary update failure must not delete or invalidate the last known good local wordbooks.

### Reports and wrong words

- [ ] **WRNG-01**: Wrong answers and skips must persist into wrong-word state on mobile the same way they do on desktop.
- [ ] **WRNG-02**: Users can review wrong words from a mobile notebook surface.
- [ ] **RPT-01**: Users can view daily progress and mode-based summaries on mobile from persisted report aggregates.

### AI enhancement

- [ ] **AI-01**: AI passage generation remains an optional action and cannot block the main study loop.
- [ ] **AI-02**: Generated passages and history must be cached locally on mobile.
- [ ] **AI-03**: AI failures must resolve into visible, non-blocking states.

## v2 Requirements

- **SYNC-01**: Support account-backed cloud backup and restore.
- **SYNC-02**: Support multi-device sync with conflict resolution.
- **MOB-06**: Support notifications and re-engagement scheduling for daily study reminders.
- **OPS-01**: Support production crash reporting and remote diagnostics suitable for mobile release operations.

## Non-Goals

| Item | Reason |
|------|--------|
| Rewriting the learning engine in TypeScript | Would duplicate the most valuable logic and increase divergence risk |
| Mobile parity with every desktop screen in v1 | Mobile MVP should focus on the primary daily learning loop first |
| Mandatory account system before mobile launch | Conflicts with offline-first MVP delivery |
| Real-time sync in the first release | High complexity and not required for single-device offline study |

## Mapping Intent

| Requirement | Planned Phase |
|-------------|---------------|
| ARCH-01 | Phase 1-2 |
| ARCH-02 | Phase 2 |
| ARCH-03 | Phase 1-3 |
| MOB-01 | Phase 3 |
| MOB-02 | Phase 3-5 |
| MOB-03 | Phase 5 |
| MOB-04 | Phase 3-6 |
| MOB-05 | Phase 3 |
| STUD-01 | Phase 5 |
| STUD-02 | Phase 5 |
| STUD-03 | Phase 5 |
| STUD-04 | Phase 5 |
| STUD-05 | Phase 5 |
| PLAN-01 | Phase 4-5 |
| PLAN-02 | Phase 6 |
| PLAN-03 | Phase 4 |
| LIB-01 | Phase 6 |
| LIB-02 | Phase 6 |
| LIB-03 | Phase 6 |
| WRNG-01 | Phase 5-7 |
| WRNG-02 | Phase 7 |
| RPT-01 | Phase 7 |
| AI-01 | Phase 7 |
| AI-02 | Phase 7 |
| AI-03 | Phase 7 |
