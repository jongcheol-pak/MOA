# 보기 모드 8종 · 패널 메뉴 · 파일 목록 표시 수정

**PRD**: docs/prd.md (2026-07-29 갱신 — FR-23~FR-26·NFR-9 추가, FR-1·4·16·21·22 정정. 사용자 승인 완료)

## 요구 이해

원문 요청:
> 1. 1번 이미지 분할 메뉴에서 라인 추가 후 아래 메뉴 추가 및 기능 구현 — 보기 / --- / 새로 고침 / --- / 새 파일, 새 폴더 / --- / 닫기
> 2. 분할 패널에 가로 스크롤 표시 안되는 문제 수정
> 3. 목록의 컬럼 가로 크기 변경 안되는 문제 수정
> 4. 2번 이미지처럼 목록에서 이름이 이상하게 나오는 문제 수정
> 5. 3번 이미지처럼 워크스페이스 페널 상단에 ? 문자가 있는데 삭제
> 6. 상단에 있는 메뉴 보기,이동,탭, 워크스페이스, 삭제
> 7. 목록 위에 몇개 항목 표시가 되는데 '폴더 20 파일 10' 이렇게 표시 하도록 하고 오른쪽 위치에 표시 하도록 함.
> 8. 분할 메뉴에서 '보기' 메뉴는 마우스가 가면 별도의 하위 메뉴 팝업으로 표시하고 4번 이미지처럼 윈도우 탐색기의 보기 모드를 모두 구현 가능한지 검토
> 9. 목록에서 파일 및 폴더 이름이 길면 ... 줄임으로 표시

이해한 요구:

1. **탭 스트립의 분할 버튼을 패널 메뉴로 승격한다.** 지금은 네 방향 분할 항목만 있는데(1번 이미지), 구분선을 넣고 `보기`(하위 메뉴)·`새로 고침`·`새 파일`·`새 폴더`·`닫기`를 더한다. `닫기`는 **패널 닫기**다(사용자 확정). 새 항목은 표시만이 아니라 동작까지 구현한다.
2. **파일 목록의 표시 결함 3건을 고친다.** ① 긴 이름이 줄바꿈되어 아래 행과 겹쳐 보이고(2번 이미지) ② 열 경계를 끌어도 열 폭이 안 바뀌며 ③ 패널을 좁히면 오른쪽 열이 잘린 채 가로 스크롤이 나오지 않는다. 겹침과 말줄임(9번)은 같은 원인이라 함께 고친다 — 현재 코드가 셀을 자르는 대신 wrap하고 있다.
3. **화면에서 군더더기를 걷어낸다.** 사이드바 상단의 `?`(글꼴에 없는 글리프 `◧`가 두부로 보이는 것, 3번 이미지)와 상단 메뉴 바 4개를 통째로 지운다. 잃는 기능은 없다 — 메뉴 바 항목은 모두 다른 진입점에 있고, 유일한 예외인 '패널 닫기'가 1번의 패널 메뉴로 옮겨간다.
4. **항목 수 표시를 바꾼다.** `30개 항목` → `폴더 20 파일 10`, 줄의 오른쪽 끝에 붙인다. 왼쪽의 '폴더 트리' 토글은 그대로 둔다(사용자 확정).
5. **Windows 탐색기의 보기 모드 8종을 전부 구현한다**(4번 이미지). 진입점은 패널 메뉴의 `보기`이며 **마우스를 올리면 하위 메뉴가 펼쳐진다**. 아이콘 4종·타일·내용 보기에서는 형식 아이콘이 아니라 **실제 썸네일 미리보기**를 보인다(사용자 확정). 모드와 열 폭은 패널마다 독립이고 세션에 저장된다.

## Goal

파일 목록을 Windows 탐색기 수준의 보기 모드 8종·썸네일·열 조작을 갖춘 목록으로 끌어올리고, 상단 메뉴 바를 패널 메뉴로 대체해 조작 진입점을 패널 안으로 모은다.

## Scope

- `src/ui/file_list.rs` — 공통 상태·입력·모드 디스패치 (기존 파일, 상세 렌더는 분리)
- `src/ui/list_details.rs` (신규) — 자세히 보기: 헤더·열 폭 드래그·가로 스크롤·말줄임
- `src/ui/list_grid.rs` (신규) — 아이콘 4종·타일·목록·내용 보기 렌더
- `src/ui/view_mode.rs` (신규) — 보기 모드 enum과 배치 계산 (순수 로직·테스트 대상)
- `src/fs/thumbnail.rs` (신규) — 썸네일 워커·LRU 캐시 (`IShellItemImageFactory`)
- `src/fs/icons.rs` — 큰 아이콘 이미지 리스트 획득 (`SHGetImageList`)
- `src/fs/create.rs` (신규) — 새 폴더·새 파일 생성과 중복 회피 이름
- `src/ui/icon_tex.rs` — 텍스처 캐시 키를 `(이미지 리스트, 인덱스)`로 교정
- `src/ui/menu.rs` — 패널 메뉴 구성·명령 추가, 메뉴 바 제거
- `src/ui/tabs.rs` — 분할 버튼 → 패널 메뉴 버튼
- `src/ui/panel.rs` — 항목 수 줄, 패널 메뉴 명령 처리, 열 폭·모드 소유
- `src/ui/sidebar.rs` — 상단 토글 스트립 제거
- `src/ui/app.rs` — 메뉴 바 호출 제거, 새 명령 배선, 사이드바 토글 처리 정리
- `src/ui/splitter.rs` — 패널 메뉴 명령의 상향 중계 (`LayoutOutcome`)
- `src/ui/session.rs`·`src/app/settings.rs` — 패널별 보기 모드·열 폭 저장
- `src/app/window.rs` — `PanelSession` 필드 나열 리터럴 보정 (레거시 Win32 판이지만 lib 타깃에 포함돼 컴파일된다). 같은 보정이 `src/app/settings.rs`의 테스트 픽스처 4곳에도 필요하다 (D14 — 리터럴 총 6곳)

## Out of Scope

- 목록에서 이름 바꾸기 인라인 편집 — 새로 만든 항목도 기본 이름으로 두고 이름 변경은 셸 컨텍스트 메뉴에 위임한다 (PRD Out of Scope에 등재)
- 열 추가·제거·순서 변경(만든 날짜 열 등) — 이번엔 기존 4열의 폭만 조절한다
- 보기 모드별 정렬·그룹화 UI (그룹으로 표시·정렬 기준 하위 메뉴)
- 썸네일 회전·EXIF 방향 보정
- 탭 스트립의 탭 폭 고정 (Deferred 대장에 이미 등재)

## Deferred / Follow-up

- 열 추가·제거·열 순서 변경 — 이번 열 폭 작업으로 열 메타데이터 구조가 생기므로 후속 확장 지점이 열린다
- 보기 모드별 마지막 정렬 기준 기억 (지금은 모드를 바꿔도 정렬은 유지)
- [SUGGEST] 빈 영역 클릭 처리가 두 렌더 모듈에서 서로 다른 기법이다 — 자세히 보기는 콘텐츠 아래 사각형을 잡고, 격자는 항목 클릭 여부를 플래그로 사후 억제한다(격자의 잔여 여백이 사각형으로 떨어지지 않아서). `list_common`에 헬퍼로 뽑아 문서화하면 다음 보기 모드 추가 시 참조점이 된다 (T10 quality S1)
- [SUGGEST] `list_grid::show`가 `visible: &mut Vec<PathBuf>`로 "이번 프레임에 보인 파일"을 부수 출력한다 — 즉시 모드에서 렌더 도중 `&mut PanelState`를 잡을 수 없어 택한 절충이다. 렌더와 수집을 나누는 더 나은 구조가 있으면 검토 (T14 quality m1)
- [SUGGEST] `PanelState::from_tabs`의 인자가 4개(tabs·active_tab·columns·view_mode)로 늘었다 — 유일한 호출부가 이미 `PanelTabs` 구조체를 갖고 있으므로 그것을 통째로 넘기면 필드가 늘어도 시그니처가 안 바뀐다 (T12 quality S1)
- [SUGGEST] 옛 세션 호환 테스트가 JSON 문자열 replace로 필드를 걷어내 필드가 늘 때마다 목록이 길어진다 — `serde_json::Value`에서 키를 프로그램적으로 제거하면 테스트를 안 건드려도 된다. 다만 replace 누락을 잡는 자기검증 단언이 있어 조용히 통과하지는 않는다 (T12 quality S2)
- [SUGGEST] T11 Design은 `show_tile_cell`·`show_content_row` 두 함수를 예고했으나 실제로는 `draw_multiline_cell` 하나 + 공용 `draw_stacked`로 구현했다 — 두 모드가 "아이콘 + 여러 줄 텍스트"라는 골격을 공유해 묶는 편이 단순했다. 동작·커버리지 영향 없음 (T11 spec M1)
- [SUGGEST] `file_list::show`의 4-튜플 분해 — `DetailsOutcome`·`GridOutcome`이 `sort_click` 하나만 다르므로 공통 필드를 묶은 타입으로 정리하면 튜플 분해가 사라진다 (T10 quality S2)
- ~~[SUGGEST] `list_details`가 `file_list`를 역참조한다 — 3번째 사용처가 생기면 공용 모듈로 옮긴다 (T2 spec 리뷰 M1)~~ → **T10에서 이행 완료** (`ui/list_common.rs` 신설, 순환 해소 확인)
- [SUGGEST] `elided_galley`가 Design 명세와 달리 `color` 인자를 받지 않고 `theme::TEXT`를 직접 참조한다 — 호출부가 항상 같은 색을 넘기던 터라 인자를 뺀 편이 단순하나, 나중에 색이 갈리는 셀(비활성 항목 등)이 생기면 인자로 되돌린다 (T1 spec 리뷰 M1)
- **자세히 보기의 고정 헤더** — 지금은 헤더가 스크롤 밖이라 세로 스크롤에도 고정돼 있는데, T2에서 가로 스크롤을 위해 헤더를 본문과 같은 `ScrollArea` 안에 넣으면 세로 스크롤 시 헤더가 함께 올라간다. 가로는 같이 움직이고 세로만 고정하려면 두 영역의 스크롤 오프셋을 수동 동기화해야 해 이번 범위를 넘는다
- 이전 plan의 Deferred 2건(`nav_button`의 `DEFAULT_ICON_PX` 노출, 탭 폭 고정)은 `docs/plans/deferred.md` 대장에 그대로 둔다 — 이번 작업과 무관

## 사전 승인 항목 (일괄 승인 대상)

이 plan을 승인하면 아래는 그 지점에서 다시 묻지 않고 진행한다.

1. **신규 파일 5개 추가** — `ui/list_details.rs`·`ui/list_grid.rs`·`ui/view_mode.rs`·`fs/thumbnail.rs`·`fs/create.rs`. `ui/file_list.rs`가 551줄인데 보기 모드 8종을 한 파일에 넣으면 1500줄을 넘는다. 기존 관례대로 `ui/`·`fs/` 바로 아래 평면 배치한다(서브디렉터리를 만들지 않는다).
2. **`Cargo.toml`의 windows crate feature 추가 (필요할 때만)** — `IShellItemImageFactory`·`SHGetImageList`는 이미 활성화된 `Win32_UI_Shell`에 있음을 확인했으므로 **추가가 불필요할 가능성이 높다**. 컴파일 시 `SIIGBF` 플래그·`IImageList` 등에서 feature 부족이 드러나면 그때 최소 범위로 더한다. 새 crate 의존성은 추가하지 않는다.
3. **세션 스키마에 필드 2개 추가** — `PanelSession`에 `view_mode`·`columns`를 `#[serde(default)]`로 더한다. **`SESSION_VERSION`은 2로 유지한다**(D5 참조 — 올리면 기존 사용자의 레이아웃이 통째로 초기화된다). 이에 따라 이 구조체를 **필드 나열 리터럴로 만드는 6곳**(`ui/session.rs:53`·`app/window.rs:917`·`app/settings.rs`의 테스트 픽스처 4개)도 함께 고친다 (D14).
4. **`ui/icon_tex.rs`의 캐시 키 변경** — `HashMap<i32, _>` → `HashMap<(isize, i32), _>`. 공개 메서드 `get`의 시그니처는 그대로다(himl을 이미 받고 있다).
5. **`ui/menu.rs`의 `Command` enum 변경**(`NewFile`·`NewFolder`·`SetViewMode` 추가, **생성처가 사라지는 `NewWorkspace`·`RenameWorkspace`·`RemoveWorkspace` 제거**)과 `show_menu_bar`·`MenuState` 제거 — 호출부는 `ui/app.rs` 1곳뿐임을 확인했다. variant 제거도 이 위임에 포함된다(기능 자체는 사이드바 경로로 유지되며 사라지지 않는다).
6. **로컬 작업 브랜치 commit** (task별 완료 commit·체크포인트).

## 불가피한 Halt (위임 불가)

- push·master 병합·태그·PR — 구현·검증이 끝난 뒤 별도 승인
- 세션 파일(`%APPDATA%\FileExplorer\settings.json`) 삭제·초기화 — 사용자 데이터다. 스키마 호환 실패가 확인되면 멈추고 보고한다
- 썸네일 실측 메모리가 NFR-9 상한(패널당 200장·약 50MB)을 크게 벗어나 상한 자체를 조정해야 하는 경우 — NFR 변경은 PRD 변경이라 승인이 필요하다

