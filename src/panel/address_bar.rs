//! 주소창 스트립 — [←][→][↑] 버튼 + 경로 입력 Edit (FR-6)
use crate::app::theme;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DT_CENTER, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextW, FillRect,
    SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_DISABLED, ODS_HOTLIGHT, ODS_SELECTED};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, VK_RETURN};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    BS_OWNERDRAW, CreateWindowExW, ES_AUTOHSCROLL, GetWindowTextLengthW, GetWindowTextW, MoveWindow,
    PostMessageW, SetWindowTextW, WINDOW_STYLE, WM_KEYDOWN, WS_CHILD, WS_CLIPSIBLINGS,
    WS_EX_CLIENTEDGE, WS_TABSTOP, WS_VISIBLE,
};
use windows::core::{HSTRING, Result, w};

/// 패널로 보내는 자식 컨트롤 명령 id (WM_COMMAND loword)
pub const ID_NAV_BACK: u32 = 201;
pub const ID_NAV_FORWARD: u32 = 202;
pub const ID_NAV_UP: u32 = 203;

/// Enter 입력 통지 — Edit 서브클래스가 패널 창으로 게시
pub const WM_APP_ADDRESS_ENTER: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 2;

/// 주소창 스트립 높이
pub const STRIP_HEIGHT: i32 = 30;
const BTN_W: i32 = 28;
const GAP: i32 = 2;

pub struct AddressBar {
    back_btn: HWND,
    forward_btn: HWND,
    up_btn: HWND,
    edit: HWND,
}

