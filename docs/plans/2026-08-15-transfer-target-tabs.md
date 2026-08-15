# Plan: 전송 대상 탭 표시·지정과 같은 이름 충돌 확인

**PRD**: docs/prd.md

## 요구 이해
- **원문 요청**: "받기 개능 개선 — 탭에 폴더 이름과 폴더 아이콘이 표시가 되는데 사용자가 마지막에 선택한 탬이면 폴더 아이콘 대신 다운로드 아이콘을 표시 / 사용자가 마지막에 선택한 탭(다운로드 아이콘이 표시된 탭)으로 다운로드 되도록 수정 / 다운 받는 곳에 동일한 파일이 있는 경우 경고 팝업을 표시하고 덮어 쓰기, 건너 뛰기 버튼을 표시. 올리기 개능 개선 — (같은 세 가지를 원격 서버 탭·업로드 아이콘으로)"
- **이해한 요구**: 전송의 **목적지가 어느 탭인지 화면에서 보이게** 하고(폴더 아이콘 자리를 받기/올리기 아이콘으로 바꿔 표시), 실제 전송도 **그 표시된 탭으로** 가게 한다. 목적지에 같은 이름이 이미 있으면 전송을 시작하기 전에 확인 대화를 띄워 `덮어쓰기`·`건너뛰기`·`취소` 중 하나를 받는다. 대상 판정은 "마지막으로 누른 패널의 활성 탭"이며 로컬 탭은 받기 대상, 원격 탭은 올리기 대상이다.
- **포함하지 않는 것으로 이해**: 이름을 바꿔 저장하기(`(2)` 자동 번호)·폴더 동기화/비교는 만들지 않는다 — 요청한 버튼은 `덮어쓰기`·`건너뛰기` 둘뿐이고 폴더 동기화는 PRD의 영구 제외 항목이다.

## Goal
전송 목적지 탭이 아이콘으로 보이고 그 탭으로 실제 전송되며, 같은 이름이 있으면 시작 전에 덮어쓸지 물어본다.

## PRD Coverage
| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-54 (신설 — 전송 대상 탭 표시·지정) | Should | T2·T3·T4 | ✅ 커버 |
| FR-55 (신설 — 같은 이름 충돌 확인) | Must | T5·T6 | ✅ 커버 |
| FR-38 (끌어다 놓기 전송 — 문구 보완) | Should | T5·T6·T7 | ✅ 커버 |
| FR-39 (원격 메뉴 — 문구 보완) | Should | T4·T7 | ✅ 커버 |
| FR-1~FR-53의 나머지 active FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope
- 이름 바꿔 저장(`report(2).zip`)·"큰 것만 덮어쓰기" 같은 조건부 결정 — 요청 버튼은 두 개다.
- 전송 큐 안에서의 충돌 재확인 — 확인은 큐에 넣기 **전에** 한 번만 한다.
- 로컬↔로컬·원격↔원격 전송 (PRD Out of Scope 그대로).

## Deferred / Follow-up
- **PRD FR-32·README의 "폴더 아이콘" 서술에 FR-54 포인터 한 절** — 대상 탭은 화살표를 다는데 그 두 곳은 폴더 아이콘만 적는다. FR-54가 같은 표에서 예외를 이미 서술하고 순서(아이콘 → 배지 → 이름 → ✕)는 그대로라 동작 서술이 틀린 것은 아니지만, FR-38·FR-39에는 포인터를 붙였으므로 관례상 여기도 붙이는 편이 낫다. **PRD 문면 변경이라 사용자 승인이 필요해 이번엔 미뤘다** (완료 검증 2라운드 m2)
- `conflict_names`는 항목 **종류를 보지 않고 이름만** 대조한다 — FR-55 문면은 "파일이면 같은 이름 파일, 폴더면 같은 이름 폴더"라 종류 일치를 함의한다. 한 폴더에 같은 이름의 파일과 폴더가 함께 있을 수 없어 실제 동작은 같지만, 문면과 구현의 표현이 다르다 (완료 검증 2라운드 m3 — 판정 유보, 결함으로 보지 않음)
- **CPU가 붐빌 때 lib 시험 1건이 간헐 실패한다** — Phase F-1의 `cargo build --release`(6분 39초)와 `cargo test`가 겹친 회차에 `725 passed; 1 failed`가 났고, 이어서 단독 실행 3회는 726건 전부 통과했다. 실패한 시험 이름은 그때 출력이 걸러져 남지 않았다. 시간 마감(2초)에 기대는 lib 시험은 `remote/manager.rs:248·306`·`remote/transfer.rs:454`·`fs/thumbnail.rs:575`뿐이고 **셋 다 이번 변경이 건드리지 않은 파일**이다(`git diff --name-only master...HEAD`로 확인) — 이번 작업이 유발한 것이 아니라 부하에 취약한 기존 마감값 문제로 본다. 재현되면 그 마감을 늘리거나 시계를 주입식으로 바꾼다
- [SUGGEST] `apply_conflict_choice`의 `Option<ConflictChoice>`가 "아직 묻기 전"과 사용자의 선택을 한 타입에 겹쳐 든다 — `enum Decision { NotAsked, Overwrite, Skip }` 3상태로 나누면 `conflicts.is_empty()`와의 결합이 사라진다 (T5 quality 리뷰 S2, 결함 아님)
- [SUGGEST] `ui/app.rs`가 3400줄을 넘겼다 — 같은 이름 확인 흐름(`start_transfer`·`drain_conflict_checks`·`show_conflict_dialog`와 세 상태 필드)은 자기 완결적이라 `ui/transfer_conflict.rs`로 뽑을 여지가 있다 (T5 quality 2라운드 S1, 결함 아님. 기존 Deferred의 `ui/app.rs` 분리 검토 항목과 같은 뿌리)
- 원격 조회 요청의 출처를 `enum ListSource`로 명시 — 이번에 조회 종류가 셋(패널 목록·트리·충돌 확인)이 되어 기준값(`1<<40`·`2<<40`)이 셋으로 늘었다. 대장에 이미 등록된 항목이며 이번엔 번호 공간만 하나 더 두기로 했다(D8, 사용자 결정).
- 충돌 확인의 **재귀 검사** — 이번엔 고른 최상위 항목 이름만 본다(D4). 폴더를 덮어쓰기로 고르면 그 안의 파일은 개별 확인 없이 덮어쓴다.
- 원격 목록의 신선도 — 올리기 충돌 확인은 대상 폴더를 **그 시점에 새로 조회**하지만, 조회와 전송 사이에 서버가 바뀌면 알 수 없다(FTP/SFTP에 변경 통지가 없다는 기존 한계와 같은 뿌리).

## Investigation Log
- 위키 참조: 관련 위키 자료 없음 — MOA는 아직 위키 미등록이고(`pending.md:60`) `20_projects/personal/`에 항목이 없다. vault 루트 `pending.md`의 FileExplorer `[DECISION]` 5건(`:16`·`:19`~`:22`)을 읽었으나 이번 변경과 충돌하는 결정은 없다. 코드 1차 출처로 진행
- Deferred 대장(`docs/plans/deferred.md`) 조회: ① 할 일 후보 — "원격 조회 요청에 출처 태그 … 요청 종류가 셋이 되면 `enum ListSource`로 바꾼다"가 이번에 임계에 닿았다(D8에서 이번 회차 미채택 결정). ② 전제 반증 — 이 plan의 전제를 부정하는 항목 없음. ③ 소진 batch — `## 대기` 40건으로 임계(100건) 미달, batch task 미생성
- 탭 그리기: `ui/tabs.rs:266-272`가 모든 탭에 `egui_phosphor::regular::FOLDER`를 고정으로 그린다. 연결 안 된 원격 탭만 `DIM_ICON_ALPHA`(0.45)로 흐리다(`:261`)
- 활성 패널 판정: `ui/splitter.rs:158`이 `i.pointer.any_pressed()` + 패널 rect 포함으로 정한다 — **우클릭도 포함**이므로 원격 메뉴를 연 패널이 곧 활성 패널이 된다
- 프레임 순서: `ui/app.rs:2319` `show_layout` → `:2360` `apply_drop` → `:2363` `apply_remote_menu`. 메뉴 실행이 그리기 뒤라 같은 프레임에 정한 대상을 그대로 쓸 수 있다
- 현재 대상 규칙: `ui/app.rs:1194` `other_panel_local` — "자기 말고 다른 패널 중 로컬인 첫 번째". 이것이 없으면(단일 패널·반대편도 원격) 받기·올리기가 **조용히 아무 일도 하지 않는다**(`:1115`·`:1134`의 `else { return }`)
- 큐 등록 지점 3곳: `app.rs:1080`(끌어다 놓은 원격 파일) · `app.rs:1813`(`TreeListed` — 원격 폴더를 훑은 결과) · `app.rs:2196`(`expand_rx` — 로컬 폴더를 펼친 결과). 셋 다 `apply_drop`이 시작점이라 `apply_drop` 앞에 게이트를 두면 전부 덮인다
- 조회 세대 번호 공간: `app.rs:2436` `TREE_LIST_BASE = 1<<40`. `Listed` 라우팅은 `app.rs:1776`에서 `pending_tree_lists` 조회 → 실패하면 패널 매칭 순서다
- 아이콘 자산: `app.rs:178`이 `egui_phosphor::add_to_fonts(.., Variant::Regular)`로 **Regular 전량**을 싣는다. `DOWNLOAD_SIMPLE`(U+E20C)·`UPLOAD_SIMPLE`(U+E4C0) 존재 확인(`egui-phosphor-0.13.0/src/variants/regular.rs:515`·`:1453`)
- 대화 본보기: `ui/remote_menu.rs:424` `show_delete_confirm` — `egui::Modal` + 18px 여백 + 30px 버튼 + 목록 5개 미리보기 + `DialogOutcome`. 같은 규격을 그대로 쓴다

