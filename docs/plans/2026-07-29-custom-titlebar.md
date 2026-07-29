# Plan: 커스텀 타이틀바 (사이드바 토글 · 설정 버튼)

**PRD**: docs/prd.md

## 요구 이해

- **원문 요청**: "이미지처럼 타이틀바 상단의 왼쪽에는 워크스페이스 패널 on/off 토글 버튼, 오른쪽 최소화 버튼 옆에는 설정 버튼 추가" / "설정 버튼을 누르면 다음 과 같이 메뉴 항목 표시 기능은 추후 구현 — 설정,업데이트,릴리즈 노트,오픈소스 라이선스,정보" / "egui-phosphor 설치해서 아이콘 사용"
- **이해한 요구**: OS가 그리는 기본 창 제목 표시줄을 끄고 앱이 직접 타이틀바를 그린다. 그 줄의 **왼쪽 끝에 워크스페이스 사이드바 on/off 토글**, **가운데에 활성 워크스페이스 이름**, **오른쪽에 설정 버튼 + 최소화·최대화·닫기**를 둔다. 설정 버튼을 누르면 `설정 / 업데이트 / 릴리즈 노트 / 오픈소스 라이선스 / 정보` 5개 항목이 뜨되, **각 항목의 기능은 이번에 만들지 않고 비활성(회색)으로만 표시**한다. 아이콘은 직접 그리거나 시스템 폰트를 쓰지 않고 `egui-phosphor` crate의 아이콘 폰트를 쓴다.
- **포함하지 않는 것으로 이해**: 설정 화면·업데이트 확인·릴리즈 노트·라이선스·정보 대화상자의 **실제 동작**은 이번 범위가 아니다(항목 표시까지만).

## Goal

기본 창 장식을 끈 자리에 앱이 직접 그리는 다크 타이틀바를 두고, 그 줄에서 사이드바를 켜고 끄며 설정 메뉴를 열 수 있게 한다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-22 (커스텀 타이틀바 — T1에서 신설) | Should | T3, T4, T5 | ✅ 커버 |
| FR-21 (탐색기 고정 다크 — 타이틀바 포함) | Should | T3 | ✅ 커버 (구현 수단이 DWM 다크 → 자체 렌더로 바뀜) |
| FR-19 (사이드바 접기/펼치기) | Should | T5 | ✅ 커버 (진입점 1개 추가 — 기존 동작 유지) |
| FR-11 (창 위치·크기 복원) | Should | T3 | ✅ 커버 (회귀 방지 — 장식 제거 후에도 복원 유지) |
| FR-1~FR-12, FR-15~FR-18, FR-20 | Must/Should | (없음) | 이번 범위 외 (기구현) |
| FR-13, FR-14 | Could | (없음) | 이번 범위 외 (미구현 Could — Deferred 대장 대기) |
| NFR-1~NFR-8 | — | (없음) | 이번 범위 외 (NFR-2는 T2 Risk에서 영향 평가 — 폰트 489KB) |

## Out of Scope

- 설정·업데이트·릴리즈 노트·오픈소스 라이선스·정보 각 항목의 **실제 기능**(사용자 지시: "기능은 추후 구현")
- 테마 전환 UI (PRD Out of Scope 유지 — "설정" 항목이 생겨도 이번엔 비활성)
- 메뉴 바(보기·이동·탭·워크스페이스) 재구성 — 지금 그대로 타이틀바 아래에 둔다
- 창 그림자·둥근 모서리 복원 (D10 — winit 확장에 eframe이 접근 경로를 주지 않는다)
- Windows 11 스냅 레이아웃 팝업(최대화 버튼 호버 메뉴) — 커스텀 캡션 버튼은 `WM_NCHITTEST`의 `HTMAXBUTTON`을 낼 수 없다

## Deferred / Follow-up

- 설정 팝업 5개 항목의 실제 기능 — 항목별 별도 plan
- 창 그림자·둥근 모서리 — `DwmSetWindowAttribute`의 `DWMWA_WINDOW_CORNER_PREFERENCE`로 모서리만 되살릴 여지가 있으나, 그림자는 winit의 `MARKER_UNDECORATED_SHADOW` 경로가 필요해 eframe 위에서는 불가. 필요해지면 `ShellHost` 서브클래스에서 `WM_NCCALCSIZE`를 직접 다루는 방식을 검토
- 기존 Deferred 대장(`docs/plans/deferred.md`) 항목은 이번 작업과 겹치지 않아 그대로 둔다

## Investigation Log

