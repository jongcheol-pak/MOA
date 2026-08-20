# Plan: 팝업 메뉴 항목 공통 시각 규약

## 요구 이해

> 원문: "메뉴 수정 작업 / 3번 이미지처럼 메뉴에서 마우스 오버ui 사각형인데 2번 이미지처럼 마우스 오버도 모서리가 둥글게 수정 / 4번 이미지처럼 마우스 오버시 문구가 왼쪽과 오른쪽 끝에 붙어 있는데 2번이미지처럼 간격을 좀 더 주워서 오버시에서 떨어져 보이도록 수정 / 5번 설정 메뉴 이미지를 보면 다른 컨텍스트 메뉴보다 폰트가 작아 보이는데 작은 경우 동일게 수정 / 현재 수정 사항은 공통 디자인으로 해서 다음 부터 동일하게 적용"

이해한 요구:

1. 우클릭 메뉴의 마우스 오버 하이라이트가 **각져** 있다 — 새 탭 메뉴(2번 이미지)처럼 **모서리를 둥글게** 한다.
2. 사이드바 등록 사이트 메뉴(4번 이미지)는 오버 시 **문구가 하이라이트 좌우 끝에 붙어** 있다 — 새 탭 메뉴처럼 **좌우 여백을 넓혀** 글자가 하이라이트 안에서 떨어져 보이게 한다.
3. 설정 메뉴(5번 이미지)가 다른 컨텍스트 메뉴보다 **작아 보인다** — 같은 크기로 맞춘다.
4. 위 셋을 **한 벌의 공통 디자인 값**으로 두어, 이후 만드는 메뉴도 따로 손대지 않아도 같은 모습이 되게 한다.

**요구 3의 원인은 글자 크기가 아니다(실측).** 사용자가 준 캡처에서 글자 높이를 재니 설정 메뉴·우클릭 메뉴 **둘 다 12px로 같았고**, 코드에서도 둘 다 13px 글꼴이다. 실제로 갈린 것은 **행 높이**다 — 설정 메뉴 18px(egui 기본 `interact_size.y`), 다른 메뉴 28px. 사용자 확인 결과 **행 높이만 28px로 통일**하고 글자 크기는 13px 그대로 둔다.

## Goal

메뉴 한 줄의 시각 값(행 높이·좌우 여백·모서리·hover 색)을 `ui::theme`의 토큰 한 벌로 모으고, 팝업 목록을 그리는 모든 자리가 그것을 거치게 한다. 값이 갈릴 자리를 소스 훑기 시험이 막는다.

## Out of Scope

- 메뉴 **폭**·항목 구성·문구 — 이번에 바꾸지 않는다(각 메뉴가 자기 폭을 정하는 현행 유지). 단 여백이 넓어져 라벨이 접히는 자리는 그 폭 상수를 늘린다(T2 ⓒ·T3 ⓓ).
- 팝업 **프레임** 모서리(`MENU_CORNER_RADIUS` 6px)·그림자·테두리 — 이미 공통이고 요구에 없다.
- 모달 대화(`ui::dialog`)의 버튼 줄 — 별개 부품이다(12px 모서리, 전폭 균등 분할).
- Windows 셸 우클릭 메뉴 — OS가 그린다(D21).
- 도크 아이콘 버튼의 hover(`dock.rs`의 `MENU_HOT` 사용처) — 메뉴 항목이 아니라 아이콘 버튼이다.

## Deferred / Follow-up

- 트리 메뉴·원격 메뉴의 화면 밖 보정 크기를 실측으로 바꾸기 — 이번에 행 높이가 토큰이 되지만 `FRAME_PAD`(8.0)는 여전히 어림값이라 대장 항목이 그대로 유효하다 (대장 2026-08-20 항목).
- `ui_sources` 재귀 헬퍼 3곳 중복 — 이번 신규 시험은 `theme.rs`의 기존 헬퍼를 재사용하므로 네 번째 사본이 생기지 않는다. 대장 항목 유지 (대장 2026-08-20 항목).

## Investigation Log

- **위키 참조**: `20_projects/personal/moa/feat-theme-i18n` — 팔레트 정본은 `src/ui/theme.rs` 한 곳이고, **전역 팔레트를 바꾸는 대신 `ui.scope` 국소 덮기**를 쓴다는 규약이 있다(전역을 바꾸면 다른 화면 버튼까지 함께 바뀐다). 규약은 문서가 아니라 **소스 훑기 시험 3부 구성**(AGENTS 한 줄 + 금지 패턴 시험 + 예외 목록)이 지킨다.
- **위키 참조**: `20_projects/personal/moa/feat-dialog-shell` — 공통 셸 선례. 공개 표면을 함수 둘로 좁히고 규약 시험(`대화는_모두_이_모듈을_거친다`)이 우회를 막는다. 이번 메뉴 공통화도 같은 형태를 따른다.
- **위키 참조**: `20_projects/personal/moa/decisions.md` — 2026-08-15 「우클릭 메뉴·토스트 영구 제외」는 **모달 버튼 줄 규칙**(버튼이 없어 규칙이 성립 안 함)에 대한 것이라 이번 항목 시각 토큰과 축이 다르다. 이번 계획을 막는 과거 결정 없음.
- **Deferred 대장 조회**: `## 대기` 96건. 주제 매칭 2건은 위 `## Deferred / Follow-up`에 반영. 전제 반증 스캔에서 이 계획의 전제를 부정하는 항목 없음. 잔량 96건 < 100건이라 소진 batch 미착수.
- **캡처 실측** (`C:\Users\jongc\Desktop\{2,3,4,5}.png`, PIL로 행 프로파일·색 히스토그램):
  - 글자 높이 — 설정 메뉴 12px, 우클릭 메뉴 12px, 사이드바 메뉴 12px. **셋이 같다.**
  - 줄 간격 — 설정 메뉴 21px(= 18 + `item_spacing.y` 3), 우클릭 메뉴 31px(= 28 + 3).
  - hover 채움 — 우클릭 메뉴·새 탭 메뉴 `#383838`(= `theme::MENU_HOT`), 사이드바 연결 메뉴 `#464646`(= egui dark 기본 `hovered.weak_bg_fill` = gray 70). **팔레트와 egui 기본이 갈려 있다.**
