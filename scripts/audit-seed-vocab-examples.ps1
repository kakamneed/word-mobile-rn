param(
  [string]$BookDir = "apps/mobile/android/app/src/main/assets/seed-vocab/book",
  [string]$OverridesPath = "apps/mobile/android/app/src/main/assets/seed-vocab/example-overrides.json",
  [string]$ReportPath = "docs/seed-vocab-example-coverage.json"
)

$ErrorActionPreference = "Stop"

node scripts/audit-seed-vocab-examples.mjs `
  "--bookDir=$BookDir" `
  "--overridesPath=$OverridesPath" `
  "--reportPath=$ReportPath"
