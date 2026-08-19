# Plan: 소품 리팩터 묶음 (Deferred 대장 소진)

## 요구 이해

- **원문 요청**: 남은 작업을 물어 F3(Deferred 대장 78건)을 고르고, 그중 **G3 소품 리팩터** 묶음을 착수 대상으로 골랐다. 이어 두 갈래를 정했다 — *"B1만 넣기"*(`apply_conflict_choice` 3상태화는 채택, `from_tabs` 인자 묶기는 제외), *"제외 확정 — 대장에서 종결"*(공통화 문턱 미달 3건은 기각 처리).
- **이해한 요구**: `docs/plans/deferred.md` 「대기」에 쌓인 **동작 변경 없는 정리 항목**을 한 회차로 소진한다. 기존 시험이 그대로 통과하는 것이 성립 조건이며, 화면·동작·저장 형식은 한 군데도 달라지지 않는다. 착수 전 조사에서 대장 항목 여덟 건은 **코드를 고칠 것이 없다**고 판정됐다 — 그중 **한 건만 서술의 전제가 코드와 다르고**(트리 메뉴 보정), 나머지 일곱은 **서술이 참이지만 고치지 않는 편이 나은 것**이다. 두 갈래를 구분해 종결한다.
- **포함하지 않는 것으로 이해**: 결함 수정(G1)·구조 부채(G2)·기능 확장(G4)은 이번 대상이 아니다. 사용자가 묶음 하나를 골랐고 나머지는 대장에 그대로 남는다.

## Goal

Deferred 대장의 소품 리팩터 **6건을 코드에 반영**하고, 나머지 **8건을 종결**로 옮겨 이 열네 주제가 「대기」에서 사라지게 한다. 종결 8건은 두 갈래다 — **사실 오류로 무효** 1건(대장 서술의 전제가 코드와 다르다)과 **판단상 기각** 7건(서술은 참이지만 고치지 않는 편이 낫다). 화면·동작·저장 형식은 한 군데도 달라지지 않는다.

## Out of Scope

- **`dialog::Shell.clicked`의 타입화** — 사용처 12곳에 제네릭을 도입해야 해 「소품」의 규모를 넘는다. 위키 `feat-dialog-shell`이 *"대화 종류별 타입이나 빌더는 두지 않았다(공통점이 프레임과 푸터뿐이다)"* 를 이미 설계 결정으로 기록했다. **대장에서는 「판단상 기각」으로 종결한다**(T7) — 대기에 두면 회차마다 같은 판정을 되풀이한다.
- **`PanelState::from_tabs` 인자 묶기** — 사용자가 이번 회차에서 뺐다(호출부가 `ui/app.rs:309` 한 곳뿐이라 지금은 이득이 없다).
- **`list_grid::show`의 `visible` 부수 출력 제거** — 렌더와 수집을 나누는 구조 변경이 필요해 동작 무변경 정리의 범위를 넘는다.
- **공통화 문턱(3회) 미달 3건의 공통화** — 사용자가 「제외 확정」을 골랐다. AGENTS.md·공통 지침의 *"실제 중복이 3회 이상 확인된 경우에만 공통화한다"* 를 지킨다.

## Deferred / Follow-up

- 트리 메뉴·원격 메뉴의 화면 밖 보정 크기를 **실측으로 바꾸기** — 조사 결과 원격 쪽(`remote_menu::menu_size()`)도 `FRAME_PAD = 8.0` 어림값이라 「원격을 본받는다」는 방향 자체가 성립하지 않는다. 둘 다 실측으로 가려면 프레임 테두리·여백을 egui 스타일에서 읽어야 하고 그것은 별도 회차다.
- `theme::OK_BAR`(개명 후)와 `theme::OK_BORDER`가 **값이 같다**(`0x2F6B4F`) — 역할이 달라 이번엔 합치지 않는다. 팔레트를 손볼 때 다시 볼 자리.

## Investigation Log

