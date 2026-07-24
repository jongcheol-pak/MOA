# Plan: 워크스페이스 사이드 패널 + 워크스페이스별 탐색기

**PRD**: docs/prd.md

## 요구 이해

- **원문 요청**: "이미지처럼 레이아웃을 분리, 왼쪽에 사이드 패널, 오른쪽에 탐색기 패널(현재 구현된 화면) / 왼쪽에 워크스페이스 목록 생성 기능 / 워크 스페이스가 생성되면 오른쪽 레이 아웃을 탐색기도 생성 / 생성된 워크스페이스 항목을 선택하면 오른쪽 탐색기 화면 전환 / 워크스페이스 항목 마다 탐색기 패널이 각각 있는 거임"
- **이해한 요구**: 창을 좌우로 나눠 왼쪽에 워크스페이스 목록(첨부 이미지와 같은 다크 카드 2줄 항목), 오른쪽에 지금까지 만든 탐색기 화면을 둔다. 워크스페이스를 새로 만들면 그 워크스페이스만의 탐색기(분할 레이아웃·패널·탭·히스토리)가 별도로 생기고, 목록에서 항목을 고르면 오른쪽이 그 워크스페이스 화면으로 통째로 전환된다. 즉 워크스페이스는 "탐색기 화면 한 벌"을 담는 그릇이며 서로 상태를 공유하지 않는다. 이름 변경·삭제·순서 변경, 사이드바 접기·폭 조절, 종료 후 복원까지 포함한다.
- **포함하지 않는 것으로 이해**: 워크스페이스를 별도 창으로 띄우는 것(창은 1개 유지), 워크스페이스 간 탭·패널 이동(드래그로 옮기기), 앱 전체 다크 테마화(다크는 사이드바에만 적용).

## Goal

한 창 안에서 좌측 워크스페이스 목록으로 여러 벌의 탐색기 화면을 만들고 즉시 전환할 수 있게 한다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-15 | Must | T1(부제 문자열), T4, T5 | ✅ 커버 |
| FR-16 | Must | T1, T4, T5, T6(생성 직후 인라인 편집) | ✅ 커버 |
| FR-17 | Must | T2, T5 | ✅ 커버 |
| FR-18 | Should | T1, T6 | ✅ 커버 |
| FR-19 | Should | T7 | ✅ 커버 |
| FR-20 | Should | T3, T5, T7 | ✅ 커버 |
| NFR-8 | — | T8 | ✅ 커버 |
| FR-1~FR-12 | Must/Should | (없음) | ✅ 기구현 (part1·part2, Phase G 통과) |
| FR-13, FR-14 | Could | (없음) | 이번 범위 외 (Deferred 대장 대기 중) |
| NFR-1~NFR-7 | — | T8(회귀 확인만) | ✅ 기구현 (part2 T5 실측) |

## Out of Scope

- 워크스페이스별 색상·아이콘 사용자 지정 (모든 항목은 시스템 폴더 아이콘 고정)
- 워크스페이스 간 탭·패널 드래그 이동
- 앱 전체 다크 테마 (사이드바 외 영역은 시스템 기본 톤 유지 — PRD Out of Scope)
- 워크스페이스를 별도 창으로 분리하기

## Deferred / Follow-up

- FR-13 숨김·시스템 파일 표시 토글 (Could — deferred 대장 대기 유지)
- FR-14 분할 프리셋 버튼 (Could — deferred 대장 대기 유지)
- 트리→목록 양방향 동기화 (part2 D14 — 대기 유지)
- shell_menu `items_menu`의 `pidls[0]` 암묵 계약 문서화 (part2 T2 리뷰 m1 — 대기 유지)
- 사이드바 항목 가상 스크롤·커스텀 다크 스크롤바 (이번엔 휠 스크롤만 — 항목이 수십 개를 넘으면 재검토)
- 인라인 이름 편집 EDIT의 다크 스타일링 (앱 전체 테마 도입 시 함께)

## Investigation Log

- `git status` → 작업 트리 깨끗, part1·part2 plan은 `## Phase Ledger`에 `Phase G 통과 (Must 100%)` 기록 확인 → 완료 상태, 새 plan을 누적 위치에 신규 작성
- `docs/plans/deferred.md` `## 대기` 4건 확인 → 모두 이번 요구와 무관(숨김파일·프리셋·트리 동기화·주석 문서화) → 이번 plan에서도 Deferred 유지
- `src/app/window.rs` 전문 Read → `AppState { host: LayoutHost, menu: HMENU }` 단일 host 소유, WM_SIZE/LBUTTONDOWN/MOUSEMOVE/LBUTTONUP/CAPTURECHANGED/SETCURSOR/PARENTNOTIFY가 모두 `state.host`에 직결
- `src/app/layout_host.rs` 전문 Read → `relayout`/`split_active`가 `client_rect(parent)`(부모 클라이언트 전체)를 영역으로 사용. 패널 HWND는 모두 메인 창의 직계 자식(`panel_win::create(parent)`). **패널 창을 일괄 파괴하는 API도 `Drop` 구현도 없다**(`close_active`는 활성 1개만) → 워크스페이스 삭제에는 신규 API가 필요(T2)
- `src/app/layout.rs:247-275` `compute_rects(area: Rect)` Read → `walk`가 area를 그대로 전파하며 원점(x,y) 비영(非零)을 이미 지원 → 사이드바 폭만큼 오프셋된 영역 주입이 가능(신규 좌표 변환 로직 불필요)
- 히트테스트 좌표계 확인 → `layout_host`의 `begin_drag:194`·`apply_splitter_cursor:244`·`set_active_by_point:149`가 모두 같은 `layout_cache`(동일 area 기준)와 부모 클라이언트 좌표를 쓰고, `window.rs`의 `coords()`·`cursor_in_client()`·`WM_PARENTNOTIFY:424-429`도 같은 좌표 공간 → area 주입만으로 정합(plan-reviewer 교차 확인 완료)
- `grep "host\.|LayoutHost|panel_win::create"` (src·tests 전수) → 호출부는 `src/app/window.rs` 24줄(그중 `host.` 19줄)뿐, `src/panel/panel.rs` 2건은 주석, `tests/` 0건 → 시그니처 변경 영향 범위가 window.rs로 한정됨
- `src/app/settings.rs` 전문 Read → `Session{version,window,layout,panels}` v1, `parse_session`이 버전 불일치·리프 수 불일치·빈 탭·비유한 비율·비양수 크기를 전부 None(전체 폴백) 처리
- `src/panel/panel.rs` 부분 Read + `grep WM_APP` → 사용 중인 번호는 WM_APP+1(enumerate)·+2(address_bar)·+3~+8·+9(watcher)·+10~+12 → 신규 +13~+16 충돌 없음
- `src/main.rs:41` Read → `TranslateAcceleratorW`가 메시지 루프 최상단에서 포커스와 무관하게 단축키를 소비함 → 편집 컨트롤과 충돌하는 무수식 키(F2·Delete)는 액셀러레이터로 넣지 않는다(D16)
- `grep "WM_SETFONT|GetStockObject|CreateSolidBrush|DrawTextW|FillRect"` → 0건. 프로젝트에 커스텀 GDI 그리기·폰트 설정 관례가 전혀 없음 → 사이드바 그리기는 전부 신규 코드
- `grep "DPI|GetDpiForWindow"` → 스케일 계산 코드 0건. `MIN_PANE_SIZE`(120)·`TREE_WIDTH`·`TAB_HEIGHT`·`STRIP_HEIGHT` 모두 96DPI 기준 고정 px 상수 → 사이드바 치수도 같은 관례를 따른다
- `build.rs`·`app.manifest` 확인 → PMv2 DPI·공용 컨트롤 v6·longPathAware를 매니페스트로 임베드 (빌드 의존성 없음)
- `src/panel/address_bar.rs:44-64` Read → 표준 `EDIT` 자식 생성 + `SetWindowSubclass`로 Enter 가로채기 패턴 존재 → 인라인 이름 편집에 그대로 재사용
- `src/fs/icons.rs` Read → `IconCache`가 시스템 이미지 리스트 핸들(`himl()`)과 폴더 아이콘 인덱스(`dir_icon`, 비공개)를 이미 보유 → 사이드바 항목 아이콘에 재사용(접근자 1개 추가 필요)
- `Cargo.toml` 확인 → `Win32_Graphics_Gdi`·`Win32_UI_Controls`·`Win32_UI_Shell`·`Win32_UI_WindowsAndMessaging` feature가 이미 활성 → GDI 그리기·ImageList·서브클래스·NONCLIENTMETRICS에 신규 feature·crate 불필요
- 위키 참조: 없음(vault 미설정·경로 부재) — 코드 1차 출처로 진행

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 워크스페이스가 늘수록 패널 창·워커 스레드가 누적돼 NFR-2/NFR-8 초과 | 메모리 목표 미달 | 지연 생성(D2) — 방문한 워크스페이스만 창 생성. T8에서 5개 방문 실측, 초과 시 유지 정책 재검토(불가피한 Halt) |
| 삭제한 워크스페이스의 패널 창·감시 스레드가 잔존 | 핸들·메모리 누수 | T2의 `destroy_all`로 일괄 파괴, T8 실측에서 삭제 반복 후 메모리 확인 |
| 숨긴 패널의 폴더 감시(`DirWatcher`)가 계속 살아 있어 CPU·핸들 소모 | 백그라운드 부하 | 감시는 패널당 활성 탭 1개뿐이고 조용 300ms 디바운스라 유휴 비용이 낮다(part2 T3 설계). T8 실측에서 확인, 초과 시 Deferred로 정지 정책 검토 |
| 사이드바 커스텀 그리기(다크)와 시스템 톤 혼재로 시각 이질감 | UX 저하 | 시각 요소 분해 표로 토큰을 고정하고 경계를 명확히 둔다. 이질감은 HUMAN-VERIFY 대상 |
| 패널 배치 영역이 사이드바 폭만큼 어긋나면 스플리터 히트테스트가 밀린다 | 드래그 오작동 | 배치·히트테스트가 동일 `layout_cache`를 쓰므로 area 주입만 정확하면 일치(조사 완료). T2 수용 기준에 폭 0/232 양쪽 확인 포함 |
| 인라인 편집 중 워크스페이스 전환·삭제가 겹치면 EDIT 창이 유령으로 남음 | UI 깨짐 | 편집 종료(커밋/취소)를 선택 변경·삭제·전환·크기 변경 진입점에서 일괄 선호출(T6) |
| "비활성 창 스크롤" 옵션을 끈 환경에서 휠이 사이드바에 안 닿음 | 스크롤 불가 | 사이드바가 포커스를 가지면 휠이 도달(D17). 키보드(↑/↓)로도 항목 이동 가능 |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `LayoutHost::new` / `from_shape` | `src/app/window.rs:91-92` | 시그니처 변경(초기 area 인자 추가) — 호출부 2곳 |
| `LayoutHost::relayout` | `src/app/window.rs:323` + `layout_host.rs` 내부 5곳(`:50,72,121,145,229`) | `parent` 파라미터 제거, 배치 기준을 `self.area`로 변경 |
| `LayoutHost::close_active` | `src/app/window.rs:338,360` | `parent` 파라미터 제거(내부 `relayout` 호출 변경에 연동) |
| `LayoutHost::split_active` | `src/app/window.rs:333,336` | 최소 크기 판정 기준을 `self.area`로 변경(패널 생성용 `parent`는 유지) |
| `LayoutHost` (신규 API) | `src/app/layout_host.rs` | `set_area`·`set_visible`·`destroy_all` 추가 |
| `LayoutHost` 필드 소유자 `AppState.host` | `src/app/window.rs:40,264,323-428` | 단일 host → 워크스페이스별 다중 host로 교체. 참조 19곳 전부 활성 워크스페이스 조회 경유로 변경 |
| `settings::Session`/`parse_session`/`save_session`/`load_session` | `src/app/window.rs:89,292`, `src/app/settings.rs` | 스키마 v2 확장(공개 구조체 필드 변경) |
| `settings::SESSION_VERSION` | `src/app/settings.rs:12`, `src/app/window.rs:293` | 1 → 2 |
| `restore_panels`/`save_current_session` | `src/app/window.rs:209,260` | 워크스페이스 단위 수집·복원으로 재작성 |
| `menu::attach_menu`/`create_accels`/`update_close_enabled` | `src/app/menu.rs`, `src/app/window.rs:87,97,365,370` | 워크스페이스 메뉴·Ctrl+B 추가(기존 시그니처 유지, 상수 추가) |
| `IconCache` (비공개 `dir_icon`) | `src/fs/icons.rs`, `src/app/sidebar.rs`(신규) | 접근자 `dir_icon()` 추가(공개 API 추가) |
| `PanelState` 경로 커밋·탭 전환 지점 | `src/panel/panel.rs` | 경로 변경 알림(WM_APP+13) 게시 추가 — 기존 동작 불변, 부모가 무시해도 무해 |
| `app` 모듈 목록 | `src/app/mod.rs` | `workspace`·`sidebar` 모듈 등록 |

