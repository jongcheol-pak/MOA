//! 워크스페이스 사이드바 — 좌측 다크 카드 목록 (FR-15·FR-16·FR-17 표시·입력)
//!
//! 시스템 컨트롤이 아니라 **직접 그리는 자식 창**이다 (plan D3):
//! 2줄 카드·다크 배경·(이후 task의) 드래그 정렬·인라인 편집을 전부 제어해야 하기 때문.
//! 아래 시각 상수는 plan의 `## 시각 요소 분해` 표와 1:1 대응한다 (폭 토큰만 `settings`가 소유).
//!
//! 이 창은 워크스페이스를 **소유하지 않는다** — 표시 스냅숏을 받아 그리고,
//! 사용자 조작은 부모(메인 창)에 메시지로 올려보낸다.
use crate::app::workspace::Workspace;
use crate::fs::icons::IconCache;
use std::cell::{RefCell, RefMut};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, CreateSolidBrush, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_SINGLELINE, DeleteObject, DrawTextW, EndPaint, FW_SEMIBOLD, FillRect, HBRUSH, HDC, HFONT,
    InvalidateRect, PAINTSTRUCT, ScreenToClient, SelectObject, SetBkMode, SetTextColor,
    TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{EM_SETSEL, ILD_TRANSPARENT, ImageList_Draw, WM_MOUSELEAVE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_DELETE,
    VK_DOWN, VK_ESCAPE, VK_F2, VK_RETURN, VK_UP,
};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, ES_AUTOHSCROLL, GWLP_USERDATA, GetClientRect,
    GetParent, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, IDC_ARROW, LoadCursorW,
    NONCLIENTMETRICSW, PostMessageW, RegisterClassExW, SPI_GETNONCLIENTMETRICS,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SendMessageW, SetWindowLongPtrW, SystemParametersInfoW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CAPTURECHANGED, WM_CONTEXTMENU, WM_CREATE,
    WM_ERASEBKGND, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_NCDESTROY, WM_PAINT, WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN,
    WS_CLIPSIBLINGS, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{HSTRING, PCWSTR, Result, w};

const SIDEBAR_CLASS: PCWSTR = w!("FileExplorerSidebar");

/// 사이드바 → 부모(메인 창) 알림.
/// SELECT는 wparam=선택 인덱스, NEW는 인자 없음. WM_APP+13은 패널의 경로 변경 알림이 쓴다
pub const WM_APP_WS_SELECT: u32 = WM_APP + 14;
pub const WM_APP_WS_NEW: u32 = WM_APP + 15;
/// 항목 우클릭 — wparam=인덱스, lparam=화면 좌표(하위 워드 x, 상위 워드 y)
pub const WM_APP_WS_CONTEXT: u32 = WM_APP + 16;
/// 이름 변경 커밋 — lparam은 같은 스레드 SendMessage로 전달되는 `RenameRequest` 포인터.
/// 게시(Post) 금지 — 포인터가 호출 스택 소유다 (패널 세션 계약과 같은 규약)
pub const WM_APP_WS_RENAME: u32 = WM_APP + 17;
/// 삭제 요청 — wparam=인덱스
pub const WM_APP_WS_DELETE: u32 = WM_APP + 18;
/// 순서 변경 — wparam=원래 인덱스, lparam=놓인 인덱스
pub const WM_APP_WS_REORDER: u32 = WM_APP + 19;

/// 사이드바 내부 전용 — 인라인 편집 EDIT의 서브클래스가 사이드바로 보내는 커밋/취소 신호
const WM_APP_RENAME_COMMIT: u32 = WM_APP + 20;
const WM_APP_RENAME_CANCEL: u32 = WM_APP + 21;

/// 이름 변경 요청 페이로드 — 사이드바와 메인 창 사이 계약
pub struct RenameRequest {
    pub index: usize,
    pub name: String,
}

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px — D13) ──
/// 사이드바 상단 접기 토글 영역 — 토글 아이콘·동작은 T7에서 채운다
const TOGGLE_STRIP_HEIGHT: i32 = 28;
const HEADER_HEIGHT: i32 = 36;
const HEADER_TEXT: PCWSTR = w!("워크스페이스");
/// 새 워크스페이스 버튼 — 헤더 우측
const PLUS_SIZE: i32 = 24;
const PLUS_MARGIN: i32 = 8;
const ITEM_HEIGHT: i32 = 60;
const ITEM_GAP: i32 = 4;
/// 항목 하나가 차지하는 세로 간격(높이 + 아래 여백)
const ITEM_PITCH: i32 = ITEM_HEIGHT + ITEM_GAP;
const ITEM_MARGIN_X: i32 = 8;
const ACCENT_BAR_WIDTH: i32 = 3;
const ICON_SIZE: i32 = 16;
const ICON_X: i32 = 12;
const TEXT_X: i32 = 38;
const NAME_TOP: i32 = 12;
const NAME_FONT_PX: i32 = 13;
const SUBTITLE_GAP: i32 = 6;
const SUBTITLE_FONT_PX: i32 = 11;
const HEADER_FONT_PX: i32 = 12;
/// 휠 한 칸(WHEEL_DELTA 120)당 스크롤 픽셀
const WHEEL_STEP: i32 = ITEM_PITCH;
/// 드래그 정렬 시작 임계 — 이만큼 움직여야 재정렬로 본다 (D12: 단순 클릭과 구분)
const DRAG_THRESHOLD: i32 = 8;
/// 드롭 위치 삽입선 두께
const INSERT_LINE_HEIGHT: i32 = 2;

