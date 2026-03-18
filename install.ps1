# install.ps1 — builds release binaries and creates shortcuts.
# Run once from PowerShell: .\install.ps1

$ErrorActionPreference = "Stop"

$projectDir  = $PSScriptRoot
$trackerExe  = Join-Path $projectDir "target\release\tracker.exe"
$uiExe       = Join-Path $projectDir "target\release\ui.exe"
$startupDir  = [Environment]::GetFolderPath("Startup")
$desktopDir  = [Environment]::GetFolderPath("Desktop")

Write-Host "Building release binaries..."
Push-Location $projectDir
cargo build --release
Pop-Location

Write-Host "Creating startup shortcut for tracker..."
$shell    = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut("$startupDir\GameTracker.lnk")
$shortcut.TargetPath       = $trackerExe
$shortcut.WorkingDirectory = $projectDir
$shortcut.Description      = "Game Tracker (background)"
$shortcut.Save()

Write-Host "Creating desktop shortcut for UI..."
$shortcut2 = $shell.CreateShortcut("$desktopDir\Game Tracker.lnk")
$shortcut2.TargetPath       = $uiExe
$shortcut2.WorkingDirectory = $projectDir
$shortcut2.Description      = "Game Tracker UI"
$shortcut2.Save()

Write-Host ""
Write-Host "Done. The tracker will start automatically at next login."
Write-Host "Use the 'Game Tracker' icon on your desktop to open the UI."
