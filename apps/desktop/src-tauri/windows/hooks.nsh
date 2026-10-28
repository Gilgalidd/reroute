; Reroute's additions to the NSIS installer that Tauri generates
; (bundle.windows.nsis.installerHooks in tauri.conf.json).

; Running in the background, Reroute starts when the user signs in through a
; value in the user's Run key (crates/platform/src/autostart/windows.rs).
; Remove it with the program, or Windows would try to start the removed
; program at every sign-in. A later Reroute that stays in the background
; writes it again.
!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Reroute"
!macroend
