# Flutter Android Release Wireless Deploy Troubleshooting

## `adb devices` is empty

Pairing is not enough. Connect the device first:

```powershell
D:\Android\Sdk\platform-tools\adb.exe connect <phone-ip:debug-port>
D:\Android\Sdk\platform-tools\adb.exe devices
```

## `flutter build apk --release` is slow or hangs

Check whether Flutter SDK or Gradle initialization is still running.

You can rerun from:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
D:\flutter\flutter\bin\flutter.bat build apk --release
```

Also verify these local prerequisites first:

- `C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot\bin\jlink.exe`
- `D:\Android\Sdk\cmake\3.22.1\bin\cmake.exe`
- `D:\Android\Sdk\build-tools\35.0.0\aapt2.exe`

If any of them are missing, the build may fail before APK packaging completes.

## APK not found

Expected output path:

```text
apps/flutter_mobile/build/app/outputs/flutter-apk/app-release.apk
```

If it does not exist, treat that as a build failure rather than an install problem.

If the APK exists but the app still says platform bridge is unavailable, inspect
whether the APK actually contains:

```text
lib/arm64-v8a/libword_platform_mobile.so
```

## APK builds but Today only shows root/affix tasks

This usually means the Flutter APK contains `seed-vocab` assets but SQLite does
not yet have ordinary vocabulary rows in `entries` and `wordbook_entries`.

Checks:

1. Inspect APK contents and verify these entries exist:

```text
assets/seed-vocab/book/CET4_3.json
assets/seed-vocab/book/CET6_3.json
assets/seed-vocab/book/KaoYan_3.json
assets/seed-vocab/book/MEDICAL_RESP.json
```

2. Run the Rust regression test:

```powershell
cargo test -p word-platform-mobile seed_vocab_import_restores_non_root_today_targets -- --nocapture
```

The bridge should lazily import bundled seed vocab before Today/startSession
uses the local word pool. If this test fails, do not patch Flutter rendering as
the only fix; repair the Rust/data layer.

## Task breakdown omits review, mixed, or wrong-word reinforcement

Flutter Today task breakdown should show the full plan-backed mode list, while
Rust start-session logic remains responsible for refusing invalid learned pools.
Do not fix one side by breaking the other.

Run the Flutter regression test:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
flutter test test\today_task_breakdown_test.dart --no-pub
```

This verifies Flutter keeps the full plan-backed task breakdown.

## Actual completed state does not sync to Today task breakdown

Do not rely on the old top-level "resume unfinished study" card. Flutter Today
should refresh after leaving Study, and Rust should include active session
results in Today completion.

Run:

```powershell
cargo test -p word-platform-mobile today_completion -- --nocapture
```

If Today shows `0/plan` after answering questions and returning home, inspect
the active session snapshot in `app_settings` and the Rust function
`load_active_session_completion_seed`. Active session progress should be counted
by mode and deduplicated against persisted `study_results`.

## Fully completing a mode shows `No active session`

The final answer submit already persists the completed session and clears the
active snapshot. The Flutter completion screen must return to Today directly
instead of calling `completeSession` a second time.

Regression checks:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
flutter test test\study_question_display_test.dart --no-pub
```

If `BridgeError(... STUDY_RULE): Failed to complete session: No active session`
appears after a mode is fully learned, inspect `StudyScreen` completion actions
before changing Rust persistence.

## Review or mixed mode uses the wrong pool

Review mode should be built from learned entries in the active wordbook.
Mixed-test mode should randomly draw from the active wordbook itself. Neither
mode should fall back to other wordbooks.

Regression checks:

```powershell
cargo test -p word-platform-mobile today_progress_tracks_partial_and_completed_study_sessions -- --nocapture
cargo test -p word-platform-mobile review_session_does_not_use_unlearned_general_word_pool -- --nocapture
cargo test -p word-platform-mobile review_uses_learned_entries_and_mixed_uses_current_wordbook_entries -- --nocapture
```

If these fail, fix the Rust bridge hydration path before changing Flutter error
handling.

Also verify that study hydration reads the saved active wordbook selection, not
a stale `today_wordbooks_json` snapshot. A stale Today selection can make the UI
say KaoYan while review/mixed still draws from Medical.

## Wrong page is empty after answering wrong

`study_results.outcome` may be stored as JSON strings such as `"incorrect"`.
Wrong-word SQL must accept both quoted and unquoted outcome values.

Unfinished sessions must also persist answered results incrementally, otherwise
Wrong and AI pages cannot see a wrong answer until final completion.

Regression check:

```powershell
cargo test -p word-platform-mobile wrong_answers_are_visible_before_session_completion -- --nocapture
```

## Same word/POS appears twice in one wordbook

Seed vocabulary can contain duplicate entries for the same word and same part of
speech, for example two `cancel` verb entries with complementary meanings. The
Flutter/Rust import layer should merge them into one entry per wordbook/POS and
preserve all meanings.

Regression check:

```powershell
cargo test -p word-platform-mobile seed_dedup_merges_same_word_and_pos_within_wordbook -- --nocapture
```

If a device already imported old seed data, startup should run the repair path;
do not rely only on a fresh import.

## Options contain stray `<`, replacement characters, or embedded labels

Choice text should be sanitized before rendering. It must remove replacement
characters, stray angle brackets, and embedded option labels such as `A;` or
`C；` that appear inside source meanings.

Regression check:

```powershell
cargo test -p word-study-core cn_choice_text_strips_embedded_option_labels_from_source_meanings -- --nocapture
```

## Task row shows values like `4/3`

Task breakdown rows should cap the displayed completed count at the displayed
target. Raw result rows can exceed a plan task unit count because a session may
have more question records than the displayed daily unit.

Regression check:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
flutter test test\today_task_breakdown_test.dart --no-pub
```