- **위키 참조**: `20_projects/personal/moa/feat-dialog-shell.md` — 셸의 공개 표면은 `show`·`show_fixed` 둘이며 **대화 종류별 타입이나 빌더는 의도적으로 두지 않았다**(공통점이 프레임과 푸터뿐). `Shell.clicked` 타입화 제외의 직접 근거다.
- **위키 참조**: `20_projects/personal/moa/decisions.md` — 리팩터 문턱·공통화에 관한 과거 결정 없음. 2026-08-15 항목이 `dialog.rs` 신설을 기록하고 있으나 이번 대상과 겹치지 않는다.
- **Deferred 대장 조회**: `## 대기` 78건 전건 제목 스캔 + 소품 리팩터 그룹 13건 전문 정독. 잔량 앵커(`▶ 현행 잔량`)는 이 대장에 없다 — 소진 batch 임계 판정은 잔량 축(78건 < 100건)으로만 적용해 **batch 미착수**로 본다.
- **전제 반증 (대장 → 이 plan)**: 대장의 2026-08-17 항목 *"트리 메뉴의 보정 크기가 어림값 — 원격 메뉴처럼 실측 상수로 둘지 검토"* 가 이 plan의 후보 13을 떠받쳤는데, `remote_menu.rs:203-204`의 주석 *"메뉴 테두리와 안쪽 여백을 **어림**한 값"* 이 그 전제를 반박한다. 후보 13을 코드 수정에서 뺀 근거다.
- **PRD 경량 확인**: `docs/prd.md`의 FR 제목을 훑어 이번 변경이 닿는 active FR이 없음을 확인했다 — 전부 동작 무변경 정리라 외부 관찰 가능한 계약이 바뀌지 않는다. `**PRD**:` 줄을 두지 않으며 Phase G는 발동하지 않는다.
- **`examples/gen_licenses.rs:39-66`**: `main`의 두 갈래가 `CrateEntry`를 각자 세우며 `name`·`version`·`spdx`·`authors`·`bundled` **다섯 필드**를 똑같이 채운다(대장은 넷이라 적었으나 `bundled: false`도 같다). 다른 것은 `text_indices`·`standard_text` 둘이다.
- **`src/fs/icons.rs`**: 셸 잠금(`shell_guard()`)을 잡는 자리가 **다섯 곳**이다. `icon_index`의 확장자 갈래(`:180`)와 `type_name`(`:281`)은 이미 `let (idx, type_name) = { let _guard = …; lookup_by_attributes(ext) };` 형태로 블록에 갇혀 있다. 갇히지 않은 것은 셋 — `icon_index`의 **경로별 조회 갈래**(`:163`, 대장이 놓친 자리)·`icon_index_for_path`(`:200`)·`shell_display_name`(`:237`)이며, 세 곳 모두 잠금이 캐시 삽입·문자열 변환까지 유지된다.
- **대장의 반대 근거 반증**: *"두 곳은 여러 지역 변수를 써서 블록으로 감싸면 오히려 길어진다"* 는 실측과 다르다. 세 곳 모두 `SHFILEINFOW`가 **이미 블록 밖에 선언**돼 있어 셸 호출식만 `let ok = { let _guard = shell_guard(); unsafe { … } };`로 감싸면 되고, 줄 수는 그대로다.
- **`src/fs/icons.rs:347-373` — 이 잠금이 실제로 잠그는 범위(T2의 이득을 정정한 근거)**: `ShellGuard`는 `#[cfg(test)]`에서만 전역 `Mutex`를 쥐고, **`#[cfg(not(test))]`에서는 빈 구조체**다(주석: *"실행 파일에서는 빈 구조체다 — UI 스레드 하나가 그리므로 겨룰 상대가 없어 잠글 이유도 없다"*). 따라서 **T2는 실행 파일의 동작·성능을 바꾸지 않는다** — 이득은 ⓐ 시험 빌드에서 전역 잠금을 쥔 구간이 줄어드는 것과 ⓑ 다섯 자리의 해제 시점이 같은 형태가 되는 것 둘이다. 같은 주석이 *"캐시 히트 앞에 두면 렌더 경로가 프레임마다 전역 잠금을 잡아 시험 스위트가 10분을 넘긴다(실측)"* 라고 적어, 이 잠금의 목적이 시험 직렬화임을 밝히고 있다.
- **`src/fs/drives.rs:88`**: 드라이브 조회 **워커가 자기 `IconCache::new()`를 만들어** `list_drives`에 넘긴다. 즉 `IconCache`는 UI 스레드 전유물이 아니라 **스레드마다 별개 인스턴스**다(`Arc`·`static`으로 공유하는 자리 없음). `icons.rs:356`의 주석 *"UI 스레드 하나가 그리므로"* 는 그 사이에 낡았다 — 결론(잠글 상대가 없다)은 여전히 서지만 이유가 다르다.
- **`src/ui/theme.rs:76` / `src/ui/queue_panel.rs:188,884`**: `PRIMARY_FILL`의 사용처는 프로덕션 1곳·시험 1곳뿐이다. 값 `0x2F6B4F`는 같은 파일의 `OK_BORDER`(`:61`)와 동일하다. 이 파일의 명명 관례는 「상태 접두(`OK_`/`WARN_`/`ERROR_`) + 역할 접미(`_DOT`/`_TEXT`/`_FILL`/`_BORDER`)」이며 `OK_DOT`·`OK_TEXT`·`OK_FILL`·`OK_BORDER` 넷이 이미 서 있다.
- **`src/ui/session.rs:383-388`**: 옛 세션 호환 시험이 직렬화된 JSON에 `.replace()`를 다섯 번 걸어 필드를 걷어낸다. 값 리터럴(`[200.0,60.0,120.0,90.0]`·`"tiles"`·`"large_icons"`)까지 문자열로 박혀 있어, 기본값이 바뀌면 **걷어내지 못한 채 통과할 수 있다**(직후의 `assert!`가 그 경우를 잡도록 이미 방어돼 있다).
- **`src/ui/tree.rs:419-428`**: `show_remote_node`가 `#[allow(clippy::too_many_arguments)]`를 달고 인자 9개(`self` 포함)를 받는다. 재귀 호출부는 두 곳(`:252`·`:469`)이다. 이미 있는 `RowCtx`(`:687`)는 `textures`·`ctx`·`himl` 셋을 든 렌더 컨텍스트이며 **로컬 경로도 함께 쓴다** — 그래서 `conn`·`cache`·`folder`를 여기에 얹으면 로컬 호출에 빈 필드가 생긴다(대장이 적은 그대로).
- **`src/ui/app.rs:3069-3089`**: `apply_conflict_choice(drop, conflicts, choice: Option<ConflictChoice>)`에서 `None`이 「아직 묻기 전」을 뜻하고, 그 경우에만 `conflicts.is_empty()`와 결합해 판정한다. 호출부는 프로덕션 2곳(`:1342` 확인 직후·`:1371` 대화 응답)과 시험 **7곳**(`:3666`·`:3668`·`:3677`·`:3681`·`:3690`·`:3720`·`:3723`)으로 모두 **9곳**이며 전부 `app.rs` 안이다.
- **`src/ui/list_common.rs:81`**: `ConflictChoice`는 `Overwrite`·`Skip` 둘뿐이며 `ui::app`과 `ui::remote_menu`가 함께 쓴다(어느 한쪽에 두면 두 모듈이 서로를 알게 되어 그 자리에 있다는 주석이 달려 있다).
- **`src/ui/address_bar.rs:144-161` — 대장 서술은 지금도 참이다(1차 판정 정정)**: `NAV_ICON_PX = 16.0`(`:17`)이고 `widgets::DEFAULT_ICON_PX = 16.0`(`widgets.rs:13`)이며 색도 양쪽 다 `theme::TEXT`다. 즉 활성 경로 `icon_button_styled(ui, icon, size, theme::CONTROL_HOT, theme::TEXT, NAV_ICON_PX)`는 `icon_button(ui, icon, size, theme::CONTROL_HOT)`과 **인자까지 완전히 같다** — 대장이 말한 「수동 재기술」이 이것이다. 처음에 *"이미 `icon_button_styled`를 쓰니 해소됐다"* 고 본 것은 오판이었다(대장의 제안 대상은 `styled`가 아니라 기본 래퍼 `icon_button`이었다). **그럼에도 고치지 않는 이유는 대장이 스스로 적은 반대 근거다** — 비활성 경로는 `TEXT_DIM`이라 `styled`가 필요하므로, 활성만 `icon_button`으로 바꾸면 두 경로가 서로 다른 함수를 쓰게 되어 **「활성·비활성의 글꼴 크기가 같다」는 제약이 코드에서 사라진다**(한쪽은 기본값, 한쪽은 `NAV_ICON_PX`. 지금 값이 같은 것은 우연이고 한쪽이 바뀌면 조용히 갈린다). → **판단상 기각**.
- **`src/ui/file_list.rs:367-427`, `:605-612`**: `render_rows`는 필드 6개짜리 `RenderOutcome` 구조체를 돌려주고 `show`는 `ListInteraction`을 돌려준다 — 대장 표제의 *"4-튜플"* 이라는 표현은 이제 맞지 않는다. 다만 대장 항목의 **실질은 `DetailsOutcome`(`list_details.rs:262`)·`GridOutcome`(`list_grid.rs:58`)의 공통화**이고 두 타입은 지금도 있다. **고치지 않는 이유는 그 사이 차이가 벌어졌기 때문이다** — 대장은 *"`sort_click` 하나만 다르다"* 고 적었으나 지금은 자세히 보기에만 있는 필드가 여럿이라 공통 타입으로 묶으면 격자 쪽에 빈 필드가 생긴다. → **판단상 기각**.
- **`src/ui/list_details.rs:465-480` / `src/ui/list_grid.rs:193-207` — 대장의 대비는 지금도 정확하다(1차 판정 정정)**: 자세히 보기는 **콘텐츠 아래에 남는 사각형**을 따로 잡아(`content_bottom`부터 `inner_rect.max`까지) 거기서만 반응을 받고, 격자는 **영역 전체를 잡은 뒤** `select_request.is_none() && action == None`이라는 **사후 게이트**로 항목 클릭과 가른다. 처음에 *"둘 다 `clear_selection` 플래그로 수렴했다"* 고 본 것은 **결과를 적는 방식**이 같다는 뜻이었지 대장이 지적한 **자리 잡는 기법**이 같다는 뜻이 아니었다. **고치지 않는 이유**: 두 기법의 차이는 레이아웃에서 온다(격자는 항목이 영역을 채워 「아래 빈 공간」이 생기지 않는다). 헬퍼로 뽑아도 그 분기가 헬퍼 안으로 옮겨갈 뿐이다. → **판단상 기각**.

