//! MOA 라이브러리 타깃 — 통합 테스트(tests/)와 bin 타깃이 내부 모듈을 쓰기 위한 재수출.
//!
//! 실행 진입점은 `main.rs`(egui) 하나다 — 이 crate는 bin+lib 구성.
//! `app`·`panel`에는 egui 이식 이전의 Win32 UI 구현이 남아 있지만 진입점이 쓰지 않는다.
pub mod app;
pub mod fs;
/// 화면 문구 카탈로그 — 모듈 선언은 **여기에만** 둔다.
/// `main.rs`에도 두면 같은 파일이 두 모듈로 컴파일돼 전역 현재 언어가 둘이 된다
pub mod i18n;
pub mod panel;
pub mod remote;
pub mod ui;