/// COLORREF는 0x00BBGGRR 순서 — 실수 방지용 헬퍼
const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
}

const COLOR_BG: COLORREF = rgb(0x1B, 0x1B, 0x1B);
const COLOR_ITEM: COLORREF = rgb(0x23, 0x23, 0x23);
const COLOR_ITEM_SELECTED: COLORREF = rgb(0x2E, 0x2E, 0x2E);
const COLOR_ITEM_HOVER: COLORREF = rgb(0x28, 0x28, 0x28);
const COLOR_ITEM_BORDER: COLORREF = rgb(0x2C, 0x2C, 0x2C);
const COLOR_ACCENT: COLORREF = rgb(0x4A, 0x9E, 0xFF);
const COLOR_NAME: COLORREF = rgb(0xE8, 0xE8, 0xE8);
const COLOR_SUBTITLE: COLORREF = rgb(0x8A, 0x8A, 0x8A);
const COLOR_HEADER: COLORREF = rgb(0x9A, 0x9A, 0x9A);
const COLOR_HEADER_HOT: COLORREF = rgb(0xE8, 0xE8, 0xE8);

/// 목록 영역이 시작하는 y (토글 영역 + 헤더 아래)
const LIST_TOP: i32 = TOGGLE_STRIP_HEIGHT + HEADER_HEIGHT;

/// 좌표 y가 가리키는 항목 인덱스 — 항목 사이 여백(gap)이면 None (순수 함수, 단위테스트 대상)
pub fn item_at(y: i32, scroll: i32, count: usize) -> Option<usize> {
    if y < LIST_TOP {
        return None;
    }
    let offset = y - LIST_TOP + scroll;
    if offset < 0 {
        return None;
    }
    let index = (offset / ITEM_PITCH) as usize;
    if index >= count || offset % ITEM_PITCH >= ITEM_HEIGHT {
        return None; // 목록 밖이거나 항목 사이 여백
    }
    Some(index)
}

/// 스크롤 오프셋을 [0, 최대]로 클램프한다 (순수 함수, 단위테스트 대상).
/// `view_h`는 창 전체 높이 — 목록 영역은 그중 LIST_TOP 아래다
pub fn clamp_scroll(scroll: i32, count: usize, view_h: i32) -> i32 {
    let content = count as i32 * ITEM_PITCH;
    let view = (view_h - LIST_TOP).max(0);
    scroll.clamp(0, (content - view).max(0))
}

/// 드래그를 놓은 y가 가리키는 삽입 위치 — 항목의 세로 중앙을 넘으면 그 다음 자리 (D12).
/// 반환값은 0..=count 범위이며 `count`는 맨 끝을 뜻한다 (순수 함수, 단위테스트 대상)
pub fn drop_index(y: i32, scroll: i32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let offset = (y - LIST_TOP + scroll).max(0);
    let slot = offset / ITEM_PITCH;
    let within = offset % ITEM_PITCH;
    let index = (slot as usize).min(count.saturating_sub(1));
    if slot as usize >= count {
        return count; // 목록 아래 빈 공간 → 맨 끝
    }
    if within >= ITEM_HEIGHT / 2 {
        index + 1
    } else {
        index
    }
}

/// 항목 카드의 사각형 (인라인 편집 EDIT 배치·그리기 공용)
fn item_rect(index: usize, scroll: i32, width: i32) -> RECT {
    let top = LIST_TOP + index as i32 * ITEM_PITCH - scroll;
    RECT {
        left: ITEM_MARGIN_X,
        top,
        right: (width - ITEM_MARGIN_X).max(ITEM_MARGIN_X),
        bottom: top + ITEM_HEIGHT,
    }
}

/// 사이드바 창 래퍼 — 상태는 창이 소유한다(GWLP_USERDATA)
pub struct Sidebar {
    hwnd: HWND,
}

impl Sidebar {
    pub fn create(parent: HWND) -> Result<Sidebar> {
        register_class()?;
        // 안전성: 자식 창 생성 — WM_CREATE에서 상태를 부착한다
        let hwnd = unsafe {
            let instance = GetModuleHandleW(None)?;
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                SIDEBAR_CLASS,
                None,
                WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_CLIPCHILDREN | WS_TABSTOP,
                0,
                0,
                0,
                0,
                Some(parent),
                None,
                Some(instance.into()),
                None,
            )?
        };
        Ok(Sidebar { hwnd })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// 표시 데이터 갱신 — 목록·활성 항목을 통째로 교체하고 다시 그린다.
    /// 진실은 메인 창의 `WorkspaceList`이고 여기 있는 것은 그리기용 스냅숏이다.
    /// 목록이 바뀌면 편집 중이던 EDIT은 유령이 되므로 먼저 정리한다
    pub fn set_items(&self, items: &[Workspace], active: usize) {
        end_rename(self.hwnd, false);
        let Some(mut state) = state_of(self.hwnd) else {
            return;
        };
        state.items = items.to_vec();
        state.active = active;
        state.hover = None;
        let height = client_height(self.hwnd);
        state.scroll = clamp_scroll(state.scroll, state.items.len(), height);
        drop(state);
        invalidate(self.hwnd);
    }
}

