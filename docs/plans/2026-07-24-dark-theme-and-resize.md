# Plan: 탐색기 다크 테마 + 사이드바 크기 조절 잔상 개선

**PRD**: docs/prd.md (FR-15 관련 · Out of Scope "앱 전체 다크 모드" 갱신 필요 — 아래 PRD 갱신안 참조)

## 요구 이해

> 원문: "① 사이즈 조절시 부드럽게 움직이지 않고 잔상도 보임 ② 탐색기 화면도 모두 다크 테마로 표시 ③ 워크스페이스 패널의 문자가 너무 작은데 조금 크게 표시"

- ③(폰트 확대)은 이미 이 세션에서 처리 완료(`NAME_FONT_PX` 13→15, `SUBTITLE` 11→13, `HEADER` 12→14). 이 계획의 대상은 ①·②.
- ①: 사이드바-탐색기 경계선을 드래그해 사이드바 폭을 조절할 때 화면이 부드럽지 않고 잔상이 남는다. `WS_CLIPCHILDREN` 추가로 흰 깜빡임은 줄었으나, 배치가 두 단계(사이드바 `MoveWindow` + 탐색기 `DeferWindowPos`)로 분리되고 사이드바 그리기에 더블버퍼링이 없어 잔상이 남는다.
- ②: 탐색기 영역 전체(파일 목록·폴더 트리·주소창·탭·타이틀바·메뉴바)를 고정 다크 스타일로 표시한다. 사용자가 범위를 **전부**로 확정(메뉴바 포함). 테마 전환 UI는 없다(사이드바처럼 고정 다크).

## Goal

사이드바 크기 조절을 부드럽게(잔상 제거) 만들고, 탐색기 영역 전체를 사이드바와 통일된 고정 다크 스타일로 전환한다.

**검증 체크포인트(HUMAN-VERIFY)**: 다크·잔상은 빌드로 검증 불가라 시각 확인이 필요하다. 권장 확인 지점 — T1 후(잔상 소멸), T2 후(타이틀바·배경 기본 다크), T7 후(전체 다크). 자율 실행은 빌드·clippy 통과까지 진행하고, 시각 회귀는 완료 보고 시 사용자 확인 항목으로 집계한다.

## Investigation Log

