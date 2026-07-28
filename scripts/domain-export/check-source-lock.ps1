[CmdletBinding(DefaultParameterSetName = 'Check')]
param(
    [Parameter(ParameterSetName = 'Capture', Mandatory = $true)]
    [switch]$Capture,

    [Parameter(ParameterSetName = 'Check', Mandatory = $true)]
    [switch]$Check,

    [Parameter(ParameterSetName = 'Promote', Mandatory = $true)]
    [switch]$Promote,

    [Parameter(ParameterSetName = 'SelfTest', Mandatory = $true)]
    [switch]$SelfTest,

    [Parameter(ParameterSetName = 'Capture', Mandatory = $true)]
    [Parameter(ParameterSetName = 'Check', Mandatory = $true)]
    [Parameter(ParameterSetName = 'Promote', Mandatory = $true)]
    [ValidateRange(0, 99)]
    [int]$Wave,

    [Parameter(ParameterSetName = 'Promote', Mandatory = $true)]
    [string]$ReviewedDiff,

    [Parameter(ParameterSetName = 'Promote', Mandatory = $true)]
    [string]$ParityEvidence
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:RepositoryRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$script:LockPath = Join-Path $script:RepositoryRoot 'fixtures\domain\v1\source-lock.json'
$script:PromotionsPath = Join-Path $script:RepositoryRoot 'fixtures\domain\v1\source-lock-promotions.json'
$script:LedgerPath = 'docs/features/learning.md'
$script:AuthoritativePaths = @(
    'crates/study-core/src/question_builder.rs'
    'crates/study-core/src/session_summary.rs'
    'crates/storage-core/src/models/study_question.rs'
    'crates/storage-core/src/models/study_requests.rs'
    'crates/storage-core/src/models/study_result.rs'
    'crates/storage-core/src/models/study_session.rs'
    'crates/app-core/src/facade/study_facade.rs'
    'crates/app-core/src/services/reports_service.rs'
    'crates/app-core/src/services/wrong_words_service.rs'
    'crates/platform-mobile/src/bridge.rs'
    'apps/flutter_mobile/test/study_question_display_test.dart'
)

function Get-Sha256ForBytes {
    param([byte[]]$Bytes)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        return ([BitConverter]::ToString($sha.ComputeHash($Bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Get-Sha256ForText {
    param([string]$Text)
    return Get-Sha256ForBytes -Bytes ([Text.Encoding]::UTF8.GetBytes($Text))
}

function Get-Sha256ForFile {
    param([string]$Path)
    return Get-Sha256ForBytes -Bytes ([IO.File]::ReadAllBytes($Path))
}

function Write-JsonAtomically {
    param(
        [string]$Path,
        [object]$Value
    )

    $directory = Split-Path $Path -Parent
    [IO.Directory]::CreateDirectory($directory) | Out-Null
    $temporaryPath = "$Path.tmp.$PID"
    $json = $Value | ConvertTo-Json -Depth 20
    $encoding = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText($temporaryPath, "$json`n", $encoding)
    Move-Item -LiteralPath $temporaryPath -Destination $Path -Force
}

function Read-JsonFile {
    param([string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "Required JSON file is missing: $Path"
    }
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Get-SourceLockSnapshot {
    param([int]$AcceptedWave)

    $entries = foreach ($relativePath in $script:AuthoritativePaths) {
        $absolutePath = Join-Path $script:RepositoryRoot $relativePath
        if (-not (Test-Path -LiteralPath $absolutePath -PathType Leaf)) {
            throw "Authoritative source is missing: $relativePath"
        }
        [ordered]@{
            path = $relativePath
            sha256 = Get-Sha256ForFile -Path $absolutePath
        }
    }

    $ledgerAbsolutePath = Join-Path $script:RepositoryRoot $script:LedgerPath
    if (-not (Test-Path -LiteralPath $ledgerAbsolutePath -PathType Leaf)) {
        throw "Learning ledger is missing: $($script:LedgerPath)"
    }
    $ledgerHash = Get-Sha256ForFile -Path $ledgerAbsolutePath

    $digestLines = @($entries | ForEach-Object { "$($_.path)=$($_.sha256)" })
    $digestLines += "$($script:LedgerPath)=$ledgerHash"
    $aggregate = Get-Sha256ForText -Text (($digestLines | Sort-Object) -join "`n")

    $gitCommit = (& git -C $script:RepositoryRoot rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to resolve the repository commit.'
    }
    $gitStatus = @(& git -C $script:RepositoryRoot status --short)
    if ($LASTEXITCODE -ne 0) {
        throw 'Unable to inspect the repository dirty state.'
    }

    return [ordered]@{
        schemaVersion = 1
        acceptedWave = $AcceptedWave
        capturedAtUtc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
        gitCommit = $gitCommit
        gitDirty = ($gitStatus.Count -gt 0)
        authoritativeInputs = @($entries)
        learningLedger = [ordered]@{
            path = $script:LedgerPath
            sha256 = $ledgerHash
        }
        aggregateSha256 = $aggregate
    }
}

function Get-SourceChanges {
    param(
        [object]$Accepted,
        [object]$Current
    )

    $acceptedByPath = @{}
    foreach ($entry in @($Accepted.authoritativeInputs)) {
        $acceptedByPath[[string]$entry.path] = [string]$entry.sha256
    }
    $currentByPath = @{}
    foreach ($entry in @($Current.authoritativeInputs)) {
        $currentByPath[[string]$entry.path] = [string]$entry.sha256
    }

    $paths = @($acceptedByPath.Keys) + @($currentByPath.Keys) | Sort-Object -Unique
    $changes = @()
    foreach ($path in $paths) {
        if (-not $acceptedByPath.ContainsKey($path) -or
            -not $currentByPath.ContainsKey($path) -or
            $acceptedByPath[$path] -ne $currentByPath[$path]) {
            $changes += $path
        }
    }
    if ([string]$Accepted.learningLedger.sha256 -ne [string]$Current.learningLedger.sha256) {
        $changes += [string]$Current.learningLedger.path
    }
    return @($changes)
}

function Compare-SourceLockSnapshot {
    param(
        [object]$Accepted,
        [object]$Current,
        [int]$CheckedWave
    )

    $changes = @(Get-SourceChanges -Accepted $Accepted -Current $Current)
    if ($changes.Count -gt 0 -or [string]$Accepted.aggregateSha256 -ne [string]$Current.aggregateSha256) {
        $display = ($changes | ForEach-Object { "  - $_" }) -join "`n"
        throw "Source lock drift detected before wave $CheckedWave. Changed authoritative inputs:`n$display"
    }
    if ($CheckedWave -lt [int]$Accepted.acceptedWave) {
        throw "Wave $CheckedWave predates accepted lock wave $($Accepted.acceptedWave)."
    }

    Write-Output "Source lock OK for wave $CheckedWave (accepted wave $($Accepted.acceptedWave), digest $($Accepted.aggregateSha256))."
}

function Assert-PromotionEvidence {
    param(
        [object]$Accepted,
        [object]$Current,
        [string]$ReviewedDiffPath,
        [string]$ParityEvidencePath
    )

    $changes = @(Get-SourceChanges -Accepted $Accepted -Current $Current)
    $sourceChanges = @($changes | Where-Object { $_ -ne $script:LedgerPath })
    if ($sourceChanges.Count -eq 0) {
        throw 'Promotion requires at least one authoritative product-source change.'
    }
    if ($changes -notcontains $script:LedgerPath) {
        throw "Promotion requires an updated $($script:LedgerPath) digest."
    }

    $review = Read-JsonFile -Path $ReviewedDiffPath
    if ($review.reviewed -ne $true) {
        throw 'Reviewed diff evidence must set reviewed=true.'
    }
    $reviewedFiles = @($review.changedFiles | ForEach-Object { [string]$_ })
    $missingReviews = @($sourceChanges | Where-Object { $reviewedFiles -notcontains $_ })
    if ($missingReviews.Count -gt 0) {
        throw "Reviewed diff does not cover changed files: $($missingReviews -join ', ')"
    }

    $parity = Read-JsonFile -Path $ParityEvidencePath
    if ($parity.success -ne $true) {
        throw 'Parity evidence must set success=true.'
    }
    if ([string]$parity.sourceLockDigest -ne [string]$Accepted.aggregateSha256) {
        throw 'Parity evidence was not produced from the currently accepted source lock.'
    }
    if ([string]$parity.learningLedgerSha256 -ne [string]$Current.learningLedger.sha256) {
        throw 'Parity evidence does not reference the updated learning-ledger digest.'
    }
    $checks = @($parity.checks)
    if ($checks.Count -eq 0 -or @($checks | Where-Object { $_.status -ne 'passed' }).Count -gt 0) {
        throw 'Parity evidence must contain at least one check and every check must have status=passed.'
    }
}

function Invoke-ContractSelfTest {
    $accepted = [pscustomobject]@{
        acceptedWave = 0
        aggregateSha256 = 'old-digest'
        authoritativeInputs = @([pscustomobject]@{ path = 'source.rs'; sha256 = 'old-source' })
        learningLedger = [pscustomobject]@{ path = $script:LedgerPath; sha256 = 'old-ledger' }
    }
    $matching = [pscustomobject]@{
        aggregateSha256 = 'old-digest'
        authoritativeInputs = @([pscustomobject]@{ path = 'source.rs'; sha256 = 'old-source' })
        learningLedger = [pscustomobject]@{ path = $script:LedgerPath; sha256 = 'old-ledger' }
    }
    Compare-SourceLockSnapshot -Accepted $accepted -Current $matching -CheckedWave 0 | Out-Null

    $changed = [pscustomobject]@{
        aggregateSha256 = 'new-digest'
        authoritativeInputs = @([pscustomobject]@{ path = 'source.rs'; sha256 = 'new-source' })
        learningLedger = [pscustomobject]@{ path = $script:LedgerPath; sha256 = 'new-ledger' }
    }
    $driftRejected = $false
    try {
        Compare-SourceLockSnapshot -Accepted $accepted -Current $changed -CheckedWave 1 | Out-Null
    }
    catch {
        $driftRejected = $true
    }
    if (-not $driftRejected) {
        throw 'Self-test expected drift to fail closed.'
    }

    $testDir = Join-Path ([IO.Path]::GetTempPath()) "word-source-lock-$PID"
    [IO.Directory]::CreateDirectory($testDir) | Out-Null
    try {
        $reviewPath = Join-Path $testDir 'review.json'
        $parityPath = Join-Path $testDir 'parity.json'
        Write-JsonAtomically -Path $reviewPath -Value ([ordered]@{
            reviewed = $true
            changedFiles = @('source.rs')
        })
        Write-JsonAtomically -Path $parityPath -Value ([ordered]@{
            success = $true
            sourceLockDigest = 'old-digest'
            learningLedgerSha256 = 'new-ledger'
            checks = @([ordered]@{ name = 'native-parity'; status = 'passed' })
        })
        Assert-PromotionEvidence -Accepted $accepted -Current $changed -ReviewedDiffPath $reviewPath -ParityEvidencePath $parityPath
    }
    finally {
        Remove-Item -LiteralPath $testDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Output 'Source-lock contract self-test passed.'
}

if ($SelfTest) {
    Invoke-ContractSelfTest
    exit 0
}

if ($Capture) {
    if ($Wave -ne 0) {
        throw 'Initial capture is only permitted for wave 0; later changes require promotion.'
    }
    $snapshot = Get-SourceLockSnapshot -AcceptedWave 0
    Write-JsonAtomically -Path $script:LockPath -Value $snapshot
    if (-not (Test-Path -LiteralPath $script:PromotionsPath)) {
        Write-JsonAtomically -Path $script:PromotionsPath -Value ([ordered]@{
            schemaVersion = 1
            promotions = @()
        })
    }
    Write-Output "Captured source lock for wave 0 (digest $($snapshot.aggregateSha256))."
    exit 0
}

$acceptedLock = Read-JsonFile -Path $script:LockPath
$currentSnapshot = Get-SourceLockSnapshot -AcceptedWave ([int]$acceptedLock.acceptedWave)

if ($Check) {
    Compare-SourceLockSnapshot -Accepted $acceptedLock -Current $currentSnapshot -CheckedWave $Wave
    exit 0
}

if ($Wave -le [int]$acceptedLock.acceptedWave) {
    throw "Promotion wave $Wave must be greater than accepted wave $($acceptedLock.acceptedWave)."
}

$reviewedDiffPath = (Resolve-Path -LiteralPath $ReviewedDiff).Path
$parityEvidencePath = (Resolve-Path -LiteralPath $ParityEvidence).Path
Assert-PromotionEvidence -Accepted $acceptedLock -Current $currentSnapshot -ReviewedDiffPath $reviewedDiffPath -ParityEvidencePath $parityEvidencePath

$currentSnapshot.acceptedWave = $Wave
$promotions = Read-JsonFile -Path $script:PromotionsPath
$promotionEntry = [ordered]@{
    wave = $Wave
    promotedAtUtc = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    oldAggregateSha256 = [string]$acceptedLock.aggregateSha256
    newAggregateSha256 = [string]$currentSnapshot.aggregateSha256
    oldLearningLedgerSha256 = [string]$acceptedLock.learningLedger.sha256
    newLearningLedgerSha256 = [string]$currentSnapshot.learningLedger.sha256
    changedFiles = @(Get-SourceChanges -Accepted $acceptedLock -Current $currentSnapshot)
    reviewedDiff = [ordered]@{
        path = $ReviewedDiff
        sha256 = Get-Sha256ForFile -Path $reviewedDiffPath
    }
    parityEvidence = [ordered]@{
        path = $ParityEvidence
        sha256 = Get-Sha256ForFile -Path $parityEvidencePath
    }
}
$promotions.promotions = @($promotions.promotions) + @($promotionEntry)
Write-JsonAtomically -Path $script:PromotionsPath -Value $promotions
Write-JsonAtomically -Path $script:LockPath -Value $currentSnapshot

$postPromotionLock = Read-JsonFile -Path $script:LockPath
$postPromotionCurrent = Get-SourceLockSnapshot -AcceptedWave $Wave
Compare-SourceLockSnapshot -Accepted $postPromotionLock -Current $postPromotionCurrent -CheckedWave $Wave
Write-Output "Promoted source lock to wave $Wave (digest $($currentSnapshot.aggregateSha256))."
