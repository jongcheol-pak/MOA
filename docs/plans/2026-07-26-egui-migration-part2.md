# egui 전면 이식 — part2 (완성·정리)

**PRD**: `docs/prd.md`
**이전 plan**: `docs/plans/2026-07-26-egui-migration-part1.md`
**전체 목표**: 현행 Win32 UI(FR-1~FR-21)를 egui(eframe/glow)로 전면 이식하고 구 Win32 UI 코드를 제거한다. 이 part2는 part1(탐색 코어) 위에 **워크스페이스·트리·메뉴·감시·세션**을 얹고, NFR을 실측한 뒤 **구 Win32 UI를 제거해 진입점을 승격**하는 마무리다.

## 요구 이해

> 원문: "NFR-2 미달 완화해서 이식해줘"
> part1과 같은 요구의 후반부다. part1이 "탐색이 되는 상태"까지라면, part2는 "현행 앱을 대체할 수 있는 상태"까지다.

이해한 요구:
1. part1에서 빠진 **Should 기능 전부**(트리·감시·세션·단축키·다크 마감)와 **워크스페이스 Must**(FR-15~17)를 egui에서 재현한다.
2. 완화된 **NFR-2(150MB)·NFR-8(200MB)을 실측으로 검증**한다 — 완화는 기준을 없앤 것이 아니라 옮긴 것이므로, 새 기준도 통과 여부를 측정해야 한다.
3. 동작을 확인한 뒤 **구 Win32 UI 코드를 제거**하고 egui 진입점을 `file_explorer.exe`로 승격한다(사용자 선택: 병행 유지 후 교체).

## Goal

`cargo run`으로 뜨는 egui 앱이 현행 Win32 앱을 **기능적으로 대체**하는 상태를 만든다. part2 종료 시점에 `src/app/{window,sidebar,menu,layout_host}.rs`·`src/panel/{panel,file_list,tabs,folder_tree,address_bar}.rs`의 Win32 UI 코드는 제거되고, 순수 로직·`fs`·`ui`만 남는다.

## Investigation Log

> part1 Investigation Log를 승계한다(중복 재조사 금지). 아래는 part2 범위에서 추가로 확인한 사항.

| 확인 사항 | 결과 | 확인 방법 |
|---|---|---|
| 사이드바 시각 명세 정본 | `sidebar.rs` 상수 — 토글 스트립 28px·토글 24px/여백 8px·글리프 `◧` / 헤더 36px·문구 "워크스페이스"·폰트 14px / `+` 24px·여백 8px / 항목 높이 60px·간격 4px·좌우 여백 8px / 강조 바 3px / 아이콘 16px@x=12 / 텍스트 x=38 · 이름 top 12px·15px · 부제 gap 6px·13px / 드래그 임계 8px · 삽입선 2px | `src/app/sidebar.rs` 상수 grep |
| 사이드바 폭 토큰 | `settings.rs`가 소유 — 기본 232 / 최소 160 / 최대 480 | `src/app/settings.rs:18-20` |
| 사이드바 순수 함수 | `item_at(y, scroll, count)`·`clamp_scroll(scroll, count, view_h)`·`drop_index(y, scroll, count)` — 좌표 계산 순수 로직 | `src/app/sidebar.rs:120-145` |
| 워크스페이스 모델 | `WorkspaceList{items, active}` + `add/rename/remove/reorder/set_subtitle/set_active` + `elide_path` — 전부 순수, 재사용 | `src/app/workspace.rs` pub 목록 |
| 세션 스키마 | `Session{version:2, window:WindowState{x,y,w,h,maximized}, sidebar:{width,collapsed}, active_workspace, workspaces:[{name, layout:LayoutNode, panels:[{tabs,active_tab}], active_panel}]}` + `save_session`/`load_session`/`parse_session`/`LayoutNode::{to_shape,from_shape}` — **UI 중립, 무변경 재사용** | `src/app/settings.rs:14-146` |
| 변경 감시 재사용 | `DirWatcher::start(path, tx, notify: Option<HWND>)` — `None`이면 채널만 사용. **수정 불필요** | `src/fs/watcher.rs:47-52` (part1에서 확인) |
| 트리 위젯 | egui 0.35 `containers/collapsing_header.rs` 존재 — 지연 확장(FR-9)을 `CollapsingHeader`의 body 클로저에서 처리 가능 | egui 0.35 소스 목록 |
| 현행 단축키 정본 | `menu.rs::create_accels` — FR-12 목록(Ctrl+T·Ctrl+W·Alt+←/→·F5·Ctrl+\·Ctrl+Shift+\) | `src/app/menu.rs:110` |
| 메뉴 구성 정본 | `menu.rs::{attach_menu, append_workspace_items, update_workspace_enabled, update_close_enabled}` | `src/app/menu.rs` pub 목록 |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| 워크스페이스 목록 모델 | `app::workspace::WorkspaceList` | **재사용** (무수정) |
| 사이드바 좌표 계산 | `app::sidebar::{item_at, clamp_scroll, drop_index}` | **미사용(신규 대체)** — egui `ScrollArea`가 스크롤·히트테스트를 자체 처리하므로 좌표 계산 함수가 필요 없다. 이 함수들은 Win32 직접 그리기 전용이라 T7에서 함께 제거 |
| 경로 축약 | `app::workspace::elide_path` | **재사용** (부제 표시) |
| 세션 저장·복원 | `app::settings::{save_session, load_session, Session, LayoutNode}` | **재사용** (무수정) |
| 변경 감시 | `fs::watcher::DirWatcher` | **재사용** (무수정, `notify: None`) |
| 트리 노드 열거 | `folder_tree.rs`의 열거 로직은 TreeView 핸들과 얽혀 있음 | **신규** — `fs::enumerate_dir`로 하위 폴더만 걸러 `CollapsingHeader`에 태운다 |
| `WorkspaceSidebar`·`FolderTreeView`·`MenuBar` | 없음(Win32판은 시스템 컨트롤·직접 그리기 래퍼) | **신규** |
| 의존성 | 신규 없음 | part1에서 확정된 eframe/glow만 사용 |

