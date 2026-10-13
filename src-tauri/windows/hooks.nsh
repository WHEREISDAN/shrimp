; Installer hooks (bundle.windows.nsis.installerHooks).
;
; A new installation starts Shrimp at login, in the background (WIN-10): the
; same Run value that Settings › System › Launch at login writes. Updates leave
; the user's choice alone. Uninstalling removes the value.

Var ShrimpUpgrade

!macro NSIS_HOOK_PREINSTALL
  StrCpy $ShrimpUpgrade "0"
  IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 +2
    StrCpy $ShrimpUpgrade "1"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  StrCmp $ShrimpUpgrade "1" +2 0
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --background'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "${PRODUCTNAME}"
!macroend
