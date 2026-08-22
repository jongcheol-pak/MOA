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
- **라이선스 고지 재생성**: `cargo run --example gen_licenses` — 의존성을 더하거나 버전을 올린 뒤 반드시 돌린다. **산출물이 둘이다**: 앱이 읽는 `assets/licenses.json`과 저장소에서 보는 `THIRD-PARTY-NOTICES.md`(레포 루트). 앞의 것이 낡으면 `Cargo.lock` 지문 대조 시험이 실패한다
- **아이콘 자산 재생성**: `cargo run --example gen_app_icon` — `docs/AppIcon.png`를 바꾼 뒤에만 돌린다. `assets/app_icon_256.png`를 덮어쓰며, 그 자산은 정보 화면(FR-58)이 읽는다
- **설치 파일 생성**: `cargo build --release` → `cargo run --example gen_installer` — `installer/moa.nsi`를 makensis에 넘겨 `target/installer/MOA-Setup-<버전>.exe`를 만든다. **NSIS가 선행 설치돼 있어야 한다**(`winget install NSIS.NSIS`) — 없으면 그 안내와 함께 실패로 끝난다(아무것도 만들지 않고 성공으로 끝나지 않는다). **릴리즈 빌드를 건너뛰어도 같은 방식으로 멈춘다** — `target/release/moa.exe`가 `src`·`assets`·`Cargo.toml`·`Cargo.lock`·`build.rs`·`app.manifest`보다 낡으면 안내와 함께 실패한다(2026-08-21에 낡은 exe가 담긴 설치 파일이 나가 설정이 옛 자리에 생긴 적이 있다)

- **릴리즈 발행**: 아래 순서를 지킨다. **본문에 SHA256이 없으면 앱이 그 릴리즈를 받지 않는다**(FR-62의 무결성 대조가 그 값을 기대값으로 쓴다).
  1. `Cargo.toml`의 `version`을 올린다 (앱이 이 값으로 새 판인지 가린다)
  2. **`cargo run --example gen_licenses`** — 버전을 올리면 `Cargo.lock`이 바뀌고, 라이선스 자산에 담긴 그 **지문이 어긋나 `cargo test`가 실패한다**(`app::licenses`의 `자산이_현재_의존성과_같은_시점의_것이다`). 의존성을 건드리지 않았어도 **버전만 올려도 다시 만들어야 한다** — 2026-08-22에 이 단계를 빠뜨려 실제로 깨졌다
  3. `cargo build --release` — 위 자산이 실행 파일에 담기므로 **반드시 재생성 뒤에** 돈다
  4. `cargo run --example gen_installer` → `target/installer/MOA-Setup-<버전>.exe`
  5. `certutil -hashfile target\installer\MOA-Setup-<버전>.exe SHA256` — 나온 값을 본문에 적는다
  6. 태그를 달고 GitHub 릴리즈를 만들어 그 설치 파일을 첨부한다
- **릴리즈 노트 작성 규약** (2026-08-22 사용자 요청 — *"일반 사용자들이 보기 때문에 내용이 너무 길거나 내용이 너무 어려우면 안됨"*): 노트는 **사용자가 읽는 글**이다.
  - **항목당 한두 줄**로, 사용자가 화면에서 겪는 변화를 적는다
  - **내부 사정은 빼거나 사용자 말로 옮긴다** — 모듈·함수 이름, 리팩토링, 시험 추가, 의존성 조정은 적지 않는다
  - 절은 `새로워진 것`·`고친 것` 둘뿐이고 **빈 절은 통째로 뺀다**
  - 커밋 목록·비교 링크를 붙이지 않는다 (GitHub이 자동으로 붙인다)
  - 사용자가 **해야 할 일**이 있으면(설정이 초기화된다 등) 맨 위에 한 줄로 알린다
  - **체크섬은 맨 아래 접어 둔다** — 앱이 읽어야 하지만 사람에게는 불필요하고 어려운 값이라, 접기 안에 두면 둘 다 만족한다(추출기는 본문 어디에 있든 64자 hex를 찾는다)

  ```markdown
  ### 새로워진 것
  - 자동 업데이트 — 새 버전이 나오면 제목 줄에 알리고, 눌러 두면 알아서 받아 설치합니다.
  - 릴리즈 노트 — 설정 메뉴에서 이 페이지를 바로 열 수 있습니다.

  ### 고친 것
  - 폴더를 빠르게 오갈 때 목록이 잠깐 비던 문제를 고쳤습니다.

  <details><summary>파일 무결성 확인용 (SHA256)</summary>

  MOA-Setup-0.2.0.exe
  `<64자 hex>`
  </details>
  ```