### 4-B. 계약·직렬화 변경

- **세션 스키마 v1 → v2**: 최상위가 `{window, layout, panels}`에서 `{window, sidebar, active_workspace, workspaces[{name, layout, panels, active_panel}]}`로 바뀐다. v1 파일은 기존 규칙대로 버전 불일치 → 전체 폴백(초기화, 사용자 선택). 마이그레이션 코드 없음.
- **신규 창 메시지**: `WM_APP_PATH_CHANGED`(WM_APP+13, 패널 → 부모 게시, wparam=발신 패널 HWND) — 부제 갱신 신호. 경로는 기존 `WM_APP_SESSION_COLLECT` 동기 질의로 조회(신규 페이로드 없음).
- **신규 창 메시지**: `WM_APP_WS_SELECT`(WM_APP+14, wparam=선택 인덱스), `WM_APP_WS_NEW`(WM_APP+15), `WM_APP_WS_CONTEXT`(WM_APP+16, wparam=인덱스, lparam=화면 좌표) — 사이드바 자식 창이 부모(메인 창)로 게시.
- **`LayoutHost` 공개 API 변경**: `relayout`/`close_active`의 `parent` 제거, `new`/`from_shape`에 area 추가, `set_area`/`set_visible`/`destroy_all` 신설. 소비자는 `window.rs` 단독.
- **패널↔레이아웃 1:1 walk 순서 계약**(part2 T4)은 워크스페이스 단위로 그대로 유지된다.

### 4-C. 테스트 파일

- 기존: `src/app/layout.rs`(레이아웃 10), `src/app/settings.rs`(세션 6), `src/panel/*`(탭·히스토리·정렬 등), `tests/watcher.rs`(통합 2) — 총 43+2
- 영향: `src/app/settings.rs` 단위테스트는 v2 스키마로 **전면 갱신 필요**(sample() 구조 변경). 나머지는 시그니처 변경 대상이 아니므로 무영향(LayoutHost는 테스트에서 사용 0건 — grep 확인)
- 신규: `src/app/workspace.rs` 단위테스트(모델 연산·경로 축약), `src/app/sidebar.rs` 순수 계산부 단위테스트(항목 히트테스트·스크롤 클램프·드롭 위치), 사이드바 폭 클램프 테스트

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `workspace::WorkspaceList` (목록 모델) | `panel::tabs::TabsModel`(탭 목록 모델) — add/close/active 구조가 유사하나 이름·순서변경·pending 보관이 없고 탭 전용 타입 | 신규. TabsModel 제네릭화는 두 도메인을 얽어 추적을 어렵게 한다(AGENTS: 명시적·직접적 코드). 구조만 참고 |
| `sidebar::Sidebar` (커스텀 그리기 목록 창) | `grep WM_DRAWITEM/DrawTextW/CreateSolidBrush` → 앱 내 커스텀 그리기 0건. 공용 컨트롤 래퍼(TabStrip·FileList·FolderTree)는 전부 시스템 컨트롤 위임 | 신규. 2줄 다크 카드·드래그 정렬은 시스템 컨트롤로 표현 불가(D3) |
| `LayoutHost::destroy_all` | `close_active`(활성 1개 파괴) — 전체 파괴·Drop 없음(파일 전문 확인) | 신규(불가피 — 워크스페이스 삭제에 필요) |
| 인라인 이름 편집 EDIT | `panel::address_bar`의 `EDIT` + `SetWindowSubclass` Enter 가로채기 | **재사용**(패턴 동일 — 서브클래스로 Enter/Esc 가로채 부모에 게시) |
| 사이드바 항목 아이콘 | `fs::icons::IconCache` — 시스템 이미지 리스트 핸들·폴더 아이콘 인덱스 보유 | **재사용**(접근자 `dir_icon()`만 추가, 아이콘 리소스 신규 제작 없음) |
| 사이드바 폭 스플리터 | `layout_host`의 `begin_drag`/`drag_move`/`end_drag` + `SetCapture` 패턴 | **패턴 재사용**(코드는 별도 — 대상이 트리 노드가 아니라 단일 경계선이라 로직이 다르다. 8줄 규모) |
| `settings::WorkspaceSession`/`settings::SidebarSession` | `settings::PanelSession`(기존 직렬화 미러 구조) | 신규(스키마 확장) — 기존 `LayoutNode`·`PanelSession`은 그대로 재사용 |
| 외부 crate | 없음 — GDI·ImageList·서브클래스 모두 이미 활성화된 windows feature로 가능 | 신규 의존성 0 (최소 의존 원칙) |

### Verified by

- `grep -rn "host\.|LayoutHost|panel_win::create|MIN_PANE_SIZE" src/ tests/` → LayoutHost 관련 전부 `src/app/window.rs`(위 표에 포함), `panel.rs` 2건은 주석, `tests/` 0건
- `grep -rn "WM_APP" src/` → 사용 번호 +1·+2·+3~+12 확인 → 신규 +13~+16 충돌 없음
- `grep -rn "settings::|Session|parse_session" src/` → 소비자는 `window.rs` 2곳(load/save)과 settings 내부 테스트뿐
- `src/app/layout.rs:247-275`, `src/app/layout_host.rs` 전문, `src/main.rs:41` 직접 Read로 확인(추정 아님)

## Decisions

