# Plan: 끊긴 네트워크 드라이브를 목록과 트리에 알리기

**PRD**: docs/prd.md

## 요구 이해
- **원문 요청**: "1번 이미지처럼 네트워크 드라이브(Z:) 연결 오류가 발생하면 위쪽에 오류 메시지가 표시 되는데 오른쪽에 이전 목록 대신 오류 메시지를 표시 하도록 수정해줘(폴더 권한 메시지 처럼 표시) / 2번 이미지처럼 네트워크 드라이브가 연결이 되지 않으면 아이콘에 X 표시가 되는데 동일하게 수정해줘"
- **이해한 요구**: 끊긴 네트워크 드라이브를 **화면 두 곳**에서 탐색기와 같게 알린다 — ① 열기에 실패하면 **이전 폴더의 목록을 남기지 말고** 그 경로로 옮긴 뒤 목록 자리에 사유를 적는다(`권한 없음`과 같은 방식이며 상태 줄에는 적지 않는다) ② 폴더 트리의 드라이브 줄 아이콘에 **연결 끊김 X 배지**를 겹쳐, 누르기 전에도 끊긴 것이 보이게 한다.
- **포함하지 않는 것으로 이해**: 끊긴 드라이브를 자동으로 다시 연결하지 않는다(상태를 보일 뿐이다). 원격(FTP·SFTP) 패널은 이 작업의 대상이 아니다 — 그쪽 실패는 이미 자기 화면(`TabPhase::Error`)이 있다.

## Goal
끊긴 네트워크 드라이브를 열면 그 사유가 목록 자리에 뜨고, 폴더 트리의 그 드라이브 줄에는 탐색기와 같은 X 배지가 붙는다.

## PRD Coverage
| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-6 (주소창·뒤로/앞으로/상위 탐색) | Must | T7 (문면에 열기 실패 표시 규칙을 더한다) | ✅ 커버 — 지금 PRD에 실패 처리 규칙이 없다(사용자 승인, 2026-08-17) |
| FR-9 (폴더 트리 — 줄마다 셸 아이콘·드라이브 표시 이름) | Should | T5(X 배지) · T7(문면 갱신) | ✅ 커버 — 트리 표시 규격에 X 배지가 더해진다 |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope
- 끊긴 네트워크 드라이브의 **자동 재연결**(`WNetAddConnection2`) — 상태를 보이는 것이 이번 요구이고, 재연결은 자격증명 대화를 부를 수 있어 성격이 다르다.
- 로컬 드라이브(빈 광학 드라이브·준비되지 않은 이동식)의 X 배지 — 탐색기도 그 자리에는 배지를 두지 않는다.
- 원격(FTP·SFTP) 트리의 배지 — 서버 항목에는 드라이브라는 개념이 없다.

## Deferred / Follow-up
- **`IconTextures`의 아이콘 변환이 셸 잠금 밖이다** — `icon_tex::icon_to_image`의 `ImageList_GetIcon`은 `fs::icons::shell_test_guard`가 감싸지 않아 병렬 시험에서 경합하고, 실패한 인덱스는 `None`으로 **영구 기억**된다. T5에서 배지를 텍스처와 분리해 이 회차의 시험은 안정됐지만, 아이콘 자체를 보는 기존 시험들은 여전히 이 위험에 노출돼 있다(이번 변경이 유발한 것이 아니라 드러낸 기존 문제다). 근본 해결은 그 변환도 같은 잠금 안에서 돌게 하는 것.
- 끊긴 드라이브의 **주기적 재확인** — 이번에는 시작 시 1회 + 그 드라이브를 열어 볼 때만 갱신한다(사용자 결정). 연결이 복구돼도 한 번 열어 보기 전에는 X가 남는다.
- 트리 **하위 폴더**(드라이브 아래 줄)의 아이콘 조회는 여전히 UI 스레드에서 한다 — 이번에 워커로 옮기는 것은 드라이브 줄뿐이다. 하위는 그 드라이브를 이미 읽은 뒤라 느릴 이유가 적다.