/// 드래그 정렬 진행 상태 — 임계(8px)를 넘기 전에는 `started=false`로 단순 클릭과 구분한다
struct DragReorder {
    from: usize,
    origin_y: i32,
    started: bool,
    /// 현재 커서가 가리키는 삽입 위치 (삽입선 표시용)
    insert_at: usize,
}

/// 사이드바 내부 상태 — 표시 스냅숏 + 그리기 자원
struct SidebarState {
    items: Vec<Workspace>,
    active: usize,
    /// 마우스가 올라간 항목 (없으면 None)
    hover: Option<usize>,
    /// `+` 버튼 위에 마우스가 있는가
    hover_plus: bool,
    /// 세로 스크롤 오프셋(px) — 스크롤바 없이 휠로만 움직인다 (D5)
    scroll: i32,
    /// WM_MOUSELEAVE 추적 등록 여부 — 중복 등록 방지
    tracking: bool,
    /// 인라인 이름 편집 중인 (EDIT 창, 대상 인덱스) — 없으면 편집 중이 아니다
    edit: Option<(HWND, usize)>,
    /// 드래그 정렬 상태 (D12)
    drag: Option<DragReorder>,
    name_font: HFONT,
    subtitle_font: HFONT,
    header_font: HFONT,
    bg_brush: HBRUSH,
    item_brush: HBRUSH,
    item_selected_brush: HBRUSH,
    item_hover_brush: HBRUSH,
    border_brush: HBRUSH,
    accent_brush: HBRUSH,
    /// 항목 아이콘용 시스템 이미지 리스트 (폴더 아이콘 재사용 — D14)
    icons: IconCache,
}

impl SidebarState {
    fn new() -> SidebarState {
        SidebarState {
            items: Vec::new(),
            active: 0,
            hover: None,
            hover_plus: false,
            scroll: 0,
            tracking: false,
            edit: None,
            drag: None,
            name_font: ui_font(NAME_FONT_PX, true),
            subtitle_font: ui_font(SUBTITLE_FONT_PX, false),
            header_font: ui_font(HEADER_FONT_PX, false),
            bg_brush: solid(COLOR_BG),
            item_brush: solid(COLOR_ITEM),
            item_selected_brush: solid(COLOR_ITEM_SELECTED),
            item_hover_brush: solid(COLOR_ITEM_HOVER),
            border_brush: solid(COLOR_ITEM_BORDER),
            accent_brush: solid(COLOR_ACCENT),
            icons: IconCache::new(),
        }
    }

    /// 창 파괴 시 GDI 객체 일괄 해제 (누수 방지)
    fn release(&self) {
        // 안전성: 우리가 만든 GDI 핸들만 해제하며 이후 참조하지 않는다.
        // 생성 실패로 널 핸들이어도 DeleteObject는 실패만 반환한다
        unsafe {
            for font in [self.name_font, self.subtitle_font, self.header_font] {
                let _ = DeleteObject(font.into());
            }
            for brush in [
                self.bg_brush,
                self.item_brush,
                self.item_selected_brush,
                self.item_hover_brush,
                self.border_brush,
                self.accent_brush,
            ] {
                let _ = DeleteObject(brush.into());
            }
        }
    }
}

/// 시스템 UI 폰트(한국어 환경이면 맑은 고딕)를 복사해 크기·굵기만 바꾼다 (D4).
/// 조회 실패 시 기본값 LOGFONTW로 만들어 시스템이 대체 글꼴을 고르게 둔다
fn ui_font(px: i32, semibold: bool) -> HFONT {
    let mut metrics = NONCLIENTMETRICSW {
        cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
        ..Default::default()
    };
    // 안전성: metrics는 스택 소유이며 cbSize를 채워 전달한다 (SPI 규약)
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETNONCLIENTMETRICS,
            size_of::<NONCLIENTMETRICSW>() as u32,
            Some(&mut metrics as *mut _ as *mut core::ffi::c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    let mut lf = metrics.lfMessageFont;
    lf.lfHeight = -px; // 음수 = 문자 높이(px) 지정
    lf.lfWeight = if semibold {
        FW_SEMIBOLD.0 as i32
    } else {
        lf.lfWeight
    };
    // 안전성: lf는 스택 소유 — 반환 핸들은 release()에서 해제한다
    unsafe { CreateFontIndirectW(&lf) }
}

fn solid(color: COLORREF) -> HBRUSH {
    // 안전성: 반환 핸들은 release()에서 해제한다
    unsafe { CreateSolidBrush(color) }
}

fn register_class() -> Result<()> {
    // 안전성: 클래스 중복 등록은 무해 (첫 등록만 유효)
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(sidebar_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            // 배경은 WM_PAINT가 직접 칠한다 (시스템 브러시로 칠하면 다크 배경이 깜빡인다)
            lpszClassName: SIDEBAR_CLASS,
            ..Default::default()
        };
        let _ = RegisterClassExW(&wc);
    }
    Ok(())
}

/// GWLP_USERDATA에서 상태를 빌린다. 부착 전(null)이거나 이미 빌린 중이면 None
fn state_of<'a>(hwnd: HWND) -> Option<RefMut<'a, SidebarState>> {
    // 안전성: 포인터는 WM_CREATE에서 넣은 Box::into_raw 산출물, WM_NCDESTROY에서 회수
    let cell = unsafe {
        (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<SidebarState>).as_ref()
    }?;
    cell.try_borrow_mut().ok()
}

