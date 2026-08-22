//! 셸 컨텍스트 메뉴 — IContextMenu 표준 절차 래퍼 (FR-8, plan D13)
//!
//! SHParseDisplayName → 부모 IShellFolder → GetUIObjectOf(IContextMenu) →
//! QueryContextMenu 까지가 공통이고, 그 뒤가 **두 갈래**다.
//!
//! - **우리가 그리는 Win11 모양 메뉴**(FR-8 개정): 채워진 HMENU를 `ShellMenu`가 읽어
//!   [`ShellMenuItem`] 목록으로 바꾸고, 고른 것을 `invoke`로 실행한다. 그리기는 `ui`가 한다.
//! - **`추가 옵션 표시`가 여는 표준 메뉴**: 종전대로 `TrackPopupMenuEx`에 그대로 넘긴다
//!   (`show_context_menu`). 글자 없이 스스로 그리는 확장은 이쪽에서만 제대로 보인다.
//!
//! IContextMenu2/3 소유자 메시지 포워딩을 이 모듈에 격리한다 (unsafe 격리).
//! 복사·삭제·이름 바꾸기는 `fs::file_op`이 셸에 걸고, 나머지 항목은 셸이 스스로 수행한다.
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Shell::Common::ITEMIDLIST;
use windows::Win32::UI::Shell::{
    CMF_NORMAL, CMINVOKECOMMANDINFO, GCS_VERBW, IContextMenu, IContextMenu2, IContextMenu3,
    ILFindLastID, ILFree, IShellFolder, SHBindToParent, SHGetDesktopFolder, SHParseDisplayName,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, GetMenuItemCount, GetMenuItemInfoW, MENUITEMINFOW, MFS_CHECKED,
    MFS_DISABLED, MFT_MENUBARBREAK, MFT_MENUBREAK, MFT_SEPARATOR, MIIM_BITMAP, MIIM_FTYPE, MIIM_ID,
    MIIM_STATE, MIIM_STRING, MIIM_SUBMENU, SW_SHOWNORMAL, TPM_LEFTALIGN, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TPM_TOPALIGN, TrackPopupMenuEx, WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM,
    WM_MENUCHAR,
};
use windows::core::{HSTRING, Interface, PCSTR, PSTR, PWSTR, Result};

use crate::fs::bitmap::bgra_from_hbitmap;

/// QueryContextMenu 명령 id 범위 시작 (0은 "선택 없음"과 구분 불가라 1부터)
const ID_FIRST: u32 = 1;
const ID_LAST: u32 = 0x7fff;

thread_local! {
    /// TrackPopupMenuEx 모달 루프 동안 소유자 메시지를 포워딩할 인터페이스.
    /// UI 스레드 전용(STA)이므로 thread_local로 충분하다
    static ACTIVE_MENU: RefCell<Option<(Option<IContextMenu2>, Option<IContextMenu3>)>> =
        const { RefCell::new(None) };
}

/// 셸 컨텍스트 메뉴 표시·실행. items가 비면 folder의 배경 메뉴(새로 만들기 포함).
/// 좌표는 화면 기준. 실패는 조용히 무시한다 — 오류 UI는 셸 위임 (T2 Edge)
pub fn show_context_menu(owner: HWND, folder: &Path, items: &[PathBuf], x: i32, y: i32) {
    // 안전성: COM은 main에서 STA 초기화됨. 내부 포인터 수명은 이 함수 안에서 닫힌다
    unsafe {
        let _ = show_inner(owner, folder, items, x, y);
    }
}

/// 소유자 메시지(WM_INITMENUPOPUP 등)를 활성 IContextMenu2/3에 포워딩.
/// 메뉴 모달 중이 아니면 None → 호출부가 기본 처리로 넘긴다
pub fn forward_menu_msg(msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    if !matches!(
        msg,
        WM_INITMENUPOPUP | WM_DRAWITEM | WM_MEASUREITEM | WM_MENUCHAR
    ) {
        return None;
    }
    ACTIVE_MENU.with_borrow(|active| {
        let (cm2, cm3) = active.as_ref()?;
        // 안전성: 인터페이스는 모달 루프 동안 유효 (show_inner가 수명 관리)
        unsafe {
            if let Some(cm3) = cm3 {
                let mut lres = LRESULT(0);
                let _ = cm3.HandleMenuMsg2(msg, wparam, lparam, Some(&mut lres));
                return Some(lres);
            }
            if let Some(cm2) = cm2 {
                let _ = cm2.HandleMenuMsg(msg, wparam, lparam);
                return Some(LRESULT(0));
            }
        }
        None
    })
}

