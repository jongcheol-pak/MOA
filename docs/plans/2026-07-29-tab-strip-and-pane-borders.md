# 탭 스트립 통합 · 아이콘 버튼 · 분할 패널 테두리

**PRD**: docs/prd.md

## 요구 이해

원문 요청:
> - 2번 이미지처럼 탭이 닫기 버튼과 분리되어 있는데 합처서 표시, 버튼의 아이콘은 이미지만 표시 1번이미지 참고해서 동일하게 ui 수정.
> - 4번 이미지처럼 분할 된 화면이 구분되어 있지 않아서 보기가 불편함. 3번 이미지처럼 화면을 분할하면 테두리를 표시해줘

이해한 요구:

1. 현재 탭은 `[제목]` 버튼과 `[×]` 버튼이 **서로 떨어진 두 개의 사각 버튼**으로 보인다(2번 이미지). 이것을 Windows 11 탐색기(1번 이미지)처럼 **하나의 탭 영역 안에 폴더 아이콘·제목·닫기 ×가 들어간 형태**로 합친다.
2. `×`·`+`·분할 버튼처럼 아이콘만 있는 버튼에 붙은 **회색 사각 배경(프레임)을 없애고 아이콘만** 보이게 한다. 마우스를 올렸을 때만 배경이 칠해진다. 적용 범위는 탭 스트립 + 주소창 탐색 버튼(사용자 확정).
3. 화면을 분할하면 패널끼리 경계가 없어 어디까지가 한 패널인지 알 수 없다(4번 이미지). VS Code(3번 이미지)처럼 **분할된 모든 패널에 테두리**를 그리고, 활성 패널은 더 밝은 테두리로 구분한다(사용자 확정).
4. 탭 스트립 높이는 22px → 28px로 넓힌다(사용자 확정). 기능(탭 추가·닫기·전환·가운데 클릭 닫기·경로 툴팁·분할·탐색) 자체는 바꾸지 않는다 — **표시 방식만** 바꾸는 작업이다.

## Goal

탭 스트립을 Windows 11 탐색기와 같은 통합 탭 형태로 바꾸고, 아이콘 버튼에서 프레임을 걷어내며, 분할된 패널마다 경계 테두리를 그려 화면 구분을 명확히 한다.

## Scope

- `src/ui/widgets.rs` (신규) — 프레임 없는 아이콘 버튼 공통 위젯
- `src/ui/tabs.rs` — 통합 탭 렌더링, 프레임 없는 `+`·분할 버튼, 높이 28px
- `src/ui/address_bar.rs` — `←`·`→`·`↑` 버튼 프레임 제거, phosphor 아이콘 전환
- `src/ui/titlebar.rs` — 자체 `icon_button`을 공통 위젯으로 교체 (중복 제거)
- `src/ui/splitter.rs` — 분할 패널 테두리
- `src/ui/theme.rs` — 색 상수 추가 (폴더 아이콘 노랑, 패널 경계 2종)

## Out of Scope

- 탭 드래그 순서 변경·탭 컨텍스트 메뉴 (요청에 없음)
- 탭을 타이틀바로 올리는 배치 변경 — 1번 이미지는 탭이 창 제목 줄에 있지만, 이 앱은 **패널마다 독립 탭**(FR-3)이라 분할 시 타이틀바 한 줄에 담을 수 없다. 탭의 **모양**만 맞춘다
- 활성/비활성 탭의 제목 글자색 차등 (1번 이미지도 동일 색)
- 파일 목록·폴더 트리·사이드바의 시각 변경

## Deferred / Follow-up

- 이전 plan(`2026-07-29-custom-titlebar`)의 Deferred 2건(설정 팝업 5개 항목 기능, 창 그림자·둥근 모서리)은 이미 `docs/plans/deferred.md` 대장에 등재돼 있어 그대로 둔다 — 이번 작업과 무관
- 탭 폭 고정(Windows 11은 탭마다 같은 폭) — 이번에는 제목 길이에 맞추는 현행 방식을 유지한다. 분할 패널이 좁을 때 고정폭이 더 나은지는 화면을 보고 판단

## 시각 요소 분해

> 기준: 사용자 첨부 1번 이미지(Windows 11 탐색기 탭 줄), 3번 이미지(VS Code 분할 경계).
> **색 규칙**: 무채색은 기존 팔레트(`ui/theme.rs`)의 **명도 대역 안에서** 고른다(대역 안의 신규 중간 단계는 허용 — 경계선용 `#333333`·`#5A5A5A`가 그 경우다). 이 앱은 고정 다크 팔레트(FR-21)를 쓰므로 대역 밖의 색을 새로 들이면 화면 전체의 색 단계가 어긋난다. **예외는 폴더 아이콘 노랑 한 색**으로, 팔레트에 유채색이 `CLOSE_HOT`(빨강)뿐이라 도출할 값이 없다 — 아래 표에 RGB를 확정 기재한다.
> **치수 규칙**: 아래 표의 px 값이 곧 구현 상수다(96DPI 기준 고정 px — 타이틀바가 쓰는 방식과 같다). 표에 없는 값을 구현자가 새로 정하지 않는다.

### 시각 속성

