@echo off
chcp 65001 >nul
title Cai LabControl cho may hoc sinh
rem ---------------------------------------------------------------------------
rem  Chuot phai -> Run as administrator. Nhap MAT KHAU PHONG (xem tren may giao vien:
rem  LabControl > Cai dat > Phong & ghep may).
rem
rem  Cai hang loat / khong hoi:
rem     cai-may-hoc-sinh.cmd <mat-khau-phong> [ten-phong] [ip-may-giao-vien]
rem ---------------------------------------------------------------------------
fltmc >nul 2>&1 || (echo   Hay chuot phai file nay, chon "Run as administrator". & pause & exit /b 1)
set "PW=%~1"
set "ROOM=%~2"
set "TIP=%~3"
if "%PW%"=="" (
  echo.
  echo   CAI LABCONTROL CHO MAY HOC SINH
  echo   May nay se tu tim va noi ve may giao vien trong mang LAN.
  echo.
  set /p PW=  Mat khau phong: 
  set /p ROOM=  Ten phong ^(bo trong neu mang chi co mot phong^): 
)
if "%PW%"=="" (echo   Chua nhap mat khau phong. & pause & exit /b 1)
"%~dp0labcontrol-agent.exe" install --password "%PW%" --room "%ROOM%" --teacher-ip "%TIP%"
if errorlevel 1 (echo. & echo   Cai dat that bai, xem thong bao phia tren. & if "%~1"=="" pause & exit /b 1)
echo.
echo   XONG. Dang xuat roi dang nhap lai (hoac khoi dong lai may).
echo   May se tu hien tren man hinh cua giao vien.
if "%~1"=="" pause
