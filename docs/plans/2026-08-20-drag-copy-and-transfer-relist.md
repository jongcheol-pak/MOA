# Plan: 드래그 복사 확장 · 전송 후 목록 갱신

**PRD**: `docs/prd.md`

## 요구 이해

- **원문 요청**: "1. ftp에 파일 전송 완료 후 새로 고침이 되지 않아 목록에 표시가 되지 않음.(새로고침하면 표시됨) 2. 탭 -> 탭으로 파일 및 폴더 드레그 시 복사가 되지 않는데 복사가 되도록 수정.(탭에서 ftp로는 잘 됨) 3. 바탕화면이나 윈도우 탐색기에서 파일 및 폴더를 탭 또는 ftp 에 드레그해도 복사가 되지 않음. 복사가 되도록 수정."
- **이해한 요구**: ① 원격으로 전송이 끝나면 그 폴더를 보고 있는 원격 목록이 **저절로** 갱신돼야 한다(지금은 손으로 새로 고쳐야 보인다). ② 로컬 탭끼리 파일·폴더를 끌어다 놓으면 **복사**돼야 한다(지금은 아무 일도 일어나지 않는다). ③ 바탕화면·윈도우 탐색기에서 끌어온 파일·폴더를 로컬 탭에 놓으면 복사, 원격(FTP) 탭에 놓으면 올리기가 돼야 한다. ②③은 PRD가 명시적으로 제외해 둔 항목이라 **PRD 개정이 함께 필요**하다.
- **추가로 채택된 것**(승인 질의 결과): 드롭 대상 패널 강조, **앱 → 탐색기로 끌어내기**(요청의 반대 방향 — 사용자가 이번 회차에 포함하기로 골랐다).
- **포함하지 않는 것으로 이해**: 원격 → 원격 드래그(같은·다른 서버끼리)는 이번에도 제외다. 이동(잘라내기)도 만들지 않는다 — 드래그는 **언제나 복사**다.

## Goal

로컬 탭끼리·OS 탐색기와 주고받는 드래그 복사를 열고, 원격 전송이 끝나면 그 폴더의 목록이 저절로 다시 읽히게 한다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-37 (문면 개정 — 전송 완료 후 목적지 목록 재조회) | Must | T1·T2 | ✅ 커버 |
| FR-60 (신설 — 로컬 패널 사이 드래그 복사) | Should | T1·T3·T4 | ✅ 커버 |
| FR-61 (신설 — OS 탐색기와 주고받는 드래그) | Should | T1·T5·T6·T7 | ✅ 커버 |
| Out of Scope 재한정(라인 114) | — | T1 | ✅ 커버 |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- **원격 → 원격 드래그**(같은 서버 안·서버끼리). PRD의 기존 제외를 그대로 둔다 — 서버 간 전송은 로컬을 경유해야 해서 성격이 다르다(2026-08-20 사용자 결정).
- **이동(잘라내기) 드래그**. 드래그는 언제나 복사이며 `Shift`·`Ctrl` 보조키로 동작을 가르지 않는다(2026-08-20 사용자 결정 — 요청 원문이 "복사"였고, 같은 드라이브에서 이동이 기본이면 원본이 사라지는 사고가 난다).
- **잘라내기/붙여넣기**(클립보드). PRD 라인 114의 그 제외는 그대로 둔다.
- **가상 파일 드롭**(Outlook 첨부·압축 파일 안의 항목 등 `CF_HDROP`이 아닌 것). winit의 드롭 처리가 `CF_HDROP`만 읽으므로 원리상 받지 못한다.

## Deferred / Follow-up

- **원격 항목을 탐색기로 끌어내기** — T7은 로컬 항목만 OLE 드래그로 내보낸다. 원격 항목은 「끌기 시작 시점에는 아직 로컬에 파일이 없다」라 지연 렌더링(`CFSTR_FILEDESCRIPTOR`/`CFSTR_FILECONTENTS`)을 구현해야 하며, 이번 범위의 몇 배다.
- **드래그 중 미리보기 그림**(끌고 있는 항목의 반투명 썸네일). `IDragSourceHelper`가 필요하다.
- **`ExplorerApp`의 채널·상태 필드가 이번 회차에 더 늘었다** — `relist`(T2)·`copy_tx`/`copy_rx`(T4)·외부 드롭의 `is_dir` 측정 채널(T5). 대장의 `[SUGGEST] 충돌 상태 6개 필드를 ConflictState로 묶기`(2026-08-20)와 대상이 겹치지는 않지만(그 항목은 `conflict_*` 한정) **같은 범주가 커졌다**는 것을 다음 회차가 알아야 한다. 캡슐화를 다룰 때 이번에 는 것까지 함께 본다.
- **[SUGGEST] `views → panels` 이중 순회가 세 곳에 반복**(`src/ui/app/remote.rs`의 `list_moved_panels`·`request_remote_list`·`pump_relist`) — 공통화 문턱 3회에 닿았다. 조건만 받는 순회 헬퍼로 묶을 수 있다(T2 quality 리뷰 S1).
- 직전 회차에서 이월된 항목 셋(다음 회차 대상): 내보내기 진행 표시 · 내보내기 기본 파일 이름의 앱 이름 표기 · `remote::connection::tests::늦게_도착한_이전_세대의_목록은_버려진다`의 간헐 실패.

## Investigation Log

