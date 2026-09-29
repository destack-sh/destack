Unicode True
RequestExecutionLevel user
SetCompressor /SOLID lzma
CRCCheck force
Name "${TITLE}"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\${TITLE}"
InstallDirRegKey HKCU "Software\Destack\${IDENTITY}" "InstallLocation"
ShowInstDetails show
ShowUninstDetails show

!include "MUI2.nsh"
!include "LogicLib.nsh"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\Destack.exe"
!insertmacro MUI_PAGE_FINISH
!ifdef UNINSTALLER_ONLY
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!endif
!insertmacro MUI_LANGUAGE "English"

Section "Install"
!ifdef UNINSTALLER_ONLY
    ; produce an uninstaller for platform signing before embedding it in Setup
    WriteUninstaller "$EXEDIR\Uninstall.exe"
    Quit
!else
    ; install the shared Evergreen runtime when Microsoft's documented registry entries are absent
    SetRegView 32
    ReadRegStr $0 HKLM "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    ${If} $0 == ""
    ${OrIf} $0 == "0.0.0.0"
        ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    ${EndIf}
    ${If} $0 == ""
    ${OrIf} $0 == "0.0.0.0"
        InitPluginsDir
        SetOutPath "$PLUGINSDIR"
        File /oname=WebView2.exe "${WEBVIEW}"
        ClearErrors
        ExecWait '"$PLUGINSDIR\WebView2.exe" /silent /install' $0
        ${If} ${Errors}
        ${OrIf} $0 != 0
            MessageBox MB_OK|MB_ICONSTOP "Microsoft WebView2 could not install. Check your internet connection and run Setup again." /SD IDOK
            SetErrorLevel 1
            Abort
        ${EndIf}
    ${EndIf}

    ; stop and unregister the previous package before replacing its executables
    SetShellVarContext current
    IfFileExists "$INSTDIR\helpers\destack.exe" 0 install
    ClearErrors
    ExecWait '"$INSTDIR\helpers\destack.exe" uninstall --native' $0
    ${If} ${Errors}
    ${OrIf} $0 != 0
        MessageBox MB_OK|MB_ICONSTOP "Could not stop the existing Destack installation." /SD IDOK
        SetErrorLevel 1
        Abort
    ${EndIf}

    ; install the complete desktop, CLI, daemon and sandbox payload
    install:
    SetOutPath "$INSTDIR"
    SetOverwrite on
    ClearErrors
    File /r "${PAYLOAD}\*"
    File /oname=Uninstall.exe "${UNINSTALLER}"
    ${If} ${Errors}
        MessageBox MB_OK|MB_ICONSTOP "Could not extract the complete Destack application." /SD IDOK
        SetErrorLevel 1
        Abort
    ${EndIf}
    ClearErrors
    ExecWait '"$INSTDIR\helpers\destack.exe" install' $0
    ${If} ${Errors}
    ${OrIf} $0 != 0
        MessageBox MB_OK|MB_ICONSTOP "Destack registration failed. Run Setup again to repair the installation." /SD IDOK
        SetErrorLevel 1
        Abort
    ${EndIf}

    ; publish the native shortcut and Windows installed-app entry
    CreateShortcut "$SMPROGRAMS\${TITLE}.lnk" "$INSTDIR\Destack.exe"
    WriteRegStr HKCU "Software\Destack\${IDENTITY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "DisplayName" "${TITLE}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "Publisher" "Symbol Industries GmbH"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "DisplayIcon" "$INSTDIR\Destack.exe"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "NoModify" 1
    WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}" "NoRepair" 1
!endif
SectionEnd

!ifdef UNINSTALLER_ONLY
Section "Uninstall"
    ; let the CLI stop processes and remove only its executable state and user registration
    SetShellVarContext current
    ClearErrors
    ExecWait '"$INSTDIR\helpers\destack.exe" uninstall --native' $0
    ${If} ${Errors}
    ${OrIf} $0 != 0
        MessageBox MB_OK|MB_ICONSTOP "Destack could not stop. No application files were removed." /SD IDOK
        SetErrorLevel 1
        Abort
    ${EndIf}

    ; remove package files while preserving databases and checkouts outside the installation
    Delete "$SMPROGRAMS\${TITLE}.lnk"
    Delete "$INSTDIR\Destack.exe"
    Delete "$INSTDIR\installation.json"
    RMDir /r "$INSTDIR\helpers"
    RMDir /r "$INSTDIR\view"
    Delete "$INSTDIR\Uninstall.exe"
    RMDir "$INSTDIR"
    DeleteRegKey HKCU "Software\Destack\${IDENTITY}"
    DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${IDENTITY}"
SectionEnd
!endif