### 전제 검증
| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 받기의 `덮어쓰기`는 지금 코드로 실제로 덮인다 (`.part` → 최종 이름) | 실측: `fs::rename`로 기존 파일 위에 옮겨 내용이 새것으로 바뀜을 확인(임시 프로그램 1회 실행). 호출부는 `remote/transfer.rs:301` | ✅ |
| 2 | 올리기의 `덮어쓰기`는 지금 코드로 실제로 덮인다 | 첫 전송은 `offset=0`(`remote/transfer.rs:245`)이고 그때 FTP는 `put_with_stream`(STOR — `remote/ftp.rs:337`), SFTP는 `sftp.create`(`remote/sftp.rs:277`)로 간다. 둘 다 기존 내용을 대체한다 | ✅ |
| 3 | 원격 탭은 자기 사이트·경로·연결을 스스로 들고 있어, 다른 패널에 있어도 대상이 될 수 있다 | `panel/tabs.rs:103` `TabSource::Remote { site, conn, path, phase }` | ✅ |
| 4 | 로컬 탭은 배경 탭이어도 폴더 경로를 들고 있다 | `panel/tabs.rs:102` `TabSource::Local(PathBuf)` | ✅ |
| 5 | 원격 목록 응답에 이름이 있어 이름 대조가 가능하다 | `remote/types.rs:286` `RemoteEntry::name` | ✅ |
| 6 | 패널의 로컬 선택은 **활성 탭의 것**이라, 배경 탭을 원본으로 삼으면 선택이 비어 있다 | `ui/panel.rs:564` `selected_local`이 `self.list`(패널 하나짜리 목록 뷰)에서 읽는다 | ✅ |
| 7 | `menu_rows`·`show_remote_menu`의 호출부는 프로덕션 3곳 + 테스트 4곳뿐이다 | `grep` → 프로덕션: `remote_menu.rs:75`(`show_remote_menu` 내부)·`:176`(`menu_size`)·`ui/panel.rs:1349`. 테스트 4개 함수: `remote_menu.rs:554`(`메뉴가_한_프레임을_그린다`)·`:566`(`끊긴_연결에서는…`)·`:582`·`:584`(`여럿을_고르면…`)·`:594`·`:610`(`고른_것이_없어도…`) | ✅ |
| 8 | 연결이 끊기면 **모든 줄이 비활성**이라는 기존 규칙이 있다 — 새 `can_*` 인자가 이것을 뒤집으면 안 된다 | `remote_menu.rs:561-570` 테스트 `끊긴_연결에서는_서버에_닿는_줄이_모두_비활성이다`가 `menu_rows(selected, false)` 전 줄에 대해 단언 | ✅ |
| 9 | 팝업이 옆 패널 위로 뻗치면 그 위의 클릭으로 **아래 깔린 패널이 활성**이 된다 | `ui/splitter.rs:158-171`(포인터 좌표만 본다) + `ui/app.rs:2571-2573` 기존 테스트 주석이 그 현상을 명시(이전 plan의 결정 D16 — 이 plan에는 없다. 코드 주석 `ui/app.rs:2348`·`:2571-2573`에 남아 있다) | ✅ |
| 10 | `Ui::rect_contains_pointer`는 **레이어 가림을 존중**한다 — 팝업에 덮인 패널은 거짓이 된다 | `egui-0.35.0/src/ui.rs:997-1006` 문서 주석("if this Ui is behind some other window, this will always return false") + `context.rs:3030-3036`("Will return false if some other area is covering the given layer") | ✅ |
| 11 | 앱에는 이미 **전송 방향 글리프·색 관례**가 있다 | `ui/queue_panel.rs:58-59`(`ARROW_UP`/`ARROW_DOWN`)·`:169-176`(`direction_mark` — Upload=`ACCENT`, Download=`OK_TEXT`)·`:554-557` 규약 테스트 | ✅ |
| 12 | `ExplorerApp`은 단위 테스트에서 만들 수 없다 | 유일 생성자 `ui/app.rs:449`가 `cc: &eframe::CreationContext<'_>`를 받고, 레포에서 이것을 부르는 곳은 `main.rs` 한 곳뿐이다. `app.rs`의 `mod tests`(`:2560~`)는 `WorkspaceView`와 순수 함수만 시험한다 | ✅ |

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| 대상 탭이 sticky라 사용자가 예상 못 한 패널로 전송될 수 있다 | 엉뚱한 폴더에 파일이 쌓인다 | 대상 탭에 아이콘을 그려 **보이는 곳이 곧 가는 곳**이게 한다(요구의 핵심). 대상이 없으면 메뉴를 비활성 |
| 올리기 충돌 확인이 서버 조회를 한 번 더 한다 | 느린 서버에서 대화가 늦게 뜬다 | 조회는 **대상 폴더 한 겹**뿐이다(재귀 없음 — D4). 조회 중에는 큐에 아무것도 넣지 않아 중간 상태가 없다 |
| 조회 응답이 영영 안 오면 전송이 시작되지 않는다 | 사용자가 기다리다 만다 | `ListFailed`를 함께 받아 확인을 포기하고 **충돌 없음으로 보고 진행**한다(전송을 막지 않는다). 사유는 서버 로그에 남는다 |
| 파일 이름 대소문자 규칙이 로컬(무시)과 원격(구분)이 다르다 | 한쪽에서 충돌을 놓치거나 헛경고 | 받기는 대소문자 무시, 올리기는 구분으로 각각 그 쪽 규칙을 따른다(D5) |
| 팝업이 옆 패널 위로 뻗친 자리를 누르면 아래 깔린 패널이 활성이 된다(기존 현상 — 전제 검증 #9) | 이번 계획이 그 값을 전송 목적지로 승격시키므로 대상이 엉뚱한 패널로 옮겨간다 | `active`를 쓰지 않고 **레이어 가림을 존중하는** 패널 자체의 클릭 신호로 갱신한다(D11). 기존 `active` 로직은 손대지 않아 회귀 범위가 없다 |
| `ExplorerApp`이 단위 테스트에서 생성 불가라 배선 검증이 자동화되지 않는다 | 판정은 맞는데 배선이 틀려도 테스트가 통과한다 | 판정을 전부 `WorkspaceView`·순수 함수로 내려 테스트로 덮고(D12), 배선은 Verification Strategy의 수동 절차 5단계로 확인한다 |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `TabState` (필드 추가) | `panel/tabs.rs` | 정의 변경. 생성자 `new`·`remote`가 id를 스스로 만들어 **호출부 시그니처 불변** (호출부: `panel/panel.rs:180·321·866`, `ui/panel.rs:225·263·264·604`, `ui/panel/tests.rs` 11곳) |
| `TabsModel` (조회 추가) | `panel/tabs.rs` | `tabs()`·`active_id()` 추가. 기존 `sources()`는 그대로 둔다 |
| `tabs::show_tab_strip` | `ui/tabs.rs`(정의), `ui/panel.rs:1052`(유일 호출부) | 인자 1개 추가 |
| `tabs::show_tab` (비공개) | `ui/tabs.rs` | 아이콘 인자 추가 |
| `PanelState::show` | `ui/panel.rs:1043`(정의), `ui/splitter.rs:185`(프로덕션 호출부), `ui/panel/tests.rs:89`(테스트 헬퍼 `draw_once`) | 인자 1개 추가 (7 → 8 — `#[allow(clippy::too_many_arguments)]` 필요). **테스트 헬퍼도 함께 고친다** |
| `splitter::show_layout` | `ui/splitter.rs:137`(정의), `ui/app.rs:2319`(유일 호출부) | 인자 1개 추가 (이미 `allow` 붙어 있음) |
| `remote_menu::menu_rows` | `ui/remote_menu.rs:101`(정의), `:75`(`show_remote_menu`), `:176`(`menu_size`), 테스트 4개 함수(`:554`·`:566`·`:582`/`:584`·`:594`/`:610`) | 인자 2개 추가. `menu_size`는 줄 **수**만 세므로 더미 값(`false, false`)을 넘겨도 되나, 비활성 줄도 그려지므로 줄 수는 불변임을 확인한다 |
| `remote_menu::show_remote_menu` | `ui/remote_menu.rs:67`(정의), `ui/panel.rs:1349`(호출부), `remote_menu.rs:74`(내부 사용), 테스트 `ui/remote_menu.rs:554` | 인자 2개 추가 |
| `PanelState::show_remote_menu` (비공개 래퍼) | `ui/panel.rs:1328`(정의), `:1277`(유일 호출부) | 인자 2개 추가 — 패널은 대상 판정을 모르므로 `TransferTargets`에서 받아 그대로 넘긴다 |
| `ExplorerApp::other_panel_local` / `LocalSide` | `ui/app.rs:1194`·`:2522` | **삭제** — 대상 판정이 `TransferTargets`로 옮겨간다. 호출부는 `:1115`·`:1134` 둘뿐 |
| `ExplorerApp::apply_drop` | `ui/app.rs:1026`(정의), `:1126`·`:1146`·`:2361`(호출부 3곳) | 앞에 충돌 확인 게이트가 붙는다 — 시그니처는 그대로, 호출부가 게이트를 거치게 바뀐다 |
| `ConnEvent::Listed` 라우팅 | `ui/app.rs:1770` | 분기 1개 추가 (충돌 확인용 조회) |
| `ConnEvent::ListFailed` 라우팅 | `ui/app.rs:1835` | 분기 1개 추가 (확인 포기 → 진행) |
| `WorkspaceView` | `ui/app.rs:199` | 필드 2개 추가 + 조회 4개(`note_pressed`·`transfer_targets`·`download_dir`·`upload_dir`·`upload_source`). `new`·`from_state` 2곳이 초기화에 걸린다(`to_state`는 세션에 담지 않으므로 불변 — D7) |
| `LayoutOutcome` | `ui/splitter.rs`(정의), `ui/app.rs:2319`(소비) | 필드 `pressed_panel: Option<PanelId>` 추가 (D11) |
| `queue_panel::{UPLOAD_GLYPH, DOWNLOAD_GLYPH, direction_mark}` | `ui/queue_panel.rs:58-59`·`:169-176`(정의 — **`ui/widgets.rs`로 이동**), `:699` 부근(그리기 호출부), `:554-557`(규약 테스트) | 모듈 이동. 시그니처 불변, import만 바뀐다 (D9) |

### 4-B. 계약·직렬화 변경
- **세션 스키마 변경 없음** — 대상 탭은 저장하지 않는다(D7). `settings.json` v3 그대로.
- **`ConnCommand`/`ConnEvent` 변경 없음** — 충돌 확인은 기존 `List`를 새 세대 번호 공간(`CONFLICT_LIST_BASE = 2<<40`)으로 보낸다.
- **공개 API 시그니처 변경 5건** (위 표) — 전부 crate 내부이며 호출부를 모두 특정했다.

### 4-C. 테스트 파일
- `src/panel/tabs.rs`의 `mod tests` — `TabId` 유일성
- `src/ui/tabs.rs`의 `mod tests` — 아이콘 고르기 규칙
- `src/ui/remote_menu.rs`의 `mod tests` — `menu_rows` 활성 규칙 (기존 2건 갱신 필요: `여럿을_고르면_이름_바꾸기가_비활성이다`·`고른_것이_없어도_할_수_있는_일은_남는다`가 인자 2개 추가로 컴파일 실패)
- `src/ui/app.rs`의 `mod tests` — `WorkspaceView`의 대상 판정(sticky·폴백·혼합 패널·팝업 가림)과 목적지 조회, 순수 함수 `conflict_names`·`apply_conflict_choice`. **`ExplorerApp`은 이 파일 테스트에서 만들 수 없다**(전제 검증 #12) — 배선은 HUMAN-VERIFY (D12)
- `src/ui/queue_panel.rs`의 규약 테스트(`:554-557`) — `direction_mark`·글리프 이동에 맞춘 경로 수정
- `src/i18n/mod.rs`의 `mod tests` — 새 문구 6개 + `conflict_count`를 원문 리터럴로 단언(`LanguageGuard::lock` 사용)
- `src/ui/panel/tests.rs` — 헬퍼 `draw_once`(`:76`)가 `panel.show(..)`를 부른다(`:89`). 인자 추가 필요 (grep으로 확인 완료 — 이 파일에서 `show`를 부르는 곳은 이 헬퍼 하나뿐이라 한 곳만 고치면 파일 전체가 따라온다)

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `TabId(u64)` | `PanelId`·`SiteId`·`ConnectionId`·`TransferId` 모두 같은 뉴타입 패턴 | 신규 — 식별 대상이 다르다. **같은 관례**(뉴타입 + 전역/모델 카운터)를 따른다 |
| `TransferTargets` | `DisplayRules`(`ui/panel.rs:217`)·`RemoteView`·`PanelMenuState` — 앱이 정해 패널로 내려보내는 번들 3종 | 신규 — 같은 관례의 네 번째. 기존 셋에 얹으면 그 타입의 뜻이 흐려진다 |
| `remote_menu::show_conflict_dialog` | `show_delete_confirm`(`remote_menu.rs:424`) | 신규 함수, **모양은 재사용** — 버튼이 3개고 내용이 달라 인자화보다 별도 함수가 짧다. 여백·버튼 크기 상수(`DIALOG_*`)는 그대로 쓴다 |
| `ConflictCheck` (보류 상태) | 유사 없음(`pending_trees`·`pending_tree_lists`가 비슷한 자리이나 담는 것이 다르다) | 신규 |
| 전송 방향 글리프·색 | **있다** — `ui/queue_panel.rs:58-59` `UPLOAD_GLYPH`(`ARROW_UP`)·`DOWNLOAD_GLYPH`(`ARROW_DOWN`), `:169-176` `direction_mark`(Upload=`ACCENT`, Download=`OK_TEXT`), `:554-557` 규약 테스트 | **재사용** — 셋을 `ui/widgets.rs`로 옮기고 큐 화면과 탭 스트립이 함께 쓴다. 새 글리프를 정하면 같은 개념의 시각 언어가 두 벌이 된다 (D9) |
| `conflict_names` / `apply_conflict_choice` (순수 함수) | 유사 없음 — 기존 충돌 판정 코드가 없다 | 신규. `ExplorerApp` 밖 자유 함수로 두어 단위 테스트가 가능하게 한다 (D12) |
| `LayoutOutcome::pressed_panel` | 유사 — `LayoutOutcome`이 이미 `menu`·`command`·`drop` 등을 같은 방식으로 올려 보낸다 | 기존 타입에 필드 추가(신규 타입 아님) |
| 로컬 존재 확인 워커 스레드 | `apply_drop`의 폴더 펼치기 스레드(`app.rs:1049`) | **패턴 재사용** — `std::thread::spawn` + `mpsc` + `wake()` 그대로 |

### Verified by
- `grep -rn "show_tab_strip\|show_layout\|other_panel_local\|apply_drop\|menu_rows\|show_remote_menu\|menu_size" src/ tests/` → 위 표에 전건 등재
- `grep -rn "panel.show(\|\.show(ui, ctx" src/` → `PanelState::show` 호출부 2곳(프로덕션 1 + 테스트 헬퍼 1) 확인, 표에 등재
- `grep -rn "TabState::new\|TabState::remote" src/ tests/` → 18곳, 전부 생성자 호출이라 필드 추가로 깨지지 않음을 확인
- `grep -rn "queue.enqueue" src/` → 27곳 중 프로덕션 3곳(`app.rs:1080`·`:1813`·`:2196`)과 세션 복원 1곳(`session.rs:223`), 나머지는 테스트. 시그니처는 바꾸지 않으므로 영향 없음

## 동반 변경 판정
| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | PRD FR 신설 2건 + FR-38·FR-39 문구 보완 | ① 축 — PRD가 이 동작의 정본인데 FR-39는 받기·올리기를 아예 적지 않고 FR-38은 충돌 처리를 모른다. 이대로 두면 승인된 요구 문서가 실제와 어긋난다 | T7에 편입 (사용자 승인) |
| 필수 | `README.md` §전송 큐·§원격 파일 작업 갱신 | ① 축 — 두 줄이 현재 동작을 서술하고 있어 대상 규칙·확인 대화가 빠지면 문서가 틀린 설명이 된다 | T7에 편입 |
| 필수 | `remote_menu.rs` 기존 테스트 **4건** 갱신 | ③ 축 — `menu_rows`·`show_remote_menu` 인자가 늘어 그대로면 컴파일되지 않는다. 그중 `끊긴_연결에서는…`은 새 인자가 기존 규칙을 뒤집지 않는지 보는 **규칙 검증**이라 기계적 인자 추가로 끝나지 않는다 | T4에 편입 |
| 필수 | `ui/panel/tests.rs`의 헬퍼 `draw_once`(`:76`) 갱신 | ③ 축 — `PanelState::show` 인자가 늘어 그대로면 컴파일되지 않는다 | T3에 편입 |
| 필수 | 전송 방향 글리프·색을 `ui/widgets.rs`로 옮겨 큐 화면과 탭이 함께 쓰기 | ② 축 — `queue_panel.rs:58-59`·`:169-176`이 이미 "전송 방향"의 시각 언어(`ARROW_UP`+`ACCENT` / `ARROW_DOWN`+`OK_TEXT`)를 정본으로 갖고 있다. 탭에 다른 글리프를 새로 정하면 같은 개념이 두 벌이 된다 | T3에 편입 (D9) |
| 선택 | 끌어다 놓기 전송에도 충돌 확인 적용 | 같은 `apply_drop` 경로라 게이트 하나로 덮인다. 안 하면 "끌면 말없이 덮어쓰고 메뉴로는 물어보는" 엇갈림이 남는다 | **채택** (사용자 결정) → T5·T6에 포함 |
| 선택 | 조회 출처를 `enum ListSource`로 리팩터 | 기준값이 셋으로 늘어 대장의 임계에 닿았다. 안 해도 이번 기능은 성립한다 | **미채택** (사용자 결정) → Deferred 유지 |
| 무관 | 세션 스키마(`ui/session.rs`)·전송 큐 모델(`remote/queue.rs`)·연결 워커(`remote/connection.rs`) | 대상 탭은 저장하지 않고(D7) 큐에 넣는 값의 모양도 그대로다. 충돌 확인은 기존 `List` 명령을 쓴다 | 건드리지 않음 |
| 무관 | `docs/design/README.md` (원격 UI 기준 문서) | 탭 전송 아이콘·충돌 대화는 그 문서에 없는 신규 요소다(디자인 미제공 — 기존 `받기`·`올리기` 메뉴 문구도 "구현이 정한 신규 문구"로 테스트에 명시돼 있다) | 건드리지 않음 |

## Decisions

### D1. "마지막에 선택한 탭"의 판정
- **Options**: A) 마지막으로 누른 패널의 활성 탭 / B) 탭 스트립을 직접 누른 탭만
- **Chosen**: A (사용자 결정)
- **Rationale**: `splitter.rs:158`이 이미 "마지막으로 누른 패널"을 추적한다. B는 앱을 켠 직후·세션 복원 직후 대상이 없어 받기·올리기가 무반응이 된다.
- **Source**: `ui/splitter.rs:150-172`

