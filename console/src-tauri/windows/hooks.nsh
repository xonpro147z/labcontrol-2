; Open the teacher machine firewall on install, remove on uninstall.
; Student machines connect to the teacher: TCP 47900 (session) and UDP 47901 (discovery).
!macro NSIS_HOOK_POSTINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="LabControl Console TCP"'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="LabControl Console UDP"'
  nsExec::Exec 'netsh advfirewall firewall add rule name="LabControl Console TCP" dir=in action=allow protocol=TCP localport=47900 enable=yes profile=any'
  nsExec::Exec 'netsh advfirewall firewall add rule name="LabControl Console UDP" dir=in action=allow protocol=UDP localport=47901 enable=yes profile=any'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::Exec 'netsh advfirewall firewall delete rule name="LabControl Console TCP"'
  nsExec::Exec 'netsh advfirewall firewall delete rule name="LabControl Console UDP"'
!macroend
