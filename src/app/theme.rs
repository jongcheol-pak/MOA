//! 다크 테마 — 탐색기 영역 고정 다크 스타일의 공용 색상·헬퍼 (plan T2).
//!
//! 전환 UI 없는 **고정 다크**다(사이드바 FR-15와 정합). 색 값은 사이드바(`sidebar.rs`)의
//! 사설 팔레트와 통일한다. 각 색은 실제로 쓰는 task에서 추가한다(YAGNI).
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};

/// COLORREF는 0x00BBGGRR 순서 — 실수 방지용 헬퍼
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// 창 배경·스플리터 틈 (사이드바 COLOR_BG와 동일)
pub const WINDOW_BG: COLORREF = rgb(0x1B, 0x1B, 0x1B);

/// 최상위 창 타이틀바를 다크로 전환한다.
/// 미지원 OS(구버전 Windows)에서는 실패하지만 앱 동작에는 영향 없으므로 반환을 무시한다.
pub fn apply_dark_titlebar(hwnd: HWND) {
    // DWMWA_USE_IMMERSIVE_DARK_MODE는 BOOL(4바이트 정수)을 받는다 — 1(TRUE) 전달
    let enabled: i32 = 1;
    // 안전성: 유효한 최상위 창 핸들에 DWM 속성 설정. pvAttribute는 i32 1개를 가리키며 그 크기를 함께 전달
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &enabled as *const i32 as *const core::ffi::c_void,
            size_of::<i32>() as u32,
        );
    }
}