| 요소 | 속성 | 디자인 값 | 확인 방법 |
|------|------|----------|-----------|
| 탭 스트립 | 높이 | 28px | 사용자 확정 (Q4 — 22px→28px) |
| 탭 | 구성 | 한 영역 안에 `폴더 아이콘 · 제목 · 닫기 ×` 가로 배치 | 1번 이미지 (탭 4개 모두 동일 구성) |
| 탭 | 좌측 여백 | 6px | 1번 이미지 비율에서 산출, 스트립 높이 28px 기준 |
| 탭 | 폴더 아이콘 구역 폭 | 16px | 아이콘 글꼴 14px + 좌우 여유 |
| 탭 | 폴더 아이콘 글꼴 | 14px | 스트립 높이 28px에 맞춘 값 — 타이틀바(16px)보다 한 단계 작다 |
| 탭 | 폴더 아이콘 색 | `#E8B34D` (팔레트 예외 — 신규 상수 `FOLDER_ICON`) | 1번 이미지의 노란 폴더를 다크 배경 명도에 맞춰 확정 |
| 탭 | 아이콘–제목 간격 | 4px | 1번 이미지 |
| 탭 | 제목 정렬·글꼴 | 세로 중앙, 좌측 정렬, 본문 글꼴 기본 크기, 넘치면 말줄임 | 1번 이미지 · 현행 `elide`(16자) 유지 |
| 탭 | 닫기 구역 폭 | 20px | 1번 이미지 (제목 오른쪽의 정사각 클릭 영역) |
| 탭 | 닫기 아이콘 글꼴 | 12px | 제목보다 작게 — 1번 이미지 |
| 탭 | 우측 여백 | 4px | 1번 이미지 |
| 탭 | 최소 폭 | 50px (= 6+16+4+0+20+4) — 제목이 0폭이 되어도 아이콘·닫기는 남는다 | 위 구역 값의 합 |
| 탭 | 활성 배경 | `CONTROL_BG`로 채움 | 1번 이미지 (좌측 `debug` 탭이 스트립보다 한 단계 밝음) |
| 탭 | 비활성 배경 | 없음 — 스트립 배경 그대로 | 1번 이미지 (`Dokkaebi`·`debug`·`바탕 화면` 탭) |
| 탭 | 탭 사이 구분선 | 인접 탭이 **둘 다 비활성**일 때만 세로 1px 선, 색 `TREE_LINE`, 높이 = 스트립의 60% 중앙 정렬 | 1번 이미지 (`Dokkaebi`↔`debug` 사이에 있고, 활성 탭 양옆에는 없음) |
| 탭 | 닫기 × | 프레임 없음, hover 시에만 `CONTROL_HOT` 배경 | 1번 이미지 |
| 탭 | 툴팁 | 탭 hover 시 전체 경로, 닫기 hover 시 "탭 닫기" | 현행 동작 보존 (`tabs.rs:67·75`) |
| `+`·분할 버튼 | 툴팁 | 각각 "새 탭"·"분할" | 현행 동작 보존 (`tabs.rs:80·97`) |
| 새 탭 `+` | 크기·프레임 | 24×28px, 프레임 없음 — hover 시에만 배경 | 1번 이미지 (마지막 탭 오른쪽 `+`) |
| 분할 버튼 | 크기·프레임 | 28×28px, 프레임 없음 — hover 시에만 배경. 아이콘은 현행 직접 그리기 유지 | 사용자 지시("버튼의 아이콘은 이미지만 표시") · 2번 이미지의 현행 사각 배경 대비 |
| 주소창 `←`·`→`·`↑` | 크기·프레임 | 24×24px, 프레임 없음 — hover 시에만 배경 | 1번 이미지 (주소 줄 왼쪽 탐색 버튼) |
| 주소창 `←`·`→`·`↑` | 아이콘 글꼴 | 16px — **활성·비활성 모두 같은 값**(상태에 따라 크기가 변하면 안 된다) | `titlebar.rs:27` `ICON_FONT_PX = 16.0`(타이틀바 버튼과 동일) |
| 주소창 `←`·`→`·`↑` | 비활성 표현 | 글자색 `TEXT_DIM`, hover 배경 없음, 클릭 무반응, **툴팁도 뜨지 않음** | 현행 `add_enabled`(`address_bar.rs:57~77`)의 동작 보존 |
| 분할 패널 | 경계 | 패널마다 1px 테두리, 색 `PANE_BORDER`(`#333333`) | 3번 이미지 (에디터 그룹마다 경계선) |
| 분할 패널 | 활성 강조 | 활성 패널만 1px 테두리, 색 `PANE_BORDER_ACTIVE`(`#5A5A5A`) | 사용자 확정 (Q3) · 현행 활성 테두리(`CONTROL_ACTIVE` 0x45)보다 한 단계 밝게 |
| 분할 패널 | 단일 패널 | 분할이 없으면 테두리 없음 | 현행 규칙 유지 (`splitter.rs:107` `panes.len() > 1`) |

### V-9 대조 결과 (구현 후 산출 — task별 누적)

> 판정 축 구분: **소스 대조 가능**(상수 값·문구·구성 요소 존재·조건 분기)은 ✅/❌로 확정하고,
> **최종 렌더 결과**(실제 픽셀 배치·정렬·색의 눈에 보이는 결과)는 데스크톱 GUI라 자율 루프에서
> 신뢰성 있게 캡처할 수단이 없어 `⏳ 미확인`으로 F-8에 인계한다.