### D2. 대상은 **탭 단위로 sticky**하다
- **Options**: A) `TabId`로 탭을 기억 / B) `PanelId`만 기억하고 그 패널의 활성 탭을 그때그때 읽기
- **Chosen**: A
- **Rationale**: B는 한 패널에 로컬 탭과 원격 탭이 섞였을 때(단일 패널 FTP 사용) 원격 탭을 보는 순간 받기 대상이 사라져 **받기가 영영 비활성**이 된다. A는 로컬 탭이 배경으로 밀려도 그 폴더로 계속 받을 수 있다.
- **Source**: 혼합 패널이 실제로 생긴다 — `ui/app.rs:2089` `open_remote_tab`(분할 자리가 없어 현재 패널에 원격 탭을 더 여는 길)

### D3. 올리기의 **원본**은 받기 아이콘이 붙은 탭의 선택
- **Options**: A) 받기 아이콘 탭의 선택 / B) 지금처럼 반대편 패널의 로컬 선택
- **Chosen**: A (사용자 결정)
- **Rationale**: 두 아이콘이 "여기서 저기로"를 화면에 그대로 보인다. 다만 그 탭이 자기 패널에서 배경 탭이면 선택이 비어 있으므로(전제 검증 #6) 그때는 올리기를 비활성으로 둔다.
- **Source**: `ui/panel.rs:564` `selected_local`

### D4. 충돌 판정 단위는 **고른 최상위 항목의 이름**
- **Options**: A) 최상위 항목 이름 / B) 전송될 모든 파일(재귀)
- **Chosen**: A (사용자 결정)
- **Rationale**: 대상 폴더를 한 번만 읽으면 되어 대화가 곧바로 뜬다. 폴더는 통째로 덮어쓰기/건너뛰기가 되고, 파일은 그 파일만 대상이 된다. B는 올리기에서 하위 폴더마다 서버 조회가 필요해 큰 폴더에서 대화가 한참 뒤에 뜬다.

