param(
  [string]$FlutterRoot = "D:\flutter\flutter",
  [string]$JavaHome = "C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot",
  [string]$AndroidSdkRoot = "D:\Android\Sdk",
  [string]$AdbPath = "D:\Android\Sdk\platform-tools\adb.exe",
  [string]$PackageName = "com.wordmobile",
  [string]$DeviceSerial = $env:ANDROID_DEVICE_SERIAL
)

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent (Split-Path -Parent $projectRoot)
$apkPath = Join-Path $projectRoot "build\app\outputs\flutter-apk\app-release.apk"
$supabaseEnvPath = Join-Path $repoRoot ".env.supabase.local"
$env:GRADLE_USER_HOME = "D:\projects\word-mobile-rn\apps\mobile\.gradle-home"
$env:GRADLE_OPTS = "-Xmx1536m -XX:MaxMetaspaceSize=512m -XX:ReservedCodeCacheSize=128m -Dfile.encoding=UTF-8 -Dorg.gradle.daemon=false -Dorg.gradle.workers.max=1"

$env:JAVA_HOME = $JavaHome
$env:ANDROID_HOME = $AndroidSdkRoot
$env:ANDROID_SDK_ROOT = $AndroidSdkRoot
$env:PATH = @(
  (Join-Path $JavaHome 'bin')
  (Join-Path $FlutterRoot 'bin')
  (Join-Path $FlutterRoot 'bin\cache\dart-sdk\bin')
  (Join-Path $AndroidSdkRoot 'platform-tools')
  (Join-Path $AndroidSdkRoot 'cmdline-tools\latest\bin')
  (Join-Path $AndroidSdkRoot 'build-tools\35.0.0')
  'D:\msys64\home\clf20\.cargo\bin'
  'C:\Windows\System32'
  'C:\Windows'
  'C:\Windows\System32\WindowsPowerShell\v1.0'
  $env:PATH
) -join ';'

if (-not (Test-Path (Join-Path $JavaHome 'bin\jlink.exe'))) {
  throw "Expected full JDK with jlink at $JavaHome"
}

if (-not (Test-Path (Join-Path $AndroidSdkRoot 'cmake\3.22.1\bin\cmake.exe'))) {
  throw "Expected Android SDK CMake 3.22.1 at $AndroidSdkRoot\cmake\3.22.1"
}

if (-not (Test-Path (Join-Path $AndroidSdkRoot 'build-tools\35.0.0\aapt2.exe'))) {
  throw "Expected Android Build-Tools 35.0.0 at $AndroidSdkRoot\build-tools\35.0.0"
}

$dartDefines = @()
if (Test-Path $supabaseEnvPath) {
  Get-Content -LiteralPath $supabaseEnvPath | ForEach-Object {
    $line = $_.Trim()
    if ($line.Length -eq 0 -or $line.StartsWith("#")) {
      return
    }
    $parts = $line.Split("=", 2)
    if ($parts.Count -eq 2 -and $parts[0] -in @("SUPABASE_URL", "SUPABASE_ANON_KEY")) {
      $dartDefines += "--dart-define=$($parts[0])=$($parts[1])"
    }
  }
}

if ($dartDefines.Count -lt 2) {
  Write-Warning "Supabase dart-defines were not found in $supabaseEnvPath. Account login will show not configured."
} else {
  Write-Host "[env] Supabase dart-defines loaded from $supabaseEnvPath"
}

Write-Host "[1/6] flutter pub get"
Set-Location $projectRoot
$previousApkHash = $null
$previousApkWriteTimeUtc = $null
if (Test-Path $apkPath) {
  $previousApkHash = (Get-FileHash -LiteralPath $apkPath -Algorithm SHA256).Hash
  $previousApkWriteTimeUtc = (Get-Item -LiteralPath $apkPath).LastWriteTimeUtc
}

& "$FlutterRoot\bin\flutter.bat" pub get
if ($LASTEXITCODE -ne 0) {
  throw "flutter pub get failed with exit code $LASTEXITCODE"
}

Write-Host "[2/6] flutter build apk --release"
& "$FlutterRoot\bin\flutter.bat" build apk --release @dartDefines
if ($LASTEXITCODE -ne 0) {
  throw "flutter build apk --release failed with exit code $LASTEXITCODE"
}

if (-not (Test-Path $apkPath)) {
  throw "Release APK not found at $apkPath"
}

$newApkHash = (Get-FileHash -LiteralPath $apkPath -Algorithm SHA256).Hash
$newApkWriteTimeUtc = (Get-Item -LiteralPath $apkPath).LastWriteTimeUtc
if ($previousApkHash -and $newApkHash -eq $previousApkHash) {
  throw "Release APK was not regenerated; refusing to install or send the previous APK"
}
if ($previousApkWriteTimeUtc -and $newApkWriteTimeUtc -le $previousApkWriteTimeUtc) {
  throw "Release APK timestamp was not refreshed; refusing to install or send the previous APK"
}
Write-Host "[verify] New APK generated: SHA256 $newApkHash"

Write-Host "[3/6] verify packaged Rust bridge"
$apkListing = tar -tf $apkPath
if (-not (($apkListing | Out-String) -match 'lib/arm64-v8a/libword_platform_mobile\.so')) {
  throw "Rust JNI library libword_platform_mobile.so is missing from APK"
}

Write-Host "[4/6] adb devices"
$connectedDevices = @(
  (& $AdbPath devices) |
    Select-Object -Skip 1 |
    Where-Object { $_ -match "^(\S+)\s+device\s*$" } |
    ForEach-Object { $matches[1] }
)
if ($DeviceSerial) {
  if ($connectedDevices -notcontains $DeviceSerial) {
    throw "Requested adb device '$DeviceSerial' is not connected"
  }
} elseif ($connectedDevices.Count -gt 1) {
  throw "Multiple adb devices are connected. Set ANDROID_DEVICE_SERIAL before retrying: $($connectedDevices -join ', ')"
}
$adbArgs = @()
if ($DeviceSerial) {
  $adbArgs = @("-s", $DeviceSerial)
}
& $AdbPath @adbArgs devices

Write-Host "[5/6] adb install -r"
& $AdbPath @adbArgs install -r $apkPath
if ($LASTEXITCODE -ne 0) {
  throw "adb install failed with exit code $LASTEXITCODE"
}

Write-Host "[6/6] launch app"
& $AdbPath @adbArgs shell monkey -p $PackageName -c android.intent.category.LAUNCHER 1
if ($LASTEXITCODE -ne 0) {
  throw "app launch failed with exit code $LASTEXITCODE"
}

Write-Host "Done. APK: $apkPath"
