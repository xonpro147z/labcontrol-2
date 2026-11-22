@echo off
chcp 65001 >nul
title Mo tuong lua cho may giao vien LabControl
rem Chay mot lan tren may giao vien (chuot phai -> Run as administrator)
rem de Console nhan duoc tin hieu "toi o day" cua cac may hoc sinh (UDP 47901).
fltmc >nul 2>&1 || (echo   Hay chuot phai file nay, chon "Run as administrator". & pause & exit /b 1)
netsh advfirewall firewall delete rule name="LabControl Console" >nul 2>&1
netsh advfirewall firewall add rule name="LabControl Console" dir=in action=allow protocol=UDP localport=47901 enable=yes profile=any
echo.
echo   Da mo tuong lua. Mo LabControl de bat dau.
pause