- **egui 0.35 기본값 확인** (`~/.cargo/registry/.../egui-0.35.0/src/`):
  - **`containers/menu.rs:23-29` `pub fn menu_style(style)`** — 팝업 안에서 `spacing.button_padding = vec2(2.0, 0.0)`으로 덮고 `inactive.weak_bg_fill = TRANSPARENT`·여러 `bg_stroke = NONE`을 세운다. `containers/popup.rs:239`(`Popup::menu`)·`:588`(`style.apply(ui.style_mut())`)이 이것을 팝업 `Ui`에 적용한다 — **그러므로 메뉴 안 좌우 여백의 실제 값은 전역 기본 4.0이 아니라 2.0이다.**
  - `style.rs:1454` `interact_size: vec2(40.0, 18.0)` — Button 최소 높이 18px의 근거(`widgets/button.rs:308`이 `min_size.y`를 여기에 맞춘다).
  - `style.rs:1413-1414` Body·Button 텍스트 모두 13.0 — 글자 크기가 이미 같다는 코드 근거.
  - `widget_style.rs:158-166` `button_style` — Button 채움은 `bg_fill`이 아니라 **`weak_bg_fill`**이고, 모서리는 `visuals.corner_radius`(dark 기본 hovered 3), 안쪽 여백은 `spacing.button_padding − bg_stroke.width`.
  - `widget_style.rs:135` — Button 라벨 글꼴은 `override_font_id`가 없으면 **`TextStyle::Body`**(Button이 아니다). 둘 다 13.0이라 현재 결과는 같다.
  - `containers/menu.rs:506` `Layout::top_down_justified` — 메뉴 안 버튼은 **메뉴 폭 전체로 늘어난다**. 4번 이미지의 넓은 hover가 이 때문이다.
  - **`containers/menu.rs:382-386` `SubMenuButton::show`** — 하위 메뉴가 열려 있는 동안 `inactive`를 **`open` 비주얼로 바꿔치기**한다. dark 기본 `open`은 `weak_bg_fill` gray(45)·모서리 2px라, 「보기」 행만 다른 색·모서리로 남는다.
  - `style.rs:1691-1698`·`1707-1713` — dark 기본 `hovered`·`open`의 `expansion`은 **둘 다 0.0**이다.
  - **`containers/menu.rs:508`** — 하위 팝업은 `.style(menu_config.style.clone())`로 **egui 기본 메뉴 스타일이 새로 적용된 별도 `Area`**다. 부모 팝업 `Ui`에 우리가 세운 값은 이어지지 않는다.
- **앱 글꼴 설정**: `ui/app.rs:147 install_fonts`는 `FontDefinitions`만 바꾸고 `text_styles`를 재정의하지 않는다 — 글자 크기는 egui 기본 13.0 그대로다.

### 전제 검증

| # | 전제 | 확인 근거 | 판정 |
|---|---|---|---|
| P1 | 설정 메뉴와 우클릭 메뉴의 글자 크기가 이미 같다 | 캡처 글자 높이 12px 동일 + `style.rs:1413-1414`(Body/Button 13.0) + `remote_menu.rs:223`(`proportional(13.0)`) | 확인 |
| P2 | 설정 메뉴가 작아 보이는 원인은 행 높이다 | `button.rs:308` + `interact_size.y = 18` → 캡처 줄 간격 21px(18+3)이 계산과 일치 | 확인 |
| P3 | Button 경로 hover 채움은 `weak_bg_fill`이라 `apply_dark`가 세운 `hovered.bg_fill`이 안 먹는다 | `widget_style.rs:160` `fill: visuals.weak_bg_fill` + 캡처 실측 `#464646`(egui 기본) | 확인 |
| P4 | 메뉴 안 Button은 메뉴 폭 전체로 늘어난다 | `containers/menu.rs:506` `top_down_justified` + 4번 이미지 hover 폭 | 확인 |
| P5 | `ui.style_mut()`으로 세운 값은 그 `Ui`와 자식에만 적용되고 전역에 남지 않는다 | egui `Ui::style_mut`가 `Arc<Style>`을 그 Ui에서만 복제·교체 | 확인 |
| P6 | `SubMenuButton`이 여는 하위 팝업은 부모 팝업의 `Ui` 스타일을 잇지 않는다 | `containers/menu.rs:508` `.style(menu_config.style.clone())` + `popup.rs:588` `style.apply` — 별도 `Area`에 egui 기본 메뉴 스타일이 새로 적용된다 | **확인** — 그러므로 하위 메뉴의 `menu_style` 호출은 대비가 아니라 **필수**다 |
| P7 | 팝업 프레임 안쪽 여백은 6px이라 항목 hover가 테두리에 닿지 않는다 | `style.rs:1451` `menu_margin: Margin::same(6)` + 2번 이미지 | 확인 |
| P8 | 메뉴 안 좌우 여백의 현재 값은 4px이 아니라 **2px**이다 | `containers/menu.rs:24` `button_padding = vec2(2.0, 0.0)` + `popup.rs:239`·`:588` | **확인** — 여백 증가폭은 한쪽 10px(양쪽 20px)이다 |
| P9 | 메뉴 항목의 평상시 배경은 **투명**이며 그것이 유지돼야 한다 | `containers/menu.rs:27` `inactive.weak_bg_fill = TRANSPARENT` | 확인 — 헬퍼가 `inactive` 채움을 세우면 모든 항목에 배경이 생긴다 |
| P10 | 하위 메뉴가 열린 동안 부모 행은 `open` 비주얼로 그려진다 | `containers/menu.rs:382-386` + `style.rs:1707-1713`(gray 45·모서리 2) | 확인 — `open`도 토큰으로 세워야 한다 |

## 시각 요소 분해

**기준**: 2번 이미지(새 탭 메뉴 — 사용자가 「이처럼」으로 지목한 모습)와 3·4·5번 이미지(고쳐야 할 모습). 원본 디자인 HTML이 아니라 **앱 자신의 화면**이 기준이므로 문구·항목 구성은 분해 대상이 아니고 **시각 속성만** 본다.

### 시각 속성 (목표값)

| 속성 | 목표 | 근거 |
|---|---|---|
| 행 높이 | 28px | 우클릭 메뉴·새 탭 메뉴의 현행. 사용자 확인(Q1) |
| 좌우 여백 | 12px | 우클릭 메뉴의 현행. 사용자 확인(Q2) |
| hover 모서리 | 4px | 새 탭 메뉴(`SITE_ROW_CORNER`)의 현행 = 2번 이미지 |
| hover 채움 | `#383838` (`theme::MENU_HOT`) | 우클릭·새 탭 메뉴의 현행. 사용자 확인(Q3) |
| 평상시 채움 | 투명 | egui 메뉴 기본(P9). 바꾸지 않는다 |
| 글자 크기 | 13px | 이미 전 메뉴 동일(P1). 바꾸지 않는다(D4) |
| 항목 확대(`expansion`) | 0 | hover 때 항목이 커지면 여백이 흔들린다. dark 기본도 0이지만 값에 기대지 않고 명시한다 |

### 참조 정합 인벤토리 — 팝업 목록 전수

`grep -rn "Popup::menu\|Popup::context_menu\|\.context_menu(\|Frame::menu(\|SubMenuButton" src/` 결과 전건 Read. 「현재 값」은 높이 / 좌우 여백 / hover 모서리 / hover 채움.