### D5. 이름 비교 규칙은 **양쪽 파일시스템의 규칙**을 따른다
- **Options**: A) 받기는 대소문자 무시·올리기는 구분 / B) 양쪽 다 무시 / C) 양쪽 다 구분
- **Chosen**: A
- **Rationale**: 받는 곳은 Windows(대소문자 무시)라 `A.TXT`가 있으면 `a.txt`도 실제로 덮인다 — 구분하면 경고를 놓친다. 올리는 곳은 대개 POSIX(구분)라 무시하면 헛경고가 난다.
- **Source**: `remote/types.rs`의 `RemotePath`는 서버 경로를 그대로 든다(정규화하지 않는다)

### D6. 확인 전에는 **아무것도 큐에 넣지 않는다**
- **Options**: A) 전량 보류 후 결정에 따라 처리 / B) 충돌 없는 것부터 먼저 큐에
- **Chosen**: A
- **Rationale**: 사용자가 고른 `취소`가 "이번 전송을 하지 않음"이 되려면 그 시점에 아직 아무것도 시작하지 않았어야 한다. B는 취소가 절반만 취소된다.

### D7. 대상 탭을 **세션에 저장하지 않는다**
- **Options**: A) 저장 안 함 / B) `settings.json`에 담아 복원
- **Chosen**: A
- **Rationale**: 스키마 v4가 필요해지는 대가에 비해 얻는 것이 작다. 복원 직후에는 "활성 패널의 활성 탭 + 폴백" 규칙이 합리적인 값을 곧바로 준다.
- **Source**: `ui/session.rs`의 v3 스키마 — 대상 탭에 대응하는 자리가 없다

### D8. 조회 세대는 **기준값을 하나 더** 둔다
- **Options**: A) `CONFLICT_LIST_BASE = 2<<40` 추가 / B) `enum ListSource`로 리팩터
- **Chosen**: A (사용자 결정)
- **Rationale**: 이번 기능에 집중한다. B는 연결·패널·트리 캐시까지 닿는 리팩터라 회귀 범위가 넓다. Deferred 대장에 그대로 남긴다.
- **Source**: `ui/app.rs:2436` `TREE_LIST_BASE`

### D9. 대상 아이콘은 **큐 화면의 방향 관례를 그대로 재사용**한다
- **Options**: A) 기존 `direction_mark`를 `ui/widgets.rs`로 옮겨 큐 화면·탭이 공유 / B) 탭 전용으로 `DOWNLOAD_SIMPLE`·`UPLOAD_SIMPLE` + `ACCENT`를 새로 정함
- **Chosen**: A
- **Rationale**: 앱은 이미 "전송 방향"의 시각 언어를 갖고 있다 — 올리기 `ARROW_UP`+`theme::ACCENT`, 받기 `ARROW_DOWN`+`theme::OK_TEXT`. B로 가면 큐 화면과 탭이 같은 개념을 다른 글리프·색으로 말하게 된다(4-D 재사용 원칙). 두 색 모두 폴더의 노랑(`FOLDER_ICON`)과 뚜렷이 달라 대상이 한눈에 띈다는 목적도 그대로 달성된다. 정렬 표시는 `CARET_UP`/`CARET_DOWN`이라 화살표와 혼동되지 않는다.
- **Rationale (부수)**: 연결 안 된 원격 탭을 흐리게 그리는 기존 규칙(`DIM_ICON_ALPHA`)은 대상 아이콘에도 그대로 적용한다.
- **Source**: `ui/queue_panel.rs:58-59`·`:169-176`·`:554-557`, `ui/theme.rs:38`·`:49`·`:53`, `ui/tabs.rs:260-272`

### D11. 대상 갱신은 **팝업에 가려지지 않은 패널 클릭**으로만 한다
- **Options**: A) 패널이 자기 레이어에서 `rect_contains_pointer` + `any_pressed`로 판정해 올려 보낸 신호를 쓴다 / B) `splitter`가 정하는 `active`를 그대로 쓴다 / C) `splitter`의 `active` 판정 자체를 고친다
- **Chosen**: A
- **Rationale**: B는 **알려진 현상에 걸린다** — 패널 메뉴·원격 메뉴 팝업은 자기 패널 밖으로 뻗을 수 있고 그 위에서 고르면 아래 깔린 패널이 활성이 된다(전제 검증 #9). 지금은 그것이 활성 테두리만 옮기지만, 이 계획은 그 값을 **전송 목적지**로 승격시키므로 사용자가 누른 적 없는 패널로 대상이 옮겨간다. C는 기존 `active` 동작을 바꿔 회귀 범위가 넓다(**이전 plan의 결정 D16** — 이 plan의 D 번호가 아니다 — 이 그 현상을 전제로 명령의 대상을 명시 전달하게 만들어 두었다. 코드 주석 `ui/app.rs:2348`). A는 `Ui::rect_contains_pointer`가 **레이어 가림을 존중**하므로(전제 검증 #10) 팝업 위 클릭이 걸러지고, 기존 `active` 로직은 손대지 않는다.
- **구현**: `splitter::show_layout`의 패널 그리기 스코프 안에서 `ui.rect_contains_pointer(pane) && ui.input(|i| i.pointer.any_pressed())`를 판정해 `LayoutOutcome::pressed_panel: Option<PanelId>`로 올린다. 앱은 그 값이 `Some`일 때만 sticky 대상을 갱신한다.
- **Source**: `egui-0.35.0/src/ui.rs:997-1006`, `ui/app.rs:2568-2571`(현상 명시), `ui/splitter.rs:158-171`

### D12. 검증 가능한 자리를 계획 단계에서 확정한다 (`ExplorerApp` 밖으로)
- **Options**: A) 판정 로직을 `WorkspaceView` 메서드와 자유 함수로 두고 그것을 단위 테스트 / B) `ExplorerApp`에 테스트 전용 생성자를 만든다
- **Chosen**: A
- **Rationale**: `ExplorerApp`은 단위 테스트에서 만들 수 없다 — 유일 생성자가 `eframe::CreationContext`를 받고 부르는 곳은 `main.rs`뿐이다(전제 검증 #12). B는 COM·글꼴·트레이·GL 컨텍스트까지 흉내 내야 해 비용이 크다. A로 두면 **판정은 전부 단위 테스트**로 덮이고 `ExplorerApp`에는 "판정 결과를 그대로 따르는 배선"만 남는다.
- **테스트 대상 (단위)**: `WorkspaceView`의 대상 판정·목적지 조회(기존 `app.rs` 테스트가 이미 `WorkspaceView`를 만들어 쓴다), `remote_menu::menu_rows`의 활성 규칙, `conflict_names`·`apply_conflict_choice` 순수 함수, `tabs`의 아이콘 고르기.
- **HUMAN-VERIFY (배선)**: 판정 결과가 실제로 큐 등록·대화 표시로 이어지는지 — Verification Strategy의 수동 절차가 덮는다.
- **Source**: `ui/app.rs:449`(생성자), `:2560~`(기존 테스트가 `WorkspaceView`만 만든다)

### D10. 확인 조회가 실패하면 **막지 않고 진행**한다
- **Options**: A) 충돌 없음으로 보고 전송 시작 / B) 전송을 취소하고 오류 표시
- **Chosen**: A
- **Rationale**: 확인은 안전장치이지 관문이 아니다. B는 조회가 안 되는 서버에서 전송 자체를 못 하게 만든다. 실패 사유는 기존 경로대로 서버 로그에 남는다.

## Tasks

<!-- T1~T4 (대상 판정과 표시) / T5~T6 (충돌 확인) / T7 (문서) -->

