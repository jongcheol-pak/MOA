# Plan: 사이드바에서 사이트를 지우면 전송 큐 탭도 함께 사라진다

**PRD**: `docs/prd.md` (FR-29 문면 개정을 포함한다 — T6)

## 요구 이해

- **원문 요청**: "연결 목록에서 원격 항목을 삭제하면 전송 큐 화면에서도 원격항목 탭 바로 삭제 하도록 수정"
- **이해한 요구**: 사이드바 `등록된 사이트` 목록에서 사이트 행을 우클릭해 지우면, 지금은 사이드바에서만 감춰지고 하단 도크의 **연결별 탭**에는 그 사이트가 그대로 남는다. 이것을 **그 자리에서 함께 사라지게** 한다. 그러려면 탭이 서는 두 근거(그 사이트의 **큐 항목**과 **열린 연결**)를 함께 걷어내야 하므로 — 진행·대기 중인 전송은 취소하고 큐 항목을 지우며, 그 사이트의 연결을 끊고 열린 원격 탭도 닫는다. 진행·대기 중 전송이 있을 때만 확인을 한 번 받는다. 사이트 기록 자체는 사이트 관리자에 그대로 남긴다(되돌릴 수 있게).
- **포함하지 않는 것으로 이해**: 사이트 관리자의 `삭제(D)`(사이트 기록 자체를 지우는 길)는 이번 변경 대상이 아니다. 거기서 지운 사이트의 **잔여 큐 항목**은 종전대로 `사이트 N` 이름으로 탭에 남는다.

## Goal

사이드바에서 사이트를 지우면 그 사이트의 연결·원격 탭·전송 큐 항목이 함께 걷혀 전송 큐 화면의 연결별 탭에서도 즉시 사라진다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-29 (사이드바 연결 섹션 · 우클릭 삭제) | Must | T4·T5·T6 | ✅ 커버 (문면 개정 포함) |
| 그 밖의 active Must FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- 사이트 관리자 `삭제(D)`의 동작 변경 — 거기서는 사이트 기록만 지운다(FR-27). 이번 변경은 사이드바 경로만이다.
- `Del` 키로 사이트를 지우는 실제 단축키 배선 — 메뉴에 `Del` 표시만 있고 키 바인딩은 지금도 없다(`Key::Delete` 검색 0건). 이번 요청과 무관한 기존 상태다.
- 사이드바 메뉴에 디자인 원본의 휴지통 아이콘(`🗑`) 추가 — 문구·hover 색만 원본에 맞춘다.

## Deferred / Follow-up

- **위키 갱신** — `20_projects/personal/moa/feat-remote-sites.md:74`의 「숨기기와 삭제는 다르다 — 사이드바 우클릭의 `삭제`는 바로가기만 없앤다」가 뒤집힌다. `feat-dock-status.md`·`feat-remote-transfer.md`의 연결별 탭 서술에도 「사이트를 지우면 탭이 사라진다」가 더해질 자리가 있다. F-6.5의 위키 큐가 맡는다.
- **[SUGGEST] `release_conn`에도 `TransferRunner::forget_connection`을 잇기** — 지금 `forget_connection`은 앱 어디에서도 불리지 않는다(시험만 부른다). 그래서 **탭 ✕로 연결을 접는 길**에서는 취소 뒤 정리를 기다리던 `.part`가 영영 남는다(그 함수의 doc이 그 상황을 그대로 서술한다). 이번 plan은 새 경로(T4)에서만 그것을 부르고 기존 경로는 손대지 않는다 — 기존 경로의 누수는 이번 변경이 유발한 것이 아니고, 고치려면 그 길의 회귀 검증이 따로 필요하다.

## Investigation Log