| # | 위치 | 그리는 방식 | 현재 값 | 목표 대비 어긋남 |
|---|---|---|---|---|
| 1 | `tabs.rs:588` → `menu.rs::panel_menu_items` | egui `Button` | 18 / 2 / 3 / `#464646` | 넷 다 |
| 2 | `menu.rs:120` `SubMenuButton` → `view_items` | egui `Button` (열린 동안 `open` 비주얼) | 18 / 2 / 3(열림 2) / `#464646`(열림 gray45) | 넷 다 |
| 3 | `list_details.rs:649` → `menu.rs::column_menu_items` | egui `Button` + `min_size(0,26)` | 26 / 2 / 3 / `#464646` | 넷 다 |
| 4 | `sidebar.rs:278` 카드 컨텍스트 | egui `Button` | 18 / 2 / 3 / `#464646` | 넷 다 |
| 5 | `sidebar.rs:620` 연결 메뉴 (4번 이미지) | egui `Button` + `min_size(0,28)` | 28 / 2 / 3 / `#464646` | 여백·모서리·채움 |
| 6 | `sidebar.rs:660` 사이트 컨텍스트 | egui `Button` + `min_size(0,28)` | 28 / 2 / 3 / `#464646` | 여백·모서리·채움 |
| 7 | `titlebar.rs:263` 설정 메뉴 (5번 이미지) | egui `Button` | 18 / 2 / 3 / `#464646` | 넷 다 |
| 8 | `queue_panel.rs:669` 전송 큐 행 메뉴 | egui `Button` | 18 / 2 / 3 / `#464646` | 넷 다 |
| 9 | `tabs.rs:276` 새 탭 메뉴의 `새 탭` 줄 | egui `Button` | 18 / 2 / 3 / `#464646` | 넷 다 |
| 10 | `tabs.rs:318` `show_site_row` (2번 이미지 = 기준) | painter | 28 / 10 / 4 / `#383838` | 여백만(10 → 12) |
| 11 | `panel.rs:1542` → `remote_menu.rs:207` `menu_row` (3번 이미지) | painter | 28 / 12 / **0** / `#383838` | 모서리만 |
| 12 | `tree.rs:300` → `tree.rs:812` `menu_row` | painter | 28 / 12 / **0** / `#383838` | 모서리만 |
| 13 | `widgets.rs:348` 드롭다운 → `widgets.rs:716` `menu_row` | painter | 28 / 8 / **0** / `#383838` | 여백·모서리 |

> 9번과 10번은 **같은 팝업 안**인데 한 줄만 `Button`이고 나머지는 painter다 — 여백·모서리가 한 메뉴 안에서 갈리는 가장 뚜렷한 자리다.
>
> `site_manager.rs`·`settings_dialog.rs`의 드롭다운은 `widgets::dropdown_field` 경유라 13번에 흡수된다.

## 동반 변경 판정

| 대상 | 구분 | 처리 |
|---|---|---|
| `AGENTS.md`의 「팝업 메뉴」 규약(지금은 모서리만 다룬다) | **필수** — 항목 토큰이 생기면 그 절이 실제와 어긋난다 | T4에 편입 |
| `titlebar.rs`의 `설정_메뉴_폭_안에서_라벨이_한_줄로_그려진다` 시험 — `ui.style().spacing.button_padding.x`를 읽어 줄바꿈 한계를 잰다 | **필수** — 메뉴 스타일을 적용하지 않은 `Ui`에서 읽으면 실제(현재 2px, 이후 12px)와 어긋나 시험이 거짓을 지킨다 | T2에 편입 |
| `titlebar.rs`의 `SETTINGS_MENU_PADDING`(24.0) — 「버튼 여백 + 숨 쉴 자리」의 합으로 정의된 값 | **필수** — 실제 여백이 2px×2 = 4라 지금은 20px이 여유분인데, 여백이 12px×2 = 24가 되면 그 여유가 **0으로 사라져** 2026-08-19의 「라벨이 두 줄로 접힘」 회귀 위험이 생긴다 | T2에 편입 — 여유분을 별도 상수로 분리 |
| `sidebar.rs`의 `MENU_ROW_HEIGHT`(28.0)·`menu.rs`의 `COLUMN_MENU_ROW`(26.0)·`tabs.rs`의 `SITE_ROW_CORNER`(4)·`SITE_ROW_PAD_X`(10) — 같은 값을 파일마다 다시 정의 | **필수** — 토큰이 생겼는데 사본이 남으면 정본이 다시 갈린다 | T2·T3에 편입 |
| `theme.rs`의 `MENU_CORNER_RADIUS` doc 주석 — 「메뉴 규약은 모서리 하나」라는 서술 | **필수** — 토큰이 넷으로 늘면 그 서술이 낡는다 | T1에 편입 |
| `remote_menu.rs::menu_size()` — `ROW_HEIGHT`로 팝업 크기를 어림해 화면 밖 보정에 쓴다 | **무관** — 행 높이 28은 그대로라 계산 결과가 달라지지 않는다(상수 출처만 토큰으로 바뀐다) | T3에서 출처만 교체 |
| `docs/prd.md` FR-21(고정 다크) | **무관** — 색 팔레트 값이 아니라 항목 치수·hover 색 정합이며 FR 문면이 치수를 규정하지 않는다 | 건드리지 않음 |
| `README.md` | **무관** — 기능·UI 구성이 바뀌지 않고 같은 메뉴의 모습만 통일된다 | 건드리지 않음 |

## Impact Analysis

### 4-A. 심볼 추적

팝업 전수는 위 `### 참조 정합 인벤토리` 13행이 정본이다(같은 표를 두 곳에 두지 않는다).

**`theme::MENU_HOT` 사용처 전수** (`grep -rn "MENU_HOT"`, 7건): 인벤토리 10~13 네 곳 + `dock.rs:339`(아이콘 버튼 — 대상 밖) + `theme.rs`(정의) + `sidebar.rs:700`(사이트 행 hover — 메뉴 아님, 대상 밖).

**메뉴 치수를 단언하는 시험 전수** (`grep -rn "MENU_ROW_HEIGHT\|SITE_ROW_HEIGHT\|MENU_CORNER_RADIUS" | grep assert`, 3건): `sidebar.rs:865`(`MENU_ROW_HEIGHT == 28.0`) · `theme.rs:254`(모서리 ≠ 대화 모서리) · `queue_panel.rs:698`(`SITE_ROW_HEIGHT == 28.0` — 전송 큐 사이트 탭 행이라 메뉴 아님, 대상 밖).

### 4-B. 계약·직렬화

없음 — 화면 치수만 바뀐다. 설정 파일 스키마·세션 직렬화·공개 API에 닿지 않는다. `widgets::menu_row`가 `pub(crate)`로 승격되지만 crate 내부다.

### 4-C. 영향 받는 테스트

