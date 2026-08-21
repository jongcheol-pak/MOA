//! 설치 스크립트 규약 검사 (`installer/moa.nsi`).
//!
//! 이 PC에는 `makensis`가 없어 설치 파일을 만들어 볼 수 없다 — 문법·실동작은 사람이 확인한다.
//! 대신 **요구 항목이 스크립트에서 사라지지 않았는지**를 여기서 기계로 막는다. 문자열이 있는지만
//! 보는 얕은 검사이지만, 이 회차에서 얻을 수 있는 유일한 회귀 검출이다.
//!
//! 단언 목록은 plan(`docs/plans/2026-08-21-nsis-installer.md`)의 T2 Acceptance ⓐ~ⓖ와 1:1로 짝을 이룬다.
use std::path::PathBuf;

/// 검사 대상 — 레포 안의 스크립트를 그대로 읽는다
fn script() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("installer")
        .join("moa.nsi");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: 읽지 못했다 — {error}", path.display()))
}

/// 그 조각이 없으면 어느 요구가 빠졌는지 이름으로 알린다
fn assert_has(text: &str, needle: &str, 요구: &str) {
    assert!(text.contains(needle), "{요구}이(가) 빠졌다: `{needle}`");
}

#[test]
fn 설치는_사용자_단위로_이뤄진다() {
    let text = script();

    // ⓐ 권한 상승 없이 사용자 폴더에 설치한다
    assert_has(&text, "RequestExecutionLevel user", "권한 상승 없는 설치");
    assert_has(
        &text,
        r#"InstallDir "$LOCALAPPDATA\Programs\MOA""#,
        "설치 경로",
    );

    // ⓑ 시작 메뉴·바탕화면 바로가기를 만든다
    assert_has(&text, "$SMPROGRAMS", "시작 메뉴 바로가기");
    assert_has(&text, "$DESKTOP", "바탕화면 바로가기");
    assert_has(
        &text,
        "MUI_FINISHPAGE_SHOWREADME_FUNCTION CreateDesktopShortcut",
        "바탕화면 바로가기 체크박스",
    );

    // ⓔ 버전을 손으로 돌려도 빌드가 성립한다
    assert_has(&text, "!ifndef VERSION", "버전 기본값");

    // ⓖ 담는 파일·아이콘·산출 경로는 모두 `installer/` 기준 상대경로다
    for (needle, 요구) in [
        (r"..\target\release\moa.exe", "담을 실행 파일"),
        (r"..\LICENSE", "동봉할 라이선스"),
        (r"..\THIRD-PARTY-NOTICES.md", "동봉할 오픈소스 고지"),
        (r"..\docs\AppIcon.ico", "설치 프로그램 아이콘"),
        (r"..\target\installer\MOA-Setup-${VERSION}.exe", "산출 경로"),
    ] {
        assert_has(&text, needle, 요구);
    }

    // ⓗ 바로가기 이름은 설치 언어를 따른다
    assert_has(&text, "LangString SHORTCUT_NAME", "언어별 바로가기 이름");
    assert_has(&text, "\"모아\"", "한국어 바로가기 이름");
    assert_has(&text, "\"MOA\"", "영어 바로가기 이름");
}

#[test]
fn 제거는_설정까지_묻지_않고_걷어낸다() {
    let text = script();
    let 제거 = text
        .split_once("Section \"Uninstall\"")
        .expect("제거 구역이 없다")
        .1;

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

    // ⓓ 설정·지문 파일을 이름으로 지운다 (설치 폴더 안에 있다 — T1)
    assert_has(제거, r"$INSTDIR\settings.json", "설정 파일 삭제");
    assert_has(제거, r"$INSTDIR\known_hosts.json", "지문 파일 삭제");

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
