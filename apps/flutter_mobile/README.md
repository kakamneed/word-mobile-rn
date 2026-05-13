# flutter_mobile

Flutter mobile shell for the Word app.

## Current role

This app is the forward mobile shell for the ongoing Flutter replacement work.
It is no longer treated as a demo-only scaffold.

## Android release install chain

Preferred release chain:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
.\scripts\android-release-wireless-deploy.ps1
```

Current known-good environment for this flow:

- Flutter SDK: `D:\flutter\flutter`
- Full JDK with `jlink.exe`: `C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot`
- Android SDK: `D:\Android\Sdk`
- Android Build-Tools: `35.0.0`
- Android CMake: `3.22.1`
- Rust cargo bin: `D:\msys64\home\clf20\.cargo\bin`

What it does:

1. `flutter pub get`
2. `flutter build apk --release` with Supabase `dart-define` values when
   `.env.supabase.local` exists
3. verify APK contains `lib/arm64-v8a/libword_platform_mobile.so`
4. `adb devices`
5. `adb install -r`
6. launch `com.wordmobile`

Current local optimization:

- reuse the existing Gradle cache under `apps/mobile/.gradle-home`
- prefer the repo-local `D:\projects\word-mobile-rn\.gradle-home\gradle-8.13-bin.zip`
  distribution instead of forcing a fresh Gradle download

Expected APK path:

```text
apps/flutter_mobile/build/app/outputs/flutter-apk/app-release.apk
```

Current signing behavior:

- Flutter release is signed with the legacy RN debug keystore at
  `apps/mobile/android/app/debug.keystore`

That is required so `adb install -r` can upgrade the existing
`com.wordmobile` install without losing its package data container.

## Data continuity note

This Flutter app is being aligned to the legacy app identity so upgrade-style installation can preserve local app data.

Important distinction:

- changing the app display name usually does not affect local data
- changing Android `applicationId` or iOS `bundle identifier` does affect the app data container

## Supabase note

Supabase integration is being added incrementally.
Core learning truth remains Rust + local SQLite first.

## Word Admin local backend run

The app can target the local `D:\projects\word-admin` backend for account auth
with these Dart defines:

```powershell
flutter run `
  --dart-define=CLOUD_BACKEND=word_admin `
  --dart-define=WORD_ADMIN_API_URL=http://<computer-lan-ip>:8787
```

For a physical phone, do not use `127.0.0.1`; use the computer's LAN IP and
make sure the phone can reach port `8787`.

Repo-root helpers read `.env.word-admin.local`, which is ignored by git:

```text
WORD_ADMIN_API_URL=http://<computer-lan-ip>:8787
```

Run/build helpers:

```powershell
D:\projects\word-mobile-rn\scripts\flutter-run-word-admin-local.cmd
D:\projects\word-mobile-rn\scripts\flutter-build-release-word-admin-local.cmd
```

## Supabase auth local run

Current auth screen expects `dart-define` values:

```powershell
flutter run `
  --dart-define=SUPABASE_URL=https://<project>.supabase.co `
  --dart-define=SUPABASE_ANON_KEY=<anon-key>
```

Without these values, the Account screen will stay in a safe "not configured"
state instead of pretending auth is available.

For release builds, use the deploy script or the repo-root helper so those
values are compiled into the APK:

```powershell
D:\projects\word-mobile-rn\apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1
```

or:

```powershell
D:\projects\word-mobile-rn\scripts\flutter-build-release-supabase-cloud.cmd
```

Both read `D:\projects\word-mobile-rn\.env.supabase.local`, which is ignored by
git.
