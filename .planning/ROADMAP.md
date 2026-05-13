# Roadmap: Word Mobile RN

## Overview

This roadmap assumes the current desktop application remains the behavior reference while the reusable Rust core is extracted and reused by a React Native mobile app.

The order is intentionally conservative:

1. Protect and extract core domain logic
2. Stabilize contracts
3. Prove mobile runtime viability
4. Deliver the primary study loop
5. Add supporting mobile surfaces
6. Harden for release

## Phases

- [ ] **Phase 1: Shared Rust Core Extraction Baseline**
  - Extract business logic from Tauri command handlers into domain crates
  - Separate storage, study, plan, report, vocabulary, and AI-validation responsibilities
  - Define platform traits for paths, storage bootstrap, clock, and runtime services

- [ ] **Phase 2: Cross-Platform Contract and Desktop Refactor**
  - Freeze DTO contracts and serialization rules
  - Refactor desktop to consume the extracted crates through an adapter layer
  - Add regression tests to ensure desktop behavior remains stable after extraction

- [ ] **Phase 3: React Native Bootstrap and Mobile Runtime**
  - Initialize the React Native app shell
  - Implement Rust bridge for bootstrap, settings, today-home reads, and database init
  - Validate on-device SQLite, sandbox paths, and offline seed data

- [ ] **Phase 4: Plan and Today Mobile Surfaces**
  - Build mobile onboarding, today page, and active-plan read flow
  - Support today snapshot reads and stable daily task display
  - Keep the mobile information architecture short and touch-first

- [ ] **Phase 5: Study Session MVP**
  - Implement the core study session flow on mobile
  - Preserve answer secrecy, result persistence, wrong-word updates, and session summaries
  - Validate interruption recovery across app backgrounding and restarts

- [ ] **Phase 6: Vocabulary Management and Plan Editing**
  - Build mobile wordbook list, enable/disable flows, import/update states, and simplified mobile plan editing
  - Preserve last-known-good vocabulary fallback behavior
  - Package mobile-safe offline seed assets

- [ ] **Phase 7: Reports, Wrong Words, and AI Surfaces**
  - Add wrong-word notebook and report pages
  - Add AI passage generation/history with non-blocking UX
  - Verify mobile parity for the full daily review loop

- [ ] **Phase 8: Release Hardening and Store Readiness**
  - Add crash-safe lifecycle handling, release build validation, observability hooks, and app-store readiness work
  - Finalize icons, permissions, privacy text, packaging, and QA passes

## Phase Details

### Phase 1: Shared Rust Core Extraction Baseline
**Goal**: Move critical product logic into platform-neutral crates without breaking existing desktop behavior.

Success criteria:
1. Tauri command handlers become thin adapters rather than business-logic owners.
2. Shared crates compile independently of Tauri app shell concerns.
3. Core domain tests cover study, plans, snapshots, reports, and vocabulary import rules.

### Phase 2: Cross-Platform Contract and Desktop Refactor
**Goal**: Lock down the boundary the mobile shell will depend on.

Success criteria:
1. DTO shapes are explicit, versioned, and tested.
2. Desktop uses the same contracts that mobile will use.
3. Core extraction does not regress desktop study-loop correctness.

### Phase 3: React Native Bootstrap and Mobile Runtime
**Goal**: Prove the mobile app can start, initialize storage, and read persisted product state offline.

Success criteria:
1. Mobile launches to a visible home shell offline.
2. SQLite opens and migrates correctly on-device.
3. Seed vocabulary is available even without a live download.

### Phase 4: Plan and Today Mobile Surfaces
**Goal**: Give users a working mobile landing experience around today's tasks.

Success criteria:
1. Users can see today goals and next actions on phone.
2. Today snapshots are loaded from the shared core rather than recreated ad hoc in UI.
3. Navigation and page structure feel mobile-native rather than desktop-transplanted.

### Phase 5: Study Session MVP
**Goal**: Deliver the core phone study loop end to end.

Success criteria:
1. Users can complete at least one full study session from today page to persisted result.
2. Answer leakage, submit flow, and state transitions remain correct on mobile.
3. Session progress survives interruption and restart safely.