/// 셸 메뉴 한 줄 — 우리가 그리는 데 필요한 것만 담는다 (FR-8).
///
/// **egui 타입을 쓰지 않는다** — `fs`는 `ui`를 모른다. 아이콘은 픽셀 바이트로 넘기고
/// 텍스처로 만드는 것은 그리는 쪽의 몫이다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellMenuItem {
    /// 실행할 때 되돌려 줄 명령 번호. 구분선·하위 메뉴 머리에는 뜻이 없다
    pub id: u32,
    /// 화면에 보일 이름 — `&` 액셀러레이터를 뗀 것
    pub label: String,
    /// 오른쪽에 흐리게 적을 단축키 — 탭 뒤에 붙어 오던 것을 갈라 둔다. 없으면 빈 문자열
    pub shortcut: String,
    /// 왼쪽 아이콘 `(폭, 높이, BGRA)` — 셸이 주지 않았으면 `None`
    pub icon: Option<(i32, i32, Vec<u8>)>,
    /// 눌릴 수 있는가
    pub enabled: bool,
    /// 체크 표시가 붙어 있는가
    pub checked: bool,
    /// 이 줄이 구분선인가 — 참이면 나머지 필드는 뜻이 없다
    pub separator: bool,
    /// 하위 메뉴가 있으면 그것을 펼칠 때 쓸 손잡이
    pub submenu: Option<SubmenuHandle>,
}

/// 하위 메뉴 하나를 가리키는 손잡이 — 안을 들여다볼 수 없는 값이다.
///
/// `HMENU`를 그대로 내보내지 않는 이유는 `ui`가 Win32 핸들을 들고 다니게 되기 때문이다.
/// **부모 안에서의 자리도 함께 든다** — 셸에게 채워 달라고 물을 때 그 값을 요구한다
/// (`WM_INITMENUPOPUP`의 `lParam`).
///
/// **이 값은 그것을 준 [`ShellMenu`]가 살아 있는 동안만 뜻이 있다.** 타입으로 막지 않은 것은
/// 그리는 쪽이 이 손잡이를 프레임 사이에 들고 있어야 해서다(빌림으로 묶으면 그 보관이 막힌다).
/// 대신 **어겨도 안전하게 실패한다** — 이미 지워진 메뉴를 물으면 `GetMenuItemCount`가 `-1`을
/// 주고 [`ShellMenu::expand`]는 빈 목록을 돌려준다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubmenuHandle {
    menu: isize,
    /// 부모 메뉴에서 이 하위 메뉴가 놓인 0부터의 자리
    index: u32,
}

/// 열려 있는 셸 메뉴 하나 — 인터페이스와 HMENU를 함께 쥔다 (FR-8).
///
/// **둘의 수명이 묶여 있다** — `InvokeCommand`는 그 메뉴를 채운 `IContextMenu`가 살아 있어야
/// 하고, HMENU는 그것과 별개로 우리가 지워야 한다. 한 타입이 둘 다 들고 `Drop`에서 정리하면
/// 그 짝이 어긋날 자리가 없다
pub struct ShellMenu {
    context_menu: IContextMenu,
    /// 동적 하위 메뉴를 채워 달라고 물을 인터페이스 — 없는 셸 확장도 있다
    owner_draw: (Option<IContextMenu2>, Option<IContextMenu3>),
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
}

