# iOS Migration Inventory

## Purpose

This inventory captures the current migration boundary for the iOS port so future work stays inside the active iOS slug and does not reopen the older completed Rust/study plan.

## Planning scope

- Active execution slug: `2026-04-19-ios-migration-and-remote-build`
- Related completed slug to treat as read-only context: `2026-04-16-rust-mobile-bridge-and-study-fixes`
- Rule: do not reopen the older slug unless new evidence shows iOS migration is blocked by a missing shared-core capability.

## Source-of-truth layers

### JavaScript contract
- Primary file: `apps/mobile/src/lib/mobile-bridge.ts`
- Rule: all iOS native bridge work must implement this contract instead of inventing iOS-only API names or payloads.

### Android reference bridge
- Main package registration: `apps/mobile/android/app/src/main/java/com/wordmobile/MainApplication.java`
- Native module entry: `apps/mobile/android/app/src/main/java/com/wordmobile/WordCoreModule.java`
- Android Rust loader: `apps/mobile/android/app/src/main/java/com/wordmobile/RustBridge.java`
- Rule: Android is the behavioral reference for module shape, but not the place to copy product logic from.

### Shared Rust core
- Shared bridge functions: `crates/platform-mobile/src/bridge.rs`
- Mobile path model: `crates/platform-mobile/src/paths.rs`
- iOS FFI wrapper: `crates/platform-mobile/src/ios.rs`
- iOS C header: `crates/platform-mobile/include/word_platform_mobile_ios.h`
- Rule: shared product behavior should converge here.

## JavaScript API mapping

### Startup-path methods wired to Rust FFI
These methods now attempt to initialize the iOS runtime, resolve Rust symbols dynamically with `dlsym`, and call into the current Rust iOS FFI surface:

- `getBootstrapState`
- `markOnboardingCompleted`
- `getTodayHomeState`
- `getSettings`

If Rust symbols are not linked yet, these methods reject with `E_WORDCORE_RUST_NOT_LINKED`.
If Rust returns an error-prefixed string, these methods reject with `E_WORDCORE_RUST_RUNTIME`.

### Direct-pass methods now wired in the iOS native bridge
These methods now forward directly from `apps/mobile/ios/WordMobile/WordCoreModule.mm` into Rust iOS FFI:

- `getActivePlan`
- `savePlan`
- `applySavedPlanToToday`
- `getWordbooks`
- `toggleWordbook`
- `startStudySession`
- `submitStudyAnswer`
- `completeStudySession`
- `cancelStudySession`
- `generateAiPassage`
- `getAiPassageHistory`
- `getAiPassage`
- `saveAiPassage`

### Still stubbed in iOS native bridge
These methods still reject with `E_WORDCORE_NOT_IMPLEMENTED` because Android currently builds local request JSON before calling Rust and iOS has not yet recreated that native-side assembly:

- `getReportsOverview`
- `getWrongWords`
- `getWrongWordDetail`
- `getTodayAiPassageContext`

### Backed by Rust iOS FFI today
These Rust exports exist and are ready to be consumed by ObjC/Swift glue:

- `word_mobile_ios_initialize`
- `word_mobile_ios_get_bridge_status`
- `word_mobile_ios_get_bootstrap_state`
- `word_mobile_ios_mark_onboarding_completed`
- `word_mobile_ios_get_today_home_state`
- `word_mobile_ios_get_settings`
- `word_mobile_ios_build_today_home_state`
- `word_mobile_ios_build_reports_overview`
- `word_mobile_ios_build_wrong_words`
- `word_mobile_ios_build_wrong_word_detail`
- `word_mobile_ios_build_today_ai_passage_context`
- `word_mobile_ios_get_active_plan`
- `word_mobile_ios_save_plan`
- `word_mobile_ios_apply_saved_plan_to_today`
- `word_mobile_ios_get_wordbooks`
- `word_mobile_ios_toggle_wordbook`
- `word_mobile_ios_start_study_session`
- `word_mobile_ios_submit_study_answer`
- `word_mobile_ios_complete_study_session`
- `word_mobile_ios_cancel_study_session`
- `word_mobile_ios_save_ai_passage`
- `word_mobile_ios_get_ai_passage_history`
- `word_mobile_ios_get_ai_passage`
- `word_mobile_ios_generate_ai_passage`
- `word_mobile_ios_string_free`

### Still missing above the Rust iOS FFI layer
The Rust iOS FFI surface now exists for the full current bridge area, but these pieces still need native-side implementation work:

- ObjC methods beyond the startup path still need to stop rejecting and start forwarding into the Rust FFI.
- Native-side request builders are still needed where Android currently assembles local-state JSON before calling Rust.
- The Xcode target still needs the Rust library actually linked so dynamic symbol lookup can succeed at runtime.

## Resource and path dependencies

### Read-only bundle resources required by iOS migration
- `vocab-snapshot/vocab-snapshot.jsonl`
- Android seed vocabulary assets currently under `apps/mobile/android/app/src/main/assets/seed-vocab/`
- Android medical root-affix asset currently under `apps/mobile/android/app/src/main/assets/seed-medical/medical-root-affix.txt`

### Current iOS bundle sync path
- Xcode phase: `Sync iOS bundle resources`
- Script: `apps/mobile/ios/scripts/sync-ios-bundle-resources.sh`
- Stable snapshot source location: `apps/mobile/resources/vocab-snapshot/vocab-snapshot.jsonl`

### Writable runtime directories required by the mobile runtime
- app data directory
- app config / no-backup directory equivalent
- app cache directory
- app logs directory derived by Rust from cache

### Current risk
- The iOS skeleton now has a bundle-resource sync step, but only `seed-vocab` and `seed-medical` have concrete sources today.
- Startup-path ObjC methods now compute iOS sandbox paths and call Rust dynamically, and the Xcode target now has a Rust build phase plus linker search path skeleton; this path is not yet validated on a real Mac build.
- Non-startup bridge methods are still stubs and will fail until the remaining iOS FFI surface is added.
- `reports / wrong words / today ai context` still need iOS-native request builders before their Rust-backed path can replace the stub.
- The bundle sync path now exists, but `apps/mobile/resources/vocab-snapshot/vocab-snapshot.jsonl` is still missing, so bootstrap should still be expected to fail until that resource is provided.

## Delivery entrypoints

- Pod install entry: `apps/mobile/skill/rn-ios-skeleton-guardrails/workflow.md`
- Baseline simulator build entry: `apps/mobile/skill/rn-ios-skeleton-guardrails/workflow.md`
- Baseline archive entry for `ios-builder`: `apps/mobile/skill/rn-ios-skeleton-guardrails/workflow.md`

## Next implementation order

1. Validate the new Rust build phase and linker settings on a real Mac build.
2. Decide where bundle resources will live in the Xcode target and record that layout.
3. Add iOS-native request builders for `reports / wrong words / today ai context`.
4. Replace the remaining iOS stubs with real Rust-backed implementations.
5. Only then wire `ios-builder` workflow details on top of the stabilized native boundary.
