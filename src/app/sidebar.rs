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
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, CreateSolidBrush, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX,
    DT_SINGLELINE, DeleteObject, DrawTextW, EndPaint, FW_SEMIBOLD, FillRect, HBRUSH, HDC, HFONT,
    InvalidateRect, PAINTSTRUCT, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{ILD_TRANSPARENT, ImageList_Draw, WM_MOUSELEAVE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, IDC_ARROW,
    LoadCursorW, NONCLIENTMETRICSW, PostMessageW, RegisterClassExW, SPI_GETNONCLIENTMETRICS,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetWindowLongPtrW, SystemParametersInfoW, WINDOW_EX_STYLE,
    WM_APP, WM_CREATE, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCDESTROY,
    WM_PAINT, WM_SIZE, WNDCLASSEXW, WS_CHILD, WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_TABSTOP,
    WS_VISIBLE,
};
use windows::core::{PCWSTR, Result, w};

const SIDEBAR_CLASS: PCWSTR = w!("FileExplorerSidebar");

/// 사이드바 → 부모(메인 창) 알림.
/// SELECT는 wparam=선택 인덱스, NEW는 인자 없음. WM_APP+13은 패널의 경로 변경 알림이 쓴다
pub const WM_APP_WS_SELECT: u32 = WM_APP + 14;
pub const WM_APP_WS_NEW: u32 = WM_APP + 15;

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
    /// 진실은 메인 창의 `WorkspaceList`이고 여기 있는 것은 그리기용 스냅숏이다
    pub fn set_items(&self, items: &[Workspace], active: usize) {
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
    // 안전성: 부모 창으로 비동기 게시 — 부모가 없으면 실패만 반환
    unsafe {
        let parent = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd);
        if let Ok(parent) = parent {
            let _ = PostMessageW(Some(parent), msg, wparam, LPARAM(0));
        }
    }
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
            // 클릭하면 포커스를 가져간다 — F2·Delete·방향키(T6)와 휠 스크롤이 이 창으로 온다 (D17)
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
            }
            if need_track {
                track_leave(hwnd);
            }
            if changed {
                invalidate(hwnd);
            }
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
