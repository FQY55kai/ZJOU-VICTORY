# ztrx launcher: runs the game for normal play (no scripted presses, no capture).
param(
    [Parameter(Mandatory = $true)][string]$Game,
    [string]$Cache = "$env:LOCALAPPDATA\ZJOU-VICTORY\cache",
    [string]$Data = "$env:LOCALAPPDATA\ZJOU-VICTORY\data",
    [string]$Exe = "$PSScriptRoot\..\target\release\victory.exe"
)
foreach ($v in "VICTORY_KEYS", "VICTORY_CAPTURE", "VICTORY_CAPTURE_FRAMES", "VICTORY_DUMP_TARGETS") {
    Remove-Item "Env:$v" -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Force $Cache, $Data | Out-Null
$env:VICTORY_CACHE_DIR = $Cache
$env:VICTORY_DATA_DIR = $Data
& $Exe run $Game --execute 2>&1 | Out-File "$env:TEMP\ZJOU-VICTORY.log" -Encoding utf8
Write-Host "Exited with $LASTEXITCODE; log: $env:TEMP\ZJOU-VICTORY.log"
