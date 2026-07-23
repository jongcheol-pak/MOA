//! 패널 컨테이너 — 주소창·파일 목록을 담는 탐색 단위 (T6 탭이 여기에 얹힌다)
//!
//! 각 패널 창은 자기 상태를 GWLP_USERDATA의 `Box<RefCell<PanelState>>`로 소유한다.
//! LayoutHost는 HWND만 알고 배치한다 (plan 패널 상호 독립 원칙).
//!
//! 탐색은 pending-커밋 모델이다: 열거가 성공했을 때만 경로·히스토리를 커밋한다.
//! 실패(삭제·권한)하면 오류 문구만 표시하고 현 위치(주소창·히스토리)는 유지된다 (T5 Edge).
use crate::fs::enumerate::{EnumOutcome, EnumResult, WM_APP_ENUM_DONE, spawn_enumerate};
use crate::panel::address_bar::{
    AddressBar, ID_NAV_BACK, ID_NAV_FORWARD, ID_NAV_UP, STRIP_HEIGHT, WM_APP_ADDRESS_ENTER,
    normalize_input,
};
use crate::panel::file_list::{FileList, apply_item_count};
use crate::panel::history::History;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    LVN_COLUMNCLICK, LVN_GETDISPINFOW, NM_DBLCLK, NMHDR, NMITEMACTIVATE, NMLVDISPINFOW,
};
use windows::Win32::UI::Shell::{SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDC_ARROW,
    LoadCursorW, RegisterClassExW, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, SetWindowLongPtrW,
    SetWindowTextW, ShowWindow, WINDOW_EX_STYLE, WM_APP, WM_COMMAND, WM_CREATE, WM_NCDESTROY,
    WM_NOTIFY, WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

const PANEL_CLASS: PCWSTR = w!("FileExplorerPanel");
/// 상태 문구 라벨(STATIC) 높이
const STATUS_HEIGHT: i32 = 24;

/// 메인 창(전역 단축키 Alt+←/→/↑)이 활성 패널로 게시하는 네비게이션 명령
pub const WM_APP_NAV_BACK: u32 = WM_APP + 3;
pub const WM_APP_NAV_FORWARD: u32 = WM_APP + 4;
pub const WM_APP_NAV_UP: u32 = WM_APP + 5;

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

/// 패널 내부 상태
struct PanelState {
    address_bar: AddressBar,
    file_list: FileList,
    status_label: HWND,
    history: History,
    /// 커밋된(화면에 유효한) 경로
    committed: PathBuf,
    /// 진행 중 탐색 대상 (성공 시 커밋)
    pending: Option<(PathBuf, PendingKind)>,
    /// 열거 세대 — 낡은 결과 폐기 (plan D5)
    generation: u64,
    tx: Sender<EnumResult>,
    rx: Receiver<EnumResult>,
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
        if let Some(target) = self.history.peek_back().map(|p| p.to_path_buf()) {
            self.navigate(hwnd, target, PendingKind::Back);
        }
    }

    fn nav_forward(&mut self, hwnd: HWND) {
        if let Some(target) = self.history.peek_forward().map(|p| p.to_path_buf()) {
            self.navigate(hwnd, target, PendingKind::Forward);
        }
    }

    fn nav_up(&mut self, hwnd: HWND) {
        if let Some(parent) = self.committed.parent().map(|p| p.to_path_buf()) {
            self.navigate(hwnd, parent, PendingKind::Push);
        }
        // 드라이브 루트(상위 없음)는 무시 — 버튼도 비활성 (D11)
    }

    /// 주소창 Enter — 입력 정규화 후 이동
    fn nav_enter(&mut self, hwnd: HWND) {
        let input = self.address_bar.text();
        if let Some(path) = normalize_input(&self.committed, &input) {
            self.navigate(hwnd, path, PendingKind::Push);
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
        let full = self.committed.join(&name);
        if is_dir {
            self.navigate(hwnd, full, PendingKind::Push);
        } else {
            shell_open(&full);
        }
    }

    /// 열거 완료 통지 처리 — 채널을 비우고 현재 세대 결과만 반영
    fn on_enum_done(&mut self) {
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
                // 성공 — 경로·히스토리 커밋
                match kind {
                    PendingKind::Push => self.history.push(target.clone()),
                    PendingKind::Back => {
                        let _ = self.history.back();
                    }
                    PendingKind::Forward => {
                        let _ = self.history.forward();
                    }
                    PendingKind::Keep => {}
                }
                self.committed = target;
                self.address_bar.set_path(&self.committed);
                if entries.is_empty() {
                    show_status(self.status_label, "빈 폴더");
                } else {
                    hide_status(self.status_label);
                }
                let dir = self.committed.to_string_lossy().into_owned();
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
    }

    /// 실패 — 현 위치(committed·히스토리) 유지, 오류 문구 표시, 주소창 복원
    fn fail_pending(&mut self, message: &str) {
        self.file_list.clear();
        show_status(self.status_label, message);
        self.address_bar.set_path(&self.committed);
    }

    fn update_nav_state(&self) {
        self.address_bar.set_nav_state(
            self.history.can_back(),
            self.history.can_forward(),
            self.committed.parent().is_some(),
        );
    }

    /// 패널 영역 재배치 — 상단 주소창 스트립 + 상태 라벨 + 목록
    fn relayout(&mut self, hwnd: HWND) {
        let mut rc = windows::Win32::Foundation::RECT::default();
        // 안전성: 유효한 자기 창 핸들 조회
        unsafe {
            let _ = GetClientRect(hwnd, &mut rc);
        }
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        self.address_bar.layout(w);
        move_child(self.status_label, 0, STRIP_HEIGHT, w, STATUS_HEIGHT);
        self.file_list
            .resize(0, STRIP_HEIGHT, w, (h - STRIP_HEIGHT).max(0));
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
            if let Some(mut state) = state_of(hwnd) {
                // 안전성: WM_NOTIFY의 lparam은 OS가 채운 NMHDR 포인터 (처리 동안 유효)
                let hdr = unsafe { &*(lparam.0 as *const NMHDR) };
                if hdr.hwndFrom == state.file_list.hwnd() {
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
            LRESULT(0)
        }
        WM_APP_ENUM_DONE => {
            let mut apply: Option<(HWND, usize)> = None;
            if let Some(mut state) = state_of(hwnd) {
                state.on_enum_done();
                apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            LRESULT(0)
        }
        WM_APP_ADDRESS_ENTER | WM_APP_NAV_BACK | WM_APP_NAV_FORWARD | WM_APP_NAV_UP => {
            let mut apply: Option<(HWND, usize)> = None;
            if let Some(mut state) = state_of(hwnd) {
                match msg {
                    WM_APP_ADDRESS_ENTER => state.nav_enter(hwnd),
                    WM_APP_NAV_BACK => state.nav_back(hwnd),
                    WM_APP_NAV_FORWARD => state.nav_forward(hwnd),
                    WM_APP_NAV_UP => state.nav_up(hwnd),
                    _ => {}
                }
                apply = Some((state.file_list.hwnd(), state.file_list.item_count()));
            }
            if let Some((list, count)) = apply {
                apply_item_count(list, count);
            }
            LRESULT(0)
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
    let address_bar = AddressBar::create(hwnd)?;
    let file_list = FileList::create(hwnd)?;
    let status_label = create_status_label(hwnd)?;
    let (tx, rx) = channel();
    let start = home_dir();
    address_bar.set_path(&start);
    let mut state = PanelState {
        address_bar,
        file_list,
        status_label,
        history: History::new(start.clone()),
        committed: start.clone(),
        pending: None,
        generation: 0,
        tx,
        rx,
    };
    // 첫 화면 — 시작 경로는 이미 히스토리·committed에 있으므로 Keep으로 열거만
    state.navigate(hwnd, start, PendingKind::Keep);
    state.update_nav_state();
    let boxed = Box::new(RefCell::new(state));
    // 안전성: 소유권을 창에 이전 — WM_NCDESTROY에서 회수
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);
    }
    Ok(())
}
