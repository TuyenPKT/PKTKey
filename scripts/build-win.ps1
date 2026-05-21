# ─────────────────────────────────────────────────────────────────────────────
# build-win.ps1 — Build PKTKey IME for Windows (TSF DLL)
# Chạy trên Windows với PowerShell (as Administrator để regsvr32).
# Usage: .\scripts\build-win.ps1 [-Install] [-InstallDir "C:\PKTKey"]
# ─────────────────────────────────────────────────────────────────────────────
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

# ── 1. Đảm bảo MSVC target đã được cài ──────────────────────────────────
Write-Host "▶ rustup target add $Target"
rustup target add $Target

# ── 2. Build release DLL ─────────────────────────────────────────────────
Write-Host "▶ cargo build (release, $Target)..."
Push-Location $ImeDir
try {
    cargo build --release --target $Target
} finally {
    Pop-Location
}
Write-Host "  ✓ $OutDll"

# ── 3. (Optional) Copy + register ────────────────────────────────────────
if ($Install) {
    $IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $IsAdmin) {
        Write-Error "Cần chạy PowerShell as Administrator để regsvr32."
        exit 1
    }

    Write-Host "▶ Copy DLL → $InstallDir"
    if (-not (Test-Path $InstallDir)) { New-Item -ItemType Directory -Path $InstallDir | Out-Null }
    $DestDll = Join-Path $InstallDir "pktkey_ime.dll"
    Copy-Item $OutDll $DestDll -Force

    Write-Host "▶ regsvr32 $DestDll"
    regsvr32 /s $DestDll
    Write-Host "  ✓ Đã đăng ký IME."
    Write-Host ""
    Write-Host "Bước tiếp:"
    Write-Host "  1. Vào Settings > Time & Language > Language & Region"
    Write-Host "  2. Thêm 'Vietnamese' (nếu chưa có)"
    Write-Host "  3. Chọn PKTKey Vietnamese IME làm input method"
    Write-Host "  Toggle Vi/En: Ctrl+Space"
} else {
    Write-Host ""
    Write-Host "✅ Build xong. Để cài đặt, chạy lại với -Install (as Administrator):"
    Write-Host "   .\scripts\build-win.ps1 -Install"
    Write-Host "   .\scripts\build-win.ps1 -Install -InstallDir 'D:\Apps\PKTKey'"
}
