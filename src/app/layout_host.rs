//! 레이아웃 호스트 — LayoutTree(순수 로직)를 실제 자식 HWND 배치로 연결
//!
//! 스플리터는 별도 HWND가 아니라 부모 클라이언트 영역의 빈 틈을 히트테스트한다
//! (plan T3 Design ④ — 창 수 절약).
use crate::app::layout::{
    ComputedLayout, LayoutError, LayoutTree, NodePath, PanelId, Rect, SplitDir,
};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_BTNFACE, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    BeginDeferWindowPos, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DeferWindowPos, DestroyWindow, EndDeferWindowPos, GetClientRect, IDC_ARROW, IDC_SIZENS,
    IDC_SIZEWE, LoadCursorW, RegisterClassExW, SWP_NOACTIVATE, SWP_NOZORDER, SetCursor,
    WINDOW_EX_STYLE, WNDCLASSEXW, WS_CHILD, WS_CLIPSIBLINGS, WS_VISIBLE,
};
use windows::core::{PCWSTR, Result, w};

const PANE_CLASS: PCWSTR = w!("FileExplorerPane");

/// 스플리터 드래그 진행 상태
struct DragState {
    path: NodePath,
    dir: SplitDir,
    node_area: Rect,
}

/// 분할 레이아웃과 패널 자식 창들을 소유·배치한다
pub struct LayoutHost {
    tree: LayoutTree,
    /// 리프 id → 패널 자식 HWND (T4에서 실제 Panel로 대체되는 자리표시 창)
    panes: Vec<(PanelId, HWND)>,
    active: PanelId,
    drag: Option<DragState>,
    /// 마지막 compute_rects 결과 캐시 — 히트테스트·커서 판정용
    layout_cache: ComputedLayout,
}

impl LayoutHost {
    /// 첫 패널 1개로 시작
    pub fn new(parent: HWND) -> Result<LayoutHost> {
        register_pane_class()?;
        let (tree, first) = LayoutTree::new();
        let hwnd = create_pane(parent)?;
        let mut host = LayoutHost {
            tree,
            panes: vec![(first, hwnd)],
            active: first,
            drag: None,
            layout_cache: ComputedLayout {
                panes: Vec::new(),
                splitters: Vec::new(),
            },
        };
        host.relayout(parent);
        Ok(host)
    }

    pub fn panel_count(&self) -> usize {
        self.tree.panel_count()
    }

    /// 활성 패널을 지정 방향으로 분할한다. 최소 크기 미달이면 무시(상태 유지 — plan T3 Edge)
    pub fn split_active(&mut self, parent: HWND, dir: SplitDir) -> Result<()> {
        let area = client_rect(parent);
        match self.tree.split(self.active, dir, area) {
            Ok(new_id) => {
                let hwnd = create_pane(parent)?;
                self.panes.push((new_id, hwnd));
                self.active = new_id;
                self.relayout(parent);
                Ok(())
            }
            Err(LayoutError::TooSmall) | Err(LayoutError::NotFound) => Ok(()),
            Err(LayoutError::LastPanel) => Ok(()), // split에서는 발생하지 않음
        }
    }

    /// 활성 패널을 닫는다. 마지막 1개면 무시 (메뉴는 비활성이지만 단축키 방어)
    pub fn close_active(&mut self, parent: HWND) {
        if self.tree.close(self.active).is_err() {
            return;
        }
        if let Some(pos) = self.panes.iter().position(|(id, _)| *id == self.active) {
            let (_, hwnd) = self.panes.remove(pos);
            // 안전성: 우리가 만든 자식 창 핸들 파괴 — 이후 참조 없음
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
        // 남은 패널 중 첫 번째를 활성으로
        if let Some((id, _)) = self.panes.first() {
            self.active = *id;
        }
        self.relayout(parent);
    }

    /// 좌표(부모 클라이언트)의 패널을 활성으로 지정 (WM_PARENTNOTIFY 클릭 배선)
    pub fn set_active_by_point(&mut self, x: i32, y: i32) {
        if let Some((id, _)) = self
            .layout_cache
            .panes
            .iter()
            .find(|(_, r)| contains(*r, x, y))
        {
            self.active = *id;
        }
    }

    /// 현재 트리 기준으로 모든 패널 자식 창을 일괄 배치한다
    pub fn relayout(&mut self, parent: HWND) {
        let area = client_rect(parent);
        self.layout_cache = self.tree.compute_rects(area);
        // 안전성: DeferWindowPos 일괄 배치 — 모든 핸들은 살아있는 자식 창.
        // 실패 시 hdwp가 무효화될 수 있으므로(공식 문서) 배칭을 중단한다
        unsafe {
            let mut hdwp = match BeginDeferWindowPos(self.panes.len() as i32) {
                Ok(h) => h,
                Err(_) => return,
            };
            for (id, hwnd) in &self.panes {
                let Some((_, r)) = self.layout_cache.panes.iter().find(|(pid, _)| pid == id) else {
                    continue;
                };
                match DeferWindowPos(
                    hdwp,
                    *hwnd,
                    None,
                    r.x,
                    r.y,
                    r.w.max(0),
                    r.h.max(0),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                ) {
                    Ok(next) => hdwp = next,
                    Err(_) => return, // 무효 hdwp로 계속하지 않음 — 다음 relayout이 복구
                }
            }
            let _ = EndDeferWindowPos(hdwp);
        }
    }

    /// 좌표(부모 클라이언트)에서 스플리터를 찾아 드래그 시작. 잡았으면 true
    pub fn begin_drag(&mut self, parent: HWND, x: i32, y: i32) -> bool {
        let Some(sp) = self
            .layout_cache
            .splitters
            .iter()
            .find(|s| contains(s.rect, x, y))
        else {
            return false;
        };
        self.drag = Some(DragState {
            path: sp.node_path,
            dir: sp.dir,
            node_area: sp.node_area,
        });
        // 안전성: 캡처는 WM_LBUTTONUP/WM_CAPTURECHANGED에서 해제된다
        unsafe {
            SetCapture(parent);
        }
        true
    }

    /// 드래그 중 마우스 이동 → 비율 갱신·재배치. 드래그 중이었으면 true
    pub fn drag_move(&mut self, parent: HWND, x: i32, y: i32) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        let (pos, start, axis_len) = match drag.dir {
            SplitDir::Horizontal => (x, drag.node_area.x, drag.node_area.w),
            SplitDir::Vertical => (y, drag.node_area.y, drag.node_area.h),
        };
        if axis_len <= 0 {
            return true;
        }
        let ratio = (pos - start) as f32 / axis_len as f32;
        let _ = self.tree.set_ratio(drag.path, ratio, axis_len);
        self.relayout(parent);
        true
    }

