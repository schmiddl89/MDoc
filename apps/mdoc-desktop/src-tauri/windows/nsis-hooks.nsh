; MDoc installer hooks + template page (nsDialogs).
; StrFunc is already included in Tauri's installer.nsi -- declare ${StrRep} only.
${StrRep}

; --- Template page (Page custom in installer.nsi) ---
Var MDocTemplatePath
Var MDocTemplateDialog
Var MDocTemplateLabel
Var MDocTemplateEdit
Var MDocTemplateBrowseBtn
Var MDocTemplateSkipCheck

!include LogicLib.nsh
!include nsDialogs.nsh

Function MDocInitDefaultTemplate
  ${If} $MDocTemplatePath != ""
    Return
  ${EndIf}
  ; Only set default when the file really exists (no Word -> empty, no error).
  IfFileExists "$APPDATA\Microsoft\Templates\Normal.dotm" 0 mdoc_init_tpl_done
    StrCpy $MDocTemplatePath "$APPDATA\Microsoft\Templates\Normal.dotm"
  mdoc_init_tpl_done:
FunctionEnd

Function MDocPageTemplate
  ; Silent/Passive: skip page, keep default if present.
  ${If} $PassiveMode = 1
    Call MDocInitDefaultTemplate
    Abort
  ${EndIf}

  !insertmacro MUI_HEADER_TEXT "Word-Vorlage" "Optional: .dotx / .dotm fuer Markdown-Einfuegen und Export"

  nsDialogs::Create 1018
  Pop $MDocTemplateDialog
  ${If} $MDocTemplateDialog == error
    Abort
  ${EndIf}

  Call MDocInitDefaultTemplate

  ${NSD_CreateLabel} 0 0 100% 36u "Pfad zur Word-Vorlage. Leer lassen oder ueberspringen, wenn Word nicht installiert ist - MDoc funktioniert trotzdem."
  Pop $MDocTemplateLabel

  ${NSD_CreateText} 0 44u -70u 12u $MDocTemplatePath
  Pop $MDocTemplateEdit

  ${NSD_CreateButton} -60u 42u 60u 14u "Durchsuchen..."
  Pop $MDocTemplateBrowseBtn
  ${NSD_OnClick} $MDocTemplateBrowseBtn MDocTemplateBrowseClick

  ${NSD_CreateCheckbox} 0 68u 100% 12u "Vorlage spaeter im MDoc-Editor festlegen"
  Pop $MDocTemplateSkipCheck
  ${If} $MDocTemplatePath == ""
    ${NSD_Check} $MDocTemplateSkipCheck
  ${EndIf}

  nsDialogs::Show
FunctionEnd

Function MDocTemplateBrowseClick
  nsDialogs::SelectFileDialog open "$APPDATA\Microsoft\Templates" "Word-Vorlage (*.dotx;*.dotm)|*.dotx;*.dotm|Word-Dokument (*.docx)|*.docx|Alle Dateien|*.*"
  Pop $0
  ${If} $0 != ""
    ${NSD_SetText} $MDocTemplateEdit $0
    ${NSD_Uncheck} $MDocTemplateSkipCheck
  ${EndIf}
FunctionEnd

Function MDocPageTemplateLeave
  ${NSD_GetState} $MDocTemplateSkipCheck $0
  ${If} $0 == ${BST_CHECKED}
    StrCpy $MDocTemplatePath ""
    Return
  ${EndIf}

  ${NSD_GetText} $MDocTemplateEdit $MDocTemplatePath
  ; Empty = ok (install without template)
  ${If} $MDocTemplatePath == ""
    Return
  ${EndIf}

  IfFileExists "$MDocTemplatePath" mdoc_tpl_ok 0
    MessageBox MB_ICONEXCLAMATION|MB_YESNO \
      "Die gewaehlte Datei wurde nicht gefunden:$\r$\n$MDocTemplatePath$\r$\n$\r$\nOhne Vorlage fortfahren?" \
      IDYES mdoc_tpl_clear
    Abort
  mdoc_tpl_clear:
    StrCpy $MDocTemplatePath ""
    Return
  mdoc_tpl_ok:
FunctionEnd

