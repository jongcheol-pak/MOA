# Plan: 전송 큐 조작 확장·열 폭 조절·크기 표기 통일

**PRD**: `docs/prd.md`

## 요구 이해

- **원문 요청**: "실패 탭의 항목 컨텍스트 메뉴에 '다시 시도' 메뉴만 있는데 다음 메뉴도 추가. 전체 다시 시도 / 삭제 / 전체 삭제 … 성공 탭에서 항목의 컨텍스트 메뉴 삭제 / 전체 삭제 … 성공 탭에 목록이 없는데 탭 이름옆에 개수가 표시됨 … 목록 컬럼의 가로 사이즈를 조절할 수 있도록 수정. 사이즈 조절시 세로 조절 라인을 표시함 … 서버 로그는 내용을 마우스로 드래그 해서 복사할 수 있도록 수정 … 파일 크기 … 1.00KB 이렇게 둘째자리까지 표시 되도록 파일 크기 표시되는 곳은 모두 수정"
- **이해한 요구**: 하단 도크의 전송 큐가 쓰기 불편한 다섯 지점을 고친다 — ⓐ 행 우클릭으로 지금 보는 목록을 통째로 다시 걸거나 지울 수 있게 하고 ⓑ 연결별 탭(`전체`·서버별)의 건수가 지금 고른 필터를 따라가게 하며 ⓒ 큐 표의 열 폭을 끌어 조절할 수 있게 한다. 나아가 ⓓ 큐 표와 파일 목록 양쪽에 열 경계가 어디인지 보이는 세로 구분선을 두고 ⓔ 서버 로그를 마우스로 끌어 선택·복사할 수 있게 한다. 마지막으로 ⓕ 파일 크기를 보이는 모든 자리(파일 목록 자세히·격자, 전송 큐, 상태 표시줄)에서 `1.00 KB`처럼 소수점 둘째자리 + KB/MB/GB 자동 승격으로 통일한다.
- **포함하지 않는 것으로 이해**: 큐 표에 다중 행 선택(Ctrl·Shift)을 새로 만들지 않는다 — `삭제`의 대상은 우클릭한 그 행 하나다.

## Goal

전송 큐를 목록 단위로 다루고(다시 시도·삭제), 열 경계를 눈으로 찾아 폭을 조절하며, 로그를 끌어 복사하고, 파일 크기를 앱 전체에서 같은 규칙으로 읽는다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-4 | Must | T1·T5·T7 | ✅ 커버 (크기 표기 규칙·열 구분선) |
| FR-36 | Must | T2·T3·T4·T7 | ✅ 커버 (행 메뉴·탭 건수·열 폭) |
| FR-40 | Should | T6·T7 | ✅ 커버 (로그 텍스트 선택) |
| 그 밖의 active Must FR | Must | — | 이번 범위 외 (기구현) |

## Out of Scope

- 큐 표의 **다중 행 선택**(클릭·Ctrl·Shift 범위 선택과 선택 강조) — 사용자 결정 2026-08-18, `삭제`는 우클릭한 한 행만 다룬다.
- 큐 표의 **가로 스크롤** — 열 폭 합이 표 폭을 넘으면 오른쪽이 잘린다(D6). 파일 목록에는 있지만 큐에는 두지 않는다 — 넘치는 것은 사용자가 폭을 줄이면 되고, 스크롤을 넣으면 머리글 고정·본문 동기화가 함께 딸려 온다.
- 로그의 **가로 스크롤** — 사용자 결정 2026-08-18, 긴 줄은 지금처럼 `…`로 자른다.

## Deferred / Follow-up

- **egui의 끌기 판정을 시험에서 재현하는 방법** — `RawInput`에 `PointerMoved`+`PointerButton{pressed}`를 넣고 시간·프레임을 진행시켜도 `Response::dragged()`가 서지 않았다(2026-08-18 두 차례 실측). 그래서 열 폭 드래그의 **화면 쪽**(가이드 선이 서는가)은 큐 표·파일 목록 양쪽 다 수동 검증에 기대고 있다. 방법을 찾으면 `queue_panel`·`list_details` 두 곳에 함께 넣는다(폭 계산 규칙 자체는 `apply_drag` 단위 시험이 이미 고정한다).

- [SUGGEST] `TransferQueue::cancel(id)`와 `remove(ids)`가 같은 `retain` 한 줄을 각자 갖는다 — `cancel`을 `remove` 위에 재구성하면 한 곳으로 좁힐 수 있다. **이번엔 채택하지 않았다**: 중복이 2회라 공통화 문턱(3회) 미달이고, 위임으로 바꾸면 단건 경로에 `HashSet` 할당이 새로 붙는다(T3 quality S1).

- 큐 표 열의 **표시·숨김·순서 바꾸기** — 이번에 열 폭 상태(`QueueColumns`)가 생겨 확장 지점이 열린다. 파일 목록의 같은 항목(2026-07-29 등재)과 한 뿌리.

## Investigation Log

- 위키 참조: `20_projects/personal/moa/feat-dock-status` — 도크 탭·연결별 탭 줄·서버 로그의 정본. **위젯 id에 건수를 넣으면 클릭이 씹힌다**(누르는 사이 건수가 바뀌면 id가 달라진다)는 함정이 등재돼 있고, 현행 코드는 이미 `("dock_tab", filter as u8)`·`("queue_site", site.0)`로 값에 걸고 있다 — T2가 건수 산출을 바꿔도 id는 건드리지 않는다.
- 위키 참조: `20_projects/personal/moa/feat-file-list` — **egui는 겹친 위젯 중 나중에 등록한 것을 위로 본다**. 열 경계 드래그 핸들을 머리글 셀보다 나중에 등록해야 경계 클릭이 정렬로 새지 않는다(T4가 큐 표에 같은 순서를 적용한다).
- 위키 참조: `20_projects/personal/moa/feat-remote-transfer` — 큐 행 우클릭은 `재시도`·`취소` 둘, 크기 표기는 `1,840 KB` 꼴. **취소한 파일은 그 자리에서 지울 수 없다**(워커가 64KB 경계마다 취소를 살펴 아직 파일을 쥐고 있다) — `cancelling` 표에 넣었다가 워커의 완료 통지 뒤에 지운다. T3의 `삭제`가 같은 길을 탄다.
- 위키 참조: `20_projects/personal/moa/decisions` — 이번 요청과 상충하는 과거 보류·기각 결정 없음.
- Deferred 대장(`docs/plans/deferred.md`, 대기 64건(실측)): 관련 항목은 `[2026-07-29] 자세히 보기 열 추가·제거·순서 변경`(이번 열 폭 상태 도입으로 확장 지점이 하나 더 열린다 — 이번에도 미착수, 위 Deferred에 큐 판을 함께 적었다). 잔량 64건은 소진 batch 임계(100건)에 미달이며 최근 재확인이 2026-08-18이라 이번 회차에 batch를 열지 않는다.
- 크기 표기 함수는 **둘로 갈려 있다**(실측) — `panel/file_list.rs:773 format_size_kb`(KB 올림 + 천 단위, `0`→`0 KB`)와 `ui/queue_panel.rs:83 format_size`(KB/MB/GB 승격, `0`→`—`). 호출부는 각각 3곳·2곳(아래 4-A).
- 큐 표의 열 폭은 **상수 배열**이다 — `queue_panel.rs:39 COLUMNS: [f32; 7]`이고 `FLEX_COLUMN = 1`(로컬 파일)이 남는 자리를 갖는다(`column_widths` `:196-205`). 조절 상태·세션 저장은 없다.
- 파일 목록의 열 폭 조절은 **이미 있다** — `list_details.rs:589~608`이 경계마다 `HANDLE_WIDTH = 6.0` 핸들을 등록하고 hover·drag에 `CursorIcon::ResizeHorizontal`을 준다. **선을 그리는 코드는 없다**(`show_header`가 `rect_filled` + 열별 글자만 그린다) — 사용자가 "어디를 끌어야 할지 모르겠다"고 한 것이 이것이다.
- 로그 본문은 `painter().text()`·`painter().galley()`로 직접 그린다(`log_panel.rs:80~119`) — 위젯이 아니라 선택 자체가 성립하지 않는다.
- egui 0.35의 `Label::selectable(true)`는 `LabelSelectionState`를 거치고, `style.interaction.selectable_labels`·`multi_widget_text_select`가 **둘 다 기본 `true`**이며 이 앱은 그것을 끄지 않는다(`grep selectable_labels src/` → 0건).
- 여러 라벨에 걸친 선택의 복사 규칙(`label_text_selection.rs:288~332`): 앞 갤리의 아래가 새 갤리의 위보다 위면 `\n`을, 같은 줄이면 공백을 넣어 잇는다 — 한 줄의 세 열은 공백으로, 다음 줄은 개행으로 이어진다.
- 잘린(elided) 갤리라도 **전체를 선택하면 원문이 복사된다**(`selected_text`가 `everything_is_selected`면 `galley.text()`를 준다) — `elided_galley_*`는 `LayoutJob`에 원문을 넣고 `overflow_character`로 자르므로 `galley.text()`가 원문이다.
- `DockSession`(`app/settings.rs:239`)은 `panel`·`filter` 두 문자열뿐이고 둘 다 `#[serde(default)]`다 — 필드를 더해도 옛 파일이 그대로 읽힌다(`PanelSession.columns`가 같은 방식으로 추가된 선례, `settings.rs:390` 주석).
- 폴더 행의 크기 칸은 **호출부가 비운다** — `list_details.rs:488-490`·`list_grid.rs:359-361`이 `entry.is_dir()`이면 `String::new()`를 준다. 크기 함수를 바꿔도 폴더 표시는 영향받지 않는다(T1 Acceptance ④의 근거).
- 상태 표시줄의 크기 표기는 `TransferState::Active { sent }`만 쓴다(`status_bar.rs:103-111`) — 전송 직후 `sent == 0`이면 `—`가 나오는데 **이는 현행과 같고 T1이 바꾸지 않는다**(코어 함수가 아니라 `queue_panel::format_size`의 0 판정이 내는 값이며 그 판정은 그대로 둔다).
- 릴리즈 규약 없음 — `Cargo.toml` 버전 `0.1.0` 고정, AGENTS.md는 "배포: 단일 exe"만 적는다. 이번 회차에 버전·태그 대상 없음.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 여러 라벨에 걸친 드래그 선택이 egui 0.35에서 동작하고, 앱이 그것을 끄지 않았다 | `egui-0.35.0/src/style.rs:1482-1483`(둘 다 기본 true) + `grep -rn "selectable_labels\|multi_widget_text_select" src/` → 0건 | ✅ |
| 2 | 라벨 3개를 한 줄에 두어도 복사문이 `시각 종류 본문\n다음 줄`로 이어진다 | `egui-0.35.0/src/text_selection/label_text_selection.rs:288-332` (`copy_text`의 y 비교 분기) | ✅ |
| 3 | 잘린 갤리도 전체 선택 시 원문이 복사된다 | 같은 파일 `709-720` (`selected_text`의 `copy_everything`) + `list_common.rs:174-181`(원문을 `LayoutJob`에 넣는다) | ✅ |
| 4 | `DockSession`에 필드를 더해도 옛 세션 파일이 폴백하지 않는다 | `app/settings.rs:239-246`(기존 두 필드 모두 `#[serde(default)]`) + `settings.rs:383-390` 주석(스키마 버전을 올리면 통째로 폴백한다는 규칙) | ✅ |
| 5 | 큐 행의 `삭제`가 진행 중 항목에도 안전하다 — 기존 `취소` 경로가 워커 정지·`.part` 정리를 이미 처리한다 | `ui/app.rs:1753-1756`(`runner.cancel` → `queue.cancel`) + `remote/transfer.rs:323 pub fn cancel` | ✅ |
| 6 | 현행 큐 표는 `로컬 파일`(자리 1)이 잔여를 흡수한다 — **그래서 그 오른쪽 경계는 어떤 드래그 규칙으로도 커서를 따라올 수 없다**(잡은 경계 `x_k`가 흡수분에 상쇄돼 불변) | `ui/queue_panel.rs:196-205 column_widths` (`widths[FLEX] = (total - fixed).max(FLEX_MIN)`) — D6이 흡수 열을 마지막으로 옮기는 근거 | ✅ |
| 7 | 파일 목록 머리글에는 지금 세로 구분선이 없다 | `ui/list_details.rs:540-587`(`rect_filled` + 열별 galley만, `vline` 호출 없음) | ✅ |
| 8 | 도크 상단 탭(`전송 큐`·`성공`·`실패`)의 건수는 이미 필터별로 맞다 — 이번에 고칠 것은 아래 연결별 탭이다 | `ui/dock.rs:190-194`(`count(All/Done/Error)`) 대 `ui/queue_panel.rs:285·304`(`counts_by_site()`·`queue.len()` — 필터를 보지 않는다) | ✅ |