## Impact Analysis

### 3-A. 영역별 해당 여부

| 영역 | 해당 | 근거 |
|---|---|---|
| DI 컨테이너 등록 | 비해당 | DI 미사용 |
| 이벤트 핸들러·옵저버 | **해당** | 감시 스레드 → 채널 → 프레임 폴링(T4) |
| 직렬화/역직렬화 | **해당** | 세션 v2 저장·복원 배선(T5) — 스키마는 무변경 |
| 마이그레이션 | 비해당 | v2 그대로. **v1 파일 초기화 폴백도 기존 규칙 유지**(FR-20) |
| 권한·보안 | **해당** | 트리 확장 시 접근 거부 폴더(T1), 감시 실패(T4) |
| 캐싱·메모이제이션 | **해당** | 워크스페이스 지연 생성(NFR-8) — 최초 선택 시에만 상태 생성 |
| 멀티 스레드·비동기 | **해당** | 트리 확장 워커(T1), 감시 스레드(T4) |
| 로깅·메트릭 | 비해당 | GUI 앱 — `println!` 금지 |

### 3-B. 변경 대상 심볼의 파급

| 변경 대상 | 변경 내용 | 호출자·참조 (전수) | 갱신 task 필요 |
|---|---|---|---|
| `src/main.rs` | Win32 진입점 → **egui 진입점으로 교체** | 진입점(호출자 없음) | **T7** |
| `src/app/{window,sidebar,menu,layout_host}.rs` | **파일 삭제** | `window.rs`→`main.rs`·`app/mod.rs` / `sidebar,menu,layout_host`→`window.rs`만 | **T7** — 삭제 전 `app/mod.rs`에서 선언 제거 |
| `src/panel/{panel,folder_tree,address_bar}.rs` | **파일 삭제** | `panel.rs`→`layout_host.rs`(함께 삭제) / `folder_tree,address_bar`→`panel.rs`만 | **T7** |
| `src/panel/file_list.rs` | **부분 삭제** — `FileList`(ListView 래퍼)·`apply_item_count` 제거, `SortKey`·`compare_entries`·`format_*`는 **존치**(ui가 사용) | `FileList`→`panel.rs`만 / 순수 함수→`ui::file_list` | **T7** — 파일을 지우지 않고 Win32 부분만 제거 |
| `src/panel/tabs.rs` | **부분 삭제** — WC_TABCONTROL 래퍼 제거, `TabsModel`·`TabState`·`tab_title`·`CloseOutcome` 존치 | 래퍼→`panel.rs`만 / 모델→`ui::tabs` | **T7** |
| `src/app/theme.rs` | **삭제 검토** — COLORREF 상수는 ui가 안 쓰지만 `enable_dark_mode`(DWM 다크 타이틀바)는 egui 창에도 유효할 수 있음 | `window.rs`·`sidebar.rs`·`panel/*`(전부 삭제 대상) | **T7** — 타이틀바 다크가 winit 기본으로 되는지 확인 후 결정 |
| `Cargo.toml` | `[[bin]] file_explorer_egui` 제거(main.rs로 승격) | 빌드 설정 | **T7** |
| `tests/` | 삭제된 Win32 심볼을 참조하는 테스트 정리 | 통합 테스트 | **T7** — 순수 로직 테스트는 전부 존치 |
| `AGENTS.md` | Repository Structure에 `src/ui/` 추가, `app/`·`panel/` 설명 갱신 | 문서 | **T7** (`record-project-fact` 경유) |

## Decisions

