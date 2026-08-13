//! 설치된 글꼴 조회 (FR-48).
//!
//! 파일시스템 열거(`fs`)도 화면(`ui`)도 아닌 **앱 환경 조회**라 여기에 둔다
//! (`app::theme`이 `DwmSetWindowAttribute`를 쓰는 것과 같은 자리).
//!
//! 글꼴 파일을 직접 찾지 않고 GDI에 묻는다 — 레지스트리의 글꼴 항목은 표시 이름과
//! 다르게 적혀 있고(`맑은 고딕 & 맑은 고딕 Semilight (TrueType)`), 사용자별 설치 글꼴과
//! 패키지 글꼴은 경로 규칙이 또 다르다. 이름에서 글꼴을 찾는 일은 OS가 이미 하고 있다 (D3).
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateFontIndirectW, DEFAULT_PITCH, DeleteDC, DeleteObject, ENUMLOGFONTEXW,
    EnumFontFamiliesExW, FF_DONTCARE, GDI_ERROR, GetFontData, GetTextFaceW, HANGUL_CHARSET, HDC,
    LOGFONTW, SelectObject, TEXTMETRICW,
};

/// `LOGFONTW.lfFaceName`의 칸 수 — Win32가 정한 상한(널 종단 포함 32)
const FACE_NAME_LEN: usize = 32;

/// 설치된 글꼴 중 **한글을 그릴 수 있는 것**의 이름을 가나다순으로 (FR-48).
///
/// 한글 문자셋으로 거르는 이유: 전체를 보이면 한글 글리프가 없는 글꼴(`Wingdings` 등)을
/// 고를 수 있고, 그 순간 파일명과 화면 문구가 통째로 두부(□)가 된다. 되돌리려면
/// 깨진 화면에서 다시 골라야 한다 (사용자 결정).
///
/// **여기서 돌려준 이름은 모두 `load_font`로 읽을 수 있어야 한다** — 목록에 있는데
/// 고르면 실패하는 항목은 사용자에게 고장으로 보인다. 그래서 실제로 읽어 확인한 것만 남긴다
pub fn installed_korean_fonts() -> Vec<String> {
    let mut names = enumerate_hangul_faces();
    names.sort();
    names.dedup();
    names.retain(|name| load_font(name).is_some());
    names
}

/// 그 글꼴의 원본 바이트 (`.ttf`/`.otf` 그대로). 읽지 못하면 `None`.
///
/// **모음 글꼴(TTC)도 특별 취급하지 않는다** — GDI가 모음에서도 단일 sfnt를 뽑아 주는 것을
/// 실측으로 확인했다(2026-08-13: 굴림·굴림체·바탕·돋움 모두 `ttcf` 테이블 없음,
/// 데이터 매직 `0x00010000`). 그래서 face 인덱스를 다룰 일이 없다
pub fn load_font(name: &str) -> Option<Vec<u8>> {
    if name.is_empty() {
        return None;
    }
    // 안전성: DC·글꼴 핸들을 이 함수 안에서만 만들고 나가기 전에 반드시 되돌려 놓는다.
    // 중간에 실패해도 그 지점까지 만든 것만 정리하고 나간다(아래 각 분기 참조)
    unsafe {
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let font = CreateFontIndirectW(&face_request(name));
        if font.is_invalid() {
            let _ = DeleteDC(hdc);
            return None;
        }
        let previous = SelectObject(hdc, font.into());

        let bytes = read_selected_font(hdc, name);

        SelectObject(hdc, previous);
        let _ = DeleteObject(font.into());
        let _ = DeleteDC(hdc);
        bytes
    }
}

/// 이름만 지정한 글꼴 요청 — 나머지는 기본값(크기·굵기는 바이트를 읽는 데 상관없다)
fn face_request(name: &str) -> LOGFONTW {
    let mut logfont = LOGFONTW {
        lfCharSet: HANGUL_CHARSET,
        lfPitchAndFamily: DEFAULT_PITCH.0 | FF_DONTCARE.0,
        ..Default::default()
    };
    // 널 종단 자리를 남긴다 — 넘치는 이름은 어차피 GDI가 찾지 못한다
    for (slot, ch) in logfont
        .lfFaceName
        .iter_mut()
        .take(FACE_NAME_LEN - 1)
        .zip(name.encode_utf16())
    {
        *slot = ch;
    }
    logfont
}

/// DC에 선택된 글꼴의 바이트를 읽는다 — **요청한 이름이 실제로 선택됐을 때만**.
///
/// # 안전성
/// 호출자가 유효한 DC에 글꼴을 선택해 둔 상태여야 한다. 이 함수는 핸들을 만들거나 지우지 않는다.
///
/// 이름 대조가 핵심이다: `CreateFontIndirectW`는 없는 이름에 **오류를 내지 않고 가장 비슷한
/// 글꼴로 조용히 대체**한다(실측 2026-08-13: `없는글꼴이름XYZ` → `굴림`, 데이터 크기까지 동일).
/// 대조하지 않으면 저장된 글꼴이 삭제된 뒤 엉뚱한 글꼴이 성공처럼 적용돼,
/// 기본 글꼴로 되돌아가야 할 자리(FR-48)가 조용히 죽는다
unsafe fn read_selected_font(hdc: HDC, requested: &str) -> Option<Vec<u8>> {
    unsafe {
        let mut face = [0u16; FACE_NAME_LEN];
        let taken = GetTextFaceW(hdc, Some(&mut face));
        if taken <= 0 {
            return None;
        }
        // 반환값은 널 종단을 포함한 글자 수다
        let selected = String::from_utf16_lossy(&face[..(taken as usize).saturating_sub(1)]);
        if selected != requested {
            return None;
        }

        let size = GetFontData(hdc, 0, 0, None, 0);
        if size == GDI_ERROR as u32 || size == 0 {
            return None;
        }
        let mut bytes = vec![0u8; size as usize];
        let read = GetFontData(hdc, 0, 0, Some(bytes.as_mut_ptr().cast()), size);
        (read == size).then_some(bytes)
    }
}