## 리뷰 이력

`plan-reviewer` 2라운드까지 돌렸고 재호출 상한(2회)을 **수렴이 아니라 예산 소진으로** 끝냈다 — **동일 지적 잔존은 0건**이고(1라운드 11건은 2라운드에서 실물 대조로 전건 해소 확인) 2라운드 신규 지적 10건은 메인이 실물에서 직접 대조해 처리했다. 아래가 그 판정이다.

| 라운드 | 지적 | 심각도 | 메인 판정 (근거) |
|---|---|---|---|
| 1 | B1 T4 acceptance 모순·드래그 상한 미정 | BLOCKER | 수용 — D6 재설계(흡수 열을 마지막 `상태`로) |
| 1 | M1 필터가 탭 멤버십을 지움 / M2 D6 근거 오류 / M3 `session.rs:781-785` 누락 / M4 i18n 문구 어긋남 | MAJOR ×4 | 전건 수용·반영 (2라운드에서 해소 확인) |
| 1 | m1~m6 | MINOR ×6 | 전건 수용·반영 (2라운드에서 해소 확인) |
| 2 | B1 가이드가 행 배경에 덮인다 | BLOCKER | **수용** — `show_queue`가 머리글(`:223`) → 행(`:252-261`) 순으로 그리고 행이 `ROW_HOT`·`HEADER_BG`로 배경을 채우는 것을 확인(`:409-413`). T4 Design ⑥·T5 Design ②를 "`ScrollArea` 이후에 긋는다"로 고쳤다 |
| 2 | M1 `상태` 하한 40px은 도달 불가 | MAJOR | **수용** — 마지막 열 오른쪽에 핸들이 없어 `상태` 저장 폭을 바꿀 경계가 없음을 확인. Acceptance ③④를 "합이 표 폭 이상이면 잘린다 / 하한은 핸들 있는 여섯 열"로 정정 |
| 2 | M2 기본 폭 합 1126 > 기본 창 1100 | MAJOR | **수용** — `src/main.rs:71`의 `[1100.0, 700.0]` 확인. `로컬 파일` 기본값을 320 → **280**(합 1086)으로 내려 slack이 남게 했다 |
| 2 | M3 4-D 기각 근거가 stale | MAJOR | **수용** — 폭 계산 규칙이 같아진 것이 맞다. 근거를 "인덱싱 축(`ColumnKind` 대 자리 번호)이 다르고 2회라 문턱 미달"로 교체(기각 결론은 유지) |
| 2 | m1 큐 머리글에 정렬이 없어 Risks 문구가 공허 | MINOR | **수용** — `queue_panel.rs:381-395`가 `painter().text()`뿐이고 `interact`가 없음을 확인. Risks·Acceptance ⑥을 "본문 행 우클릭"으로 좁혔다 |
| 2 | m2 `MIN_COL_WIDTH`는 신설이 아니라 기존 | MINOR | **수용** — `list_details.rs:27`에 `pub const`로 실재. 재사용으로 바꾸고 4-D에 행을 더했다. 핸들 폭은 private이라 자체 상수 |
| 2 | m3 모듈 주석 stale / m4 T5 가이드 순서 미정 / m5 승격 경계 반올림 / m6 라인 드리프트 | MINOR ×4 | 전건 수용·반영 |

