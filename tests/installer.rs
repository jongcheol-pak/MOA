//! 설치 스크립트 규약 검사 (`installer/moa.nsi`).
//!
//! 이 PC에는 `makensis`가 없어 설치 파일을 만들어 볼 수 없다 — 문법·실동작은 사람이 확인한다.
//! 대신 **요구 항목이 스크립트에서 사라지지 않았는지**를 여기서 기계로 막는다. 문자열이 있는지만
//! 보는 얕은 검사이지만, 이 회차에서 얻을 수 있는 유일한 회귀 검출이다.
//!
//! 단언 목록은 plan(`docs/plans/2026-08-21-nsis-installer.md`)의 T2 Acceptance ⓐ~ⓗ와 1:1로 짝을 이룬다.
use std::path::PathBuf;

/// 검사 대상 — 레포 안의 스크립트를 그대로 읽는다
fn script() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("installer")
        .join("moa.nsi");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: 읽지 못했다 — {error}", path.display()))
}

/// 섹션 하나만 잘라 낸다 — **구역을 나누지 않으면 검출력이 사라진다**.
/// 설치와 제거가 같은 토큰(`$SMPROGRAMS`·`$DESKTOP`·`"MOA"`)을 쓰기 때문에,
/// 파일 전체에서 찾으면 설치 쪽 코드가 통째로 사라져도 제거 쪽 문자열이 시험을 통과시킨다
fn section<'a>(text: &'a str, name: &str) -> &'a str {
    let start = text
        .split_once(&format!("Section \"{name}\""))
        .unwrap_or_else(|| panic!("`{name}` 구역이 없다"))
        .1;
    start
        .split_once("SectionEnd")
        .unwrap_or_else(|| panic!("`{name}` 구역이 닫히지 않았다"))
        .0
}

/// 그 조각이 없으면 어느 요구가 빠졌는지 이름으로 알린다
fn assert_has(text: &str, needle: &str, 요구: &str) {
    assert!(text.contains(needle), "{요구}이(가) 빠졌다: `{needle}`");
}

