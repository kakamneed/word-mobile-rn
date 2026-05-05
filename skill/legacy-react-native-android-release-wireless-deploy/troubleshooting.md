# Android Release Wireless Deploy Troubleshooting

> Deprecated for current delivery path.
> Keep this only as legacy React Native reference. Do not use it as the
> default install chain for the Flutter mobile app.

## `adb pair` succeeded but `adb devices` is empty

Pairing only creates trust. It does not connect the device automatically.

Do this next:

```powershell
D:\Android\Sdk\platform-tools\adb.exe connect <phone-ip:debug-port>
D:\Android\Sdk\platform-tools\adb.exe devices
```

Use the phone's wireless debugging page for the `connect` address, not the
pairing port.

## `INSTALL_FAILED_ABORTED: User rejected permissions`

This is not a build failure.

It means the phone rejected the install prompt. Re-run:

```powershell
D:\Android\Sdk\platform-tools\adb.exe install -r "D:\projects\word-mobile-rn\apps\mobile\android_build3\app\build\outputs\apk\release\app-release.apk"
```

Then allow installation on the phone.

## `Unable to load script`

This usually means a debug package was launched without Metro.

For device delivery, prefer:
- `assembleRelease`
- `app-release.apk`

Do not debug this as a release issue unless the release package shows the same
behavior.

## Build hangs for a long time

Check for stale Java processes:

```powershell
Get-Process | Where-Object { $_.ProcessName -eq 'java' }
```

If old mobile-project Java processes are still around, stop them and rerun the
cached-build workflow.

## `mergeJavaRes` or zip-cache file lock errors

This usually means an older build process still has a handle open.

Preferred recovery:
1. Stop stale Java processes.
2. Reuse `android_build3` again.
3. Only create a fresh build directory if the cached one stays corrupted.

## Gradle redownload or cold-start behavior

If a new build directory starts redownloading lots of dependencies, you are not
getting the speed advantage anymore. Prefer reusing:

- `android_build3`
- `android_build2\.gradle-home`

instead of starting from a fresh directory.