- `Cargo.toml` 확인: `eframe 0.35`(default-features off, glow만), `windows 0.62.2`, `raw-window-handle 0.6`. 아이콘 폰트 의존 없음
- `src/main.rs:22` 확인: `ViewportBuilder::default().with_title("파일 탐색기")` — **장식 관련 설정이 없어 현재는 OS 기본 타이틀바**다. 즉 타이틀바 안쪽은 Windows가 그리는 영역이라 egui가 버튼을 넣을 수 없다
- `egui-0.35.0/src/viewport.rs:1103·1124·1135·1138·1151` 확인: `ViewportCommand::{StartDrag, BeginResize(ResizeDirection), Minimized(bool), Maximized(bool), Decorations(bool)}` 모두 존재. `ResizeDirection`은 8방향(`viewport.rs:1055`)
- `egui-winit-0.35.0/src/lib.rs:1726·1760` 확인: `StartDrag` → `window.drag_window()`, `BeginResize` → `window.drag_resize_window(dir)`로 그대로 위임된다
- `winit-0.30.13/src/platform_impl/windows/window.rs:523·532` 확인: 두 호출 모두 `handle_os_dragging(HTCAPTION | HTLEFT…)` — **OS에 넘기는 방식이라 Aero Snap(가장자리 드래그 스냅)은 그대로 동작**한다
- `winit-0.30.13/src/platform_impl/windows/event_loop.rs:1162~1180` 확인: 무장식 창의 `WM_NCCALCSIZE`를 winit이 처리하며, **최대화 시 `monitorInfo.rcWork`로 제한**한다 → 커스텀 타이틀바의 대표적 함정(최대화하면 작업 표시줄을 덮고 화면 밖으로 넘침)은 발생하지 않는다
- `winit/src/platform_impl/windows/window_state.rs:433~437` 확인: 최상위 무장식 창은 **창 스타일을 지우지 않고**(주석 "Frameless style implemented by manually overriding the non-client area in `WM_NCCALCSIZE`") 프레임만 없앤다 — 즉 `WS_SIZEBOX`가 남아 Win+방향키 스냅·최대화는 살아 있고, 마우스 가장자리 리사이즈만 `BeginResize`로 직접 처리하면 된다. (`window_state.rs:286`의 스타일 제거는 `WindowFlags::CHILD` 블록 안이라 자식 창 전용 — 최상위 창과 무관하다)
- `eframe-0.35.0/src/epi.rs:42` 확인: `WindowBuilderHook = FnOnce(egui::ViewportBuilder) -> egui::ViewportBuilder` — **egui 타입만 다루므로 winit의 `with_undecorated_shadow`에 접근할 수 없다** → 창 그림자는 이번에 포기(D10)
- `cargo search` / `cargo info egui-phosphor` + 레지스트리 `egui-phosphor-0.13.0/Cargo.toml:58` 확인: 최신 `0.13.0`이 **`egui 0.35`를 요구** — 현재 버전과 정확히 일치. `default = ["regular"]`이며 `res/Phosphor.ttf`는 **489KB, `include_bytes!`로 exe에 정적 포함**(파일 부재 폴백 불필요)
- `egui-phosphor-0.13.0/src/variants/regular.rs` 확인: 필요한 상수 전부 존재 — `SIDEBAR_SIMPLE`(EC24) · `GEAR`(E270) · `MINUS`(E32A) · `SQUARE`(E45E) · `CORNERS_IN`(E1CE) · `X`(E4F6)
- `egui-phosphor-0.13.0/src/lib.rs:4` 확인: `add_to_fonts(&mut FontDefinitions, Variant)`가 Proportional 패밀리 **index 1**에 삽입한다 → 현재 한글 폰트가 index 0에 들어가는 방식(`app.rs:112`)과 충돌하지 않는다
- `src/ui/app.rs:98~120` 확인: `install_korean_font`는 폰트 파일을 못 읽으면 **`set_fonts` 자체를 호출하지 않고 early return** — phosphor를 여기 얹으려면 구조를 바꿔야 한다(한글 실패 시 아이콘까지 사라지면 안 됨)
- `grep install_korean_font` 전수: 정의(`app.rs:98`) + 호출 1곳(`app.rs:290`) + 표시 플래그(`app.rs:255·297·655`). `tests/`·`src/bin` 사용처 없음 → 시그니처 변경 영향은 app.rs 한 파일에 국한
- `src/ui/app.rs:439~476` + `src/main.rs:26~27` 확인: 저장은 `viewport().outer_rect`(장식 포함), 복원은 `with_inner_size(w, h)`다 — **장식이 있는 지금은 복원 때마다 창이 장식 두께만큼 커지는 누적 증가가 있다.** 무장식이 되면 outer == inner라 이 누적이 사라지고 크기가 안정된다(D12)
- `src/ui/tabs.rs:88~110` 확인: 분할 버튼이 `MenuButton::from_button`으로 팝업을 낸다 — 설정 팝업이 재사용할 수 있는 기존 패턴
- `src/ui/sidebar.rs:15~20·139~166` 확인: 사이드바 상단에 이미 접기 토글(`◧` 글리프, 28px 스트립)이 있고 `SidebarAction::ToggleCollapse`를 낸다. 사이드바가 **접히면 그 버튼도 함께 사라져** 지금은 메뉴·Ctrl+B로만 펼 수 있다 → 타이틀바 버튼이 항상 보이는 진입점이 된다
- `src/ui/app.rs:569` 확인: `Command::ToggleSidebar => self.sidebar_collapsed = !self.sidebar_collapsed` — 타이틀바 버튼이 재사용할 기존 명령이 이미 있다
- `src/ui/shell_host.rs:31~63` 확인: HWND 서브클래스는 `forward_menu_msg` 외 전부 `DefSubclassProc`로 넘긴다 → winit의 `WM_NCCALCSIZE` 처리와 충돌하지 않는다. `to_screen`은 `ClientToScreen` 기반이라 클라이언트 영역이 창 전체로 커져도 그대로 성립한다
- `docs/prd.md:36` 확인: FR-21이 "…탭·**타이틀바**·메뉴바를 고정 다크"로 이미 타이틀바를 포함한다 → 이번 변경은 FR-21에 닿는다(PRD 갱신 대상, T1)
- Deferred 대장(`docs/plans/deferred.md`) 확인: 이번 작업과 겹치는 대기 항목 없음. 다만 "구 Win32 UI 코드 제거"가 `app/theme.rs`를 언급하는데, 그 파일의 `apply_dark_titlebar`는 이번 변경으로 **호출 의미가 사라진다**(D11에서 처리)
- 위키 참조: 없음(vault 미설정) — 코드 1차 출처로 진행
- 참조 이미지: `C:\Users\jongc\Desktop\1.png`, 1801×58 px. 캡처 배율이 불명이라 **정확한 px 수치는 추출 불가** — 아래 시각 요소 분해의 값은 이미지의 상대 비율 + 이 앱의 기존 시각 토큰(`sidebar.rs`의 36px 헤더·14px 폰트)에서 정했다

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 무장식 창의 최대화가 작업 표시줄을 덮음 | 최대화 시 화면 하단이 가려짐 | winit이 `WM_NCCALCSIZE`에서 `rcWork`로 제한함을 소스로 확인(Investigation Log) — 코드 대응 불필요, T3에서 수동 확인 |
| 창 그림자·둥근 모서리 상실 | 창 경계가 배경과 붙어 보임 | eframe 경로에서 복원 불가를 확인 → Out of Scope로 명시하고 사용자에게 보고 |
| 가장자리 리사이즈 핸들과 내부 위젯의 입력 충돌 | 파일 목록 가장자리에서 드래그가 리사이즈로 가로채짐 | 핸들 폭 4px + 창 최외곽 프레임에서만 판정, 최대화 중에는 비활성 (T4) |
| 드래그 이동과 더블클릭 최대화가 서로를 삼킴 | 더블클릭이 인식되지 않거나 창이 튐 | `double_clicked()`를 먼저 판정하고 그 프레임에는 `StartDrag`를 보내지 않는다 (T3) |
| Phosphor 폰트 489KB 추가 | 메모리(NFR-2 150MB)·exe 크기 증가 | 폰트 데이터 0.5MB + 사용 글리프 6종의 아틀라스뿐 — NFR-2 여유 대비 무시 가능. `default-features`(regular 1종)만 사용 |
| 기존 세션의 창 크기 의미가 바뀜 | 첫 실행에서 창 안쪽(클라이언트)이 장식 두께만큼 넓어짐 | 창 바깥 크기는 그대로이고, 오히려 지금까지 재시작마다 창이 커지던 누적 증가가 사라진다 — 마이그레이션하지 않는다(D12) |
| `enable_dark_mode`를 "이제 불필요"로 오해해 제거 | Win32 셸 컨텍스트 메뉴(FR-8)가 밝은 색으로 회귀 | 이 함수는 셸 메뉴 테마도 담당하므로 **유지**하고 `main.rs`의 주석만 갱신한다 (D13) |
| 타이틀바가 입력을 가로채 기존 단축키가 죽음 | Ctrl+B 등 동작 불가 | 타이틀바는 `egui::Panel::top`으로 분리하고 단축키 폴링 경로(`menu::poll_shortcuts`)는 건드리지 않는다 (T3) |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `ViewportBuilder`(main) | `src/main.rs` | `.with_decorations(false)` 추가 — 창 생성 파라미터 변경 |
| `install_korean_font` | `src/ui/app.rs`(정의 98, 호출 290) | **함수명·책임 변경**(`install_fonts`) — 호출부 1곳뿐(grep 전수 확인, `tests/`·`src/bin` 없음) |
| `ExplorerApp::ui` | `src/ui/app.rs:650` | 타이틀바 패널 그리기 추가 — `CentralPanel` 앞에 `egui::Panel::top` |
| `app::theme::enable_dark_mode` | `src/main.rs:18`(호출), `src/app/theme.rs:45` | **유지**(D13). 호출은 그대로 두고 `main.rs:17`의 주석 문구만 갱신 — 이 함수는 타이틀바뿐 아니라 **Win32 셸 컨텍스트 메뉴(FR-8)의 다크 테마**도 담당한다 |
| `ExplorerApp` 필드 | `src/ui/app.rs:250~280` | 필드 추가 없음 — 최대화 여부는 `ctx.input().viewport()`에서 매 프레임 읽는다(`track_window`와 같은 출처) |
| `Command::ToggleSidebar` | `src/ui/menu.rs:50`, `src/ui/app.rs:569` | **변경 없이 재사용** — 타이틀바 버튼이 같은 명령을 낸다 |
| `theme` 팔레트 | `src/ui/theme.rs` | 상수 추가(`CLOSE_HOT`) — 기존 상수 변경 없음. **T3 구현에서 정정**: 계획 단계에는 `TITLEBAR_BG`도 적었으나, 시각 요소 분해가 타이틀바 배경을 기존 `WINDOW_BG`(#1B1B1B)로 지정하므로 값이 같은 별칭 상수를 만들지 않았다 |
| `app::theme::apply_dark_titlebar` | `src/app/theme.rs:85` | **건드리지 않는다**. **F-3에서 정정**: 계획 단계에 "호출 0곳"으로 적었으나 실제로는 `src/app/window.rs:183`에 호출이 있다 — 다만 그 파일은 egui 이식 이전의 Win32 UI로 **진입점(`main.rs`)이 쓰지 않는 코드**라 실행 경로에는 없다(AGENTS.md 모듈 지도 참조). 지우면 그 파일이 깨지므로 더더욱 손대지 않으며, Win32 잔재 정리는 Deferred 대장 소관이다 |
| `ShellHost::to_screen` | `src/ui/shell_host.rs:53` | 변경 없음 — `ClientToScreen` 기반이라 클라이언트 영역 확대와 무관 |

### 4-B. 계약·직렬화 변경

- 세션 스키마(`settings.json`) **변경 없음**. `WindowState{x,y,w,h,maximized}` 그대로 사용한다
- 저장 값의 **의미**만 미세하게 바뀐다(장식 포함 outer → 무장식 outer). 마이그레이션 불필요(D12)
- 공개 API 변경 1건: `pub fn install_korean_font(&Context) -> bool` → `pub fn install_fonts(&Context) -> bool` (lib 재수출 대상이나 외부 호출 0곳)

### 4-C. 테스트 파일

- `tests/layout_flow.rs`, `tests/watcher.rs` — 이번 변경과 무관(레이아웃 트리·감시 로직). 회귀 확인 대상으로만 실행
- 타이틀바 그리기·창 명령은 HWND/이벤트 루프가 필요해 단위테스트 비대상(AGENTS.md "UI(HWND 필요) 로직은 테스트 비대상")
- **테스트 가능한 순수 로직은 분리해 테스트한다**: 리사이즈 방향 판정(`resize_direction`)은 좌표만 다루므로 `#[cfg(test)] mod tests`로 덮는다. 제목 말줄임은 자체 함수를 만들지 않고 egui가 폭 기준으로 처리하므로(D14) 테스트 대상이 아니다

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `ui::titlebar` 모듈 | 없음 (`grep title_bar\|titlebar` → `app/window.rs`·`app/theme.rs`의 Win32 잔재뿐, egui 경로에 없음) | **신규** — 창 장식 대체는 기존 어느 모듈의 책임도 아니다 |
| `TitlebarOutcome`(버튼 결과 묶음) | `TabStripOutcome`(`tabs.rs:21`) | **신규**(패턴은 재사용) — 같은 "그리고 결과만 값으로 반환" 규약을 따르되 담는 내용이 다르다 |
| 설정 팝업 | `MenuButton::from_button`(`tabs.rs:93`), `ui.menu_button`(`menu.rs:66`) | **재사용** — 분할 버튼과 같은 방식으로 낸다 |
| 아이콘 그리기 | `draw_split_icon`(`tabs.rs:106`, painter 직접 그리기) | **미재사용 — 신규 방식**: 사용자 지시로 `egui-phosphor` 폰트 글리프를 쓴다. 기존 painter 아이콘(분할 버튼)은 그대로 두고 이번 것만 폰트로 그린다 |
| `TITLEBAR_BG` 상수 | `theme::WINDOW_BG`(`theme.rs:9`) | **재사용 — 신규 만들지 않음**(T3에서 확정): 시각 분해가 지정한 값(#1B1B1B)이 `WINDOW_BG`와 같아, 별칭 상수는 중복일 뿐이다 |
| 사이드바 토글 명령 | `Command::ToggleSidebar`(`menu.rs:50`) | **재사용** — 새 명령을 만들지 않는다 |
| 좌/중앙/우 3분할 배치 | `egui::Sides`(`tabs.rs:41`) | **재사용** — 좌·우 고정 배치는 `Sides`, 중앙 제목은 타이틀바 rect 기준 중앙 정렬로 그린다 |
| 다크 팔레트 | `ui::theme`(`theme.rs`) | **재사용 + 상수 1개 추가**(`CLOSE_HOT`) — 색을 파일 안에 흩지 않는다. `TITLEBAR_BG`는 만들지 않았다(위 행 참조) |

### Verified by

- `grep "install_korean_font\|korean_font"` → 5 hits(정의 1·호출 1·플래그 3), 모두 `src/ui/app.rs`. 위 표에 포함
- `grep "Command::"` → 60 hits, `Command::ToggleSidebar`는 정의(`menu.rs:50`)·메뉴 항목(`menu.rs:84`)·단축키(`menu.rs:222`)·실행(`app.rs:569`) 4곳. 전부 변경 없이 재사용
- `grep "title_bar|titlebar|캡션"` → `src/app/window.rs`·`src/app/theme.rs`(Win32 이식 이전 코드, 실행 파일 미사용)만 매칭 — egui 경로에 기존 타이틀바 구현 없음
- `grep "apply_dark_titlebar"` → 정의(`src/app/theme.rs:85`) 1건, 호출 1건(`src/app/window.rs:183` — 진입점이 쓰지 않는 Win32 이식 이전 코드. **F-3에서 정정**: 계획 단계의 "호출 0건"은 잘못된 기록이었다)

## Decisions

### D1. 창 장식 제거 지점
- **Options**: A) `main.rs`의 `ViewportBuilder::with_decorations(false)` / B) 앱 시작 후 `ViewportCommand::Decorations(false)`
- **Chosen**: A
- **Rationale**: B는 창이 장식과 함께 한 번 뜬 뒤 벗겨져 깜빡인다. 창 크기·위치 설정과 같은 자리에서 결정하는 편이 읽기도 쉽다
- **Source**: `src/main.rs:22~31`, `egui-0.35.0/src/viewport.rs:367`

