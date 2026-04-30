# RN Bridge To Shared Contract Mapping Matrix

Status: Draft
Owner: Rust shared core + mobile adapter layer
Phase: Slice 2 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document maps the current React Native bridge surface to the intended shared contract surface that Flutter and desktop should consume through Rust-owned semantics.

It answers four questions for each current bridge function:

1. what it means today
2. whether the semantic contract should be preserved
3. what the future shared contract name/family should be
4. what must stay adapter-private instead of leaking into the shared layer

## Mapping status vocabulary

- `direct_preserve`
- `preserve_but_tighten`
- `split_contract`
- `adapter_only_detail`
- `defer_from_flutter_v1`
- `candidate_drop`

## Matrix

| Current RN bridge function | Current contract family | Mapping status | Future shared contract family | Adapter-private concerns to strip out | Notes |
|---|---|---|---|---|---|
| `getBootstrapState` | bootstrap | `preserve_but_tighten` | `bootstrap.getState` | raw platform bootstrap diagnostics | Keep readiness semantics unchanged |
| `markOnboardingCompleted` | bootstrap/onboarding | `direct_preserve` | `onboarding.markCompleted` | none | Simple mutation, clear semantic |
| `getTodayHomeState` | today | `direct_preserve` | `today.getHomeState` | transport JSON string | Highest-value read contract |
| `getSettings` | settings | `preserve_but_tighten` | `settings.getSummary` | raw database path formatting, platform wording | Shared DTO should stay product-safe |
| `getAiProviderConfig` | settings/ai | `preserve_but_tighten` | `settings.aiProviders.getConfig` | secure-storage specifics | Avoid exposing secret storage implementation |
| `saveAiProviderConfig` | settings/ai | `preserve_but_tighten` | `settings.aiProviders.saveConfig` | token persistence implementation | Preserve semantic intent |
| `getActivePlan` | plan | `direct_preserve` | `plan.getActive` | none | Shared across desktop/mobile |
| `savePlan` | plan | `direct_preserve` | `plan.save` | bridge JSON envelope | Preserve current plan edit semantics |
| `applySavedPlanToToday` | plan/today | `preserve_but_tighten` | `plan.applySavedToToday` | implicit snapshot mutation details | Needs clearer persistence semantics |
| `getWordbooks` | wordbooks | `direct_preserve` | `wordbooks.list` | none | Stable list read |
| `toggleWordbook` | wordbooks | `direct_preserve` | `wordbooks.setActive` | native int/bool transport details | Preserve behavior |
| `startStudySession` | study | `direct_preserve` | `study.startSession` | JSON-string bridge encoding | Must stay Rust-owned |
| `submitStudyAnswer` | study | `direct_preserve` | `study.submitAnswer` | JSON-string bridge encoding | Must keep side effects exactly |
| `completeStudySession` | study | `direct_preserve` | `study.completeSession` | none | Preserve summary semantics |
| `cancelStudySession` | study | `direct_preserve` | `study.cancelSession` | none | Keep distinct from complete |
| `getReportsOverview` | reports | `preserve_but_tighten` | `reports.getOverview` | client chart consumption assumptions | Contract should stay domain-level |
| `getWrongWords` | wrong words | `direct_preserve` | `wrongWords.list` | none | Preserve filtering input |
| `getWrongWordDetail` | wrong words | `preserve_but_tighten` | `wrongWords.getDetail` | none | Keep risk breakdown Rust-owned |
| `getTodayAiPassageContext` | ai | `direct_preserve` | `ai.getTodayContext` | none | Keep non-blocking semantics |
| `generateAiPassage` | ai | `preserve_but_tighten` | `ai.generatePassage` | provider transport/fallback implementation | Contract result shape shared |
| `getAiPassageHistory` | ai | `direct_preserve` | `ai.listHistory` | none | Shared history read |
| `getAiPassage` | ai | `direct_preserve` | `ai.getPassage` | none | Shared read |
| `saveAiPassage` | ai | `preserve_but_tighten` | `ai.savePassage` | persistence internals | Clarify validation ownership |

## Type-level mapping notes

### Preserve as-is or near-as-is

- `BootstrapState`
- `TodayHomeState`
- `WordbookSummary`
- `StudySession`
- `StudyResult`
- `SessionSummary`
- `WrongWordEntry`
- `TodayAiPassageContext`
- `AIPassageHistoryItem`

### Preserve semantics but tighten structure

- `PlanSummary`
  - Growth-rule fields now exceed the older contract doc and need one canonical definition.
- `DailySnapshot`
  - Base target and carryover target fields exist in current bridge and must be formally documented.
- `SettingsSummary`
  - Separate product-facing summary from platform runtime details.
- `ReportsOverview`
  - Keep aggregate semantics, avoid injecting chart-layout concerns.
- `WrongWordDetail`
  - Preserve risk/history/detail richness, but formalize stable schema.
- `AIPassage`
  - Clarify which fields are generator output vs validation output vs persistence metadata.

### Split candidates

These are places where one current bridge contract probably carries more than one concern:

- `getSettings`
  - likely split into:
  - product settings summary
  - runtime diagnostics summary
- `getAiProviderConfig` / `saveAiProviderConfig`
  - keep semantic family, but separate safe display fields from write-secret request fields
- `applySavedPlanToToday`
  - clarify whether this is:
  - plan activation
  - today snapshot materialization
  - or both

## Adapter-only details that must not become shared contract truth

- `Promise<string>` JSON transport shape
- Android primitive parameter marshalling details
- Objective-C / Swift / JNI symbol names
- raw native exception text
- platform path strings and sandbox directory structures
- secure-storage backend identity
- provider failover implementation details

## Flutter-facing bundle proposal

### Initial Flutter bundle

- `bootstrap.getState`
- `onboarding.markCompleted`
- `today.getHomeState`
- `settings.getSummary`
- `plan.getActive`
- `plan.save`
- `plan.applySavedToToday`
- `wordbooks.list`
- `wordbooks.setActive`
- `study.startSession`
- `study.submitAnswer`
- `study.completeSession`
- `study.cancelSession`
- `reports.getOverview`
- `wrongWords.list`
- `wrongWords.getDetail`

### Defer from initial Flutter bundle

- `settings.aiProviders.getConfig`
- `settings.aiProviders.saveConfig`
- `ai.getTodayContext`
- `ai.generatePassage`
- `ai.listHistory`
- `ai.getPassage`
- `ai.savePassage`

## Slice 2 follow-up tasks driven by this matrix

- Produce a canonical shared contract document that supersedes the current drift between `docs/CONTRACT.md` and `mobile-bridge.ts`.
- Define stable error codes per contract family.
- Decide whether TypeScript and Dart contract mirrors are generated from Rust DTOs or checked against Rust DTO metadata.
- Add CI checks so future bridge changes cannot bypass the mapping/inventory review path.

## Exit criteria for this document

- Every `mobile-bridge.ts` exported function appears in the matrix.
- Every row has a mapping status.
- Every row declares whether the concern is shared or adapter-private.
- Slice 4 can consume this matrix as the handoff for Flutter bridge construction.
