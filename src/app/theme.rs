//! 다크 테마 — 창에 거는 Win32·DWM 설정.
//!
//! 전환 UI 없는 **고정 다크**다(FR-15·FR-21). egui 화면의 색은 `ui::theme`가 갖고,
//! 여기 남은 것은 **창 자체**에 거는 Win32·DWM 설정과 그것이 쓰는 배경색 하나다.
use windows::Win32::Foundation::{COLORREF, HWND};
use windows::Win32::Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmSetWindowAttribute};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::core::{PCSTR, w};

/// COLORREF는 0x00BBGGRR 순서 — 실수 방지용 헬퍼
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

/// 창 배경·스플리터 틈 (사이드바 COLOR_BG와 동일)
pub const WINDOW_BG: COLORREF = rgb(0x1B, 0x1B, 0x1B);

/// uxtheme 언문서 다크 모드 API로 **앱 전역 다크**를 켠다 (Windows 10 1903+ / 11).
/// 이걸 켜야 각 컨트롤의 `SetWindowTheme("DarkMode_Explorer")`가 목록 헤더·스크롤바·트리·탭·
/// 팝업 메뉴를 시스템이 자동으로 다크로 그린다(오너드로우 각개격파보다 안정적 — Windows 탐색기 방식).
/// 문서화되지 않은 ordinal API지만 다크 앱들이 표준으로 쓴다. 미지원 OS면 조용히 무시된다.
/// 순서가 중요하다: SetPreferredAppMode → RefreshImmersiveColorPolicyState로 정책을 **즉시 반영**해야
/// 이후 창·컨트롤 생성 시 다크가 결정적으로 적용된다(둘째를 빠뜨리면 반영이 타이밍에 따라 갈린다).
/// - ordinal 135: `SetPreferredAppMode(PreferredAppMode)` — 2=ForceDark
/// - ordinal 104: `RefreshImmersiveColorPolicyState()` — 앱 다크 정책 즉시 반영
/// - ordinal 136: `FlushMenuThemes()` — 메뉴 테마 갱신
pub fn enable_dark_mode() {
    // 안전성: uxtheme.dll을 로드해 ordinal 함수 포인터를 얻어 호출한다. 시그니처는 문서화된 형태를
    // 그대로 옮긴 것이며, 실패(미지원 OS·심볼 부재)는 조용히 무시한다(다크만 안 될 뿐 앱은 정상)
    unsafe {
        let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) else {
            return;
        };
        // ordinal은 PCSTR 포인터 값 자체로 전달한다(MAKEINTRESOURCE 규약)
        if let Some(proc) = GetProcAddress(uxtheme, PCSTR(135 as *const u8)) {
            let set_preferred: extern "system" fn(i32) -> i32 = std::mem::transmute(proc);
            set_preferred(2); // PreferredAppMode::ForceDark
        }
        if let Some(proc) = GetProcAddress(uxtheme, PCSTR(104 as *const u8)) {
            let refresh: extern "system" fn() = std::mem::transmute(proc);
            refresh();
        }
        if let Some(proc) = GetProcAddress(uxtheme, PCSTR(136 as *const u8)) {
            let flush: extern "system" fn() = std::mem::transmute(proc);
            flush();
        }
    }
}

/// 최대화·복원·최소화 때 DWM이 넣는 전환 애니메이션을 이 창에서 끈다 (FR-22).
///
/// 그 애니메이션은 **바뀌기 전 창의 스냅샷과 새 화면을 겹쳐 교차 페이드**한다. 창 크기에 따라
/// 패널·목록이 재배치되는 이 앱에서는 두 화면의 글자 위치가 크게 달라, 겹치는 동안 글자가
/// 이중으로 보이며 화면이 떨리는 것처럼 보인다. 게다가 애니메이션이 도는 동안 새 화면을
/// 내보내는 시각이 300ms 가까이 밀려 그 겹침이 오래 남는다(실측).
///
/// 끄면 최대화·복원이 애니메이션 없이 곧바로 바뀐다 — 창 장식을 앱이 직접 그리는 이 앱에서는
/// OS 애니메이션과 어차피 어울리지 않는다. 미지원 OS에서는 실패하지만 앱 동작에는 영향 없다.
pub fn disable_window_transitions(hwnd: HWND) {
    // DWMWA_TRANSITIONS_FORCEDISABLED는 BOOL(4바이트 정수)을 받는다 — 1(TRUE)이 "끔"
    let disabled: i32 = 1;
    // 안전성: 유효한 최상위 창 핸들에 DWM 속성 설정. pvAttribute는 i32 1개를 가리키며 그 크기를 함께 전달
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_TRANSITIONS_FORCEDISABLED,
            &disabled as *const i32 as *const core::ffi::c_void,
            size_of::<i32>() as u32,
        );
    }
}

/// 창이 **아직 그리지 않은 자리**를 흰색 대신 앱 배경색으로 칠하게 한다.
///
/// 창 클래스의 기본 배경 브러시는 흰색이라, 창이 처음 표시되거나 크기가 바뀌어 GL이 아직
/// 내보내지 않은 영역이 생기면 그 자리가 **흰 사각형으로 번쩍인다**(2026-08-13 실측:
/// 최대화 복원 순간 약 60ms). 앱은 고정 다크라 브러시를 창 배경색으로 바꿔 두면
/// 그 순간에도 색이 이어져 번쩍임이 보이지 않는다.
///
/// 브러시는 프로세스가 사는 동안 창 클래스가 들고 있어야 하므로 해제하지 않는다 —
/// 창 하나짜리 앱에서 한 번 만들고 끝이며, 종료 시 OS가 거둔다.
/// 기존 브러시도 지우지 않는다(시스템 소유일 수 있다)
pub fn paint_unpainted_as_window_bg(hwnd: HWND) {
    use windows::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows::Win32::UI::WindowsAndMessaging::{GCLP_HBRBACKGROUND, SetClassLongPtrW};
    // 안전성: 유효한 최상위 창 핸들에 클래스 배경 브러시를 건다. 브러시는 앱 수명 동안 유효하며
    // (해제하지 않는다) 실패해도 종전처럼 기본 브러시가 쓰일 뿐이라 동작에는 영향이 없다
    unsafe {
        let brush = CreateSolidBrush(WINDOW_BG);
        if !brush.is_invalid() {
            SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, brush.0 as isize);
        }
    }
}