- **T1 근본 원인 확정**: `src/ui/app/remote.rs:448`의 `ConnEvent::TransferDone`은 `runner.on_done`(큐 상태 반영)만 하고 목록을 다시 청하지 않는다. 같은 파일 `:490`의 `OpDone`은 `OpOutcome::Relist → request_remote_list`로 이어진다. `request_remote_list` 호출부를 전수 조사했고(6건 — `remote.rs:122·369·490·540·552`, `app.rs:1672`) 전송 경로에서 부르는 곳이 없다.
- **로컬 쪽은 이미 갱신된다**: `src/ui/panel.rs:399`의 `watch`가 표시 중인 폴더에 `DirWatcher`를 걸고 `:415 poll_watch`가 프레임마다 거둔다. 받기가 끝날 때의 `.part` → 본이름 이름 바꾸기도 파일시스템 이벤트라 잡힌다. 그래서 증상이 원격에만 난다(사용자 보고와 일치).
- **목적지 정보는 이미 큐에 있다**: `TransferItem`(`src/remote/queue.rs:64`)이 `site`·`direction`·`remote`를 들고, 완료 후에도 항목이 큐에 남는다(`on_done`은 `queue.update(id, Done)`만 한다 — `src/remote/transfer.rs:314`). `TransferQueue::get`(`queue.rs:160`)으로 읽을 수 있어 **`TransferRunner::on_done`의 시그니처를 바꾸지 않아도 된다**.
- **재조회 대상은 연결이 아니라 「사이트 + 폴더」다**: 한 사이트가 탭마다 연결을 따로 열고(`connect_site` — `remote.rs:723`, 호출마다 연결 하나), 전송은 그 사이트의 **노는 아무 연결**에 배정된다(`start_ready` — `transfer.rs:206~216`은 `connection.site`로만 묶는다). 그래서 전송을 처리한 연결이 패널의 연결과 다를 수 있고, 연결로 거르는 기존 `request_remote_list(conn)`을 그대로 쓰면 갱신이 새어 나간다.
- **로컬↔로컬은 버그가 아니라 미구현**: `drop_direction`(`src/ui/list_common.rs:111`)이 Local→Local을 의도적으로 `None`으로 돌리고 시험 두 건(`list_common.rs:233·234`)이 그것을 고정한다. `docs/prd.md:114`가 「로컬↔로컬 파일 이동·복사 드래그, OS(탐색기)와 주고받는 드래그」를 명시적 제외로 둔다.
- **파일 복사 엔진이 레포에 없다**: `grep -rni "SHFileOperation|IFileOperation|CopyFile|fs::copy"` 결과 2건뿐이며 둘 다 설정 파일 마이그레이션(`app/settings.rs:479`·`remote/hostkey.rs:137`)이다.
- **OS 드래그는 이미 켜져 있다**: winit의 Windows `drag_and_drop` 기본값이 `true`(`winit-0.30.13/src/platform_impl/windows/mod.rs:49`)이고, 그때 `OleInitialize`를 부르는데 MTA일 때만 패닉한다(`window.rs:1168~1178`). 이 앱은 `CoInitializeEx(COINIT_APARTMENTTHREADED)`(`src/ui/app.rs:110`)라 충돌하지 않는다. 앱이 `dropped_files`를 한 번도 읽지 않을 뿐이다(`grep -rni "dropped_files|hovered_files" src/` → 0건).
- **한 번의 드롭은 한 프레임에 모두 온다**: winit의 `iterate_filenames`가 `HDROP`의 모든 경로를 한 `Drop()` 안에서 이벤트로 내보낸다(`drop_handler.rs:143`, `:180~193`). egui-winit이 그것을 `egui_input.dropped_files`에 쌓는다(`egui-winit-0.35.0/src/lib.rs:457`).
- **드롭 이벤트에 커서 좌표가 없다**: `egui::DroppedFile`에 `path`만 채워지고(`lib.rs:459`) OLE 드래그 중에는 `WM_MOUSEMOVE`가 오지 않아 egui의 `hover_pos`도 낡은 값이다. Win32 `GetCursorPos` + `ScreenToClient`로 직접 재야 한다.
- **패널 사각형은 이미 계산돼 있다**: `splitter::show_layout`이 `tree.compute_rects(...)`로 `computed.panes: Vec<(PanelId, LayoutRect)>`를 얻어 그리기에 쓴다(`src/ui/splitter.rs:187·199`). 위로 올리지 않을 뿐이다.
- **중첩 모달 루프는 이 앱의 확립된 방식이다**: 셸 컨텍스트 메뉴가 `TrackPopupMenuEx`(자체 메시지 루프)를 **그리기가 모두 끝난 뒤** `update()` 말미에서 부른다(`src/ui/app.rs:1988~1994`, 주석에 그 이유가 적혀 있다). 파일 대화도 같은 자리에서 셸 메뉴와 겹치지 않게 미룬다(`:1998~1999`). `DoDragDrop`도 같은 성질이라 같은 자리·같은 규칙을 따른다.
- **필요한 Win32 바인딩은 대부분 이미 켜져 있다**: `IFileOperation`(`windows-0.62.2/src/Windows/Win32/UI/Shell/mod.rs:23135`)·`SHCreateDataObject`(`:2591`)·`SHCreateItemFromParsingName`(`:2678`)은 `Win32_UI_Shell`에, `CoCreateInstance`는 `Win32_System_Com`에 있고 둘 다 `Cargo.toml`에 있다. **`DoDragDrop`·`IDropSource`만 `Win32_System_Ole`**(`Ole/mod.rs:107`·`:5984`)이라 feature 하나를 더해야 한다(신규 crate 아님). `windows::core::implement` 매크로는 feature 게이트 없이 재수출된다(`windows-core-0.62/src/lib.rs:58`).
- 위키 참조: `20_projects/personal/moa/feat-remote-transfer` — 전송 3계층(큐=순수 모델 / 실행기=배정·`.part` / 워커=바이트)과 **세대 번호 공간을 용도별로 나누는 규약**(패널 목록·트리 캐시·같은 이름 확인) 확인. T2의 재조회는 **패널 목록 경로를 그대로 재사용**하므로 새 번호 공간이 필요 없다. 같은 페이지의 「알려진 미배선 지점」(`requeue_site`·`forget_connection` 미호출)은 이번 변경과 인과가 없다.
- 위키 참조: `20_projects/personal/moa/decisions.md` — 로컬↔로컬·OS 드래그를 과거에 기각·보류한 결정이 없다. Out of Scope를 푸는 선례는 즐겨찾기(2026-08-16)·사이트 목록 내보내기(2026-08-20) 둘이며 둘 다 「제외 해제 + FR 신설」 형태였다.
- Deferred 대장 조회(`docs/plans/deferred.md`, `## 대기` 67건(실측)): **반증 1건** — 「egui의 끌기 판정을 시험에서 재현하는 방법」(2026-08-18 등록, 두 차례 실측 실패)이라 드래그의 **화면 쪽 동작은 자동 시험으로 고정할 수 없다**. 이 plan의 acceptance는 그 경계를 지켜 순수 로직만 시험으로 묶고 화면은 수동 검증에 둔다. 관련 항목 둘 더 — 「올리기 충돌 확인의 원격 목록 신선도」(T2와 한 뿌리이나 조회~전송 사이의 변경은 여전히 알 수 없다)·「숨김 토글이 원격 재조회를 부른다」(재조회 비용 인식). 잔량 67건은 소진 batch 임계(100건) 미만이라 batch task를 넣지 않는다.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 전송 완료 시 목적지 원격 폴더를 큐에서 읽을 수 있다 | `src/remote/queue.rs:64~72`(`TransferItem`), `:160`(`get`), `src/remote/transfer.rs:314`(완료해도 항목이 남는다) | ✅ |
| 2 | 한 사이트에 연결이 여럿일 수 있어 전송 연결 ≠ 패널 연결 | `src/ui/app/remote.rs:723`(호출마다 연결 하나), `src/remote/transfer.rs:206~216`(사이트로만 묶어 배정) | ✅ |
| 3 | winit이 이 앱에 파일 드롭 이벤트를 이미 보낸다 | `winit-0.30.13/.../windows/mod.rs:49`(기본 `true`), `window.rs:1168~1178`(STA면 통과), `src/ui/app.rs:110`(STA) | ✅ |
| 4 | 한 번의 드롭에서 여러 경로가 같은 프레임에 도착한다 | `winit-0.30.13/.../windows/drop_handler.rs:143·180~193`, `egui-winit-0.35.0/src/lib.rs:457` | ✅ |
| 5 | 드롭 이벤트만으로는 놓인 자리를 알 수 없다 | `egui-winit-0.35.0/src/lib.rs:457~461`(`path`만 채운다) | ✅ |
| 6 | 패널별 사각형이 프레임마다 계산돼 있다 | `src/ui/splitter.rs:187`(`compute_rects`), `:199`(`computed.panes` 순회) | ✅ |
| 7 | `update()` 말미의 중첩 모달 루프가 이 앱에서 동작한다 | `src/ui/app.rs:1988~1994`(셸 메뉴가 그 방식으로 이미 출하됨, FR-8) | ✅ |
| 8 | `IFileOperation`·`DoDragDrop` 바인딩을 새 crate 없이 쓸 수 있다 | `windows-0.62.2/.../UI/Shell/mod.rs:23135·2591`, `.../System/Ole/mod.rs:107·5984`, `Cargo.toml` feature 목록, `windows-core-0.62/src/lib.rs:58` | ✅ (`Win32_System_Ole` feature 추가 필요) |
| 9 | 드래그의 화면 동작은 자동 시험으로 고정할 수 없다 | `docs/plans/deferred.md`의 2026-08-18 항목(두 차례 실측 실패) | ✅ |
| 10 | feature만 더하면 라이선스 자산 지문이 바뀌지 않는다 | 지문 대조는 `Cargo.lock` 기준(AGENTS.md 「라이선스 자산 재생성」)이고 feature는 `Cargo.lock`에 기록되지 않는다 | ⚠ 미확인 — T7의 `cargo test`가 곧 판정한다(붉어지면 `cargo run --example gen_licenses`를 T7 안에서 돌린다). 성립을 좌우하지 않는다 |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| `DoDragDrop`이 이벤트 루프를 재진입시켜 드래그 중 창이 굳는다 | T7만 실패 | 셸 메뉴와 **같은 자리·같은 상호 배제 규칙**(그리기 종료 후, 셸 메뉴·파일 대화가 뜬 프레임에는 미룸)으로 붙인다. T7을 마지막 task로 두어 실패해도 T2~T6이 온전하다 |
| `IFileOperation`이 UI 스레드를 막는다 | 복사 중 앱이 굳는다 | 워커 스레드에서 자체 STA로 돌린다(`fs/thumbnail.rs:277`·`fs/drives.rs:72`의 기존 패턴). UI 스레드는 결과만 채널로 받는다 |
| 대량 전송에서 재조회가 서버를 때린다 | 서버 부하·UI 잠식 | 「사이트별 대기·진행이 0일 때 + 2초 간격 상한」으로 합친다(D3) |
| 드래그 동작을 자동 시험으로 확인할 수 없다 | 회귀를 시험이 못 잡는다 | 순수 판정 로직(대상 패널 고르기·재조회 시점·경로 조립)을 UI에서 떼어내 그 부분만 시험으로 고정하고, 화면은 Verification Strategy의 수동 절차로 남긴다 |
| 탐색기에서 끌어온 경로가 원격 패널 위에 놓였는데 그 사이트에 연결이 없다 | 조용히 아무 일도 안 일어난다 | 기존 `start_transfer`가 연결 없으면 확인을 건너뛰고 큐에 넣는다(D10 규약) — 큐에 들어가 연결되면 나간다. 큐 등록 자체가 화면 신호다 |
| 물리 픽셀(Win32 커서)과 논리 pt(egui 사각형)를 섞어 배율 125%·150% 화면에서 대상 패널이 어긋난다 | T5·T6이 엉뚱한 패널에 복사·강조한다 | 환산을 `client_px_to_pt` 순수 함수로 떼어 배율 1.25·1.5 시험으로 고정하고, 수동 검증 4-1에 배율 변경을 넣는다 |
| 셸 복사가 실패·취소됐는데 아무 데도 안 보인다 | 사용자가 복사된 줄 안다 | T4가 `CopyOutcome`을 거둬 기존 `notice` 알림으로 올린다(성공은 셸 대화가 이미 알리므로 침묵) |
| 셸 복사 대화의 소유자 창은 UI 스레드 것인데 작업은 워커 STA에서 돈다 | 대화가 뜨는 동안 MOA 창이 굳거나 대화가 뒤로 숨을 수 있다 | 레포에 선례가 없어 **실증 근거가 없다**(4-D의 grep 0건). 수동 검증 3에서 「복사 대화가 뜬 동안 MOA 창이 응답하는가」를 확인하고, 문제가 나면 소유자를 넘기지 않는 것(`owner`에 `None`)부터 시도한다 — 소유자는 대화 위치·모달성에만 쓰이므로 빼도 복사 자체는 돈다 |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `ConnEvent::TransferDone` 처리부 | `src/ui/app/remote.rs`(:448) | 로직 추가 — 목적지를 재조회 대기로 표시 |
| `ExplorerApp` 필드 | `src/ui/app.rs`(구조체 정의·`new`) | 필드 추가 — `relist: RelistPending`(T2), `copy_tx`/`copy_rx`(T4 — 셸 복사 결과 수신), 외부 드롭의 `is_dir` 측정 결과 채널(T5) |
| `i18n::dynamic` | `src/i18n/mod.rs`(:615~ 모듈), `src/i18n/mod.rs`의 `mod tests` | 함수 2개 추가(`local_copy_failed`·`local_copy_cancelled`) — 값이 끼어드는 문구라 `strings!` 매크로가 아니라 손수 쓴 함수(AGENTS 「화면 문구」) |
| `ShellHost` | `src/ui/shell_host.rs`(:24 구조체), 호출부는 `src/ui/app.rs` | 메서드 1개 추가(`cursor_client_pos`) + 순수 헬퍼 `client_px_to_pt` — 기존 `to_screen`(:59)의 역방향 |
| `list_common::drop_direction` | `src/ui/app/transfer_conflict.rs`(:35·68·191·226), `src/ui/list_common.rs`(:111 정의, :221·225·233·234 시험) | **변경 없음** — 로컬↔로컬은 새 판정 함수로 가른다(D4) |
| `list_common::DropOutcome` | `src/ui/panel.rs`(:1581 생성), `src/ui/app/remote.rs`(:97·116), `src/ui/app.rs`(:1935), `src/ui/app/transfer_conflict.rs`(전역) | 변경 없음 — 그대로 쓴다 |
| `splitter::LayoutOutcome` | `src/ui/splitter.rs`(:51 정의, :185·328·360·384·437·471·502·538 시험), `src/ui/app.rs`(:1871~1917 소비부) | 필드 1개 추가(`pane_rects`) + 소비부 1줄. **`show_layout`의 인자는 늘지 않는다**(T6 Design ③ — 그 함수가 `pane_rects`를 반환하므로 인자로 되먹이면 순환이다) |
| `ExplorerApp::start_transfer` | `src/ui/app.rs`(:1935), `src/ui/app/remote.rs`(:97·116), `src/ui/app/transfer_conflict.rs`(:27 정의) | 호출부 1곳 추가(T5의 탐색기 드롭) — 시그니처 불변 |
| `ExplorerApp::request_remote_list` | `src/ui/app/remote.rs`(:547 정의, :122·369·490 호출) | 변경 없음 — 사이트+폴더로 고르는 새 함수를 옆에 둔다 |
| `fs` 모듈 목록 | `src/fs/mod.rs` | `pub mod file_op;` 한 줄 추가 |
| `Cargo.toml` windows features | `Cargo.toml` | `Win32_System_Ole` 추가(T7) |

