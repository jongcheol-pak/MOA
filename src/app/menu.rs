//! 메뉴 바·액셀러레이터 — 분할 명령 (plan D10, 한국어 문구)
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_B, VK_F5, VK_LEFT, VK_OEM_5, VK_RIGHT, VK_T, VK_UP, VK_W,
};
use windows::Win32::UI::WindowsAndMessaging::{
    ACCEL, AppendMenuW, CreateAcceleratorTableW, CreateMenu, EnableMenuItem, FALT, FCONTROL,
    FSHIFT, FVIRTKEY, HACCEL, HMENU, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
    MF_STRING, SetMenu,
};
use windows::core::{Result, w};

/// WM_COMMAND 명령 id (u16 범위)
pub const IDM_SPLIT_H: u32 = 101;
pub const IDM_SPLIT_V: u32 = 102;
pub const IDM_CLOSE_PANE: u32 = 103;
pub const IDM_NAV_BACK: u32 = 104;
pub const IDM_NAV_FORWARD: u32 = 105;
pub const IDM_NAV_UP: u32 = 106;
pub const IDM_TAB_NEW: u32 = 107;
pub const IDM_TAB_CLOSE: u32 = 108;
pub const IDM_TREE_TOGGLE: u32 = 109;
pub const IDM_REFRESH: u32 = 110;
/// 워크스페이스 명령 (FR-16·FR-18) — 사이드바 컨텍스트 메뉴와 메뉴 바가 공유한다
pub const IDM_WS_NEW: u32 = 111;
pub const IDM_WS_RENAME: u32 = 112;
pub const IDM_WS_DELETE: u32 = 113;
/// 사이드바 접기/펼치기 (FR-19) — 접힌 상태에서 되돌아오는 유일한 경로이기도 하다 (D11)
pub const IDM_SIDEBAR_TOGGLE: u32 = 114;

/// 메뉴 바를 만들어 창에 붙인다. 반환값은 이후 활성/비활성 갱신용 메뉴 핸들.
pub fn attach_menu(hwnd: HWND) -> Result<HMENU> {
    // 안전성: 메뉴 핸들은 SetMenu로 창에 귀속되어 창 파괴 시 함께 해제된다
    unsafe {
        let bar = CreateMenu()?;
        let view = CreateMenu()?;
        AppendMenuW(
            view,
            MF_STRING,
            IDM_SPLIT_H as usize,
            w!("좌우 분할(&H)\tCtrl+\\"),
        )?;
        AppendMenuW(
            view,
            MF_STRING,
            IDM_SPLIT_V as usize,
            w!("상하 분할(&V)\tCtrl+Shift+\\"),
        )?;
        AppendMenuW(
            view,
            MF_STRING,
            IDM_CLOSE_PANE as usize,
            w!("패널 닫기(&C)\tCtrl+Shift+W"),
        )?;
        AppendMenuW(view, MF_SEPARATOR, 0, None)?;
        AppendMenuW(
            view,
            MF_STRING,
            IDM_TREE_TOGGLE as usize,
            w!("폴더 트리(&T)"),
        )?;
        AppendMenuW(
            view,
            MF_STRING,
            IDM_REFRESH as usize,
            w!("새로 고침(&R)\tF5"),
        )?;
        AppendMenuW(
            view,
            MF_STRING,
            IDM_SIDEBAR_TOGGLE as usize,
            w!("워크스페이스 사이드바(&S)\tCtrl+B"),
        )?;
        AppendMenuW(bar, MF_POPUP, view.0 as usize, w!("보기(&V)"))?;

        let go = CreateMenu()?;
        AppendMenuW(go, MF_STRING, IDM_NAV_BACK as usize, w!("뒤로(&B)\tAlt+←"))?;
        AppendMenuW(
            go,
            MF_STRING,
            IDM_NAV_FORWARD as usize,
            w!("앞으로(&F)\tAlt+→"),
        )?;
        AppendMenuW(
            go,
            MF_STRING,
            IDM_NAV_UP as usize,
            w!("상위 폴더(&U)\tAlt+↑"),
        )?;
        AppendMenuW(bar, MF_POPUP, go.0 as usize, w!("이동(&G)"))?;

        let tab = CreateMenu()?;
        AppendMenuW(
            tab,
            MF_STRING,
            IDM_TAB_NEW as usize,
            w!("새 탭(&N)\tCtrl+T"),
        )?;
        AppendMenuW(
            tab,
            MF_STRING,
            IDM_TAB_CLOSE as usize,
            w!("탭 닫기(&C)\tCtrl+W"),
        )?;
        AppendMenuW(bar, MF_POPUP, tab.0 as usize, w!("탭(&T)"))?;

        let workspace = CreateMenu()?;
        append_workspace_items(workspace)?;
        AppendMenuW(bar, MF_POPUP, workspace.0 as usize, w!("워크스페이스(&W)"))?;

        SetMenu(hwnd, Some(bar))?;
        Ok(bar)
    }
}