| 요소·속성 | 디자인 값 | 구현 근거 | 판정 |
|---|---|---|---|
| 탭 스트립 높이 | 28px | `ui/tabs.rs` `STRIP_HEIGHT = 28.0` | ✅ 소스 |
| 탭 구성(아이콘·제목·닫기 한 영역) | — | `ui/tabs.rs` `show_tab` — `allocate_exact_size` 1회 + `tab_parts` 3구역 painter 그리기 | ✅ 소스 |
| 탭 좌측 여백 | 6px | `TAB_PAD_LEFT = 6.0` | ✅ 소스 |
| 폴더 아이콘 구역 폭 | 16px | `TAB_ICON_WIDTH = 16.0` | ✅ 소스 |
| 폴더 아이콘 글꼴 | 14px | `TAB_ICON_PX = 14.0` | ✅ 소스 |
| 폴더 아이콘 색 | `#E8B34D` | `ui/theme.rs` `FOLDER_ICON = from_rgb(0xE8, 0xB3, 0x4D)` | ✅ 소스 |
| 아이콘–제목 간격 | 4px | `TAB_ICON_GAP = 4.0` | ✅ 소스 |
| 제목 정렬 | 세로 중앙·좌측·말줄임 | `Align2::LEFT_CENTER` + `parts.label.left_center()` + `elide()` | ✅ 소스 |
| 닫기 구역 폭 | 20px | `TAB_CLOSE_WIDTH = 20.0` | ✅ 소스 |
| 닫기 아이콘 글꼴 | 12px | `TAB_CLOSE_PX = 12.0` | ✅ 소스 |
| 탭 우측 여백 | 4px | `TAB_PAD_RIGHT = 4.0` | ✅ 소스 |
| 탭 최소 폭 | 50px | `TAB_MIN_WIDTH` = 6+16+4+20+4, 테스트 `최소_폭에서도_아이콘과_닫기_크기가_유지된다` | ✅ 소스 |
| 활성 탭 배경 | `CONTROL_BG` 채움 | `if active { rect_filled(rect, 0.0, theme::CONTROL_BG) }` | ✅ 소스 |
| 비활성 탭 배경 | 없음 | 위 `if active` 밖에 배경 그리기 없음 | ✅ 소스 |
| 탭 사이 구분선 | 둘 다 비활성일 때만·`TREE_LINE`·60% | `draw_separator` + `!active && next != active_index` 조건 + `SEPARATOR_RATIO = 0.6` | ✅ 소스 |
| 탭·닫기 툴팁 | 전체 경로 / "탭 닫기" | `response.on_hover_text(path…)`, `close.on_hover_text("탭 닫기")` | ✅ 소스 |
| `+`·분할 툴팁 | "새 탭" / "분할" | `.on_hover_text("새 탭")`, `.on_hover_text("분할")` | ✅ 소스 |
| 새 탭 `+` | 24×28px·프레임 없음 | `widgets::icon_button(…, vec2(NEW_TAB_WIDTH, STRIP_HEIGHT), …)` | ✅ 소스 |
| 분할 버튼 | 28×28px·프레임 없음·아이콘 직접 그리기 | `widgets::icon_button(ui, "", vec2(STRIP_HEIGHT, STRIP_HEIGHT), …)` + `draw_split_icon` | ✅ 소스 |
| **탭 줄 전체의 실제 화면 결과** | 1번 이미지와 동일한 인상 | — | ⏳ 미확인 (F-8 인계) |
| **닫기 우선 규칙의 실제 조작 결과** | 비활성 탭 × 클릭 시 닫힘·전환 없음 | 배선은 `show_tab`의 `!clicked_on_close`로 확인 | ⏳ 미확인 (F-8 인계) |

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-3 (패널별 독립 탭) | Must | T2 | ✅ 커버 — 탭 표시 방식만 변경, 추가/닫기/전환 동작은 불변 |
| FR-21 (탐색기 영역 고정 다크) | Should | T1~T4 | ✅ 커버 — 새 색도 고정 다크 팔레트 안에서 추가 |
| FR-1·FR-2 (자유 분할·스플리터) | Must | T4 | ✅ 커버 — 레이아웃 계산은 불변, 경계 표시만 추가 |
| FR-6 (주소창 탐색 버튼) | Must | T3 | ✅ 커버 — 버튼 모양만 변경, 탐색 동작 불변 |
| FR-4·5·7~20·22 | Must/Should | — | 이번 범위 외 (기구현) |

## 사전 승인 항목 (일괄 승인 대상)

- `src/ui/widgets.rs` 신규 모듈 추가와 `src/ui/mod.rs` 등록 (비파괴 구조 추가)
- `titlebar.rs`의 private `icon_button`을 공통 위젯으로 교체 — 호출부 5곳 모두 이 plan 안에서 갱신 (외부 노출 없음)
- `theme.rs`에 색 상수 3개 추가 (기존 상수 값 변경 없음)
- 로컬 작업 브랜치(`task/custom-titlebar`)에 대한 task 완료 commit

## 불가피한 Halt (위임 불가)

- push · master 병합 · 태그 · PR
- 기존 팔레트 색 **값 자체**를 바꿔야 한다고 판단되는 경우 (화면 전체에 영향)
- 탭 통합 결과 기존 탭 조작(전환·닫기·가운데 클릭)이 회귀해 구조 변경이 필요한 경우

## Tasks

### [x] T1. 프레임 없는 아이콘 버튼 공통 위젯 (Type C)

**Design**:
- 배치: `src/ui/widgets.rs` 신규 — `ui` 계층 안의 공용 그리기 헬퍼 자리
- 신규 심볼:
  - `icon_button(ui, icon: &str, size: egui::Vec2, hover_fill: Color32) -> egui::Response` — 지정 크기 영역을 잡고, hover일 때만 배경을 칠한 뒤 아이콘 글리프를 중앙에 그린다
  - `icon_button_styled(ui, icon: &str, size: egui::Vec2, hover_fill: Color32, tint: Color32, font_px: f32) -> egui::Response` — 글자색·글꼴 크기를 지정하는 변형. 주소창의 비활성 버튼(`TEXT_DIM` + `hover_fill = TRANSPARENT`)이 쓴다. `icon_button`은 이 함수에 기본값(`theme::TEXT`, 16px)을 넘기는 얇은 래퍼다
  - **두 함수 모두 "스스로 rect를 할당하고 클릭을 감지하는 독립 버튼"용이다.** 다른 위젯 **안쪽**에 그리는 요소(탭 안의 폴더 아이콘·닫기 ×)에는 쓰지 않는다 — 쓰면 아이콘이 탭 밖에 배치되거나 그 구역에서 탭 본체 클릭이 삼켜진다
- 의존 방향: `theme`만 참조. `titlebar`·`tabs`·`address_bar`가 이 모듈을 참조한다 (역방향 없음)
- 비추상화 선언: 버튼 종류별 타입·빌더 패턴·트레이트를 만들지 않는다. 호출부가 3개 모듈이고 인자가 6개 이하라 함수 2개로 충분하다

- `titlebar.rs`의 private `icon_button`(286~306행)을 삭제하고 공통 위젯 호출로 바꾼다. 크기를 **`Vec2`(폭·높이)로 받아** 캡션 버튼 `46×36`·토글/설정 버튼 `36×36`을 그대로 재현한다
- `hover_fill`을 인자로 유지해 닫기 버튼의 `CLOSE_HOT`(빨강)과 나머지의 `CONTROL_HOT` 구분을 보존한다
- Files: `src/ui/widgets.rs`(신규), `src/ui/mod.rs`, `src/ui/titlebar.rs`
- Edge cases: 크기가 0 이하 → `allocate_exact_size`가 빈 rect를 주므로 그리기가 무해하게 생략된다 / `hover_fill`이 `TRANSPARENT` → hover여도 배경이 보이지 않는다(주소창 비활성 버튼이 이 경로를 쓴다) / **아이콘 문자열이 빈 값** → 영역 확보와 hover 배경만 하고 글리프는 그리지 않는다(분할 버튼이 이 경로로 배경만 받고 아이콘은 `draw_split_icon`으로 직접 그린다)
- Halt Forecast: 없음 (내부 모듈 추가, 외부 의존 없음)
- Acceptance:
  - `cargo build` 경고 0, `cargo clippy --all-targets -- -D warnings` 통과
  - `titlebar.rs`에 `icon_button` 정의가 남아 있지 않고, 타이틀바 버튼 **5개**(사이드바 토글·설정·최소화·최대화·닫기)가 모두 공통 위젯을 호출한다
  - 캡션 버튼 폭 46px·높이 36px, 토글/설정 36×36, 닫기 hover 색 `CLOSE_HOT`이 코드상 유지된다
  - 기존 타이틀바 테스트 7개 전부 통과 (`cargo test`)

