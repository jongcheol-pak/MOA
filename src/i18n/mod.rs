//! 화면 문구 카탈로그와 현재 언어 (FR-53).
//!
//! **이 모듈은 화면 계층을 참조하지 않는다** — `ui`·`remote`·`fs` 어디서나 부르므로
//! 최상위에 두고 의존을 한 방향으로 유지한다(AGENTS 계층 규약). 참조하는 것은
//! `app::settings::LanguageSetting` 하나뿐이며, 그것은 저장 값을 담는 순수 데이터라
//! 이쪽을 되참조하지 않는다
//!
//! 문구는 [`strings!`] 매크로에 한 줄씩 적고, 매크로가 그것을 함수로 펼친다.
//! 키가 곧 함수 이름이라 **오타는 컴파일 오류**이고, 한·영 둘 중 하나를 빠뜨려도
//! 마찬가지다 — 화면을 띄워 봐야 아는 실수를 컴파일러가 먼저 잡는다.
//!
//! 문구에 값이 끼어드는 것(`"폴더 3 파일 12"`)은 매크로가 아니라 **손수 쓴 함수**로 둘 것이다
//! — 조사·어순·복수형이 언어마다 달라 자리표시자로는 담기지 않는다. 그 함수들은 문구를
//! 옮기는 task가 들어올 때 이 파일에 함께 들어온다(아직 없다).
use std::sync::atomic::{AtomicU8, Ordering};

use crate::app::settings::LanguageSetting;

/// 화면에 실제로 쓰이는 언어 — `시스템 기본`은 시작할 때 이 둘 중 하나로 풀린다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Korean,
    English,
}

/// 현재 언어. **잠금 없이 스레드 경계를 넘는다** — 화면은 UI 스레드가 그리지만
/// 원격 워커도 오류 문구를 만든다(`remote::connection`)
static CURRENT: AtomicU8 = AtomicU8::new(KOREAN);

const KOREAN: u8 = 0;
const ENGLISH: u8 = 1;

/// 지금 쓰는 언어. 매 프레임 수백 번 불리므로 원자적 읽기 하나로 끝낸다
pub fn current() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        ENGLISH => Language::English,
        // 알 수 없는 값은 한국어로 — 이 앱의 화면이 원래 한국어다
        _ => Language::Korean,
    }
}

/// 설정 값을 받아 현재 언어를 정한다. `시스템 기본`은 **여기서 한 번** 풀어 둔다.
///
/// 문구를 만들 때마다 Win32에 묻지 않는 이유: 시스템 UI 언어는 앱이 도는 동안 바뀌지
/// 않는다(바꾸려면 로그아웃해야 한다) — 매 프레임 수백 번 부를 값이 아니다 (D5)
pub fn set_language(setting: LanguageSetting) {
    let language = match setting {
        LanguageSetting::Korean => Language::Korean,
        LanguageSetting::English => Language::English,
        LanguageSetting::System => system_language(),
    };
    let value = match language {
        Language::Korean => KOREAN,
        Language::English => ENGLISH,
    };
    CURRENT.store(value, Ordering::Relaxed);
}

/// Windows 사용자 UI 언어가 한국어인가.
///
/// 하위 10비트가 언어를 가리키고 상위 6비트가 지역이다 — `ko-KR`뿐 아니라 다른 한국어
/// 변종도 함께 잡으려면 하위 10비트만 본다
pub fn system_language() -> Language {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    /// `LANG_KOREAN` — winnt.h의 값
    const LANG_KOREAN: u16 = 0x12;
    // 안전성: 인자도 반환 자원도 없는 순수 조회다
    let id = unsafe { GetUserDefaultUILanguage() };
    // 실패(0)면 한국어로 — 이 앱의 기존 화면이 한국어이므로 그것이 안전한 쪽이다
    if id == 0 || id & 0x3FF == LANG_KOREAN {
        Language::Korean
    } else {
        Language::English
    }
}

