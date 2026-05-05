# Flutter Android Release Wireless Deploy

## Purpose

Use this workflow when the Flutter mobile shell should be built as a release APK
and installed to a wireless-debugging Android device.

This workflow assumes:

- Flutter project root: `D:\projects\word-mobile-rn\apps\flutter_mobile`
- Flutter SDK: `D:\flutter\flutter`
- Full JDK with `jlink.exe`: `C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot`
- adb: `D:\Android\Sdk\platform-tools\adb.exe`
- package name: `com.wordmobile`
- Android SDK CMake `3.22.1` is installed locally
- Android Build-Tools `35.0.0` is installed locally
- repo-local Gradle zip is available at `D:\projects\word-mobile-rn\.gradle-home\gradle-8.13-bin.zip`

## Main Flow

Always run the flow in this order:

1. Test the changed behavior.
2. Rebuild the Android Rust release `.so` when Rust bridge/native behavior changed.
3. Build the Flutter release APK.
4. Inspect the APK contents before install.
5. Install and launch the APK.
6. Verify the real device UI or logs.

Do not skip APK inspection. A Flutter build can succeed while still packaging an
old native library or missing bundled vocabulary assets.

### 1. Verify Device

```powershell
D:\Android\Sdk\platform-tools\adb.exe devices
```

### 2. Test Before Packaging

Run focused Flutter tests for UI/data-shaping changes:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
flutter test test\today_task_breakdown_test.dart --no-pub
flutter test test\study_question_display_test.dart --no-pub
flutter analyze --no-pub
```

Run focused Rust checks/tests for bridge, Today, study, seed vocabulary, or
root/affix changes:

```powershell
Set-Location D:\projects\word-mobile-rn
cargo check -p word-platform-mobile
cargo test -p word-platform-mobile seed_vocab_import_restores_non_root_today_targets -- --nocapture
cargo test -p word-platform-mobile today_completion -- --nocapture
cargo test -p word-platform-mobile today_progress_tracks_partial_and_completed_study_sessions -- --nocapture
cargo test -p word-platform-mobile review_session_does_not_use_unlearned_general_word_pool -- --nocapture
cargo test -p word-platform-mobile review_uses_learned_entries_and_mixed_uses_current_wordbook_entries -- --nocapture
cargo test -p word-platform-mobile wrong_answers_are_visible_before_session_completion -- --nocapture
cargo test -p word-platform-mobile seed_dedup_merges_same_word_and_pos_within_wordbook -- --nocapture
cargo test -p word-platform-mobile root_affix -- --nocapture
```

For UI-only changes, Flutter tests plus `flutter analyze --no-pub` are the
minimum. For Rust bridge/native changes, Rust tests/checks are mandatory too.

### 3. Rebuild Android Rust Native Library When Needed

If any file under `crates/platform-mobile` or native bridge behavior changed,
rebuild and copy the Android arm64 release library before building the APK:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile\android
$env:JAVA_HOME='C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot'
$env:ANDROID_HOME='D:\Android\Sdk'
$env:ANDROID_SDK_ROOT='D:\Android\Sdk'
$env:GRADLE_USER_HOME='D:\projects\word-mobile-rn\apps\mobile\android_build2\.gradle-home'
$env:ANDROID_USER_HOME='D:\projects\word-mobile-rn\.android-home'
.\gradlew.bat :app:buildRustReleaseArm64 :app:copyRustReleaseArm64 --console=plain
```

Expected result:

```text
BUILD SUCCESSFUL
Finished `release` profile
```

### 4. Build Flutter Release APK

Preferred path: use the repo deploy script so Supabase compile-time config,
release environment, APK inspection, install, and launch stay together:

```powershell
Set-Location D:\projects\word-mobile-rn
powershell.exe -NoProfile -ExecutionPolicy Bypass -File apps\flutter_mobile\scripts\android-release-wireless-deploy.ps1
```

Expected Supabase env output:

```text
[env] Supabase dart-defines loaded from D:\projects\word-mobile-rn\.env.supabase.local
```

If the script warns that Supabase dart-defines were not found, do not use the APK
for account/auth/cloud validation. Fix `D:\projects\word-mobile-rn\.env.supabase.local`
first.

Manual fallback, only when the deploy script cannot be used:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
$env:JAVA_HOME='C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot'
$env:ANDROID_HOME='D:\Android\Sdk'
$env:ANDROID_SDK_ROOT='D:\Android\Sdk'
$env:GRADLE_USER_HOME='D:\projects\word-mobile-rn\apps\mobile\android_build2\.gradle-home'
$env:ANDROID_USER_HOME='D:\projects\word-mobile-rn\.android-home'
$env:SUPABASE_URL='<load from D:\projects\word-mobile-rn\.env.supabase.local>'
$env:SUPABASE_ANON_KEY='<load from D:\projects\word-mobile-rn\.env.supabase.local>'
flutter build apk --release --no-pub `
  --dart-define=SUPABASE_URL=$env:SUPABASE_URL `
  --dart-define=SUPABASE_ANON_KEY=$env:SUPABASE_ANON_KEY
