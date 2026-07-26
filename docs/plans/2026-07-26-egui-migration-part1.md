# egui 전면 이식 — part1 (탐색 코어)

**PRD**: `docs/prd.md`
**다음 plan**: `docs/plans/2026-07-26-egui-migration-part2.md`
**전체 목표**: 현행 Win32 UI(FR-1~FR-21)를 egui(eframe/glow)로 전면 이식하고 구 Win32 UI 코드를 제거한다. 이 part1은 그중 **탐색 코어**(창 골격·파일 목록·셸 연동·주소창·탭·패널·자유 분할)까지다.

## 요구 이해

> 원문: "NFR-2 미달 완화해서 이식해줘" (선행 맥락: "남은작업은?" → PoC 결론 제시 → 이식 결정)
> 범위 확인 결과 — **NFR-2 150MB 완화** / **병행 유지 후 교체** / **행 클릭 선행 수동 확인** / **현행 기능 전부 이식** 선택.

이해한 요구:
1. PoC로 검증된 egui(eframe/glow)를 **실제 앱에 전면 적용**한다. PoC는 별도 바이너리였지만 이번엔 제품 UI를 대체한다.
2. 이식의 걸림돌이던 **NFR-2(유휴 메모리 50MB)를 150MB로 완화**한다 — PoC 실측상 egui의 바닥값이 100MB라 기존 기준으로는 이식이 정의상 불가능했다. PRD는 2026-07-26 승인으로 갱신 완료.
3. **기능 요구는 하나도 줄이지 않는다** — 현행 FR-1~FR-21(Must+Should) 전부를 egui에서 동등하게 재현한다. 미구현 Could(FR-13·FR-14)는 이번에도 제외.
4. **현행 Win32 앱을 즉시 지우지 않는다** — egui 구현이 동작을 확인할 때까지 병행 유지하고, 교체·삭제는 part2 마지막에 한다.
5. 셸 연동(아이콘·컨텍스트 메뉴·변경 감시·파일 실행)은 windows-rs로 **유지**한다(PoC에서 egui 창에서도 동작 확인됨).

## Goal

`cargo run --bin file_explorer_egui`로 뜨는 egui 창에서 **자유 분할된 패널들이 각자 탭·주소창·파일 목록을 갖고 실제로 탐색되는** 상태를 만든다. part1 종료 시점에 워크스페이스 사이드바·폴더 트리·세션 복원은 아직 없고, 현행 Win32 앱(`file_explorer.exe`)은 그대로 살아 있다.

## Investigation Log

