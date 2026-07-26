//! egui 파일 탐색기 — 진입점 (이식 plan: docs/plans/2026-07-26-egui-migration-part1.md)
//!
//! 현행 Win32 앱(`src/main.rs`)과 **병행 유지**되는 이식 바이너리다.
//! 이식이 완료되면 이 진입점이 `src/main.rs`로 승격되고 Win32 판은 제거된다(part2 T7).
#![windows_subsystem = "windows"]

use eframe::egui;
use file_explorer::ui::app::{ExplorerApp, init_com, uninit_com};

fn main() -> eframe::Result {
    let com = init_com();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_title("파일 탐색기"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "file_explorer",
        options,
        Box::new(move |cc| Ok(Box::new(ExplorerApp::new(cc, com)))),
    );
    // 안전성: init_com과 같은 스레드에서 1회 호출한다 (owned일 때만 실제 해제)
    unsafe { uninit_com(com) };
    result
}