## Investigation Log
- 위키 참조: `20_projects/personal/moa/conventions.md` — ⓐ 화면 결과를 위로 올리는 홉 중 컴파일러가 강제하는 것은 `splitter::merge_panel_outcome`(구조 분해)뿐이고 `LayoutOutcome`→`ui::app`은 필드별 대입이라 한 홉을 빠뜨려도 빌드가 통과한다 ⓑ `ExplorerApp`은 단위 시험에서 만들 수 없어 판정 로직을 `app::` 계층 자유 함수로 내려야 시험이 덮는다 ⓒ 함수를 사이에 끼워 넣을 때 앞 함수의 doc 주석이 딸려 붙는다(빈 줄 확인).
- 위키 참조: `20_projects/personal/moa/decisions.md` — 이 작업과 상충하는 기각·보류 결정 없음(즐겨찾기·전송 대상 탭·모달 규격이 최근 결정이며 드라이브 상태 표시는 처음이다).
- 위키 참조: `20_projects/personal/moa/feat-navigation.md:60` — "폴더가 없거나 여는 중 문제가 생긴 경우는 현 위치를 지키고 상태 줄에만 사유를 알린다"가 이번 변경으로 어긋난다(T7에서 큐 1줄).
- 셸 오버레이 실측(`SHGetFileInfoW`, 조사 스크립트는 임시 폴더): 끊긴 `Z:\`에 대해 **경로 문자열·PIDL 두 경로 모두 오버레이 인덱스 0**을 돌려준다(`SHGFI_OVERLAYINDEX`·`SHGFI_ADDOVERLAYS` 조합 4가지). 셸에서 X 오버레이를 얻는 길이 없어 앱이 직접 그려야 한다.
- 끊긴 드라이브 접근 실측: `GetFileAttributesW("Z:\")` → 실패, `GetLastError` **53 (ERROR_BAD_NETPATH)**, 소요 **2793ms(첫 시도)**. 이어진 `GetDiskFreeSpaceExW` 74ms, `FindFirstFileW` 123ms. 첫 시도가 초 단위라 UI 스레드에서 할 수 없다.
- `WNetGetConnectionW("Z:")`는 끊긴 매핑에도 **0(성공)**을 돌려준다 — `net use`가 `Disconnected`로 보이는 상태와 구분되지 않아 연결 판정에 쓸 수 없다.
- 드라이브 종류 판정 `GetDriveTypeW("Z:\")` → **4(DRIVE_REMOTE)**, 소요 0ms. 로컬은 3(DRIVE_FIXED). 네트워크 드라이브만 골라내는 데 쓸 수 있다.
- 셸 표시 이름 `SHGetFileInfoW(SHGFI_DISPLAYNAME)`는 끊긴 `Z:\`에도 `Share(\\LKJ-GRAM) (Z:)`를 즉시 돌려준다(0ms) — 아이콘 인덱스 조회도 11ms였다.
- `fs/thumbnail.rs:277` — 워커 스레드에서 `CoInitializeEx(None, COINIT_APARTMENTTHREADED)` 후 셸을 부르고 종료 시 `CoUninitialize`하는 선례가 이미 있다.
- 탐색기 배지 스크린샷 픽셀 측정(사용자 제공 2번 이미지): 배지는 아이콘 오른쪽 아래에 겹치고 붉은 계열(측정값 `216,64,6` 부근 — 축소·안티에일리어싱이 섞인 값이라 정확한 원색은 아니다).

### 전제 검증
| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 끊긴 네트워크 드라이브의 열거 실패는 지금 `EnumOutcome::Error`로 떨어진다 | `src/fs/enumerate.rs:118-124`(`_ => Error`) + 실측 오류 53이 `ERROR_ACCESS_DENIED`·`ERROR_FILE_NOT_FOUND`·`ERROR_PATH_NOT_FOUND` 어디에도 안 맞음. 화면 문구도 `open_failed`("여는 중 문제가 발생했습니다")로 스크린샷과 일치 | ✅ |
| 2 | 셸은 끊긴 드라이브에 X 오버레이를 주지 않는다 → 직접 그려야 한다 | 위 Log의 오버레이 실측(경로·PIDL 4조합 전부 0) | ✅ |
| 3 | 연결 상태 판정은 UI 스레드에서 할 수 없다 | 위 Log의 2793ms 실측 | ✅ |
| 4 | 시스템 이미지 리스트 인덱스는 워커에서 얻어 UI에서 그려도 된다 | 인덱스는 프로세스 전역 이미지 리스트(`IconCache::himl` — `src/fs/icons.rs:129`)의 자리 번호이며 스레드에 매이지 않는다. `IconTextures::get`도 `(himl, index)`만으로 텍스처를 만든다(`src/ui/icon_tex.rs:60-84`) | ✅ |
| 5 | `PanelOutcome`에 필드를 더하면 `splitter::merge_panel_outcome`이 컴파일 오류로 잡는다 | `src/ui/splitter.rs:92-102`가 구조 분해로 전 필드를 열거 | ✅ |
| 6 | `LayoutOutcome`→`ui::app` 홉은 컴파일러가 강제하지 않는다 | `src/ui/app.rs:2718-2725`가 필드별 대입 — 한 줄을 빠뜨려도 빌드가 통과 | ✅ (위험으로 등록) |
| 7 | 드라이브 줄의 소유를 앱으로 올려도 되는 선례가 있다 | 즐겨찾기가 같은 모양이다 — `ui/app.rs:2709`가 `FavoriteStore`를 들고 `&[FavoriteEntry]`를 `panel.show`(`ui/panel.rs:1103`)→`tree.show`(`ui/tree.rs:203`, 호출은 `ui/panel.rs:1159`)로 내려보낸다 | ✅ |
| 8 | 트리는 패널마다 하나씩 있어 드라이브 조회가 패널 수만큼 중복된다 | `FolderTreeView`가 `PanelState`의 필드이며 `roots`를 각자 만든다(`src/ui/tree.rs:131`, `224-234`) | ✅ |
| 9 | 드라이브 뿌리와 하위 폴더가 **같은 `show_node`**를 지난다 — 아이콘 출처와 배지 유무를 그 안에서 갈라야 한다 | 정의 `src/ui/tree.rs:513`, 호출 `:235`(뿌리)·`:566`(하위 재귀). 아이콘은 `:524`의 `icon_for` 하나에서 온다 | ✅ (리뷰 1라운드 B2에서 드러나 T4·T5 Design에 반영) |
| 10 | `drive_roots`를 부르는 곳이 `ui/tree.rs` 밖에도 있다 | `grep -rn "drive_roots" src/` → `ui/panel/tests.rs:109`(헬퍼 `drive_labels`)·`:2363`. 그 헬퍼를 쓰는 시험은 `:1143`·`:1407`·`:1653` | ✅ (리뷰 1라운드 B1 — 첫 조사가 한 파일로 스코프를 좁혀 놓쳤다) |
| 11 | 즐겨찾기 줄도 같은 트리 그리기 경로에서 UI 스레드로 셸을 부르며, 그 호출은 하위 폴더와 **같은 `icon_for`**(`tree.rs:397`)를 지난다 | `src/ui/tree.rs:368`(`show_favorites` → `icon_for`) · `:524`(`show_node` → `icon_for`) → `src/fs/icons.rs:196` | ✅ (그래서 T4 Acceptance를 **드라이브 갈래**로 좁혔다 — `icon_for` 함수 자체는 남는다. 즐겨찾기·하위 폴더는 이 plan의 Deferred가 범위 밖으로 둔 것이다) |
| 12 | 워커에서 `IconCache::new()`를 만들어 셸 래퍼를 재사용할 수 있다 | `src/fs/icons.rs:90-126`(생성자는 셸 조회 몇 번뿐이고 스레드 제약이 없다) · `src/ui/panel/tests.rs:108`이 이미 시험 안에서 만들어 쓴다 | ✅ (리뷰 2라운드 M2 — 첫 계획은 "워커가 `IconCache`를 못 쓴다"고 근거 없이 단정해 `unsafe` 조회를 한 벌 더 지을 뻔했다) |
| 13 | 이 레포의 계층 방향은 `app` → `fs`다 | `src/app/sidebar.rs:10`이 `fs::icons`를 쓰고, `src/fs/*.rs` 어디에도 `use crate::{app,panel,ui}`가 없다 | ✅ (그래서 `DriveRow`를 `fs::drives`에 두고 `app::drives`가 참조한다) |
| 14 | 트리 아이콘 자리는 **16×16 정사각**이다 | `src/ui/tree.rs:41`(`ROW_ICON = 16.0`) · `:723-726`(`icon_rect`를 `vec2(ROW_ICON, ROW_ICON)`로 짓는다) | ✅ (그래서 배지 비율을 기준 스크린샷의 **높이 축**으로 잡았다 — 시각 표 참조) |

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| `LayoutOutcome`→`ui::app` 홉을 빠뜨려도 빌드가 통과한다(전제 6) | 열어 본 결과가 앱에 닿지 않아 X가 영영 갱신되지 않는다 | **이 한 홉은 시험으로 덮을 수 없다** — `ExplorerApp`은 실 HWND(`CreationContext`)가 있어야 만들어져 프레임을 돌리는 시험을 세울 수 없다(AGENTS: UI 로직 비대상). 그래서 사슬을 세 층으로 나눠 각각 덮고(패널이 관측을 세움 `ui/panel/tests.rs` · 여럿을 모음 `ui/splitter.rs` · 반영 규칙 `app/drives.rs`) **그 사이의 `for`문 3줄만 리뷰가 지킨다**. 그 사실을 코드 주석에도 적어 두어 다음 세션이 시험 보호를 오해하지 않게 했다(2026-08-17 quality 리뷰 지적 — 종전 주석은 "프레임 시험이 지킨다"고 잘못 주장했다) |
| 워커가 셸을 부르기 전에 COM을 초기화하지 않으면 조회가 실패할 수 있다 | 드라이브 줄에 표시 이름·아이콘이 비어 경로 문자열로 폴백된다 | `fs/thumbnail.rs:277` 선례대로 워커 진입에서 `CoInitializeEx`, 종료에서 `CoUninitialize` |
| 드라이브 줄을 워커가 만들면 **첫 프레임에 드라이브가 없다** | 트리가 잠깐 즐겨찾기만 보인다 | 워커가 **목록과 접근 판정을 나눠 두 번 보낸다**(T4) — 무거운 접근 판정(끊긴 드라이브 실측 2.8초)을 기다리지 않고 목록이 먼저 뜬다. 빈 사이에 "읽는 중" 표시는 두지 않는다(깜빡임이 더 눈에 띈다) |
| `drive_roots`가 시험 헬퍼에서도 쓰여, 그것을 접근 판정 포함 함수로 갈아 끼우면 `cargo test`가 느려진다 | 끊긴 드라이브가 있는 PC에서 시험이 드라이브마다 초 단위로 늘어진다 | 조회를 `list_drives`(판정 없음)와 `is_reachable`(판정)로 나눠 **시험은 앞의 것만** 쓴다(T4) |
| 네트워크 오류 코드 집합(T1)을 좁게 잡으면 일부 실패가 목록에서 샌다 | **문구만** 덜 구체적해진다(`이 폴더를 여는 중 문제가…`로 떨어진다). 목록 자리 표시와 **트리 배지는 영향받지 않는다** | ⓐ 알려진 네트워크 오류 코드를 목록으로 두고 시험으로 고정 ⓑ **배지 판정을 이 깃발에서 떼어 놓았다**(T6 갈래 규칙 — 배지는 "열어서 닿았는가"로만 판정한다). 이 분리가 없으면 코드 하나가 빠질 때 이 plan의 두 목표 중 하나(X 배지)가 통째로 실패한다 |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `EnumOutcome::Error` | `src/fs/enumerate.rs:123,139`(생성) · `src/ui/panel.rs:486`(매치) · `src/panel/panel.rs:284`(매치 — 구 Win32, 컴파일 대상) | 변형에 필드가 생겨 **모든 매치·생성 지점이 컴파일 오류**로 드러난다 |
| `EnumOutcome`(그 밖의 변형) | `src/ui/tree.rs:603` · `src/panel/folder_tree.rs:191` · `src/ui/panel/workers.rs:19,54` · `src/ui/panel/tests.rs:267,514,1058,2216,2314,2344` | `Ok`·`AccessDenied`만 만들거나 타입만 실어 나른다 — **변경 없음**(`Error`를 짓는 자리가 없다) |
| `PanelState::denied_dir` / `shows_denied` | `src/ui/panel.rs:182,255,461,474,1386-1392,1420` · `src/ui/panel/tests.rs:1071,1109` | 세 사유(권한·네트워크·그 밖)를 들도록 타입이 바뀐다 — 시험 2건 갱신 |
| `FolderTreeView::roots` / `drive_roots` | `src/ui/tree.rs:131,172,224-234,643-659` · 자체 시험 `src/ui/tree.rs:865,876` · **`src/ui/panel/tests.rs:106-110`(헬퍼 `drive_labels` — 이를 쓰는 시험 `:1143`,`:1407`,`:1653`)** · **`src/ui/panel/tests.rs:2363`** | 소유가 앱으로 옮겨가며 필드·함수가 이동한다. **다른 파일의 시험 헬퍼 2곳이 이 함수를 직접 부르므로 대체 함수를 함께 정한다**(T4) |
| `FolderTreeView::show_node` | 정의 `src/ui/tree.rs:513` · 호출 `:235`(드라이브 뿌리) · `:566`(하위 폴더 재귀) · 그 안의 `icon_for` `:524`와 `tree_row` `:531`,`:543` | **드라이브 줄과 하위 폴더 줄이 같은 함수를 지난다** — 아이콘 출처(워커 vs `icon_for`)와 배지 유무를 여기서 갈라야 한다 |
| `FolderTreeView::show` (인자) | 정의 `src/ui/tree.rs:203` · 호출 `src/ui/panel.rs:1159`(`PanelState::show` 본문 인라인 — 별도 `show_tree` 함수는 없다) · 시험 헬퍼 `src/ui/tree.rs:899`(`draw_remote`) | 드라이브 줄을 인자로 받는다 — 시험 헬퍼도 함께 고친다 |
| `PanelState::show` (인자) | `src/ui/panel.rs:1103`(정의) · `src/ui/splitter.rs`(호출) · `src/ui/panel/tests.rs`의 그리기 헬퍼(`draw_once*`) | 즐겨찾기와 같은 자리에 드라이브 줄이 더해진다 |
| `PanelOutcome` | `src/ui/panel.rs:107-135` · `src/ui/splitter.rs:92-102`(구조 분해) | 관측 결과 필드 추가 — 구조 분해가 강제 |
| `LayoutOutcome` | `src/ui/splitter.rs:49-84` · `src/ui/app.rs:2718-2725` | 필드 추가 + 앱의 소비 1줄 (컴파일러 미강제 — 위험 등록) |
| `tree_row` | 정의 `src/ui/tree.rs:691` · 호출 **5곳** — `:377`(즐겨찾기) · `:435`,`:448`(원격) · `:531`,`:543`(로컬 `show_node` — 드라이브와 하위가 함께 지난다) | 배지 여부 인자 추가 |

### 4-B. 계약·직렬화 변경
- **세션 저장 형식은 바뀌지 않는다** — 드라이브 연결 상태는 실행할 때마다 다시 확인하는 휘발 정보라 `Session`에 담지 않는다. 스키마 v3 그대로.
- `EnumOutcome`은 프로세스 안에서만 오가는 타입이라 외부 계약이 아니다.

### 4-C. 테스트 파일
- `src/fs/enumerate.rs` (같은 파일의 `#[cfg(test)] mod tests` — 오류 코드 판정)
- `src/app/drives.rs` (신규 — 드라이브 줄 목록의 갱신 규칙)
- `src/ui/panel/tests.rs` (목록 자리 문구·드라이브 상태가 트리까지 닿는지)
- `src/ui/tree.rs` (같은 파일 tests — 드라이브 줄 시험 2건이 이동)

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `fs::enumerate::is_network_error` | `grep -rn "ERROR_BAD_NETPATH\|network" src/fs` → 0건. `src/remote/ftp.rs:635-665`에 FTP 응답 코드 판정이 있으나 그쪽은 **서버 응답 문자열**이라 Win32 오류 코드와 무관 | 신규 — 판정 대상이 다르다 |
| `app::drives::DriveList` | `grep -rn "GetLogicalDrives" src` → `src/ui/tree.rs:645`, `src/panel/folder_tree.rs:212`(구 Win32) 둘뿐이고 상태를 들고 있는 타입은 없다 | 신규. 다만 **모양은 `app::favorites::FavoriteStore`를 그대로 따른다**(앱이 소유 → 화면에 슬라이스로 내려보냄) |
| `fs::drives::list_drives` | `src/ui/tree.rs:643`의 `drive_roots`가 같은 일을 UI 스레드에서 한다 | **이동**(신규 작성 아님) — 그 함수의 몸통을 옮기고 아이콘 인덱스를 함께 담는다. 셸 조회는 **기존 `IconCache::shell_display_name`·`icon_index_for_path`를 그대로 재사용**한다(새 `unsafe SHGetFileInfoW`를 짓지 않는다 — 전제 12) |
| `fs::drives::is_reachable` | `grep -rn "GetFileAttributesW" src` → 0건 | 신규 — 접근 판정을 하는 자리가 지금 없다. 이 task가 더하는 **유일한 새 `unsafe`**다 |
| `enum DriveScan` (워커→앱 전송) | `src/ui/panel/workers.rs:19`의 `Receiver<(u64, EnumOutcome)>`가 비슷한 모양이나 세대 번호가 달린 열거 전용이다 | 신규 — 한 채널로 두 종류(목록·판정)를 보내야 해서 열거형이 필요하다 |
| 트리 줄의 X 배지 그리기 | `grep -rn "circle_filled\|배지" src/ui` → `src/ui/sidebar.rs`의 연결 상태 점(`OK_DOT` 등)이 원을 그리지만 **아이콘 위에 겹치지 않는 독립 점**이다 | 신규 — 겹치는 배지는 처음이다. `theme` 팔레트는 재사용 |

### Verified by
- `grep -rn "EnumOutcome" src/ tests/` → **38줄**(`src/fs/enumerate.rs` 안 15줄 포함), 전부 위 표에 반영. `Error`를 **짓거나 매치하는** 자리는 4곳뿐이고(`enumerate.rs:123,139` · `ui/panel.rs:486` · `panel/panel.rs:284`) 나머지는 `Ok`·`AccessDenied`라 타입 확장에 영향이 없다
- `grep -n "denied_dir\|shows_denied" src/ui/panel.rs src/ui/panel/tests.rs` → 9 hits, 전부 반영
- `grep -rn "drive_roots" src/` → **7 hits, 두 파일**(`ui/tree.rs` 4 · `ui/panel/tests.rs` 3) — 한 파일로 좁힌 첫 조사가 `ui/panel/tests.rs`를 놓쳤고, 리뷰 1라운드에서 잡아 T4에 반영했다
- `grep -n "tree_row(" src/ui/tree.rs` → 정의 1 + 호출 5, 전부 반영
- `grep -rn "GetLogicalDrives" src` → 2 hits (`ui/tree.rs`·`panel/folder_tree.rs`), 후자는 구 Win32라 손대지 않는다

## 동반 변경 판정
| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `README.md:154`의 핵심 플로우 서술 | "폴더가 없거나 여는 중 문제가 생기면 현 위치·목록 유지"가 이번 변경으로 사실과 어긋난다 | T7에 편입 |
| 필수 | `README.md:25`의 폴더 트리 서술 | 드라이브 줄에 X 배지가 생기는데 문면에 없다 | T7에 편입 |
| 필수 | PRD FR-9 문면 | 트리 표시 규격을 적은 자리라 X 배지가 빠지면 요구 정본과 화면이 갈린다 | T7에 편입 |
| 필수 | 위키 `feat-navigation.md:60` | 세션 밖 정본이라 어긋난 채 두면 다음 세션이 옛 서술을 근거로 삼는다 | T7에서 큐 1줄(구현 세션은 위키 본문을 고치지 않는다) |
| 필수 | `src/fs/icons.rs:218-220`의 doc 주석 | "이 메서드와 `icon_index_for_path` 모두 **부르는 곳이 하나씩뿐**"이라 적혀 있는데, T4가 `fs::drives`라는 둘째 호출부를 만든다 — 틀린 주석은 다음 작업을 오도한다 | T4에 편입(그 주석 한 줄 갱신) |
| 선택→채택 | PRD FR-6 문면에 열기 실패 표시 규칙 | 지금 PRD에 이 규칙이 아예 없고 README에만 있다 | 사용자 채택(2026-08-17) — T7에 편입 |
| 선택→채택 | Deferred 항목 "트리 아이콘·표시 이름 조회를 워커로" | 이번에 드라이브 줄을 워커가 만들면서 같은 자리를 손댄다 | 사용자 채택(2026-08-17) — T4에 편입 |
| 무관 | 세션 스키마(`app/settings.rs`) | 연결 상태는 저장 대상이 아니다(4-B) | 건드리지 않음 |
| 무관 | AGENTS.md "아이콘은 `egui_phosphor`에서만" 규약 | 그 규약은 **글꼴 글리프**를 겨냥한다(없는 부호점이 두부가 되는 것을 막는 규칙이며 `is_icon_font`가 글리프만 본다). X 배지는 도형 그리기라 글꼴을 거치지 않는다 — 2026-08-17 셸 비트맵 판정과 같은 축 | 건드리지 않음 |
| 무관 | 구 Win32 `src/panel/folder_tree.rs` | 실행 파일에서 쓰이지 않고 이번 타입 변경도 그 파일의 매치(`Ok`만)에 닿지 않는다 | 건드리지 않음 |

## Decisions
### D1. 네트워크 실패를 어디서 판정하는가
- **Options**: A) 열거 워커가 Win32 오류 코드로 판정해 결과에 싣는다 / B) 화면이 경로의 드라이브 종류(`GetDriveTypeW`)로 판정한다
- **Chosen**: A
- **Rationale**: 오류 코드를 아는 자리는 열거 워커뿐이고 추가 조회가 없다. B는 네트워크 드라이브에서 난 **비네트워크 실패**(디스크 가득 등)까지 네트워크 문구로 만든다.
- **Source**: `src/fs/enumerate.rs:118-124`가 이미 오류 코드로 세 갈래를 나눈다 — 그 자리에 한 갈래를 더하는 것이다.

### D2. `EnumOutcome`을 어떻게 넓히는가
- **Options**: A) `Error { network: bool }`로 필드를 붙인다 / B) `NetworkError` 변형을 새로 더한다
- **Chosen**: A
- **Rationale**: A는 기존 `EnumOutcome::Error` 패턴을 **전부 컴파일 오류로 만들어** 손볼 자리를 컴파일러가 짚어 준다. B는 `_ =>`가 있는 매치를 조용히 지나쳐 한 화면만 갱신되지 않을 수 있다(위키 규약 ⓐ와 같은 축).
- **Source**: 위키 `conventions.md` [2026-08-17] "컴파일러가 잡아 주는 것은 하나뿐" 항목.

### D3. 목록 자리 문구를 몇 개로 두는가
- **Options**: A) 오류 종류로 갈라 둘 / B) 하나로 통일 / C) 상태 줄 문구를 그대로 옮김
- **Chosen**: A (사용자 결정, 2026-08-17)
- **Rationale**: 사용자가 할 일이 갈린다 — 네트워크면 연결을 살피고, 그 밖이면 다시 열어 본다.
- **Source**: 사용자 답변.

### D4. 상태 줄에도 사유를 남기는가
- **Options**: A) 목록 자리에만 적는다 / B) 둘 다 적는다
- **Chosen**: A
- **Rationale**: 사용자가 "폴더 권한 메시지처럼"이라고 못 박았고, 그 방식(`AccessDenied`)은 상태 줄을 비운다. 같은 사유를 두 곳에 적으면 좁은 패널에서 상태 줄이 항목 수를 밀어낸다.
- **Source**: `src/ui/panel.rs:471-481`(권한 없음 경로는 `self.status`를 건드리지 않는다).

### D5. 드라이브 줄을 누가 소유하는가
- **Options**: A) 앱이 소유하고 트리에 내려보낸다 / B) 트리가 각자 만든다(현행)
- **Chosen**: A
- **Rationale**: 트리는 패널마다 있어(전제 8) B는 패널 수만큼 셸·네트워크 조회를 되풀이한다. 연결 상태도 패널마다 갈리면 같은 드라이브에 X가 있는 트리와 없는 트리가 한 화면에 선다.
- **Source**: 즐겨찾기가 같은 이유로 앱 전역이다(전제 7, `decisions.md` [2026-08-16]).

### D6. X 판정을 언제 하고 언제 갱신하는가
- **Options**: A) 시작 시 1회 + 열어 볼 때 갱신 / B) 열어 봤을 때만 / C) A + 주기 재확인
- **Chosen**: A (사용자 결정, 2026-08-17)
- **Rationale**: 누르기 전에도 보인다는 요구를 채우면서 주기 조회의 상시 비용은 지지 않는다.
- **Source**: 사용자 답변. 복구가 늦게 반영되는 한계는 Deferred에 적었다.

### D7. 배지를 무엇으로 그리는가
- **Options**: A) 도형(원+선)을 직접 그린다 / B) phosphor `X_CIRCLE` 글리프를 겹친다
- **Chosen**: A (사용자 결정, 2026-08-17)
- **Rationale**: 탐색기 배지는 **채워진** 원이고 phosphor regular는 윤곽선이라 16px에서 모양이 다르게 읽힌다. 채움 변형(`fill`)은 crate feature 추가가 필요해 의존성이 늘어난다.
- **Source**: `egui-phosphor 0.13` Cargo.toml — `fill`은 기본 비활성 feature. 실측.

### D8. 배지 색을 어디서 가져오는가
- **Options**: A) `theme`에 전용 상수를 새로 둔다 / B) 기존 `theme::ERROR`(#FF6B6B) 재사용 / C) `theme::CLOSE_HOT`(#C42B1C) 재사용
- **Chosen**: A — 이름은 `theme::OFFLINE_BADGE`, 값은 `0xC4,0x2B,0x1C`
- **Rationale**: `ERROR`는 글자용이라 작은 원에 칠하면 연해서 탐색기 배지와 다르게 읽힌다. `CLOSE_HOT`은 값은 맞지만 "닫기 버튼 hover"라는 뜻이라 이 자리에서 이름이 거짓말을 한다. 팔레트는 이미 용도별 상수를 두는 관례다(`OK_DOT`·`OK_TEXT`·`OK_FILL`).
- **Source**: `src/ui/theme.rs:29,52-66`.

## 시각 요소 분해

**기준**: 사용자 제공 탐색기 스크린샷 (`%USERPROFILE%\Desktop\2.png`)

### V-9 대조 결과 (T5, 2026-08-17)
정적 축 8행은 **✅ 일치** — 구현 위치를 각각 지목했다(`ui/tree.rs`의 `draw_offline_badge`·`BADGE_RATIO`·`theme::OFFLINE_BADGE`). spec 리뷰가 같은 대조를 독립적으로 확인했다.

**F-8 인계 (⏳ 미확인 2행)** — 아래 표에 `⚠ 어림값`으로 표시한 두 행은 배지가 9px 남짓이라 기준 스크린샷에서 정밀 측정이 불가능하다. 값은 코드에 반영됐으나(`BADGE_STROKE_RATIO = 0.15`·`BADGE_MARK_RATIO = 0.5`) **눈으로 보는 판정은 완료 선언 직전 F-8에서 사용자 확인으로 한다**:
- X 선 굵기 (배지 지름의 0.15배)
- X 선 길이 (배지 지름의 0.5배)

### 시각 속성
| 요소 | 속성 | 디자인 값 | 확인 방법 |
|------|------|----------|-----------|
| X 배지 | 붙는 대상 | 트리 드라이브 줄의 셸 아이콘 (16px) | 기준 스크린샷 — 배지가 드라이브 아이콘 위에만 있고 글자에는 없다 |
| X 배지 | 위치 | 배지의 **오른쪽 끝·아래 끝을 아이콘 사각형의 오른쪽 끝·아래 끝에 맞춘다**(배지가 아이콘 안쪽에 들어가고 밖으로 넘치지 않는다) | 기준 스크린샷 픽셀 측정(드라이브 줄 `y 30~55`로 한정 — 첫 측정은 스캔 범위에 잘려 값이 틀렸다): 아이콘 `x 110~132`·`y 34~51`, 배지 `x 122~132`·`y 41~50`. 두 오른쪽 끝이 `x 132`로 같고 아래 끝은 1px 안쪽이다 |
| X 배지 | 지름 | 아이콘 한 변의 **0.55배** (16px 아이콘 → 8.8px) | 같은 측정 — 배지 높이 10px(y 41~50) ÷ 아이콘 높이 18px(y 34~51) = **0.55**. 기준 스크린샷의 아이콘 상자는 23×18로 정사각이 아니라(드라이브 아이콘이 가로로 넓다) 폭으로 재면 0.48이 나오는데, 우리 `icon_rect`는 `ROW_ICON` **16×16 정사각**이므로(전제 14) 세로 축을 기준으로 잡았다 |
| X 배지 | 원 | **채워진** 원 (윤곽선 없음) | 기준 스크린샷 — 원 안쪽이 붉게 차 있다 |
| X 배지 | 원 색 | `theme::OFFLINE_BADGE` = `#C42B1C` (D8) | 기준 스크린샷은 붉은 계열까지만 읽힌다(축소·안티에일리어싱). 정확한 원색은 못 뽑아 앱 팔레트 규격으로 정했다 |
| X 배지 | X 선 색 | 흰색 (`Color32::WHITE`) | 기준 스크린샷 — 원 안 X가 흰색이다 |
| X 배지 | X 선 굵기 | 배지 지름의 **0.15배** (8.8px 배지 → 약 1.3px) | ⚠ 어림값 — 배지가 10px 남짓이라 스크린샷에서 선 굵기를 정밀히 잴 수 없다. 값을 못 박아 두는 것은 구현자가 즉석에서 고르지 않게 하려는 것이고, 최종 판정은 화면 확인이다 |
| X 배지 | X 선 길이 | 배지 지름의 **0.5배**(중심에서 양쪽 0.25배씩), 두 대각선 | ⚠ 어림값 — 위와 같다. 기준 스크린샷에서는 X가 원 안쪽에 여백을 두고 들어 있는 것까지만 읽힌다 |
| 드라이브 줄 | 배지 유무 | **네트워크 드라이브가 끊겼을 때만**. 로컬 드라이브·연결된 네트워크 드라이브에는 없다 | 기준 스크린샷의 다른 드라이브 줄에는 배지가 없다 |
| 드라이브 줄 | 그 밖의 표시 | 표시 이름·글자색·선택 강조는 지금 그대로 | 기준 스크린샷 — 끊긴 줄도 글자가 흐려지지 않는다 |

## Tasks

- [x] T1. 열거 결과에 네트워크 실패 여부를 싣는다
  - **Type**: C
  - **Design**: ① `src/fs/enumerate.rs`에 둔다(오류 코드를 아는 유일한 자리). ② 신규 심볼 — `EnumOutcome::Error { network: bool }`(변형 확장), `fn is_network_error(code: WIN32_ERROR) -> bool`(비공개 판정 함수 — 알려진 네트워크 오류 코드 집합). ③ 의존 방향 — `fs`는 아무것도 새로 참조하지 않고, `ui`·`panel`이 이 값을 읽는다. ④ 비추상화 선언 — 오류를 타입으로 감싸지 않는다(`EnumError` 열거형·오류 코드 원본 전달 모두 안 한다). 화면이 갈라 적어야 하는 것은 "네트워크인가"뿐이라 `bool` 하나로 족하다.
  - **Acceptance**:
    - Given `ERROR_BAD_NETPATH`(53)·`ERROR_BAD_NET_NAME`(67)·`ERROR_NETWORK_UNREACHABLE`(1231)·`ERROR_NO_NET_OR_BAD_PATH`(1203)·`ERROR_NETNAME_DELETED`(64)·`ERROR_UNEXP_NET_ERR`(59)·`ERROR_REM_NOT_LIST`(51)·`ERROR_DEV_NOT_EXIST`(55), When `is_network_error`, Then 전부 `true`
    - Given `ERROR_DISK_FULL`(112)처럼 목록 밖 코드, When `is_network_error`, Then `false`
    - `cargo build`가 통과하고, `EnumOutcome::Error`를 매치·생성하던 4곳(`fs/enumerate.rs:123,139` · `ui/panel.rs:486` · `panel/panel.rs:284`)이 모두 새 형태로 갱신돼 있다
  - **Files**:
    - 주: `src/fs/enumerate.rs`
    - 동반: `src/ui/panel.rs` · `src/panel/panel.rs`(구 Win32 — 컴파일 유지용 최소 갱신)
    - 테스트: `src/fs/enumerate.rs`(같은 파일 `mod tests`)
  - **Edge Cases**:
    - 열거 **도중** 실패(`FindNextFileW`, `enumerate.rs:139`)에도 같은 판정을 적용한다 — 네트워크가 목록을 읽는 중에 끊기는 경우가 있다
    - `e.code().0 as u32 & 0xffff` 마스킹이 1000번대 코드(1203·1231)에도 맞는지 확인한다 — HRESULT 하위 16비트라 65535까지는 그대로 남는다
  - **Halt Forecast**:
    - (i) 구 Win32 `panel/panel.rs`가 이 타입을 쓰는지 몰라 빌드가 깨진다 → 4-A 표에 그 자리(`:284`)를 이미 적었다
  - **Depends on**: -

- [x] T2. 목록 자리에 열기 실패 사유를 적는다
  - **Type**: C
  - **Design**: ① `src/ui/panel.rs`의 기존 `denied_dir` 자리를 넓힌다. ② 신규 심볼 — `enum ListBlock { AccessDenied, NetworkUnavailable, OpenFailed }`(그 파일 안 비공개), 필드 `blocked: Option<(PathBuf, ListBlock)>`(`denied_dir`을 대체), `fn blocked_hint(&self) -> Option<&'static str>`(`shows_denied`를 대체 — 지금 보는 곳이 막힌 그곳이면 적을 문구를 돌려준다). ③ 의존 방향 — `ui::panel`이 `i18n`을 부르고, 아무도 이 열거형을 밖에서 보지 않는다. ④ 비추상화 선언 — 사유별 표시 전략을 트레이트로 만들지 않는다(세 갈래가 문구 하나씩만 다르다).
  - **Acceptance**:
    - Given 네트워크가 끊긴 드라이브, When 그 경로로 이동, Then 주소창·트리가 **그 경로**를 가리키고 목록에는 `..` 한 줄만 남으며 그 자리에 `네트워크 드라이브에 연결할 수 없어 내용을 표시할 수 없습니다`가 뜬다
    - Given 그 밖의 열기 실패, When 그 경로로 이동, Then 같은 방식으로 `이 폴더를 여는 중 문제가 생겨 내용을 표시할 수 없습니다`가 뜬다
    - Given 위 두 경우, When 이동 직후, Then **상태 줄에는 사유가 적히지 않는다**(`self.status`가 비어 있다)
    - Given 폴더를 찾을 수 없는 경우(`NotFound`), When 이동 시도, Then 지금처럼 현 위치·목록이 유지되고 상태 줄에만 사유가 뜬다
    - 권한 없음의 기존 동작과 문구(`list_access_denied`)는 그대로다
  - **Files**:
    - 주: `src/ui/panel.rs`
    - 동반: `src/i18n/mod.rs`(문구 2개 신설 — `list_network_unavailable` · `list_open_failed`)
    - 테스트: `src/ui/panel/tests.rs`(기존 권한 시험 2건 갱신 + 네트워크·일반 실패 시험 신설)
  - **Edge Cases**:
    - 막힌 폴더에서 **다른 폴더로 옮겨 성공**하면 표시가 사라진다(기존 `denied_dir = None` 자리와 같다)
    - 원격 탭으로 전환하면 저절로 꺼진다 — 판정이 깃발이 아니라 **경로 대조**이기 때문이다(`shows_denied`의 설계를 그대로 잇는다)
    - 막힌 폴더는 **감시하지 않는다** — 권한 없음과 같다(읽지 못한 폴더를 감시하면 실패를 되풀이한다)
    - 목록이 비어 있고 읽는 중도 아닐 때만 문구를 그린다 — 기존 빈 폴더 안내와 같은 자리라 셋 중 하나만 뜬다
  - **Halt Forecast**:
    - (i) 문구를 어떻게 쓸지 → D3에서 확정(문안은 위 Acceptance에 그대로)
  - **Depends on**: T1

- [x] T3. 드라이브 줄 자료 타입·조회 함수와 연결 상태 모델을 만든다
  - **Type**: C
  - **Design**: ① 상태·규칙은 `src/app/drives.rs`(신규 — 순수 로직이라 시험이 덮는다), 자료 타입 `DriveRow`·`DriveScan`과 **Win32 조회 함수는 `src/fs/drives.rs`에 둔다**(`app`이 `fs`를 참조하는 것이 이 레포의 기존 방향이다: `src/app/sidebar.rs:10`. 조회 함수가 T4가 아니라 여기 온 경위는 아래 「경계 갱신」). ② 신규 심볼 — `struct DriveRow { path: PathBuf, label: String, icon: i32, network: bool, offline: bool }`(**필드는 `pub`** — `fs::drives`가 만들고 `app::drives`·시험이 읽고 짓는다. `list_drives`가 만들 때 `offline`은 **언제나 `false`**로 시작한다: 판정 전에는 배지를 두지 않는다), `struct DriveList { rows: Vec<DriveRow> }`와 그 메서드 셋 — `rows() -> &[DriveRow]` · `replace(Vec<DriveRow>)`(워커의 **첫** 결과인 목록을 담는다) · `apply_reachable(&[(PathBuf, bool)])`(워커의 **둘째** 결과인 접근 판정을 덮는다 — 목록을 다시 만들지 않는다) · `observe(&Path, reachable: bool)`(사용자가 열어 본 결과 하나를 반영한다) · 자유 함수 `fn drive_root_of(path: &Path) -> Option<PathBuf>`(경로에서 드라이브 뿌리를 뽑는다). **`bool`의 뜻은 세 자리 모두 `reachable`(닿았으면 `true`)로 통일한다** — `offline`은 `DriveRow`의 필드 이름으로만 쓰고 인자 이름으로는 쓰지 않는다(극성이 뒤집혀 읽히면 배지가 정반대로 붙는다). ③ 의존 방향 — `app::drives`는 표준 라이브러리와 `fs::drives::DriveRow`만 쓰고, `ui::app`이 소유하며 `ui::tree`가 슬라이스로 읽는다. ④ 비추상화 선언 — 저장소 트레이트를 두지 않는다(구현이 하나뿐이고 `FavoriteStore`도 구조체 하나다).
  - **Acceptance**:
    - Given `Z:\`가 `offline: false`인 목록, When `observe(Path::new(r"Z:\Docs"), false)`, Then `Z:\` 행만 `offline: true`가 된다
    - Given 같은 목록, When `observe(Path::new(r"C:\Users"), false)`, Then **네트워크가 아닌 드라이브는 바뀌지 않는다**(로컬에는 배지를 두지 않는다 — Out of Scope)
    - Given 목록에 없는 드라이브의 경로, When `observe`, Then 아무 일도 없고 패닉하지 않는다
    - Given `offline: true`인 목록, When `replace`로 새 목록이 오고 이어 `apply_reachable`이 판정을 덮음, Then 판정 결과대로만 남는다(옛 값이 살아남지 않는다)
    - Given `list_drives`가 갓 만든 행, When 접근 판정이 아직 오지 않음, Then `offline`은 `false`다(판정 전에는 배지를 두지 않는다)
    - `drive_root_of`가 `Z:\Docs\a.txt` → `Z:\`, `Z:\` → `Z:\`, UNC 경로(`\\host\share\x`) → `None`을 돌려준다
  - **Files**:
    - 주: `src/app/drives.rs`(신규 — `DriveList`와 규칙) · `src/fs/drives.rs`(신규 — `DriveRow`·`DriveScan` 타입과 **조회 함수 `list_drives`·`is_reachable`·`is_network_drive`까지 함께**)
    - 동반: `src/app/mod.rs` · `src/fs/mod.rs`(모듈 등록) · `Cargo.toml`(`Win32_System_WindowsProgramming` feature — `DRIVE_REMOTE` 상수가 그 모듈에만 있다)
    - 테스트: `src/fs/drives.rs`(조회 함수 시험 5건) · `src/app/drives.rs`(같은 파일 `mod tests` 8건)
  > **경계 갱신 (2026-08-17, 구현 중)**: 원안은 이 task를 `DriveRow` **정의만**으로 한정하고 조회 함수를 T4로 미뤘다. 실제로는 한 파일에 함께 만들었고 spec 리뷰가 이 선취를 BLOCKER로 잡았다 — **되돌리지 않고 경계를 여기로 옮긴다**: 코드가 T4 Design ②의 시그니처와 글자 그대로 일치해 품질 문제가 없고, 지웠다가 T4에서 다시 쓰는 것은 순수 낭비다. 원안이 경계를 그은 목적(*"T3이 T4를 기다려 순서가 꼬이는 것"*)은 오히려 이 배치에서 더 잘 지켜진다(기다릴 것이 없다). **T4에 남는 fs 쪽 몫은 워커 `spawn_scan` 하나**이며 나머지(트리 소유 이동·`show_node` 홉·`ui/app.rs` 배선·시험 헬퍼 대체)는 그대로다.
  - **Edge Cases**:
    - 드라이브 문자가 없는 경로(UNC·상대 경로)는 `None`으로 조용히 빠진다
    - 대소문자 — Windows 드라이브 문자는 대소문자를 가리지 않으므로 비교 전에 맞춘다
    - 목록이 아직 비었을 때(워커 대기) `observe`가 와도 안전하다
  - **Halt Forecast**:
    - (i) `ExplorerApp`을 시험에서 만들 수 없어 배선을 못 덮는다 → 갱신 규칙을 이 순수 모듈에 둬서 해소(위키 규약 ⓑ)
  - **Depends on**: -

- [x] T4. 드라이브 줄을 워커가 만들어 앱이 소유하게 한다
  - **Type**: D
  > **T3 선취 반영 (2026-08-17)**: 아래 Design ②가 열거한 `list_drives`·`is_reachable`·`DriveScan`은 **T3에서 이미 만들었다**(T3 「경계 갱신」 참조). **이 task에 남은 fs 쪽 몫은 워커 `spawn_scan` 하나**이고, 나머지(트리 소유 이동·`show_node` 홉·`ui/app.rs` 배선·시험 헬퍼 대체·`draw_remote` 갱신)는 그대로다. Files의 `Cargo.toml`도 T3으로 옮겨 갔다.

  - **Design**: ① Win32 조회는 `src/fs/drives.rs`(T3이 만든 파일), 소유·배선은 `src/ui/app.rs`, 트리 쪽 홉은 `src/ui/tree.rs`. ② 신규 심볼 — **조회를 둘로 나눈다**: `fs::drives::list_drives(icons: &mut IconCache) -> Vec<DriveRow>`(논리 드라이브 열거 → `GetDriveTypeW`로 종류 판정 → **기존 `IconCache::shell_display_name`·`icon_index_for_path` 래퍼를 그대로 써서** 표시 이름·아이콘 인덱스를 얻는다. **접근 판정을 하지 않아 빠르다** — 시험도 이것을 쓴다. 지금 `drive_roots(icons)`와 시그니처가 같아 시험 헬퍼가 거의 그대로다)와 `fs::drives::is_reachable(root: &Path) -> bool`(`GetFileAttributesW` 성공 여부 — 끊긴 드라이브에서 초 단위가 걸리는 무거운 쪽. 여기만 새 `unsafe`다). `fs::drives::spawn_scan(tx, ctx)`가 워커에서 **자기 `IconCache::new()`를 만들어** 두 번 보낸다 — ⓐ `list_drives` 결과를 먼저(화면에 드라이브 줄이 곧 선다) ⓑ 네트워크 드라이브만 `is_reachable`로 판정한 결과를 뒤이어. 전송 타입은 `enum DriveScan { Listed(Vec<DriveRow>), Reachability(Vec<(PathBuf, bool)>) }` 하나로 두고 채널도 하나다(둘로 나누면 수신부가 두 곳이 된다). `ui::app`에는 `drives: DriveList` 필드와 `Receiver<DriveScan>` 수신부. ③ 의존 방향 — **`DriveRow`는 `fs::drives`에 둔다**(`app::drives::DriveList`가 그것을 담는다). `app`이 `fs`를 참조하는 것이 이 레포의 기존 방향이고(`src/app/sidebar.rs:10`이 `fs::icons`를 쓴다), 반대로 두면 `fs`가 처음으로 `app`을 보게 된다. `fs::drives`는 `ui`를 모른다(AGENTS 단방향). ④ 비추상화 선언 — 드라이브 조회를 트레이트로 감싸지 않는다. **셸 조회 래퍼를 새로 짓지도 않는다** — `IconCache`의 두 메서드가 이미 그 일을 하고, 새로 지으면 `unsafe SHGetFileInfoW`가 한 벌 더 생긴다(그 메서드들의 doc 주석이 "부르는 곳이 하나씩뿐"이라 적은 근거도 깨진다).
  - **Design (트리 홉)**: `show_node`는 **드라이브 뿌리와 하위 폴더가 함께 지나는 함수**다(호출 `tree.rs:235`·`:566`). 인자 `drive: Option<&DriveRow>`를 더해 갈라 쓴다 — 드라이브 뿌리 호출은 `Some(row)`라 아이콘을 `row.icon`에서 얻고(워커가 준 값), 하위 재귀 호출은 `None`이라 지금처럼 `icon_for(path, icons)`로 얻는다. 배지 여부(`row.offline`)도 같은 자리에서 갈린다(T5가 쓴다). `FolderTreeView::roots` 필드와 그것을 채우는 `tree.rs:224-234` 분기는 사라지고, `show`가 받은 슬라이스를 그대로 훑는다. **인자가 8개가 되어 `clippy::too_many_arguments`에 걸리므로** 같은 파일의 `show_remote_node`(`tree.rs:412`)처럼 `#[allow(clippy::too_many_arguments)]`를 단다 — `-D warnings`라 달지 않으면 빌드가 깨진다.
  - **Acceptance**:
    - `src/ui/tree.rs`에서 `drive_roots` 함수와 `FolderTreeView::roots` 필드가 사라지고, `FolderTreeView::show`가 드라이브 줄(`&[DriveRow]`)을 **인자로** 받는다
    - Given 앱 시작, When 첫 프레임, Then **드라이브 줄을 만드는 갈래**가 UI 스레드에서 셸을 부르지 않는다 — 확인 수단은 ⓐ `src/ui/tree.rs`에서 `drive_roots`와 `IconCache::shell_display_name` 호출이 사라졌고 ⓑ `show`의 `TreeSource::Local` 뿌리 분기와 `show_node`의 `drive: Some(_)` 갈래에 `icon_for` 호출이 없어 아이콘이 `DriveRow::icon`에서만 온다는 것. **`icon_for` 함수 자체는 tree.rs에 남는다** — 즐겨찾기(`tree.rs:368`)와 하위 폴더(`:524`)가 계속 쓰며, 그 둘은 이 plan의 Deferred가 범위 밖으로 둔 부분이다
    - Given 워커의 첫 결과 도착, When 다음 프레임, Then 모든 패널의 트리에 같은 드라이브 줄이 같은 순서(A→Z)로 선다
    - Given 워커의 둘째 결과(접근 판정) 도착, When 다음 프레임, Then 끊긴 네트워크 드라이브의 `offline`이 `true`가 된다
    - 드라이브 줄의 표시 이름·아이콘은 지금과 같다(끊긴 드라이브도 이름을 얻는다 — 실측)
    - **`src/ui/panel/tests.rs:106-110`의 헬퍼 `drive_labels`와 `:2363`의 인라인 조회가 `fs::drives::list_drives()`를 부르도록 바뀌고**(접근 판정이 없어 시험이 느려지지 않는다), 그것을 쓰는 시험 4건(`:1143`·`:1407`·`:1653`·`:2363`)이 통과한다
    - `src/ui/tree.rs`의 드라이브 시험 2건(`드라이브_루트는_루트_경로_형태다`·`드라이브는_셸_표시_이름으로_보인다`)이 사라진다 — **T3이 `src/fs/drives.rs`에 같은 이름으로 대체본을 이미 만들었으므로 여기서는 원본을 지우기만 한다**(새로 쓰려 하면 이름이 겹친다)
    - `src/ui/tree.rs:899`의 시험 헬퍼 `draw_remote`가 새 인자를 넘기도록 갱신된다(원격 트리는 빈 슬라이스)
  - **Files**:
    - 주: `src/fs/drives.rs`(신규) · `src/ui/tree.rs` · `src/ui/app.rs`
    - 동반: `src/fs/mod.rs`(모듈 등록) · `src/ui/panel.rs:1103,1159`(`PanelState::show` 인자와 `tree.show` 호출 — 별도 `show_tree` 함수는 없고 본문 인라인이다) · `src/ui/splitter.rs`(`show_layout` 인자 — 호출부는 `src/ui/app.rs:2691` 하나다) · `src/app/drives.rs`(T3이 만든 `DriveList`가 `fs::drives::DriveRow`를 가리키게 한다) · `src/fs/icons.rs:218-220`(doc 주석의 "부르는 곳이 하나씩뿐" 갱신 — 동반 변경 판정)
    - 테스트: `src/fs/drives.rs`(옮겨온 시험 2건) · `src/ui/tree.rs:899`(`draw_remote` 헬퍼) · `src/ui/panel/tests.rs:106-110,2363`(헬퍼 2곳) · `src/ui/panel/tests.rs`(드라이브 줄이 트리에 그려지는지)
  - **Edge Cases**:
    - **워커에서 COM 초기화** — `CoInitializeEx(None, COINIT_APARTMENTTHREADED)`로 열고 끝에 `CoUninitialize`(`fs/thumbnail.rs:277` 선례). 실패해도 조회만 폴백되고 앱은 계속 돈다
    - 결과가 오기 전 프레임에는 드라이브 줄이 **없다** — 즐겨찾기만 보인다. 목록 조회는 접근 판정과 나뉘어 있어 곧 도착한다
    - 표시 이름을 얻지 못하면 경로 문자열(`C:\`)로 폴백한다 — 지금 동작 그대로
    - 끊긴 드라이브의 접근 판정이 초 단위로 걸려도(실측 2.8초) 워커라 창이 멈추지 않는다. 드라이브가 여럿 끊겨 있으면 X만 그만큼 늦게 붙는다
    - 앱이 먼저 닫히면 채널 전송이 실패한다 — 무해하게 끝낸다(`DirLoad::start` 선례)
    - 시험은 병렬로 도는데 `SHGetFileInfoW`는 프로세스 전역 셸 상태를 쓴다 — `list_drives`를 부르는 시험은 기존 규약대로 `fs::icons::shell_test_guard()`로 잠근다(`ui/panel/tests.rs:106` 선례)
  - **Halt Forecast**:
    - (i) 워커에서 셸 조회를 어떻게 하는가 → 워커가 자기 `IconCache::new()`를 만들어 기존 두 래퍼를 쓴다(전제 12). 얻은 **인덱스**를 UI 스레드가 그대로 그려도 되는 것은 전제 4에서 확인했다
    - (i) 시험 헬퍼가 사라진 함수를 부른다 → 위 Acceptance가 대체 함수(`list_drives`)를 지정했다
    - (ii-a) `PanelState::show`·`FolderTreeView::show`·`show_node`의 시그니처가 바뀐다 → `## 사전 승인 항목`에 등록
  - **Depends on**: T3

- [x] T5. 끊긴 네트워크 드라이브 줄에 X 배지를 그린다
  - **Type**: C
  - **Design**: ① `src/ui/tree.rs`의 `tree_row`에 배지 그리기를 더하고, 색 상수는 `src/ui/theme.rs`. ② 신규 심볼 — `theme::OFFLINE_BADGE`(색 상수), `fn draw_offline_badge(painter: &egui::Painter, icon_rect: egui::Rect)`(`ui::tree` 안 비공개 — 아이콘 자리를 받아 배지를 그린다). `tree_row`는 인자 `offline: bool`을 받는다. ③ 의존 방향 — `ui::tree`가 `ui::theme`를 참조한다(이미 그렇다). ④ 비추상화 선언 — 배지 종류를 열거형으로 일반화하지 않는다(그릴 배지가 이번엔 하나다).
  - **Design (배지 값이 흐르는 길)**: `tree_row` 호출 **5곳** 중 배지를 켤 수 있는 것은 `show_node`의 두 자리(`tree.rs:531`·`:543`)뿐이고, 그 값은 T4가 더한 `drive: Option<&DriveRow>`에서 온다 — `Some(d) => d.offline`, `None => false`. 즐겨찾기(`:377`)와 원격(`:435`·`:448`)은 `false`를 넘긴다. 배지는 `tree_row`가 **아이콘을 그린 그 사각형**(`icon_rect`)을 그대로 넘겨 받아 그린다 — 자리를 두 번 계산하면 아이콘과 어긋난다.
  - **Acceptance**:
    - Given `offline: true`인 드라이브 줄, When 트리를 그림, Then 그 아이콘의 오른쪽 아래에 채워진 원과 흰 X가 겹쳐 그려진다(`## 시각 요소 분해`의 값 그대로 — 지름은 아이콘 한 변의 0.55배, 배지의 오른쪽·아래 끝이 아이콘 끝과 맞고, 선 굵기 0.15·선 길이 0.5는 그 표가 `⚠ 어림값`으로 표시한 대로 화면 확인으로 판정)
    - Given 로컬 드라이브·연결된 네트워크 드라이브·즐겨찾기·하위 폴더·원격 노드, When 트리를 그림, Then 배지가 없다 — 특히 **하위 폴더는 드라이브 줄과 같은 `show_node`를 지나므로**(T4 홉) 그 자리에서 `false`가 흐르는지 시험이 본다
    - Given 끊긴 드라이브 줄, When 그 줄을 선택, Then 선택 강조·글자색은 다른 줄과 같다(배지만 다르다)
    - 프레임을 그리는 시험이 끊긴 드라이브 줄에서만 배지 도형이 늘어나는 것을 관측한다
  - **Files**:
    - 주: `src/ui/tree.rs`
    - 동반: `src/ui/theme.rs`
    - 테스트: `src/ui/panel/tests.rs`(배지 유무를 셰이프로 관측)
  - **Edge Cases**:
    - 아이콘 텍스처가 아직 안 올라온 프레임(`IconTextures`의 프레임 상한) — **배지는 그대로 그린다**(원안은 "배지도 그리지 않는다"였다). 구현 중 뒤집은 근거: 변환이 실패한 인덱스는 `IconTextures`가 `None`으로 기억해 **다시 시도하지 않으므로**, 묶어 두면 그 드라이브는 아이콘도 배지도 영영 없어 **끊긴 것을 화면으로 알 수 없다** — 이 기능의 목적이 통째로 무력해진다. 텍스처는 대개 몇 프레임 안에 올라와 배지만 보이는 프레임은 스쳐 지나가고, 영구 실패한 자리에서는 아무것도 없는 것보다 배지만이라도 보이는 편이 낫다. **부수 효과**: 시험이 텍스처 성공에 매이지 않아 병렬 실행에서 안정된다(묶여 있던 동안 `cargo test` 병렬 판에서만 간헐 실패했다 — 단일 스레드는 통과. `IconTextures`의 `ImageList_GetIcon`이 `fs::icons::shell_test_guard` 잠금 밖이라 다른 시험과 경합한다)
    - 트리 폭이 좁아 줄이 잘려도 배지는 아이콘 자리에 붙어 함께 잘린다(별도 처리 없음)
  - **Halt Forecast**:
    - (i) 배지 색·크기 → `## 시각 요소 분해`와 D8에서 확정
  - **Depends on**: T4

- [x] T6. 열어 본 결과로 드라이브 상태를 갱신한다
  - **Type**: D
  - **Design**: ① 관측은 `src/ui/panel.rs`(열거 결과를 받는 자리), 반영은 `src/ui/app.rs`. ② 신규 심볼 — `PanelState::observed_drive: Option<(PathBuf, bool)>`(**중간 보관 필드** — `apply_enumerated`(`panel.rs:447`)는 `poll_load` 안에서 돌아 그 자리에서 `PanelOutcome`을 만들 수 없다. 여기 담아 두었다가 `PanelState::show`가 끝나며 `take()`로 옮긴다), `PanelOutcome::drive_observed: Option<(PathBuf, bool)>`(연 경로와 닿았는지), `LayoutOutcome::drive_observed: Vec<(PathBuf, bool)>`(패널 여럿의 관측을 모은다 — `tree_requests` 선례). ③ 의존 방향 — 패널이 값으로 올리고 앱이 `DriveList::observe`로 반영한다(패널은 `DriveList`를 모른다). ④ 비추상화 선언 — 관측 이벤트를 열거형·버스로 만들지 않는다(한 종류뿐이다).
  - **Design (갈래 규칙)**: 배지는 **`network` 깃발이 아니라 "열어서 닿았는가"로 판정한다** — `apply_enumerated`의 다섯 갈래를 이렇게 가른다. `Ok` → `(dir, true)` · `AccessDenied` → `(dir, true)`(권한이 없을 뿐 드라이브에는 닿았다) · `NotFound` → `(dir, false)` · `Error { network: true }` → `(dir, false)` · `Error { network: false }` → `(dir, false)`. **`network` 깃발은 T2의 문구 고르기에만 쓴다** — 배지까지 그 깃발에 걸면 T1의 오류 코드 목록에서 빠진 실패 하나가 곧 "X가 영영 안 붙는" 결함이 된다(Risks 참조). 로컬 드라이브의 실패는 `DriveList::observe`가 걸러낸다(T3).
  - **Acceptance**:
    - Given 끊긴 `Z:\`를 트리에서 누름, When 열거가 실패(`Error`·`NotFound` 어느 쪽이든), Then 그 프레임 뒤 모든 패널의 트리에서 `Z:\` 줄에 X가 붙는다
    - Given X가 붙은 `Z:\`, When 연결이 복구된 뒤 그 드라이브를 다시 열어 성공, Then X가 사라진다
    - Given 권한이 막힌 네트워크 폴더, When 그리로 이동, Then **X가 붙지 않는다**(드라이브에는 닿았다)
    - Given 두 패널이 같은 프레임에 서로 다른 드라이브를 관측, When 프레임 종료, Then 두 관측이 **모두** 반영된다(하나로 압착되지 않는다)
    - `cargo build` 통과 — `merge_panel_outcome`의 구조 분해가 새 필드를 강제한다(전제 5)
  - **Files**:
    - 주: `src/ui/panel.rs` · `src/ui/splitter.rs` · `src/ui/app.rs`
    - 테스트: `src/ui/panel/tests.rs`(패널이 관측을 올리는지) · `src/app/drives.rs`(반영 규칙은 T3에서 이미 덮었다)
  - **Edge Cases**:
    - **`LayoutOutcome`→`ui::app` 홉은 컴파일러가 강제하지 않는다**(전제 6) — 이 한 줄을 빠뜨리면 조용히 갱신이 끊긴다. 프레임을 그리는 시험으로 덮는다
    - 로컬 드라이브를 열어 실패해도 `DriveList::observe`가 무시한다(T3 Acceptance)
    - 원격 탭의 실패는 이 경로에 오지 않는다 — `apply_enumerated`가 원격이면 결과를 버린다(`ui/panel.rs:453-456`)
    - 같은 드라이브를 두 패널이 다르게 관측하면 **나중 것이 이긴다**(마지막 사실이 가장 새롭다)
  - **Halt Forecast**:
    - (ii-a) `PanelOutcome`·`LayoutOutcome`에 공개 필드가 는다 → `## 사전 승인 항목`에 등록
  - **Depends on**: T1, T3, T4

- [ ] T7. PRD·README 문면을 맞추고 위키에 큐를 남긴다
  - **Type**: A
  - **Acceptance**:
    - `docs/prd.md` FR-6 문면에 **열기 실패 표시 규칙**이 더해진다 — 권한 없음·네트워크 끊김·그 밖의 실패는 그 경로로 옮겨 목록 자리에 사유를 적고, 폴더를 찾을 수 없으면 현 위치를 지킨다
    - `docs/prd.md` FR-9 문면에 **끊긴 네트워크 드라이브 줄의 X 배지**가 한 구절로 더해진다
    - `README.md:154`의 핵심 플로우에서 "여는 중 문제가 생기면 현 위치·목록 유지"가 새 규칙으로 고쳐진다
    - `README.md:25`의 폴더 트리 항목에 X 배지 서술이 더해진다
    - `README.md`의 디렉터리 구조에 `app/drives.rs`·`fs/drives.rs` 두 줄이 더해진다
    - 위키 vault 루트 `pending.md`에 `[PROJECT-FACT]` 1줄 — `feat-navigation.md`의 열거 실패 서술이 바뀌었다는 것
  - **Files**:
    - 주: `docs/prd.md` · `README.md`
    - 동반: 위키 vault 루트 `pending.md`(큐 1줄 — 위키 본문은 고치지 않는다)
  - **Edge Cases**:
    - vault가 없거나 읽지 못하면 큐잉을 조용히 건너뛰고 이 plan에 폴백 기록한다
    - PRD·README에 실제 호스트 이름·경로를 적지 않는다 — 문면은 `네트워크 드라이브`처럼 일반 낱말로 쓴다(보안 규칙)
  - **Halt Forecast**:
    - 없음 — 문면 갱신뿐이고 PRD FR-6·FR-9 개정은 사용자가 이미 승인했다(Q5·`## 동반 변경 판정`). vault 부재는 위 Edge Case가 처리한다
  - **Depends on**: T2, T5, T6

## 사전 승인 항목 (일괄 승인 대상)
- T4 — `FolderTreeView::show`·`FolderTreeView::show_node`·`PanelState::show`·`splitter::show_layout`의 시그니처에 드라이브 줄 인자가 는다(계획된 공개 API 변경).
- T4 — `src/ui/tree.rs`의 `drive_roots` 함수와 `FolderTreeView::roots` 필드를 제거하고 `src/fs/drives.rs`의 `list_drives`로 옮긴다. 그것을 부르던 시험 헬퍼 2곳(`ui/panel/tests.rs:106-110`·`:2363`)과 자체 시험 2건(`ui/tree.rs:865,876`)을 함께 옮긴다(계획된 구조 변경 — 파일 삭제는 없다).
- T3·T4 — 새 모듈 파일 `src/app/drives.rs`·`src/fs/drives.rs`를 더하고 각 `mod.rs`에 등록한다(T3이 파일과 `DriveRow`를, T4가 Win32 함수를 넣는다).
- T4 — `src/fs/icons.rs:218-220`의 doc 주석을 갱신한다(호출부가 둘이 되어 "하나씩뿐"이 거짓이 된다 — 동반 변경 판정).
- T6 — `PanelOutcome`·`LayoutOutcome`에 공개 필드가 는다(계획된 공개 API 변경).
- 새 외부 의존성은 없다 — 쓰는 Win32 함수(`GetLogicalDrives`·`GetDriveTypeW`·`GetFileAttributesW`·`SHGetFileInfoW`)는 이미 쓰는 `windows` crate 안에 있다.

## 불가피한 Halt (위임 불가)
- push·master 병합·태그·릴리즈·PR — 구현·검증이 끝난 뒤 별도 승인.
- 위 사전 승인 목록에 없는 파일 삭제·이동.

## Verification Strategy
- 빌드: `cargo build`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 형식: `cargo fmt --check`
- 단위·통합 시험: `cargo test`
- 수동 검증 (사용자): 네트워크 드라이브를 끊은 상태로 앱을 띄워 ① 트리의 그 드라이브 줄에 X가 있는지 ② 그 드라이브를 눌렀을 때 오른쪽 목록이 이전 폴더를 남기지 않고 사유를 보이는지 ③ 연결을 되살려 그 드라이브를 다시 열면 X가 사라지는지.

## Phase Ledger

## Retry Ledger
- T6: 수정 사이클 1/5 (quality MAJOR — 주석이 없는 시험 보호를 주장 / MINOR — 루프 시험 메시지). 재리뷰 OK, spec은 1라운드 OK.
- T5: 수정 사이클 2/5 (1라운드 MAJOR — 하위 폴더 갈래 미검증 / 2라운드 MAJOR — 배지-텍스처 분리의 회귀 방지선 없음). 동일 지적 재발 0, 3라운드 quality OK·2라운드 spec OK.
- T4: 수정 사이클 1/5 (quality M1 — plan이 동반 변경으로 명시한 `fs/icons.rs` doc 주석 갱신 누락). 주석만 바뀌어 증분 재리뷰 ① 판정으로 quality 재실행 생략, spec이 현재 트리에서 반영을 재확인. spec은 판정문 빈 응답 1회 → 재요청으로 회수(D 분기).
- T3: 수정 사이클 1/5 (spec B1 — task 경계 선취. 코드 변경 없이 plan 경계 갱신으로 해소, 재리뷰 BLOCKER 0 / MINOR 2건도 그 자리에서 반영). quality MINOR 1건은 `(판정 유보)` 표시라 대장 미등재.
- T2: 수정 사이클 1/5 (quality M1 — 고아 심볼 `dynamic::open_failed` 제거). 동일 지적 재발 0, 재리뷰 OK.

## Progress Log
- T5-T6 (커밋 5950a80, T6은 리뷰 중): 배지를 그리고, 열어 본 결과를 4단 홉으로 앱까지 올려 반영한다. 시험 808건.
  - 결정: **배지를 아이콘 텍스처와 묶지 않았다** — `IconTextures`가 변환 실패 인덱스를 `None`으로 영구 기억하므로, 묶으면 그 드라이브는 아이콘도 배지도 없어 끊긴 것을 알 수 없다. plan T5 Edge Case를 이 판정으로 갱신.
  - 결정: **배지 판정을 `network` 깃발에서 떼었다** — 관측은 "열어서 닿았는가"로만 한다(`Ok`·`AccessDenied`→true, 나머지→false). 그 깃발에 걸면 T1 오류 코드 목록에서 빠진 실패 하나가 곧 "X가 영영 안 붙는" 결함이 된다.
  - 함정: `ExplorerApp`을 시험에서 만들 수 없어(위키 규약) 앱 배선 한 줄은 시험 밖이다 — 사슬을 세 층(패널 관측·merge 모으기·`DriveList::observe`)으로 나눠 덮었다.
- T3-T4 (커밋 91ded93, 1c418f1): 드라이브 줄의 소유를 트리에서 앱으로 옮겼다. `fs::drives`가 조회·워커를, `app::drives`가 상태 규칙을 들고, 앱이 `poll_drives`로 거둬 4단 홉으로 내려보낸다. `ui/tree.rs`의 `drive_roots`·`roots` 제거.
  - 결정: **조회를 목록과 접근 판정으로 나눴다** — 끊긴 드라이브 판정이 첫 시도에 2.8초라(실측) 한 함수로 묶으면 드라이브 줄이 화면에 서는 것부터 그만큼 늦고 시험도 늘어진다. 워커가 두 번 보낸다.
  - 결정: T3이 T4의 fs 몫을 선취해 **plan 경계를 T3으로 옮겼다**(되돌리지 않음 — 코드가 T4 Design과 일치, 근거는 T3 「경계 갱신」).
  - 함정: 시험 하네스가 **프레임마다** 드라이브를 조회하자 전체 스위트가 10분을 넘겼다(`IconCache::new()` + 셸 잠금을 프레임마다). `OnceLock`으로 프로세스 1회 조회로 바꿔 10.6초로 회복.
- T1-T2 (커밋 7700e8d, 708fd78): 열거 결과에 `network: bool`을 싣고(오류 코드 8종 목록), 목록 자리에 사유를 적는 규칙을 권한 없음 → 네트워크 끊김·그 밖의 실패까지 넓혔다. `denied_dir` → `blocked: Option<(PathBuf, ListBlock)>`. 시험 787건 통과.
  - 결정: `EnumOutcome::Error`에 **변형을 더하지 않고 필드를 붙였다** — 컴파일러가 매치 지점 4곳을 모두 짚어 주게 하려는 것(`_ =>`가 있는 곳을 조용히 지나는 것을 막는다). plan D2 그대로.
  - 결정: 세 사유의 공통 처리를 `block_list`로 모으고 갈라지는 것은 문구뿐으로 두었다(plan 비추상화 선언 — 전략 트레이트 없음).

## Next Steps

## Open Questions
- [x] Q1: 목록에 오류를 적는 범위 → **여는 중 문제가 생긴 경우 전부**(폴더를 찾을 수 없는 경우는 현 위치 유지) — D3·T2
- [x] Q2: 목록 자리 문구 → **오류 종류로 갈라 적기**(네트워크 / 그 밖) — D3·T2
- [x] Q3: X 표시 시점 → **시작 시 워커 확인 + 열어 볼 때 갱신** — D6·T4·T6
- [x] Q4: 배지 모양 → **빨간 원에 흰 X를 직접 그리기** — D7·T5
- [x] Q5: PRD FR-6에 열기 실패 규칙을 더할지 → **더한다** — T7
- [x] Q6: 보류 항목(트리 조회를 워커로)을 이번에 할지 → **함께 한다**(드라이브 줄에 한정) — T4
- [x] Q7: T4의 "UI 스레드에서 셸을 부르지 않는다" 기준이 리뷰 2라운드 연속 만족 불가로 판정(즐겨찾기·하위 폴더가 드라이브와 같은 `icon_for`를 쓴다) → **기준을 드라이브 갈래로 한정한다**(2026-08-17 사용자 결정, 에스컬레이션 후) — T4 Acceptance 2번째 항목·전제 11