fn client_height(hwnd: HWND) -> i32 {
    let mut rc = RECT::default();
    // 안전성: 유효한 창 핸들의 클라이언트 영역 조회
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    rc.bottom - rc.top
}

fn client_size(hwnd: HWND) -> (i32, i32) {
    let mut rc = RECT::default();
    // 안전성: 유효한 창 핸들의 클라이언트 영역 조회
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    (rc.right - rc.left, rc.bottom - rc.top)
}

fn invalidate(hwnd: HWND) {
    // 안전성: 전체 무효화 — 배경 지우기는 WM_PAINT가 직접 하므로 false
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

fn post_to_parent(hwnd: HWND, msg: u32, wparam: WPARAM) {
    post_to_parent_with(hwnd, msg, wparam, LPARAM(0));
}

fn post_to_parent_with(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) {
    // 안전성: 부모 창으로 비동기 게시 — 부모가 없으면 실패만 반환
    unsafe {
        if let Ok(parent) = GetParent(hwnd) {
            let _ = PostMessageW(Some(parent), msg, wparam, lparam);
        }
    }
}

/// 화면 좌표를 이 창의 클라이언트 좌표로 (우클릭 위치 판정용)
fn client_point(hwnd: HWND, sx: i32, sy: i32) -> Option<(i32, i32)> {
    let mut pt = POINT { x: sx, y: sy };
    // 안전성: pt는 스택 소유, 유효한 창 핸들
    unsafe {
        if ScreenToClient(hwnd, &mut pt).as_bool() {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

/// 기본 프로시저 위임 래퍼
fn def_sidebar(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 안전성: 받은 인자를 그대로 OS 기본 처리에 전달
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

/// `+` 버튼 사각형 (헤더 우측)
fn plus_rect(width: i32) -> RECT {
    let top = TOGGLE_STRIP_HEIGHT + (HEADER_HEIGHT - PLUS_SIZE) / 2;
    RECT {
        left: width - PLUS_MARGIN - PLUS_SIZE,
        top,
        right: width - PLUS_MARGIN,
        bottom: top + PLUS_SIZE,
    }
}

fn in_rect(rc: &RECT, x: i32, y: i32) -> bool {
    x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom
}

fn fill(hdc: HDC, rc: &RECT, brush: HBRUSH) {
    // 안전성: 유효한 DC·스택 사각형·소유 브러시
    unsafe {
        FillRect(hdc, rc, brush);
    }
}

/// 사각형 테두리를 1px 두께로 그린다 (FillRect 4번 — Pen 관리 없이 단순하게)
fn frame(hdc: HDC, rc: &RECT, brush: HBRUSH) {
    let edges = [
        RECT {
            top: rc.top,
            bottom: rc.top + 1,
            ..*rc
        },
        RECT {
            top: rc.bottom - 1,
            bottom: rc.bottom,
            ..*rc
        },
        RECT {
            right: rc.left + 1,
            ..*rc
        },
        RECT {
            left: rc.right - 1,
            ..*rc
        },
    ];
    for edge in &edges {
        fill(hdc, edge, brush);
    }
}

/// 한 줄 텍스트를 말줄임(DT_END_ELLIPSIS)으로 그린다
fn draw_line(hdc: HDC, rc: &mut RECT, text: &str, font: HFONT, color: COLORREF) {
    let mut buf: Vec<u16> = text.encode_utf16().collect();
    if buf.is_empty() {
        return;
    }
    // 안전성: DC 상태 변경 후 그리기 — buf·rc는 호출 동안 유효한 지역 소유
    unsafe {
        let old = SelectObject(hdc, font.into());
        SetTextColor(hdc, color);
        SetBkMode(hdc, TRANSPARENT);
        DrawTextW(
            hdc,
            &mut buf,
            rc,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        SelectObject(hdc, old);
    }
}

/// 창 전체를 다시 그린다 — 배경 → 헤더 → 항목 순
fn paint(hwnd: HWND, state: &mut SidebarState) {
    let (width, height) = client_size(hwnd);
    let mut ps = PAINTSTRUCT::default();
    // 안전성: BeginPaint/EndPaint 짝 — 사이의 DC만 사용한다
    let hdc = unsafe { BeginPaint(hwnd, &mut ps) };

    let full = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    fill(hdc, &full, state.bg_brush);

    // 헤더: 제목 + 새 워크스페이스 버튼
    let mut title = RECT {
        left: ITEM_MARGIN_X + 4,
        top: TOGGLE_STRIP_HEIGHT,
        right: width - PLUS_MARGIN - PLUS_SIZE,
        bottom: TOGGLE_STRIP_HEIGHT + HEADER_HEIGHT,
    };
    draw_header_text(hdc, &mut title, state.header_font, COLOR_HEADER);
    let mut plus = plus_rect(width);
    let plus_color = if state.hover_plus {
        COLOR_HEADER_HOT
    } else {
        COLOR_HEADER
    };
    draw_line(hdc, &mut plus, "+", state.header_font, plus_color);

    // 항목 카드
    for (index, item) in state.items.iter().enumerate() {
        let top = LIST_TOP + index as i32 * ITEM_PITCH - state.scroll;
        if top + ITEM_HEIGHT <= LIST_TOP || top >= height {
            continue; // 화면 밖 — 그리지 않는다
        }
        let card = RECT {
            left: ITEM_MARGIN_X,
            top,
            right: (width - ITEM_MARGIN_X).max(ITEM_MARGIN_X),
            bottom: top + ITEM_HEIGHT,
        };
        let brush = if index == state.active {
            state.item_selected_brush
        } else if state.hover == Some(index) {
            state.item_hover_brush
        } else {
            state.item_brush
        };
        fill(hdc, &card, brush);
        frame(hdc, &card, state.border_brush);
        if index == state.active {
            let accent = RECT {
                right: card.left + ACCENT_BAR_WIDTH,
                ..card
            };
            fill(hdc, &accent, state.accent_brush);
        }

        // 폴더 아이콘 (시스템 이미지 리스트 공유 — 복사본을 만들지 않는다)
        // 안전성: 시스템 이미지 리스트 핸들과 유효한 DC로 그리기만 수행
        unsafe {
            let _ = ImageList_Draw(
                state.icons.himl(),
                state.icons.dir_icon(),
                hdc,
                card.left + ICON_X,
                top + (ITEM_HEIGHT - ICON_SIZE) / 2,
                ILD_TRANSPARENT,
            );
        }

        let text_right = (card.right - 8).max(card.left + TEXT_X);
        let mut name_rc = RECT {
            left: card.left + TEXT_X,
            top: top + NAME_TOP,
            right: text_right,
            bottom: top + NAME_TOP + NAME_FONT_PX + 4,
        };
        draw_line(hdc, &mut name_rc, &item.name, state.name_font, COLOR_NAME);

        let mut sub_rc = RECT {
            left: card.left + TEXT_X,
            top: name_rc.bottom + SUBTITLE_GAP,
            right: text_right,
            bottom: top + ITEM_HEIGHT - 4,
        };
        draw_line(
            hdc,
            &mut sub_rc,
            &item.subtitle,
            state.subtitle_font,
            COLOR_SUBTITLE,
        );
    }

    // 드래그 정렬 중이면 놓일 자리에 삽입선을 그린다 (D12)
    if let Some(drag) = &state.drag
        && drag.started
    {
        let y = LIST_TOP + drag.insert_at as i32 * ITEM_PITCH - state.scroll - ITEM_GAP / 2;
        let line = RECT {
            left: ITEM_MARGIN_X,
            top: y,
            right: (width - ITEM_MARGIN_X).max(ITEM_MARGIN_X),
            bottom: y + INSERT_LINE_HEIGHT,
        };
        fill(hdc, &line, state.accent_brush);
    }

    // 안전성: BeginPaint와 짝을 이루는 해제
    unsafe {
        let _ = EndPaint(hwnd, &ps);
    }
}

/// 헤더 제목은 고정 문구(PCWSTR)라 별도 경로로 그린다
fn draw_header_text(hdc: HDC, rc: &mut RECT, font: HFONT, color: COLORREF) {
    let mut buf: Vec<u16> = {
        // 안전성: 정적 널종단 와이드 문자열을 길이만큼 복사
        let text = unsafe { HEADER_TEXT.as_wide() };
        text.to_vec()
    };
    // 안전성: DC 상태 변경 후 그리기 — buf·rc는 지역 소유
    unsafe {
        let old = SelectObject(hdc, font.into());
        SetTextColor(hdc, color);
        SetBkMode(hdc, TRANSPARENT);
        DrawTextW(
            hdc,
            &mut buf,
            rc,
            DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
        );
        SelectObject(hdc, old);
    }
}

/// 인라인 편집 EDIT을 항목 이름 줄 위에 띄운다 (FR-16 생성 직후·FR-18 이름 변경).
/// 이미 편집 중이면 그 편집을 커밋하고 새로 연다.
/// 메인 창은 자기 상태 차용을 놓은 뒤 이 함수를 호출한다(편집 커밋이 부모로 동기 전달되므로)
pub fn begin_rename(hwnd: HWND, index: usize) {
    end_rename(hwnd, true);
    let (width, _) = client_size(hwnd);
    let (name, rect) = {
        let Some(state) = state_of(hwnd) else {
            return;
        };
        let Some(item) = state.items.get(index) else {
            return;
        };
        (item.name.clone(), item_rect(index, state.scroll, width))
    };
    // 안전성: 표준 EDIT 자식 생성 후 서브클래스 부착 — 핸들은 end_rename에서 파괴한다
    let edit = unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let Ok(edit) = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("EDIT"),
            &HSTRING::from(name.as_str()),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            rect.left + TEXT_X,
            rect.top + NAME_TOP - 2,
            (rect.right - rect.left - TEXT_X - 8).max(40),
            NAME_FONT_PX + 10,
            Some(hwnd),
            None,
            Some(instance.into()),
            None,
        ) else {
            return;
        };
        let _ = SetWindowSubclass(edit, Some(edit_subclass_proc), 1, 0);
        // 전체 선택 후 포커스 — 새 워크스페이스의 자동 이름을 바로 덮어쓸 수 있게 한다
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
        let _ = SetFocus(Some(edit));
        edit
    };
    if let Some(mut state) = state_of(hwnd) {
        state.edit = Some((edit, index));
    }
}

/// 인라인 편집 종료 — `commit`이면 입력값을 부모에 전달한다.
/// 편집 중이 아니면 아무 일도 하지 않는다(모든 진입점에서 선호출해도 안전)
fn end_rename(hwnd: HWND, commit: bool) {
    let Some((edit, index)) = state_of(hwnd).and_then(|mut s| s.edit.take()) else {
        return;
    };
    let text = window_text(edit);
    // 안전성: 우리가 만든 EDIT 파괴 — 서브클래스는 창과 함께 사라진다
    unsafe {
        let _ = DestroyWindow(edit);
    }
    if commit {
        let request = RenameRequest { index, name: text };
        // 포인터는 이 스택이 소유 — 반드시 동기 SendMessage (Post 금지)
        send_to_parent(
            hwnd,
            WM_APP_WS_RENAME,
            WPARAM(0),
            LPARAM(&request as *const RenameRequest as isize),
        );
    }
    // 안전성: 편집이 끝나면 목록이 다시 키 입력을 받아야 한다
    unsafe {
        let _ = SetFocus(Some(hwnd));
    }
    invalidate(hwnd);
}

/// 창 텍스트 읽기 (인라인 편집 EDIT 전용 — address_bar와 같은 표준 패턴)
fn window_text(hwnd: HWND) -> String {
    // 안전성: 길이 조회 후 그 크기 버퍼로 읽기
    unsafe {
        let len = GetWindowTextLengthW(hwnd);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let read = GetWindowTextW(hwnd, &mut buf);
        String::from_utf16_lossy(&buf[..read.max(0) as usize])
    }
}

/// 인라인 편집 EDIT 서브클래스 — Enter는 커밋, Esc는 취소, 포커스 상실은 커밋으로 처리한다
unsafe extern "system" fn edit_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    let notify = match (msg, wparam.0 as u16) {
        (WM_KEYDOWN, key) if key == VK_RETURN.0 => Some(WM_APP_RENAME_COMMIT),
        (WM_KEYDOWN, key) if key == VK_ESCAPE.0 => Some(WM_APP_RENAME_CANCEL),
        (WM_KILLFOCUS, _) => Some(WM_APP_RENAME_COMMIT),
        _ => None,
    };
    if let Some(msg) = notify {
        // 안전성: 부모(사이드바)에 게시 — 편집 창 파괴는 부모가 수행하므로 여기서는 알리기만 한다
        unsafe {
            if let Ok(parent) = GetParent(hwnd) {
                let _ = PostMessageW(Some(parent), msg, WPARAM(0), LPARAM(0));
            }
        }
        if msg == WM_APP_RENAME_COMMIT && lparam.0 == 0 {
            return LRESULT(0); // Enter 기본 처리(경고음) 억제
        }
    }
    // 안전성: 나머지는 원래 프로시저로 위임
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

/// 부모(메인 창)로 동기 질의 — 포인터 페이로드 계약 전용
fn send_to_parent(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) {
    // 안전성: 부모 조회 후 동기 전달 — lparam 포인터는 호출 동안 유효하다
    unsafe {
        if let Ok(parent) = GetParent(hwnd) {
            SendMessageW(parent, msg, Some(wparam), Some(lparam));
        }
    }
}

/// 마우스가 창을 벗어날 때 WM_MOUSELEAVE를 받도록 등록 (hover 표시 해제용)
fn track_leave(hwnd: HWND) {
    let mut track = TRACKMOUSEEVENT {
        cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    // 안전성: track은 스택 소유이며 cbSize를 채워 전달한다
    unsafe {
        let _ = TrackMouseEvent(&mut track);
    }
}

fn coords(lparam: LPARAM) -> (i32, i32) {
    let x = (lparam.0 & 0xffff) as u16 as i16 as i32;
    let y = ((lparam.0 >> 16) & 0xffff) as u16 as i16 as i32;
    (x, y)
}

/// WM_MOUSEWHEEL의 wparam 상위 워드 = 휠 회전량(부호 있음)
fn wheel_delta(wparam: WPARAM) -> i32 {
    ((wparam.0 >> 16) & 0xffff) as u16 as i16 as i32
}

unsafe extern "system" fn sidebar_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let boxed = Box::new(RefCell::new(SidebarState::new()));
            // 안전성: 소유권을 창에 이전 — WM_NCDESTROY에서 회수
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(boxed) as isize);
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1), // 배경은 WM_PAINT가 칠한다 (깜빡임 방지)
        WM_PAINT => {
            if let Some(mut state) = state_of(hwnd) {
                paint(hwnd, &mut state);
            }
            LRESULT(0)
        }
        WM_SIZE => {
            if let Some(mut state) = state_of(hwnd) {
                let count = state.items.len();
                let height = client_height(hwnd);
                state.scroll = clamp_scroll(state.scroll, count, height);
            }
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            // 클릭하면 포커스를 가져간다 — F2·Delete·방향키와 휠 스크롤이 이 창으로 온다 (D17)
            end_rename(hwnd, true); // 편집 중이었으면 커밋하고 목록 조작을 이어간다
            // 안전성: 유효한 자식 창에 포커스 이동
            unsafe {
                let _ = SetFocus(Some(hwnd));
            }
            let (x, y) = coords(lparam);
            let (width, _) = client_size(hwnd);
            if in_rect(&plus_rect(width), x, y) {
                post_to_parent(hwnd, WM_APP_WS_NEW, WPARAM(0));
                return LRESULT(0);
            }
            let hit = state_of(hwnd).and_then(|s| item_at(y, s.scroll, s.items.len()));
            if let Some(index) = hit {
                // 드래그 후보로 기록만 한다 — 임계(8px)를 넘겨야 재정렬이 시작된다 (D12)
                if let Some(mut state) = state_of(hwnd) {
                    state.drag = Some(DragReorder {
                        from: index,
                        origin_y: y,
                        started: false,
                        insert_at: index,
                    });
                }
                // 안전성: 드래그 추적을 위해 캡처 — WM_LBUTTONUP·WM_CAPTURECHANGED에서 해제
                unsafe {
                    SetCapture(hwnd);
                }
                post_to_parent(hwnd, WM_APP_WS_SELECT, WPARAM(index));
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let (x, y) = coords(lparam);
            let (width, _) = client_size(hwnd);
            let mut need_track = false;
            let mut changed = false;
            if let Some(mut state) = state_of(hwnd) {
                let hover = item_at(y, state.scroll, state.items.len());
                let hover_plus = in_rect(&plus_rect(width), x, y);
                changed = hover != state.hover || hover_plus != state.hover_plus;
                state.hover = hover;
                state.hover_plus = hover_plus;
                if !state.tracking {
                    state.tracking = true;
                    need_track = true;
                }
                // 드래그 정렬 진행 — 임계를 넘으면 시작하고 삽입 위치를 갱신한다
                let count = state.items.len();
                let scroll = state.scroll;
                if let Some(drag) = &mut state.drag {
                    if !drag.started && (y - drag.origin_y).abs() >= DRAG_THRESHOLD {
                        drag.started = true;
                        changed = true;
                    }
                    if drag.started {
                        let insert_at = drop_index(y, scroll, count);
                        if insert_at != drag.insert_at {
                            drag.insert_at = insert_at;
                            changed = true;
                        }
                    }
                }
            }
            if need_track {
                track_leave(hwnd);
            }
            if changed {
                invalidate(hwnd);
            }
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let drop = state_of(hwnd).and_then(|mut s| s.drag.take());
            // 안전성: 자기 스레드가 잡은 캡처 해제
            unsafe {
                let _ = ReleaseCapture();
            }
            if let Some(drag) = drop
                && drag.started
            {
                // 자기 자신 앞뒤로 놓으면 순서가 그대로다 — 그 경우는 알리지 않는다
                let to = if drag.insert_at > drag.from {
                    drag.insert_at - 1
                } else {
                    drag.insert_at
                };
                if to != drag.from {
                    post_to_parent_with(
                        hwnd,
                        WM_APP_WS_REORDER,
                        WPARAM(drag.from),
                        LPARAM(to as isize),
                    );
                }
            }
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_CAPTURECHANGED => {
            // 캡처 상실(Alt+Tab 등) → 드래그 취소 (순서 불변)
            if let Some(mut state) = state_of(hwnd) {
                state.drag = None;
            }
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_CONTEXTMENU => {
            // 우클릭 위치의 항목을 선택 상태로 만든 뒤 부모에 메뉴 표시를 위임한다.
            // lparam은 화면 좌표이며 키보드 메뉴 키(-1,-1)는 부모가 커서 위치로 대체한다
            let (sx, sy) = coords(lparam);
            let index = client_point(hwnd, sx, sy).and_then(|(_, y)| {
                state_of(hwnd).and_then(|s| item_at(y, s.scroll, s.items.len()))
            });
            if let Some(index) = index {
                end_rename(hwnd, true);
                post_to_parent(hwnd, WM_APP_WS_SELECT, WPARAM(index));
                post_to_parent_with(hwnd, WM_APP_WS_CONTEXT, WPARAM(index), lparam);
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            // 키 조작은 사이드바가 포커스를 가진 동안에만 동작한다 (D16 — 전역 액셀러레이터 금지)
            let key = wparam.0 as u16;
            let (active, count) = match state_of(hwnd) {
                Some(state) => (state.active, state.items.len()),
                None => return LRESULT(0),
            };
            match key {
                k if k == VK_F2.0 => begin_rename(hwnd, active),
                k if k == VK_DELETE.0 => post_to_parent(hwnd, WM_APP_WS_DELETE, WPARAM(active)),
                k if k == VK_UP.0 && active > 0 => {
                    post_to_parent(hwnd, WM_APP_WS_SELECT, WPARAM(active - 1));
                }
                k if k == VK_DOWN.0 && active + 1 < count => {
                    post_to_parent(hwnd, WM_APP_WS_SELECT, WPARAM(active + 1));
                }
                _ => return def_sidebar(hwnd, msg, wparam, lparam),
            }
            LRESULT(0)
        }
        WM_APP_RENAME_COMMIT => {
            end_rename(hwnd, true);
            LRESULT(0)
        }
        WM_APP_RENAME_CANCEL => {
            end_rename(hwnd, false);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            if let Some(mut state) = state_of(hwnd) {
                state.tracking = false;
                state.hover = None;
                state.hover_plus = false;
            }
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = wheel_delta(wparam);
            if let Some(mut state) = state_of(hwnd) {
                let count = state.items.len();
                let height = client_height(hwnd);
                // 위로 굴리면(+) 목록이 위로 — 스크롤 오프셋은 감소
                let next = state.scroll - delta / 120 * WHEEL_STEP;
                state.scroll = clamp_scroll(next, count, height);
            }
            invalidate(hwnd);
            LRESULT(0)
        }
        WM_NCDESTROY => {
            // 안전성: WM_CREATE에서 넣은 포인터를 정확히 한 번 회수하고 GDI 자원을 해제
            unsafe {
                let ptr = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut RefCell<SidebarState>;
                if !ptr.is_null() {
                    let cell = Box::from_raw(ptr);
                    cell.borrow().release();
                }
            }
            // 안전성: 기본 처리 위임
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        // 안전성: 나머지는 OS 기본 처리
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 목록_영역_위쪽은_항목이_아니다() {
        assert_eq!(item_at(0, 0, 3), None);
        assert_eq!(item_at(LIST_TOP - 1, 0, 3), None);
    }

    #[test]
    fn 항목_히트테스트는_여백을_제외한다() {
        assert_eq!(item_at(LIST_TOP, 0, 3), Some(0));
        assert_eq!(item_at(LIST_TOP + ITEM_HEIGHT - 1, 0, 3), Some(0));
        // 항목 사이 여백
        assert_eq!(item_at(LIST_TOP + ITEM_HEIGHT, 0, 3), None);
        assert_eq!(item_at(LIST_TOP + ITEM_PITCH, 0, 3), Some(1));
        // 목록 밖
        assert_eq!(item_at(LIST_TOP + ITEM_PITCH * 3, 0, 3), None);
    }

    #[test]
    fn 스크롤된_상태의_히트테스트() {
        assert_eq!(item_at(LIST_TOP, ITEM_PITCH, 3), Some(1));
        assert_eq!(item_at(LIST_TOP, ITEM_PITCH * 2, 3), Some(2));
    }

    #[test]
    fn 스크롤은_0과_최대치_사이로_클램프된다() {
        // 항목 3개(192px)에 목록 영역 100px → 최대 92px
        let view_h = LIST_TOP + 100;
        assert_eq!(clamp_scroll(-50, 3, view_h), 0);
        assert_eq!(clamp_scroll(50, 3, view_h), 50);
        assert_eq!(clamp_scroll(9999, 3, view_h), 3 * ITEM_PITCH - 100);
    }

    #[test]
    fn 항목이_영역보다_적으면_스크롤이_없다() {
        let view_h = LIST_TOP + 500;
        assert_eq!(clamp_scroll(0, 2, view_h), 0);
        assert_eq!(clamp_scroll(300, 2, view_h), 0);
    }

    #[test]
    fn 드롭_위치는_항목_중앙을_기준으로_갈린다() {
        // 첫 항목 위쪽 절반 → 그 앞(0), 아래쪽 절반 → 그 뒤(1)
        assert_eq!(drop_index(LIST_TOP + 1, 0, 3), 0);
        assert_eq!(drop_index(LIST_TOP + ITEM_HEIGHT / 2, 0, 3), 1);
        // 두 번째 항목 아래쪽 절반 → 2
        assert_eq!(drop_index(LIST_TOP + ITEM_PITCH + ITEM_HEIGHT - 1, 0, 3), 2);
    }

    #[test]
    fn 목록_아래_빈_공간에_놓으면_맨_끝이다() {
        assert_eq!(drop_index(LIST_TOP + ITEM_PITCH * 5, 0, 3), 3);
        assert_eq!(drop_index(LIST_TOP - 100, 0, 3), 0); // 위쪽 밖은 맨 앞
        assert_eq!(drop_index(LIST_TOP, 0, 0), 0); // 빈 목록
    }

    #[test]
    fn 스크롤된_상태의_드롭_위치() {
        assert_eq!(drop_index(LIST_TOP + 1, ITEM_PITCH, 3), 1);
        assert_eq!(drop_index(LIST_TOP + ITEM_HEIGHT / 2, ITEM_PITCH, 3), 2);
    }

    #[test]
    fn 항목_사각형은_스크롤과_여백을_반영한다() {
        let rc = item_rect(1, 0, 232);
        assert_eq!(rc.top, LIST_TOP + ITEM_PITCH);
        assert_eq!(rc.bottom - rc.top, ITEM_HEIGHT);
        assert_eq!(rc.left, ITEM_MARGIN_X);
        assert_eq!(rc.right, 232 - ITEM_MARGIN_X);

        let scrolled = item_rect(1, ITEM_PITCH, 232);
        assert_eq!(scrolled.top, LIST_TOP);
    }

    #[test]
    fn 새_워크스페이스_버튼은_헤더_우측이다() {
        let rc = plus_rect(232);
        assert_eq!(rc.right, 232 - PLUS_MARGIN);
        assert_eq!(rc.right - rc.left, PLUS_SIZE);
        assert!(rc.top >= TOGGLE_STRIP_HEIGHT);
        assert!(rc.bottom <= TOGGLE_STRIP_HEIGHT + HEADER_HEIGHT);
        assert!(in_rect(&rc, rc.left + 1, rc.top + 1));
        assert!(!in_rect(&rc, rc.left - 1, rc.top + 1));
    }
}
