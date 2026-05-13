---
name: flutter-android-release-wireless-deploy
description: Build, verify, install, launch, and visually check the Flutter Android release APK for com.wordmobile over adb. Use when the requested target is the Flutter mobile app, especially after Flutter UI, Rust bridge, seed-vocab, Today/study-flow, account, auth, cloud sync, or Supabase changes.
---

# Flutter Android Release Wireless Deploy

Use this skill for the Flutter app under `apps/flutter_mobile`. Do not use the
React Native cached Android build workflow for Flutter fixes.

For release builds, prefer the repo script:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1
```

For `word_admin` builds, do not use the default Supabase deploy script. Follow
the `Word Admin Variant` section in [workflow.md](workflow.md). The correct
flow reads `.env.word-admin.local`, sets the same fixed JDK/Android/Gradle/PATH
environment as the stable release script, runs Flutter outside the Codex
sandbox when needed so `D:\flutter\flutter\bin\cache\lockfile` is writable,
then inspects, installs, and launches the APK.

That script is the default because it:

- Loads `SUPABASE_URL` and `SUPABASE_ANON_KEY` from repo-root `.env.supabase.local`.
- Passes those values as Flutter `--dart-define` values so account/auth works in release APKs.
- Uses the full Microsoft JDK with `jlink.exe`.
- Builds the release APK.
- Verifies the packaged Rust bridge library.
- Installs with `adb install -r`.
- Launches `com.wordmobile`.

Before any account/auth/cloud/Supabase-related packaging or debugging, also apply
the `supabase-mobile-auth-checks` skill:

- `.env.supabase.local` must exist at repo root.
- `.env.supabase.local` must contain `SUPABASE_URL=` and `SUPABASE_ANON_KEY=`.
- `.env.supabase.local` must be git-ignored.
- `apps/flutter_mobile/lib/supabase/supabase_config.dart` must read
  `String.fromEnvironment`.
- Do not expose anon keys, tokens, refresh tokens, or service-role keys in logs
  or final messages.
- If an installed release APK says Supabase is not configured, assume it was
  built without dart-defines until proven otherwise.

Do not use bare `flutter build apk --release` for release APKs that need account,
auth, cloud sync, or Supabase validation. If building manually, include both
Supabase dart-defines explicitly.

Follow [workflow.md](workflow.md) every time. Check
[troubleshooting.md](troubleshooting.md) when any step fails or when Today data,
task breakdown, bundled vocab assets, native Rust bridge, adb install, or
device screenshots behave unexpectedly.