- [x] T1. 탭에 안정된 식별자(`TabId`) 부여
  - **Type**: C
  - **Design**: ① `src/panel/tabs.rs`(탭 순수 모델이 사는 곳)에 둔다. ② 신규 심볼 — `TabId(u64)`: 탭 하나를 앱이 사는 동안 유일하게 가리키는 값 / `TabState.id`: 그 값을 드는 자리 / `TabsModel::tabs()`: 인덱스와 id를 함께 보려는 화면에 슬라이스를 준다 / `TabsModel::active_id()`. ③ `TabState::new`·`TabState::remote`가 모듈 안의 `AtomicU64`에서 다음 값을 받아 스스로 채운다 — 호출부는 아무것도 넘기지 않는다(호출부 18곳 불변). ④ 비추상화 선언 — id 발급기를 주입 가능한 트레이트로 만들지 않는다(테스트도 "서로 다르다"만 보면 된다). 세션 저장 대상이 아니므로 직렬화도 붙이지 않는다.
  - **Acceptance**: Given 탭을 세 개 만들고 하나를 닫은 뒤 두 개를 더 만든 상태, When 각 탭의 `id`를 모으면, Then 모두 서로 다르다(닫힌 탭의 값이 재사용되지 않는다). 그리고 `TabsModel::active_id()`가 `tabs()[active_index()].id`와 같다.
  - **Files**:
    - 주: `src/panel/tabs.rs`
    - 테스트: `src/panel/tabs.rs`의 `mod tests`
  - **Edge Cases**:
    - 세션 복원(`TabsModel::from_tabs`)으로 만든 탭들도 서로 다른 id를 갖는다 — `TabState`를 만드는 길이 생성자 둘뿐이므로 자동으로 성립하는지 테스트로 확인한다
    - `panel/panel.rs`(구 Win32 판)의 `TabState::new` 호출도 그대로 컴파일되는지 — 이 파일은 실행에 쓰이지 않지만 빌드 대상이다
  - **Halt Forecast**:
    - (i) `TabState`에 `#[derive]`가 붙어 있어 필드 추가가 막히는 경우 → 현재 파생은 없다(구조체 정의 `panel/tabs.rs:31-34`에서 확인). 문제 없음
  - **Depends on**: -

- [x] T2. 전송 대상 판정 (`TransferTargets`)
  - **Type**: C
  - **Design**: ① 판정은 `src/ui/app.rs`에 둔다(워크스페이스가 패널을 소유한다). **다만 `TransferTargets` 값 타입의 정의는 `src/ui/tabs.rs`에 둔다**(구현 중 정정 — `ui/tabs.rs`가 그리기에 쓰므로 `ui/app.rs`에 두면 `ui::tabs` → `ui::app` 상향 의존이 생긴다. 아래로 흐르는 값은 아래에 둔다). ② 신규 심볼 — `TransferTargets { download: Option<TabId>, upload: Option<TabId>, can_download: bool, can_upload: bool }`(앱이 정해 패널·탭 스트립에 내려보내는 값, `Copy`) / `WorkspaceView::{last_local_tab, last_remote_tab}`(sticky 상태) / `WorkspaceView::note_pressed(&mut self, panel: PanelId)`(D11의 신호를 받아 sticky를 옮긴다) / `WorkspaceView::transfer_targets(&mut self) -> TransferTargets`(사라진 탭 폴백을 정리하고 값을 낸다) / **목적지 조회 3개** — `download_dir(&self) -> Option<PathBuf>`, `upload_dir(&self) -> Option<(SiteId, RemotePath)>`, `upload_source(&self) -> Vec<(PathBuf, bool)>`. ③ `WorkspaceView`가 `PanelState`를 읽는다. `ui`·`panel`은 `TransferTargets`를 받기만 하고 만들지 않는다. ④ 비추상화 선언 — 대상 판정을 트레이트나 별도 모듈로 빼지 않는다(규칙이 짧고 `WorkspaceView` 밖에서 쓰이지 않는다).
  - **판정을 `WorkspaceView`에 두는 이유**: `ExplorerApp`은 단위 테스트에서 만들 수 없고 `WorkspaceView`는 만들 수 있다 — 목적지 조회까지 여기 둬야 acceptance가 테스트로 판정된다 (D12).
  - **갱신 규칙**:
    1. `note_pressed(panel)` — 그 패널의 활성 탭이 로컬이면 `last_local_tab`을, 원격이면 `last_remote_tab`을 그 탭의 id로 바꾼다. **호출은 `LayoutOutcome::pressed_panel`이 `Some`일 때만**이다(D11 — 팝업에 가려진 클릭은 신호가 오지 않는다). **호출 지점은 `show_layout` 반환 직후, `apply_drop`(`app.rs:2361`)·`apply_remote_menu`(`:2363`) 앞**이다 — 같은 프레임의 우클릭이 대상에 곧바로 반영되어 "메뉴를 연 패널이 곧 올리기 대상"이 성립한다.
    2. `transfer_targets()` — `last_local_tab`이 어느 패널에도 없는 id를 가리키면 패널을 `layout.panel_ids()` 순서로 훑어 **활성 탭이 로컬인 첫 패널**의 탭 id로 되돌린다(그런 패널이 없으면 `None`). `last_remote_tab`도 같은 규칙(원격 탭 기준).
  - **`can_*` 판정**: `can_download` = `download`가 가리키는 로컬 탭이 있다. `can_upload` = `upload`가 가리키는 **연결된**(`conn`이 `Some`) 원격 탭이 있고 **그리고** `upload_source()`가 비어 있지 않다(= `download` 탭이 자기 패널의 활성 탭이면서 선택이 있다 — D3).
  - **Acceptance**: Given 패널 A(활성 탭이 로컬)와 패널 B(활성 탭이 원격), When `note_pressed(A)` 뒤 `note_pressed(B)`를 부르면, Then `download`는 A의 탭 id를, `upload`는 B의 탭 id를 가리키고 `download_dir()`는 A의 탭 폴더를 낸다. / Given 패널 하나에 로컬 탭 L·원격 탭 R이 있고 L을 보다 R로 전환한 뒤 `note_pressed`를 부른 상태, Then `download`는 여전히 L을 가리킨다(sticky). / Given `download`가 가리키던 탭을 닫은 상태, When `transfer_targets()`를 부르면, Then **활성 탭이 로컬인 패널**이 있으면 그 탭으로 옮겨가고 그런 패널이 하나도 없으면 `None`이다. / Given `download` 탭이 자기 패널의 배경 탭인 상태, Then `upload_source()`는 비어 있고 `can_upload`는 거짓이다.
  - **Files**:
    - 주: `src/ui/app.rs`
    - 동반: `src/ui/panel.rs` (조회 추가 — ⓐ 활성 탭의 id·종류 ⓑ **`TabId`로 그 탭의 `TabSource`를 찾는 조회**(배경 탭이 대상일 수 있다 — D2) ⓒ 그 패널이 그 `TabId`를 갖고 있는지. `tabs` 필드가 비공개라 위임 조회가 필요하다), `src/ui/splitter.rs` (`LayoutOutcome::pressed_panel` 산출 — D11)
    - 테스트: `src/ui/app.rs`의 `mod tests` (`WorkspaceView`를 직접 만들어 시험 — `:2560~`에 선례가 있다)
  - **Edge Cases**:
    - 워크스페이스를 전환하면 그 워크스페이스의 값이 따로 쓰인다 — 상태를 `WorkspaceView`에 두는 것으로 자동 성립
    - 패널이 하나뿐이고 로컬 탭만 있으면 `upload`는 `None`, `can_upload`는 거짓
    - 아직 뷰가 만들어지지 않은 워크스페이스는 갱신 대상이 아니다(`ensure_active_view` 뒤에 부른다)
    - 팝업·모달이 옆 패널 위로 뻗친 상태에서 그 위를 눌러도 대상이 옮겨가지 않는다 (D11 — 테스트는 `pressed_panel`이 `None`이면 sticky가 그대로임을 본다)
  - **Halt Forecast**:
    - (i) `PanelState`가 활성 탭의 id를 내주는 조회가 없다 → T1이 `TabsModel::active_id()`를 만들고 이 task가 `PanelState`에 얇은 위임 조회를 더한다
    - (ii-a) `LayoutOutcome`에 필드 추가 → `## 사전 승인 항목`에 등록
  - **Depends on**: T1

