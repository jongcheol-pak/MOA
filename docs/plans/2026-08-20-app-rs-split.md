# Plan: `ui/app.rs` 책임 분리 — 원격·충돌 확인을 자식 모듈로

## 요구 이해

- **원문 요청**: *"g2 작업"* → 범위를 좁힌 뒤 *"분리할 가치가 있는것을 판단해서 진행해"*(무엇을 뗄지 판단을 위임받았다).
- **이해한 요구**: G2(구조 부채) 묶음 중 **`src/ui/app.rs`(3802줄) 책임 분리 하나만** 이번에 한다. 어느 덩어리를 뗄지는 내가 근거로 판정한다. **순수한 이동**이어야 하고 동작·화면·저장 형식이 한 군데도 달라지면 안 되며, 기존 시험 866건이 그대로 통과해야 한다.
- **판정 결과**: **원격 연결·조회(702줄)와 전송 충돌 확인(155줄)을 둘 다** 떼어 `src/ui/app/` 아래 자식 모듈 둘로 나눈다. 근거는 아래 D1.
- **포함하지 않는 것으로 이해**: G2의 나머지(구 Win32 코드 제거 · 모듈 재배치 · `panel.rs`/`site_manager.rs` 판정)는 사용자가 이번 범위에서 뺐다.

## Goal

`ui/app.rs`에서 **「원격 서버와의 연결·조회·명령」과 「전송 충돌 확인」 두 책임이 사라지게** 한다. 둘을 `src/ui/app/remote.rs`·`src/ui/app/transfer_conflict.rs`로 옮기되 **가시성을 한 곳도 넓히지 않고**(자식 모듈이라 부모의 private에 접근된다), 동작은 한 톨도 바꾸지 않는다.

## Out of Scope

- **구 Win32 UI 코드 제거**(7파일 4530줄) — 대장에 「사용자가 화면 확인 후 판단하기로 보류」로 남아 있고 그 화면 확인이 끝나지 않았다.
- **`app/`·`panel/` 모듈 재배치**(`core/` 신설 등) — 대장이 「Win32 제거 후에 검토」로 적었다.
- **`ui/panel.rs`(1645줄)·`ui/site_manager.rs`(1869줄)** — 네 질문을 적용한 적이 없어 판정부터 필요하다.
- **옮기는 코드의 로직 수정** — 이번은 이동뿐이다. 옮기면서 눈에 띈 것은 대장에 등재만 한다.

## Deferred / Follow-up

- app.rs에 남는 나머지 책임(세션·워크스페이스 211줄 · 창·트레이 173줄 · 도크·큐·로그 184줄 · 그 밖 화면 206줄 · 명령 배선 112줄)의 추가 분리 — 이번에 둘을 떼면 ①이 얼마나 남는지 실측 후 판단한다.
- `ui/app.rs`의 시험 644줄도 `app/tests.rs`로 뺄지 — `panel/tests.rs` 선례가 있다. 이번엔 **옮긴 자유 함수의 시험만** 따라 옮기고 나머지는 그대로 둔다.

## Investigation Log

- **위키 참조**: `20_projects/personal/moa/conventions.md` — **`ExplorerApp`은 단위 테스트에서 만들 수 없다**(유일 생성자가 `eframe::CreationContext`를 받고 부르는 곳은 `main`뿐). 그래서 app.rs의 시험 36개는 전부 **자유 함수·순수 로직**만 보며, **이번에 옮기는 `&mut self` 메서드에는 애초에 시험이 없다.** 이동의 회귀 위험을 낮추는 사실이자, acceptance를 「시험 전건 통과」로 잡아도 그것이 메서드 이동을 직접 지켜 주지는 않는다는 뜻이기도 하다(그 축은 컴파일러가 든다).
- **위키 참조**: 같은 파일 — `ExplorerApp`의 `LayoutOutcome` 소비 홉은 「필드별 대입이라 한 줄을 빠뜨려도 컴파일러도 시험도 잡지 못한다」. **이번 작업은 그 홉을 건드리지 않는다**(옮기는 대상에 없다). 만약 옮겨야 할 상황이 생기면 그 자리는 수동 대조가 필요하다.
- **Deferred 대장 조회**: `## 대기` 64건 제목 스캔 + 관련 항목 전문 정독. 이번 작업의 근거 항목은 둘 — 2026-08-05(`ui/app.rs` 책임 분리, 2026-08-20에 판정 근거를 네 질문으로 교체)과 2026-08-15(같은 이름 확인 흐름을 `ui/transfer_conflict.rs`로).
- **전제 반증 (대장 → 이 plan)**: 2026-08-05 항목 *"원격 조회 요청에 출처 태그 — 트리 조회 세대를 `1<<40`부터 세어 목록 조회와 번호 공간을 나눈다. 요청 종류가 셋이 되면 `enum ListSource`로 바꾼다"* 가 이번에 옮기는 `TREE_LIST_BASE`·`CONFLICT_LIST_BASE`에 직접 닿는다. **이 plan은 그것을 고치지 않는다**(순수 이동 제약) — 옮긴 뒤에도 그 항목은 대기에 그대로 남는다.
- **`src/ui/app.rs` 구조 실측(3802줄)**: 상수·COM·글꼴(1~204) · `WorkspaceView`(205~450) · `ExplorerApp` 필드(451~589) · **`impl ExplorerApp`(590~2581, 메서드 65개 약 1993줄)** · 자유 함수 셋(2587~2625) · `impl eframe::App`(2626~2918, 293줄) · 원격·충돌 자유 함수·타입(2919~3158, 240줄) · 시험(3159~3802, 644줄).
- **책임 그룹별 실측**(`impl ExplorerApp` 1993줄을 메서드 단위로 갈라 합산):

  | 그룹 | 메서드 | 줄 수(실측) |
  |---|---|---|
  | **원격 연결·조회** | 25개 | **702** |
  | 그 밖(`new`·`shell_available`·`poll_drives`) | 3개 | 164 |
  | 세션·워크스페이스 | 8개 | 211 |
  | 그 밖 화면 | 5개 | 206 |
  | 도크·큐·로그 | 6개 | 184 |
  | 창·트레이 | 5개 | 173 |
  | **전송 충돌 확인** (`apply_drop` 65줄 포함 — m2) | 6개 | **220** |
  | 명령 배선 | 4개 | 112 |
  | 전송 대상 판정 | 3개 | 21 |

