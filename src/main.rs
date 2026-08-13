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
use file_explorer::ui::window_start;

fn main() -> eframe::Result {
    let com = init_com();
    // 셸 팝업 메뉴를 다크로 만드는 프로세스 전역 정책 — 창을 만들기 전에 켜야 적용된다.
    // 제목 표시줄은 앱이 직접 그리므로(FR-22) 이 정책의 대상이 아니지만,
    // 우클릭 셸 컨텍스트 메뉴(FR-8)는 여전히 이 정책으로 다크가 된다
    app::theme::enable_dark_mode();
    // 창을 만들기 전에 세션을 읽는다 — 지난번 크기·위치로 떠야 한다 (FR-11).
    // 화면 밖으로 저장된 위치는 창이 뜬 뒤 모니터 크기를 알고 나서 앱이 보정한다
    let session = load_session();
    // 창 장식을 끄고 제목 표시줄을 앱이 그린다 (FR-22) — 그 줄에 사이드바 토글·설정 버튼을 두기 위함.
    // 대가로 창 그림자·둥근 모서리를 잃는다(eframe에는 winit의 무장식 그림자 확장에 닿을 길이 없다)
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("파일 탐색기")
        .with_decorations(false);
    match session.as_ref().map(|s| &s.window) {
        Some(window) => {
            // **최대화는 여기서 걸지 않는다** — `with_maximized(true)`는 창을 만든 직후
            // `ShowWindow(SW_MAXIMIZE)`로 이어져 아직 아무것도 그리지 않은 창을 드러내고,
            // 그 흰 사각형이 번쩍인다(`ui::window_start` 참조). 최대화는 첫 프레임을 그린 뒤
            // `ui::app`이 건다. 대신 처음부터 그 자리의 작업 영역만 하게 띄워 크기가 튀지 않게 한다
            let rect =
                window_start::start_rect(window, window_start::work_area_at(window.x, window.y));
            viewport = viewport
                .with_position([rect.x as f32, rect.y as f32])
                .with_inner_size([rect.w as f32, rect.h as f32]);
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