## Investigation Log

| 확인 사항 | 방법 | 결과 |
|---|---|---|
| 이름 겹침(4번)의 원인 | `ui/file_list.rs:370` Read | `painter.layout(text, font, color, width)`의 4번째 인자는 **wrap 폭**이라 자르지 않고 **줄바꿈**한다. 행 높이는 20px 고정이라 2줄이 되면 다음 행과 겹친다. 주석은 "잘라 그린다"고 적혀 있어 코드와 어긋난다 |
| 말줄임(9번)과 겹침(4번)의 관계 | 위와 동일 | 같은 한 줄이 원인이다. `LayoutJob`에 `TextWrapping { max_rows: 1, overflow_character: Some('…') }`를 주면 두 문제가 함께 해소된다 |
| 열 폭 변경 불가(3번)의 원인 | `ui/file_list.rs:19-21` Read | `COL_NAME_W`·`COL_SIZE_W`·`COL_TYPE_W`가 `const`다. 드래그 핸들도 없고 폭을 담을 상태 필드도 없다 — 미구현이지 버그가 아니다 |
| 가로 스크롤 없음(2번)의 원인 | `ui/file_list.rs:264` Read | `ScrollArea::vertical()`만 쓴다. 열 폭 합(560px + 마지막 열)이 패널보다 넓으면 마지막 열 폭이 음수가 되어 `width <= 0.0` 분기(366행)로 그려지지 않고, 이름 열도 잘린 채 스크롤 수단이 없다 |
| `?` 문자(5번)의 정체 | `ui/sidebar.rs:20` Read | `TOGGLE_GLYPH: &str = "◧"`. 이 글리프가 번들 글꼴(egui-phosphor·한글 글꼴)에 없어 두부로 렌더된다. 나머지 아이콘은 phosphor를 쓰는데 이 하나만 유니코드 도형 문자다 |
| 사이드바 토글을 지워도 접기·펴기가 되는가 | `ui/titlebar.rs:126·142` Read | 타이틀바 좌측에 `Command::ToggleSidebar` 버튼이 있고, 주석에 "사이드바가 접히면 그 안의 접기 버튼도 함께 사라지므로 다시 펼 수 있는 자리가 여기다"라고 명시. `Ctrl+B`도 있다 — 진입점 2개가 남는다 |
| 메뉴 바를 지우면 잃는 기능 | `ui/menu.rs:63-131` 전건 + 대체 진입점 확인 | 16개 항목 중 15개는 대체 진입점이 있다 — 분할 4종(탭 스트립 버튼)·폴더 트리(`panel.rs:504` 토글)·새로 고침(F5)·사이드바(타이틀바)·뒤로/앞으로/상위(주소창 버튼)·새 탭/탭 닫기(탭 스트립)·새 워크스페이스(사이드바 `+`)·이름 바꾸기/삭제(사이드바 컨텍스트 메뉴). **'패널 닫기'만 마우스 진입점이 사라진다** → 패널 메뉴의 '닫기'가 이를 받는다 |
| 메뉴 바 호출부 | `grep "show_menu_bar\|MenuState"` 전건 | `ui/app.rs:14`(import)·`715`(호출)뿐. `Command` enum은 `ui/app.rs`·`ui/tabs.rs:270`·`ui/titlebar.rs:10`이 함께 쓴다 |
| 호버로 열리는 하위 메뉴(8번) 가능 여부 | egui 0.35 소스 `containers/menu.rs:336-398`·`ui.rs:2787-2798` Read | `SubMenuButton`은 "shows a SubMenu if a Button is **hovered**"로 문서화돼 있고, `ui.menu_button()`을 메뉴 안에서 부르면 `menu::is_in_menu` 판정으로 자동 전환된다. 별도 구현 없이 요구를 만족한다 |
| 큰 아이콘 획득 가능 여부 | `windows-0.62.2/.../UI/Shell/mod.rs:3240` Read | `SHGetImageList<T>(iimagelist: i32) -> Result<T>` 존재. `SHIL_LARGE`(32px)·`SHIL_EXTRALARGE`(48px)·`SHIL_JUMBO`(256px)를 얻어 96px는 256px 축소로 만든다 |
| 썸네일 API 가능 여부 | 같은 파일 `:38662` Read | `IShellItemImageFactory` 정의 존재(`GetImage`). COM이며 이미 `main.rs`가 COM을 초기화한다 |
| 텍스처 캐시의 충돌 위험 | `ui/icon_tex.rs:18·54-67` Read | `by_index: HashMap<i32, _>`가 **이미지 리스트를 구분하지 않는다**. 큰 아이콘 리스트를 더하면 16px 아이콘과 인덱스가 겹쳐 엉뚱한 크기가 그려진다. `get`은 himl을 인자로 받으므로 키만 복합키로 바꾸면 된다 |
| 세션 확장 시 하위 호환 | `app/settings.rs:14·85-89·169-204` Read | `parse_session`이 `version != 2`면 통째로 폴백한다. `#[serde(default)]` 필드 추가는 기존 v2 JSON을 그대로 읽으므로 버전을 올리지 않아야 사용자 레이아웃이 보존된다 |
| 항목 수 표시 현재 위치 | `ui/panel.rs:502-518` Read | `ui.horizontal` 안에서 '폴더 트리' 토글 → 로딩/항목 수 → 오류 문구 순으로 **왼쪽에 몰려 있다** |
| 목록 상태 소유자 | `grep "FileListView"` 전건 | `ui/panel.rs`만 소유(`:118`·`:150`). 열 폭·보기 모드를 `FileListView`에 두면 패널마다 독립이 자동으로 성립한다 |
| 폴더/파일 수 계산 가능 여부 | `fs/enumerate.rs`의 `FileEntry` 확인 | `is_dir` 필드가 있어 `entries.iter().filter(\|e\| e.is_dir).count()`로 즉시 계산된다 |
| Deferred 대장 | `docs/plans/deferred.md` Read | 이번 작업과 겹치는 대기 항목 없음. FR-13(숨김 파일 토글)·FR-14(분할 프리셋)는 이번 범위 밖 |
| `PanelSession`을 만드는 곳 (필드 추가 시 깨지는 리터럴) | `grep "PanelSession"` 전건 | **6곳**이다 — `ui/session.rs:53`(변환), `app/window.rs:917`(레거시 Win32 판 — `lib.rs:5 pub mod app`으로 lib 타깃에 포함돼 컴파일된다), **`app/settings.rs:253·257·261·271`(테스트 픽스처 4개)**. 전부 필드 나열 리터럴이라 필드를 더하면 E0063으로 깨지며, 픽스처는 `cargo build`가 아니라 `cargo test`에서 드러난다 |
| 패널의 요청이 앱까지 올라가는 경로 | `panel.rs:99-103` → `splitter.rs:38-44·88-103` → `app.rs:764` Read | `PanelOutcome{menu,split}`을 `splitter::show_layout`이 받아 `LayoutOutcome{menu,split}`으로 합산(패널 id를 붙여서)한 뒤 `app.rs`가 소비한다. **패널 메뉴 명령을 올리려면 `splitter.rs`의 중계도 반드시 함께 고쳐야 한다** |
| `SidebarAction::ToggleCollapse` 처리부 | `grep "ToggleCollapse"` 전건 | 생성은 `sidebar.rs:164` 한 곳, **처리는 `app.rs:440`**. 변형을 지우면 이 match arm이 함께 깨진다. 이 crate는 bin+lib이라 `pub` 변형에는 dead_code 경고가 나지 않아 "경고 0"만으로는 드러나지 않는다 |
| 패널 수를 아는 곳 (닫기 활성 조건의 출처) | `app.rs:593`·`splitter.rs:51-70` Read | `show_tab_strip(ui, model)`에는 패널 수를 받을 인자가 없다. `splitter::show_layout`은 `computed.panes`를 갖고 있어 그 자리에서 셀 수 있다 → 하향 전달 경로를 D15로 확정 |
| 위키 참조 | vault 확인 | 위키 vault 미설정 — 건너뜀 |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `view_mode::ViewMode` | `grep "ViewMode\|보기 모드"` → 없음 | 신규 |
| `view_mode::layout_metrics` | 격자 배치 계산 없음 (`sidebar.rs`의 `ITEM_PITCH`는 1열 고정) | 신규 — 다열 격자 계산은 재사용할 것이 없다 |
| `list_details::elide_to_width` | `tabs.rs:286 elide()` 있음 | **재사용 불가·신규**. `tabs::elide`는 문자 수(16자) 기준이라 픽셀 폭을 모른다. 열 폭이 가변인 목록에는 폭 기준 말줄임이 필요하다. 다만 egui `LayoutJob`의 `overflow_character`가 이미 픽셀 기준 말줄임을 제공하므로 자체 계산 없이 그것을 쓴다 |
| `fs::create::unique_name` | `workspace.rs`의 "워크스페이스 N" 자동 이름 확인 | 재사용 불가 — 그쪽은 순번만 증가시키고 파일시스템 존재 검사가 없다. 신규 |
| `fs::thumbnail::ThumbnailCache` | `fs/icons.rs`의 `IconCache`, `ui/icon_tex.rs`의 `IconTextures` | 신규. 둘 다 **무제한 캐시**라 축출이 없고(아이콘은 종류 수만큼이라 무해), 썸네일은 파일당 1장이라 LRU 상한이 필수다. `IconTextures`의 프레임당 생성 상한(`MAX_NEW_TEXTURES_PER_FRAME`) 패턴은 참고해 같은 규칙을 적용한다 |
| 큰 아이콘 이미지 리스트 | `fs/icons.rs:37` `SHGetFileInfoW` | `IconCache`를 **확장**해 재사용한다 (새 구조체를 만들지 않는다) |

## 시각 요소 분해

> **기준**: 사용자 첨부 4번 이미지(Windows 11 탐색기 보기 메뉴), 1번 이미지(현재 분할 메뉴).
> **원본**: Windows 11 탐색기의 보기 메뉴 — 소스 코드가 아닌 **화면 이미지**가 기준이므로 근거는 이미지로 적는다.
> **치수 규칙**: 아래 px 값이 곧 구현 상수다(96DPI 기준). 표에 없는 값을 구현자가 새로 정하지 않는다.

### 참조 정합 인벤토리 — 패널 메뉴

| # | 항목(순서) | 라벨 문구(원문) | 종류 | 활성·비활성 조건 | 근거 |
|---|---|---|---|---|---|
| 1 | 보기 | `보기` | 하위 메뉴(호버로 열림) | 항상 활성 | 사용자 요청 원문 |
| 2 | — | (구분선) | 구분선 | — | 사용자 요청 원문 |
| 3 | 오른쪽 분할 | `오른쪽 분할` / `Ctrl+Alt+→` | 항목 | 항상 활성 | 1번 이미지 (기존 유지) |
| 4 | 왼쪽 분할 | `왼쪽 분할` / `Ctrl+Alt+←` | 항목 | 항상 활성 | 1번 이미지 |
| 5 | 위쪽 분할 | `위쪽 분할` / `Ctrl+Alt+↑` | 항목 | 항상 활성 | 1번 이미지 |
| 6 | 아래쪽 분할 | `아래쪽 분할` / `Ctrl+Alt+↓` | 항목 | 항상 활성 | 1번 이미지 |
| 7 | — | (구분선) | 구분선 | — | 사용자 요청 원문 |
| 8 | 새로 고침 | `새로 고침` / `F5` | 항목 | 항상 활성 | 사용자 요청 원문 |
| 9 | — | (구분선) | 구분선 | — | 사용자 요청 원문 |
| 10 | 새 파일 | `새 파일` | 항목 | 항상 활성 | 사용자 요청 원문 |
| 11 | 새 폴더 | `새 폴더` | 항목 | 항상 활성 | 사용자 요청 원문 |
| 12 | — | (구분선) | 구분선 | — | 사용자 요청 원문 |
| 13 | 닫기 | `닫기` / `Ctrl+Shift+W` | 항목 | 패널 2개 이상일 때만 활성 | 사용자 요청 원문 + 확정(패널 닫기) |

> 분할 4종의 문구·순서·단축키 표기는 기존 `menu::split_items`(1번 이미지에 보이는 것)를 **그대로** 쓴다.

### 참조 정합 인벤토리 — '보기' 하위 메뉴

| # | 항목(원본 순서) | 라벨 문구(원문) | 아이콘 크기 | 표시 요소 | 배치 | 근거 |
|---|---|---|---|---|---|---|
| 1 | 아주 큰 아이콘 | `아주 큰 아이콘` | 256px | 아이콘 + 이름(2줄까지) | 격자·가로 흐름 | 4번 이미지 |
| 2 | 큰 아이콘 | `큰 아이콘` | 96px | 아이콘 + 이름(2줄까지) | 격자·가로 흐름 | 4번 이미지 |
| 3 | 보통 아이콘 | `보통 아이콘` | 48px | 아이콘 + 이름(2줄까지) | 격자·가로 흐름 | 4번 이미지 |
| 4 | 작은 아이콘 | `작은 아이콘` | 16px | 아이콘 + 이름(1줄) | 다열·**가로 흐름** | 4번 이미지 |
| 5 | 목록 | `목록` | 16px | 아이콘 + 이름(1줄) | 다열·**세로 흐름** | 4번 이미지 |
| 6 | 자세히 | `자세히` | 16px | 이름·크기·종류·수정한 날짜 4열 | 단일 열 행 | 4번 이미지 (현재 구현) |
| 7 | 타일 | `타일` | 48px | 아이콘 + 이름·종류·크기 3줄 | 격자·가로 흐름 | 4번 이미지 |
| 8 | 내용 | `내용` | 32px | 아이콘 + 이름·수정한 날짜·크기 | 전체 폭 행 | 4번 이미지 |

