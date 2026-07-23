//! 패널 컨테이너 — 탭 스트립·주소창·파일 목록을 담는 탐색 단위 (FR-3)
//!
//! 각 패널 창은 자기 상태를 GWLP_USERDATA의 `Box<RefCell<PanelState>>`로 소유한다.
//! LayoutHost는 HWND만 알고 배치한다 (plan 패널 상호 독립 원칙).
//!
//! 탐색은 pending-커밋 모델이다: 열거가 성공했을 때만 경로·히스토리를 커밋한다.
//! 실패(삭제·권한)하면 오류 문구만 표시하고 현 위치(주소창·히스토리)는 유지된다 (T5 Edge).
use crate::fs::enumerate::{EnumOutcome, EnumResult, WM_APP_ENUM_DONE, spawn_enumerate};
use crate::fs::shell_menu;
use crate::fs::watcher::{DirWatcher, WM_APP_DIR_CHANGED};
use crate::panel::address_bar::{
    AddressBar, ID_NAV_BACK, ID_NAV_FORWARD, ID_NAV_UP, STRIP_HEIGHT, WM_APP_ADDRESS_ENTER,
    normalize_input,
};
use crate::panel::file_list::{FileList, apply_item_count};
use crate::panel::folder_tree::{FolderTree, TREE_WIDTH, apply_expand};
use crate::panel::tabs::{CloseOutcome, TAB_HEIGHT, TabState, TabStrip, TabsModel, tab_title};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    LVN_COLUMNCLICK, LVN_GETDISPINFOW, NM_DBLCLK, NMHDR, NMITEMACTIVATE, NMLVDISPINFOW,
    NMTREEVIEWW, TCN_SELCHANGE, TVN_ITEMEXPANDINGW, TVN_SELCHANGEDW,
};
use windows::Win32::UI::Shell::{SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetClientRect, GetCursorPos, GetWindowLongPtrW,
    IDC_ARROW, LoadCursorW, RegisterClassExW, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, WINDOW_EX_STYLE, WM_APP, WM_COMMAND, WM_CONTEXTMENU, WM_CREATE,
    WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR, WM_NCDESTROY, WM_NOTIFY, WM_SIZE,
    WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

const PANEL_CLASS: PCWSTR = w!("FileExplorerPanel");
/// 상태 문구 라벨(STATIC) 높이
const STATUS_HEIGHT: i32 = 24;

/// 메인 창(전역 단축키 Alt+←/→/↑)이 활성 패널로 게시하는 네비게이션 명령
pub const WM_APP_NAV_BACK: u32 = WM_APP + 3;
pub const WM_APP_NAV_FORWARD: u32 = WM_APP + 4;
pub const WM_APP_NAV_UP: u32 = WM_APP + 5;
/// 탭 명령 (Ctrl+T / Ctrl+W). TAB_CLOSE는 SendMessage 반환값 1=처리, 0=마지막 탭(패널 닫기로 연결)
pub const WM_APP_TAB_NEW: u32 = WM_APP + 6;
pub const WM_APP_TAB_CLOSE: u32 = WM_APP + 7;
/// 폴더 트리 표시/숨김 토글 (메뉴 "보기 > 폴더 트리" — 패널별, FR-9)
pub const WM_APP_TREE_TOGGLE: u32 = WM_APP + 8;
/// 현재 폴더 재열거 — F5 (FR-12 잔여). WM_APP+9는 watcher::WM_APP_DIR_CHANGED가 사용
pub const WM_APP_REFRESH: u32 = WM_APP + 10;

/// 성공 시 히스토리에 반영할 동작 종류
enum PendingKind {
    /// 새 이동 — push
    Push,
    /// 뒤로 — 성공 시 커서 이동
    Back,
    /// 앞으로 — 성공 시 커서 이동
    Forward,
    /// 새로고침 등 — 히스토리 불변
    Keep,
}

/// 패널 내부 상태 — 경로·히스토리는 탭별(TabsModel)로 소유 (FR-3)
struct PanelState {
    tab_strip: TabStrip,
    address_bar: AddressBar,
    file_list: FileList,
    folder_tree: FolderTree,
    status_label: HWND,
    tabs: TabsModel,
    /// 진행 중 탐색 대상 (성공 시 활성 탭에 커밋)
    pending: Option<(PathBuf, PendingKind)>,
    /// 열거 세대 — 낡은 결과 폐기 (plan D5)
    generation: u64,
    tx: Sender<EnumResult>,
    rx: Receiver<EnumResult>,
    /// 활성 탭 폴더 변경 감시 — 커밋 경로가 바뀔 때 재시작 (FR-10)
    watcher: Option<DirWatcher>,
    watch_tx: Sender<()>,
    watch_rx: Receiver<()>,
}

impl PanelState {
    /// 탐색 시작 — pending에 담고 백그라운드 열거. 성공 시 on_enum_done이 커밋한다
    fn navigate(&mut self, hwnd: HWND, path: PathBuf, kind: PendingKind) {
        self.generation += 1;
        self.file_list.clear();
        show_status(self.status_label, "읽는 중…");
        spawn_enumerate(path.clone(), self.generation, self.tx.clone(), hwnd);
        self.pending = Some((path, kind));
    }

    fn nav_back(&mut self, hwnd: HWND) {
        if let Some(target) = self
            .tabs
            .active()
            .history
            .peek_back()
            .map(Path::to_path_buf)
        {
            self.navigate(hwnd, target, PendingKind::Back);
        }
    }

    fn nav_forward(&mut self, hwnd: HWND) {
        if let Some(target) = self
            .tabs
            .active()
            .history
            .peek_forward()
            .map(Path::to_path_buf)
        {
            self.navigate(hwnd, target, PendingKind::Forward);
        }
    }

    fn nav_up(&mut self, hwnd: HWND) {
        if let Some(parent) = self.tabs.active().committed.parent().map(Path::to_path_buf) {
            self.navigate(hwnd, parent, PendingKind::Push);
        }
        // 드라이브 루트(상위 없음)는 무시 — 버튼도 비활성 (D11)
    }

    /// 주소창 Enter — 입력 정규화 후 이동
    fn nav_enter(&mut self, hwnd: HWND) {
        let input = self.address_bar.text();
        if let Some(path) = normalize_input(&self.tabs.active().committed, &input) {
            self.navigate(hwnd, path, PendingKind::Push);
        }
    }

    /// 새 탭 — 현재 활성 탭의 경로 복제 (plan D4)
    fn tab_new(&mut self, hwnd: HWND) {
        let path = self.tabs.active().committed.clone();
        let index = self.tabs.add(TabState::new(path.clone()));
        self.tab_strip.insert(index, &tab_title(&path));
        self.tab_strip.set_selection(index);
        self.address_bar.set_path(&path);
        self.navigate(hwnd, path, PendingKind::Keep);
        self.update_nav_state();
    }

    /// 활성 탭 닫기 — 처리했으면 true, 마지막 탭이면 false (패널 닫기로 연결 — 호출부 몫)
    fn tab_close(&mut self, hwnd: HWND) -> bool {
        let old = self.tabs.active_index();
        match self.tabs.close_active() {
            CloseOutcome::Removed(new_active) => {
                self.tab_strip.remove(old);
                self.tab_strip.set_selection(new_active);
                let path = self.tabs.active().committed.clone();
                self.address_bar.set_path(&path);
                self.navigate(hwnd, path, PendingKind::Keep);
                self.update_nav_state();
                true
            }
            CloseOutcome::LastTab => false,
        }
    }

    /// 탭 전환 — 해당 탭의 커밋 경로를 다시 열거해 표시 (탭별 히스토리 유지)
    fn tab_switch(&mut self, hwnd: HWND, index: usize) {
        if self.tabs.switch(index) {
            let path = self.tabs.active().committed.clone();
            self.address_bar.set_path(&path);
            self.navigate(hwnd, path, PendingKind::Keep);
            self.update_nav_state();
        }
    }

    /// 목록 더블클릭 — 폴더 진입 / 파일 실행 (FR-7)
    fn activate_item(&mut self, hwnd: HWND, index: i32) {
        if index < 0 {
            return;
        }
        let Some(entry) = self.file_list.entry_at(index as usize) else {
            return;
        };
        let name = entry.name_string();
        let is_dir = entry.is_dir;
        let full = self.tabs.active().committed.join(&name);
        if is_dir {
            self.navigate(hwnd, full, PendingKind::Push);
        } else {
            shell_open(&full);
        }
    }

    /// 열거 완료 통지 처리 — 채널을 비우고 현재 세대 결과만 반영
    fn on_enum_done(&mut self, hwnd: HWND) {
        let mut latest: Option<EnumResult> = None;
        while let Ok(r) = self.rx.try_recv() {
            if r.generation == self.generation {
                latest = Some(r);
            }
            // 세대 불일치 결과는 폐기 (탭 전환·연속 이동 중 낡은 응답)
        }
        let Some(result) = latest else {
            return;
        };
        let Some((target, kind)) = self.pending.take() else {
            return;
        };
        match result.outcome {
            EnumOutcome::Ok(entries) => {
                // 성공 — 활성 탭에 경로·히스토리 커밋
                let tab = self.tabs.active_mut();
                match kind {
                    PendingKind::Push => tab.history.push(target.clone()),
                    PendingKind::Back => {
                        let _ = tab.history.back();
                    }
                    PendingKind::Forward => {
                        let _ = tab.history.forward();
                    }
                    PendingKind::Keep => {}
                }
                tab.committed = target;
                let committed = tab.committed.clone();
                self.address_bar.set_path(&committed);
                self.tab_strip
                    .set_title(self.tabs.active_index(), &tab_title(&committed));
                if entries.is_empty() {
                    show_status(self.status_label, "빈 폴더");
                } else {
                    hide_status(self.status_label);
                }
                let dir = committed.to_string_lossy().into_owned();
                self.file_list.set_entries(dir, entries);
            }
            EnumOutcome::AccessDenied => {
                self.fail_pending("이 폴더에 접근할 수 없습니다");
            }
            EnumOutcome::NotFound => {
                self.fail_pending("경로를 찾을 수 없습니다");
            }
            EnumOutcome::Error => {
                self.fail_pending("폴더를 읽는 중 오류가 발생했습니다");
            }
        }
        self.update_nav_state();
        self.sync_watcher(hwnd);
    }

    /// 감시 대상을 활성 탭 커밋 경로에 맞춘다 — 다르면 이전 감시 정지 후 재시작.
    /// 이전 워커는 Drop이 정지 신호·join으로 회수한다 (탭 고속 전환 누수 금지 — T3 Edge)
    fn sync_watcher(&mut self, hwnd: HWND) {
        let target = self.tabs.active().committed.clone();
        if self
            .watcher
            .as_ref()
            .is_some_and(|w| w.path() == target.as_path())
        {
            return;
        }
        self.watcher = None;
        self.watcher = Some(DirWatcher::start(target, self.watch_tx.clone(), Some(hwnd)));
    }

    /// 현재 폴더 재열거 (F5·변경 감시 공용). 탐색 진행 중이면 생략 — pending 완료가 우선
    fn refresh(&mut self, hwnd: HWND) {
        if self.pending.is_some() {
            return;
        }
        let path = self.tabs.active().committed.clone();
        self.navigate(hwnd, path, PendingKind::Keep);
    }

    /// 실패 — 현 위치(활성 탭 committed·히스토리) 유지, 오류 문구 표시, 주소창 복원
    fn fail_pending(&mut self, message: &str) {
        self.file_list.clear();
        show_status(self.status_label, message);
        self.address_bar.set_path(&self.tabs.active().committed);
    }

    fn update_nav_state(&self) {
        let tab = self.tabs.active();
        self.address_bar.set_nav_state(
            tab.history.can_back(),
            tab.history.can_forward(),
            tab.committed.parent().is_some(),
        );
    }

    /// 패널 영역 재배치 — 탭 스트립 + 주소창 스트립 + 상태 라벨 + 목록
    fn relayout(&mut self, hwnd: HWND) {
        let mut rc = windows::Win32::Foundation::RECT::default();
        // 안전성: 유효한 자기 창 핸들 조회
        unsafe {
            let _ = GetClientRect(hwnd, &mut rc);
        }
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        self.tab_strip.resize(0, 0, w, TAB_HEIGHT);
        self.address_bar.layout_at(TAB_HEIGHT, w);
        let top = TAB_HEIGHT + STRIP_HEIGHT;
        // 트리 표시 시 좌측 고정폭을 차지하고 상태·목록은 우측으로 밀린다 (FR-9)
        let (content_x, content_w) = if self.folder_tree.visible() {
            let tree_w = TREE_WIDTH.min(w);
            self.folder_tree.resize(0, top, tree_w, (h - top).max(0));
            (tree_w, (w - tree_w).max(0))
        } else {
            (0, w)
        };
        move_child(self.status_label, content_x, top, content_w, STATUS_HEIGHT);
        self.file_list
            .resize(content_x, top, content_w, (h - top).max(0));
    }
}

/// 패널 창 생성 — LayoutHost가 배치할 HWND 반환
pub fn create(parent: HWND) -> Result<HWND> {
    register_class()?;
    // 안전성: 자식 창 생성 — WM_CREATE에서 상태를 부착한다
    unsafe {
        let instance = GetModuleHandleW(None)?;
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PANEL_CLASS,
            None,
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN,
            0,
            0,
            0,
            0,
            Some(parent),
            None,
            Some(instance.into()),
            None,
        )
    }
}

