<#
.SYNOPSIS
Creates a new Rust + Goose project from this template.

.DESCRIPTION
Copies the template folder to a new folder named after the project and
replaces the placeholder name everywhere, in file contents and in file and
folder names. Then it pins the installed Rust and Goose versions, starts a
git repository with a first commit, and checks that every tool is present.

Works in PowerShell 7 and Windows PowerShell 5.1.

.PARAMETER Name
The project name: lowercase letters, digits and single hyphens, starting
with a letter (for example my-app). It becomes the folder name, the crate
names (my-app, my-app-core) and the prefix of the Goose exports (my_app_).

.PARAMETER Path
The folder to create the project in. Default: the current folder.

.PARAMETER NoGit
Skip `git init` and the first commit.

.PARAMETER SkipDoctor
Skip the tool check at the end.

.EXAMPLE
rgnew my-app

.EXAMPLE
rgnew my-app -Path D:\code
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Name,
    [string]$Path = (Get-Location).Path,
    [switch]$NoGit,
    [switch]$SkipDoctor
)

$ErrorActionPreference = 'Stop'
$templateDir = Join-Path $PSScriptRoot 'template'

function Stop-WithError([string]$Message) {
    Write-Host "error: $Message" -ForegroundColor Red
    exit 1
}

# --- The name ---------------------------------------------------------------

if ($Name -cnotmatch '^[a-z][a-z0-9]*(-[a-z0-9]+)*$') {
    Stop-WithError ("'{0}' is not a valid project name. Use lowercase letters, digits and single hyphens, starting with a letter (for example: my-app)." -f $Name)
}
if ($Name.Length -gt 48) {
    Stop-WithError 'the project name is longer than 48 characters.'
}
$snake = $Name.Replace('-', '_')
$title = ($Name.Split('-') | ForEach-Object { $_.Substring(0, 1).ToUpper() + $_.Substring(1) }) -join ' '

# Names that would break Rust, Goose, C or Windows somewhere in the project.
$reserved = @(
    # Rust keywords, strict and reserved
    'as', 'async', 'await', 'break', 'const', 'continue', 'crate', 'dyn', 'else', 'enum', 'extern',
    'false', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop', 'match', 'mod', 'move', 'mut', 'pub',
    'ref', 'return', 'self', 'static', 'struct', 'super', 'trait', 'true', 'type', 'unsafe', 'use',
    'where', 'while', 'abstract', 'become', 'box', 'do', 'final', 'gen', 'macro', 'override', 'priv',
    'try', 'typeof', 'unsized', 'virtual', 'yield',
    # Rust's own crates
    'core', 'std', 'alloc', 'test', 'proc_macro',
    # Goose keywords and the template's fixed names
    'import', 'namespace', 'export', 'simd', 'recursive', 'var', 'pad', 'reusable', 'guard', 'defer',
    'block', 'null', 'bool', 'varint', 'thread_fn', 'goose', 'kernels', 'bridge', 'main',
    'i8', 'i16', 'i32', 'i64', 'u8', 'u16', 'u32', 'u64', 'f32', 'f64',
    # Goose standard library modules
    'dictionary', 'vec', 'math', 'os', 'binary', 'base64', 'csv', 'json', 'regex', 'audio', 'gfx',
    'physics', 'ui', 'ffi', 'rng',
    # C keywords
    'auto', 'case', 'char', 'default', 'double', 'float', 'goto', 'int', 'long', 'register',
    'short', 'signed', 'sizeof', 'switch', 'typedef', 'union', 'unsigned', 'void', 'volatile',
    'inline', 'restrict',
    # Windows device names, which cannot be folder names
    'con', 'prn', 'aux', 'nul', 'com1', 'com2', 'com3', 'com4', 'com5', 'com6', 'com7', 'com8',
    'com9', 'lpt1', 'lpt2', 'lpt3', 'lpt4', 'lpt5', 'lpt6', 'lpt7', 'lpt8', 'lpt9'
)
if ($reserved -contains $snake -or $snake.StartsWith('gs_')) {
    Stop-WithError ("'{0}' is a name Rust, Goose, C or Windows reserves. Pick another name." -f $Name)
}