/// 한글 문자셋을 가진 글꼴 이름을 모은다 (중복·정렬은 부르는 쪽이 한다)
fn enumerate_hangul_faces() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    // 안전성: DC는 이 함수 안에서 만들고 지운다. 콜백은 열거가 끝나기 전에만 불리므로
    // `names`를 가리키는 포인터가 그 사이에 무효가 되지 않는다
    unsafe {
        let hdc = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return names;
        }
        let request = LOGFONTW {
            lfCharSet: HANGUL_CHARSET,
            lfPitchAndFamily: DEFAULT_PITCH.0 | FF_DONTCARE.0,
            ..Default::default()
        };
        EnumFontFamiliesExW(
            hdc,
            &request,
            Some(collect_face),
            LPARAM(&mut names as *mut Vec<String> as isize),
            0,
        );
        let _ = DeleteDC(hdc);
    }
    names
}

/// `EnumFontFamiliesExW` 콜백 — 글꼴 하나마다 불린다.
///
/// # 안전성
/// `lparam`은 `enumerate_hangul_faces`가 넘긴 `&mut Vec<String>`이어야 하고,
/// 그 열거가 도는 동안에만 불린다
unsafe extern "system" fn collect_face(
    logfont: *const LOGFONTW,
    _metric: *const TEXTMETRICW,
    _font_type: u32,
    lparam: LPARAM,
) -> i32 {
    unsafe {
        let (Some(logfont), Some(names)) = (
            logfont.cast::<ENUMLOGFONTEXW>().as_ref(),
            (lparam.0 as *mut Vec<String>).as_mut(),
        ) else {
            // 계속 열거한다 — 항목 하나를 못 읽었다고 나머지를 버릴 이유가 없다
            return 1;
        };
        let face = &logfont.elfLogFont.lfFaceName;
        let len = face.iter().position(|&ch| ch == 0).unwrap_or(face.len());
        let name = String::from_utf16_lossy(&face[..len]);
        // `@`로 시작하는 것은 세로쓰기용 이름이다 — 같은 글꼴이 두 벌로 보이고
        // 가로쓰기 화면에서 고를 이유가 없다
        if !name.is_empty() && !name.starts_with('@') {
            names.push(name);
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 한글_글꼴만_목록에_담긴다() {
        let fonts = installed_korean_fonts();
        assert!(
            fonts.iter().any(|name| name == "맑은 고딕"),
            "맑은 고딕이 없다: {fonts:?}"
        );
        // 모음 글꼴(gulim.ttc)도 남아야 한다 — 빼면 선택지가 맑은 고딕 계열만 남는다
        assert!(fonts.iter().any(|name| name == "굴림"), "굴림이 없다");
        for excluded in ["Wingdings", "Webdings", "Marlett"] {
            assert!(
                !fonts.iter().any(|name| name == excluded),
                "한글이 없는 {excluded}이 목록에 있다"
            );
        }
        assert!(
            !fonts.iter().any(|name| name.starts_with('@')),
            "세로쓰기 이름이 남아 있다"
        );
    }

    #[test]
    fn 목록은_정렬되고_중복이_없다() {
        let fonts = installed_korean_fonts();
        let mut sorted = fonts.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(fonts, sorted, "목록이 정렬돼 있지 않거나 중복이 있다");
    }

    #[test]
    fn 목록의_모든_이름은_바이트를_얻을_수_있다() {
        // 목록에 있는데 읽지 못하는 이름이 있으면 고르는 순간 실패한다.
        //
        // **여기까지가 이 계층의 몫이다** — 그 바이트가 실제로 글꼴로 파싱되는지는
        // egui에 등록해 봐야 알 수 있고(실측: `D2Coding`은 읽히지만 폭이 0), `app`은
        // `ui`를 모르므로 그 검증은 T5가 화면 쪽에서 한다
        let fonts = installed_korean_fonts();
        assert!(!fonts.is_empty(), "한글 글꼴이 하나도 없다");
        for name in &fonts {
            assert!(load_font(name).is_some(), "{name}을 읽지 못했다");
        }
    }

    #[test]
    fn 없는_글꼴은_대체되지_않고_실패한다() {
        // GDI는 없는 이름을 **오류 없이 비슷한 글꼴로 바꿔치기**한다(실측: 굴림으로 대체).
        // 그대로 두면 지워진 글꼴이 다른 글꼴로 조용히 되살아나 기본값 폴백이 죽는다
        assert_eq!(load_font("없는글꼴이름XYZ"), None);
        assert_eq!(load_font(""), None);
    }
}