### D1. 워크스페이스별 탐색기 영역 구현 방식
- **Options**: A) 워크스페이스마다 컨테이너 자식 창을 만들고 그 안에 패널을 담아 창 단위로 show/hide / B) 패널은 지금처럼 메인 창의 직계 자식으로 두고, `LayoutHost`에 배치 영역(Rect)을 주입 + 패널 HWND를 일괄 show/hide
- **Chosen**: B
- **Rationale**: `compute_rects`가 이미 임의 원점 area를 지원하고 배치·히트테스트가 동일 좌표계라(조사 완료) 좌표 변환 신규 로직이 필요 없다. A는 새 창 클래스와 함께 WM_SIZE·스플리터 드래그·WM_SETCURSOR·WM_PARENTNOTIFY 처리를 컨테이너로 이관해야 해(window.rs 약 150줄 이동) diff와 회귀 위험이 크다.
- **Source**: `src/app/layout.rs:247-275`, `src/app/layout_host.rs:149,161-191,194,244`, `src/app/window.rs:320-431`

### D2. 비활성 워크스페이스의 탐색기 창 수명
- **Options**: A) 지연 생성 후 숨김 유지 / B) 전환 시 파괴·재생성
- **Chosen**: A (사용자 선택)
- **Rationale**: 전환이 즉시이고 스크롤·선택 상태가 보존된다. 메모리 누적은 "방문한 워크스페이스만 생성"으로 억제하고 NFR-8(5개 방문 100MB)로 실측 검증한다.
- **Source**: 사용자 결정(2026-07-24), PRD NFR-8

### D3. 사이드바 렌더링 방식
- **Options**: A) 커스텀 창 클래스 + WM_PAINT 직접 그리기 / B) ListBox `LBS_OWNERDRAWFIXED` / C) SysListView32 커스텀 드로우
- **Chosen**: A
- **Rationale**: 2줄 카드·다크 배경·드래그 정렬·인라인 편집 오버레이를 전부 직접 제어해야 하는데, B/C는 시스템이 그리는 배경·선택 하이라이트(밝은 테마)를 매번 덮어써야 해 오히려 코드가 늘고 이질감이 남는다. 기존 오너드로우 인프라도 없다(grep 0건).
- **Source**: `grep WM_DRAWITEM/DrawTextW` 0건, 첨부 디자인 스크린샷

### D4. 사이드바 폰트
- **Options**: A) `SystemParametersInfoW(SPI_GETNONCLIENTMETRICS)`의 `lfMessageFont` 복사 후 높이만 조정 / B) 폰트명 하드코딩 / C) `GetStockObject(DEFAULT_GUI_FONT)`
- **Chosen**: A
- **Rationale**: 한국어 UI(NFR-6)에서 한글 글리프가 있는 시스템 UI 폰트를 자동으로 얻는다. B는 한글 폴백 의존, C는 구식 비트맵 폰트라 다크 배경에서 품질이 나쁘다.
- **Source**: PRD NFR-6, `grep WM_SETFONT` 0건

### D5. 사이드바 세로 스크롤
- **Options**: A) 마우스 휠 + 키보드만(스크롤바 없음) / B) 표준 `WS_VSCROLL` / C) 커스텀 다크 스크롤바
- **Chosen**: A
- **Rationale**: 기준 디자인에 스크롤바가 없고, B는 밝은 시스템 스크롤바가 다크 사이드바에 붙어 이질감이 크다. C는 이번 범위에 과하다(Deferred 등록).
- **Source**: 첨부 디자인 스크린샷

### D6. 항목 부제(경로) 갱신 규칙
- **Options**: A) 워크스페이스 전환·생성 시에만 갱신 / B) 패널이 경로 커밋·탭 전환 시 부모에 알림(WM_APP+13) → 부모가 기존 `WM_APP_SESSION_COLLECT`로 조회해 갱신
- **Chosen**: B + 다음 두 규칙 고정:
  1. 알림의 **발신 HWND가 활성 워크스페이스의 `active_hwnd()`와 같을 때만** 부제를 갱신한다(비활성 패널의 이동은 부제를 바꾸지 않음)
  2. **활성 패널이 바뀌는 순간**(`set_active_by_point` 직후)에도 같은 조회로 부제를 갱신한다
- **Rationale**: A는 폴더 이동 후 부제가 옛 경로로 남아 FR-15("활성 탭 폴더 경로")를 만족하지 못한다. 두 규칙이 없으면 분할 2개 이상일 때 "마지막으로 이동한 패널"의 경로가 표시돼 역시 FR-15와 어긋난다.
- **Source**: `src/panel/panel.rs:53-64`(SESSION_COLLECT 계약), `src/app/window.rs:419-429`(활성 패널 전환 지점), PRD FR-15

### D7. 자동 이름·이름 규칙
- **Options**: A) "워크스페이스 {목록 길이+1}" / B) "워크스페이스 {사용 중이지 않은 최소 번호}"
- **Chosen**: B
- **Rationale**: 삭제 후 재생성 시 A는 기존 이름과 중복될 수 있다(1·2·3에서 2 삭제 후 생성 → "워크스페이스 3" 중복). 빈 이름·공백만 입력한 rename은 거부하고 이전 이름을 유지한다.
- **Source**: PRD FR-16, `settings::parse_session`의 엄격 폴백 관례

### D8. 워크스페이스 삭제 정책
- **Options**: A) 확인 없이 즉시 삭제 / B) `MessageBoxW` 확인 후 삭제 / C) 실행 취소 지원
- **Chosen**: B (마지막 1개는 삭제 불가 — 메뉴 항목 비활성 + 키 입력 무시)
- **Rationale**: 삭제하면 분할·탭 구성이 즉시 사라져 되돌릴 수 없다. 마지막 1개 금지는 FR-2(마지막 패널 닫기 불가)와 같은 원칙이며 `update_close_enabled`와 같은 비활성 패턴을 쓴다.
- **Source**: PRD FR-18, `src/app/menu.rs:154-164`

### D9. 세션 스키마 v2 검증 정책
- **Options**: A) 기존과 같이 위반 시 전체 폴백 / B) 항목 단위 복구
- **Chosen**: A + **사이드바 폭만 클램프**
- **Rationale**: 기존 `parse_session`의 전체 폴백 관례를 유지해 일관성을 지킨다. 다만 사이드바 폭은 창 크기 변화로 정상 사용 중에도 범위를 벗어날 수 있으므로 오염이 아니라 `[160, 480]` 클램프로 처리한다.
- **Source**: `src/app/settings.rs:131-154`

### D10. 시각 토큰(색·치수)
- **Options**: A) 기준 스크린샷에서 추정한 값을 plan에 고정하고 코드 상수로 1:1 반영 / B) 구현 중 눈대중 조정
- **Chosen**: A — `## 시각 요소 분해` 표가 정본
- **Rationale**: B는 구현자마다 값이 달라지고 사후 검증(V-9)이 불가능하다.
- **Source**: 첨부 디자인 스크린샷

### D11. 접힘 상태에서의 복원 경로
- **Options**: A) 접으면 폭 0(완전 숨김), 복원은 메뉴 "보기 > 워크스페이스 사이드바" + Ctrl+B / B) 접어도 32px 아이콘 바를 남김
- **Chosen**: A
- **Rationale**: 기준 디자인에 축소 바가 없고, 32px 바는 아이콘 전용 렌더 경로가 하나 더 생긴다. 복원 경로가 2개라 갇히지 않는다.
- **Source**: 첨부 디자인 스크린샷, PRD FR-19

### D12. 드래그 순서 변경 조작
- **Options**: A) 8px 이상 이동 시 드래그 시작, 커서가 놓인 항목의 세로 중앙 기준 앞/뒤 삽입, 삽입선 2px 표시 / B) 즉시 드래그 시작
- **Chosen**: A
- **Rationale**: B는 단순 클릭 선택이 의도치 않은 재정렬을 일으킨다. 임계값은 고정 8px(프로젝트 고정 px 관례).
- **Source**: PRD FR-18

### D13. DPI 스케일링
- **Options**: A) 기존 관례대로 96DPI 기준 고정 px / B) 사이드바만 `GetDpiForWindow` 스케일 적용
- **Chosen**: A
- **Rationale**: 앱 전체가 고정 px 상수로 되어 있어 사이드바만 스케일하면 톤이 어긋난다. 전면 DPI 스케일 도입은 이번 요구 범위 밖.
- **Source**: `grep DPI` 0건, `src/app/layout.rs:6`

### D14. 워크스페이스 항목 아이콘
- **Options**: A) `IconCache`의 시스템 폴더 아이콘(16x16) 재사용 / B) 자체 도형 그리기 / C) 아이콘 없음
- **Chosen**: A (`IconCache::dir_icon()` 접근자 추가, `ImageList_Draw`로 그림)
- **Rationale**: 기준 디자인에 항목 아이콘이 있고, 셸 아이콘 재사용은 리소스 추가 없이 가능하며 다른 화면과 시각적으로 일관된다.
- **Source**: `src/fs/icons.rs:33-59`

### D15. 신규 의존성
- **Options**: A) 신규 crate 없음(windows feature 재사용) / B) 그리기 보조 crate 도입
- **Chosen**: A
- **Rationale**: GDI·ImageList·서브클래스·NONCLIENTMETRICS feature가 이미 활성이라 직접 구현으로 충분하다(최소 의존 원칙).
- **Source**: `Cargo.toml` features 목록