| 확인 사항 | 결과 | 확인 방법 |
|---|---|---|
| **행 클릭 HUMAN-VERIFY** (PoC 최대 리스크) | **해소 — 실제 마우스로 hover·click·더블클릭 진입·우클릭 셸 메뉴가 모두 동작.** PoC에서 "잡히지 않는다"고 본 것은 자동 마우스 주입(`SetCursorPos`+`mouse_event`)의 한계였다 | 사용자 수동 확인 (2026-07-26, `ScrollArea::show_rows` 직접 구성 버전) |
| 슬래시 경로 열거 실패 | **실재 버그.** `to_extended_pattern`이 `\\?\`를 붙이는데 이 접두사는 경로 정규화를 건너뛰므로 `/`가 구분자로 인식되지 않는다 → `C:/Windows/System32`로 실행 시 열거 실패, 백슬래시로는 정상. **현행 Win32 앱도 동일**(같은 함수 사용, 주소창 입력 경로) | PoC를 슬래시/백슬래시 경로로 각각 실행해 비교 (사용자 확인) |
| 순수 로직 재사용 범위 | `app/layout.rs`·`app/workspace.rs`·`app/settings.rs`·`panel/history.rs` — **Win32 의존 0**(HWND·`use windows` 각 0건, layout/workspace의 HWND 1건은 주석). 그대로 재사용 | 각 파일 `grep -c 'HWND\|use windows\|unsafe'` |
| 부분 재사용 심볼 | `panel/tabs.rs`의 `TabsModel`·`TabState`·`tab_title`(순수 모델, 파일 내 Win32 래퍼와 분리) / `panel/file_list.rs`의 `SortKey`·`format_size_kb`·`format_filetime` / `panel/address_bar.rs`의 `normalize_input` | 각 파일 pub 심볼 grep |
| `fs/watcher.rs` egui 적용 | **가능 — 수정 불필요.** `DirWatcher::start(path, tx, notify: Option<HWND>)`의 notify가 **이미 Option**이라 `None`을 주면 채널 송신만 한다 | `src/fs/watcher.rs:47-52` |
| `fs/enumerate.rs` egui 적용 | `spawn_enumerate`는 `notify: HWND` 필수라 쓸 수 없지만, **동기 `enumerate_dir`을 자체 워커로 감싸면 된다**(PoC D9에서 검증된 방식) | `src/fs/enumerate.rs:71,98` |
| `fs/icons.rs`·`fs/shell_menu.rs` | PoC에서 재사용 검증 완료 — `IconCache`(인덱스)→`IconTextures`(RGBA 변환), `show_context_menu`/`forward_menu_msg`(HWND 공급) | PoC T3·T4 실측 결과 |
| 다크 색상 정본 | `app/theme.rs`에 COLORREF 상수 9개(`WINDOW_BG` 0x1B1B1B, `SURFACE_BG` 0x1E1E1E, `TEXT` 0xE8E8E8, `HEADER_BG` 0x252525, `HEADER_TEXT` 0xC8C8C8, `TREE_LINE` 0x454545, `CONTROL_BG` 0x2A2A2A, `CONTROL_HOT` 0x383838, `CONTROL_ACTIVE` 0x454545, `TEXT_DIM` 0x6A6A6A) | `src/app/theme.rs` Read |
| egui 0.35 위젯 가용성 | `containers/`에 `collapsing_header`(트리)·`text_edit`(주소창)·`menu`(메뉴바)·`scroll_area`(목록)·`panel`(사이드바)·`resize`·`modal` 존재. **탭 스트립·자유 분할 스플리터는 내장 없음 → 직접 구현** | `~/.cargo/registry/.../egui-0.35*/src/{widgets,containers}/` 목록 |
| eframe 0.35 App trait | `update`가 아니라 **`logic(ctx, frame)` + `ui(ui, frame)`** 2메서드 구조 | PoC `main.rs:612-638` (실동작 코드) |
| wgpu 중복 포함 | 현재 `eframe = { features = ["glow"] }`로 **default-features가 켜져 있어 wgpu도 함께 빌드**된다(PoC exe 10.1MB "둘 다 포함"). `default-features = false`로 줄일 여지 → T1에서 실측 | `Cargo.toml:36` + PoC T5 실측 표 |
| 세션 스키마 재사용 | `Session{version:2, window:WindowState{x,y,w,h,maximized}, sidebar, active_workspace, workspaces}` — 전부 UI 프레임워크 중립. egui/winit도 창 위치·크기·최대화를 제공하므로 **스키마 변경 불필요** | `src/app/settings.rs:14-89` |
| Deferred 대장 | 대기 6건 — FR-13/14(Could, 이번 제외 유지), 트리→목록 동기화(part2 T1 재검토), shell_menu 주석(part1 T3), 사이드바 가상 스크롤(part2 T2에서 자동 해소 — egui `ScrollArea`), 인라인 편집 다크(part2 T2에서 자동 해소) | `docs/plans/deferred.md` Read |
| 이전 plan Deferred | egui PoC plan 6건 — 행 클릭(**해소**), 아이콘 스로틀·지연 조회(완료, T2에 이관), `sort_entries` 테스트(완료), glow/wgpu 비교(완료), 메모리 절감(T1에 이관), AGENTS stale(완료) | `docs/plans/2026-07-25-egui-poc.md` Read |
| 위키 참조 | vault 미설정 — 건너뜀 | — |
| AGENTS.md 신선도 | Build/Test 명령·`Plan Location`·구조 항목 모두 실재 확인. **단 Repository Structure에 `src/ui/`가 없다** → 이 이식으로 신설되므로 part2 T7에서 갱신 제안 | AGENTS.md ↔ `src/` 대조 |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| 분할 트리 연산 | `app::layout::{LayoutTree, compute_rects, SplitDir, TreeShape}` | **재사용** — i32 Rect를 egui f32로 변환만 |
| 탭 모델 | `panel::tabs::{TabsModel, TabState, tab_title}` | **재사용** |
| 히스토리 | `panel::history::History` | **재사용** |
| 정렬 키·비교 | `panel::file_list::SortKey` | **재사용** (비교 함수 `compare_entries`는 private `fn`(`file_list.rs:493`) → T2에서 `pub` 승격. 시그니처 `(a, type_a, b, type_b, key)`라 **종류 문자열이 함께 필요** — 목록이 `type_names`를 보유해야 한다) |
| 크기·날짜 문자열 | `panel::file_list::{format_size_kb, format_filetime}` (PoC에서 이미 `pub`) | **재사용** |
| 경로 입력 정규화 | `panel::address_bar::normalize_input` (이미 `pub` — `address_bar.rs:130`) | **재사용** (승격 불필요) |
| 디렉터리 열거·감시·아이콘·셸 메뉴 | `fs::*` | **재사용** (무수정) |
| 다크 팔레트 | `app::theme.rs` COLORREF 상수 | **값 재사용, 타입 신규** — egui는 `Color32`라 `ui::theme`에 동일 값으로 재정의(COLORREF는 BGR 순서라 직접 못 씀) |
| `IconTextures` (인덱스→텍스처) | PoC `src/bin/egui_poc/icon_tex.rs` | **이동 재사용** — `src/ui/icon_tex.rs`로 옮김 |
| `ShellHost` (HWND·서브클래스) | PoC `src/bin/egui_poc/shell_host.rs` | **이동 재사용** — `src/ui/shell_host.rs`로 옮김 |
| `FileListView` (목록 위젯) | PoC `main.rs::table` (단일 함수, 정렬·다중 선택 없음) | **신규** — 열 클릭 정렬·다중 선택·헤더가 필요해 위젯으로 승격 |
| `TabStrip`·`Splitter` | 없음 (egui 내장 없음, Win32판은 시스템 컨트롤 래퍼라 이식 불가) | **신규** |
| 의존성 | eframe(기존 optional → 정식 승격), raw-window-handle | 신규 추가 아님 — PoC에서 이미 도입, feature 게이트만 해제 |

## Impact Analysis

### 3-A. 영역별 해당 여부

| 영역 | 해당 | 근거 |
|---|---|---|
| DI 컨테이너 등록 | 비해당 | DI 컨테이너를 쓰지 않는다(계층형 단일 crate — AGENTS.md) |
| 이벤트 핸들러·옵저버 | **해당** | Win32 WndProc 콜백 → egui 즉시 모드로 **패러다임 전환**. 콜백 등록이 사라지고 매 프레임 폴링이 된다 |
| 직렬화/역직렬화 | **해당(무변경 재사용)** | `Session` v2 스키마를 그대로 쓴다 — part2 T5에서 배선. part1은 저장하지 않는다 |
| 마이그레이션 | 비해당 | 세션 스키마를 바꾸지 않으므로 마이그레이션 없음 |
| 권한·보안 | **해당** | 접근 거부 폴더(T2 Edge), 셸 메뉴는 기존대로 셸에 위임 |
| 캐싱·메모이제이션 | **해당** | 아이콘 텍스처 캐시 + 프레임당 생성 상한(PoC 실측 3096ms→522ms 스파이크 억제) |
| 멀티 스레드·비동기 | **해당** | 열거 워커 + 세대 번호 취소(T2), 감시 스레드(part2 T4) |
| 로깅·메트릭 | 비해당 | GUI 앱 — `println!` 금지(AGENTS.md). PoC의 진단 표시는 제품에 옮기지 않는다 |

### 3-B. 변경 대상 심볼의 파급

| 변경 대상 | 변경 내용 | 호출자·참조 (전수) | 갱신 task 필요 |
|---|---|---|---|
| `file_list::compare_entries` | private `fn` → `pub fn` (시그니처·동작 동일) | `file_list.rs` 내부 1곳 + 같은 파일 테스트 | **불필요** — 가시성 확대는 기존 호출을 깨지 않음 |
| `address_bar::normalize_input` | 이미 `pub` (파일 내 확인) — 변경 없음 | `panel.rs` 1곳 | **불필요** |
| `enumerate::to_extended_pattern` | **슬래시→백슬래시 정규화 추가** (동작 변경: 지금은 실패하는 입력이 성공하게 됨) | `enumerate_dir` 1곳(같은 파일). `enumerate_dir` 호출자는 `spawn_enumerate`·PoC·트리 | **T4에서 처리** — 실패하던 입력이 성공하는 방향이라 기존 정상 동작에 회귀 없음. 단위 테스트 추가 |
| `Cargo.toml` | eframe optional 해제 + `default-features = false` + `egui-poc` feature·`[[bin]] egui_poc` 제거, `[[bin]] file_explorer_egui` 추가 | 빌드 설정 | **T1** — 기본 빌드에 eframe이 들어오게 되는 의도된 변경 |

**기존 Win32 UI 코드는 part1에서 전혀 수정하지 않는다**(병행 유지). `src/app/{window,sidebar,menu,layout_host}.rs`·`src/panel/{panel,file_list,tabs,folder_tree,address_bar}.rs`는 가시성 승격 2곳을 빼면 무변경이며, `src/main.rs`(현행 진입점)도 그대로다.

## Decisions

- **D1 (병행 구조)**: 이식 코드는 **`src/ui/` 신규 모듈** + **`src/bin/egui_app/main.rs` 신규 진입점**(`file_explorer_egui.exe`)에 둔다. 현행 `src/main.rs`(`file_explorer.exe`)는 손대지 않는다. part2 T7에서 진입점을 승격하고 구 코드를 제거한다. Source: 사용자 선택(병행 유지 후 교체).
- **D2 (feature 게이트 해제)**: PoC의 `egui-poc` feature와 `[[bin]] egui_poc`를 제거하고 eframe을 **정식 의존성**으로 승격한다. 게이트의 목적이 "기본 빌드 오염 방지"였는데 egui가 주력이 된 이상 두 벌 빌드 경로를 유지할 이유가 없다(불필요한 간접화 — AGENTS 원칙). PoC 자산(`icon_tex.rs`·`shell_host.rs`)은 `src/ui/`로 이동해 살리고 PoC bin만 없앤다.
- **D3 (렌더 백엔드)**: **glow 고정**, `default-features = false`. PoC 실측에서 wgpu 359MB vs glow 131MB로 백엔드가 메모리를 2.7배 갈랐다. wgpu를 빌드에서 빼면 exe·초기 메모리가 더 줄 여지가 있어 T1에서 실측해 기록한다. Source: PoC T5 실측 표.
- **D4 (UI 계층 배치)**: `src/ui/`는 **lib에 포함**(`lib.rs`에 `pub mod ui`)하고 bin은 얇은 진입점만 둔다. 순수 로직(레이아웃 계산·정렬·탭 모델)이 `tests/`에서 테스트 가능해야 하기 때문(AGENTS "UI 로직은 순수 로직을 분리해 테스트"). Source: AGENTS.md Conventions.
- **D5 (다크 팔레트)**: `app/theme.rs`의 색상 **값**을 `ui/theme.rs`에 `egui::Color32`로 재정의한다. COLORREF는 0x00BBGGRR 순서라 그대로 못 쓰고, 현행과 같은 화면색을 유지하려면 값만 옮기는 것이 정확하다. egui 기본 다크를 그대로 쓰지 않는 이유는 FR-21이 "현행 고정 다크"를 요구하기 때문.
- **D6 (COM 초기화)**: PoC D5와 동일 — `CoInitializeEx(APARTMENTTHREADED)`의 `S_OK`/`S_FALSE`는 정상 진행, `RPC_E_CHANGED_MODE`면 셸 메뉴 비활성 + 화면에 사유 표시(임의 우회 금지). PoC 실측에서 `S_OK`였다.
- **D7 (열거 방식)**: `enumerate_dir` + 자체 워커 스레드 + `mpsc` + `ctx.request_repaint()`, 세대 번호로 늦은 결과 폐기. `spawn_enumerate`(HWND 통지)는 쓰지 않는다. Source: PoC D9 검증 완료.
- **D8 (아이콘)**: 보이는 행만 지연 조회 + 프레임당 텍스처 생성 **8개 상한**. PoC 후속 조사에서 지연 조회로 System32 로드 585→84ms, 상한으로 최대 렌더 3096→522ms로 개선됐다. Source: PoC 후속 처리 결과 ③④.
- **D9 (분할 렌더)**: `LayoutTree::compute_rects(area)`가 주는 i32 픽셀 Rect를 egui `Rect`(f32)로 변환해 각 패널을 그린다. **egui의 `SidePanel`/`TopBottomPanel`은 쓰지 않는다** — 그건 고정 방향 도킹이라 중첩 자유 분할(FR-1)을 표현하지 못하고, 이미 검증된 레이아웃 트리를 버리게 된다. 스플리터는 `SPLITTER_THICKNESS`(4px) 틈을 `Sense::drag()`로 히트테스트한다(현행 Win32판과 같은 "빈 틈 히트테스트" 방식).
- **D10 (다중 선택)**: 목록은 단일·Ctrl·Shift 선택을 지원한다. FR-8의 셸 컨텍스트 메뉴가 다중 항목을 받으므로(`show_context_menu(.., items: &[PathBuf], ..)`) 선택 모델이 먼저 있어야 T3가 성립한다. PoC는 단일 선택뿐이었다.
- **D11 (슬래시 경로 수정 위치)**: `to_extended_pattern` 안에서 `/`를 `\`로 치환한다. 호출부(주소창)에서 고치면 트리·다른 경로 진입에는 여전히 버그가 남는다 — 확장 접두사를 붙이는 그 지점이 근본 원인 위치다. Source: 원인 분석(Investigation Log).
- **D13 (내림차순에서도 폴더 우선 — T2 구현 중 확인)**: `compare_entries`는 폴더 우선을 정렬 방향과 무관하게 early return으로 처리하는데, 현행 `FileList::resort`는 그 결과 전체에 `.reverse()`를 걸어(`file_list.rs:316-319`) **내림차순일 때 파일이 폴더보다 위로 올라온다**. egui 판은 폴더/파일 판정을 먼저 하고 **같은 종류끼리만** reverse를 적용해 어느 방향에서도 폴더가 위에 오게 한다(Windows 탐색기 동작이자 T2 acceptance "폴더 우선 규칙이 유지된다"). `compare_entries`는 그대로 재사용하며, **현행 Win32 코드는 수정하지 않는다**(part1 불가피한 Halt — 병행 유지 전제). 두 앱의 이 동작 차이는 의도된 것이며 part2 T7에서 Win32 판이 제거되면 자연히 해소된다.
- **D12 (part1 미포함)**: 워크스페이스·사이드바·트리·메뉴바·단축키·감시·세션 저장은 part2다. part1은 **단일 워크스페이스·시작 폴더 고정**으로 동작하며 종료 시 아무것도 저장하지 않는다(part2 T5가 배선). 이렇게 나눠야 각 part가 독립 검증된다.

## Tasks

- [x] T1 의존성 정리·앱 골격·다크 팔레트 (Type C)
- [x] T2 파일 목록 위젯 — 가상 스크롤·정렬·선택·아이콘 (Type C)
- [x] T3 셸 연동 — 컨텍스트 메뉴·파일 실행 (Type C)
- [x] T4 주소창·히스토리 + 슬래시 경로 버그 수정 (Type C)
- [x] T5 탭 스트립 (Type C)
- [x] T6 패널 조립 (Type C)
- [x] T7 자유 분할 레이아웃·스플리터 (Type D)

### T1. 의존성 정리·앱 골격·다크 팔레트 (Type C)

- **PRD**: FR-21(다크, 기반), NFR-1·NFR-2(실측 기반선)
- **Files**: `Cargo.toml`, `src/lib.rs`, `src/ui/mod.rs`(신규), `src/ui/theme.rs`(신규), `src/ui/app.rs`(신규), `src/ui/icon_tex.rs`(PoC에서 이동), `src/ui/shell_host.rs`(PoC에서 이동), `src/bin/egui_app/main.rs`(신규), `src/bin/egui_poc/`(삭제)
- **Design**: ① 배치 — UI 구현은 `src/ui/`(lib 포함), bin은 `run_native` 호출만 하는 얇은 진입점 ② 신규 심볼 — `ui::app::ExplorerApp`(eframe::App 구현, 앱 전역 상태 보유) / `ui::theme::{apply_dark(&egui::Context), WINDOW_BG, SURFACE_BG, TEXT, HEADER_BG, HEADER_TEXT, TREE_LINE, CONTROL_BG, CONTROL_HOT, CONTROL_ACTIVE, TEXT_DIM}`(Color32 상수) / `ui::app::install_korean_font(&egui::Context)` ③ 의존 방향 — `ui` → `fs`·`app`(순수 로직)·`panel`(순수 모델) 단방향. 기존 모듈은 `ui`를 모른다 ④ 비추상화 — UI 추상 트레이트·위젯 팩토리·테마 전환 레이어를 두지 않는다. 팔레트는 상수, 앱은 `eframe::App` 구현체 하나
- **내용**: eframe optional 해제 + `default-features = false, features = ["glow"]`, `egui-poc` feature·bin 제거, `file_explorer_egui` bin 추가. PoC의 `icon_tex.rs`·`shell_host.rs`를 `src/ui/`로 이동(경로·가시성만 조정, 로직 무변경). 창 기동 + 맑은 고딕 로드 + 다크 팔레트 적용 + COM STA(D6)
- **Acceptance**:
  - `cargo run --bin file_explorer_egui` → 창이 뜨고 한글이 두부(□) 없이 표시된다
  - `cargo build`(현행 Win32 앱)가 여전히 성공하고 `file_explorer.exe`가 생성된다
  - `cargo tree -e normal | grep wgpu` → **출력 없음**(wgpu 미포함)
  - 창 배경이 `WINDOW_BG`(0x1B1B1B), 기본 글자가 `TEXT`(0xE8E8E8) — 현행 앱과 같은 색
  - **실측 기록**: release exe 크기 / 창 표시까지 ms / Working Set MB를 plan에 표로 남긴다(PoC의 10.1MB·47ms·131MB 대비 wgpu 제거 효과)
  - `cargo clippy --all-targets -- -D warnings` 경고 0, `cargo fmt --check` 통과
- **실측 결과 (release, T1 골격 상태 — 목록·패널 미구현)**:

  | 항목 | egui (glow) | 현행 Win32 | PoC 당시 (glow) |
  |---|---|---|---|
  | 창 표시 (첫 실행) | 618 ms | 358 ms | 47 ms |
  | 창 표시 (2·3회차) | **51 / 53 ms** | — | — |
  | Working Set | **130.0 MB** | 26.4 MB | 131 MB |
  | Private | 87.5 MB | 5.7 MB | 91.3 MB |
  | exe 크기 | **3.87 MB** | 0.42 MB | 10.1 MB |

  - **wgpu 제거 효과가 exe에서 크다**: 10.1 MB → **3.87 MB**(-62%). 메모리는 131 → 130 MB로 거의 같아, PoC에서 본 "wgpu vs glow 2.7배" 차이는 **런타임 백엔드 선택**이 만든 것이지 빌드 포함 여부가 아니었다.
  - 첫 실행 618 ms는 셰이더·폰트 캐시 초기화가 섞인 값이고, 2회차 이후 51~53 ms로 PoC 실측(47 ms)과 일치한다.
  - NFR-1(1초) ✅ / NFR-2(150MB) ✅ — 단 **여유가 20 MB뿐**이며 이 수치는 목록·패널이 없는 골격 상태다. 최종 판정은 part2 T6.
- **Edge Cases**: 폰트 파일 부재 → 기본 폰트 폴백(한글 깨져도 앱은 뜬다) / COM 3분기(D6) / glow 컨텍스트 생성 실패(구형 GPU·원격 데스크톱) → eframe 에러를 그대로 보고하고 종료(임의 wgpu 폴백 금지 — D3가 glow 고정)
- **Halt Forecast**: `Cargo.toml` 의존성 구조 변경(→ 사전 승인) / PoC bin 삭제(→ 사전 승인) / wgpu 제거 후 빌드 실패(→ eframe이 glow만으로 빌드 안 되면 default-features 복구하고 사유 기록)

### T2. 파일 목록 위젯 — 가상 스크롤·정렬·선택·아이콘 (Type C)

- **PRD**: FR-4(상세 보기·열 클릭 정렬), FR-5(시스템 아이콘), NFR-3(10만 파일)
- **Files**: `src/ui/file_list.rs`(신규), `src/ui/app.rs`, `src/panel/file_list.rs`(가시성 승격 1줄)
- **Design**: ① 배치 — 목록 위젯을 `ui/file_list.rs`에 격리(그리기·입력·선택 상태) ② 신규 심볼 — `ui::file_list::FileListView`(스크롤 위치·선택 집합·정렬 키·행별 종류 문자열 보유 — `compare_entries`가 종류를 인자로 받으므로 정렬에도 필요) / `FileListView::show(&mut self, ui, entries, ctx) -> FileListAction`(더블클릭·우클릭·선택 변경을 값으로 반환 — 즉시 모드라 콜백을 두지 않는다) / `enum FileListAction { None, Open(usize), Context{ index: Option<usize>, pos: Pos2 } }` / `ui::app::DirLoad`(워커 열거 상태: 세대·수신부) ③ 의존 방향 — `fs::enumerate`·`fs::icons`·`panel::file_list::{SortKey, compare_entries, format_*}` 참조 ④ 비추상화 — 컬럼 정의를 데이터 주도 테이블 엔진으로 일반화하지 않는다. 4열 고정이므로 그리기 코드에 직접 쓴다
- **내용**: `ScrollArea::show_rows`로 보이는 행만 렌더(PoC 검증 방식). 4열(이름·크기·종류·수정일) + 헤더(열 클릭 시 정렬 토글, 방향 표시). 행 rect를 `allocate_space`+`interact`로 잡아 hover·click·더블클릭·우클릭 감지(PoC에서 동작 확인된 방식). 선택은 단일/Ctrl 토글/Shift 범위(D10). 아이콘은 지연 조회 + 프레임당 8개 상한(D8). 열거는 워커 스레드 + 세대 번호(D7)
- **Acceptance**:
  - 10만 파일 폴더 스크롤 시 프레임 시간 **p95 16ms 이하**(release 측정, PoC는 debug에서 p95 10.55ms)
  - 열거 중에도 창이 응답한다(스피너 갱신·창 이동 가능)
  - 이름/크기/종류/수정일 **헤더 클릭으로 각각 정렬**되고 재클릭 시 역순, 폴더 우선 규칙이 유지된다
  - 크기·날짜 문자열이 현행 앱과 같은 형식(`1,206 KB`, `2026-07-25 09:30`)
  - 폴더·`.exe`·`.txt`가 각각 다른 시스템 아이콘으로 표시되고, 같은 확장자 수백 행을 스크롤해도 텍스처 수가 **아이콘 종류 수만큼**만 는다
  - 단일 클릭 선택 / Ctrl+클릭 토글 / Shift+클릭 범위 선택이 동작한다
  - GDI 개체 수가 10만 파일 스크롤 후에도 안정적(누수 없음 — PoC 60→60)
  - **정렬 단위 테스트**: 폴더 우선·자연 정렬(숫자 인지)·빈 목록 + 열별 정렬(크기·수정일) — PoC 삭제로 `sort_entries` 테스트 3개가 함께 없어졌으므로 이번에 순수 로직으로 되살린다
- **실측 결과 (release, 10만 파일 폴더 `%TEMP%\egui_migration_100k`)**:

  | 항목 | 값 | 판정 |
  |---|---|---|
  | **렌더 시간**(`ui()` 내부, 스크롤 380프레임) | avg 0.49 / **p95 0.63** / max 4.93 ms | ✅ acceptance(16ms) 충족 |
  | 프레임 간 간격(vsync·present 대기 포함) | avg 14.37 / p95 26.09 / max 61.93 ms | 참고 — 평균 약 70fps |
  | 창 표시까지 (10만 파일) | 887 ms | 열거는 창 표시 후 워커에서 진행 |
  | Working Set (10만 파일) | 154.1 MB | 참고 — NFR-2는 일반 폴더 기준 |
  | Working Set (홈 폴더) | **129.9 MB** / Private 90.2 MB | ✅ NFR-2(150MB) 충족 |
  | GDI 개체 (스크롤 중) | 62 | ✅ 누수 징후 없음 (PoC 60과 동일 수준) |

  - **지표 주의**: PoC가 잰 "프레임 시간"은 `ui()` 내부 렌더 시간이다. 프레임 **간격**(26ms)은 vsync 대기가 포함돼 우리 코드 비용이 아니다. 같은 기준(렌더 시간)으로는 release에서 **p95 0.63ms**로, PoC의 debug 실측(p95 16.99ms) 대비 최적화 효과가 크다.
  - 측정은 임시 벤치 패치(자동 스크롤 + 표본 수집)로 수행하고 **측정 후 되돌렸다** — 진단 코드는 제품에 남기지 않는다(plan 3-A 로깅·메트릭 비해당).
  - ⏳ **HUMAN-VERIFY**: 아이콘이 종류별로 다르게 보이는지 · 선택(단일/Ctrl/Shift) 시각 동작 · 정렬 화살표 표시 · 열거 중 창 이동 가능 여부는 화면 확인이 필요하다(코드 경로·단위 테스트는 통과).
- **Edge Cases**: 빈 폴더(행 0) / 접근 거부(`EnumOutcome::AccessDenied` → 목록 비우고 사유 표시) / 260자 초과·유니코드 파일명(NFR-5) / 열거 중 폴더 이동 → 세대 불일치 결과 폐기 / 정렬 중 항목 0 / `ImageList_GetIcon` 실패(null HICON) → 아이콘 생략 / Shift 범위 선택 중 목록이 갱신됨 → 선택 인덱스 클램프
- **Halt Forecast**: `compare_entries` 가시성 승격(→ 사전 승인) / 10만 파일 테스트 폴더 생성(→ 사전 승인, 시스템 임시 폴더) / release에서 p95가 16ms를 넘으면 원인(아이콘·텍스트 레이아웃)을 분리 측정해 기록하고 계속 진행(자동 기각하지 않음 — PoC와 같은 보고 원칙)

### T3. 셸 연동 — 컨텍스트 메뉴·파일 실행 (Type C)

- **PRD**: FR-7(더블클릭 실행), FR-8(셸 컨텍스트 메뉴)
- **Files**: `src/ui/shell_host.rs`, `src/ui/app.rs`, `src/fs/shell_menu.rs`(주석만 — Deferred 항목)
- **Design**: ① 배치 — HWND 획득·서브클래스·메뉴 호출을 `ui/shell_host.rs`에 유지(PoC 구조 계승) ② 신규 심볼 — `ShellHost::popup_items(&self, folder, items: &[PathBuf], screen_pos)`(다중 선택 대응) / `ui::shell_host::execute(path: &Path)`(`ShellExecuteEx` 래퍼) ③ 의존 방향 — `fs::shell_menu` 재사용, 역참조 없음 ④ 비추상화 — 플랫폼 추상 트레이트를 두지 않는다(Windows 전용 앱)
- **내용**: 행 우클릭 → 선택된 항목들을 대상으로 셸 메뉴, 빈 영역 우클릭 → 폴더 배경 메뉴. egui 좌표를 화면 좌표로 변환해 `show_context_menu` 호출(모달 루프 동안 프레임 정지는 정상). 파일 더블클릭 → `ShellExecuteEx`, 폴더 더블클릭 → 진입. `shell_menu.rs`에 `items` 비지 않음 계약을 doc 주석으로 명시(Deferred 대장 항목 소진)
- **Acceptance**:
  - 파일 행 우클릭 → 셸 메뉴 표시, **"속성"이 실제로 열린다**
  - **"보내기" 등 서브메뉴가 펼쳐진다**(= `forward_menu_msg` 포워딩 동작 — 서브클래스 미설치 시 빈 서브메뉴가 나오므로 구분 가능)
  - 여러 항목 선택 후 우클릭 → **선택 전체가 대상**인 메뉴가 뜬다(예: 여러 파일 복사)
  - 빈 영역 우클릭 → 배경 메뉴("새로 만들기" 포함). **단 "빈 영역"은 목록이 뷰포트를 채우지 않을 때만 존재한다** — T2에서 배경 감지 rect를 콘텐츠 끝~뷰포트 바닥으로 좁혔기 때문이며(행 클릭을 가로채지 않으려면 겹칠 수 없다), Windows 탐색기도 같다. 스크롤이 생기는 폴더에서 배경 메뉴 진입점이 따로 필요한지는 T3에서 판단해 기록한다
  - `.txt` 더블클릭 → 연결 프로그램 실행 / 폴더 더블클릭 → 진입
  - 메뉴를 닫은 뒤 창이 정상 갱신되고 입력이 계속 동작한다
- **구현 결과**:
  - `ui::shell_host::execute(path)` 신규(`ShellExecuteExW` 래퍼) — 현행 `panel.rs::shell_open`과 같은 동작이지만, 그 함수는 private이고 part1에서 Win32 코드를 건드리지 않기로 했으므로(불가피한 Halt) 승격 대신 신규 작성했다. 원본은 part2 T7에서 파일째 사라진다.
  - `ShellHost::popup`이 이미 `items: &[PathBuf]`를 받으므로 plan Design의 `popup_items` 신규 심볼은 만들지 않았다(같은 일을 하는 함수가 이미 있어 YAGNI).
  - **배경 메뉴 진입점 판단**: 스크롤이 생기는 폴더에서 빈 영역이 없어지는 문제에 대해 **별도 진입점을 만들지 않기로 했다** — Windows 탐색기도 동일하며, 대체 진입점(헤더 우클릭 등)은 표준 동작이 아니라 오히려 혼란스럽다. 파일 작업은 항목을 선택해 우클릭하면 되고, "새로 만들기"는 목록이 짧은 폴더에서 주로 쓰인다.
  - ⏳ **HUMAN-VERIFY**: 셸 메뉴 표시·"속성" 실행·서브메뉴 펼침·다중 선택 대상 메뉴·배경 메뉴·파일 실행·폴더 진입은 모두 화면 확인이 필요하다(PoC에서 단일 선택 기준으로는 이미 확인됨 — 다중 선택은 이번이 처음).
- **Edge Cases**: HWND 획득 실패 → 셸 메뉴 비활성 + 사유 표시 / 선택 0개인데 행 우클릭(선택되지 않은 행 우클릭 시 그 행을 선택 후 메뉴) / 메뉴 표시 중 대상 파일 삭제(기존 코드가 조용히 무시) / `ShellExecuteEx` 실패(연결 프로그램 없음) → 오류 무시, 앱 유지 / 모달 메뉴 중 창 크기 변경
- **Halt Forecast**: `RPC_E_CHANGED_MODE`로 STA 확보 실패(D6) → 셸 메뉴 비활성으로 보고 / winit이 창을 재생성해 HWND 무효화 → 매 프레임 재확인 방식으로 전환

### T4. 주소창·히스토리 + 슬래시 경로 버그 수정 (Type C)

- **PRD**: FR-6(주소 입력·뒤로·앞으로·상위)
- **Files**: `src/ui/address_bar.rs`(신규), `src/ui/app.rs`, `src/fs/enumerate.rs`(경로 정규화 수정)
- **Design**: ① 배치 — 주소 스트립 위젯을 `ui/address_bar.rs`에 격리 ② 신규 심볼 — `ui::address_bar::AddressBar`(입력 버퍼·포커스 상태) / `AddressBar::show(&mut self, ui, current: &Path, hist: &History) -> Option<NavAction>` / `enum NavAction { Back, Forward, Up, Goto(PathBuf) }` ③ 의존 방향 — `panel::history::History`·`panel::address_bar::normalize_input` 재사용 ④ 비추상화 — 경로 자동완성·드롭다운을 만들지 않는다(현행에도 없음, Out of Scope)
- **내용**: `[←][→][↑]` 버튼(히스토리 가능 여부로 enable) + `TextEdit` 경로 입력(Enter 커밋, Esc 되돌리기). `to_extended_pattern`에서 `/`→`\` 정규화(D11) + 단위 테스트
- **Acceptance**:
  - 주소창에 경로 입력 후 Enter → 해당 폴더로 이동, 실패 시 현 위치 유지 + 사유 표시(현행 pending-커밋 모델과 동일)
  - 뒤로/앞으로/상위 버튼이 동작하고, 불가능한 상태에서 **비활성으로 보인다**
  - **`C:/Windows`처럼 슬래시가 섞인 경로도 정상 열거된다** (수정 전에는 실패)
  - `to_extended_pattern` 단위 테스트: 슬래시 경로·백슬래시 경로·UNC(`\\server\share`)·이미 `\\?\`인 경로·끝에 구분자가 있는 경로 → 각각 기대 패턴
  - 폴더 이동 시 히스토리가 쌓이고 뒤로 가면 이전 폴더로 돌아온다
- **Edge Cases**: 존재하지 않는 경로 입력 / 파일 경로 입력(폴더가 아님) / 빈 문자열 Enter / 상대 경로(`..`, `sub`) → `normalize_input`이 처리 / 드라이브 루트에서 상위 버튼 / UNC 경로 / 입력 중 다른 곳 클릭(포커스 이탈 시 입력 버림)
- **Halt Forecast**: `to_extended_pattern` 동작 변경(→ 사전 승인. 실패하던 입력이 성공하는 방향이라 회귀 위험은 낮지만 `fs` 공용 함수라 명시)

### T5. 탭 스트립 (Type C)

- **PRD**: FR-3(패널별 독립 탭·탭별 히스토리)
- **Files**: `src/ui/tabs.rs`(신규), `src/ui/app.rs`
- **Design**: ① 배치 — 탭 그리기·입력을 `ui/tabs.rs`에, 상태는 재사용 모델(`TabsModel`)이 보유 ② 신규 심볼 — `ui::tabs::show_tab_strip(ui, model: &TabsModel) -> Option<TabAction>` / `enum TabAction { Switch(usize), Close(usize), New }` ③ 의존 방향 — `panel::tabs::{TabsModel, TabState, tab_title, CloseOutcome}` 재사용 ④ 비추상화 — 탭 드래그 재정렬·탭 미리보기를 만들지 않는다(현행에 없음)
- **내용**: 가로 탭 버튼 나열(활성 탭 강조 — `CONTROL_ACTIVE`) + 각 탭 `×` 닫기 + 끝에 `+` 새 탭. 탭 제목은 `tab_title`(폴더명). 탭마다 `History`를 갖는다
- **Acceptance**:
  - `+`로 탭 추가, `×`로 닫기, 클릭으로 전환된다
  - **마지막 탭은 닫아도 패널이 사라지지 않는다**(`CloseOutcome`의 기존 계약대로)
  - 탭을 전환하면 그 탭의 폴더·히스토리가 복원된다(탭 A에서 뒤로 가도 탭 B 히스토리에 영향 없음)
  - 탭 제목이 폴더명으로 표시되고, 탭이 많아지면 스트립 안에서 가로 스크롤된다
- **Edge Cases**: 탭 1개일 때 닫기 / 탭 제목이 매우 긴 폴더명 → 말줄임 / 드라이브 루트(`C:\`)의 탭 제목 / 탭 20개 이상 / 탭 닫은 뒤 활성 인덱스 조정(`CloseOutcome`이 규정)
- **Halt Forecast**: 없음 — 재사용 모델이 이미 테스트된 순수 로직이고 신규 코드는 그리기·입력뿐

### T6. 패널 조립 (Type C)

- **PRD**: FR-3(패널 단위), FR-4·FR-6 배치
- **Files**: `src/ui/panel.rs`(신규), `src/ui/app.rs`
- **Design**: ① 배치 — 한 패널의 세로 구성(탭 스트립 / 주소창 / 파일 목록)을 `ui/panel.rs`가 조립 ② 신규 심볼 — `ui::panel::PanelState`(탭 모델·탭별 히스토리·목록 뷰·열거 상태·현재 항목) / `PanelState::show(&mut self, ui, ctx, shell: &ShellHost)` / `PanelState::navigate(&mut self, path, ctx)`(pending-커밋: 열거 성공 시에만 경로·히스토리 커밋) ③ 의존 방향 — `ui::{tabs, address_bar, file_list, shell_host}`를 조립. 패널끼리는 서로를 모른다 ④ 비추상화 — 패널 인터페이스 트레이트를 두지 않는다(구현체가 하나뿐)
- **내용**: 패널 하나가 완전한 탐색 단위가 되도록 T2~T5를 결합. 탐색은 pending-커밋 모델(열거 성공 시에만 커밋 — 현행 `panel.rs` 계약 계승). 패널 테두리로 활성 표시
- **Acceptance**:
  - 패널 1개로 폴더 탐색이 끝까지 된다: 진입 → 목록 → 정렬 → 우클릭 메뉴 → 주소 입력 → 뒤로 → 탭 전환
  - 열거 실패(삭제된 폴더·권한 없음) 시 **현 위치가 유지**되고 오류 문구만 뜬다(주소창·히스토리 미커밋)
  - 탭마다 독립 히스토리가 유지된다
  - 패널 내부가 탭 / 주소창 / 목록 순으로 세로 배치되고 창 크기를 바꿔도 목록이 남은 공간을 채운다
- **Edge Cases**: 창이 아주 작을 때(목록 높이 0) / 열거 중 탭 전환 → 세대 폐기 / 패널 생성 직후 첫 프레임(빈 상태) / 시작 폴더가 존재하지 않음 → 홈 폴더 폴백
- **Halt Forecast**: 없음 — 신규 파일 조립이며 기존 코드 변경 없음

### T7. 자유 분할 레이아웃·스플리터 (Type D)

- **PRD**: FR-1(자유 분할), FR-2(스플리터 드래그·패널 닫기)
- **Files**: `src/ui/splitter.rs`(신규), `src/ui/app.rs`, `tests/`(레이아웃 통합 테스트 보강)
- **Design**: ① 배치 — 분할 렌더·스플리터 히트테스트·드래그를 `ui/splitter.rs`에, 패널 소유는 `ui::app::ExplorerApp`(패널 맵 `HashMap<PanelId, PanelState>`) ② 신규 심볼 — `ui::splitter::show_layout(ui, tree: &mut LayoutTree, panels: &mut HashMap<PanelId, PanelState>, active: &mut PanelId, ..)` / `ui::splitter::hit_splitter(computed: &ComputedLayout, pos: Pos2) -> Option<NodePath>` / `to_egui_rect(layout::Rect) -> egui::Rect` ③ 의존 방향 — `app::layout::{LayoutTree, ComputedLayout, SplitterRect, NodePath, SplitDir, MIN_PANE_SIZE, SPLITTER_THICKNESS}` 재사용, egui는 그리기만 ④ 비추상화 — 도킹 프레임워크·탭 드래그 병합을 만들지 않는다(FR 범위 밖). `SidePanel`류 egui 컨테이너도 쓰지 않는다(D9)
- **내용**: `compute_rects`로 얻은 사각형에 각 `PanelState`를 그린다. 스플리터 틈에서 커서 모양 변경 + `Sense::drag()` 드래그 → `set_ratio` 호출(`MIN_PANE_SIZE` 클램프는 기존 로직이 수행). 활성 패널 클릭 전환. 좌우/상하 분할·닫기 명령(part1은 임시 버튼, part2 T3에서 메뉴·단축키로 대체)
- **Acceptance**:
  - 좌우 분할·상하 분할이 되고 **중첩 분할**(분할된 패널을 다시 분할)이 동작한다
  - 스플리터 드래그로 비율이 바뀌고, `MIN_PANE_SIZE`(120px) 미만으로는 줄지 않는다
  - 패널을 닫으면 형제 패널이 공간을 흡수하고, **마지막 1개 패널은 닫히지 않는다**
  - 각 패널이 **독립 탭·독립 폴더**를 유지한다(패널 A 이동이 B에 영향 없음)
  - 활성 패널이 시각적으로 구분되고 클릭으로 전환된다
  - 창 크기 변경 시 비율이 유지된 채 재배치된다
  - 통합 테스트: `LayoutTree` split→set_ratio→close 시퀀스 후 `compute_rects` 결과 검증(순수 로직, HWND 불필요)
- **구현 결과**:
  - plan Design의 `hit_splitter(computed, pos) -> Option<NodePath>`는 **만들지 않았다** — egui `ui.interact(rect, id, Sense::drag())`가 히트테스트와 드래그 상태를 함께 처리하므로 좌표 판정 함수를 따로 두면 같은 일을 두 벌로 하게 된다(T3의 `popup_items`와 같은 판단).
  - **활성 패널 판정은 포인터 위치로** 한다(`ctx.input(...interact_pos)`). 패널 rect에 `interact`를 걸면 그 위젯이 나중에 등록돼 목록·버튼 클릭을 가로챈다(T2에서 겪은 문제와 같은 원리).
  - **명령 실행은 명령 줄을 그린 뒤**에 한다 — 분할 가능 영역은 명령 줄·구분선이 자리를 차지한 뒤에야 확정되므로, 그리기 전 영역으로 판정하면 상하 분할에서 `MIN_PANE_SIZE` 검사가 명령 줄 높이만큼 느슨해진다(리뷰 지적).
  - **닫은 뒤 활성 패널**은 닫힌 자리와 가장 많이 겹치는 패널로 정한다 — 트리 순서상 첫 패널을 고르면 포커스가 화면 반대편으로 튄다(리뷰 지적).
  - ⏳ **HUMAN-VERIFY**: 분할·중첩 분할·스플리터 드래그·활성 테두리·패널별 독립 탐색은 화면 확인이 필요하다(배치 계산은 통합 테스트로 검증).
- **Edge Cases**: 4분할 이상 중첩 / 창이 `MIN_PANE_SIZE`보다 작아짐 → 클램프 / 스플리터 드래그 중 창 크기 변경 / 활성 패널을 닫았을 때 다음 활성 선택 / 패널 닫힘 시 `PanelState` 정리(열거 워커의 늦은 결과 무시) / 분할 직후 새 패널의 시작 폴더(원본 패널과 동일)
- **Halt Forecast**: `compute_rects`의 i32↔f32 변환에서 스플리터 폭이 어긋나 히트테스트가 안 잡히면 → 정수 반올림 규칙을 맞추고 기록(설계 변경 아님) / 패널 수가 많을 때 프레임 시간 급증 → 실측해 기록하고 계속 진행

## 사전 승인 항목 (일괄 승인 대상)

- `Cargo.toml` 변경: eframe optional 해제 + `default-features = false, features = ["glow"]`, `raw-window-handle` optional 해제, `egui-poc` feature 및 `[[bin]] egui_poc` 제거, `[[bin]] file_explorer_egui` 추가
- `src/bin/egui_poc/` 삭제 (자산 `icon_tex.rs`·`shell_host.rs`는 `src/ui/`로 이동 후)
- `src/ui/` 신규 파일 생성 (mod·theme·app·file_list·shell_host·icon_tex·address_bar·tabs·panel·splitter)
- `src/lib.rs`에 `pub mod ui;` 추가
- `src/panel/file_list.rs`의 `compare_entries` 가시성 `pub` 승격 (동작·시그니처 무변경)
- `src/fs/enumerate.rs`의 `to_extended_pattern` 슬래시 정규화 (버그 수정 — 실패하던 입력이 성공하는 방향)
- `src/fs/shell_menu.rs`에 `items` 계약 doc 주석 추가 (Deferred 대장 소진)
- 성능 측정용 대량 파일 폴더 생성·삭제 (**시스템 임시 폴더 한정**)
- 각 task 완료 시 로컬 작업 브랜치 commit

## 불가피한 Halt (위임 불가)

- push · PR · main 병합 · 태그 · 릴리즈
- **현행 Win32 UI 코드(`src/app/{window,sidebar,menu,layout_host}.rs`·`src/panel/{panel,file_list,tabs,folder_tree,address_bar}.rs`·`src/main.rs`)의 동작 변경 또는 삭제** — part1은 병행 유지가 전제다(가시성 승격 1곳은 위 사전 승인에 포함)
- `docs/prd.md` 추가 변경 (2026-07-26 승인분 외)
- 위 사전 승인 범위를 넘는 의존성 추가

## Out of Scope

- FR-13(숨김 파일 토글)·FR-14(분할 프리셋) — Could, 현행에도 미구현. 이번 이식 범위 밖(Deferred 대장 유지)
- 드래그 앤 드롭, 잘라내기/붙여넣기, 파일 검색, 즐겨찾기, 셸 가상 폴더 (PRD Out of Scope 그대로)
- 탭 드래그 재정렬, 패널 간 탭 이동 (현행에도 없음)
- 테마 전환 UI (PRD Out of Scope)

## Deferred / Follow-up

- **part2로 이월**: 폴더 트리(FR-9) · 워크스페이스 사이드바(FR-15~19) · 메뉴바·단축키(FR-12) · 변경 감시(FR-10) · 세션 저장·복원(FR-11·FR-20) · NFR 실측 · 구 Win32 코드 제거
- **트리→목록 양방향 동기화** (deferred 대장 [2026-07-23]) — part2 T1에서 재검토. 현행은 단방향
- **[SUGGEST] 전역 공유 자원 묶기** — `PanelState::show`가 `icons`·`textures`·`shell`을, `splitter::show_layout`이 인자 8개를 받는다. `SharedResources<'a>` 묶음으로 감싸면 공유 자원이 늘어도 시그니처가 안 바뀐다. **part2 T3(메뉴바·단축키)에서 함께 정리한다** — T3가 명령 관련 인자를 더 얹을 가능성이 높아 지금 묶으면 두 번 손대게 된다는 T7 리뷰 의견을 따랐다 (T6 리뷰 S1 → T7 리뷰 m2)
- **[MINOR] 앱 경고 문구 위치** — 폰트·셸 경고가 탭 스트립보다 위(창 최상단)에 표시된다. 앱 전역 경고라 최상단이 자연스럽다고 보고 두었으나, 패널이 여러 개가 되면 위치를 재검토할 여지가 있다 (T6 spec 리뷰 M1)
- **[SUGGEST] 탭 제목 말줄임의 grapheme 경계** — `ui::tabs::elide`가 코드포인트(`chars()`) 단위로 자른다. 패닉은 없지만 결합 문자·ZWJ 이모지가 섞인 폴더명이 경계에 걸리면 글자가 깨져 보일 수 있다. 실사용 빈도가 낮아 이번엔 두고, 필요해지면 grapheme 단위로 바꾼다 (T5 quality 리뷰 m1)
- **한글 폰트 서브셋** — PoC 조사에서 폰트가 약 27MB 기여. NFR-2를 150MB로 완화해 당장은 불필요하지만, 메모리를 더 줄여야 하면 상용 2350자 서브셋이 첫 후보
- `debug-2026-07-24-dark-ownerdraw.md`가 레포 루트에 커밋돼 있음(T1 커밋 `26e8002`에 포함) — 위치가 `docs/`가 아니라 루트. part2 정리 시 이동 여부 결정

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|---|---|---|---|
| FR-1 | Must | part1 T7 | ✅ 커버 |
| FR-2 | Must | part1 T7 | ✅ 커버 |
| FR-3 | Must | part1 T5, T6 | ✅ 커버 |
| FR-4 | Must | part1 T2 | ✅ 커버 |
| FR-5 | Must | part1 T2 | ✅ 커버 |
| FR-6 | Must | part1 T4 | ✅ 커버 |
| FR-7 | Must | part1 T3 | ✅ 커버 |
| FR-8 | Must | part1 T3 | ✅ 커버 |
| FR-15 | Must | part2 T2 | ⏭️ 다음 part |
| FR-16 | Must | part2 T2 | ⏭️ 다음 part |
| FR-17 | Must | part2 T2 | ⏭️ 다음 part |
| FR-9 | Should | part2 T1 | ⏭️ 다음 part |
| FR-10 | Should | part2 T4 | ⏭️ 다음 part |
| FR-11 | Should | part2 T5 | ⏭️ 다음 part |
| FR-12 | Should | part2 T3 | ⏭️ 다음 part |
| FR-18 | Should | part2 T2 | ⏭️ 다음 part |
| FR-19 | Should | part2 T2 | ⏭️ 다음 part |
| FR-20 | Should | part2 T5 | ⏭️ 다음 part |
| FR-21 | Should | part1 T1(팔레트) + 전 task 적용 | ✅ 커버 |
| FR-13 | Could | (없음) | 명시적 제외 — Out of Scope |
| FR-14 | Could | (없음) | 명시적 제외 — Out of Scope |
| NFR-1 | — | part1 T1 실측 · part2 T6 최종 | ✅ 커버 |
| NFR-2 | — | part1 T1 실측 · part2 T6 최종 | ✅ 커버 |
| NFR-3 | — | part1 T2 | ✅ 커버 |
| NFR-4 (DPI) | — | part2 T6 | ⏭️ 다음 part |
| NFR-5 (긴 경로·유니코드) | — | part1 T2, T4 | ✅ 커버 |
| NFR-6 (한국어 고정) | — | part1 T1(폰트) + 전 task 문구 | ✅ 커버 |
| NFR-7 (settings.json) | — | part2 T5 | ⏭️ 다음 part |
| NFR-8 (워크스페이스 5개) | — | part2 T6 | ⏭️ 다음 part |

