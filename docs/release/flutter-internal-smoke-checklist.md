# Flutter Internal Smoke Checklist

Status: Active draft
Owner: Mobile release / QA
Phase: Slice 8 of `2026-04-22-flutter-app-supabase-implementation`

## Purpose

This checklist defines the minimum smoke surface before treating the Flutter
mobile shell as an internal-test candidate.

It is intentionally execution-oriented: each item should be pass/fail, not a
design discussion.

## Build And Install

- Run `flutter pub get` from `apps/flutter_mobile`.
- Run `flutter build apk --release`.
- Confirm APK exists at
  `apps/flutter_mobile/build/app/outputs/flutter-apk/app-release.apk`.
- Install with `adb install -r` rather than uninstall/reinstall.
- Launch package `com.wordmobile`.

## Identity Continuity

- Android `applicationId` is `com.wordmobile`.
- iOS bundle identifier is `com.wordmobile`.
- App is installed as an upgrade over the previous package when validating data
  continuity.
- Local app data is not cleared manually during upgrade validation.

## Bootstrap Smoke

- App starts without showing the Flutter counter demo.
- Rust bridge initializes.
- Bootstrap route resolves to onboarding, today, or a typed startup error.
- Startup error shows a supportable code/message instead of a blank screen.

## Auth Smoke

- With no Supabase `dart-define`, Account shows `not configured` safely.
- With Supabase configured, signup either establishes a session or clearly asks
  for email confirmation.
- Login transitions to `signed_in_active`.
- Logout transitions to `signed_out_retained_local`.
- Restart restores the expected account state.
- Expired/revoked/deleted account cases degrade into typed account state and do
  not block local study bootstrap.

## Study Loop Smoke

- Today loads through the Rust bridge.
- Plan opens and saves without blocking on cloud sync.
- Saved plan can be applied to Today.
- Study starts from Today.
- Answer submit advances or returns a typed error.
- Complete/cancel returns to Today through authoritative refresh.
- Wrong Words and Reports open after at least one completed study flow.

## Sync Smoke

- Saving a plan creates or updates a Rust-owned `plan_config` outbox item.
- Today sync card shows pending count and pending domain.
- Sync status read failure does not block Today or Study entry.
- Queue mutation is not exposed through Flutter UI.
- Cloud transport remains disabled until Supabase transport is implemented and
  verified.

## Supabase Smoke

- `supabase db reset` succeeds in a local Supabase environment.
- Unauthenticated clients cannot read protected tables.
- User A cannot read or write User B rows.
- User can register its own device through `register_device`.
- `sync_dead_letters` has no ordinary client write path.

## Rollback / Retained Local Smoke

- Local SQLite remains readable after logout.
- Secure auth session clearing does not delete study history.
- Retained-local mode allows Today / Plan / Study entry.
- Rollback testing does not rely on uninstall/reinstall unless explicitly
  testing reinstall behavior.

## Known Current Gaps

- Flutter CLI commands are timing out in the current Windows environment.
- Supabase CLI is not installed in the current environment.
- Cloud push/pull transport is not implemented yet.