### [x] T2. 탭 스트립 — 통합 탭 · 프레임 없는 버튼 · 높이 28px (Type C, T1 의존)

**Design**:
- 배치: `src/ui/tabs.rs` 수정 (그리기·입력만 담당하는 현행 역할 유지 — 상태는 `panel::tabs::TabsModel`이 계속 소유)
- 신규 심볼:
  - `TabParts { icon: Rect, label: Rect, close: Rect }` — 탭 rect를 세 구역으로 나눈 결과
  - `tab_parts(rect: Rect) -> TabParts` — 순수 계산(시각 요소 분해 표의 px 상수만 사용). 단위 테스트 대상
  - `is_close_hit(parts: &TabParts, pos: Pos2) -> bool` — 포인터가 닫기 구역 안인가. 순수 판정이라 단위 테스트 대상이며, **탭 본체 클릭이 이 판정을 통과하면 전환을 내지 않는다**(닫기 우선 규칙의 정본)
  - `show_tab(ui, title, path, active) -> TabHit { switch: bool, close: bool }` — 탭 하나 그리기
- 의존 방향: `widgets`·`theme`·`panel::tabs`를 참조. 이 모듈을 참조하는 곳은 `ui::panel` 한 곳(`panel.rs:436`)이며 `show_tab_strip`의 시그니처·반환 타입(`TabStripOutcome`·`TabAction`)은 바꾸지 않는다
- 비추상화 선언: 탭 렌더러 트레이트·제네릭 아이템을 만들지 않는다. 탭은 이 한 곳에서만 그린다
- 구역 상수: 좌여백 6 · 아이콘 16 · 간격 4 · 닫기 20 · 우여백 4 · 최소 폭 50 (전부 시각 요소 분해 표에서 옴)
- **탭 폭 계약**: 탭 폭 = `6 + 16 + 4 + 제목 galley 폭 + 20 + 4`이며, 이 값이 50px 미만이면 **호출부(`show_tabs`)가 50px로 올려 할당**한다. `tab_parts`는 **폭 ≥ 50을 전제**하므로 내부 클램프를 하지 않는다 — 클램프를 넣으면 반환 rect가 입력 rect를 벗어나 "닫기 구역이 항상 탭 rect 안"이라는 계약이 깨진다

- 탭 하나를 `allocate_exact_size`로 한 영역에 잡고 그 안에 폴더 아이콘·제목·닫기 ×를 배치한다. 닫기 구역만 별도로 `interact`를 걸고, **탭 본체 클릭은 `is_close_hit`이 참이면 무시**해 닫기가 전환보다 우선하게 한다
- **탭 안의 폴더 아이콘·제목·닫기 ×는 위젯을 새로 할당하지 않고 `tab_parts` 구역에 painter로 직접 그린다**(색·글꼴은 시각 요소 분해 표의 값). 아이콘을 `icon_button` 계열로 그리면 자기 rect를 새로 할당해 탭 밖에 배치되거나, 그 구역에서 탭 본체 클릭이 삼켜져 아이콘 위에서는 전환이 되지 않는다
- **보존할 기존 동작**: ① 가운데 버튼 클릭으로 탭 닫기(`tabs.rs:71~74`) ② 탭 hover 시 전체 경로 툴팁(`tabs.rs:67`) ③ 닫기 hover 시 "탭 닫기" 툴팁(`tabs.rs:75`) ④ 긴 제목 말줄임(`elide`) ⑤ `+` 버튼 "새 탭" 툴팁(`tabs.rs:80`) ⑥ 분할 버튼 "분할" 툴팁(`tabs.rs:97`) — 설정 버튼 선례처럼 `icon_button(...).on_hover_text(...)`와 `Popup::menu`는 함께 쓸 수 있다
- 활성 탭만 배경을 채우고, 비활성 탭은 배경을 그리지 않는다. **인접한 두 탭이 모두 비활성일 때만** 그 사이에 세로 구분선을 그린다
- `+` 버튼은 T1의 공통 위젯(`icon_button`, 24×28)으로 바꿔 프레임을 없앤다
- **분할 버튼은 `MenuButton::from_button`을 걷어내고 `titlebar.rs:184~198`의 선례를 따른다** — `widgets::icon_button`으로 28×28 영역을 그린 뒤 그 `Response`에 `egui::Popup::menu(&response)`로 메뉴를 붙인다. 이유: `MenuButton`은 `Button` **위젯 값**을 요구해 `Response`를 돌려주는 공통 위젯을 끼울 수 없고, `Button`을 남기면 hover 배경을 egui의 `visuals.hovered`가 그려 "hover 시에만 배경" 규칙이 다른 버튼과 갈린다. 설정 버튼이 이미 같은 방식으로 팝업을 연다
- 분할 버튼의 아이콘은 현행 `draw_split_icon`(직접 그린 사각형+중앙선)을 그대로 쓴다 — `icon_button`이 hover 배경을 칠한 뒤 그 위에 그린다(아이콘 문자열은 빈 값을 넘긴다)
- `STRIP_HEIGHT`를 22.0 → 28.0으로 올린다
- Files: `src/ui/tabs.rs`, `src/ui/theme.rs`(`FOLDER_ICON` 상수)
- Edge cases:
  - 분할이 깊어 탭 폭이 좁을 때 → 탭 최소 폭 50px를 보장하고, 제목이 0폭이 되어도 아이콘·닫기 구역은 남긴다(닫기 불가 상태를 만들지 않는다)
  - 탭이 많아 가로 스크롤될 때 → 기존 `ScrollArea::horizontal` 유지, 분할 버튼은 계속 오른쪽 고정(현행 D6)
  - 마지막 탭 닫기 → `TabsModel::close`의 `CloseOutcome::LastTab` 처리 그대로 (패널의 마지막 탭은 남는다)
  - 닫기와 전환이 같은 프레임에 잡히는 경우 → `is_close_hit`으로 닫기를 먼저 판정하고 그 프레임의 전환은 내지 않는다
