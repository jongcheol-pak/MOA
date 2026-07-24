//! 메인 창 — 클래스 등록·생성·윈도우 프로시저·명령 배선
use crate::app::layout::SplitDir;
use crate::app::layout_host::{self, LayoutHost};
use crate::app::menu::{
    self, IDM_CLOSE_PANE, IDM_NAV_BACK, IDM_NAV_FORWARD, IDM_NAV_UP, IDM_REFRESH, IDM_SPLIT_H,
    IDM_SPLIT_V, IDM_TAB_CLOSE, IDM_TAB_NEW, IDM_TREE_TOGGLE,
};
use crate::app::settings::{
    self, LayoutNode, PanelSession, Session, WindowState, WorkspaceSession,
};
use crate::panel::panel::{
    PanelSessionData, WM_APP_NAV_BACK, WM_APP_NAV_FORWARD, WM_APP_NAV_UP, WM_APP_REFRESH,
    WM_APP_SESSION_COLLECT, WM_APP_SESSION_RESTORE, WM_APP_TAB_CLOSE, WM_APP_TAB_NEW,
    WM_APP_TREE_TOGGLE,
};
use std::cell::{RefCell, RefMut};
use std::path::PathBuf;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    COLOR_WINDOW, HBRUSH, MONITOR_DEFAULTTONULL, MonitorFromRect, ScreenToClient,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, GWLP_USERDATA,
    GetCursorPos, GetWindowLongPtrW, GetWindowPlacement, HACCEL, HMENU, HTCLIENT, IDC_ARROW,
    LoadCursorW, PostQuitMessage, RegisterClassExW, SW_SHOW, SW_SHOWMAXIMIZED, SWP_NOACTIVATE,
    SWP_NOZORDER, SetWindowLongPtrW, SetWindowPlacement, SetWindowPos, ShowWindow, WINDOW_EX_STYLE,
    WINDOWPLACEMENT, WM_CAPTURECHANGED, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_DPICHANGED,
    WM_INITMENUPOPUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_PARENTNOTIFY, WM_SETCURSOR,
    WM_SIZE, WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
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
            // 세션이 있으면 활성 워크스페이스의 분할 구조를 복원, 없거나 손상이면 기본 1패널
            // (FR-11·FR-20 — 나머지 워크스페이스의 지연 생성은 T5)
            let session = settings::load_session();
            let active_ws = session
                .as_ref()
                .and_then(|s| s.workspaces.get(s.active_workspace));
            // 배치 영역은 창 클라이언트 전체 — 사이드바가 생기면 그만큼 좁혀 주입한다 (T5·T7)
            let area = layout_host::client_rect(hwnd);
            let host = match active_ws {
                Some(ws) => LayoutHost::from_shape(hwnd, &ws.layout.to_shape(), area)?,
                None => LayoutHost::new(hwnd, area)?,
            };
            if let Some(ws) = active_ws {
                restore_panels(&host, ws);
            }
            menu::update_close_enabled(menu, host.panel_count());
            let state = Box::new(RefCell::new(AppState { host, menu }));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            let haccel = menu::create_accels()?;
            match &session {
                Some(s) => apply_window_state(hwnd, &s.window),
                None => {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }
            }

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

/// 패널로 인자 없는 앱 메시지 게시
fn post_to(hwnd: HWND, msg: u32) {
    // 안전성: 비동기 게시 — 대상 창이 파괴됐으면 실패만 반환
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
            Some(hwnd),
            msg,
            WPARAM(0),
            LPARAM(0),
        );
    }
}

/// 패널로 동기 질의 — 패널 상태 RefCell은 메인 창 것과 별개라 차용 충돌 없음
fn send_to(hwnd: HWND, msg: u32) -> LRESULT {
    send_ptr(hwnd, msg, 0)
}

/// 패널로 동기 질의 (lparam 포인터 전달) — 세션 수집/복원(PanelSessionData) 계약 전용.
/// 반드시 SendMessage(동기)여야 한다 — 포인터가 호출 스택 소유라 Post는 금지
fn send_ptr(hwnd: HWND, msg: u32, lparam: isize) -> LRESULT {
    // 안전성: 유효한 자식 창 핸들에 동기 메시지 — lparam 포인터는 호출 동안 유효
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::SendMessageW(
            hwnd,
            msg,
            Some(WPARAM(0)),
            Some(LPARAM(lparam)),
        )
    }
}

/// 워크스페이스의 패널별 탭을 각 패널에 복원 (walk 순서 1:1 — layout_host 계약)
fn restore_panels(host: &LayoutHost, workspace: &WorkspaceSession) {
    for (panel, ps) in host.panel_hwnds().iter().zip(&workspace.panels) {
        let data = PanelSessionData {
            tabs: ps.tabs.iter().map(PathBuf::from).collect(),
            active: ps.active_tab,
        };
        send_ptr(
            *panel,
            WM_APP_SESSION_RESTORE,
            &data as *const PanelSessionData as isize,
        );
    }
}