- [x] T3. 대상 탭에 받기·올리기 아이콘 그리기
  - **Type**: C
  - **Design**: ① 방향 글리프·색은 `src/ui/widgets.rs`(두 화면이 함께 쓰는 자리)로 옮기고, 아이콘 고르기는 `src/ui/tabs.rs`에 둔다. ② 신규 심볼 — `widgets::direction_mark(TransferDirection) -> (&'static str, egui::Color32)`(`queue_panel`에서 **이동**, 상수 `UPLOAD_GLYPH`·`DOWNLOAD_GLYPH`도 함께) / `tabs::tab_icon(id, targets) -> (&'static str, egui::Color32)`: 그 탭이 대상이면 `direction_mark`의 값을, 아니면 `FOLDER` + `theme::FOLDER_ICON`을 고르는 순수 함수(테스트 대상). ③ `ui/tabs.rs`·`ui/queue_panel.rs`가 `ui/widgets.rs`를 참조한다(역방향 없음). `ui/tabs.rs`는 `TransferTargets`를 값으로 받는다. ④ 비추상화 선언 — 아이콘 규칙을 테마 계층으로 올리거나 트레이트로 만들지 않는다. `direction_mark`는 **이미 있는 함수를 옮기기만** 하고 시그니처를 바꾸지 않는다.
  - **배선**: `app.rs`에서 `show_layout(.., targets)` → `splitter.rs`에서 `panel.show(.., targets)` → `panel.rs`에서 `show_tab_strip(.., targets)`. `PanelState::show`는 인자가 8개가 되므로 `#[allow(clippy::too_many_arguments)]`를 붙인다(같은 이유로 이미 `show_layout`에 붙어 있다). `ui/panel/tests.rs`의 헬퍼 `draw_once`(`:76`)도 같은 인자를 넘기게 고친다.
  - **Acceptance**: Given 받기 대상으로 판정된 로컬 탭, When `tab_icon`을 부르면, Then `widgets::direction_mark(Download)`와 같은 값(`ARROW_DOWN` + `theme::OK_TEXT`)이다. / Given 올리기 대상 원격 탭, Then `direction_mark(Upload)`와 같은 값(`ARROW_UP` + `theme::ACCENT`)이다. / Given 그 밖의 모든 탭, Then `FOLDER` + `theme::FOLDER_ICON`이다. / Given 연결되지 않은 원격 대상 탭, Then 글리프는 `ARROW_UP`이되 기존 규칙대로 흐리다(`DIM_ICON_ALPHA`). / `cargo test`에서 `queue_panel`의 기존 규약 테스트(`is_icon_font(UPLOAD_GLYPH)` 등)가 이동 후에도 통과한다.
  - **Files**:
    - 주: `src/ui/tabs.rs`, `src/ui/widgets.rs`
    - 동반: `src/ui/queue_panel.rs`(글리프·`direction_mark` 이동에 따른 import), `src/ui/panel.rs`, `src/ui/splitter.rs`, `src/ui/app.rs`
    - 테스트: `src/ui/tabs.rs`의 `mod tests`, `src/ui/queue_panel.rs`의 기존 규약 테스트(이동에 맞춘 경로 수정), `src/ui/panel/tests.rs`(헬퍼 `draw_once`의 인자)
  - **Edge Cases**:
    - 아이콘 폭(`TAB_ICON_WIDTH` 16px)은 그대로 — 화살표가 폴더보다 좁아 배치가 흔들리지 않는지 기존 배치 테스트로 확인
    - 같은 패널에 받기 대상과 올리기 대상이 함께 있을 수 있다(혼합 패널) — 두 아이콘이 동시에 보인다
    - 정렬 표시(`CARET_UP`/`CARET_DOWN`)와 전송 방향(`ARROW_UP`/`ARROW_DOWN`)이 섞이지 않는지 확인 — 이미 다른 글리프다
    - AGENTS 규약: 아이콘은 `egui_phosphor::regular::*`에서만 가져온다(유니코드 기호 직접 사용 금지 — `is_icon_font` 테스트가 지킨다)
  - **Halt Forecast**:
    - (ii-a) `PanelState::show`·`show_layout`·`show_tab_strip` 세 함수의 시그니처 변경 + `direction_mark`·글리프 상수의 모듈 이동 → `## 사전 승인 항목`에 등록
  - **Depends on**: T2