# --- The destination --------------------------------------------------------

if (-not (Test-Path $templateDir)) {
    Stop-WithError ("the template folder is missing: {0}" -f $templateDir)
}
if (-not (Test-Path $Path -PathType Container)) {
    Stop-WithError ("the folder '{0}' does not exist." -f $Path)
}
$dest = Join-Path (Resolve-Path $Path).Path $Name
if (Test-Path $dest) {
    Stop-WithError ("'{0}' already exists. Pick another name or folder." -f $dest)
}

# --- Copy with the name replaced --------------------------------------------

$textExtensions = @('.rs', '.toml', '.md', '.goose', '.build', '.json', '.ps1', '.txt', '.yml',
    '.yaml', '.lock', '.gitignore', '.gitattributes', '.editorconfig', '.goose-version')
$textNames = @('justfile')
# Build output and hidden folders (git, editor and tool state) are not copied,
# except the two hidden folders that belong to every project.
$keepHidden = @('.cargo', '.vscode')
$utf8NoBom = New-Object System.Text.UTF8Encoding $false

function Get-NewName([string]$Text) {
    return $Text.Replace('rgt-app', $Name).Replace('rgt_app', $snake)
}

function Get-NewText([string]$Text) {
    return $Text.Replace('Rgt App', $title).Replace('rgt-app', $Name).Replace('rgt_app', $snake)
}

