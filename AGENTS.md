# AGENTS.md — Agent Guide

> Rust 프로젝트용 가이드.

## Stack
- **언어**: Rust stable (1.80+)
- **에디션**: 2024
- **주요 crates**: windows (windows-rs — Win32·COM·셸 API), serde + serde_json (설정 직렬화)
- **빌드 도구**: Cargo
- **대상 플랫폼**: Windows 11 이상, x64 전용 (GUI 앱, 콘솔 창 없음)

## Build & Test
- **Build (debug)**: `cargo build`
- **Build (release)**: `cargo build --release`
- **Run**: `cargo run`
- **Test**: `cargo test`
- **Lint**: `cargo clippy --all-targets -- -D warnings`
- **Format check**: `cargo fmt --check`
- **Format**: `cargo fmt`

## 데이터 접근
- **DB/스토어**: 없음 (설정은 `%APPDATA%\FileExplorer\settings.json` 로컬 파일)

## Repository Structure

```
<repo>/
├── Cargo.toml
├── docs/
│   └── prd.md               # 승인된 PRD (요구사항 정본)
├── src/
│   ├── main.rs              # 진입점 — COM 초기화, 세션 로드, egui 창 실행
│   ├── ui/                  # egui(eframe/glow) UI 계층 — 화면·입력 전부
│   ├── app/                 # 순수 로직 — 워크스페이스·분할 레이아웃·세션 스키마
│   ├── panel/               # 순수 모델 — 탭·히스토리·정렬/표시 규칙
│   └── fs/                  # 디렉터리 열거·감시·아이콘·셸 연동
└── tests/                   # 통합 테스트
```

> `app/{window,sidebar,menu,layout_host}.rs`·`panel/{panel,folder_tree,address_bar}.rs`와
> `panel/{file_list,tabs}.rs`의 Win32 컨트롤 래퍼는 **egui 이식 이전 구현**이다.
> 소스에는 남아 있지만 실행 파일에서는 쓰이지 않으므로, 새 UI 작업은 `src/ui/`에서 한다.

## 산출물·파일 관리
- **빌드 산출물**: `target/` (gitignore)
- **런타임 생성물**: `%APPDATA%\FileExplorer\settings.json` (설정·세션)

## Conventions
- **아키텍처**: 계층형(단일 crate) — 모듈로만 분리 (ui / app / panel / fs). 의존은 단방향이며 `ui`만 상위다: `app`·`panel`·`fs`는 `ui`를 모른다. GUI 도구로 도메인 규칙이 얇아 crate 분리는 하지 않는다.
- **에러 처리**: `Result<T, E>`. Win32 호출 실패는 `windows::core::Result` 전파. `unwrap()`, `expect()` 금지 (테스트·main 진입부 제외).
- **unsafe**: Win32 FFI 특성상 불가피 — 반드시 함수 단위로 격리하고 사유 주석 의무. 안전 래퍼를 만들어 상위 로직에서는 safe 코드만.
- **UI 스레드 원칙**: UI 스레드에서 블로킹 I/O 금지. 디렉터리 열거·감시는 워커 스레드 → 윈도우 메시지로 결과 전달.
- **동시성**: tokio 등 async 런타임 사용 안 함 (GUI 메시지 루프 + std::thread + 채널로 충분).
- **테스트**: 단위는 `#[cfg(test)] mod tests`, 통합은 `tests/`. UI(HWND 필요) 로직은 테스트 비대상 — 순수 로직(레이아웃 트리·정렬·히스토리·직렬화)을 UI에서 분리해 테스트.
- **Cargo.lock**: 커밋
- **파일**: 1500라인 내외, UTF-8, 주석은 한글

## DO NOT
- `target/` 커밋 (gitignore 필수)
- `unsafe` 무분별 사용 — 사유 주석 의무, 래퍼 밖 노출 금지
- `println!` production 로깅 금지 (GUI 앱 — 필요 시 `OutputDebugStringW` 래퍼)
- `panic!` 직접 호출 (예외: main에서 초기화 실패)
- UI 스레드에서 파일시스템 블로킹 호출
- 코드·문서·notes·plan 등 어떤 파일에도 실제 IP·계정·비밀번호·토큰 기록

## Plan Location

```
Plan Location: docs/plans/<YYYY-MM-DD>-<slug>.md   (누적 — 덮어쓰지 않는다)
PRD Location:  docs/prd.md
```

## 추가 정보
- MSRV: stable 최신 (rust-toolchain.toml 미사용, v1 기준)
- CI/CD: 없음 (로컬 빌드)
- 배포: 단일 exe (cargo build --release)
