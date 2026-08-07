[CmdletBinding()]
param(
    [string]$WriteEvidence,
    [switch]$FinalArtifact,
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent

function Assert-OrdinalArray([string[]]$Expected, [string[]]$Actual, [string]$Label) {
    if ($Expected.Count -ne $Actual.Count) { throw "$Label count mismatch." }
    for ($index = 0; $index -lt $Expected.Count; $index++) {
        if (-not [StringComparer]::Ordinal.Equals($Expected[$index], $Actual[$index])) { throw "$Label mismatch." }
    }
}

function Assert-BrowserMeasurements([object[]]$Measurements, [object]$Manifest) {
    [string[]]$expectedBrowsers = @('chromium', 'firefox', 'webkit')
    [string[]]$actualBrowsers = @($Measurements | ForEach-Object { [string]$_.browser })
    [Array]::Sort($expectedBrowsers, [StringComparer]::Ordinal)
    [Array]::Sort($actualBrowsers, [StringComparer]::Ordinal)
    Assert-OrdinalArray $expectedBrowsers $actualBrowsers 'Browser measurement set'

    [string[]]$expectedIds = @($Manifest.fixtures.fixtureInventory | ForEach-Object { [string]$_.id })
    foreach ($measurement in $Measurements) {
        [string[]]$actualIds = @($measurement.fixtureIds | ForEach-Object { [string]$_ })
        Assert-OrdinalArray $expectedIds $actualIds "$($measurement.browser) fixture IDs"
        if ([int]$measurement.fixtureCount -ne $expectedIds.Count) { throw "$($measurement.browser) fixture count mismatch." }
        if (-not [StringComparer]::Ordinal.Equals([string]$measurement.fixtureInventorySha256, [string]$Manifest.fixtures.fixtureInventorySha256)) {
            throw "$($measurement.browser) fixture inventory digest mismatch."
        }
    }
}

function Invoke-BrowserGateSelfTest {
    $manifest = [pscustomobject]@{ fixtures = [pscustomobject]@{
        fixtureInventory = @([pscustomobject]@{ id = 'one' }, [pscustomobject]@{ id = 'phase6-two' })
        fixtureInventorySha256 = 'fixture-digest'
    } }
    $valid = @('chromium', 'firefox', 'webkit') | ForEach-Object { [pscustomobject]@{
        browser = $_
        fixtureIds = @('one', 'phase6-two')
        fixtureCount = 2
        fixtureInventorySha256 = 'fixture-digest'
    } }
    Assert-BrowserMeasurements $valid $manifest

    $mutations = @(
        @($valid | Select-Object -First 2),
        @($valid[0], $valid[0], $valid[2]),
        @($valid | ForEach-Object { $_.PSObject.Copy() }),
        @($valid | ForEach-Object { $_.PSObject.Copy() })
    )
    $mutations[2][0].fixtureIds = @('one')
    $mutations[2][0].fixtureCount = 1
    $mutations[3][0].fixtureInventorySha256 = 'mutated'
    foreach ($mutation in $mutations) {
        $rejected = $false
        try { Assert-BrowserMeasurements $mutation $manifest } catch { $rejected = $true }
        if (-not $rejected) { throw 'Browser-gate self-test expected identity rejection.' }
    }
    Write-Output 'Browser-gate identity self-test passed.'
}

if ($SelfTest) {
    Invoke-BrowserGateSelfTest
    exit 0
}

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
$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
Assert-BrowserMeasurements $measurements $manifest

if ($WriteEvidence) {
    $sourceLock = Get-Content (Join-Path $repositoryRoot 'fixtures\domain\v1\source-lock.json') -Raw | ConvertFrom-Json
    $ledgerHash = (Get-FileHash (Join-Path $repositoryRoot 'docs\features\learning.md') -Algorithm SHA256).Hash.ToLowerInvariant()
    $fixtureCount = [int]$manifest.fixtures.fixtureCount
    $fixtureDigest = [string]$manifest.fixtures.fixtureInventorySha256
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
            fixtureIds = @($manifest.fixtures.fixtureInventory | ForEach-Object { $_.id })
            fixtureCount = $fixtureCount
            fixtureInventorySha256 = $fixtureDigest
        }
        browsers = @($measurements)
        checks = @(
            [ordered]@{ name = 'canonical-generated-wasm-fixtures'; status = 'passed'; detail = "$fixtureCount artifact-bound canonical fixtures ($fixtureDigest) and a structured error executed through the generated package; native parity is verified independently by the Rust workspace gate." }
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