### D16. 워크스페이스 키 입력 라우팅 (F2·Delete·방향키)
- **Options**: A) 액셀러레이터 테이블에 `VK_F2`·`VK_DELETE` 추가(전역) / B) 사이드바 창의 `WM_KEYDOWN`에서 로컬 처리하고, 액셀러레이터는 Ctrl 조합(Ctrl+B)만 추가
- **Chosen**: B
- **Rationale**: 이 앱의 액셀러레이터는 `main.rs:41`의 `TranslateAcceleratorW`가 **포커스와 무관하게 최상단에서 소비**한다. `VK_DELETE`를 전역으로 넣으면 주소창 EDIT(`address_bar.rs`)과 신규 인라인 편집 EDIT에서 Delete로 글자를 지울 수 없게 되어 FR-6이 회귀한다. F2·Delete·↑/↓는 사이드바가 포커스를 가진 동안에만 동작한다(메뉴 항목은 포커스와 무관하게 항상 사용 가능).
- **Source**: `src/main.rs:38-45`, `src/app/menu.rs:101-151`, `src/panel/address_bar.rs:43-64`

### D17. 사이드바 포커스·휠 정책
- **Options**: A) 클릭해도 포커스를 가져가지 않음(휠·키 입력 불가) / B) 클릭 시 `SetFocus`로 포커스를 가져감
- **Chosen**: B
- **Rationale**: D16의 키 조작(F2·Delete·방향키)과 휠 스크롤이 성립하려면 포커스가 필요하다. 워크스페이스 전환 직후 사용자는 대개 목록을 더 보거나 이름을 고치므로 포커스가 사이드바에 있는 편이 자연스럽고, 탐색기 조작은 패널 클릭으로 즉시 되돌아온다(기존 `WM_PARENTNOTIFY` 활성 패널 갱신 경로 그대로). 휠은 Windows 10/11 기본값인 "비활성 창 스크롤"이 켜져 있으면 호버만으로도 도달하며, 꺼진 환경에서는 포커스가 있을 때 동작한다 — 별도 라우팅 코드는 두지 않는다(과설계 방지).
- **Source**: `src/app/window.rs:419-429`, D16

### D18. 미방문(Pending) 워크스페이스의 부제 산출
- **Options**: A) `WorkspaceSession`에 `subtitle` 문자열을 따로 저장 / B) `WorkspaceSession`에 `active_panel: usize`를 저장하고 부제 = `panels[active_panel].tabs[active_tab]`을 축약 / C) 항상 `panels[0]` 사용
- **Chosen**: B
- **Rationale**: 재시작 직후 활성 외 워크스페이스는 살아있는 패널이 없어 D6의 갱신 경로가 발동하지 않으므로 세션 데이터만으로 부제를 만들 수 있어야 한다(FR-15는 Must). A는 경로가 두 곳에 중복 저장돼 어긋날 수 있고, C는 분할 상태에서 실제 활성 패널과 다른 경로를 보여준다. B는 Live 워크스페이스의 부제 규칙(활성 패널의 활성 탭)과 정확히 일치한다.
- **승격 시 동작**: `LayoutHost::from_shape`는 활성 패널을 첫 리프로 초기화한다(`layout_host.rs:61` — 활성 지정 API를 새로 만들지 않는다). 따라서 `Pending` → `Live` 승격 순간 활성 패널이 첫 패널이 되며, 부제도 그 시점에 D6 규칙으로 재계산돼 `panels[0]` 기준으로 바뀔 수 있다(의도된 동작 — 저장된 `active_panel`은 부제 산출 전용이며 활성 패널 복원까지는 하지 않는다).
- **Source**: PRD FR-15, `src/app/settings.rs:48-53`(PanelSession 구조)

## 시각 요소 분해

기준 디자인: 사용자 첨부 스크린샷(`화면 캡처 2026-07-23 153052.png`, 좌측 사이드바 영역). "스크린샷 추정"은 원본이 명세가 아닌 렌더 이미지라 정확한 토큰을 얻을 수 없기 때문이며, 아래 값이 이 프로젝트의 **정본 토큰**이다(구현 시 `sidebar.rs` 상수로 1:1 반영, V-9는 코드 상수와 이 표를 대조).

| 요소 | 속성 | 디자인 값 | 확인 방법 |
|------|------|----------|-----------|
| 사이드바 | 기본 폭 | 232px | 스크린샷 측정(전체 1364px 중 좌측 영역) |
| 사이드바 | 최소/최대 폭 | 160px / 480px | plan 확정값(스플리터 클램프) |
| 사이드바 | 배경색 | `#1B1B1B` | 스크린샷 색 추정 |
| 헤더 | 높이 | 36px | 스크린샷 측정 |
| 헤더 | 문구 | "워크스페이스" | 스크린샷 문구 그대로 |
| 헤더 | 글자색/크기 | `#9A9A9A` / 12px | 스크린샷 색·크기 추정 |
| 헤더 `+` 버튼 | 크기·위치 | 24x24px, 우측 여백 8px | 스크린샷 측정 |
| 헤더 `+` 버튼 | 글자색 | 기본 `#9A9A9A`, 마우스 오버 `#E8E8E8` | 스크린샷 추정(호버는 plan 확정값) |
| 사이드바 상단 토글 | 크기·위치 | 24x24px, 좌측 여백 8px, 헤더 위 28px 영역 | 스크린샷 측정(좌상단 패널 토글 아이콘) |
| 항목 | 높이 / 세로 간격 | 60px / 4px (pitch 64px) | 스크린샷 측정(항목 중심 간격 64px) |
| 항목 | 좌우 바깥 여백 | 8px | 스크린샷 측정 |
| 항목 | 배경색 | 기본 `#232323`, 선택 `#2E2E2E`, 마우스 오버 `#282828` | 스크린샷 색 추정(호버는 plan 확정값) |
| 항목 | 테두리 | 1px `#2C2C2C` | 스크린샷 추정 |
| 항목(선택) | 좌측 강조 바 | 3px 폭, `#4A9EFF` | 스크린샷 추정 |
| 항목 아이콘 | 크기·위치 | 16x16px, 좌측 패딩 12px, 세로 중앙 | 스크린샷 측정 |
| 항목 1줄(이름) | 색 / 크기 / 위치 | `#E8E8E8` / 13px / 텍스트 좌측 x=38px, 상단 패딩 12px | 스크린샷 추정 |
| 항목 2줄(경로) | 색 / 크기 / 줄 간격 | `#8A8A8A` / 11px / 이름 아래 6px | 스크린샷 추정 |
| 항목 텍스트 | 넘침 처리 | 말줄임(`DT_END_ELLIPSIS`) | plan 확정값 |
| 드래그 삽입선 | 두께·색 | 2px, `#4A9EFF` | plan 확정값(스크린샷에 없음 — D12) |
| 인라인 편집 EDIT | 배치 | 항목 1줄 영역과 동일 사각형, 시스템 기본(밝은) 배경 | plan 확정값(다크 EDIT 커스텀은 Deferred) |
| 사이드바↔탐색기 경계 | 스플리터 두께 | 4px (`layout::SPLITTER_THICKNESS`와 동일) | `src/app/layout.rs:9` |

### V-9 대조 결과 (T4 시점 — 코드 상수 기준)

데스크톱 UI라 자율 루프에서 창 렌더 캡처를 신뢰성 있게 수행할 수단이 없다 → **렌더 일치는 전부 `⏳ 미확인`이며 Phase F-8이 게이트한다**. 아래는 디자인 값 ↔ 코드 상수 대조다(빌드 통과는 시각 일치의 근거가 아니다).

| 요소·속성 | 디자인 값 | 코드 위치 | 상수 대조 | 렌더 |
|---|---|---|---|---|
| 사이드바 기본/최소/최대 폭 | 232 / 160 / 480 | `settings.rs:18-20` | ✅ | ⏳ 미확인 |
| 사이드바 배경 | `#1B1B1B` | `sidebar.rs:70` | ✅ | ⏳ 미확인 |
| 상단 토글 영역 높이 | 28px | `sidebar.rs` `TOGGLE_STRIP_HEIGHT` | ✅ | ⏳ 미확인 |
| 상단 토글 버튼 크기·여백 (T7) | 24x24px·좌측 8px | `sidebar.rs` `TOGGLE_SIZE`/`TOGGLE_MARGIN`/`toggle_rect` | ✅ | ⏳ 미확인 |
| 헤더 높이 / 문구 / 글자색·크기 | 36px / "워크스페이스" / `#9A9A9A`·12px | `sidebar.rs:43,44,78,61` | ✅ | ⏳ 미확인 |
| `+` 버튼 크기·여백 / 색·호버 | 24x24·8px / `#9A9A9A`→`#E8E8E8` | `sidebar.rs:46,47,78,79` | ✅ | ⏳ 미확인 |
| 항목 높이 / 간격(pitch) | 60 / 4 (64) | `sidebar.rs:48,49,51` | ✅ | ⏳ 미확인 |
| 항목 좌우 바깥 여백 | 8px | `sidebar.rs:52` | ✅ | ⏳ 미확인 |
| 항목 배경 기본/선택/호버 | `#232323`/`#2E2E2E`/`#282828` | `sidebar.rs:71,72,73` | ✅ | ⏳ 미확인 |
| 항목 테두리 1px | `#2C2C2C` | `sidebar.rs:74` + `frame()` | ✅ | ⏳ 미확인 |
| 선택 좌측 강조 바 | 3px `#4A9EFF` | `sidebar.rs:53,75` | ✅ | ⏳ 미확인 |
| 항목 아이콘 크기·좌측 패딩 | 16x16·12px, 세로 중앙 | `sidebar.rs:54,55` + `ImageList_Draw` 호출 | ✅ | ⏳ 미확인 |
| 이름 색/크기/좌측 x/상단 패딩 | `#E8E8E8`/13px/38px/12px | `sidebar.rs:76,58,56,57` | ✅ | ⏳ 미확인 |
| 경로 색/크기/줄 간격 | `#8A8A8A`/11px/6px | `sidebar.rs:77,60,59` | ✅ | ⏳ 미확인 |
| 텍스트 넘침 말줄임 | `DT_END_ELLIPSIS` | `sidebar.rs` `draw_line`/`draw_header_text` | ✅ | ⏳ 미확인 |
| 드래그 삽입선 2px `#4A9EFF` | — | (T6에서 구현) | ⏳ 미구현 | ⏳ 미확인 |
| 인라인 편집 EDIT 배치 | — | (T6에서 구현) | ⏳ 미구현 | ⏳ 미확인 |
| 사이드바↔탐색기 경계 4px | `SPLITTER_THICKNESS` | `window.rs` `explorer_area` | ✅ | ⏳ 미확인 |