### 전제 검증

| # | 이 plan이 참으로 삼는 것 | 확인 근거 | 판정 |
|---|---|---|---|
| 1 | 여섯 항목 전부 **동작을 바꾸지 않는다** | 각 대상의 호출부를 전수 Read해 입출력 계약이 보존됨을 확인(4-A 표). T3은 값이 같은 상수의 이름만, T4는 시험 내부만, T1·T2·T5는 지역 구조만 바뀐다. T6만 시그니처가 바뀌며 그 계약은 아래 #4 | ✅ 확인 |
| 2 | 종결 8건은 **코드를 고칠 것이 없다** — 단 사유가 두 갈래다 | `address_bar.rs:144`·`file_list.rs:605`·`list_details.rs:465`·`list_grid.rs:193`·`remote_menu.rs:194` 직접 Read. **사실 오류로 무효는 1건뿐**(트리 메뉴 보정 — 본받으라던 원격 쪽도 어림값이다). 나머지 7건은 서술이 참이고 **판단상 기각**이다(각 근거는 Investigation Log) | ✅ 확인(1차 판정을 리뷰 지적으로 정정) |
| 3 | `fs::icons`의 셸 잠금을 좁혀도 **캐시 일관성이 깨지지 않는다** | 잠금이 지키는 것은 `SHGetFileInfoW` 호출 자체이고, 그 뒤의 `HashMap::insert`는 `&mut self`가 배타 접근을 보장한다. **`IconCache`는 스레드마다 별개 인스턴스**이며(`drives.rs:88`의 워커가 자기 것을 만든다) `Arc`·`static`으로 공유하는 자리가 없다. 덧붙여 `ShellGuard`는 **비시험 빌드에서 빈 구조체**라(`icons.rs:356`) 실행 파일에는 애초 잠금이 없다 | ✅ 확인(근거를 리뷰 지적으로 교체 — 「UI 스레드 단독 소유」는 사실이 아니었다) |
| 4 | `apply_conflict_choice`의 3상태화가 **호출부 9곳으로 닫힌다** | `grep "apply_conflict_choice\|ConflictChoice"` 전수 20건 확인 — 프로덕션 2(`app.rs:1342`·`:1371`) · 시험 7(`:3666`·`:3668`·`:3677`·`:3681`·`:3690`·`:3720`·`:3723`) · 정의 1 · `remote_menu` 3(그쪽은 `ConflictChoice`만 쓰고 이 함수를 부르지 않는다). **전부 `app.rs` 한 파일 안이다** | ✅ 확인 |
| 5 | `show_remote_node`의 인자를 **7개**로 줄이면 `allow` 없이 clippy를 통과한다 | `too_many_arguments`의 기본 임계는 7이고 **`args > 7`에서 발화**한다(그래서 현행 9개가 `allow`를 달았다). 묶는 대상이 `conn`·`cache`·`folder` 셋이므로 9 − 3 + 1 = **7개**이며 그 값이 곧 상한이다. T5 acceptance가 `allow` 제거 후 clippy 0으로 실증 | ✅ 확인(빌드로 실증) |
| 6 | 이번 변경이 닿는 **active FR이 없다** | `docs/prd.md` FR 제목 훑기 — 화면 문구·동작·저장 형식이 그대로라 FR 문면과 어긋나는 자리가 없다 | ✅ 확인 |

## 동반 변경 판정

| 축 | 발견 | 구분 | 처리 |
|---|---|---|---|
| ① 서술 문서 | README·PRD·AGENTS.md에 이번 대상 심볼을 서술한 자리 없음(`PRIMARY_FILL`·`apply_conflict_choice` 등으로 grep 0건) | 무관 | 갱신 없음 |
| ② 규약 복제 | 없음 — 이번 대상 중 정본이 둘 이상인 지점 없음 | 무관 | — |
| ③ 검증 자산 | T3은 `queue_panel.rs:884` 시험이, T6은 `app.rs:3666~3723` 시험 7건이 대상 심볼을 직접 부른다 | **필수** | 각 task 안에서 함께 고친다(T3·T6의 Files·Acceptance에 포함) |
| ④ 버전·매니페스트 | 버전을 올리지 않는다(동작 무변경 정리) | 무관 | — |
| ⑤ 무효화되는 기능·설정 | 없음 — 저장 형식·설정 키·화면 문구가 그대로다 | 무관 | — |
| 대장 정합 | 반영 6건·종결 8건(무효 1 + 판단상 기각 7)이 「대기」에 그대로 남으면 다음 회차가 같은 판정을 되풀이한다 | **필수** | T7로 편입 |

