//! egui 파일 탐색기 — 진입점 (이식 plan: docs/plans/2026-07-26-egui-migration-part1.md)
//!
//! 현행 Win32 앱(`src/main.rs`)과 **병행 유지**되는 이식 바이너리다.
//! 이식이 완료되면 이 진입점이 `src/main.rs`로 승격되고 Win32 판은 제거된다(part2 T7).
#![windows_subsystem = "windows"]

use eframe::egui;
use file_explorer::app::settings::load_session;
use file_explorer::ui::app::{ExplorerApp, init_com, uninit_com};

fn main() -> eframe::Result {
    let com = init_com();
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
