[CmdletBinding()]
param(
    [string]$WriteEvidence,
    [switch]$FinalArtifact
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$artifactRoot = if ($FinalArtifact) { 'artifacts/domain-wasm' } else { 'target/domain-wasm-candidate' }
$manifestPath = Join-Path $repositoryRoot "$artifactRoot\manifest.json"
if (-not (Test-Path -LiteralPath $manifestPath)) { throw "Artifact manifest is missing: $manifestPath" }

$env:DOMAIN_WASM_ARTIFACT_ROOT = $artifactRoot
$resultsDirectory = Join-Path $repositoryRoot 'tests\domain-browser\test-results'
if (Test-Path -LiteralPath $resultsDirectory) { Remove-Item -LiteralPath $resultsDirectory -Recurse -Force }
[IO.Directory]::CreateDirectory($resultsDirectory) | Out-Null

$measurements = @()
foreach ($project in @('chromium', 'firefox', 'webkit')) {
    & npm.cmd --prefix (Join-Path $repositoryRoot 'tests\domain-browser') run test:domain-wasm -- --project=$project --reporter=line
    if ($LASTEXITCODE -ne 0) { throw "Playwright $project gate failed." }
    $measurementPath = Join-Path $resultsDirectory "measurements-$project.json"
    if (-not (Test-Path -LiteralPath $measurementPath)) { throw "Missing $project measurement output." }
    $measurements += Get-Content $measurementPath -Raw | ConvertFrom-Json
}

if ($WriteEvidence) {
    $sourceLock = Get-Content (Join-Path $repositoryRoot 'fixtures\domain\v1\source-lock.json') -Raw | ConvertFrom-Json
    $ledgerHash = (Get-FileHash (Join-Path $repositoryRoot 'docs\features\learning.md') -Algorithm SHA256).Hash.ToLowerInvariant()
    $manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
    $evidence = [ordered]@{
        schemaVersion = 1
        wave = 4
        generatedAtUtc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
        success = $true
        sourceLockDigest = [string]$sourceLock.aggregateSha256
        learningLedgerSha256 = $ledgerHash
        artifact = [ordered]@{
            commit = [string]$manifest.source.commit
            dirty = [bool]$manifest.source.dirty
            wasmSha256 = [string]$manifest.artifacts.wasm.sha256
            javascriptSha256 = [string]$manifest.artifacts.javascript.sha256
        }
        browsers = @($measurements)
        checks = @(
            [ordered]@{ name = 'canonical-generated-wasm-fixtures'; status = 'passed'; detail = 'Seven canonical result fixtures and a structured error executed through the generated package; native parity is verified independently by the Rust workspace gate.' }
            [ordered]@{ name = 'playwright-chromium'; status = 'passed'; detail = 'Generated web package imported and executed in Chromium.' }
            [ordered]@{ name = 'playwright-firefox'; status = 'passed'; detail = 'Generated web package imported and executed in Firefox.' }
            [ordered]@{ name = 'playwright-webkit'; status = 'passed'; detail = 'Generated web package imported and executed in Playwright WebKit.' }
        )
    }
    $absoluteEvidence = Join-Path $repositoryRoot $WriteEvidence
    [IO.Directory]::CreateDirectory((Split-Path $absoluteEvidence -Parent)) | Out-Null
    [IO.File]::WriteAllText($absoluteEvidence, "$(ConvertTo-Json $evidence -Depth 20)`n", (New-Object Text.UTF8Encoding($false)))
}

Write-Output 'Chromium, Firefox, and Playwright WebKit artifact gates passed.'