- 메인 창(`window.rs:146-165`): `WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN`(이번 세션 추가), 배경 브러시 `COLOR_WINDOW+1`(흰색), 클래스 스타일 `CS_HREDRAW|CS_VREDRAW`. 별도 `WM_ERASEBKGND` 핸들러 없음 → `DefWindowProc`가 흰 브러시로 erase.
- 메인 창의 직접 자식 = 사이드바(`window.rs`가 `MoveWindow`로 직접 배치) + 탐색기 패널들(`layout_host`가 `DeferWindowPos`로 배치). 패널(`panel.rs`)은 다시 file_list/folder_tree/tabs/address_bar의 부모.
- `layout_children`(`window.rs:373-403`): 사이드바 `MoveWindow(…, true)`(bRepaint=true) 후 `host.set_area()`+`host.relayout()`. 두 배치가 분리 → 시차. 사이드바는 매 이동마다 `WM_SIZE`→전체 무효화→`WM_PAINT`.
- 사이드바 `paint`(`sidebar.rs:521-533`): `BeginPaint` DC에 직접 그림(배경→헤더→항목). **더블버퍼링(메모리 DC) 없음** → 크기 조절 중 요소별 순차 그리기가 잔상/깜빡임으로 보임. `WM_ERASEBKGND`=`LRESULT(1)`(안 지움), `WM_SIZE`에서 `invalidate` 전체 무효화.
- 파일 목록(`file_list.rs:59`): `WC_LISTVIEWW`, `WS_EX_CLIENTEDGE`, LVS_REPORT|OWNERDATA. 다크: `SetWindowTheme("DarkMode_Explorer")` + `LVM_SETBKCOLOR`/`SETTEXTCOLOR`/`SETTEXTBKCOLOR` + 헤더는 별도 커스텀드로우.
- 폴더 트리(`folder_tree.rs:54`): `WC_TREEVIEWW`, `WS_EX_CLIENTEDGE`. 다크: `SetWindowTheme("DarkMode_Explorer")` + `TVM_SETBKCOLOR`/`SETTEXTCOLOR`/`SETLINECOLOR`.
- 주소창(`address_bar.rs:43`): `WC_EDIT`, `WS_EX_CLIENTEDGE`. 다크: 부모(패널)의 `WM_CTLCOLOREDIT`로 배경/글자 브러시 반환.
- 탭(`tabs.rs:138`): `WC_TABCONTROLW`. 테마 다크가 안 먹음 → `TCS_OWNERDRAWFIXED` + 부모 `WM_DRAWITEM`로 직접 그리기.
- 부모 패널(`panel.rs`): `WM_NOTIFY`(568), `WM_DRAWITEM|WM_MEASUREITEM`(731, 셸 메뉴용) 이미 처리 — 다크 커스텀드로우(`NM_CUSTOMDRAW`)·탭 오너드로우·`WM_CTLCOLOR*` 배선을 여기에 추가.
- 메뉴바: Win32 메뉴 다크는 표준 API가 없음. `MFT_OWNERDRAW` 오너드로우(`WM_MEASUREITEM`/`WM_DRAWITEM`, 메뉴 소유 창=메인 창) 방식. 팝업 배경은 언문서라 완전한 다크는 어려움 — 최상위 메뉴바 항목 위주.
- Cargo.toml features: `Win32_Graphics_Gdi`·`Win32_UI_Controls`·`Win32_UI_WindowsAndMessaging` 있음. **`Win32_Graphics_Dwm` 없음** → 타이틀바 다크(`DwmSetWindowAttribute` + `DWMWA_USE_IMMERSIVE_DARK_MODE`)에 추가 필요.
- 사이드바 다크 팔레트 이미 정의(`sidebar.rs:105-114`): `COLOR_BG=0x1B1B1B`, `COLOR_NAME=0xE8E8E8`, `COLOR_SUBTITLE=0x8A8A8A` 등. 탐색기 팔레트는 이와 통일.
- 위키 참조: vault 미설정 — 코드 1차 출처로 진행.

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 | 재사용/신규 사유 |
|---|---|---|
| `app/theme.rs`(다크 색상 상수 + 적용 헬퍼) | 사이드바가 `sidebar.rs` 내부에 다크 색상 const 보유(모듈 사설) | 신규 — 사이드바 상수는 사설이고 탐색기/메뉴가 공유할 공용 팔레트가 없다. 사이드바 리팩토링(사설→공용 이전)은 범위 확대라 이번엔 안 함(신규 모듈에 탐색기용만 정의, 값은 사이드바와 통일). |
| 더블버퍼 그리기 헬퍼(T1) | 없음(사이드바가 유일한 커스텀 그리기 창) | 신규 — 사이드바 `paint`에 국한 적용. |
| `apply_dark_titlebar`(T2) | 없음 | 신규 — DWM 다크 속성 얇은 래퍼. |
| `draw_tab`(T6)·네비 버튼 그리기(T5)·메뉴 오너드로우 헬퍼(T7) | 셸 메뉴가 `panel.rs:731` WM_DRAWITEM 처리 중이나 용도 상이 | 신규 — 각 컨트롤 전용 오너드로우. 셸 메뉴 처리와 컨트롤 ID로 분기 분리(공용 프레임워크화 안 함). |

## Impact Analysis

- **4-A 심볼 추적**: `layout_children`(호출: `window.rs` WM_SIZE·생성·`toggle_sidebar`·`drag_sidebar` — 이번 T1이 내부 구현만 바꾸고 시그니처 유지 → 호출부 무영향). `paint`/`sidebar_proc`은 sidebar.rs 내부. 각 컨트롤 `create`는 해당 패널이 1회 호출.
- **4-B 계약·직렬화**: 없음. 다크는 렌더링만 — 세션 스키마·공개 계약 불변.
- **4-C 테스트**: 순수 로직 변경 없음(색상·그리기는 UI). 기존 단위테스트(레이아웃·정렬·히스토리 등)에 영향 없음. 다크·잔상은 UI라 HUMAN-VERIFY.
- **4-D**: 위 표.

## 진행 체크리스트

- [x] T1. 사이드바 크기 조절 잔상 개선 — 배치 통합 + 더블버퍼링
- [x] T2. 다크 인프라 + 메인 창(타이틀바·배경·splitter)
- [x] T3. 파일 목록(ListView) 다크
- [ ] T4. 폴더 트리(TreeView) 다크
- [ ] T5. 주소창(Edit) + 네비 버튼 다크
- [ ] T6. 탭 컨트롤 오너드로우 다크
- [ ] T7. 메뉴바 오너드로우 다크

