//! 알림 영역(트레이) 아이콘 (FR-50).
//!
//! `종료` 설정이 켜져 있으면 **창이 떠 있는 동안에도** 아이콘을 보인다(사용자 결정) —
//! 닫기를 누르기 전에 "이 앱은 트레이로 간다"는 것을 알 수 있어야 하기 때문이다.
//!
//! ## 창을 되살리는 일은 프로시저가 직접 한다
//!
//! eframe은 창이 숨겨져 있으면 UI 콜백을 건너뛸 수 있다(`run_ui = is_visible || …`).
//! 그래서 "채널로 보내고 프레임에서 창을 띄운다"는 방식은 **그 프레임이 오지 않으면
//! 영영 되살아나지 않는다** — 사용자는 작업 관리자로 앱을 죽여야 한다.
//! 창 프로시저는 프레임 루프와 무관하게 메시지를 받으므로, 더블클릭·`실행`을 받으면
//! 그 자리에서 `ShowWindow`+`SetForegroundWindow`를 부른다. 앱에는 **사후 통지**만 보낸다.
use crate::ui::app_icon;
use eframe::egui;
use std::sync::OnceLock;
use std::sync::mpsc::Sender;
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    HICON, ICONINFO, IsIconic, MF_STRING, RegisterWindowMessageW, SW_RESTORE, SW_SHOW,
    SetForegroundWindow, ShowWindow, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP,
    WM_LBUTTONDBLCLK, WM_RBUTTONUP,
};
use windows::core::{HSTRING, w};

/// 트레이가 창에 보내는 메시지.
///
/// **`WM_APP + 1`을 비워 두고 `+ 2`를 쓴다** — 종전에 `fs::enumerate`가 그 번호로 열거 완료를
/// 알렸고(egui로 옮기며 채널만 쓰게 되어 사라졌다), 번호를 당겨 쓰면 그 시절 코드를 되짚을 때
/// 같은 번호가 두 뜻으로 읽힌다. 실행에 영향은 없고 이력을 헷갈리지 않기 위한 예약이다
pub const TRAY_CALLBACK: u32 = WM_APP + 2;
/// 아이콘 식별자 — 이 앱은 아이콘을 하나만 둔다
const TRAY_ICON_ID: u32 = 1;
/// 트레이 아이콘 한 변 (알림 영역 표준 크기)
const TRAY_ICON_PX: u32 = 16;

/// `TrackPopupMenu`가 돌려주는 항목 번호 (0은 "고르지 않음"이라 1부터 쓴다)
const CMD_SHOW: usize = 1;
const CMD_QUIT: usize = 2;

/// 트레이에서 온 통지 — **창을 띄우는 일은 이미 끝난 뒤**다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    /// 창을 되살렸다 (더블클릭 또는 메뉴 `실행`)
    Shown,
    /// 앱을 끝내 달라 (메뉴 `종료`)
    Quit,
    /// 알림 영역이 다시 만들어졌다 — 아이콘을 **다시 올려야 한다** (FR-50 Edge Case).
    ///
    /// 탐색기(`explorer.exe`)가 죽었다 살아나면 그때까지 올려 둔 아이콘이 전부 사라진다.
    /// 다시 올리지 않으면 사용자는 재부팅 전까지 트레이로 앱을 부를 수 없다
    Recreated,
}