| 시험 | 영향 | 처리 |
|---|---|---|
| `titlebar.rs::설정_메뉴_폭_안에서_라벨이_한_줄로_그려진다` | `button_padding.x`를 읽어 줄바꿈 한계를 잰다 — 메뉴 스타일 적용 후 읽어야 실제와 같다 | T2에서 스타일 적용 후 읽도록 고친다 |
| `sidebar.rs::assert_eq!(MENU_ROW_HEIGHT, 28.0)` | 상수가 토큰으로 대체되면 그 이름이 사라진다 | T2에서 토큰 단언으로 바꾼다 |
| `menu.rs::패널_메뉴는_요청한_순서와_문구를_그린다`·`보기_하위_메뉴는…` | `Ui`를 만들어 항목 함수를 직접 부른다 — 스타일 헬퍼가 그 안에서 불려도 값만 바뀌고 문구·순서는 그대로 | 통과 예상, T2에서 실행 확인 |
| `remote_menu.rs::메뉴가_한_프레임을_그린다` | painter 경로가 공통 함수로 바뀐다 | T3에서 실행 확인 |
| `tabs.rs::사이트가_있으면_메뉴가_그_사이트를_싣는다` | `show_site_row` 상수 출처가 바뀐다 | T3에서 실행 확인 |
| `theme.rs::팝업_메뉴는_모서리를_따로_적지_않는다` | 그대로 유효(프레임 모서리 규약) | 유지 |
| 신규 규약 시험 | 없음 → T4에서 추가 |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 | 재사용/신규 사유 |
|---|---|---|
| `theme::MENU_ITEM_HEIGHT` 등 토큰 3종 | `sidebar::MENU_ROW_HEIGHT`(28) · `menu::COLUMN_MENU_ROW`(26) · `tabs::SITE_ROW_*`(28/10/4) · `remote_menu::ROW_HEIGHT`(28) — 같은 뜻의 상수가 **파일마다 재정의**돼 있다 | 신규(정본 통합). 기존 사본은 T2·T3에서 제거해 이름이 남지 않게 한다 |
| `theme::menu_style(ui)` | `widgets::design_button`이 `ui.scope` 안에서 국소로 위젯 색을 덮는 같은 형태를 쓴다(`widgets.rs:118`) | 신규. `design_button`은 버튼 하나를 그리는 함수라 메뉴 전체 `Ui`에 스타일만 세우는 용도로 쓸 수 없다. **형태는 그것을 따른다**. 이름이 egui의 `containers::menu::menu_style`과 같지만 `theme::` 경로로 갈리고 하는 일(그 위에 앱 값을 덮는다)도 이어진다 |
| `widgets::menu_row(ui, label, enabled)` | `widgets.rs:716`에 이미 같은 이름의 private 함수가 있고, `remote_menu.rs:207`·`tree.rs:812`가 **거의 같은 사본**을 각자 들고 있다(3회 중복) | **기존 것을 확장해 재사용** — `enabled` 인자를 더해 셋을 하나로 모은다 |
| `widgets::hover_backdrop` | 이미 있다 — 정사각형으로 잘라 그리는 아이콘 버튼용이라 메뉴 행에 맞지 않는다 | 재사용 안 함(사유 명시) |

### 4-E. 동반 변경 판정

위 `## 동반 변경 판정` 표 참조 — 필수 5건은 T1·T2·T3·T4에 편입, 선택 0건, 무관 3건.

## Decisions

- **D1. 토큰의 정본은 `src/ui/theme.rs`** — 이미 `MENU_CORNER_RADIUS`(팝업 프레임)가 거기 있고, 위키 `feat-theme-i18n`이 「팔레트 정본 한 곳」을 규약으로 적었다. Source: `src/ui/theme.rs:83-94`, 위키 `feat-theme-i18n`.
- **D2. 전역 스타일을 바꾸지 않고 메뉴 `Ui`에만 국소로 덮는다** — `apply_dark`에서 `spacing.button_padding`·`interact_size`를 바꾸면 앱 전체 버튼이 함께 커진다. 게다가 egui가 팝업마다 자기 `menu_style`을 새로 적용하므로(P8) 전역에 세워도 메뉴 안에서 덮여 **효과가 없다**. Source: `containers/popup.rs:588`, 위키 `feat-theme-i18n`, `widgets.rs:118`.
- **D3. 행 높이 28px / 좌우 여백 12px / 모서리 4px / hover `#383838`** — 사용자 확인(2026-08-20). 근거: 행 높이·hover는 우클릭 메뉴와 새 탭 메뉴의 현행 값, 여백 12px는 우클릭 메뉴 현행, 모서리 4px는 새 탭 메뉴(`SITE_ROW_CORNER`)의 현행.
- **D4. 글자 크기는 13px 그대로 두고 토큰으로도 두지 않는다** — 이미 egui 기본(Body 13.0)과 painter 경로(13.0)가 같고, 토큰을 만들면 「메뉴만 다른 글꼴 크기를 쓴다」는 뜻이 생겨 앱 전역 글꼴과 갈린다(`ui::dialog`가 버튼 글꼴을 따로 정하지 않는 것과 같은 이유). Source: 위키 `feat-dialog-shell`.
- **D5. painter 경로 3곳은 `widgets::menu_row` 하나로 모은다** — 바이트 단위로 거의 같은 사본이 3개다(공통화 문턱 3회 충족). Source: `remote_menu.rs:207`·`tree.rs:812`·`widgets.rs:716`.
- **D6. `tabs::show_site_row`는 공통 함수로 흡수하지 않는다** — 한 줄에 상태 점·이름·프로토콜 셋이 서로 다른 정렬로 앉아 라벨 하나짜리 `menu_row`로 표현되지 않는다(그 함수의 기존 주석이 같은 이유를 적었다). **토큰만 공유**한다. Source: `tabs.rs:314-317`.
- **D7. 규약은 소스 훑기 시험이 지킨다** — 이 레포가 아이콘·문구·모달·프레임 모서리에서 네 번 쓴 형태다. 검사는 파일 존재 여부가 아니라 **「팝업을 여는 구문 수 ≤ 공통 경로 호출 수」**를 견준다 — 한 파일에 팝업이 여럿인 곳(`sidebar.rs` 3, `tabs.rs` 2)에서 하나만 고쳐도 통과하는 사각을 막기 위해서다. Source: AGENTS.md Conventions, `theme.rs:213`.
- **D8. Button 채움은 `weak_bg_fill`과 `bg_fill`을 함께 세우되 `inactive`는 투명을 지킨다** — egui가 버튼 프레임 채움으로 `weak_bg_fill`을 읽고(`widget_style.rs:160`) 다른 위젯은 `bg_fill`을 읽는다. 다만 `inactive`에 채움을 주면 **모든 항목에 배경이 생겨** 메뉴가 버튼 목록처럼 보인다(P9). Source: `containers/menu.rs:27`.
- **D9. `open` 상태도 함께 세운다** — 하위 메뉴가 열린 동안 부모 행은 `open` 비주얼로 그려지므로(P10), 그것을 두면 「보기」 행만 gray(45)·모서리 2px로 남아 요구 1·2가 그 자리에서 어긋난다. Source: `containers/menu.rs:382-386`.

## Tasks

