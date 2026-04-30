# Flutter SDK Surface

Status: Draft
Owner: Flutter shell layer
Phase: Slice 4 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the intended typed SDK surface that Flutter code should consume instead of calling bridge primitives directly.

The SDK is the only Flutter-facing layer that can talk to the bridge.

## Design rules

- All SDK methods return typed models, not raw JSON strings.
- SDK methods map bridge/protocol/runtime failures into normalized error classes.
- SDK methods preserve Rust contract semantics.
- SDK methods may improve ergonomics, but may not invent new business truth.

## Slice 4 first-pass decisions

- SDK methods should be domain-grouped by contract family, not by platform transport.
- SDK should be the only place that knows bridge method names.
- first-wave feature code should not need direct bridge access at all
- v1 should use normalized exception-first handling with stable error classes
- later `authClient` and `syncClient` may be added, but are not required to unblock Slice 6

## Proposed package layout

```text
lib/sdk/
  bootstrap_client.dart
  today_client.dart
  settings_client.dart
  plan_client.dart
  wordbook_client.dart
  study_client.dart
  reports_client.dart
  wrong_words_client.dart
  ai_client.dart
```

## Proposed client interfaces

### BootstrapClient

Responsibilities:

- bootstrap state read
- onboarding completion acknowledgement

Suggested methods:

- `Future<BootstrapState> getBootstrapState()`
- `Future<void> markOnboardingCompleted()`

### TodayClient

Responsibilities:

- read current today home state

Suggested methods:

- `Future<TodayHomeState> getTodayHomeState()`

### SettingsClient

Responsibilities:

- read app/runtime settings summary
- later AI provider config access

Suggested methods:

- `Future<SettingsSummary> getSettings()`
- second wave:
  - `Future<AiProviderConfig> getAiProviderConfig()`
  - `Future<AiProviderConfig> saveAiProviderConfig(...)`

### PlanClient

Responsibilities:

- active plan read
- plan mutation
- apply saved plan to today

Suggested methods:

- `Future<PlanSummary?> getActivePlan()`
- `Future<PlanSummary> savePlan({required int planId, required PlanEditInput input})`
- `Future<PlanSummary> applySavedPlanToToday()`

### WordbookClient

Responsibilities:

- list wordbooks
- toggle activation state

Suggested methods:

- `Future<List<WordbookSummary>> getWordbooks()`
- `Future<void> toggleWordbook({required int wordbookId, required bool isActive})`

### StudyClient

Responsibilities:

- start session
- submit answer
- complete session
- cancel session

Suggested methods:

- `Future<StartSessionResponse> startStudySession(StartSessionRequest request)`
- `Future<SubmitAnswerResponse> submitStudyAnswer(SubmitAnswerRequest request)`
- `Future<CompleteSessionResponse> completeStudySession(String sessionId)`
- `Future<void> cancelStudySession(String sessionId)`

### ReportsClient

Responsibilities:

- read reports overview

Suggested methods:

- `Future<ReportsOverview> getReportsOverview()`

### WrongWordsClient

Responsibilities:

- wrong-word list and detail

Suggested methods:

- `Future<List<WrongWordEntry>> getWrongWords(String filter)`
- `Future<WrongWordDetail> getWrongWordDetail(int entryId)`

### AiClient

Responsibilities:

- non-blocking AI context/history/content methods

Suggested methods:

- `Future<TodayAiPassageContext> getTodayAiPassageContext()`
- `Future<AIPassage> generateAiPassage({required List<String> targetWords, required String level})`
- `Future<List<AIPassageHistoryItem>> getAiPassageHistory()`
- `Future<AIPassage?> getAiPassage(String passageId)`
- `Future<void> saveAiPassage(AIPassage passage)`

## Internal SDK shape

Recommended internal structure:

- one public client per domain
- one shared `RustBridge` transport wrapper
- one shared codec/mapper layer
- one shared normalized SDK error hierarchy

Feature code should see:

- typed request objects
- typed response objects
- typed exceptions

Feature code should not see:

- bridge method names
- raw success/error envelopes
- platform channel names
- Objective-C/Java/JNI strings

## Model source rules

- Shared DTO semantics come from Rust-owned contracts.
- Dart models should mirror shared contracts, not mirror platform adapter payload quirks.
- Flutter-only view models must stay outside shared DTO model files.

## Error surface rules

- Clients should throw normalized SDK exceptions in v1.
- Clients must not expose raw Objective-C, Java, JNI, or symbol-resolution strings as stable app API.
- Protocol decode failures must be distinguishable from domain failures.
- Unsupported-path failures must be distinguishable from ordinary runtime failures.

## Suggested SDK error classes

- `SdkDomainException`
- `SdkRuntimeException`
- `SdkPlatformException`
- `SdkProtocolException`
- `SdkUnsupportedException`

## Method naming rules

- use domain names already frozen by the contract inventory
- do not rename semantics just to fit Flutter naming taste
- keep `save` and `apply` distinct where Rust semantics are distinct
- keep `cancel` and `complete` distinct where Rust semantics are distinct

## Logging rules

- SDK may log method name, latency, and error class.
- SDK must not log tokens, secrets, or full answer content in production logs.
- debug logging should be centrally gated, not reimplemented per feature screen

## First-wave smoke readiness

Slice 4 SDK should be sufficient to support smoke calls for:

- bootstrap
- today
- study round-trip
- reports overview
- wrong-word read

## Open questions

- Whether AI methods belong in the initial Flutter shell or second wave
- Whether Dart DTOs should be generated or handwritten against Rust source metadata
- Whether debug mode should later expose result-wrapper helpers on top of exception-first APIs

## Exit criteria

- Each first-wave contract family has a Flutter SDK home.
- No Flutter feature code needs direct bridge access.
- Error behavior is consistent across all first-wave clients.
- The SDK surface is stable enough to support feature work in Slice 6.