impl AddressBar {
    /// 패널 자식으로 버튼 3개 + Edit 생성. Enter는 WM_APP_ADDRESS_ENTER로 패널에 게시된다
    pub fn create(parent: HWND) -> Result<AddressBar> {
        let back_btn = create_button(parent, w!("←"), ID_NAV_BACK)?;
        let forward_btn = create_button(parent, w!("→"), ID_NAV_FORWARD)?;
        let up_btn = create_button(parent, w!("↑"), ID_NAV_UP)?;
        // 안전성: 표준 EDIT 자식 생성
        let edit = unsafe {
            let instance = GetModuleHandleW(None)?;
            CreateWindowExW(
                WS_EX_CLIENTEDGE,
                w!("EDIT"),
                None,
                WS_CHILD
                    | WS_VISIBLE
                    | WS_CLIPSIBLINGS
                    | WS_TABSTOP
                    | WINDOW_STYLE(ES_AUTOHSCROLL as u32),
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
        // 안전성: 서브클래스 콜백은 창 수명 동안 유효한 함수 포인터, 참조 데이터 없음(0)
        unsafe {
            let _ = SetWindowSubclass(edit, Some(edit_subclass_proc), 1, 0);
        }
        Ok(AddressBar {
            back_btn,
            forward_btn,
            up_btn,
            edit,
        })
    }

    /// 스트립 재배치 — 세로 오프셋 y부터 버튼 3개 + 나머지 폭 Edit
    pub fn layout_at(&self, y: i32, w: i32) {
        let mut x = GAP;
        for btn in [self.back_btn, self.forward_btn, self.up_btn] {
            move_child(btn, x, y + GAP, BTN_W, STRIP_HEIGHT - GAP * 2);
            x += BTN_W + GAP;
        }
        move_child(
            self.edit,
            x,
            y + GAP + 1,
            (w - x - GAP).max(0),
            STRIP_HEIGHT - GAP * 2 - 2,
        );
    }

    /// 주소창 텍스트를 커밋된 경로로 갱신
    pub fn set_path(&self, path: &Path) {
        // 안전성: 유효한 Edit 핸들에 텍스트 설정
        unsafe {
            let _ = SetWindowTextW(self.edit, &HSTRING::from(path.to_string_lossy().as_ref()));
        }
    }

    /// 입력된 텍스트 (따옴표·공백 정리는 호출부의 normalize_input 몫)
    pub fn text(&self) -> String {
        // 안전성: 길이 조회 후 그 크기 버퍼로 읽기 — 표준 패턴
        unsafe {
            let len = GetWindowTextLengthW(self.edit);
            if len <= 0 {
                return String::new();
            }
            let mut buf = vec![0u16; len as usize + 1];
            let read = GetWindowTextW(self.edit, &mut buf);
            String::from_utf16_lossy(&buf[..read.max(0) as usize])
        }
    }

    /// 뒤로/앞으로/상위 버튼 활성 상태 갱신 (드라이브 루트 상위 비활성 — D11)
    pub fn set_nav_state(&self, can_back: bool, can_forward: bool, can_up: bool) {
        // 안전성: 유효한 버튼 핸들 활성/비활성
        unsafe {
            let _ = EnableWindow(self.back_btn, can_back);
            let _ = EnableWindow(self.forward_btn, can_forward);
            let _ = EnableWindow(self.up_btn, can_up);
        }
    }
}

/// 입력 문자열 정규화 — 따옴표·공백 제거 후, 상대 경로면 현재 경로 기준 절대화 (T5 Edge)
pub fn normalize_input(current: &Path, input: &str) -> Option<PathBuf> {
    let trimmed = input.trim().trim_matches('"').trim();
    if trimmed.is_empty() {
        return None;
    }
    let p = Path::new(trimmed);
    if p.is_absolute() || trimmed.starts_with(r"\\") {
        Some(p.to_path_buf())
    } else {
        Some(current.join(p))
    }
}

/// 네비 버튼(←→↑) 오너드로우 — 부모 패널의 WM_DRAWITEM에서 호출한다 (plan T5).
/// 배경을 상태별 다크색으로 채우고 버튼 텍스트(글리프)를 중앙에 그린다.
pub fn draw_nav_button(dis: &DRAWITEMSTRUCT) {
    let disabled = (dis.itemState.0 & ODS_DISABLED.0) != 0;
    let pressed = (dis.itemState.0 & ODS_SELECTED.0) != 0;
    let hot = (dis.itemState.0 & ODS_HOTLIGHT.0) != 0;
    let bg = if pressed {
        theme::CONTROL_ACTIVE
    } else if hot && !disabled {
        theme::CONTROL_HOT
    } else {
        theme::CONTROL_BG
    };
    let mut rc = dis.rcItem;
    // 안전성: 오너드로우가 넘긴 유효 DC에 배경·글리프를 그린다. 브러시는 생성 즉시 해제
    unsafe {
        let brush = CreateSolidBrush(bg);
        FillRect(dis.hDC, &rc, brush);
        let _ = DeleteObject(brush.into());
        // 버튼 텍스트(←→↑)를 읽어 중앙에 그린다
        let mut buf = [0u16; 8];
        let len = GetWindowTextW(dis.hwndItem, &mut buf);
        SetTextColor(
            dis.hDC,
            if disabled { theme::TEXT_DIM } else { theme::TEXT },
        );
        SetBkMode(dis.hDC, TRANSPARENT);
        DrawTextW(
            dis.hDC,
            &mut buf[..len.max(0) as usize],
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
    }
}

fn create_button(parent: HWND, label: windows::core::PCWSTR, id: u32) -> Result<HWND> {
    // 안전성: 표준 BUTTON 자식 생성 — hMenu 자리에 컨트롤 id 전달 (Win32 관례)
    unsafe {
        let instance = GetModuleHandleW(None)?;
        CreateWindowExW(
            Default::default(),
            w!("BUTTON"),
            label,
            // BS_OWNERDRAW: 표준 버튼은 배경 다크가 안 먹으므로 부모 WM_DRAWITEM에서 직접 그린다 (plan T5)
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | WS_TABSTOP | WINDOW_STYLE(BS_OWNERDRAW as u32),
            0,
            0,
            0,
            0,
            Some(parent),
            Some(windows::Win32::UI::WindowsAndMessaging::HMENU(
                id as isize as *mut core::ffi::c_void,
            )),
            Some(instance.into()),
            None,
        )
    }
}

fn move_child(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    // 안전성: 자식 창 이동
    unsafe {
        let _ = MoveWindow(hwnd, x, y, w.max(0), h.max(0), true);
    }
}

/// Edit 서브클래스 — Enter를 패널(부모)로 게시. 나머지는 기본 처리
unsafe extern "system" fn edit_subclass_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == WM_KEYDOWN && wparam.0 as u16 == VK_RETURN.0 {
        // 안전성: 부모(패널) 핸들 조회 후 게시 — 실패해도 무해
        unsafe {
            if let Ok(parent) = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd) {
                let _ = PostMessageW(Some(parent), WM_APP_ADDRESS_ENTER, WPARAM(0), LPARAM(0));
            }
        }
        return LRESULT(0);
    }
    // 안전성: 서브클래스 기본 체인 위임
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 절대_경로는_그대로() {
        let cur = Path::new("C:\\base");
        assert_eq!(
            normalize_input(cur, r"D:\data").unwrap(),
            PathBuf::from(r"D:\data")
        );
    }

    #[test]
    fn 따옴표와_공백을_벗긴다() {
        let cur = Path::new("C:\\base");
        assert_eq!(
            normalize_input(cur, "  \"D:\\my folder\"  ").unwrap(),
            PathBuf::from(r"D:\my folder")
        );
    }

    #[test]
    fn 상대_경로는_현재_기준_절대화() {
        let cur = Path::new(r"C:\base");
        assert_eq!(
            normalize_input(cur, "sub\\dir").unwrap(),
            PathBuf::from(r"C:\base\sub\dir")
        );
    }

    #[test]
    fn 빈_입력은_none() {
        assert!(normalize_input(Path::new("C:\\"), "   ").is_none());
    }

    #[test]
    fn unc_경로_지원() {
        assert!(
            normalize_input(Path::new("C:\\"), r"\\server\share")
                .unwrap()
                .to_string_lossy()
                .starts_with(r"\\server")
        );
    }
}