## Tasks

<!-- T1~T3 (모델·배치·세션 기반) / T4~T6 (사이드바 UI·워크스페이스 동작) / T7~T8 (부가 기능·검증) -->

- [x] T1. 워크스페이스 목록 모델 (순수 로직)
  - **Type**: C
  - **Design**: ① `src/app/workspace.rs` 신규(HWND 비의존 — `layout.rs`와 같은 순수 로직 계층). ② `WorkspaceId(u32)` — 생성 순서 증가·재사용 없음(`PanelId` 관례), `Workspace{id, name, subtitle}` — 표시 데이터, `WorkspaceList{items, active, next_id}` — `add`/`rename`/`remove`/`reorder`/`set_active`/`active_index`/`auto_name` 소유, `elide_path(path, max_chars) -> String` — 부제 문자열 생성. ③ std만 참조하고, `sidebar.rs`·`window.rs`가 참조한다. ④ 추상화하지 않을 것: `TabsModel`과의 공통 트레이트 추출 금지(4-D), 직렬화 타입은 `settings.rs`에 두고 여기서 갖지 않는다.
  - **Acceptance**: Given 항목 3개, When `remove(1)` 후 `add()`, Then 자동 이름이 "워크스페이스 2"(사용 중이지 않은 최소 번호, D7)이고 활성 인덱스가 범위 안이다 / 마지막 1개 `remove`는 `Err` 반환 / `reorder(0→2)` 후 순서가 바뀌고 활성 항목의 id가 보존된다 / 빈 문자열·공백만 `rename`은 거부되고 이전 이름 유지 / `elide_path`가 260자 초과 경로를 지정 길이로 말줄임한다 / `cargo test` 신규 테스트 전부 통과
  - **Files**:
    - 주: `src/app/workspace.rs`(신규)
    - 동반: `src/app/mod.rs`
    - 테스트: `src/app/workspace.rs` 내 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 항목 0개는 만들 수 없음(마지막 삭제 거부)
    - `reorder`·`remove`·`set_active` 인덱스 범위 밖 → 무시(상태 불변)
    - 이름 길이 상한(128자)·개행 문자 제거
    - 경로가 비정상적으로 긴 경우(260자 초과) 말줄임
  - **Halt Forecast**:
    - (i) 자동 이름 규칙 모호 → D7에서 확정
  - **Depends on**: -

- [x] T2. LayoutHost 배치 영역 주입·표시 토글·전체 파괴
  - **Type**: D
  - **Design**: ① `src/app/layout_host.rs` 수정. ② `LayoutHost`에 `area: Rect` 필드 추가; `new(parent, area)`/`from_shape(parent, shape, area)`로 변경, `set_area(&mut self, area: Rect)`, `set_visible(&self, visible: bool)`(소유 패널 HWND 일괄 `ShowWindow`), `destroy_all(&mut self)`(모든 패널 HWND `DestroyWindow` 후 `panes` 비움 — 워크스페이스 삭제용), `relayout(&mut self)`·`close_active(&mut self)`는 `parent` 파라미터 제거(내부에서 `self.area` 사용), `split_active(&mut self, parent, dir)`는 패널 생성에만 `parent`를 쓰고 크기 판정은 `self.area` 기준. ③ `layout.rs`(순수 로직)를 참조하고 `window.rs`가 참조 — 의존 방향 불변. ④ 추상화하지 않을 것: 워크스페이스 개념을 LayoutHost에 넣지 않는다(호스트는 "영역 하나에 그려지는 분할 트리"만 안다).
  - **Acceptance**: Given 영역=클라이언트 전체, When 분할·스플리터 드래그·패널 닫기 수행, Then 변경 전과 동일 동작(회귀 없음, HUMAN-VERIFY) / Given 영역 x=232 주입, When 스플리터 위 커서·드래그, Then 232px 오프셋 위치에서 정확히 반응(HUMAN-VERIFY) / `set_visible(false)` 후 해당 호스트의 모든 패널 창이 비표시 / `destroy_all` 후 `active_hwnd()`가 `None`이고 이후 `relayout`·`set_visible` 호출이 크래시 없이 무시된다(`panel_count()`는 레이아웃 트리 리프 수를 세므로 판정 기준으로 쓰지 않는다 — `layout.rs:104-112`) / `cargo build`·`cargo clippy --all-targets -- -D warnings`(미사용 파라미터 경고 0)·`cargo test` 통과(기존 43+2 유지)
  - **Files**:
    - 주: `src/app/layout_host.rs`
    - 동반: `src/app/window.rs`(호출부: 생성 2·relayout 1·split 2·close 2 — 이 단계에서는 클라이언트 전체 영역 주입으로 기존 동작 유지)
    - 테스트: 기존 `src/app/layout.rs` 테스트로 회귀 확인(HWND 의존부는 테스트 비대상 — AGENTS 규약)
  - **Edge Cases**:
    - 주입 영역 폭/높이 0 이하 → 배치 생략, 패널 크기 음수 방지(`max(0)` 기존 유지)
    - `MIN_PANE_SIZE` 판정이 축소된 영역 기준이라 분할 거부 가능 → 기존 `TooSmall` 무시 경로 유지
    - `set_visible(true)` 직후 배치가 낡을 수 있음 → 항상 `relayout`과 짝 호출
    - **창 크기 변경(WM_SIZE)** → `relayout`이 더 이상 부모 클라이언트를 읽지 않으므로 `set_area(계산된 영역)` 후 `relayout()`을 반드시 짝으로 호출(`window.rs:321-326` 수정 대상)
    - `destroy_all` 후 재사용 금지(엔트리 자체를 목록에서 제거) — 빈 `panes`에서 `active_hwnd()`는 `None`
  - **Halt Forecast**:
    - (ii-a) `LayoutHost` 공개 시그니처 변경(area 주입·파라미터 제거) + 신규 공개 메서드 3개 → `## 사전 승인 항목`에 등록
  - **Depends on**: -

- [x] T3. 세션 스키마 v2 (워크스페이스 1개 상태 유지)
  - **Type**: D
  - **Design**: ① `src/app/settings.rs`(스키마·검증·테스트)와 `src/app/window.rs`(수집·복원 배선). ② 신규 직렬화 타입 `WorkspaceSession{name: String, layout: LayoutNode, panels: Vec<PanelSession>, active_panel: usize}`(D18), `SidebarSession{width: i32, collapsed: bool}`(사이드바 창 내부 상태 타입 `sidebar::SidebarState`와 이름이 겹치지 않게 `Session` 접미사 사용); `Session`을 `{version: 2, window, sidebar, active_workspace, workspaces}`로 변경, `SESSION_VERSION = 2`. 검증은 기존 정책 계승(워크스페이스 비었거나 `active_workspace` 범위 밖·`active_panel` 범위 밖·리프 수 불일치·빈 탭·비유한 비율·비양수 창 크기 → 전체 폴백) + 사이드바 폭은 `[160,480]` 클램프(D9). 이 단계의 `window.rs`는 **워크스페이스 1개짜리 v2 세션**을 저장/복원한다(이름은 "워크스페이스 1", 사이드바 상태는 기본값) — 화면 동작은 이번 단계에서 변하지 않는다. ③ `layout::TreeShape`·기존 `LayoutNode`·`PanelSession`을 재사용하고 `window.rs`가 소비한다. ④ 추상화하지 않을 것: 스키마 마이그레이션 프레임워크를 만들지 않는다(v1은 폴백 — 사용자 결정).
  - **Acceptance**: Given v2 샘플 세션, When `serde_json` 직렬화 후 `parse_session`, Then 왕복 동일성이 성립한다 / v1 스키마 파일·손상 JSON·`active_workspace` 범위 밖·`active_panel` 범위 밖·리프 수 불일치는 모두 `None`(폴백) / 사이드바 폭 100·9999는 각각 160·480으로 클램프된다 / Given 분할 2개·탭 여러 개 상태, When 앱 종료 후 재실행, Then 기존과 동일하게 레이아웃·탭이 복원된다(HUMAN-VERIFY) / `cargo test` 통과(settings 테스트 전면 갱신 포함)
  - **Files**:
    - 주: `src/app/settings.rs`
    - 동반: `src/app/window.rs`(`restore_panels`·`save_current_session` v2 대응)
    - 테스트: `src/app/settings.rs` 내 `#[cfg(test)] mod tests`(기존 6개 갱신 + 신규 3개 이상)
  - **Edge Cases**:
    - v1 파일 존재 → 폴백(초기화)하고 크래시 없음
    - 이름에 따옴표·개행 등 특수문자 → serde 이스케이프 위임(개행은 T1에서 제거)
    - 디스크 쓰기 실패 → 기존과 동일하게 조용히 생략
    - `workspaces`가 빈 배열 → 폴백
  - **Halt Forecast**:
    - (ii-a) 공개 구조체 `settings::Session` 필드 변경 + `SESSION_VERSION` 1→2(직렬화 계약 변경) → `## 사전 승인 항목`에 등록
    - (i) v1 호환 정책 → 사용자 결정(초기화 폴백)으로 확정
  - **Depends on**: -

