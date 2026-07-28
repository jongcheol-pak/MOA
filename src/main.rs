//! 진입점 — COM 초기화, 세션 로드, egui 창 실행.
//!
//! UI는 `ui` 모듈(eframe/glow)이 전부 담당한다. 모듈 정의는 lib 타깃(lib.rs)에 있고
//! 여기서는 사용만 한다(tests/ 공유).
//!
//! 이식 이전의 Win32 UI 구현(`app::window`·`app::sidebar`·`panel::panel` 등)은 소스에 남아
//! 있지만 이 진입점에서는 쓰지 않는다 — 빌드되는 실행 파일은 이 egui 판 하나다.
#![windows_subsystem = "windows"]

use eframe::egui;
use file_explorer::app;
use file_explorer::app::settings::load_session;
use file_explorer::ui::app::{ExplorerApp, init_com, uninit_com};

fn main() -> eframe::Result {
    let com = init_com();
    // 창 제목 표시줄을 다크로 만드는 프로세스 전역 정책 — 창을 만들기 전에 켜야 적용된다
    app::theme::enable_dark_mode();
    // 창을 만들기 전에 세션을 읽는다 — 지난번 크기·위치로 떠야 한다 (FR-11).
    // 화면 밖으로 저장된 위치는 창이 뜬 뒤 모니터 크기를 알고 나서 앱이 보정한다
    let session = load_session();
    let mut viewport = egui::ViewportBuilder::default().with_title("파일 탐색기");
    match session.as_ref().map(|s| &s.window) {
        Some(window) => {
            viewport = viewport
                .with_position([window.x as f32, window.y as f32])
                .with_inner_size([window.w as f32, window.h as f32])
                .with_maximized(window.maximized);
        }
        None => viewport = viewport.with_inner_size([1100.0, 700.0]),
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let result = eframe::run_native(
        "file_explorer",
        options,
        Box::new(move |cc| Ok(Box::new(ExplorerApp::new(cc, com, session)))),
    );
    // 안전성: init_com과 같은 스레드에서 1회 호출한다 (owned일 때만 실제 해제)
    unsafe { uninit_com(com) };
    result
}