- [x] **T1. `theme.rs`에 메뉴 항목 토큰과 국소 스타일 헬퍼를 만든다** — Type C
  - **Design**: ① 배치 — `src/ui/theme.rs`, 기존 `MENU_CORNER_RADIUS` 바로 아래. ② 신규 심볼과 책임 — `MENU_ITEM_HEIGHT`(28.0) · `MENU_ITEM_PAD_X`(12.0) · `MENU_ITEM_CORNER_RADIUS`(4) 세 상수와 `menu_style(ui: &mut egui::Ui)`(그 `Ui`와 자식에만 메뉴 항목 스타일을 세운다 — 여백·최소 높이·모서리·`hovered`/`active`/`open` 채움·`expansion` 0). hover 색은 기존 `MENU_HOT`을 그대로 쓴다(새 상수를 만들지 않는다). ③ 의존 방향 — `theme`은 egui만 참조한다. 호출자는 `ui` 하위 모듈들. ④ 비추상화 — 메뉴 **폭**·캡션 글자 크기·프레임 여백은 토큰으로 만들지 않는다(메뉴마다 정당하게 다르다). 글자 크기 토큰도 만들지 않는다(D4). 팝업을 여는 래퍼 함수도 만들지 않는다(T2 ④).
  - **Acceptance**:
    - `theme::MENU_ITEM_HEIGHT == 28.0` · `MENU_ITEM_PAD_X == 12.0` · `MENU_ITEM_CORNER_RADIUS == 4`.
    - `menu_style(ui)` 호출 뒤 그 `Ui`에서 다음이 성립함을 단언하는 시험이 있다:
      - `spacing.button_padding.x == 12.0` · **`spacing.button_padding.y == 0.0`** · `spacing.interact_size.y == 28.0`
        - y를 함께 단언하는 이유: `interact_size.y`는 **최소**일 뿐이라(`button.rs:308`) 세로 여백을 크게 주면 행이 28px를 넘어 커진다. 토큰은 가로 하나뿐이므로 세로는 egui 메뉴 기본값(0.0)을 그대로 지킨다는 것을 시험이 못 박는다
      - `visuals.widgets.hovered.weak_bg_fill == MENU_HOT` · `active.weak_bg_fill == MENU_HOT` · **`open.weak_bg_fill == MENU_HOT`**
      - `hovered.corner_radius == CornerRadius::same(4)` · `active` · **`open`** · `inactive`도 같은 모서리
      - `hovered.expansion == 0.0` · `open.expansion == 0.0`
      - **`inactive.weak_bg_fill`이 투명으로 남는다**(D8·P9 — 평상시 항목에 배경이 생기지 않는다)
    - **호출하지 않은 형제 `Ui`의 값이 변하지 않는다**(전역 오염 없음)를 단언하는 시험이 있다 — D2가 지키려는 것이 이것이다.
    - **egui 기본 메뉴 스타일이 먼저 적용된 상태에서 덮어써도 위 값이 성립한다**를 단언한다 — 실제 팝업은 `Popup::menu`가 egui `menu_style`을 먼저 적용한 뒤 우리 헬퍼가 불리므로(P8), 시험도 그 순서를 흉내 낸다(`egui::containers::menu::menu_style(ui.style_mut())` 후 호출).
    - `MENU_CORNER_RADIUS`의 doc 주석이 「프레임 모서리와 항목 토큰은 다른 부품」임을 적는다.
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: ⓐ `inactive`에 채움을 주면 안 된다(P9) — 모서리와 여백만 세우고 `weak_bg_fill`은 egui가 세운 투명을 유지한다. ⓑ `expansion`은 dark 기본도 0.0이지만(m1 정정) **값에 기대지 않고 명시**한다 — 기본이 바뀌면 hover 때 항목이 커져 여백이 흔들린다. ⓒ `noninteractive`(비활성 항목)에도 모서리를 세운다 — 설정 메뉴의 `업데이트`·`릴리즈 노트`가 그 상태다.
  - **Halt Forecast**: 없음 — 새 심볼 추가뿐이고 기존 호출부가 없다.
  - **Files**: 주 — `src/ui/theme.rs`
- [x] **T2. egui `Button`으로 그리는 팝업 아홉 곳이 헬퍼를 거치게 한다** — Type D
  - **Design**: ① 배치 — 각 팝업의 `show(|ui| …)` 클로저 **첫 줄**에서 `theme::menu_style(ui)`를 부른다(팝업을 여는 쪽이 부른다는 한 가지 규칙). ② 신규 심볼 — 없음(T1 헬퍼 사용). ③ 의존 방향 — `ui::{tabs,menu,sidebar,titlebar,queue_panel,list_details} → ui::theme` 단방향, 기존과 같다. ④ 비추상화 — 팝업을 여는 공통 래퍼 함수(`open_menu(...)`)는 만들지 않는다 — 아홉 자리가 각각 다른 응답·상태를 다뤄 클로저 시그니처가 통일되지 않고, 한 줄 호출로 끝나는 일에 간접층을 두면 실제 동작 추적이 늘어난다.
  - **Acceptance**:
    - 인벤토리 1~9번 아홉 자리가 모두 `theme::menu_style(ui)`를 거친다. **`SubMenuButton`이 여는 보기 하위 메뉴(2번)도 자기 클로저에서 부른다** — 하위 팝업은 부모 스타일을 잇지 않으므로(P6) 이것은 필수다.
    - `sidebar::MENU_ROW_HEIGHT`·`menu::COLUMN_MENU_ROW`가 **사라지고** 그 자리의 `min_size` 지정도 사라진다(높이는 헬퍼가 세운 `interact_size.y`가 정한다 — `button.rs:308`이 그 경로다). 두 이름으로 레포를 검색해 잔존 0건.
    - `sidebar.rs`의 `assert_eq!(MENU_ROW_HEIGHT, 28.0)`가 `assert_eq!(theme::MENU_ITEM_HEIGHT, 28.0)`로 바뀐다.
    - **`titlebar::SETTINGS_MENU_PADDING`이 「항목 여백 + 여유분」으로 분해된다** — `theme::MENU_ITEM_PAD_X * 2.0 + SETTINGS_MENU_BREATH`(여유분은 현행과 같은 20.0). 주석이 그 분해와 「여유분이 0이 되면 라벨이 경계에 붙어 접힘 회귀가 난다」는 이유를 적는다. 값은 24 → 44로 커진다.
    - `설정_메뉴_폭_안에서_라벨이_한_줄로_그려진다` 시험이 **메뉴 스타일을 적용한 `Ui`에서** `button_padding.x`를 읽는다(egui 메뉴 스타일 → `theme::menu_style` 순서까지 흉내 낸다). 한국어·English 둘 다 접힌 항목 0.
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: ⓐ 열 메뉴(3번)는 항목 높이가 26 → 28로 커지므로 체크 글리프 세로 정렬이 어긋나지 않는지 본다(글리프는 `LayoutJob`으로 라벨과 같은 줄에 있어 함께 가운데 정렬된다). ⓑ 설정 메뉴의 비활성 두 항목은 `add_enabled(false)`라 `noninteractive` 상태로 그려진다 — 그 상태에도 모서리·높이가 적용되는지 확인(T1 ⓒ와 짝). ⓒ **여백이 2 → 12로 늘어 라벨에 남는 자리가 좌우 합 20px 줄어든다** — 폭이 고정된 `CONNECT_MENU_WIDTH`(246)·`SITE_MENU_WIDTH`(180)·`COLUMN_MENU_WIDTH`(186)·`NEW_TAB_MENU_WIDTH`(250) 넷에서 **한국어·English 각각 가장 긴 라벨이 접히지 않는지** 확인하고, 접히면 그 폭 상수를 늘린다(Out of Scope의 단서). ⓓ 「보기」 항목은 오른쪽에 `CARET_RIGHT` 아이콘이 붙어 라벨 폭 계산에 그 몫이 더 필요하다 — ⓒ 확인에 포함한다.
  - **Halt Forecast**: 없음 — 크레이트 내부 호출이고 상수 제거 누락은 컴파일 오류다. 파일을 지우거나 옮기지 않는다.
  - **Files**: 주 — `src/ui/tabs.rs`, `src/ui/menu.rs`, `src/ui/sidebar.rs`, `src/ui/titlebar.rs`, `src/ui/queue_panel.rs`, `src/ui/list_details.rs`
