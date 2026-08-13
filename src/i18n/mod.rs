//! 화면 문구 카탈로그와 현재 언어 (FR-53).
//!
//! **이 모듈은 아무 모듈도 참조하지 않는다** — `ui`·`remote`·`fs` 어디서나 부르므로
//! 최상위에 두고 의존을 한 방향으로 유지한다(AGENTS 계층 규약).
//!
//! 문구는 [`strings!`] 매크로에 한 줄씩 적고, 매크로가 그것을 함수로 펼친다.
//! 키가 곧 함수 이름이라 **오타는 컴파일 오류**이고, 한·영 둘 중 하나를 빠뜨려도
//! 마찬가지다 — 화면을 띄워 봐야 아는 실수를 컴파일러가 먼저 잡는다.
//!
//! 문구에 값이 끼어드는 것(`"폴더 3 파일 12"`)은 매크로가 아니라 **손수 쓴 함수**로 둔다
//! (아래 `mod dynamic`) — 조사·어순·복수형이 언어마다 달라 자리표시자로는 담기지 않는다.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험은 전역 언어를 건드리므로 **끝나면 되돌린다** — 병렬로 도는 다른 시험이
    /// 남은 값을 보면 엉뚱한 언어로 단언하게 된다
    struct Restore(Language);

    impl Restore {
        fn now() -> Restore {
            Restore(current())
        }
    }

    impl Drop for Restore {
        fn drop(&mut self) {
            let setting = match self.0 {
                Language::Korean => LanguageSetting::Korean,
                Language::English => LanguageSetting::English,
            };
            set_language(setting);
        }
    }

    #[test]
    fn 언어에_따라_다른_문구를_돌려준다() {
        let _restore = Restore::now();
        set_language(LanguageSetting::Korean);
        assert_eq!(settings_title(), "설정");
        set_language(LanguageSetting::English);
        assert_eq!(settings_title(), "Settings");
    }

    #[test]
    fn 시스템_기본은_윈도우_ui_언어를_따른다() {
        let _restore = Restore::now();
        set_language(LanguageSetting::System);
        assert_eq!(
            current(),
            system_language(),
            "시스템 기본인데 시스템 언어와 다르다"
        );
    }

    #[test]
    fn 알_수_없는_값은_한국어로_읽는다() {
        // 이 앱의 화면이 원래 한국어다 — 모르는 값에 영어를 주면 갑자기 화면이 바뀐다
        let _restore = Restore::now();
        CURRENT.store(99, Ordering::Relaxed);
        assert_eq!(current(), Language::Korean);
    }
}
