//! 파일 탐색기 라이브러리 타깃 — 통합 테스트(tests/)와 bin 타깃이 내부 모듈을 쓰기 위한 재수출.
//! 실행 진입점은 main.rs(현행 Win32)와 bin/egui_app(이식 중) — 이 crate는 bin+lib 구성
pub mod app;
pub mod fs;
pub mod panel;
pub mod ui;
