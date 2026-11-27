@echo off
rem First time shader prewarm, takes about 21 minutes. Run once only.
setlocal
set VICTORY_GAME_DIR=C:\GTAVI\GTA6.Xbox.v1.0.2.1
"%~dp0victory.exe" shaders "%VICTORY_GAME_DIR%"
"%~dp0victory.exe" warm
echo Prewarm done. Use Start-4080-GTA6.cmd to launch.
pause