/// 알림 영역이 새로 만들어졌음을 알리는 메시지 번호.
///
/// 이 번호는 고정값이 아니라 **시스템이 이름으로 배정**한다(`RegisterWindowMessageW`) —
/// 한 번 얻어 두고 창 프로시저가 매 메시지마다 견준다
fn taskbar_created_message() -> u32 {
    static ID: OnceLock<u32> = OnceLock::new();
    // 안전성: 문자열 상수로 메시지를 등록한다. 같은 이름이면 시스템이 같은 번호를 준다
    *ID.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

/// 창 프로시저가 앱과 이어지는 통로.
///
/// **`OnceLock`에 두는 이유**: `SetWindowSubclass`의 참조 데이터에 `Box::into_raw`를 실으면
/// 그것을 언제 되돌려 받을지가 문제가 된다 — `RemoveWindowSubclass`보다 먼저 해제하면
/// 이후 도착하는 메시지가 해제된 메모리를 읽는다(`ShellHost`는 "창이 파괴되면 서브클래스도
/// 사라진다"는 전제로 `Drop`을 두지 않았다). 앱 인스턴스가 하나뿐이라 프로세스 수명과
/// 같은 자리에 두면 그 문제가 아예 없고 `unsafe` 표면도 늘지 않는다
struct TrayContext {
    tx: Sender<TrayEvent>,
    ctx: egui::Context,
}

static TRAY: OnceLock<TrayContext> = OnceLock::new();

/// 창 프로시저가 쓸 통로를 등록한다 (앱 시작 때 한 번).
///
/// 두 번째 호출은 조용히 무시된다 — 창은 하나뿐이라 두 번 부를 일이 없다
pub fn install_channel(tx: Sender<TrayEvent>, ctx: egui::Context) {
    let _ = TRAY.set(TrayContext { tx, ctx });
}

/// 트레이 아이콘 하나. **`Drop`이 아이콘을 거둔다** — 토글을 끄거나 앱이 끝나면 사라진다
pub struct Tray {
    hwnd: HWND,
    icon: HICON,
}

impl Tray {
    /// 알림 영역에 아이콘을 올린다. 실패하면 `None`(호출부가 토글을 되돌린다)
    pub fn add(hwnd: HWND) -> Option<Tray> {
        let icon = load_icon()?;
        let data = icon_data(hwnd, icon);
        // 안전성: `data`는 스택 소유이고 이 호출 동안만 읽힌다. 실패하면 아이콘을 되돌려 놓는다
        let added = unsafe { Shell_NotifyIconW(NIM_ADD, &data).as_bool() };
        if !added {
            // 안전성: 방금 우리가 만든 아이콘이고 아직 아무 데도 넘기지 않았다
            unsafe {
                let _ = DestroyIcon(icon);
            }
            return None;
        }
        Some(Tray { hwnd, icon })
    }

    /// 툴팁을 지금 언어로 다시 올린다 (FR-53) — 아이콘·콜백은 그대로 둔다.
    ///
    /// 언어를 바꿔도 알림 영역은 스스로 다시 묻지 않으므로, 앱이 `NIM_MODIFY`로
    /// 알려 주지 않으면 툴팁만 옛 언어로 남는다
    pub fn update_tooltip(&self) {
        let data = icon_data(self.hwnd, self.icon);
        // 안전성: 우리가 올린 아이콘을 우리가 고친다. 아이콘이 이미 사라졌으면
        // 실패만 반환하며(`Drop`과 같은 취급) 앱은 그대로 돈다
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        let data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            ..Default::default()
        };
        // 안전성: 우리가 올린 아이콘을 우리가 내린다. 이미 사라졌으면 실패만 반환한다
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            let _ = DestroyIcon(self.icon);
        }
    }
}

/// 알림 영역에 넘길 아이콘 설명
fn icon_data(hwnd: HWND, icon: HICON) -> NOTIFYICONDATAW {
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: TRAY_CALLBACK,
        hIcon: icon,
        ..Default::default()
    };
    // 툴팁은 화면에 보이는 이름이라 언어를 따른다 (FR-53) — `szTip`은 128 UTF-16 단위
    // 고정 배열이고 `zip`이 짧은 쪽에서 멈추므로 이름이 길어져도 넘치지 않는다
    for (slot, ch) in data
        .szTip
        .iter_mut()
        .zip(crate::i18n::app_name().encode_utf16())
    {
        *slot = ch;
    }
    data
}

