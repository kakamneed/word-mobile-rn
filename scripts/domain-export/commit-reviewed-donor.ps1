[CmdletBinding(DefaultParameterSetName = 'Commit')]
param(
    [Parameter(ParameterSetName = 'Commit', Mandatory = $true)]
    [string[]]$Paths,

    [Parameter(ParameterSetName = 'Commit', Mandatory = $true)]
    [string]$Message,

    [Parameter(ParameterSetName = 'Commit')]
    [string]$ExpectedParent,

    [Parameter(ParameterSetName = 'SelfTest', Mandatory = $true)]
    [switch]$SelfTest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:RequiredRepositoryRoot = 'D:/projects/word-mobile-rn'
[string[]]$script:AllowedPaths = @(
    'fixtures/domain/v1/requests/phase6-six-modes.json',
    'fixtures/domain/v1/expected/phase6-six-modes.json',
    'fixtures/domain/v1/requests/phase6-reconcile.json',
    'fixtures/domain/v1/expected/phase6-reconcile.json',
    'fixtures/domain/v1/requests/phase6-mastery.json',
    'fixtures/domain/v1/expected/phase6-mastery.json',
    'fixtures/domain/v1/manifest.json',
    'crates/app-core/tests/baseline_runner.rs',
    'scripts/domain-export/validate-fixture-manifest.mjs',
    'scripts/domain-export/commit-reviewed-donor.ps1',
    'crates/domain-models/src/study.rs',
    'crates/domain-models/src/projections.rs',
    'crates/domain-core/src/study.rs',
    'crates/domain-core/src/progress.rs',
    'crates/domain-protocol/src/v1.rs',
    'crates/domain-wasm/src/lib.rs',
    'scripts/domain-export/check-source-lock.ps1',
    'scripts/domain-export/build-package.ps1',
    'scripts/domain-export/validate-manifest.mjs',
    'scripts/domain-export/validate-manifest.test.mjs',
    'scripts/domain-export/run-browser-gates.ps1',
    'tests/domain-browser/domain-wasm.spec.ts',
    'fixtures/domain/v1/source-lock.json',
    'fixtures/domain/v1/source-lock-promotions.json',
    'fixtures/domain/v1/evidence/wave-6.json',
    'fixtures/domain/v1/evidence/wave-6-reviewed-diff.json',
    'docs/features/learning.md',
    'docs/features/plan-page.md',
    'docs/features/today-page.md',
    'artifacts/domain-wasm/manifest.json',
    'artifacts/domain-wasm/package/package.json',
    'artifacts/domain-wasm/package/word_domain_wasm.js',
    'artifacts/domain-wasm/package/word_domain_wasm.d.ts',
    'artifacts/domain-wasm/package/word_domain_wasm_bg.wasm',
    'artifacts/domain-wasm/package/word_domain_wasm_bg.wasm.d.ts'
)

function Invoke-Git {
    param(
        [string]$RepositoryRoot,
        [string[]]$Arguments,
        [switch]$AllowFailure
    )

    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = @(& git -C $RepositoryRoot @Arguments 2>&1)
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if (-not $AllowFailure -and $exitCode -ne 0) {
        throw "git $($Arguments -join ' ') failed: $($output -join [Environment]::NewLine)"
    }
    return [pscustomobject]@{ ExitCode = $exitCode; Output = [string[]]$output }
}

function Assert-OrdinalPathSet {
    param([string[]]$Expected, [string[]]$Actual, [string]$Label)

    [string[]]$expectedSorted = @($Expected)
    [string[]]$actualSorted = @($Actual)
    [Array]::Sort($expectedSorted, [StringComparer]::Ordinal)
    [Array]::Sort($actualSorted, [StringComparer]::Ordinal)
    if ($expectedSorted.Length -ne $actualSorted.Length) {
        throw "$Label path count mismatch"
    }
    for ($index = 0; $index -lt $expectedSorted.Length; $index++) {
        if (-not [StringComparer]::Ordinal.Equals([string]$expectedSorted[$index], [string]$actualSorted[$index])) {
            throw "$Label path mismatch"
        }
    }
}

function Normalize-ReviewedPaths {
    param([string]$RepositoryRoot, [string[]]$CandidatePaths)

    if ($CandidatePaths.Count -eq 0) {
        throw 'At least one reviewed path is required.'
    }
    $seen = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $allowed = [Collections.Generic.HashSet[string]]::new($script:AllowedPaths, [StringComparer]::Ordinal)
    [string[]]$normalized = @()
    foreach ($candidate in $CandidatePaths) {
        if ([string]::IsNullOrWhiteSpace($candidate) -or
            [IO.Path]::IsPathRooted($candidate) -or
            $candidate.Contains('\') -or
            $candidate.StartsWith('/') -or
            $candidate.EndsWith('/')) {
            throw "Reviewed path must be a normalized relative POSIX path: $candidate"
        }
        [string[]]$segments = @($candidate.Split('/'))
        if ($segments.Count -eq 0 -or @($segments | Where-Object { $_ -eq '' -or $_ -eq '.' -or $_ -eq '..' }).Count -gt 0) {
            throw "Reviewed path escapes or is not normalized: $candidate"
        }
        if (-not $seen.Add($candidate)) {
            throw "Duplicate reviewed path: $candidate"
        }
        if (-not $allowed.Contains($candidate)) {
            throw "Path is not in the Phase 6 donor allowlist: $candidate"
        }
        $absolute = Join-Path $RepositoryRoot $candidate
        if (-not (Test-Path -LiteralPath $absolute -PathType Leaf)) {
            throw "Reviewed path is missing: $candidate"
        }
        $normalized += $candidate
    }
    return [string[]]$normalized
}

function Invoke-ReviewedCommit {
    param(
        [string]$RepositoryRoot,
        [string[]]$ReviewedPaths,
        [string]$CommitMessage,
        [string]$RequiredParent
    )

    $resolvedRoot = (Resolve-Path -LiteralPath $RepositoryRoot).Path.Replace('\', '/').TrimEnd('/')
    $gitRoot = (Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('rev-parse', '--show-toplevel')).Output
    if ($gitRoot.Count -ne 1 -or -not [StringComparer]::Ordinal.Equals($resolvedRoot, $gitRoot[0].Replace('\', '/').TrimEnd('/'))) {
        throw 'Repository root does not resolve to the Git top-level.'
    }
    if ([string]::IsNullOrWhiteSpace($CommitMessage) -or $CommitMessage.Contains("`n") -or $CommitMessage.Contains("`r")) {
        throw 'Commit message must be one nonempty subject line.'
    }

    [string[]]$normalizedPaths = @(Normalize-ReviewedPaths -RepositoryRoot $resolvedRoot -CandidatePaths $ReviewedPaths)
    [string[]]$stagedBefore = @((Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('diff', '--cached', '--name-only')).Output)
    if ($stagedBefore.Length -gt 0) {
        throw 'Preexisting staged paths are not permitted.'
    }

    $oldHeadOutput = (Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('rev-parse', 'HEAD')).Output
    if ($oldHeadOutput.Count -ne 1 -or $oldHeadOutput[0] -notmatch '^[0-9a-f]{40}$') {
        throw 'Cannot resolve one lowercase 40-hex parent.'
    }
    $oldHead = $oldHeadOutput[0]
    if ($RequiredParent) {
        if ($RequiredParent -notmatch '^[0-9a-f]{40}$' -or
            -not [StringComparer]::Ordinal.Equals([string]$RequiredParent, [string]$oldHead)) {
            throw 'ExpectedParent does not equal the current donor HEAD.'
        }
    }

    $stage = Invoke-Git -RepositoryRoot $resolvedRoot -Arguments (@('add', '--') + $normalizedPaths) -AllowFailure
    if ($stage.ExitCode -ne 0) {
        throw "Unable to stage reviewed paths: $($stage.Output -join [Environment]::NewLine)"
    }
    try {
        [string[]]$stagedPaths = @((Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('diff', '--cached', '--name-only')).Output)
        Assert-OrdinalPathSet -Expected $normalizedPaths -Actual $stagedPaths -Label 'Staged'

        $commit = Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('commit', '--no-verify', '-m', $CommitMessage) -AllowFailure
        if ($commit.ExitCode -ne 0) {
            throw "Reviewed donor commit failed: $($commit.Output -join [Environment]::NewLine)"
        }
    }
    catch {
        Invoke-Git -RepositoryRoot $resolvedRoot -Arguments (@('reset', '--quiet', 'HEAD', '--') + $normalizedPaths) -AllowFailure | Out-Null
        throw
    }

    $newHeadOutput = (Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('rev-parse', 'HEAD')).Output
    $newParentOutput = (Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('rev-parse', 'HEAD^')).Output
    $subjectOutput = (Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('log', '-1', '--pretty=%s')).Output
    if ($newHeadOutput.Count -ne 1 -or $newHeadOutput[0] -notmatch '^[0-9a-f]{40}$') {
        throw 'Committed donor HEAD is not one lowercase 40-hex hash.'
    }
    if ($newParentOutput.Count -ne 1 -or
        -not [StringComparer]::Ordinal.Equals([string]$oldHead, [string]$newParentOutput[0])) {
        throw 'Committed donor parent does not equal the reviewed old HEAD.'
    }
    if ($subjectOutput.Count -ne 1 -or
        -not [StringComparer]::Ordinal.Equals([string]$CommitMessage, [string]$subjectOutput[0])) {
        throw 'Committed donor subject does not equal the planned message.'
    }
    [string[]]$committedPaths = @((Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('diff-tree', '--no-commit-id', '--name-only', '-r', 'HEAD')).Output)
    Assert-OrdinalPathSet -Expected $normalizedPaths -Actual $committedPaths -Label 'Committed'
    [string[]]$stagedAfter = @((Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('diff', '--cached', '--name-only')).Output)
    if ($stagedAfter.Length -gt 0) {
        throw 'Staged paths remain after the reviewed donor commit.'
    }

    $emittedHash = $newHeadOutput[0]
    $verifiedHead = (Invoke-Git -RepositoryRoot $resolvedRoot -Arguments @('rev-parse', 'HEAD')).Output
    if ($verifiedHead.Count -ne 1 -or
        -not [StringComparer]::Ordinal.Equals([string]$emittedHash, [string]$verifiedHead[0])) {
        throw 'Emitted donor hash does not equal the new HEAD.'
    }
    return $emittedHash
}

function New-SelfTestRepository {
    param([string]$Name)

    $root = Join-Path ([IO.Path]::GetTempPath()) "word-donor-helper-$PID-$Name"
    if (Test-Path -LiteralPath $root) {
        Remove-Item -LiteralPath $root -Recurse -Force
    }
    [IO.Directory]::CreateDirectory($root) | Out-Null
    Invoke-Git -RepositoryRoot $root -Arguments @('init', '--quiet') | Out-Null
    Invoke-Git -RepositoryRoot $root -Arguments @('config', 'user.name', 'Donor Helper Self Test') | Out-Null
    Invoke-Git -RepositoryRoot $root -Arguments @('config', 'user.email', 'donor-helper@example.invalid') | Out-Null
    foreach ($relative in @('fixtures/domain/v1/manifest.json', 'scripts/domain-export/validate-fixture-manifest.mjs')) {
        $absolute = Join-Path $root $relative
        [IO.Directory]::CreateDirectory((Split-Path $absolute -Parent)) | Out-Null
        [IO.File]::WriteAllText($absolute, "initial`n")
    }
    Invoke-Git -RepositoryRoot $root -Arguments @('add', '--', 'fixtures/domain/v1/manifest.json', 'scripts/domain-export/validate-fixture-manifest.mjs') | Out-Null
    Invoke-Git -RepositoryRoot $root -Arguments @('commit', '--quiet', '-m', 'initial') | Out-Null
    return $root
}

function Assert-Rejected {
    param([scriptblock]$Action, [string]$Label)
    $rejected = $false
    try { & $Action | Out-Null } catch { $rejected = $true }
    if (-not $rejected) { throw "Self-test expected rejection: $Label" }
}

function Invoke-HelperSelfTest {
    [string[]]$roots = @()
    try {
        $root = New-SelfTestRepository -Name 'pollution'; $roots += $root
        [IO.File]::AppendAllText((Join-Path $root 'fixtures/domain/v1/manifest.json'), "change`n")
        [IO.File]::AppendAllText((Join-Path $root 'scripts/domain-export/validate-fixture-manifest.mjs'), "staged`n")
        Invoke-Git -RepositoryRoot $root -Arguments @('add', '--', 'scripts/domain-export/validate-fixture-manifest.mjs') | Out-Null
        Assert-Rejected { Invoke-ReviewedCommit $root @('fixtures/domain/v1/manifest.json') 'self test' $null } 'staged pollution'

        $root = New-SelfTestRepository -Name 'traversal'; $roots += $root
        Assert-Rejected { Normalize-ReviewedPaths $root @('../manifest.json') } 'traversal'

        $root = New-SelfTestRepository -Name 'dirty'; $roots += $root
        [IO.File]::WriteAllText((Join-Path $root 'unrelated.txt'), "preserve me`n")
        [IO.File]::AppendAllText((Join-Path $root 'fixtures/domain/v1/manifest.json'), "reviewed`n")
        $hash = Invoke-ReviewedCommit $root @('fixtures/domain/v1/manifest.json') 'self test dirty preservation' $null
        [string[]]$paths = @((Invoke-Git $root @('diff-tree', '--no-commit-id', '--name-only', '-r', $hash)).Output)
        Assert-OrdinalPathSet @('fixtures/domain/v1/manifest.json') $paths 'Self-test committed'
        if (-not (Test-Path -LiteralPath (Join-Path $root 'unrelated.txt'))) { throw 'Self-test lost unrelated untracked file.' }

        $root = New-SelfTestRepository -Name 'commit-failure'; $roots += $root
        [IO.File]::AppendAllText((Join-Path $root 'fixtures/domain/v1/manifest.json'), "reviewed`n")
        $savedAuthorDate = $env:GIT_AUTHOR_DATE
        $savedCommitterDate = $env:GIT_COMMITTER_DATE
        try {
            $env:GIT_AUTHOR_DATE = 'not-a-valid-git-date'
            $env:GIT_COMMITTER_DATE = 'not-a-valid-git-date'
            Assert-Rejected { Invoke-ReviewedCommit $root @('fixtures/domain/v1/manifest.json') 'self test commit failure' $null } 'commit failure'
        }
        finally {
            $env:GIT_AUTHOR_DATE = $savedAuthorDate
            $env:GIT_COMMITTER_DATE = $savedCommitterDate
        }
        [string[]]$staged = @((Invoke-Git $root @('diff', '--cached', '--name-only')).Output)
        if ($staged.Length -gt 0) { throw 'Self-test commit failure left staged paths.' }

        $root = New-SelfTestRepository -Name 'wrong-parent'; $roots += $root
        [IO.File]::AppendAllText((Join-Path $root 'fixtures/domain/v1/manifest.json'), "reviewed`n")
        Assert-Rejected { Invoke-ReviewedCommit $root @('fixtures/domain/v1/manifest.json') 'self test wrong parent' ('0' * 40) } 'wrong parent'

        $root = New-SelfTestRepository -Name 'dirty-tracked'; $roots += $root
        [IO.File]::AppendAllText((Join-Path $root 'scripts/domain-export/validate-fixture-manifest.mjs'), "unrelated dirty`n")
        $before = [IO.File]::ReadAllText((Join-Path $root 'scripts/domain-export/validate-fixture-manifest.mjs'))
        [IO.File]::AppendAllText((Join-Path $root 'fixtures/domain/v1/manifest.json'), "reviewed`n")
        Invoke-ReviewedCommit $root @('fixtures/domain/v1/manifest.json') 'self test tracked preservation' $null | Out-Null
        $after = [IO.File]::ReadAllText((Join-Path $root 'scripts/domain-export/validate-fixture-manifest.mjs'))
        if (-not [StringComparer]::Ordinal.Equals($before, $after)) { throw 'Self-test changed unrelated dirty file.' }
    }
    finally {
        foreach ($root in $roots) {
            if (Test-Path -LiteralPath $root) { Remove-Item -LiteralPath $root -Recurse -Force }
        }
    }
    Write-Output 'Reviewed donor helper self-test passed.'
}

if ($SelfTest) {
    Invoke-HelperSelfTest
    exit 0
}

$actualRoot = (Resolve-Path -LiteralPath (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)).Path.Replace('\', '/').TrimEnd('/')
if (-not [StringComparer]::Ordinal.Equals([string]$script:RequiredRepositoryRoot, [string]$actualRoot)) {
    throw "Helper must run from $($script:RequiredRepositoryRoot); resolved $actualRoot"
}

$donorCommit = Invoke-ReviewedCommit -RepositoryRoot $actualRoot -ReviewedPaths $Paths -CommitMessage $Message -RequiredParent $ExpectedParent
Write-Output "DONOR_COMMIT=$donorCommit"
