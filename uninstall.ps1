# uninstall.ps1 — removes shortcuts and stops the tracker.
# Does NOT delete your config.json or playtime.json data.
# Run from PowerShell: .\uninstall.ps1

$startupDir = [Environment]::GetFolderPath("Startup")
$desktopDir = [Environment]::GetFolderPath("Desktop")

Write-Host "Stopping tracker if running..."
Stop-Process -Name "tracker" -Force -ErrorAction SilentlyContinue

Write-Host "Removing startup shortcut..."
$startupLink = "$startupDir\GameTracker.lnk"
if (Test-Path $startupLink) { Remove-Item $startupLink }

Write-Host "Removing desktop shortcut..."
$desktopLink = "$desktopDir\Game Tracker.lnk"
if (Test-Path $desktopLink) { Remove-Item $desktopLink }

Write-Host ""
Write-Host "Uninstalled. Your config.json and playtime.json data have been kept."
