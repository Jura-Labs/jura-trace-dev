; ── Jura Trace — NSIS installer hooks ──────────────────────────────
;
; Tauri v2 lets us inject snippets into the bundled NSIS installer
; via the bundle.windows.nsis.installerHooks config field. This file
; defines the four NSIS_HOOK_* macros Tauri will splice into the
; install / uninstall flow at the appropriate points.
;
; Two real bugs this addresses (both observed during the rc.21
; Windows VM smoke test on 2026-04-23):
;
; 1. Re-install over a running instance failed mid-extract with
;    "Error opening file for writing: ...\jura-sidecar.exe"
;    because the running sidecar process held the .exe locked.
;    Fix: PREINSTALL hook kills jura-trace.exe + jura-sidecar.exe
;    before any file write begins.
;
; 2. Uninstall left orphan jura-sidecar.exe processes running
;    when the Tauri app was force-killed before the uninstaller
;    ran.  Files in %LOCALAPPDATA%\Jura Trace stayed locked, and
;    the next install failed with the same write error.
;    Fix: PREUNINSTALL hook kills the same two processes before
;    file removal.
;
; taskkill ships with every supported Windows version (XP+) so no
; external dependency.  Stderr is suppressed because the kill is
; expected to fail when nothing is running — that's fine.
;
; Sleep after taskkill gives Windows ~500ms to release the file
; handles; without it, file ops can race with the kernel's
; cleanup and still hit the lock.

!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Stopping any running Jura Trace processes..."
  nsExec::Exec '"taskkill" /F /IM jura-trace.exe /T'
  Pop $0
  nsExec::Exec '"taskkill" /F /IM jura-sidecar.exe /T'
  Pop $0
  Sleep 500
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Stopping any running Jura Trace processes..."
  nsExec::Exec '"taskkill" /F /IM jura-trace.exe /T'
  Pop $0
  nsExec::Exec '"taskkill" /F /IM jura-sidecar.exe /T'
  Pop $0
  Sleep 500
!macroend

; PostInstall + PostUninstall left as no-ops; Tauri tolerates
; missing macros gracefully but defining them silences any future
; deprecation warning if the upstream tooling adds one.

!macro NSIS_HOOK_POSTINSTALL
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
