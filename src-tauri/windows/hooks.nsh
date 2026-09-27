; NSIS hooks of the Windows installer (see tauri.conf.json → bundle.windows.nsis).

; Uninstall: put Rocket League's files back to stock before the app goes
; away, so no swap, palette, decal, ball or map is left in the game. Skipped
; when the uninstaller runs as part of an update (/UPDATE).
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    DetailPrint "Restoring Rocket League's original files..."
    ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --restore-stock'
  ${EndIf}
!macroend