/// 앱 아이콘을 16px 트레이 아이콘으로 만든다.
///
/// exe에 담긴 것과 같은 그림을 쓴다(`app_icon`) — 트레이용 파일을 따로 두면
/// 하나를 바꿀 때 다른 하나가 남는다
fn load_icon() -> Option<HICON> {
    let image = app_icon::decode(app_icon::ICO_BYTES, TRAY_ICON_PX)?;
    let width = image.width as i32;
    let height = image.height as i32;
    if width <= 0 || height <= 0 {
        return None;
    }
    // 색 성분 순서만 바꾼다(RGBA → BGRA). **행 순서는 건드리지 않는다** —
    // `app_icon::decode`가 이미 위에서 아래로 내주고, 아래 `biHeight`를 음수로 줘서
    // "이 데이터는 top-down"임을 GDI에 명시한다(`ui::icon_tex`가 쓰는 것과 같은 방법).
    // 손으로 뒤집고 방향을 밝히지 않는 API에 넘기면 어느 쪽이 맞는지 코드로 알 수 없다
    let mut bgra = Vec::with_capacity(image.rgba.len());
    for pixel in image.rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    // 안전성: 비트맵·아이콘 핸들을 이 함수 안에서 만들고, 아이콘을 만든 뒤 비트맵은 지운다
    // (`CreateIconIndirect`가 자기 사본을 갖는다 — MSDN)
    unsafe {
        use windows::Win32::Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
            DeleteDC, DeleteObject, SetDIBits,
        };
        let color = CreateBitmap(width, height, 1, 32, None);
        if color.is_invalid() {
            return None;
        }
        let mut header = BITMAPINFO::default();
        header.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        header.bmiHeader.biWidth = width;
        // 음수 = top-down (첫 행이 그림 위쪽) — 뒤집기 없이 그대로 실을 수 있다
        header.bmiHeader.biHeight = -height;
        header.bmiHeader.biPlanes = 1;
        header.bmiHeader.biBitCount = 32;
        header.bmiHeader.biCompression = BI_RGB.0;
        let hdc = CreateCompatibleDC(None);
        let written = SetDIBits(
            Some(hdc),
            color,
            0,
            height as u32,
            bgra.as_ptr().cast(),
            &header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        if written == 0 {
            let _ = DeleteObject(color.into());
            return None;
        }
        // 마스크는 32비트 알파를 쓸 때도 있어야 한다 — 전부 0(불투명)으로 둔다.
        // 한 줄은 word(2바이트) 정렬이어야 한다(`CreateBitmap` 규약)
        let stride = (width as usize).div_ceil(16) * 2;
        let mask_bits = vec![0u8; stride * height as usize];
        let mask = CreateBitmap(width, height, 1, 1, Some(mask_bits.as_ptr().cast()));
        if mask.is_invalid() {
            let _ = DeleteObject(color.into());
            return None;
        }
        let info = ICONINFO {
            fIcon: true.into(),
            hbmMask: mask,
            hbmColor: color,
            ..Default::default()
        };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
        icon
    }
}

