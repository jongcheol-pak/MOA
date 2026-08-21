//! 자동 업데이트 (FR-62) — GitHub 릴리즈에서 새 판을 찾아 받아 설치한다.
//!
//! 다섯으로 나눈다: `http`(WinHTTP GET)·`sha256`(무결성 대조용 해시)·
//! `release`(릴리즈 조회·버전 비교)·`install`(설치본 판정·내려받기·설치 실행)·
//! `service`(상태와 워커).
//!
//! **상태 기계를 이 파일에 담지 않고 `service`로 뺐다** — 여기에 두면 하위 모듈을 하나
//! 더할 때마다 상태 흐름 300줄을 스크롤해 지나야 하고, 「모듈 구성」과 「상태 흐름」이라는
//! 서로 다른 변경 이유가 한 파일에 섞인다(AGENTS 분할 판정 ①②). 대신 아래에서 재수출해
//! **밖에서 보는 이름은 `app::update::UpdateService` 그대로**다.
//!
//! `ui`를 모른다 — 화면은 여기가 내놓는 상태를 읽어 그릴 뿐이다(계층 단방향).
pub mod http;
pub mod install;
pub mod release;
pub mod service;
pub mod sha256;

pub use release::{ReleaseInfo, UpdateError};
pub use service::{UpdateService, UpdateStatus, Wake};