impl ShellMenu {
    /// 메뉴를 연다 — `items`가 비면 폴더 배경 메뉴다.
    ///
    /// 열지 못하면 `None`이며 부르는 쪽은 아무것도 보이지 않는다(종전과 같이 조용하다)
    pub fn open(owner: HWND, folder: &Path, items: &[PathBuf]) -> Option<ShellMenu> {
        // 안전성: COM이 STA로 초기화된 UI 스레드에서만 부른다. 만든 HMENU는 `Drop`이 지운다
        unsafe {
            let context_menu = if items.is_empty() {
                background_menu(owner, folder).ok()?
            } else {
                items_menu(owner, items).ok()?
            };
            let menu = CreatePopupMenu().ok()?;
            // **`CMF_CANRENAME`을 주지 않는다** — 셸의 `rename` verb는 탐색기 자신의 목록 뷰가
            // 받아 편집을 시작하는 것이라 다른 호스트에서 부르면 아무 일도 일어나지 않는다.
            // 주면 눌러도 반응 없는 줄이 생긴다. 이름 바꾸기는 아이콘 줄이 자체 기능으로 한다
            if context_menu
                .QueryContextMenu(menu, 0, ID_FIRST, ID_LAST, CMF_NORMAL)
                .is_err()
            {
                let _ = DestroyMenu(menu);
                return None;
            }
            let owner_draw = (context_menu.cast().ok(), context_menu.cast().ok());
            Some(ShellMenu {
                context_menu,
                owner_draw,
                menu,
            })
        }
    }

    /// 맨 위 단계의 줄들을 읽는다
    pub fn model(&self) -> Vec<ShellMenuItem> {
        // 안전성: `open`이 만든 유효한 HMENU를 읽기만 한다
        unsafe { read_menu(self.menu, &self.context_menu) }
    }

    /// 하위 메뉴를 펼친다 — **읽기 전에 셸에게 채워 달라고 먼저 묻는다**.
    ///
    /// `보내기` 같은 하위 메뉴는 `WM_INITMENUPOPUP`을 받는 순간에야 항목이 생긴다. 종전
    /// 경로에서는 창 프로시저가 그 메시지를 받아 넘겼지만, 우리가 그리는 메뉴에는 그 메시지가
    /// 오지 않으므로 **직접 보낸다**
    pub fn expand(&self, handle: SubmenuHandle) -> Vec<ShellMenuItem> {
        let submenu =
            windows::Win32::UI::WindowsAndMessaging::HMENU(handle.menu as *mut core::ffi::c_void);
        // 안전성: 손잡이는 `model`이 이 메뉴에서 읽어 준 것이다. 그 메뉴가 이미 지워졌으면
        // `GetMenuItemCount`가 `-1`을 주어 빈 목록으로 끝난다(잘못된 핸들에도 안전하게 실패한다)
        unsafe {
            // 규격: `wParam`은 하위 메뉴 핸들, `lParam`의 하위 워드는 **부모 안에서의 자리**,
            // 상위 워드는 시스템 메뉴 여부다. 우리 것은 팝업이라 상위 워드가 0이고, 자리는
            // 손잡이가 들고 온 값을 그대로 쓴다 — 0으로 뭉개면 같은 항목이 여러 하위 메뉴에
            // 걸린 확장이 엉뚱한 쪽을 채운다
            let wparam = WPARAM(submenu.0 as usize);
            let lparam = LPARAM(handle.index as isize);
            if let Some(cm3) = &self.owner_draw.1 {
                let mut result = LRESULT(0);
                let _ = cm3.HandleMenuMsg2(WM_INITMENUPOPUP, wparam, lparam, Some(&mut result));
            } else if let Some(cm2) = &self.owner_draw.0 {
                let _ = cm2.HandleMenuMsg(WM_INITMENUPOPUP, wparam, lparam);
            }
            read_menu(submenu, &self.context_menu)
        }
    }

    /// 고른 항목을 실행한다 — 실패는 셸이 자기 UI로 알린다
    pub fn invoke(&self, id: u32, owner: HWND) {
        if id < ID_FIRST {
            return;
        }
        // 안전성: 이 메뉴를 채운 인터페이스가 아직 살아 있다(같은 구조체가 쥐고 있다)
        unsafe {
            let info = CMINVOKECOMMANDINFO {
                cbSize: size_of::<CMINVOKECOMMANDINFO>() as u32,
                hwnd: owner,
                // MAKEINTRESOURCE 관례: 하위 16비트가 명령 오프셋인 유사 포인터
                lpVerb: PCSTR((id - ID_FIRST) as usize as *const u8),
                nShow: SW_SHOWNORMAL.0,
                ..Default::default()
            };
            let _ = self.context_menu.InvokeCommand(&info);
        }
    }