- [x] T4. 사이드바 창 — 다크 목록 렌더링·선택·생성 버튼
  - **Type**: D
  - **Design**: ① `src/app/sidebar.rs` 신규(패널 창과 같은 자식 창 패턴 — 자체 창 클래스 `FileExplorerSidebar`, 상태는 `Box<RefCell<SidebarState>>` in GWLP_USERDATA). ② `Sidebar{hwnd}` — 창 래퍼, `create(parent) -> Result<Sidebar>`, `set_items(&[Workspace], active: usize)` — 표시 데이터 갱신 후 무효화, `hwnd()`; 내부 `SidebarState{items, active, hover, scroll, fonts, brushes, icon}`, 그리기 `paint`, 순수 함수 `item_at(y, scroll, count) -> Option<usize>`·`clamp_scroll(scroll, count, view_h) -> i32`. 시각 상수(색·항목 높이·패딩 등)는 파일 상단에 `## 시각 요소 분해` 표와 1:1 선언하되, **폭 토큰(기본 232·최소 160·최대 480)은 T3에서 이미 선언된 `settings::SIDEBAR_DEFAULT_WIDTH`/`SIDEBAR_MIN_WIDTH`/`SIDEBAR_MAX_WIDTH`를 재사용하고 재선언하지 않는다**(T3 리뷰 MINOR — 세션 검증이 같은 값을 쓰므로 소유는 settings). 클릭 시 `SetFocus`(D17). ③ `workspace.rs`(표시 데이터)·`fs::icons`(폴더 아이콘)를 참조하고 부모에 `WM_APP_WS_SELECT`/`WM_APP_WS_NEW`를 게시한다 — 워크스페이스를 소유하지 않는다(표시·입력 전용). ④ 추상화하지 않을 것: 범용 리스트 컨트롤·테마 시스템을 만들지 않는다.
  - **Acceptance**: Given 더미 워크스페이스 3개를 `set_items`로 주입(이 단계에서는 `window.rs`가 고정 더미 목록을 만들어 검증 — 실제 목록 연결은 T5), When 창 표시, Then 헤더("워크스페이스")·`+` 버튼·2줄 항목이 `## 시각 요소 분해` 표의 색·치수대로 그려진다(HUMAN-VERIFY) / When 항목 클릭, Then `WM_APP_WS_SELECT`(wparam=인덱스)가 부모에 게시되고 사이드바가 포커스를 갖는다 / When `+` 클릭, Then `WM_APP_WS_NEW` 게시 / When 항목이 화면보다 많고 휠을 굴리면, Then 스크롤 오프셋이 [0,최대]로 클램프된다 / `item_at`·`clamp_scroll` 단위테스트 통과 / `cargo clippy --all-targets -- -D warnings` 통과
  - **Files**:
    - 주: `src/app/sidebar.rs`(신규)
    - 동반: `src/app/mod.rs`, `src/fs/icons.rs`(`dir_icon()` 접근자 추가), `src/app/window.rs`(사이드바 생성·WM_SIZE 배치·더미 목록 주입)
    - 테스트: `src/app/sidebar.rs` 내 `#[cfg(test)] mod tests`(순수 계산부만)
  - **Edge Cases**:
    - 항목 0개(그리기 방어) → 배경만 그리고 크래시 없음
    - 이름·경로가 폭 초과 → `DT_END_ELLIPSIS` 말줄임
    - 창 폭이 최소치 미만 → 텍스트 영역 폭 음수 방지
    - GDI 객체(폰트 2·브러시 N) 누수 → `WM_NCDESTROY`에서 전량 해제, 생성 실패 시 시스템 기본으로 저하
    - 리페인트 깜빡임 → `WM_ERASEBKGND` 1 반환 + `WM_PAINT`에서 전체 배경 채움
    - 포커스 이동 시 선택 표시 유지(포커스 유무로 항목 색을 바꾸지 않음 — 토큰 단순화)
  - **Halt Forecast**:
    - (i) 색·치수 값 → `## 시각 요소 분해` 표에서 확정
    - (ii-a) `IconCache` 공개 접근자 추가 → `## 사전 승인 항목`에 등록
  - **Depends on**: T1

- [x] T5. 워크스페이스 다중 호스트·전환·지연 생성·복원
  - **Type**: D
  - **Design**: ① `src/app/window.rs` 수정(상태 소유는 계속 메인 창). ② `AppState`를 `{sidebar: Sidebar, list: WorkspaceList, entries: Vec<EntryState>, menu, sidebar_width, sidebar_collapsed}`로 재구성. `enum EntryState { Pending(WorkspaceSession), Live(LayoutHost) }`(T3에서 정의된 타입 사용) — 미방문 워크스페이스는 세션 데이터만 보관하고 최초 선택 시 `Live`로 승격(D2). 헬퍼 `active_host(&mut self) -> Option<&mut LayoutHost>`, `materialize(&mut self, hwnd, index)`, `explorer_area(hwnd, width, collapsed) -> Rect`(클라이언트 − 사이드바 폭 − 스플리터), `refresh_subtitle(hwnd)`(D6 두 규칙 적용). 시작 시 세션의 모든 워크스페이스를 `Pending`으로 적재하고 활성 1개만 승격, 종료 시 `Live`는 수집·`Pending`은 보관 데이터를 그대로 재저장. ③ `sidebar`·`workspace`·`layout_host`·`settings`를 참조하고, 패널의 `WM_APP_PATH_CHANGED`를 받아 부제를 갱신한다. ④ 추상화하지 않을 것: 워크스페이스 매니저 타입을 따로 만들지 않는다(AppState가 소유자).
  - **Acceptance**: Given 워크스페이스 1개로 시작, When `+`로 생성, Then 홈 폴더 1패널 탐색기가 만들어지고 즉시 전환된다(FR-16) / Given A에서 폴더 이동·분할, When B로 전환 후 다시 A 선택, Then A의 분할·탭·경로·스크롤이 그대로다(FR-17, HUMAN-VERIFY) / When 활성 패널이 폴더를 이동하거나 활성 패널이 바뀌면, Then 사이드바 해당 항목 2줄이 활성 패널·활성 탭 경로로 갱신되고 **비활성 패널의 이동은 부제를 바꾸지 않는다**(D6) / 전역 명령(분할·탭·네비게이션·트리 토글·F5)이 활성 워크스페이스의 활성 패널에만 전달된다 / Given 워크스페이스 3개 저장 세션, When 재시작, Then 목록·이름·순서·활성 워크스페이스가 복원되고 **활성 워크스페이스만 탐색기가 생성**되며 미방문 항목 부제는 `panels[active_panel].tabs[active_tab]`로 표시된다(D18, FR-20) / `cargo build`·`clippy -D warnings`·`test` 통과
  - **Files**:
    - 주: `src/app/window.rs`
    - 동반: `src/panel/panel.rs`(`WM_APP_PATH_CHANGED` 게시 — 경로 커밋·탭 전환·탭 닫기 지점), `src/app/sidebar.rs`(`set_items` 호출·더미 목록 제거), `src/app/layout_host.rs`(T2 API 사용), `src/app/settings.rs`(다중 워크스페이스 저장/복원)
    - 테스트: 없음(HWND 배선 — 모델은 T1, 스키마는 T3에서 검증)
  - **Edge Cases**:
    - 같은 워크스페이스 재선택 → 재생성·재배치 없이 무시
    - 패널 창 생성 실패 → 전환하지 않고 이전 워크스페이스 유지(조용한 저하)
    - `Pending` 승격 중 탭 경로가 삭제된 폴더 → 기존 열거 실패 경로에 위임(part2 D18과 동일)
    - `WM_APP_PATH_CHANGED` 발신 HWND가 활성 워크스페이스의 활성 패널이 아니면 무시(D6)
    - 전환 도중 스플리터 드래그 중이면 → 전환 전 `end_drag(false)`
    - 세션 복원 시 워크스페이스 수가 많아도 창 생성은 1개뿐(NFR-8)
  - **Halt Forecast**:
    - (ii-a) `AppState` 구조 변경 + `restore_panels`/`save_current_session` 재작성 → `## 사전 승인 항목`에 등록
    - (ii-a) `src/panel/panel.rs`에 알림 메시지 게시 추가 → `## 사전 승인 항목`에 등록
  - **Depends on**: T1, T2, T3, T4