- [x] **T3. painter로 그리는 네 곳을 공통 행 함수와 토큰으로 모은다** — Type D
  - **Design**: ① 배치 — `src/ui/widgets.rs`의 기존 private `menu_row`를 `pub(crate)`로 올려 공통 행 함수로 삼는다. ② 신규 심볼과 책임 — 없음(기존 함수 확장). 시그니처는 `menu_row(ui, label, enabled) -> bool`로, 비활성이면 hover를 그리지 않고 글자를 `TEXT_DIM`으로 그리며 클릭을 돌려주지 않는다(`remote_menu`·`tree` 사본이 이미 하던 동작). ③ 의존 방향 — `ui::{remote_menu,tree} → ui::widgets`. `widgets`는 `theme`만 참조한다(기존과 같다). ④ 비추상화 — `tabs::show_site_row`를 이 함수로 흡수하지 않는다(D6). 드롭다운 전용 옵션(스크롤·선택 표시)도 넣지 않는다.
  - **Acceptance**:
    - `remote_menu.rs`·`tree.rs`의 `menu_row` 사본이 **사라지고** 두 파일이 `widgets::menu_row`를 부른다. `fn menu_row`로 레포를 검색해 정의가 **1건**(`widgets.rs`)만 남는다.
    - 공통 `menu_row`가 `theme::MENU_ITEM_HEIGHT`·`MENU_ITEM_PAD_X`·`MENU_ITEM_CORNER_RADIUS`·`MENU_HOT`을 쓴다 — 이 함수에 행 높이·여백·모서리 리터럴이 남지 않는다.
    - `tabs::SITE_ROW_CORNER`·`SITE_ROW_PAD_X`·`SITE_ROW_HEIGHT`가 토큰을 가리키거나 사라진다(값이 두 곳에 적히지 않는다). `show_site_row`의 hover가 `MENU_ITEM_CORNER_RADIUS`로 그려지고 좌우 여백이 12px가 된다.
    - `remote_menu::menu_size()`가 `theme::MENU_ITEM_HEIGHT`를 쓴다(값 28은 그대로라 화면 밖 보정 결과가 달라지지 않는다).
    - 드롭다운 목록(13번)의 행이 같은 여백·모서리로 그려진다 — `FORM_FIELD_PAD_X`(8)를 쓰던 자리가 토큰으로 바뀐다.
    - `cargo test` 전건 통과(특히 `메뉴가_한_프레임을_그린다`·`사이트가_있으면_메뉴가_그_사이트를_싣는다`) · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: ⓐ 드롭다운 행은 종전에 `Sense::click()`만 썼고 비활성 개념이 없다 — `enabled = true`로 부르면 동작이 같다. ⓑ 드롭다운은 `FORM_FIELD_HEIGHT`(28)와 행 높이가 같아 높이 변화가 없다. ⓒ `tree.rs`의 `MENU_ROW_HEIGHT`는 팝업 크기 어림(`size` 계산)에도 쓰이므로, 상수를 지울 때 그 계산이 토큰을 가리키게 한다. ⓓ **드롭다운은 여백이 8 → 12로 늘고 폭이 필드 폭(`ui.set_width(width)`)에 묶여 있다** — 글꼴 이름처럼 긴 항목이 잘리지 않는지 확인한다(잘리면 그 자리는 말줄임으로 그려지므로 폭 상수를 늘릴지 판단). ⓔ `show_site_row`의 여백이 10 → 12로 늘면 이름이 그릴 수 있는 폭이 4px 줄어든다 — 이름·프로토콜이 겹치지 않는지 확인(그 함수가 이미 `max(0.0)`으로 자르므로 겹치지는 않는다).
  - **Halt Forecast**: 없음 — 함수 이동·가시성 승격이며 누락은 컴파일 오류다.
  - **Files**: 주 — `src/ui/widgets.rs`, `src/ui/remote_menu.rs`, `src/ui/tree.rs`, `src/ui/tabs.rs`