### D2. 타이틀바 배치
- **Options**: A) `egui::Panel::top(Id)`으로 `CentralPanel` 위에 / B) `CentralPanel` 안 최상단에 직접 그림
- **Chosen**: A. 사이드바가 쓰는 `egui::Panel::left`(`app.rs:677`)와 같은 API의 top 방향이며, **`show_separator_line(false)`로 구분선을 끈다**(egui 기본값이 `true` — 참조 이미지에는 타이틀바 아래 선이 없다)
- **Rationale**: B는 지금 메뉴 바가 쓰는 방식인데, 분할 영역 계산이 `available_rect_before_wrap`(`app.rs:695`)에 의존해 타이틀바 높이를 손으로 빼야 한다. A는 egui가 남은 영역을 자동으로 좁혀 주므로 기존 계산이 그대로 성립한다
- **Source**: `src/ui/app.rs:654~695·677`, `egui-0.35.0/src/containers/panel.rs:238`(`Panel::top`)·`:270`(`show_separator_line` 기본 true)·`:303`. **`TopBottomPanel`은 egui 0.35에 존재하지 않는다**(grep 0 hits) — 구버전 API다

### D3. 창 끌어 이동
- **Options**: A) 타이틀바 빈 영역 드래그 → `ViewportCommand::StartDrag` / B) 마우스 이동량을 받아 `OuterPosition`을 매 프레임 갱신
- **Chosen**: A
- **Rationale**: B는 OS의 Aero Snap(가장자리로 끌어 반쪽 배치)을 잃고 좌표 누적 오차가 생긴다. A는 winit이 `WM_NCLBUTTONDOWN(HTCAPTION)`으로 OS에 위임하므로 스냅이 그대로 살아 있다
- **Source**: `winit/src/platform_impl/windows/window.rs:523`

### D4. 더블클릭 최대화
- **Chosen**: 타이틀바 빈 영역 더블클릭 시 `Maximized(!maximized)` 전송. **더블클릭이 인식된 프레임에는 `StartDrag`를 보내지 않는다**
- **Rationale**: 둘 다 보내면 OS 드래그 루프가 먼저 잡혀 더블클릭이 삼켜진다
- **Source**: `egui-0.35.0/src/viewport.rs:1138`

