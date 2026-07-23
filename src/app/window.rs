//! 메인 창 — 클래스 등록·생성·윈도우 프로시저
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, IDC_ARROW, LoadCursorW,
    PostQuitMessage, RegisterClassExW, SW_SHOW, ShowWindow, WINDOW_EX_STYLE, WM_DESTROY,
    WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{PCWSTR, Result, w};

const WINDOW_CLASS: PCWSTR = w!("FileExplorerMainWindow");

/// 메인 창. 생성 후 메시지 루프는 main이 소유한다.
pub struct MainWindow {
    // T3(레이아웃 렌더링)부터 사용 — 사용 시점에 expect가 자동 해제 경고를 낸다
    #[expect(dead_code)]
    pub hwnd: HWND,
}

impl MainWindow {
    pub fn create() -> Result<MainWindow> {
        // 안전성: Win32 창 생성 FFI — 인자는 모두 유효한 정적/스택 값이며 실패는 Result로 전파
        unsafe {
            let instance = GetModuleHandleW(None)?;

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                // 시스템 배경 브러시 관례: COLOR_* + 1 값을 HBRUSH로 전달
                hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut core::ffi::c_void),
                lpszClassName: WINDOW_CLASS,
                ..Default::default()
            };
            if RegisterClassExW(&wc) == 0 {
                return Err(windows::core::Error::from_thread());
            }

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WINDOW_CLASS,
                w!("파일 탐색기"),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                1200,
                800,
                None,
                None,
                Some(instance.into()),
                None,
            )?;
            let _ = ShowWindow(hwnd, SW_SHOW);

            Ok(MainWindow { hwnd })
        }
    }
}

/// 메인 창 프로시저 — 이후 task(T3~)에서 레이아웃·명령 배선이 추가된다
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_DESTROY => {
            // 안전성: 자기 스레드 메시지 큐에 종료 통지
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