### Phase 6: Vocabulary Management and Plan Editing
**Goal**: Bring the user-controlled inputs of the study loop onto mobile.

Success criteria:
1. Users can manage wordbooks and observe update/import state.
2. Users can edit plan targets from a mobile-appropriate form.
3. Failed vocabulary updates never destroy the active local dataset.

### Phase 7: Reports, Wrong Words, and AI Surfaces
**Goal**: Complete the mobile learning loop beyond the session itself.

Success criteria:
1. Wrong-word review is available on mobile.
2. Report views are backed by persisted aggregates rather than temporary counters.
3. AI passage flow is usable and clearly non-blocking.

### Phase 07.3: Mobile runtime truth alignment, AI passage hookup, and data continuity (INSERTED)

**Goal:** Replace the remaining mobile stub/runtime truth gaps with desktop-backed behavior, finish AI passage/history integration, and define a trustworthy local-data continuity strategy.
**Requirements**: [ARCH-02, MOB-02, MOB-05, STUD-02, STUD-04, PLAN-01, RPT-01, AI-01, AI-02, AI-03]
**Depends on:** Phase 7
**Plans:** 4 plans

Plans:
- [ ] 07.3-01 - Replace Android/mobile stub truth with real bridge-backed contracts
- [ ] 07.3-02 - Align study mode sourcing, question generation, and session counts with desktop truth
- [ ] 07.3-03 - Hook mobile AI passage/history and persisted report surfaces to backend truth
- [ ] 07.3-04 - Harden local data continuity and define non-account backup/recovery strategy

### Phase 07.4: Mobile study UX polish and desktop-parity closure (INSERTED)

**Goal:** Close the remaining mobile study UX and behavior mismatches surfaced in live device testing, with desktop parity as the acceptance reference.
**Requirements**: [STUD-01, STUD-02, STUD-04, STUD-05, AI-01, AI-02, AI-03, RPT-01]
**Depends on:** Phase 07.3
**Plans:** 4 plans

Plans:
- [ ] 07.4-01 - Remove redundant study-card copy and improve submit/keyboard ergonomics
- [ ] 07.4-02 - Restore Today progress truthfulness for in-progress resumed sessions
- [ ] 07.4-03 - Fix root-affix and multiple-choice fairness/parity issues
- [ ] 07.4-04 - Align AI passage generation semantics with desktop contextual composition

### Phase 07.2: Mobile desktop parity alignment and session state fidelity (INSERTED)

**Goal:** [Urgent work - to be planned]
**Requirements**: TBD
**Depends on:** Phase 7
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd:plan-phase 07.2 to break down)

### Phase 07.1: Mobile parity and study correctness fixes (INSERTED)

**Goal:** Restore missing mobile learning capability parity and fix the Chinese-input study correctness bug discovered during live phone testing.
**Requirements**: [LIB-01, STUD-02, STUD-04]
**Depends on:** Phase 7
**Plans:** 0 plans

Plans:
- [ ] TBD (run /gsd:plan-phase 07.1 to break down)

Success criteria:
1. The mobile app exposes the medical-English wordbook in the active vocabulary universe instead of omitting it.
2. The mobile app exposes the root/affix learning mode instead of omitting it.
3. The most visible mobile user-facing surfaces use Chinese rather than English placeholder copy.
4. For Chinese-meaning input questions, answer validation matches backend truth and users can continue to the next question normally.

### Phase 8: Release Hardening and Store Readiness
**Goal**: Make the mobile app safe to distribute.

Success criteria:
1. Cold start, restart, interrupted-session, and offline tests pass on target devices.
2. Release packaging and privacy/compliance tasks are complete.
3. Crash handling and diagnostics are sufficient for real-user support.

### Phase 9: Learning flow structure map and pitfall inventory

**Goal:** Build a complete, evidence-backed map of the mobile learning loop before any destructive cleanup, covering Today, study answering, AI passage/history, native bridge contracts, Rust core, SQLite/Supabase-adjacent data boundaries, and every known stale implementation path.
**Requirements**: [ARCH-02, MOB-02, STUD-02, STUD-03, STUD-04, STUD-05, PLAN-01, WRNG-01, RPT-01, AI-01, AI-02, AI-03]
**Depends on:** Phase 8
**Plans:** 4 plans