- Halt Forecast: 없음 — 조작 결과 타입(`TabAction`·`TabStripOutcome`)을 바꾸지 않으므로 `ui::panel`의 처리 로직은 그대로다
- Acceptance:
  - `tab_parts` 단위 테스트: 최소 폭·넓은 폭 모두에서 세 구역이 서로 겹치지 않고, 닫기 구역이 항상 탭 rect 안에 있으며, 최소 폭에서도 아이콘·닫기 구역 크기가 표의 값(16·20px)을 유지한다
  - `is_close_hit` 단위 테스트: 닫기 구역 안의 좌표는 참, 제목·아이콘 구역 좌표는 거짓 (**구역 판정만** 검증한다 — 이 술어를 `show_tab`이 실제로 호출하는지는 아래 코드 확인·화면 확인이 맡는다)
  - 코드 확인: `show_tab`의 전환 분기가 `is_close_hit`을 통과한 클릭을 걸러낸다(호출 지점이 실재하는지 diff에서 확인)
  - `cargo test` 전체 통과 (기존 `elide` 테스트 3개 + `ui::panel`의 위젯 ID 충돌 테스트 포함)
  - `cargo clippy --all-targets -- -D warnings` 통과
  - 화면 확인(HUMAN-VERIFY): ⓐ 탭이 하나의 영역으로 보이고 폴더 아이콘·제목·×가 그 안에 있다 ⓑ `+`·분할 버튼에 사각 배경이 없고 hover 시에만 배경이 뜬다 ⓒ **가운데 버튼 클릭으로 탭이 닫힌다** ⓓ **탭에 마우스를 올리면 전체 경로 툴팁이 뜬다** ⓔ 비활성 탭 사이에만 구분선이 보인다 ⓕ **비활성 탭의 ×를 누르면 그 탭이 닫히고, 그 탭으로 전환되지 않는다**(닫기 우선 규칙의 실제 배선 확인 — 활성 탭에서는 전환 여부가 드러나지 않으므로 비활성 탭으로 확인한다) ⓖ 분할 버튼을 누르면 네 방향 메뉴가 그대로 뜬다

### T3. 주소창 탐색 버튼 — 프레임 제거 · 아이콘 글꼴 전환 (Type C, T1 의존)

**Design**:
- 배치: `src/ui/address_bar.rs` 수정 — 신규 모듈 없음
- 신규 심볼: `nav_button(ui, icon: &str, enabled: bool, hint: &str) -> bool` (파일 내 private 헬퍼) — 세 버튼이 같은 비활성 처리를 반복하므로 한 곳에 모은다. 활성이면 `widgets::icon_button`(기본 `TEXT`·16px), 비활성이면 `icon_button_styled(tint = TEXT_DIM, hover_fill = TRANSPARENT, font_px = 16.0)`를 호출한다 — **두 경로의 글꼴 크기는 16px로 같아야 한다**(상태에 따라 아이콘 크기가 변하면 안 된다). **비활성일 때는 툴팁도 걸지 않는다**(현행 `add_enabled`와 같은 동작)
- 의존 방향: `widgets`·`theme`·`panel::address_bar`를 참조. 이 모듈을 참조하는 곳은 `ui::panel` 한 곳이며 `AddressBar::show`의 시그니처·반환 타입(`NavAction`)은 바꾸지 않는다
- 비추상화 선언: 버튼 정의를 배열+반복으로 묶지 않는다 — 세 버튼의 활성 조건·반환 액션이 각각 달라 묶으면 조건표를 따로 들고 다녀야 한다