### 4-B. 계약·직렬화 변경

- 직렬화 형식(`settings.json` 세션 스키마) 변경 **없음**. 재조회 대기 상태·복사 진행은 전부 휘발성이라 세션에 담지 않는다.
- 공개 API 변경 둘: `LayoutOutcome`에 필드 추가(구조체가 `Default` 파생이라 기존 생성부는 그대로 컴파일된다), `fs::file_op` 신규 모듈. 둘 다 crate 내부이며 외부 계약이 아니다.

### 4-C. 테스트 파일

- `src/ui/list_common.rs`(`mod tests`) — 새 판정 함수 시험 추가. 기존 `같은_쪽끼리_끌면_아무_일도_없다`(`list_common.rs:231`)는 **단언을 그대로 두고 주석의 근거만 고친다**(`drop_direction`이 전송 방향만 답하는 계약은 유지 — D4. 「PRD Out of Scope다」라는 사유가 T1의 개정으로 틀려진다).
- `src/ui/splitter.rs`(`mod tests`) — **기존 시험 8건은 전부 `merge_panel_outcome`만 부르고 `show_layout`을 부르는 시험은 하나도 없다**(호출부는 `app.rs:1871` 하나뿐이고, 부르려면 `Ui`+`PanelState`+`IconCache`+`IconTextures`+`RemoteView`+`TransferTargets` 하네스를 새로 세워야 한다). 그래서 `show_layout` 자체를 시험하지 않고 **순수 부분을 떼어 시험한다**(T5·T6의 Design 참조). `computed.panes` → `pane_rects` 대입 홉은 `app.rs:1911`의 `drive_observed`와 같은 성질이라 **리뷰가 지키는 자리**로 둔다(그 주석이 이미 같은 판단을 적어 두었다).
- `src/ui/panel/tests.rs` — 원격 목록 재조회 관련 기존 시험 6건(:957·2169·2202·2213·2232·2265) 영향 없음(패널 쪽 API 불변).
- 신규: `src/fs/file_op.rs`(`mod tests`) — 경로 조립·인자 검증만. 실제 셸 복사는 COM·UI라 시험 비대상(AGENTS 규약).
- 신규: 재조회 시점 판정(`RelistPending`)의 단위 시험 — 순수 로직이라 서버 없이 전부 돈다.

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `fs::file_op`(로컬 복사) | `grep -rni "SHFileOperation\|IFileOperation\|CopyFile\|fs::copy" src/` → 2건, 둘 다 설정 마이그레이션(`app/settings.rs:479`·`remote/hostkey.rs:137`) | 신규 — 복사 엔진이 없다. 단 **워커 + 자체 STA 패턴은 `fs/thumbnail.rs:277`·`fs/drives.rs:72`를 그대로 따른다** |
| `RelistPending`(재조회 대기) | `grep -n "request_remote_list" src/` → 6건 모두 즉시 발송, 합치기·간격 제어 없음 | 신규 — 합칠 자리가 없다. 발송 자체는 기존 `panel.request_remote_list`를 재사용 |
| `list_common::local_copy_target` | `drop_direction`(`list_common.rs:111`)이 인접 판정이나 **전송 방향**만 답한다 | 신규(작은 함수) — `drop_direction`에 복사를 섞으면 그것을 필터로 쓰는 4곳(`transfer_conflict.rs:35·68·191·226)의 뜻이 조용히 바뀐다 |
| `LayoutOutcome::pane_rects` | 같은 구조체의 기존 필드들과 같은 성질(패널→앱 방향 보고) | 기존 구조체에 필드 추가 — 새 통로를 만들지 않는다 |
| `DropSource`(`IDropSource` 구현) | 레포에 COM 인터페이스 구현 전례 없음 — `grep -rn "implement" src/`의 2건은 둘 다 `src/remote/ftp.rs`의 무관한 문장이고 `windows::core::implement`는 한 번도 쓰이지 않았다 | 신규 — `DoDragDrop`이 요구하는 최소 구현(두 메서드). 데이터 객체는 셸이 만들어 주는 것을 쓰고 직접 구현하지 않는다 |
| `panel_at`·`drop_highlight`(드롭 지점 → 패널 판정) | `splitter.rs:211`의 `rect_contains_pointer`가 인접하나 egui 포인터 전제 | 신규(작은 자유 함수) — OLE 드래그 중에는 egui 포인터가 낡아 Win32 커서를 써야 한다 |
| `ShellHost::cursor_client_pos`·`client_px_to_pt` | 같은 파일의 `to_screen`(`shell_host.rs:59`)이 **정확히 반대 방향** | 기존 타입에 메서드 추가 — 좌표 환산을 HWND 보유자 한 곳에 모은다(새 통로를 만들지 않는다) |
| `i18n::dynamic::local_copy_failed`·`local_copy_cancelled` | `src/i18n/mod.rs:1062`의 `site_export_done` 등 값이 끼어드는 문구의 기존 패턴 | 신규(문구가 새로 생긴다) — 형식·시험 방식은 기존 함수를 그대로 따른다 |

### Verified by

- `grep -rn "request_remote_list" src/` → 정의 2 + 호출 6 + 시험 8, 전부 위 표에 반영
- `grep -rn "\bdrop_direction\b" src/` → 정의 1 + 호출 4 + 시험 4, 전부 확인(변경 없음으로 판정)
- `grep -rn "\bLayoutOutcome\b" src/` → 정의 1 + 생성·시험 10 + 소비 1, 전부 확인
- `grep -rn "\bstart_transfer\b" src/` → 정의 1 + 호출 3 + 주석 1, 전부 확인
- `grep -rni "dropped_files\|hovered_files\|DoDragDrop\|IFileOperation" src/` → 0건(신규 배선임을 확인)

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md` Out of Scope(라인 114) 재한정 + FR-60·FR-61 신설 + FR-37 문면 개정 + 성공 기준·결정 이력 | PRD가 요구 정본인데 라인 114가 이번 기능을 명시적으로 **제외**한다 — 고치지 않으면 산출물이 요구 정본과 정면으로 어긋나고 Phase G 재검증의 기준 자체가 틀린다 | T1에 편입 |
| 필수 | `README.md` §드래그 서술 갱신 | README는 「현재 존재하는 기능」을 적는 문서이고 드래그 동작이 늘어난다 | T8에 편입 |
| 필수 | 「PRD Out of Scope라서」를 근거로 든 주석 4곳 | `src/ui/list_common.rs:65`(`FileDrag` — 「이번 범위 밖」)·`:108`(`drop_direction` — 「PRD Out of Scope다」)·`:232`(시험 주석)·`src/ui/app/transfer_conflict.rs:179~180`(`apply_drop` — 「아무 일도 하지 않는다(PRD Out of Scope)」)가 **T1의 PRD 개정으로 사실이 아니게 된다**. 레포 전수(`grep -rn "Out of Scope\|범위 밖\|로컬↔로컬" src/`)에서 이번 변경에 걸리는 것은 이 넷뿐이다 | T4에 편입 |
| 필수 | `src/ui/app/remote.rs` 모듈 주석의 드롭 서술 | 드롭 처리 경로가 늘어 기존 서술이 부분만 설명하게 된다 | T5에 편입 |
| 무관 | 위키 `feat-remote-transfer`·`feat-file-list` 갱신 | 레포 밖 산출물이고, 계획·구현 세션은 위키를 **읽기만** 한다(스킬 규약). 이 plan의 범위가 아니라 별도 위키 세션의 일이다 | 건드리지 않음 |
| 무관 | `docs/prd.md:112`(자체 파일 작업 UI 제외) | 복사를 **셸에 위임**하므로 그 제외를 푸는 것이 아니라 오히려 그 원칙을 따른다 | 건드리지 않음 |
| 필수 | `src/ui/app.rs:551`의 `notice` 필드 주석 | 「상태 줄에 잠깐 띄울 실패 사유 (FR-39)」인데 T4가 FR-60의 로컬 복사 실패·취소를 같은 필드에 싣는다 | T4에 편입 |
| 조건부 필수 | `assets/licenses.json` 재생성 | 전제 10이 `⚠ 미확인`이다 — feature 추가가 `Cargo.lock` 지문을 바꾸지 않는다는 판정이 맞으면 손댈 것이 없지만, **T7의 `cargo test`에서 지문 대조가 붉어지면 그 자리에서 `cargo run --example gen_licenses`를 돌린다**(AGENTS 「라이선스 자산 재생성」) | T7에 편입(조건부) |
| 무관 | 세션 스키마(`src/ui/session.rs`) | 저장 대상이 늘지 않는다 — 재조회 대기·복사 진행·드롭 강조는 전부 휘발성이다(4-B) | 건드리지 않음 |

## Decisions

### D1. 로컬↔로컬 복사 엔진
- **Options**: A) Windows 셸 `IFileOperation` 위임 / B) 자체 복사 구현 + 전송 큐 통합
- **Chosen**: A
- **Rationale**: 진행률 대화·같은 이름 충돌 대화·`Ctrl+Z` 되돌리기·권한 승격·긴 경로·심볼릭 링크를 OS가 처리한다. PRD 라인 112의 「자체 파일 작업 UI는 셸에 위임」 원칙과 같은 방향이라 그 제외를 풀 필요도 없다. B는 같은 것을 전부 직접 만들어야 하고 PRD 제외를 하나 더 풀어야 한다.
- **Source**: 2026-08-20 사용자 결정. `docs/prd.md:112`, `windows-0.62.2/.../UI/Shell/mod.rs:23135`