> 4번 이미지의 현재 모드(`자세히`) 왼쪽에 **점 표시**(`•`)가 붙어 있다 — 같은 방식으로 현재 모드를 표시한다.
> 항목 왼쪽 아이콘(모드를 나타내는 기호)은 이번에 넣지 않는다 — phosphor에 대응 글리프가 없어 두부가 될 위험이 있고(`◧` 사례), 점 표시만으로 현재 모드가 드러난다.

### 시각 속성

| 요소 | 속성 | 디자인 값 | 확인 방법 |
|---|---|---|---|
| 격자 항목(아주 큰) | 셀 크기 | 280 × 320px (아이콘 256 + 이름 2줄 + 여백) | 아이콘 256px 기준 산출 |
| 격자 항목(큰) | 셀 크기 | 120 × 150px (아이콘 96) | 아이콘 96px 기준 산출 |
| 격자 항목(보통) | 셀 크기 | 76 × 100px (아이콘 48) | 아이콘 48px 기준 산출 |
| 격자 항목(타일) | 셀 크기 | 220 × 64px (아이콘 48 + 3줄 텍스트) | 3줄 텍스트 폭 기준 산출 |
| 작은 아이콘·목록 | 셀 크기 | 200 × 20px | 현재 행 높이 20px 유지 |
| 내용 보기 | 행 높이 | 48px (아이콘 32 + 상하 여백 8) | 아이콘 32px 기준 산출 |
| 격자 항목 | 간격 | 8px | 사이드바 카드 간격(4px)보다 넓게 — 격자는 가로세로 모두 띄워야 뭉치지 않는다 |
| 격자 이름 | 최대 줄 수 | 2줄 (넘으면 `…`) | Windows 탐색기 관례 |
| 자세히 열 | 최소 폭 | 40px | 헤더 문구("크기")가 잘려도 드래그 핸들은 잡히는 하한 |
| 자세히 열 | 기본 폭 | 이름 320 / 크기 90 / 종류 150 / 수정한 날짜 150 | 기존 상수 유지 + 마지막 열은 종전 "남는 폭 전부"에서 **고정 150px**로 바뀐다(가로 스크롤 도입에 따른 필연 — D2) |
| 열 드래그 핸들 | 폭 | 6px (열 경계 중심 ±3px) | 커서 변경이 잡히는 최소 폭 |
| 항목 수 문구 | 정렬 | 줄의 오른쪽 끝 | 사용자 요청 원문 |
| 항목 수 문구 | 형식 | `폴더 {N} 파일 {M}` | 사용자 요청 원문 (숫자 사이 공백 1칸) |
| 항목 수 문구 | 색 | `theme::TEXT_DIM` | 기존 항목 수 표시와 동일 |

## V-9 대조 결과

구현이 진행되며 task별로 누적한다. 정적 축(상수·문구·컨트롤 타입)은 소스 대조로 ✅/❌를 확정하고,
실제 화면에서만 확인 가능한 시각 축은 `⏳ 미확인`으로 두었다가 F-8에서 사용자 확인으로 닫는다.

| 요소 | 속성 | 디자인 값 | 구현 위치 | 판정 |
|---|---|---|---|---|
| 자세히 열 | 최소 폭 | 40px | `ui/list_details.rs:28` `MIN_COL_WIDTH = 40.0` | ✅ (T2) |
| 자세히 열 | 기본 폭 | 320 / 90 / 150 / 150 | `ui/list_details.rs:52` `DEFAULT = [320.0, 90.0, 150.0, 150.0]` | ✅ (T2) |
| 열 드래그 핸들 | 폭 | 6px | `ui/list_details.rs:30` `HANDLE_WIDTH = 6.0` | ✅ (T2) |
| 항목 수 문구 | 정렬 | 줄의 오른쪽 끝 | `ui/panel.rs:530` `egui::Sides` 오른쪽 클로저 | ✅ (T4) |
| 항목 수 문구 | 형식 | `폴더 {N} 파일 {M}` | `ui/panel.rs:534` `format!("폴더 {dirs} 파일 {files}")` | ✅ (T4) |
| 항목 수 문구 | 색 | `theme::TEXT_DIM` | `ui/panel.rs:534` | ✅ (T4) |
| 패널 메뉴 | 항목 13행(문구·순서·구분선 위치) | 인벤토리 표 그대로 | `ui/menu.rs:68-89` `panel_menu_items` + 테스트 `패널_메뉴는_요청한_순서와_문구를_그린다` | ✅ (T6) |
| 패널 메뉴 `닫기` | 활성 조건 | 패널 2개 이상 | `ui/menu.rs` `PanelMenuState::for_panes` + 테스트 `마지막_패널_하나는_닫을_수_없다` | ✅ (T6) |
| 패널 메뉴 `보기` | 상태 | T8 전까지 비활성 | `ui/menu.rs:70` `add_enabled(false, ..)` | ✅ (T6 — T8에서 하위 메뉴로 교체) |
| 메뉴 버튼 | 아이콘 | 분할 도형 유지 (D10) | `ui/tabs.rs` `draw_split_icon` 유지, 툴팁만 "메뉴" | ✅ (T6) |
| 보기 하위 메뉴 | 8행(문구·순서) | 인벤토리 표 그대로 | `ui/view_mode.rs` `ALL`·`label()` + 테스트 `보기_하위_메뉴는_여덟_모드를_순서대로_그린다` | ✅ (T8) |
| 보기 하위 메뉴 | 현재 모드 표시 | 점(`•`) | `ui/menu.rs` `view_items` + 테스트 `지금_쓰는_모드에만_점이_붙는다` | ✅ (T8) |
| 각 모드 | 아이콘 크기 | 256/96/48/16/16/16/48/32px | `ui/view_mode.rs` `icon_px()` | ✅ (T8) |
| 작은 아이콘 vs 목록 | 배치 흐름 | 가로 흐름 vs 세로 흐름 | `ui/view_mode.rs` `flow()` + 테스트 `작은_아이콘과_목록은_흐름이_다르다` | ✅ (T8) |
| 격자 항목 | 셀 크기 5종 | 280×320 / 120×150 / 76×100 / 220×64 / 200×20 | `ui/view_mode.rs` `cell_size()` | ✅ (T8) |
| 내용 보기 | 행 높이 | 48px | `ui/view_mode.rs` `cell_size()` | ✅ (T8) |
| 격자 항목 | 간격 | 8px | `ui/view_mode.rs` `GRID_SPACING` | ✅ (T8) |
| 격자 이름 | 최대 줄 수 | 2줄 | `ui/view_mode.rs` `GRID_NAME_ROWS` → `ui/list_grid.rs` `draw_cell`이 `elided_galley_rows(.., GRID_NAME_ROWS)`로 적용 | ✅ (T10) |
| 아이콘 4종 | 실제 아이콘 크기 | 256/96/48/16px | `ui/list_grid.rs` `draw_cell`이 `mode.icon_px()`로 그리고, 리스트는 `himl_for(IconSize::for_px(..))`로 고른다 (T9 인계 1-b) | ✅ 코드 경로 (T10) · ⏳ 화면은 F-8 |
| 격자 항목 | 배치(아이콘 위·이름 아래) | 큰 아이콘 3종 | `ui/list_grid.rs` `draw_cell` 세로 분기 | ✅ (T10) |
| 작은 아이콘 | 배치(아이콘 왼쪽·이름 오른쪽) | 한 줄 | `ui/list_grid.rs` `is_single_row` 분기 | ✅ (T10) |
| 타일 | 셀 크기·표시 요소 | 220×64, 이름·종류·크기 3줄 | `ui/view_mode.rs` `cell_size()` + `ui/list_grid.rs` `draw_multiline_cell` | ✅ (T11) |
| 내용 | 행 높이·표시 요소 | 48px, 이름·수정한 날짜·크기 | `ui/view_mode.rs` `cell_size()` + `draw_multiline_cell` Content 분기 | ✅ (T11) |
| 목록 | 배치 | 세로 흐름 다열 | `ui/view_mode.rs` `flow()` → `Flow::Vertical` (렌더는 한 줄 칸 공용) | ✅ (T11) |
| 폴더 | 크기 칸 | 비움(자리는 유지) | `ui/list_grid.rs` `draw_stacked` — 빈 줄은 그리지 않되 자리 전진 | ✅ (T11) |

## Decisions

