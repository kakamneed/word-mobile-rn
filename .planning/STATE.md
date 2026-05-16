# Project State: Word Mobile RN

**Initialized:** 2026-04-09
**Current Status:** Phase 11 in progress: converting the cleaned Flutter learning-flow implementation into project-local skill-standard docs and regression guardrails.

## Core Value

Users should be able to complete the full daily vocabulary-learning loop on mobile with reliable local persistence and without network dependency for the main flow.

## Chosen Direction

- Scheme B selected
- React Native mobile shell
- Shared Rust core
- SQLite as on-device source of truth
- AI remains optional and non-blocking

## Reference System

- Source reference repository: `D:\projects\word-desktop-tauri`
- Reference architecture: React + Tauri shell on desktop, Rust service layer, SQLite persistence

## Active Assumptions

- The desktop product remains the functional reference during migration.
- The most important reusable assets are the Rust services and data model, not the desktop UI.
- Mobile UX should be redesigned around touch-first navigation rather than adapted from sidebar-first desktop layouts.
- Sync is postponed until the single-device mobile loop is stable.

## Immediate Next Step

Finish `$gsd-execute-phase 11`: verify `.planning/skills/learning-flow/INDEX.md` and related guides, then preserve the remaining Phase 10 human/device UAT items.

## Resume Point

- Resume file: `.planning/phases/11-learning-flow-skillization-and-regression-guardrails/11-01-PLAN.md`
- Canonical skill entry: `.planning/skills/learning-flow/INDEX.md`
- Stopped at: Phase 11 execution in progress; skill docs and guardrails are being created from Phase 9/10 facts.

## Session Log

