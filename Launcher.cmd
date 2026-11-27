@echo off
rem ============================================================
rem  GTA VI (Xbox) PC Launcher - victory-scarlett-3070 "scarlett-fox"
rem  RTX 4080 Laptop 12GB verified combo (run8/run9 tuning series)
rem  Final: BUFFERS=1024 ROOM=85 BUDGET=6144 MIP_DROP=1 30fps tier
rem ============================================================
setlocal EnableDelayedExpansion
title GTA VI Xbox->PC Launcher
set "EXE=%~dp0victory.exe"
set "GAME=C:\GTAVI\GTA6.Xbox.v1.0.2.1"
set "LOGDIR=%~dp0logs"
if not exist "%LOGDIR%" mkdir "%LOGDIR%"

:menu
cls
echo  ============================================================
echo    GTA VI (Xbox) -^> PC  Launcher
echo    fork: victory-scarlett-3070 (scarlett-fox)
echo    verified: 21-30 fps on RTX 4080 Laptop 12GB
echo  ------------------------------------------------------------
echo    1. Start game            (shader cache hot, ~10s to menu)
echo    2. First-time prewarm    (~21 min, only run once ever)
echo    3. Show key params
echo    4. Toggle game dir       (current: %GAME%)
echo    5. Open usage notes
echo    6. Exit
echo  ============================================================
set /p CHOICE=Select: 
if "%CHOICE%"=="1" goto start
if "%CHOICE%"=="2" goto prewarm
if "%CHOICE%"=="3" goto params
if "%CHOICE%"=="4" goto gamedir
if "%CHOICE%"=="5" start notepad "%~dp0USAGE.txt" & goto menu
if "%CHOICE%"=="6" exit /b 0
goto menu

:start
for /f "tokens=1-4 delims=/- " %%a in ("%date%") do set TS=%%a%%b%%c
for /f "tokens=1 delims=:. " %%a in ("%time: =0%") do set TS=!TS!%%a
echo Launching with %GAME% ...
call "%~dp0Start-4080-GTA6.cmd"
echo.
echo Session ended.
pause
goto menu

:prewarm
echo Prewarm shaders, about 21 minutes, one time only ...
"%EXE%" shaders "%GAME%"
"%EXE%" warm
echo Prewarm done.
pause
goto menu

:params
echo  --- final verified combo ---
echo  VICTORY_FRAME_INTERVAL_MS  = 33.3      (30 fps tier)
echo  VICTORY_CARD_BUFFERS_MB    = 1024      (512 causes GPU copy fragmentation stall)
echo  VICTORY_TEXTURE_BUDGET_MB  = 6144      (prevents VRAM over-budget eviction = silent crash)
echo  VICTORY_TEXTURE_ROOM_PERCENT = 85      (80: black blocks / 90: fps drop / 85: balanced)
echo  VICTORY_TEXTURE_MIP_DROP   = 1
echo  others: fork 3070 baseline, see CONFIG.md section 3.4
pause
goto menu

:gamedir
set /p GAME=New game folder path (the one with game_scarlett_master.exe): 
echo Game dir set to: %GAME%
pause
goto menu
