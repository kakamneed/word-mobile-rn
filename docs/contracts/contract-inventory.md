# Contract Inventory

Status: Draft
Owner: Rust shared core
Phase: Slice 2 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document freezes the current cross-platform contract surface that must be preserved, tightened, split, deferred, or dropped during the move from the current React Native bridge to a Flutter-facing shared contract.

It does not define wire format for a specific FFI.
It defines the domain-facing contract inventory that Rust should own.

## Contract rules

- Rust DTO semantics are the source of truth.
- Request and response shapes must be documented separately from bridge-specific JSON serialization details.
- Each contract must explicitly record side effects and persistence semantics.
- Contract status vocabulary in this file:
  - `preserve`
  - `tighten`
  - `split`
  - `defer`
  - `drop`

## Current sources

- [CONTRACT.md](/d:/projects/word-mobile-rn/docs/CONTRACT.md)
- [mobile-bridge.ts](/d:/projects/word-mobile-rn/apps/mobile/src/lib/mobile-bridge.ts)
- [index.ts](/d:/projects/word-mobile-rn/packages/contracts/src/index.ts)

## Inventory

### Bootstrap

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Bootstrap state read | `getBootstrapState` | `tighten` | desktop + mobile | May trigger runtime readiness checks | Keep semantics stable, but move platform-specific runtime diagnostics out of shared DTO if possible |
| Onboarding completion acknowledgement | `markOnboardingCompleted` | `preserve` | mobile-first, desktop-compatible | Writes onboarding completion state | Preserve semantic meaning; bridge transport can change |

### Today

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Today home read | `getTodayHomeState` | `preserve` | desktop + mobile | Read-only from caller perspective | Shared contract must preserve plan/snapshot/progress semantics |

### Settings

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Settings summary read | `getSettings` | `tighten` | desktop + mobile | Read-only | Shared DTO should keep app/runtime/storage summary; platform path internals may need adapter-only detail |
| AI provider config read | `getAiProviderConfig` | `tighten` | mobile + desktop optional | Read-only | Do not expose platform secret storage details |
| AI provider config save | `saveAiProviderConfig` | `tighten` | mobile + desktop optional | Persists provider config | Auth token write path is side-effectful and must remain explicit |

### Plan

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Active plan read | `getActivePlan` | `preserve` | desktop + mobile | Read-only | Keep plan summary semantics stable |
| Plan update | `savePlan` | `preserve` | desktop + mobile | Persists plan template | Shared contract must preserve growth-rule semantics |
| Apply saved plan to today | `applySavedPlanToToday` | `tighten` | mobile + desktop | Persists plan/snapshot relationship | Clarify whether this mutates today's effective plan only, or also snapshot generation triggers |

### Wordbooks

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Wordbook list read | `getWordbooks` | `preserve` | desktop + mobile | Read-only | Shared list semantics stable |
| Wordbook activation toggle | `toggleWordbook` | `preserve` | desktop + mobile | Persists enabled state | Future sync may observe this as preference mutation |

### Study

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Start study session | `startStudySession` | `preserve` | desktop + mobile | Creates/loads session state | Must remain Rust-owned |
| Submit study answer | `submitStudyAnswer` | `preserve` | desktop + mobile | Mutates session, wrong-word state, report aggregates | Highest-value cross-platform contract |
| Complete study session | `completeStudySession` | `preserve` | desktop + mobile | Finalizes session and summaries | Must keep summary semantics exact |
| Cancel study session | `cancelStudySession` | `preserve` | desktop + mobile | Cancels session state | Must stay distinct from complete |

### Reports

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Reports overview read | `getReportsOverview` | `tighten` | desktop + mobile | Read-only | Keep aggregates shared; chart-shaping fields may be client concern |

### Wrong Words

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Wrong-word list read | `getWrongWords` | `preserve` | desktop + mobile | Read-only | Preserve filtering semantics |
| Wrong-word detail read | `getWrongWordDetail` | `tighten` | desktop + mobile | Read-only | Risk breakdown and detail richness should stay domain-owned |

### AI

| Contract | Current function(s) | Status | Shared scope | Side effects | Notes |
|---|---|---|---|---|---|
| Today AI passage context read | `getTodayAiPassageContext` | `preserve` | mobile + desktop optional | Read-only | Must remain non-blocking relative to study |
| Generate AI passage | `generateAiPassage` | `tighten` | mobile + desktop optional | May call remote provider and produce cached result | Provider transport can change; domain result shape should stay shared |
| AI passage history read | `getAiPassageHistory` | `preserve` | mobile + desktop optional | Read-only | History semantics shared |
| Single AI passage read | `getAiPassage` | `preserve` | mobile + desktop optional | Read-only | Shared domain read |
| AI passage save | `saveAiPassage` | `tighten` | mobile + desktop optional | Persists AI artifact | Clarify caller-owned vs backend-owned validation fields |

### Future sync-facing contracts

These are not current bridge contracts, but Slice 2 should reserve space for them instead of letting them appear ad hoc later.

| Contract family | Status | Notes |
|---|---|---|
| Auth session status | `defer` | Keep out of shared learning DTOs; define later in sync/auth slice |
| Sync diagnostics/status | `defer` | Should not leak into today/study contracts |
| Device registration hooks | `defer` | Future cloud-facing capability, not part of current RN bridge |
| Conflict-resolution results | `defer` | Rust-owned, but not needed in Flutter minimum bundle yet |

## Flutter minimum bundle

The minimum contract bundle Flutter should consume first:

- `getBootstrapState`
- `markOnboardingCompleted`
- `getTodayHomeState`
- `getSettings`
- `getActivePlan`
- `savePlan`
- `applySavedPlanToToday`
- `getWordbooks`
- `toggleWordbook`
- `startStudySession`
- `submitStudyAnswer`
- `completeStudySession`
- `cancelStudySession`
- `getReportsOverview`
- `getWrongWords`
- `getWrongWordDetail`

The following should be treated as second-wave contracts:

- `getAiProviderConfig`
- `saveAiProviderConfig`
- `getTodayAiPassageContext`
- `generateAiPassage`
- `getAiPassageHistory`
- `getAiPassage`
- `saveAiPassage`

## Known mismatches to resolve

- [CONTRACT.md](/d:/projects/word-mobile-rn/docs/CONTRACT.md) still reflects an older narrower contract and does not include newer fields now exposed by [mobile-bridge.ts](/d:/projects/word-mobile-rn/apps/mobile/src/lib/mobile-bridge.ts), such as:
  - `rootAffix`
  - richer `PlanSummary` growth-rule fields
  - richer `DailySnapshot` carryover/base target fields
  - `WrongWordDetail`
  - `ReportsOverview`
  - AI passage structures
- The RN bridge is still transport-oriented and string-JSON based; the shared contract should not encode that JSON-string detail as domain truth.
- Settings and AI config currently mix product-facing semantics with runtime/storage details; Slice 2 should tighten this split.

## Exit criteria for this document

- Every active bridge function is present in this inventory.
- Every contract has a status.
- Every contract records whether it is shared, deferred, or adapter-only.
- Slice 4 implementation should be able to consume this document without guessing contract scope.