- 2026-05-13: Phase 09 added to map the full learning-flow hierarchy and inventory historical pitfalls before cleanup, including selected-wrong-option highlighting and correct-answer index drift toward A.
- 2026-05-13: Phase 10 added to clean Today, study answering, AI, bridge, Rust core, SQLite, and cloud-adjacent data layers while preserving current behavior and effects.
- 2026-05-13: Phase 11 added to convert the cleaned implementation knowledge into skill-standard guides and regression guardrails for future learning-flow changes.
- 2026-05-13: Phase 09 context captured with the active scope narrowed to Flutter only; React Native paths are legacy context only, while Flutter SDK/bridge, Rust core, SQLite, AI, sync, and Flutter skills/docs remain in scope where they affect the learning flow.
- 2026-05-13: Phase 10 context captured for Flutter-only cleanup, locking behavior preservation, layered cleanup order, answer-regression gates, AI non-blocking rules, and local SQLite/Rust truth boundaries.
- 2026-05-13: Phase 10 planning completed with research, five executable plans, and plan-check pass. Added explicit coverage for wrong words, reports, cold-start Study bounce to Today, sidebar entries, leaderboard, image voting leaderboard mode, image extraction/upload, selected-wrong-option red feedback, and non-A correct-answer preservation.
- 2026-05-14: Phase 10 execution reached verifier score 9/9 with `human_needed` only. Remaining items are release-device cold start -> Study -> submit -> Today refresh and visual/route smoke across Today, Study, AI, Wrong Words, Reports, leaderboard, image vote/upload, and sidebar routes.
- 2026-05-14: Phase 11 execution created project-local Flutter learning-flow skill docs under `.planning/skills/learning-flow/`, with `INDEX.md` as the canonical future entry point.
- 2026-04-09: New mobile migration project initialized.
- 2026-04-09: Scheme B selected as the target architecture.
- 2026-04-09: Initial planning documents created for Rust-core extraction and React Native mobile delivery.
- 2026-04-09: Phase 1 discussion completed. Decisions captured in `.planning/phases/01-shared-rust-core-extraction-baseline/01-CONTEXT.md`.
- 2026-04-09: Phase 1 context locked with these primary choices: dedicated third shared-core repository, medium-granularity crate split, coarse platform runtime abstraction, and desktop-safe incremental migration.
- 2026-04-09: Phase 2 discussion completed. Decisions captured in `.planning/phases/02-cross-platform-contract-and-desktop-refactor/02-CONTEXT.md`.
- 2026-04-09: Phase 2 context locked with these primary choices: Rust-owned contract truth, dual-layer facade plus fine-grained API shape, strong synchronization instead of version compatibility, and desktop refactor acceptance across bootstrap, today, study, wrong words, and reports.
- 2026-04-09: Phase 3 context captured from discussion with these primary choices: real RN native bridge from the start, bundled formal seed baseline, blocking startup failures, and runtime validation including one real study launch.
- 2026-04-09: Phase 3 planning completed with 3 plans in 2 waves covering RN shell and bridge scaffolding, mobile sandbox storage plus formal seed runtime, and blocking bootstrap plus study-launch smoke validation.
- 2026-04-09: Phase 4 discussion completed. Decisions captured in `.planning/phases/04-plan-and-today-mobile-surfaces/04-CONTEXT.md`.
- 2026-04-09: Phase 4 context locked with these primary choices: hybrid Today layout, full onboarding wizard, lightweight mobile plan editing, and Today as the default mobile home.
- 2026-04-09: Phase 4 planning completed with 3 plans in 2 waves covering Today-first navigation plus full onboarding, hybrid Today home construction, and lightweight mobile plan access/editing.
- 2026-04-09: Phase 5 discussion completed. Decisions captured in `.planning/phases/05-study-session-mvp/05-CONTEXT.md`.
- 2026-04-09: Phase 5 context locked with these primary choices: two-step answer rhythm, progressive disclosure with phonetic info in the minimum visible payload, explicit resume-or-abandon recovery, and completion summary with both Today and next-round exit paths.
- 2026-04-09: Phase 5 planning completed with 3 plans in 2 waves covering the mobile study shell plus submit/next flow, phonetic-first progressive disclosure plus interruption recovery, and completion summary with backend-driven persistence validation.
- 2026-04-09: Phase 6 discussion completed. Decisions captured in `.planning/phases/06-vocabulary-management-and-plan-editing/06-CONTEXT.md`.
- 2026-04-09: Phase 6 context locked with these primary choices: status-overview-first vocabulary management, a single primary update action with page-level status, a significantly deeper mobile plan editor, and explicit fallback messaging that preserves user trust.
- 2026-04-09: Phase 6 planning completed with 3 plans in 2 waves covering status-first library management, fallback-aware wordbook management, and a significantly deeper mobile plan editor approaching desktop capability.
- 2026-04-09: Phase 7 discussion completed. Decisions captured in `.planning/phases/07-reports-wrong-words-and-ai-surfaces/07-CONTEXT.md`.
- 2026-04-09: Phase 7 context locked with these primary choices: overview-first reports with mode drill-down, wrong-word list-plus-detail review, a dedicated AI page, and a unified review-center information architecture.
- 2026-04-09: Phase 7 planning completed with 3 plans in 2 waves covering the review-center shell plus overview-first reports, wrong-word list-plus-detail review, and a dedicated AI page with reading and history flows.
- 2026-04-09: Phase 8 discussion completed. Decisions captured in `.planning/phases/08-release-hardening-and-store-readiness/08-CONTEXT.md`.
- 2026-04-09: Phase 8 context locked with these primary choices: update prompts with version details, explicit key-exception recovery, strong release validation gates, and full store-readiness scope.
- 2026-04-10: Phase 07.1 inserted after Phase 7: Mobile parity and study correctness fixes (URGENT).
- 2026-04-10: Phase 07.1 context captured with four urgent mobile-fix items: restore medical wordbook, restore root/affix mode, normalize Chinese UI copy, and fix Chinese-input answer correctness/progression.
- 2026-04-10: Phase 07.1 planning completed with 3 plans in 2 waves covering content parity restoration, pragmatic Chinese UI normalization, and the urgent Chinese-input study correctness fix.
- 2026-04-10: Phase 07.2 inserted after Phase 7: Mobile desktop parity alignment and session state fidelity (URGENT).
- 2026-04-10: Phase 07.2 planning completed with 3 waves covering session-state fidelity, Today/Plan/mode semantic parity, and a dedicated desktop parity audit/closure wave.
- 2026-04-10: Phase 07.2 execution completed for the current code pass: resumable mobile session semantics, plan apply-today choice, plan-integrated wordbook placement, independent wrong-word tab, and a dedicated mobile-desktop parity audit document. Android debug build succeeded from `android_build3`.
- 2026-04-12: Phase 07.3 repurposed from placeholder into a concrete urgent phase after fresh analysis found that Android/mobile still relies on hardcoded stub truth for Today, Plan, Study, Reports, and AI passage behavior.
- 2026-04-12: Phase 07.3 planning completed with 4 plans covering bridge-truth replacement, desktop-aligned study semantics, AI passage/report integration, and local-data continuity strategy without making account login mandatory for v1.
- 2026-04-12: Phase 07.3-01 execution started. Removed hardcoded study launch payloads from mobile navigation and replaced the Android stub module with a persistent local-state implementation for plan, today snapshot, wordbooks, sessions, and wrong-word carryover.
- 2026-04-12: `npm run typecheck` passed in `apps/mobile`. Android `assembleDebug` could not be fully validated because the sandbox blocked Gradle from downloading `gradle-8.3-all.zip`.
- 2026-04-13: Phase 07.3-02 execution started. Mobile study contracts now recognize root/affix-specific question types, the study card was rewritten to remove broken feedback text, and Android-side question generation was tightened toward desktop ordering and 4/4/3/3/2 mode counts.
- 2026-04-13: Phase 07.3-02 continued. Android-side study sourcing now distinguishes unseen new-word cards, learned review cards, mixed-test cards, and wrong-word pool cards instead of relying on a single generic active-card rotation.
- 2026-04-13: Phase 07.3-03 execution started. Mobile bridge contracts now expose reports, wrong-word, and AI passage APIs; report and wrong-word screens were rewritten to consume real native data instead of mock payloads; AI passage generation/history is now backed by native local state rather than hardcoded sample text.
- 2026-04-13: Phase 07.3-03 continued. Mobile frontend now has an explicit AI page and bottom-tab entry, with dedicated AI passage client calls, generation action, latest passage display, and history browsing backed by the native module contract.
- 2026-04-13: Phase 07.3-03 continued again. AI passage contracts were upgraded from plain text blobs to a more desktop-like structured shape with wrong-word inputs, passage blocks/segments, coverage metadata, validation status, and shared rendering across the dedicated AI page and wrong-word preview area.
- 2026-04-13: Phase 07.3-03 continued further. Native AI generation now prefers today's completed-session wrong-word inputs and only falls back to the current wrong-word notebook when the day has no recorded wrong answers, matching the intent of the desktop `collect_day_wrong_words` boundary more closely.
- 2026-04-13: Fixed the plan-page wordbook count regression by replacing the Android native module's hardcoded demo totals (`6/2/2/4`) with product-scale values (`4500/3000/5500/1800`). Followed the desktop wireless deploy workflow to build `android_build3` release successfully. Wireless install reached the device but was aborted because the phone rejected the install permission prompt.
- 2026-04-13: Phase 07.4 inserted after renewed live-device testing surfaced six remaining parity/UX gaps spanning redundant study-card copy, AI passage semantics, in-progress Today progress truth, root-affix reveal behavior, answer-choice fairness, and Chinese-input keyboard ergonomics.
- 2026-04-13: Phase 07.4 execution started. Reworked the mobile study card and session screen to remove duplicate example prompts, preserve progress on exit-to-Today, improve Chinese-input autofocus/Enter-submit ergonomics, and stop immediate root-affix answer leakage. Android-side choice generation was also updated to distribute correct answers more evenly across A/B/C/D.
- 2026-04-13: Phase 07.4 continued. Root/affix example panels were shifted toward “example word + gloss” presentation, input questions now hide the inline submit button in favor of keyboard send, partial Today progress now rounds preserved in-progress work upward, and native AI passage generation was rewritten into a more contextual two-paragraph review narrative instead of per-word stitched lines.
- 2026-04-14: Phase 07.4 continued again. The mobile AI client was switched away from local fake copy and now calls the same live primary/backup model gateways as desktop, using prompt-template/validation-style-compatible prompt assembly and marker parsing on the mobile side.