/// 초기 표시 경로 — 사용자 프로필(홈) 폴더, 없으면 C:\ (plan D4 각주: 첫 패널)
fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\"))
}

fn register_class() -> Result<()> {
    // 안전성: 클래스 중복 등록은 무해 (첫 등록만 유효)
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(panel_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as isize as *mut core::ffi::c_void),
            lpszClassName: PANEL_CLASS,
            ..Default::default()
        };
        let _ = RegisterClassExW(&wc);
    }
    Ok(())
}

/// 상태 라벨(STATIC) 생성
fn create_status_label(parent: HWND) -> Result<HWND> {
    // 안전성: 표준 STATIC 자식 생성
    unsafe {
        let instance = GetModuleHandleW(None)?;
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            None,
            WS_CHILD | WS_CLIPSIBLINGS,
            0,
            0,
            0,
            STATUS_HEIGHT,
            Some(parent),
            None,
            Some(instance.into()),
            None,
        )
    }
}

fn show_status(label: HWND, text: &str) {
    // 안전성: 유효한 라벨 핸들에 텍스트·표시 설정
    unsafe {
        let _ = SetWindowTextW(label, &HSTRING::from(text));
        let _ = ShowWindow(label, SW_SHOW);
    }
}

fn hide_status(label: HWND) {
    // 안전성: 유효한 라벨 핸들 숨김
    unsafe {
        let _ = ShowWindow(label, SW_HIDE);
    }
}