    /// 그 항목의 verb 이름 — 아이콘 줄이 `공유` 같은 표준 동작을 찾는 데 쓴다.
    ///
    /// 이름을 주지 않는 확장도 많아 `None`이 흔하다
    pub fn verb(&self, id: u32) -> Option<String> {
        // 안전성: 이 메뉴를 채운 인터페이스가 아직 살아 있다(같은 구조체가 쥐고 있다)
        unsafe { verb_of(&self.context_menu, id) }
    }
}

impl Drop for ShellMenu {
    fn drop(&mut self) {
        // 안전성: `open`이 만든 핸들을 한 번만 지운다(이 값을 소비하는 자리가 여기뿐이다)
        unsafe {
            let _ = DestroyMenu(self.menu);
        }
    }
}

/// HMENU 한 단계를 훑어 줄 목록으로 바꾼다.
///
/// **읽을 수 없는 줄은 뺀다** — 글자 없이 스스로 그리는 확장(`MFT_OWNERDRAW`)이 그렇다.
/// verb 이름이라도 있으면 그것을 쓰고, 그마저 없으면 그 줄만 빠진다(`추가 옵션 표시`가
/// 여는 표준 메뉴에서는 정상으로 보인다).
///
/// 안전성: 유효한 HMENU에만 부른다. 읽어 온 문자열 버퍼는 이 함수가 소유한다
unsafe fn read_menu(
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    cm: &IContextMenu,
) -> Vec<ShellMenuItem> {
    // 안전성: 위 주석 참조
    unsafe {
        let count = GetMenuItemCount(Some(menu));
        if count <= 0 {
            return Vec::new();
        }
        let mut out: Vec<ShellMenuItem> = Vec::with_capacity(count as usize);
        for index in 0..count {
            // 글자는 길이를 먼저 물어야 해서 두 번 읽는다 — 첫 번째는 `cch`만 받는다
            let mut info = MENUITEMINFOW {
                cbSize: size_of::<MENUITEMINFOW>() as u32,
                fMask: MIIM_FTYPE | MIIM_ID | MIIM_STATE | MIIM_STRING | MIIM_SUBMENU | MIIM_BITMAP,
                ..Default::default()
            };
            if GetMenuItemInfoW(menu, index as u32, true, &mut info).is_err() {
                continue;
            }
            let separator = info.fType & MFT_SEPARATOR
                != windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_TYPE(0);
            if separator {
                out.push(separator_row());
                continue;
            }
            // 줄바꿈 표시는 우리가 세로로만 그리므로 뜻이 없다 — 구분선으로 접는다
            let breaks = MFT_MENUBREAK | MFT_MENUBARBREAK;
            if info.fType & breaks != windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_TYPE(0) {
                out.push(separator_row());
            }

            let raw = read_label(menu, index as u32, &mut info);
            let raw = match raw {
                Some(text) if !text.is_empty() => text,
                // 글자를 읽지 못했다 — verb 이름이라도 있으면 그것으로 대신한다
                _ => match verb_of(cm, info.wID) {
                    Some(name) => name,
                    None => continue,
                },
            };
            let (label, shortcut) = split_shortcut(&raw);
            if label.is_empty() {
                continue;
            }
            out.push(ShellMenuItem {
                id: info.wID,
                label,
                shortcut,
                icon: is_real_bitmap(info.hbmpItem)
                    .then(|| bgra_from_hbitmap(info.hbmpItem))
                    .flatten(),
                enabled: info.fState & MFS_DISABLED
                    == windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_STATE(0),
                checked: info.fState & MFS_CHECKED
                    != windows::Win32::UI::WindowsAndMessaging::MENU_ITEM_STATE(0),
                separator: false,
                submenu: (!info.hSubMenu.is_invalid()).then_some(SubmenuHandle {
                    menu: info.hSubMenu.0 as isize,
                    index: index as u32,
                }),
            });
        }
        collapse_separators(out)
    }
}