### D5. 가장자리 리사이즈
- **Options**: A) 창 4변·4모서리에 4px 히트 영역을 두고 `BeginResize` / B) 리사이즈 포기(고정 크기·최대화만)
- **Chosen**: A
- **Rationale**: B는 기존 동작(FR-11의 창 크기 저장)을 사실상 무의미하게 만든다. A는 `WS_SIZEBOX`가 남아 있어 OS 리사이즈 루프가 정상 동작한다
- **Source**: `winit/src/platform_impl/windows/window_state.rs:433~437`, `egui-winit-0.35.0/src/lib.rs:1760`

### D6. 아이콘 소스
- **Chosen**: `egui-phosphor 0.13.0`(default features = regular). 상수 매핑 — 사이드바 토글 `SIDEBAR_SIMPLE` / 설정 `GEAR` / 최소화 `MINUS` / 최대화 `SQUARE` / 최대화 상태의 복원 `CORNERS_IN` / 닫기 `X`
- **Rationale**: 사용자 지시. 폰트가 exe에 정적 포함되어 시스템 폰트 부재·글리프 누락(두부 □) 위험이 없다
- **Source**: 사용자 요청, `egui-phosphor-0.13.0/Cargo.toml:58`(egui 0.35), `src/variants/regular.rs`

### D7. 폰트 등록 구조
- **Options**: A) `install_korean_font`를 `install_fonts`로 바꿔 한글+phosphor를 한 번에 등록 / B) phosphor 등록 함수를 따로 두고 두 번 `set_fonts`
- **Chosen**: A
- **Rationale**: B는 `set_fonts`를 두 번 호출해 뒤엣것이 앞엣것을 덮어쓸 위험이 있다. 또 현재 함수는 한글 파일이 없으면 early return이라(`app.rs:99`) 그 경로에서 아이콘까지 사라진다 — **한글 실패와 무관하게 phosphor는 항상 등록**되어야 한다. 반환 bool의 의미는 "한글 폰트 적용 여부"로 유지한다(`app.rs:655`의 안내 문구가 그 뜻을 쓴다)
- **Source**: `src/ui/app.rs:98~120`, `egui-phosphor-0.13.0/src/lib.rs:4`

### D8. 타이틀바 제목 문자열
- **Options**: A) 활성 워크스페이스 이름 / B) "파일 탐색기" 고정 / C) 워크스페이스 이름 + 현재 폴더
- **Chosen**: A (참조 이미지가 "워크스페이스 1"을 보여준다). **작업 표시줄에 뜨는 OS 창 제목은 "파일 탐색기"를 그대로 둔다**
- **Rationale**: 사이드바 카드가 이미 이름+폴더 2줄을 보여주므로(FR-15) C는 중복이다. OS 창 제목까지 바꾸면 작업 표시줄에서 앱을 알아보기 어려워진다
- **Source**: 참조 이미지, `src/main.rs:22`, `src/ui/sidebar.rs`

### D9. 설정 팝업의 미구현 항목 표시
- **Chosen**: 5개 항목을 순서대로(`설정` / `업데이트` / `릴리즈 노트` / `오픈소스 라이선스` / `정보`) 두되 `add_enabled(false, …)`로 **전부 비활성(회색)**. `오픈소스 라이선스` 위에 구분선 하나
- **Rationale**: 사용자 선택. 활성처럼 보이면서 눌러도 반응이 없으면 고장으로 오인된다
- **Source**: 사용자 요청, `src/ui/menu.rs:157`(`add_enabled` 기존 사용례)

### D10. 창 그림자·둥근 모서리
- **Chosen**: 이번엔 복원하지 않는다(Out of Scope + Deferred 기록)
- **Rationale**: winit의 `with_undecorated_shadow`는 winit `WindowAttributes` 확장인데 eframe의 훅은 `egui::ViewportBuilder`만 받아 접근 경로가 없다
- **Source**: `eframe-0.35.0/src/epi.rs:42`, `winit/src/platform/windows.rs:509`

### D11. `app::theme::apply_dark_titlebar` 처리
- **Chosen**: **건드리지 않는다**(진입점이 쓰지 않는 코드라 동작 영향 없음 — 호출 수는 4-A 행의 F-3 정정 참조)
- **Rationale**: Win32 이식 잔재 정리는 Deferred 대장의 "구 Win32 UI 코드 제거"가 통째로 다루기로 보류된 항목이다. 여기서 일부만 지우면 그 판단이 쪼개진다
- **Source**: `src/app/window.rs:183`(호출 1건 — 단 진입점이 쓰지 않는 Win32 이식 이전 코드), `docs/plans/deferred.md:7`

### D12. 기존 세션 창 크기 호환
- **Chosen**: 마이그레이션하지 않는다. 저장된 `w/h`를 그대로 쓴다
- **Rationale**: 저장은 outer, 복원은 `with_inner_size`라 지금은 재시작마다 창이 장식 두께만큼 커지는 누적 증가가 있는데, 무장식이 되면 outer == inner라 그 어긋남 자체가 사라진다. 창 바깥 크기는 유지되고 안쪽만 넓어지므로 보정할 것이 없다. 보정 코드를 넣으면 "언제 한 번 보정했는지"를 세션에 기록해야 해 스키마가 늘어난다
- **Source**: `src/ui/app.rs:439~476`, `src/main.rs:26~27`, `src/app/settings.rs`의 `WindowState`

### D13. `app::theme::enable_dark_mode` 존치
- **Options**: A) 유지하고 `main.rs:17` 주석만 갱신 / B) 장식이 없어졌으니 호출 제거
- **Chosen**: A
- **Rationale**: 이 함수의 `SetPreferredAppMode(ForceDark)`·`FlushMenuThemes`는 타이틀바만이 아니라 **아직 살아 있는 Win32 셸 컨텍스트 메뉴(FR-8, `ui/shell_host.rs:48`)의 다크 테마**를 담당한다. 제거하면 우클릭 메뉴가 밝은 색으로 회귀한다(FR-21 위반). 다만 `main.rs:17`의 주석("창 제목 표시줄을 다크로 만드는 프로세스 전역 정책")은 사실과 어긋나게 되므로 "셸 팝업 메뉴를 다크로 만드는 정책" 취지로 고친다
- **Source**: `src/app/theme.rs:36~66`, `src/main.rs:17~18`, `src/ui/shell_host.rs:48`

### D14. 제목 말줄임 방식
- **Options**: A) egui `Label::truncate()`로 가용 폭 기준 자동 말줄임 / B) `tabs.rs:114 elide`처럼 문자 수 고정 컷
- **Chosen**: A
- **Rationale**: B는 창 폭·좌우 버튼 크기와 무관한 고정 상수라, 창을 좁히면 제목이 버튼을 침범하고 넓히면 불필요하게 잘린다. A는 남은 폭을 egui가 직접 재어 자르므로 상수를 고를 필요가 없고 커스텀 함수·테스트도 필요 없다(탭 제목은 스크롤 영역 안이라 폭 기준을 쓸 수 없어 B를 썼던 것 — 사정이 다르다)
- **배치 방법 (확정)**: 좌·우 버튼군은 `egui::Sides`로 배치하고, **제목은 그 위에 타이틀바 rect 중앙 정렬로 따로 그린다.** 최대 폭은 `타이틀바 폭 − 2 × 우측 버튼군 폭(174px)`으로 잡는다 — `Sides`에는 중앙 슬롯이 없고(`sides.rs`의 `show`는 left/right 2개뿐), 좌측(38px)과 우측(174px)이 비대칭이라 "좌우 그룹 사이 틈"을 그대로 쓰면 제목 중심이 한쪽으로 밀린다. 좌우 중 **더 넓은 쪽을 양쪽에서 뺀 폭**이라야 가로 중앙과 버튼 비침범을 동시에 만족한다
- **Source**: `egui-0.35.0/src/widgets/label.rs:71`(`truncate`), `egui-0.35.0/src/containers/sides.rs`(중앙 슬롯 없음), `src/ui/tabs.rs:114`