## Impact Analysis

### 4-A. 심볼 추적

| 변경 대상 | 사용처 전수 | 확인 방법 |
|---|---|---|
| `theme::PRIMARY_FILL` | `queue_panel.rs:188`(프로덕션) · `queue_panel.rs:884`(시험) · `theme.rs:76`(정의) — **3곳** | `grep -rn PRIMARY_FILL` 3건 전건 Read |
| `apply_conflict_choice` | `app.rs:1342`·`:1371`(프로덕션 2) · `:3666`·`:3668`·`:3677`·`:3681`·`:3690`·`:3720`·`:3723`(시험 7) — **호출부 9곳** + `:3069`(정의) | `grep -rn "apply_conflict_choice\|ConflictChoice"` 20건 전건 문맥 확인 |
| `show_remote_node` | `tree.rs:252`·`:469`(재귀 호출) · `:420`(정의) — **3곳** | `grep -n "show_remote_node("` 3건 |
| `IconCache`의 셸 잠금 3자리 | 함수 밖으로 나가지 않는 지역 변경 — 호출부 영향 0 | `icons.rs:150~270` 정독 |
| `gen_licenses::main`의 두 갈래 | 같은 함수 안 — 호출부 없음(`main`) | `examples/gen_licenses.rs:25~70` 정독 |
| 세션 호환 시험의 `.replace` 체인 | `session.rs:383~388` 시험 함수 내부 — 프로덕션 영향 0 | `grep -n "\.replace(" src/ui/session.rs` 9건 중 해당 5건 확인 |

### 4-B. 계약·직렬화

- **직렬화 형식 변화 없음** — `settings.json` 스키마 v3와 그 필드가 한 글자도 바뀌지 않는다. T4는 그 스키마를 **읽는 시험**의 작성 방식만 바꾼다.
- **공개 API 변화**: `theme::PRIMARY_FILL` → `theme::OK_BAR`(크레이트 내부 상수 개명) · `apply_conflict_choice`의 셋째 인자 타입(파일 지역 자유 함수). 둘 다 crate 밖으로 나가지 않으며 누락은 컴파일 오류로 즉시 드러난다.

### 4-C. 영향 받는 테스트

| 시험 | 영향 | 처리 |
|---|---|---|
| `queue_panel.rs:884` `bar_color` 단언 | 상수 이름이 바뀐다 | T3에서 함께 고친다 |
| `app.rs:3660~3730` 충돌 판정 시험 7개 호출 | 셋째 인자 타입이 바뀐다 | T6에서 함께 고친다 |
| `session.rs:370~` 옛 세션 호환 시험 | 본문을 통째로 다시 쓴다 | T4 자체가 그 작업 |
| `icons.rs`의 `shell_queries` 계측 시험 | 잠금 스코프는 조회 횟수를 바꾸지 않는다 | 변경 없이 통과해야 한다(T2 acceptance) |
| 그 밖의 858건 | 무관 | `cargo test` 전건 통과가 각 task의 acceptance |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 | 재사용/신규 사유 |
|---|---|---|
| `theme::OK_BAR` | `OK_DOT`·`OK_TEXT`·`OK_FILL`·`OK_BORDER` 넷이 이미 있다. 값이 같은 `OK_BORDER`가 존재 | **개명이지 신규가 아니다.** `OK_BORDER`와 합치지 않는 이유는 역할이 다르기 때문(테두리 vs 막대 채움) — 합치면 한쪽 색을 조정할 때 다른 쪽이 함께 움직인다 |
| `ConflictDecision`(T6) | `ConflictChoice`(list_common.rs:81)가 「사용자가 고른 것」을 이미 든다. `DialogOutcome`(remote_menu)은 대화 진행 상태를 든다 | **신규** — 기존 둘 어느 쪽도 「아직 묻기 전」을 표현하지 않는다. `DialogOutcome`에 얹으면 대화를 모르는 `apply_conflict_choice`가 대화 타입에 묶인다 |
| `RemoteRowCtx`(T5) | `RowCtx`(tree.rs:687)가 렌더 자원 셋을 든다 | **신규** — `RowCtx`는 로컬 경로도 쓰므로 원격 전용 필드를 얹으면 로컬 호출에 빈 필드가 생긴다(대장이 지적한 그대로). 별도 구조체로 두고 `RowCtx`는 그대로 둔다 |

### 4-E. 동반 변경 판정

위 `## 동반 변경 판정` 표 참조 — **필수 2건**(검증 자산 · 대장 정합)은 T3·T6·T7에 편입했고, 선택 항목은 없다.

## Decisions

- **D1. 개명 후 이름은 `OK_BAR`** — `theme.rs`의 명명 관례가 「상태 접두 + 역할 접미」이고 `OK_DOT`이 이미 같은 상태의 다른 역할을 든다. `QUEUE_DONE_FILL` 같은 이름은 그 관례에서 혼자 벗어난다. **Source**: `src/ui/theme.rs:58-76`.
- **D2. `PRIMARY_FILL`을 `OK_BORDER`와 합치지 않는다** — 값이 같아도 역할이 다르다(막대 채움 vs 상태 배지 테두리). 합치면 한쪽을 조정할 때 다른 쪽이 끌려간다. **Source**: `queue_panel.rs:188`·`remote_states.rs`의 배지 사용.
- **D3. `ConflictDecision`은 평탄한 3변형** — 대장이 적은 `{ NotAsked, Overwrite, Skip }` 그대로다. `Chosen(ConflictChoice)`로 감싸면 중첩 매치가 생겨 지금보다 읽기 어려워진다. `ConflictChoice → ConflictDecision` 변환은 `From` 구현으로 둔다.
- **D4. `ConflictDecision`은 `ui/app.rs` 지역에 둔다** — 「아직 묻기 전」은 `ui::app`이 확인 대기 큐를 들고 있어서 생기는 개념이고 `ui::remote_menu`는 그것을 모른다. `list_common`에 두면 대화 쪽이 쓰지 않는 변형을 알게 된다. **Source**: `list_common.rs:74-80`의 같은 취지 주석.
- **D5. `RemoteRowCtx`는 `show_remote_node` 전용** — `conn`·`cache`·`folder` 셋을 들고 재귀 내내 불변이다. `RowCtx`에 얹지 않는 이유는 4-D 표에 적은 그대로다.
- **D6. `fs::icons`는 세 자리 모두 손본다** — 대장이 적은 둘에 `icon_index`의 경로별 조회 갈래를 더한다. 같은 성질의 자리를 하나만 남기면 다음 사람이 「왜 여기만 다르지」를 다시 조사한다.
- **D7. 세션 시험은 `serde_json::Value`에서 키를 제거한다** — 지금은 값 리터럴까지 문자열로 박혀 있어 기본 열 폭이 바뀌면 걷어내지 못한다. `Value`를 훑어 `columns`·`view_mode` 키를 지우면 값과 무관해진다. 직후의 `assert!`(걷어냈는지 확인)는 그대로 남긴다.
- **D8. 문턱 미달 3건은 「기각」으로 종결한다** — 사용자 결정. 대장 「대기」에 두면 회차마다 같은 판정을 되풀이한다.