- **D1 (워크스페이스 지연 생성)**: 워크스페이스별 상태(`LayoutTree` + 패널들)는 **최초 선택 시에만 생성**한다. NFR-8(5개 200MB)이 이를 전제로 한 요구다(PRD 문구 "최초 선택 시 지연 생성"). 비활성 워크스페이스는 egui에서 그리지 않으므로 Win32판의 "생성 후 숨김"과 달리 **창 자원 자체가 없다** — 이 점에서 egui가 유리하다.
- **D2 (사이드바 구현)**: egui `SidePanel::left`를 쓴다. 사이드바는 좌측 고정 도킹이라 자유 분할(D9/part1)과 달리 egui 컨테이너 의미와 정확히 일치하고, 폭 조절(FR-19)·접기가 `resizable`·표시 토글로 바로 된다. 항목은 `ScrollArea` + 직접 그리기(2줄 카드는 표준 위젯으로 표현 불가). **이로써 Deferred 대장의 "사이드바 가상 스크롤·커스텀 다크 스크롤바"가 자동 해소**된다.
- **D3 (시각 재현 기준)**: 사이드바는 현행 `sidebar.rs` 상수를 정본으로 **같은 치수·색을 재현**한다(`## 시각 요소 분해` 표). 현행 화면을 사용자가 승인한 상태이므로 이식에서 임의로 바꾸지 않는다.
- **D4 (인라인 편집)**: 워크스페이스 이름 변경(FR-18)은 egui `TextEdit`을 항목 자리에 인라인으로 띄운다. **Deferred 대장의 "인라인 편집 EDIT 다크 스타일링"이 자동 해소**된다(Win32 EDIT의 밝은 배경 제약이 사라짐).
- **D5 (트리 지연 확장)**: `CollapsingHeader`의 body가 처음 열릴 때 그 폴더의 하위 **1단계만** 워커 스레드로 열거한다(현행 D14 계승 — UI 무정지). 드라이브 루트만 미리 나열.
- **D6 (트리→목록 동기화 방향)**: 현행대로 **단방향**(트리 선택 → 목록 이동)을 유지한다. 양방향은 deferred 대장 항목이며 이식 범위를 넘는다(목록 이동마다 트리 경로를 펼치는 별도 설계가 필요). Deferred에 유지.
- **D7 (감시 통지)**: `DirWatcher::start(.., notify: None)`으로 채널만 쓰고, 프레임마다 `try_recv`로 확인해 변경 시 재열거 + `request_repaint`. 300ms 디바운스는 기존 구현이 이미 수행한다.
- **D8 (창 상태 저장)**: 창 위치·크기·최대화는 winit 뷰포트 정보로 얻어 기존 `WindowState`에 그대로 담는다. eframe의 `persistence` feature는 **쓰지 않는다** — 자체 `settings.json`(NFR-7)이 정본이며 두 저장 경로가 생기면 안 된다.
- **D9 (제거 순서)**: T7은 **의존 역순으로** 제거한다 — 진입점 교체(main.rs) → `window.rs` → 그 하위(`sidebar/menu/layout_host`, `panel/panel.rs`) → 리프(`folder_tree`·`address_bar`·`file_list`/`tabs`의 Win32 부분). 각 단계마다 `cargo build`로 잔여 참조를 확인한다.
- **D12 (사이드바 계약 조정, T2에서 결정)**: Design ②의 두 곳을 실제 구현에서 바꿨다. ① 뷰 보관을 `HashMap<usize, _>`가 아니라 **`HashMap<WorkspaceId, _>`** — 순서 변경·삭제로 인덱스가 흔들려 뷰가 엉뚱한 워크스페이스에 붙는다. ② `SidebarAction::SetWidth`를 두지 않는다 — egui `Panel`이 폭을 스스로 관리·클램프하므로 그린 뒤 `response.rect.width()`로 읽으면 되고, 왕복시키면 같은 값이 두 곳에 생긴다. ③ `show`가 `Option<SidebarAction>`이 아니라 **`Vec<SidebarAction>`**을 돌려준다 — 이름 편집 커밋과 다른 항목 클릭이 한 프레임에 겹치면 하나가 조용히 사라졌다(T2 spec 리뷰 M2).
- **D13 (세션 변환 계약, T5에서 결정)**: Design ②의 `to_session(app: &ExplorerApp, ..)`·`restore(..) -> RestoredState` 대신 **UI 비의존 중간 표현**(`WorkspaceState`·`PanelTabs`)을 두고 그것을 주고받는다. `ExplorerApp`을 직접 받으면 변환이 UI 타입에 묶여 단위 테스트로 왕복을 검증할 수 없다 — Design ①이 요구한 "순수 변환·테스트 대상"을 지키려면 이 형태여야 한다.
- **D11 (트리 토글 진입점, T1에서 결정)**: 트리 표시 토글을 **패널 상태 줄의 버튼**으로 둔다(현행 Win32 판은 메뉴에만 있다). FR-9가 "패널별 토글"이고 메뉴 명령은 활성 패널 하나만 대상으로 하므로, 각 패널에서 직접 켜고 끄는 편이 자연스럽다. T3에서 메뉴에 "폴더 트리" 명령이 추가돼도 이 버튼은 **유지**한다.
- **D10 (제거 게이트)**: T7 진입 전에 **T6 실측이 통과**해야 한다. 실측이 NFR을 못 넘긴 상태에서 구현을 지우면 되돌릴 곳이 사라진다(사용자의 "병행 유지 후 교체" 선택의 취지). T6 미통과 시 T7은 실행하지 않고 보고한다.

## Tasks

- [x] T1 폴더 트리 (Type C)
- [x] T2 워크스페이스 사이드바 (Type D)
- [x] T3 메뉴바·단축키 (Type C)
- [x] T4 디렉터리 변경 감시 (Type C)
- [x] T5 세션 저장·복원 (Type C)
- [ ] T6 NFR 실측·검증 (Type B)
- [ ] T7 구 Win32 코드 제거·진입점 승격 (Type D)

### T1. 폴더 트리 (Type C)

- **PRD**: FR-9(패널별 토글 표시·지연 확장)
- **Files**: `src/ui/tree.rs`(신규), `src/ui/panel.rs`
- **Design**: ① 배치 — 트리 위젯을 `ui/tree.rs`에 격리, 패널이 좌측 영역에 배치 ② 신규 심볼 — `ui::tree::FolderTreeView`(확장 상태·자식 캐시·워커 수신부) / `FolderTreeView::show(&mut self, ui, ctx) -> Option<PathBuf>`(선택된 경로 반환) / `ui::tree::drive_roots() -> Vec<PathBuf>` ③ 의존 방향 — `fs::enumerate::enumerate_dir` 재사용, 패널이 트리를 소유 ④ 비추상화 — 범용 트리 컴포넌트로 일반화하지 않는다(폴더 전용). 가상화도 하지 않는다(펼친 노드만 그려지므로 불필요)
- **내용**: `CollapsingHeader`로 계층 표시. 처음 펼칠 때만 하위 1단계를 워커로 열거(D5), 결과는 노드별 캐시. 선택 시 목록을 그 폴더로 이동(단방향 — D6). 패널당 토글로 표시/숨김
- **Acceptance**:
  - 드라이브 루트(`C:`, `D:` …)가 나열되고 펼치면 하위 폴더가 나온다
  - **펼치기 중에도 UI가 멈추지 않는다**(대용량 폴더에서 확인)
  - 트리에서 폴더를 선택하면 **같은 패널의 목록이 그 폴더로 이동**한다
  - 토글로 트리를 숨기면 목록이 그 공간을 차지한다
  - 트리 색이 다크 팔레트(`SURFACE_BG`·`TREE_LINE`)를 따른다
- **Edge Cases**: 접근 거부 폴더 펼치기 → 빈 자식 + 조용히 무시 / 하위 폴더 0개 → 펼침 화살표 없음 / 네트워크 드라이브 응답 지연 → 워커라 UI 무정지 / 펼친 상태에서 폴더가 삭제됨 / 드라이브 목록 변경(USB 착탈) — 갱신은 하지 않음(현행과 동일) / 순환 심볼릭 링크 → 깊이 제한 없이 사용자 조작에만 반응하므로 무한 확장 없음
- **Halt Forecast**: `CollapsingHeader`가 지연 로딩과 맞지 않으면(body 클로저가 접힌 상태에서도 호출되는 등) → `ui.collapsing` 대신 직접 들여쓰기 + 화살표 버튼으로 구성(설계 대안, 승인 불요)

### T2. 워크스페이스 사이드바 (Type D)