**기각한 지적은 없다.** 3라운드를 열지 않은 이유는 상한 규정(2회)이며, 그 구간의 추가 수정이 새 결함을 만든다는 근거를 따랐다 — 2라운드 지적이 모두 **문면 수정으로 끝나고 구조 재설계를 요구하지 않는다**는 리뷰어 판단도 같다.

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 로그를 라벨로 바꾸면 가상 스크롤(`show_rows`) 밖의 줄은 위젯이 없어 드래그 선택 범위에 들지 않는다 | 화면 밖까지 한 번에 끌어 선택하는 것이 제한된다 | 가상 스크롤은 유지한다(2000줄 링 버퍼를 전부 위젯으로 만들면 프레임이 무너진다). 전문 복사는 기존 `⧉` 버튼이 그대로 준다 — T6 Acceptance에 이 한계를 명시하고 README에도 적는다 |
| 라벨 배치로 바꾸면서 로그 줄 높이(17px)·열 폭(62/44px)이 미세하게 달라질 수 있다 | 「로그 치수는 원본과 같다」 시험과 화면이 어긋난다 | `add_sized`로 폭·높이를 상수 그대로 주고, 기존 치수 시험을 그대로 통과시킨다 |
| 큐 표에 드래그 핸들을 얹으면 본문 행의 우클릭 메뉴가 핸들에 먹힐 수 있다 | 행 메뉴가 안 열린다 | 핸들은 **머리글 rect 안에서만** 등록한다 — 큐 머리글은 위젯이 아니라 `painter().text()`뿐이라(`queue_panel.rs:381-395`, 정렬 클릭 없음) 핸들이 가로챌 머리글 위젯 자체가 없고, 본문 행 rect와도 겹치지 않는다 |
| 흡수 열을 `로컬 파일`에서 `상태`로 옮기면(D6) 창이 넓을 때 `상태` 열이 과하게 늘어 보인다 | 디자인 원본의 `1fr` 배치와 화면 인상이 달라진다 | `상태` 열은 실패 사유(서버 응답 원문)를 담아 넓어져도 쓸모가 있다. `로컬 파일` 기본 폭을 320px로 두어 도입 직후 화면 차이를 줄이고, 사용자가 원하면 직접 넓힐 수 있다(그것이 이번 요청이다) |
| 크기 표기를 바꾸면 목록 `크기` 열의 기본 폭(90px)에 `1,205.63 KB` 같은 긴 문자열이 안 들어갈 수 있다 | 크기가 `…`로 잘려 읽히지 않는다 | 단위 자동 승격을 채택했으므로 자릿수가 `1.18 MB` 수준으로 줄어든다(KB 고정 안을 고르지 않은 이유). 기본 폭은 건드리지 않는다 |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `panel::file_list::format_size_kb` | `src/panel/file_list.rs:276·773·790`(정의+Win32 판 호출+`format_filetime` 문서주석의 참조), `src/ui/list_details.rs:7·491`, `src/ui/list_grid.rs:7·362` | 제거 → `format_size`로 교체. 호출 3곳 + `use` 2곳 + 주석 1곳 + 시험 5줄(`file_list.rs:876-880`) |
| `ui::queue_panel::format_size` | `src/ui/queue_panel.rs:83·472`, `src/ui/status_bar.rs:16·108` | 본문 교체(코어 함수 위임). 시그니처 불변 |
| `ui::queue_panel::group_digits` | `src/ui/queue_panel.rs:100`(정의), `:645`(시험) | 정수 자릿수 구분용이었으나 새 규칙에는 쓰이지 않는다 → 정의·시험 함께 제거 |
| `remote::queue::TransferQueue::counts_by_site` | `src/remote/queue.rs:320·510·632`(정의+시험 2), `src/ui/queue_panel.rs:285` | 시그니처에 `filter: QueueFilter` 추가 |
| `ui::queue_panel::show_site_tabs` | `src/ui/queue_panel.rs:266·216·764`(정의+큐 호출+시험), `src/ui/app.rs:1182`(로그 호출) | 시그니처에 "건수를 보일지" 인자 추가 |
| `ui::queue_panel::QueueAction` | `src/ui/queue_panel.rs:64·214·404·512·516`, `src/ui/app.rs:38·1144·1747-1757` | variant 3개 추가(`RetryAll`·`Remove`·`RemoveAll`), `apply_queue_action` 확장 |
| `app::settings::DockSession` | `src/app/settings.rs:58·239·588·807·840`, `src/ui/dock.rs:93·110`, `src/ui/session.rs:9·68·281·501` | 필드 `columns: Vec<f32>` 추가(`#[serde(default)]`) |
| `ui::dock::DockState` | `src/ui/dock.rs:57·93·110`, `src/ui/app.rs`(도크 배선), **`src/ui/session.rs:781-785`(시험이 세 필드를 전개 리터럴로 짓는다 — `..default()`가 없어 필드를 더하면 컴파일이 깨진다)** | 큐 열 폭 상태를 이 구조에 담는다(도크 화면 상태의 정본). `status_bar.rs:451`·`queue_panel.rs:721`·`dock.rs:455`는 `..DockState::default()`라 영향 없음 |
| `ui::log_panel::show_line` | `src/ui/log_panel.rs:80` | painter 직접 그리기 → 라벨 배치로 교체 |
| `ui::list_details::show_header` | `src/ui/list_details.rs:521`, `DetailsOutcome` `:262-274` | 구분선 그리기 추가, 드래그 중 가이드용 출력 추가 |

### 4-B. 계약·직렬화 변경

- `DockSession`에 `columns: Vec<f32>` 추가 — `#[serde(default)]`라 옛 파일은 빈 벡터로 읽히고 기본 폭이 된다. **스키마 버전(v3)은 올리지 않는다** — 올리면 `parse_session`이 통째로 폴백해 워크스페이스·탭까지 초기화된다(`settings.rs:503`).
- `counts_by_site`·`show_site_tabs`의 시그니처 변경은 crate 내부 전용(외부 API 없음).

### 4-C. 테스트 파일

- `src/panel/file_list.rs` `mod tests` — `크기_표시` 계열 5줄(`:876-880`)
- `src/ui/queue_panel.rs` `mod tests` — `크기와_속도_표기가_원본_꼴이다`(`:637-644`), `표_치수는_원본과_같다`(열 폭 상수), `남는_자리는_로컬_파일_열이_갖는다`(`column_widths`), `머리글_문구는_인벤토리_원문_그대로다`
- `src/ui/dock.rs` `mod tests` — `도크_스트립은_큐와_로그_탭을_보인다`
- `src/ui/log_panel.rs` `mod tests` — `로그_치수는_원본과_같다`
- `src/remote/queue.rs` `mod tests` — `counts_by_site` 시험 2곳(`:510·632`)
- `src/app/settings.rs` `mod tests` — 세션 왕복(`:807·840`)
- `src/ui/session.rs` `mod tests` — 옛 세션 폴백(`:383-404`)
- `tests/` 아래 통합 시험 중 위 심볼을 부르는 것: 없음(`grep -rn "format_size\|counts_by_site\|QueueAction" tests/` → 0건)

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| 크기 표기 코어 함수(`panel::file_list::format_size`) | `format_size_kb`(같은 파일)·`queue_panel::format_size`가 이미 두 벌 — 규칙이 하나로 합쳐지므로 **둘을 하나로 접는다** | 신규가 아니라 통합. 이름은 `format_size`로 하고 `format_size_kb`를 없앤다 |
| `queue_panel::QueueColumns`(큐 열 폭 상태) | `list_details::Columns`가 같은 성질(폭 배열·드래그·세션 왕복) | **재사용하지 않는다** — 폭 계산 규칙은 D6 재설계로 **같아졌지만**(고정 폭 배열 + 마지막 열 slack 흡수 + 하한 클램프 + `from_saved`/`to_saved`), 인덱싱 축이 다르다: `Columns`는 `ColumnKind`(여섯 열거값)로 자리를 찾고 보이는 열이 패널·원격 여부에 따라 달라지는 반면, 큐는 일곱 열이 늘 고정 순서다. 공통화 문턱(3회)에도 미달이고(2회), `queue_panel.rs` 모듈 주석이 이미 "일반 표 부품으로 만들지 않는다"고 선언해 두었다 |
| 열 구분선·드래그 가이드 그리기 | `egui::Painter::vline` 한 줄이면 된다(`dock.rs:227`이 탭 구분선에 이미 쓴다) | 재사용 — 헬퍼를 새로 만들지 않는다 |
| 큐 열 하한 상수 | `list_details::MIN_COL_WIDTH = 40.0`이 `pub`으로 있다(`list_details.rs:27`) | **재사용** — 같은 `ui` 계층이라 그대로 부른다. 핸들 폭(`HANDLE_WIDTH`)은 private이라 `queue_panel`에 같은 값(6.0)을 자체 상수로 둔다 |
| 로그 줄의 선택 가능 텍스트 | `egui::Label` + `selectable(true)`(프레임워크 기본 기능) | 재사용 — 직접 히트테스트·선택 상태를 만들지 않는다 |
| `TransferQueue`의 목록 단위 조작 | `clear_done`(끝난 것 치우기)·`cancel`(한 건 지우기)가 있다 | 확장 — `retain`으로 지우는 같은 방식을 따르되 대상 목록을 인자로 받는다 |
| i18n 키 3개(`queue_retry_all`·`queue_remove`·`queue_remove_all`) | `queue_retry`·`queue_cancel`이 `src/i18n/mod.rs:434-435`에 있다 | 같은 자리에 이어서 신규(카탈로그 규약) |

### Verified by

- `grep -rn "format_size_kb" src/ tests/` → 12 hits(실측 — `file_list.rs` 8·`list_details.rs` 2·`list_grid.rs` 2 = 정의 1·`use` 2·호출 3·문서주석 1·시험 5), 모두 위 표에 포함
- `grep -rn "format_size\b" src/ tests/`(정의 제외) → 7 hits, 모두 포함
- `grep -rn "counts_by_site\|show_site_tabs\|QueueAction\|DockSession" src/ tests/` → 30 hits, 모두 위 표에 포함
- `grep -rn "format_size\|counts_by_site\|QueueAction" tests/` → 0 hits (통합 시험 영향 없음)

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `README.md` 22·30·32·35행 | 열 폭 조절·크기 표기(`KB·MB·GB로 보입니다`)·로그 복사·세션 저장 항목을 문장으로 적고 있어, 고치지 않으면 문서가 실제와 어긋난다 | T7에 편입 |
| 필수 | `docs/prd.md` FR-4·FR-36·FR-40 + 결정 이력 | 사용자 결정 2026-08-18(함께 갱신) | T7에 편입 |
| 필수 | 각 모듈의 치수·문구 시험 | `format_size_kb` 제거·열 폭 상수 변경·머리글 구성 변경이 기존 단언을 깨뜨린다 | T1·T4·T5·T6에 각각 편입 |
| 무관 | `Cargo.toml` 버전·태그 | 릴리즈 규약이 없고 버전은 `0.1.0` 고정이다(Investigation Log) | 건드리지 않음 |
| 무관 | 위키(`20_projects/personal/moa/feat-*`) | 위키 갱신은 별도 세션의 일이며 `implement-task` F-6.5의 큐가 처리한다 | 이 plan에서 건드리지 않음 |

## Decisions

