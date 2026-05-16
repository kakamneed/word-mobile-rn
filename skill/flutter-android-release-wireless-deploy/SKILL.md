---
name: flutter-android-release-wireless-deploy
description: Build, verify, install, launch, and visually check the Flutter Android release APK for com.wordmobile over adb. Use when the requested target is the Flutter mobile app, especially after Flutter UI, Rust bridge, seed-vocab, Today/study-flow, account, auth, cloud sync, or Supabase changes.
---

# Flutter Android Release Wireless Deploy

Use this skill for the Flutter app under `apps/flutter_mobile`. Do not use the
React Native cached Android build workflow for Flutter fixes.

For learning-flow work, also read
`.planning/skills/learning-flow/RELEASE-VALIDATION.md` before closing the task.
It defines the current Phase 11 result accounting terms (`PASS`, `PASS WITH NOTE`,
`BLOCKED`, `DEFERRED`, `HUMAN NEEDED`) and the required cold-start Study smoke.

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

## Supabase APK Update Rules

Treat `1.0.0+1` as the initial Flutter APK that contains the self-update client.
It may be installed directly, but do not publish it as an update record for
already-installed `versionCode=1` clients.

For every later APK intended for Supabase direct update delivery:

- Increase `apps/flutter_mobile/pubspec.yaml` `version:` before building.
  Android `versionCode` is the number after `+`; it must strictly increase.
- Use the Supabase release build path so the APK includes Supabase dart-defines.
- Inspect the built APK with `aapt dump badging` and confirm `versionCode`
  matches the intended update record.
- Compute the APK SHA-256 and store it in `public.app_releases.sha256`.
- Publish the APK as a GitHub Release asset and store that asset URL in
  `public.app_releases.download_url`. Do not use Supabase Storage as the normal
  APK host; APKs are large enough to consume the free storage quota quickly.
- Insert or update one enabled `public.app_releases` row for
  `platform='android'`, `runtime='flutter'`, and the target `channel`.
- Never mark a release row `enabled=true` until the uploaded APK exists, the
  `sha256` matches the file, and `version_code` is greater than the installed
  baseline that should receive the update.
- GitHub CLI login is persistent. Run `gh auth login` only when `gh auth status`
  says the token is missing or invalid. In this workspace, GitHub access usually
  needs proxy `http://127.0.0.1:7897`.

Distinguish test packages from staged stable packages:

- Test packages are for the developer's device only. Install them with wireless
  adb (`adb install -r`) and do not create an enabled `app_releases` row for
  them. They must not be delivered through the client self-update flow.
- Staged stable packages are the only APKs that may be uploaded to GitHub
  Releases and enabled in `public.app_releases` for automatic client update
  checks.
- If a test package needs a higher `versionCode` to install over a local build,
  that is still a local testing version. Do not reuse that row as an automatic
  update unless the user explicitly promotes the same APK to staged stable after
  validation.

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