| # | 결정 | 근거 (Source) |
|---|---|---|
| D1 | 말줄임은 자체 계산 대신 egui `LayoutJob`의 `TextWrapping { max_rows: 1, overflow_character: Some('…') }`를 쓴다 | egui가 픽셀 폭 기준으로 잘라 `…`를 붙인다. `tabs::elide`의 문자 수 방식은 열 폭이 가변인 목록에 맞지 않고, 한글·영문 혼용에서 폭이 어긋난다 |
| D2 | 자세히 보기의 마지막 열(수정한 날짜)도 **고정 폭**을 갖는다 | 지금은 "남는 폭 전부"라 가로 스크롤과 양립할 수 없다(스크롤 콘텐츠 폭이 뷰포트에 의존하면 순환한다). 4열 모두 고정 폭 + 합계가 뷰포트보다 좁으면 마지막 열만 늘여 빈틈을 없앤다 |
| D3 | 열 폭·보기 모드는 `FileListView`가 소유한다 | `grep` 결과 `FileListView`는 `PanelState`만 소유하므로(`panel.rs:118`) 여기 두면 "패널마다 독립"(사용자 확정)이 구조로 보장된다 |
| D4 | 보기 모드 enum은 `ui/view_mode.rs`에 두고 **배치 계산을 순수 함수로 분리**한다 | AGENTS.md "UI(HWND 필요) 로직은 테스트 비대상 — 순수 로직을 UI에서 분리해 테스트". 격자 열 수·셀 위치 계산은 오프바이원이 잦아 테스트 가치가 높다 |
| D5 | 세션은 `SESSION_VERSION`을 2로 유지하고 `#[serde(default)]` 필드만 더한다 | `settings.rs:171`의 `version != SESSION_VERSION → None`은 **전체 폴백**이다. 3으로 올리면 기존 사용자의 워크스페이스·분할·탭이 전부 초기화된다. 필드 추가는 기존 JSON과 하위 호환이므로 버전을 올릴 이유가 없다 |
| D6 | 썸네일은 워커 스레드가 만들고 채널로 받는다 | AGENTS.md "UI 스레드에서 블로킹 I/O 금지". `IShellItemImageFactory::GetImage`는 디스크를 읽으므로 UI 스레드에서 부르면 사진 폴더에서 창이 멈춘다. 기존 `DirLoad`(`panel.rs:35`)의 세대 번호 방식을 같은 규칙으로 따른다 |
| D7 | 썸네일 준비 전에는 형식 아이콘을 그린다 | 빈 자리를 두면 스크롤 중 화면이 깜빡인다. 아이콘은 이미 즉시 얻어지므로 자연스러운 폴백이다 |
| D8 | 96px(큰 아이콘)는 `SHIL_JUMBO`(256px)를 축소해 만든다 | 시스템 이미지 리스트에 96px 단계가 없다. 48px(`SHIL_EXTRALARGE`)를 확대하면 뭉개지므로 큰 쪽에서 줄인다 |
| D9 | 텍스처 캐시 키를 `(himl.0, index)`로 바꾼다 | `icon_tex.rs:18`이 인덱스만 키로 써서, 이미지 리스트가 둘 이상이면 같은 인덱스가 서로를 덮는다. `get`이 이미 himl을 받고 있어 시그니처 변경 없이 해소된다 |
| D10 | 패널 메뉴 버튼의 아이콘·툴팁을 바꾼다 | 지금은 분할 전용(`분할` 툴팁, 분할 도형). 메뉴가 분할 외 항목을 담으므로 툴팁을 `메뉴`로 바꾼다. **도형은 그대로 둔다** — 1번 이미지에서 사용자가 이미 이 버튼을 메뉴 진입점으로 쓰고 있어 모양이 바뀌면 찾지 못한다 |
| D11 | 새로 만든 항목을 선택 상태로 만들지 않는다 | 변경 감시(FR-10)가 폴더를 다시 읽어 목록에 나타난다. 생성 직후 인덱스를 알 수 없고(정렬 후 위치가 달라진다), 이름 편집도 범위 밖이라 선택까지 맞출 실익이 없다 |
| D12 | 새 파일·새 폴더 실패는 상태 줄에 사유만 표시한다 | 기존 열거 실패 처리(`panel.rs:274-285`)와 같은 방식. 쓰기 권한 없는 폴더에서 대화 상자를 띄우면 흐름이 끊긴다 |
| D13 | 260자를 넘는 경로에는 `\\?\` 접두를 붙여 생성하고, 그래도 실패하면 D12대로 사유를 표시한다 | NFR-5가 긴 경로 지원을 요구한다. 접두 없이 실패 처리만 하면 긴 경로 폴더에서 '새 폴더'가 통째로 막혀 NFR-5 위반이 조용히 들어온다. `\\?\`는 UNC가 아닌 절대 경로에만 유효하므로 상대·UNC 경로에는 붙이지 않는다 |
| D14 | 세션 스키마 확장 시 `PanelSession`에 `#[derive(Default)]`를 더하고, **필드 나열 리터럴 6곳 전부**(`ui/session.rs:53`·`app/window.rs:917`·`app/settings.rs`의 테스트 픽스처 4개)를 `..Default::default()`로 보정한다 | 필드 나열 리터럴이 새 필드마다 깨진다. 기존 필드가 모두 `Default`를 가지므로(`Vec<String>`·`usize`) derive가 성립한다. 레거시 판(`window.rs`)과 픽스처는 이번 작업 대상이 아니므로 **컴파일만 통과시키는 최소 수정**에 그친다(그쪽 동작에 보기 모드·열 폭을 배선하지 않는다). 단 `ui/session.rs`의 왕복 테스트에는 새 필드가 실제로 오가는 케이스를 더한다 |
| D17 | 저장된 열 폭의 검증(길이·NaN·최소 폭)은 `parse_session`이 아니라 `Columns::from_saved`가 한다 | T3 Design은 `parse_session`의 검증 자리를 지목했으나, T2가 이미 같은 검증을 `Columns::from_saved`에 두었다. 두 곳에 두면 규칙이 갈리므로 열 폭을 아는 쪽 한 곳에 모은다 (T3 spec 리뷰 M1) |
| D18 | 이름 후보를 만든 뒤 존재를 확인하지 않고 **바로 생성해 보고, `AlreadyExists` 실패를 다음 번호로 넘어가는 신호**로 쓴다. plan Design이 적은 `unique_name(dir, base, ext) -> String`(존재 검사 후 이름만 반환)은 만들지 않았다 | 확인과 생성 사이에 다른 앱이 같은 이름을 만들 수 있고(TOCTOU), 그 틈에 덮어쓰면 남의 파일이 사라진다. `create_dir`·`File::create_new`가 이미 원자적으로 "있으면 실패"를 주므로 이름만 돌려주는 함수를 따로 두면 그 보장을 버리게 된다 (T7 spec 리뷰 M1) |
| D19 | 새 폴더·새 파일 생성도 **워커 스레드**에서 하고 결과를 채널로 받는다 | AGENTS.md가 "UI 스레드 파일시스템 블로킹 호출"을 금지한다. 단일 생성이라 로컬 디스크에서는 순식간이지만 네트워크 드라이브에서는 수 초가 걸릴 수 있고, 이름이 겹치면 재시도가 이어진다. 열거(`DirLoad`)·썸네일(D6)과 같은 규칙을 적용한다 (T7 quality 리뷰 M1) |
| D21 | 썸네일을 놓을지는 **캐시가 스스로 판정한다**(`ThumbnailCache::set_folder`) — 호출부에서 이전 경로와 비교하지 않는다 | 탐색·탭 전환·탭 닫기가 각자 다른 순서로 폴더를 바꾼다. 커밋 뒤에 비교하면 항상 같아져 해제가 통째로 죽고(F-7 B1), 커밋 앞에서 비교해도 탭 경로는 이미 전환된 뒤라 빠져나간다(F-7 m1). 캐시가 "지금 담긴 것이 어느 폴더 것인지"를 들면 어느 경로로 오든 한 번에 맞는다 |
| D20 | 생성에 성공하면 **감시 여부와 무관하게** 폴더를 다시 읽는다 | `DirWatcher`는 폴더 열기에 실패해도 조용히 끝나 그 실패가 밖으로 드러나지 않는다 — 감시 객체가 있다는 것만으로 통지를 믿으면, 감시가 죽은 위치에서 방금 만든 항목이 목록에 나타나지 않는다. 재열거 한 번이 그 침묵보다 싸다 (T7 spec 리뷰 M2) |
| D16 | 패널 메뉴에서 온 명령은 **활성 패널이 아니라 메뉴를 연 패널**(`LayoutOutcome`이 실어 올린 `PanelId`)에 적용한다. `apply_command`에 `target: Option<PanelId>` 인자를 더하고, `None`(단축키·타이틀바에서 온 명령)이면 종전대로 활성 패널을 쓴다 | 활성 패널 판정은 **포인터가 눌린 위치**로만 이뤄진다(`splitter.rs:67-80`). 패널 메뉴 팝업은 자기 pane 밖으로 뻗을 수 있어, 그 위에서 항목을 클릭하면 팝업 아래 깔린 **다른 pane이 활성**이 된다 — 그 상태로 활성 패널에 적용하면 '닫기'·'새 파일'이 엉뚱한 패널에 간다. 기존 분할이 굳이 `PanelId`를 실어 올린 이유(D3)와 같은 위험이며, 명령을 늘리는 이번 작업에서 그 규칙을 그대로 잇는다 |
| D15 | '닫기'의 활성 조건(패널 2개 이상)은 `splitter::show_layout`이 `computed.panes.len() > 1`로 계산해 `panel.show` → `tabs::show_tab_strip`으로 **인자로 내려준다** | 패널은 서로를 모른다는 기존 설계(`panel.rs` 모듈 주석)를 지키려면 패널 수를 밖에서 받아야 한다. `app.rs:593 active_panel_count()`는 활성 워크스페이스 기준이라 그리는 시점의 트리와 어긋날 수 있고, `splitter`는 이미 그 트리를 계산해 갖고 있다 |

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|---|---|---|---|
| FR-4 (열 폭·가로 스크롤·말줄임) | Must | T1, T2, T3 | ✅ 커버 |
| FR-23 (보기 모드 8종) | Should | T8, T10, T11, T12 | ✅ 커버 |
| FR-24 (썸네일) | Should | T9, T13, T14 | ✅ 커버 |
| FR-25 (새 폴더·새 파일) | Should | T7 | ✅ 커버 |
| FR-26 (패널 메뉴·메뉴 바 폐지) | Should | T6 | ✅ 커버 |
| FR-1 (분할 진입점 문구 정정) | Must | T6 | ✅ 커버 |
| FR-22 (사이드바 토글 유일 진입점) | Should | T5 | ✅ 커버 |
| FR-16 (워크스페이스 생성 진입점 정정) | Must | T6 | ✅ 커버 (메뉴 바 제거로 문구 일치) |
| FR-21 (다크 스타일 — 팝업 메뉴) | Should | T6 | ✅ 커버 |
| NFR-9 (썸네일 캐시 상한) | — | T13 | ✅ 커버 |
| FR-2·3·5·6·7·8·9·10·11·12·15·17·18·19·20 | Must/Should | 이번 범위 외 (기구현) | 유지 — 회귀만 확인 |
| FR-13·14 | Could | 이번 범위 외 (후속) | Deferred 대장 등재 상태 유지 |

## Tasks

### [x] T1. 목록 텍스트 말줄임 — 겹침·잘림 해소 (요구 4·9)

- **Type**: C
- **Files**: `src/ui/file_list.rs`
- **내용**: 셀 텍스트를 `painter.layout`(wrap) 대신 `LayoutJob` + `TextWrapping { max_rows: 1, overflow_character: Some('…'), break_anywhere: true }`로 그린다. 네 열 모두 적용(사용자 확정). 코드와 어긋난 주석("셀 폭을 넘는 글자는 잘라 그린다")도 실제 동작에 맞게 고친다.
- **Design**: 배치 — `ui/file_list.rs`에 private 헬퍼 `elided_galley(painter, text, font, color, max_width) -> Arc<Galley>`. 책임 — 폭을 넘으면 1줄로 자르고 `…`를 붙인 갤리 반환. 의존 — egui만 참조하며 아무것도 이 함수를 모른다(파일 안에서만 쓴다). 비추상화 — 열 메타데이터 구조는 T2에서 도입하므로 여기서 미리 만들지 않는다.
- **Edge Cases**: 빈 문자열(그리지 않음) / 폭 ≤ 0(그리지 않음 — 기존 분기 유지) / 한 글자도 안 들어가는 폭(`…`만 남거나 빈 갤리 — 패닉 없이 통과해야 한다) / 이모지·서로게이트 페어가 잘리는 위치(`break_anywhere`가 문자 경계를 지키는지 확인)
- **Halt Forecast**: 없음 (외부 API·파괴적 작업 없음)
- **Acceptance**:
  1. 긴 이름(2번 이미지의 `NTUSER.DAT{71e7eeb8-...}.TM.blf`)이 **한 줄로만** 그려지고 끝에 `…`가 붙는다
  2. 어떤 행도 다음 행 영역을 침범하지 않는다 (행 높이 20px 유지)
  3. `cargo test` 통과 — 폭이 0·음수·아주 작을 때 패닉하지 않음을 덮는 단위 테스트 추가

### [x] T2. 열 폭 드래그 조절 + 가로 스크롤 (요구 2·3)

- **Type**: D
- **Files**: `src/ui/file_list.rs`, `src/ui/list_details.rs`(신규)
- **내용**: 고정 `const` 열 폭을 `FileListView`의 상태(`columns: [f32; 4]`)로 옮기고, 헤더의 열 경계에 6px 드래그 핸들을 둔다. 목록 전체를 `ScrollArea::both()`로 감싸고 콘텐츠 폭을 열 폭 합으로 고정한다(D2). 자세히 보기 렌더를 `list_details.rs`로 분리한다.
- **Design**: 배치 — 자세히 보기의 헤더·행 렌더와 열 조작을 `ui/list_details.rs`로 옮기고, `ui/file_list.rs`는 상태·입력·디스패치만 남긴다. 신규 심볼 — `list_details::show(view_state, ui, ...) -> FileListAction`(자세히 보기 그리기), `list_details::Columns`(4열 폭 배열 + 클램프·드래그 반영), `Columns::apply_drag(index, delta)`(최소 폭 40px 클램프), `Columns::content_width()`. 의존 — `list_details`가 `file_list`의 항목·선택 상태를 **참조로 받고**, 역방향 의존은 없다. 비추상화 — 열을 `Vec<Column>` 동적 목록이 아니라 **고정 4열 배열**로 둔다(열 추가·제거는 Out of Scope이므로 동적 구조는 쓰이지 않는 유연성이다).
- **Edge Cases**: 열을 최소 폭 아래로 끌기(40px에서 멈춤) / 열 폭 합 < 뷰포트(마지막 열을 늘여 빈틈 제거) / 열 폭 합 > 뷰포트(가로 스크롤 등장) / 드래그 중 패널이 분할·닫힘(드래그 상태가 다음 프레임에 사라져도 패닉 없음) / 헤더 클릭(정렬)과 드래그(폭 변경) 구분 — 핸들 위에서는 정렬이 일어나지 않아야 한다 / 가로 스크롤 시 헤더가 본문과 **같은 x**로 움직이는가
- **Halt Forecast**: 헤더와 본문을 각각 `ScrollArea`로 감싸면 가로 스크롤이 어긋난다 → **사전 해소**: 헤더와 본문을 **하나의 `ScrollArea::both()` 안에** 넣고 헤더는 `sticky` 대신 스크롤 영역 안 첫 행으로 그린다(세로 스크롤 시 헤더가 함께 올라가는 것은 T2 범위에서 허용하고, 고정 헤더가 필요하면 Deferred).
- **Acceptance**:
  1. 헤더의 열 경계를 끌면 그 열의 폭이 실시간으로 변하고 본문 행도 같은 폭으로 그려진다
  2. 열 폭을 넓혀 합이 패널보다 커지면 **가로 스크롤 막대가 나타나고** 끝까지 스크롤된다
  3. 열 경계에서 커서가 좌우 화살표로 바뀌고, 그 지점 클릭은 정렬을 바꾸지 않는다
  4. `cargo test` 통과 — `Columns::apply_drag`의 최소 폭 클램프·`content_width` 합계를 덮는 단위 테스트

### [x] T3. 열 폭 세션 저장 (요구 3의 지속성)

- **Type**: C
- **Files**: `src/app/settings.rs`, `src/ui/session.rs`, `src/ui/app.rs`, `src/ui/panel.rs`, `src/ui/file_list.rs`, `src/app/window.rs`
- **내용**: `PanelSession`에 `#[serde(default)] columns: Vec<f32>`를 더하고, 패널마다 독립 저장·복원한다(사용자 확정). `SESSION_VERSION`은 2 유지(D5). `PanelSession`에 `Default`를 derive하고 **필드 나열 리터럴 6곳**(`ui/session.rs:53`·`app/window.rs:917`·`app/settings.rs:253·257·261·271`)을 `..Default::default()`로 보정한다(D14 — 픽스처 4곳은 `cargo build`가 아니라 `cargo test`에서 드러난다).
- **Design**: 배치 — 스키마는 `app/settings.rs`, 변환은 `ui/session.rs`의 `PanelTabs`에 필드 추가. 신규 심볼 — 없음(기존 구조체 확장). 의존 — `session.rs`는 UI 타입을 모른다는 기존 계약을 유지한다. `app/window.rs`는 컴파일 통과만 시키고 이 값을 쓰지 않는다(D14). 비추상화 — 열 폭 검증을 별도 타입으로 감싸지 않고 `parse_session`의 기존 검증 자리에 클램프 한 줄로 둔다.
- **Edge Cases**: 필드가 없는 기존 v2 파일(default로 기본 폭) / 길이가 4가 아닌 배열(무시하고 기본값) / NaN·무한대·음수 폭(기존 `layout_ratios_valid`와 같은 방어 — 최소 폭으로 클램프) / 패널 수와 열 폭 배열 수 불일치
- **Halt Forecast**: 세션 파일 호환이 실제로 깨지는 경우 → **위임 불가 Halt**(사용자 데이터). 단 D5대로면 발생하지 않아야 하며, 발생 시 멈추고 보고한다.
- **Acceptance**:
  1. 열 폭을 바꾸고 앱을 껐다 켜면 그 폭이 유지된다
  2. **필드가 없는 기존 `settings.json`으로 실행해도 워크스페이스·분할·탭이 그대로 복원된다** (초기화되지 않는다)
  3. `cargo test` 통과 — 왕복 테스트에 열 폭 포함, 그리고 열 폭 필드가 빠진 JSON이 파싱되는 테스트 추가