/// 단축키 테이블 (plan D10): Ctrl+\ 좌우, Ctrl+Shift+\ 상하, Ctrl+Shift+W 패널 닫기
pub fn create_accels() -> Result<HACCEL> {
    let accels = [
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_OEM_5.0, // '\' 키
            cmd: IDM_SPLIT_H as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_OEM_5.0,
            cmd: IDM_SPLIT_V as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL | FSHIFT,
            key: VK_W.0,
            cmd: IDM_CLOSE_PANE as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FALT,
            key: VK_LEFT.0,
            cmd: IDM_NAV_BACK as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FALT,
            key: VK_RIGHT.0,
            cmd: IDM_NAV_FORWARD as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FALT,
            key: VK_UP.0,
            cmd: IDM_NAV_UP as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_T.0,
            cmd: IDM_TAB_NEW as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY,
            key: VK_F5.0,
            cmd: IDM_REFRESH as u16,
        },
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_W.0,
            cmd: IDM_TAB_CLOSE as u16,
        },
        // Ctrl 조합만 액셀러레이터로 둔다 — 무수식 키(F2·Delete)는 편집 컨트롤 입력을 가로챈다 (D16)
        ACCEL {
            fVirt: FVIRTKEY | FCONTROL,
            key: VK_B.0,
            cmd: IDM_SIDEBAR_TOGGLE as u16,
        },
    ];
    // 안전성: 배열은 호출 동안 유효하며 OS가 내부 복사본을 만든다
    unsafe { CreateAcceleratorTableW(&accels) }
}

/// 워크스페이스 명령 3종을 메뉴에 추가한다 (메뉴 바·컨텍스트 메뉴 공용 — 문구 일치 보장)
pub fn append_workspace_items(menu: HMENU) -> Result<()> {
    // 안전성: 유효한 메뉴 핸들에 항목 추가
    unsafe {
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_WS_NEW as usize,
            w!("새 워크스페이스(&N)"),
        )?;
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_WS_RENAME as usize,
            w!("이름 바꾸기(&R)\tF2"),
        )?;
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_WS_DELETE as usize,
            w!("삭제(&D)\tDelete"),
        )?;
    }
    Ok(())
}

/// 워크스페이스 수에 따라 "삭제" 활성/비활성 갱신 (마지막 1개는 삭제 불가 — D8)
pub fn update_workspace_enabled(menu: HMENU, workspace_count: usize) {
    let flags = if workspace_count > 1 {
        MF_BYCOMMAND | MF_ENABLED
    } else {
        MF_BYCOMMAND | MF_GRAYED
    };
    // 안전성: 유효한 메뉴 핸들·명령 id에 대한 상태 변경뿐
    unsafe {
        let _ = EnableMenuItem(menu, IDM_WS_DELETE, flags);
    }
}

/// 패널 수에 따라 "패널 닫기" 메뉴 활성/비활성 갱신 (마지막 1개는 닫기 불가 — FR-2)
pub fn update_close_enabled(menu: HMENU, panel_count: usize) {
    let flags = if panel_count > 1 {
        MF_BYCOMMAND | MF_ENABLED
    } else {
        MF_BYCOMMAND | MF_GRAYED
    };
    // 안전성: 유효한 메뉴 핸들·명령 id에 대한 상태 변경뿐
    unsafe {
        let _ = EnableMenuItem(menu, IDM_CLOSE_PANE, flags);
    }
}