### D2. 드래그의 동작은 언제나 복사
- **Options**: A) 언제나 복사 / B) 복사 기본 + `Shift`로 이동 / C) 탐색기 관례(같은 드라이브면 이동)
- **Chosen**: A
- **Rationale**: 요청 원문이 "복사가 되도록"이다. C는 같은 드라이브에서 끌면 원본이 사라져 요청과 어긋난다. B는 지금 필요 없는 분기를 미리 만드는 것이다(YAGNI).
- **Source**: 2026-08-20 사용자 결정

### D3. 전송 완료 후 재조회 시점
- **Options**: A) 사이트별 큐가 빌 때 + 2초 간격 상한 / B) 큐가 완전히 빌 때만 / C) 건마다 즉시
- **Chosen**: A
- **Rationale**: C는 서버 왕복이 파일 수에 비례해 수천 건 업로드에서 서버를 때리고 UI가 그 응답 처리에 잠식된다. B는 오래 걸리는 전송에서 끝날 때까지 아무 변화가 없다. A는 그 둘의 최악을 모두 피한다.
- **Source**: 2026-08-20 사용자 결정

### D4. `drop_direction`을 건드리지 않고 별도 판정 함수를 둔다
- **Options**: A) `drop_direction`에 `LocalCopy` 갈래를 더한다 / B) `local_copy_target` 자유 함수를 옆에 둔다
- **Chosen**: B
- **Rationale**: `drop_direction`은 지금 **전송 큐에 넣을 항목을 거르는 필터**로 네 곳(`transfer_conflict.rs:35·68·191·226`)에서 쓰인다. 여기에 복사를 섞으면 로컬↔로컬 항목이 그 필터를 통과해 **전송 큐에 들어간다** — 조용한 회귀다. 함수 이름이 답하는 물음("전송 방향이 무엇인가")을 그대로 지킨다.
- **Source**: `src/ui/app/transfer_conflict.rs:35·68·191·226`

### D5. 재조회 대상은 「사이트 + 폴더」로 고른다
- **Options**: A) 기존 `request_remote_list(conn)` 재사용 / B) 사이트와 폴더가 모두 맞는 패널만 고른다
- **Chosen**: B
- **Rationale**: A는 연결로 거르는데 전송을 처리한 연결이 패널의 연결과 다를 수 있어(전제 2) 갱신이 새어 나간다. 반대로 A는 목적지와 무관한 폴더를 보는 같은 연결의 패널까지 다시 읽게 해 헛왕복을 만든다. B는 두 결함을 함께 없앤다.
- **Source**: `src/ui/app/remote.rs:547~556`, `src/remote/transfer.rs:206~216`

### D6. 셸 복사는 워커 스레드에서 자체 STA로 돈다
- **Options**: A) UI 스레드에서 직접 / B) 워커 스레드 + 자체 `CoInitializeEx(APARTMENTTHREADED)`
- **Chosen**: B
- **Rationale**: `IFileOperation::PerformOperations`는 복사가 끝날 때까지 반환하지 않는다. UI 스레드에서 부르면 대용량 복사 내내 앱이 굳는다(AGENTS의 UI 스레드 블로킹 I/O 금지). 같은 이유로 이미 `fs/thumbnail.rs:277`·`fs/drives.rs:72`가 워커에서 자체 STA를 연다.
- **Source**: AGENTS.md 「UI 스레드 원칙」, `src/fs/thumbnail.rs:277`

### D7. OLE 드래그 시작은 「포인터가 창 밖으로 나갔을 때」로 미룬다
- **Options**: A) 로컬 항목의 끌기 시작과 동시에 `DoDragDrop` / B) 끌기 중 포인터가 창 밖으로 나가면 그때 시작
- **Chosen**: B
- **Rationale**: A로 하면 앱 안의 드롭까지 전부 OLE 경로가 되고, `DoDragDrop`이 도는 동안 `update()`가 막혀 **T6의 드롭 대상 강조를 그릴 수 없다**. 게다가 T4·T5가 T7의 성패에 묶여, 재진입 문제가 드러나면 세 기능이 함께 무너진다. B는 앱 안 드래그를 종전 egui 경로 그대로 두므로 T7이 실패해도 T2~T6이 온전하다.
- **Source**: `src/ui/app.rs:1988~1994`(중첩 모달 루프의 기존 규칙), `src/ui/panel.rs:1496~1507`(현행 끌기 시작)

### D8. 탐색기에서 끌어온 것은 놓인 패널의 종류가 처리를 정한다
- **Options**: A) 언제나 로컬 복사 / B) 로컬 패널이면 복사, 원격 패널이면 올리기
- **Chosen**: B
- **Rationale**: 요청 원문이 "탭 또는 ftp 에 드레그"다. 원격 패널에 놓인 것은 이미 있는 `start_transfer` 앞문(FR-55 확인 포함)을 그대로 지나면 되므로 새로 만들 것이 없다.
- **Source**: `src/ui/app/transfer_conflict.rs:27`, 요청 원문

### D9. 로컬↔로컬 복사는 FR-55 확인 대화를 거치지 않는다
- **Options**: A) 앱의 같은 이름 확인 대화를 먼저 띄운다 / B) 셸의 충돌 대화에 맡긴다
- **Chosen**: B
- **Rationale**: `IFileOperation`이 자기 충돌 대화(덮어쓰기·건너뛰기·둘 다 두기·이름 바꾸기)를 띄운다. 앞에 앱 대화를 한 번 더 두면 같은 것을 두 번 묻고, 사용자가 앱 대화에서 「덮어쓰기」를 골라도 셸이 다시 묻는다. FR-55의 문면도 대상을 「전송」으로 한정한다.
- **Source**: `docs/prd.md:84`(FR-55 — "전송 전에"), `IFileOperation` 기본 동작

