# Checks the template end to end: generates a throwaway project in a temp
# folder whose path contains a space, with a hyphenated name that has a
# digit, then runs that project's full `just ci` plus a few extra checks.
# The folder is deleted when everything passes and kept when something fails.
# Run it with `just selftest`.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$name = 'selftest-app2'
$parent = Join-Path ([System.IO.Path]::GetTempPath()) ('rgt selftest ' + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $parent | Out-Null
$project = Join-Path $parent $name

function Stop-Selftest([string]$Message) {
    Write-Host "selftest FAILED: $Message" -ForegroundColor Red
    Write-Host "The project is kept for inspection: $project"
    exit 1
}

function Invoke-Step([string]$Label, [scriptblock]$Step) {
    Write-Host "== $Label" -ForegroundColor Cyan
    & $Step
    if ($LASTEXITCODE) { Stop-Selftest "$Label (exit $LASTEXITCODE)" }
}

Invoke-Step 'generate the project' { & (Join-Path $root 'new-project.ps1') $name -Path $parent -SkipDoctor }

Write-Host '== no placeholder left in names or contents' -ForegroundColor Cyan
$placeholders = @('rgt-app', 'rgt_app', 'Rgt App')
$badNames = Get-ChildItem -Path $project -Recurse -Force | Where-Object { $_.Name -match 'rgt' }
if ($badNames) { Stop-Selftest ('placeholder in names: ' + ($badNames.FullName -join ', ')) }
$badText = Get-ChildItem -Path $project -Recurse -File -Force |
    Where-Object { $_.FullName -notmatch '[\\/](target|\.git)[\\/]' } |
    Select-String -Pattern $placeholders -SimpleMatch -List
if ($badText) { Stop-Selftest ('placeholder in contents: ' + (($badText | ForEach-Object { $_.Path }) -join ', ')) }

Write-Host '== a clean slate: nothing about the template, empty record folders' -ForegroundColor Cyan
$traces = Get-ChildItem -Path $project -Recurse -File -Force |
    Where-Object { $_.FullName -notmatch '[\\/](target|\.git)[\\/]' } |
    Select-String -Pattern 'template', 'rgnew', 'install.ps1' -SimpleMatch -List
if ($traces) { Stop-Selftest ('mentions of the template in: ' + (($traces | ForEach-Object { $_.Path }) -join ', ')) }
foreach ($record in @('docs\design\.gitkeep', '.claude\handoff\HANDOFF.md')) {
    $file = Join-Path $project $record
    if (-not (Test-Path $file)) { Stop-Selftest "missing $record" }
    if ((Get-Item $file).Length -ne 0) { Stop-Selftest "$record is not empty" }
}
$extra = @(Get-ChildItem -Path (Join-Path $project 'docs\design'), (Join-Path $project '.claude') -Recurse -File -Force |
    Where-Object { $_.Name -notin @('.gitkeep', 'HANDOFF.md') })
if ($extra.Count) { Stop-Selftest ('template history was copied: ' + ($extra.FullName -join ', ')) }

Write-Host '== standard folders, each with a README, and git handles asset files' -ForegroundColor Cyan
foreach ($doc in @('assets\README.md', 'config\README.md', 'data\README.md', 'platform\README.md', 'docs\CONVENTIONS.md')) {
    $file = Join-Path $project $doc
    if (-not (Test-Path $file) -or (Get-Item $file).Length -eq 0) { Stop-Selftest "missing or empty $doc" }
}
if (-not (Test-Path (Join-Path $project '.git'))) { Stop-Selftest 'the project is not a git repository' }
git -C $project check-ignore -q assets/models/ship.obj
if ($LASTEXITCODE -eq 0) { Stop-Selftest 'git ignores .obj models in assets/' }
git -C $project check-ignore -q crates/stray.obj
if ($LASTEXITCODE -ne 0) { Stop-Selftest 'git no longer ignores stray compiler .obj files' }
foreach ($asset in @('assets/a.png', 'assets/a.ogg', 'assets/a.ttf', 'assets/a.glb', 'assets/a.ktx2')) {
    $attr = git -C $project check-attr binary -- $asset
    if ($attr -notmatch ': set$') { Stop-Selftest "$asset is not marked binary ($attr)" }
}
$global:LASTEXITCODE = 0

Push-Location $project
try {
    Invoke-Step 'just ci (lint, Goose tests, Rust tests in debug and release, audit)' { just ci }
    Invoke-Step 'plain cargo test (threads instead of one process per test)' { cargo test --workspace -q }
    Invoke-Step 'the program runs' { cargo run -q -p $name }

    Write-Host '== editing a Goose file rebuilds the bridge' -ForegroundColor Cyan
    $kernels = Join-Path $project "crates\$name-core\goose\kernels\kernels.goose"
    (Get-Item $kernels).LastWriteTime = Get-Date
    $log = cargo build -p "$name-core" -v 2>&1 | Out-String
    if ($log -notmatch 'build-script-build') { Stop-Selftest 'the build script did not run again after a Goose edit' }
} finally {
    Pop-Location
}

Remove-Item -Recurse -Force $parent
Write-Host ''
Write-Host 'selftest passed: the template generates a project that passes every check.' -ForegroundColor Green
