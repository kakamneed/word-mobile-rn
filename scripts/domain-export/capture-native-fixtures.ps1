[CmdletBinding(DefaultParameterSetName = 'Verify')]
param(
    [Parameter(ParameterSetName = 'Verify', Mandatory = $true)]
    [switch]$Verify,

    [Parameter(ParameterSetName = 'Accept', Mandatory = $true)]
    [switch]$AcceptCurrentMobileTruth,

    [Parameter(ParameterSetName = 'Verify')]
    [string]$WriteEvidence
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$fixtureRoot = Join-Path $repositoryRoot 'fixtures\domain\v1'
$manifestPath = Join-Path $fixtureRoot 'manifest.json'
$sourceLockPath = Join-Path $fixtureRoot 'source-lock.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$sourceLock = Get-Content -LiteralPath $sourceLockPath -Raw | ConvertFrom-Json
$sourceLockScript = Join-Path $PSScriptRoot 'check-source-lock.ps1'

if ([string]::IsNullOrWhiteSpace($WriteEvidence)) {
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $sourceLockScript -Check -Wave ([int]$sourceLock.acceptedWave)
    if ($LASTEXITCODE -ne 0) {
        throw 'Source lock check failed; native fixtures cannot be captured or verified.'
    }
}

if ([string]$manifest.sourceLockDigest -ne [string]$sourceLock.aggregateSha256) {
    throw 'Fixture manifest sourceLockDigest does not match the accepted source lock.'
}

$fixtureIds = @($manifest.fixtures | ForEach-Object { [string]$_.id })
if ($fixtureIds.Count -lt 7) {
    throw 'Canonical fixture manifest does not name every required behavior group.'
}

Push-Location $repositoryRoot
try {
    if ($AcceptCurrentMobileTruth) {
        $previousAccept = $env:DOMAIN_FIXTURE_ACCEPT
        try {
            $env:DOMAIN_FIXTURE_ACCEPT = '1'
            & cargo test -p word-app-core --test baseline_runner canonical_domain_fixture_corpus_matches_current_native_behavior -- --test-threads=1
            if ($LASTEXITCODE -ne 0) {
                throw 'Native fixture capture failed.'
            }
        }
        finally {
            if ($null -eq $previousAccept) {
                Remove-Item Env:DOMAIN_FIXTURE_ACCEPT -ErrorAction SilentlyContinue
            }
            else {
                $env:DOMAIN_FIXTURE_ACCEPT = $previousAccept
            }
        }
    }

    foreach ($fixture in @($manifest.fixtures)) {
        $expectedPath = Join-Path $fixtureRoot ([string]$fixture.expected)
        if (-not (Test-Path -LiteralPath $expectedPath -PathType Leaf)) {
            throw "Missing accepted output for fixture $($fixture.id). Re-run with -AcceptCurrentMobileTruth to create it explicitly."
        }
    }

    & cargo test -p word-app-core --test baseline_runner canonical_domain_fixture_corpus_matches_current_native_behavior -- --test-threads=1
    if ($LASTEXITCODE -ne 0) {
        throw 'Native fixture verification failed.'
    }
}
finally {
    Pop-Location
}

$mode = if ($AcceptCurrentMobileTruth) { 'captured and verified' } else { 'verified' }
Write-Output "Canonical native fixtures ${mode}: $($fixtureIds -join ', ')."

if (-not [string]::IsNullOrWhiteSpace($WriteEvidence)) {
    $ledgerPath = Join-Path $repositoryRoot 'docs\features\learning.md'
    $ledgerSha256 = (Get-FileHash -LiteralPath $ledgerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $evidence = [ordered]@{
        schemaVersion = 1
        wave = [int]$sourceLock.acceptedWave
        generatedAtUtc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
        success = $true
        sourceLockDigest = [string]$sourceLock.aggregateSha256
        learningLedgerSha256 = $ledgerSha256
        checks = @(
            [ordered]@{
                name = 'canonical-native-fixtures'
                status = 'passed'
                detail = "$($fixtureIds.Count) accepted fixtures matched current native behavior"
            }
        )
    }
    $evidencePath = if ([IO.Path]::IsPathRooted($WriteEvidence)) {
        $WriteEvidence
    }
    else {
        Join-Path $repositoryRoot $WriteEvidence
    }
    [IO.Directory]::CreateDirectory((Split-Path $evidencePath -Parent)) | Out-Null
    $encoding = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText(
        $evidencePath,
        "$(ConvertTo-Json -InputObject $evidence -Depth 10)`n",
        $encoding
    )
    Write-Output "Wrote native parity evidence: $WriteEvidence"
}
