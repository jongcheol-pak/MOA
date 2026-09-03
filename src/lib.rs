//! MOA 라이브러리 타깃 — 통합 테스트(tests/)와 bin 타깃이 내부 모듈을 쓰기 위한 재수출.
//!
//! 실행 진입점은 `main.rs`(egui) 하나다 — 이 crate는 bin+lib 구성.
//! 화면은 `ui`가 그린다 — `app`·`panel`에는 순수 로직만 있다.
pub mod app;
pub mod fs;
/// 화면 문구 카탈로그 — 모듈 선언은 **여기에만** 둔다.
/// `main.rs`에도 두면 같은 파일이 두 모듈로 컴파일돼 전역 현재 언어가 둘이 된다
pub mod i18n;
pub mod panel;
/// 임시 성능 계측 — 일 때만 동작한다 (원인 규명 후 걷어낸다)
pub mod perf;
pub mod remote;
pub mod ui;