**합집합 확인**: active Must FR(1~8, 15~17)이 part1(1~8) + part2(15~17)로 **전수 커버**된다.

## Open Questions

- [x] NFR-2 완화 수준 → **150MB**(NFR-8은 200MB) — PRD 갱신 승인 완료
- [x] 현행 Win32 처리 → **병행 유지 후 교체**(part2 T7에서 제거)
- [x] 행 클릭 미검증 → **선행 수동 확인 완료, 정상 동작**(이식 최대 리스크 해소)
- [x] 이식 범위 → **현행 기능 전부**(Must+Should), 미구현 Could 제외
- [x] plan 분할 → **2개**(part1 탐색 코어 / part2 완성·정리)

## Phase Ledger

- T1~T7 완료 (T7만 Type D, 나머지 Type C — 전부 V-1~V-8 수행·spec·quality 리뷰 통과)

## Progress Log

- **T1 완료** (커밋: checkpoint start `3b60111` → pre-review `fc34a33` → review-fix `4de059c` → 완료): `src/ui/` 신규 UI 계층 + `file_explorer_egui` 진입점. PoC 자산(`icon_tex`·`shell_host`) 이관 후 `src/bin/egui_poc/` 삭제. 현행 Win32 앱 무변경 확인(diff 0).
  - **실측 성과**: wgpu를 빌드에서 빼자 exe가 **10.1MB → 3.87MB**(-62%). 메모리는 131→130MB로 거의 불변 — PoC에서 본 "wgpu 359MB vs glow 131MB"는 **런타임 백엔드 선택**이 만든 차이지 빌드 포함 여부가 아니었다.
  - **리뷰 지적 (실제 결함)**: `eframe::App::ui`가 주는 `Ui`는 배경이 없고, 창 배경은 `App::clear_color`가 결정한다(기본 구현이 `rgba(12,12,12,180)` 하드코딩). `theme::apply_dark`의 `panel_fill`만으로는 배경이 칠해지지 않아 acceptance가 미충족이었다 → `clear_color` 오버라이드 + `CentralPanel::show`로 해소. **이후 모든 UI task가 이 구조를 전제로 한다.**
  - **API 메모**: egui 0.35에서 `CentralPanel::show_inside`는 deprecated이고 `show(ui, ..)`가 정식. `App` trait은 `update`가 아니라 `logic`+`ui` 2메서드.
  - **T2 인계**: PoC 삭제로 `sort_entries` 단위 테스트 3개가 함께 사라졌다 → T2 acceptance에 정렬 테스트를 명시적으로 추가했다.
