# AGENTS.md — Agent Guide

> Rust 프로젝트용 가이드.

## Stack
- **언어**: Rust stable (1.80+)
- **에디션**: 2024
- **주요 crates**: eframe/egui (UI — glow 백엔드), windows (windows-rs — Win32·COM·셸 API), serde + serde_json (설정 직렬화), image (정보 화면 아이콘 디코드·축소 — png feature만), suppaftp (FTP·FTPS), ssh2 (SFTP)
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
- **라이선스 자산 재생성**: `cargo run --example gen_licenses` — 의존성을 더하거나 버전을 올린 뒤 반드시 돌린다. `assets/licenses.json`이 낡으면 `Cargo.lock` 지문 대조 시험이 실패한다
- **아이콘 자산 재생성**: `cargo run --example gen_app_icon` — `docs/AppIcon.png`를 바꾼 뒤에만 돌린다. `assets/app_icon_256.png`를 덮어쓰며, 그 자산은 정보 화면(FR-58)이 읽는다

## 데이터 접근
- **DB/스토어**: 없음 (`%APPDATA%\MOA\settings.json` 로컬 파일 하나에 **세션 + 앱 설정**을 함께 담는다 — 스키마 v3, v2는 승격해 읽는다. 앱 설정(`settings` 객체 — 글꼴·자동 실행·트레이·파일 보기·언어)이 깨져 있어도 세션은 살린다: 그 자리만 기본값으로 되돌린다)
- **레지스트리**: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 `MOA` 값이 **자동 실행 설정의 정본**이다 (설정 파일 값은 사본 — 다른 도구가 지웠을 수 있어 화면에 보일 때마다 다시 읽는다)
- **비밀번호**: `%APPDATA%` 파일에 **DPAPI로 봉인해서만** 담는다 (`remote::secret`). 평문을 파일·로그·문서에 남기지 않는다

## 원격 기능 테스트
- **기본은 실서버가 필요 없다** — `remote::testing`의 가짜 서버·세션이 지연·무응답·연결 거절·대량 목록·`SITE CHMOD` 미지원까지 흉내 낸다. `cargo test`만으로 전부 돈다.
- **실서버로 확인하고 싶을 때**는 환경변수에 주소를 담아 수동으로 돌린다 — 값은 **각자 환경의 것**이며 저장소·문서·커밋 어디에도 적지 않는다.
  ```
  $env:FE_TEST_FTP_URL  = "ftp://<사용자>:<비밀번호>@<호스트>:<포트>/<경로>"
  $env:FE_TEST_SFTP_URL = "sftp://<사용자>:<비밀번호>@<호스트>:<포트>/<경로>"
  cargo run --release     # 앱을 띄워 그 주소로 직접 연결해 본다
  ```
  두 변수가 없으면 자동 테스트는 그대로 전부 통과한다(가짜 서버만 쓴다).

## Repository Structure

```
<repo>/
├── Cargo.toml
├── docs/
│   └── prd.md               # 승인된 PRD (요구사항 정본)
├── assets/                  # 실행 파일에 담기는 생성물·원문 (커밋 대상)
│   ├── licenses.json        # 라이선스 고지 자산 — 생성기가 만든다 (손으로 고치지 않는다)
│   ├── app_icon_256.png     # 정보 화면 아이콘 자산 — 생성기가 만든다 (손으로 고치지 않는다)
│   └── spdx/                # SPDX 표준 전문 — 배포 패키지에 원문이 없는 구성 요소에 쓴다
├── examples/
│   ├── gen_licenses.rs      # 라이선스 자산 생성기 (개발용 — `cargo build`가 빌드하지 않는다)
│   └── gen_app_icon.rs      # 아이콘 자산 생성기 (`docs/AppIcon.png` → 256px)
├── src/
│   ├── main.rs              # 진입점 — COM 초기화, 세션 로드, egui 창 실행
│   ├── ui/                  # egui(eframe/glow) UI 계층 — 화면·입력 전부
│   ├── app/                 # 순수 로직 — 워크스페이스·분할 레이아웃·세션 스키마
│   ├── panel/               # 순수 모델 — 탭·히스토리·정렬/표시 규칙
│   ├── remote/              # 원격 연결 — FTP/FTPS/SFTP 세션, 연결 워커, 전송 큐, 사이트 저장소
│   └── fs/                  # 디렉터리 열거·감시·아이콘·셸 연동
└── tests/                   # 통합 테스트
```