## 시각 요소 분해

**기준**: 참조 이미지 `C:\Users\jongc\Desktop\1.png` (1801×58 px, 캡처 배율 불명 — 절대 px는 추출 불가)

### 시각 속성

> `구현 근거`·`판정`은 V-9(구현 후 대조)에서 채운다. 이 앱은 데스크톱 UI라 자율 루프에서 창을 띄워
> 캡처할 수단이 없어, 코드 값이 디자인 값과 같음은 소스로 확인하고 화면에 그려진 최종 모습은
> 미확인으로 두었다. **2026-07-29 F-8에서 사용자가 실행 화면을 보고 "잘됨"으로 확인**했다
> (항목별 정밀 대조가 아니라 화면 전체를 보고 이상 없음을 확인한 것).

| 요소 | 속성 | 디자인 값 | 확인 방법 | 구현 근거 | 판정 |
|------|------|----------|-----------|----------|------|
| 타이틀바 | 높이 | 36px | 이미지 비율(제목 글자 높이 대비 약 2.6배) + 기존 토큰 `sidebar.rs:21` HEADER_HEIGHT 36 통일 | `titlebar.rs:16` TITLEBAR_HEIGHT 36.0 / `app.rs:499` exact_size | ✅ 사용자 확인 (코드 값 일치) |
| 타이틀바 | 배경색 | `#1B1B1B` (`theme::WINDOW_BG`) | 이미지 육안(거의 검정) + 기존 팔레트 재사용 `theme.rs:9` | `app.rs:502` `Frame::NONE.fill(theme::WINDOW_BG)` | ✅ 사용자 확인 (코드 값 일치) |
| 타이틀바 | 아래 경계 | 구분선 없음 (메뉴 바가 바로 이어짐) | 이미지 육안 | `app.rs:501` `show_separator_line(false)` | ✅ 사용자 확인 (코드 값 일치) |
| 제목 | 문구 | 활성 워크스페이스 이름 (예: "워크스페이스 1") | 이미지 육안 (D8) | `app.rs:496` `workspaces.active().name` | ✅ 사용자 확인 (코드 값 일치) |
| 제목 | 정렬 | 타이틀바 가로 중앙 (좌우 버튼군 배치와 무관하게 바 중심) | 이미지 육안 | `titlebar.rs:101~113` `Rect::from_center_size(bar.center(), …)` + `ui.put` | ✅ 사용자 확인 (코드 값 일치) |
| 제목 | 최대 폭 | 타이틀바 폭 − 2 × 174px(우측 버튼군 폭) | D14 — 가로 중앙과 버튼 비침범을 동시에 만족하는 유일한 폭 | `titlebar.rs:102` `bar.width() - 2.0 * RIGHT_GROUP_WIDTH` | ✅ 사용자 확인 (코드 값 일치) |
| 제목 | 글자 크기·색 | 14px, `#E8E8E8`(`theme::TEXT`) | 기존 토큰 `sidebar.rs:23` HEADER_FONT_PX 14 통일 | `titlebar.rs:26` TITLE_FONT_PX 14.0 / `titlebar.rs:107~109` | ✅ 사용자 확인 (코드 값 일치) |
| 제목 | 넘침 처리 | 좌우 버튼을 뺀 가용 폭에서 말줄임(…) | 이미지에 없음 — egui `Label::truncate()`로 폭 기준 처리 (D14) | `titlebar.rs:111` `.truncate()` | ✅ 사용자 확인 (코드 값 일치) |
| 좌측 토글 버튼 | 위치·크기 | 타이틀바 왼쪽 끝, 36×36px, 좌측 여백 2px | 이미지 육안(왼쪽 끝에 거의 붙음) | `titlebar.rs:128` `add_space(LEFT_MARGIN=2.0)` + `titlebar.rs:134~140` `icon_button(…, BUTTON_SIZE=36.0, …)` | ✅ 사용자 확인 (코드 값 일치) |
| 좌측 토글 버튼 | 아이콘 | Phosphor `SIDEBAR_SIMPLE`, 16px | D6 | `titlebar.rs:135` `SIDEBAR_SIMPLE` + `titlebar.rs:25` ICON_FONT_PX 16.0 | ✅ 사용자 확인 (코드 값 일치) |
| 우측 버튼군 | 순서 | (왼→오) 설정 → 최소화 → 최대화 → 닫기 | 이미지 육안 + 요청문("최소화 버튼 옆에 설정") | `titlebar.rs:120~149` (`Sides` 우측은 오른쪽부터 채우므로 닫기→최대화→최소화 순으로 추가) | ✅ 사용자 확인 (코드 값 일치) |
| 설정 버튼 | 크기·아이콘 | 36×36px, Phosphor `GEAR` 16px | D6 | `titlebar.rs:184~189` `icon_button(GEAR, BUTTON_SIZE, …)` | ✅ 사용자 확인 (코드 값 일치) |
| 설정 팝업 | 항목·순서 | 설정 / 업데이트 / 릴리즈 노트 / (구분선) / 오픈소스 라이선스 / 정보, 전부 비활성 | 사용자 지시 (D9) | `titlebar.rs:191~198` (`pending_item` = `add_enabled(false, …)`) | ✅ 사용자 확인 (코드 값 일치) |
| 캡션 버튼 3종 | 크기 | 46×36px (Windows 캡션 버튼 관례 폭) | 이미지 육안(정사각보다 가로로 넓음) | `titlebar.rs:20` CAPTION_WIDTH 46.0 / `titlebar.rs:152` | ✅ 사용자 확인 (코드 값 일치) |
| 캡션 버튼 3종 | 아이콘 | `MINUS` / `SQUARE`(최대화 시 `CORNERS_IN`) / `X`, 16px | D6 | `titlebar.rs:120·131~137·143` + `titlebar.rs:25` ICON_FONT_PX 16.0 | ✅ 사용자 확인 (코드 값 일치) |
| 버튼 공통 | 기본 배경 | 투명(타이틀바와 동일) | 이미지 육안 | `titlebar.rs:166~168` (hover일 때만 `rect_filled`) | ✅ 사용자 확인 (코드 값 일치) |
| 버튼 공통 | hover 배경 | `#383838` (`theme::CONTROL_HOT`) | 기존 팔레트 재사용 `theme.rs:23` | `titlebar.rs:131·137` (`theme::CONTROL_HOT` 전달) | ✅ 사용자 확인 (코드 값 일치) |
| 닫기 버튼 | hover 배경 | `#C42B1C` (신규 `theme::CLOSE_HOT`) | Windows 11 표준 닫기 hover 색 | `theme.rs:29` CLOSE_HOT / `titlebar.rs:120` | ✅ 사용자 확인 (코드 값 일치) |
| 아이콘 색 | 기본 | `#E8E8E8` (`theme::TEXT`) | 이미지 육안 | `titlebar.rs:174` `theme::TEXT` | ✅ 사용자 확인 (코드 값 일치) |

## Tasks