- **T2 완료** (커밋: start `8d51607` → pre-review `d94f1f9` → review-fix `d1268c3` → 완료): `ui::file_list::FileListView`(가상 스크롤·4열 헤더 정렬·다중 선택·지연 아이콘) + `ui::app::DirLoad`(워커 열거·세대 취소). 정렬 단위 테스트 7개 추가(82 passed).
  - **성능 지표의 함정**: PoC가 잰 "프레임 시간"은 `ui()` 내부 렌더 시간이다. 프레임 **간격**을 재면 vsync 대기가 섞여 p95 26ms처럼 보이지만, 같은 기준(렌더 시간)으로는 **p95 0.63ms**다. 앞으로 성능을 논할 때 어느 지표인지 반드시 명시할 것.
  - **리뷰 지적 (BLOCKER, 실제 기능 파괴 위험)**: 빈 영역 클릭 감지 위젯이 `ScrollArea`의 `inner_rect` **전체**를 덮고 있었다. egui 히트 테스트는 겹칠 때 **나중에 등록된 위젯을 위로** 보므로, 행보다 뒤에 등록된 이 위젯이 행 클릭·더블클릭·우클릭을 전부 가로챌 수 있었다 → 감지 영역을 `콘텐츠 끝~뷰포트 바닥`으로 좁혀 해소. **egui에서 겹치는 위젯을 만들 때는 등록 순서가 곧 우선순위임을 기억할 것.**
  - **결정**: 내림차순에서도 폴더 우선(D13). 현행 Win32 판은 `reverse()`로 폴더 우선이 깨지는데 egui 판은 같은 종류끼리만 뒤집는다 — 두 앱의 의도된 동작 차이이며 part2 T7에서 Win32 판이 사라지면 해소된다.
  - **T3 인계**: "빈 영역 우클릭 → 배경 메뉴"는 목록이 뷰포트를 채우지 않을 때만 가능하다(위 수정의 귀결). T3 acceptance에 이 제약을 명시했고, 스크롤이 생기는 폴더에서 별도 진입점이 필요한지는 T3에서 판단한다.
