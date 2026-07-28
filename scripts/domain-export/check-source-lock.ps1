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

function Get-SourceLockSnapshot {
    throw 'Not implemented'
}

function Compare-SourceLockSnapshot {
    throw 'Not implemented'
}

function Assert-PromotionEvidence {
    throw 'Not implemented'
}

function Invoke-ContractSelfTest {
    $snapshot = Get-SourceLockSnapshot
    if (-not $snapshot.aggregateSha256) {
        throw 'Snapshot must contain an aggregate digest.'
    }
    Compare-SourceLockSnapshot -Accepted $snapshot -Current $snapshot
    Assert-PromotionEvidence -Accepted $snapshot -Current $snapshot
}

if ($SelfTest) {
    Invoke-ContractSelfTest
    exit 0
}

throw 'Source-lock commands are not implemented yet.'
