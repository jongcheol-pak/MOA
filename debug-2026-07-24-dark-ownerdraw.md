# Debug: 탐색기 다크 오너드로우 3종 미작동 (메뉴바·탭·컬럼 헤더)

## Symptom
탐색기 다크 테마 적용 후 실행하니 다음이 **밝은 회색 그대로**(다크 미적용):
- 메뉴바 (보기/이동/탭/워크스페이스)
- 탭 스트립 (각 패널 상단 탭)
- 파일 목록 컬럼 헤더 (이름/크기/종류/수정한 날짜)

정상 적용됨: 타이틀바, 사이드바, 주소창, 파일 목록 배경·텍스트(=SetWindowTheme+색상 메시지 계열).
안 됨 3종의 공통점: 전부 **오너드로우/커스텀드로우** 방식.

## Reproduction
`cargo run` → 4분할 워크스페이스 화면. 항상 재현(스크린샷 확인).

## Phase 1 — Evidence
- 실패 레이어: ① 메뉴바 WM_DRAWITEM(ODT_MENU, window.rs) ② 탭 WM_DRAWITEM(ODT_TAB, panel.rs) ③ 컬럼 헤더 NM_CUSTOMDRAW(ListView 서브클래스 list_dark_proc, file_list.rs)
- 최근 변경: 이번 세션 T3(헤더)·T6(탭)·T7(메뉴)에서 도입.
- panel.rs WM_NOTIFY(596~)는 `hwndFrom == file_list.hwnd()`만 분기 — **헤더(SysHeader32)로부터의 통지는 처리 지점 없음**.

## Phase 2 — Hypotheses
### 컬럼 헤더
- H-H1: ListView 헤더의 NM_CUSTOMDRAW는 ListView가 **부모(panel)로 forward**한다(MSDN). 그래서 ListView 서브클래스(list_dark_proc)는 이를 못 받고, panel WM_NOTIFY는 헤더 hwndFrom을 처리 안 해 다크 실패 — 예측: 헤더 CD 프로브 색이 안 나타나면 확정(통지가 list_dark_proc에 안 옴)

### 탭
- H-T1: WM_DRAWITEM(ODT_TAB)이 panel에 오지만 draw_tab 미실행 — 예측: 탭 프로브 색 나타나면 호출됨(색 로직 문제), 안 나타나면 통지 미도달
- H-T2: TCS_OWNERDRAWFIXED 런타임 미적용 → WM_DRAWITEM 자체 미발생

### 메뉴바
- H-M1: top-level 메뉴바 항목 오너드로우가 Windows에서 미작동(WM_DRAWITEM 미발생)
- H-M2: MFT_OWNERDRAW + MIIM_STRING 병행(F-7)이 오너드로우를 무효화 — 예측: 메뉴 프로브 색 안 나타나면 오너드로우 미작동

## Phase 3 — 진단 프로브 (시각) → 결과
각 오너드로우 draw 배경을 임시 색으로: 메뉴=빨강, 탭=초록, 헤더=파랑.
사용자 스크린샷(2번 이미지) 판정:
- ✅ 메뉴바 = **빨강** → draw_menu_item 호출됨 (색 로직만 다크로 복원하면 됨)
- ✅ 탭 = **초록** → draw_tab 호출됨 (색 로직만 복원)
- ❌ 컬럼 헤더 = **밝은 회색 유지** → list_dark_proc(ListView 서브클래스) 미호출

## Phase 3 — Root Cause
- **메뉴·탭**: 오너드로우는 정상 호출. 첫 스크린샷이 밝았던 건 최신 재빌드 전 exe였던 것. → 프로브만 제거(다크색 복원).
- **컬럼 헤더 (H-H1 확정)**: ListView 헤더(SysHeader32)의 NM_CUSTOMDRAW를 ListView가 자기 부모(패널)로 **forward**한다(MSDN 표준). 그래서 ListView를 서브클래싱한 `list_dark_proc`은 통지를 받지 못한다(프로브 파랑 미출현이 이를 확정). 헤더 다크는 **패널의 WM_NOTIFY**에서 처리해야 한다.

## Phase 4 — Fix
- window.rs `draw_menu_item`, tabs.rs `draw_tab`: 진단 프로브 제거 → 다크색 복원.
- file_list.rs: `list_dark_proc`(ListView 서브클래스) 제거 → `draw_header_dark(cd: &NMCUSTOMDRAW) -> LRESULT` pub 함수로 전환 + `header_hwnd()` 게터(LVM_GETHEADER) 추가. SetWindowSubclass·관련 import 제거.
- panel.rs: WM_NOTIFY에 `hwndFrom == file_list.header_hwnd() && code == NM_CUSTOMDRAW` 분기 추가 → `draw_header_dark` 호출.

## Verification
- Build/clippy 통과. 헤더 다크 실제 적용은 HUMAN-VERIFY(재실행 필요 — 헤더 통지가 panel로 온다는 MSDN 근거를 실측으로 최종 확인).

## Phase 5 — 재조사 & 정정 (2026-07-24, 사용자 재보고: 헤더·탭 여전히 밝음)