## Progress Log

- T1 완료 (ae08343): `LayoutHost::defer_into`/`pane_count` 추가 → `layout_children`이 사이드바+패널을 단일 `DeferWindowPos` 배치로 통합. 사이드바 `paint`에 메모리 DC 더블버퍼링. 빌드·clippy·test(75+2) OK, spec/quality 리뷰 OK.
- T2 완료: `app/theme.rs` 신규(WINDOW_BG + apply_dark_titlebar), Cargo.toml에 Win32_Graphics_Dwm 추가, 메인 창 배경 브러시 다크(CreateSolidBrush) + DWM 다크 타이틀바. 빌드·clippy·test OK, spec/quality 리뷰 OK. theme 색은 각 후속 task에서 필요분만 추가(YAGNI).

## Tasks

### T1: 사이드바 크기 조절 잔상 개선 — 배치 통합 + 더블버퍼링

- **Type**: C
- **파일**: `src/app/window.rs`(`layout_children`), `src/app/sidebar.rs`(`paint` 더블버퍼링)
- **Design**:
  - ① 배치: `layout_children`에서 사이드바+탐색기 패널을 한 `BeginDeferWindowPos`/`EndDeferWindowPos` 배치로 묶는다. `layout_host`에 사이드바 HWND·위치를 함께 넘기는 대신, `window.rs`에서 `host`의 패널 목록을 받아 사이드바와 함께 defer한다 — `LayoutHost`에 `defer_into(hdwp, …)` 형태의 배치 위임 메서드를 추가해 window.rs가 하나의 hdwp에 사이드바+패널을 모두 넣는다.
  - ② 신규 심볼: `LayoutHost::defer_into(&self, hdwp) -> hdwp`(기존 `relayout`의 배치 로직을 외부 hdwp로 수행) — 책임: 자기 패널들을 주어진 defer 배치에 추가. `sidebar::paint`에 메모리 DC 더블버퍼(CreateCompatibleDC/Bitmap→그리기→BitBlt→해제).
  - ③ 의존 방향: window.rs → layout_host(defer_into 호출). sidebar는 자기 paint만.
  - ④ 비추상화: 범용 "레이아웃 엔진" 추상화 안 함. 더블버퍼 헬퍼는 사이드바 paint 내부에 국한(공용 유틸 승격 안 함 — 사용처 1곳).
- **Acceptance**:
  - 사이드바 폭 드래그 중 사이드바와 탐색기가 한 프레임에 함께 이동한다(시차 소멸 — 코드상 단일 defer 배치 확인).
  - 사이드바 그리기가 메모리 DC 경유 후 1회 BitBlt(코드 확인). 빌드·clippy·기존 테스트 통과.
  - 드래그 시 잔상·깜빡임 소멸(HUMAN-VERIFY).
- **Edge Cases**: 사이드바 접힘(폭 0)일 때 defer 대상에서 제외. 패널 0개(destroy 직후) 방어. hdwp 실패 시 `EndDeferWindowPos`까지 안전 종료.
- **Halt Forecast**: 없음(내부 구현·시그니처 유지).

### T2: 다크 인프라 + 메인 창(타이틀바·배경·splitter)

- **Type**: D
- **파일**: `src/app/theme.rs`(신규), `src/app/mod.rs`(모듈 등록), `src/app/window.rs`(배경 브러시·DWM·splitter 색), `Cargo.toml`(`Win32_Graphics_Dwm` 추가)
- **Design**:
  - ① 배치: `app/theme.rs`에 다크 팔레트 상수(배경·글자·선택·경계 등, 사이드바 값과 통일)와 헬퍼(`apply_dark_titlebar(hwnd)`, `enable_dark_controls(hwnd)` 얇은 래퍼).
  - ② 신규 심볼: `theme::COLOR_*` 상수군, `apply_dark_titlebar(hwnd)`(DWM 다크 속성 세팅), 메인 창 배경 브러시를 다크로.
  - ③ 의존 방향: window.rs·panel 계열 → theme(색·헬퍼 참조). theme는 windows API만 의존.
  - ④ 비추상화: 테마 "전환" 시스템 안 만듦(고정 다크 — 라이트/다크 토글 상태·설정 없음). 색상은 const, 런타임 스왑 없음.
