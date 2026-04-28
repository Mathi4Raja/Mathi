param(
    [ValidateSet('debug', 'release')]
    [string]$Profile = 'release'
)

$ErrorActionPreference = 'Stop'

function Require-Command {
    param([string]$Name, [string]$InstallHint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Error "Missing required command: $Name. $InstallHint"
    }
}

Require-Command cargo "Install Rust from https://rustup.rs"

Push-Location (Join-Path $PSScriptRoot '..\src-tauri')
try {
    if ($Profile -eq 'release') {
        Write-Host 'Building release bundles with Tauri...'
        cargo tauri build
    }
    else {
        Write-Host 'Building debug bundle artifacts with Tauri...'
        cargo tauri build --debug
    }
}
finally {
    Pop-Location
}