    /// 드래그 종료 (버튼 업·캡처 상실 공용 — plan T3 Edge)
    pub fn end_drag(&mut self, release: bool) {
        if self.drag.take().is_some() && release {
            // 안전성: 자기 스레드가 잡은 캡처 해제
            unsafe {
                let _ = ReleaseCapture();
            }
        }
    }

    /// 좌표가 스플리터 위면 리사이즈 커서를 적용하고 true (WM_SETCURSOR 배선)
    pub fn apply_splitter_cursor(&self, x: i32, y: i32) -> bool {
        let Some(sp) = self
            .layout_cache
            .splitters
            .iter()
            .find(|s| contains(s.rect, x, y))
        else {
            return false;
        };
        set_size_cursor(sp.dir);
        true
    }

    /// 드래그 중이면 방향 커서 적용 (히트테스트 무관 — 캡처 중 커서 유지)
    pub fn apply_drag_cursor(&self) -> bool {
        let Some(drag) = &self.drag else {
            return false;
        };
        set_size_cursor(drag.dir);
        true
    }
}

/// 분할 방향에 맞는 시스템 리사이즈 커서 적용
fn set_size_cursor(dir: SplitDir) {
    let id = match dir {
        SplitDir::Horizontal => IDC_SIZEWE,
        SplitDir::Vertical => IDC_SIZENS,
    };
    // 안전성: 시스템 공유 커서 로드·적용 — 실패 시 기본 커서 유지, 해제 불필요
    unsafe {
        if let Ok(cur) = LoadCursorW(None, id) {
            SetCursor(Some(cur));
        }
    }
}

fn contains(r: Rect, x: i32, y: i32) -> bool {
    x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h
}

/// 부모 클라이언트 영역을 자체 Rect로
fn client_rect(hwnd: HWND) -> Rect {
    let mut rc = windows::Win32::Foundation::RECT::default();
    // 안전성: 유효한 창 핸들의 클라이언트 영역 조회
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    Rect {
        x: 0,
        y: 0,
        w: rc.right - rc.left,
        h: rc.bottom - rc.top,
    }
}

/// 패널 자리표시 자식 창 클래스 등록 (1회 — 중복 등록은 무해하게 무시)
fn register_pane_class() -> Result<()> {
    // 안전성: 클래스 등록 실패(중복 포함)는 치명 아님 — 첫 등록만 유효
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(pane_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as isize as *mut core::ffi::c_void),
            lpszClassName: PANE_CLASS,
            ..Default::default()
        };
        let _ = RegisterClassExW(&wc);
    }
    Ok(())
}

fn create_pane(parent: HWND) -> Result<HWND> {
    // 안전성: 부모 창에 귀속되는 자식 창 생성 — 부모 파괴 시 함께 파괴됨
    unsafe {
        let instance = GetModuleHandleW(None)?;
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PANE_CLASS,
            None,
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            0,
            0,
            Some(parent),
            None,
            Some(instance.into()),
            None,
        )
    }
}

/// 자리표시 패널 프로시저 — T4에서 파일 목록 등 실제 내용으로 대체된다
unsafe extern "system" fn pane_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // 안전성: 기본 처리 위임뿐
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
