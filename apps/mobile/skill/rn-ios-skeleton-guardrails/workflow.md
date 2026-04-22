# RN iOS Skeleton Workflow

## Purpose

This workflow captures the intended commands around the initial iOS skeleton so future edits do not guess at Pod or Xcode entrypoints.

## Preconditions

- project root: `D:\projects\word-mobile-rn\apps\mobile`
- iOS project dir: `apps/mobile/ios`
- Xcode target / scheme: `WordMobile`
- bundle id placeholder: `com.wordmobile`
- current native bridge state: `WordCoreModule` exists as a contract-only stub and returns `E_WORDCORE_NOT_IMPLEMENTED`
- current Rust iOS FFI surface: `crates/platform-mobile/include/word_platform_mobile_ios.h`
- current Rust iOS build script: `apps/mobile/ios/scripts/build-rust-ios.sh`
- current iOS bundle resource sync script: `apps/mobile/ios/scripts/sync-ios-bundle-resources.sh`

## Baseline Pod install

Run after remote Mac checkout and Node dependency restore:

```bash
cd apps/mobile/ios
pod install
```

Expected result:
- `apps/mobile/ios/WordMobile.xcworkspace` exists
- CocoaPods target support files are generated for `WordMobile` and `WordMobileTests`

## Baseline simulator build

Use after Pod install when validating the pure RN skeleton:

```bash
xcodebuild \
  -workspace apps/mobile/ios/WordMobile.xcworkspace \
  -scheme WordMobile \
  -configuration Debug \
  -sdk iphonesimulator \
  -destination 'platform=iOS Simulator,name=iPhone 15' \
  build
```

This now also triggers the Xcode build phase `Build Rust iOS library`, which copies
`libword_platform_mobile.a` into `apps/mobile/ios/rust-artifacts/<platform>-<arch>/`.

It also triggers `Sync iOS bundle resources`, which copies:
- `seed-vocab`
- `seed-medical`
- optional `apps/mobile/resources/vocab-snapshot/vocab-snapshot.jsonl`

## Baseline archive command

Use as the starting point for remote release packaging or `ios-builder` workflows:

```bash
xcodebuild \
  -workspace apps/mobile/ios/WordMobile.xcworkspace \
  -scheme WordMobile \
  -configuration Release \
  -destination 'generic/platform=iOS' \
  -archivePath apps/mobile/ios/build/WordMobile.xcarchive \
  archive
```

## Guardrails

- Do not treat the current bridge stub as product-complete iOS support.
- Do not bypass `apps/mobile/ios/scripts/build-rust-ios.sh` by hardcoding one-off local library paths in Xcode settings.
- Do not hardcode bundle resource files directly into unrelated native classes; keep resource layout flowing through `apps/mobile/ios/scripts/sync-ios-bundle-resources.sh` and `apps/mobile/resources/`.
- Do not add business logic to make the archive succeed faster.
- If build settings change because of Rust integration, document whether the change is bootstrap-only or Rust-boundary-only.
- If the scheme, target name, or workspace path changes, update `SKILL.md` and this workflow in the same patch.
