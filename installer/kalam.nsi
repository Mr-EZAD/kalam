; Kalam Installer Script
; نیاز به NSIS 3.x دارد: https://nsis.sourceforge.io/

!include "MUI2.nsh"
!include "LogicLib.nsh"

; ── تنظیمات اصلی ─────────────────────────────────────────────────────────────
Name "Kalam"
OutFile "KalamSetup.exe"
InstallDir "$PROGRAMFILES64\Kalam"
InstallDirRegKey HKCU "Software\Kalam" "InstallDir"
RequestExecutionLevel admin
Unicode True

; ── آیکون ────────────────────────────────────────────────────────────────────
!define MUI_ICON "..\assets\kalam.ico"
!define MUI_UNICON "..\assets\kalam.ico"

; ── صفحات ────────────────────────────────────────────────────────────────────
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Farsi"

; ── نصب ──────────────────────────────────────────────────────────────────────
Section "Kalam" SecMain
    SectionIn RO

    SetOutPath "$INSTDIR"
    File "..\target\release\kalam.exe"
    File "..\assets\kalam.ico"
    File "..\assets\kalam-16.ico"

    ; Registry برای Add/Remove Programs
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "DisplayName" "Kalam - Keyboard Layout Converter"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "UninstallString" "$INSTDIR\uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "DisplayIcon" "$INSTDIR\kalam.ico"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "Publisher" "YOUR_NAME"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "DisplayVersion" "1.0.0"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "URLInfoAbout" "https://github.com/YOUR_USERNAME/kalam"
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "NoModify" 1
    WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam" \
        "NoRepair" 1

    ; ذخیره مسیر نصب
    WriteRegStr HKCU "Software\Kalam" "InstallDir" "$INSTDIR"

    ; Uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Start Menu shortcut
    CreateDirectory "$SMPROGRAMS\Kalam"
    CreateShortcut "$SMPROGRAMS\Kalam\Kalam.lnk" "$INSTDIR\kalam.exe" "" "$INSTDIR\kalam.ico"
    CreateShortcut "$SMPROGRAMS\Kalam\Uninstall Kalam.lnk" "$INSTDIR\uninstall.exe"

    ; اجرا بعد از نصب
    Exec '"$INSTDIR\kalam.exe"'
SectionEnd

; ── حذف ──────────────────────────────────────────────────────────────────────
Section "Uninstall"
    ; بستن برنامه اگه باز باشه
    ExecWait 'taskkill /F /IM kalam.exe'
    Sleep 500

    ; حذف فایل‌ها
    Delete "$INSTDIR\kalam.exe"
    Delete "$INSTDIR\kalam.ico"
    Delete "$INSTDIR\kalam-16.ico"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    ; حذف shortcuts
    Delete "$SMPROGRAMS\Kalam\Kalam.lnk"
    Delete "$SMPROGRAMS\Kalam\Uninstall Kalam.lnk"
    RMDir "$SMPROGRAMS\Kalam"

    ; حذف registry
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Kalam"
    DeleteRegKey HKCU "Software\Kalam"

    ; حذف autostart اگه set شده
    DeleteRegValue HKCU "SOFTWARE\Microsoft\Windows\CurrentVersion\Run" "Kalam"
SectionEnd