/// 트레이 콜백 메시지를 처리한다. 우리 메시지가 아니면 `false`.
///
/// # 안전성
/// 창 프로시저 안에서만 부른다 — `hwnd`가 우리 창이어야 한다
pub(crate) unsafe fn handle_callback(hwnd: HWND, msg: u32, lparam: LPARAM) -> bool {
    // 탐색기가 되살아났다 — 사라진 아이콘을 다시 올려야 한다
    if msg != 0 && msg == taskbar_created_message() {
        notify(TrayEvent::Recreated);
        return false; // 다른 곳도 이 통지를 봐야 하므로 삼키지 않는다
    }
    // 두 번째 프로세스가 "네가 이미 떠 있으니 나와라"고 알렸다 (FR-51).
    // 하는 일이 트레이 더블클릭과 같으므로 같은 루틴을 그대로 쓴다
    if msg != 0 && msg == crate::app::single_instance::wake_message() {
        // 안전성: 우리 창 프로시저가 받은 메시지라 `hwnd`는 우리 창이다
        unsafe { restore_window(hwnd) };
        notify(TrayEvent::Shown);
        return true;
    }
    if msg != TRAY_CALLBACK {
        return false;
    }
    // 알림 영역은 마우스 메시지를 `lparam`의 아래 16비트에 실어 보낸다
    match (lparam.0 as u32) & 0xFFFF {
        WM_LBUTTONDBLCLK => {
            // 안전성: 유효한 창 핸들에 대한 표시 요청
            unsafe { restore_window(hwnd) };
            notify(TrayEvent::Shown);
            true
        }
        WM_RBUTTONUP => {
            // 안전성: 같은 창의 커서 위치에 메뉴를 띄운다
            match unsafe { popup_menu(hwnd) } {
                Some(CMD_SHOW) => {
                    unsafe { restore_window(hwnd) };
                    notify(TrayEvent::Shown);
                }
                Some(CMD_QUIT) => {
                    // 끝내기 전에 창을 되살린다 — 그래야 프레임이 돌아 세션이 저장된다
                    unsafe { restore_window(hwnd) };
                    notify(TrayEvent::Quit);
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}

/// 창을 보이게 하고 앞으로 가져온다.
///
/// # 안전성
/// 유효한 창 핸들이어야 한다
unsafe fn restore_window(hwnd: HWND) {
    unsafe {
        // 최소화된 창은 `SW_SHOW`만으로는 펴지지 않는다
        let command = if IsIconic(hwnd).as_bool() {
            SW_RESTORE
        } else {
            SW_SHOW
        };
        let _ = ShowWindow(hwnd, command);
        let _ = SetForegroundWindow(hwnd);
    }
}

/// 우클릭 메뉴를 띄우고 고른 항목 번호를 돌려준다.
///
/// # 안전성
/// 유효한 창 핸들이어야 한다
unsafe fn popup_menu(hwnd: HWND) -> Option<usize> {
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_SHOW,
            &HSTRING::from(crate::i18n::tray_show()),
        );
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            CMD_QUIT,
            &HSTRING::from(crate::i18n::tray_quit()),
        );
        let mut point = Default::default();
        let _ = GetCursorPos(&mut point);
        // 메뉴를 띄우기 전에 창을 앞으로 — Win32 관례다. 그러지 않으면 다른 곳을 눌렀을 때
        // 메뉴가 닫히지 않고 화면에 남는다
        let _ = SetForegroundWindow(hwnd);
        let picked = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            None,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        (picked.0 != 0).then_some(picked.0 as usize)
    }
}

/// 앱에 사후 통지를 보내고 화면을 깨운다
fn notify(event: TrayEvent) {
    let Some(tray) = TRAY.get() else {
        return;
    };
    // 수신부가 사라졌으면(앱이 끝나는 중) 전송 실패는 무해하다
    let _ = tray.tx.send(event);
    tray.ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 콜백_메시지_번호가_기존_것과_겹치지_않는다() {
        // 겹치면 폴더 변경 통지가 트레이 조작으로 읽힌다 — `fs::watcher`가 `WM_APP + 9`를 쓴다
        assert_ne!(TRAY_CALLBACK, crate::fs::watcher::WM_APP_DIR_CHANGED);
        assert_eq!(TRAY_CALLBACK, WM_APP + 2);
    }

    #[test]
    fn 메뉴_항목_번호는_0이_아니다() {
        // `TrackPopupMenu`는 "고르지 않음"을 0으로 알린다 — 항목이 0이면 구분할 수 없다
        assert_ne!(CMD_SHOW, 0);
        assert_ne!(CMD_QUIT, 0);
        assert_ne!(CMD_SHOW, CMD_QUIT);
    }

    #[test]
    fn 트레이_아이콘을_만들_수_있다() {
        // exe에 담긴 아이콘에서 16px 아이콘이 나와야 한다 — 못 만들면 트레이 기능 자체가 선다
        let icon = load_icon().expect("트레이 아이콘을 만들지 못했다");
        // 안전성: 방금 우리가 만든 아이콘이고 다른 곳에 넘기지 않았다
        unsafe {
            let _ = DestroyIcon(icon);
        }
    }

    #[test]
    fn 깨우기_메시지가_다른_메시지와_겹치지_않는다() {
        // 겹치면 중복 실행 알림이 트레이 조작이나 폴더 변경 통지로 읽힌다
        let wake = crate::app::single_instance::wake_message();
        assert_ne!(wake, 0);
        assert_ne!(wake, TRAY_CALLBACK);
        assert_ne!(wake, taskbar_created_message());
        assert_ne!(wake, crate::fs::watcher::WM_APP_DIR_CHANGED);
    }

    #[test]
    fn 탐색기_재시작_메시지_번호를_얻는다() {
        // 시스템이 이름으로 배정하는 번호다 — 0이면 등록에 실패한 것이고,
        // 그러면 탐색기가 되살아나도 아이콘을 다시 올리지 못한다
        let id = taskbar_created_message();
        assert_ne!(id, 0, "TaskbarCreated 메시지를 등록하지 못했다");
        // 같은 이름은 언제나 같은 번호를 받는다
        assert_eq!(id, taskbar_created_message());
        assert_ne!(id, TRAY_CALLBACK, "트레이 콜백과 번호가 겹친다");
    }

    #[test]
    fn 우리_메시지가_아니면_처리하지_않는다() {
        // 안전성: 창 핸들을 쓰지 않는 경로만 탄다(메시지 번호가 달라 즉시 반환)
        let handled = unsafe { handle_callback(HWND::default(), WM_APP, LPARAM(0)) };
        assert!(!handled, "남의 메시지를 가로챘다");
    }
}