- `←`·`→`·`↑` 세 버튼을 `nav_button`으로 바꾸고, 문자 화살표 대신 phosphor 아이콘(`ARROW_LEFT`·`ARROW_RIGHT`·`ARROW_UP`)을 쓴다 — 타이틀바가 이미 같은 글꼴을 쓰므로 획 굵기가 통일된다
- `add_enabled`가 사라지므로 **비활성 처리를 직접 한다**: 흐린 글자색(`TEXT_DIM`) + hover 배경 없음 + 클릭 무시 + 툴팁 억제
- Files: `src/ui/address_bar.rs`
- Edge cases:
  - 히스토리가 비어 뒤로·앞으로가 모두 불가 → 두 버튼이 흐리게 표시되고 클릭·hover 모두 반응하지 않는다
  - 드라이브 루트(`C:\`)에서 상위 버튼 → `parent()`가 `None`이라 흐리게 표시
  - 비활성 버튼 위에서 마우스를 눌렀다 떼는 경우 → `nav_button`이 `false`를 반환해 `NavAction`이 만들어지지 않는다
- Halt Forecast: 없음
- Acceptance:
  - `cargo build` 경고 0, `cargo clippy --all-targets -- -D warnings` 통과
  - `nav_button`이 `enabled == false`이면 클릭 여부와 무관하게 `false`를 반환한다(코드상 조기 반환 확인 — `Ui`가 필요해 단위 테스트 비대상)
  - 화면 확인(HUMAN-VERIFY): ⓐ 세 버튼에 사각 배경이 없고 hover 시에만 배경이 뜬다 ⓑ 갈 수 없는 방향이 흐리게 보이며 hover해도 배경·툴팁이 뜨지 않는다 ⓒ 탐색 동작(뒤로·앞으로·상위)이 그대로다

### T4. 분할 패널 경계 테두리 (Type C)

**Design**:
- 배치: `src/ui/splitter.rs`의 `show_layout` 안, 현행 활성 테두리 코드(`splitter.rs:106~116`) 자리를 확장. `src/ui/theme.rs`에 색 상수 2개 추가
- 신규 심볼: `theme::PANE_BORDER`(`#333333`) · `theme::PANE_BORDER_ACTIVE`(`#5A5A5A`) — 상수 2개뿐이며 함수·타입은 추가하지 않는다. 현행 활성 테두리가 `CONTROL_ACTIVE`(버튼 눌림색)를 빌려 쓰는데, 경계선과 버튼 상태색은 쓰임이 달라 각각의 이름을 준다
- 의존 방향: `splitter` → `theme` (기존과 동일, 새 방향 없음)
- 비추상화 선언: 테두리 그리기를 함수로 빼지 않는다 — 호출 지점이 이 한 곳이다
- 상수 `ACTIVE_BORDER: f32 = 1.0`(현행)을 그대로 재사용한다

- 패널을 모두 그린 **뒤**, 패널이 2개 이상일 때 각 패널 rect에 `PANE_BORDER` 1px 테두리를 그리고, 활성 패널만 `PANE_BORDER_ACTIVE` 1px 테두리로 덮는다. 순서를 지키는 이유: egui는 나중에 그린 도형이 위에 오므로 먼저 그리면 패널 내용에 가려진다
- Files: `src/ui/splitter.rs`, `src/ui/theme.rs`
- Edge cases:
  - 패널이 1개(분할 없음) → 테두리를 그리지 않는다(현행 규칙 유지 — 창 가장자리와 겹쳐 의미가 없다)
  - **테두리 패스에서 폭·높이 0 이하인 pane 재검사** → 패널 그리기 루프의 `continue`(`splitter.rs:73`)는 이 새 패스에 상속되지 않으므로 여기서 다시 검사해 1px 잔상을 막는다
  - `panels`에 아직 실체가 없는 `PanelId`(분할 직후 한 프레임) → 그 자리에도 테두리를 그린다. 빈 자리도 레이아웃상 한 칸이므로 경계가 보이는 편이 맞다
  - 활성 패널 id가 `computed.panes`에 없는 경우 → 일반 테두리만 그려지고 강조는 생략된다(현행 `find` 실패 경로와 동일)
- Halt Forecast: 없음 — 레이아웃 계산(`LayoutTree::compute_rects`)을 건드리지 않는다
- Acceptance:
  - `cargo test` 전체 통과 (레이아웃 트리 테스트 회귀 없음)
  - `cargo clippy --all-targets -- -D warnings` 통과
  - 화면 확인(HUMAN-VERIFY): ⓐ 2개 이상으로 분할하면 패널마다 테두리가 보인다 ⓑ 클릭한 패널의 테두리가 더 밝다 ⓒ 분할하지 않은 상태에서는 테두리가 없다

## Decisions

- **D1 (구조)**: 탭을 `Button` 위젯 2개(제목·닫기)로 두지 않고 **한 영역을 직접 그린다**. 이유: egui의 `Button`은 자기 프레임·여백을 스스로 그려 두 위젯을 붙여도 시각적으로 한 덩어리가 되지 않는다. `titlebar.rs:287~306`이 같은 이유로 이미 직접 그리기 방식을 쓴다. Source: `src/ui/titlebar.rs:286`
- **D2 (구조)**: 탭 rect 분할과 닫기 히트 판정을 순수 함수(`tab_parts`·`is_close_hit`)로 분리한다. 이유: AGENTS.md의 "UI(HWND 필요) 로직은 테스트 비대상 — 순수 로직을 UI에서 분리해 테스트" 규약. 좁은 폭에서의 구역 겹침과 "×를 눌렀는데 전환만 되는" 회귀가 이 두 계산에서 나오므로 테스트 가치가 높다. Source: `AGENTS.md` Conventions 테스트 항목
- **D3 (재사용)**: 아이콘 버튼 그리기를 `ui/widgets.rs`로 추출한다. 이유: 호출부가 타이틀바(5개)·탭 스트립(3종)·주소창(3개)으로 **3개 모듈**에 걸쳐 반복 사용이 확정된다(공통 규칙: 2회 이상 반복 시 공통화). `titlebar.rs`에 둔 채 다른 모듈이 가져다 쓰면 "타이틀바가 탭 스트립의 상위"라는 잘못된 의존이 생긴다
- **D4 (색)**: 신규 상수 3개(`FOLDER_ICON`·`PANE_BORDER`·`PANE_BORDER_ACTIVE`)를 `theme.rs`에 추가하고 **기존 상수 값은 바꾸지 않는다**. 폴더 노랑은 팔레트에 유채색 기준이 없어 값을 이 plan에서 확정했고(`#E8B34D` — 다크 배경에서 눈부시지 않은 명도), 경계 2색은 기존 무채색 단계 사이(0x2A ~ 0x45 ~ 0x6A)에 맞춰 골랐다. 이유: FR-21의 고정 다크 팔레트가 화면 전체의 기준이며, 기존 값 변경은 이번 요청 범위 밖이다
- **D5 (분할 버튼 아이콘)**: 분할 버튼 아이콘을 phosphor 글리프로 교체하지 않고 현행 `draw_split_icon`을 유지한다. 이유: 사용자 요청은 "프레임 제거"이지 아이콘 교체가 아니며, 현행 아이콘은 이미 모양이 명확하다. 최소 수정 원칙
- **D6 (주소창 아이콘)**: 반면 주소창은 문자 화살표(`←`)를 phosphor 아이콘으로 **바꾼다**. 이유: 문자 화살표는 본문 글꼴로 렌더돼 다른 아이콘 버튼과 획 굵기·크기가 눈에 띄게 다르다. 프레임을 없애면 그 차이가 더 드러난다
- **D7 (탭 배경)**: 활성 탭은 `CONTROL_BG` 단계로 채우고 비활성은 비운다(현행은 활성 `CONTROL_ACTIVE`·비활성 `CONTROL_BG`). 이유: 1번 이미지에서 활성 탭과 스트립 배경의 명도 차이가 작고, 비활성 탭에는 배경이 없다. Source: 시각 요소 분해 표
- **D8 (구분선 조건)**: 탭 사이 세로 구분선은 인접 두 탭이 **모두 비활성**일 때만 그린다. 이유: 1번 이미지에서 활성 탭 양옆에는 구분선이 없다(활성 탭의 배경 자체가 경계 역할을 한다)
- **D9 (테두리 시점)**: 패널 테두리는 패널 내용을 모두 그린 뒤 그린다. 이유: egui는 나중에 그린 도형이 위에 온다. Source: 현행 `splitter.rs:106`이 이미 같은 순서
- **D10 (비활성 버튼 툴팁)**: 주소창 비활성 버튼은 툴팁을 걸지 않는다. 이유: 현행 `add_enabled(false, …)`에 붙은 `on_hover_text`는 비활성 응답에 표시되지 않는다(`egui/src/response.rs:703·707` — `Tooltip::for_enabled`) — 직접 그리기로 바꾸면서 툴팁이 새로 뜨면 그것이 곧 동작 변경이다. "표시 방식만 바꾼다"는 요구 이해 4와 정합
- **D11 (분할 버튼 팝업 구조)**: 분할 버튼을 `MenuButton::from_button`에서 `icon_button` + `egui::Popup::menu(&response)`로 바꾼다. 이유: `MenuButton`은 `Button` 위젯 값을 요구해 `Response`를 돌려주는 공통 위젯과 맞지 않고, `Button`을 남기면 hover 배경을 egui의 `visuals.hovered`가 그려 다른 아이콘 버튼과 규칙이 갈린다. 설정 버튼(`titlebar.rs:184~198`)이 이미 이 구조로 팝업을 연다. Source: `src/ui/titlebar.rs:184`
- **D12 (탭 내부 그리기)**: 탭 안의 폴더 아이콘·제목·닫기 ×는 위젯이 아니라 painter로 그린다. 이유: `icon_button` 계열은 자기 rect를 새로 할당하고 클릭을 감지하므로 탭 **안쪽** 요소로 쓰면 배치가 어긋나거나 아이콘 구역에서 탭 전환 클릭이 죽는다. 탭은 한 덩어리 위젯이어야 한다(D1과 같은 이유)

## 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `widgets::icon_button` | `titlebar.rs:287`의 private `icon_button`이 **같은 규칙**(평소 배경 없음 + hover 시 배경 + 아이콘 중앙 그리기)을 이미 구현 | **재사용** — 신규 작성이 아니라 위치 이동. 원본은 삭제해 중복을 남기지 않는다 (T1) |
| `widgets::icon_button_styled` | 같은 파일에 글자색·글꼴 크기를 받는 변형 없음. `grep "FontId::proportional"` → `titlebar.rs:302` 1곳뿐 | **신규** — 주소창 비활성 표현(T3)에 필요한 인자(흐린 글자색·투명 hover)가 원본에 없다 |
| `tabs::tab_parts` · `is_close_hit` | `grep -rn "fn.*parts\|Rect::from_min_max" src/ui` → 구역 분할 헬퍼 없음. `panel/tabs.rs`는 순수 모델(경로·인덱스)만 다루고 좌표를 모른다 | **신규** — 탭 내부 좌표 계산은 이 앱에 처음 생긴다 |
| `tabs::show_tab` | 현행 `show_tabs`(`tabs.rs:50`) 안의 인라인 루프가 유일한 그리기 코드 | **신규(대체)** — 인라인 코드를 함수로 승격하는 것이며 별도 중복이 생기지 않는다 |
| `address_bar::nav_button` | `grep "add_enabled" src/ui` → `address_bar.rs:58·65·72`(대상 3곳) · `titlebar.rs:204`(메뉴 항목, 성격 다름) | **신규** — 세 버튼이 같은 비활성 처리를 반복하므로 파일 내 헬퍼로 묶는다 |
| `theme::PANE_BORDER` · `PANE_BORDER_ACTIVE` | `theme.rs:9~29` 확인 — 경계선 전용 색 없음(`TREE_LINE`은 트리 연결선, `CONTROL_ACTIVE`는 버튼 상태) | **신규** — 쓰임이 다른 색을 빌려 쓰던 것을 이름 있는 상수로 분리 |

## 자율 실행 준비도 자문

- 이 plan을 다른 사람에게 넘겨도 추가 질문 없이 끝낼 수 있는가? → 예 (색 RGB·치수 px·구성·적용 범위가 모두 표에 확정, 보존할 기존 동작 4가지도 명시)
- 구현 중 사용자에게 물어야 할 결정 분기가 있는가? → 없음 (Q1~Q4로 갈림길 4개 해소, 나머지는 D1~D12로 근거 확정)
- 검증 방법이 각 task에 명시되어 있는가? → 예 (빌드·clippy·단위 테스트 + 화면 확인 항목을 구분해 기재). 회귀 위험이 큰 닫기 우선 규칙은 **단위 테스트(구역 판정) → 코드 확인(호출 지점 실재) → 화면 확인 ⓕ(비활성 탭의 × 클릭)** 3단으로 잡는다 — 순수 함수 테스트만으로는 배선 누락을 잡지 못하기 때문이다

## Investigation Log

- `src/ui/tabs.rs:64~82` 확인: 탭이 `egui::Button`(제목) + `ui.small_button("×")` **두 위젯**으로 그려진다 — 사용자가 지적한 "분리되어 보임"의 직접 원인. `+`도 `small_button`이라 프레임이 붙는다
- `src/ui/tabs.rs:67·71~74·75` 확인: 탭에 **전체 경로 툴팁**, **가운데 클릭 닫기**, 닫기 버튼 "탭 닫기" 툴팁이 붙어 있다 → 재작성 시 보존 대상(T2에 명시)
- `src/ui/tabs.rs:88~101` 확인: 분할 버튼은 `Button::new("").fill(theme::CONTROL_BG)` — 배경을 명시적으로 칠하고 그 위에 `draw_split_icon`으로 아이콘을 그린다. 프레임 제거는 이 `fill`을 거두고 hover 시에만 칠하는 방식으로 바꾸면 된다
- `grep show_tab_strip` 전수(src·tests): 정의 1곳 + 호출 1곳(`src/ui/panel.rs:436`)뿐. `TabStripOutcome`·`TabAction`도 `ui/panel.rs`에서만 소비된다 → 반환 타입을 유지하면 영향은 `tabs.rs` 안에 갇힌다
- `src/ui/titlebar.rs:286~306` 확인: `icon_button`이 이미 "평소 배경 없음 + hover 시 배경 + 아이콘 중앙 그리기" 규칙을 구현하고 있다 — 이번 요구와 정확히 같은 규칙이라 재사용 대상(신규 작성 아님)
- `src/ui/titlebar.rs:134·184·283` 확인: 호출부는 토글 1 · 설정 1 · `caption_button`(닫기·최대화·최소화 3) = **5개**. 크기는 `BUTTON_SIZE 36`·`CAPTION_WIDTH 46`이며 높이는 모두 `TITLEBAR_HEIGHT 36`
- `src/ui/titlebar.rs:27~28` 확인: `ICON_FONT_PX = 16.0`, `TITLE_FONT_PX = 14.0` — **타이틀바 버튼의 아이콘 글꼴은 16px**이다(14px는 제목 글꼴). 주소창 버튼을 "타이틀바와 동일"하게 맞추면 16px가 된다
- `src/ui/titlebar.rs:184~198` 확인: 설정 버튼은 `icon_button`으로 그린 `Response`에 `egui::Popup::menu(&response)`를 붙여 팝업을 연다 — `MenuButton`을 쓰지 않고도 메뉴가 열리는 선례이며, 분할 버튼을 같은 구조로 바꿀 수 있다
- `egui-0.35.0/src/response.rs:703·707` 확인: `on_hover_text`는 `Tooltip::for_enabled`를 쓰며 문서주석이 비활성 위젯에는 `on_disabled_hover_text`를 쓰라고 안내한다 → **현행 비활성 탐색 버튼은 툴팁이 뜨지 않는다**(D10의 전제 확인)
- `src/ui/app.rs:127` 확인: `egui_phosphor::add_to_fonts(&mut fonts, Variant::Regular)`가 이미 호출된다. 아이콘 글꼴은 exe에 정적 포함이라 실패 경로가 없다(`app.rs:126` 주석) → 탭·주소창에서 아이콘 글리프를 바로 쓸 수 있다
- `egui-phosphor-0.13.0/src/variants/regular.rs` 확인: 필요한 상수 전부 존재 — `FOLDER`(E24A) · `PLUS`(E3D4) · `X`(E4F6) · `ARROW_LEFT`(E058) · `ARROW_RIGHT`(E06C) · `ARROW_UP`(E08E)
- `src/ui/address_bar.rs:55~77` 확인: 세 버튼이 `ui.add_enabled(조건, Button::new("←")).on_hover_text(…)` 형태 — 프레임 제거 시 `add_enabled`가 주던 흐린 표시·클릭 차단·툴팁 억제가 모두 사라지므로 직접 처리해야 한다(T3 Design·Edge case에 반영)
- `src/ui/theme.rs:9~29` 확인: 팔레트는 무채색 9종 + `CLOSE_HOT`(빨강)뿐 — **노랑 계열이 없다.** 폴더 아이콘 색은 팔레트에서 도출할 수 없어 이 plan에서 값을 확정했다(D4)
- `src/ui/splitter.rs:106~116` 확인: 활성 패널 테두리가 이미 `panes.len() > 1` 조건 아래 `CONTROL_ACTIVE` 색으로 그려진다. 4번 이미지에서 구분이 안 되는 이유는 **활성 1개에만 테두리가 있고 나머지는 없기 때문**이다(테두리 자체가 없는 것이 아니다)
- `src/ui/splitter.rs:71~104` 확인: 0크기 pane을 거르는 `continue`(73행)는 **패널 그리기 루프 안**의 가드다 → 테두리를 별도 패스로 그리면 상속되지 않으므로 재검사가 필요하다(T4 Edge case에 반영)
- `src/ui/splitter.rs:124` 확인: 스플리터 틈은 `WINDOW_BG`(0x1B1B1B)로 칠해지는데 패널 배경 `SURFACE_BG`(0x1E1E1E)와 명도차가 3단계뿐이라 눈으로 구분되지 않는다 → 패널마다 테두리를 그리는 이번 방식이 근본 해결
- `src/app/layout.rs:9` 확인: `SPLITTER_THICKNESS = 4` — 패널 사이에 4px 간격이 실제로 존재하므로, 인접 패널의 테두리 2개가 겹쳐 두꺼워 보이는 문제는 생기지 않는다
- `docs/plans/deferred.md` 확인: `## 대기` 15건 중 이번 작업과 겹치는 항목 없음 (FR-14 분할 프리셋은 레이아웃 전환 기능이라 경계 표시와 별개)
- `docs/prd.md:16·17·18·21·36` 확인: FR-1/2(분할)·FR-3(탭)·FR-6(주소창)·FR-21(고정 다크)에 닿는다. 네 FR 모두 **요구 문구는 그대로 만족**하며(추가/닫기/전환·다크 표시·분할·탐색 동작 불변) 표시 방식만 바뀐다 → PRD 갱신 불필요, 연결만 한다
- 위키 참조: vault 미설정 — 코드 1차 출처로 진행

## Progress Log

- T1-T2 완료 (커밋 a4c835b, 8445287+): `ui/widgets.rs` 추출 후 탭 스트립 전면 재작성. 빌드·clippy·테스트(lib 141) 전부 통과.
  - 결정: 탭 제목 폭 측정은 `ui.fonts(...)`가 `&Fonts`를 주어 `layout_no_wrap`(`&mut self`)을 부를 수 없다 — `Painter::layout_no_wrap`(`&self`, `egui-0.35.0/src/painter.rs:503`)로 대체했다.
  - 결정: 닫기 우선 규칙을 `ui.interact` 등록 순서와 `is_close_hit` 좌표 검사 두 겹으로 뒀다. quality 리뷰가 "누른 곳과 뗀 곳이 어긋나는 경계에서 좌표 검사가 추가로 막는다"고 정당성을 확인.
  - V-9: T2 귀속 시각 행은 전부 소스 대조 ✅, 실제 화면 결과 2행은 `⏳ 미확인`으로 F-8 인계.

## Phase Ledger

- (미시작)

## Retry Ledger

- (없음)