## Tasks

- [x] **T1. `gen_licenses`의 `main` 갈래 중복을 없앤다** — Type C
  - **Acceptance**:
    - 두 갈래가 공유하는 다섯 필드(`name`·`version`·`spdx`·`authors`·`bundled`)를 한 번만 적고, 갈래는 `text_indices`·`standard_text` 둘만 정한다.
    - `cargo run --example gen_licenses`가 만든 `assets/licenses.json`이 **현재 파일과 바이트 단위로 같다** — **판정은 실행 전후 `Get-FileHash`(SHA256) 대조가 1차**이고 `git diff --stat`은 보조다(개행 정규화가 걸리면 바이트가 달라도 diff가 0줄로 나올 수 있어 그것만으로는 판정할 수 없다).
    - `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: 원문이 없는 크레이트(`standard_indices` 갈래)가 오류를 내면 종전과 같은 메시지(`{name} {version}: {err}`)로 종료해야 한다 — 공통 필드를 먼저 만들어도 `?`의 조기 반환 위치가 달라지지 않게 한다.
  - **Halt Forecast**: 없음 — 예제 타깃 한 파일이고 산출물 동일성이 즉시 판정된다.
  - **Files**: 주 — `examples/gen_licenses.rs`
- [x] **T2. `fs::icons`의 셸 잠금을 세 자리에서 좁힌다** — Type C
  - **이 task가 바꾸는 것과 바꾸지 않는 것**: `ShellGuard`는 **비시험 빌드에서 빈 구조체**라(`icons.rs:356`) **실행 파일의 동작·성능은 달라지지 않는다**. 이득은 ⓐ 시험 빌드에서 전역 `SHELL_LOCK`을 쥔 구간이 셸 호출 자체로 좁아지는 것과 ⓑ 다섯 자리의 해제 시점이 같은 형태가 되어 다음 사람이 「왜 여기만 다르지」를 다시 조사하지 않는 것이다.
  - **Acceptance**:
    - `icon_index`의 경로별 조회 갈래(`:163`)·`icon_index_for_path`(`:200`)·`shell_display_name`(`:237`) 셋 모두 `shell_guard()`가 **`SHGetFileInfoW` 호출식만 감싸는 블록** 안에 든다. 캐시 삽입·`wide_to_string` 변환은 잠금 밖에서 돈다.
    - 다섯 자리(위 셋 + 이미 블록인 `icon_index` 확장자 갈래·`type_name`)의 해제 시점이 같은 형태가 된다.
    - `unsafe` 블록에 딸린 안전성 주석이 새 구조에 맞게 남는다(블록 이동으로 주석이 호출과 떨어지지 않게).
    - **`icons.rs:356`의 낡은 주석 한 줄을 함께 고친다** — 지금 *"UI 스레드 하나가 그리므로 겨룰 상대가 없어"* 라고 적혀 있으나 `drives.rs:88`의 워커도 자기 `IconCache`를 만든다. 결론(잠글 상대가 없다)은 그대로이고 **이유를 「인스턴스가 스레드마다 별개라」로** 고친다. 이 잠금 구조를 손대는 회차가 이번뿐이라 여기서 처리한다.
    - `cargo test` 전건 통과 — 특히 `shell_queries` 계측 시험이 **같은 횟수**를 본다.
    - `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: `SHGetFileInfoW`가 0을 돌려주는 실패 경로에서 `info`를 읽지 않는 기존 규칙이 유지돼야 한다(`icon_index_for_path`는 `dir_icon`으로, `shell_display_name`은 `None`으로 떨어진다). 블록이 `ok`만 내보내므로 `info`는 밖에 그대로 남는다.
  - **Halt Forecast**: 없음 — 한 파일 지역 변경이고 `unsafe` 범위가 늘지 않는다(오히려 준다).
  - **Files**: 주 — `src/fs/icons.rs`
- [x] **T3. `theme::PRIMARY_FILL`을 `OK_BAR`로 개명한다** — Type C
  - **Acceptance**:
    - `theme.rs:76`의 상수 이름이 `OK_BAR`가 되고 **값은 그대로**(`0x2F, 0x6B, 0x4F`)다. doc 주석이 「전송 큐 완료 막대 채움」이라는 실제 용도를 적는다.
    - `queue_panel.rs:188`(프로덕션)·`:884`(시험) 둘 다 새 이름을 쓴다.
    - 옛 이름 `PRIMARY_FILL`로 레포 전체를 검색해 **잔존 0건**이다.
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: `OK_BORDER`와 값이 같아 실수로 합치기 쉽다 — D2대로 **합치지 않고** 두 상수를 각각 남긴다.
  - **Halt Forecast**: 없음 — 크레이트 내부 상수이고 누락은 컴파일 오류다.
  - **Files**: 주 — `src/ui/theme.rs`, `src/ui/queue_panel.rs`