/// `hbmpItem`이 진짜 비트맵인가 — 시스템 의사 핸들이면 아니다.
///
/// `HBMMENU_*`(닫기·최소화 단추 등)는 **핸들이 아니라 1~11의 작은 표식**이다. 창 시스템
/// 메뉴용이라 셸 확장이 쓰는 일은 드물지만, 걸러 두지 않으면 그 값이 유효한 핸들처럼
/// `GetObjectW`에 넘어간다 — 지금은 GDI가 실패로 돌려주어 탈이 없지만 **그것은 우연이지
/// 우리가 정한 것이 아니다**. `HBMMENU_CALLBACK`(-1)과 널은 `fs::bitmap`이 이미 거른다
fn is_real_bitmap(hbm: windows::Win32::Graphics::Gdi::HBITMAP) -> bool {
    !(1..=11).contains(&(hbm.0 as isize))
}

/// 구분선 한 줄 — 나머지 필드는 뜻이 없다
fn separator_row() -> ShellMenuItem {
    ShellMenuItem {
        id: 0,
        label: String::new(),
        shortcut: String::new(),
        icon: None,
        enabled: false,
        checked: false,
        separator: true,
        submenu: None,
    }
}

/// 그 줄의 글자를 읽는다 — 길이를 먼저 묻고 그만큼 받는다.
///
/// 안전성: `info`는 방금 `GetMenuItemInfoW`로 채운 것이고, 버퍼는 이 함수가 잡아 넘긴다
unsafe fn read_label(
    menu: windows::Win32::UI::WindowsAndMessaging::HMENU,
    index: u32,
    info: &mut MENUITEMINFOW,
) -> Option<String> {
    // 스스로 그리는 줄(`MFT_OWNERDRAW`)은 글자를 담지 않는 것이 보통이라 아래 `cch == 0`에서
    // 걸린다 — 담아 두는 확장이 있으면 그대로 읽힌다. 종류로 미리 가르지 않는 이유가 그것이다
    // (가르면 글자를 담은 확장까지 함께 버린다)
    if info.cch == 0 {
        return None;
    }
    // 안전성: 위 주석 참조 — 널 종단 자리를 하나 더 잡는다
    unsafe {
        let mut buffer = vec![0u16; info.cch as usize + 1];
        info.dwTypeData = PWSTR(buffer.as_mut_ptr());
        info.cch = buffer.len() as u32;
        info.fMask = MIIM_STRING;
        if GetMenuItemInfoW(menu, index, true, info).is_err() {
            return None;
        }
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..len]))
    }
}

/// verb 이름을 읽는다 — `ShellMenu::verb`와 같은 절차이며 내부용이다.
///
/// 안전성: 유효한 인터페이스에만 부른다
unsafe fn verb_of(cm: &IContextMenu, id: u32) -> Option<String> {
    if id < ID_FIRST {
        return None;
    }
    // verb는 `copy`·`delete`·`Windows.Share`처럼 짧은 식별자라 64면 넉넉하다 — 넘치면
    // `GetCommandString`이 실패하고 그 줄은 이름 없이 빠진다(아이콘 줄이 표준 동작을 못 찾을 뿐)
    let mut buffer = [0u16; 64];
    // 안전성: 위 주석 참조
    unsafe {
        cm.GetCommandString(
            (id - ID_FIRST) as usize,
            GCS_VERBW,
            None,
            PSTR(buffer.as_mut_ptr() as *mut u8),
            buffer.len() as u32,
        )
        .ok()?;
    }
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    (len > 0).then(|| String::from_utf16_lossy(&buffer[..len]))
}

/// 메뉴 문자열을 `(이름, 단축키)`로 가른다.
///
/// 셸은 `속성(&R)\tAlt+Enter`처럼 **탭 뒤에** 단축키를 붙여 온다. `&`는 키보드로 고를 때
/// 쓰는 표시라 화면에 그대로 두면 이름이 지저분해진다 — `&&`는 진짜 `&` 한 글자다
pub fn split_shortcut(raw: &str) -> (String, String) {
    let (name, shortcut) = match raw.split_once('\t') {
        Some((name, shortcut)) => (name, shortcut.trim()),
        None => (raw, ""),
    };
    (strip_accelerator(name), shortcut.to_owned())
}

