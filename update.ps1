# update.ps1 — rebuilds release binaries and restarts the tracker.
# The desktop and startup shortcuts already point to target\release\,
# so no need to touch them — just rebuild and relaunch.
# Run from PowerShell: .\update.ps1

$ErrorActionPreference = "Stop"

$projectDir = $PSScriptRoot
$trackerExe = Join-Path $projectDir "target\release\tracker.exe"

Write-Host "Stopping tracker if running..."
Stop-Process -Name "tracker" -Force -ErrorAction SilentlyContinue

Write-Host "Building release binaries..."
Push-Location $projectDir
cargo build --release
Pop-Location

Write-Host "Restarting tracker..."
Start-Process $trackerExe

Write-Host ""
Write-Host "Update complete. Tracker is running in the background."
