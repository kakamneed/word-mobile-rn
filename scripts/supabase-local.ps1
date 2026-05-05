$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$supabase = Join-Path $repoRoot ".tools\supabase-cli\node_modules\.bin\supabase.cmd"

if (-not (Test-Path $supabase)) {
    throw "Supabase CLI not found at $supabase. Install it with: npm.cmd install --prefix $repoRoot\.tools\supabase-cli --cache $repoRoot\.npm-cache supabase@2.95.6"
}

& $supabase @args