## Tasks

- [x] T1. PRD 개정 — 로컬 복사·OS 드래그 채택, 전송 후 재조회 명문화
  - **Type**: A
  - **Acceptance**: Given 현행 `docs/prd.md`, When 개정, Then ⓐ Out of Scope 라인 114가 「로컬↔로컬 파일 이동·복사 드래그, OS(탐색기)와 주고받는 드래그」를 제외에서 풀고 **원격↔원격 드래그와 잘라내기/붙여넣기는 계속 제외**로 남는다 ⓑ `FR-60`(로컬 패널 사이 드래그 복사 — 셸 위임·언제나 복사, Should)과 `FR-61`(OS 탐색기와 주고받는 드래그 — 받기는 로컬 복사·원격 올리기, 내보내기는 로컬 항목만, Should)이 신설된다 ⓒ `FR-37`에 「전송이 끝나면 그 목적지 폴더를 보고 있는 원격 패널의 목록을 다시 읽는다」가 더해진다 ⓓ 성공 기준의 Should 목록에 FR-60·FR-61이 들어간다 ⓔ 결정 이력 맨 위에 2026-08-20 항목이 선다. **잔존 문면 확인의 범위는 `## Out of Scope` 절과 FR 표 두 곳뿐이다** — `## 결정 이력`의 과거 기록(예: 2026-08-04의 「로컬↔로컬·OS 간 드래그 제외로 재한정한다」)은 그때의 사실이므로 고치지 않는다
  - **Files**: 주: `docs/prd.md`
  - **Edge Cases**: 라인 112(자체 파일 작업 UI 제외)는 건드리지 않는다 — 복사를 셸에 위임하므로 그 제외를 푸는 것이 아니다. 라인 114의 다른 제외 항목(잘라내기/붙여넣기)이 함께 풀리지 않게 문장을 나눈다
  - **Halt Forecast**: (i) FR 번호 충돌 → 현행 최대가 FR-59(`docs/prd.md:88`)임을 확인했으므로 FR-60·FR-61이 다음 번호다
  - **Depends on**: -

- [x] T2. 전송이 끝나면 그 폴더를 보는 원격 패널의 목록을 다시 읽는다
  - **Type**: C
  - **Design**: ① `src/ui/app/remote.rs`에 `RelistPending`(순수 상태)과 그것을 소비하는 `pump_relist`를 둔다 — 원격 배선이 이미 사는 자리다. ② `RelistPending`: `dirty: HashSet<(SiteId, RemotePath)>`와 **사이트별** `last_sent: HashMap<SiteId, f64>`를 들고, `mark(site, dir)`·`take_ready(busy: &HashSet<SiteId>, now) -> Vec<(SiteId, RemotePath)>` 두 메서드만 노출한다(시점 판정이 여기 있어야 서버 없이 시험된다). **`busy`는 대기·진행이 남은 사이트들이며 기존 공개 API `TransferQueue::items()` + `TransferState::is_pending()`으로 만든다** — 큐에 새 조회 API를 더하지 않는다. **`counts_by_site`를 쓰지 않는 이유**(2026-08-20 T2 구현 중 확인): `QueueFilter`에 `Pending` 갈래가 없어(`All`·`Done`·`Error` 셋뿐 — `src/remote/queue.rs:104~108`) 대기·진행만 세려면 세 번 호출해 빼야 한다. 게다가 `take_ready`가 쓰는 것은 건수가 아니라 **「남아 있는가」 하나**라 집합이면 족하다. **시각을 사이트별로 두는 이유**: 하나로 두면 사이트 A의 발송이 사이트 B의 2초 창을 먹어, B가 조건을 채웠는데도 내주지 않는다. ③ 의존 방향: `remote.rs`가 `remote::queue`·`remote::types`를 참조하고, 아무도 `RelistPending`을 참조하지 않는다(`ExplorerApp` 필드로만 산다). ④ 비추상화 선언 — 「새로 고침 정책」을 트레이트로 추상화하지 않는다. 로컬 감시는 `DirWatcher`가 이미 다른 방식으로 하고 있어 공통 뼈대를 만들면 양쪽 다 어색해진다
  - **Acceptance**: Given 원격 폴더 `/pub`를 보고 있는 패널과 그 폴더로 가는 업로드, When 업로드가 성공하고 그 사이트의 대기·진행 항목이 0이 됨, Then 그 패널이 목록을 다시 청한다(같은 사이트라도 **다른 폴더**를 보는 패널은 청하지 않는다). 단위 시험으로 고정할 것: ⓐ 성공한 업로드가 목적지 폴더를 대기로 표시한다 ⓑ 실패한 전송은 표시하지 않는다 ⓒ 받기(다운로드)는 원격을 표시하지 않는다 ⓓ 사이트에 대기·진행이 남아 있으면 2초 전에는 내주지 않는다 ⓔ 2초가 지나면 진행 중이어도 한 번 내준다 ⓕ 같은 폴더로 100건이 끝나도 내주는 항목은 하나다 ⓖ **사이트 A에 방금 보냈어도 사이트 B는 자기 조건만으로 내준다**(간격 상한이 사이트별이다)
  - **Files**:
    - 주: `src/ui/app/remote.rs`
    - 동반: `src/ui/app.rs`(`ExplorerApp`에 `relist: RelistPending` 필드 + 프레임마다 `pump_relist` 호출)
    - 테스트: `src/ui/app/remote.rs`의 `mod tests`(기존 6건 옆에 추가)
  - **Edge Cases**: 목적지가 서버 루트(`/`)라 `parent()`가 없는 경우 — 루트 자신을 대상으로 본다 · 전송 도중 그 패널이 다른 폴더로 옮겨간 경우 — 폴더가 안 맞으므로 청하지 않는다(의도된 동작) · 전송 도중 탭이 닫혀 연결이 사라진 경우 — 대상 패널이 없어 조용히 버려진다 · 앱이 시작하며 세션에서 복원한 큐는 스스로 시작하지 않으므로 표시도 없다
  - **Halt Forecast**: (i) 재조회 요청의 세대 번호가 다른 용도와 겹칠 위험 → 패널의 기존 `request_remote_list` 경로를 그대로 쓰므로 새 번호 공간이 필요 없다(위키 `feat-remote-transfer`의 세대 번호 규약 확인 완료)
  - **Depends on**: -

- [x] T3. 로컬 파일 복사 엔진 — 셸 `IFileOperation` 위임
  - **Type**: D
  - **Design**: ① 신규 모듈 `src/fs/file_op.rs`(`fs/mod.rs`에 등록). ② 신규 심볼 — `copy_into(dest: PathBuf, sources: Vec<PathBuf>, owner: HWND, done: Sender<CopyOutcome>, wake: Wake)`: 워커 스레드를 띄워 자체 STA에서 `IFileOperation`을 돌리고 결과만 채널로 보낸 뒤 다시 그리게 한다 / `CopyOutcome { requested: usize, cancelled: bool, error: Option<String> }`: UI가 알림에 쓸 최소 결과. **`error`는 대개 Win32가 준 사유이며 그때는 카탈로그를 거치지 않는다**(외부에서 온 문자열 — AGENTS 「화면 문구」의 명시된 예외이고, 전송 실패 사유를 서버 원문 그대로 보이는 기존 처리와 같다). **예외 하나 — 원본을 하나도 걸지 못한 경우**는 Win32가 준 사유가 아니라 이쪽 판정이므로 카탈로그를 거친다(`i18n::copy_no_source`). ③ 의존 방향 — `fs::file_op`은 `windows` crate·`std`·**`crate::i18n`**만 참조하고 `ui`를 모른다(AGENTS 계층 규약. `i18n`은 `ui` 하위가 아닌 독립 모듈이며 `src/fs/create.rs`가 같은 방식으로 `create_folder_base`를 쓴다). 부르는 쪽은 `ui::app`뿐이다. ④ **`HWND`는 `Send`가 아니므로 워커 클로저에 그대로 넘어가지 않는다** — `src/fs/enumerate.rs:100~105`의 `struct HwndSend(isize); unsafe impl Send for HwndSend {}` 패턴을 그대로 쓴다(같은 파일 `:119`가 사용 예다). ⑤ 비추상화 선언 — 「파일 작업」 트레이트도, 이동·삭제·이름 바꾸기 갈래도 만들지 않는다. 이번에 필요한 것은 복사 하나다
  - **Acceptance**: Given 원본 경로 목록과 대상 폴더, When `copy_into` 호출, Then 워커가 떠서 UI 스레드가 즉시 반환되고 복사가 끝나면 `CopyOutcome`이 채널로 온다. 빌드·clippy가 경고 0으로 통과하고, 순수 부분(빈 원본 목록이면 워커를 띄우지 않는다 · 대상과 원본이 같은 폴더면 그대로 셸에 넘긴다)이 단위 시험으로 고정된다. 실제 복사 동작은 수동 검증(Verification Strategy)
  - **Files**:
    - 주: `src/fs/file_op.rs`(신규)
    - 동반: `src/fs/mod.rs`, `src/i18n/mod.rs`(카탈로그 키 `copy_no_source` — 원본을 하나도 걸지 못한 경우의 사유)
    - 테스트: `src/fs/file_op.rs`의 `mod tests`
  - **Edge Cases**: 원본이 0개 — 워커를 띄우지 않는다 · 대상 폴더가 사라진 뒤 — 셸이 자기 오류 대화를 띄우고 `CopyOutcome.error`에 사유가 담긴다 · 사용자가 셸 대화에서 취소 — `cancelled: true`(오류가 아니다) · 관리자 권한이 필요한 대상 — 셸이 승격 대화를 띄운다(우리가 판단하지 않는다) · COM 초기화 실패 — 워커가 곧바로 `error`를 돌려준다(패닉하지 않는다) · 원본에 대상 폴더 자신이 섞인 경우 — 셸이 거부한다
  - **Halt Forecast**: (ii-a) `windows` crate feature 확인이 필요할 수 있다 → `Win32_UI_Shell`·`Win32_System_Com`이 이미 있음을 확인했다(전제 8). 모자라면 `## 사전 승인 항목`의 feature 추가 위임에 든다
  - **Depends on**: -