/// 정적 문구를 함수로 펼친다.
///
/// 한 줄이 `키 => "한국어" / "English"` 형태이고, 매크로는 그 줄마다
/// `pub fn 키() -> &'static str`을 만든다. **하는 일은 이 펼침 하나**이며 조건·분기·
/// 타입 조작이 없다 — 반복을 줄이는 것이지 간접화를 더하는 것이 아니다.
macro_rules! strings {
    ($($(#[$doc:meta])* $name:ident => $ko:literal / $en:literal;)*) => {
        $(
            $(#[$doc])*
            pub fn $name() -> &'static str {
                match current() {
                    Language::Korean => $ko,
                    Language::English => $en,
                }
            }
        )*
    };
}

strings! {
    /// 설정 대화 제목
    settings_title => "설정" / "Settings";
    /// 설정 대화의 그룹 제목 — 모양
    settings_group_appearance => "모양" / "Appearance";
    settings_group_startup => "시작" / "Startup";
    settings_group_exit => "종료" / "Exit";
    settings_group_files => "파일 보기" / "Files";
    settings_group_language => "언어" / "Language";
    settings_font => "글꼴" / "Font";
    /// 글꼴을 고르지 않은 상태 — 목록 맨 앞 항목이기도 하다
    settings_font_default => "기본값 (맑은 고딕)" / "Default (Malgun Gothic)";
    settings_font_scanning => "글꼴 목록을 읽는 중…" / "Reading font list…";
    settings_auto_start => "윈도우 시작 시 실행" / "Run at Windows startup";
    settings_auto_start_failed
        => "시작 프로그램 설정을 바꾸지 못했습니다"
        / "Could not change the startup setting";
    settings_tray_on_close => "닫으면 트레이로 보내기" / "Send to tray on close";
    settings_show_extensions => "파일 확장명" / "File name extensions";
    settings_show_hidden => "숨김 항목" / "Hidden items";
    /// 언어 선택지 — `English`는 두 언어에서 같다
    settings_language_system => "시스템 기본" / "System default";
    settings_language_korean => "한국어" / "Korean";
    settings_language_english => "English" / "English";
    close => "닫기" / "Close";
    delete => "삭제" / "Delete";
    rename => "이름 바꾸기" / "Rename";
    connect => "연결" / "Connect";

    // ── 타이틀바 (FR-22) ──
    titlebar_show_workspaces => "워크스페이스 목록 보이기" / "Show workspace list";
    titlebar_hide_workspaces => "워크스페이스 목록 숨기기" / "Hide workspace list";
    titlebar_restore => "이전 크기로" / "Restore";
    titlebar_maximize => "최대화" / "Maximize";
    titlebar_minimize => "최소화" / "Minimize";
    /// 설정 메뉴의 나머지 넷 — 아직 비활성이다
    titlebar_updates => "업데이트" / "Updates";
    titlebar_release_notes => "릴리즈 노트" / "Release notes";
    titlebar_licenses => "오픈소스 라이선스" / "Open source licenses";
    titlebar_about => "정보" / "About";

    // ── 패널 메뉴 (FR-23) ──
    /// 열 메뉴 캡션 (인벤토리 #22)
    menu_columns => "표시할 컬럼" / "Columns";
    menu_view => "보기" / "View";
    menu_refresh => "새로 고침" / "Refresh";
    menu_new_file => "새 파일" / "New file";
    menu_new_folder => "새 폴더" / "New folder";
    menu_split_right => "오른쪽 분할" / "Split right";
    menu_split_left => "왼쪽 분할" / "Split left";
    menu_split_up => "위쪽 분할" / "Split up";
    menu_split_down => "아래쪽 분할" / "Split down";

    // ── 보기 모드 8종 (FR-23) ──
    view_extra_large_icons => "아주 큰 아이콘" / "Extra large icons";
    view_large_icons => "큰 아이콘" / "Large icons";
    view_medium_icons => "보통 아이콘" / "Medium icons";
    view_small_icons => "작은 아이콘" / "Small icons";
    view_list => "목록" / "List";
    view_details => "자세히" / "Details";
    view_tiles => "타일" / "Tiles";
    view_content => "내용" / "Content";

    // ── 워크스페이스 사이드바 (FR-20) ──
    sidebar_workspaces => "워크스페이스" / "Workspaces";
    sidebar_saved_sites => "등록된 사이트" / "Saved sites";
    sidebar_add_site => "새 사이트 추가…" / "Add site…";
    sidebar_new_workspace => "새 워크스페이스" / "New workspace";
    sidebar_refresh_sites => "사이트 목록 새로 고침" / "Refresh site list";

    // ── 탭 스트립·주소창 ──
    tabs_new => "새 탭" / "New tab";
    tabs_close => "탭 닫기" / "Close tab";
    tabs_menu => "메뉴" / "Menu";
    address_back => "뒤로" / "Back";
    address_forward => "앞으로" / "Forward";
    address_up => "상위 폴더" / "Up";

    // ── 사이트 드롭다운 (FR-27) ──
    /// 사이트 드롭다운 캡션 (인벤토리 #92)
    site_dropdown_open => "연결 사이트를 새 탭으로" / "Open site in a new tab";
    site_dropdown_other => "다른 사이트로 새 탭 열기" / "Open another site in a new tab";

    // ── 폴더 트리 (FR-9) ──
    /// 아직 읽는 중인 노드의 자리 표시 — 로컬·원격 트리가 같은 문구를 쓴다
    tree_loading => "읽는 중…" / "Loading…";

    // ── 트레이 메뉴 (FR-50) ──
    tray_show => "실행" / "Open";
    tray_quit => "종료" / "Quit";

    /// 설정 대화의 언어 항목 라벨
    settings_language_label => "앱 언어" / "App language";
}

/// 값이 끼어드는 문구 — 조사·어순이 언어마다 달라 자리표시자로 담기지 않는다 (D2).
///
/// 정적 문구와 달리 매크로로 펼치지 않는다: 인자 개수·순서를 컴파일러가 검사해야 하고,
/// 언어마다 문장을 통째로 다시 쓸 수 있어야 한다
pub mod dynamic {
    use super::{Language, current};

    /// 사이트를 등록한 뒤 뜨는 알림 (FR-27)
    pub fn site_registered(host: &str) -> String {
        match current() {
            Language::Korean => format!("{host} 등록됨 · 더블클릭하여 연결"),
            Language::English => format!("{host} added · double-click to connect"),
        }
    }
}

/// 전역 언어를 건드리는 시험끼리 겹치지 않게 한다.
///
/// `cargo test`는 시험을 **여러 스레드에서 동시에** 돌린다 — 값을 끝에 되돌리는
/// 것만으로는 부족하다: 한 시험이 영어로 두고 단언하는 그 찰나에 다른 시험이
/// 값을 바꾸면 엉뚱한 언어를 읽는다. 그래서 **본문이 도는 동안 잠근다**
#[cfg(test)]
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 언어를 바꾸는 시험이 드는 가드 — 들고 있는 동안 다른 시험이 끼어들지 못하고,
/// 떨어질 때 시험 전의 언어로 되돌아간다.
///
/// **문구를 쓰는 모든 모듈의 시험이 이것을 쓴다** — `ui`·`remote`의 시험도 화면 문구를
/// 단언하려면 언어를 정해야 하고, 그때 잠그지 않으면 서로를 흔든다.
///
/// 둘째 자리를 읽는 코드는 없다 — **들고 있는 것 자체가 하는 일**이다
#[cfg(test)]
pub struct LanguageGuard(
    Language,
    #[allow(dead_code)] std::sync::MutexGuard<'static, ()>,
);

#[cfg(test)]
impl LanguageGuard {
    /// 언어를 고정하고 가드를 든다. 가드가 떨어지면 원래 언어로 돌아간다
    pub fn lock(setting: LanguageSetting) -> LanguageGuard {
        // 앞선 시험이 단언에 실패해 잠금이 오염됐어도 이어서 돈다 —
        // 우리가 지키는 것은 `AtomicU8` 하나이고 그 값은 곧 덮인다
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = current();
        set_language(setting);
        LanguageGuard(before, guard)
    }
}

#[cfg(test)]
impl Drop for LanguageGuard {
    fn drop(&mut self) {
        let setting = match self.0 {
            Language::Korean => LanguageSetting::Korean,
            Language::English => LanguageSetting::English,
        };
        set_language(setting);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 언어에_따라_다른_문구를_돌려준다() {
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        assert_eq!(settings_title(), "설정");
        set_language(LanguageSetting::English);
        assert_eq!(settings_title(), "Settings");
    }

    #[test]
    fn 시스템_기본은_윈도우_ui_언어를_따른다() {
        let _guard = LanguageGuard::lock(LanguageSetting::System);
        assert_eq!(
            current(),
            system_language(),
            "시스템 기본인데 시스템 언어와 다르다"
        );
    }

    #[test]
    fn 알_수_없는_값은_한국어로_읽는다() {
        // 이 앱의 화면이 원래 한국어다 — 모르는 값에 영어를 주면 갑자기 화면이 바뀐다
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        CURRENT.store(99, Ordering::Relaxed);
        assert_eq!(current(), Language::Korean);
    }
}
