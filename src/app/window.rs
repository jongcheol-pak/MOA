//! 메인 창 — 클래스 등록·생성·윈도우 프로시저·명령 배선
use crate::app::layout::SplitDir;
use crate::app::layout_host::LayoutHost;
use crate::app::menu::{self, IDM_CLOSE_PANE, IDM_SPLIT_H, IDM_SPLIT_V};
use std::cell::{RefCell, RefMut};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH, ScreenToClient};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, GWLP_USERDATA,
    GetCursorPos, GetWindowLongPtrW, HACCEL, HMENU, HTCLIENT, IDC_ARROW, LoadCursorW,
    PostQuitMessage, RegisterClassExW, SW_SHOW, SWP_NOACTIVATE, SWP_NOZORDER, SetWindowLongPtrW,
    SetWindowPos, ShowWindow, WINDOW_EX_STYLE, WM_CAPTURECHANGED, WM_COMMAND, WM_DESTROY,
    WM_DPICHANGED, WM_INITMENUPOPUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PARENTNOTIFY,
    WM_SETCURSOR, WM_SIZE, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, Result, w};

const WINDOW_CLASS: PCWSTR = w!("FileExplorerMainWindow");

/// 창별 상태 — GWLP_USERDATA에 `Box<RefCell<...>>`로 귀속.
/// RefCell인 이유: 자식 창 생성/파괴(split·close) 중 WM_PARENTNOTIFY 등이 같은 스레드로
/// 동기 재진입할 수 있다. 현재는 각 핸들러의 조건 필터(예: WM_LBUTTONDOWN 서브타입만
/// 상태 접근)가 1차로 별칭을 차단하며, RefCell의 try_borrow_mut은 그 필터가 변경·확장돼
/// 재진입 경로에서 상태에 접근하게 되더라도 별칭 &mut이 만들어질 수 없게 하는 구조적 안전망이다.
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
            let state = Box::new(RefCell::new(AppState { host, menu }));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let haccel = menu::create_accels()?;
            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(MainWindow { hwnd, haccel })
        }
    }
}

/// GWLP_USERDATA에서 상태를 빌린다. 부착 전(null)이거나 이미 빌린 중(동기 재진입)이면 None.
fn state_of<'a>(hwnd: HWND) -> Option<RefMut<'a, AppState>> {
    // 안전성: 포인터는 create의 Box::into_raw 산출물뿐이며 WM_DESTROY에서 회수된다.
    // &RefCell 공유 참조는 별칭 가능하고, &mut 배타성은 RefCell 런타임 검사가 보장한다.
    let cell =
        unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<AppState>).as_ref() }?;
    cell.try_borrow_mut().ok()
}

/// WM_DESTROY에서 상태 Box를 회수한다 (이후 메시지는 null 가드로 기본 처리)
fn detach_state(hwnd: HWND) {
    // 안전성: create에서 넣은 포인터를 정확히 한 번 Box로 되돌려 해제
    unsafe {
        let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut RefCell<AppState>;
        if !ptr.is_null() {
            drop(Box::from_raw(ptr));
        }
    }
}

/// 기본 프로시저 위임 래퍼
fn def_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 안전성: 받은 인자를 그대로 OS 기본 처리에 전달
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// 현재 커서 위치를 창 클라이언트 좌표로 반환
fn cursor_in_client(hwnd: HWND) -> Option<(i32, i32)> {
    let mut pt = POINT::default();
    // 안전성: pt는 스택 소유, 두 호출 모두 유효한 핸들·포인터만 사용
    unsafe {
        if GetCursorPos(&mut pt).is_ok() && ScreenToClient(hwnd, &mut pt).as_bool() {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

/// DPI 변경 시 OS 제안 사각형으로 이동 (NFR-4)
fn move_to_suggested(hwnd: HWND, lparam: LPARAM) {
    // 안전성: WM_DPICHANGED의 lparam은 OS가 채운 RECT 포인터 (메시지 처리 동안 유효)
    unsafe {
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
    }
}

fn loword(v: usize) -> u32 {
    (v & 0xffff) as u32
}

fn coords(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    (x, y)
}

/// 메인 창 프로시저 — 레이아웃·명령·스플리터 드래그 배선.
/// unsafe는 소단위 헬퍼(state_of·def_proc 등)에 격리되어 본문은 safe 로직만 담는다.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_SIZE => {
            if let Some(mut state) = state_of(hwnd) {
                state.host.relayout(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let Some(mut state) = state_of(hwnd) else {
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
                _ => return def_proc(hwnd, msg, wparam, lparam),
            }
            menu::update_close_enabled(state.menu, state.host.panel_count());
            LRESULT(0)
        }
        WM_INITMENUPOPUP => {
            if let Some(state) = state_of(hwnd) {
                menu::update_close_enabled(state.menu, state.host.panel_count());
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN => {
            if let Some(mut state) = state_of(hwnd) {
                let (x, y) = coords(lparam);
                if state.host.begin_drag(hwnd, x, y) {
                    return LRESULT(0);
                }
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEMOVE => {
            if let Some(mut state) = state_of(hwnd) {
                let (x, y) = coords(lparam);
                if state.host.drag_move(hwnd, x, y) {
                    return LRESULT(0);
                }
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            if let Some(mut state) = state_of(hwnd) {
                state.host.end_drag(true);
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_CAPTURECHANGED => {
            // 드래그 중 캡처 상실(Alt+Tab 등) → 드래그 상태 정리 (plan T3 Edge)
            if let Some(mut state) = state_of(hwnd) {
                state.host.end_drag(false);
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            if let Some(state) = state_of(hwnd) {
                if state.host.apply_drag_cursor() {
                    return LRESULT(1);
                }
                if loword(lparam.0 as usize) == HTCLIENT
                    && let Some((x, y)) = cursor_in_client(hwnd)
                    && state.host.apply_splitter_cursor(x, y)
                {
                    return LRESULT(1);
                }
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_PARENTNOTIFY => {
            // 자식 패널 클릭 → 활성 패널 갱신 (좌표는 부모 클라이언트 기준).
            // split/close 도중 동기 재진입하는 WM_CREATE/WM_DESTROY 서브타입은
            // 아래 WM_LBUTTONDOWN 필터가 먼저 걸러 상태 접근 자체가 없다(1차 방어).
            // 필터가 바뀌어도 state_of의 try_borrow_mut이 None을 돌려 별칭은 생기지 않는다(2차 방어).
            if loword(wparam.0) == WM_LBUTTONDOWN
                && let Some(mut state) = state_of(hwnd)
            {
                let (x, y) = coords(lparam);
                state.host.set_active_by_point(x, y);
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_DPICHANGED => {
            // 제안 사각형으로 이동 → WM_SIZE가 재배치를 수행 (NFR-4)
            move_to_suggested(hwnd, lparam);
            LRESULT(0)
        }
        WM_DESTROY => {
            detach_state(hwnd);
            // 안전성: 자기 스레드 메시지 큐에 종료 통지
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => def_proc(hwnd, msg, wparam, lparam),
    }
}