- [x] T4. 탭 → 탭 드래그 복사 배선
  - **Type**: C
  - **Design**: ① 판정은 `src/ui/list_common.rs`에 `local_copy_target(drop: &DropOutcome) -> Option<&Path>`로 둔다 — 드래그 타입이 사는 자리이고, 항목이 전부 `DragItem::Local`이고 대상이 `DropTarget::Local`일 때만 대상 폴더를 돌려준다. ② 배선은 `src/ui/app.rs`의 드롭 소비 지점(:1935)에서 **`start_transfer`보다 먼저** 가른다 — 로컬 복사면 `fs::file_op::copy_into`, 아니면 종전대로 `start_transfer`. ③ **결과 수신도 이 task가 만든다** — `ExplorerApp`에 `copy_tx: Sender<CopyOutcome>` / `copy_rx: Receiver<CopyOutcome>`를 두고(충돌 확인의 `conflict_tx/rx`와 같은 방식), 프레임마다 `pump_local_copy`가 거둬 **기존 `self.notice` 한 줄 알림**으로 올린다(`remote.rs:493`·`:528`과 같은 자리). ④ **문구는 카탈로그를 거친다**(AGENTS 「화면 문구」) — `i18n::dynamic`에 손수 쓴 함수 둘을 더한다: `local_copy_failed(detail: &str)`(셸이 준 사유를 문장 틀에 끼운다)과 `local_copy_cancelled(count: usize)`(걸었던 항목 수를 적는다 — `CopyOutcome.requested`를 그대로 쓴다. **실제로 몇 개가 복사됐는지는 적지 않는다**: 그 값은 사용자가 셸 충돌 대화에서 무엇을 골랐는지에 달려 셸만 안다). **성공은 알리지 않는다** — 셸이 자기 진행률 대화로 이미 알렸고 대상 목록에 파일이 나타나는 것이 곧 확인이라, 여기서 또 띄우면 조작 하나에 알림이 둘이 된다. ⑤ 의존 방향 — `list_common`은 계속 아무것도 새로 참조하지 않고, `ui::app`만 `fs::file_op`을 안다. ⑥ 비추상화 선언 — 드롭 종류를 `enum DropKind`로 승격하지 않는다(갈래가 둘뿐이고 `Option`으로 충분하다)
  - **Acceptance**: Given 로컬 탭 A에서 파일·폴더를 고름, When 로컬 탭 B의 목록 위에 놓음, Then 셸 복사가 시작되고 끝나면 B의 목록에 나타난다(로컬 감시가 갱신한다). 셸이 오류를 주면 그 사유가 상태 줄 알림으로 뜨고, 사용자가 셸 대화에서 취소하면 취소 알림이 뜬다(성공은 알리지 않는다). 단위 시험으로 고정할 것: ⓐ 로컬 항목 + 로컬 대상이면 대상 폴더를 돌려준다 ⓑ 원격 항목이 하나라도 섞이면 `None` ⓒ 대상이 원격이면 `None` ⓓ 항목이 비면 `None` ⓔ `i18n` 카탈로그 시험(`화면_문구가_카탈로그를_거치지_않은_곳이_없다`)이 통과한다 ⓕ 새 `dynamic` 함수 둘이 한국어·영어 양쪽 값을 낸다(`LanguageGuard::lock`으로 언어를 잠그고 기대값은 원문 리터럴 — AGENTS 시험 규약). 화면 조작은 수동 검증
  - **Files**:
    - 주: `src/ui/list_common.rs`, `src/ui/app.rs`
    - 동반: `src/i18n/mod.rs`(`dynamic`에 함수 2개), 「PRD Out of Scope라서」를 근거로 든 주석 4곳 — `src/ui/list_common.rs:65`·`:108`·`:232`, `src/ui/app/transfer_conflict.rs:179~180`, 그리고 `src/ui/app.rs:551`의 `notice` 필드 주석(「실패 사유 **(FR-39)**」 — 이제 FR-60의 로컬 복사 실패·취소도 같은 자리를 쓴다). 모두 동반 변경 판정의 필수 항목이다
    - 테스트: `src/ui/list_common.rs`의 `mod tests`, `src/i18n/mod.rs`의 `mod tests`
  - **Edge Cases**: `..` 항목은 끌기에 실리지 않는다(`file_list.rs:966` 시험이 이미 고정) · 같은 패널·같은 폴더에 놓기 — 셸이 「사본」을 만들거나 거부한다(우리가 막지 않는다) · 폴더를 자기 하위 폴더에 놓기 — 셸이 거부한다 · 드래그 중 대상 탭이 다른 폴더로 옮겨간 경우 — 놓는 순간의 폴더가 대상이다(`take_drop`이 그때 읽는다)
  - **Halt Forecast**: (i) 로컬 복사가 전송 큐에 잘못 들어갈 위험 → D4로 `drop_direction`을 건드리지 않아 원리상 막힌다
  - **Depends on**: T3

- [ ] T5. 탐색기·바탕화면에서 끌어온 파일을 받는다
  - **Type**: D
  - **Design**: ① 드롭 지점 → 패널 판정은 `src/ui/app.rs`에 자유 함수 `panel_at(pane_rects: &[(PanelId, egui::Rect)], pos: egui::Pos2) -> Option<PanelId>`로 둔다(순수 함수라 시험 가능). ② 커서 위치는 `src/ui/shell_host.rs`에 **`cursor_client_pos(&self, pixels_per_point: f32) -> Option<egui::Pos2>`**를 더해 얻는다 — `GetCursorPos` + `ScreenToClient`는 **물리 픽셀**을 주므로 **`pixels_per_point`로 나눠 논리 pt로 돌려준다**(`pane_rects`가 논리 pt다). 이 앱은 이미 반대 방향에서 같은 환산을 한다(`app.rs:1992~1993` — 셸 메뉴가 `menu.pos.x * scale`로 물리 픽셀을 만든다). 환산이 빠지면 배율 125%·150% 화면에서 대상 패널이 어긋난다. ③ 패널 사각형은 `splitter::LayoutOutcome`에 `pane_rects: Vec<(PanelId, egui::Rect)>`를 더해 `show_layout`이 `computed.panes`에서 채운다. ④ 소비는 `ui::app`의 프레임 말미 — `ctx.input(|i| i.raw.dropped_files.clone())`으로 경로를 거두고 대상 패널을 정한 뒤 **대상 종류가 처리를 가른다**: **로컬 탭이면 경로 목록을 그대로 `fs::file_op::copy_into`로 보낸다**(`IFileOperation`이 경로만 받으므로 `is_dir`을 잴 일이 없다). **원격 탭이면 경로 목록을 워커 스레드에 넘겨 거기서 `std::fs::metadata`로 `is_dir`을 재고**(`DragItem::Local`이 그 값을 요구한다) 채널로 받아 다음 프레임에 `DropOutcome`을 조립해 `start_transfer`로 보낸다 — `apply_drop`의 `expand_tx`와 같은 방식이며(`transfer_conflict.rs:184~218`), UI 스레드에서 수천 번 stat을 도는 것을 막는다(AGENTS DO NOT 「UI 스레드에서 파일시스템 블로킹 호출」). ⑤ 비추상화 선언 — 「외부 드롭」을 위한 별도 통로 타입을 만들지 않는다. 원격 쪽은 기존 `DropOutcome`을 그대로 조립해 기존 앞문을 지난다
  - **Acceptance**: Given 탐색기에서 파일·폴더 여럿을 끌어옴, When 로컬 탭 위에 놓음, Then 그 탭의 폴더로 셸 복사가 시작된다. When 원격 탭 위에 놓음, Then 그 탭의 폴더로 올리기가 큐에 들어간다(같은 이름이 있으면 FR-55 확인 대화를 거친다). 패널 밖(사이드바·도크·타이틀바)에 놓으면 아무 일도 일어나지 않는다. 단위 시험으로 고정할 것: ⓐ `panel_at`이 사각형 안의 점에 해당 패널을 돌려준다 ⓑ 어느 사각형에도 안 들면 `None` ⓒ 사각형이 겹치면 **나중 것**(위에 그려진 것)이 이긴다 ⓓ 물리 픽셀 → 논리 pt 환산이 `pixels_per_point`가 1.0이 아닐 때(1.25·1.5) 올바른 점을 낸다 — 그 환산을 `shell_host`의 Win32 호출과 **분리된 순수 함수**(`client_px_to_pt(px: (i32, i32), pixels_per_point: f32) -> egui::Pos2`)로 두어 시험한다. `show_layout` 자체는 시험하지 않는다(4-C의 근거 — 하네스 비용이 크고 대입 홉은 리뷰가 지킨다). 실제 드롭은 수동 검증
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/splitter.rs`, `src/ui/shell_host.rs`
    - 동반: `src/ui/app/remote.rs` 모듈 주석(드롭 경로가 늘어난 것을 반영 — 동반 변경 판정의 필수 항목)
    - 테스트: `src/ui/app.rs`의 `mod tests`(`panel_at`), `src/ui/shell_host.rs`의 `mod tests`(`client_px_to_pt`)
  - **Edge Cases**: **화면 배율이 100%가 아닌 모니터**(125%·150%) — ②의 환산이 없으면 대상이 어긋난다 · 창이 최소화·비활성 상태에서의 드롭 — winit이 이벤트를 보내므로 그대로 처리한다 · 커서를 읽지 못하면(`GetCursorPos` 실패) 아무 일도 하지 않는다 · HWND가 없으면(`ShellHost`가 `None`) 외부 드롭 기능 전체가 조용히 꺼진다(셸 메뉴와 같은 규칙) · 원격 탭인데 연결이 없으면 확인을 건너뛰고 큐에 넣는다(기존 D10 규약) · 드롭한 경로가 그 사이 사라진 경우 — 셸·전송이 각자 오류를 낸다 · 한 번에 수천 개 경로 — `is_dir` 측정도 폴더 펼치기도 워커가 한다(④)
  - **Halt Forecast**: (i) 놓인 자리를 알 수 없는 문제 → `GetCursorPos` 보조로 해결(전제 5) · (ii-a) `LayoutOutcome` 필드 추가는 계획된 구조 변경 → `## 사전 승인 항목`
  - **Depends on**: T3, T4

