# Flutter Bridge Architecture

Status: Draft
Owner: Mobile platform layer + Rust shared core
Phase: Slice 4 of `2026-04-22-flutter-rust-supabase-rearchitecture`

## Purpose

This document defines the intended architecture for the Flutter mobile shell and its bridge into the Rust shared core.

It exists to prevent three failure modes:

- Flutter UI reaching directly into low-level bridge calls
- platform-specific adapter details leaking into shared contracts
- Rust domain truth getting reimplemented in Dart

## Goals

- Keep Rust as the owner of study, today, wrong-word, and report truth.
- Give Flutter a stable typed SDK surface.
- Reuse the existing Android/iOS adapter direction where it reduces risk.
- Keep lifecycle, path, and secure-storage concerns in adapter layers.

## Non-goals

- Defining Supabase schema or auth flows in detail
- Rewriting learning rules in Flutter
- Encoding all future sync behavior in the bridge layer

## Slice 4 first-pass decisions

- v1 bridge transport should use `platform channel -> native adapter -> Rust`.
- `dart:ffi` is not the default first delivery path.
- Flutter UI never calls raw bridge primitives directly.
- Flutter SDK is the only Flutter-facing bridge consumer.
- Adapter-private details stay adapter-private even if Android/iOS implementations differ.
- Shared DTO semantics remain Rust-owned and platform-neutral.

## Layered architecture

```text
Flutter UI
  -> Flutter SDK
    -> Bridge / codec layer
      -> Native adapter layer
        -> Rust shared core
```

## Layer responsibilities

### Flutter UI

Owns:

- routing
- view composition
- user input
- loading/error/empty states

Does not own:

- session progression truth
- answer evaluation
- today snapshot generation
- report aggregation

### Flutter SDK

Owns:

- typed client interfaces
- request construction
- response decoding
- error normalization
- app-facing API ergonomics

Does not own:

- platform path logic
- secret storage implementation
- domain rule composition

### Bridge / codec layer

Owns:

- method dispatch across the Flutter/native boundary
- payload encoding/decoding
- protocol validation
- bridge-level logging and tracing

Does not own:

- business fallback logic
- domain state derivation

### Native adapter layer

Owns:

- Android/iOS runtime bootstrap
- app sandbox path resolution
- secure storage access
- lifecycle hook handoff
- native-to-Rust symbol invocation

Does not own:

- UI state
- shared DTO semantics
- study correctness rules

### Rust shared core

Owns:

- shared DTO semantics
- today state assembly
- study session lifecycle
- answer evaluation
- wrong-word updates
- report aggregation

Does not own:

- Flutter route structure
- platform widget lifecycle
- platform-specific keychain/keystore details

## Current reusable assets

Existing repository assets that should inform the first Flutter bridge:

- Android adapter: [RustBridge.java](/d:/projects/word-mobile-rn/apps/mobile/android/app/src/main/java/com/wordmobile/RustBridge.java)
- iOS adapter: [WordCoreModule.mm](/d:/projects/word-mobile-rn/apps/mobile/ios/WordMobile/WordCoreModule.mm)
- iOS FFI header: [word_platform_mobile_ios.h](/d:/projects/word-mobile-rn/crates/platform-mobile/include/word_platform_mobile_ios.h)
- Current RN contract surface: [mobile-bridge.ts](/d:/projects/word-mobile-rn/apps/mobile/src/lib/mobile-bridge.ts)
- Contract inventory: [contract-inventory.md](/d:/projects/word-mobile-rn/docs/contracts/contract-inventory.md)
- Bridge mapping: [bridge-mapping-matrix.md](/d:/projects/word-mobile-rn/docs/contracts/bridge-mapping-matrix.md)

## Recommended first implementation direction

Preferred first path:

1. Keep Rust contract semantics unchanged.
2. Reuse native adapter patterns already proven in Android/iOS.
3. Put a typed Flutter SDK in front of bridge calls.
4. Delay more direct `dart:ffi` experimentation until adapter behavior is stable.

Rationale:

- Android/iOS bootstrap and runtime wiring already exist.
- The riskiest work is runtime integration, not widget rendering.
- A stable adapter-first bridge reduces early migration variance.

## Transport decision