- 위키 참조: `20_projects/personal/moa/feat-remote-sites.md` — 「숨기기와 삭제는 다르다: 사이드바 우클릭의 `삭제`는 바로가기만 없애고 사이트는 관리자 목록에 남는다」(:74). 이번 변경이 뒤집는 문장이다.
- 위키 참조: `20_projects/personal/moa/feat-remote-transfer.md` — 「연결을 지우기 전에 물어 둔 확인을 거둬야 한다. 거둘 때는 **그 연결로 물어 둔 것만** 골라야 한다 — 사이트 단위로 거두면 멀쩡한 확인까지 날아간다」(:108). 연결 회수 순서 설계의 근거다.
- 위키 참조: `20_projects/personal/moa/feat-dock-status.md` — 연결별 탭 줄은 큐·로그가 공유하고 고른 사이트(`DockState.site`)도 하나다(:46·:33). 지운 사이트를 고른 채면 되돌려야 한다.
- 위키 참조: `20_projects/personal/moa/decisions.md` — 사이드바 숨기기·큐 탭에 관한 과거 결정 없음(2026-08-20 메뉴 토큰 결정만 있다).
- 큐 탭 목록의 판정: `src/ui/queue_panel.rs:392-408` — `sites.sites()` 중 **큐에 항목이 있거나(`members`) 지금 연결된(`view.connected`)** 사이트만 선다. 숨김 여부는 보지 않으므로 지금은 숨겨도 탭이 남는다.
- `SiteStore::hide` 호출부 전수(3곳, 시험 제외): ① `src/ui/app.rs:906`(사이드바 우클릭 — 이번 대상) ② `src/ui/app/remote.rs:690`(주소창으로 처음 붙은 서버를 숨긴 사이트로 등록한 직후 **그대로 연결한다**) ③ `src/remote/site_export.rs:398`(가져오기가 숨김 표시를 복원). ②·③에서 걷어내기가 돌면 방금 연 연결이 끊기거나 가져오기가 큐를 지운다 — 그래서 걷어내기는 `SiteStore::hide`가 아니라 **사이드바 처리부**에 둔다.
- 확인 대기의 부활 경로: `abandon_conflict_lists`(`src/ui/app/transfer_conflict.rs:117`) → `settle_conflict` → 겹침 없음으로 보고 **큐에 넣는다**(D10). 그래서 큐를 먼저 비우고 연결을 닫으면 그 순간 항목이 되살아나 탭이 다시 선다.
- 연결 회수: `release_conn`(`src/ui/app/remote.rs:338`)이 확인 거두기·`manager.close`·트리 캐시·대기 자리 정리를 한다. 그 사이트의 연결은 여럿일 수 있다(FR-37 탐색 1 + 전송 2) — `site_connection`은 **하나만** 돌려준다(`:286`).
- `.part` 정리: `TransferRunner::cancel`이 받기 항목을 `cancelling`에 담아 워커가 놓기를 기다리는데, 연결을 닫으면 완료 통지가 채널째 버려져 `on_done`이 오지 않는다. 그 자리를 `forget_connection`(`src/remote/transfer.rs:388`)이 맡도록 만들어져 있으나 **앱에서 부르는 곳이 없다**(전수 검색: 시험 1곳뿐).
- 탭 닫기 규칙: `PanelState::handle_tab`(`src/ui/panel.rs:993`) — 마지막 탭이면 `CloseOutcome::LastTab`이라 탭이 남고 `close_requested`가 서며, 그것을 받은 `WorkspaceView::close_panel`(`src/ui/app.rs:273`)이 패널을 닫되 **마지막 패널은 닫지 않는다**(FR-2).
- 사이트를 잃은 원격 탭의 선례: 세션 복원은 그 탭을 **로컬 홈으로 되돌린다**(`src/ui/session.rs:179-190`), 큐 항목은 버린다(`:231-237`).
- 디자인 원본의 사이드바 우클릭 메뉴: 문구 `삭제` · 오른쪽에 `Del` · hover 배경 `#C42B1C`(`docs/design/design-files/FileExplorer-FTP.dc.html:358-360`). 지금 구현은 문구를 `사이드바에서 숨기기`로 바꾸고 파괴색을 뺀 상태다(`src/ui/sidebar.rs:669-680`, 2026-08-16 검토).
- 확인 대화 선례: `show_remove_confirm`(`src/ui/app.rs:1034`) — `dialog::show` + `REMOVE_DIALOG_WIDTH`(360.0) + `delete()`/`cancel()` 버튼. 모달이 떠 있는 동안 단축키를 막는 조건은 `src/ui/app.rs:2073`이다.
- 시험 가능 범위: `PanelState`·`TransferQueue`·순수 함수는 시험 대상이고(`src/ui/panel/tests.rs`가 `egui::Context::default()`로 프레임을 돌린다), `ExplorerApp`은 실 창 핸들이 있어야 만들어져 시험 대상이 아니다(AGENTS).
- Deferred 대장 조회: `docs/plans/deferred.md` `## 대기` 74건(실측) — 이번 요청과 걸리는 항목 없음. 소진 batch 임계(잔량 100 초과 + 신규 30건 / 최소 판정일 30일 초과 / 절대 상한 130) 미달이라 batch task를 두지 않는다.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 큐 항목과 연결이 모두 없어지면 그 사이트는 연결별 탭에 서지 않는다 | `src/ui/queue_panel.rs:394-408` — `members`·`connected` 어느 쪽에도 없으면 `order`에서 빠지고, `extra`는 **저장소에 없는** 사이트만 담는다(지운 사이트는 저장소에 남는다) | ✅ |
| 2 | 사이드바 처리부에 걷어내기를 두면 주소창 자동 등록·가져오기 경로는 영향이 없다 | `SiteStore::hide` 호출부 전수 3곳(위 Log) | ✅ |
| 3 | 연결을 닫는 순간 그 연결로 물어 둔 확인이 큐에 들어간다 | `transfer_conflict.rs:117-130` → `settle_conflict` → `apply_drop` | ✅ |
| 4 | 사이드바 처리부에서 패널을 닫는 데 필요한 `area`를 이미 받고 있다 | `handle_sidebar(action, area, now)` 호출부 `src/ui/app.rs:2063-2067`(`area`를 그 자리에서 만들어 넘긴다) | ✅ |
| 5 | `PanelState`의 탭 조작은 실 창 없이 시험할 수 있다 | `src/ui/panel/tests.rs`의 `egui::Context::default()` 사용 20여 곳 | ✅ |
| 6 | 확인 대화를 `ui::dialog` 셸로 세우면 다른 팝업과 모양이 같다 | `show_remove_confirm`(`src/ui/app.rs:1034-1082`)이 같은 셸을 쓴다 | ✅ |
| 7 | `TransferRunner::forget_connection`은 지금 앱에서 불리지 않아, 새로 부르는 것이 기존 경로의 동작을 바꾸지 않는다 | `grep forget_connection src` — 정의·doc과 시험 호출 1곳(`src/remote/transfer.rs:895`)뿐, 앱 호출부 0 | ✅ |
| 8 | `runner.cancel`은 연결이 살아 있어야 성립하고, `forget_connection`을 먼저 부르면 그 뒤 `cancel`이 아무 일도 하지 않는다 | `src/remote/transfer.rs:323-333`(`assigned.remove` → `manager.send(Cancel)` → `cancelling` 등록)·`:388-403`(`assigned` 정리 없이 버리고 `cancelling`만 옮긴다) | ✅ |
| 9 | 큐에 넣는 비동기 경로가 T3이 막는 것 말고 셋 더 있고, 그중 둘만 새 조치가 필요하다 | `src/ui/app.rs:1957-1963`(`expand_rx` → `queue.enqueue`, 사이트 검사 없음)·`:1842-1850`(`os_drop_rx` → `start_transfer`) — 조치 필요 / `src/ui/app/remote.rs:427-445`(트리 훑기 답)은 `release_conn`의 `pending_trees` 정리(`:349-354`)가 이미 막는다 | ✅ |
| 10 | 활성 탭이 원격일 때 `new_tab`은 `start_dir()`를 가리키는 로컬 탭을 연다 | `src/ui/panel.rs:1032-1046`(`TabAction::New` — 원격이면 `crate::ui::app::start_dir()`) | ✅ |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 걷어내는 순서를 틀리면 지운 큐 항목이 되살아나거나(전제 3) `.part`가 남는다(전제 8) | 탭이 다시 서서 요구가 미충족 · 디스크에 조각 방치 | T4에서 **확인 대기 버리기 → 탭 닫기 → 취소·큐 비우기 → 연결 회수 → `forget_connection`** 순서를 고정하고 그 근거(D2)를 코드 주석에 남긴다 |
| 폴더 펼치기·OS 드롭 스캔의 결과가 삭제 뒤 프레임에 도착해 큐를 되살린다 | 탭이 다시 선다 | `detached_sites` 표시로 그 결과를 버리고, 다시 연결하면 표시를 푼다(D2-1) |
| 그 사이트가 마지막 패널의 유일한 탭인 경우 FR-2로 패널이 닫히지 않아 원격 탭이 남는다 | 연결이 없어도 탭이 남아 사용자가 다시 연결하면 큐 탭이 되살아난다 | 그때는 로컬 시작 폴더 탭으로 되돌린다(세션 복원 선례와 같은 처리 — D3) |
| 받는 중이던 `.part`가 남는다 | 디스크에 조각 파일이 방치 | 연결이 살아 있을 때 `runner.cancel`로 워커를 멈춰 `cancelling`에 담고, 연결을 닫은 **직후** `runner.forget_connection(conn)`으로 정리 목록에 넘긴다(D2·D8) |
| 확인 대화가 떠 있는 사이에 대상 사이트가 사라진다(사이트 관리자에서 지움) | 없는 사이트를 지우려다 헛돈다 | 대화를 그릴 때마다 대상 존재를 다시 확인하고, 없으면 대기를 푼다(`show_remove_confirm`과 같은 처리) |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `SidebarAction::HideSite` → `RemoveSite`로 이름 변경 | `src/ui/sidebar.rs:100`(정의)·`:682`(발생), `src/ui/app.rs:903`(처리) | 열거 변형 이름 변경 — 호출부 2곳 전수 |
| `i18n::sidebar_hide_site` (제거) | `src/i18n/mod.rs:221`(정의), `src/ui/sidebar.rs:670`(유일 사용) | 카탈로그 항목 제거 · 사용처는 `i18n::delete()`로 교체 |
| `i18n::dynamic::site_hidden` → `site_removed` | `src/i18n/mod.rs:1074-1085`, `src/ui/app.rs:908` | 문구·이름 변경 — 사용처 1곳 |
| `sidebar::HIDE_SITE_SHORTCUT` → `DELETE_SITE_SHORTCUT` | `src/ui/sidebar.rs:79`·`:673`·`:850`(시험) | 상수 이름 변경 — 파일 안 3곳 |
| `ExplorerApp::handle_sidebar` (인자 `ctx` 추가) | `src/ui/app.rs:875`(정의), `:2067`(유일 호출) | 비공개 메서드 시그니처 변경 |
| `PanelState::close_site_tabs` (신규 — `bool` 반환) | `src/ui/panel.rs`, `src/ui/app/remote.rs`(호출), `src/ui/panel/tests.rs` | 공개 API 추가 |
| `TransferQueue::site_items` (신규) | `src/remote/queue.rs`, `src/ui/app/remote.rs`(호출) | 공개 API 추가 |
| `ExplorerApp::site_connections` (신규) | `src/ui/app/remote.rs` | 비공개 헬퍼 추가 (`site_connection`:286의 복수형) |
| `transfer_conflict::conflict_site` (신규 순수 함수) + `ExplorerApp::drop_site_conflicts` (신규) | `src/ui/app/transfer_conflict.rs` | 모듈 내 추가 |
| `ExplorerApp::detach_site` (신규) | `src/ui/app/remote.rs`, `src/ui/app.rs`(호출) | 비공개 메서드 추가 |
| `ExplorerApp::detached_sites`·`pending_site_remove` (신규 필드) | `src/ui/app.rs`(정의·초기화), `src/ui/app/remote.rs`(쓰기·검사) | 앱 상태 추가 |
| `expand_rx` 소비 지점 (`src/ui/app.rs:1957-1963`) · `os_drop_rx` 소비 지점 (`:1842-1850`) | `src/ui/app.rs` | 큐 등재 앞에 `detached_sites` 검사 추가 |
| `ExplorerApp::connect_site` (`src/ui/app/remote.rs:775`) | `src/ui/app/remote.rs` | 본문에 `detached_sites` 표시 해제 한 줄 추가(시그니처 불변) |
| `PanelState::new_tab` (변경 없음 — 재사용) | `src/ui/app/remote.rs`(호출 추가) | 마지막 패널 폴백에 그대로 쓴다(D3) |
| `SiteStore::hide` (변경 없음) | — | 호출부 3곳 중 사이드바 경로만 앞뒤 처리가 늘어난다 |

