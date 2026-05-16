# Flutter Learning Flow Skill Index

Use this project-local skill index before changing Flutter learning-flow code in `D:\projects\word-mobile-rn`.

## Scope

The active mobile product surface is Flutter under `apps/flutter_mobile`. React Native paths under `apps/mobile` are legacy context only unless a task explicitly asks for React Native.

Canonical layer order:

1. Flutter lifecycle and app state.
2. Flutter feature screens.
3. Typed Flutter SDK clients through `WordSdk`.
4. Flutter bridge codec/error boundary.
5. Native Android/iOS bridge adapters.
6. Rust `platform-mobile` bridge.
7. Rust app-core/study-core services.
8. SQLite storage-core persistence.
9. Optional cloud/Supabase/sync and AI transport.

Screens should consume typed SDK clients. Rust/SQLite remains the source of learning truth. Flutter owns display state, selected UI state, navigation handoff, loading/errors, and visual feedback.

## Choose The Right Guide

| Task | Read |
|---|---|
| Today page, active plan fallback, Study launch, cold-start bounce | `TODAY-STUDY-HANDOFF.md` |
| Study cards, selected option state, answer correctness, non-A labels | `STUDY-ANSWERING.md` |
| AI page, Today AI shortcut, wrong-word import, Wrong Words, Reports | `AI-WRONG-WORDS-REPORTS.md` |
| Bridge DTOs, SQLite state, local data owner, leaderboard, image vote/upload, sidebar | `BRIDGE-DATA-LEADERBOARD.md` |
| Release build, analyzer/test matrix, device smoke, human UAT | `RELEASE-VALIDATION.md` |
| Known historical pitfalls and modification recipes | `PITFALLS.md` |
| Required regression checks by change type | `REGRESSION-GUARDRAILS.md` |

## Non-Negotiables

- Do not route Flutter feature screens directly to native bridge primitives.
- Do not reintroduce fallback logic that silently turns missing `correctChoiceLabel` into A.
- Do not fix cold-start Study bounce with delays or "ignore first return" UI hacks.
- Do not auto-apply a saved plan to Today while merely loading/displaying Today fallback state.
- Do not treat Supabase/cloud as the source of local answer/progress correctness.
- Do not claim Flutter tests passed when they timed out.

## Current Phase 10 Status

Phase 10 implementation verification reached `9/9 must-haves verified`. Remaining work is human/device verification:

- Release-device cold start -> immediate Study -> submit -> return Today.
- Visual/route smoke across Today, Study, AI, Wrong Words, Reports, leaderboard, image vote/upload, account drawer/sidebar entries.

See `.planning/phases/10-today-answer-ai-and-data-layer-cleanup/10-VERIFICATION.md` and `10-HUMAN-UAT.md`.
