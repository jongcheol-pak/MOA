; MOA 설치 스크립트 (NSIS 3) — `cargo run --example gen_installer`가 makensis에 넘긴다.
;
; **사용자 단위 설치**다: `%LOCALAPPDATA%\Programs\MOA`에 놓아 UAC를 띄우지 않는다.
; 이 앱은 설정을 실행 파일 옆에, 자동 실행을 HKCU에 쓰므로 권한 모델이 사용자 단위와 맞는다.
;
; **경로는 모두 이 파일이 있는 폴더 기준**이다 — `gen_installer`가 makensis의 작업 디렉터리를
; `installer\`로 지정하고 부르므로, makensis가 스크립트 폴더로 옮기든 아니든 같은 자리를 가리킨다.

; 산출물을 유니코드 exe로 만든다
Unicode true

; **이 파일은 BOM 없는 UTF-8이다**(레포 규약 — AGENTS). makensis는 BOM이 없으면
; 시스템 코드페이지로 읽어 아래 한글 문구가 깨지므로, 빌더(`examples/gen_installer.rs`)가
; `/INPUTCHARSET UTF8`을 주며 부른다. 손으로 돌릴 때도 그 인자가 필요하다

; 버전은 `gen_installer`가 `/DVERSION`으로 넘긴다. 손으로 makensis를 돌려도 빌드가
; 성립하도록 기본값을 둔다 — 그때는 산출물 이름이 `MOA-Setup-0.0.0-dev.exe`가 된다
!ifndef VERSION
  !define VERSION "0.0.0-dev"
!endif

!define APP_NAME "MOA"
!define PUBLISHER "jongcheol-pak"
!define EXE_NAME "moa.exe"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\MOA"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"

Name "${APP_NAME} ${VERSION}"
OutFile "..\target\installer\MOA-Setup-${VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\MOA"
; 관리자 권한을 청하지 않는다 — 사용자 폴더에만 쓴다
RequestExecutionLevel user
SetCompressor /SOLID lzma

!include "MUI2.nsh"

!define MUI_ICON "..\docs\AppIcon.ico"
!define MUI_UNICON "..\docs\AppIcon.ico"
!define MUI_ABORTWARNING

; 설치 페이지 — 라이선스는 레포의 MIT 원문을 그대로 보인다
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE_NAME}"
Function CreateDesktopShortcut
  CreateShortcut "$DESKTOP\$(SHORTCUT_NAME).lnk" "$INSTDIR\${EXE_NAME}"
FunctionEnd

; 바탕화면 바로가기는 마침 페이지의 체크박스로 고른다(`SHOWREADME` 자리를 빌린다).
; **`MUI_PAGE_FINISH`보다 먼저 정의해야 한다** — NSIS 전처리기는 위에서 아래로 훑고,
; 그 매크로가 전개되는 줄에서 `!ifdef`로 이 정의들을 찾기 때문이다(뒤에 두면 체크박스가 통째로 빠진다).
; 문구는 `LangString`으로 둔다 — `..._TEXT_KOREAN` 같은 언어별 접미사는 MUI2가 읽지 않는다
!define MUI_FINISHPAGE_SHOWREADME ""
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
!define MUI_FINISHPAGE_SHOWREADME_TEXT "$(DESKTOP_SHORTCUT_TEXT)"
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; 한국어가 먼저다 — 첫 언어가 기본 선택이 된다
!insertmacro MUI_LANGUAGE "Korean"
!insertmacro MUI_LANGUAGE "English"
; 솔리드 압축이라 언어 대화 리소스를 앞쪽에 예약한다 — 예약하지 않으면 대화가 뜨기 전에
; 아카이브 전체를 풀어야 해 시작이 그만큼 늦다
!insertmacro MUI_RESERVEFILE_LANGDLL

; 바로가기 이름 — 설치 언어를 따른다 (2026-08-21 사용자 요청)
LangString SHORTCUT_NAME ${LANG_KOREAN} "모아"
LangString SHORTCUT_NAME ${LANG_ENGLISH} "MOA"
; 제거하면 설정까지 사라진다는 것을 **미리** 알린다 — 묻지 않고 지우기 때문이다(D11)
LangString REMOVE_NOTICE ${LANG_KOREAN} "제거하면 사이트 목록·저장한 비밀번호·서버 지문이 함께 지워집니다."
LangString REMOVE_NOTICE ${LANG_ENGLISH} "Uninstalling also deletes your site list, saved passwords, and server fingerprints."
LangString DESKTOP_SHORTCUT_TEXT ${LANG_KOREAN} "바탕화면에 바로가기 만들기"
LangString DESKTOP_SHORTCUT_TEXT ${LANG_ENGLISH} "Create a desktop shortcut"
; 실행 중이면 파일을 바꿀 수 없다 — 플러그인 없이 안내만 한다(D5)
LangString CLOSE_APP_NOTICE ${LANG_KOREAN} "MOA가 실행 중이면 먼저 닫아 주세요."
LangString CLOSE_APP_NOTICE ${LANG_ENGLISH} "Please close MOA before continuing."



Function .onInit
  !insertmacro MUI_LANGDLL_DISPLAY
  MessageBox MB_OK "$(CLOSE_APP_NOTICE)"
FunctionEnd

Section "Install"
  SetOutPath "$INSTDIR"
  File "..\target\release\moa.exe"
  File "..\LICENSE"
  File "..\THIRD-PARTY-NOTICES.md"

  CreateDirectory "$SMPROGRAMS\$(SHORTCUT_NAME)"
  CreateShortcut "$SMPROGRAMS\$(SHORTCUT_NAME)\$(SHORTCUT_NAME).lnk" "$INSTDIR\${EXE_NAME}"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; 「앱 및 기능」 목록에 뜨는 항목 — 사용자 단위 설치라 HKCU에 쓴다
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE_NAME}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1

  MessageBox MB_OK "$(REMOVE_NOTICE)"
SectionEnd

Function un.onInit
  !insertmacro MUI_UNGETLANGUAGE
FunctionEnd

Section "Uninstall"
  ; 바로가기는 **두 이름을 모두** 지운다 — 언인스톨러는 자기 언어를 새로 정하므로
  ; 설치 때 쓴 이름과 갈릴 수 있다. `Delete`는 없는 파일에 관대해 두 번 지워도 해가 없다
  Delete "$DESKTOP\모아.lnk"
  Delete "$DESKTOP\MOA.lnk"
  Delete "$SMPROGRAMS\모아\모아.lnk"
  Delete "$SMPROGRAMS\MOA\MOA.lnk"
  RMDir "$SMPROGRAMS\모아"
  RMDir "$SMPROGRAMS\MOA"

  ; 우리가 놓은 파일만 이름으로 지운다 — 런타임 생성물 둘을 함께 적는 이유는
  ; 설정이 이 폴더 안에 있기 때문이다(T1). 묻지 않고 지우는 것이 사용자 결정이다(D11)
  Delete "$INSTDIR\${EXE_NAME}"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD-PARTY-NOTICES.md"
  Delete "$INSTDIR\settings.json"
  Delete "$INSTDIR\known_hosts.json"
  Delete "$INSTDIR\uninstall.exe"
  ; 재귀로 지우지 않는다 — 사용자가 넣어 둔 파일이 있으면 그대로 남긴다
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "${UNINST_KEY}"

  ; 자동 실행은 **이 설치본을 가리킬 때만** 지운다 — 개발 빌드로 켜 둔 값이 있으면
  ; 그것까지 지우게 되기 때문이다. 앱이 쓰는 값 형식은 `"<exe 경로>" --tray`다
  ReadRegStr $0 HKCU "${RUN_KEY}" "MOA"
  StrCmp $0 '"$INSTDIR\${EXE_NAME}" --tray' 0 +2
    DeleteRegValue HKCU "${RUN_KEY}" "MOA"
SectionEnd