- [x] **T4. 옛 세션 호환 시험을 `serde_json::Value` 방식으로 바꾼다** — Type C
  - **Acceptance**:
    - `session.rs:383~388`의 `.replace()` 다섯 줄이 사라지고, 직렬화 결과를 `serde_json::Value`로 파싱해 **패널 객체에서 `columns`·`view_mode` 키를 제거**하는 방식이 된다.
    - 제거가 실제로 됐는지 보는 기존 `assert!`(「테스트가 새 필드를 실제로 걷어내지 못했다」)가 남는다.
    - 시험이 지키던 계약(옛 세션이 `parse_session`에 거부되지 않고 `restore`가 같은 값을 돌려준다)이 그대로 판정된다.
    - **시험 본문에 열 폭 값·보기 모드 이름 리터럴이 하나도 남지 않는다**(`200.0`·`"tiles"`·`"large_icons"` 등으로 그 함수를 검색해 잔존 0). 값 의존이 사라졌다는 판정은 이 검색으로 하며, **기본값을 임시로 바꿔 보는 확인은 하지 않는다** — 그 상수는 이 task의 Files 밖에 있고 되돌림을 놓치면 동작이 바뀐다.
    - 주석의 *"앞으로 필드를 더할 때도 여기에 함께 추가한다"* 가 새 방식에 맞게 고쳐진다(키 목록에 더한다는 뜻으로).
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: 세션 JSON에서 패널 객체가 중첩 배열 안에 있으면 키 제거가 그 깊이까지 닿아야 한다 — 재귀로 훑거나 실제 구조를 따라 내려간다. 제거 대상이 하나도 없으면 위 `assert!`가 실패해 즉시 드러난다.
  - **Halt Forecast**: 없음 — 시험 함수 하나의 내부이며 프로덕션 코드에 닿지 않는다.
  - **Files**: 주 — `src/ui/session.rs`
- [x] **T5. `show_remote_node`의 `too_many_arguments`를 없앤다** — Type C
  - **Design**: ① 배치 — `src/ui/tree.rs`, 기존 `RowCtx` 곁. ② 신규 심볼과 책임 — `RemoteRowCtx<'a>`(원격 노드를 그리는 동안 불변인 `conn: ConnectionId`·`cache: &'a TreeCache`·`folder: i32`를 든다). ③ 의존 방향 — `ui::tree` 안에서만 쓰이며 밖으로 내보내지 않는다(`pub` 아님). ④ 비추상화 — `RowCtx`와 합치지 않고, 로컬 경로용 대응 구조체도 만들지 않는다(로컬은 인자가 상한 아래다).
  - **Acceptance**:
    - `#[allow(clippy::too_many_arguments)]`가 `show_remote_node`에서 **사라진다**.
    - 인자가 9개 → **7개**가 되고(`conn`·`cache`·`folder` 셋을 `RemoteRowCtx` 하나로 묶는다 — 9 − 3 + 1), 재귀 호출부 두 곳(`:252`·`:469`)이 새 시그니처를 쓴다. 7은 clippy 기본 임계와 같은 값이라 `args > 7` 판정에 걸리지 않는다.
    - `cargo clippy --all-targets -- -D warnings`가 **경고 0**으로 통과한다(`allow` 없이 상한을 지킨다는 실증).
    - `cargo test` 전건 통과 · `cargo fmt --check` 경고 0.
    - 원격 트리의 그리기 결과가 달라지지 않는다(`TreeOutcome`을 보는 기존 시험이 그대로 통과).
  - **Edge Cases**: 재귀 호출에서 `RemoteRowCtx`를 다시 만들지 않고 그대로 넘겨야 한다 — 매 깊이마다 새로 만들면 `cache` 참조 수명이 꼬인다. `folder`는 `Copy`(i32)라 값으로 든다.
  - **Halt Forecast**: 없음 — 한 파일 안이고 시그니처 누락은 컴파일 오류다. 파일을 지우거나 옮기지 않는다.
  - **Files**: 주 — `src/ui/tree.rs`
- [ ] **T6. `apply_conflict_choice`를 3상태 enum으로 바꾼다** — Type C
  - **Design**: ① 배치 — `src/ui/app.rs`, `apply_conflict_choice` 바로 위. ② 신규 심볼과 책임 — `ConflictDecision { NotAsked, Overwrite, Skip }`(같은 이름 확인이 **어느 단계에 있는지와 사용자가 무엇을 골랐는지**를 한 타입에 담는다) + `impl From<ConflictChoice> for ConflictDecision`. ③ 의존 방향 — `ui::app`만 안다. `ui::remote_menu`는 종전대로 `ConflictChoice`만 쓴다(D4). ④ 비추상화 — `DialogOutcome`과 합치지 않고, `Cancelled`를 이 enum에 넣지 않는다(취소는 호출부가 아예 부르지 않는다는 기존 계약을 그대로 지킨다 — `list_common.rs:74`의 주석과 같은 취지).
  - **Acceptance**:
    - `apply_conflict_choice`의 셋째 인자가 `ConflictDecision`이 되고, 본문 매치가 `NotAsked`·`Overwrite`·`Skip` 셋을 각각 다룬다. `Option`의 `Some`/`None` 중첩이 사라진다.
    - 프로덕션 호출부 둘이 각각 `ConflictDecision::NotAsked`(`:1342`)와 `choice.into()`(`:1371`)를 넘긴다.
    - 시험 7개 호출이 새 타입을 쓰고 **판정 결과가 종전과 같다**(덮어쓰기는 전부, 건너뛰기는 겹치는 것만 빼고, 아직 묻기 전 + 겹침이면 아무것도 보내지 않는다).
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: `NotAsked`에서 `conflicts`가 빈 경우만 통과시키는 기존 판정이 그대로여야 한다 — 이 결합을 없애는 것이 목적이 아니라 「아직 묻기 전」을 타입으로 드러내는 것이 목적이다. `Skip`에서 남는 것이 없으면 `None`을 돌려주는 반환 계약(`Option<DropOutcome>`)도 그대로 둔다.
  - **Halt Forecast**: 없음 — 파일 지역 자유 함수이고 매치 누락은 컴파일 오류다(`_` 갈래를 두지 않는다).
  - **Files**: 주 — `src/ui/app.rs`
