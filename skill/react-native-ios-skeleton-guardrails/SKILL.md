---
name: react-native-ios-skeleton-guardrails
description: Guard React Native iOS scaffold boundaries in apps/mobile/ios. Use when work touches the legacy React Native iOS project scaffold, native bridge surface, Pod wiring, bundle resource layout, or remote iOS build workflow docs.
---

# RN iOS Skeleton Guardrails

## Purpose

This skill records the intended boundaries for the initial React Native iOS skeleton in `apps/mobile/ios`.
Use it whenever work touches the iOS project scaffold, native bridge surface, Pod wiring, or bundle resource layout.
The goal is to prevent accidental edits that blur the line between:
- React Native app bootstrap
- iOS native bridge glue
- Rust mobile core ownership
- remote build workflow concerns

## Scope

This guardrail applies to:
- `apps/mobile/ios/Podfile`
- `apps/mobile/ios/WordMobile/*`
- `apps/mobile/ios/WordMobile.xcodeproj/*`
- future iOS native bridge files under `apps/mobile/ios/WordMobile/`
- future remote iOS build workflow docs/config tied to this skeleton

## Boundary rules

### 1. React Native bootstrap stays thin

`AppDelegate` and related app bootstrap files should only:
- configure the React Native root module
- point to the JS bundle location
- initialize native platform services needed before JS starts
- later pass sandbox and bundle paths into the Rust/mobile bridge

Do not place product business logic, study rules, wordbook parsing, or report assembly into `AppDelegate`.

### 2. JS bridge contract is source-first

The canonical JS-facing contract remains `apps/mobile/src/lib/mobile-bridge.ts`.
Any iOS native module must implement that contract rather than inventing iOS-only method names or payload shapes.
If an API changes, update the JS contract deliberately and then update Android/iOS together.

### 3. Rust owns shared product behavior

The iOS native layer should prefer:
- forwarding requests to Rust
- translating primitive arguments and JSON payloads
- mapping iOS filesystem/bundle paths into the Rust runtime

Do not re-implement Android Java fallback business logic in Swift/Objective-C unless the plan explicitly says that a temporary iOS-only fallback is required.

### 4. Resource layout is explicit

The initial skeleton must preserve a clear distinction between:
- app bundle resources shipped read-only with the app
- writable sandbox data such as database/config/cache/logs

Do not hardcode temporary absolute paths.
Do not hide seed-resource assumptions inside unrelated native files.
Any new resource dependency should be documented alongside the Rust/mobile path model.

### 5. Remote build concerns stay outside app logic

`ios-builder`, signing, `xcodebuild archive`, export options, and CI secrets are delivery concerns.
They may shape project settings and docs, but they must not leak app business decisions into native runtime code.

## Current intended interfaces

### React Native entry
- App name: `WordMobile`
- JS registration source: `apps/mobile/index.js`
- iOS root module name must remain `WordMobile` unless JS registration changes intentionally

### Native bridge contract
- Expected native module name: `WordCoreModule`
- JS contract source: `apps/mobile/src/lib/mobile-bridge.ts`
- iOS implementation status: contract-only stub exists in `apps/mobile/ios/WordMobile/WordCoreModule.mm` and currently rejects every method with `E_WORDCORE_NOT_IMPLEMENTED`

### Rust runtime contract
- iOS native code will eventually provide:
  - app data directory
  - app config/no-backup directory equivalent
  - app cache directory
  - bundle resource directory
- Those paths must feed the shared mobile runtime instead of platform-specific business forks
- Current Rust iOS FFI header: `crates/platform-mobile/include/word_platform_mobile_ios.h`
- Current minimal exported functions: `initialize`, `get_bridge_status`, `get_bootstrap_state`, `mark_onboarding_completed`, `get_today_home_state`, `get_settings`, `string_free`
- Current FFI error convention: string results prefixed with `__WORDMOBILE_ERROR__:` represent bridge/runtime failure

## Safe modifications during skeleton phase

Safe:
- adding missing RN template files
- fixing target names, bundle ids, plist entries, and Pod target wiring
- preparing file locations for future `WordCoreModule` and Rust bridge glue
- documenting path and ownership rules

Unsafe without plan update:
- adding business logic into `AppDelegate`
- changing JS/native method names independently on iOS
- introducing iOS-only payload formats
- copying Android Java study/report logic into iOS native files
- mixing CI/signing concerns into runtime bridge classes

## Change protocol

When modifying the iOS skeleton, always note:
1. whether the change is bootstrap-only, bridge-only, Rust-boundary-only, or delivery-only
2. whether it changes the JS contract in `mobile-bridge.ts`
3. whether Android must change in lockstep
4. whether the Vico plan acceptance criteria should be updated
