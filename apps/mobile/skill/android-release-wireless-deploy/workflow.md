# Android Release Wireless Deploy

## Purpose

Use this workflow when a React Native mobile project has already built
successfully before, the user wants a faster rebuild than starting from a fresh
Android directory, and the APK should be installed through wireless adb.

This workflow assumes:
- mobile project root: `D:\projects\word-mobile-rn\apps\mobile`
- source Android directory: `apps/mobile/android`
- cached build directory: `apps/mobile/android_build3`
- shared Gradle cache: `apps/mobile/android_build2\.gradle-home`
- JDK: `D:\pycharm\PyCharm 2025.2.3\jbr`
- apk package name: `com.wordmobile`

## Main Flow

1. Verify the phone is connected:

```powershell
D:\Android\Sdk\platform-tools\adb.exe devices
```

If the list is empty, reconnect wireless debugging before continuing.

2. Stop leftover Java processes from older builds:

```powershell
Get-Process | Where-Object {
  $_.ProcessName -eq 'java' -and $_.Path -like 'D:\projects\word-mobile-rn\apps\mobile\*'
} | Stop-Process -Force
```

3. Sync the latest Android source into the cached build directory:

```powershell
robocopy D:\projects\word-mobile-rn\apps\mobile\android `
  D:\projects\word-mobile-rn\apps\mobile\android_build3 `
  /E /XD build .gradle-home
```

4. Rebuild release from the cached directory:

```powershell
$env:JAVA_HOME='D:\pycharm\PyCharm 2025.2.3\jbr'
$env:GRADLE_USER_HOME='D:\projects\word-mobile-rn\apps\mobile\android_build2\.gradle-home'
Set-Location 'D:\projects\word-mobile-rn\apps\mobile\android_build3'
cmd /c gradlew.bat assembleRelease
```

5. Install and launch:

```powershell
D:\Android\Sdk\platform-tools\adb.exe install -r "D:\projects\word-mobile-rn\apps\mobile\android_build3\app\build\outputs\apk\release\app-release.apk"
D:\Android\Sdk\platform-tools\adb.exe shell monkey -p com.wordmobile -c android.intent.category.LAUNCHER 1
```

## Why Reuse `android_build3`

- It already has proven-good Gradle intermediates.
- It avoids a full cold rebuild.
- It avoids repeating the slowest dependency setup work.
- It is usually faster than creating `android_build4`, `android_build5`, etc.

## Success Checklist

- `assembleRelease` finishes successfully.
- APK exists at:
  `D:\projects\word-mobile-rn\apps\mobile\android_build3\app\build\outputs\apk\release\app-release.apk`
- `adb install -r` returns `Success`.
- Launch command is sent without adb errors.
