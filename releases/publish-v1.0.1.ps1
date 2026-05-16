$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$apk = Join-Path $PSScriptRoot 'word-mobile-1.0.1+2.apk'
$notes = Join-Path $PSScriptRoot 'v1.0.1-release-notes.md'
$tag = 'v1.0.1'
$title = 'Word Mobile 1.0.1'
$gh = Get-Command gh -ErrorAction SilentlyContinue
if (-not $gh -and (Test-Path 'C:\Program Files\GitHub CLI\gh.exe')) {
  $gh = Get-Item 'C:\Program Files\GitHub CLI\gh.exe'
}

if (-not $gh) {
  throw 'GitHub CLI (gh) is required. Install gh and run gh auth login first.'
}

if (-not (Test-Path -LiteralPath $apk)) {
  throw "APK not found: $apk"
}

Set-Location $repoRoot
if (-not $env:HTTP_PROXY) { $env:HTTP_PROXY = 'http://127.0.0.1:7897' }
if (-not $env:HTTPS_PROXY) { $env:HTTPS_PROXY = 'http://127.0.0.1:7897' }

& $gh.Source release create $tag "$apk#word-mobile-1.0.1+2.apk" `
  --repo kakamneed/word-mobile-rn `
  --target main `
  --title $title `
  --notes-file $notes