$templateFull = (Resolve-Path $templateDir).Path.TrimEnd('\', '/')
$files = Get-ChildItem -Path $templateFull -Recurse -File -Force | Where-Object {
    $relative = $_.FullName.Substring($templateFull.Length + 1)
    $folders = @($relative -split '[\\/]' | Select-Object -SkipLast 1)
    -not ($folders | Where-Object { $_ -eq 'target' -or ($_.StartsWith('.') -and $keepHidden -notcontains $_) })
}

New-Item -ItemType Directory -Path $dest | Out-Null
foreach ($file in $files) {
    $relative = $file.FullName.Substring($templateFull.Length + 1)
    $target = Join-Path $dest (Get-NewName $relative)
    $targetDir = Split-Path -Parent $target
    if (-not (Test-Path $targetDir)) { New-Item -ItemType Directory -Path $targetDir -Force | Out-Null }
    $isText = ($textExtensions -contains $file.Extension.ToLower()) -or ($textNames -contains $file.Name)
    if ($isText) {
        $content = [System.IO.File]::ReadAllText($file.FullName, [System.Text.Encoding]::UTF8)
        [System.IO.File]::WriteAllText($target, (Get-NewText $content), $utf8NoBom)
    } else {
        Copy-Item -LiteralPath $file.FullName -Destination $target
    }
}

# --- Pin the installed toolchains -------------------------------------------

$rustVersion = $null
foreach ($probe in @(@('rustup', 'run', 'stable', 'rustc', '--version'), @('rustc', '--version'))) {
    try {
        $out = & $probe[0] $probe[1..($probe.Length - 1)] 2>$null | Select-Object -First 1
        if ($out -match 'rustc (\d+)\.(\d+)\.(\d+)') { $rustVersion = $matches; break }
    } catch { }
}
if ($rustVersion) {
    $full = '{0}.{1}.{2}' -f $rustVersion[1], $rustVersion[2], $rustVersion[3]
    $minor = '{0}.{1}' -f $rustVersion[1], $rustVersion[2]
    $toolchainFile = Join-Path $dest 'rust-toolchain.toml'
    $text = [System.IO.File]::ReadAllText($toolchainFile) -replace 'channel = "[^"]*"', ('channel = "{0}"' -f $full)
    [System.IO.File]::WriteAllText($toolchainFile, $text, $utf8NoBom)
    $cargoFile = Join-Path $dest 'Cargo.toml'
    $text = [System.IO.File]::ReadAllText($cargoFile) -replace 'rust-version = "[^"]*"', ('rust-version = "{0}"' -f $minor)
    [System.IO.File]::WriteAllText($cargoFile, $text, $utf8NoBom)
} else {
    Write-Host 'warning: Rust was not found, so rust-toolchain.toml keeps the template''s version.' -ForegroundColor Yellow
}

$goose = Get-Command goose -ErrorAction SilentlyContinue
if ($goose) {
    $gooseVersion = (& $goose.Source version | Select-Object -First 1).Trim()
    [System.IO.File]::WriteAllText((Join-Path $dest '.goose-version'), "$gooseVersion`n", $utf8NoBom)
} else {
    Write-Host 'warning: goose was not found, so .goose-version keeps the template''s version.' -ForegroundColor Yellow
}

# --- A clean slate ----------------------------------------------------------

# The new project is a product, not a copy of the template's development.
# The template's own history (its design notes, handoff notes, git history)
# is never copied: it lives outside template/, and hidden folders inside it
# are skipped. Here the few lines of the manual that mention the template
# are removed, so nothing in the project says where it came from.
$manual = Join-Path $dest 'docs\MANUAL.md'
if (Test-Path $manual) {
    $text = [System.IO.File]::ReadAllText($manual)
    $text = [regex]::Replace($text, '(?s)\n### Starting another project\n.*?(?=\n---\n)', '')
    $text = $text.Replace('When this template was' + "`n" + '  made, a typical run', 'A typical run')
    $text = $text.Replace(' (the template installer sets this up)', '')
    [System.IO.File]::WriteAllText($manual, $text, $utf8NoBom)
}

# The standard places for the project's own records, starting empty: design
# notes in docs/design/, and session handoff notes in .claude/handoff/.
$designDir = Join-Path $dest 'docs\design'
$handoffDir = Join-Path $dest '.claude\handoff'
New-Item -ItemType Directory -Force -Path $designDir, $handoffDir | Out-Null
[System.IO.File]::WriteAllText((Join-Path $designDir '.gitkeep'), '', $utf8NoBom)
[System.IO.File]::WriteAllText((Join-Path $handoffDir 'HANDOFF.md'), '', $utf8NoBom)

# --- Format -----------------------------------------------------------------

# The new name changes line lengths, so let rustfmt re-wrap the Rust code;
# otherwise a long name would fail the project's own format check.
# (Goose's formatter never moves line breaks, so the Goose code needs nothing.)
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Push-Location $dest
    try {
        cargo fmt --all 2>$null
        if ($LASTEXITCODE) { Write-Host 'warning: cargo fmt failed; run `just fmt` in the project.' -ForegroundColor Yellow }
    } finally {
        Pop-Location
    }
}

# --- Git --------------------------------------------------------------------

if (-not $NoGit) {
    if (Get-Command git -ErrorAction SilentlyContinue) {
        git -C $dest init -q -b main
        $userName = git -C $dest config user.name
        $userEmail = git -C $dest config user.email
        if ($userName -and $userEmail) {
            git -C $dest add -A
            git -C $dest commit -q -m 'Initial commit'
        } else {
            Write-Host 'note: git has no user name or email set, so there is no first commit yet.' -ForegroundColor Yellow
        }
    } else {
        Write-Host 'note: git was not found, so the project is not a git repository.' -ForegroundColor Yellow
    }
}

# --- Done -------------------------------------------------------------------

Write-Host ''
Write-Host ("Created {0} in {1}" -f $title, $dest) -ForegroundColor Green
if (-not $SkipDoctor) {
    Write-Host ''
    & (Join-Path $dest 'scripts\doctor.ps1')
}
Write-Host ''
Write-Host 'Next:'
Write-Host ('  cd "{0}"' -f $dest)
Write-Host '  just build      # the first build takes a minute or two'
Write-Host '  just run'
Write-Host '  just            # every command'
Write-Host '  The manual is in docs\MANUAL.md'
exit 0