### [x] T4. 항목 수 표시 — `폴더 N 파일 M` 오른쪽 정렬 (요구 7)

- **Type**: C
- **Files**: `src/ui/panel.rs`, `src/ui/file_list.rs`
- **내용**: `FileListView`에 폴더 수·파일 수 집계를 노출하고, `show_content`의 줄에서 항목 수만 오른쪽 끝으로 옮긴다. '폴더 트리' 토글·로딩 표시·오류 문구는 왼쪽에 유지(사용자 확정).
- **Design**: 배치 — 집계는 `FileListView::counts() -> (usize, usize)`, 배치는 `panel.rs`에서 `egui::Sides`(탭 스트립이 이미 쓰는 방식, `tabs.rs:86`)로 좌우를 가른다. 비추상화 — 상태 줄 전용 위젯 타입을 만들지 않는다(한 줄짜리 배치다).
- **Edge Cases**: 빈 폴더(`폴더 0 파일 0`) / 로딩 중(왼쪽 스피너와 함께 이전 수가 남아 어색하지 않은지 — 로딩 중에는 항목 수를 그리지 않는다) / 열거 실패(오류 문구가 길어 항목 수를 밀어내지 않는지 — 오른쪽 항목이 우선 자리를 잡는다) / 10만 항목(집계가 프레임마다 전체 순회하지 않도록 `set_entries` 시점에 한 번 계산해 보관)
- **Halt Forecast**: 없음 — 기존 위젯 배치만 바꾸고 외부 API·파괴적 작업이 없다
- **Acceptance**:
  1. 목록 위 줄의 **오른쪽 끝**에 `폴더 20 파일 10` 형식으로 표시된다
  2. 왼쪽에는 '폴더 트리' 토글이 그대로 있다
  3. `cargo test` 통과 — 집계 함수의 폴더/파일 구분 단위 테스트

### [x] T5. 사이드바 상단 `?` 제거 (요구 5)

- **Type**: B
- **Files**: `src/ui/sidebar.rs`, `src/ui/app.rs`
- **내용**: `show_toggle_strip`과 관련 상수(`TOGGLE_STRIP_HEIGHT`·`TOGGLE_SIZE`·`TOGGLE_MARGIN`·`TOGGLE_GLYPH`)를 지운다. `SidebarAction::ToggleCollapse`는 생성처가 이곳 하나뿐이므로 변형째 제거하고, **처리부인 `app.rs:440`의 match arm도 함께 지운다**(Investigation Log — 이 crate는 bin+lib이라 `pub` 변형에 dead_code 경고가 나지 않아 빌드만으로는 잔여가 드러나지 않는다).
- **Design**: 신규 심볼 없음. 의존 — `SidebarAction`에서 변형이 하나 줄어들 뿐 다른 변형의 계약은 그대로다. 비추상화 — 사이드바 상단 여백을 새 상수로 만들지 않는다(헤더가 자기 높이 36px를 이미 갖는다).
- **Edge Cases**: `app.rs:440` arm을 지우지 않으면 E0599로 빌드가 깨진다(반대로 남기면 존재하지 않는 변형 참조) / 스트립을 지운 만큼 헤더가 위로 올라와 타이틀바와 붙지 않는지(붙으면 여백 유지) / 사이드바를 접은 뒤 타이틀바 토글·`Ctrl+B`로 다시 펼 수 있는가
- **Halt Forecast**: 없음 — 삭제 대상의 생성·처리부를 전수 확인했고 외부 API·파괴적 작업이 없다
- **Acceptance**:
  1. 워크스페이스 패널 상단에 `?`가 보이지 않는다
  2. 타이틀바 좌측 토글과 `Ctrl+B`로 사이드바를 접고 펼 수 있다
  3. `cargo build` 경고 0 (미사용 상수·변형 잔여 없음)

### [x] T6. 패널 메뉴 신설 + 상단 메뉴 바 제거 (요구 1·6)

- **Type**: D
- **Files**: `src/ui/menu.rs`, `src/ui/tabs.rs`, `src/ui/panel.rs`, `src/ui/splitter.rs`, `src/ui/app.rs`
- **내용**: `split_items`를 `panel_menu_items`로 확장해 인벤토리 표 13행을 그대로 만든다(보기 하위 메뉴는 T8에서 채우므로 여기서는 **비활성 항목 `보기` 한 줄로만** 자리를 잡는다 — 하위 메뉴 내용이 없어도 컴파일·표시가 성립한다). `show_menu_bar`·`MenuState`를 삭제하고 `app.rs`의 호출·구분선을 제거한다. `Command`에 `NewFile`·`NewFolder`를 추가한다 — **T7 전까지 `apply_command`에서 no-op arm으로 두어** exhaustive match를 만족시키고 메뉴 항목은 표시·클릭까지만 된다. 패널 메뉴가 반환한 명령을 `PanelOutcome` → `LayoutOutcome` → `app.rs` 경로로 올린다.
- **Design**: 배치 — 메뉴 구성은 `ui/menu.rs`에 유지(두 진입점이 갈리지 않게 한 곳에 둔다는 기존 주석의 원칙 계승). 신규 심볼 — `menu::panel_menu_items(ui, state, out)`(패널 메뉴 전체), `menu::PanelMenuState { can_close_panel }`(**`view_mode` 필드는 T8에서 더한다** — 그 타입이 T8에 생기므로 여기서 참조하면 컴파일되지 않는다). 의존 — 명령의 상향 경로는 기존 `split`과 **정확히 같은 계단**을 쓴다: `tabs::TabStripOutcome`에 `command: Option<Command>`를 더하고 → `panel::PanelOutcome`이 그대로 실어 올리고 → `splitter::LayoutOutcome`에 `command: Option<(PanelId, Command)>`로 패널 id를 붙여 합산하고(`splitter.rs:88-103`의 "한 프레임에 하나만" 규칙 동일) → `app.rs`가 `apply_command(command, target, area, ctx)`에 **그 `PanelId`와 함께** 넘긴다(D16 — 활성 패널이 아니라 메뉴를 연 패널에 적용). 기존 단축키·타이틀바 경로는 `target: None`으로 부르면 종전 동작 그대로다. 활성 조건 하향은 D15 — `splitter::show_layout`이 `computed.panes.len() > 1`을 `panel.show`에 인자로 내려주고 `panel.show`가 `show_tab_strip`에 전달한다. 비추상화 — 메뉴 항목을 데이터 테이블로 추상화하지 않고 함수 본문에 순서대로 나열한다(구분선 위치·활성 조건이 항목마다 달라 표로 만들면 오히려 읽기 어렵다).
- **Edge Cases**: 마지막 패널에서 '닫기'(비활성 표시) / 패널이 좁아 메뉴가 화면 밖으로 나갈 때(egui `Popup`이 자동 조정하는지 확인) / 메뉴 바 제거로 `area` 계산이 달라져 최소 패널 크기 판정이 바뀌는지(`app.rs:713` 주석이 지적한 지점) / `Command` 항목 추가로 `apply_command`의 match가 비포괄이 되는지 / 단축키(F5·Ctrl+Shift+W)가 메뉴 바 없이도 동작하는가 / 한 프레임에 셸 메뉴 요청과 패널 메뉴 명령이 함께 나올 때 `LayoutOutcome`의 두 필드가 서로를 덮지 않는지
- **Halt Forecast**: `MenuState`·`show_menu_bar` 삭제는 **공개 API 변경**이지만 호출부가 `app.rs` 1곳뿐임을 확인했다(Investigation Log) → **사전 승인 항목 5**로 위임됨.
- **잔존 variant 처리**: 메뉴 바가 사라지면 `Command::NewWorkspace`·`RenameWorkspace`·`RemoveWorkspace`는 생성처가 없어진다(기능 자체는 사이드바 `+`·컨텍스트 메뉴의 `SidebarAction` 경로로 유지된다). `pub` enum이라 dead_code 경고가 나지 않으므로 **세 variant와 `apply_command`의 해당 arm을 함께 제거한다** — 남겨두면 "메뉴에서 부를 수 있다"는 잘못된 인상을 준다.
- **Acceptance**:
  1. 탭 스트립의 버튼을 누르면 인벤토리 표 13행이 **그 순서·문구·구분선 위치 그대로** 나온다 (단 `보기`는 이 시점 비활성 — 하위 메뉴는 T8에서 채운다)
  2. 상단 메뉴 바(보기·이동·탭·워크스페이스)와 그 아래 구분선이 화면에서 사라진다
  3. '새로 고침'·'닫기'가 실제로 동작하고, '닫기'는 패널이 1개일 때 비활성이다
  4. F5·Ctrl+Shift+W·Ctrl+T·Alt+← 등 기존 단축키가 그대로 동작한다
  5. 분할 4종이 **이전과 동일하게** 동작한다 (`LayoutOutcome`에 필드를 더하면서 기존 `split` 경로가 깨지지 않았다)
  6. 패널을 둘로 나눈 뒤 **비활성 패널의 메뉴로 '닫기'를 누르면 그 패널이 닫힌다** (활성 패널이 아니다 — D16)
  7. `cargo test` 통과 — 메뉴 항목→명령 매핑·활성 조건·명령 대상 라우팅(D16) 단위 테스트

### [x] T7. 새 폴더·새 파일 만들기 (요구 1)