- [ ] **T7. Deferred 대장을 정리한다** — Type A
  - **Acceptance**:
    - **반영 6건**(T1~T6에 대응)이 「대기」에서 「종결」로 옮겨지고 `- [등록일 → 2026-08-19] … — **반영**(plan `2026-08-19-small-refactors`)` 형식을 따른다.
    - **「사실 오류로 무효」 1건**이 종결로 옮겨지며 무엇이 어긋났는지가 적힌다 — 트리 메뉴 보정 어림값(본받으라던 `remote_menu::menu_size()`도 `FRAME_PAD = 8.0` 어림값이라 「원격처럼 실측으로」라는 방향 자체가 성립하지 않는다).
    - **「판단상 기각」 7건**이 종결로 옮겨지며, 각각 **서술은 참인데 왜 고치지 않는지**를 한 줄로 적는다. **「어긋났다」고 적지 않는다** — 아래 넷은 대장 서술이 지금도 정확하다.
      - `nav_button` — 활성 경로가 `icon_button`과 인자까지 같은 것은 사실이나, 비활성은 `TEXT_DIM`이라 `styled`가 필요하다. 활성만 바꾸면 「활성·비활성 글꼴 크기 동일」 제약이 코드에서 사라진다(대장이 스스로 적은 반대 근거).
      - `file_list::show` — 실질인 `DetailsOutcome`·`GridOutcome` 공통화는, 대장이 적은 *"`sort_click` 하나만 다르다"* 와 달리 지금은 자세히 보기 전용 필드가 여럿이라 묶으면 격자 쪽에 빈 필드가 생긴다.
      - 빈 영역 클릭 두 기법 — 차이가 레이아웃에서 온다(격자는 항목이 영역을 채워 「아래 빈 공간」이 없다). 헬퍼로 뽑아도 분기가 헬퍼 안으로 옮겨갈 뿐이다.
      - `dialog::Shell.clicked` — 사용처 12곳에 제네릭이 필요해 소품 규모를 넘고, 위키 `feat-dialog-shell`이 대화별 타입을 두지 않기로 이미 결정했다.
      - `menu_row`(2곳) · `cancel`/`remove`(2회) · `ListRow::matches_display_rules`(2곳) — 「AGENTS.md 공통화 문턱(3회) 미달 — 2026-08-19 사용자 결정」.
    - **대기에 남기는 2건**이 근거 보강된 채 남는다 — `PanelState::from_tabs` 인자 묶기(호출부 1곳이라 지금은 이득 없음, 사용자가 이번 회차에서 뺌) · `list_grid::show`의 `visible` 부수 출력(구조 변경 필요).
    - 이 plan의 `## Deferred / Follow-up` 2건이 대장 「대기」로 이관된다.
    - **T1~T6이 다룬 여섯 주제와 종결 8건(무효 1 + 판단상 기각 7)이 「대기」에 하나도 남지 않는다** — 각 주제의 핵심 낱말(`gen_licenses`·`잠금을 블록 스코프로`·`PRIMARY_FILL`·`replace`·`too_many_arguments`·`ConflictChoice`·`nav_button`·`4-튜플`·`빈 영역`·`menu_size`·`Shell.clicked`·`menu_row`·`cancel`·`matches_display_rules`)로 「대기」 구간을 검색해 잔존 0을 확인한다. **중복 등재분을 포함한 전수**가 대상이다.
    - 작업 전후의 「대기」 항목 수를 실제로 세어 Progress Log에 적는다(감소 폭은 중복 등재 때문에 처리 건수와 다를 수 있다 — 세어서 확정한다).
    - 문서에 실제 IP·계정·비밀번호·토큰이 없다.
  - **Edge Cases**: 이번 처리 대상 중 **중복 등재는 `gen_licenses`의 `main` 중복 하나**다(`deferred.md:8`과 `:93`에 2026-08-19·2026-08-18 날짜로 두 번 올라 있다) — 종결로 옮길 때 **두 줄 모두** 처리해 한쪽이 대기에 남지 않게 한다. 같은 파일의 「라이선스 검색·필터」(`:6`·`:91`)와 「종류별 묶어 보기」(`:7`·`:92`)도 중복 등재이나 **이번 처리 대상이 아니므로 건드리지 않는다**(기능 확장 묶음 G4 소관).
  - **Halt Forecast**: 없음(문서 편집만).
  - **Files**: 주 — `docs/plans/deferred.md`

## 사전 승인 항목 (일괄 승인 대상)

- **`theme::PRIMARY_FILL` 개명**(크레이트 내부 상수의 공개 이름 변경) — 사용처 3곳이 전부이고 누락은 컴파일 오류로 드러난다.
- **`apply_conflict_choice`의 시그니처 변경**(파일 지역 자유 함수) — 호출부 9곳(프로덕션 2 · 시험 7)을 함께 고치며 전부 `app.rs` 한 파일 안이다.
- **`RemoteRowCtx`·`ConflictDecision` 두 타입 신설** — 둘 다 모듈 밖으로 나가지 않는다.
- **`assets/licenses.json` 재생성 실행**(T1 검증) — 산출물이 현재 파일과 같아야 하는 것이 acceptance이므로 내용이 바뀌면 그 자체가 실패 신호다.

## 불가피한 Halt (위임 불가)

- commit / push / 태그 / 릴리즈 — 구현·검증이 끝난 뒤 별도로 승인받는다.
- 위 사전 승인 항목 밖에서 파일을 지우거나 이름을 바꿔야 하는 상황이 생기면 그 지점에서 멈춘다.
- T1의 산출물 대조가 **불일치**로 나오면(리팩터가 자산을 바꿨다는 뜻) 멈추고 보고한다 — 동작 무변경 전제가 깨진 것이다.

## Open Questions

- [x] Q1. 판정이 갈리는 2건의 채택 범위 → **B1(`apply_conflict_choice` 3상태화)만 채택**, B2(`from_tabs` 인자 묶기)는 제외(사용자 결정, 2026-08-19).
- [x] Q2. 공통화 문턱(3회) 미달 3건 처리 → **기각으로 종결**(사용자 결정, 2026-08-19). AGENTS.md 규약을 지키고 대장에서 뺀다.
- [x] Q3. 코드를 고칠 것이 없는 8건 처리 → **두 갈래로 갈라 종결**(T7) — 「사실 오류로 무효」 1건은 **무엇이 어긋났는지**를, 「판단상 기각」 7건은 **서술은 참인데 왜 고치지 않는지**를 적는다. 후자에 「어긋났다」고 적으면 영구 기록에 거짓이 남는다(1차 판정을 리뷰 지적으로 정정했다). 근거는 Investigation Log에 전건 기록.

## 리뷰 이력

**1라운드** — BLOCKER 2 / MAJOR 2 / MINOR 6. 전건 수용.

