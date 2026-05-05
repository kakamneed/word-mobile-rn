param(
  [string]$FlutterRoot = "D:\flutter\flutter",
  [string]$JavaHome = "C:\Program Files\Microsoft\jdk-21.0.10.7-hotspot",
  [string]$AndroidSdkRoot = "D:\Android\Sdk",
  [string]$AdbPath = "D:\Android\Sdk\platform-tools\adb.exe",
  [string]$PackageName = "com.wordmobile"
)

$ErrorActionPreference = "Stop"

$projectRoot = Split-Path -Parent $PSScriptRoot
$repoRoot = Split-Path -Parent (Split-Path -Parent $projectRoot)
$apkPath = Join-Path $projectRoot "build\app\outputs\flutter-apk\app-release.apk"
$supabaseEnvPath = Join-Path $repoRoot ".env.supabase.local"
$env:GRADLE_USER_HOME = "D:\projects\word-mobile-rn\apps\mobile\.gradle-home"

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
& "$FlutterRoot\bin\flutter.bat" pub get

Write-Host "[2/6] flutter build apk --release"
& "$FlutterRoot\bin\flutter.bat" build apk --release @dartDefines

if (-not (Test-Path $apkPath)) {
  throw "Release APK not found at $apkPath"
}

Write-Host "[3/6] verify packaged Rust bridge"
$apkListing = tar -tf $apkPath
if (-not (($apkListing | Out-String) -match 'lib/arm64-v8a/libword_platform_mobile\.so')) {
  throw "Rust JNI library libword_platform_mobile.so is missing from APK"
}

Write-Host "[4/6] adb devices"
& $AdbPath devices

Write-Host "[5/6] adb install -r"
& $AdbPath install -r $apkPath

Write-Host "[6/6] launch app"
& $AdbPath shell monkey -p $PackageName -c android.intent.category.LAUNCHER 1

Write-Host "Done. APK: $apkPath"