- [ ] T6. 드롭 대상 패널을 끌고 있는 동안 강조한다
  - **Type**: C
  - **Design**: ① **대상은 외부(OS) 드래그뿐이다** — 앱 안의 탭↔탭 드래그(T4)는 강조하지 않는다. 사용자가 고른 항목의 문안이 「탐색기에서 파일을 끌고 창 위로 오면」이었고, 앱 안 드래그는 egui가 이미 끌고 있는 항목을 커서에 붙여 보여 준다. ② 판정은 `src/ui/app.rs`에 순수 함수 `drop_highlight(hovering: bool, cursor: Option<egui::Pos2>, pane_rects: &[(PanelId, egui::Rect)]) -> Option<PanelId>`로 떼어 둔다 — `hovered_files`가 비었는지·커서가 어느 사각형에 드는지를 합치는 규칙이 여기 모여 시험된다(`panel_at`을 그대로 쓴다). ③ **`show_layout`의 시그니처는 건드리지 않는다** — 그 함수가 `pane_rects`를 **반환**하므로 인자로 `highlight`를 넣으면 같은 프레임 안에서 순환이 된다(입력이 그 호출의 결과다). 대신 **`show_layout`이 돌아온 뒤 `app.rs:1871`의 같은 `ui`에서 `ui.painter().rect_stroke`로 덧그린다** — egui는 나중에 그린 도형이 위에 오므로(`splitter.rs:240~241` 주석의 같은 근거) 활성 테두리 위에 정상적으로 얹힌다. 이러면 직전 프레임 값을 캐시할 필요도, 레이아웃이 바뀐 프레임에 한 박자 늦을 일도 없다. ④ 선 굵기·모양은 활성 테두리(`splitter.rs:247~252`)와 같은 규칙을 쓰고 색만 `ui::theme`의 기존 강조색으로 가른다(새 상수를 만들지 않는다). ⑤ 비추상화 선언 — 「하이라이트 종류」를 열거형으로 나누지 않는다(활성 테두리와 드롭 강조 둘뿐이다)
  - **Acceptance**: Given 탐색기에서 파일을 끌어 창 위로 옴, When 어떤 패널 위를 지남, Then 그 패널 테두리가 강조된다. 드롭하거나 창 밖으로 나가면 강조가 사라진다. 단위 시험으로 고정할 것: ⓐ 끌고 있지 않으면 `drop_highlight`가 `None` ⓑ 끌고 있고 커서가 패널 안이면 그 패널 ⓒ 끌고 있어도 커서가 어느 패널에도 안 들면 `None`(사이드바·도크 위) ⓓ 커서를 읽지 못하면 `None`. 그려지는 모양은 수동 검증(`show_layout` 하네스는 세우지 않는다 — 4-C의 근거)
  - **Files**:
    - 주: `src/ui/app.rs`(판정 + 덧그리기 — `splitter.rs`는 건드리지 않는다, Design ③)
    - 테스트: `src/ui/app.rs`의 `mod tests`(`drop_highlight`)
  - **Edge Cases**: `hovered_files`가 비지 않았는데 커서가 패널 밖(사이드바·도크) — 아무 패널도 강조하지 않는다 · 드래그를 창 밖에서 취소(`HoveredFileCancelled`) — egui가 목록을 비우므로 강조도 사라진다 · 강조 중 레이아웃이 바뀜(분할·닫기) — 다음 프레임의 사각형으로 다시 판정한다 · 외부 드래그와 앱 내부 egui 드래그가 겹칠 일은 없다(마우스 버튼은 하나다)
  - **Halt Forecast**: (i) 드래그 중 앱이 다시 그려지지 않으면 강조가 보이지 않는다 → OLE 드래그 중에도 winit이 `HoveredFile` 이벤트를 보내고 egui-winit이 `repaint: true`를 돌려준다(`egui-winit-0.35.0/src/lib.rs:445~455`)
  - **Depends on**: T5

- [ ] T7. 앱에서 탐색기로 끌어내기
  - **Type**: D
  - **Design**: ① 신규 모듈 `src/fs/drag_source.rs` — `start_copy_drag(paths: &[PathBuf]) -> bool`이 경로마다 `SHParseDisplayName`으로 PIDL을 얻어(`windows-0.62.2/.../UI/Shell/mod.rs:3689`) `SHCreateShellItemArrayFromIDLists`(`:2807`)로 묶고, `IShellItemArray::BindToHandler(None, &BHID_DataObject, ...)`(`:6138`)로 `IDataObject`를 얻는다. 그 다음 최소 `IDropSource` 구현(`windows::core::implement`)과 함께 `DoDragDrop(..., DROPEFFECT_COPY, ...)`을 부른다. **PIDL은 얻은 쪽이 `CoTaskMemFree`로 되돌린다**(누수 방지). ② 발동은 `ui::app`의 **그리기가 모두 끝난 뒤**, 셸 메뉴·파일 대화와 같은 자리에서 셋 중 하나만 뜨게 상호 배제한다(`app.rs:1988~1999`의 기존 규칙). ③ 조건은 D7 — egui 드래그가 진행 중이고 실린 항목이 전부 로컬이며 **포인터가 뷰포트 밖으로 나갔을 때** 한 번만. 시작하면 egui 페이로드를 거둬 앱 안 드롭과 겹치지 않게 한다. ④ 의존 방향 — `fs::drag_source`는 `windows`와 `std`만 안다. ⑤ 비추상화 선언 — 드래그 이미지(`IDragSourceHelper`)·이동/링크 효과·원격 항목의 지연 렌더링은 만들지 않는다(Deferred)
  - **Acceptance**: Given 로컬 탭에서 파일·폴더를 골라 끌기 시작, When 포인터가 MOA 창 밖의 탐색기·바탕화면으로 나가 놓음, Then 그 자리에 복사된다. 창 안에서 놓으면 종전대로 앱 안 드롭(T4·기존 전송)이 처리한다. 원격 항목을 끌 때는 OLE 드래그가 시작되지 않는다. `cargo build`·`cargo test`·`cargo clippy -- -D warnings`가 경고 0으로 통과한다. 실제 드래그는 수동 검증
  - **Files**:
    - 주: `src/fs/drag_source.rs`(신규), `src/ui/app.rs`
    - 동반: `src/fs/mod.rs`, `Cargo.toml`(`Win32_System_Ole` feature)
    - 테스트: `src/fs/drag_source.rs`의 `mod tests`(빈 경로 목록이면 시작하지 않는다 — COM 호출 없는 부분만)
  - **Edge Cases**: 경로 목록이 비면 시작하지 않는다 · 셸 항목을 만들지 못하는 경로(사라진 파일)가 섞이면 그것만 빼고 나머지로 진행하되 전부 실패면 시작하지 않는다 · 사용자가 `Esc`로 취소 — `IDropSource::QueryContinueDrag`가 `DRAGDROP_S_CANCEL`을 돌려준다 · 드래그가 MOA 자신의 창으로 되돌아온 경우 — winit이 드롭 이벤트를 주고 T5 경로가 처리한다(자기 자신에게 복사) · 셸 메뉴·파일 대화가 뜬 프레임 — 시작을 미룬다 · 마우스 버튼이 이미 떼진 뒤 — `DoDragDrop`이 즉시 반환한다
  - **Halt Forecast**: (i) `DoDragDrop`의 중첩 메시지 루프가 eframe 이벤트 루프를 재진입시킨다 → 셸 메뉴가 같은 구조로 이미 출하돼 있고(전제 7) 같은 자리·같은 규칙을 쓴다. 수동 검증에서 창이 굳거나 패닉하면 **이 task만 되돌리고**(D7 덕에 T2~T6은 영향 없다) Deferred로 옮긴다 · (ii-a) `Cargo.toml` feature 추가 → `## 사전 승인 항목`
  - **Depends on**: T4, T5

