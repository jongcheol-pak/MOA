//! 패널 컨테이너 — 파일 목록을 담는 탐색 단위 (T5 주소창·T6 탭이 여기에 얹힌다)
//!
//! 각 패널 창은 자기 상태를 GWLP_USERDATA의 `Box<RefCell<PanelState>>`로 소유한다.
//! LayoutHost는 HWND만 알고 배치한다 (plan 패널 상호 독립 원칙).
use crate::fs::enumerate::{EnumOutcome, EnumResult, WM_APP_ENUM_DONE, spawn_enumerate};
use crate::panel::file_list::FileList;
use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{LVN_COLUMNCLICK, LVN_GETDISPINFOW, NMHDR, NMLVDISPINFOW};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDC_ARROW,
    LoadCursorW, RegisterClassExW, SW_HIDE, SW_SHOW, SetWindowLongPtrW, SetWindowTextW, ShowWindow,
    WINDOW_EX_STYLE, WM_CREATE, WM_NCDESTROY, WM_NOTIFY, WM_SIZE, WNDCLASSEXW, WS_CHILD,
    WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

const PANEL_CLASS: PCWSTR = w!("FileExplorerPanel");
/// 상태 문구 라벨(STATIC) 높이
const STATUS_HEIGHT: i32 = 24;

/// 패널 내부 상태
struct PanelState {
    file_list: FileList,
    status_label: HWND,
    /// 현재 표시(요청) 중인 경로
    path: PathBuf,
    /// 열거 세대 — 낡은 결과 폐기 (plan D5)
    generation: u64,
    tx: Sender<EnumResult>,
    rx: Receiver<EnumResult>,
}

impl PanelState {
    /// 경로 이동 — 백그라운드 열거 시작, 완료 전까지 "읽는 중…" 표시
    fn navigate(&mut self, hwnd: HWND, path: PathBuf) {
        self.generation += 1;
        self.path = path.clone();
        self.file_list.clear();
        show_status(self.status_label, "읽는 중…");
        spawn_enumerate(path, self.generation, self.tx.clone(), hwnd);
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
        match result.outcome {
            EnumOutcome::Ok(entries) => {
                if entries.is_empty() {
                    show_status(self.status_label, "빈 폴더");
                } else {
                    hide_status(self.status_label);
                }
                let dir = self.path.to_string_lossy().into_owned();
                self.file_list.set_entries(dir, entries);
            }
            EnumOutcome::AccessDenied => {
                self.file_list.clear();
                show_status(self.status_label, "이 폴더에 접근할 수 없습니다");
            }
            EnumOutcome::NotFound => {
                self.file_list.clear();
                show_status(self.status_label, "경로를 찾을 수 없습니다");
            }
            EnumOutcome::Error => {
                self.file_list.clear();
                show_status(self.status_label, "폴더를 읽는 중 오류가 발생했습니다");
            }
        }
    }

    /// 패널 영역 재배치 — 상단 상태 라벨 + 나머지 목록
    fn relayout(&mut self, hwnd: HWND) {
        let mut rc = windows::Win32::Foundation::RECT::default();
        // 안전성: 유효한 자기 창 핸들 조회
        unsafe {
            let _ = GetClientRect(hwnd, &mut rc);
        }
        let w = rc.right - rc.left;
        let h = rc.bottom - rc.top;
        move_child(self.status_label, 0, 0, w, STATUS_HEIGHT);
        self.file_list.resize(0, 0, w, h);
    }
}

/// 패널 창 생성 — LayoutHost가 배치할 HWND 반환. 초기 경로는 홈 폴더(D4 각주: 첫 패널)
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

/// 초기 표시 경로 — 사용자 프로필(홈) 폴더, 없으면 C:\
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

/// GWLP_USERDATA에서 패널 상태를 빌린다 (재진입 시 None — window.rs와 동일 패턴)
fn state_of<'a>(hwnd: HWND) -> Option<std::cell::RefMut<'a, PanelState>> {
    // 안전성: 포인터는 WM_CREATE에서 넣은 Box::into_raw 산출물, WM_NCDESTROY에서 회수
    let cell =
        unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<PanelState>).as_ref() }?;
    cell.try_borrow_mut().ok()
}

/// 패널 프로시저 — 상태 수명(WM_CREATE/WM_NCDESTROY)과 목록 알림 배선
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
        WM_NOTIFY => {
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
                        }
                        _ => {}
                    }
                }
            }
            LRESULT(0)
        }
        WM_APP_ENUM_DONE => {
            if let Some(mut state) = state_of(hwnd) {
                state.on_enum_done();
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
    let file_list = FileList::create(hwnd)?;
    let status_label = create_status_label(hwnd)?;
    let (tx, rx) = channel();
    let mut state = PanelState {
        file_list,
        status_label,
        path: PathBuf::new(),
        generation: 0,
        tx,
        rx,
    };
    state.navigate(hwnd, home_dir());
    let boxed = Box::new(RefCell::new(state));
    // 안전성: 소유권을 창에 이전 — WM_NCDESTROY에서 회수
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);
    }
    Ok(())
}