fn move_child(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    // 안전성: 자식 창 이동
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::MoveWindow(
            hwnd,
            x,
            y,
            w.max(0),
            h.max(0),
            true,
        );
    }
}

/// 파일을 연결 프로그램으로 실행 (FR-7). 실패 UI는 셸에 위임 (T5 Edge)
fn shell_open(path: &std::path::Path) {
    let file = HSTRING::from(path.to_string_lossy().as_ref());
    let mut sei = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        lpVerb: w!("open"),
        lpFile: PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // 안전성: sei·file은 호출 동안 살아있는 스택/지역 소유. 실패는 셸이 UI로 알림
    unsafe {
        let _ = ShellExecuteExW(&mut sei);
    }
}

/// GWLP_USERDATA에서 패널 상태를 빌린다 (재진입 시 None — window.rs와 동일 패턴)
fn state_of<'a>(hwnd: HWND) -> Option<std::cell::RefMut<'a, PanelState>> {
    // 안전성: 포인터는 WM_CREATE에서 넣은 Box::into_raw 산출물, WM_NCDESTROY에서 회수
    let cell =
        unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<PanelState>).as_ref() }?;
    cell.try_borrow_mut().ok()
}

fn loword(v: usize) -> u32 {
    (v & 0xffff) as u32
}