- [ ] T8. README 갱신
  - **Type**: A
  - **Acceptance**: Given 개정된 PRD와 구현된 동작, When README를 갱신, Then 드래그 관련 서술이 ⓐ 로컬↔원격 전송 ⓑ 로컬↔로컬 복사 ⓒ 탐색기에서 받기 ⓓ 탐색기로 내보내기(T7이 살아남은 경우만) ⓔ 전송 완료 후 원격 목록 자동 갱신을 모두 담고, **구현되지 않은 것이 하나도 적혀 있지 않다**(요청받지 않은 새 절(`##`)은 만들지 않는다)
  - **Files**: 주: `README.md`
  - **Edge Cases**: T7이 Halt로 빠졌으면 ⓓ를 적지 않는다 — README는 현재 존재하는 기능만 적는다
  - **Halt Forecast**: 없음 — 문서 갱신이라 파괴적·외부 요소가 없고, 유일한 분기(T7 실패 시 ⓓ 제외)는 위 Edge Cases가 이미 정한다
  - **Depends on**: T1, T2, T4, T5, T6, T7

## 사전 승인 항목 (일괄 승인 대상)

- T3·T5 — 신규 모듈 추가(`src/fs/file_op.rs`, `src/ui/shell_host.rs`에 함수 1개)와 `src/fs/mod.rs` 등록. 구조 변경이나 파일 삭제·이동은 없다
- T5 — `splitter::LayoutOutcome`에 `pane_rects` 필드 추가. crate 내부 계약이며 `Default` 파생이라 기존 생성부 10곳이 그대로 컴파일된다(`show_layout`의 인자는 늘지 않는다 — T6 Design ③)
- T4 — `i18n` 카탈로그(`src/i18n/mod.rs`의 `dynamic`)에 화면 문구 함수 2개 추가
- T7 — `Cargo.toml`의 `windows` crate에 `Win32_System_Ole` feature 추가. **신규 crate가 아니라 이미 있는 의존성의 feature**이며 `Cargo.lock`을 바꾸지 않는다. `src/fs/drag_source.rs` 신규 모듈
- T1·T8 — `docs/prd.md`·`README.md` 개정(요구 정본 변경 — 이 plan 승인이 곧 그 개정의 승인이다)
- 로컬 작업 브랜치에 대한 task별 commit

## 불가피한 Halt (위임 불가)

- `master` 병합 · `push` · 태그 · 릴리즈 — 외부·비가역 작업이라 이 plan 승인에 포함되지 않는다. 모든 task와 Phase F/G가 끝난 뒤 따로 승인받는다
- T7의 `DoDragDrop`이 수동 검증에서 창을 굳히거나 앱을 패닉시키는 경우 — plan에 없던 방향 전환이므로 사용자에게 보고하고 지시를 받는다(Deferred 이관 또는 D7의 대안인 「끌기 시작과 동시에 OLE」로 전환)

## Verification Strategy

- 빌드: `cargo build`
- 단위·통합 테스트: `cargo test`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 서식: `cargo fmt --check`
- **수동 검증**(드래그의 화면 동작은 자동 시험으로 재현할 수 없다 — Deferred 대장 2026-08-18 항목):
  1. 원격 탭 하나를 열고 로컬에서 파일 여러 개를 올린 뒤, **손대지 않고** 원격 목록에 나타나는지 본다(T2)
  2. 파일 수천 개짜리 폴더를 올리면서 목록이 2초쯤마다 늘어나는지, 서버 로그에 조회가 폭주하지 않는지 본다(T2)
  3. 로컬 탭 두 개를 나란히 열고 파일·폴더를 끌어다 놓아 셸 복사 대화가 뜨고 복사되는지 본다. **그 대화가 떠 있는 동안 MOA 창이 응답하는지**(스크롤·탭 전환) 함께 본다(T4·T3의 소유자 창 위험)
  4. 탐색기에서 파일 여러 개를 끌어 로컬 탭에 놓아 복사되는지, 원격 탭에 놓아 큐에 올리기로 들어가는지 본다(T5)
  4-1. **화면 배율을 125%·150%로 바꾼 뒤 4를 다시 한다** — 놓은 패널과 실제로 받는 패널이 같은지 본다(T5의 좌표 환산)
  5. 4를 하는 동안 대상 패널 테두리가 강조되는지, 패널 밖(사이드바·하단 도크)에서는 강조가 없는지 본다(T6)
  5-1. 로컬 탭에 쓰기 권한이 없는 폴더(예: `C:\Windows\System32`)로 복사를 걸어 실패 사유가 상태 줄에 뜨는지, 셸 대화에서 취소하면 취소 알림이 뜨는지 본다(T4)
  6. 로컬 탭에서 파일을 끌어 바탕화면에 놓아 복사되는지, **그 동안 창이 굳지 않는지** 본다(T7)
  7. 6에서 끌던 것을 다시 MOA 창 안의 다른 탭에 놓아도 복사되는지 본다(T7 ↔ T5 상호작용)

## Phase Ledger

## Retry Ledger

- T3: 동일 BLOCKER/MAJOR 1/3, 수정 사이클 1/5, 복구 0/2 — spec 리뷰 B1(같은 폴더 순수 시험 누락)로 1회 되돌림.
- T4: 동일 BLOCKER/MAJOR 1/3, 수정 사이클 1/5, 복구 0/2 — spec 리뷰 M1(필드 삽입으로 doc 주석 오귀속)로 1회 되돌림.

## Progress Log

- T3-T4 완료 (커밋 `5982f75`, T4 완료 커밋): 로컬 복사를 셸 `IFileOperation`에 맡기는 `fs::file_op`을 만들고 탭↔탭 드롭을 그리로 배선했다. 판정은 `list_common::local_copy_target`이 하고 `start_transfer`보다 먼저 갈라진다.
  - 결정: 셸 복사 실패·취소만 상태 줄로 알리고 **성공은 알리지 않는다** — 셸 진행률 대화가 이미 알렸고 대상 목록에 파일이 나타나는 것이 곧 확인이라, 또 띄우면 조작 하나에 알림이 둘이 된다.
  - **같은 실수를 두 번 했다**: 함수·필드를 기존 항목 바로 위에 끼워 넣어 그 항목의 doc 주석을 가로챘다(T2 `pump_relist`, T4 `copy_tx`). 두 리뷰가 각각 잡았다 — 다음에 `impl` 블록이나 구조체에 항목을 더할 때는 **삽입 지점 바로 위의 `///`가 누구 것인지 먼저 본다**.

- T1-T2 완료 (커밋 `b1092d5`, `4e125fc`): PRD에 FR-60·FR-61을 신설하고 FR-37에 전송 후 재조회를 명문화했다. 재조회는 `RelistPending`(순수 상태) + `pump_relist`로 붙였고 시험 7건이 acceptance ⓐ~ⓖ를 고정한다.
  - 결정: `take_ready`의 인자를 plan의 `site_pending_counts`(건수)에서 `busy: &HashSet<SiteId>`(멤버십)로 바꿨다 — `QueueFilter`에 `Pending` 갈래가 없어 `counts_by_site`로는 대기·진행만 셀 수 없고(세 번 호출해 빼야 한다), 판정에 필요한 것은 「남아 있는가」 하나뿐이다. plan Design ②를 그 사실에 맞게 정정했다.
  - quality 리뷰가 잡은 doc 주석 오배치(`pump_relist`가 `request_remote_list`의 설명 줄을 뺏었다)를 고치고, 두 함수가 대상을 고르는 기준이 어떻게 다른지를 주석에 남겼다.

## Next Steps

- 권장 다음 액션: 승인 후 `pjc:implement-task`로 T1부터 실행

## Open Questions

- [x] Q1: 로컬↔로컬 복사 엔진 → **셸 `IFileOperation` 위임**(D1)
- [x] Q2: 드래그의 복사/이동 → **언제나 복사**(D2)
- [x] Q3: 전송 완료 후 재조회 시점 → **큐가 빌 때 + 2초 간격 상한**(D3)
- [x] Q4: 함께 넣을 것 → **드롭 대상 패널 강조**(T6)와 **앱 → 탐색기 끌어내기**(T7). 원격↔원격 드래그는 제외
