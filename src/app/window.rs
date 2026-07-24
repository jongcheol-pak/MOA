//! 메인 창 — 클래스 등록·생성·윈도우 프로시저·명령 배선
use crate::app::layout::{Rect, SPLITTER_THICKNESS, SplitDir};
use crate::app::layout_host::{self, LayoutHost};
use crate::app::menu::{
    self, IDM_CLOSE_PANE, IDM_NAV_BACK, IDM_NAV_FORWARD, IDM_NAV_UP, IDM_REFRESH, IDM_SPLIT_H,
    IDM_SPLIT_V, IDM_TAB_CLOSE, IDM_TAB_NEW, IDM_TREE_TOGGLE,
};
use crate::app::settings::{
    self, LayoutNode, PanelSession, Session, WindowState, WorkspaceSession,
};
use crate::app::sidebar::{Sidebar, WM_APP_WS_NEW, WM_APP_WS_SELECT};
use crate::app::workspace::WorkspaceList;
use crate::panel::panel::{
    PanelSessionData, WM_APP_NAV_BACK, WM_APP_NAV_FORWARD, WM_APP_NAV_UP, WM_APP_PATH_CHANGED,
    WM_APP_REFRESH, WM_APP_SESSION_COLLECT, WM_APP_SESSION_RESTORE, WM_APP_TAB_CLOSE,
    WM_APP_TAB_NEW, WM_APP_TREE_TOGGLE,
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
    LoadCursorW, MoveWindow, PostQuitMessage, RegisterClassExW, SW_SHOW, SW_SHOWMAXIMIZED,
    SWP_NOACTIVATE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPlacement, SetWindowPos, ShowWindow,
    WINDOW_EX_STYLE, WINDOWPLACEMENT, WM_CAPTURECHANGED, WM_CLOSE, WM_COMMAND, WM_DESTROY,
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
    menu: HMENU,
    /// 좌측 워크스페이스 목록 창 (FR-15)
    sidebar: Sidebar,
    /// 워크스페이스 목록 모델 — 사이드바는 이 스냅숏을 받아 그린다
    list: WorkspaceList,
    /// 워크스페이스별 탐색기 상태 — `list.items()`와 **인덱스 1:1**로 대응한다 (FR-17)
    entries: Vec<EntryState>,
}

/// 워크스페이스 하나의 탐색기 상태 — 방문 전에는 세션 데이터만 들고 있다 (D2 지연 생성)
enum EntryState {
    /// 미방문 — 저장된 세션 데이터 보관 (창 없음). 선택되면 Live로 승격된다
    Pending(WorkspaceSession),
    /// 방문함 — 탐색기 창들이 만들어져 있다 (비활성이면 숨김 상태)
    Live(LayoutHost),
}

impl AppState {
    /// 활성 워크스페이스의 탐색기 — 미방문(Pending)이면 None
    fn active_host(&mut self) -> Option<&mut LayoutHost> {
        match self.entries.get_mut(self.list.active_index()) {
            Some(EntryState::Live(host)) => Some(host),
            _ => None,
        }
    }

    /// 활성 워크스페이스의 탐색기 (읽기 전용)
    fn active_host_ref(&self) -> Option<&LayoutHost> {
        match self.entries.get(self.list.active_index()) {
            Some(EntryState::Live(host)) => Some(host),
            _ => None,
        }
    }

    /// 미방문 워크스페이스를 실제 탐색기 창으로 승격한다 (지연 생성 — D2).
    /// 창 생성에 실패하면 Pending 상태 그대로 두고 호출부가 전환을 포기한다
    fn materialize(&mut self, hwnd: HWND, index: usize) {
        let Some(EntryState::Pending(ws)) = self.entries.get(index) else {
            return;
        };
        let ws = ws.clone();
        let area = explorer_area(hwnd);
        let Ok(host) = LayoutHost::from_shape(hwnd, &ws.layout.to_shape(), area) else {
            return;
        };
        restore_panels(&host, &ws);
        self.entries[index] = EntryState::Live(host);
    }