Plans:
- [ ] 09-01 - Map the current Today-to-study-to-result flow across Flutter, legacy React Native, Rust bridge, core crates, and persistence
- [ ] 09-02 - Inventory all historical learning-flow pitfalls, including selected-wrong-option highlighting and correct-answer index drift toward A
- [ ] 09-03 - Classify stale layers, empty adapters, mock truth, duplicate clients, and abandoned UI/data paths by ownership and deletion risk
- [ ] 09-04 - Produce canonical structure docs that define the intended latest layer hierarchy and cleanup acceptance gates

Success criteria:
1. Every active learning-flow surface has a named owner layer and canonical file path.
2. Known historical failures are traced to concrete causes and linked to the layers where regression guards belong.
3. Obsolete, duplicate, mock, or empty paths are listed with keep/delete/replace recommendations before cleanup starts.
4. Downstream cleanup plans can proceed without rediscovering the same architecture history.

### Phase 10: Today answer AI and data layer cleanup

**Goal:** Thoroughly clean the learning-flow implementation layers identified in Phase 9, removing obsolete remnants and simplifying Today, answer evaluation, AI, bridge, and persistence code while preserving current user-facing behavior and effects.
**Requirements**: [ARCH-02, MOB-02, STUD-02, STUD-03, STUD-04, STUD-05, PLAN-01, WRNG-01, RPT-01, AI-01, AI-02, AI-03]
**Depends on:** Phase 9
**Plans:** 5 plans

Plans:
- [ ] 10-01 - Clean Today page state sourcing, progress truth, resume paths, and navigation handoff into study sessions
- [ ] 10-02 - Clean study question rendering, answer evaluation, selected-option feedback, correct-answer indexing, and submit/next state transitions
- [ ] 10-03 - Clean AI page generation/history wiring so mobile uses one canonical non-blocking AI boundary and no abandoned mock path
- [ ] 10-04 - Clean native bridge, Rust core DTOs, SQLite repositories, and cloud-adjacent data code to remove duplicate truth and empty adapters
- [ ] 10-05 - Run focused regression validation across Today, answering, AI, reports/wrong words, persistence restart, and release-device smoke paths

Success criteria:
1. The selected wrong answer is visibly marked wrong while the correct option remains accurately identified.
2. Correct answers remain bound to their real option index for A/B/C/D, with no fallback that collapses results to A.
3. Today, study, AI, report, wrong-word, and persistence flows use the latest canonical layers only.
4. Removed legacy code has no surviving imports, navigation entries, bridge methods, DTO variants, test fixtures, or docs that imply it is still active.
5. Existing product behavior and visual effects are preserved unless Phase 9 explicitly marked them obsolete.

### Phase 11: Learning flow skillization and regression guardrails

**Goal:** Convert the cleaned learning-flow implementation knowledge into reusable skill-standard documentation and regression guardrails so future Today, answer, AI, and database changes start from the canonical structure rather than old residue.
**Requirements**: [ARCH-02, MOB-02, STUD-02, STUD-03, STUD-04, STUD-05, WRNG-01, RPT-01, AI-01, AI-02, AI-03]
**Depends on:** Phase 10
**Plans:** 4 plans

Plans:
- [ ] 11-01 - Create skill-standard docs for Today, study answering, AI passage/history, bridge/data persistence, and mobile release validation workflows
- [ ] 11-02 - Encode historical pitfalls and canonical modification recipes into skills so future fixes avoid stale paths and answer-index regressions
- [ ] 11-03 - Add focused tests/checklists for selected-option feedback, correct-answer index preservation, Today progress truth, AI non-blocking behavior, and persisted result continuity
- [ ] 11-04 - Update roadmap/state references so future GSD or Vico work enters through the new skills and canonical structure map

Success criteria:
1. Each major learning-flow capability has a skill-standard guide with purpose, trigger conditions, canonical files, workflow, verification, and known pitfalls.
2. Future agents can modify Today, answering, AI, and persistence without needing to rediscover which historical paths are stale.
3. Regression guards cover the two named hard bugs and the broader layer-cleanup risks from Phase 10.
4. Project planning docs point to the new skills and structure docs as the default entry points for future learning-flow work.
