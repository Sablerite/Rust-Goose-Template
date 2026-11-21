<#
.SYNOPSIS
Adds the `rgnew` command to your PowerShell profile.

.DESCRIPTION
Writes a small, clearly marked block into the profile of the PowerShell
that runs this script (the profile for all hosts, so it also works in the
VS Code and Windows Terminal shells). The block defines
New-RustGooseProject and its short alias rgnew, which run this template's
new-project.ps1 from wherever you are. It also saves tab completion for
`goose` and `just`, unless -NoCompletions is given.

Run it again after moving the template folder. -Uninstall removes the
block and the saved completion files.

.EXAMPLE
pwsh -File install.ps1

.EXAMPLE
pwsh -File install.ps1 -Uninstall
#>
[CmdletBinding()]
param(
    [switch]$Uninstall,
    [switch]$NoCompletions
)

$ErrorActionPreference = 'Stop'
$begin = '# >>> rust-goose-template >>>'
$end = '# <<< rust-goose-template <<<'
$profilePath = $PROFILE.CurrentUserAllHosts
$completionDir = Join-Path $HOME '.config\rust-goose-template'
$utf8NoBom = New-Object System.Text.UTF8Encoding $false

# The profile without any block this script added before.
$existing = ''
if (Test-Path $profilePath) { $existing = [System.IO.File]::ReadAllText($profilePath) }
$pattern = '(?s)\r?\n?' + [regex]::Escape($begin) + '.*?' + [regex]::Escape($end) + '\r?\n?'
$cleaned = [regex]::Replace($existing, $pattern, "`n").TrimEnd()

if ($Uninstall) {
    if ($cleaned) { $cleaned += "`n" }
    if (Test-Path $profilePath) { [System.IO.File]::WriteAllText($profilePath, $cleaned, $utf8NoBom) }
    if (Test-Path $completionDir) { Remove-Item -Recurse -Force $completionDir }
    Write-Host ('Removed rgnew from {0}. Open a new shell for it to take effect.' -f $profilePath) -ForegroundColor Green
    exit 0
}

function ConvertTo-Quoted([string]$Text) {
    return "'" + $Text.Replace("'", "''") + "'"
}

$generator = Join-Path $PSScriptRoot 'new-project.ps1'
$lines = @(
    $begin,
    ('# Added by {0}. Remove with: install.ps1 -Uninstall' -f (Join-Path $PSScriptRoot 'install.ps1')),
    ('function New-RustGooseProject {{ & {0} @args }}' -f (ConvertTo-Quoted $generator)),
    'Set-Alias -Name rgnew -Value New-RustGooseProject'
)

if (-not $NoCompletions) {
    New-Item -ItemType Directory -Force -Path $completionDir | Out-Null
    foreach ($tool in @(@('goose', 'completion', 'powershell'), @('just', '--completions', 'powershell'))) {
        if (Get-Command $tool[0] -ErrorAction SilentlyContinue) {
            $file = Join-Path $completionDir ($tool[0] + '.ps1')
            $script = (& $tool[0] $tool[1] $tool[2]) -join "`n"
            [System.IO.File]::WriteAllText($file, $script + "`n", $utf8NoBom)
            $lines += ('if (Test-Path {0}) {{ . {0} }}' -f (ConvertTo-Quoted $file))
        }
    }
}
$lines += $end

$profileDir = Split-Path -Parent $profilePath
if (-not (Test-Path $profileDir)) { New-Item -ItemType Directory -Force -Path $profileDir | Out-Null }
$newProfile = $cleaned
if ($newProfile) { $newProfile += "`n`n" }
$newProfile += ($lines -join "`n") + "`n"
[System.IO.File]::WriteAllText($profilePath, $newProfile, $utf8NoBom)

Write-Host ('Added rgnew to {0}.' -f $profilePath) -ForegroundColor Green
Write-Host 'Open a new shell, then run: rgnew my-app'