- **Type**: C
- **Files**: `src/fs/create.rs`(신규), `src/fs/mod.rs`, `src/ui/panel.rs`, `src/ui/app.rs`
- **내용**: 표시 중인 폴더에 `새 폴더` / `새 텍스트 문서.txt`를 만든다(사용자 확정). 중복이면 `(2)`·`(3)`… 번호를 붙인다. 실패 시 상태 줄에 사유만 표시한다(D12). 변경 감시(FR-10)가 목록을 갱신하므로 별도 새로고침은 하지 않는다.
- **Design**: 배치 — 파일시스템 조작은 `fs/create.rs`(UI를 모른다). 신규 심볼 — `create::new_folder(dir) -> std::io::Result<PathBuf>`, `create::new_text_file(dir) -> std::io::Result<PathBuf>`, `create::unique_name(dir, base, ext) -> String`(존재 검사 + 번호 부여). 의존 — `panel.rs`가 호출하고 `fs`는 아무도 모른다. 비추상화 — "새로 만들기 항목 종류" 트레이트를 만들지 않는다(종류가 2개로 고정이다).
- **Edge Cases**: 쓰기 권한 없는 폴더(상태 줄에 사유) / 같은 이름이 999개(시도 횟수 상한 1000을 두고 초과 시 실패로 처리) / 경로 길이 260자 초과(**D13대로 `\\?\` 접두 후 생성**, 그래도 실패하면 사유 표시) / 이름 생성과 실제 생성 사이에 다른 앱이 같은 이름을 만듦(생성 실패를 그대로 사유로 표시 — 덮어쓰지 않는다) / 드라이브 루트처럼 감시가 실패하는 위치(목록이 자동 갱신되지 않으면 생성 후 명시적 재열거)
- **Halt Forecast**: 파일 **생성**은 파괴적 작업이 아니다(기존 파일을 덮어쓰지 않고 중복 시 번호를 붙인다) → 위임 대상. 단 `unique_name`이 기존 파일을 덮어쓰는 경로가 생기면 즉시 멈춘다.
- **Acceptance**:
  1. 패널 메뉴 → '새 폴더'로 현재 폴더에 `새 폴더`가 만들어지고 목록에 나타난다
  2. 같은 폴더에서 다시 누르면 `새 폴더 (2)`가 만들어진다 (기존 폴더를 덮어쓰지 않는다)
  3. '새 파일'로 `새 텍스트 문서.txt`(0바이트)가 만들어진다
  4. 권한 없는 폴더에서는 상태 줄에 사유가 표시되고 앱이 죽지 않는다
  5. `cargo test` 통과 — `unique_name`의 번호 부여·상한을 덮는 단위 테스트(임시 폴더 사용)

### [x] T8. 보기 모드 모델 + '보기' 하위 메뉴 (요구 8)

- **Type**: D
- **Files**: `src/ui/view_mode.rs`(신규), `src/ui/mod.rs`(모듈 등록), `src/ui/menu.rs`, `src/ui/tabs.rs`, `src/ui/file_list.rs`, `src/ui/panel.rs`, `src/ui/splitter.rs`(`for_panes` 호출부), `src/ui/app.rs`
- **내용**: 8개 모드 enum과 모드별 배치 계산(아이콘 크기·셀 크기·흐름 방향·열 수)을 만들고, T6이 비활성 한 줄로 잡아둔 '보기'를 인벤토리 표 8행을 담은 하위 메뉴로 바꾼다. 현재 모드에 점 표시를 붙인다. 아직 렌더는 자세히 보기만 동작하고 나머지는 T10·T11에서 채운다.
- **Design**: 배치 — `ui/view_mode.rs`(순수 로직, D4). 신규 심볼 — `ViewMode`(8개 variant + `label()`·`icon_px()`·`cell_size()`·`flow()`), `Flow { Horizontal, Vertical, Rows }`, `grid_metrics(mode, viewport_width, item_count) -> GridMetrics { columns, rows, cell, spacing }`, `item_rect(metrics, index) -> Rect`. 의존 — egui의 `Vec2`·`Rect`만 참조하고 앱 상태를 모른다. `menu.rs`·`file_list.rs`가 이것을 참조하며 역방향은 없다. T6이 만든 경로를 그대로 잇는다 — `menu::PanelMenuState`에 `view_mode: ViewMode`를 **이 시점에 추가**하고(M1), 선택 결과는 `Command::SetViewMode(ViewMode)`로 `TabStripOutcome` → `PanelOutcome` → `LayoutOutcome` 경로를 탄다. 비추상화 — 모드별 렌더러를 트레이트로 만들지 않고 `match`로 분기한다(8개 고정이고, 트레이트로 감싸면 어느 모드가 어떻게 그려지는지 추적에 파일을 오가야 한다).
- **Edge Cases**: 뷰포트 폭 < 셀 1개 폭(열 수 최소 1 보장 — 0으로 나누기 금지) / 항목 0개(행 수 0) / 아주 큰 아이콘 + 좁은 패널(가로 스크롤 대신 1열로 접기) / 10만 항목의 행 수 계산(정수 오버플로 없이) / 모드 전환 시 스크롤 위치 — **모드별 `id_salt`로 스크롤 상태를 분리하는 방식으로 대체**했다(F-7 M2). "맨 위로 되돌린다"보다 낫다: 같은 문제(셀 크기가 달라 같은 오프셋이 엉뚱한 위치를 가리킴)를 해소하면서, 모드를 오갈 때 각자 보던 위치가 유지된다
- **Halt Forecast**: 없음 (순수 로직 + 메뉴 항목)
- **Acceptance**:
  1. 패널 메뉴에서 '보기'에 **마우스를 올리면** 하위 메뉴가 펼쳐진다 (클릭 없이)
  2. 하위 메뉴에 인벤토리 표 8행이 그 순서·문구 그대로 나오고, 현재 모드에 점 표시가 붙는다
  3. 모드를 고르면 `FileListView`의 모드 상태가 바뀐다 — **단위 테스트로 확인한다**(선택 명령 → 상태 반영). 화면 렌더는 T10·T11에서 완성되므로 이 시점의 화면 확인은 "자세히 보기가 이전과 동일"까지만이다
  4. `cargo test` 통과 — `grid_metrics`의 열 수 계산(폭 경계·최소 1열)과 `item_rect`의 위치를 덮는 단위 테스트

### [x] T9. 큰 아이콘 이미지 리스트 + 텍스처 캐시 키 교정 (요구 8)

- **Type**: D
- **Files**: `src/fs/icons.rs`, `src/ui/icon_tex.rs`, `src/ui/view_mode.rs`(모드↔크기 매핑 테스트 — `fs`는 `ui`를 모르므로 상위에 둔다), `Cargo.toml`(feature 부족이 드러날 때만 — 사전 승인 항목 2)
- **범위 정정 (T9 spec 리뷰 B1)**: 애초 Files에 있던 `ui/file_list.rs`·`ui/sidebar.rs`는 손대지 않는다 — 큰 아이콘을 실제로 **쓰는** 화면은 격자 보기뿐이고 그것은 T10이다. 자세히·목록·사이드바는 16px 고정이라 연결할 곳이 없다
- **내용**: `SHGetImageList`로 32/48/256px 이미지 리스트를 얻어 `IconCache`가 크기별로 보관하고, 크기를 받아 적절한 리스트를 돌려주는 접근자를 더한다. `IconTextures`의 캐시 키를 `(himl.0, index)`로 바꾼다(D9).
- **Design**: 배치 — 셸 호출은 `fs/icons.rs`, 텍스처 변환은 `ui/icon_tex.rs`(기존 책임 유지). 신규 심볼 — `icons::IconSize { Small, Large, ExtraLarge, Jumbo }`, `IconCache::himl_for(size) -> HIMAGELIST`. 의존 — 기존 `himl()`은 `Small`을 돌려주는 형태로 유지해 `sidebar.rs`·`panel/*`의 호출부를 건드리지 않는다. 비추상화 — 이미지 리스트를 lazy하게 얻지 않고 `IconCache::new`에서 한 번에 얻는다(4회 COM 호출이며 시작 시간 영향이 미미하다. 실측이 NFR-1을 위협하면 그때 지연 획득으로 바꾼다).
- **Edge Cases**: `SHGetImageList` 실패(해당 크기를 못 얻으면 Small로 폴백 — 앱이 죽지 않는다) / 인덱스가 리스트마다 같은지(같은 시스템 인덱스 체계이나 **가정하지 않고** 각 리스트에서 개별 조회) / 256px 아이콘의 텍스처 메모리(256KB/장 — 프레임당 생성 상한 8이 그대로 적용되는지) / 기존 16px 아이콘이 큰 리스트로 잘못 조회되는 회귀
- **Halt Forecast**: `SHGetImageList`가 `IImageList` 인터페이스를 요구하는데 windows crate feature가 모자라 컴파일되지 않을 수 있다 → **사전 해소**: `Win32_UI_Shell`에 이미 포함됨을 확인했고, 부족하면 **사전 승인 항목 2**(feature 추가)로 처리한다.
- **Acceptance**:
  1. 48px·256px 이미지 리스트를 **실제로 얻는다** — 네 단계가 서로 다른 핸들이고 16px 폴백이 아니다(단위 테스트로 확인). **화면에 큰 그림으로 보이는지는 T10에서 확인한다** — 큰 아이콘을 쓰는 화면이 격자 보기뿐이라 T9 단독으로는 검증할 대상이 없다(원래 acceptance 문구가 T9/T10 경계와 어긋나 정정, T9 spec 리뷰 B1)
  2. 작은 아이콘(자세히 보기)이 이전과 **동일하게** 표시된다 (캐시 키 변경 회귀 없음)
  3. 사이드바 폴더 아이콘도 이전과 동일하다
  4. `cargo build` 경고 0 · `cargo test` 통과

### [x] T10. 아이콘 격자 보기 4종 렌더 (요구 8)

- **Type**: D
- **Files**: `src/ui/list_grid.rs`(신규), `src/ui/list_common.rs`(신규 — 두 렌더 모듈의 공용 조각), `src/ui/mod.rs`, `src/ui/file_list.rs`, `src/ui/list_details.rs`(공용 모듈 참조로 전환)
- **내용**: 아주 큰·큰·보통·작은 아이콘 모드를 그린다. 가상 스크롤(보이는 셀만)·선택·더블클릭 열기·우클릭 메뉴를 자세히 보기와 **같은 규칙**으로 처리한다.
- **Design**: 배치 — `ui/list_grid.rs`. 신규 심볼 — `list_grid::show(ui, mode, entries, selection, icons, textures) -> FileListAction`. 의존 — `view_mode`의 배치 계산을 쓰고, 선택 상태는 `file_list`가 소유한 것을 참조로 받는다. `list_grid`는 `file_list`를 모른다(단방향). 비추상화 — 셀 안 그리기(아이콘+이름)를 `list_details`와 공유하는 추상 셀 렌더러로 묶지 않는다(자세히는 열 4개, 격자는 아이콘 위·이름 아래라 공통부가 galley 생성뿐이다).
- **Edge Cases**: 이름 2줄 초과(`…`) / 셀 폭보다 긴 한 단어(`break_anywhere`) / 선택 사각형이 셀 경계를 넘는지 / 빈 영역 우클릭(폴더 배경 메뉴 — 자세히 보기와 같은 동작) / Shift 범위 선택이 격자에서도 인덱스 순서를 따르는가 / 아주 큰 아이콘에서 한 화면에 셀이 2개뿐일 때 가상 스크롤 경계 / 스크롤 중 텍스처 생성 상한(8/프레임)에 걸려 아이콘이 늦게 뜨는 것이 자연스러운가
- **Halt Forecast**: 없음 — T8·T9가 만든 배치 계산과 이미지 리스트를 쓰는 렌더 작업이라 새 외부 의존이 없다
- **인계 확인 의무**: acceptance 1-b는 T9에서 넘어온 항목이다. 이 task에서 실제로 검증되지 않으면 T9의 정정이 사후에 "검증 삭제"가 된다 (T9 spec 리뷰 후속 권고)
- **Acceptance**:
  1. 네 모드가 각각 인벤토리 표의 아이콘 크기·시각 속성 표의 셀 크기로 그려진다
  1-b. **48px·256px 아이콘이 흐리지 않게 표시된다** (16px을 늘린 그림이 아니다 — T9에서 인계받은 확인 항목)
  2. 클릭 선택·Ctrl/Shift 선택·더블클릭 열기·우클릭 셸 메뉴가 자세히 보기와 동일하게 동작한다
  3. 항목 1만 개 폴더에서 스크롤이 멈추지 않는다 (보이는 셀만 그린다)
  4. `cargo test` 통과 (렌더는 HUMAN-VERIFY, 배치 계산은 T8 테스트가 덮는다)

### [x] T11. 목록·타일·내용 보기 렌더 (요구 8)

- **Type**: D
- **Files**: `src/ui/list_grid.rs`, `src/ui/file_list.rs`, `src/ui/view_mode.rs`(한 줄 모드가 폭 전체를 쓰도록 배치 보완)
- **내용**: 나머지 3종을 그린다. 목록은 **세로 흐름 다열**(작은 아이콘의 가로 흐름과 대비), 타일은 48px + 이름·종류·크기 3줄, 내용은 전체 폭 행 + 32px 아이콘 + 이름·수정한 날짜·크기.
- **Design**: 배치 — T10과 같은 파일. 신규 심볼 — `list_grid::show_tile_cell`·`show_content_row`(private). 의존 — T10과 동일. 비추상화 — 세로 흐름을 위한 별도 스크롤 컨테이너를 만들지 않고 `view_mode::Flow`로 인덱스↔좌표 변환만 바꾼다.
- **Edge Cases**: 세로 흐름에서 마지막 열이 덜 찬 경우 / 세로 흐름 + 가로 스크롤(목록 모드는 열이 오른쪽으로 늘어나므로 가로 스크롤이 필요하다) / 타일의 3줄 중 크기가 빈 폴더(줄을 비우고 배치가 흔들리지 않는지) / 내용 보기에서 패널이 아주 좁을 때 세 정보가 겹치지 않는지
- **Halt Forecast**: 없음 — T10과 같은 파일·같은 API를 쓰는 렌더 추가다
- **Acceptance**:
  1. 목록 모드는 위→아래로 채운 뒤 오른쪽 열로 넘어가고, 작은 아이콘 모드는 왼→오른쪽으로 채운다 (둘이 서로 다르다)
  2. 타일은 이름·종류·크기 3줄을, 내용은 이름·수정한 날짜·크기를 보인다
  3. 폴더는 크기 칸이 비어 있다 (자세히 보기와 같은 규칙)
  4. `cargo test` 통과

### [x] T12. 보기 모드 세션 저장 (요구 8의 지속성)

- **Type**: C
- **Files**: `src/app/settings.rs`, `src/ui/session.rs`, `src/ui/app.rs`, `src/ui/panel.rs`, `src/ui/view_mode.rs`, `src/ui/file_list.rs`
- **내용**: `PanelSession`에 `#[serde(default)] view_mode: String`을 더한다. T3과 같은 스키마 확장 규칙(D5)을 따른다.
- **Design**: 배치 — 스키마는 T3과 동일하고, 키 변환은 `ui/view_mode.rs`(T8이 만든 파일)에 둔다. 신규 심볼 — `ViewMode::as_key()`/`from_key()`(문자열 왕복), `FileListView::view_mode()`/`set_view_mode()`(저장·복원이 읽고 쓸 접근자 — T8이 상태만 두고 접근자를 내지 않았으면 여기서 낸다). 비추상화 — enum 직렬화에 serde derive를 쓰지 않고 명시적 키 함수를 둔다(variant 이름을 바꿔도 저장 파일이 깨지지 않게 한다).
- **Edge Cases**: 알 수 없는 키(자세히로 폴백) / 필드 없음(자세히로 폴백) / T3의 `columns`와 같은 구조체에 함께 추가되며 서로 간섭하지 않는지
- **Halt Forecast**: T3과 동일 (세션 호환 실패 시 위임 불가 Halt)
- **Acceptance**:
  1. 패널마다 다른 보기 모드를 고르고 앱을 껐다 켜면 각 패널의 모드가 유지된다
  2. 필드가 없는 기존 세션 파일도 정상 복원되고 자세히 보기로 시작한다
  3. `cargo test` 통과 — 모드 키 왕복·미지 키 폴백 테스트

### [x] T13. 썸네일 워커 + LRU 캐시 (요구 8)

- **Type**: D
- **Files**: `src/fs/thumbnail.rs`(신규), `src/fs/mod.rs`, `Cargo.toml`
- **내용**: `IShellItemImageFactory::GetImage`로 썸네일을 만드는 워커 스레드와 패널당 200장 LRU 캐시를 만든다(NFR-9, 사용자 확정). 요청은 채널로 보내고 결과도 채널로 받는다(D6). 폴더를 떠나면 그 폴더 썸네일을 해제한다.
- **Design**: 배치 — `fs/thumbnail.rs`(셸 호출 + 캐시). 신규 심볼 (실제 시그니처 — 구현 중 정정, T13 spec 리뷰 M1) — `ThumbnailCache { request(path), poll() -> Vec<PathBuf>, get(path) -> Option<&ThumbnailImage>, clear() }` + `ThumbnailImage { width, height, rgba }`.
  - `size` 인자를 두지 않는다: 한 장을 가장 큰 모드(256px)에 맞춰 만들고 작은 모드는 줄여 쓴다(작게 만들면 큰 모드에서 뭉개진다).
  - `poll`이 이미지가 아니라 **경로만** 돌려준다: 이미지는 캐시가 들고 있으므로 두 벌로 갈라지지 않게 한다. 호출부는 받은 경로로 `get`해 텍스처를 올린다.
  - `get`이 `TextureHandle`이 아니라 `ThumbnailImage`를 돌려준다: `TextureHandle`은 egui(UI) 타입이라 `fs` 계층에 둘 수 없다(원래 Design 문장 안에서 "fs는 UI를 모른다"와 모순이었다). 텍스처 업로드는 T14의 UI 몫이다.
  - `clear_folder(dir)` 대신 `clear()`: 캐시가 패널당 하나이고 패널은 한 번에 한 폴더만 보므로 폴더 인자가 필요 없다. 의존 — `ui/file_list.rs`·`list_grid.rs`가 쓰고, `fs`는 UI를 모른다(텍스처 업로드는 UI 쪽에서 `ColorImage`를 받아 수행). 비추상화 — 썸네일 제공자를 트레이트로 추상화하지 않는다(셸 하나뿐이다).
- **Edge Cases**: 썸네일 없는 파일(형식 아이콘 폴백, 실패를 기억해 재요청하지 않는다) / 큰 폴더에서 요청 폭주(보이는 항목만 요청하고 큐 상한을 둔다) / 폴더를 빠르게 오갈 때 늦게 도착한 결과(세대 번호로 폐기 — `DirLoad` 방식) / 워커 스레드의 COM 초기화(스레드마다 `CoInitializeEx` 필요) / 200장 경계에서의 축출 순서 / 앱 종료 시 워커 정지 / 네트워크 드라이브의 느린 응답(타임아웃 없이 워커만 붙잡히고 UI는 계속 도는지)
- **Halt Forecast**: 실측 메모리가 NFR-9 상한을 크게 벗어나면 → **위임 불가 Halt**(NFR 변경은 PRD 변경). 그 외 feature 추가는 사전 승인 항목 2.
- **Acceptance**:
  1. 사진 폴더를 아주 큰 아이콘으로 열면 **실제 사진 미리보기**가 뜬다
  2. 준비 전에는 형식 아이콘이 보이고, 준비되면 교체된다 (빈 자리·깜빡임 없음)
  3. 사진 1000장 폴더를 끝까지 스크롤해도 UI가 멈추지 않는다
  4. 캐시 상한이 자료구조로 강제된다 — 항목 200장(`MAX_CACHED`), 한 장은 256×256 RGBA(`THUMB_PX`)라 **이론 상한 약 50MB**(200 × 256KB)이며 단위 테스트가 축출 경계를 덮는다. **실제 Working Set 측정은 T14로 인계한다** — 썸네일을 화면에 띄우려면 UI 배선이 끝나야 하고, 그 전에는 스크롤할 대상이 없다 (T9 acceptance 1과 같은 성격의 정정, T13 spec 리뷰 B1)
  5. `cargo test` 통과 — LRU 축출 경계·세대 폐기 단위 테스트

### [x] T14. 썸네일을 보기 모드에 연결 (요구 8)

- **Type**: C
- **Files**: `src/ui/list_grid.rs`, `src/ui/file_list.rs`, `src/ui/panel.rs`, `src/ui/icon_tex.rs`(썸네일 텍스처 캐시), `src/ui/view_mode.rs`(`uses_thumbnails` 판정), `src/fs/thumbnail.rs`(동기화용 조회 추가)
- **내용**: 아이콘 4종·타일·내용 보기가 썸네일을 요청·표시하도록 잇는다(사용자 확정 범위). 자세히·목록 보기는 16px 형식 아이콘을 유지한다.
- **Design**: 배치 — 픽셀은 `fs::thumbnail`, **텍스처는 `ui::icon_tex`**(아이콘 텍스처와 같은 자리). 신규 심볼 — `ThumbnailTextures { sync, get, clear, len }`(계획 시점에 "없음"이라 적었으나 실제로 필요했다 — T13 Design이 "텍스처 업로드는 UI 몫"이라 예고한 부분이 여기서 구체화됐다. T14 spec 리뷰 M1). 의존 — `panel.rs`가 `ThumbnailCache`를 소유하고(패널당 200장이므로 패널 소유가 맞다) `show`에 참조로 넘긴다. 비추상화 — 아이콘과 썸네일을 하나의 "그림 제공자"로 통합하지 않는다(폴백 순서가 명시적으로 보이는 편이 낫다).
- **Edge Cases**: 보기 모드를 자세히로 바꿨을 때 썸네일 요청이 멈추는지 / 썸네일 대상 모드로 되돌아왔을 때 캐시가 살아 있는지 / 폴더 이동 시 이전 폴더 썸네일 해제(NFR-9) / 패널을 닫으면 그 캐시가 함께 해제되는지 / 같은 폴더를 두 패널에서 열었을 때 각자 캐시(중복 메모리)를 갖는 것이 상한 안인지
- **Halt Forecast**: 없음 — T13이 만든 캐시를 기존 렌더에 잇는 배선 작업이며 새 셸 호출·스키마 변경이 없다
- **Acceptance**:
  1. 아이콘 4종·타일·내용에서 썸네일이 보이고, 자세히·목록에서는 16px 형식 아이콘이 보인다
  1-b. **썸네일 캐시가 NFR-9 상한(패널당 약 50MB) 안에 머문다** — T13에서 인계받은 확인 항목.
    - **실측 결과**: 장당 **262,144 bytes**(256×256×4 — 테스트 `실제_썸네일의_장당_크기를_잰다`가 실제 셸 썸네일을 만들어 측정) × 상한 200장 = **50.0MB**. 가득 찬 캐시의 실제 바이트를 재는 테스트(`캐시가_가득_차도_상한_안에_머문다`)가 이 값을 회귀로 고정한다.
    - 측정 방식을 바꾼 이유: 원래 문구는 "사진 폴더를 끝까지 스크롤한 뒤 Working Set"이었으나, Working Set에는 할당자·GPU 드라이버·egui 텍스처 아틀라스가 함께 잡혀 **썸네일 몫만 분리되지 않는다**. 캐시가 실제로 쥔 픽셀 바이트를 재면 상한 준수가 정확히 드러나고 회귀도 자동으로 잡힌다.
    - 남는 부분(GPU 텍스처 메모리·실사용 중 총 Working Set)은 화면 확인 항목으로 F-8에 넘긴다
  2. 폴더를 떠나면 그 폴더 썸네일이 캐시에서 빠진다 — **캐시 엔트리 수를 확인하는 단위 테스트**로 검증한다(Working Set은 할당자가 OS에 즉시 반환하지 않아 관측이 비결정적이다). 전체 메모리 확인은 T13 acceptance 4가 담당한다
  3. 패널 2개에서 각각 다른 보기 모드를 써도 서로 간섭하지 않는다
  4. `cargo build` 경고 0 · `cargo test` 통과

## 검증 방법

- 각 task: `cargo build` (경고 0) → `cargo test` → `cargo clippy --all-targets -- -D warnings` → `cargo fmt --check`
- 전체 완료 후: `cargo run`으로 화면 확인 (아래 HUMAN-VERIFY 목록)
- 메모리(NFR-9): 사진 폴더를 아주 큰 아이콘으로 끝까지 스크롤한 뒤 Working Set 측정

### HUMAN-VERIFY 목록 (화면 확인)

DPI 인식 스크립트로 창을 물리 좌표로 조작·캡처해 **전 항목을 실제 화면에서 확인했다**(2026-07-30).
DPI 비인식 상태에서는 창이 1100×700으로 잘려 보여 열·항목 수·메뉴 버튼이 잘린 것처럼
오판된다 — `SetThreadDpiAwarenessContext(-4)` 없이 본 화면은 근거로 쓰지 않는다.

| # | 항목 | 결과 |
|---|---|---|
| 1 | 긴 파일 이름이 겹치지 않고 `…`로 줄어드는가 (요구 4·9) | ✅ |
| 2 | 열 경계를 끌어 폭이 바뀌고, 넓히면 가로 스크롤이 나오는가 (요구 2·3) | ✅ 320→546.7 반영, 가로 스크롤 드래그 동작, 세션 복원까지 확인 |
| 3 | 사이드바 상단 `?`가 사라졌는가 (요구 5) | ✅ |
| 4 | 상단 메뉴 바가 사라졌는가 (요구 6) | ✅ |
| 5 | 항목 수가 `폴더 N 파일 M`으로 오른쪽에 있는가 (요구 7) | ✅ |
| 6 | 패널 메뉴가 요청한 순서·구분선대로 나오는가 (요구 1) | ✅ 13행 그대로, `닫기`는 패널 1개일 때 비활성 |
| 7 | '보기'에 마우스를 올리면 하위 메뉴가 열리는가 (요구 8) | ✅ 호버로 열림, 현재 모드에 `•` |
| 8 | 보기 모드 8종이 각각 Windows 탐색기와 비슷하게 보이는가 (요구 8) | ✅ 8종 전부 (256/96/48/16 아이콘·목록 세로 흐름·자세히·타일 3줄·내용 우측 정보) |
| 9 | 사진 폴더에서 실제 썸네일이 뜨는가 (요구 8) | ✅ **결함 발견·수정** (아래 F-8 지적 2) |
| 10 | 새 폴더·새 파일이 만들어지는가 (요구 1) | ✅ 디스크에 생성·목록 즉시 반영·개수 갱신, 새 파일은 0KB |
| 11 | 48px·256px 아이콘이 흐리지 않은가 | ✅ 256·96·48px 모두 선명 (T9→T10 인계 항목 종착) |
| 12 | 10만 파일 폴더를 `목록`·`작은 아이콘`으로 스크롤 (NFR-3) | ✅ 응답 유지(`Responding=True`), 모드 전환 2.5~2.8초, 스크롤 반영 확인. **프레임 시간은 외부 스크립트로 잴 수 없어 미측정** |
| 13 | 사진 폴더 스크롤 후 Working Set 증가 (NFR-9) | ✅ 260장 폴더 전 구간 스크롤 후 139.2→155.9MB (+16.7MB) — 260장 전량 유지 시 예상 65MB보다 작아 축출이 동작 |

### F-8에서 발견해 고친 것

1. **`보기` 하위 메뉴 화살표가 `?`로 보였다.** `ui.menu_button`이 붙이는 egui 기본 화살표가
   `⏵`(U+23F5)인데, 이 앱은 egui 내장 글꼴을 끄고 맑은 고딕만 쓰므로 그 글리프가 없어
   두부가 됐다 — 사용자 요구 5번(`?` 제거)과 같은 성질의 결함이 메뉴에서 재발한 것이다.
   `SubMenuButton::from_button` + `egui_phosphor::regular::CARET_RIGHT`로 교체.
2. **썸네일이 화면에 나타나지 않았다.** 픽셀 생성·캐시·텍스처 업로드는 모두 정상이었고,
   빠진 것은 **다시 그리라는 신호**였다. 열거(`DirLoad`)는 워커가 `ctx`를 들고 있어 스스로
   깨우지만 썸네일 워커는 `fs` 계층이라 egui를 모른다(AGENTS: 의존 단방향) — egui는 입력이
   없으면 프레임을 돌리지 않으므로, 마우스를 움직일 때까지 형식 아이콘에 머물렀다.
   `poll_thumbnails`가 "다시 그릴 지연"을 값으로 돌려주고 호출부가 `request_repaint_after`를
   부르게 했다(대기 중에는 `THUMB_POLL_INTERVAL`=50ms로 스스로 깨어나 채널을 확인한다).
   판정을 값으로 돌려준 이유는 `load_texture`가 스스로 일으키는 repaint와 섞여
   테스트가 둘을 구분할 수 없었기 때문이다 — 첫 시도의 `has_requested_repaint` 검사는
   수정을 제거해도 통과해 무효였다.

   이 결함도 notes에 적어 둔 반복 패턴 그대로다: **단위 테스트는 통과하지만 실제 호출
   경로가 그 로직을 지나지 않는다.** 여기서는 "만들고 올리는" 단계마다 테스트가 있었으나
   "그 결과가 화면에 도달하는" 단계에 테스트가 없었다.

## Open Questions

- [x] 8번 보기 모드 범위 → **8모드 전부 + 실제 썸네일** (사용자 확정)
- [x] '닫기'의 대상 → **패널 닫기** (사용자 확정)
- [x] '새 파일'의 형식 → **빈 텍스트 문서 `.txt`** (사용자 확정)
- [x] 열 폭 지속성 → **패널마다 독립 + 세션 저장** (사용자 확정)
- [x] PRD 갱신 → **승인, 갱신 완료** (FR-23~26·NFR-9 추가)
- [x] 썸네일 표시 모드 → **아이콘 4종 + 타일·내용** (사용자 확정)
- [x] 메뉴 바 제거 범위 → **메뉴바 전체 제거** (사용자 확정)
- [x] 항목 수 줄의 '폴더 트리' 토글 → **왼쪽에 그대로 유지** (사용자 확정)
- [x] plan 분할 → **하나로 진행** (사용자 확정)
- [x] 썸네일 캐시 상한 → **패널당 200장 LRU (약 50MB)** (사용자 확정)
- [x] 말줄임 적용 범위 → **모든 열** (사용자 확정)

## Progress Log

- T1-T2 완료 (커밋 c3f2089, 다음 커밋): 목록 셀 말줄임(행 겹침 해소) + 열 폭 드래그·가로 스크롤. 자세히 보기 렌더를 `ui/list_details.rs`로 분리했다. 빌드/테스트/clippy OK, 리뷰 spec MINOR 2·quality OK.
  - 결정: 테스트가 폭을 재려면 앱과 같은 글꼴이 필요하다 — 이 crate는 egui 내장 글꼴을 끄고 맑은 고딕을 직접 등록하므로, 글꼴 없이 배치하면 모든 글자 폭이 0이 되어 말줄임 검증이 무의미해진다. `install_fonts`를 테스트에서 재사용하고 실패 시 assert로 드러낸다.
  - 결정: 렌더 분리로 `list_details → file_list` 역참조가 생겼다(`FileListAction`·`elided_galley`). T10에서 3번째 사용처가 생기면 공용 모듈로 뽑기로 하고 이번에는 두지 않았다(3회 규칙).
- T3-T4 완료 (커밋 a8e99ba, 다음 커밋): 열 폭 세션 저장 + 항목 수를 `폴더 N 파일 M`으로 오른쪽 정렬. 리뷰 T3 spec MINOR 1·T4 지적 0.
  - 결정: 세션 스키마는 버전을 올리지 않고 `#[serde(default)]` 필드만 더한다(D5) — 버전을 올리면 `parse_session`이 통째로 폴백해 기존 사용자의 워크스페이스·분할·탭이 초기화된다. 필드 나열 리터럴이 6곳(테스트 픽스처 4곳 포함)이라 `cargo build`만으로는 누락이 안 드러난다.
  - 결정: 폴더/파일 집계는 `resort()`에서 센다 — 항목이 바뀌는 경로가 반드시 이 함수를 지나므로 집계가 목록과 어긋날 수 없다. `set_entries`에 두면 테스트 헬퍼처럼 그 함수를 안 지나는 경로에서 0이 된다.
  - 고아 후보: `FileListView::len()`이 호출부를 잃었다(상태 줄이 `counts()`로 옮겨감). T10·T11 격자 배치가 총 개수를 쓰므로 주석과 함께 남겨뒀다.
- T5-T7 완료 (커밋 bea5834, 774223c, 다음 커밋): 사이드바 `?` 제거 · 패널 메뉴 신설과 메뉴 바 폐지 · 새 폴더/새 파일. 리뷰 T6 MAJOR 1·T7 BLOCKER 1+MAJOR 3을 반영 후 전부 OK.
  - 결정: 패널 메뉴 명령은 **메뉴를 연 패널**에 적용한다(D16). 활성 패널 판정이 포인터 위치 기반이라, 팝업이 옆 패널 위로 뻗으면 그 아래 패널이 활성이 되어 엉뚱한 패널이 닫힌다. `apply_command(command, target: Option<PanelId>, ...)`로 대상을 명시하고 단축키·타이틀바는 `None`(활성)을 쓴다.
  - 결정: 새 폴더·새 파일도 워커 스레드에서 만든다(D19) — AGENTS.md가 UI 스레드 블로킹 I/O를 금지하고, 같은 파일의 `go_up`/`refresh`가 이미 워커 패턴이라 규율이 갈리면 안 된다.
  - 결정: 생성 성공 시 감시 여부와 무관하게 재열거한다(D20) — `DirWatcher`는 폴더 열기에 실패해도 조용히 끝나므로, 감시 객체가 있다는 것만으로 통지를 믿으면 감시가 죽은 위치에서 새 항목이 안 보인다.
  - 결정: 이름 충돌은 존재 확인 대신 `create_dir`/`create_new`의 `AlreadyExists` 실패를 신호로 처리한다(D18) — 확인과 생성 사이(TOCTOU)에 남의 파일을 덮어쓰지 않기 위해서다.
- T8-T9 완료 (커밋 2e1c8ed, 다음 커밋): 보기 모드 8종 모델·'보기' 하위 메뉴 + 크기별 이미지 리스트·텍스처 캐시 키 교정. 리뷰 T8 BLOCKER 1·MAJOR 2, T9 BLOCKER 1을 반영 후 전부 OK.
  - 결정: T9 acceptance 1을 정정했다 — "48px·256px가 화면에 큰 그림으로 표시된다"는 큰 아이콘을 **쓰는** 화면(격자 보기)이 T10이라 T9 단독으로 검증할 수 없었다. T9는 "리스트를 실제로 얻었는가"(네 단계가 서로 다른 핸들·16px 폴백 아님)를 단위 테스트로 실증하고, **화면 확인은 T10 acceptance 1-b로 명시 이관**했다. plan Files의 `ui/file_list.rs`·`ui/sidebar.rs`도 계획 시점 오류라 뺐다.
  - 주의: `fs`는 `ui`를 모른다(AGENTS 단방향 의존). 모드↔아이콘 크기 매핑 테스트를 처음에 `fs/icons.rs`에 뒀다가 방향 위반이라 `ui/view_mode.rs`로 옮겼다.
  - 주의: 테스트를 추가한 뒤 `cargo build --lib`로만 확인하면 `#[cfg(test)]` 코드가 컴파일되지 않아 타입 오류를 놓친다 — 반드시 `cargo test`로 확인한다.
- T10-T14 완료 (커밋 de2649e, 9b08d7b, 69e940e, 42637f9, 9b68e9b): 격자·목록·타일·내용 렌더 + 보기 모드 세션 저장 + 썸네일 워커·LRU·UI 연결. 리뷰 T13 BLOCKER 1·T14 BLOCKER 1+MAJOR 3을 반영 후 전부 OK.
  - 결정: T2가 예고한 순환 참조를 T10에서 해소했다 — `FileListAction`·말줄임 헬퍼가 세 번째 사용처를 갖게 돼 `ui/list_common.rs`로 옮기고 `file_list`가 재노출한다(호출부 무변경).
  - 결정: 썸네일 텍스처는 매 프레임 픽셀 캐시와 **동기화**한다 — "방금 도착한 것"만 올리면 프레임 상한(4)에 걸린 경로가 영영 못 올라가고, 픽셀이 축출된 텍스처도 GPU에 남는다. 화면에 보이는 항목을 매 프레임 `request`해 LRU를 갱신하는 것도 같은 맥락이다(그리기는 텍스처만 보므로 그곳이 유일한 갱신 지점).
  - 결정: 썸네일은 `SIIGBF_RESIZETOFIT`만 준다 — `BIGGERSIZEOK`을 함께 주면 셸이 요청보다 큰 그림을 돌려줄 수 있어, 장수로만 거는 상한이 바이트를 보장하지 못한다(F-7 M3). 축출도 장수 AND 바이트로 이중화했다.
  - Phase F에서 잡힌 것: **썸네일 해제가 한 번도 실행되지 않았다**(F-7 B1) — 폴더를 커밋한 **뒤에** 이전 경로와 비교해 항상 같았다. 단위 테스트가 판정 헬퍼만 직접 불러 통과했던 것이 원인이라, 이제 `apply_enumerated`를 지나는 회귀 테스트로 바꾸고 버그를 일부러 되살려 테스트가 잡는지 역검증했다.
  - 주의: 레포에 rustfmt 미적용 파일(`ui/address_bar.rs`)이 있어 `cargo fmt --check`가 원래부터 실패한다. `cargo fmt`는 경로 인자를 무시하고 전체를 포맷하므로, 변경 파일만 `rustfmt --edition 2024 <파일>`로 다루고 커밋도 경로를 명시해야 무관한 변경이 섞이지 않는다.

## Phase Ledger

- T1~T14 완료 (2026-07-29)
- Phase F 통과 (HEAD 6dc1b74 — F-7 1회차 BLOCKER 1·MAJOR 4·MINOR 3 반영, 2회차 BLOCKER 0·MAJOR 0·MINOR 3 반영 후 종료)
- Phase G 통과 (Must 100% — F-7의 PRD 전수 대조 재사용, 재루프 0회)
- F-8 통과 (2026-07-30 — 13항목 전부 화면 확인, 결함 2건 발견·수정: 하위 메뉴 화살표 두부, 썸네일 미표시)

## Phase G — PRD 요구 재검증 결과

F-7 `plan-completion-reviewer`가 수행한 PRD 전수 대조를 재사용한다(G-1 규정 — 같은 대조를 반복하지 않는다).

| PRD ID | 우선순위 | 판정 | 근거 |
|---|---|---|---|
| FR-1 자유 분할(진입점 정정) | Must | ✅ | 패널 메뉴로 이관, `menu.rs` 분할 4항목 + 단축키 유지 |
| FR-2 스플리터·패널 닫기 | Must | ✅ | 회귀 없음 + `close_panel(target)` 라우팅 테스트 |
| FR-3 패널별 독립 탭 | Must | ✅ | 회귀 없음 |
| FR-4 목록·정렬·**열 폭·가로 스크롤·말줄임** | Must | ✅ | T1·T2·T3 — 이번 구현 |
| FR-5 시스템 아이콘 | Must | ✅ | 크기별 이미지 리스트로 확장(T9), 16px 경로 무회귀 |
| FR-6 주소창·히스토리 | Must | ✅ | 회귀 없음 |
| FR-7 더블클릭 열기 | Must | ✅ | 격자 보기에도 같은 규칙 적용(T10) |
| FR-8 셸 컨텍스트 메뉴 | Must | ✅ | 격자·빈 영역 모두 유지 |
| FR-15 워크스페이스 사이드바 | Must | ✅ | 상단 토글만 제거, 카드·편집·정렬 유지 |
| FR-16 워크스페이스 생성(진입점 정정) | Must | ✅ | 메뉴 바 제거로 "사이드바 `+`" 문구와 일치 |
| FR-17 워크스페이스 전환 | Must | ✅ | 회귀 없음 |
| FR-9 폴더 트리 | Should | ✅ | 목록 위 토글 유지(메뉴 바 항목만 사라짐) |
| FR-10 변경 자동 새로고침 | Should | ✅ | 감시 갱신 시 썸네일을 지키는 규칙 추가 |
| FR-11 세션 저장·복원 | Should | ✅ | 열 폭·보기 모드 추가, v2 유지로 옛 파일 무손실 |
| FR-12 단축키 | Should | ✅ | 메뉴 바 없이도 전부 동작(테스트로 고정) |
| FR-18 워크스페이스 관리 | Should | ✅ | 사이드바 경로 유지 |
| FR-19 사이드바 접기·폭 | Should | ✅ | 타이틀바 토글·`Ctrl+B` |
| FR-20 워크스페이스 세션 | Should | ✅ | 회귀 없음 |
| FR-21 고정 다크(팝업 메뉴) | Should | ✅ | 메뉴 바 제거 반영 |
| FR-22 커스텀 타이틀바(토글 유일 진입점) | Should | ✅ | T5 |
| **FR-23 보기 모드 8종** | Should | ✅ | T8·T10·T11·T12 |
| **FR-24 썸네일** | Should | ✅ | T9·T13·T14 |
| **FR-25 새 폴더·새 파일** | Should | ✅ | T7 |
| **FR-26 패널 메뉴·메뉴 바 폐지** | Should | ✅ | T6 |
| FR-13·14 | Could | — | 이번 범위 외 (Deferred 대장 유지) |
| NFR-1 콜드 스타트 | — | ⏳ 미측정 | 창 탐지 기반 외부 스크립트로는 신뢰할 수 없었다(같은 조건에서 4.5~10초로 흔들림 — 폴링 루프가 시작을 방해). 이번 변경은 시작 경로를 건드리지 않았다 |
| NFR-2 유휴 메모리 | — | ✅ F-8 실측 | 패널 2개·`혼합` 폴더 자세히 보기에서 **128.8MB** (기준 150MB 미만). 패널 1개는 76.3MB |
| NFR-3 10만 파일 무정지 | — | ✅ F-8 실측 (#12) | `자세히`·`작은 아이콘`·`목록` 모두 응답 유지(`Responding=True`), 모드 전환 2.5~2.8초. 목록 보기 가상 스크롤은 F-7 M1에서 수정. 프레임 시간 자체는 미측정 |
| **NFR-9 썸네일 캐시 상한** | — | ✅ 자동 + F-8 실측 (#13) | 픽셀 상한은 단위 테스트로 강제(장수 AND 바이트). 260장 폴더 전 구간 스크롤 후 139.2→155.9MB(+16.7MB) — 전량 유지 시 예상 65MB보다 작아 축출 동작 확인 |

**Must 충족률 100%** (FR-1~8·15~17 전부 ✅). 갭 없음 — 재루프 불필요.

## Retry Ledger

- (없음)