- **PRD**: FR-15(2줄 카드·고정 다크), FR-16(추가·자동 이름·즉시 편집), FR-17(전환·독립 상태), FR-18(이름 변경·삭제·순서), FR-19(접기·폭 조절)
- **Files**: `src/ui/sidebar.rs`(신규), `src/ui/app.rs`
- **Design**: ① 배치 — 사이드바 그리기·입력을 `ui/sidebar.rs`에, 워크스페이스별 탐색 상태는 `ui::app::ExplorerApp`가 `HashMap<usize, WorkspaceView>`로 소유(지연 생성 — D1) ② 신규 심볼 — `ui::sidebar::WorkspaceSidebar`(스크롤·드래그·편집 상태) / `WorkspaceSidebar::show(&mut self, ui, list: &WorkspaceList) -> Option<SidebarAction>` / `enum SidebarAction { Select(usize), Add, Rename(usize, String), Remove(usize), Reorder(usize, usize), ToggleCollapse, SetWidth(i32) }` / `ui::app::WorkspaceView{ tree: LayoutTree, panels: HashMap<PanelId, PanelState>, active: PanelId }` ③ 의존 방향 — `app::workspace::{WorkspaceList, elide_path}`·`app::settings::{SIDEBAR_*}` 재사용 ④ 비추상화 — 사이드바를 범용 리스트 컴포넌트로 만들지 않는다. 드래그 정렬도 egui `drag_started/drag_stopped`로 직접 처리(DnD 프레임워크 미도입)
- **내용**: `SidePanel::left`(resizable, 폭 클램프 160~480) + 헤더("워크스페이스" + `+`) + `ScrollArea` 항목 목록. 항목은 2줄 카드(이름 15px / 부제 = 활성 탭 경로 `elide_path` 13px) + 활성 강조 바 3px. F2·컨텍스트 메뉴로 인라인 편집(D4), 삭제(마지막 1개 불가), 드래그 순서 변경(임계 8px·삽입선 2px)
- **Acceptance**:
  - 사이드바가 좌측에 표시되고 **항목이 2줄**(이름 / 활성 탭 폴더 경로)로 보인다
  - `+` 클릭 → "워크스페이스 N"이 추가되고 **즉시 인라인 편집 상태**가 된다
  - 항목 선택 → 우측 탐색기가 그 워크스페이스로 전환되고, **각 워크스페이스가 독립 분할·탭·히스토리**를 유지한다(A에서 분할해도 B는 그대로)
  - F2로 이름 변경, 삭제(마지막 1개는 비활성), 드래그로 순서 변경이 동작한다
  - 접기 토글·스플리터 폭 조절이 되고 160~480px로 클램프된다
  - **비활성 워크스페이스는 최초 선택 전까지 상태가 생성되지 않는다**(지연 생성 — 코드 확인 + T6 메모리 실측)
  - 시각이 `## 시각 요소 분해` 표와 일치한다
- **Edge Cases**: 워크스페이스 1개일 때 삭제 시도 / 이름 빈 문자열 커밋 → 이전 이름 유지 / 매우 긴 이름·경로 → 말줄임 / 활성 워크스페이스 삭제 → 인접 항목으로 전환 / 드래그를 자기 자리에 놓기 / 항목 수십 개 → `ScrollArea`가 처리 / 편집 중 다른 항목 클릭 → 커밋 후 전환 / 접힌 상태에서 항목 추가
- **Halt Forecast**: `SidePanel`의 resizable 폭이 세션 저장값과 어긋나 진동하면 → 폭을 앱 상태가 소유하고 `SidePanel`에 매 프레임 지정하는 방식으로 전환 / 워크스페이스 5개 메모리가 200MB를 넘으면 T6에서 실측 보고(자동 기각 없음)

### T3. 메뉴바·단축키 (Type C)

- **PRD**: FR-12(단축키), FR-1·FR-2·FR-16의 명령 진입점, FR-21(메뉴 다크)
- **Files**: `src/ui/menu.rs`(신규), `src/ui/app.rs`, `src/ui/panel.rs`·`src/ui/sidebar.rs`(구현 중 추가 — 명령이 닿을 공개 진입점)
- **Design**: ① 배치 — 메뉴 정의·단축키 처리를 `ui/menu.rs`에 ② 신규 심볼 — `ui::menu::show_menu_bar(ui, state: MenuState) -> Option<Command>` / `ui::menu::poll_shortcuts(ctx) -> Option<Command>` / `enum Command { NewTab, CloseTab, Back, Forward, Refresh, SplitH, SplitV, ClosePanel, NewWorkspace, ToggleTree, ToggleSidebar }` / `struct MenuState{ can_close_panel: bool, can_remove_workspace: bool }` ③ 의존 방향 — 명령을 값으로 반환하고 실행은 `ExplorerApp`가 한다(메뉴가 앱 상태를 직접 조작하지 않음) ④ 비추상화 — 명령 디스패처·액션 레지스트리를 만들지 않는다. `enum` + `match` 하나로 충분
- **내용**: egui `MenuBar`로 상단 메뉴 + `ctx.input`으로 단축키 감지. 현행 `create_accels` 목록 그대로: Ctrl+T 새 탭 / Ctrl+W 탭 닫기 / Alt+← Alt+→ 히스토리 / F5 새로고침 / Ctrl+\ 좌우 분할 / Ctrl+Shift+\ 상하 분할. 마지막 패널일 때 "패널 닫기" 비활성, 워크스페이스 1개일 때 "삭제" 비활성
- **Acceptance**:
  - 메뉴바가 표시되고 각 항목이 대응 동작을 수행한다
  - **FR-12의 단축키 6종이 모두 동작**한다(각각 확인)
  - 마지막 패널일 때 "패널 닫기"가 **비활성으로 보인다**, 워크스페이스 1개일 때 "삭제"도 마찬가지
  - **메뉴 팝업 배경이 다크**다(현행 Win32판의 best-effort 제약이 사라진 것 — FR-21 개선)
  - 주소창에 포커스가 있을 때 단축키가 텍스트 입력을 가로채지 않는다