- **T3·T4 완료** (커밋 `3caefdd`, T4는 start `449f418` → pre-review `0b42a76` → 완료): 셸 메뉴·파일 실행 배선(T3)과 주소창·히스토리·슬래시 경로 수정(T4). 두 task 모두 리뷰 지적 0건. 테스트 85+2.
  - **슬래시 경로 버그 확정·수정**: `\\?\` 접두사는 Win32 경로 정규화를 **건너뛰게** 하므로 그 뒤의 `/`가 구분자로 인식되지 않는다. `to_extended_pattern`에서 접두사를 붙이기 **전에** `/`→`\`로 통일해 해결. 회귀 테스트 3개(패턴 2 + 실제 열거 1). 현행 Win32 앱도 같은 함수를 쓰므로 이 수정으로 함께 고쳐졌다.
  - **pending-커밋 모델**: 경로·히스토리 커서를 열거 **성공 후에만** 옮긴다(`pending_dir`/`PendingNav`). 먼저 옮기면 실패 시 화면과 히스토리가 어긋난다 — 뒤로/앞으로도 같은 이유로 지연 적용한다. 실패해도 목록·경로가 그대로 남아 사용자가 길을 잃지 않는다.
  - **설계 메모**: 셸 메뉴는 `TrackPopupMenuEx` 모달이라 프레임이 그 안에서 멈춘다 — 반드시 `ui()` 그리기 클로저가 **끝난 뒤**에 호출해야 한다(egui 위젯 트리가 부분 구성된 채로 모달 메시지 펌프에 노출되지 않게).

## Next Steps

- part1 승인 후 `implement-task`로 T1부터 실행
- part1 완료 시 중간 점검 → part2(`docs/plans/2026-07-26-egui-migration-part2.md`) 실행
- 이 브랜치(`task/egui-poc`)의 push·PR은 별도 승인 필요. **master에 61개 커밋 미병합 상태**(다크 테마·사이드바 작업 포함)라 병합 전략도 별도 결정 필요