- [x] T1. PRD에 FR-22(커스텀 타이틀바) 추가 · FR-21 표현 정정
  - **Type**: A
  - **Acceptance**: Given `docs/prd.md`, When T1 완료, Then ① FR-22 행이 "창 제목 표시줄을 앱이 직접 그린다 — 좌측 사이드바 토글, 중앙 활성 워크스페이스 이름, 우측 설정 메뉴(설정·업데이트·릴리즈 노트·오픈소스 라이선스·정보 — v1은 표시만)와 최소화·최대화·닫기" 취지로 존재하고 우선순위 Should ② FR-21의 "타이틀바"가 DWM 다크가 아니라 자체 렌더임이 드러나며 ③ `## 결정 이력`에 2026-07-29 항목 1줄이 추가돼 있고 ④ `## 성공 기준`의 Should 열거(`docs/prd.md:66`)에 FR-22가 포함돼 있다
  - **Files**:
    - 주: `docs/prd.md`
  - **Edge Cases**: 기존 FR 번호와 충돌하지 않게 FR-22를 새 ID로 부여(FR-21이 마지막) — 폐기·부활 아님
  - **Halt Forecast**:
    - (ii-a) PRD 수정은 승인 대상 → `## 사전 승인 항목`에 등록
  - **Depends on**: -

- [x] T2. `egui-phosphor` 도입과 폰트 등록 일원화
  - **Type**: C
  - **Design**: ① 배치 — `Cargo.toml` 의존성 + `src/ui/app.rs`의 폰트 등록 함수 ② 신규 심볼 — `install_fonts(&egui::Context) -> bool`(한글 폰트와 Phosphor 아이콘 폰트를 **한 번의 `set_fonts`로** 등록, 반환은 한글 적용 여부) ③ 의존 방향 — `ui::app` → `egui_phosphor`(단방향, 다른 모듈은 이 crate를 모른다) ④ 비추상화 — 아이콘 이름을 감싸는 자체 enum·래퍼를 만들지 않고 `egui_phosphor::regular::GEAR` 상수를 쓰는 자리에서 직접 참조한다
  - **Acceptance**: Given 앱 실행, When 어느 화면이든 Phosphor 글리프를 그릴 때, Then 두부(□) 없이 아이콘이 표시된다 / Given 맑은 고딕 파일이 없는 환경(경로 변조로 재현), When 앱 시작, Then 한글 안내 문구가 뜨면서도 **아이콘은 정상 표시**된다 / `cargo build` 경고 0
  - **Files**:
    - 주: `Cargo.toml`, `src/ui/app.rs`
    - 동반: `Cargo.lock`
  - **Edge Cases**:
    - 한글 폰트 파일 부재 → phosphor만 등록하고 `false` 반환(기존 안내 문구 경로 유지)
    - Proportional 패밀리 인덱스 충돌 — 한글은 0번, `add_to_fonts`가 1번에 삽입하므로 **한글 등록 뒤에 phosphor를 얹는다**
  - **Halt Forecast**:
    - (ii-a) 의존성 추가(`egui-phosphor 0.13.0`) → `## 사전 승인 항목`에 등록
  - **Depends on**: -

- [x] T3. 창 장식 제거와 타이틀바 골격 (제목 · 드래그 이동 · 더블클릭 최대화 · 캡션 버튼 3종)
  - **Type**: D
  - **Design**: ① 배치 — 신규 파일 `src/ui/titlebar.rs`(`ui::mod`에 등록), 창 설정은 `src/main.rs`. 타이틀바는 `egui::Panel::top(egui::Id::new("titlebar"))`에 `show_separator_line(false)`로 그린다(D2) ② 신규 심볼 — `show_titlebar(&mut egui::Ui, &str, TitlebarState) -> TitlebarOutcome`(타이틀바 한 줄을 그리고 이번 프레임의 요청만 값으로 반환) / `TitlebarState{maximized: bool, sidebar_collapsed: bool}`(그리기에 필요한 현재 상태) / `TitlebarOutcome{command: Option<Command>, window: Option<WindowRequest>}` / `WindowRequest{Minimize, ToggleMaximize, Close, Drag}`. 제목 말줄임 함수는 만들지 않는다(D14 — `Label::truncate()`) ③ 의존 방향 — `ui::titlebar` → `ui::theme`·`ui::menu::Command`·`egui_phosphor`. 상태 변경은 하지 않고 `ui::app`이 결과를 실행한다(`ui::menu`·`ui::tabs`와 같은 규약) ④ 비추상화 — 창 명령을 감싸는 트레이트·추상 레이어를 만들지 않는다. `ViewportCommand`는 `ui::app`에서 직접 보낸다
  - **Acceptance**: Given 앱 실행, When 창이 뜸, Then OS 기본 제목 표시줄이 없고 다크 타이틀바에 활성 워크스페이스 이름이 중앙에 보인다 / Given 타이틀바 빈 영역, When 드래그, Then 창이 따라 움직이고 화면 가장자리로 끌면 Windows 스냅이 동작한다 / Given 타이틀바 빈 영역, When 더블클릭, Then 최대화·복원이 토글되고 **드래그로 오인되지 않는다** / Given 최대화 상태, When 최대화 버튼 확인, Then 아이콘이 `CORNERS_IN`으로 바뀐다 / Given 최소화·닫기 버튼, When 클릭, Then 각각 창이 최소화되고 종료되며 **종료 시 세션이 저장된다**(`on_exit` 경로 유지) / Given 최대화 후 복원, When 재시작, Then 이전 크기·위치로 복원된다(FR-11 회귀 없음) / Given 파일 목록, When 우클릭, Then Win32 셸 컨텍스트 메뉴가 **여전히 다크로** 뜬다(D13 — `enable_dark_mode` 존치 확인) / `cargo test` 통과(기존 테스트 회귀 없음)
  - **Files**:
    - 주: `src/ui/titlebar.rs`(신규), `src/main.rs`, `src/ui/app.rs`
    - 동반: `src/ui/mod.rs`, `src/ui/theme.rs`(상수 1개 추가 — `CLOSE_HOT`. 계획 단계에는 `TITLEBAR_BG`도 적었으나 만들지 않았다, 4-A·4-D 정정 참조)
    - 테스트: 없음 — 이 task에는 순수 함수가 없다(제목 말줄임은 egui가 처리, D14). 단위테스트는 T4의 `resize_direction`이 담당
  - **Edge Cases**:
    - 워크스페이스 이름이 매우 길다 → `Label::truncate()`가 가용 폭에서 자른다(좌우 버튼 영역 침범 금지)
    - `main.rs`를 고치면서 `enable_dark_mode` 호출을 지우지 않는다 — 주석만 갱신한다 (D13)
    - 이름이 빈 문자열 → 제목 없이 빈 줄(패닉 없음)
    - 최대화 중 더블클릭 → 복원
    - 닫기 버튼으로 종료 → `eframe::App::on_exit`가 호출되는 경로(`ViewportCommand::Close`)를 쓴다. 강제 종료 경로는 쓰지 않는다
    - 창이 비활성(포커스 없음) 상태 → 타이틀바 색은 동일하게 유지(활성/비활성 구분 없음 — 기존 앱에도 그 구분이 없다)
  - **Halt Forecast**:
    - (i) 최대화 시 작업 표시줄을 덮는가 → winit이 `rcWork`로 제한함을 Investigation Log에서 확인 완료
    - (i) 세션 창 크기 호환 → D12에서 확정
    - (ii-a) `install_korean_font` → `install_fonts` 공개 API 변경(T2), 창 생성 파라미터 변경 → `## 사전 승인 항목`에 등록
  - **Depends on**: T2

