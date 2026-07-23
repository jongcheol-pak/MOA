//! 메인 창 — 클래스 등록·생성·윈도우 프로시저·명령 배선
use crate::app::layout::SplitDir;
use crate::app::layout_host::LayoutHost;
use crate::app::menu::{self, IDM_CLOSE_PANE, IDM_SPLIT_H, IDM_SPLIT_V};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH, ScreenToClient};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, GWLP_USERDATA,
    GetCursorPos, GetWindowLongPtrW, HACCEL, HMENU, HTCLIENT, IDC_ARROW, LoadCursorW,
    PostQuitMessage, RegisterClassExW, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, SetCursor,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WM_CAPTURECHANGED, WM_COMMAND,
    WM_DESTROY, WM_DPICHANGED, WM_INITMENUPOPUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_PARENTNOTIFY, WM_SETCURSOR, WM_SIZE, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, Result, w};

const WINDOW_CLASS: PCWSTR = w!("FileExplorerMainWindow");

/// 창별 상태 — GWLP_USERDATA에 Box로 귀속
struct AppState {
    host: LayoutHost,
    menu: HMENU,
}

/// 메인 창. 메시지 루프(main 소유)가 hwnd·haccel을 사용한다.
pub struct MainWindow {
    pub hwnd: HWND,
    pub haccel: HACCEL,
}

impl MainWindow {
    pub fn create() -> Result<MainWindow> {
        // 안전성: Win32 창 생성 FFI — 인자는 모두 유효한 정적/스택 값이며 실패는 Result로 전파
        unsafe {
            let instance = GetModuleHandleW(None)?;

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                // 시스템 배경 브러시 관례: COLOR_* + 1 값을 HBRUSH로 전달
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut core::ffi::c_void),
                lpszClassName: WINDOW_CLASS,
                ..Default::default()
            };
            if RegisterClassExW(&wc) == 0 {
                return Err(windows::core::Error::from_thread());
            }

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WINDOW_CLASS,
                w!("파일 탐색기"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1200,
                800,
                None,
                None,
                Some(instance.into()),
                None,
            )?;

            // 상태는 창 생성 후 부착 — 부착 전 도착하는 메시지는 null 가드로 기본 처리
            let menu = menu::attach_menu(hwnd)?;
            let host = LayoutHost::new(hwnd)?;
            menu::update_close_enabled(menu, host.panel_count());
            let state = Box::new(AppState { host, menu });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let haccel = menu::create_accels()?;
            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(MainWindow { hwnd, haccel })
        }
    }
}

/// GWLP_USERDATA에서 상태를 꺼낸다 (부착 전이면 None)
///
/// 안전성: 포인터는 create에서 Box::into_raw로 넣은 것뿐이며 WM_DESTROY에서 회수된다
unsafe fn state_of<'a>(hwnd: HWND) -> Option<&'a mut AppState> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut AppState;
    // 안전성: null 가드 후 단일 UI 스레드에서만 접근 — 별칭 없음
    unsafe { ptr.as_mut() }
}

fn loword(v: usize) -> u32 {
    (v & 0xffff) as u32
}

fn coords(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    (x, y)
}

/// 메인 창 프로시저 — 레이아웃·명령·스플리터 드래그 배선
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // 안전성: 아래 각 분기의 Win32 호출은 유효한 핸들·스택 값만 사용한다
    unsafe {
        match msg {
            WM_SIZE => {
                if let Some(state) = state_of(hwnd) {
                    state.host.relayout(hwnd);
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let Some(state) = state_of(hwnd) else {
                    return LRESULT(0);
                };
                match loword(wparam.0) {
                    IDM_SPLIT_H => {
                        let _ = state.host.split_active(hwnd, SplitDir::Horizontal);
                    }
                    IDM_SPLIT_V => {
                        let _ = state.host.split_active(hwnd, SplitDir::Vertical);
                    }
                    IDM_CLOSE_PANE => state.host.close_active(hwnd),
                    _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
                }
                menu::update_close_enabled(state.menu, state.host.panel_count());
                LRESULT(0)
            }
            WM_INITMENUPOPUP => {
                if let Some(state) = state_of(hwnd) {
                    menu::update_close_enabled(state.menu, state.host.panel_count());
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_LBUTTONDOWN => {
                if let Some(state) = state_of(hwnd) {
                    let (x, y) = coords(lparam);
                    if state.host.begin_drag(hwnd, x, y) {
                        return LRESULT(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_MOUSEMOVE => {
                if let Some(state) = state_of(hwnd) {
                    let (x, y) = coords(lparam);
                    if state.host.drag_move(hwnd, x, y) {
                        return LRESULT(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_LBUTTONUP => {
                if let Some(state) = state_of(hwnd) {
                    state.host.end_drag(true);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_CAPTURECHANGED => {
                // 드래그 중 캡처 상실(Alt+Tab 등) → 드래그 상태 정리 (plan T3 Edge)
                if let Some(state) = state_of(hwnd) {
                    state.host.end_drag(false);
                }
                LRESULT(0)
            }
            WM_SETCURSOR => {
                if let Some(state) = state_of(hwnd) {
                    if state.host.is_dragging() && state.host.apply_drag_cursor() {
                        return LRESULT(1);
                    }
                    if loword(lparam.0 as usize) == HTCLIENT {
                        let mut pt = POINT::default();
                        if GetCursorPos(&mut pt).is_ok()
                            && ScreenToClient(hwnd, &mut pt).as_bool()
                            && let Some(cur) = state.host.cursor_at(pt.x, pt.y)
                        {
                            SetCursor(Some(cur));
                            return LRESULT(1);
                        }
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_PARENTNOTIFY => {
                // 자식 패널 클릭 → 활성 패널 갱신 (좌표는 부모 클라이언트 기준)
                if loword(wparam.0) == WM_LBUTTONDOWN
                    && let Some(state) = state_of(hwnd)
                {
                    let (x, y) = coords(lparam);
                    state.host.set_active_by_point(x, y);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_DPICHANGED => {
                // 제안 사각형으로 이동 → WM_SIZE가 재배치를 수행 (NFR-4)
                let suggested = lparam.0 as *const RECT;
                if let Some(r) = suggested.as_ref() {
                    let _ = SetWindowPos(
                        hwnd,
                        None,
                        r.left,
                        r.top,
                        r.right - r.left,
                        r.bottom - r.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                // 상태 Box 회수 — 이후 메시지는 null 가드로 기본 처리
                let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut AppState;
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
