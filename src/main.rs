//! 진입점 — COM 초기화, 메인 창 생성, 메시지 루프
#![windows_subsystem = "windows"]

mod app;

use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, MB_ICONERROR, MB_OK, MSG, MessageBoxW, TranslateAcceleratorW,
    TranslateMessage,
};
use windows::core::w;

fn main() {
    if let Err(e) = run() {
        // 치명 초기화 실패만 메시지박스로 알린다 (plan D6)
        let text = windows::core::HSTRING::from(format!(
            "프로그램을 시작할 수 없습니다.\n오류: {}",
            e.message()
        ));
        // 안전성: 초기화 실패 알림용 모달 — 인자는 모두 정적/스택 값
        unsafe {
            MessageBoxW(None, &text, w!("파일 탐색기"), MB_OK | MB_ICONERROR);
        }
        std::process::exit(1);
    }
}

fn run() -> windows::core::Result<()> {
    // 셸 COM(IContextMenu 등)을 쓰므로 STA로 초기화
    // 안전성: COM은 이 스레드에서 1회 초기화, 인자는 정적 상수 — 실패는 .ok()?로 전파
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()? };

    let window = app::window::MainWindow::create()?;

    // 안전성: 표준 Win32 메시지 루프 — msg는 스택 소유, 종료 후 COM 해제 순서 보장
    unsafe {
        let mut msg = MSG::default();
        // GetMessageW는 오류 시 -1을 반환하므로 bool 변환 대신 양수 판정 (공식 문서 명시 함정)
        while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
            // 단축키(Ctrl+\ 등) 우선 처리 — 소비되면 일반 디스패치 생략
            if TranslateAcceleratorW(window.hwnd, window.haccel, &msg) == 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        CoUninitialize();
    }
    Ok(())
}