### 4-B. 계약·직렬화 변경

- 없다. 세션 스키마(v3)·설정 파일 형식은 그대로다. 큐 항목이 줄고 원격 탭이 닫힌 결과가 **다음 저장**에 반영될 뿐이다.

### 4-C. 테스트 파일

- `src/ui/panel/tests.rs` — `close_site_tabs` 신규 시험(T2)
- `src/remote/queue.rs`의 `#[cfg(test)] mod tests` — `site_items` 신규 시험(T1)
- `src/ui/app/transfer_conflict.rs`의 `#[cfg(test)] mod tests` — `conflict_site` 신규 시험(T3)
- `src/ui/sidebar.rs`의 `#[cfg(test)] mod tests` — `연결_섹션_문구는_인벤토리_원문_그대로다`(`:850`의 `HIDE_SITE_SHORTCUT` 단언)가 이름 변경으로 깨진다(T5). `숨긴_사이트는_사이드바에서만_사라진다`(`:893`)는 저장소 수준 규칙이라 그대로 유효하다
- `src/i18n`의 소스 훑기 시험 — 새 문구가 카탈로그를 거치는지 자동 검사(T4·T5)

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `TransferQueue::site_items` | `filter`(`queue.rs:290`)·`counts_by_site`(`:352`)·`requeue_site`(`:279`)가 같은 축으로 훑는다 | 반환 형태가 다르다(`Vec<&TransferItem>`) — `filter`와 같은 관례로 하나 더 둔다. 지울 번호와 진행 건수를 **한 번의 훑기**로 얻으려는 것이라 둘로 나누지 않는다 |
| `PanelState::close_site_tabs` | `close_tab`(`panel.rs:595`)은 활성 탭 하나, `open_remote_tab_only`(`:733`)가 여러 탭을 닫는 반복 패턴을 이미 쓴다 | 조건(사이트 일치)으로 여러 탭을 닫는 길은 없다 — 신규. 내부는 `tabs.close`를 그대로 쓰고, 반환은 `bool` 하나라 새 타입을 만들지 않는다 |
| `ExplorerApp::site_connections` | `site_connection`(`remote.rs:286`)이 **첫 연결 하나**만 준다 | 사이트 하나가 연결 셋을 쓰므로(FR-37) 복수형이 필요하다 — 같은 자리에 나란히 둔다 |
| `transfer_conflict::conflict_site` | `DropOutcome`에 `source_site`·`target`이 이미 있다(`list_common.rs:100-105`) | 두 자리를 한 판정으로 합치는 함수가 없다 — 순수 함수로 신규(시험 대상) |
| `ExplorerApp::detach_site` | `release_conn`·`apply_queue_action`의 `RemoveAll`이 부분 절차를 각각 갖고 있다 | 그 둘을 **부르는** 조율자라 신규. 절차를 복제하지 않는다 |
| 확인 대화 | `show_remove_confirm`(`app.rs:1034`)·`site_manager::show_delete_confirm` | 같은 `ui::dialog` 셸·`REMOVE_DIALOG_WIDTH`를 그대로 쓴다(문구만 신규) |
| 파괴색 hover | `theme`에 파괴색 토큰 없음(`ERROR` #FF6B6B는 글자용) | 디자인 원본 값(`#C42B1C`)으로 `theme::MENU_HOT_DANGER` 신규 — 값을 사이드바 파일에 박으면 메뉴 토큰 규약(AGENTS)을 어긴다 |

### Verified by

- `grep -rn "HideSite\|sidebar_hide_site\|HIDE_SITE_SHORTCUT\|site_hidden" src tests` → 10 hits, 모두 위 표에 포함
- `grep -rn "\.hide(\|\.unhide(" src` → 비시험 3곳, 모두 Investigation Log에 기록
- `grep -rn "release_conn(\|forget_connection\|requeue_site" src` → `release_conn` 앱 호출 2곳(`app.rs:1638`·`:2214`) + 정의 1, `forget_connection`·`requeue_site`는 **앱 호출 0**(각각 시험 1곳: `transfer.rs:895`·`queue.rs:676`). 셋 다 위 표·Deferred·전제 7에 반영
- `grep -rn "close_panel(" src` → 정의 1(`app.rs:273`) + 호출 2(`:1631`·`:2427`), 모두 확인
- `grep -rn "queue.enqueue\|start_transfer(" src/ui` → 비시험 등재 지점 6곳. **늦게 도착해 되살릴 수 있는 비동기 경로는 넷**이다 — ① 확인 답(T3이 막는다) ② `expand_rx`(D2-1) ③ `os_drop_rx`(D2-1) ④ 원격 트리 훑기 답(`src/ui/app/remote.rs:427-445`) — ④는 `release_conn`이 마지막 연결에서 `pending_trees`를 지우므로(`:349-354`) 추가 조치가 필요 없다. 나머지 둘(`remote.rs:101`·`:120`·`app.rs:2187`)은 사용자의 조작 프레임에서 곧바로 도는 동기 경로다

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md` FR-29의 「삭제(Del, 사이드바 바로가기만 제거하고 등록 사이트는 남긴다)」 | 그 구가 이번 동작과 정면으로 어긋난다 — 고치지 않으면 Must FR이 거짓이 된다 | T6 |
| 필수 | `README.md:35`의 「`사이드바에서 숨기기`는 목록에서 감출 뿐」 | 사용자용 서술이 실제와 어긋난다 | T7 |
| 필수 | `src/remote/sites.rs:6-7` 모듈 주석 · `src/ui/sidebar.rs:652-654`(「여기서 지우는 것은 사이드바 바로가기다」)·`:676-679`(파괴색을 쓰지 않는 이유) · `src/i18n/mod.rs:1074` 토스트 주석 | 이번 변경이 유발한 stale 주석(자기 유발이라 이연 불가) | T5(주석 넷 모두 그 task가 여는 파일이다) |
| 필수 | `src/ui/app.rs:900-902`의 「목록에서 감출 뿐 사이트는 남는다 (README §1)」 주석 | 같은 자리에서 처리가 바뀌므로 그대로 두면 거짓이 된다 | T4 |
| 무관 | `docs/prd.md` FR-36(연결별 탭 규칙) | 탭이 서는 규칙(큐 항목 ∩ 필터·연결)은 그대로다 — 이번 변경은 그 **입력**을 없앨 뿐이다 | 건드리지 않음 |
| 무관 | `src/ui/queue_panel.rs:400`의 「저장소에 없는 사이트의 항목도 빠뜨리지 않는다(지운 사이트의 잔여 전송)」 | 사이트 관리자 `삭제(D)` 경로를 가리키는 문장이라 여전히 참이다 | 건드리지 않음 |
| 무관 | `src/remote/site_export.rs`의 숨김 표시 복원 | 저장소 수준 표시만 다루고 실행 중 상태를 건드리지 않는다 | 건드리지 않음 |

## Decisions

### D1. 걷어내기를 `SiteStore::hide`가 아니라 사이드바 처리부에 둔다
- **Options**: A) `SiteStore::hide` 안에서 / B) 사이드바 액션 처리부(`ExplorerApp`)에서
- **Chosen**: B
- **Rationale**: `hide`는 주소창 자동 등록(연결 직전)과 가져오기(숨김 복원)도 부른다 — 거기서 걷어내기가 돌면 방금 연 연결이 끊기고 가져오기가 큐를 지운다. 또 `SiteStore`는 순수 모델이라 연결·큐를 알지 못한다(계층 규약).
- **Source**: `src/ui/app/remote.rs:690`, `src/remote/site_export.rs:398`, AGENTS 「의존은 단방향」

### D2. 걷어내는 순서를 고정한다
- **Options**: A) 큐 비우기 → 연결 회수 / B) 확인 대기 버리기 → 탭 닫기 → 연결 회수 → 큐 비우기 / C) 확인 대기 버리기 → 탭 닫기 → **큐 비우기(연결이 살아 있는 채로)** → 연결 회수 → `forget_connection`
- **Chosen**: C
- **Rationale**: 두 제약이 서로 반대 방향으로 당긴다. ⓐ 연결을 닫으면 그 연결로 물어 둔 확인이 「겹침 없음」으로 **큐에 들어간다**(D10) — 그래서 큐를 나중에 비워야 할 것 같지만, 확인 대기를 **맨 앞에서 버리면**(T3) 그 부활 경로 자체가 없어진다. ⓑ `TransferRunner::cancel`은 `manager.send(conn, Cancel)`로 워커를 멈추고 받기 항목을 `cancelling`에 담는데, **연결이 이미 닫혔으면 그 명령이 갈 곳이 없고** 받다 만 `.part`는 `cancelling`에 들어간 적이 없어 `forget_connection`도 그것을 정리하지 못한다. 그래서 취소·비우기가 연결 회수보다 **앞**이어야 한다. B 순서(초안)는 ⓑ를 어겨 `.part`가 남는다(plan-reviewer B1).
- **Source**: `src/remote/transfer.rs:323-333`(`cancel`)·`:388-403`(`forget_connection`), `src/ui/app/transfer_conflict.rs:117-130`, 위키 `feat-remote-transfer.md:108`

### D2-1. 늦게 도착하는 워커 결과가 큐를 되살리지 못하게 막는다
- **Options**: A) 그냥 둔다 / B) `sites.is_hidden(site)`로 거른다 / C) `detached_sites: HashSet<SiteId>`에 담아 두고 그 사이트의 결과를 버린다
- **Chosen**: C
- **Rationale**: 큐에 넣는 비동기 경로가 셋인데 T3이 막는 것은 하나뿐이다 — 나머지 둘(**올리기 펼치기** `expand_rx`(`src/ui/app.rs:1957-1963`)와 **OS 드롭 스캔** `os_drop_rx`(`:1842-1850`))은 사이트를 보지 않고 큐에 넣으므로, 걷어낸 뒤 다음 프레임에 항목이 되살아나 탭이 다시 선다. B는 못 쓴다 — 주소창으로 처음 붙은 서버도 **숨긴 사이트**라(`remote.rs:690`) 그 서버로의 정상 올리기가 함께 버려진다. C의 표시는 `connect_site`(`src/ui/app/remote.rs:775`)에서 지운다 — 사용자가 그 사이트를 다시 열면 종전대로 돌아야 한다.
- **Source**: `src/ui/app.rs:1957-1963`·`:1842-1850`, `src/ui/app/transfer_conflict.rs:201-219`(plan-reviewer M1)

### D3. 마지막 패널의 유일한 탭이면 로컬 시작 폴더로 되돌린다
- **Options**: A) 그 탭을 남긴다 / B) 로컬 탭으로 되돌린다
- **Chosen**: B (사용자 결정 2026-08-21)
- **Rationale**: FR-2로 마지막 패널은 닫히지 않아 A면 원격 탭이 남고, 사용자가 `다시 연결`을 누르면 큐 탭이 되살아나 요구가 무너진다. 「사이트를 잃은 원격 탭을 로컬로 되돌린다」는 처리는 세션 복원에도 있다(개념상의 선례일 뿐 코드는 재사용하지 않는다 — 그쪽은 탭을 **만드는** 시점의 `TabSpec` 경로다).
- **수단·목적지 확정**: `PanelState`에 새 API를 만들지 않는다. **`new_tab(ctx)`를 부른 뒤 `close_site_tabs`를 한 번 더 부른다** — `new_tab`은 활성 탭이 원격이면 `crate::ui::app::start_dir()`를 가리키는 로컬 탭을 열고(`src/ui/panel.rs:1032-1046`) 그 뒤에는 그 사이트 탭을 모두 닫을 수 있다. 따라서 목적지는 **`start_dir()`**(새 탭 관례와 같은 값)이며 세션 복원의 `home_dir()`이 아니다.
- **Source**: `src/ui/panel.rs:1032-1046`, `src/ui/session.rs:179-190`, `src/ui/app.rs:273-296`(FR-2), plan-reviewer M2

### D4. 진행·대기 중인 전송이 있을 때만 확인을 받는다
- **Options**: A) 언제나 즉시 / B) 진행·대기 전송이 있을 때만 확인 / C) 항상 확인
- **Chosen**: B (사용자 결정 2026-08-21)
- **Rationale**: 요청의 「바로 삭제」를 지키되, 되돌릴 수 없는 손실(도는 전송의 중단)이 실제로 있을 때만 한 번 묻는다. 끝난·실패한 항목만 있으면 묻지 않는다 — 그것들은 다시 걸 수 있는 기록일 뿐이다.
- **Source**: 사용자 답변, `TransferState::is_pending`(`queue.rs:53`)

### D5. 메뉴 문구를 `삭제`로 되돌리고 파괴색 hover를 복원한다
- **Options**: A) `사이드바에서 숨기기` 유지 / B) `삭제` + 파괴색 / C) `삭제`만
- **Chosen**: B (사용자 결정 2026-08-21)
- **Rationale**: 디자인 원본(`:358-360`)·PRD FR-29 문면과 같아진다. 2026-08-16에 파괴색을 뺀 근거는 「되돌릴 수 있는 일」이었는데, 이제 도는 전송이 끊기고 큐가 비워지므로 그 근거가 없어졌다.
- **Source**: `docs/design/design-files/FileExplorer-FTP.dc.html:358-360`, `src/ui/sidebar.rs:676-679`

### D6. 사이트 기록은 사이트 관리자에 남긴다
- **Options**: A) 남긴다 / B) `SiteStore::remove`까지 부른다
- **Chosen**: A (사용자 결정 2026-08-21)
- **Rationale**: 호스트·계정·봉인된 비밀번호가 보존돼 실수로 눌러도 되돌릴 수 있다. 목록에서 기록을 지우는 것은 관리자의 `삭제(D)`라는 FR-27·FR-29의 역할 분담도 그대로 남는다.
- **Source**: 사용자 답변, `docs/prd.md` FR-27·FR-29

### D7. 파괴색 값을 `theme`에 둔다
- **Options**: A) `sidebar.rs`에 상수로 / B) `theme::MENU_HOT_DANGER`
- **Chosen**: B
- **Rationale**: AGENTS의 팝업 메뉴 한 줄 규약이 「행 높이·여백·hover 색을 각 메뉴가 적지 않는다」이고 그 정본이 `ui::theme`다. 값을 메뉴 파일에 박으면 같은 실수가 다시 흩어진다.
- **Source**: AGENTS.md 「팝업 메뉴 한 줄」

### D8. `.part` 정리를 위해 `forget_connection`을 새 경로에서만 부른다
- **Options**: A) `release_conn`에 넣어 모든 경로를 함께 고친다 / B) `detach_site`에서만 부른다
- **Chosen**: B
- **Rationale**: A는 탭 ✕ 경로의 기존 동작까지 바꾸므로 이번 요청 범위를 넘고 회귀 검증이 따로 필요하다. 새 경로는 취소 직후 연결을 닫으므로 이 호출 없이는 `cancelling`에 담긴 `.part`가 영영 남는다 — 그래서 새 경로에는 필수다. **부르는 자리는 `release_conn` 바로 뒤**여야 한다(그 전에 부르면 `assigned`가 먼저 비워져 이어지는 `runner.cancel`이 아무 일도 하지 않는다 — D2의 순서와 함께 지킨다). 기존 경로의 같은 누수는 Deferred에 적었다.
- **Source**: `src/remote/transfer.rs:383-403`, 호출부 전수 검색

## Tasks

- [x] T1. `TransferQueue::site_items` — 사이트 하나의 큐 항목을 골라 준다
  - **Type**: C
  - **Design**: ① `src/remote/queue.rs`의 `impl TransferQueue`, `filter`(`:290`) 옆 ② `site_items(&self, site: SiteId) -> Vec<&TransferItem>` — 그 사이트의 항목을 등록 순서대로 돌려준다 ③ 순수 모델이라 아무것도 참조하지 않고, 앱(`detach_site`)이 지울 번호와 진행 건수를 이 한 번의 훑기에서 얻는다 ④ 「진행 건수만 세는 함수」·「번호만 주는 함수」로 나누지 않는다 — 호출부가 하나뿐이다
  - **Acceptance**: Given 사이트 둘의 항목이 섞인 큐, When `site_items(첫 사이트)`, Then 그 사이트 항목만 등록 순서대로 나오고 다른 사이트 항목은 하나도 섞이지 않는다. 없는 사이트를 물으면 빈 벡터다
  - **Files**:
    - 주: `src/remote/queue.rs`
    - 테스트: `src/remote/queue.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 빈 큐 · 그 사이트 항목이 0건 → 빈 벡터(패닉 없음)
    - 상태가 섞여 있어도(대기·진행·완료·실패) 전부 돌려준다 — 거르는 것은 호출부의 몫이다
  - **Halt Forecast**: 없음 — 순수 모델에 함수 하나를 더한다
  - **Depends on**: -

- [ ] T2. `PanelState::close_site_tabs` — 그 사이트를 가리키는 원격 탭을 모두 닫는다
  - **Type**: D
  - **Design**: ① `src/ui/panel.rs`의 `impl PanelState`, `close_tab`(`:595`) 옆 ② `close_site_tabs(&mut self, site: SiteId, ctx: &egui::Context) -> bool` — **모든 탭이 그 사이트라 마지막 하나를 닫지 못했으면 `true`**다. 닫은 탭이 쓰던 연결은 돌려주지 않는다: 연결 회수는 탭이 아니라 매니저에서 사이트로 고르므로(T4 4단계의 `site_connections`) 그 값을 읽을 곳이 없다 ③ `TabsModel::close`(`panel/tabs.rs:298`)를 그대로 쓰고, 활성 탭이 닫혔으면 `reload_active_tab`을 부른다. 패널을 닫을지 로컬로 되돌릴지는 **앱이 정한다**(패널이 마지막인지 모른다) ④ 「조건으로 탭을 닫는 일반 함수」로 추상화하지 않는다 — 사례가 하나다
  - **Acceptance**: Given 로컬 탭 1 + 사이트 A 탭 2 + 사이트 B 탭 1인 패널, When `close_site_tabs(A)`, Then A 탭 둘이 사라지고 로컬·B 탭은 그대로 남으며 반환은 `false`다. Given 사이트 A 탭만 2개인 패널, When 같은 호출, Then 탭 하나가 남고 반환은 `true`이며 그 남은 탭도 A를 가리킨다
  - **Files**:
    - 주: `src/ui/panel.rs`
    - 테스트: `src/ui/panel/tests.rs`
  - **Edge Cases**:
    - 그 사이트 탭이 없으면 아무것도 하지 않고 `false`
    - 연결이 아직 붙지 않은 원격 탭(`conn: None`)도 닫는다
    - 활성 탭이 닫혀 배경 탭이 활성이 되면 그 탭의 내용을 다시 읽는다(로컬이면 열거, 원격이면 그대로)
  - **Halt Forecast**:
    - (i) 닫는 도중 인덱스가 밀려 엉뚱한 탭이 닫히는 문제 → 매번 **처음부터 다시 찾아** 하나씩 닫는 방식으로 해결(`open_remote_tab_only`와 같은 반복 형태)
  - **Depends on**: -

- [ ] T3. 그 사이트로 오가는 「같은 이름 확인」 대기를 버린다
  - **Type**: C
  - **Design**: ① `src/ui/app/transfer_conflict.rs` ② 순수 함수 `conflict_site(drop: &DropOutcome) -> Option<SiteId>`(올리기는 `target`의 사이트, 받기는 `source_site`)와 `ExplorerApp::drop_site_conflicts(&mut self, site: SiteId)` — `pending_conflicts`·`conflict_queue`·`conflict_dialog`에서 그 사이트 것을 버리고, `conflict_lists`에서 **그 사이트의 연결로 물어 둔 조회**도 지운다(`settle_conflict`를 부르지 않는다 — 부르면 큐에 들어간다) ③ 순수 함수는 `ui::list_common`의 타입만 읽고, 메서드는 앱 상태를 만진다 ④ 「사이트로 거두는 일반 회수기」를 만들지 않는다 — 연결 단위 회수(`abandon_conflict_lists`)는 그대로 남기고 그 옆에 둔다
  - **Acceptance**: Given 사이트 A로 올리는 확인과 사이트 B로 올리는 확인이 함께 대기 중, When `drop_site_conflicts(A)`, Then A의 대기·대화·조회만 사라지고 B의 것은 그대로 남으며 **A의 전송은 큐에 들어가지 않는다**. `conflict_site`는 올리기·받기 양쪽에서 사이트를 옳게 집어낸다(단위 시험)
  - **Files**:
    - 주: `src/ui/app/transfer_conflict.rs`
    - 동반: `src/ui/app.rs`(필드 접근만 — 변경 없음)
    - 테스트: `src/ui/app/transfer_conflict.rs`의 `#[cfg(test)] mod tests` (`conflict_site` 판정)
  - **Edge Cases**:
    - 대기·대화·조회가 하나도 없으면 아무 일도 하지 않는다
    - 로컬↔로컬 드롭은 사이트가 없다(`None`) — 버리지 않는다
    - 대화가 그 사이트 것이면 화면에서 즉시 사라진다(사용자가 답할 대상이 없어졌다)
  - **Halt Forecast**:
    - (i) `ExplorerApp` 메서드라 시험할 수 없다 → 판정을 순수 함수로 뽑아 그것만 시험하고, 나머지는 수동 검증 항목으로 남긴다
  - **Depends on**: -

- [ ] T4. `ExplorerApp::detach_site` — 사이드바 삭제가 연결·탭·큐를 함께 걷어낸다
  - **Type**: D
  - **Design**: ① 걷어내기는 `src/ui/app/remote.rs`(연결·사이트 소관), 확인 대화와 사이드바 처리는 `src/ui/app.rs` ② 신규 `detach_site(&mut self, site, area, ctx)`가 **D2의 순서 그대로** 실행한다:
      1. `drop_site_conflicts(site)` — 확인 대기·대화·조회를 버린다(T3)
      2. 모든 워크스페이스의 패널에서 `close_site_tabs(site, ctx)`(T2). `all_tabs`인 패널은 `view.close_panel(id, area)`로 닫는다. `close_panel`은 반환값이 없으므로 **닫혔는지는 그 뒤 `view.panels.contains_key(&id)`로 판정한다**(`Command::ClosePanel`이 쓰는 것과 같은 방법 — `src/ui/app.rs:1633`). 닫히지 않았으면(FR-2의 마지막 패널) `panel.new_tab(ctx)` 후 `close_site_tabs`를 한 번 더 부른다(D3)
      3. `site_items(site)`(T1)로 번호를 모아 `runner.cancel(&manager, id)`을 돌린 뒤 `queue.remove(&ids)` — **연결이 살아 있는 동안** 해야 Cancel 명령이 워커에 닿는다(D2 ⓑ)
      4. `site_connections(site)`의 연결마다 `release_conn(conn)` → 곧바로 `runner.forget_connection(conn)`(D8)
      5. `detached_sites.insert(site)`(D2-1) · `dock.site`가 그 사이트면 `None` · `persist_session()`
    그 밖의 신규: `site_connections(&self, site) -> Vec<ConnectionId>`(`site_connection`:286의 복수형), `detached_sites: HashSet<SiteId>`·`pending_site_remove: Option<SiteId>` 필드, `show_site_remove_confirm(&mut self, ctx, area)`. `expand_rx`(`app.rs:1957`)·`os_drop_rx`(`:1842`) 소비 지점에 `detached_sites` 검사를 넣고, `connect_site`(`remote.rs:775`)에서 그 표시를 지운다 ③ `detach_site`는 T1~T3이 만든 것과 기존 `release_conn`을 부르는 조율자다 — 절차를 스스로 복제하지 않는다 ④ 「사이트 정리기」 같은 타입도, 탭을 로컬로 바꾸는 신규 `PanelState` API도 만들지 않는다(기존 `new_tab`으로 된다)
  - **Acceptance**: Given 사이트 A에 연결이 열려 있고 원격 탭과 큐 항목이 있는 상태, When 사이드바에서 A를 지우고(진행·대기 전송이 있으면 확인에서 `삭제`) 프레임이 한 번 돌면, Then ⓐ A의 연결이 모두 닫히고 ⓑ A를 가리키던 원격 탭이 모두 닫히며(마지막 패널의 유일한 탭이면 `start_dir()`를 가리키는 로컬 탭이 그 자리에 선다) ⓒ A의 큐 항목이 하나도 남지 않고 **그 뒤 프레임에도 되살아나지 않으며**(펼치기·OS 드롭 스캔의 늦은 결과가 버려진다) ⓓ 도크의 연결별 탭에서 A가 사라진다(고른 탭이 A였으면 `전체`로 돌아간다). 진행·대기 전송이 0건이면 확인 없이 곧바로 실행된다. 빌드 경고 0 · 기존 시험 전부 통과
  - **참고(중간 상태)**: 이 task가 끝난 시점에는 동작만 바뀌고 메뉴 문구는 아직 `사이드바에서 숨기기`다 — 문구·파괴색은 T5가 맡는다(빌드·시험은 그대로 통과한다)
  - **Files**:
    - 주: `src/ui/app/remote.rs`, `src/ui/app.rs`
    - 동반: `src/i18n/mod.rs`(확인 대화 문구 3종 + 토스트 문구 교체 `site_hidden`→`site_removed`), `src/ui/sidebar.rs`(`SidebarAction::HideSite`→`RemoveSite` 이름 변경)
    - 테스트: 없음 — `ExplorerApp`은 실 창 핸들이 필요해 시험 대상이 아니다(AGENTS). 판정 부품은 T1~T3에서 시험한다
  - **Edge Cases**:
    - 연결이 하나도 없는 사이트 · 큐가 빈 사이트 → 확인 없이 사이드바에서만 사라진다
    - 그 사이트의 연결이 여럿(탐색 1 + 전송 2) → 전부 닫는다
    - 확인 대화가 떠 있는 사이에 대상이 사라지면 대기를 풀고 아무 일도 하지 않는다
    - 첫 프레임이라 배치(`area`)를 모르면 그 프레임에는 실행하지 않고 다음 프레임에 한다 — 임의의 영역을 지어내 패널을 닫지 않는다
    - 확인 대화가 떠 있는 동안 단축키가 뒤에서 실행되지 않게 `src/ui/app.rs:2073`의 모달 조건에 이 대기를 더한다
    - 받다 만 `.part`는 `forget_connection`이 정리 목록으로 넘긴다(즉시 못 지우면 다음 기회에 다시 시도한다)
    - `pump_relist`의 재조회 표시가 그 사이트에 남아 있어도 해가 없다 — 그 사이트를 보는 패널이 없어 조회가 나가지 않고 다음 `take_ready`에서 목록에서 빠진다(`src/ui/app/remote.rs:559-568`·`:820-856`)
    - 폴더를 펼치는 중이거나 OS 드롭을 재는 중에 지우면 그 결과가 **다음 프레임에** 온다 → `detached_sites` 검사로 버린다(D2-1). 그 사이트를 다시 연결하면 표시가 풀려 종전대로 큐에 들어간다
    - **보이지 않는 워크스페이스의 패널**도 대상이다. 그 뷰의 패널을 닫을 때 활성 워크스페이스의 `area`를 쓴다 — `close_panel`이 그 값으로 「다음 활성 패널」만 고르고 창은 하나이므로 결과가 어긋나지 않는다(보이지 않는 뷰의 활성 패널 선택이라 화면에 영향도 없다)
  - **Halt Forecast**:
    - (i) 걷어낸 뒤 항목이 되살아나는 문제 → D2 순서로 해결
    - (i) 패널을 닫을 `area`가 필요한 문제 → `handle_sidebar`가 이미 받고 있고(전제 4), 대화 경로는 `update` 말미의 `layout_area`를 쓴다
    - (ii-a) `handle_sidebar`에 `ctx` 인자 추가 · `SidebarAction` 변형 이름 변경(계획된 시그니처 변경) → `## 사전 승인 항목`
  - **Depends on**: T1, T2, T3

- [ ] T5. 사이드바 메뉴를 `삭제`로 되돌리고 파괴색 hover를 복원한다
  - **Type**: C
  - **Design**: ① `src/ui/theme.rs`에 `MENU_HOT_DANGER`(#C42B1C — 디자인 `:359`), 사용은 `src/ui/sidebar.rs::show_site_context_menu` ② 신규 심볼은 그 상수 하나 ③ `sidebar`가 `theme`를 참조하는 기존 방향 그대로 ④ 「파괴적 메뉴 항목 위젯」을 만들지 않는다 — 사례가 하나라 그 자리에서 hover 색만 덮는다
  - **Acceptance**: Given 사이드바 사이트 행 우클릭 메뉴, When 그리면, Then 문구가 `삭제`(영어 `Delete`)이고 오른쪽에 `Del`이 서며 그 줄의 hover 배경이 `#C42B1C`다. `i18n::sidebar_hide_site`는 카탈로그에서 사라지고 남은 사용처가 없다(`cargo build` 경고 0). 사이드바 문구 시험이 새 이름·문구로 통과한다
  - **Files**:
    - 주: `src/ui/sidebar.rs`, `src/ui/theme.rs`
    - 동반: `src/i18n/mod.rs`(`sidebar_hide_site` 제거 · 토스트 doc 주석), `src/remote/sites.rs`(모듈 주석 `:6-7` — 숨기기·삭제 서술을 실제와 맞춘다)
    - 테스트: `src/ui/sidebar.rs`의 `연결_섹션_문구는_인벤토리_원문_그대로다`(상수 이름·문구), 신규 1건(메뉴 문구가 `삭제`로 그려지는지)
  - **Edge Cases**:
    - 영어 UI에서도 문구가 `Delete`로 나온다(카탈로그 왕복)
    - hover 색을 그 줄에만 덮고 메뉴의 다른 줄·다른 팝업은 종전 `MENU_HOT`을 쓴다
  - **Halt Forecast**:
    - (i) `theme` 소스 훑기 시험(팝업 메뉴 규약)이 새 상수 때문에 깨질 가능성 → 그 시험은 「팝업을 여는 구문 수 ≤ 공통 경로 호출 수」를 보므로 `menu_style` 호출을 유지하면 그대로 통과한다(`src/ui/theme.rs`의 시험 본문 확인 후 진행)
  - **Depends on**: T4

- [ ] T6. PRD FR-29 문면 개정
  - **Type**: A
  - **Acceptance**: `docs/prd.md` FR-29의 「삭제(Del, 사이드바 바로가기만 제거하고 등록 사이트는 남긴다)」가 새 동작(연결 해제 · 원격 탭 닫기 · 큐 항목 제거 · 진행·대기 전송이 있으면 확인 · 사이트 기록은 관리자에 남음)을 서술하도록 바뀌고, `## 결정 이력`에 2026-08-21 항목이 더해진다. FR-27·FR-36 문면은 그대로다
  - **Files**:
    - 주: `docs/prd.md`
  - **Edge Cases**: 없음 — 문면 개정
  - **Halt Forecast**:
    - (ii-a) 승인된 PRD의 Must FR 개정 → `## 사전 승인 항목`
  - **Depends on**: T4

- [ ] T7. README 서술 갱신
  - **Type**: A
  - **Acceptance**: `README.md`의 원격 연결 절(`:35`)에서 「`사이드바에서 숨기기`는 목록에서 감출 뿐」이 새 동작 서술로 바뀐다 — 지우면 연결이 끊기고 그 사이트의 원격 탭·전송 큐 항목이 함께 사라지며, 사이트 기록은 사이트 관리자에 남는다는 것까지. 존재하지 않는 기능은 적지 않는다
  - **Files**:
    - 주: `README.md`
  - **Edge Cases**: 없음
  - **Halt Forecast**: 없음
  - **Depends on**: T4

## 사전 승인 항목 (일괄 승인 대상)

- T4 — `ExplorerApp::handle_sidebar`에 `ctx` 인자 추가(비공개 메서드, 호출부 1곳)와 `SidebarAction::HideSite` → `RemoveSite` 이름 변경(호출부 2곳). 계획된 시그니처 변경이다.
- T4 — `ExplorerApp`에 상태 필드 2개(`pending_site_remove`·`detached_sites`) 추가.
- T6 — 승인된 PRD의 Must FR(FR-29) 문면 개정. 이 변경 없이는 PRD가 실제와 어긋난다.

## 불가피한 Halt (위임 불가)

- commit 이후의 push·태그·릴리즈 — 이 plan의 범위 밖이며 각 지점에서 따로 승인받는다.
- 구현 중 「사이트 기록까지 지워야 한다」처럼 D6을 뒤집는 방향 전환이 필요해지는 경우 — plan에 없던 결정이라 사용자에게 묻는다.

## Verification Strategy

- 빌드: `cargo build`
- 단위·통합 시험: `cargo test` (직전 기준선 956건 통과)
- 린트: `cargo clippy --all-targets -- -D warnings`
- 형식: `cargo fmt --check`
- 수동 검증(HUMAN-VERIFY — 실 창이 필요해 자동화 대상이 아니다):
  1. 사이트에 연결해 원격 탭을 열고 파일 전송을 건 뒤, 사이드바에서 그 사이트를 지운다 → 확인 대화가 뜨고 `삭제`를 누르면 도크의 연결별 탭에서 그 사이트가 **즉시** 사라지는가.
  2. 전송이 하나도 없는 사이트를 지운다 → 확인 없이 사라지는가.
  3. 원격 탭이 창의 유일한 패널의 유일한 탭일 때 지운다 → 로컬 시작 폴더 탭이 그 자리에 서는가.
  4. 우클릭 메뉴의 `삭제` 줄에 마우스를 올리면 빨간 배경이 칠해지는가.
  5. 받는 중이던 파일의 `.part`가 남지 않는가.

## Phase Ledger

## Retry Ledger

## Progress Log

## Next Steps

- 권장 다음 액션: 승인 후 `pjc:implement-task`로 T1부터 실행

## Open Questions

- [x] Q1: 어느 삭제 경로인가 → **사이드바 우클릭**(사이트 관리자 `삭제(D)` 아님)
- [x] Q2: 큐 항목·진행 중 전송 처리 → **취소 + 전부 제거**
- [x] Q3: 열린 원격 탭 → **연결 끊고 닫는다**
- [x] Q4: 메뉴 문구 → **`삭제` + 파괴색 hover 복원**(D5)
- [x] Q5: 확인 대화 → **진행·대기 전송이 있을 때만**(D4)
- [x] Q6: 마지막 패널의 유일한 탭 → **로컬 시작 폴더 탭으로 되돌린다**(D3)
- [x] Q7: 사이트 기록 → **사이트 관리자에 남긴다**(D6)