- [ ] **T4. 규약을 소스 훑기 시험과 AGENTS.md에 못 박는다** — Type C
  - **Design**: ① 배치 — 시험은 `src/ui/theme.rs`의 기존 `mod tests`(`ui_sources` 헬퍼를 재사용해 사본을 늘리지 않는다), 규약 문장은 `AGENTS.md`의 Conventions 「팝업 메뉴」 절. ② 신규 심볼과 책임 — `#[test] 팝업_메뉴는_항목_스타일을_거친다`(파일마다 **팝업을 여는 구문 수**와 **공통 경로 호출 수**를 견준다) + 검사기 자신을 시험하는 `#[test]`. ③ 의존 방향 — 시험 전용, 프로덕션 코드에 닿지 않는다. ④ 비추상화 — 「어떤 값을 썼는가」까지 파싱하지 않는다(정규식으로 스타일 세부를 검사하면 형태가 조금만 달라도 거짓 실패한다). **경로를 거쳤는가와 그 횟수**만 본다.
  - **Acceptance**:
    - `src/ui`를 하위 폴더까지 재귀로 훑어, 파일마다 **팝업을 여는 구문 수 > 공통 경로 호출 수**이면 실패하는 시험이 있다.
      - 팝업을 여는 구문: `Popup::menu(` · `Popup::context_menu(` · `.context_menu(` · `Frame::menu(` · **`SubMenuButton`**(하위 팝업은 부모 스타일을 잇지 않으므로 자체 호출이 필요하다 — P6).
      - 공통 경로 호출: `theme::menu_style(` 또는 무자격 `menu_style(` / `widgets::menu_row(` 또는 무자격 `menu_row(`.
      - **개수를 견주는 이유**: 한 파일에 팝업이 여럿인 곳(`sidebar.rs` 3, `tabs.rs` 2)에서 하나만 고쳐도 존재 검사는 통과한다.
    - **계수 규칙을 넷 다 지킨다** — 이것을 적지 않으면 오차가 서로 상쇄돼 「통과하지만 아무것도 보증하지 않는」 시험이 된다(2라운드 M1):
      - ⓐ **자격/무자격을 배타로 센다** — `theme::menu_style(`는 `menu_style(`를 부분 문자열로 포함하므로, 자격 형태를 먼저 세어 그 자리를 지운 뒤 남은 것에서 무자격을 센다(또는 `(?:theme::)?menu_style\(` 한 대안으로 1회만 센다). 그러지 않으면 한 호출이 2로 세어져 `tabs.rs`가 팝업 2 vs 호출 2로 통과한다.
      - ⓑ **정의는 호출로 세지 않는다** — `fn ` 접두가 붙은 `fn menu_row(`(`widgets.rs`의 정의)를 제외한다.
      - ⓒ **`//`로 시작하는 줄은 세지 않는다** — `menu.rs`의 주석이 백틱 안에 `SubMenuButton`을 담고 있어, 이 규칙이 없으면 그 파일의 팝업 수가 부풀어 거짓 실패한다. 예외 목록을 늘리는 대신 이 규칙으로 해결한다(ⓑ의 `list_details.rs` 판단과 같은 취지).
      - ⓓ **같은 자리의 이중 패턴을 한 번만 센다** — `widgets.rs:348-352`는 팝업 한 자리인데 `Popup::menu(`와 `Frame::menu(`가 함께 잡힌다. `Frame::menu(`는 프레임 지정일 뿐이므로 같은 파일에 `Popup::menu(`가 있으면 opener로 세지 않는다.
      - **egui 자신의 `menu_style`은 인정하지 않는다** — `egui::containers::menu::menu_style(`도 무자격 매처에 걸리는데 그것은 앱 토큰을 세우지 않는다. 자격 경로(`theme::`) 또는 같은 모듈 무자격 호출만 인정하고 `egui::` 접두는 제외한다.
    - 예외 목록은 **파일 경로 전체로 견주고**(이름만 보면 하위 폴더의 동명 파일이 조용히 빠진다 — 기존 모서리 시험과 같은 규칙) 각 예외에 사유 주석이 붙는다. 예외: `theme.rs`(규약을 설명하느라 그 문자열을 담고, T1 시험이 egui 쪽 `menu_style`을 부른다) · `panel.rs`(**프레임만 열고 행은 `remote_menu`가 그린다** — 그 모듈이 `widgets::menu_row`를 거치므로 규약은 지켜진다).
    - **검사기 자신을 시험하는 `#[test]`가 있다** — 통과해야 할 형태(팝업 2 + 호출 2)와 실패해야 할 형태를 각각 문자열로 든다: 팝업 2 + 호출 1 · `SubMenuButton`만 있고 호출 0 · **`theme::menu_style(` 하나를 2로 세면 통과해 버리는 형태(ⓐ 회귀)** · **`fn menu_row(` 정의만 있고 호출이 없는 형태(ⓑ 회귀)** · **주석 줄의 `SubMenuButton`(ⓒ 회귀)** · **`Popup::menu(`+`Frame::menu(`가 한 자리인 형태(ⓓ 회귀)**.
    - `AGENTS.md` Conventions의 「팝업 메뉴」 절이 **항목 토큰 규약**을 함께 적는다 — 정본 위치(`theme::MENU_ITEM_*`)·값 넷(28/12/4/`#383838`)·`menu_style` 호출 규칙(하위 메뉴 포함)·시험 이름. 기존 프레임 모서리 문단은 유지한다.
    - 규약 문장이 실제 값과 일치한다(문서에 적은 값이 코드 상수와 같다).
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: ⓐ 시험이 **자기 자신을 잡지 않게** 예외 처리하되, 예외를 넓게 열면 그 파일의 진짜 위반이 함께 빠져나간다 — 예외는 위 둘로 한정하고 사유를 적는다. ⓑ `list_details.rs`는 팝업을 열되 항목은 `menu.rs`가 그리므로 **예외가 아니라 T2에서 `menu_style`을 부르게 해** 통과시킨다(예외를 늘리지 않는 쪽). ⓒ 주석·문자열 안의 패턴까지 세면 거짓 실패가 난다 — 이 시험은 그 구분을 하지 않으므로, 규약을 설명하는 파일(`theme.rs`)을 예외에 둔 것이 그 대응이다.
  - **Halt Forecast**: 없음 — 시험과 문서만 더한다. AGENTS.md 수정은 기존 절 안에서의 추가라 구조를 바꾸지 않는다.
  - **Files**: 주 — `src/ui/theme.rs`, `AGENTS.md`

## 사전 승인 항목 (일괄 승인 대상)

- `widgets::menu_row`의 `pub(crate)` 승격과 인자 추가 (crate 내부 계약 변경, T3)
- `sidebar::MENU_ROW_HEIGHT`·`menu::COLUMN_MENU_ROW`·`tabs::SITE_ROW_CORNER` 등 중복 상수 제거 (T2·T3)
- 라벨이 접히는 경우에 한해 메뉴 폭 상수 증가 (T2 ⓒ·T3 ⓓ)
- `AGENTS.md` Conventions 절 갱신 (T4)
- 로컬 작업 브랜치 commit (task 단위)

## 불가피한 Halt (위임 불가)

- push · 병합 · 태그 · 릴리즈 · PR
- 파괴적 작업 (해당 없음 — 이번 계획에 파일 삭제·이동이 없다)
- 화면 확인이 필요한 판정 — 빌드·시험은 값만 지키므로 **실제 화면의 모습은 사용자 확인(HUMAN-VERIFY)** 이다. 구현 후 최종 보고에서 확인을 청한다.

## Open Questions

- [x] Q1. 설정 메뉴를 어떻게 맞출까 — **행 높이만 28px로 통일**(글자는 13px 유지). 실측으로 글자 크기가 이미 같음을 확인한 뒤의 결정.
- [x] Q2. 좌우 여백 통일 값 — **12px**(현행 우클릭 메뉴 값).
- [x] Q3. 적용 범위 — **모든 팝업 목록**(우클릭·설정·새 탭·사이드바·전송 큐·열 메뉴 + 설정 대화의 드롭다운). hover 색도 팔레트 `#383838`로 통일.

## 리뷰 이력

**1라운드** — BLOCKER 0 / MAJOR 4 / MINOR 5. **전건 수용**(기각 없음). 지적된 사실은 모두 egui 소스에서 직접 재확인했다.