- **Acceptance**: 타이틀바가 다크(DWM 속성 적용 코드 확인). 메인 창 배경·splitter 영역이 다크색(흰 splitter 소멸). 빌드·clippy 통과. 화면 HUMAN-VERIFY.
- **Edge Cases**: DWM 속성 미지원 OS(구버전)에서 실패해도 앱 정상 — 반환값 무시(다크만 안 됨). Windows 11 대상이라 지원 가정.
- **Halt Forecast**: **Cargo.toml feature 추가**(`Win32_Graphics_Dwm`) → 사전 승인 항목(비파괴 의존성 feature 확장).

### T3: 파일 목록(ListView) 다크

- **Type**: C
- **파일**: `src/panel/file_list.rs`(생성 후 다크 색·SetWindowTheme), `src/panel/panel.rs`(헤더 `NM_CUSTOMDRAW` 다크)
- **Design**: 생성 직후 `SetWindowTheme(hwnd,"DarkMode_Explorer")` + `LVM_SETBKCOLOR`/`SETTEXTCOLOR`/`SETTEXTBKCOLOR`(theme 색). 목록 헤더(SysHeader32)는 **ListView의 자식**이라 그 `NM_CUSTOMDRAW`가 ListView로 가고 패널로 직접 오지 않을 수 있음 — 필요 시 헤더 핸들(`LVM_GETHEADER`)을 서브클래싱하거나 ListView를 서브클래싱해 헤더 커스텀드로우를 받는다. 빌드 무관(헤더만 라이트로 남는 형태)이라 HUMAN-VERIFY로 확인. 신규 심볼 없음(기존 create 확장) → Design 4요소 생략.
- **Acceptance**: 목록 배경·글자·헤더가 다크(코드 확인 + HUMAN-VERIFY). 정렬·가상목록 동작 회귀 없음(기존 테스트 통과).
- **Edge Cases**: 빈 폴더(항목 0)·선택 강조 다크 대비. `WS_EX_CLIENTEDGE` 흰 테두리 → 필요 시 제거/다크.
- **Halt Forecast**: 없음 — 기존 create 확장 + 표준 색상 메시지(LVM_SET*COLOR), 신규 외부 의존 없음. 헤더 서브클래싱은 표준 `SetWindowSubclass`(주소창에서 이미 사용).

### T4: 폴더 트리(TreeView) 다크

- **Type**: C
- **파일**: `src/panel/folder_tree.rs`(생성 후 다크 색·SetWindowTheme)
- **Design**: `SetWindowTheme(hwnd,"DarkMode_Explorer")` + `TVM_SETBKCOLOR`/`SETTEXTCOLOR`/`SETLINECOLOR`(theme 색). 기존 create 확장 → Design 4요소 생략.
- **Acceptance**: 트리 배경·글자·연결선이 다크(코드 확인 + HUMAN-VERIFY). 지연 확장·선택 동작 회귀 없음.
- **Edge Cases**: 확장 아이콘(+/-) 대비, 선택 항목 다크 강조.
- **Halt Forecast**: 없음 — 기존 create 확장 + 표준 TVM_SET*COLOR 메시지, 신규 외부 의존 없음.

### T5: 주소창(Edit) + 네비 버튼 다크

- **Type**: D
- **파일**: `src/panel/panel.rs`(`WM_CTLCOLOREDIT`·버튼 `WM_DRAWITEM` 처리), `src/panel/address_bar.rs`(Edit 다크 브러시 보관 + 버튼 `BS_OWNERDRAW` 스타일)
- **Design**:
  - ① 배치: Edit는 부모 패널 `WM_CTLCOLOREDIT`로 다크 배경 브러시 + `SetTextColor`/`SetBkColor`. 네비 버튼(←→↑)은 **별도 BUTTON 컨트롤 3개로 확정**(`address_bar.rs:37-39`) — 표준 버튼은 `WM_CTLCOLORBTN`만으로 버튼 면이 완전 다크가 안 되므로 `BS_OWNERDRAW` + 부모 `WM_DRAWITEM`로 직접 그린다(다크 배경·글리프·hover/press 상태).
  - ② 신규 심볼: `address_bar.rs`에 버튼 그리기 헬퍼(`draw_nav_button(hdc, rect, glyph, state)`), panel `WM_DRAWITEM`의 버튼 ID(ID_NAV_BACK/FORWARD/UP) 분기.
  - ③ 의존: panel WM_DRAWITEM → address_bar 버튼 그리기 → theme 색.
  - ④ 비추상화: 버튼·탭·메뉴 오너드로우를 공용 프레임워크로 묶지 않음(각 전용, ID로 분기). 셸 메뉴 WM_DRAWITEM(panel.rs:731)과 버튼 ID로 분리.