### 정정된 Root Cause
- **컬럼 헤더 (H-H1은 절반만 맞음)**: 헤더(SysHeader32)의 NM_CUSTOMDRAW는 자기 부모인 **ListView로만** 간다. ListView는 이를 **자기 부모(패널)로 다시 forward하지 않는다** — Phase 4에서 "패널 WM_NOTIFY로 처리"한 것이 오판이었다(그래서 계속 밝음). 검증된 방식은 ysc3839/win32-darkmode: **ListView 자체를 서브클래싱**해 헤더의 통지를 가로챈다.
- **탭 스트립**: 오너드로우(TCS_OWNERDRAWFIXED)는 **탭 항목만** 그리고, 항목 밖 스트립 여백·배경은 탭 컨트롤이 자체 도색 → 밝게 잔존. WM_DRAWITEM(부모)로는 배경을 못 덮는다.

### 정정 Fix (Phase 5)
- **헤더**: `SetWindowTheme(header, "ItemsView")`(앱 전역 `enable_dark_mode`와 함께 헤더 배경을 시스템이 다크로 도색) + `SetWindowSubclass(listview, list_dark_proc)`로 NM_CUSTOMDRAW 가로채 CDDS_ITEMPREPAINT에서 글자색만 HEADER_TEXT 지정, CDRF_DODEFAULT 반환. `draw_header_dark`(패널 호출용 pub fn)·패널 WM_NOTIFY 헤더 분기 제거.
- **탭**: `SetWindowSubclass(tab, tab_dark_proc)`로 WM_ERASEBKGND에서 클라이언트 전체를 WINDOW_BG로 채우고 1 반환(항목은 draw_tab이 위에 그림).

### Verification (Phase 5)
- 참조: https://github.com/ysc3839/win32-darkmode (ListViewUtil.h) — 헤더는 ListView 서브클래스에서 NM_CUSTOMDRAW 처리 + SetWindowTheme("ItemsView").
- build/clippy/fmt/test(75+2) 통과. 실제 헤더·탭 다크 렌더링은 `cargo run` 시각 확인 필요(HUMAN-VERIFY).

## Phase 6 — 컬럼 헤더 배경 잔존 (2026-07-25, 사용자 재보고: 헤더만 여전히 흰 배경)

### Evidence
- 사용자 스크린샷: 헤더 **글자는 밝은 회색(HEADER_TEXT)**, **배경만 흰색**. 탭·메뉴는 다크 정상.
- 판정: NM_CUSTOMDRAW는 `list_dark_proc`에 정상 도달(글자색이 적용된 것이 증거) → Phase 5의 서브클래스 위치는 옳았다.

### Root Cause
- `SetWindowTheme(header, "ItemsView")` + `AllowDarkModeForWindow`로 **시스템이 헤더 배경을 다크로 그리게 하는 방식이 이 환경(Windows 11 26300)에서 동작하지 않는다**. CDDS_ITEMPREPAINT에서 `CDRF_DODEFAULT`를 반환하면 기본 도색이 밝은 배경을 그대로 그린다.

### Fix
- `file_list.rs`: 헤더를 **완전 커스텀드로우**로 전환.
  - `CDDS_PREPAINT`: `CDRF_NOTIFYITEMDRAW | CDRF_NOTIFYPOSTPAINT`.
  - `CDDS_ITEMPREPAINT`: `draw_header_item`이 항목 배경(hot/pressed 포함)·오른쪽 구분선·제목(`HDM_GETITEMW`로 텍스트·정렬 형식 조회, 헤더 폰트 선택)을 직접 그리고 **`CDRF_SKIPDEFAULT`**로 기본 도색 차단.
  - 무효인 `SetWindowTheme(header,"ItemsView")` 호출 제거(`allow_dark_for_window`는 유지).

### 잔여 증상 → 추가 수정 (같은 Phase, 사용자 재보고: 마지막 열 오른쪽 여백만 흰색)
- **원인**: 열이 없는 오른쪽 여백은 항목이 아니라 **헤더 기본 도색**이 그린다. `CDDS_PREPAINT`에서 클라이언트 전체를 채워도 그 뒤 기본 도색이 다시 덮어 흰색으로 남는다.
- **수정**: `CDRF_NOTIFYPOSTPAINT`를 함께 요청하고, 항목을 모두 그린 뒤 `CDDS_POSTPAINT`에서 `fill_header_gap`이 여백을 `HEADER_BG`로 덮는다. 여백 시작점은 `HDM_GETITEMCOUNT`+`HDM_GETITEMRECT`로 구한 **모든 항목 rect의 오른쪽 끝 최댓값**(열 순서가 바뀌어도 안전).

### Verification (Phase 6)
- build/clippy(-D warnings)/fmt/test(75+2) 통과.
- **시각 확인 완료**: 앱 실행 후 `PrintWindow`로 창 캡처 — 헤더 배경·마지막 열 오른쪽 여백이 모두 다크(`HEADER_BG`), 제목은 밝은 회색, 열 구분선 표시 확인.
- 참고: 겹친 창 때문에 화면 캡처(`CopyFromScreen`)는 앱이 아닌 창을 찍을 수 있다 — 창 내용 확인은 `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT)`가 확실하다.
