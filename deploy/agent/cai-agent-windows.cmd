@echo off
chcp 65001 >nul
title Cai LabControl Agent (may hoc sinh)
rem Chuot phai -> Run as administrator. Dan "ma giao vien" (LCT-...) lay trong Console > Cai dat.
rem Cai hang loat:  cai-agent-windows.cmd LCT-xxxx [TenPhong]
fltmc >nul 2>&1 || (echo   Hay chuot phai file nay, chon "Run as administrator". & pause & exit /b 1)
set "CODE=%~1"
if "%CODE%"=="" set /p CODE=  Dan ma giao vien LCT-... roi bam Enter: 
set "ROOM=%~2"
"%~dp0labcontrol-agent.exe" install --teacher %CODE% --room "%ROOM%"
if errorlevel 1 (echo. & echo   Cai dat that bai, xem thong bao phia tren. & if "%~1"=="" pause & exit /b 1)
echo.
echo   XONG. Dang xuat roi dang nhap lai (hoac khoi dong lai may) de Agent bat dau chay.
if "%~1"=="" pause
