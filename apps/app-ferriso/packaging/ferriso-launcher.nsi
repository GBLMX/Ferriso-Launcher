Unicode true
!include "MUI2.nsh"
!define APPNAME "Ferriso Launcher"
!define APPID "com.ferriso.FerrisoLauncher"
!ifndef VERSION
  !define VERSION "0.1.0"
!endif
Name "${APPNAME}"
OutFile "Ferriso-Launcher-${VERSION}-x64-setup.exe"
InstallDir "$LOCALAPPDATA\Ferriso Launcher"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ICON "ferriso-launcher.ico"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\..\..\target\release\ferriso-launcher.exe"
  File "ferriso-launcher.ico"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Ferriso Launcher"
  CreateShortcut "$SMPROGRAMS\Ferriso Launcher\Ferriso Launcher.lnk" "$INSTDIR\ferriso-launcher.exe"
  CreateShortcut "$DESKTOP\Ferriso Launcher.lnk" "$INSTDIR\ferriso-launcher.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPID}" "DisplayName" "${APPNAME}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPID}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPID}" "UninstallString" "$INSTDIR\uninstall.exe"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\ferriso-launcher.exe"
  Delete "$INSTDIR\ferriso-launcher.ico"
  Delete "$INSTDIR\uninstall.exe"
  Delete "$SMPROGRAMS\Ferriso Launcher\Ferriso Launcher.lnk"
  RMDir "$SMPROGRAMS\Ferriso Launcher"
  Delete "$DESKTOP\Ferriso Launcher.lnk"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPID}"
  ; Keep %APPDATA%\ModrinthApp: accounts/instances are shared with Modrinth App.
SectionEnd