- [x] T6. 이름 변경·삭제·순서 변경 + 메뉴 바 워크스페이스 항목
  - **Type**: D
  - **Design**: ① `src/app/sidebar.rs`(입력·인라인 편집·드래그), `src/app/menu.rs`(메뉴 항목), `src/app/window.rs`(명령 처리·모델 갱신). ② `Sidebar::begin_rename(index)` — 항목 1줄 영역에 `EDIT` 자식 생성 + `SetWindowSubclass`로 Enter/Esc 가로채기(`address_bar` 패턴 재사용), `Sidebar::end_rename(commit: bool)`, 드래그 상태 `DragReorder{from, cursor_y, started}`(임계 8px, D12)와 순수 함수 `drop_index(cursor_y, scroll, count) -> usize`, 컨텍스트 메뉴는 `WM_APP_WS_CONTEXT`로 부모에 위임. 키 처리는 사이드바 `WM_KEYDOWN` 로컬(F2·Delete·↑/↓ — D16). 메뉴 상수 `IDM_WS_NEW`/`IDM_WS_RENAME`/`IDM_WS_DELETE` 추가. ③ 사이드바는 조작을 부모에 알리고, 모델 변경(`WorkspaceList::rename/remove/reorder`)과 `LayoutHost::destroy_all`(T2)은 `window.rs`가 수행한 뒤 `set_items`로 되돌린다(단방향 흐름). ④ 추상화하지 않을 것: 범용 드래그 프레임워크·명령 패턴을 만들지 않는다.
  - **Acceptance**: Given 항목 선택 상태, When F2(사이드바 포커스) 또는 컨텍스트 메뉴 "이름 바꾸기", Then 인라인 EDIT이 뜨고 Enter는 커밋·Esc는 취소한다(FR-18) / **Given 워크스페이스 목록, When `+` 버튼 또는 메뉴 "새로 만들기", Then 자동 이름("워크스페이스 N")이 부여된 상태로 인라인 EDIT이 즉시 열리고 Esc를 누르면 자동 이름이 유지된다(FR-16)** / When 삭제 명령, Then 확인 대화상자 후 삭제되고 해당 워크스페이스가 `Live`면 `destroy_all`로 패널 창이 전부 파괴되며 `Pending`이면 보관 데이터만 폐기된다 / 워크스페이스가 1개면 삭제 메뉴가 비활성이고 Delete 키도 무시된다 / When 항목을 8px 이상 드래그해 다른 항목 위에 놓으면, Then 순서가 바뀌고 활성 항목이 유지된다(HUMAN-VERIFY) / 메뉴 바 "워크스페이스(&W)"에 새로 만들기·이름 바꾸기·삭제가 있고 상황에 맞게 활성/비활성된다 / **주소창·인라인 EDIT에서 Delete 키로 문자 삭제가 정상 동작한다(D16 회귀 확인)** / `drop_index` 단위테스트 통과 / `cargo clippy --all-targets -- -D warnings`·`test` 통과
  - **Files**:
    - 주: `src/app/sidebar.rs`
    - 동반: `src/app/menu.rs`, `src/app/window.rs`, `src/app/layout_host.rs`(`destroy_all` 사용 — T2에서 추가)
    - 테스트: `src/app/sidebar.rs`(`drop_index`), `src/app/workspace.rs`(T1 모델 테스트)
  - **Edge Cases**:
    - 편집 중 워크스페이스 전환·삭제·창 크기 변경·포커스 상실 → 진입점마다 `end_rename(commit)` 선호출로 유령 EDIT 방지
    - 빈 이름·공백만 커밋 → 거부하고 이전 이름 유지(D7)
    - 드래그 중 캡처 상실(Alt+Tab) → `WM_CAPTURECHANGED`에서 드래그 취소
    - 자기 자신 위에 드롭 → 순서 불변
    - 마지막 1개 삭제 시도(키 경로) → 무시
    - 삭제 대상이 활성이었으면 → 인접 항목을 활성으로 전환하고 필요 시 승격
    - `Pending` 워크스페이스 삭제 → 창이 없으므로 `destroy_all` 호출 없이 데이터만 제거
  - **Halt Forecast**:
    - (ii-a) 메뉴 바에 최상위 팝업 "워크스페이스(&W)" 추가(사용자 가시 UI 변경) → `## 사전 승인 항목`에 등록
  - **Depends on**: T5

- [ ] T7. 사이드바 접기·폭 조절 + 상태 저장 배선
  - **Type**: C
  - **Design**: ① `src/app/window.rs`(경계 히트테스트·드래그·영역 재계산·세션 반영), `src/app/sidebar.rs`(상단 토글 버튼 그리기·클릭), `src/app/menu.rs`(보기 메뉴 항목·Ctrl+B). ② 신규 심볼: `window.rs`의 `SidebarDrag{start_x, start_width}` 상태, `toggle_sidebar(hwnd)`·`sidebar_splitter_rect(hwnd) -> Rect`·`clamp_sidebar_width(w: i32) -> i32`(순수 함수, 테스트 대상), `menu::IDM_SIDEBAR_TOGGLE`. T5의 `explorer_area`가 폭·접힘을 반영하도록 값만 연결하고, 종료 저장 시 `settings::SidebarSession{width, collapsed}`에 실제 값을 기록한다(T3 스키마 사용). ③ 기존 흐름에 상태 1쌍을 추가할 뿐 새 의존 방향은 없다. ④ 추상화하지 않을 것: `layout_host`의 스플리터 드래그 로직을 공용화하지 않는다(대상이 단일 경계선 — 4-D).
  - **Acceptance**: Given 사이드바 표시 상태, When 경계선 드래그, Then 폭이 160~480px로 클램프되며 탐색기 영역이 즉시 재배치된다(FR-19) / When Ctrl+B 또는 보기 메뉴 또는 상단 토글 버튼, Then 사이드바가 접히고 탐색기가 창 전체를 차지한다 / 접힌 상태에서 Ctrl+B·메뉴로 복원된다(D11) / 경계선 위에서 좌우 리사이즈 커서가 표시된다 / Given 폭 320·접힘 해제 상태, When 재시작, Then 같은 폭·상태로 복원된다(FR-20, HUMAN-VERIFY) / `clamp_sidebar_width` 단위테스트 통과 / `cargo clippy --all-targets -- -D warnings`·`test` 통과
  - **Files**:
    - 주: `src/app/window.rs`
    - 동반: `src/app/sidebar.rs`, `src/app/menu.rs`
    - 테스트: `src/app/window.rs` 내 `#[cfg(test)] mod tests`(`clamp_sidebar_width`)
  - **Edge Cases**:
    - 창 폭이 작아 사이드바+최소 탐색기 폭을 못 만드는 경우 → 사이드바를 최소 160px까지 축소, 그래도 부족하면 탐색기 영역 폭이 0 이하가 되지 않게 배치 생략
    - 접힌 상태에서 폭 드래그 시도 → 경계선 히트테스트 비활성
    - 드래그 중 캡처 상실 → 드래그 취소(현재 폭 유지)
    - 접기/펼치기 직후 배치 누락 → 항상 `set_area` + `relayout` 짝 호출
    - 접힌 상태로 종료 → 다음 실행도 접힌 상태(복원 경로는 메뉴·Ctrl+B)
  - **Halt Forecast**:
    - (ii-a) 보기 메뉴 항목 + Ctrl+B 액셀러레이터 추가 → `## 사전 승인 항목`에 등록
    - (i) 접힘 복원 경로 → D11에서 확정
  - **Depends on**: T5

- [ ] T8. 전체 검증 — 성능 실측·시각 대조·회귀
  - **Type**: C
  - **Design**: ① 검증 스크립트는 시스템 임시 폴더(스크래치)에 두고 저장소에 남기지 않는다(part2 T5 방식 — `APPDATA` 리다이렉트로 사용자 설정 미오염). ② 신규 심볼 없음(코드 수정은 실측 미달 시에만). ③ 대상은 릴리즈 빌드 산출물. ④ 추상화하지 않을 것: 벤치마크 하네스를 저장소에 추가하지 않는다.
  - **Acceptance**: NFR-8 — 워크스페이스 5개를 각각 1회 방문한 상태의 유휴 Working Set이 100MB 미만(측정값 기록) / NFR-1 회귀 — 시작→창 표시 1초 미만 / NFR-2 회귀 — 워크스페이스 1개·패널 2개 유휴 50MB 미만 / 워크스페이스 5개 생성 후 4개 삭제 시 Working Set이 생성 전 수준으로 회수된다(누수 확인) / `## 시각 요소 분해` 표의 모든 항목이 코드 상수와 1:1 일치(색·치수는 `sidebar.rs`, 폭 토큰은 `settings::SIDEBAR_*` — 코드 대조) / `cargo build --release`·`cargo clippy --all-targets -- -D warnings`·`cargo fmt --check`·`cargo test` 전부 통과 / 미달 항목은 원인·조치를 Progress Log에 기록
  - **Files**:
    - 주: (없음 — 측정·대조. 미달 시 해당 소스 수정)
    - 테스트: `cargo test` 전체
  - **Edge Cases**:
    - 측정 편차 → 3회 측정 중앙값(part2 T5 관례)
    - 사용자 실제 설정 파일 오염 방지 → `APPDATA` 리다이렉트, 측정 후 임시 데이터 삭제
    - NFR-8 미달 → 원인 분리(패널 창 수 vs 아이콘 캐시 vs 감시 스레드) 후 조치, 3회 실패 시 보고 후 중단
  - **Halt Forecast**:
    - (ii-b) 실측 미달로 D2(지연 생성 후 유지) 또는 D1(배치 구조) 자체를 바꿔야 하면 plan에 없던 설계 변경 → `## 불가피한 Halt`
  - **Depends on**: T1, T2, T3, T4, T5, T6, T7