- **`ExplorerApp`의 필드는 전부 private다**(`grep -c "^    pub "` → **0**). 형제 모듈에서는 접근할 수 없어, 분리 방식이 이 사실에 좌우된다.
- **가시성 검증(실제 컴파일)**: scratchpad에 별도 crate를 만들어 두 가지를 확인했다 — ⓐ **자식 모듈에서 부모 타입의 private 필드에 접근하고 `impl`을 추가할 수 있다**(출력 `8`) ⓑ **형제 자식 모듈끼리 서로의 `pub(super)` 메서드를 부를 수 있고, 부모에 남은 private 메서드도 부를 수 있다**(출력 `30`) ⓒ **부모가 자식의 `pub(super)` 타입·상수를 이름으로 쓸 수 있다**(출력 `30`, 경고는 미사용 상수뿐) — **이 셋째 방향이 초안에 빠져 있었고 리뷰가 BLOCKER로 잡았다.** 프로젝트 파일은 건드리지 않았다.
- **기존 선례**: `src/ui/panel.rs`가 `mod workers;`·`#[cfg(test)] mod tests;`를 선언하고 `src/ui/panel/{workers,tests}.rs`에 둔다. **다만 `workers.rs`는 `impl PanelState`가 아니라 자체 타입(`DirLoad`)을 `pub(super)`로 두는 형태**라, 이번의 「impl을 나눠 쓰기」와는 다른 패턴이다. 파일·폴더 병존 구조만 선례로 따른다.
- **충돌 확인 ↔ 원격의 얽힘(호출부 실측)**: `settle_conflict` ← `poll_remote` 2곳(`:2222`·`:2292`)·`drain_conflict_checks`·`abandon_conflict_lists` / `abandon_conflict_lists` ← `release_conn` 2곳(`:2150`·`:2173`)·`poll_remote`(`:2207`) / `start_transfer` ← `apply_remote_menu` 2곳(`:1485`·`:1504`)·프레임 루프(`:2851`) / `drain_conflict_checks`·`show_conflict_dialog` ← 프레임 루프(`:2667`·`:2895`). **얽힘은 양방향이다** — 반대로 `start_transfer`가 `site_connection`(`:1273`), `apply_drop`이 `request_tree`(`:1434`)를 부른다(둘 다 원격 그룹). **그래도 T1 → T2 순서는 유효하다**: Rust에서 형제 모듈의 상호 호출은 순환 문제가 아니고, T1 시점에 `site_connection`·`request_tree`는 아직 부모의 private 메서드라 자식이 그대로 부른다(전제 #1).
- **`settle_dialog`·`matching_site`·`to_tab_phase`는 원격 전용이다(구현으로 미루지 않고 여기서 확정)**: `settle_dialog` 호출부는 `:1592`·`:1616`·`:1631` 셋뿐이고 **전부 `show_remote_dialogs`(1570~1645) 안**이며 시그니처(`:3014`)가 `dialog: &mut Option<RemoteDialog>`로 원격 타입에 묶여 있다(시험 `:3499`·`:3505`·`:3512`도 원격 대화를 만든다). `matching_site`는 `open_remote_url`(`:2457`), `to_tab_phase`는 `poll_remote`(`:2194`)에서만 불린다. **셋 다 T2가 가져가며 「남긴다」 분기는 없다.**
- **`apply_drop`은 T1에 편입한다(m2)**: 호출부가 `start_transfer`(`:1274`)·`settle_conflict`(`:1344`)·`show_conflict_dialog`(`:1372`) **셋뿐이고 전부 T1 대상**이며, 자기 문서주석(`:1223`)이 `start_transfer`를 *"`apply_drop`의 유일한 앞문"*이라 적는다. 남기면 부모에 **호출자가 하나도 없는 65줄 메서드**가 뜬다.
- **비재귀 소스 훑기 시험이 둘이다(M1 — 초안이 하나만 봤다)**: `ui::dialog`의 `대화는_모두_이_모듈을_거친다`(`dialog.rs:477`)와 **`ui::widgets`의 아이콘 리터럴 시험**(`widgets.rs:977`) 둘 다 `read_dir`로 `src/ui` **바로 아래만** 훑는다. 이번에 나가는 850줄에 `show_conflict_dialog`·`show_remote_dialogs`가 들어 있어, 옮긴 뒤에는 그 코드의 `Modal::new` 직접 사용·금지 아이콘 리터럴을 **어느 시험도 잡지 못한다.** `theme.rs:156-167`의 재귀 헬퍼(`ui_sources`)가 그 함정을 주석으로 명시하고 대장 61행에도 등재돼 있다 — **이번 이동이 그 대기 항목을 실제 결함으로 만들므로 T3으로 함께 고친다.**
- **`docs/prd.md` 경량 확인**: 이번 변경은 파일 배치만 바꾸므로 어떤 FR 문면과도 어긋나지 않는다. `**PRD**:` 줄을 두지 않으며 Phase G는 발동하지 않는다.

### 전제 검증

| # | 이 plan이 참으로 삼는 것 | 확인 근거 | 판정 |
|---|---|---|---|
| 1 | **자식 모듈에서 부모의 private 필드·메서드에 접근할 수 있다** — 이것이 부정되면 필드를 `pub(crate)`로 넓혀야 해 「가시성을 넓히지 않는다」가 무너지고 task 자체가 성립하지 않는다 | scratchpad crate를 실제로 컴파일·실행해 확인(출력 `8`) | ✅ 확인(실행) |
| 2 | **형제 자식 모듈끼리 서로를 부를 수 있다** — `app/remote.rs`의 `poll_remote`가 `app/transfer_conflict.rs`의 `settle_conflict`를 부른다 | 같은 crate에 형제 모듈을 더해 확인(출력 `30`) | ✅ 확인(실행) |
| 2-B | **부모가 자식의 `pub(super)` 타입·상수를 이름으로 쓸 수 있다** — `ExplorerApp` 필드(`:550` `remote_ops: RemoteOps`, `:567`·`:583`·`:585` `ConflictCheck`)와 `new`(`:687` `RemoteOps::default()`)가 **app.rs에 남는데** 그 타입은 자식으로 간다. 이것이 부정되면 T1·T2가 컴파일되지 않는다 | 같은 crate에 부모→자식 참조를 더해 확인(출력 `30`). **초안에 없던 방향이며 리뷰가 BLOCKER로 지적해 추가 검증했다** | ✅ 확인(실행) |
| 3 | 옮기는 메서드에는 **직접 시험이 없다** | 위키 `conventions.md`의 *"`ExplorerApp`은 단위 테스트에서 만들 수 없다"* + app.rs 시험 36개가 전부 자유 함수·순수 로직 대상임을 목록으로 확인 | ✅ 확인 |
| 4 | **순수 이동이면 컴파일러가 누락을 전수 검출한다** — 필드 접근 불가·이름 불일치·시그니처 어긋남이 전부 컴파일 오류다 | Rust의 가시성·이름 해석 규칙. 이번 이동에 동적 디스패치·리플렉션·문자열 기반 참조가 없음을 확인(옮기는 대상은 전부 inherent 메서드와 자유 함수) | ✅ 확인 |
| 5 | 원격 그룹 25개·충돌 그룹 6개(`apply_drop` 포함)라는 **대상 집합이 빠짐없다** | `impl ExplorerApp` 구간(590~2581)에서 메서드 시작 줄을 기계로 뽑아 **65개**를 전수 열거한 뒤 그룹에 배정했다(그룹 표 합계와 일치). 미배정 **0**을 확인했다 — 초안의 「64개」는 집계 오류이며 리뷰가 실측으로 정정했다 | ✅ 확인(리뷰가 재실측) |
| 6 | 이번 변경이 닿는 **active FR이 없다** | `docs/prd.md` FR 제목 훑기 — 파일 배치만 바뀌고 동작·화면·저장 형식이 그대로다 | ✅ 확인 |

## 동반 변경 판정

| 축 | 발견 | 구분 | 처리 |
|---|---|---|---|
| ① 서술 문서 | `AGENTS.md`의 Repository Structure가 `src/ui/`를 「egui UI 계층」으로만 적어 하위 폴더를 나열하지 않는다. `src/ui/panel/`도 적혀 있지 않아 **이번에 `src/ui/app/`이 생겨도 어긋나지 않는다** | 무관 | 갱신 없음 |
| ② 규약 복제 | 없음 — 이번 대상 중 정본이 둘 이상인 지점 없음 | 무관 | — |
| ③ 검증 자산 | 소스 훑기 시험이 **셋**이다 — `ui::dialog`의 `대화는_모두_이_모듈을_거친다`(`dialog.rs:477`, **비재귀**) · `ui::widgets`의 아이콘 리터럴 시험(`widgets.rs:977`, **비재귀**) · `ui::theme`의 `팝업_메뉴는_모서리를_따로_적지_않는다`(`theme.rs:158`, 재귀). **앞의 둘은 `src/ui` 바로 아래만 훑어 `src/ui/app/`을 통째로 놓친다** — 이번에 그리로 나가는 850줄에 대화를 그리는 `show_conflict_dialog`·`show_remote_dialogs`가 들어 있다 | **필수** | **T3 신설** — 두 시험을 재귀로 바꾼다. 이번 이동이 만드는 검증 공백이라 이연할 수 없다(규칙 4-1) |
| ④ 버전·매니페스트 | 버전을 올리지 않는다 | 무관 | — |
| ⑤ 무효화되는 기능·설정 | 없음 — 파일 배치만 바뀐다 | 무관 | — |
| 대장 정합 | 2026-08-05(app.rs 분리)·2026-08-15(transfer_conflict)·2026-08-15(비재귀 훑기 시험) **세 항목**이 이번 작업의 대상이다 | **필수** | T4로 편입 |

## Impact Analysis

### 4-A. 심볼 추적

**대상 집합을 먼저 소스에서 열거했다** — `impl ExplorerApp`(590~2581)의 메서드 **65개 전수**를 기계로 뽑아 그룹에 배정하고 미배정 0을 확인했다.

**T1이 옮길 것 (전송 충돌 확인)**

| 종류 | 심볼 | 현 위치 |
|---|---|---|
| 메서드 **6** | `start_transfer` · `drain_conflict_checks` · `abandon_conflict_lists` · `settle_conflict` · `show_conflict_dialog` · **`apply_drop`**(m2로 편입 — 호출부 셋이 전부 이 목록 안이다) | `:1228`~**`:1447`** |
| 상수 1 | `CONFLICT_LIST_BASE` | `:2944` |
| 자유 함수 4 | `conflict_generation` · `conflict_id` · `conflict_names` · `apply_conflict_choice` | `:2948`·`:2953`·`:3050`·`:3093` |
| 타입 3 | `ConflictDecision` + `impl From<ConflictChoice>` · `ConflictCheck` | `:3071`~`:3123` |
| 시험 | 충돌 관련 시험과 그 헬퍼(`받기_전송`·`이름들` 등 — T1이 실제로 세어 확정) | 시험 구간 |

**T2가 옮길 것 (원격 연결·조회)** — 메서드 25개(702줄 실측) + 자유 함수·타입 **11개**

> **`NOTICE_SECS`(`:2966`)는 옮기지 않는다**(2라운드 M2) — 사용처 셋 중 `:1872`가 **`sync_tray`**(창·트레이 그룹, app.rs 잔류) 안이다. 원격 전용이 아니라 「알림 표시 시간」이라는 공용 상수이므로 부모에 남긴다. 옮기면 잔류 코드가 자식 모듈 상수를 되참조해 Goal의 「원격 책임이 사라진다」가 역방향 의존으로 상쇄된다.

`panel_conn` · `remote_dir` · `request_tree_children` · `request_tree` · `site_connection` · `log_connection` · `connected_conn_sites` · `connected_sites` · `failed_sites` · `poll_remote` · `on_op_done` · `revert_remote_move` · `list_moved_panels` · `request_remote_list` · `apply_remote_action` · `reconnect_panel` · `open_remote_url` · `release_conn` · `conn_in_use` · `connect_site` · `open_site_tab` · `open_site_tab_here` · `open_site_tab_at` · `apply_remote_menu` · `show_remote_dialogs`

여기에 원격 소속 자유 함수·타입: `RemoteOps`(`:2919`) · `TREE_LIST_BASE`(`:2938`) · `sort_tree_children`(`:2959`) · `op_failure_message`(`:2969`) · `OpOutcome`(`:2982`) · `op_outcome`(`:2995`) · `settle_dialog`(`:3014`) · `delete_command`(`:3035`) · `RemoteDialog`(`:3124`) · `matching_site`(`:2587`) · `to_tab_phase`(`:2604`)

**호출부**: 옮기는 심볼은 전부 `src/ui/app.rs` **안에서만** 불린다(`grep -rn` 전수 — `connect_site`만 `pub`이며 그 호출부도 같은 파일이다. 자식 모듈로 옮겨도 `pub(super)`면 부모가 부를 수 있다). **모듈 밖 호출부 0.**

### 4-B. 계약·직렬화

- **직렬화 변화 0** — 세션 스키마·설정 파일에 손대지 않는다.
- **공개 표면 변화**: `ExplorerApp::connect_site`가 지금 `pub`인데 호출부가 같은 파일뿐이다. **이번엔 `pub`을 그대로 둔다**(가시성을 좁히는 것도 이동 범위 밖이며, 좁히면 그것대로 계약 변경이다). 나머지 옮긴 메서드는 `pub(super)`로 둔다 — 지금 private인 것이 crate 밖으로 새지 않는다.

### 4-C. 영향 받는 테스트

| 시험 | 영향 | 처리 |
|---|---|---|
| app.rs 시험 36개 중 **옮긴 자유 함수를 부르는 것** | 그 함수가 자식 모듈로 가면 경로가 바뀐다 | 해당 시험을 함께 옮기거나 `use super::…`를 더한다(T1·T2가 각각 판정) |
| `ui::dialog`의 `대화는_모두_이_모듈을_거친다`(비재귀) · `ui::widgets`의 아이콘 리터럴 시험(비재귀) | `src/ui/app/`이 **둘의 사각지대에 든다** — 옮긴 대화 코드를 아무도 검사하지 않게 된다 | **T3이 재귀로 바꾼다**(이번 이동이 만드는 공백이라 이연 불가) |
| `ui::theme`의 `팝업_메뉴는_모서리를_따로_적지_않는다`(재귀) | 새 폴더를 훑는다 | T2 acceptance에 통과 명시 |
| 그 밖 866건 | 무관 | 전건 통과가 각 task의 acceptance |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 | 재사용/신규 사유 |
|---|---|---|
| `src/ui/app/transfer_conflict.rs`(모듈) | `src/ui/panel/workers.rs`가 같은 「파일 + 동명 폴더」 구조를 쓴다 | **신규** — 선례의 구조만 따르고 내용은 이동분이다. 새 타입·새 함수를 만들지 않는다 |
| `src/ui/app/remote.rs`(모듈) | 같음 | **신규** — 위와 같다 |

**신규 로직은 0이다** — 두 파일 모두 기존 심볼을 옮겨 담을 뿐이다.

### 4-E. 동반 변경 판정

위 `## 동반 변경 판정` 표 참조 — **필수 2건**(소스 훑기 시험 · 대장 정합)은 **T3·T4**에 편입했고 선택 항목은 없다.

## Decisions

- **D1. 원격과 충돌 확인을 둘 다 뗀다** (사용자가 판단을 위임 — 2026-08-20). 근거 셋:
  ① **원격이 702줄로 최대 덩어리**라 ①(변경 이유가 둘 이상)에 가장 크게 기여한다. 충돌 확인만 떼면 155줄이 빠질 뿐 원격이 그대로 남아 ①이 거의 해소되지 않는다.
  ② **충돌 확인은 원격의 하위 흐름**이다 — 호출부 실측에서 `settle_conflict`가 `poll_remote`에서 2번, `abandon_conflict_lists`가 `release_conn`·`poll_remote`에서 3번 불린다. 원격만 떼면 그 경계를 계속 넘나든다.
  ③ 세대 번호 공간을 공유한다(`TREE_LIST_BASE` = `1<<40`, `CONFLICT_LIST_BASE` = `2<<40` — 같은 체계).
- **D2. 그래도 파일은 둘로 나눈다** — 합치면 850줄짜리 새 파일이 되어 방금 푼 문제를 다시 만든다. 둘은 도메인이 다르다(서버와의 통신 vs 같은 이름 판정).
- **D3. 자식 모듈 방식(`src/ui/app/`)을 쓴다** — `ExplorerApp`의 필드가 전부 private이라 형제 모듈(`src/ui/remote_ops.rs` 등)로는 접근할 수 없고, 접근하려면 필드를 `pub(crate)`로 넓혀야 한다. 자식 모듈이면 **가시성을 한 곳도 넓히지 않는다**(전제 검증 #1·#2). **Source**: 실제 컴파일 검증 + `src/ui/panel.rs`의 `mod workers;` 선례.
- **D4. `impl ExplorerApp`을 나눠 쓴다** — `panel/workers.rs`처럼 별도 타입으로 뽑지 않는다. 그것은 상태 필드까지 옮기는 **구조 변경**이라 「순수 이동」 제약을 넘고, 옮기면서 로직을 손대게 된다.
- **D5. 옮긴 메서드는 `pub(super)`** — 부모(`ui::app`)와 형제 자식 모듈이 부를 수 있으면 충분하다. `pub`으로 올리면 crate 밖으로 샌다. 예외는 이미 `pub`인 `connect_site` 하나이며 그대로 둔다(4-B).
- **D6. 파일 이름은 `remote.rs`·`transfer_conflict.rs`** — 후자는 대장 2026-08-15 항목이 지목한 이름을 그대로 쓴다(위치만 `ui/` → `ui/app/`으로 바뀐다). 전자는 `ui::remote`(프로토콜 계층)와 이름이 겹치지만 **경로가 `ui::app::remote`라 구분되고**, 파일이 다루는 것이 정확히 「앱이 원격을 다루는 배선」이다.
- **D7. 시험은 옮긴 자유 함수의 것만 따라 옮긴다** — app.rs 시험 644줄 전체를 `app/tests.rs`로 빼는 것은 이번 범위 밖이다(Deferred에 등재). 옮긴 함수를 부르는 시험만 함께 가고, 나머지는 app.rs에 남는다.

## Tasks

- [ ] **T1. 전송 충돌 확인을 `src/ui/app/transfer_conflict.rs`로 옮긴다** — Type D
  - **Design**: ① 배치 — `src/ui/app.rs`에 `mod transfer_conflict;`를 선언하고 새 파일 `src/ui/app/transfer_conflict.rs`를 만든다. ② 신규 심볼과 책임 — **없다.** 이 파일은 기존 심볼(메서드 **6** · 상수 1 · 자유 함수 4 · 타입 3)을 담을 뿐이고 `impl ExplorerApp` 블록 하나를 연다. ③ 의존 방향 — `ui::app`의 자식이며 부모의 private 필드·메서드를 쓴다. 밖으로는 아무것도 내보내지 않는다(옮긴 메서드는 `pub(super)`). ④ 비추상화 — 상태를 별도 타입으로 뽑지 않는다(D4). 「충돌 확인 서비스」 같은 추상을 만들지 않는다.
  - **Acceptance**:
    - `src/ui/app/transfer_conflict.rs`가 생기고 4-A 표의 **T1 대상 전부**(메서드 **6**(`apply_drop` 포함) · `CONFLICT_LIST_BASE` · 자유 함수 4 · 타입 3)가 거기 있다. `app.rs`에는 그 심볼들의 정의가 **하나도 남지 않는다**(각 이름으로 `app.rs`를 검색해 정의 잔존 0).
    - **옮긴 코드의 본문이 한 글자도 바뀌지 않는다** — 주석·시그니처·로직이 그대로다. 바뀔 수 있는 것은 **`fn`·`struct`·`enum`·`const`의 `pub(super)` 표기**와 `use` 뿐이다(전제 2-B — `ConflictCheck`는 부모 필드가 이름으로 쓰므로 `pub(super) struct`여야 한다). `git diff`에서 이동분을 대조해 확인한다.
    - **`ExplorerApp`의 필드 가시성이 하나도 바뀌지 않는다** — 검사는 `grep -cE "^    pub" src/ui/app.rs`로 한다(**공백을 요구하지 않는 접두**라 `pub(super) x: T`도 잡힌다. 2라운드 m4 — 종전 `"^    pub "`는 뒤 공백 때문에 `pub(super)`로 넓혀도 0이 나와 막으려는 것을 통과시켰다). 작업 전후 값이 같아야 한다.
    - 충돌 관련 시험이 새 위치에서 돌고 통과한다(옮길지 `use`로 이을지는 T1이 판정해 적용).
    - `cargo build` · `cargo test` **866건 전건 통과** · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: 옮긴 뒤 `app.rs`에서 쓰이지 않게 된 `use`가 남으면 clippy가 잡는다 — 그것만 정리하고 다른 `use`는 건드리지 않는다. `ConflictDecision`이 `ui::list_common::ConflictChoice`를 `From`으로 받으므로 새 파일에 그 `use`가 필요하다. 시험이 `super::*`로 부모를 참조하면 옮긴 심볼이 안 보일 수 있다 — 그 경우 시험도 함께 옮기거나 경로를 명시한다.
  - **Halt Forecast**: 없음 — 새 파일 추가와 코드 이동뿐이고 삭제·이름 변경이 없다. 누락은 전부 컴파일 오류로 즉시 드러난다(전제 #4).
  - **Files**: 주 — `src/ui/app.rs`, `src/ui/app/transfer_conflict.rs`(신규)
- [ ] **T2. 원격 연결·조회를 `src/ui/app/remote.rs`로 옮긴다** — Type D
  - **Design**: ① 배치 — `src/ui/app.rs`에 `mod remote;`를 선언하고 `src/ui/app/remote.rs`를 만든다. ② 신규 심볼과 책임 — **없다.** 메서드 25개와 원격 소속 자유 함수·타입 12개를 담는다. ③ 의존 방향 — `ui::app`의 자식이며, **형제인 `app::transfer_conflict`의 `pub(super)` 메서드를 부른다**(`poll_remote` → `settle_conflict` 등 — 전제 검증 #2가 이 경로를 확인했다). ④ 비추상화 — 「원격 배선」을 트레이트나 서비스 타입으로 감싸지 않는다. `RemoteOps`는 이미 있는 구조체라 그대로 옮긴다.
  - **Acceptance**:
    - `src/ui/app/remote.rs`가 생기고 4-A 표의 **T2 대상 전부**(메서드 25 · 자유 함수·타입 12)가 거기 있다. `app.rs`에 그 정의가 **하나도 남지 않는다**(각 이름으로 검색해 정의 잔존 0).
    - **옮긴 코드의 본문이 한 글자도 바뀌지 않는다**(T1과 같은 기준 — `pub(super)` 표기와 `use`만 예외. `RemoteOps`는 부모 필드가 이름으로 쓰므로 `pub(super) struct`여야 한다).
    - **`ExplorerApp`의 필드 가시성이 하나도 바뀌지 않는다**(T1과 같은 검사 — `grep -cE "^    pub"`, 공백 없는 접두).
    - **`조회_세대는_다른_조회와_번호가_겹치지_않는다`(`:3790`)는 T2가 맡는다**(m4) — 이 시험이 `TREE_LIST_BASE`와 `CONFLICT_LIST_BASE`를 한 자리에서 단언하는데 두 상수가 서로 다른 자식 모듈로 갈린다. **두 상수 모두 `pub(super)`가 되어야 하고**, 시험은 `app/remote.rs`에 두되 충돌 쪽 상수를 경로로 참조한다.
    - `ui::theme`의 재귀 시험이 새 폴더를 훑고도 통과한다(app.rs에 `Frame::menu(`가 0건이라 위험은 낮지만 확인은 한다). **비재귀 두 시험의 공백은 T3이 닫는다.**
    - `cargo build` · `cargo test` **866건 전건 통과** · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
    - **네 질문 ①의 잔존 상태를 실측해 Progress Log에 적는다** — 두 책임을 뗀 뒤 `app.rs`에 남은 책임 그룹을 열거한다(줄 수가 목표가 아니라 **어떤 변경 이유가 남았는지**가 기록 대상이다).
  - **Edge Cases**: `poll_remote`가 형제 모듈의 `settle_conflict`·`abandon_conflict_lists`를 부르므로 T1이 먼저 끝나야 한다(의존: T1 → T2). `matching_site`·`to_tab_phase`는 `impl` 밖 자유 함수라 옮길 때 `use`가 함께 따라가야 한다. `settle_dialog`는 **원격 전용임이 계획 단계에서 확정됐다**(Investigation Log — 호출부 셋이 전부 `show_remote_dialogs` 안) — 조건부로 남기지 않고 옮긴다.
  - **Halt Forecast**: 없음 — T1과 같은 성질이다. 다만 이동량이 커(약 850줄) 중간에 컴파일이 여러 번 깨질 수 있는데, 그것은 정상 진행이며 Halt 사유가 아니다.
  - **Files**: 주 — `src/ui/app.rs`, `src/ui/app/remote.rs`(신규)
- [ ] **T3. 비재귀 소스 훑기 시험 둘을 재귀로 바꾼다** — Type C
  - **왜 이번에 하는가**: T1·T2가 850줄을 `src/ui/app/`으로 내보내는데 그 폴더는 두 비재귀 시험의 사각지대다. **이번 이동이 만드는 검증 공백이라 이연할 수 없다**(규칙 4-1). 대장 61행에 이미 등재된 항목이며 `theme.rs:156-157`의 주석이 그 함정을 명시한다.
  - **Design**: ① 배치 — `src/ui/dialog.rs`·`src/ui/widgets.rs`의 각 시험 안. ② 신규 심볼과 책임 — 두 파일에 **재귀 수집 헬퍼**를 각각 둔다(`theme.rs`의 `ui_sources`와 같은 형태). ③ 의존 방향 — 시험 전용(`#[cfg(test)]`)이며 프로덕션 코드는 건드리지 않는다. ④ 비추상화 — 세 시험이 공유하는 헬퍼를 **공용 모듈로 뽑지 않는다**(중복 3회지만 시험 전용 코드이고 공용화하면 `#[cfg(test)]` 가시성을 넘나드는 배선이 생긴다 — 그 판단은 대장에 남긴다).
  - **Acceptance**:
    - `dialog.rs`의 `대화는_모두_이_모듈을_거친다`와 `widgets.rs`의 아이콘 리터럴 시험이 **`src/ui` 하위 폴더까지 재귀로** 훑는다(`src/ui/app/`·`src/ui/panel/` 모두 대상).
    - **`dialog.rs`의 자기 제외를 파일 이름이 아니라 전체 경로 비교로 바꾼다**(2라운드 m3) — 지금은 `path.file_name() == "dialog.rs"`(`dialog.rs:484`)라, 재귀로 바꾸면 하위 폴더에 같은 이름이 생겼을 때 그 파일까지 조용히 빠진다. `theme.rs:223-225`가 정확히 그 이유로 경로 비교를 쓰고 있어 그 방식을 따른다.
    - **재귀로 바꾼 뒤에도 두 시험이 통과한다** — 새로 훑게 된 파일들(`app/remote.rs`·`app/transfer_conflict.rs`·`panel/workers.rs`·`panel/tests.rs`)에 위반이 없어야 한다. **위반이 나오면 그것이 곧 이번 이동이 숨길 뻔한 결함이므로 그 자리에서 고친다.**
    - 재귀화가 실제로 동작함을 확인한다 — 훑은 파일 수가 비재귀 때보다 늘었음을 시험 안에서 단언하거나, 임시로 하위 폴더에 위반을 넣어 걸리는지 본 뒤 되돌린다(둘 중 T3이 택한다).
    - `cargo test` **전건 통과** · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 경고 0.
  - **Edge Cases**: `dialog.rs` 시험은 자기 파일을 건너뛰는 예외가 있다 — 재귀로 바꿔도 그 예외가 유지돼야 한다. `panel/tests.rs`가 새로 훑히는데 시험 코드의 기대값 문자열에 금지 기호가 있을 수 있다 — 걸리면 진짜 위반인지 판정한다.
  - **Halt Forecast**: 없음 — 시험 코드만 바꾼다. **새로 훑히는 두 파일에 위반이 없음을 2라운드 리뷰가 실측했다**(`panel/workers.rs`·`panel/tests.rs`에 `Modal::new` 0건, 금지 아이콘 리터럴 11종 0건). 그래도 예상 밖 위반이 나오면 규칙 4-1대로 이번에 고치되 5개 파일 이상 연쇄에 닿으면 멈춘다.
  - **Files**: 주 — `src/ui/dialog.rs`, `src/ui/widgets.rs`
- [ ] **T4. 대장·문서 정합** — Type A
  - **Acceptance**:
    - `docs/plans/deferred.md`의 **2026-08-15 항목**(같은 이름 확인 흐름을 `ui/transfer_conflict.rs`로)이 **반영**으로 종결된다. 실제 위치가 `ui/app/transfer_conflict.rs`인 것과 그 사유(자식 모듈이라야 private 필드에 접근된다)를 적는다.
    - **2026-08-05 항목**(`ui/app.rs` 책임 분리)은 **대기에 남되 갱신된다** — 원격·충돌 두 책임이 빠졌다는 것과, 남은 책임 그룹·실측 줄 수를 적는다. 완전히 해소된 것이 아니므로 종결하지 않는다.
    - **2026-08-15 항목「규약 시험이 `src/ui` 바로 아래만 훑는다」(대장 61행)가 T3 결과로 종결**된다 — 두 시험이 재귀가 됐음을 적는다.
    - 이번에 새로 생긴 Deferred 3건(app.rs 잔여 책임의 추가 분리 · 시험 644줄의 `app/tests.rs` 분리 · **세 소스 훑기 시험의 재귀 헬퍼 중복 3회**)이 대기로 이관된다.
    - 문서에 실제 IP·계정·비밀번호·토큰이 없다.
  - **Edge Cases**: 2026-08-05 항목은 2026-08-20에 이미 한 번 고쳐 썼다 — 그 문면을 지우지 말고 **이번 결과를 덧붙인다**(판정 근거 교체 이력이 남아야 한다).
  - **Halt Forecast**: 없음(문서 편집만).
  - **Files**: 주 — `docs/plans/deferred.md`

## 사전 승인 항목 (일괄 승인 대상)

- **`src/ui/app/` 폴더와 파일 둘 신규 생성** — 기존 파일을 지우거나 이름을 바꾸지 않는다. `src/ui/panel/` 선례와 같은 구조다.
- **`src/ui/app.rs`에 `mod` 선언 2줄 추가** — 크레이트 내부 모듈 구조 변경이며 crate 밖 표면은 그대로다.
- **메서드 가시성을 `pub(super)`로 표기** — 지금 private인 것이 부모·형제에게만 보이게 된다. crate 밖으로 나가지 않는다.

## 불가피한 Halt (위임 불가)

- commit / push / 태그 / 릴리즈 — 구현·검증이 끝난 뒤 별도로 승인받는다.
- **가시성을 `pub(crate)` 이상으로 넓혀야 하는 상황이 생기면 멈춘다** — 그것은 이 plan의 전제(#1·#2·**#2-B**)가 깨졌다는 뜻이라 설계를 다시 봐야 한다. `pub(super)`까지는 계획된 범위다.
- **순수 이동으로는 옮길 수 없어 로직을 고쳐야 하는 자리가 나오면 멈춘다** — 「옮기면서 고치지 않는다」는 제약을 넘는다.
- 위 사전 승인 항목 밖에서 파일을 지우거나 이름을 바꿔야 하면 그 지점에서 멈춘다.

## Open Questions

- [x] Q1. G2 중 이번 회차 범위 → **`app.rs` 책임 분리만**(사용자 결정, 2026-08-20). 구 Win32 제거·모듈 재배치·`panel.rs`/`site_manager.rs`는 Out of Scope.
- [x] Q2. 무엇을 뗄 것인가 → **사용자가 판단을 위임**했고, 호출 관계·크기 실측을 근거로 **원격 연결·조회 + 전송 충돌 확인 둘 다**로 정했다(D1).
- [x] Q3. 형제 모듈이냐 자식 모듈이냐 → **자식 모듈**(D3). `ExplorerApp`의 필드가 전부 private이라 형제로는 접근이 불가능하며, 자식이면 가시성을 넓히지 않아도 된다(실제 컴파일로 검증).

## 리뷰 이력

**1라운드** — BLOCKER 1 / MAJOR 2 / MINOR 4. **전건 수용**(기각 0).

| 지적 | 심각도 | 판정 |
|---|---|---|
| B1 부모에 남는 코드가 자식으로 옮길 타입을 이름으로 쓴다(`ExplorerApp` 필드의 `RemoteOps`·`ConflictCheck`, `new`의 `RemoteOps::default()`) — 전제 검증이 **부모→자식 방향을 안 봤다** | BLOCKER | 수용 — 그 방향을 scratchpad에서 추가 컴파일해 확인(전제 2-B 신설). acceptance의 가시성 예외를 `fn`·`struct`·`enum`·`const`의 `pub(super)`로 넓혔다 |
| M1 비재귀 소스 훑기 시험이 **둘**인데 하나만 봤고 그것도 「무관」으로 처리 — 850줄이 그 사각지대로 나간다 | MAJOR | 수용 — **T3을 신설**해 두 시험을 재귀로 바꾼다. 이번 이동이 만드는 검증 공백이라 이연 불가(규칙 4-1) |
| M2 `settle_dialog` 소속 판정을 구현으로 이연했고 그 분기가 acceptance와 충돌 | MAJOR | 수용 — 리뷰어가 실측한 호출부 셋을 근거로 **원격 전용 확정**, Edge Case의 「남긴다」 삭제 |
| m1 메서드 수가 64/65로 어긋남 | MINOR | 수용 — 65개로 정정(그룹 표 합계와 일치) |
| m2 `apply_drop`의 「그 밖」 분류에 근거 없음 — 호출부 셋이 전부 T1 대상 | MINOR | 수용 — **T1에 편입**(주석이 `start_transfer`를 「유일한 앞문」이라 적는다) |
| m3 얽힘이 한 방향만 적혀 있음 | MINOR | 수용 — 양방향으로 정정(`start_transfer`→`site_connection`, `apply_drop`→`request_tree`). 순서 의존은 그대로 유효 |
| m4 세대 시험이 두 상수를 함께 단언해 귀속이 안 정해짐 | MINOR | 수용 — **T2가 맡는다**고 명시하고 두 상수 모두 `pub(super)`가 필요함을 적었다 |


**2라운드** — BLOCKER 0 / MAJOR 2 / MINOR 4. **재호출 상한(2회)을 소진해 메인이 직접 대조해 처리했다.** 전건 수용이며 기각은 없다. CONFLICT 없음.

| 지적 | 심각도 | 판정 |
|---|---|---|
| M1 1라운드 m2(`apply_drop` 편입) 반영이 **정본 표에 닿지 않았다** — 그룹 표·Investigation Log만 고치고 4-A T1 표·Design ②·Acceptance는 「메서드 5」 그대로 | MAJOR (**RECURRING**) | 수용 — **내 반영 누락이다.** 세 자리를 「메서드 6」·범위 `:1228~:1447`로 고쳤다. 그대로 뒀으면 `apply_drop`이 app.rs에 남는데도 acceptance가 통과했을 것이다 |
| M2 `NOTICE_SECS`가 원격 전용이 아님 — `sync_tray`(잔류)가 쓴다 | MAJOR | 수용 — 근거를 직접 확인했다(`:1872`가 `sync_tray` 안, 나머지 둘만 원격). T2 목록에서 빼고 app.rs 잔류로 명시(자유 함수·타입 12 → **11**) |
| m1 4-C 표의 dialog 행이 T3 신설과 모순 | MINOR | 수용 — 「T3이 재귀로 바꾼다」로 갱신하고 widgets 행도 합쳤다 |
| m2 T 번호 재배치 후 잔존 참조(4-E의 「T2·T3」, Halt의 전제 목록) | MINOR | 수용 — 「T3·T4」로, Halt에 전제 **2-B** 추가 |
| m3 `dialog.rs` 자기 제외를 이름 비교로 두면 재귀화가 새 구멍을 만든다 | MINOR | 수용 — T3 acceptance에 **전체 경로 비교로 바꾼다**를 명시(`theme.rs:223-225`가 같은 이유로 그 방식을 쓴다) |
| m4 필드 가시성 검사가 막으려는 것을 못 잡는다 — `"^    pub "`는 뒤 공백 때문에 `pub(super)`를 놓친다 | MINOR | 수용 — `grep -cE "^    pub"`(공백 없는 접두)로 고쳤다. **내가 만든 acceptance가 자기 목적을 배반하던 자리다** |

**2라운드 리뷰가 실측으로 확인해 준 것**(계획에 반영): 부모가 이름으로 쓰는 자식 소속 타입은 `RemoteOps`·`ConflictCheck` **둘이 전부**이며 나머지 후보(`ConflictDecision`·`OpOutcome`·`RemoteDialog`·`TREE_LIST_BASE`)는 함께 옮겨 가는 시험 안에서만 쓰인다 · `panel/workers.rs`·`panel/tests.rs`에 T3이 걸릴 위반이 **0건** · `apply_drop`의 콜리가 T1·T2 두 시점 모두 접근 가능 · T1→T2→T3→T4 순서 유효.

## Phase Ledger

## Retry Ledger

## Progress Log