- **Acceptance**: 주소창 배경·입력 글자 + 네비 버튼 3개가 모두 다크(코드 확인 + HUMAN-VERIFY). Enter 네비게이션·버튼 클릭·포커스 회귀 없음(기존 히스토리 테스트 통과).
- **Edge Cases**: 텍스트 선택 하이라이트 대비, 캐럿 가시성, 버튼 비활성(뒤로 갈 곳 없음) 상태 대비, 셸 메뉴 WM_DRAWITEM과 버튼 ID 충돌 방지.
- **Halt Forecast**: 없음 — `WM_CTLCOLOR*`·버튼 오너드로우 모두 표준 메시지, 신규 외부 의존 없음.

### T6: 탭 컨트롤 오너드로우 다크

- **Type**: D
- **파일**: `src/panel/tabs.rs`(생성 시 `TCS_OWNERDRAWFIXED`), `src/panel/panel.rs`(`WM_DRAWITEM` 탭 분기)
- **Design**:
  - ① 배치: 탭 생성 스타일에 `TCS_OWNERDRAWFIXED` 추가. 부모 패널 `WM_DRAWITEM`에서 탭 컨트롤 ID 분기 → 직접 그리기(활성/비활성 배경·글자·경계 theme 색).
  - ② 신규 심볼: `tabs.rs`에 탭 아이템 그리기 헬퍼(`draw_tab(hdc, rect, label, active)`), panel `WM_DRAWITEM` 탭 분기.
  - ③ 의존: panel WM_DRAWITEM → tabs 그리기 헬퍼 → theme 색.
  - ④ 비추상화: 범용 오너드로우 프레임워크 안 만듦(탭 전용). 셸 메뉴 기존 WM_DRAWITEM 처리와 ID로 분리(공용화 안 함).
- **Acceptance**: 탭 배경·글자·활성 강조가 다크(코드 확인 + HUMAN-VERIFY). 탭 전환·복제(Ctrl+T)·닫기(Ctrl+W) 회귀 없음(기존 TabsModel 테스트 통과).
- **Edge Cases**: 탭 다수로 폭 초과 시 잘림, 활성/비활성/hover 상태 구분, 셸 메뉴 WM_DRAWITEM(731)과 탭 ID 충돌 방지.
- **Halt Forecast**: 없음(오너드로우는 표준 API).

### T7: 메뉴바 오너드로우 다크

- **Type**: D
- **파일**: `src/app/window.rs`(`WM_MEASUREITEM`/`WM_DRAWITEM` 메뉴 분기), `src/app/menu.rs`(`MFT_OWNERDRAW` 세팅)
- **Design**:
  - ① 배치: 메뉴 항목을 `MFT_OWNERDRAW`로 등록(`menu.rs`). 메뉴 소유 창(메인 창, `window.rs`)의 `WM_MEASUREITEM`(크기)·`WM_DRAWITEM`(그리기)에서 다크 배경/글자.
  - ② 신규 심볼: `menu.rs`에 오너드로우 등록 헬퍼, window.rs 메뉴 그리기 분기.
  - ③ 의존: window.rs 그리기 → theme 색. menu.rs 등록.
  - ④ 비추상화: 팝업 하위 메뉴 완전 다크는 언문서라 시도 범위 한정(최상위 메뉴바 + 가능한 항목). 범용 메뉴 테마 시스템 안 만듦.
  - ⑤ 리스크: Win32 메뉴 다크는 불완전할 수 있음(팝업 배경 등 일부 시스템 색 잔존 가능) — 사용자 고지 완료.
- **Acceptance**: 메뉴바 최상위 항목(파일·워크스페이스 등)이 다크 배경/글자(코드 확인 + HUMAN-VERIFY). 메뉴 클릭·단축키·팝업 동작 회귀 없음. 완전한 팝업 다크는 best-effort.
- **Edge Cases**: 선택/hover 항목 강조, 비활성(gray) 항목 대비, 팝업 배경 시스템 색 잔존 허용(best-effort), 메뉴 텍스트 `&` 니모닉 유지.
- **Halt Forecast**: 없음(오너드로우 표준 — 팝업 배경 한계는 acceptance에서 best-effort로 수용).

## Decisions