> `app/{window,sidebar,menu,layout_host}.rs`·`panel/{panel,folder_tree,address_bar}.rs`와
> `panel/{file_list,tabs}.rs`의 Win32 컨트롤 래퍼는 **egui 이식 이전 구현**이다.
> 소스에는 남아 있지만 실행 파일에서는 쓰이지 않으므로, 새 UI 작업은 `src/ui/`에서 한다.

## 산출물·파일 관리
- **빌드 산출물**: `target/` (gitignore)
- **런타임 생성물**: `%APPDATA%\MOA\settings.json` (설정·세션)
- **커밋되는 생성물**: 둘 다 **손으로 고치지 않는다** — 생성기가 만든다.
  - `assets/licenses.json` — `examples/gen_licenses.rs`가 만든다. 레지스트리 캐시를 훑어 만들므로 생성은 개발 PC에서만 하고, 빌드·시험은 그 결과만 읽는다(네트워크·캐시 비의존)
  - `assets/app_icon_256.png` — `examples/gen_app_icon.rs`가 `docs/AppIcon.png`에서 만든다. 원본 그림을 바꿨을 때만 다시 돌리면 된다

## Conventions
- **아키텍처**: 계층형(단일 crate) — 모듈로만 분리 (ui / app / panel / fs). 의존은 단방향이며 `ui`만 상위다: `app`·`panel`·`fs`는 `ui`를 모른다. GUI 도구로 도메인 규칙이 얇아 crate 분리는 하지 않는다.
- **에러 처리**: `Result<T, E>`. Win32 호출 실패는 `windows::core::Result` 전파. `unwrap()`, `expect()` 금지 (테스트·main 진입부 제외).
- **unsafe**: Win32 FFI 특성상 불가피 — 반드시 함수 단위로 격리하고 사유 주석 의무. 안전 래퍼를 만들어 상위 로직에서는 safe 코드만.
- **UI 스레드 원칙**: UI 스레드에서 블로킹 I/O 금지. 디렉터리 열거·감시는 워커 스레드가 하고 **결과는 채널로 받아 프레임에서 반영**한다(`ctx.request_repaint()`로 다시 그리게 한다). 윈도우 메시지 통지(`PostMessageW`)는 Win32 판의 방식이며 egui 경로에서는 쓰지 않는다.
- **동시성**: tokio 등 async 런타임 사용 안 함 (GUI 메시지 루프 + std::thread + 채널로 충분).
- **테스트**: 단위는 `#[cfg(test)] mod tests`, 통합은 `tests/`. UI(HWND 필요) 로직은 테스트 비대상 — 순수 로직(레이아웃 트리·정렬·히스토리·직렬화)을 UI에서 분리해 테스트.
- **Cargo.lock**: 커밋
- **아이콘**: **`egui_phosphor`에서만 가져온다** (`egui_phosphor::regular::*`). 디자인 원문의 유니코드 기호(`⏸` U+23F8 · `✕` U+2715 · `⧉` U+29C9 · `▲▼` · `✓` 등)를 문자열로 직접 쓰지 않는다 — 이 앱은 egui 내장 글꼴을 끄고 **맑은 고딕 + phosphor**만 얹으므로, 그 글꼴에 없는 부호점은 두부(`?`)로 그려진다(2026-08-05 실측: 도크 아이콘 3종이 그렇게 나갔다). 규약은 `ui::widgets::is_icon_font`와 그 테스트(`화면_코드에_원본_아이콘_기호가_남아_있지_않다`)가 지킨다
- **화면 문구**: **`i18n` 카탈로그를 거친다** — 소스에 한글을 직접 박지 않는다. 키 하나에 한국어·영어를 함께 적으면(`src/i18n/mod.rs`의 `strings!`) 매크로가 함수로 펼치고, 한쪽을 빠뜨리면 컴파일 오류가 된다. 값이 끼어드는 문구는 매크로가 아니라 `i18n::dynamic`의 손수 쓴 함수로 둔다(조사·어순·복수형이 언어마다 다르다). 규약은 `i18n`의 소스 훑기 테스트(`화면_문구가_카탈로그를_거치지_않은_곳이_없다`)가 지킨다
  - **예외**: 위젯 상태를 잇는 열쇠(`Id::new`·`id_salt`), 서버·파일시스템에서 온 문자열을 살피는 낱말, 개발자에게만 보이는 단언·오류 메시지. 앞의 둘은 화면 언어를 따르면 **동작이 틀어진다**
  - **시험**: 카탈로그 값을 단언할 때 **기대값은 언제나 원문 리터럴**이고, 그 시험은 `i18n::LanguageGuard::lock`으로 언어를 잠근다. 기대값에 카탈로그를 부르면 값이 무엇으로 바뀌어도 통과하고, 잠그지 않으면 병렬 실행에서 다른 시험이 바꾼 언어를 만난다