```

The repo script `apps/flutter_mobile/scripts/android-release-wireless-deploy.ps1`
loads these values from `D:\projects\word-mobile-rn\.env.supabase.local` when it
exists. If the values are missing, the Account drawer will show Supabase as not
configured in the release APK.

Never use bare `flutter build apk --release` for releases that need account,
auth, cloud sync, or Supabase validation.

Expected APK path:

```text
D:\projects\word-mobile-rn\apps\flutter_mobile\build\app\outputs\flutter-apk\app-release.apk
```

### 5. Inspect APK Contents

Before install, verify the APK contains both the native Rust library and bundled
resources:

```powershell
Add-Type -AssemblyName System.IO.Compression.FileSystem
$apk='D:\projects\word-mobile-rn\apps\flutter_mobile\build\app\outputs\flutter-apk\app-release.apk'
[IO.Compression.ZipFile]::OpenRead($apk).Entries |
  Where-Object { $_.FullName -match 'seed-vocab|seed-medical|libword_platform_mobile\.so' } |
  Select-Object -First 40 FullName,Length
```

Required entries:

```text
lib/arm64-v8a/libword_platform_mobile.so
assets/seed-vocab/bookLists.json
assets/seed-vocab/book/CET4_3.json
assets/seed-vocab/book/CET6_3.json
assets/seed-vocab/book/KaoYan_3.json
assets/seed-vocab/book/MEDICAL_RESP.json
assets/seed-medical/medical-root-affix.txt
```

If any required entry is missing, do not install the build.

### 6. Install And Launch

```powershell
D:\Android\Sdk\platform-tools\adb.exe install -r "D:\projects\word-mobile-rn\apps\flutter_mobile\build\app\outputs\flutter-apk\app-release.apk"
D:\Android\Sdk\platform-tools\adb.exe shell monkey -p com.wordmobile -c android.intent.category.LAUNCHER 1
```

Expected install output:

```text
Success
```

Expected launch output:

```text
Events injected: 1
```

If install returns `INSTALL_FAILED_ABORTED: User rejected permissions`, resend
the same install command and ask the user to approve the phone-side install
dialog.

If streamed install keeps getting rejected, push the APK and install from the
device shell:

```powershell
D:\Android\Sdk\platform-tools\adb.exe push "D:\projects\word-mobile-rn\apps\flutter_mobile\build\app\outputs\flutter-apk\app-release.apk" /data/local/tmp/word_flutter_release.apk
D:\Android\Sdk\platform-tools\adb.exe shell pm install -r /data/local/tmp/word_flutter_release.apk
```

If `pm install` also returns `INSTALL_FAILED_ABORTED: User rejected permissions`,
the build is ready but the phone-side permission dialog or device policy must be
approved before installation can complete.

### 7. Device UI Verification

For real UI verification, capture a device screenshot after launch:

```powershell
D:\Android\Sdk\platform-tools\adb.exe shell screencap -p /sdcard/flutter_today_screen.png
D:\Android\Sdk\platform-tools\adb.exe pull /sdcard/flutter_today_screen.png D:\projects\word-mobile-rn\flutter_today_screen.png
```

Then inspect the pulled image. This is an external, device-level verification,
not a replacement for Flutter tests.

For Today task breakdown specifically, expected behavior after the latest fixes:

- No top "发现未完成学习" resume card on Today.
- Task breakdown can show new words, review, mixed test, wrong-word
  reinforcement, and root/affix when the active plan has those tasks.
- Flutter Today task breakdown displays plan-backed modes so users can see the
  full daily plan.
- Completion percentage is capped at `100%` and uses the same displayed targets
  as the task breakdown.
- Completing a mode returns to Today without a `No active session` Study error.
- Review sessions must use learned entries from the active wordbook. Mixed-test
  sessions must randomly draw from the active wordbook, not from a learned-only
  pool or another book.
- Saved active wordbook selection is the source of truth for study-session
  hydration; stale Today wordbook state must not make review/mixed draw from
  another book.
- Wrong answers must enter the wrong-word pool even before the session is fully
  completed, so Wrong and AI passage pages can see them after returning home.
- Seed vocabulary import/repair must merge same-word same-POS duplicates within
  the same wordbook while preserving all meanings.
- Today progress must update after both unfinished returns and fully completed
  modes.
- Today task rows must cap displayed completion at the displayed target, so
  surplus question records never render as values like `4/3`.
- Chinese-to-English questions show the Chinese prompt in the top card, never
  the English answer, and long prompts use compact typography.

## Data continuity note

This Flutter app is being aligned to the legacy app identity:

- Android `applicationId`: `com.wordmobile`
- iOS main bundle id: `com.wordmobile`

That alignment is a prerequisite for upgrade-style installation that preserves
local app data.

Current signing rule:

- Flutter release reuses the legacy RN debug keystore under `apps/mobile/android/app/debug.keystore`

That is what allows `adb install -r` to upgrade the existing `com.wordmobile`
package instead of failing with a signature mismatch.

## Do not use

Do not use the old React Native cached-build workflow under:

```text
skill/legacy-react-native-android-release-wireless-deploy/
```

except as historical reference.