- [x] T4. 가장자리·모서리 리사이즈 핸들
  - **Type**: C
  - **Design**: ① 배치 — `src/ui/titlebar.rs`(창 프레임 제어라는 같은 책임) ② 신규 심볼 — `show_resize_handles(&egui::Context, bool)`(창 최외곽에서 방향을 판정해 크기 조절을 요청) / `resize_direction(pointer: Pos2, window: Rect, margin: f32) -> Option<ResizeDirection>`(좌표 → 방향 판정, 순수 함수) ③ 의존 방향 — `ui::titlebar` → `egui`만. `ui::app`이 매 프레임 호출한다 ④ 비추상화 — 커서 모양·히트 영역을 위한 별도 위젯 타입을 만들지 않는다. **T4 구현에서 정정**: `Area` + `Sense::drag` 대신 **포인터 좌표를 직접 보는 방식**을 썼다 — `Area`는 8방향마다 하나씩 만들어야 하는데, 판정은 좌표 하나로 끝나므로 그편이 더 짧고 순수 함수로 테스트도 된다. 창 명령 전송은 이 모듈이 하지 않고 `WindowRequest`로 돌려준다(T3에서 세운 규약)
  - **Acceptance**: Given 일반(비최대화) 창, When 창 4변·4모서리 가장자리(4px)에 마우스를 올림, Then 방향에 맞는 리사이즈 커서가 뜨고 드래그하면 그 방향으로 크기가 바뀐다 / Given 최대화 상태, When 가장자리에 마우스를 올림, Then 리사이즈가 동작하지 않는다 / Given 파일 목록·사이드바 가장자리 안쪽, When 드래그, Then 리사이즈로 가로채지 않고 기존 동작(스크롤·스플리터)이 그대로 된다 / `cargo test` 통과(`resize_direction` 단위테스트: 8방향 + 중앙 None, **최대화 시 None은 `show_resize_handles` 테스트로** — 그 가드는 `Context`가 필요해 순수 함수 쪽에 둘 수 없다)
  - **Files**:
    - 주: `src/ui/titlebar.rs`
    - 동반: `src/ui/app.rs`
    - 테스트: `src/ui/titlebar.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 창이 최소 크기일 때 안쪽으로 드래그 → OS가 처리(별도 하한 설정 없음)
    - 모서리 판정이 변 판정보다 **먼저** 와야 한다(모서리는 두 변이 겹치는 자리)
    - 사이드바 스플리터가 창 왼쪽 가장자리와 겹치는 구간 → 리사이즈 핸들이 최상위 `Area`로 4px만 차지하므로 스플리터는 그 안쪽에서 정상 동작
    - 타이틀바 버튼과 핸들이 겹치는 구간 → **모서리는 크기 조절이 우선, 위쪽 변은 버튼이 우선**한다. **T4 구현에서 정정**: 계획 단계에는 "최외곽 4px는 전부 크기 조절 우선(버튼은 남은 영역으로 누를 수 있다)"으로 적었으나, 품질 리뷰가 실제로는 버튼 위쪽 4px를 누른 순간 크기 조절 루프가 열려 **그 클릭이 삼켜진다**는 것을 짚어 가정이 반증됐다. 그래서 위쪽 변(North)이 버튼 구간(좌 38px·우 174px)과 겹치면 버튼에 양보하고, 모서리(NorthWest·NorthEast)는 그대로 크기 조절이 가져간다 — 모서리까지 내주면 대각선으로 창을 잡을 자리가 없어지기 때문이다
  - **Halt Forecast**:
    - (i) 내부 위젯과의 입력 충돌 → 핸들 폭 4px·최외곽 한정으로 해소(Risks)
  - **Depends on**: T3

- [x] T5. 좌측 사이드바 토글 버튼과 우측 설정 팝업(5개 항목, 비활성)
  - **Type**: C
  - **Design**: ① 배치 — `src/ui/titlebar.rs`의 좌·우 영역(T3이 만든 `egui::Sides` 배치 안) ② 신규 심볼 — `show_settings_menu(&mut egui::Ui)`(톱니 버튼 + 비활성 5항목 팝업. 고를 것이 없으므로 반환값 없음) ③ 의존 방향 — 사이드바 토글은 **기존 `Command::ToggleSidebar`를 그대로 반환**해 `ui::app`의 기존 처리(`app.rs:569`)로 흘려보낸다. 새 명령·새 상태 필드를 만들지 않는다 ④ 비추상화 — 항목 5개를 데이터 배열+루프로 돌리지 않고 그대로 나열한다(각 항목이 곧 개별 기능으로 갈라질 자리라 공통화가 이르다)
  - **Acceptance**: Given 사이드바가 펼쳐진 상태, When 타이틀바 좌측 버튼 클릭, Then 사이드바가 접힌다 / Given 사이드바가 접힌 상태, When 같은 버튼 클릭, Then 다시 펼쳐진다(Ctrl+B·메뉴와 같은 결과) / Given 사이드바 내부 접기 버튼(`◧`), When 클릭, Then 기존과 동일하게 접힌다(회귀 없음) / Given 톱니 버튼, When 클릭, Then `설정 · 업데이트 · 릴리즈 노트 · 오픈소스 라이선스 · 정보` 5개가 이 순서로 뜨고 **전부 회색(클릭 불가)** 이다 / Given 팝업이 열린 상태, When 바깥 클릭 또는 Esc, Then 팝업이 닫힌다
  - **Files**:
    - 주: `src/ui/titlebar.rs`
    - 동반: `src/ui/app.rs`
  - **Edge Cases**:
    - 사이드바가 접힌 채로 워크스페이스 추가·이름 변경 명령이 오면 기존 로직이 자동으로 펼친다(`app.rs:573·579`) — 타이틀바 버튼 상태와 어긋나지 않게 매 프레임 `sidebar_collapsed`를 읽어 그린다
    - 팝업이 열린 채 창을 드래그 → 타이틀바 드래그 판정은 버튼·팝업 영역을 제외한 빈 영역에서만
  - **Halt Forecast**:
    - (i) 팝업 항목 문구·순서·활성 여부 → 사용자 지시로 확정(요구 이해·D9)
  - **Depends on**: T3

## 사전 승인 항목 (일괄 승인 대상)

- T1 — `docs/prd.md`에 FR-22 추가 및 FR-21 표현 정정, 결정 이력 1줄 추가 (PRD는 승인 후 고정 문서라 변경에 승인이 필요)
- T2 — 의존성 추가: `egui-phosphor = "0.13"` (사용자가 직접 지시. egui 0.35 요구 확인, 폰트 489KB 정적 포함)
- T2 — 공개 API 변경: `ui::app::install_korean_font` → `install_fonts` (호출부 1곳, 외부 사용 0곳)
- T3 — 창 생성 파라미터 변경: `with_decorations(false)` (OS 기본 창 장식 제거 — 창 그림자·둥근 모서리·스냅 레이아웃 팝업 상실을 동반)
- T3 — 구조 변경: 신규 모듈 `src/ui/titlebar.rs` 추가 및 `src/ui/mod.rs` 등록

## 불가피한 Halt (위임 불가)

- 커밋한 작업의 `master` 병합·push·태그·릴리즈·PR (작업 브랜치 `task/egui-migration` 로컬 커밋까지만 위임)
- plan에 없던 돌발 결정 — 예: 리사이즈·드래그가 eframe 경로에서 끝내 동작하지 않아 `ShellHost` 서브클래스로 `WM_NCHITTEST`를 직접 다루는 방향으로 트는 경우(아키텍처 변경이므로 반드시 확인)

## Verification Strategy

- 빌드: `cargo build` (경고 0)
- 린트: `cargo clippy --all-targets -- -D warnings`
- 포맷: `cargo fmt --check`
- 단위·통합 테스트: `cargo test` (기존 `tests/layout_flow.rs`·`tests/watcher.rs` 회귀 포함)
- 수동 검증 (HUMAN-VERIFY — 빌드로 확인 불가):
  1. 창을 띄워 타이틀바 모양이 참조 이미지와 맞는지(좌측 토글·중앙 이름·우측 설정+캡션 3종)
  2. 타이틀바 드래그 이동, 화면 가장자리 스냅, 더블클릭 최대화·복원
  3. 최소화·최대화·닫기 각 버튼, 최대화 시 아이콘이 복원 모양으로 바뀌는지
  4. 최대화했을 때 작업 표시줄을 덮지 않는지
  5. 창 4변·4모서리 리사이즈와 커서 모양
  6. 좌측 토글로 사이드바 접기/펼치기, 사이드바 내부 `◧` 버튼도 그대로 동작하는지
  7. 톱니 팝업의 5개 항목 문구·순서·회색 표시
  8. 종료 후 재실행 시 창 위치·크기·워크스페이스 복원(FR-11·FR-20 회귀)
  9. 파일 목록에서 우클릭했을 때 Windows 셸 컨텍스트 메뉴가 여전히 다크로 뜨는지 (D13 — `enable_dark_mode` 존치 회귀)
  10. 창을 아주 좁게 줄였을 때 제목이 좌우 버튼을 침범하지 않고 말줄임되는지 (D14)
  11. 설정 팝업이 **바깥 클릭·Esc로 닫히는지** (T5 acceptance — `egui::Popup::menu` 기본 동작에 의존하므로 화면으로만 확인 가능)
  12. **파일 목록·사이드바 스플리터를 창 가장자리 근처에서 드래그**했을 때 크기 조절이 가로채지 않고 기존 동작(스크롤·폭 조절)이 되는지 (T4 acceptance)
  13. 한글 글꼴(`C:\Windows\Fonts\malgun.ttf`)을 읽지 못하는 환경에서도 **타이틀바 아이콘이 두부(□) 없이 보이는지** (T2 acceptance — 코드 경로상 `set_fonts`가 항상 아이콘 글꼴을 포함하지만, 재현에는 글꼴 경로 변조가 필요해 실행 확인은 하지 않았다)

## Phase Ledger

- T1~T5 완료
- Phase F 통과 (F-7 1회차 MAJOR 2·MINOR 4 반영 후 재검토 — 2회차 BLOCKER·MAJOR 0)
- Phase G 통과 (Must 100% — F-7 전수 대조 재사용, 재루프 0회. 커버 대상 FR-22·21·19·11 전부 충족, active Must FR은 `## PRD Coverage`에서 이번 범위 외로 선언한 기구현 항목)
- F-8 통과 (2026-07-29 — 사용자가 실행 화면 확인 후 "잘됨". 시각 요소 분해 20행 판정을 사용자 확인으로 갱신)

