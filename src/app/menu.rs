//! 메뉴 바·액셀러레이터 — 분할 명령 (plan D10, 한국어 문구)
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_OEM_5, VK_W};
use windows::Win32::UI::WindowsAndMessaging::{
    ACCEL, AppendMenuW, CreateAcceleratorTableW, CreateMenu, EnableMenuItem, FCONTROL, FSHIFT,
    FVIRTKEY, HACCEL, HMENU, MF_BYCOMMAND, MF_ENABLED, MF_GRAYED, MF_POPUP, MF_STRING, SetMenu,
};
use windows::core::{Result, w};

/// WM_COMMAND 명령 id (u16 범위)
pub const IDM_SPLIT_H: u32 = 101;
pub const IDM_SPLIT_V: u32 = 102;
pub const IDM_CLOSE_PANE: u32 = 103;

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
        AppendMenuW(bar, MF_POPUP, view.0 as usize, w!("보기(&V)"))?;
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
    ];
    // 안전성: 배열은 호출 동안 유효하며 OS가 내부 복사본을 만든다
    unsafe { CreateAcceleratorTableW(&accels) }
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
