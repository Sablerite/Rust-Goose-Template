# Checks that every tool this project needs is installed and that the pinned
# versions match. Run it with `just doctor`.
#
# Exit status: 0 when everything `just ci` needs is present, 1 otherwise.
# Works in PowerShell 7 and Windows PowerShell 5.1.

$root = Split-Path -Parent $PSScriptRoot
$script:failures = 0
$script:warnings = 0
$onWindows = $env:OS -eq 'Windows_NT'
$llvmBin = 'C:\Program Files\LLVM\bin'

function Write-Check([string]$Name, [string]$State, [string]$Detail) {
    $color = 'Red'
    if ($State -eq 'ok') { $color = 'Green' }
    elseif ($State -eq 'warn') { $color = 'Yellow'; $script:warnings++ }
    else { $script:failures++ }
    Write-Host ('  {0,-16} ' -f $Name) -NoNewline
    Write-Host ('{0,-5}' -f $State) -ForegroundColor $color -NoNewline
    Write-Host " $Detail"
}

function Find-Tool([string]$Name) {
    $cmd = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($cmd) { return $cmd.Source }
    return $null
}

function Get-FirstLine([string]$Exe, [string[]]$Arguments) {
    try {
        $out = & $Exe @Arguments 2>$null
        if ($out) { return ([string]($out | Select-Object -First 1)).Trim() }
    } catch { }
    return ''
}

Write-Host 'Checking the tools this project needs:'

# Rust, compared with the version pinned in rust-toolchain.toml.
$pinnedRust = ''
$match = Select-String -Path (Join-Path $root 'rust-toolchain.toml') -Pattern 'channel\s*=\s*"([^"]+)"' -ErrorAction SilentlyContinue | Select-Object -First 1
if ($match) { $pinnedRust = $match.Matches[0].Groups[1].Value }
if (-not (Find-Tool 'cargo')) {
    Write-Check 'Rust' 'FAIL' 'cargo not found. Install Rust from https://rustup.rs'
} else {
    Push-Location $root
    $rustc = Get-FirstLine 'rustc' @('--version')
    Pop-Location
    if ($pinnedRust -and ($rustc -notmatch [regex]::Escape($pinnedRust))) {
        Write-Check 'Rust' 'warn' ('{0} (pinned {1}; run: rustup toolchain install {1})' -f $rustc, $pinnedRust)
    } else {
        Write-Check 'Rust' 'ok' $rustc
    }
}

# Goose, compared with .goose-version.
$goose = $env:GOOSE
if (-not $goose) { $goose = Find-Tool 'goose' }
if (-not $goose -and $env:LOCALAPPDATA) {
    $candidate = Join-Path $env:LOCALAPPDATA 'Programs\goose\goose.exe'
    if (Test-Path $candidate) { $goose = $candidate }
}
if (-not $goose -or -not (Test-Path $goose)) {
    Write-Check 'Goose' 'FAIL' 'goose not found. Install Goose, or set GOOSE to its full path'
} else {
    $installed = Get-FirstLine $goose @('version')
    $pinnedFile = Join-Path $root '.goose-version'
    $pinned = ''
    if (Test-Path $pinnedFile) { $pinned = (Get-Content $pinnedFile -TotalCount 1).Trim() }
    if ($pinned -and $installed -ne $pinned) {
        Write-Check 'Goose' 'warn' ('{0} (project made with: {1})' -f $installed, $pinned)
    } else {
        Write-Check 'Goose' 'ok' $installed
    }
}

# The C compiler for the Goose code, and libclang for the Rust declarations.
if ($onWindows) {
    $choice = $env:GOOSE_C_COMPILER
    if ($choice -eq 'cl') {
        Write-Check 'C compiler' 'warn' 'MSVC cl (chosen by GOOSE_C_COMPILER); clang-cl is faster'
    } else {
        $clang = $null
        if ($choice -and $choice -ne 'clang-cl') { $clang = $choice }
        if (-not $clang) { $clang = Find-Tool 'clang-cl' }
        if (-not $clang -and (Test-Path (Join-Path $llvmBin 'clang-cl.exe'))) { $clang = Join-Path $llvmBin 'clang-cl.exe' }
        if ($clang -and (Test-Path $clang)) {
            Write-Check 'C compiler' 'ok' ('{0} ({1})' -f (Get-FirstLine $clang @('--version')), $clang)
        } else {
            Write-Check 'C compiler' 'warn' 'clang-cl not found: Goose code will build with MSVC cl (2-12% slower). Fix: winget install LLVM.LLVM'
        }
    }
    $libclangDir = $env:LIBCLANG_PATH
    if (-not $libclangDir) { $libclangDir = $llvmBin }
    if (Test-Path (Join-Path $libclangDir 'libclang.dll')) {
        Write-Check 'libclang' 'ok' (Join-Path $libclangDir 'libclang.dll')
    } else {
        Write-Check 'libclang' 'FAIL' 'libclang.dll not found. Install LLVM (winget install LLVM.LLVM) or set LIBCLANG_PATH'
    }
}

# Tools the just recipes call.
$tools = @(
    @('just', 'just', 'cargo install just, or: winget install Casey.Just'),
    @('cargo-nextest', 'cargo-nextest', 'cargo install cargo-nextest, or: winget install nextest.cargo-nextest'),
    @('cargo-deny', 'cargo-deny', 'cargo install cargo-deny')
)
foreach ($t in $tools) {
    $path = Find-Tool $t[1]
    if ($path) { Write-Check $t[0] 'ok' $path } else { Write-Check $t[0] 'FAIL' ('not found. Install: {0}' -f $t[2]) }
}

# Optional extras.
if (Find-Tool 'git') { Write-Check 'git' 'ok' (Get-FirstLine 'git' @('--version')) }
else { Write-Check 'git' 'warn' 'not found (optional, for version control)' }
$code = Find-Tool 'code'
if ($code) {
    $extensions = & $code --list-extensions 2>$null
    if ($extensions -contains 'aardappel.goose-language') {
        Write-Check 'VS Code Goose' 'ok' 'extension installed'
    } else {
        Write-Check 'VS Code Goose' 'warn' 'extension missing. Install: code --install-extension "<goose folder>\vscode\goose-language.vsix"'
    }
}

Write-Host ''
if ($script:failures -gt 0) {
    Write-Host ('{0} problem(s) to fix before `just ci` can pass.' -f $script:failures) -ForegroundColor Red
    exit 1
}
if ($script:warnings -gt 0) {
    Write-Host ('Ready, with {0} warning(s).' -f $script:warnings) -ForegroundColor Yellow
} else {
    Write-Host 'Everything is ready.' -ForegroundColor Green
}
exit 0