- **D1 (위치)**: 다크 팔레트를 `app/theme.rs` 신규 모듈에 둔다. Source: 사이드바 색상은 `sidebar.rs` 사설 const(105-114) — 공유 팔레트 부재. 사이드바 리팩토링은 범위 외라 신규 모듈에 탐색기용만 정의(값은 통일).
- **D2 (방식)**: Win32 다크모드 + 컨트롤별 색상 지정(SetWindowTheme·SETBKCOLOR·CTLCOLOR·오너드로우). 고정 다크(전환 UI·설정 없음). Source: PRD Out of Scope는 "테마 전환 기능" 제외 — 고정 다크는 사이드바 선례(FR-15)와 정합.
- **D3 (T1 배치)**: 사이드바+패널을 단일 `DeferWindowPos` 배치로 통합. Source: 현재 분리 배치가 잔상 원인(Investigation Log). `LayoutHost`에 외부 hdwp 배치 위임 메서드 추가(기존 relayout 로직 재사용).
- **D4 (의존성)**: `Win32_Graphics_Dwm` feature 추가 — 다크 타이틀바(`DwmSetWindowAttribute`)는 표준 API로 직접 구현, 대체 불가(feature 없이 심볼 미정의). 사전 승인 항목.

## PRD Coverage

이 작업은 기존 PRD의 Out of Scope를 갱신하고 신규 FR을 추가한다(아래 갱신안 — 사용자 승인 후 적용).

| PRD ID | 우선순위 | 대응 task | 상태 |
|---|---|---|---|
| FR-21(신규): 탐색기 영역 고정 다크 | Should(제안) | T2~T7 | ✅ 커버 |
| FR-15(사이드바 다크·2줄) | Must | (기구현) | 이번 범위 외 (기구현) |
| 그 외 active Must FR-1~FR-20 | Must | (기구현) | 이번 범위 외 (기구현) |

**PRD 갱신안(승인 후 적용)**:
- Out of Scope 55행 수정: "앱 전체 다크 모드·테마 전환 기능" → "**테마 전환 UI**(라이트/다크 토글) 제외. 탐색기·사이드바의 고정 다크 스타일은 FR-15·FR-21에 포함".
- FR-21 신규 추가: "탐색기 영역(파일 목록·폴더 트리·주소창·탭·타이틀바·메뉴바)을 고정 다크 스타일로 표시한다(전환 UI 없음). 메뉴바 팝업 배경은 Win32 제약상 best-effort." 우선순위 Should. 검증: 코드 확인 + 화면 HUMAN-VERIFY.
- 변경 이력 1줄 추가.

## 사전 승인 항목 (일괄 승인 대상)

- `Cargo.toml`에 `Win32_Graphics_Dwm` feature 추가(비파괴 의존성 feature 확장 — 다크 타이틀바용, T2).
- `src/app/theme.rs` 신규 모듈 + `src/app/mod.rs` 모듈 등록(구조 추가, T2).
- PRD 갱신(Out of Scope 문구 수정 + FR-21 추가) — 위 갱신안대로.

## 불가피한 Halt (위임 불가)

- commit/push, PR, 태그·릴리즈(항상 별도 승인).
- 파괴적 작업 없음.

## Deferred / Follow-up

- 라이트/다크 테마 **전환 UI**(토글·설정) — 이번은 고정 다크만. 필요 시 후속.
- 사이드바 사설 색상 상수를 `theme.rs` 공용 팔레트로 이전(리팩토링) — 이번 범위 외.
- [T1 quality MINOR] `relayout()`에서 `BeginDeferWindowPos` 실패 시 `layout_cache` 미갱신(리팩터 전엔 실패해도 히트테스트 캐시 갱신됨). 실전 발생 거의 없고 다음 relayout이 복구 — 저위험 follow-up.

## Out of Scope

- 앱 전체 테마 전환 기능(라이트↔다크 토글 UI·설정 저장).
- 커스텀 다크 스크롤바(시스템 제약 — 목록/트리 스크롤바 완전 다크는 안 함).

## Open Questions

(없음 — 다크 범위·방식·잔상 개선 방식 모두 사용자 확정.)

## 통과 체크리스트

- [x] 요구 이해 작성
- [x] Impact Analysis 4-A~4-D
- [x] 각 task Type/Acceptance/Edge/Halt Forecast
- [x] Design 필드(Type D 전부 + 신규 심볼 Type C)
- [x] Decisions(위치·방식·의존성)
- [x] PRD Coverage + 갱신안
- [x] 사전 승인 항목 명시
- [x] Open Questions 0