## Chinese-to-English hero pushes choices below the fold

For `cnToEnChoice`, the hero must not reveal the English answer. It should show
the Chinese prompt, and long Chinese prompts need compact hero typography so the
answer choices remain reachable without excessive scrolling.

Regression check:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile
flutter test test\study_question_display_test.dart --no-pub
```

## Completion shows more than 100 percent

Completion must be capped and deduplicated.

Checks:

- Run `cargo test -p word-platform-mobile today_completion -- --nocapture`.
- Run `flutter test test\today_task_breakdown_test.dart --no-pub`.
- Confirm Flutter completion uses the same displayed targets as the task
  breakdown and clamps the final percentage to `0..100`.

## FlutterDrive vs Flutter tests

Use Flutter's current testing stack:

- `flutter test`: unit and widget tests, good for deterministic task breakdown
  assertions.
- `flutter test integration_test`: current Flutter integration-test path for
  real device app flows.
- `flutter drive`: older driver workflow; only use if this project already has
  `test_driver/`.

Use adb screenshots only as a final real-device visual check. Screenshots prove
what is visible on the device, but they are not a stable substitute for
Flutter/widget/integration tests.

## Native Rust bridge changes do not appear on device

If `crates/platform-mobile` changed, `flutter build apk` alone may package an
old copied `.so`.

Always run:

```powershell
Set-Location D:\projects\word-mobile-rn\apps\flutter_mobile\android
.\gradlew.bat :app:buildRustReleaseArm64 :app:copyRustReleaseArm64 --console=plain
```

Then rebuild the APK and inspect `lib/arm64-v8a/libword_platform_mobile.so`.

## Duplicate native library packaging error

Do not keep a manually copied library under:

```text
apps/flutter_mobile/android/app/src/main/jniLibs/arm64-v8a/libword_platform_mobile.so
```

The Flutter Gradle task should package the generated copy from:

```text
apps/flutter_mobile/build/app/generated/rust/jniLibs/arm64-v8a/libword_platform_mobile.so
```

If both locations contain the same `.so`, Gradle may fail with duplicate JNI
library errors.

## `adb install -r` fails with user confirmation issues

Re-run the same install command and approve installation on the device:

```powershell
D:\Android\Sdk\platform-tools\adb.exe install -r "D:\projects\word-mobile-rn\apps\flutter_mobile\build\app\outputs\flutter-apk\app-release.apk"
```

If the phone shows an install confirmation dialog, approve it there. Otherwise
adb may return:

- `INSTALL_FAILED_ABORTED: User rejected permissions`

## `adb install -r` fails with signature mismatch

If you see:

- `INSTALL_FAILED_UPDATE_INCOMPATIBLE`

then the APK was not signed with the same key as the installed `com.wordmobile`
package.

Current expected signing behavior:

- Flutter release reuses `apps/mobile/android/app/debug.keystore`

If that signing link is broken, upgrade-style installation will stop preserving
the existing data container.

## Local data continuity concern

Changing display name usually does not affect local app data.

Changing Android `applicationId` or iOS `bundle identifier` does affect the app
data container and may prevent the new app from reading the old local database.