## Retry Ledger

- T3: 리뷰 지적(BLOCKER) 수정 사이클 1/5 — 배경 끌기와 버튼 클릭 경합
- T4: 리뷰 지적(MAJOR) 수정 사이클 1/5 — 상단 크기 조절 띠가 버튼을 가로챔
- Phase F: F-7 재진입 1/3 — MAJOR 2건(최대화 가드 테스트 부재·수동 확인 목록 누락)

## Progress Log

- **T1~T3 완료** (커밋 `5977c39`·`326b440`·T3는 pre-review `8ffb061` → review-fix `c40e848`): PRD FR-22 신설, egui-phosphor 도입, 커스텀 타이틀바 골격.
  - **품질 리뷰가 잡은 BLOCKER가 이번 task의 핵심 교훈**: 타이틀바 배경을 `Sense::click_and_drag()`로 잡고 그 위에 버튼을 그리면, "나중에 그린 버튼이 클릭을 가져간다"는 통념과 달리 **버튼을 누른 프레임에 배경의 끌기 신호도 함께 나간다.** egui는 클릭 위젯과 끌기 위젯을 각각 독립으로 히트 판정하고(`hit_test.rs`), `is_pointer_button_down_on()`은 둘 중 하나만 걸려도 참이기 때문이다(`context.rs:1422~1426`). 그 결과 `StartDrag`로 OS 창 이동 루프가 열려 캡션 버튼 클릭이 삼켜질 수 있었다 — **끌기 판정 영역에서 버튼 자리를 아예 빼는 것**이 해법이었다.
  - `clicked()` 해소(어느 위젯이 클릭을 받나)와 `is_pointer_button_down_on()`(눌리자마자 나오는 신호)은 규칙이 다르다 — 겹쳐 놓고 전자로 후자를 기대하면 안 된다.
  - 좌측 38px(`LEFT_GROUP_WIDTH`)는 T5의 사이드바 토글 자리라 지금부터 끌기 영역에서 제외했다. T5에서 버튼을 넣을 때 같은 겹침 문제를 다시 겪지 않기 위함이다.
  - `TITLEBAR_BG` 상수는 만들지 않았다 — 시각 분해가 지정한 값이 기존 `WINDOW_BG`와 같아 별칭이 될 뿐이다(4-A·4-D에 정정 기록).
- **T4~T5 완료** (커밋 `9a1c0f6`·T5는 pre-review `0d449c1`): 가장자리 크기 조절, 사이드바 토글·설정 팝업.
  - **T3의 교훈이 T4에서 되풀이됐다** — 크기 조절 띠도 같은 겹침 문제를 안고 있었다. 상단 4px를 창 폭 전체에 균일하게 적용하니 캡션 버튼 위쪽 4px가 눌리는 순간 크기 조절 루프가 열려 그 클릭이 삼켜졌다(품질 리뷰 MAJOR). **모서리는 크기 조절, 버튼 위쪽 변은 버튼**으로 가른 것이 결론이다 — 모서리까지 버튼에 주면 대각선으로 창을 잡을 자리가 사라진다.
  - 계획 단계의 "최외곽 4px는 전부 크기 조절 우선(버튼은 남은 영역으로 누를 수 있다)"이라는 가정이 **실측으로 반증된 사례**다. T4 Edge Cases에 정정 기록.
  - 설정 팝업은 `egui::Popup::menu`의 기본 동작(클릭 토글·바깥 클릭·Esc 닫힘)에 얹어 별도 코드가 필요 없었다. 항목 5개는 배열로 묶지 않고 나열했다 — 곧 각자 다른 화면으로 갈라질 자리라 지금 묶으면 채우는 순간 다시 풀어야 한다.

## 완료 후 추가 수정 (사용자 화면 확인 중 발견)

- **패널 안 스크롤 영역 ID 충돌** (커밋 `41e90ef` 계열): 화면에 빨간 `First/Second use of ScrollArea ID` 경고가 떴다. egui가 이름 없는 하위 영역에 모두 같은 id를 주는 탓에 탭 스트립 스크롤과 파일 목록 스크롤의 ID가 겹친 것 — **이번 작업의 회귀가 아니라 기존 결함**(관련 파일 무변경을 `git diff`로 확인). `panel.rs`의 두 하위 영역에 이름을 부여해 해소하고, 헤드리스 egui로 경고 텍스트를 잡는 회귀 테스트를 추가했다(수정 전 RED 확인).

## Next Steps

- **완료** (2026-07-29) — 사용자 화면 확인까지 마쳤다. 브랜치 `task/custom-titlebar`에 로컬 커밋만 있고 push·병합은 하지 않았다.
- 권장 다음 액션: `master` 병합·push 전략 결정 (Deferred 대장의 "master 미병합 커밋 다수" 항목과 함께)
- 후속 작업: 설정 팝업 5개 항목의 실제 기능 (`docs/plans/deferred.md` 등록됨)

## Open Questions

- [x] Q1: 타이틀바 방식 → **커스텀 타이틀바 도입**(이미지 그대로), 창 그림자·스냅 레이아웃 상실 감수 (사용자 선택)
- [x] Q2: 설정 버튼 동작 → **설정 / 업데이트 / 릴리즈 노트 / 오픈소스 라이선스 / 정보** 5개 항목 표시, 기능은 추후 구현 (사용자 지시)
- [x] Q3: 미구현 항목 표시 방식 → **비활성(회색, 클릭 불가)** (사용자 선택)
- [x] Q4: 사이드바 내부 접기 토글 → **유지** (사용자 선택)
- [x] Q5: 아이콘 렌더 방식 → **`egui-phosphor` crate 설치해 사용** (사용자 지시)
