@echo off
chcp 65001 >nul
title Go LabControl khoi may hoc sinh
rem Chuot phai -> Run as administrator.
fltmc >nul 2>&1 || (echo   Hay chuot phai file nay, chon "Run as administrator". & pause & exit /b 1)
set "EXE=%ProgramFiles%\LabControl\labcontrol-agent.exe"
if not exist "%EXE%" set "EXE=%~dp0labcontrol-agent.exe"
"%EXE%" uninstall
taskkill /f /im labcontrol-agent.exe >nul 2>&1
echo.
echo   Da go. Co the xoa thu muc "%ProgramFiles%\LabControl" va "%ProgramData%\LabControl".
pause