| 라운드 | 지적 | 심각도 | 반영 방식 |
|---|---|---|---|
| 1 | M1 `open` 상태 미커버 — 하위 메뉴 열린 동안 「보기」 행이 토큰을 벗어난다 | MAJOR | D9 신설, T1 Design·Acceptance에 `open` 단언 추가 (`containers/menu.rs:382-386` 확인) |
| 1 | M2 「현재 여백 4px」이 사실과 다르다 — 메뉴 안 실제 값은 2px | MAJOR | Investigation Log·P8·인벤토리 표 전면 정정, 증가폭을 양쪽 20px로 다시 계산 (`containers/menu.rs:24`·`popup.rs:239,588` 확인) |
| 1 | M3 `SETTINGS_MENU_PADDING = PAD_X*2`는 여유분을 0으로 만든다 | MAJOR | T2 Acceptance를 「여백 + `SETTINGS_MENU_BREATH`(20.0)」 분해로 교체 |
| 1 | M4 소스 훑기 시험이 파일 단위 근사 + `SubMenuButton` 누락 | MAJOR | D7·T4를 **개수 비교**로 바꾸고 스캔 패턴에 `SubMenuButton` 추가 |
| 1 | m1 「dark 기본 `expansion`이 0이 아니다」는 사실과 다르다 | MINOR | T1 Edge Case ⓑ를 「기본도 0이지만 값에 기대지 않는다」로 정정 (`style.rs:1691-1698` 확인) |
| 1 | m2 `widgets.rs`의 무자격 `menu_row(` 호출이 거짓 실패를 낸다 | MINOR | T4 매처에 무자격 호출 형태 추가 |
| 1 | m3 드롭다운 여백 증가(8→12) 검토 누락 | MINOR | T3 Edge Case ⓓ 신설 |
| 1 | m4 `## 시각 요소 분해` 섹션 부재 | MINOR | 섹션 신설 — `### 시각 속성`(목표값) + `### 참조 정합 인벤토리`(4-A 표를 승격) |
| 1 | m5 전제 P6은 코드로 확정 가능한데 미확인으로 남았다 | MINOR | P6을 「확인」으로 승격하고, T2에서 하위 메뉴 호출을 **필수**로 문면 수정 |

**2라운드** — BLOCKER 0 / MAJOR 1 / MINOR 3. **전건 수용**(기각 없음). 1라운드 9건 중 8건은 닫혔고(특히 M3은 리뷰어가 `24 − 2×2 = 20`으로 재검산해 `SETTINGS_MENU_BREATH = 20.0`이 현행 여유분과 정확히 같음을 확인), 남은 것은 **1라운드 M4를 고치며 새로 쓴 계수 규칙의 버그**다(같은 지적의 재발이 아니라 새 구현안의 결함 — 리뷰어도 RECURRING으로 올리지 않았다).

| 라운드 | 지적 | 심각도 | 반영 방식 |
|---|---|---|---|
| 2 | M1 계수 규칙이 네 곳에서 어긋나 상쇄된다(자격/무자격 이중계수 · 주석 · opener 이중 패턴 · 정의를 호출로 계수) — 지금은 전부 통과하지만 `tabs.rs`·`sidebar.rs`의 누락을 못 잡는다 | MAJOR | 수용 — T4 Acceptance에 계수 규칙 ⓐ~ⓓ + egui 경로 제외를 명시하고, 검사기 자기 시험에 **네 오차 각각의 회귀 케이스**를 추가 |
| 2 | m1 `button_padding.y`가 계획에 없다 — `vec2(12.0, 12.0)`을 쓰면 행 높이 토큰이 깨지는데 시험이 못 잡는다 | MINOR | 수용 — T1 Acceptance에 `button_padding.y == 0.0` 단언과 그 이유 추가 |
| 2 | m2 T4 예외 `panel.rs`의 사유 문면이 사실과 다르다(그 파일에는 `menu_row` 호출이 없다) | MINOR | 수용 — 「프레임만 열고 행은 `remote_menu`가 그린다」로 정정 |
| 2 | m3 매처 `menu_style(`가 egui 자신의 함수도 잡는다 | MINOR | 수용 — `egui::` 접두 제외를 계수 규칙에 포함 |

**종결 방식**: 재호출 상한(2회)을 **수렴이 아니라 예산 소진으로** 끝냈다 — 동일 지적 잔존은 0이고, 마지막 라운드의 신규 지적 4건은 위와 같이 메인이 근거를 직접 대조해(문자열 포함 관계·`menu.rs:117` 주석·`widgets.rs:348-352,716` 실물 확인) 전건 수용·반영했다. 기각한 지적은 없다.

## Phase Ledger

| Phase | 상태 |
|---|---|
| 구현 (T1~T4) | 대기 |
| Phase F (전체 검증) | 대기 |

## Progress Log

- T1-T2 완료 (커밋 a377934, 98fc68d): `theme.rs`에 메뉴 항목 토큰 3종 + `menu_style(ui)` 헬퍼를 만들고, egui 버튼으로 그리는 팝업 9곳이 그것을 거치게 했다. 861/861 통과.
  - 확인: 메뉴 안 좌우 여백의 현행 값은 4px이 아니라 **2px**이다(egui가 `Popup::menu`에서 자기 메뉴 스타일로 덮는다) — 계획의 P8이 이 사실을 담고 있고, 그래서 증가폭이 양쪽 20px다.
  - 결정: `menu::column_menu_items` 안에서는 스타일을 세우지 않는다 — 「팝업을 여는 쪽이 부른다」는 한 가지 규칙으로 통일(두 곳에서 세우면 어느 값이 먹는지 흐려진다).
  - 실측: 폭이 고정된 메뉴 4곳에서 여백 확대 후에도 라벨이 접히지 않는다(가장 빠듯한 한국어 `사이드바에서 숨기기`+`Del`이 여유 15.0px) → **폭 상수는 그대로 뒀다**.
  - 결정: `titlebar::SETTINGS_MENU_PADDING`을 「항목 여백 두 번 + `SETTINGS_MENU_BREATH`(20.0)」로 분해했다 — 합만 유지하면 라벨 여유가 0이 되어 2026-08-19의 접힘 회귀로 돌아간다.
- T3 완료 (커밋 예정): 직접 그리던 메뉴 한 줄 사본 3개(`remote_menu`·`tree`·`widgets`)를 `widgets::menu_row` 하나로 모으고, `tabs::show_site_row`는 흡수하지 않고 토큰만 공유했다(D6). 861/861 통과.
  - 실측: 드롭다운 여백이 8 → 12px로 늘어 긴 항목이 잘리는지 확인했다 — 이 PC의 한글 글꼴 73개 중 최장 이름이 `Microsoft JhengHei UI Light`(165.2px)이고 가용 폭은 216px(`FONT_FIELD_WIDTH` 240 − 12×2)라 **여유 50.8px**. 그래서 `FONT_FIELD_WIDTH`를 늘리지 않았다.
  - 결정: 공통 `menu_row`의 글꼴은 `TextStyle::Body.resolve`로 고른다 — 종전 세 사본은 13.0을 하드코딩했는데, egui가 버튼 라벨에 쓰는 것이 `Body`라 버튼 경로와 크기가 갈리지 않는다(D4의 「글자 크기 토큰을 만들지 않는다」와 정합).
