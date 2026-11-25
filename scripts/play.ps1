# ZJOU-VICTORY launcher: runs the game for normal play (no scripted presses, no capture).
#
# Performance switches for this build (see README):
#   VICTORY_TEXTURE_NO_WAIT=1   read title memory through the kernel while the
#                               runtime remaps it instead of waiting: the world's
#                               load waited 123 ms a frame there
#   VICTORY_FRAME_INTERVAL      the title paces itself at 33.3 ms (30 fps); 16667
#                               gives 60 fps of headroom once frames get shorter
#   VICTORY_VSYNC=0             no 60 Hz present wait (a test switch; the screen
#                               may tear)
#   VICTORY_DSTORAGE_NOTIFY=1   DirectStorage fills the title's physical pages
#                               through a view of its own; the host's copies of
#                               those addresses are told the bytes (otherwise the
#                               first room's walls can read stale copies)
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
$env:VICTORY_TEXTURE_NO_WAIT = '1'
$env:VICTORY_FRAME_INTERVAL = '16667'
$env:VICTORY_VSYNC = '0'
$env:VICTORY_DSTORAGE_NOTIFY = '1'
& $Exe run $Game --execute 2>&1 | Out-File "$env:TEMP\ZJOU-VICTORY.log" -Encoding utf8
Write-Host "Exited with $LASTEXITCODE; log: $env:TEMP\ZJOU-VICTORY.log"