/// `&` 액셀러레이터 표시를 뗀다 — `&&`는 글자 하나로 남긴다
fn strip_accelerator(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut chars = name.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        if chars.peek() == Some(&'&') {
            chars.next();
            out.push('&');
        }
    }
    out.trim().to_owned()
}

/// 맨 앞·맨 뒤·연달아 오는 구분선을 접는다 — 그대로 그리면 빈 칸처럼 보인다
fn collapse_separators(items: Vec<ShellMenuItem>) -> Vec<ShellMenuItem> {
    let mut out: Vec<ShellMenuItem> = Vec::with_capacity(items.len());
    for item in items {
        if item.separator && out.last().is_none_or(|last| last.separator) {
            continue;
        }
        out.push(item);
    }
    while out.last().is_some_and(|last| last.separator) {
        out.pop();
    }
    out
}

/// 절차 본체 — 실패는 Err로 조기 종료(호출부가 무시)
unsafe fn show_inner(owner: HWND, folder: &Path, items: &[PathBuf], x: i32, y: i32) -> Result<()> {
    // 안전성: 이 함수 전체가 COM/Win32 FFI — 포인터는 지역 소유, 해제 경로 명시
    unsafe {
        let context_menu: IContextMenu = if items.is_empty() {
            background_menu(owner, folder)?
        } else {
            items_menu(owner, items)?
        };

        let menu = CreatePopupMenu()?;
        let result = (|| -> Result<()> {
            context_menu
                .QueryContextMenu(menu, 0, ID_FIRST, ID_LAST, CMF_NORMAL)
                .ok()?;

            // 서브메뉴(보내기 등) 동적 채움을 위한 포워딩 대상 등록 (T2 Edge)
            let cm2: Option<IContextMenu2> = context_menu.cast().ok();
            let cm3: Option<IContextMenu3> = context_menu.cast().ok();
            ACTIVE_MENU.set(Some((cm2, cm3)));

            let cmd = TrackPopupMenuEx(
                menu,
                (TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_TOPALIGN).0,
                x,
                y,
                owner,
                None,
            );
            ACTIVE_MENU.set(None);

            let cmd = cmd.0 as u32;
            if cmd >= ID_FIRST {
                // 선택된 항목 실행 — 실패(메뉴 표시 중 대상 삭제 등)는 셸이 UI로 알림
                let info = CMINVOKECOMMANDINFO {
                    cbSize: size_of::<CMINVOKECOMMANDINFO>() as u32,
                    hwnd: owner,
                    // MAKEINTRESOURCE 관례: 하위 16비트가 명령 오프셋인 유사 포인터
                    lpVerb: PCSTR((cmd - ID_FIRST) as usize as *const u8),
                    nShow: SW_SHOWNORMAL.0,
                    ..Default::default()
                };
                let _ = context_menu.InvokeCommand(&info);
            }
            Ok(())
        })();
        // 메뉴 핸들은 성공·실패 무관하게 해제
        let _ = DestroyMenu(menu);
        result
    }
}

