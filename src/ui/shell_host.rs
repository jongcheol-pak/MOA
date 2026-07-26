//! Win32 셸 컨텍스트 메뉴 연동 (FR-8)
//!
//! 기존 `fs::shell_menu`를 그대로 재사용한다. 필요한 것은 두 가지뿐이다:
//! ① eframe(winit) 창의 **HWND** ② 그 창의 **창 프로시저에 끼어들 지점**.
//!
//! ②가 필요한 이유: 셸 메뉴의 "보내기" 같은 서브메뉴는 `IContextMenu2/3`가
//! `WM_INITMENUPOPUP` 등을 받아야 채워지는데, winit은 그 메시지를 우리 코드로 넘겨주지 않는다.
//! 그래서 창을 서브클래싱해 `forward_menu_msg`로 전달한다(서브클래스가 없으면 서브메뉴가 빈다).
use crate::fs::shell_menu::{forward_menu_msg, show_context_menu};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Shell::{
    DefSubclassProc, SHELLEXECUTEINFOW, SetWindowSubclass, ShellExecuteExW,
};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

/// 서브클래스 식별자 — 이 바이너리가 붙인 것임을 구분하는 임의의 상수
const SUBCLASS_ID: usize = 1;

/// 셸 메뉴를 띄울 창 핸들 보유자
pub struct ShellHost {
    hwnd: HWND,
}

impl ShellHost {
    /// eframe 창에서 HWND를 얻고 서브클래스를 설치한다.
    /// HWND를 얻지 못하면(다른 백엔드·headless) `None` — 호출부는 셸 메뉴를 비활성한다
    pub fn new(cc: &eframe::CreationContext<'_>) -> Option<ShellHost> {
        let handle = cc.window_handle().ok()?;
        let RawWindowHandle::Win32(win32) = handle.as_raw() else {
            return None;
        };
        let hwnd = HWND(win32.hwnd.get() as *mut core::ffi::c_void);
        // 안전성: 방금 얻은 유효한 창 핸들에 표준 서브클래스 등록.
        // 창이 파괴되면 서브클래스도 함께 사라지므로 별도 해제는 필요 없다
        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(shell_menu_proc), SUBCLASS_ID, 0);
        }
        Some(ShellHost { hwnd })
    }

    /// 셸 컨텍스트 메뉴를 화면 좌표에 띄운다.
    /// `items`가 비면 폴더 배경 메뉴("새로 만들기" 포함)가 나온다.
    /// 메뉴가 닫힐 때까지 이 호출은 반환하지 않는다(TrackPopupMenuEx 모달 루프)
    pub fn popup(&self, folder: &Path, items: &[PathBuf], screen_x: i32, screen_y: i32) {
        show_context_menu(self.hwnd, folder, items, screen_x, screen_y);
    }

    /// egui가 주는 클라이언트 좌표(물리 픽셀)를 셸 메뉴가 요구하는 화면 좌표로 바꾼다
    pub fn to_screen(&self, client_x: i32, client_y: i32) -> (i32, i32) {
        let mut point = POINT {
            x: client_x,
            y: client_y,
        };
        // 안전성: 유효한 창 핸들 + 스택에 있는 POINT 하나를 변환한다
        unsafe {
            let _ = ClientToScreen(self.hwnd, &mut point);
        }
        (point.x, point.y)
    }
}

/// 파일을 연결 프로그램으로 연다 (FR-7).
/// 실패해도 셸이 자기 UI로 알리므로 여기서는 따로 알리지 않는다
pub fn execute(path: &Path) {
    let file = HSTRING::from(path.to_string_lossy().as_ref());
    let mut sei = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        lpVerb: w!("open"),
        lpFile: PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // 안전성: sei와 file은 호출이 끝날 때까지 살아 있는 지역 소유다
    unsafe {
        let _ = ShellExecuteExW(&mut sei);
    }
}

/// 창 서브클래스 프로시저 — 메뉴 모달 중의 소유자 메시지를 셸에 전달한다.
/// 메뉴가 떠 있지 않으면 `forward_menu_msg`가 `None`을 주므로 원래 프로시저로 넘긴다
unsafe extern "system" fn shell_menu_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if let Some(result) = forward_menu_msg(msg, wparam, lparam) {
        return result;
    }
    // 안전성: 그 외 메시지는 원래(winit) 창 프로시저로 위임한다
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