/// WM_CONTEXTMENU 대상이 파일 목록이면 (현재 폴더, 선택 경로들, 화면 좌표)를 수집한다.
/// RefCell 차용은 이 함수 안에서 끝난다 — 호출부는 차용 없이 모달 메뉴를 연다
fn collect_context_menu_request(
    hwnd: HWND,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<(PathBuf, Vec<PathBuf>, i32, i32)> {
    let state = state_of(hwnd)?;
    if wparam.0 as isize != state.file_list.hwnd().0 as isize {
        return None;
    }
    let folder = state.tabs.active().committed.clone();
    let items: Vec<PathBuf> = state
        .file_list
        .selected_indices()
        .into_iter()
        .filter_map(|i| state.file_list.entry_at(i))
        .map(|e| folder.join(e.name_string()))
        .collect();
    // lparam은 화면 좌표. 키보드(메뉴 키) 호출은 -1,-1 → 커서 위치로 대체
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    if x == -1 && y == -1 {
        let mut pt = windows::Win32::Foundation::POINT::default();
        // 안전성: pt는 스택 소유
        unsafe {
            let _ = GetCursorPos(&mut pt);
        }
        return Some((folder, items, pt.x, pt.y));
    }
    Some((folder, items, x, y))
}

/// 패널 프로시저 — 상태 수명·목록/주소창 알림·네비게이션 명령 배선.
/// 목록을 바꾼 분기는 RefCell 차용을 닫은 뒤 apply_item_count로 카운트를 반영한다
unsafe extern "system" fn panel_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            match init_state(hwnd) {
                Ok(()) => LRESULT(0),
                Err(_) => LRESULT(-1), // 생성 실패 — CreateWindowExW가 Err 반환
            }
        }
        WM_SIZE => {
            if let Some(mut state) = state_of(hwnd) {
                state.relayout(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let mut apply: Option<(HWND, usize)> = None;
            if let Some(mut state) = state_of(hwnd) {
                match loword(wparam.0) {
                    ID_NAV_BACK => state.nav_back(hwnd),
                    ID_NAV_FORWARD => state.nav_forward(hwnd),
                    ID_NAV_UP => state.nav_up(hwnd),
                    _ => {}
                }
                apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            // 카운트 반영은 차용 스코프 밖에서 — file_list::apply_item_count 주석 참조
            let mut apply: Option<(HWND, usize)> = None;
            // TVN_ITEMEXPANDING 보류(1) 반환용
            let mut result = LRESULT(0);
            if let Some(mut state) = state_of(hwnd) {
                // 안전성: WM_NOTIFY의 lparam은 OS가 채운 NMHDR 포인터 (처리 동안 유효)
                let hdr = unsafe { &*(lparam.0 as *const NMHDR) };
                if hdr.hwndFrom == state.tab_strip.hwnd() && hdr.code == TCN_SELCHANGE {
                    let sel = state.tab_strip.selection();
                    if sel >= 0 {
                        state.tab_switch(hwnd, sel as usize);
                        apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
                    }
                } else if hdr.hwndFrom == state.folder_tree.hwnd() {
                    match hdr.code {
                        TVN_ITEMEXPANDINGW => {
                            // 안전성: TVN_*의 lparam은 NMTREEVIEWW
                            let nmtv = unsafe { &*(lparam.0 as *const NMTREEVIEWW) };
                            if state.folder_tree.on_expanding(nmtv, hwnd) {
                                result = LRESULT(1); // 확장 보류 — 열거 완료 후 apply_expand
                            }
                        }
                        TVN_SELCHANGEDW => {
                            // 안전성: TVN_*의 lparam은 NMTREEVIEWW
                            let nmtv = unsafe { &*(lparam.0 as *const NMTREEVIEWW) };
                            if let Some(path) = state.folder_tree.on_sel_changed(nmtv) {
                                state.navigate(hwnd, path, PendingKind::Push);
                                apply =
                                    Some((state.file_list.hwnd(), state.file_list.item_count()));
                            }
                        }
                        _ => {}
                    }
                } else if hdr.hwndFrom == state.file_list.hwnd() {
                    match hdr.code {
                        LVN_GETDISPINFOW => {
                            // 안전성: LVN_GETDISPINFOW의 lparam은 NMLVDISPINFOW
                            let info = unsafe { &mut *(lparam.0 as *mut NMLVDISPINFOW) };
                            state.file_list.on_get_disp_info(info);
                        }
                        LVN_COLUMNCLICK => {
                            // 안전성: LVN_COLUMNCLICK의 lparam은 NMLISTVIEW
                            let nmlv = unsafe {
                                &*(lparam.0 as *const windows::Win32::UI::Controls::NMLISTVIEW)
                            };
                            state.file_list.on_column_click(nmlv.iSubItem);
                            apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
                        }
                        NM_DBLCLK => {
                            // 안전성: NM_DBLCLK(리스트뷰)의 lparam은 NMITEMACTIVATE
                            let nmia = unsafe { &*(lparam.0 as *const NMITEMACTIVATE) };
                            state.activate_item(hwnd, nmia.iItem);
                            apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
                        }
                        _ => {}
                    }
                }
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            result
        }
        WM_APP_ENUM_DONE => {
            // 목록·트리가 채널을 각자 소유하므로 둘 다 비운다 (통지 메시지는 공용)
            let mut apply: Option<(HWND, usize)> = None;
            let mut expand: Option<(HWND, Vec<isize>)> = None;
            if let Some(mut state) = state_of(hwnd) {
                state.on_enum_done(hwnd);
                let items = state.folder_tree.on_enum_done();
                if !items.is_empty() {
                    expand = Some((state.folder_tree.hwnd(), items));
                }
                apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            // 확장 실행은 차용 해제 후 — TVN_ITEMEXPANDING 동기 재진입 대비 (apply_expand 주석)
            if let Some((tree, items)) = expand {
                apply_expand(tree, &items);
            }
            LRESULT(0)
        }
        WM_APP_DIR_CHANGED => {
            // 감시 스레드 통지 — 채널을 비우고 현재 폴더 재열거 (FR-10)
            let mut apply: Option<(HWND, usize)> = None;
            if let Some(mut state) = state_of(hwnd) {
                while state.watch_rx.try_recv().is_ok() {}
                state.refresh(hwnd);
                apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            LRESULT(0)
        }
        WM_APP_ADDRESS_ENTER | WM_APP_NAV_BACK | WM_APP_NAV_FORWARD | WM_APP_NAV_UP
        | WM_APP_TAB_NEW | WM_APP_TAB_CLOSE | WM_APP_TREE_TOGGLE | WM_APP_REFRESH => {
            let mut apply: Option<(HWND, usize)> = None;
            // TAB_CLOSE: 1=탭 제거됨, 0=마지막 탭(호출부가 패널 닫기로 연결)
            let mut result = LRESULT(0);
            if let Some(mut state) = state_of(hwnd) {
                match msg {
                    WM_APP_ADDRESS_ENTER => state.nav_enter(hwnd),
                    WM_APP_NAV_BACK => state.nav_back(hwnd),
                    WM_APP_NAV_FORWARD => state.nav_forward(hwnd),
                    WM_APP_NAV_UP => state.nav_up(hwnd),
                    WM_APP_TAB_NEW => state.tab_new(hwnd),
                    WM_APP_TAB_CLOSE => {
                        result = LRESULT(if state.tab_close(hwnd) { 1 } else { 0 });
                    }
                    WM_APP_TREE_TOGGLE => {
                        state.folder_tree.toggle();
                        state.relayout(hwnd);
                    }
                    WM_APP_REFRESH => state.refresh(hwnd),
                    _ => {}
                }
                apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            result
        }
        WM_CONTEXTMENU => {
            // 파일 목록 우클릭/메뉴 키 → 셸 컨텍스트 메뉴 (FR-8).
            // 대상 수집은 차용 안에서, 모달 메뉴 표시는 차용 해제 후
            // (TrackPopupMenuEx 모달 루프 중 도착하는 메시지가 상태에 접근할 수 있게)
            match collect_context_menu_request(hwnd, wparam, lparam) {
                Some((folder, items, x, y)) => {
                    shell_menu::show_context_menu(hwnd, &folder, &items, x, y);
                    LRESULT(0)
                }
                // 대상이 파일 목록이 아니면 기본 처리 (부모로 전파)
                None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR => {
            // 셸 메뉴 모달 중이면 IContextMenu2/3로 포워딩 — 서브메뉴(보내기 등) 채움 (T2 Edge)
            match shell_menu::forward_menu_msg(msg, wparam, lparam) {
                Some(result) => result,
                // 안전성: 기본 처리 위임
                None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
            }
        }
        WM_NCDESTROY => {
            // 안전성: WM_CREATE에서 넣은 포인터를 정확히 한 번 회수
            unsafe {
                let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut RefCell<PanelState>;
                if !ptr.is_null() {
                    drop(Box::from_raw(ptr));
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
        }
        _ => {
            // 안전성: 기본 처리 위임
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }
}

/// WM_CREATE — 자식 컨트롤·상태 생성 후 첫 탐색 시작
fn init_state(hwnd: HWND) -> Result<()> {
    let tab_strip = TabStrip::create(hwnd)?;
    let address_bar = AddressBar::create(hwnd)?;
    let file_list = FileList::create(hwnd)?;
    let folder_tree = FolderTree::create(hwnd)?;
    let status_label = create_status_label(hwnd)?;
    let (tx, rx) = channel();
    let (watch_tx, watch_rx) = channel();
    let start = home_dir();
    address_bar.set_path(&start);
    tab_strip.insert(0, &tab_title(&start));
    tab_strip.set_selection(0);
    let mut state = PanelState {
        tab_strip,
        address_bar,
        file_list,
        folder_tree,
        status_label,
        tabs: TabsModel::new(TabState::new(start.clone())),
        pending: None,
        generation: 0,
        tx,
        rx,
        // 감시는 첫 열거 성공(커밋) 시 sync_watcher가 시작한다
        watcher: None,
        watch_tx,
        watch_rx,
    };
    // 첫 화면 — 시작 경로는 이미 활성 탭 히스토리에 있으므로 Keep으로 열거만
    state.navigate(hwnd, start, PendingKind::Keep);
    state.update_nav_state();
    let boxed = Box::new(RefCell::new(state));
    // 안전성: 소유권을 창에 이전 — WM_NCDESTROY에서 회수
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);
    }
    Ok(())
}