- **Edge Cases**: 텍스트 입력 중 Ctrl+W(탭이 닫히면 안 됨 — 포커스 확인) / 인라인 편집 중 F2 / 단축키 조합이 OS에 선점된 경우 / 메뉴 열린 채 창 크기 변경 / F5를 열거 중에 연타 → 세대 번호가 처리
- **Halt Forecast**: egui의 `consume_shortcut`이 TextEdit 포커스와 충돌해 입력이 먹히면 → 포커스 상태를 앱이 판정해 단축키 처리를 건너뛰는 방식으로 전환(설계 대안)

### T4. 디렉터리 변경 감시 (Type C)

- **PRD**: FR-10(표시 중 폴더 자동 새로고침)
- **Files**: `src/ui/panel.rs`, `src/ui/file_list.rs`(구현 중 정정 — 선택 유지가 여기 있다. `app.rs`는 이미 `panel.poll()`을 부르고 있어 변경 불필요했다)
- **Design**: ① 배치 — 패널이 자기 폴더의 `DirWatcher`를 소유(패널 독립 원칙) ② 신규 심볼 — `PanelState::watch(&mut self, path)`(기존 감시 중단 후 새 경로 감시 시작) / `PanelState::poll_watch(&mut self, ctx)`(`try_recv` → 재열거) ③ 의존 방향 — `fs::watcher::DirWatcher` 재사용(무수정, `notify: None` — D7) ④ 비추상화 — 감시 이벤트 버스를 만들지 않는다. 패널마다 채널 하나
- **내용**: 폴더 이동 시 감시 대상 교체. 프레임마다 채널 확인 → 변경 감지 시 재열거 + `request_repaint`. 300ms 디바운스는 기존 구현이 수행
- **Acceptance**:
  - 표시 중인 폴더에 **탐색기로 파일을 만들면 목록에 자동으로 나타난다**(삭제·이름 변경도 동일)
  - 폴더를 이동하면 이전 폴더 감시가 중단되고 새 폴더가 감시된다
  - 감시 갱신이 **선택·스크롤 위치를 파괴하지 않는다**(가능한 범위에서 유지)
  - 통합 테스트: 임시 폴더에 파일 생성 → 채널 수신 확인(기존 `watcher` 테스트 방식 계승)
- **Edge Cases**: 감시 대상 폴더가 삭제됨 → 감시 종료, 목록은 오류 표시 / 네트워크 경로(감시 미지원 가능) → 실패 시 조용히 비활성 / 대량 변경(수천 파일 동시 생성) → 디바운스가 흡수 / 앱 종료 시 감시 스레드 정리 / 같은 폴더를 여러 패널이 감시(각자 독립 — 중복 허용)
- **Halt Forecast**: 감시 스레드가 종료 시 정리되지 않아 프로세스가 남으면 → 정지 이벤트(기존 `CreateEventW` 경로) 배선을 확인해 수정

### T5. 세션 저장·복원 (Type C)

- **PRD**: FR-11(레이아웃·탭·창 위치), FR-20(워크스페이스·사이드바 상태, v2 스키마·v1 폴백), NFR-7(저장 위치)
- **Files**: `src/ui/app.rs`, `src/ui/session.rs`(신규)
- **Design**: ① 배치 — 앱 상태 ↔ `Session` 변환을 `ui/session.rs`에 격리(순수 변환 — 테스트 대상) ② 신규 심볼 — `ui::session::to_session(app: &ExplorerApp, window: WindowState) -> Session` / `ui::session::restore(session: Session) -> RestoredState` ③ 의존 방향 — `app::settings::{Session, save_session, load_session, LayoutNode}` 재사용(무수정) ④ 비추상화 — eframe `persistence`를 쓰지 않는다(D8). 저장 포맷 추상화도 없음
- **내용**: 시작 시 `load_session` → 워크스페이스 목록·활성·사이드바 상태·창 위치 복원(활성 워크스페이스만 실제 생성 — D1). 종료 시 `save_session`. 창 위치·크기·최대화는 winit 뷰포트에서 취득(D8)
- **Acceptance**:
  - 앱을 종료하고 다시 켜면 **분할 레이아웃·탭 구성·워크스페이스 목록·활성 워크스페이스·사이드바 폭/접힘·창 위치가 복원**된다
  - 저장 위치가 `%APPDATA%\FileExplorer\settings.json`이다(NFR-7)
  - **v1 세션 파일은 기존 규칙대로 초기화 폴백**한다(승격하지 않음 — FR-20)
  - 단위 테스트: 앱 상태 → `Session` → 앱 상태 왕복이 동일 구조를 낳는다(레이아웃 트리·탭 목록)
  - 손상된 JSON·존재하지 않는 파일 → 기본 상태로 시작하고 크래시하지 않는다
- **Edge Cases**: 저장된 경로가 사라짐(외장 드라이브) → 홈 폴더 폴백 / 저장된 창 위치가 화면 밖(모니터 구성 변경) → 화면 안으로 보정 / 최대화 상태 저장 / 워크스페이스 0개인 세션 파일 → 기본 1개 생성 / `%APPDATA%` 쓰기 실패 → 조용히 무시(앱은 정상 종료) / 종료 중 저장과 감시 스레드 정리 순서
- **Halt Forecast**: winit에서 "최대화 이전의 일반 크기"를 얻지 못하면 → 최대화 전 크기를 앱이 추적해 저장하는 방식으로 전환

### T6. NFR 실측·검증 (Type B)

