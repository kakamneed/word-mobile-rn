[CmdletBinding()]
param(
    [switch]$Candidate,
    [switch]$CleanCommittedWorktree,
    [switch]$ReproducibilityCheck,
    [switch]$InternalBuild,
    [string]$OutputDirectory,
    [string]$SourceCommit
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent

function Get-FileSha256([string]$Path) {
    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Get-TextSha256([string]$Text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($Text)))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Get-ValueSha256([object]$Value) {
    return Get-TextSha256 ($Value | ConvertTo-Json -Depth 20 -Compress)
}

function Get-FixtureInventory([string]$FixtureRoot, [object]$FixtureManifest) {
    $seenIds = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $seenPaths = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $inventory = foreach ($collection in @('fixtures', 'lifecycleFixtures')) {
        foreach ($fixture in @($FixtureManifest.$collection)) {
            $id = [string]$fixture.id
            $requestPath = [string]$fixture.request
            $expectedPath = [string]$fixture.expected
            if ([string]::IsNullOrWhiteSpace($id) -or -not $seenIds.Add($id)) { throw "Duplicate or empty fixture id: $id" }
            foreach ($path in @($requestPath, $expectedPath)) {
                if ([string]::IsNullOrWhiteSpace($path) -or -not $seenPaths.Add($path)) { throw "Duplicate or empty fixture path: $path" }
                if (-not (Test-Path -LiteralPath (Join-Path $FixtureRoot $path) -PathType Leaf)) { throw "Fixture file is missing: $path" }
            }
            [ordered]@{
                id = $id
                collection = $collection
                requestPath = $requestPath
                requestSha256 = Get-FileSha256 (Join-Path $FixtureRoot $requestPath)
                expectedPath = $expectedPath
                expectedSha256 = Get-FileSha256 (Join-Path $FixtureRoot $expectedPath)
            }
        }
    }
    return @($inventory | Sort-Object { $_.id })
}

function Get-ArtifactEntry([string]$Path, [string]$RelativePath, [switch]$IncludeGzip) {
    $entry = [ordered]@{
        path = $RelativePath
        sha256 = Get-FileSha256 $Path
        rawBytes = (Get-Item -LiteralPath $Path).Length
    }
    if ($IncludeGzip) { $entry.gzipBytes = Get-GzipSize $Path }
    return $entry
}

function Get-GzipSize([string]$Path) {
    $input = [IO.File]::OpenRead($Path)
    $output = New-Object IO.MemoryStream
    try {
        $gzip = New-Object IO.Compression.GZipStream($output, [IO.Compression.CompressionMode]::Compress, $true)
        try { $input.CopyTo($gzip) } finally { $gzip.Dispose() }
        return $output.Length
    }
    finally { $input.Dispose(); $output.Dispose() }
}

function Write-Json([string]$Path, [object]$Value) {
    [IO.Directory]::CreateDirectory((Split-Path $Path -Parent)) | Out-Null
    [IO.File]::WriteAllText($Path, "$(ConvertTo-Json $Value -Depth 20)`n", (New-Object Text.UTF8Encoding($false)))
}

function Assert-SafeBuildPath([string]$Path) {
    $resolved = [IO.Path]::GetFullPath($Path)
    if ($resolved -eq [IO.Path]::GetFullPath($repositoryRoot) -or $resolved.Length -lt 10) {
        throw "Refusing unsafe build path: $resolved"
    }
    return $resolved
}

function Invoke-PackageBuild([string]$Destination, [string]$Commit, [bool]$ReleaseReady) {
    $destination = Assert-SafeBuildPath $Destination
    if (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Recurse -Force }
    [IO.Directory]::CreateDirectory($destination) | Out-Null
    $packageDirectory = Join-Path $destination 'package'

    & wasm-pack build (Join-Path $repositoryRoot 'crates\domain-wasm') --release --target web --out-dir $packageDirectory --out-name word_domain_wasm
    if ($LASTEXITCODE -ne 0) { throw 'wasm-pack build failed.' }

    $packageJsonPath = Join-Path $packageDirectory 'package.json'
    $wasmPath = Join-Path $packageDirectory 'word_domain_wasm_bg.wasm'
    $wasmTypesPath = Join-Path $packageDirectory 'word_domain_wasm_bg.wasm.d.ts'
    $jsPath = Join-Path $packageDirectory 'word_domain_wasm.js'
    $jsTypesPath = Join-Path $packageDirectory 'word_domain_wasm.d.ts'
    $sourceLock = Get-Content (Join-Path $repositoryRoot 'fixtures\domain\v1\source-lock.json') -Raw | ConvertFrom-Json
    $fixtureRoot = Join-Path $repositoryRoot 'fixtures\domain\v1'
    $fixtureManifestPath = Join-Path $fixtureRoot 'manifest.json'
    $fixtureManifest = Get-Content $fixtureManifestPath -Raw | ConvertFrom-Json
    $fixtureInventory = @(Get-FixtureInventory -FixtureRoot $fixtureRoot -FixtureManifest $fixtureManifest)
    $phase6Inventory = @($fixtureInventory | Where-Object { $_.id.StartsWith('phase6-', [StringComparison]::Ordinal) })
    $actualCommit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
    $dirty = @(& git -C $repositoryRoot status --short).Count -gt 0
    if ($Commit -and $actualCommit -ne $Commit) { throw "Build commit $actualCommit does not match required commit $Commit." }
    if ($ReleaseReady -and $dirty) { throw 'Pin-ready package build must use a clean worktree.' }

    $manifest = [ordered]@{
        schemaVersion = 1
        protocolVersion = [int]$fixtureManifest.protocolVersion
        source = [ordered]@{
            commit = $actualCommit
            dirty = $dirty
            sourceLockSha256 = [string]$sourceLock.aggregateSha256
            cargoLockSha256 = Get-TextSha256 ((Get-Content (Join-Path $repositoryRoot 'Cargo.lock') -Raw).Replace("`r`n", "`n"))
        }
        build = [ordered]@{
            target = 'wasm32-unknown-unknown'
            profile = 'release'
            command = 'wasm-pack build crates/domain-wasm --release --target web --out-dir <output>/package --out-name word_domain_wasm'
            rustc = (& rustc --version).Trim()
            cargo = (& cargo --version).Trim()
            wasmPack = (& wasm-pack --version).Trim()
        }
        artifacts = [ordered]@{
            packageJson = Get-ArtifactEntry $packageJsonPath 'package/package.json'
            javascript = Get-ArtifactEntry $jsPath 'package/word_domain_wasm.js'
            javascriptTypes = Get-ArtifactEntry $jsTypesPath 'package/word_domain_wasm.d.ts'
            wasm = Get-ArtifactEntry $wasmPath 'package/word_domain_wasm_bg.wasm' -IncludeGzip
            wasmTypes = Get-ArtifactEntry $wasmTypesPath 'package/word_domain_wasm_bg.wasm.d.ts'
        }
        fixtures = [ordered]@{
            manifestPath = 'fixtures/domain/v1/manifest.json'
            fixtureManifestSha256 = Get-TextSha256 ((Get-Content $fixtureManifestPath -Raw).Replace("`r`n", "`n"))
            fixtureCount = $fixtureInventory.Count
            fixtureInventorySha256 = Get-ValueSha256 $fixtureInventory
            fixtureInventory = $fixtureInventory
            phase6FixtureIds = @($phase6Inventory | ForEach-Object { $_.id })
            phase6FixtureCount = $phase6Inventory.Count
            phase6FixtureInventorySha256 = Get-ValueSha256 $phase6Inventory
        }
        budgets = [ordered]@{
            wasmRawBytes = 1048576
            wasmGzipBytes = 524288
            javascriptRawBytes = 65536
            startupMs = 5000
            firstCommandMs = 1000
            repeatCommandMs = 250
        }
        releaseReady = $ReleaseReady
    }
    Write-Json (Join-Path $destination 'manifest.json') $manifest
    return $manifest
}

if ($InternalBuild) {
    if (-not $OutputDirectory) { throw '-InternalBuild requires -OutputDirectory.' }
    Invoke-PackageBuild $OutputDirectory $SourceCommit $true | Out-Null
    exit 0
}

if ($Candidate) {
    $candidateRoot = Join-Path $repositoryRoot 'target\domain-wasm-candidate'
    Invoke-PackageBuild $candidateRoot '' $false | Out-Null
    Write-Output "Candidate package built at $candidateRoot"
    exit 0
}

if ($CleanCommittedWorktree) {
    if (-not $ReproducibilityCheck) { throw 'Pin-ready builds require -ReproducibilityCheck.' }
    $commit = (& git -C $repositoryRoot rev-parse HEAD).Trim()
    $temporaryRoot = Assert-SafeBuildPath (Join-Path ([IO.Path]::GetTempPath()) "word-domain-wasm-repro-$PID")
    $worktreeOne = Join-Path $temporaryRoot 'worktree-1'
    $worktreeTwo = Join-Path $temporaryRoot 'worktree-2'
    $buildOne = Join-Path $temporaryRoot 'build-1'
    $buildTwo = Join-Path $temporaryRoot 'build-2'
    [IO.Directory]::CreateDirectory($temporaryRoot) | Out-Null
    try {
        & git -C $repositoryRoot worktree add --detach $worktreeOne $commit
        if ($LASTEXITCODE -ne 0) { throw 'Unable to create first clean worktree.' }
        & git -C $repositoryRoot worktree add --detach $worktreeTwo $commit
        if ($LASTEXITCODE -ne 0) { throw 'Unable to create second clean worktree.' }
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $worktreeOne 'scripts\domain-export\build-package.ps1') -InternalBuild -OutputDirectory $buildOne -SourceCommit $commit
        if ($LASTEXITCODE -ne 0) { throw 'First clean package build failed.' }
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $worktreeTwo 'scripts\domain-export\build-package.ps1') -InternalBuild -OutputDirectory $buildTwo -SourceCommit $commit
        if ($LASTEXITCODE -ne 0) { throw 'Second clean package build failed.' }

        $manifestOneText = (Get-Content (Join-Path $buildOne 'manifest.json') -Raw).Trim()
        $manifestTwoText = (Get-Content (Join-Path $buildTwo 'manifest.json') -Raw).Trim()
        if ($manifestOneText -ne $manifestTwoText) { throw 'Normalized manifests differ across clean builds.' }
        $manifestOne = $manifestOneText | ConvertFrom-Json
        $manifestTwo = $manifestTwoText | ConvertFrom-Json
        if ($manifestOne.artifacts.wasm.sha256 -ne $manifestTwo.artifacts.wasm.sha256) { throw 'Optimized WASM hashes differ across clean builds.' }

        $finalRoot = Assert-SafeBuildPath (Join-Path $repositoryRoot 'artifacts\domain-wasm')
        if (Test-Path -LiteralPath $finalRoot) { Remove-Item -LiteralPath $finalRoot -Recurse -Force }
        [IO.Directory]::CreateDirectory($finalRoot) | Out-Null
        Copy-Item -LiteralPath (Join-Path $buildOne 'package') -Destination (Join-Path $finalRoot 'package') -Recurse
        $manifestOne | Add-Member -NotePropertyName reproducibility -NotePropertyValue ([ordered]@{
            checked = $true
            builds = 2
            normalizedManifestSha256 = Get-TextSha256 $manifestOneText
        })
        Write-Json (Join-Path $finalRoot 'manifest.json') $manifestOne
        Write-Output "Reproducible pin-ready package built from $commit at $finalRoot"
    }
    finally {
        if (Test-Path -LiteralPath $worktreeOne) { & git -C $repositoryRoot worktree remove --force $worktreeOne }
        if (Test-Path -LiteralPath $worktreeTwo) { & git -C $repositoryRoot worktree remove --force $worktreeTwo }
        if (Test-Path -LiteralPath $temporaryRoot) { Remove-Item -LiteralPath $temporaryRoot -Recurse -Force }
        & git -C $repositoryRoot worktree prune
    }
    exit 0
}

throw 'Specify -Candidate or -CleanCommittedWorktree -ReproducibilityCheck.'