- **모달 대화**: **`ui::dialog`의 셸을 거친다** — `egui::Modal`을 직접 쓰지 않는다. 프레임(모서리 12px·여백 0·스크림·그림자)과 하단 버튼 줄(전폭 균등 분할·구분선·hover 채움·굵은 주 버튼)이 그 모듈 한 곳에 있고, 대화는 본문만 그린다. 높이를 본문이 정하면 `show`(본문 폭을 준다), 대화가 자기 크기를 스스로 잡으면 `show_fixed`(프레임 크기를 준다)를 쓴다. **본문과 버튼 줄 사이 여백은 셸이 쥔다** — `show`는 본문 Frame의 `BODY_MARGIN`이, `show_fixed`는 `BODY_GAP_BOTTOM`(content 사각형에서 빼 준다)이 그 자리를 만드므로 대화가 따로 빼지 않는다(2026-08-19 사용자 요청 — 각자 관리하면 새 팝업이 빠뜨려 본문이 버튼에 붙는다). 직접 쓰면 팝업 모양이 다시 제각각이 되므로 규약은 `ui::dialog`의 소스 훑기 테스트(`대화는_모두_이_모듈을_거친다`)가 지킨다
- **팝업 메뉴**: **모서리를 각 메뉴가 적지 않는다** — 값의 정본은 `ui::theme::MENU_CORNER_RADIUS`(6px)이고 `apply_dark`가 egui 스타일(`visuals.menu_corner_radius`)에 세우므로, `Frame::menu`를 쓰는 쪽은 아무것도 적지 않으면 같은 모양이 된다. 종전에는 메뉴마다 `.corner_radius(0)`을 덧붙이거나 붙이지 않아 같은 우클릭 메뉴인데 원격 목록은 각지고 설정 메뉴는 둥글었다(2026-08-19 사용자 보고). 모달 대화의 12px(`ui::dialog::CORNER_RADIUS`)과는 별개 부품이다 — 그쪽은 버튼 줄을 낀 팝업이라 더 둥글다. 규약은 `ui::theme`의 소스 훑기 테스트(`팝업_메뉴는_모서리를_따로_적지_않는다`)가 지키며, 그 테스트는 **`src/ui` 하위 폴더까지 재귀로** 훑는다(모달 규약 테스트는 비재귀라 `src/ui/panel/`을 놓친다 — 같은 함정을 반복하지 않는다)
- **예제 타깃(`examples/`)**: 개발용 CLI라 **stdout/stderr 출력을 허용**하고 `fn main() -> Result<_, String>`으로 오류를 종료 코드에 싣는다 (아래 DO NOT의 `println!` 금지는 콘솔 창이 없는 **GUI 실행 파일**을 겨냥한 것이다 — 예제에는 오류를 알릴 다른 수단이 없다). `unwrap`·`expect` 금지는 예제에도 그대로 적용된다
- **파일**: UTF-8, 주석은 한글. **분할은 줄 수가 아니라 책임으로 판정한다** — ① 변경 이유가 둘 이상인가 ② 부분 수정에 전체 읽기가 필요한가 ③ 찾는 데 헤매는가 ④ (반대 가드) 분리하면 관련 로직이 흩어지는가. **①~③ 중 하나라도 「예」이고 ④가 「아니오」면 나누고, 그 밖에는 줄 수와 무관하게 둔다.** 고정 라인 임계를 두지 않는 이유는 그것이 양쪽으로 다 틀리기 때문이다 — 임계 미만이면 아무 신호도 없어 *나누는 편이 나은* 파일을 놓치고, 넘겼다는 이유만으로 단일 책임 파일에 분리 압력을 준다(종전 「1500라인 내외」를 2026-08-20에 이 판정으로 교체했다). 다만 **수천 줄인데 네 질문이 전부 「아니오」면 그 판정 자체를 의심한다**

## DO NOT
- `target/` 커밋 (gitignore 필수)
- `unsafe` 무분별 사용 — 사유 주석 의무, 래퍼 밖 노출 금지
- `println!` production 로깅 금지 (GUI 앱 — 필요 시 `OutputDebugStringW` 래퍼)
- 아이콘 자리에 유니코드 기호 직접 사용 (phosphor 대신 `"✕"` 같은 문자열 — 두부가 된다)
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