- [x] T4. 받기·올리기의 대상 교체와 메뉴 활성 규칙
  - **Type**: C
  - **Design**: `ui/app.rs`의 `apply_remote_menu`에서 `other_panel_local`·`LocalSide`를 지우고 T2가 만든 `WorkspaceView`의 목적지 조회(`download_dir`·`upload_dir`·`upload_source`)를 쓴다 — 앱은 판정하지 않고 **결과를 따르기만** 한다(D12). `remote_menu::menu_rows`는 `can_download`·`can_upload`를 받아 두 줄의 활성 여부에 반영하되, **`connected`가 거짓이면 그것이 우선**이라 모든 줄이 비활성인 기존 규칙(전제 검증 #8)을 지킨다. 신규 심볼은 `ExplorerApp`이 활성 워크스페이스로 위임하는 얇은 조회 3개(`download_dir`·`upload_dir`·`upload_source` — `WorkspaceView`의 동명 판정을 재노출할 뿐 로직은 없다)뿐이고, 나머지는 기존 함수의 대상 산출만 바뀐다.
  - **Acceptance**: Given 패널 A의 로컬 탭이 받기 대상인 `WorkspaceView`, When `download_dir()`를 부르면, Then A의 그 탭이 가리키는 폴더가 나온다(우클릭한 패널의 반대편이 아니다). / Given 패널 C의 원격 탭이 올리기 대상, When `upload_dir()`를 부르면, Then C의 그 탭의 `(site, path)`가 나온다. / Given `can_download`가 거짓, When `menu_rows`를 부르면, Then `받기` 줄이 비활성이다. / Given `can_upload`가 거짓, Then `올리기` 줄이 비활성이다. / Given `connected`가 거짓이면 `can_download`·`can_upload`가 **둘 다 참이어도** 모든 줄이 비활성이다(기존 규칙 유지). / `apply_remote_menu`가 그 값을 실제로 전송에 넘기는 것은 HUMAN-VERIFY (D12).
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/remote_menu.rs`
    - 동반: `src/ui/panel.rs` (비공개 래퍼 `PanelState::show_remote_menu`(`:1328`)와 그 호출부(`:1277`)에 인자 전달)
    - 테스트: `src/ui/remote_menu.rs`의 `mod tests`(기존 **4건** 갱신 + 신규), `src/ui/app.rs`의 `mod tests`(`WorkspaceView` 목적지 조회)
  - **Edge Cases**:
    - 기존 테스트 4건이 모두 인자 추가에 걸린다 — `메뉴가_한_프레임을_그린다`(`:554`)·`끊긴_연결에서는…`(`:566`)·`여럿을_고르면…`(`:583`)·`고른_것이_없어도…`(`:594`)
    - `끊긴_연결에서는…`은 인자만 늘리면 안 된다 — `can_*`를 참으로 준 조합도 함께 돌려 **연결 끊김이 우선**임을 단언한다
    - `고른_것이_없어도_할_수_있는_일은_남는다`의 기대값이 바뀐다 — `올리기`는 이제 `can_upload`에도 걸린다. 주석의 "올리기는 **반대편 패널의 선택**이 대상"도 새 규칙으로 고친다(AGENTS: 코드를 고치면 딸린 주석을 함께 고친다)
    - `menu_size()`(`:176`)는 줄 **수**만 세므로 `can_*`에 `false`를 넘겨도 되지만, 비활성 줄도 그려져 줄 수가 같음을 확인하고 넘긴다
    - 대상 원격 탭의 사이트가 연결되지 않았으면 올리기 비활성 — `TabSource::Remote { conn: None }`
  - **Halt Forecast**:
    - (ii-a) `menu_rows`·`show_remote_menu` 시그니처 변경 → `## 사전 승인 항목`에 등록
  - **Depends on**: T2, **T3** (`can_*`가 패널까지 닿는 통로 — `PanelState::show`의 `targets` 인자 — 는 T3이 만든다)

- [x] T5. 같은 이름 확인 게이트와 대화 — 받기
  - **Type**: D
  - **Design**: ① 판정은 `ui/app.rs`의 **자유 함수**(`ExplorerApp` 밖)로, 배선은 `ExplorerApp`에, 대화는 `ui/remote_menu.rs`에 둔다(원격 대화들이 이미 사는 곳). ② 신규 심볼 —
    - `conflict_names(items: &[DragItem], existing: &[String], ignore_case: bool) -> Vec<String>` (자유 함수, 순수): 고른 최상위 항목 중 대상에 이미 있는 이름을 낸다
    - `apply_conflict_choice(drop: DropOutcome, conflicts: &[String], choice: Option<ConflictChoice>) -> Option<DropOutcome>` (자유 함수, 순수): **D6의 계약을 한 함수에 담는다** — `choice=None`이면 충돌 0건일 때만 `Some`, `Overwrite`면 `Some(drop)`, `Skip`이면 충돌을 뺀 `Some`(전부 빠지면 `None`), 취소는 호출부가 `None`으로 처리
    - `ConflictChoice { Overwrite, Skip }` / `remote_menu::show_conflict_dialog(ctx, names: &[(String, bool)]) -> DialogOutcome<ConflictChoice>` (이름과 폴더 여부)
    - `ExplorerApp::start_transfer(&mut self, drop: DropOutcome)`: `apply_drop`의 **유일한 앞문** / `ConflictCheck { drop: DropOutcome }` / `conflict_rx`·`conflict_tx`: 로컬 확인 워커의 통로
    ③ `apply_drop`은 그대로 두고 호출부 3곳(`app.rs:1126`·`:1146`·`:2361`)이 `start_transfer`를 부르게 바꾼다 — 게이트를 우회하는 길이 남지 않는다. `ExplorerApp`은 위 두 순수 함수의 결과를 그대로 따르기만 한다. ④ 비추상화 선언 — 받기/올리기 확인을 공통 트레이트로 묶지 않는다. 한쪽은 파일시스템, 한쪽은 서버 조회라 비동기 경로 자체가 다르다(T6이 분기를 하나 더 더한다). 두 순수 함수만 공유한다.
  - **흐름**: `start_transfer` → 대상이 로컬이면 워커 스레드에 `(대상 폴더, 최상위 이름들)`을 보낸다 → 워커가 `dir.join(name)`의 존재를 확인해 충돌 이름을 돌려준다 → `apply_conflict_choice(.., None)`이 `Some`이면 그대로 `apply_drop`, `None`이면 대화를 띄우고 조작 전체를 보류(D6) → 사용자의 결정을 같은 함수에 다시 넣어 나온 값으로 `apply_drop`(또는 아무것도 안 함).
  - **화면 문구** (`src/i18n/mod.rs` — AGENTS 규약대로 카탈로그를 거친다. 한/영 원문을 그대로 적는다):
    - `conflict_title => "같은 이름이 이미 있습니다" / "Items with the same name already exist"`
    - `conflict_irreversible => "덮어쓰면 되돌릴 수 없습니다." / "Overwriting cannot be undone."`
    - `conflict_overwrite => "덮어쓰기" / "Overwrite"`
    - `conflict_skip => "건너뛰기" / "Skip"`
    - `conflict_folder_mark => "(폴더)" / "(folder)"`
    - 취소 버튼은 기존 `cancel`을 재사용한다
    - `dynamic::conflict_count(count)` — 한국어 `"{count}개 항목이 대상에 이미 있습니다."` / 영어 1건 `"1 item already exists at the destination."` / 복수 `"{count} items already exist at the destination."` (`remote_delete_count`와 같은 모양)
  - **Acceptance**: Given 대상에 `report.zip`이 있고 고른 최상위 이름이 `report.zip`·`a.txt`·`b.txt`인 상태, When `conflict_names(.., ignore_case=true)`를 부르면, Then `["report.zip"]`이다(`REPORT.ZIP`이 있어도 같다 — D5). / Given 충돌 1건, When `apply_conflict_choice(drop, &["report.zip"], None)`이면, Then `None`이다(**아직 아무것도 큐에 넣지 않는다** — D6). / `Some(Overwrite)`면 3건이 그대로 든 `Some(drop)`이다. / `Some(Skip)`이면 2건만 든 `Some`이다. / 충돌 항목이 전부라 남는 것이 없으면 `Skip`도 `None`이다. / Given 충돌 0건, When `.., None`이면, Then 3건이 그대로 든 `Some`이다. / 대화가 뜨고 결정이 실제 큐 등록으로 이어지는 것과 끌어다 놓기 경로가 같은 게이트를 지나는 것은 HUMAN-VERIFY (D12 — Verification Strategy 3·5번 절차).
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/remote_menu.rs`
    - 동반: `src/i18n/mod.rs` (위 문구 6개 + `dynamic::conflict_count`)
    - 테스트: `src/ui/app.rs`의 `mod tests`(두 순수 함수), `src/i18n/mod.rs`의 문구 시험(원문 리터럴로 단언 + `LanguageGuard::lock` — AGENTS 규약)
  - **Edge Cases**:
    - 대상 폴더 자체가 없다 → 충돌 0건 (전송이 폴더를 만든다)
    - 대소문자만 다른 이름(`A.TXT` vs `a.txt`) → 충돌로 본다(D5)
    - 이름이 4개를 넘으면 대화 목록은 5개까지 보이고 나머지는 `…`로 줄인다(`show_delete_confirm`과 같은 규칙)
    - 확인을 기다리는 동안 사용자가 같은 조작을 또 걸 수 있다 → 보류 항목을 목록으로 들어 순서대로 처리한다(대화는 한 번에 하나)
    - 워커 스레드가 답하기 전에 앱이 닫힌다 → 채널이 닫혀 워커가 조용히 끝난다(기존 펼치기 스레드와 같다)
    - AGENTS 규약: 존재 확인은 파일시스템 호출이라 **UI 스레드에서 하지 않는다**
    - AGENTS 규약: 화면 문구는 `i18n` 카탈로그를 거친다(소스에 한글 직접 기입 금지)
  - **Halt Forecast**:
    - (i) `apply_drop`을 우회하는 다른 큐 등록 경로가 있으면 게이트가 새는가 → 4-A에서 프로덕션 등록 3곳이 전부 `apply_drop` 하류임을 확인했다. `session.rs:223`은 저장된 큐의 복원이라 게이트 대상이 아니다
    - (ii-a) `ExplorerApp`에 채널·보류 목록 필드 추가(구조 변경) → `## 사전 승인 항목`에 등록
  - **Depends on**: T4

- [x] T6. 같은 이름 확인 — 올리기
  - **Type**: D
  - **Design**: `start_transfer`(T5)에 대상이 원격일 때의 분기를 더한다. `DropOutcome`은 `SiteId`만 들고 있으므로(`list_common.rs:69-75`) 보낼 연결은 기존 `ExplorerApp::site_connection(site)`(`app.rs:1329`)로 찾는다 — **그 사이트에 연결이 없으면 확인을 건너뛰고 그대로 진행한다**(D10과 같은 처리). 대상 원격 폴더를 `ConnCommand::List { generation: CONFLICT_LIST_BASE + n, path }`로 조회하고(D8), `ConnEvent::Listed`가 오면 `pending_conflict_lists`에서 그 조작을 꺼내 `RemoteEntry::name`과 최상위 이름을 대조한다. `ConnEvent::ListFailed`면 확인을 포기하고 충돌 0건으로 진행한다(D10). 대화·세 결정 처리는 T5 것을 그대로 쓴다. 신규 심볼 — `CONFLICT_LIST_BASE` 상수와 `pending_conflict_lists` 필드뿐.
  - **Acceptance**: Given 대상 원격 폴더의 목록이 `["setup.exe", "log"]`이고 올릴 최상위 이름이 `setup.exe`·`data.bin`인 상태, When `conflict_names(.., ignore_case=false)`를 부르면, Then `["setup.exe"]`이다. / Given 원격 목록이 `["Setup.exe"]`이고 올리는 것이 `setup.exe`면, Then 빈 목록이다(대소문자 구분 — D5). / 세 결정의 결과는 T5의 `apply_conflict_choice` 시험이 이미 덮는다(방향과 무관한 순수 함수다). / Given 대상 폴더 조회가 실패하면, Then 확인을 건너뛰고 전송이 진행된다(D10) — 이 배선은 HUMAN-VERIFY.
  - **Files**:
    - 주: `src/ui/app.rs`
    - 테스트: `src/ui/app.rs`의 `mod tests`(원격 쪽 대소문자 규칙)
  - **Edge Cases**:
    - 충돌 확인 조회의 답이 패널 목록·트리 캐시로 새지 않아야 한다 — `Listed` 라우팅에서 `pending_conflict_lists`를 **가장 먼저** 조회하고 맞으면 `continue`한다(`pending_tree_lists`와 같은 방식)
    - 연결이 도중에 끊긴다 → 답이 오지 않아 보류 항목이 남는다. `ConnEvent::Phase(Closed|Failed)`를 받을 때 그 연결을 기다리던 보류 항목을 확인 포기로 처리한다
    - 대상 폴더가 서버에 없다 → 조회 실패로 들어와 D10대로 진행한다
    - 같은 사이트에 확인 조회와 패널 목록 조회가 겹칠 수 있다 — 번호 공간이 달라 섞이지 않는다
  - **Halt Forecast**:
    - (i) `List` 응답이 어느 조작의 답인지 잃어버리는 경우 → 세대 번호 + 전용 지도로 잇는다(기존 트리 조회와 같은 방식)
    - (ii-a) `ExplorerApp`에 보류 지도 필드 추가 → `## 사전 승인 항목`에 등록
  - **Depends on**: T5

- [x] T7. PRD·README 갱신
  - **Type**: A
  - **Acceptance**: `docs/prd.md`에 아래 두 FR이 추가되고 FR-38·FR-39 문구가 보완되며, 성공 기준의 Must·Should 목록과 결정 이력에 반영된다. `README.md`의 전송 큐·원격 파일 작업 두 줄이 새 동작을 서술한다. 문서에 실제 호스트·계정 등 민감 정보가 들어가지 않는다.
  - **PRD 추가 문안** (그대로 적는다):
    - `| FR-54 | **전송 대상 탭을 화면에 표시하고 그 탭으로 전송한다** — 마지막으로 누른 패널의 활성 탭이 그 종류(로컬/원격)의 전송 대상이 되며, 대상 탭은 탭 스트립에서 폴더 아이콘 대신 **받기·올리기 아이콘**으로 표시된다. 받기는 받기 아이콘이 붙은 로컬 탭의 폴더로 내려받고, 올리기는 올리기 아이콘이 붙은 원격 탭의 폴더로 올린다(올릴 항목은 받기 아이콘 탭에서 고른 것이다). 대상이나 올릴 항목이 없으면 원격 메뉴의 해당 항목이 비활성이다 | Should | 단위테스트: 대상 판정·아이콘 선택·메뉴 활성 규칙 검증 + 화면은 HUMAN-VERIFY |`
    - `| FR-55 | **대상에 같은 이름이 이미 있으면 전송 전에 확인을 받는다** — 고른 최상위 항목의 이름(파일이면 같은 이름 파일, 폴더면 같은 이름 폴더)이 대상 폴더에 있으면 그 목록을 보이는 확인 대화를 띄우고 `덮어쓰기`·`건너뛰기`·`취소` 중 하나를 받는다. 세 결정은 그 전송 전체에 일괄 적용되며, 결정을 받기 전에는 **아무 항목도 큐에 들어가지 않는다**. 원격 메뉴의 받기·올리기와 끌어다 놓기 전송(FR-38)이 같은 확인을 거친다. 대상을 조회하지 못하면 확인을 건너뛰고 전송을 진행한다 | Must | 단위테스트: 충돌 판정·세 결정의 큐 등록 결과 검증 + 대화는 HUMAN-VERIFY |`
    - FR-38 끝에 덧붙임: ` 대상에 같은 이름이 있으면 FR-55의 확인을 거친다`
    - FR-39 제목 보완: `원격 항목에 대해 **받기·올리기·삭제·이름 바꾸기·새 폴더·권한 변경(chmod)** 을 한다` + 끝에 덧붙임: ` 받기·올리기의 대상 탭 규칙은 FR-54, 같은 이름 처리는 FR-55다`
    - 성공 기준: Must 목록에 `FR-55`, Should 목록에 `FR-54` 추가
    - 결정 이력에 `2026-08-15` 항목 추가 — 대상 판정(마지막으로 누른 패널의 활성 탭·탭 단위 sticky)·충돌 판정 단위(최상위 항목 이름)·세 버튼 일괄 적용·끌어다 놓기에도 적용을 사용자 결정으로 기록
  - **Files**:
    - 주: `docs/prd.md`, `README.md`
  - **Edge Cases**:
    - PRD 표의 열 구성(ID·내용·우선순위·검증)을 기존 행과 맞춘다
    - README는 요청 없는 대규모 재구성을 하지 않는다 — 해당 두 줄만 고친다
  - **Halt Forecast**:
    - (ii-a) PRD 요구사항 신설·개정 → `## 사전 승인 항목`에 등록(문안을 위에 그대로 실어 승인받았다)
  - **Depends on**: T6

- [x] T8. 올리기 확인의 세대 키 불일치 수정과 연결 회수 경로 정리 (완료 검증에서 추가)
  - **Type**: C
  - **Design**: ① `src/ui/app.rs`. ② 신규 심볼 — `conflict_generation(id)`·`conflict_id(generation)` 자유 함수 한 쌍. ③ 등록·발송·조회가 **이 한 쌍만** 쓰게 해 양쪽에서 손으로 더하고 빼는 길을 없앤다(`conflict_lists`의 키를 확인 번호가 아니라 **조회 세대**로 바꾼다 — `pending_tree_lists`와 같은 규칙). ④ 비추상화 선언 — 세대 공간을 타입으로 감싸지 않는다(D8의 `enum ListSource` 미채택 결정과 같은 이유).
  - **Acceptance**: Given 확인 번호 `n`, When `conflict_id(conflict_generation(n))`이면, Then `n`이다(0·1·7·4096·u32::MAX). / 그 세대는 `CONFLICT_LIST_BASE` 이상이다. / 서로 다른 확인은 서로 다른 세대를 쓴다. / 등록 키와 발송 세대는 **같은 지역 변수**에서 나온다(구조적으로 어긋날 수 없다). / 앱이 연결을 접는 두 경로(`release_conn`·`close_tab`)도 물어 둔 확인을 거둔다 — 배선은 HUMAN-VERIFY.
  - **Files**: 주 `src/ui/app.rs` / 동반 `src/ui/panel.rs`(고아가 된 `local_dir` 제거)·`src/ui/file_list.rs`(stale 주석) / 테스트 `src/ui/app.rs`의 `mod tests`
  - **Edge Cases**: 연결이 스스로 끊기는 길(`ConnPhase::Closed|Failed`)과 앱이 접는 길(`release_conn`·`close_tab`)은 서로 다른 경로다 — 후자는 `manager.close`가 연결을 지워 사이트를 알 수 없게 되므로 **접기 전에** 거둔다
  - **Halt Forecast**: (i) 없음 — 계획된 수정이다
  - **Depends on**: T6

## 사전 승인 항목 (일괄 승인 대상)
- T2 — `LayoutOutcome`에 `pressed_panel` 필드 추가 (D11 — 팝업에 가려지지 않은 패널 클릭 신호)
- T3 — `PanelState::show`·`splitter::show_layout`·`tabs::show_tab_strip` 시그니처에 인자 1개 추가 (대상 정보를 패널까지 내려보내기 위함. 호출부는 4-A에 전건 등재)
- T3 — `UPLOAD_GLYPH`·`DOWNLOAD_GLYPH`·`direction_mark`를 `ui/queue_panel.rs`에서 `ui/widgets.rs`로 **이동** (D9 — 큐 화면과 탭이 같은 시각 언어를 쓰게. 시그니처는 그대로)
- T4 — `remote_menu::menu_rows`·`show_remote_menu` 시그니처에 인자 2개 추가, `ExplorerApp::other_panel_local`·`LocalSide` 삭제 (대상 판정이 `TransferTargets`로 옮겨가 쓰이지 않게 된다)
- T5·T6 — `ExplorerApp`에 필드 추가 (확인 워커 채널 2개, 보류 조작 목록, 원격 확인 조회 지도, 세대 기준 상수)
- T7 — PRD 요구사항 신설 2건(FR-54·FR-55)과 기존 FR-38·FR-39 문구 보완, 성공 기준·결정 이력 갱신 (문안은 T7에 그대로 실려 있다)

## 불가피한 Halt (위임 불가)
- commit 이후의 push·태그·릴리즈 — 이 plan 승인에 포함되지 않는다. 구현·검증이 끝난 뒤 별도로 승인받는다
- plan에 없던 구조 결정이 필요해지는 경우 (예: 확인 게이트가 전송 큐 모델 자체를 바꿔야 하는 상황) — 그 지점에서 멈추고 확인받는다

## Verification Strategy
- 빌드: `cargo build`
- 단위·통합 테스트: `cargo test`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 서식: `cargo fmt --check`
- 수동 검증 (사용자 확인 필요 — GUI 상호작용):
  1. 로컬 패널과 원격 패널을 나란히 두고 각각을 눌러 아이콘이 받기/올리기로 바뀌는지 본다
  2. 원격에서 파일을 골라 `받기` → 받기 아이콘 탭의 폴더에 내려오는지 확인
  3. 같은 파일을 다시 받아 확인 대화가 뜨는지, 세 버튼이 각각 제 일을 하는지 확인
  4. 로컬에서 파일을 골라 `올리기` → 올리기 아이콘 탭의 폴더로 올라가는지, 같은 이름일 때 대화가 뜨는지 확인
  5. 끌어다 놓기로도 3·4와 같은 확인이 뜨는지 본다

## Phase Ledger
- Phase F 통과 (HEAD 033db04)
- Phase G 통과 (Must 100% — FR-55 충족, 기존 Must FR 회귀 없음)

## Retry Ledger

## Progress Log
- T1 완료 (커밋 954e171): `TabId` 도입 — 탭을 인덱스가 아니라 신원으로 가리킨다. 전역 `AtomicU64`를 쓴 이유(`TabsModel`이 패널마다 있어 인스턴스 카운터로는 번호가 겹친다)를 주석에 남겼다.
- **T2·T3 합쳐서 완료** (커밋 a8ea9b3): T2만 커밋하면 새 필드를 읽는 곳이 없어 `clippy -D warnings`가 dead_code로 막는다 — 값을 만드는 쪽(T2)과 처음 읽는 쪽(T3 아이콘)이 나뉘어 분리 시 중간 상태가 검증을 통과하지 못한다. Files 범위는 두 task의 합집합을 벗어나지 않았다.
  - 결정: `TransferTargets` 정의를 `ui/app.rs`가 아니라 `ui/tabs.rs`에 뒀다(상향 의존 회피 — T2 Design ①에 정정 기록).
  - 결정: 아이콘 흐림 판정을 `tabs::icon_color`로 뽑아 단위 시험 대상으로 삼았다(spec 리뷰 M2).
  - 결정: 방향 글리프·색을 `queue_panel`에서 `widgets`로 옮겨 큐 화면과 탭이 같은 시각 언어를 쓰게 했다(D9).
- T4~T6 완료 (커밋 3ed7709·45073a0·64290ba): 대상 교체·메뉴 활성 규칙 → 받기 충돌 확인(게이트+대화) → 올리기 충돌 확인(원격 조회). 리뷰에서 BLOCKER 1건(대화가 떠 있는 동안 도착한 확인 결과가 유실됨 — 워커는 답을 한 번만 보낸다)을 잡아 `conflict_queue`(겹침 목록까지 함께 보관하는 대기열)로 고쳤다.
  - 결정: `conflict_lists`가 물어본 사이트를 함께 들어, 연결이 끊겼을 때 `pending_conflicts`를 되짚지 않고 바로 가려낸다 (T6 quality S1).
  - 결정: 세대 기준값을 하나 더 두는 절충(D8)의 이유를 `CONFLICT_LIST_BASE` 주석에 남겼다 — 다음에 또 늘릴 때 `enum ListSource`로 안 간 까닭을 헤매지 않게.
- T7 완료 (커밋 e74ee7b): PRD FR-54·FR-55 신설과 README 두 줄 갱신. 1라운드 리뷰에서 승인 문안을 임의로 늘린 것(MAJOR)과 README 범위 초과(MAJOR)를 지적받아, 문안을 plan에서 그대로 뽑아 되돌리고 뺀 사실은 결정 이력으로 옮겼다.
- **T8 완료** (커밋 033db04, 완료 검증에서 추가): 올리기 확인의 등록 키(`id`)와 조회 키(`CONFLICT_LIST_BASE + id`)가 달라 **모든 올리기가 조용히 사라지던 회귀**를 고쳤다. 726건 통과·clippy 0이 이 결함을 하나도 덮지 못했다 — 배선을 전부 HUMAN-VERIFY로 미룬 자리(D12)의 사각이었다.
  - 결정: 변환 함수 한 쌍을 두고 등록·발송이 같은 지역 변수를 쓰게 해 재발을 구조로 막았다.
  - 결정: 앱이 연결을 접는 두 경로(`release_conn`·`close_tab`)는 워커의 단계 통지를 거치지 않으므로 `manager.close` 전에 확인을 거둔다.

## Next Steps
- 권장 다음 액션: **사용자 수동 확인** — `## Verification Strategy`의 5단계, 특히 4·5번(올리기·끌어다 놓기 올리기). 이번 회차의 BLOCKER가 바로 그 배선 사각에서 나왔다
- 그다음: master 병합·push는 별도 승인 대상

## Open Questions
- [x] Q1: "마지막에 선택한 탭"의 판정 기준 → **마지막으로 누른 패널의 활성 탭** (D1)
- [x] Q2: 올리기의 원본은 어디의 선택인가 → **받기 아이콘이 붙은 탭의 선택** (D3)
- [x] Q3: 충돌 판정 단위 → **고른 최상위 항목 이름 기준** (D4)
- [x] Q4: 대화 버튼 구성과 적용 범위 → **목록 + `덮어쓰기`·`건너뛰기`·`취소` 일괄 적용, Esc·바깥 클릭은 취소** (D6)
- [x] Q5: PRD를 함께 갱신할 것인가 → **FR-54·FR-55 신설 + FR-38·FR-39 보완** (T7)
- [x] Q6: 대상·원본이 없을 때의 처리 → **메뉴 항목을 비활성으로** (토스트 안내는 넣지 않는다 — T4)
- [x] Q7: 끌어다 놓기 전송에도 확인을 적용할 것인가 → **적용** (T5·T6)
- [x] Q8: 조회 종류가 셋이 되는데 `enum ListSource`로 리팩터할 것인가 → **이번엔 번호 공간만 추가**, 대장에 유지 (D8)