## 사전 승인 항목 (일괄 승인 대상)

- T2 — `LayoutHost::new`/`from_shape`에 area 인자 추가, `relayout`/`close_active`의 `parent` 파라미터 제거, `set_area`/`set_visible`/`destroy_all` 공개 메서드 추가. 소비자는 `src/app/window.rs` 단독이며 함께 수정된다
- T3 — 공개 구조체 `settings::Session` 필드 변경 및 `SESSION_VERSION` 1 → 2(직렬화 계약 변경, v1 파일은 폴백)
- T4 — `fs::icons::IconCache`에 공개 접근자 `dir_icon()` 추가(읽기 전용, 기존 동작 불변)
- T5 — `AppState` 구조 변경 및 `restore_panels`/`save_current_session` 재작성(워크스페이스 단위 처리)
- T5 — `src/panel/panel.rs`에 경로 변경 알림(`WM_APP_PATH_CHANGED`) 게시 추가(기존 동작 불변, 부모가 무시해도 무해)
- T6 — 메뉴 바에 최상위 팝업 "워크스페이스(&W)" 추가
- T7 — 보기 메뉴에 "워크스페이스 사이드바" 항목 + Ctrl+B 액셀러레이터 추가
- T1·T4 — `src/app/mod.rs`에 신규 모듈(`workspace`, `sidebar`) 등록

## 불가피한 Halt (위임 불가)

- commit 이후의 push·태그·릴리즈·PR (구현·검증 완료 후 별도 승인)
- T8 실측 미달로 D2(비활성 워크스페이스 유지 정책) 또는 D1(배치 구조)을 바꿔야 하는 경우 — plan에 근거 없는 설계 변경
- 사용자 `%APPDATA%\FileExplorer\settings.json` 실파일 삭제·덮어쓰기(측정은 리다이렉트로 수행하며, 실파일 조작이 필요해지면 중단)

## Known Workarounds (있는 경우만)

- 사이드바 인라인 편집 EDIT은 시스템 기본(밝은) 배경을 사용한다 — 다크 EDIT 커스텀 렌더는 이번 범위 밖(편집 중 짧은 순간만 노출). 근본 해결은 앱 전체 테마 도입 시 함께 처리(Deferred)
- 사이드바 치수는 96DPI 고정 px이라 고DPI 환경에서 작게 보인다 — 앱 전체가 동일 관례(D13)이며 전면 DPI 스케일 도입은 별도 작업
- "비활성 창 스크롤"을 끈 환경에서는 사이드바에 포커스가 있을 때만 휠 스크롤이 동작한다(D17) — 키보드 ↑/↓로도 이동 가능

## Verification Strategy

- 빌드: `cargo build` / `cargo build --release`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 포맷: `cargo fmt --check`
- 단위·통합 테스트: `cargo test`
- 성능 측정(T8): part2 T5 방식 — 스크래치 폴더 스크립트, `APPDATA` 리다이렉트, 3회 측정 중앙값
- 수동 검증(HUMAN-VERIFY): ① 사이드바 시각(색·간격·2줄) ② 워크스페이스 생성·전환 시 상태 보존 ③ 이름 변경·삭제·드래그 정렬 ④ 접기·폭 드래그 ⑤ 재시작 후 복원 ⑥ 주소창 Delete 키 회귀

## Phase Ledger

## Retry Ledger

## Progress Log
- T1-T2 완료 (커밋 861e4e1, T2 pre-review f0e1870): ① `src/app/workspace.rs` 신규 — `WorkspaceList`(add/rename/remove/reorder/set_active/set_subtitle)·자동 이름(사용 중이지 않은 최소 번호)·`elide_path`, 단위테스트 13개. ② `LayoutHost`에 `area` 주입 — `new`/`from_shape`에 area 인자, `relayout`/`close_active`/`drag_move`의 `parent` 제거, `set_area`/`set_visible`/`destroy_all` 추가, `client_rect`를 `pub`으로 승격해 window.rs가 영역을 계산·주입(WM_SIZE에서 `set_area`+`relayout` 짝 호출).
  - 결정: `drag_move`의 `parent` 제거는 Design에 없던 파생이지만 `relayout()` 변경으로 미사용 파라미터가 되어 함께 정리(리뷰에서 정상 파생 확인).
  - 결정: 테스트 함수명에 영문 대문자(`NotFound`)를 쓰면 `-D warnings`의 `non_snake_case`에 걸린다 — 한글 이름으로 통일.
- T3-T4 완료 (커밋 c4d8ed1, T4 pre-review 1e6c225): ① 세션 스키마 v2(`Session{sidebar, active_workspace, workspaces[{name,layout,panels,active_panel}]}`) + 워크스페이스 단위 검증·폭 클램프, window.rs는 워크스페이스 1개로 저장/복원. ② `src/app/sidebar.rs` 신규 — 커스텀 창 클래스 `FileExplorerSidebar`, WM_PAINT 직접 그리기(다크 2줄 카드·헤더·`+` 버튼·선택 강조 바·시스템 폴더 아이콘), 휠 스크롤·hover(TrackMouseEvent)·클릭 시 SetFocus, 부모에 `WM_APP_WS_SELECT`/`WM_APP_WS_NEW` 게시. window.rs는 `explorer_area`/`layout_children`로 사이드바+탐색기를 분할 배치.
  - 결정: 사이드바 폭 토큰(232/160/480)은 세션 검증이 같은 값을 쓰므로 `settings`가 소유하고 sidebar는 재선언 없이 참조(T3 spec 리뷰 MINOR 해소).
  - 결정: T4 단계의 목록은 `sample_list()` 임시 3개 — T5에서 세션 기반 실제 목록으로 교체. `AppState`에는 아직 목록을 보관하지 않는다(미사용 필드 경고 회피).
  - V-9: 데스크톱 UI라 렌더 확인 불가 → 시각 행 전부 `⏳ 미확인`으로 F-8 인계, 코드 상수 대조는 전 행 ✅.
- T5-T6 완료 (커밋 c16c846, T6 review-fix a4f07d1): ① AppState를 `list + entries(Pending/Live)` 구조로 재편해 워크스페이스별 탐색기를 지연 생성·숨김 유지로 전환, 전역 명령·드래그·활성 패널 추적을 활성 워크스페이스 경유로 변경, 부제는 `WM_APP_PATH_CHANGED`(panel.rs 신규) + 활성 패널 전환 시 갱신. ② 인라인 이름 편집(EDIT+서브클래스)·삭제(확인 대화상자+destroy_all)·드래그 정렬(8px 임계·중앙 기준 삽입선)·컨텍스트 메뉴·메뉴 바 "워크스페이스(&W)" 추가.
  - 결정: F2·Delete·↑/↓는 사이드바 `WM_KEYDOWN` 로컬 처리로 한정(D16) — 액셀러레이터는 포커스 무관하게 소비되어 주소창 Delete가 회귀하기 때문.
  - 결정(리뷰 M1): `layout_children`은 `MoveWindow` 전에 AppState 차용을 놓는다 — 사이드바 리사이즈가 동기 WM_SIZE→편집 커밋→부모 SendMessage로 재진입해 차용 중이면 이름 변경이 조용히 유실된다.
  - 결정(리뷰 M2·m1): 창 생성 시 `update_workspace_enabled` 초기화, 서브클래스는 Enter/Esc만 기본 처리를 삼키고 WM_KILLFOCUS는 위임.

## Next Steps

## Open Questions

- [x] Q1: 사이드바 항목 표시 정보 → **이름 + 활성 탭 폴더 경로(자동) 2줄**
- [x] Q2: 관리 기능 범위 → **이름 변경 · 삭제 · 드래그 순서 변경 모두 포함**
- [x] Q3: 사이드바 외형 → **첨부 이미지와 동일한 다크 카드**(PRD Out of Scope를 "앱 전체 테마 전환"으로 한정 수정, 승인 완료)
- [x] Q4: 기존 v1 세션 처리 → **초기화 폴백**(마이그레이션 없음)
- [x] Q5: 사이드바 폭 동작 → **접기/펼치기 토글 + 스플리터 폭 조절 둘 다**
- [x] Q6: 비활성 워크스페이스 → **지연 생성 후 숨김 유지**
- [x] Q7: 새 워크스페이스 초기 상태 → **홈 폴더 1패널 + 자동 이름 후 인라인 편집**
- [x] Q8: 명령 진입점 → **사이드바(+ 버튼·컨텍스트 메뉴) + 메뉴 바**