/// 저장된 창 위치·최대화 적용. 모니터 밖(분리된 모니터)이면 위치는 생략하고
/// 기본 위치(주 모니터)로 표시만 한다 (T4 Edge)
fn apply_window_state(hwnd: HWND, w: &WindowState) {
    let rect = RECT {
        left: w.x,
        top: w.y,
        right: w.x + w.w,
        bottom: w.y + w.h,
    };
    // 안전성: rect는 스택 소유 — 어느 모니터에도 안 걸리면 널 핸들 반환
    let on_screen = unsafe { !MonitorFromRect(&rect, MONITOR_DEFAULTTONULL).is_invalid() };
    if on_screen {
        let wp = WINDOWPLACEMENT {
            length: size_of::<WINDOWPLACEMENT>() as u32,
            showCmd: SW_SHOW.0 as u32,
            rcNormalPosition: rect,
            ..Default::default()
        };
        // 안전성: wp는 스택 소유, 유효한 창 핸들
        unsafe {
            let _ = SetWindowPlacement(hwnd, &wp);
        }
    }
    // 안전성: 표시 상태 적용
    unsafe {
        let _ = ShowWindow(
            hwnd,
            if w.maximized {
                SW_SHOWMAXIMIZED
            } else {
                SW_SHOW
            },
        );
    }
}

/// WM_CLOSE — 현재 레이아웃·패널 탭·창 위치를 수집해 저장 (FR-11, D15: 종료 시 1회)
fn save_current_session(hwnd: HWND) {
    let Some(state) = state_of(hwnd) else {
        return;
    };
    let (shape, hwnds) = state.host.session_snapshot();
    let mut panels = Vec::with_capacity(hwnds.len());
    for panel in hwnds {
        let mut data = PanelSessionData::default();
        // 패널 RefCell은 메인 창 것과 별개 — 동기 수집 안전 (send_to 주석 참조)
        send_ptr(
            panel,
            WM_APP_SESSION_COLLECT,
            &mut data as *mut PanelSessionData as isize,
        );
        panels.push(PanelSession {
            tabs: data
                .tabs
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect(),
            active_tab: data.active,
        });
    }
    let mut wp = WINDOWPLACEMENT {
        length: size_of::<WINDOWPLACEMENT>() as u32,
        ..Default::default()
    };
    // 안전성: wp는 스택 소유, 유효한 창 핸들 — 실패 시 기본값(0 크기)은 저장 검증에서 걸러짐
    unsafe {
        let _ = GetWindowPlacement(hwnd, &mut wp);
    }
    let rc = wp.rcNormalPosition;
    settings::save_session(&Session {
        version: settings::SESSION_VERSION,
        window: WindowState {
            x: rc.left,
            y: rc.top,
            w: rc.right - rc.left,
            h: rc.bottom - rc.top,
            maximized: wp.showCmd == SW_SHOWMAXIMIZED.0 as u32,
        },
        // 사이드바 상태와 워크스페이스 목록은 T5·T7에서 실제 값으로 채운다.
        // 이 단계는 스키마만 v2이고 화면은 워크스페이스 1개짜리와 같다
        sidebar: settings::SidebarSession::default(),
        active_workspace: 0,
        workspaces: vec![WorkspaceSession {
            name: "워크스페이스 1".to_string(),
            layout: LayoutNode::from_shape(&shape),
            panels,
            active_panel: 0,
        }],
    });
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
                // 영역 주입 → 배치 (relayout은 더 이상 부모 클라이언트를 스스로 읽지 않는다)
                state.host.set_area(layout_host::client_rect(hwnd));
                state.host.relayout();
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
                IDM_CLOSE_PANE => state.host.close_active(),
                // 전역 네비게이션·새 탭·트리 토글·새로 고침 → 활성 패널로 게시
                // (Post — 차용 해제 후 처리됨)
                id @ (IDM_NAV_BACK | IDM_NAV_FORWARD | IDM_NAV_UP | IDM_TAB_NEW
                | IDM_TREE_TOGGLE | IDM_REFRESH) => {
                    if let Some(panel) = state.host.active_hwnd() {
                        let nav_msg = match id {
                            IDM_NAV_BACK => WM_APP_NAV_BACK,
                            IDM_NAV_FORWARD => WM_APP_NAV_FORWARD,
                            IDM_NAV_UP => WM_APP_NAV_UP,
                            IDM_TREE_TOGGLE => WM_APP_TREE_TOGGLE,
                            IDM_REFRESH => WM_APP_REFRESH,
                            _ => WM_APP_TAB_NEW,
                        };
                        post_to(panel, nav_msg);
                    }
                }
                // 탭 닫기는 동기 질의 — 0(마지막 탭)이면 패널 닫기로 연결 (FR-2·T6 Edge)
                IDM_TAB_CLOSE => {
                    if let Some(panel) = state.host.active_hwnd()
                        && send_to(panel, WM_APP_TAB_CLOSE).0 == 0
                    {
                        state.host.close_active();
                    }
                }
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
                if state.host.drag_move(x, y) {
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
        WM_CLOSE => {
            // 파괴 전에 세션 저장 — 이후 기본 처리(DestroyWindow)로 진행
            save_current_session(hwnd);
            def_proc(hwnd, msg, wparam, lparam)
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