#[test]
fn 설치_스크립트는_사용자_단위로_설치한다() {
    let text = script();
    let 설치 = section(&text, "Install");

    // ⓐ 권한 상승 없이 사용자 폴더에 설치한다
    assert_has(&text, "RequestExecutionLevel user", "권한 상승 없는 설치");
    assert_has(
        &text,
        r#"InstallDir "$LOCALAPPDATA\Programs\MOA""#,
        "설치 경로",
    );

    // ⓑ 시작 메뉴 바로가기는 **설치 구역 안에서** 만들어져야 한다 —
    //    파일 전체를 보면 제거 구역의 `Delete "$SMPROGRAMS\..."`가 대신 걸린다
    assert_has(설치, "CreateDirectory \"$SMPROGRAMS", "시작 메뉴 폴더 생성");
    assert_has(
        설치,
        "CreateShortcut \"$SMPROGRAMS",
        "시작 메뉴 바로가기 생성",
    );
    assert_has(
        &text,
        r#"CreateShortcut "$DESKTOP\$(SHORTCUT_NAME).lnk""#,
        "바탕화면 바로가기 생성",
    );
    assert_has(
        &text,
        "MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut",
        "바탕화면 바로가기 체크박스",
    );

    // ⓔ 버전을 손으로 돌려도 빌드가 성립한다
    assert_has(&text, "!ifndef VERSION", "버전 기본값");

    // ⓖ 담는 파일·아이콘·산출 경로는 모두 `installer/` 기준 상대경로다.
    //    경로만 찾지 않고 **그 경로를 쓰는 명령까지** needle에 넣는다 — `..\LICENSE`는
    //    라이선스 페이지(`MUI_PAGE_LICENSE`)에도, `..\docs\AppIcon.ico`는 제거 아이콘
    //    (`MUI_UNICON`)에도 있어서, 경로만 찾으면 동봉이 빠져도 그쪽에 걸려 통과한다
    for (needle, 요구) in [
        (r#"File "..\target\release\moa.exe""#, "담을 실행 파일"),
        (r#"File "..\LICENSE""#, "동봉할 라이선스"),
        (
            r#"File "..\THIRD-PARTY-NOTICES.md""#,
            "동봉할 오픈소스 고지",
        ),
        (r#"MUI_ICON "..\docs\AppIcon.ico""#, "설치 프로그램 아이콘"),
        (
            r#"MUI_UNICON "..\docs\AppIcon.ico""#,
            "제거 프로그램 아이콘",
        ),
        (
            r#"OutFile "..\target\installer\MOA-Setup-${VERSION}.exe""#,
            "산출 경로",
        ),
        (
            r#"MUI_PAGE_LICENSE "..\LICENSE""#,
            "설치 중 보이는 라이선스 원문",
        ),
    ] {
        assert_has(&text, needle, 요구);
    }

    // ⓗ 바로가기 이름은 설치 언어를 따른다
    //    `"MOA"`는 `APP_NAME` 정의·레지스트리 값 이름에도 있어, 정의 줄 전체를 찾는다
    assert_has(
        &text,
        "LangString SHORTCUT_NAME ${LANG_KOREAN} \"모아\"",
        "한국어 바로가기 이름",
    );
    assert_has(
        &text,
        "LangString SHORTCUT_NAME ${LANG_ENGLISH} \"MOA\"",
        "영어 바로가기 이름",
    );
}

#[test]
fn 제거는_자동_실행과_설정_처리를_모두_담는다() {
    let text = script();
    let 제거 = section(&text, "Uninstall");

    // ⓑ 바로가기는 **두 이름 모두** 지운다 — 언인스톨러 언어가 설치 때와 갈릴 수 있다
    for needle in [
        r"$DESKTOP\모아.lnk",
        r"$DESKTOP\MOA.lnk",
        r"$SMPROGRAMS\모아\모아.lnk",
        r"$SMPROGRAMS\MOA\MOA.lnk",
    ] {
        assert_has(제거, needle, "바로가기 삭제");
    }
    assert_has(제거, r#"RMDir "$SMPROGRAMS\모아""#, "시작 메뉴 폴더 삭제");

    // ⓕ·ⓓ `RMDir`는 재귀가 아니라, 우리가 놓은 것을 **하나도 빠짐없이** 지워야 폴더가 걷힌다.
    //      하나라도 빠지면 제거 뒤 그 파일과 폴더가 남는다(설정·지문은 T1이 이 폴더에 둔다).
    //      `Delete`까지 needle에 넣는 이유: 경로만 찾으면 같은 구역의 자동 실행 비교 문자열
    //      (`StrCmp $0 '"$INSTDIR\${EXE_NAME}" --tray'`)에 걸려 삭제 줄이 사라져도 통과한다
    for (needle, 요구) in [
        (r#"Delete "$INSTDIR\${EXE_NAME}""#, "실행 파일 삭제"),
        (r#"Delete "$INSTDIR\LICENSE""#, "라이선스 삭제"),
        (
            r#"Delete "$INSTDIR\THIRD-PARTY-NOTICES.md""#,
            "오픈소스 고지 삭제",
        ),
        (r#"Delete "$INSTDIR\uninstall.exe""#, "언인스톨러 삭제"),
        (r#"Delete "$INSTDIR\settings.json""#, "설정 파일 삭제"),
        (r#"Delete "$INSTDIR\known_hosts.json""#, "지문 파일 삭제"),
    ] {
        assert_has(제거, needle, 요구);
    }

    // ⓓ **묻지 않는다** — 확인 대화가 있으면 사용자 결정과 어긋난다
    assert!(
        !제거.contains("MB_YESNO"),
        "제거가 무언가를 묻고 있다 — 설정은 묻지 않고 지우는 것이 결정이다(D11)"
    );

    // ⓕ 설치 폴더와 등록 정보를 걷는다
    assert_has(제거, r#"RMDir "$INSTDIR""#, "설치 폴더 삭제");
    assert_has(제거, "DeleteRegKey HKCU", "제거 항목 등록 삭제");

    // ⓒ 자동 실행 값은 **이 설치본을 가리킬 때만** 지운다
    assert_has(제거, "ReadRegStr $0 HKCU", "자동 실행 값 읽기");
    assert_has(제거, r#"--tray'"#, "자동 실행 값 비교");
    assert_has(제거, "DeleteRegValue HKCU", "자동 실행 값 삭제");
}