## 데이터 접근
- **DB/스토어**: 없음 (**실행 파일과 같은 폴더의** `settings.json` 하나에 **세션 + 앱 설정**을 함께 담는다 — 설치본이면 `%LOCALAPPDATA%\Programs\MOA\settings.json`, 개발 실행이면 `target/debug\settings.json`이다. 2026-08-21 결정으로 `%APPDATA%\MOA`에서 옮겼고 **옛 파일은 읽지 않는다**(마이그레이션 없음) — 스키마 v3, v2는 승격해 읽는다. 앱 설정(`settings` 객체 — 글꼴·자동 실행·트레이·파일 보기·언어)이 깨져 있어도 세션은 살린다: 그 자리만 기본값으로 되돌린다)
- **레지스트리**: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 `MOA` 값이 **자동 실행 설정의 정본**이다 (설정 파일 값은 사본 — 다른 도구가 지웠을 수 있어 화면에 보일 때마다 다시 읽는다)
- **비밀번호**: 그 `settings.json`에 **DPAPI로 봉인해서만** 담는다 (`remote::secret`). 평문을 파일·로그·문서에 남기지 않는다
  - **통로가 하나 더 있다 — 사이트 목록 내보내기 파일**(`remote::envelope`, FR-59): 그 파일은 다른 PC로 옮기려고 만드는 것이라 DPAPI(사용자·PC에 묶인다)로는 뜻이 없어, **앱 내장 키**에서 키를 파생해 봉한다(CNG PBKDF2-HMAC-SHA256 + AES-256-GCM). **이것은 DPAPI급 보호가 아니다** — 키가 실행 파일에 실려 있어 MOA를 가진 사람은 누구나 풀 수 있고, 막는 것은 파일을 텍스트로 열었을 때 비밀번호가 그대로 보이는 것뿐이다. 그 파일 자체가 자격증명이라고 보아야 하며, 사용자가 불편을 이유로 암호 입력을 걷어내면서 이 대가를 택했다(2026-08-20). **읽을 때는 두 갈래를 모두 받는다** — 직전 버전이 사용자 암호로 만든 파일은 `kdf` 값으로 갈라 종전대로 암호를 물어 연다. **그 문서에 `password_sealed`(DPAPI 바이트)를 담지 않는다** — 그것은 이 PC에 묶여 있어 옮겨 봐야 풀리지 않는데, 담아 두면 같은 PC에서만 조용히 되살아난다

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
├── installer/
│   └── moa.nsi              # NSIS 설치 스크립트 (사용자 단위 설치 — 사람이 읽고 고치는 소스)
├── examples/
│   ├── gen_licenses.rs      # 라이선스 자산 생성기 (개발용 — `cargo build`가 빌드하지 않는다)
│   ├── gen_app_icon.rs      # 아이콘 자산 생성기 (`docs/AppIcon.png` → 256px)
│   └── gen_installer.rs     # 설치 파일 생성기 — makensis를 찾아 `installer/moa.nsi`를 넘긴다
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
- **빌드 산출물**: `target/` (gitignore) — 설치 파일도 그 아래 `target/installer/`에 떨어지므로 커밋되지 않는다
- **런타임 생성물**: 실행 파일 옆의 `settings.json`(설정·세션)과 `known_hosts.json`(SSH 서버 지문). 설치본에서는 설치 폴더 안에 생기고, **제거하면 묻지 않고 함께 지워진다**
- **커밋되는 생성물**: 셋 다 **손으로 고치지 않는다** — 생성기가 만든다.
  - `assets/licenses.json` — `examples/gen_licenses.rs`가 만든다. 레지스트리 캐시를 훑어 만들므로 생성은 개발 PC에서만 하고, 빌드·시험은 그 결과만 읽는다(네트워크·캐시 비의존)
  - `THIRD-PARTY-NOTICES.md` — **같은 생성기가 같은 자료로 함께 만든다**(레포 루트). 앱을 띄우지 않고 저장소에서 바로 보는 고지라 전문까지 담는다
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
- **팝업 메뉴**: **모서리를 각 메뉴가 적지 않는다** — 값의 정본은 `ui::theme::MENU_CORNER_RADIUS`(6px)이고 `apply_dark`가 egui 스타일(`visuals.menu_corner_radius`)에 세우므로, `Frame::menu`를 쓰는 쪽은 아무것도 적지 않으면 같은 모양이 된다. 종전에는 메뉴마다 `.corner_radius(0)`을 덧붙이거나 붙이지 않아 같은 우클릭 메뉴인데 원격 목록은 각지고 설정 메뉴는 둥글었다(2026-08-19 사용자 보고). 모달 대화의 12px(`ui::dialog::CORNER_RADIUS`)과는 별개 부품이다 — 그쪽은 버튼 줄을 낀 팝업이라 더 둥글다. 규약은 `ui::theme`의 소스 훑기 테스트(`팝업_메뉴는_모서리를_따로_적지_않는다`)가 지키며, 그 테스트는 **`src/ui` 하위 폴더까지 재귀로** 훑는다 — **모달 규약(`ui::dialog`)·아이콘 규약(`ui::widgets`) 시험도 2026-08-20에 재귀로 맞췄다**(종전에는 둘이 비재귀라 `src/ui/app/`·`src/ui/panel/`을 놓쳤다)
- **팝업 메뉴 한 줄**: **행 높이·좌우 여백·hover 모서리·hover 색을 각 메뉴가 적지 않는다** — 값의 정본은 `ui::theme`의 `MENU_ITEM_HEIGHT`(28px)·`MENU_ITEM_PAD_X`(12px)·`MENU_ITEM_CORNER_RADIUS`(4px)·`MENU_HOT`(#383838)이고, 두 경로 중 하나를 거치면 같은 모습이 된다: **egui 버튼으로 그리는 메뉴**는 팝업 클로저 첫 줄에서 `theme::menu_style(ui)`를 부르고(하위 메뉴는 부모 스타일을 잇지 않는 별도 `Area`라 **거기서도 따로 부른다**), **직접 그리는 메뉴**는 `widgets::menu_row(ui, 라벨, 활성)`를 쓴다. 전역(`apply_dark`)에 세우면 안 되는 이유가 둘이다 — 앱 전체 버튼이 함께 커지고, egui가 팝업마다 자기 메뉴 스타일을 새로 입히므로(`Popup::menu` → `containers::menu::menu_style`, 좌우 여백을 2px로 덮는다) **전역 값은 메뉴 안에서 효과가 없다**. 종전에는 같은 값이 파일마다 재정의돼 우클릭 메뉴 hover가 각지고(0px) 설정·사이드바 메뉴는 문구가 하이라이트 끝에 붙었으며(2px) 행 높이도 18·26·28로 갈려 설정 메뉴만 작아 보였다(2026-08-20 사용자 보고). 항목의 평상시 배경은 **투명을 지킨다** — 칠하면 메뉴가 버튼 목록처럼 보인다. 프레임 모서리(위 6px)와는 다른 부품이다. 규약은 `ui::theme`의 소스 훑기 테스트(`팝업_메뉴는_항목_스타일을_거친다`)가 지키며, **파일마다 「팝업을 여는 구문 수 ≤ 공통 경로 호출 수」를 견준다** — 한 파일에 팝업이 여럿인 곳(`sidebar.rs` 셋·`tabs.rs` 둘)에서 하나만 고쳐도 존재 여부만 보는 검사는 통과하기 때문이다
- **예제 타깃(`examples/`)**: 개발용 CLI라 **stdout/stderr 출력을 허용**하고 `fn main() -> Result<_, String>`으로 오류를 종료 코드에 싣는다 (아래 DO NOT의 `println!` 금지는 콘솔 창이 없는 **GUI 실행 파일**을 겨냥한 것이다 — 예제에는 오류를 알릴 다른 수단이 없다). `unwrap`·`expect` 금지는 예제에도 그대로 적용된다
- **파일**: UTF-8, 주석은 한글. **분할은 줄 수가 아니라 책임으로 판정한다** — ① 변경 이유가 둘 이상인가 ② 부분 수정에 전체 읽기가 필요한가 ③ 찾는 데 헤매는가 ④ (반대 가드) 분리하면 관련 로직이 흩어지는가. **①~③ 중 하나라도 「예」이고 ④가 「아니오」면 나누고, 그 밖에는 줄 수와 무관하게 둔다.** 고정 라인 임계를 두지 않는 이유는 그것이 양쪽으로 다 틀리기 때문이다 — 임계 미만이면 아무 신호도 없어 *나누는 편이 나은* 파일을 놓치고, 넘겼다는 이유만으로 단일 책임 파일에 분리 압력을 준다(종전 「1500라인 내외」를 2026-08-20에 이 판정으로 교체했다). 다만 **수천 줄인데 네 질문이 전부 「아니오」면 그 판정 자체를 의심한다**

## DO NOT
- `target/` 커밋 (gitignore 필수)
- `unsafe` 무분별 사용 — 사유 주석 의무, 래퍼 밖 노출 금지
- `println!` production 로깅 금지 (GUI 앱 — 필요 시 `OutputDebugStringW` 래퍼)
- 아이콘 자리에 유니코드 기호 직접 사용 (phosphor 대신 `"✕"` 같은 문자열 — 두부가 된다)
- `panic!` 직접 호출 (예외: main에서 초기화 실패)
- UI 스레드에서 파일시스템 블로킹 호출 — 겨냥하는 것은 **매 프레임 도는 렌더·탐색 경로**다(디렉터리 열거·감시). 사용자가 직접 누른 드문 조작의 작은 파일 I/O는 예외이며 지금 셋뿐이다: 세션 저장(`persist_session`), 사이트 목록 내보내기·가져오기(`ui::app::pump_site_file_dialog` — 키 파생 0.126초 실측을 포함해 UI 스레드에서 돈다), **앱→탐색기 드래그 내보내기**(`fs::drag_source::start_copy_drag`의 `SHParseDisplayName`**과 첫 항목의 미리보기 그림 조회**(`fs::drag_image` — `SIIGBF_INCACHEONLY` → `ICONONLY` 순으로 물어 **디스크에서 썸네일을 새로 만들지 않는다**) — 끄는 항목 수만큼 셸 네임스페이스를 조회한다. 여기서 워커로 못 미루는 이유는 `DoDragDrop` 자체가 **UI 스레드에서 마우스를 쥐어야** 하기 때문이다). **같은 회차의 OS 드롭 받기는 이 예외가 아니다** — 거기서는 폴더 여부 판정을 워커로 보냈다(`ui::app::spawn_os_drop_scan`)
- 코드·문서·notes·plan 등 어떤 파일에도 실제 IP·계정·비밀번호·토큰 기록

## Plan Location

```
Plan Location: docs/plans/<YYYY-MM-DD>-<slug>.md   (누적 — 덮어쓰지 않는다)
PRD Location:  docs/prd.md
```

## 추가 정보
- MSRV: stable 최신 (rust-toolchain.toml 미사용, v1 기준)
- CI/CD: 없음 (로컬 빌드)
- 배포: 단일 exe (`cargo build --release`) 또는 그 exe를 담은 NSIS 설치 파일 (`cargo run --example gen_installer` — 사용자 단위 설치, 코드 서명은 없다). **설치본은 GitHub 릴리즈에서 스스로 새 판을 찾아 받아 설치한다**(FR-62 — 위 「릴리즈 발행」 절차를 지켜야 그 기능이 동작한다). 개발 실행(설치본이 아닌 exe)은 확인조차 하지 않는다