Function MDocWriteTemplatePath
  ${If} $MDocTemplatePath == ""
    Return
  ${EndIf}
  CreateDirectory "$APPDATA\MDoc"
  ClearErrors
  FileOpen $R1 "$APPDATA\MDoc\template-path.txt" w
  ${If} ${Errors}
    DetailPrint "Hinweis: Vorlagen-Pfad konnte nicht geschrieben werden (ignoriert)."
    Return
  ${EndIf}
  FileWrite $R1 "$MDocTemplatePath$\r$\n"
  FileClose $R1
  DetailPrint "Vorlagen-Pfad hinterlegt: $MDocTemplatePath"
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\word-addin\MdocWordAddIn.dll" 0 mdoc_skip_word_addin
    DetailPrint "Registriere MDoc Word-Add-in (HKCU)..."
    ; CodeBase: file:/// + slash path (backslashes break .NET LoadBehavior).
    ${StrRep} $R0 "$INSTDIR\word-addin\MdocWordAddIn.dll" "\" "/"
    StrCpy $R0 "file:///$R0"

    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}" "" "mdoc.Connect"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32" "" "mscoree.dll"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32" "ThreadingModel" "Both"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32" "Class" "MdocWordAddIn.Connect"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32" "Assembly" "MdocWordAddIn, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32" "RuntimeVersion" "v4.0.30319"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32" "CodeBase" "$R0"

    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32\1.0.0.0" "" "mscoree.dll"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32\1.0.0.0" "ThreadingModel" "Both"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32\1.0.0.0" "Class" "MdocWordAddIn.Connect"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32\1.0.0.0" "Assembly" "MdocWordAddIn, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32\1.0.0.0" "RuntimeVersion" "v4.0.30319"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\InprocServer32\1.0.0.0" "CodeBase" "$R0"

    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\ProgID" "" "mdoc.Connect"
    WriteRegStr HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}\Implemented Categories\{62C8FE65-4EBB-45e7-B440-6E39B2CDBF29}" "" ""

    WriteRegStr HKCU "Software\Classes\mdoc.Connect" "" "MDoc Word Add-in"
    WriteRegStr HKCU "Software\Classes\mdoc.Connect\CLSID" "" "{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}"

    WriteRegStr HKCU "Software\Microsoft\Office\Word\Addins\mdoc.Connect" "Description" "MDoc Markdown-Integration"
    WriteRegStr HKCU "Software\Microsoft\Office\Word\Addins\mdoc.Connect" "FriendlyName" "MDoc"
    WriteRegDWORD HKCU "Software\Microsoft\Office\Word\Addins\mdoc.Connect" "LoadBehavior" 3
  mdoc_skip_word_addin:

  ; Passive/Silent: default Normal.dotm if page was skipped.
  ${If} $PassiveMode = 1
  ${OrIf} ${Silent}
    Call MDocInitDefaultTemplate
  ${EndIf}
  Call MDocWriteTemplatePath
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Entferne MDoc Word-Add-in (HKCU)..."
  DeleteRegKey HKCU "Software\Microsoft\Office\Word\Addins\mdoc.Connect"
  DeleteRegKey HKCU "Software\Classes\mdoc.Connect"
  ; Legacy ProgId cleanup
  DeleteRegKey HKCU "Software\Microsoft\Office\Word\Addins\mdword.Connect"
  DeleteRegKey HKCU "Software\Classes\mdword.Connect"
  DeleteRegKey HKCU "Software\Classes\CLSID\{B7E4A912-6C31-4F8D-9E2A-55D0C1B3A874}"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  DetailPrint "Entferne MDoc-Benutzerdaten..."
  RMDir /r "$APPDATA\MDoc"
  RMDir /r "$APPDATA\mdoc"
  RMDir /r "$APPDATA\mdword"
  RMDir /r "$LOCALAPPDATA\MDoc"
  RMDir /r "$LOCALAPPDATA\mdoc"
  RMDir /r "$LOCALAPPDATA\mdword"
  IfFileExists "$INSTDIR\*.*" 0 mdoc_postun_done
    RMDir /r "$INSTDIR"
  mdoc_postun_done:
!macroend
