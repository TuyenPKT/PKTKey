# =============================================================================
# build-win.ps1 - Build PKTKey IME for Windows (TSF DLL)
# Chay tren Windows voi PowerShell (as Administrator de regsvr32).
# Usage: .\scripts\build-win.ps1 [-Install] [-InstallDir "C:\PKTKey"]
# =============================================================================
param(
    [switch]$Install,
    [string]$InstallDir = "C:\PKTKey"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot  = Split-Path -Parent $PSScriptRoot
$ImeDir    = Join-Path $RepoRoot "platforms\windows\pktkey-ime"
$Target    = "x86_64-pc-windows-msvc"
$OutDll    = Join-Path $ImeDir "target\$Target\release\pktkey_ime.dll"

# -- 1. Dam bao MSVC target da duoc cai --------------------------------------
Write-Host ">> rustup target add $Target"
rustup target add $Target

# -- 2. Build release DLL ----------------------------------------------------
Write-Host ">> cargo build (release, $Target)..."
Push-Location $ImeDir
try {
    cargo build --release --target $Target
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed (exit $LASTEXITCODE)"
    }
} finally {
    Pop-Location
}
Write-Host "  [ok] $OutDll"

# -- 3. (Optional) Copy + register -------------------------------------------
if ($Install) {
    $IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $IsAdmin) {
        Write-Error "Can chay PowerShell as Administrator de regsvr32."
        exit 1
    }

    Write-Host ">> Copy DLL -> $InstallDir"
    if (-not (Test-Path $InstallDir)) { New-Item -ItemType Directory -Path $InstallDir | Out-Null }
    $DestDll = Join-Path $InstallDir "pktkey_ime.dll"
    Copy-Item $OutDll $DestDll -Force

    Write-Host ">> regsvr32 $DestDll"
    regsvr32 /s $DestDll
    Write-Host "  [ok] Da dang ky IME."
    Write-Host ""
    Write-Host "Buoc tiep:"
    Write-Host "  1. Vao Settings > Time & Language > Language & Region"
    Write-Host "  2. Them 'Vietnamese' (neu chua co)"
    Write-Host "  3. Chon PKTKey Vietnamese IME lam input method"
    Write-Host "  Toggle Vi/En: Ctrl+Space"
} else {
    Write-Host ""
    Write-Host "[done] Build xong. De cai dat, chay lai voi -Install (as Administrator):"
    Write-Host "   .\scripts\build-win.ps1 -Install"
    Write-Host "   .\scripts\build-win.ps1 -Install -InstallDir 'D:\Apps\PKTKey'"
}
