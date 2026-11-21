# Commands for the template itself. A generated project has its own justfile.

set shell := ["pwsh", "-NoLogo", "-NoProfile", "-Command"]

# List the template's commands
default:
    @just --list --unsorted

# Create a new project in the folder you ran this from: just new my-app
new name *flags:
    pwsh -NoLogo -NoProfile -File new-project.ps1 {{name}} -Path '{{invocation_directory()}}' {{flags}}

# Add the rgnew command to your PowerShell profile
install:
    pwsh -NoLogo -NoProfile -File install.ps1

# Generate a throwaway project and run its full `just ci` (checks the template end to end)
selftest:
    pwsh -NoLogo -NoProfile -File scripts/selftest.ps1