### D1. 크기 표기 규칙
- **Options**: A) KB/MB/GB 자동 승격 + 소수 둘째자리(전 화면 통일) / B) 자동 승격 + 둘째자리, 1KB 미만은 바이트 / C) 현행 단위 규칙 유지, 소수만 추가
- **Chosen**: A
- **Rationale**: 사용자 결정 2026-08-18. 단위가 하나로 읽히고, 파일 목록의 KB 고정을 유지하면 큰 파일이 `2,097,152.00 KB`가 되어 열 폭을 넘는다.
- **Source**: 사용자 답변(미리보기 채택) — `512 B→0.50 KB` / `1,024→1.00 KB` / `1,234→1.21 KB` / `1,234,567→1.18 MB` / `2GB→2.00 GB` / 폴더→빈 칸 / 크기 미상(0)→`—`.

### D2. 크기 함수의 배치와 0 처리
- **Options**: A) `panel/file_list.rs`에 코어 함수 하나를 두고 큐가 그것을 부른다 / B) `ui/queue_panel.rs`에 두고 목록이 부른다 / C) 새 모듈을 만든다
- **Chosen**: A
- **Rationale**: 의존은 단방향(`ui` → `panel`)이며 `panel`이 `ui`를 알아서는 안 된다(AGENTS.md Conventions). B는 그 방향을 뒤집는다. C는 함수 하나짜리 모듈이라 과하다.
- **Source**: AGENTS.md 「아키텍처」 + `panel/file_list.rs:771` 기존 주석(`pub`인 이유가 이미 "egui UI 계층과 표시 규칙 공유").
- **0 처리**: 코어 `format_size(bytes)`는 `0`을 `0.00 KB`로 준다(0바이트 파일은 실재한다). 큐는 `0`이 "크기를 모른다"는 뜻이라 `queue_panel::format_size`가 앞에서 `UNKNOWN`(`—`)으로 가른다 — 현행 동작 그대로.

### D3. 연결별 탭 건수의 기준
- **Options**: A) 지금 고른 필터를 따른다, 서버 로그 화면에서는 건수를 감춘다 / B) 필터를 따르되 로그 화면도 전체 건수를 보인다 / C) 현행 유지
- **Chosen**: A
- **Rationale**: 사용자 결정 2026-08-18. 성공 탭(0건)인데 아래 줄이 `전체 (1)`을 보이던 것이 요청의 발단이다. 로그 화면에는 셀 대상이 없다.
- **Source**: 사용자 답변 + `queue_panel.rs:285·304`(현행이 `counts_by_site()`·`queue.len()`로 필터를 보지 않음).

### D4. 목록 단위 조작의 "전체" 범위
- **Options**: A) 지금 보고 있는 목록(상단 필터 ∩ 연결별 탭) / B) 필터만 적용, 서버 무시 / C) 큐 전체
- **Chosen**: A
- **Rationale**: 사용자 결정 2026-08-18. 눈에 보이는 것과 지워지는 것이 일치한다.
- **Source**: 사용자 답변. 구현상 `queue_panel::visible_items(queue, state.filter, state.site)`가 이미 그 목록을 만든다 — 앱이 같은 함수로 대상을 다시 구한다.

### D5. 행 메뉴 구성
- **Options**: A) 상태별로 `전송 취소`(진행 중·대기) 또는 `삭제`(끝난 것) 한 쪽만 / B) `삭제` 하나로 통일 / C) 둘 다 나란히
- **Chosen**: A
- **Rationale**: 사용자 결정 2026-08-18. 두 조작의 동작이 같아(전송 중단 + 목록 제거 + `.part` 삭제) 나란히 두면 무엇을 눌러야 할지 헷갈린다.
- **Source**: 사용자 답변. 최종 구성 —
  | 행 상태 | 메뉴 |
  |---|---|
  | 진행 중·대기 | `전송 취소` · (보이는 목록에 실패가 있으면 `전체 다시 시도`) · `전체 삭제` |
  | 실패 | `다시 시도` · `전체 다시 시도` · `삭제` · `전체 삭제` |
  | 완료 | (보이는 목록에 실패가 있으면 `전체 다시 시도`) · `삭제` · `전체 삭제` |
  `전체 다시 시도`는 **보이는 목록에 실패 항목이 하나라도 있을 때만** 보인다(없으면 눌러도 아무 일이 없다). `전체 삭제`는 항상 보이며 진행 중인 것까지 함께 멈추고 지운다. 이 구성은 `전송 큐`·`성공`·`실패` 어느 탭에서나 같다(사용자 결정 — 탭이 아니라 행 상태가 메뉴를 정한다).

### D6. 큐 표 열 폭 조절 규칙
- **Options**: A) 파일 목록과 같은 규칙 — 일곱 열 모두 고정 폭을 갖고, 합이 표 폭보다 좁을 때만 **마지막 열(`상태`)**이 빈틈을 흡수 / B) 현행 `로컬 파일`(`1fr`) 흡수 구조를 유지하고 경계 드래그가 왼쪽 열의 폭을 바꾼다 / C) B에 더해 흡수 열의 오른쪽 경계만 예외로 오른쪽 열을 반대로 바꾼다
- **Chosen**: A
- **Rationale**: **B·C는 구조상 성립하지 않는다** — 경계 위치는 `x_k = Σ w[0..k]`이고 흡수 열이 자리 1(왼쪽 끝 다음)에 있어, 그 오른쪽의 어떤 열 폭을 바꿔도 흡수 열이 같은 양을 반대로 먹어 **잡은 경계가 제자리에 선다**(예: `크기|진행률` 경계에서 크기 열을 +d 하면 `로컬 파일`이 −d 되어 `x_5`는 불변이고 엉뚱하게 왼쪽 경계가 −d 움직인다). 손을 따라오는 경계는 흡수 열 양쪽 둘뿐이라 "열 폭을 끌어 조절한다"는 요구 자체가 성립하지 않는다. 흡수가 마지막 열에서 일어나야 잡은 경계와 그 오른쪽이 모두 손을 따라온다.
- **Source**: `queue_panel.rs:39·196-205`(`widths[FLEX] = (total - fixed).max(FLEX_MIN)` — 위 산식의 근거), `list_details.rs:130-136`(파일 목록이 같은 이유로 모든 열 고정으로 옮겨간 선례).
- **바뀌는 것(외부 관찰 가능)**: 창을 넓혔을 때 늘어나는 열이 `로컬 파일` → `상태`로 바뀐다. 디자인 원본의 `1fr` 배치와 달라지지만, 대신 사용자가 `로컬 파일` 폭을 원하는 만큼 직접 잡을 수 있다(그것이 이번 요청이다). 기본 폭은 `로컬 파일` **280px**로 둔다 — 일곱 열 합이 `34+280+300+120+84+118+150 = 1086px`이라 기본 창 폭 1100px(`src/main.rs:71`, 세션 복원이 없을 때)에서 **slack 14px이 남아 흡수가 실제로 작동**하고, 현행 화면의 `로컬 파일` 실효 폭(1100 창에서 294px)과도 거의 같다. 320px로 잡으면 합이 1126px이 되어 첫 실행부터 오른쪽이 잘리고 흡수가 한 번도 돌지 않는다(2라운드 리뷰 M2 — 실측으로 확인).
- **드래그 규칙**: 경계 k를 끌면 **그 왼쪽 열(k−1)**의 폭이 delta만큼 바뀐다(`list_details::Columns::apply_drag`와 같다). 마지막 열(`상태`)의 오른쪽 경계에는 핸들을 두지 않는다.
- **하한**: 핸들이 있는 여섯 열이 40px — `list_details::MIN_COL_WIDTH`(`list_details.rs:27`, 이미 `pub`이고 같은 `ui` 계층)를 그대로 쓴다(신설하지 않는다). **단 기본 폭이 그보다 좁은 열은 그 기본값이 하한이다** — `방향` 열은 34px이라 40px로 올리면 세션 왕복에서 화면이 넓어진다(2026-08-18 구현 중 시험이 잡았다 — 계획 시점에는 `COLUMNS[0] = 34.0 < 40.0`을 대조하지 않았다). 흡수는 "합 < 표 폭"일 때만 일어나고, 합이 표 폭을 넘으면 **오른쪽이 잘린다**(가로 스크롤은 Out of Scope) — 사용자가 폭을 줄이면 돌아온다.

### D7. 큐 열 폭의 저장 위치
- **Options**: A) `DockSession.columns: Vec<f32>` / B) 저장하지 않는다 / C) 새 최상위 세션 필드
- **Chosen**: A
- **Rationale**: 파일 목록 열 폭이 세션에 저장되는 관례(FR-4·FR-11, `PanelSession.columns`)를 따른다. 도크는 패널마다가 아니라 앱에 하나뿐이라 `DockSession`이 제자리다.
- **Source**: `app/settings.rs:239·390`, PRD FR-4·FR-11.

### D8. 구분선의 모양과 범위
- **Options**: A) 머리글 안에만 상시 구분선 + 드래그 중 목록 바닥까지 가이드 / B) 머리글·본문 전체에 상시 격자선 / C) 드래그 중에만 가이드
- **Chosen**: A
- **Rationale**: 사용자 결정 2026-08-18. 평소 화면은 조용하되 경계 위치는 늘 보인다.
- **Source**: 사용자 답변. 색은 `theme::BORDER_SUBTLE`(상시)·`theme::ACCENT`(드래그 중) — 앞의 것은 `queue_panel.rs:279`가 이미 연결별 탭 줄 아래 경계선에 쓰는 색이다. 파일 목록·큐 표 양쪽에 같은 규칙을 적용한다.