/// 선택 항목들의 공통 부모 IShellFolder에서 IContextMenu 획득.
///
/// **계약: `items`는 비어 있으면 안 된다** — 부모 폴더를 첫 항목(`pidls[0]`)으로 정하기 때문이다.
/// 빈 목록은 배경 메뉴를 뜻하므로 호출부(`show_inner`)가 `background_menu`로 분기시킨다
unsafe fn items_menu(owner: HWND, items: &[PathBuf]) -> Result<IContextMenu> {
    debug_assert!(
        !items.is_empty(),
        "items_menu는 항목이 하나 이상이어야 한다 (빈 목록은 background_menu 담당)"
    );
    // 안전성: PIDL은 이 함수에서 생성·해제(ILFree). 자식 PIDL 포인터는 절대 PIDL 내부를
    // 가리키므로(ILFindLastID) 절대 PIDL 해제 전까지만 사용한다
    unsafe {
        let mut pidls: Vec<*mut ITEMIDLIST> = Vec::with_capacity(items.len());
        let result = (|| -> Result<IContextMenu> {
            for path in items {
                let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
                SHParseDisplayName(
                    &HSTRING::from(path.as_os_str()),
                    None::<&windows::Win32::System::Com::IBindCtx>,
                    &mut pidl,
                    0,
                    None,
                )?;
                pidls.push(pidl);
            }
            // 모든 항목은 같은 폴더(파일 목록 한 화면)에서 왔으므로 부모는 첫 항목 기준
            let parent: IShellFolder = SHBindToParent(pidls[0], None)?;
            let children: Vec<*const ITEMIDLIST> = pidls
                .iter()
                .map(|&p| ILFindLastID(p) as *const ITEMIDLIST)
                .collect();
            parent.GetUIObjectOf(owner, &children, None)
        })();
        for pidl in pidls {
            ILFree(Some(pidl as *const ITEMIDLIST));
        }
        result
    }
}

/// 폴더 자신의 배경 컨텍스트 메뉴(새로 만들기 포함) 획득
unsafe fn background_menu(owner: HWND, folder: &Path) -> Result<IContextMenu> {
    // 안전성: PIDL 생성·해제 이 함수 안에서 완결
    unsafe {
        let mut pidl: *mut ITEMIDLIST = std::ptr::null_mut();
        SHParseDisplayName(
            &HSTRING::from(folder.as_os_str()),
            None::<&windows::Win32::System::Com::IBindCtx>,
            &mut pidl,
            0,
            None,
        )?;
        let result = (|| -> Result<IContextMenu> {
            let desktop = SHGetDesktopFolder()?;
            let shell_folder: IShellFolder =
                desktop.BindToObject(pidl, None::<&windows::Win32::System::Com::IBindCtx>)?;
            shell_folder.CreateViewObject(owner)
        })();
        ILFree(Some(pidl as *const ITEMIDLIST));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 액셀러레이터_표시를_떼고_단축키를_가른다() {
        // 셸이 주는 실제 모양이다 — 그대로 그리면 이름에 `&`와 탭이 섞여 나온다
        assert_eq!(
            split_shortcut("속성(&R)\tAlt+Enter"),
            ("속성(R)".to_owned(), "Alt+Enter".to_owned())
        );
        assert_eq!(
            split_shortcut("경로로 복사\tCtrl+Shift+C"),
            ("경로로 복사".to_owned(), "Ctrl+Shift+C".to_owned())
        );
    }

    #[test]
    fn 단축키가_없으면_이름만_남는다() {
        assert_eq!(
            split_shortcut("&보내기"),
            ("보내기".to_owned(), String::new())
        );
    }

    #[test]
    fn 두_번_적은_앰퍼샌드는_글자_하나다() {
        // `&&`는 진짜 `&` 한 글자다 — 이것까지 떼면 이름이 달라진다
        assert_eq!(
            split_shortcut("Tom && Jerry"),
            ("Tom & Jerry".to_owned(), String::new())
        );
    }

    #[test]
    fn 구분선은_앞뒤와_연속을_접는다() {
        // 접지 않으면 메뉴 위아래에 빈 칸이 생기고 가운데에도 두 줄이 겹친다
        let 줄 = |name: &str| ShellMenuItem {
            label: name.to_owned(),
            ..separator_row()
        };
        let mut 항목 = 줄("열기");
        항목.separator = false;
        let mut 둘째 = 줄("복사");
        둘째.separator = false;

        let 접힘 = collapse_separators(vec![
            separator_row(),
            separator_row(),
            항목.clone(),
            separator_row(),
            separator_row(),
            둘째.clone(),
            separator_row(),
        ]);
        let 모양: Vec<bool> = 접힘.iter().map(|item| item.separator).collect();
        assert_eq!(
            모양,
            vec![false, true, false],
            "가운데 구분선 하나만 남는다"
        );
    }

    #[test]
    fn 구분선만_있으면_아무것도_남지_않는다() {
        assert!(collapse_separators(vec![separator_row(), separator_row()]).is_empty());
    }
}