- **PRD**: NFR-1(1초), NFR-2(**150MB**), NFR-3(10만 파일), NFR-4(DPI), NFR-5(긴 경로·유니코드), NFR-8(**워크스페이스 5개 200MB**)
- **Files**: 이 plan의 결과 표, `notes.md`
- **내용**: release 빌드로 현행 Win32 앱과 나란히 측정해 표로 기록. 콜드 스타트(2회차 포함) / Working Set·Private(패널 2개 기준) / 10만 파일 스크롤 p95 / 워크스페이스 5개 방문 후 메모리 / exe 크기. DPI는 서로 다른 배율 모니터 간 이동으로 확인, 긴 경로·유니코드는 테스트 폴더로 확인
- **Acceptance**:
  - 표에 **egui 앱과 현행 Win32 앱 수치가 모두** 기재되고 각 수치에 측정 방법이 1줄씩 명시된다
  - NFR-1·2·3·8 각각에 **충족/미달 판정**이 명시된다(미달 시 자동 기각하지 않고 실측값과 원인을 보고 — PoC와 같은 원칙)
  - NFR-4(DPI)·NFR-5(긴 경로·유니코드)는 HUMAN-VERIFY 항목으로 구분해 기록한다
  - **NFR-5는 실제로 확인한다** — part1이 `\?\` 접두사 사용으로 "코드상 가능"까지만 확인했고 260자 초과·유니코드 폴더를 만들어 열거·표시를 본 기록이 없다(part1 F-7 리뷰 m4)
- **Edge Cases**: 첫 실행 셰이더·폰트 캐시로 콜드 스타트 과대 측정 → 2회차 이후 값 병기 / 측정 중 백그라운드 프로세스 간섭 → 3회 측정 중앙값
- **Halt Forecast**: **NFR-2(150MB) 또는 NFR-8(200MB) 미달** → T7(구 코드 제거)을 실행하지 않고 실측값·원인 분해와 함께 보고한다(D10). 되돌릴 곳을 남긴 채 사용자가 판단 — 완화폭 재조정 / 폰트 서브셋 등 절감 / 이식 중단 중 선택

### T7. 구 Win32 코드 제거·진입점 승격 (Type D)

- **PRD**: 요구 변경 없음 — 구현 정리
- **선행 조건**: **T6 실측 통과**(D10). 미통과 시 이 task는 실행하지 않는다
- **Files**: `README.md`(갱신 — 아래 참조), `src/main.rs`(교체), `src/app/{window,sidebar,menu,layout_host}.rs`(삭제), `src/app/theme.rs`(삭제 검토), `src/app/mod.rs`, `src/panel/{panel,folder_tree,address_bar}.rs`(삭제), `src/panel/{file_list,tabs}.rs`(Win32 부분 제거), `src/panel/mod.rs`, `src/bin/egui_app/`(제거 — main.rs로 승격), `Cargo.toml`, `tests/`, `AGENTS.md`
- **Design**: ① 배치 — `src/ui/`가 유일한 UI 계층이 되고 `app/`은 순수 로직(layout·workspace·settings)만, `panel/`은 순수 모델(tabs 모델·history·정렬/포맷)만 남는다 ② 신규 심볼 — 없음(제거 작업) ③ 의존 방향 — 제거 후 `main.rs` → `ui` → `app`(순수)·`panel`(순수)·`fs` ④ 비추상화 — 제거하면서 남는 순수 로직을 새 모듈로 재배치하지 않는다(파일 이동은 diff를 키우고 이식 검증과 섞인다 — 필요하면 별도 작업)
- **내용**: D9의 의존 역순으로 제거하며 단계마다 `cargo build`로 잔여 참조 확인. `main.rs`를 egui 진입점으로 교체하고 `[[bin]] file_explorer_egui` 제거(기본 `cargo run`이 egui 앱). `theme.rs`는 `enable_dark_mode`(DWM 다크 타이틀바)가 egui 창에 필요한지 확인 후 존치/삭제 결정. `tests/`에서 삭제된 심볼 참조 정리(순수 로직 테스트는 전부 존치). AGENTS.md Repository Structure 갱신
- **Acceptance**:
  - `cargo run`이 **egui 앱**을 띄운다(`file_explorer.exe`)
  - `cargo build`·`cargo test`·`cargo clippy --all-targets -- -D warnings`·`cargo fmt --check` **전부 통과**
  - `grep -r "HWND" src/app src/panel` → **UI 창 관련 잔여 참조 0**(순수 로직 파일의 주석 제외)
  - 순수 로직 테스트(레이아웃·워크스페이스·세션·탭·히스토리·정렬·포맷)가 **전부 존치되고 통과**한다
  - part1+part2의 모든 FR 동작이 제거 후에도 유지된다(회귀 확인)
  - AGENTS.md Repository Structure에 `src/ui/`가 반영된다
  - **README.md가 갱신된다** — 현재 "GUI 프레임워크 없이 구현", "단일 exe 약 380KB", 2026-07-24 성능 실측치가 모두 적혀 있어 진입점 승격과 동시에 stale해진다(part1 F-7 리뷰 m5)
- **Edge Cases**: `file_list.rs`·`tabs.rs`의 부분 삭제에서 순수 함수까지 지우지 않도록 주의 / `mod.rs` 선언 누락으로 인한 컴파일 실패 / 삭제한 심볼을 참조하는 테스트 / `theme.rs`의 `enable_dark_mode`가 여전히 필요한 경우 / 삭제 후 미사용 `windows` crate feature가 남음(Cargo.toml 정리 여부 — 이번엔 건드리지 않음, `fs`가 여전히 사용)
- **Halt Forecast**: **대량 파일 삭제**(→ 위임 불가 Halt. 사전 승인 대상이 아니라 실행 시점에 삭제 목록을 제시하고 별도 승인) / 제거 후 회귀가 발견되면 되돌리고 보고

## 시각 요소 분해 (사이드바 — T2)

> 기준: 현행 `src/app/sidebar.rs` 상수(사용자 승인된 화면). 이식에서 임의 변경하지 않는다(D3).

| 요소 | 속성 | 디자인 값 | 확인 방법 |
|---|---|---|---|
| 사이드바 | 기본/최소/최대 폭 | 232 / 160 / 480 px | `settings.rs:18-20` |
| 사이드바 | 배경 | `COLOR_BG` 0x1B1B1B | `sidebar.rs:105` |
| 토글 스트립 | 높이 | 28 px | `sidebar.rs:68` |
| 토글 버튼 | 크기 / 여백 / 글리프 | 24 px / 8 px / `◧` | `sidebar.rs:70-73` |
| 헤더 | 높이 / 문구 / 폰트 | 36 px / "워크스페이스" / 14 px | `sidebar.rs:74-75,92` |
| `+` 버튼 | 크기 / 여백 | 24 px / 8 px | `sidebar.rs:77-78` |
| 항목 카드 | 높이 / 간격 / 좌우 여백 | 60 px / 4 px / 8 px | `sidebar.rs:79-83` |
| 활성 강조 바 | 폭 | 3 px | `sidebar.rs:84` |
| 항목 아이콘 | 크기 / x 위치 | 16 px / 12 px | `sidebar.rs:85-86` |
| 항목 텍스트 | x 위치 | 38 px | `sidebar.rs:87` |
| 이름(1줄) | top / 폰트 | 12 px / 15 px | `sidebar.rs:88-89` |
| 부제(2줄) | 간격 / 폰트 / 내용 | 6 px / 13 px / 활성 탭 경로(`elide_path`) | `sidebar.rs:90-91`, `workspace.rs:212` |
| 드래그 | 임계 / 삽입선 두께 | 8 px / 2 px | `sidebar.rs:96,98` |

**V-9 시각 충실도 결과 (T2)**: 위 13개 행 전부 `src/ui/sidebar.rs`의 상수·그리기 호출과 1:1 대조 완료(spec 리뷰 항목 I에서 값 불일치 0). 다만 **실제 렌더 결과 확인은 전 행 `⏳ 미확인`** — 데스크톱 창이라 자동 캡처가 불가하다. **F-8에서 사용자 화면 확인으로 마감한다**(완료 선언 전 게이트).

## 사전 승인 항목 (일괄 승인 대상)

- `src/ui/` 신규 파일 생성 (tree·sidebar·menu·session)
- `src/main.rs` 교체 및 `Cargo.toml`의 `[[bin]] file_explorer_egui` 제거 (T7 — 진입점 승격)
- `AGENTS.md` Repository Structure 갱신 (`record-project-fact` 경유)
- 성능 측정용 대량 파일 폴더 생성·삭제 (**시스템 임시 폴더 한정**)
- 각 task 완료 시 로컬 작업 브랜치 commit

## 불가피한 Halt (위임 불가)

- push · PR · main 병합 · 태그 · 릴리즈
- **T7의 구 Win32 소스 파일 삭제** — 되돌리기 어려운 대량 삭제라 실행 시점에 **삭제 대상 목록을 제시하고 별도 승인**받는다(사전 승인에 포함하지 않음)
- **T6 실측이 NFR-2/NFR-8을 미달**하는 경우 — T7로 진행하지 않고 보고 (D10)
- `docs/prd.md` 추가 변경
- 의존성 추가

## Out of Scope

- FR-13(숨김 파일 토글)·FR-14(분할 프리셋) — Could, 이번 이식 범위 밖
- 트리→목록 **양방향** 동기화 — 현행도 단방향(D6). Deferred 유지
- PRD Out of Scope 전부(파일 작업 UI·DnD·검색·즐겨찾기·가상 폴더·테마 전환 UI·다국어)
- 순수 로직의 모듈 재배치(`app/`·`panel/` 구조 개편) — T7은 삭제만, 이동은 별도 작업

## Deferred / Follow-up

- **[SUGGEST] 전역 공유 자원 묶기 — 이번엔 묶지 않음 (T3에서 재평가)**: part1이 T3에서 정리하기로 미룬 항목이나, T3가 명령을 `ExplorerApp`에서 직접 처리해 `PanelState::show`(4개)·`splitter::show_layout`(7개)의 인자가 **하나도 늘지 않았다**. 인자가 늘지 않는 한 묶어도 호출부 표기만 줄고 내부에서 다시 필드를 분해해야 해(부분 borrow) 실익이 없다 — T4·T5에서 실제로 늘면 그때 다시 본다 (T3 quality 리뷰 소견)
- **[MINOR] 사이드바 마지막 항목 뒤 여백** — `show_items`가 항목마다 `add_space(ITEM_GAP)`을 붙여 마지막 카드 뒤에도 4px가 남는다. 스크롤 영역이라 화면상 문제는 없으나 "항목 사이 간격"이라는 의도와 미세하게 어긋난다 (T2 quality 리뷰 m2)
- **트리→목록 양방향 동기화** (deferred 대장 [2026-07-23]) — 이식 후에도 미해결로 유지
- **한글 폰트 서브셋** — 메모리를 더 줄여야 할 때의 첫 후보(약 27MB 기여)
- **`app/`·`panel/` 모듈 재배치** — 이식 후 남는 순수 로직을 더 적절한 위치로 옮길지(예: `core/`) 별도 검토
- `debug-2026-07-24-dark-ownerdraw.md` 루트 위치 정리 — `docs/`로 이동할지 결정
- **master 미병합 61커밋** — 다크 테마·사이드바·PoC·이식 작업이 모두 `task/*` 브랜치에 쌓여 있다. 병합 전략을 사용자와 결정해야 한다

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|---|---|---|---|
| FR-15 | Must | part2 T2 | ✅ 커버 |
| FR-16 | Must | part2 T2 | ✅ 커버 |
| FR-17 | Must | part2 T2 | ✅ 커버 |
| FR-1~FR-8 | Must | part1 T2~T7 | ✅ 이전 part 기구현 |
| FR-9 | Should | part2 T1 | ✅ 커버 |
| FR-10 | Should | part2 T4 | ✅ 커버 |
| FR-11 | Should | part2 T5 | ✅ 커버 |
| FR-12 | Should | part2 T3 | ✅ 커버 |
| FR-18 | Should | part2 T2 | ✅ 커버 |
| FR-19 | Should | part2 T2 | ✅ 커버 |
| FR-20 | Should | part2 T5 | ✅ 커버 |
| FR-21 | Should | part1 T1 + part2 T1~T3(트리·사이드바·메뉴 다크) | ✅ 커버 |
| FR-13 | Could | (없음) | 명시적 제외 — Out of Scope |
| FR-14 | Could | (없음) | 명시적 제외 — Out of Scope |
| NFR-1·2·3 | — | part2 T6 (part1 T1·T2 중간 실측) | ✅ 커버 |
| NFR-4 (DPI) | — | part2 T6 | ✅ 커버 |
| NFR-5 | — | part1 T2·T4 + part2 T6 확인 | ✅ 커버 |
| NFR-6 (한국어) | — | 전 task 문구 | ✅ 커버 |
| NFR-7 (settings.json) | — | part2 T5 | ✅ 커버 |
| NFR-8 (5개 200MB) | — | part2 T6 (D1 지연 생성이 전제) | ✅ 커버 |

**합집합 확인**: active Must FR(1~8, 15~17)이 part1(1~8) + part2(15~17)로 **전수 커버**된다. Should도 두 part 합집합으로 전수 커버.

## Open Questions

- [x] part1과 동일한 4개 결정(NFR-2 150MB / 병행 유지 후 교체 / 행 클릭 선행 확인 / 현행 기능 전부) — 해결됨
- [x] plan 분할 → **2개**로 확정

## Phase Ledger

- T1~T5 완료 (part1 완료 후 진입)

## Progress Log

- **T1·T2 완료** (커밋 `fea0660`·T2는 pre-review `a6ba163` → review-fix `78b5c8b` → 완료): 폴더 트리(`ui/tree.rs`)와 워크스페이스 사이드바(`ui/sidebar.rs`) + `ExplorerApp` 구조 개편(`WorkspaceView` 도입).
  - **egui 지연 확장 확인**: `CollapsingState::show_body_unindented`는 `openness <= 0.0`이면 본문 클로저를 **호출하지 않는다**(egui 0.35 소스 확인) — 본문 클로저가 곧 "처음 펼쳐진 순간"이라 D5의 지연 열거가 별도 장치 없이 성립한다. T1 Halt Forecast(직접 들여쓰기 대안)는 발동하지 않았다.
  - **egui 0.35 API 변경**: plan D2가 적은 `SidePanel::left`는 이 버전에 없다 — `egui::Panel::left(id)`로 통합됐고 크기 설정도 `default_size`/`size_range`다. 폭 클램프(160~480)를 `size_range`가 직접 수행한다.
  - **리뷰가 잡은 실제 결함(T2 M2)**: 사이드바가 조작을 `Option` 하나로 돌려주는 바람에, 이름 편집 커밋(`Rename`)과 다른 항목 클릭(`Select`)이 한 프레임에 겹치면 하나가 조용히 유실됐다 → `Vec` 반환으로 구조적으로 해소(D12 ③). **한 프레임에 조작이 둘 이상 나올 수 있는 위젯은 단일 슬롯으로 받으면 안 된다.**
- **T5 완료** (커밋 pre-review `b2fbef3` → review-fix): 세션 저장·복원(`ui/session.rs`). 리뷰 지적은 MINOR 3건(전부 반영 또는 근거 기록).
  - **지연 생성과 저장의 공존**: 복원한 워크스페이스를 곧바로 만들지 않고 `restored` 맵에 두었다가 처음 선택될 때 뷰로 바꾼다. 저장할 때는 아직 열지 않은 것을 **불러온 상태 그대로 다시 내보낸다**(이름만 최신값) — 이러지 않으면 앱을 켜고 워크스페이스 하나만 보다 끄면 나머지 구성이 전부 날아간다.
  - **최대화 창의 함정**: 최대화 상태에서 `outer_rect`는 화면 전체다. 그대로 저장하면 다음 실행에서 "최대화 해제 시 돌아갈 크기"가 사라진다 → **최대화 중에는 위치·크기를 갱신하지 않는다**(plan Halt Forecast가 예고한 대안 그대로).
  - **모니터 정보 타이밍**: 화면 밖 위치 보정은 모니터 크기를 알아야 하는데 첫 프레임에 `monitor_size`가 비어 있을 수 있다 → 값이 없으면 보정을 **소비하지 않고 다음 프레임으로 미룬다**(T5 quality 리뷰 m2).
- **T3·T4 완료** (커밋 `fdd3851`·`221bef3`): 메뉴 바·단축키(`ui/menu.rs`)와 변경 감시 배선. 두 task 모두 BLOCKER/MAJOR 0건.
  - **임시 UI 정리**: part1의 명령 줄(좌우 분할/상하 분할/패널 닫기 버튼)을 메뉴가 대체해 제거했다. 사이드바 접기 복원 진입점도 메뉴 "워크스페이스 사이드바"(Ctrl+B)로 옮겼다.
  - **단축키 가드**: `ctx.egui_wants_keyboard_input()`이 참이면 단축키를 아예 보지 않는다. 이 값은 이름 그대로 "텍스트 입력 중"이 아니라 **포커스를 가진 위젯이 있으면 참**이라 실제로는 더 넓게 막는다 — 가로채기를 막는 방향이라 그대로 두고 주석을 사실에 맞췄다.
  - **선택 유지(T4)**: `set_entries`가 무조건 선택을 지워, 감시가 갱신할 때마다 고르던 파일이 풀렸다 → **같은 폴더 재열거면 이름 기준으로 선택을 되살린다**. 감시 갱신은 사용자가 조작하지 않아도 일어나므로, 이 보존이 없으면 다른 앱이 파일 하나 만들 때마다 선택이 사라진다.
  - **자기 트리거 없음**: `poll_watch` → 재열거 → 커밋 → `watch()` 경로가 돌지만 같은 경로면 `watch()`가 no-op이라 감시자가 재생성되지 않는다.
  - **결정**: 트리 토글은 패널 상태 줄 버튼(D11), 뷰 보관 키는 `WorkspaceId`(D12 ①). 사이드바를 접으면 현행처럼 완전히 사라지므로 되돌릴 진입점을 임시 명령 줄("워크스페이스 목록")에 뒀다 — T3에서 메뉴로 이관된다.

## Next Steps

- **part1을 먼저 완료**한 뒤 이 plan을 실행한다
- T6 실측 결과에 따라 T7(구 코드 제거) 진행 여부가 갈린다(D10)
- 전체 완료 후 push·PR·master 병합 전략을 사용자와 결정