    /// 활성 워크스페이스의 패널 수 — 메뉴 활성 상태 갱신용 (미방문이면 1로 본다)
    fn active_panel_count(&self) -> usize {
        self.active_host_ref().map_or(1, LayoutHost::panel_count)
    }
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
            // 세션이 있으면 워크스페이스 전부를 미방문(Pending)으로 적재하고 활성 1개만 승격한다
            // (FR-20·D2 — 방문한 워크스페이스만 창을 만들어 메모리를 억제)
            let session = settings::load_session();
            // 좌측 사이드바를 먼저 만들고, 탐색기는 그만큼 좁아진 영역에 배치한다 (FR-15)
            let sidebar = Sidebar::create(hwnd)?;
            let (list, mut entries) = restore_workspaces(session.as_ref());
            let area = explorer_area(hwnd);
            if entries.is_empty() {
                // 세션 없음·손상 → 기본 워크스페이스 1개로 시작
                entries.push(EntryState::Live(LayoutHost::new(hwnd, area)?));
            } else {
                let active = list.active_index();
                match &entries[active] {
                    EntryState::Pending(ws) => {
                        let ws = ws.clone();
                        let host = LayoutHost::from_shape(hwnd, &ws.layout.to_shape(), area)?;
                        restore_panels(&host, &ws);
                        entries[active] = EntryState::Live(host);
                    }
                    EntryState::Live(_) => {}
                }
            }
            let state = AppState {
                menu,
                sidebar,
                list,
                entries,
            };
            menu::update_close_enabled(menu, state.active_panel_count());
            state
                .sidebar
                .set_items(state.list.items(), state.list.active_index());
            let state = Box::new(RefCell::new(state));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
            layout_children(hwnd);
            refresh_subtitle(hwnd);

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

/// 탐색기(LayoutHost)에 줄 배치 영역 — 창 클라이언트에서 사이드바 폭과 경계선을 뺀 나머지.
/// 폭·접힘을 상태에서 읽는 것은 T7이며, 지금은 기본 폭 고정이다
fn explorer_area(hwnd: HWND) -> Rect {
    let client = layout_host::client_rect(hwnd);
    let taken = (settings::SIDEBAR_DEFAULT_WIDTH + SPLITTER_THICKNESS).min(client.w);
    Rect {
        x: taken,
        y: client.y,
        w: (client.w - taken).max(0),
        h: client.h,
    }
}

/// 사이드바와 활성 워크스페이스 탐색기를 현재 창 크기에 맞춰 배치한다 (WM_SIZE·생성 직후 공용)
fn layout_children(hwnd: HWND) {
    let client = layout_host::client_rect(hwnd);
    let sidebar_w = settings::SIDEBAR_DEFAULT_WIDTH.min(client.w);
    let area = explorer_area(hwnd);
    let Some(mut state) = state_of(hwnd) else {
        return;
    };
    // 안전성: 우리가 만든 살아있는 자식 창 이동
    unsafe {
        let _ = MoveWindow(state.sidebar.hwnd(), 0, 0, sidebar_w, client.h, true);
    }
    if let Some(host) = state.active_host() {
        host.set_area(area);
        host.relayout();
    }
}

/// 세션에서 워크스페이스 목록·엔트리를 복원한다. 세션이 없거나 목록이 비면 기본 1개 목록과
/// 빈 엔트리를 돌려주고, 호출부가 탐색기를 즉시 만든다
fn restore_workspaces(session: Option<&Session>) -> (WorkspaceList, Vec<EntryState>) {
    let restored = session.and_then(|s| {
        let names = s.workspaces.iter().map(|w| w.name.clone()).collect();
        WorkspaceList::from_names(names, s.active_workspace).map(|list| (list, s))
    });
    let Some((mut list, s)) = restored else {
        return (WorkspaceList::new(), Vec::new());
    };
    // 미방문 워크스페이스의 부제는 살아있는 패널이 없으므로 저장 데이터에서 계산한다 (D18)
    for (index, ws) in s.workspaces.iter().enumerate() {
        if let Some(path) = pending_subtitle(ws) {
            list.set_subtitle(index, &path);
        }
    }
    let entries = s
        .workspaces
        .iter()
        .cloned()
        .map(EntryState::Pending)
        .collect();
    (list, entries)
}

/// 미방문 워크스페이스의 부제 경로 — 저장된 활성 패널의 활성 탭 (D18)
fn pending_subtitle(ws: &WorkspaceSession) -> Option<PathBuf> {
    ws.panels
        .get(ws.active_panel)
        .and_then(|p| p.tabs.get(p.active_tab))
        .map(PathBuf::from)
}

/// 활성 워크스페이스의 부제를 활성 패널·활성 탭 경로로 갱신한다 (D6).
/// 패널 질의는 상태 차용을 놓은 뒤 수행한다 (동기 SendMessage 중 재진입 대비)
fn refresh_subtitle(hwnd: HWND) {
    let target = {
        let Some(state) = state_of(hwnd) else {
            return;
        };
        state
            .active_host_ref()
            .and_then(LayoutHost::active_hwnd)
            .map(|panel| (state.list.active_index(), panel))
    };
    let Some((index, panel)) = target else {
        return;
    };
    let mut data = PanelSessionData::default();
    send_ptr(
        panel,
        WM_APP_SESSION_COLLECT,
        &mut data as *mut PanelSessionData as isize,
    );
    let Some(path) = data.tabs.get(data.active).cloned() else {
        return;
    };
    let Some(mut state) = state_of(hwnd) else {
        return;
    };
    state.list.set_subtitle(index, &path);
    let active = state.list.active_index();
    state.sidebar.set_items(state.list.items(), active);
}

/// 사이드바 표시 스냅숏을 현재 목록으로 갱신한다
fn sync_sidebar(hwnd: HWND) {
    let Some(state) = state_of(hwnd) else {
        return;
    };
    let active = state.list.active_index();
    state.sidebar.set_items(state.list.items(), active);
}

/// 워크스페이스 전환 (FR-17) — 미방문이면 먼저 승격하고, 실패하면 전환하지 않는다
fn switch_workspace(hwnd: HWND, index: usize) {
    {
        let Some(mut state) = state_of(hwnd) else {
            return;
        };
        if index >= state.entries.len() || index == state.list.active_index() {
            return; // 같은 워크스페이스 재선택은 무시
        }
        if matches!(state.entries[index], EntryState::Pending(_)) {
            state.materialize(hwnd, index);
            if matches!(state.entries[index], EntryState::Pending(_)) {
                return; // 창 생성 실패 — 이전 워크스페이스 유지 (조용한 저하)
            }
        }
        if let Some(previous) = state.active_host() {
            previous.end_drag(false); // 전환 중 스플리터 드래그 정리
            previous.set_visible(false);
        }
        state.list.set_active(index);
        let area = explorer_area(hwnd);
        if let Some(host) = state.active_host() {
            host.set_area(area);
            host.relayout();
            host.set_visible(true);
        }
        let count = state.active_panel_count();
        menu::update_close_enabled(state.menu, count);
    }
    sync_sidebar(hwnd);
    refresh_subtitle(hwnd);
}

/// 새 워크스페이스 (FR-16) — 홈 폴더 1패널로 만들고 즉시 전환한다.
/// 생성 직후 인라인 이름 편집으로 들어가는 것은 T6
fn new_workspace(hwnd: HWND) {
    {
        let area = explorer_area(hwnd);
        let Some(mut state) = state_of(hwnd) else {
            return;
        };
        let Ok(host) = LayoutHost::new(hwnd, area) else {
            return; // 창 생성 실패 — 목록도 바꾸지 않는다
        };
        if let Some(previous) = state.active_host() {
            previous.end_drag(false);
            previous.set_visible(false);
        }
        let index = state.list.add(); // 목록 끝에 추가되고 활성이 된다
        state.entries.insert(index, EntryState::Live(host));
        let count = state.active_panel_count();
        menu::update_close_enabled(state.menu, count);
    }
    sync_sidebar(hwnd);
    refresh_subtitle(hwnd);
}

/// 경로 변경 알림의 발신 패널이 활성 워크스페이스의 활성 패널인가 (D6 규칙 1)
fn is_active_panel(hwnd: HWND, source: HWND) -> bool {
    let Some(state) = state_of(hwnd) else {
        return false;
    };
    state
        .active_host_ref()
        .and_then(LayoutHost::active_hwnd)
        .is_some_and(|active| active == source)
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

/// 방문한 워크스페이스 하나의 상태 수집 — 분할 구조·패널별 탭·활성 패널 인덱스.
/// 패널 HWND 순서는 layout 리프 walk 순서와 1:1이다 (layout_host 계약)
fn collect_workspace(host: &LayoutHost, name: &str) -> WorkspaceSession {
    let (shape, hwnds) = host.session_snapshot();
    let mut panels = Vec::with_capacity(hwnds.len());
    for panel in &hwnds {
        let mut data = PanelSessionData::default();
        // 패널 RefCell은 메인 창 것과 별개 — 동기 수집 안전 (send_to 주석 참조)
        send_ptr(
            *panel,
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
    // 활성 패널은 walk 순서상의 위치로 저장한다 (부제 산출 전용 — D18)
    let active_panel = host
        .active_hwnd()
        .and_then(|active| hwnds.iter().position(|h| *h == active))
        .unwrap_or(0);
    WorkspaceSession {
        name: name.to_string(),
        layout: LayoutNode::from_shape(&shape),
        panels,
        active_panel,
    }
}

/// WM_CLOSE — 워크스페이스 전체·창 위치를 수집해 저장 (FR-11·FR-20, D15: 종료 시 1회)
fn save_current_session(hwnd: HWND) {
    let Some(state) = state_of(hwnd) else {
        return;
    };
    // 방문한 워크스페이스는 실제 상태를 수집하고, 미방문은 보관 중이던 데이터를 그대로 다시 저장한다
    // (이름은 항상 목록 기준 — 이름 변경이 목록에만 반영되기 때문. list와 entries는 인덱스 1:1)
    let workspaces: Vec<WorkspaceSession> = state
        .list
        .items()
        .iter()
        .zip(&state.entries)
        .map(|(item, entry)| match entry {
            EntryState::Live(host) => collect_workspace(host, &item.name),
            EntryState::Pending(ws) => WorkspaceSession {
                name: item.name.clone(),
                ..ws.clone()
            },
        })
        .collect();
    if workspaces.is_empty() {
        return; // 저장할 것이 없다 (정상 경로에서는 발생하지 않음)
    }
    let active_workspace = state.list.active_index().min(workspaces.len() - 1);
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
        // 사이드바 폭·접힘은 T7에서 실제 값으로 채운다
        sidebar: settings::SidebarSession::default(),
        active_workspace,
        workspaces,
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
            // 사이드바 배치 + 탐색기 영역 주입 → 배치
            // (relayout은 더 이상 부모 클라이언트를 스스로 읽지 않는다)
            layout_children(hwnd);
            LRESULT(0)
        }
        WM_COMMAND => {
            let Some(mut state) = state_of(hwnd) else {
                return LRESULT(0);
            };
            // 모든 전역 명령은 **활성 워크스페이스**의 탐색기에만 적용된다 (FR-17)
            match loword(wparam.0) {
                IDM_SPLIT_H => {
                    if let Some(host) = state.active_host() {
                        let _ = host.split_active(hwnd, SplitDir::Horizontal);
                    }
                }
                IDM_SPLIT_V => {
                    if let Some(host) = state.active_host() {
                        let _ = host.split_active(hwnd, SplitDir::Vertical);
                    }
                }
                IDM_CLOSE_PANE => {
                    if let Some(host) = state.active_host() {
                        host.close_active();
                    }
                }
                // 전역 네비게이션·새 탭·트리 토글·새로 고침 → 활성 패널로 게시
                // (Post — 차용 해제 후 처리됨)
                id @ (IDM_NAV_BACK | IDM_NAV_FORWARD | IDM_NAV_UP | IDM_TAB_NEW
                | IDM_TREE_TOGGLE | IDM_REFRESH) => {
                    if let Some(panel) = state.active_host().and_then(|h| h.active_hwnd()) {
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
                    let panel = state.active_host().and_then(|h| h.active_hwnd());
                    if let Some(panel) = panel
                        && send_to(panel, WM_APP_TAB_CLOSE).0 == 0
                        && let Some(host) = state.active_host()
                    {
                        host.close_active();
                    }
                }
                _ => return def_proc(hwnd, msg, wparam, lparam),
            }
            let count = state.active_panel_count();
            menu::update_close_enabled(state.menu, count);
            LRESULT(0)
        }
        WM_INITMENUPOPUP => {
            if let Some(state) = state_of(hwnd) {
                let count = state.active_panel_count();
                menu::update_close_enabled(state.menu, count);
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONDOWN => {
            if let Some(mut state) = state_of(hwnd) {
                let (x, y) = coords(lparam);
                if let Some(host) = state.active_host()
                    && host.begin_drag(hwnd, x, y)
                {
                    return LRESULT(0);
                }
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEMOVE => {
            if let Some(mut state) = state_of(hwnd) {
                let (x, y) = coords(lparam);
                if let Some(host) = state.active_host()
                    && host.drag_move(x, y)
                {
                    return LRESULT(0);
                }
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            if let Some(mut state) = state_of(hwnd)
                && let Some(host) = state.active_host()
            {
                host.end_drag(true);
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_CAPTURECHANGED => {
            // 드래그 중 캡처 상실(Alt+Tab 등) → 드래그 상태 정리 (plan T3 Edge)
            if let Some(mut state) = state_of(hwnd)
                && let Some(host) = state.active_host()
            {
                host.end_drag(false);
            }
            LRESULT(0)
        }
        WM_SETCURSOR => {
            if let Some(state) = state_of(hwnd)
                && let Some(host) = state.active_host_ref()
            {
                if host.apply_drag_cursor() {
                    return LRESULT(1);
                }
                if loword(lparam.0 as usize) == HTCLIENT
                    && let Some((x, y)) = cursor_in_client(hwnd)
                    && host.apply_splitter_cursor(x, y)
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
            if loword(wparam.0) == WM_LBUTTONDOWN {
                if let Some(mut state) = state_of(hwnd)
                    && let Some(host) = state.active_host()
                {
                    let (x, y) = coords(lparam);
                    host.set_active_by_point(x, y);
                }
                // 활성 패널이 바뀌었을 수 있으므로 부제를 다시 계산한다 (D6 규칙 2)
                refresh_subtitle(hwnd);
            }
            def_proc(hwnd, msg, wparam, lparam)
        }
        WM_APP_WS_SELECT => {
            switch_workspace(hwnd, wparam.0);
            LRESULT(0)
        }
        WM_APP_WS_NEW => {
            new_workspace(hwnd);
            LRESULT(0)
        }
        WM_APP_PATH_CHANGED => {
            // 활성 워크스페이스의 활성 패널이 보낸 것만 부제에 반영한다 (D6 규칙 1)
            let source = HWND(wparam.0 as *mut core::ffi::c_void);
            if is_active_panel(hwnd, source) {
                refresh_subtitle(hwnd);
            }
            LRESULT(0)
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