| 지적 | 심각도 | 판정 |
|---|---|---|
| B1 T5 인자 수 산술 모순(9−3+1=7인데 acceptance는 6) | BLOCKER | 수용 — acceptance·전제 #5를 7로 정정 |
| B2 `Shell.clicked`가 Out of Scope(존치)와 T7(종결)에서 반대 지시 | BLOCKER | 수용 — 「판단상 기각으로 종결」로 통일 |
| M1 T2 안전 근거가 사실과 다름 | MAJOR | 수용 — `IconCache`는 UI 스레드 단독 소유가 아니고(`drives.rs:88` 워커가 자기 것을 만든다) `ShellGuard`는 비시험 빌드에서 빈 구조체다. 근거를 「스레드별 별개 인스턴스 + `&mut self` 배타 + 비시험 빌드 no-op」로 교체 |
| M2 「기각 5건 = 사실과 어긋난 항목」이 최소 2건에서 성립 안 함 | MAJOR | 수용 — **1차 조사 오류였다**. `nav_button`·빈 영역 클릭은 대장 서술이 지금도 참이다. 종결을 「사실 오류로 무효 1 + 판단상 기각 7」로 재분류 |
| m1 T6 호출부 수치가 plan 안에서 갈림 | MINOR | 수용 — 전부 9곳(프로덕션 2 + 시험 7)으로 통일 |
| m2 clippy 임계 서술 오류 | MINOR | 수용 — `args > 7`에서 발화(8개부터)로 정정 |
| m3 T1 검증 수단이 문면보다 약함 | MINOR | 수용 — `Get-FileHash` 1차, `git diff`는 보조 |
| m4 T4의 임시 편집이 Files 밖 파일을 건드림 | MINOR | 수용 — 그 항목을 삭제하고 「값 리터럴 잔존 0을 검색으로 판정」으로 교체 |
| m5 T7 Edge Case 중복 목록이 처리 대상과 어긋남 | MINOR | 수용 — 처리 대상 중복은 `gen_licenses` 하나임을 명시 |
| m6 file_list 기각 사유가 초점을 비껴감 | MINOR | 수용 — `DetailsOutcome`·`GridOutcome` 차이가 벌어졌다는 사실로 교체 |

**2라운드** — BLOCKER 0 / MAJOR 2 / MINOR 4. **재호출 상한(2회)을 소진해 메인이 직접 대조해 처리했다.** 전건 수용이며 기각한 지적은 없다.

| 지적 | 심각도 | 판정 |
|---|---|---|
| M1 T7 검색어 `shell_display_name`이 무관한 대기 항목(`deferred.md:80` 실패 캐시)을 끌어당김 | MAJOR | 수용 — 검색어를 대장 `:9`의 고유 문구 `잠금을 블록 스코프로`로 교체. 근거를 실물에서 확인했다(`:80`은 별개의 살아 있는 항목이다) |
| M2 1라운드 M2가 `## 요구 이해`·`Q3`에 미반영 — 새 분류와 반대 지시 | MAJOR | 수용 — **1라운드 반영 때 두 섹션을 훑지 않은 누락이다**(같은 지적의 잔존). 두 자리를 새 분류에 맞게 고쳤다 |
| m1 `## 동반 변경 판정` ③행에 「시험 6건」 잔존 | MINOR | 수용 — 7건으로 정정 |
| m2 「기각 8건」이라는 옛 용어가 두 자리에 남음 | MINOR | 수용 — 「종결 8건(무효 1 + 판단상 기각 7)」으로 |
| m3 `remote_menu.rs:206` 라인 지시 어긋남 | MINOR | 수용 — `:203-204`로 정정 |
| m4 T2가 손대는 파일의 낡은 주석이 어느 task에도 없음(판정 유보) | MINOR | 수용 — T2 acceptance에 `icons.rs:356` 주석 한 줄 정정을 넣었다. 이 잠금 구조를 손대는 회차가 이번뿐이다 |

## Phase Ledger

## Retry Ledger

## Progress Log

- **T3-T4 완료** (커밋 `945dca2`, T4는 아래 완료 커밋): `PRIMARY_FILL` → `OK_BAR` 개명 + 옛 세션 호환 시험의 값 의존 제거.
  - **T3은 quality SUGGEST를 그 자리에서 반영했다** — 개명으로 `OK_` 접두를 얻자 그 계열 블록(`OK_DOT`~`OK_BORDER`)에서 떨어져 있는 것이 눈에 걸린다는 지적. Files 안이고 같은 명령으로 재검증되므로 대장에 올리지 않고 바로 옮겼다(F-6.5 등재 게이트).
  - **T4의 값 리터럴 의존이 실제로 사라졌다** — 시험 함수 본문에서 `200.0`·`tiles`·`large_icons`·`replace` 검색 0건. 해당 시험을 이름으로 단독 지정 실행해 실제로 돌고 통과함을 확인했다(전체 통과 수만으로는 그 시험이 돌았는지 알 수 없다).

- **T1-T2 완료** (커밋 `cc7de56`, T2는 아래 완료 커밋): `gen_licenses`의 `main` 갈래 중복 제거 + `fs::icons`의 셸 잠금 세 자리 축소. 866/866 통과, clippy·fmt 경고 0.
  - **T1 동작 무변경이 해시로 실증됐다** — `cargo run --example gen_licenses` 재실행 후 `assets/licenses.json`의 SHA256이 변경 전과 동일(`533f578f…4ad448`). spec 리뷰어가 독립 재실행해 같은 해시를 재현했다.
  - **T2에서 대상이 아닌 여섯 번째 잠금 자리를 확인했다** — `IconCache::new()`(`icons.rs:94`)도 `shell_guard()`를 잡지만 **본문 전체가 셸 호출**이고 `system_image_list`가 그 잠금 안에서 불려 좁히면 재진입 데드락이다(주석이 이미 그 사유를 적고 있다). plan의 「다섯 자리」는 캐시 조회 메서드 쪽을 가리킨 것이며 그 다섯은 모두 같은 블록 형태가 됐다.
  - **T2의 `ShellGuard` 주석은 두 번 고쳤다** — 1차 수정("인스턴스가 스레드마다 별개라 겨룰 상대가 없다")에 quality 리뷰가 인과 비약을 지적했고(m1), `SHELL_LOCK`의 원 주석을 확인해 **진짜 이유가 시험 병렬성**임을 확인했다(`Rust 시험은 기본이 병렬이라 동시에 부르면 SHGetImageList가 16px로 폴백한다`). 원래 문면("UI 스레드 하나가 그리므로")도 1차 수정도 정확하지 않았고, 최종 문면이 그 사실을 인용한다. **자기 유발 지적이라 이연하지 않고 그 자리에서 고쳤다**(규칙 4-1).