### D9. 로그 텍스트 선택의 구현 방식
- **Options**: A) 줄마다 `Label` 3개(시각·종류·본문)를 `add_sized`로 폭 고정 / B) 줄 전체를 `LayoutJob` 한 갤리로 만들어 `Label` 하나 / C) 읽기 전용 `TextEdit::multiline`
- **Chosen**: A
- **Rationale**: 열 폭이 픽셀로 고정(62/44px)이라 B는 고정폭 글꼴의 공백 개수로 자리를 맞춰야 해 정렬이 어긋난다. C는 링 버퍼 전체를 한 문자열로 만들어야 하고 종류별 색을 잃는다. A는 열마다 색을 그대로 두면서 egui의 다중 위젯 선택이 줄·열을 알아서 잇는다(전제 검증 #2).
- **Source**: `log_panel.rs:22-27`(열 폭 상수), 전제 검증 #1·#2·#3.
- **한계(수용)**: 가상 스크롤 밖의 줄은 위젯이 없어 선택 범위에 들지 않는다. 전문은 `⧉` 버튼이 준다.

## Tasks

- [x] T1. 크기 표기를 한 규칙으로 통일한다 (소수 둘째자리 · KB/MB/GB 자동)
  - **Type**: D
  - **Design**: ① `src/panel/file_list.rs`에 `pub fn format_size(bytes: u64) -> String`을 두고 기존 `format_size_kb`·`group_digits`를 없앤다(배치 근거는 D2). ② 책임 — 바이트를 `{값:.2} {단위}`로 만든다(단위는 KB/MB/GB, 1024로 나눠 올린다). ③ `ui::list_details`·`ui::list_grid`·`ui::queue_panel`이 이것을 부르고, `panel`은 아무것도 참조하지 않는다(단방향 유지). `ui::queue_panel::format_size`는 이름·시그니처를 그대로 두고 `0 → UNKNOWN` 판정만 남긴 뒤 본문을 코어에 위임한다 — `ui::status_bar`가 그 이름으로 부르고 있어 호출부를 건드리지 않는다. ④ 비추상화 선언 — 단위 목록을 표·트레이트로 만들지 않는다(세 단계뿐이고 `format_speed`와 합치지도 않는다. 속도는 `B/s`부터 시작해 하한 규칙이 다르다).
  - **Acceptance**: ① Given 1,024바이트 파일, When 자세히 보기·격자 보기·전송 큐 어디서든 크기를 볼 때, Then `1.00 KB`로 같게 보인다 ② `format_size(1)=="0.01 KB"`(Edge Case ⓐ — 0이 아닌 값은 최소 한 칸) · `format_size(512)=="0.50 KB"` · `format_size(1_234)=="1.21 KB"` · `format_size(1_234_567)=="1.18 MB"` · `format_size(2*1024*1024*1024)=="2.00 GB"` · `format_size(0)=="0.00 KB"` ③ `queue_panel::format_size(0)=="—"`(크기 미상은 그대로) ④ 폴더 행의 크기 칸은 여전히 빈 칸이다 ⑤ `grep -rn "format_size_kb\|group_digits" src/` → 0건 ⑥ `cargo test` 전건 통과, `cargo clippy --all-targets -- -D warnings` 경고 0
  - **Files**:
    - 주: `src/panel/file_list.rs`(정의 교체 `:770-788`, Win32 판 호출 `:276`), `src/ui/queue_panel.rs`(`format_size` 본문 `:76-108`)
    - 동반: `src/ui/list_details.rs`(`:7`·`:491`), `src/ui/list_grid.rs`(`:7`·`:362`), `src/ui/status_bar.rs`(호출은 불변 — 표기만 바뀐다, 확인용)
    - 테스트: `src/panel/file_list.rs` `mod tests`(`:876-880` 교체), `src/ui/queue_panel.rs` `mod tests`(`:637-644` 교체)
  - **Edge Cases**:
    - `0` — 목록은 `0.00 KB`, 큐는 `—`(두 자리가 다르다는 것이 이 task의 핵심 분기)
    - `u64::MAX` — GB로 나눠도 `{:.2}`가 지수 표기로 새지 않는지 확인(f64 변환 후 `17179869184.00 GB` 꼴)
    - **단위 승격 경계의 반올림** — 1,048,570바이트는 1MB 미만이라 KB 갈래로 가는데 반올림하면 `1024.00 KB`가 되어 어색하다. 판정을 **반올림한 값 기준**으로 한다 — 그 단위의 값이 반올림 후 `1024.00` 이상이면 한 단계 올린다(`1,048,570 → 1.00 MB`). GB 위로는 올릴 단위가 없어 그대로 둔다
    - 1023바이트 — `1.00 KB`로 반올림된다(0.999… → 소수 둘째자리 반올림). 0이 아닌데 `0.00 KB`로 보이는 값은 없어야 한다(1바이트 = `0.00 KB`가 되므로 **1KB 미만은 소수 둘째자리로 잘려 0이 될 수 있다** — 이 경우 `0.01 KB`로 올릴지 `0.00 KB`로 둘지는 ⓐ로 정한다)
    - ⓐ **1바이트~5바이트 처리**: `0.00 KB`로 두면 "빈 파일"과 구분되지 않는다 → **0이 아닌 값은 최소 `0.01 KB`로 올린다**(`bytes > 0`이면 계산값과 0.01 중 큰 쪽)
  - **Halt Forecast**:
    - (i) 크기 열 기본 폭(90px)에 새 문자열이 안 들어감 → 자동 승격이라 자릿수가 늘지 않는다(Risks 표), 폭은 건드리지 않는다
    - (ii-a) `panel::file_list`의 공개 함수 제거·신설(계획된 시그니처 변경) → `## 사전 승인 항목`
  - **Depends on**: -

- [x] T2. 연결별 탭 건수가 지금 필터를 따른다
  - **Type**: C
  - **Design**: ① `remote::queue::TransferQueue::counts_by_site`에 `filter: QueueFilter` 인자를 더해 그 필터에 걸리는 것만 센다(같은 파일 안의 `filter`/`count`와 같은 판정식). ② **탭 멤버십과 라벨 건수를 서로 다른 집계로 구한다** — 지금은 `queue_panel.rs:285-301`이 `counts` 하나로 둘을 겸하는데(`counts.contains_key(id) || connected.contains(id)`), 거기에 필터를 먹이면 `성공` 탭에서 Done 0건인 사이트가 **탭에서 통째로 사라진다**. 멤버십은 `counts_by_site(QueueFilter::All)`로, 라벨 건수는 `counts_by_site(state.filter)`로 따로 구한다(두 호출). ③ `ui::queue_panel::show_site_tabs`에 `show_counts: bool`을 더해 로그 화면에서는 라벨에 `(N)`을 붙이지 않는다. ④ `전체` 탭의 수도 `queue.len()`이 아니라 `queue.count(filter)`로 바꾼다. ⑤ 비추상화 선언 — "탭 라벨 만들기"를 별도 타입으로 뽑지 않는다(포맷 두 줄이다).
  - **Acceptance**: ① Given 큐에 LG의 실패 1건뿐이고 **LG에 연결돼 있지 않음**, When `성공` 탭을 볼 때, Then 아래 줄이 `전체 (0)`·`LG (0)`으로 선다(LG 탭이 사라지지 않는다 — Design ②) ② Given 같은 상태, When `실패` 탭을 볼 때, Then `전체 (1)`·`LG (1)` ③ Given 같은 상태, When `서버 로그` 탭을 볼 때, Then 아래 줄이 `전체`·`LG`로 **건수 없이** 선다 ④ 탭을 누르는 사이 건수가 바뀌어도 클릭이 씹히지 않는다(위젯 id는 여전히 `SiteId`로 잡는다 — 위키 함정) ⑤ `cargo test` 전건 통과
  - **Files**:
    - 주: `src/remote/queue.rs`(`:320`), `src/ui/queue_panel.rs`(`:266-285`·`:304`·`:216`)
    - 동반: `src/ui/app.rs`(`:1182` 로그 호출에 `show_counts: false`)
    - 테스트: `src/remote/queue.rs` `mod tests`(`:510`·`:632`), `src/ui/queue_panel.rs` `mod tests`(`:764` 호출), 신설 — 필터별 건수가 갈리는 것을 세는 시험 1건
  - **Edge Cases**:
    - 저장소에 없는 사이트(지운 사이트의 잔여 전송)도 필터 적용 대상이다 — 현행 `extra` 수집 경로가 그대로 살아야 한다
    - 필터에 걸리는 항목이 0건인 사이트도 **탭 자체는 남는다** — 연결이 열려 있는 경우(2026-08-05 사용자 보고)뿐 아니라 **큐에 그 사이트 항목이 있기만 하면** 남아야 한다(Design ②의 두 집계 분리가 지키는 것이 이것이다). 건수만 `(0)`이 된다
    - 큐가 비고 연결도 없으면 `전체 (0)` 하나만 선다
  - **Halt Forecast**:
    - (ii-a) `counts_by_site`·`show_site_tabs` 시그니처 변경(crate 내부) → `## 사전 승인 항목`
  - **Depends on**: -

- [x] T3. 큐 행 메뉴에 목록 단위 조작을 더한다
  - **Type**: D
  - **Design**: ① `ui::queue_panel::QueueAction`에 `RetryAll`·`Remove(TransferId)`·`RemoveAll` 세 variant를 더한다(`Retry`·`Cancel`은 그대로). ② 메뉴 구성은 D5 표대로 행 상태로 가른다 — `show_row`가 `has_error_in_view: bool`(보이는 목록에 실패가 있는가)를 인자로 받아 `전체 다시 시도` 표시 여부를 정한다. ③ 대상 계산은 **앱이 한다** — `ui::app::apply_queue_action`이 `queue_panel::visible_items(&self.queue, self.dock.filter, self.dock.site)`로 지금 보이는 목록을 다시 구해 그 `TransferId`들에 적용한다(화면은 큐를 고치지 않는다는 모듈 규약 유지). ④ `remote::queue::TransferQueue`에 `retry(ids)`·`remove(ids)`를 더한다 — 각각 상태를 `Wait`로 돌리고, 목록에서 지운다(기존 `cancel`의 `retain`과 같은 방식). ⑤ 비추상화 선언 — "선택 집합" 타입을 만들지 않는다(`Vec<TransferId>`를 그대로 넘긴다).
  - **Acceptance**: ① Given `실패` 탭 + `LG` 탭에 실패 3건, When 한 행에서 `전체 다시 시도`, Then 그 3건이 모두 `대기 중`이 되고 다른 서버의 실패는 그대로다 ② Given 같은 상태, When `삭제`, Then 우클릭한 그 한 행만 사라진다 ③ Given 같은 상태, When `전체 삭제`, Then LG의 실패 3건이 사라지고 다른 서버·다른 상태의 항목은 남는다 ④ Given `전체` 연결 탭 + `전송 큐` 필터에 진행 중 1건이 섞여 있음, When `전체 삭제`, Then 진행 중이던 것도 워커가 멈추고 `.part`가 지워진다(기존 취소 경로) ⑤ 진행 중·대기 행의 메뉴에는 `삭제`가 없고 `전송 취소`가 있다. 완료 행에는 그 반대다 ⑥ 보이는 목록에 실패가 없으면 `전체 다시 시도`가 메뉴에 없다 ⑦ 새 문구 셋이 `src/i18n/mod.rs`에 한·영 모두 등록돼 소스 훑기 시험을 통과한다 ⑧ `cargo test` 전건 통과
  - **Files**:
    - 주: `src/ui/queue_panel.rs`(`QueueAction` `:62-70`, `show_row` 메뉴 `:506-519`, `show_queue`가 `has_error_in_view` 산출 `:208-261`), `src/ui/app.rs`(`apply_queue_action` `:1747-1757`), `src/remote/queue.rs`(`retry`·`remove` 신설)
    - 동반: `src/i18n/mod.rs`(`:434` 뒤에 `queue_retry_all` = `전체 다시 시도` / `Retry all` · `queue_remove` = `삭제` / `Remove` · `queue_remove_all` = `전체 삭제` / `Remove all` — **사용자 원문 문구 그대로**(`모두 …`로 바꾸지 않는다))
    - 테스트: `src/remote/queue.rs` `mod tests`(`retry`·`remove` 신설 시험 2건), `src/ui/queue_panel.rs` `mod tests`(메뉴 구성이 행 상태로 갈리는 것을 세는 시험 1건 — 판정 함수를 순수 함수로 뽑아 시험한다)
  - **Edge Cases**:
    - 진행 중 항목을 `전체 삭제`로 지울 때 — 워커가 64KB 경계마다 취소를 살펴 그 순간에도 파일을 쥐고 있다. 기존 `cancelling` 경로를 그대로 타야 하며 `runner.cancel`을 **건마다** 부른다(위키 함정)
    - 보이는 목록이 비었는데 우클릭할 행이 없다 — 빈 목록에서는 메뉴 자체가 안 열린다(행이 없으므로). `전체 삭제`를 행 밖에서 부를 길은 두지 않는다
    - `전체 다시 시도` 도중 다른 서버가 실패를 새로 만들어도 이번 조작 대상은 누른 시점의 목록이다(다음 프레임에 다시 그려진다)
    - 큐가 일시정지 상태에서 `전체 다시 시도` — 상태만 `대기 중`으로 돌고 실제 재시작은 `⏸`를 풀 때다(기존 `Retry`와 같다)
  - **Halt Forecast**:
    - (i) 대상 목록을 화면이 아니라 앱이 다시 구하는 것이 옳은가 → D4·Design ③에서 확정
    - (ii-a) `QueueAction` variant 추가·`TransferQueue` 공개 메서드 신설 → `## 사전 승인 항목`
  - **Depends on**: T2 (같은 `visible_items`·필터 경로를 건드린다)

- [x] T4. 큐 표의 열 폭을 끌어 조절한다 (구분선 포함)
  - **Type**: D
  - **Design**: ① `ui::queue_panel`에 `pub struct QueueColumns { widths: [f32; 7] }`를 두고 `COLUMNS`를 그 기본값으로 옮긴다(`로컬 파일` 자리는 `0.0`이 아니라 **320.0**으로 채운다 — D6). 책임 — 폭 배열 보관·드래그 반영(`apply_drag`)·세션 왕복(`to_saved`/`from_saved`)·표 폭에 맞춘 실효 폭(`effective`). ② `effective(total)`은 **합이 `total`보다 좁을 때만 마지막 열(`상태`)에 그 차이를 더한다**(파일 목록 `Columns::effective`와 같은 규칙). 넘칠 때는 저장 폭 그대로 돌려주고 오른쪽이 잘린다 — `FLEX_COLUMN`·`FLEX_MIN`과 지금의 `column_widths`는 사라진다. ③ 소유는 `ui::dock::DockState`가 갖는다(도크 화면 상태의 정본이며 큐·로그가 함께 쓰는 줄과 같은 자리). ④ `app::settings::DockSession`에 `#[serde(default)] pub columns: Vec<f32>`를 더하고 `DockState::to_session`/`from_session`이 왕복시킨다 — `from_saved`는 앞에서부터 있는 만큼만 받고 유한하지 않은 값은 그 자리만 기본값으로 되돌린다(`list_details::Columns::from_saved`와 같은 규칙). ⑤ 드래그는 경계 k에서 **왼쪽 열(k−1)**의 폭을 delta만큼 바꾼다(`apply_drag`). 핸들은 경계 여섯 곳(`x_1`~`x_6`)에 두고 **마지막 열의 오른쪽 끝에는 두지 않는다** — 그래서 `상태` 열의 **저장 폭(150)은 드래그로 바뀌지 않는다**(표시 폭만 slack으로 늘어난다). 하한 `MIN_COL_WIDTH`가 걸리는 것도 그 여섯 열이다. 핸들 폭은 `queue_panel`에 자체 상수(6.0)를 둔다(`list_details::HANDLE_WIDTH`는 private). ⑥ 구분선은 D8 — `show_header`가 열 경계마다 `theme::BORDER_SUBTLE` `vline`을 긋는다. **드래그 가이드는 `show_header`가 그리지 않는다** — 머리글이 본문 행보다 먼저 그려져(`show_queue`가 `:223` 머리글 → `:252-261` 행 순서) 행 배경(`ROW_HOT`·`HEADER_BG`, `:409-413`)이 같은 레이어에서 그 선을 덮는다. `show_header`는 드래그 중인 **경계의 x**(커서 x가 아니라 반영된 위치)를 반환만 하고, `show_queue`가 **`ScrollArea`가 끝난 뒤** `ui.painter().vline`으로 본문 바닥까지 `theme::ACCENT`를 긋는다. ⑦ 비추상화 선언 — `list_details::Columns`와 공통 타입·트레이트로 묶지 않는다(4-D 근거: `ColumnKind` 결합·열 종류가 다르고 3회 문턱 미달).
  - **Acceptance**: ① Given 큐 화면, When `크기`와 `진행률` 사이 경계를 오른쪽으로 d만큼 끌 때, Then `크기` 열이 d만큼 넓어지고 **그 경계와 오른쪽 경계들이 함께 d만큼 이동**하며 `상태` 열이 d만큼 줄어든다(합 < 표 폭인 동안) ② Given 같은 화면, When `로컬 파일`과 `원격 파일` 사이 경계를 끌 때, Then `로컬 파일` 폭이 그만큼 바뀐다 ③ 저장 폭 합이 표 폭보다 좁으면 `상태` 열이 그 차이를 표시 폭으로 채우고, **합이 표 폭 이상이 되는 순간부터 오른쪽이 잘린다** — 어느 쪽이든 가로 스크롤은 생기지 않는다 ④ 핸들이 있는 여섯 열(`상태` 제외)은 각자의 하한 아래로 줄지 않는다 — `방향`은 34px, 나머지는 40px(D6 「하한」). `상태`의 저장 폭(150)은 드래그 대상이 아니다 ⑤ 평소에도 머리글에 열 경계마다 세로 구분선이 보이고, 끄는 동안에는 그 경계가 **행 바닥까지 끊김 없이** 강조색으로 이어진다(행 배경이 덮지 않는다 — Design ⑥의 그리기 순서) ⑥ 핸들이 본문 행의 우클릭 메뉴를 가로채지 않는다(핸들은 머리글 rect 안에만 있고 본문과 겹치지 않는다) ⑦ Given 폭을 바꾼 뒤 앱을 껐다 켬, Then 그 폭이 그대로 돌아온다 ⑧ `columns` 필드가 없는 옛 `settings.json`을 읽어도 세션이 폴백하지 않고 기본 폭이 된다 ⑨ `cargo test` 전건 통과
  - **Files**:
    - 주: `src/ui/queue_panel.rs`(**모듈 주석 `:1-5`** — "열 폭은 디자인이 픽셀로 못 박아 두었으므로 `34px 1fr 300px …`"가 이번 변경으로 어긋난다, `COLUMNS`·`FLEX_COLUMN`·`FLEX_MIN` `:38-42`, `column_widths` `:196-205`, `show_header` `:381-395`, `show_queue` `:207-261`), `src/ui/dock.rs`(`DockState` `:57-70`, `to_session`/`from_session` `:92-127`), `src/app/settings.rs`(`DockSession` `:236-246`)
    - 동반: `src/ui/app.rs`(도크 배선 — `show_dock_body`가 `DockState`를 넘기는 경로 `:1173`·`:1182`), `src/ui/session.rs`(`:68`·`:281`·`:501` — `DockSession` 기본값 경로, **`:781-785` — `DockState` 전개 리터럴 시험**)
    - 테스트: `src/ui/queue_panel.rs` `mod tests`(`표_치수는_원본과_같다` 갱신, `남는_자리는_로컬_파일_열이_갖는다`는 **`남는_자리는_상태_열이_갖는다`로 대체** — 흡수 열이 바뀐다, + 드래그 규칙·하한 시험 신설), `src/app/settings.rs` `mod tests`(`:807`·`:840` 왕복), `src/ui/session.rs` `mod tests`(`:383-404` 옛 세션 폴백에 `columns` 부재 사례 추가, `:781-785` 전개 리터럴 갱신)
  - **Edge Cases**:
    - 저장 폭 합이 표 폭을 넘음(창이 좁거나 사용자가 넓게 잡음) — `effective`가 저장 폭을 그대로 주고 오른쪽 열이 잘린다. 표 밖으로 나간 경계의 핸들은 잡히지 않는다(클립 영역 밖)
    - 세션에 저장된 폭이 유한하지 않거나(NaN·inf) 개수가 모자람 — 그 자리만 기본값으로 되돌린다
    - 드래그 중 창 크기가 바뀜 — 다음 프레임의 `effective`가 새 폭으로 다시 계산한다. 저장 폭은 건드리지 않는다
    - `상태` 열이 흡수로 늘어난 상태에서 그 왼쪽 경계를 끔 — 늘어난 것은 표시 폭일 뿐이라 저장 폭은 `진행률` 열만 바뀐다(파일 목록과 같다)
  - **Halt Forecast**:
    - (i) 세션 스키마를 올려야 하는가 → 아니다(`#[serde(default)]`, 전제 검증 #4)
    - (i) 흡수 열을 옮기면 디자인 원본의 `1fr` 배치와 달라진다 → D6에서 근거와 함께 확정(승인 프롬프트에 명시)
    - (ii-a) `DockSession` 필드 추가·`DockState` 구조 변경·`show_header` 시그니처 변경·`FLEX_COLUMN`/`FLEX_MIN` 제거 → `## 사전 승인 항목`
  - **Depends on**: T2 (같은 `show_header`·`show_queue` 경로를 건드린다)

- [x] T5. 파일 목록 머리글에 열 구분선과 드래그 가이드를 둔다
  - **Type**: C
  - **Design**: ① `ui::list_details::show_header`가 열 경계마다 `theme::BORDER_SUBTLE` 세로선을 긋는다(머리글 높이 안에서만). ② 드래그 중인 경계의 x를 `DetailsOutcome.resize_guide_x`에 담고, **같은 파일의 `show` 안**(`list_details.rs:309-336`의 `show_viewport` 클로저 — 머리글과 행을 그리는 그 `Ui`)에서 목록 바닥까지 `theme::ACCENT` 선을 긋는다. **행 루프가 끝난 뒤에 긋는다** — 머리글(`:321-336`)이 행(`:348-`)보다 먼저 그려지므로 앞에서 그으면 행 배경이 같은 레이어에서 선을 덮는다(T4 Design ⑥과 같은 함정). `src/ui/file_list.rs`의 바깥 호출부는 건드리지 않는다(그쪽은 스크롤 좌표계 밖이다). ③ 신규 심볼은 `DetailsOutcome`의 필드 하나(`resize_guide_x: Option<f32>`)뿐이다 — 새 함수·타입을 만들지 않는다. ④ 비추상화 선언 — 큐 표(T4)와 선 그리기를 공통 헬퍼로 묶지 않는다(각각 `vline` 한 줄이고 좌표 계산이 다르다).
  - **Acceptance**: ① Given 자세히 보기, When 아무것도 하지 않을 때, Then 머리글의 열 경계마다 세로 구분선이 보인다 ② Given 같은 화면, When 경계를 끌 때, Then 그 경계가 목록 바닥까지 **끊김 없이** 강조색 선으로 이어지고(행 배경이 덮지 않는다 — Design ②의 그리기 순서) 손을 떼면 사라진다 ③ 구분선이 열 폭 조절 동작·정렬 클릭·열 메뉴를 바꾸지 않는다 ④ 마지막 열의 오른쪽 끝에는 선을 긋지 않는다(표 바깥 경계) ⑤ `cargo test` 전건 통과
  - **Files**:
    - 주: `src/ui/list_details.rs`(`show_header` `:521-608`, `DetailsOutcome` `:262-274`, `show` 호출부 `:296-340`)
    - 테스트: `src/ui/list_details.rs` `mod tests` — **머리글 구분선을 세는 시험 1건**(셰이프에서 세로선의 x·색을 모아 경계 수·마지막 열 제외·비드래그 시 강조선 부재를 단언). **드래그 중 가이드 x는 시험하지 않는다**(2026-08-18 구현 중 정정): egui의 끌기 판정을 `RawInput` 포인터 이벤트로 재현하려 두 번 시도했으나(press+move / 시간 진행 + 프레임 추가) `dragged()`가 서지 않았다 — 계획 시점에 그 재현 가능성을 확인하지 않은 것이 원인이다. 그 축은 plan `## Verification Strategy` 수동 검증 3번이 받는다(T4의 `apply_drag` 단위 시험이 폭 변경 규칙 자체는 이미 고정한다).
  - **Edge Cases**:
    - 열이 원격 전용까지 여섯이면 선도 다섯 개다(보이는 열 수 - 1)
    - 가로로 스크롤한 상태 — 선은 열 오프셋을 그대로 따르므로 콘텐츠와 함께 움직인다
    - 폭이 0 이하인 열은 건너뛴다(현행 `show_header`가 이미 `continue`한다)
  - **Halt Forecast**:
    - (ii-a) `DetailsOutcome` 필드 추가 → `## 사전 승인 항목`
  - **Depends on**: -

- [ ] T6. 서버 로그를 끌어 선택·복사할 수 있게 한다
  - **Type**: C
  - **Design**: ① `ui::log_panel::show_line`이 painter 직접 그리기 대신 라벨 셋을 놓는다 — 행 rect를 `allocate_exact_size`로 잡아 오류 배경을 먼저 칠하고, 그 안에 시각·종류·본문 순으로 `Label::new(RichText::new(…).font(FontId::monospace(FONT_PX)).color(…)).selectable(true).truncate()`를 놓는다. **자리는 절대 좌표로 잡는다**(`시각 10 · 종류 82 · 본문 136`) — 커서 배치는 두 방식 모두 x를 밀었다(2026-08-18 실측: `add_sized`는 위젯을 셀 가운데에 놓아 `10/82/136 → 41/104/363`, `allocate_ui_with_layout`은 요청 폭이 아니라 내용 크기만 차지해 `10/20/30`). 계획 당시에는 `add_sized` + `item_spacing.x = COLUMN_GAP`로 재현된다고 적었으나 그 전제가 틀렸다. ② 그 셀 배치를 `selectable_cell` private 함수로 둔다 — 세 열이 같은 일(자리 계산 + child `Ui` + 라벨)을 하므로 3회 문턱을 채운다(계획 당시에는 "신규 심볼 없음"으로 적었으나 ①의 수단이 바뀌며 반복이 생겼다). ③ `list_common::elided_galley_colored`는 이 함수에서 더 쓰지 않는다(다른 호출부는 그대로). ④ 비추상화 선언 — "선택 가능한 표 줄" 부품을 만들지 않는다(로그 한 곳뿐이다).
  - **Acceptance**: ① Given 로그에 여러 줄, When 마우스로 끌어 여러 줄을 지날 때, Then 지난 글자가 선택 강조되고 `Ctrl+C`로 복사된다 ② 복사문에서 한 줄의 세 열은 공백으로, 줄 사이는 개행으로 이어진다 ③ 종류별 색·오류 줄 배경·시각 62px/종류 44px/줄 높이 17px이 지금과 같다(치수 시험 통과) ④ 긴 줄은 여전히 `…`로 잘려 한 줄에 들어간다 ⑤ 새 줄이 오면 바닥에 붙고, 위로 올려 둔 상태에서는 따라가지 않는다(`stick_to_bottom` 유지) ⑥ 가상 스크롤(`show_rows`)이 그대로라 2000줄에서도 프레임이 유지된다 — **화면 밖 줄은 선택 범위에 들지 않는다**(수용된 한계, D9) ⑦ `cargo test` 전건 통과
  - **Files**:
    - 주: `src/ui/log_panel.rs`(`show_line` `:80-119`, `show_log` `:60-78`)
    - 테스트: `src/ui/log_panel.rs` `mod tests`(`로그_치수는_원본과_같다` 유지 확인 + 줄이 선택 가능한 위젯으로 놓이는 것을 세는 시험 1건)
  - **Edge Cases**:
    - 빈 로그 — 줄이 없어 아무것도 안 그린다(현행과 같다)
    - 아주 긴 한 줄(2000자) — `truncate`가 한 줄 안에 가두고 행 높이가 늘지 않는다
    - 로그가 갱신되는 도중 드래그 중 — 새 줄이 아래에 붙어도 위젯 id는 줄 인덱스가 아니라 egui의 자동 id라, 선택이 끊기면 그대로 끊긴다(선택 유실은 허용 — 데이터 손상이 아니다)
    - 마우스 커서가 텍스트 위에서 I-빔으로 바뀐다 — 선택 가능 라벨의 기본 동작이며 로그에서는 맞는 표시다
  - **Halt Forecast**:
    - (i) 라벨 배치로 치수가 어긋남 → `add_sized`로 상수를 그대로 주고 치수 시험이 지킨다
  - **Depends on**: -

- [ ] T7. 문서를 실제와 맞춘다
  - **Type**: A
  - **Acceptance**: ① `README.md` 22행(파일 목록)이 열 구분선과 새 크기 표기를, 30행(전송 큐)이 행 메뉴 넷·열 폭 조절·탭 건수 규칙·크기 표기를, 32행(서버 로그)이 드래그 선택과 그 한계를, 35행(세션 저장)이 큐 열 폭을 적는다 ② `docs/prd.md` FR-4에 크기 표기 규칙과 열 구분선, FR-36에 행 메뉴·열 폭 조절·연결별 탭 건수 규칙, FR-40에 텍스트 선택이 들어간다 ③ `docs/prd.md`의 `## 결정 이력`에 2026-08-18 줄이 선다(D1·D3~D9 요지) ④ 문서에 없는 기능을 적지 않는다 — T1~T6 산출물과 문장 단위로 역대조한 표를 완료 보고에 낸다 ⑤ 실제 IP·계정·비밀번호를 적지 않는다
  - **Files**:
    - 주: `README.md`, `docs/prd.md`
  - **Edge Cases**:
    - README 30행은 이미 한 문단이 길다 — 절을 새로 만들지 않고 그 문장 안에서 고친다(요청받지 않은 `##` 신설 금지)
    - PRD FR-36 문면에 이미 탭 구성이 적혀 있다 — 건수 규칙만 더하고 기존 서술을 다시 쓰지 않는다
  - **Halt Forecast**:
    - (i) PRD 문면 변경 승인 → 사용자 결정 2026-08-18(함께 갱신)으로 해소
  - **Depends on**: T1·T2·T3·T4·T5·T6

## 사전 승인 항목 (일괄 승인 대상)

- T1 — `panel::file_list::format_size_kb`·`group_digits` 제거와 `format_size` 신설 (계획된 공개 함수 시그니처 변경, crate 내부 전용)
- T2 — `TransferQueue::counts_by_site`에 필터 인자 추가, `queue_panel::show_site_tabs`에 건수 표시 인자 추가 (계획된 시그니처 변경)
- T3 — `QueueAction` variant 3개 추가, `TransferQueue::retry`·`remove` 신설, i18n 키 3개 추가 (계획된 공개 API 추가)
- T4 — `DockSession`에 `columns` 필드 추가(비파괴, `#[serde(default)]`), `DockState`에 열 폭 상태 추가, `queue_panel::show_header` 시그니처 변경, `QueueColumns` 신설, `FLEX_COLUMN`·`FLEX_MIN`·`column_widths` 제거와 **흡수 열을 `로컬 파일`에서 `상태`로 옮기는 것**(D6 — 창을 넓혔을 때 늘어나는 열이 바뀐다) (계획된 구조 변경)
- T5 — `DetailsOutcome`에 필드 추가 (계획된 구조 변경)
- 위 전부에 딸린 **로컬 작업 브랜치 커밋** (task 단위)

## 불가피한 Halt (위임 불가)

- `master` 병합·push·태그·릴리즈 — 구현·검증이 끝난 뒤 최종 보고에서 별도 승인
- 세션 스키마 버전(v3 → v4) 상향이 필요해지는 경우 — 이 plan은 필요 없다고 판정했으나(전제 검증 #4) 구현 중 뒤집히면 옛 세션이 통째로 버려지므로 그 지점에서 멈춘다
- plan에 없던 구조 결정 (예: `list_details::Columns`와 `QueueColumns`를 공통 타입으로 묶기 — 4-D에서 기각했으므로 뒤집으려면 승인)

## Verification Strategy

- 빌드: `cargo build`
- 린트: `cargo clippy --all-targets -- -D warnings` (경고 0)
- 서식: `cargo fmt --check`
- 단위·통합 테스트: `cargo test` (직전 회차 기준 815건 통과 — 이번 회차에 신설 시험 6~7건이 는다)
- 수동 검증 (사용자 확인 필요 — 빌드로는 못 잰다):
  1. 큐에 실패·성공·진행 중을 섞어 두고 각 탭에서 우클릭 메뉴 구성을 확인 (D5 표)
  2. `성공` 탭에서 아래 줄이 `전체 (0)`으로, `서버 로그`에서 건수 없이 서는지 확인
  3. 큐 표·파일 목록에서 열 경계를 끌어 구분선·가이드·폭 변화를 확인, 앱 재시작 후 폭 유지 확인
  4. 서버 로그를 여러 줄에 걸쳐 끌어 선택하고 `Ctrl+C`로 붙여넣기 확인
  5. 같은 파일의 크기가 자세히·격자·큐에서 같게 보이는지 확인

## Phase Ledger

## Retry Ledger

## Progress Log

- T1-T2 완료 (커밋 edfde56, 다음): 크기 표기를 `panel::file_list::format_size` 한 벌로 접고(소수 둘째자리 + KB/MB/GB 자동, 큐만 `0 → —`), 연결별 탭 건수가 상단 필터를 따르게 했다. 826건 통과.
  - 결정: `filter`·`count`·`counts_by_site` 세 곳의 같은 match를 `QueueFilter::matches`로 뽑았다(3회 문턱 도달 — plan 미명시였으나 두 리뷰어 모두 정당하다고 판정).
  - 함정: 연결별 탭은 **멤버십과 건수를 다른 집계로** 구해야 한다 — 하나로 겸하면 그 필터에 항목이 없는 서버가 탭에서 사라진다.
- T3-T4 완료 (커밋 77b9bde, 다음): 큐 행 메뉴를 넷으로 넓히고(행 상태가 메뉴를 정한다), 큐 표에 열 폭 드래그·구분선·세션 저장을 넣었다. 831건 통과.
  - 결정(계획 정정): `방향` 열 기본 폭 34px이 일반 하한 40px보다 좁아 **세션 왕복에서 34 → 40으로 넓어지는 것**을 신규 시험이 잡았다. `min_column_width(slot) = min(기본 폭, MIN_COL_WIDTH)`로 열별 하한을 두고 plan Acceptance ④·D6 「하한」을 정정했다.
  - 함정: 드래그 가이드 선을 `show_header`에서 그으면 **행 배경이 같은 레이어에서 덮는다**(머리글이 먼저 그려진다). x만 반환하고 `ScrollArea` 이후에 긋는다 — T5도 같은 순서다.
  - 관측: `remote::connection::tests::한_연결이_막혀도_다른_연결은_계속_처리된다`가 전체 실행에서 1회 간헐 실패 → 단독 통과. `deferred.md`에 등재된 시간 마감(2초) 의존 시험이며 이번 변경과 무관(그 파일은 diff에 없다).

## Next Steps

- 권장 다음 액션: T1부터 `pjc:implement-task` 실행

## Open Questions

- [x] Q1. 크기 표기의 단위 규칙 → **KB/MB/GB 자동 승격 + 소수 둘째자리로 전 화면 통일**(D1)
- [x] Q2. `전체 다시 시도`·`전체 삭제`의 "전체" 범위 → **지금 보고 있는 목록(필터 ∩ 연결별 탭)**(D4)
- [x] Q3. 개수 문제의 대상 → **아래 연결별 탭 줄**, 필터를 따르게 하고 로그 화면에서는 감춘다(D3)
- [x] Q4. 구분선 모양 → **머리글 상시 + 드래그 중 전체 높이 가이드**(D8)
- [x] Q5. `삭제`의 대상 → **우클릭한 항목 하나**, 다중 선택은 만들지 않는다(Out of Scope)
- [x] Q6. `전송 큐` 탭의 메뉴 → **탭이 아니라 행 상태가 메뉴를 정한다**(D5)
- [x] Q7. 로그 긴 줄 → **잘림 유지, 보이는 만큼 복사**(D9, 전체 선택 시에는 egui가 원문을 준다 — 전제 검증 #3)
- [x] Q8. PRD 갱신 → **함께 갱신**(T7)
- [x] Q9. 진행 중 행의 `전송 취소`와 `삭제` 중복 → **상태별로 한 쪽만 보인다**(D5)
