@echo off
chcp 65001 >nul
title Mo tuong lua cho may giao vien LabControl
rem Bo cai LabControl da tu mo tuong lua. Chi chay file nay (chuot phai -> Run as administrator)
rem neu may hoc sinh khong tim thay may giao vien.
rem May giao vien can nhan: TCP 47900 (may hoc sinh noi vao) va UDP 47901 (may hoc sinh tim may giao vien).
fltmc >nul 2>&1 || (echo   Hay chuot phai file nay, chon "Run as administrator". & pause & exit /b 1)
netsh advfirewall firewall delete rule name="LabControl Console" >nul 2>&1
netsh advfirewall firewall delete rule name="LabControl Console TCP" >nul 2>&1
netsh advfirewall firewall delete rule name="LabControl Console UDP" >nul 2>&1
netsh advfirewall firewall add rule name="LabControl Console TCP" dir=in action=allow protocol=TCP localport=47900 enable=yes profile=any
netsh advfirewall firewall add rule name="LabControl Console UDP" dir=in action=allow protocol=UDP localport=47901 enable=yes profile=any
echo.
echo   Da mo tuong lua. Mo LabControl de bat dau.
pause
