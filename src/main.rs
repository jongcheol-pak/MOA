//! 진입점 — COM 초기화, 메인 창 생성, 메시지 루프
#![windows_subsystem = "windows"]

mod app;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MB_ICONERROR, MB_OK, MSG, MessageBoxW, TranslateMessage,
};
use windows::core::w;

fn main() {
    if let Err(e) = run() {
        // 치명 초기화 실패만 메시지박스로 알린다 (plan D6)
        let text = windows::core::HSTRING::from(format!(
            "프로그램을 시작할 수 없습니다.\n오류: {}",
            e.message()
        ));
        unsafe {
            MessageBoxW(None, &text, w!("파일 탐색기"), MB_OK | MB_ICONERROR);
        }
        std::process::exit(1);
    }
}

fn run() -> windows::core::Result<()> {
    // 셸 COM(IContextMenu 등)을 쓰므로 STA로 초기화
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };

    let _window = app::window::MainWindow::create()?;

    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        CoUninitialize();
    }
    Ok(())
}
