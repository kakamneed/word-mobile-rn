# Word Mobile

Forward mobile client for the Word learning product, built around a shared Rust
core, local SQLite runtime, and the ongoing Flutter + Supabase migration.

React Native remains in this repository as legacy/reference code while the
active mobile delivery path moves to Flutter.

## Goal

Migrate the current desktop-first vocabulary learning app to mobile without rewriting the core study rules, offline storage model, or vocabulary pipeline.

## Architecture Direction

- Forward mobile shell: Flutter under `apps/flutter_mobile`
- Legacy mobile shell: React Native under `apps/mobile`
- Shared domain core: Rust crates
- Local persistence: SQLite on device
- Cloud account/sync direction: Supabase
- AI boundary: Rust-side service layer, non-blocking to main study flow
- Product rule: offline-first, local-first, study-loop-first

## Active Android Release Install Path

Use the Flutter workflow:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
.\scripts\android-release-wireless-deploy.ps1
```

Expected APK:

```text
apps/flutter_mobile/build/app/outputs/flutter-apk/app-release.apk
```

The old React Native cached-build workflow under
`skill/legacy-react-native-android-release-wireless-deploy/` is deprecated for current
delivery and should be used only as historical reference.

## Data Continuity

Flutter is aligned to the legacy app identity so upgrade-style installation can
preserve local app data:

- Android `applicationId`: `com.wordmobile`
- iOS bundle identifier: `com.wordmobile`

Avoid uninstall/reinstall when validating continuity; use upgrade install
(`adb install -r`) with the same package identity.

## Planning Docs

- `.vico/plans/active/2026-04-22-flutter-app-supabase-implementation.md`
- `.vico/plans/active/2026-04-22-flutter-rust-supabase-rearchitecture.md`
- `docs/flutter/`
- `docs/supabase/`
- `docs/sync/`
- `docs/release/`

## Recommended Next Step

Continue the active Vico plan for the Flutter + Supabase migration. Current
work is centered on the Flutter shell, Rust bridge parity, Supabase auth, local
sync outbox/status, and replacing RN delivery workflows with Flutter workflows.