### Chosen v1 direction

- Flutter uses platform channels to talk to the native adapter layer.
- Native adapter invokes Rust through the existing platform-native direction.
- Rust returns platform-neutral payloads that the adapter wraps into a stable bridge envelope.

### Why not direct `dart:ffi` first

- it would push more iOS/Android/runtime bootstrapping risk into the first Flutter slice
- it increases early symbol, lifecycle, and packaging variance
- it makes the first migration problem more about transport mechanics than product correctness

### When `dart:ffi` may be reconsidered later

- adapter behavior is already stable
- startup and lifecycle hooks are already proven
- there is a measured performance or maintainability reason to bypass the v1 transport

## Bridge protocol rules

- every bridge call should have a stable method name and a typed request/response contract
- requests and successful responses should be envelope-wrapped, not ad hoc payloads
- error responses should use the shared bridge error envelope from [bridge-error-model.md](/d:/projects/word-mobile-rn/docs/flutter/bridge-error-model.md)
- method names should stay domain-oriented, not platform-oriented
- adapter-private diagnostics may exist internally, but are not part of the stable Flutter contract

## Suggested bridge envelope

### Success envelope

```json
{
  "ok": true,
  "data": {}
}
```

### Error envelope

```json
{
  "ok": false,
  "error": {
    "category": "runtime_error",
    "code": "RUST_RUNTIME_INIT_FAILED",
    "message": "Runtime initialization failed.",
    "retryable": true
  }
}
```

Rules:

- `ok` is mandatory
- `data` exists only on success
- `error` exists only on failure
- Flutter SDK must not inspect native implementation details outside the envelope

## Package and directory shape

Recommended Flutter-side shape:

```text
apps/flutter_mobile/
  lib/
    app/
      router/
      theme/
      bootstrap/
    sdk/
      bootstrap_client.dart
      today_client.dart
      settings_client.dart
      plan_client.dart
      wordbook_client.dart
      study_client.dart
      reports_client.dart
      wrong_words_client.dart
      ai_client.dart
    bridge/
      rust_bridge.dart
      bridge_codec.dart
      bridge_method_names.dart
      bridge_error.dart
    features/
      today/
      plan/
      study/
      reports/
      wrong_words/
      ai/
    state/
      app_state.dart
      session_state.dart
```

## Bridge boundary rules

- Flutter UI must never call raw native methods directly.
- SDK must be the only Flutter-facing entrypoint into bridge operations.
- Native adapter must not return platform-private error text as the shared contract.
- Shared contract DTOs must remain platform-neutral.
- Rust must not know about Flutter widgets, routes, or screen-specific loading flows.
- Flutter feature code must not hold both Supabase client logic and Rust bridge write logic in parallel.

## First-wave bridge scope

The first bridgeable bundle should include:

- bootstrap
- today
- settings summary
- plan read/write
- wordbook list/toggle
- study start/submit/complete/cancel
- reports overview
- wrong-word list/detail

Second wave:

- AI provider config
- AI passage generation/history/read/save
- later auth/sync diagnostics

## Smoke paths Slice 4 should freeze

### Startup smoke

- Flutter -> bootstrap call -> valid bootstrap DTO
- Flutter -> today read -> valid today DTO

### Session smoke

- start study -> submit answer -> complete session round-trip

### Read smoke

- reports overview read
- wrong-word list read
- wrong-word detail read

### Failure smoke

- runtime init failure
- secure storage/path failure in adapter
- protocol decode failure
- unsupported symbol or method path

## Downstream handoff

- Slice 5 can add auth/runtime layering without redefining the bridge stack.
- Slice 6 can build Today/Plan/Study flows against the SDK instead of bridge primitives.
- Slice 7 can extend the same stack for reports/wrong words/AI surfaces.

## Open questions

- Whether a single shared bridge codec can cover both Android and iOS without drift
- Whether AI methods should enter the first-wave bridge or remain second-wave
- Whether debug builds should expose richer adapter diagnostics behind an explicit toggle

## Exit criteria

- Architecture is specific enough to build the Flutter SDK without guessing ownership.
- The v1 transport decision is frozen.
- The first-wave bridge scope is frozen.
- Adapter-private concerns are clearly separated from shared DTO concerns.
