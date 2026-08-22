# Plan: Windows 11 모양 컨텍스트 메뉴와 탐색기 단축키

**PRD**: `docs/prd.md` (FR-8 개정 · FR-12 확장 · **FR-64 신설** — 2026-08-22 사용자 승인으로 **이미 반영 완료**)

## 요구 이해

- **원문 요청**: "1. 현재 앱에서 1번 이미지처럼 메뉴가 표시되고 윈도우 탐색기에서는 2번 이미지처럼 메뉴가 표시 되는데 윈도우 탐색기와 동일하게 메뉴를 표시할 수 있으면 표시해줘 / 2. 단축키 구현 Ctrl+Shift+N 새 폴더, Alt+← 뒤로, Alt+→ 앞으로, Alt+↑ 상위 폴더, F5 새로 고침, F2 선택한 파일/폴더 이름 변경, Delete 휴지통으로 이동, Shift+Delete 영구 삭제, Ctrl+C 복사, Ctrl+X 잘라내기, Ctrl+V 붙여넣기"
- **이해한 요구**: ① 우클릭 메뉴를 **Windows 11 탐색기와 같은 모양**으로 바꾼다 — 위쪽 아이콘 줄(잘라내기·복사·이름 바꾸기·공유·삭제), 아이콘이 붙은 항목 줄, 오른쪽 단축키 표기, 맨 아래 `추가 옵션 표시`. 설치된 셸 확장(반디집·PowerRename·Zed 등)도 **그 메뉴 안에** 나와야 한다. ② 단축키 11종을 동작하게 한다. 이 중 F2·Delete·Shift+Delete·Ctrl+C/X/V는 **부를 기능 자체가 앱에 없어** 로컬 이름 바꾸기·삭제·클립보드를 새로 만든다.
- **포함하지 않는 것으로 이해**: Windows 11 모던 메뉴 표면 자체를 호스팅하는 것(공개 API가 없다 — 우리가 같은 모양으로 그린다), 자체 진행률 창(작업은 `IFileOperation`이 하므로 OS 대화를 쓴다), 원격 탭의 클립보드(대응 개념 없음), 새로 만든 항목을 곧바로 편집 상태로 여는 것.

## Goal

우클릭하면 Windows 11 탐색기와 같은 모양의 메뉴가 셸 확장 항목까지 담아 뜨고, 탐색기에서 쓰던 파일 조작 단축키 11종이 그대로 동작한다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-8 (컨텍스트 메뉴 — 2026-08-22 개정) | Must | T4·T5·T6·T7 | ✅ 커버 |
| FR-12 (단축키 — 2026-08-22 확장) | Should | T9·T10·T11 | ✅ 커버 |
| FR-64 (이름 바꾸기·삭제·클립보드 — 신설) | Should | T2·T3·T8 | ✅ 커버 |
| FR-39 (원격 파일 작업) | Should | T11 | ✅ 커버 (진입점만 추가 — 기능 불변) |
| FR-25 (새 폴더·새 파일) | Should | T10 | ✅ 커버 (단축키 진입점만 추가 — 기능 불변) |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- **Windows 11 모던 메뉴 표면 자체를 호스팅하기** — 그 UI는 `Windows.UI.FileExplorer`가 탐색기 프로세스 안에서만 그리며 공개 API가 없다. 우리가 같은 모양으로 그린다.
- **패키지 앱(MSIX) 확장의 `IExplorerCommand` 항목 열거** — 탐색기의 Win11 메뉴는 패키지 매니페스트를 읽어 그 항목도 담지만, 그것을 재현하려면 `PackagedCom` 등록 정보를 직접 파싱해야 한다. 이번에는 `IContextMenu`가 주는 항목만 담고, 그 밖의 것은 `추가 옵션 표시`가 여는 표준 메뉴에 맡긴다.
- **자체 파일 작업 진행률·충돌 대화** — `IFileOperation`이 셸 대화를 띄운다 (PRD Out of Scope 유지, FR-60과 같은 판단).
- **원격 탭의 클립보드(Ctrl+C/X/V)** — 원격에는 대응 개념이 없다(전송은 큐가 담당).
- **새로 만든 항목을 곧바로 이름 편집 상태로 열기** — FR-25는 기본 이름 그대로 둔다.
- **워크스페이스 Delete 키 배정 되살리기** — 2026-08-22 사용자가 이번 동반 처리 대상에서 뺐다. `docs/plans/deferred.md`의 2026-07-28 항목으로 그대로 남는다.

## Deferred / Follow-up

- **`추가 옵션 표시`가 여는 표준 메뉴의 다크 테마** — 종전과 같이 `uxtheme` ordinal 정책에 기대며 이번에 손대지 않는다.
- **owner-draw 확장 항목을 그림째 옮겨 그리기** — `WM_MEASUREITEM`/`WM_DRAWITEM`을 우리 화면에 중계하면 이론상 가능하나 GDI 비트맵을 매 프레임 egui 텍스처로 옮겨야 해 비용이 맞지 않는다. 이번에는 그 줄을 빼고 `추가 옵션 표시`로 보내며, 실제로 빠지는 확장이 있는지는 HUMAN-VERIFY 3에서 관측한다.

## Investigation Log

- 위키 참조: `20_projects/personal/moa/feat-shell-integration.md` — 셸 메뉴는 「요청을 값으로 올려 프레임 밖에서 실행」이 유일하게 안전한 배치이고, `IContextMenu2/3` 서브메뉴 채움에는 창 서브클래싱이 필수다. 이번에 우리가 그리는 메뉴도 **하위 메뉴를 채울 때 같은 포워딩**이 필요하다.
- 위키 참조: `40_guides/recipes/rust/shell-context-menu-in-immediate-mode-ui.md` — 같은 결론의 레시피. `QueryContextMenu`의 id 범위를 1부터 잡는 이유(0은 「고르지 않음」)와 `lpVerb`가 `MAKEINTRESOURCE` 유사 포인터라는 점을 확인.
- 위키 참조: `20_projects/personal/moa/decisions.md` — 이번 요청과 상충하는 과거 결정 없음. 파일 작업을 셸에 맡기는 방향(2026-08-20 FR-60)은 이번 `IFileOperation` 사용과 같은 축이다.
- Deferred 대장(`docs/plans/deferred.md`) 조회: `## 대기` 77건(실측). batch 소진 임계 미달 — 잔량 77 < 100, 절대 상한 130 미달, 최고령 등록일 2026-07-23으로 오늘(2026-08-22) 기준 정확히 30일이라 「30일 초과」에 닿지 않는다. 이번 plan에 batch task를 넣지 않는다.
- 대장 ②(전제 반증) 조회로 **2026-07-28 「워크스페이스 Delete 키 배정」** 을 찾았다 — *"사이드바가 키를 전역으로 보는 현재 구조에서는 파일 목록에서 누른 Delete까지 워크스페이스를 지운다. … F2도 같은 성질"*. 이것이 이번 요청의 F2·Delete 전제를 직접 반박하며, T9이 그 구조를 바꾼다.
- 스킬 개선 큐: 이 세션 대상이 하네스 레포가 아니므로 조회하지 않음.
- `src/ui/menu.rs:319-346` — `shortcut_table()`에 Alt+←(Back)·Alt+→(Forward)·Alt+↑(Up)·F5(Refresh)가 **이미 있다**. 요청 11종 중 이 넷은 배선이 끝나 있고 나머지 7종이 신규다.
- `src/fs/shell_menu.rs:81` — `QueryContextMenu`에 `CMF_NORMAL`만 넘긴다. `CMF_CANRENAME`이 없어 셸이 `rename` verb를 메뉴에 넣지 않으며, 이것이 1번 이미지에 「이름 바꾸기」가 없는 이유다. **이번에도 그 플래그를 넣지 않는다** — 넣으면 눌러도 아무 일이 없는 줄이 생긴다(D2·D9).
- `src/fs/drag_source.rs:62-66` — `SHCreateShellItemArrayFromIDLists` → `BindToHandler::<IDataObject>(BHID_DataObject)`로 **셸이 만든 `IDataObject`**를 얻는 경로가 이미 있다. 클립보드도 같은 객체를 쓰면 `CF_HDROP` 채움을 다시 만들 필요가 없다.
- `src/panel/panel.rs:760,784,821` — **egui 이식 이전 Win32 판**도 `shell_menu::show_context_menu`·`forward_menu_msg`를 부른다(AGENTS.md 「Repository Structure」 주석 — 실행 파일에서는 쓰이지 않지만 **컴파일 대상**이다). 이번 변경은 그 둘의 시그니처를 **바꾸지 않으므로** 그쪽은 손대지 않는다.
- `src/ui/icon_tex.rs:287,352` — `GetDIBits` 호출이 **둘**이고 둘 다 `biBitCount = 32`로 **청해서** 받는다. `src/fs/drag_image.rs:88` 주석도 *"셸이 8·24bpp를 주더라도 `GetDIBits`에 32bpp를 청하므로"*라고 적는다 — 즉 원본 bpp를 판정 조건으로 쓰면 안 된다(T1 Edge).
- `windows` 0.62.2 확인 — `GetMenuItemInfoW`/`MENUITEMINFOW`/`MIIM_*`(`Win32/UI/WindowsAndMessaging/mod.rs:952,4674,5008-5016`), `IFileOperation::RenameItem`/`MoveItem`/`DeleteItem`(`Win32/UI/Shell/mod.rs:23187,23202,23234`), `OleSetClipboard`/`OleGetClipboard`(`Win32/System/Ole/mod.rs:776,533`), `CFSTR_PREFERREDDROPEFFECT`(`Win32/UI/Shell/mod.rs:6466`) 전부 실재. **`Cargo.toml`의 기존 feature(`Win32_UI_WindowsAndMessaging`·`Win32_UI_Shell`·`Win32_System_Ole`·`Win32_System_Com`)로 모두 닿으므로 의존성 변경이 없다.**
- `egui_phosphor::regular` 확인 — `SCISSORS`·`COPY`·`PENCIL`·`SHARE`·`TRASH`·`ARROW_SQUARE_OUT`·`CARET_RIGHT` 전부 존재(`variants/regular.rs`). 아이콘 규약(phosphor 전용)을 지킬 수 있다.
- `src/ui/theme.rs:431-465` — 소스 훑기 시험이 **파일마다 「팝업 여는 구문 수 ≤ `theme::menu_style`·`widgets::menu_row` 호출 수」**를 견준다. `Frame::menu(`도 opener로 센다(`menu_openers`, `theme.rs:475-492`). 새 메뉴 파일은 이 부등식을 만족해야 한다.
- `src/ui/widgets.rs:734` — `menu_row(ui, label, enabled)`는 **아이콘·우측 단축키·하위 메뉴 화살표를 그리지 못한다**. Win11 줄은 새 함수가 필요하며, 토큰이 갈리지 않게 같은 모듈에 둔다.
- `src/ui/list_details.rs:353-381`·`src/ui/list_grid.rs:130-145` — 로컬 목록 렌더러는 **둘뿐**이다(자세히 / 아이콘·타일·목록 계열). 행은 위젯이 아니라 `painter`로 그리므로, 인라인 편집은 그 자리에 `TextEdit`을 얹는 방식이 된다.
- `src/ui/sidebar.rs:272-277` — F2 판정이 `index == list.active_index()`뿐이고 **포커스·소유 판정이 없다**. `ui.input(...)`은 전역 입력이라 파일 목록에서 F2를 눌러도 워크스페이스 이름 편집이 시작된다.
- `src/ui/panel.rs:711` — `PanelState::selected_local() -> Vec<(PathBuf, bool)>`로 선택 항목을 이미 꺼낼 수 있다. 단축키·메뉴가 대상 목록을 새로 만들 필요가 없다.
- `src/ui/app.rs:1780-1810` — `apply_command`가 `command_panel_mut(target)`으로 활성 패널을 찾아 명령을 넘긴다. 신규 명령도 같은 자리에 붙는다.
- `src/ui/remote_menu.rs:203` — 화면 밖 보정에 쓰는 `FRAME_PAD = 8.0`은 주석에 「어림한 값」으로 적혀 있다(실측 아님). `clamp_menu_pos` 사용처는 `panel.rs:1588`·`tree.rs` 둘(실측).

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | Windows 11 모던 컨텍스트 메뉴를 서드파티 앱이 호스팅하는 공개 API가 없다 | Windows SDK·`windows` 0.62.2에 해당 인터페이스 부재. 같은 부류 앱(Files 등)이 전부 자체 렌더링하는 것이 방증 | ⚠ 미확인(부재 증명은 원리상 불가) — **성립을 좌우하지 않는다**: 있더라도 이번 설계(자체 렌더링)는 그대로 동작한다 |
| 2 | `QueryContextMenu`가 채운 HMENU를 `GetMenuItemInfoW`로 읽어 라벨·상태·하위 메뉴·비트맵을 얻을 수 있다 | `windows` 0.62.2 `Win32/UI/WindowsAndMessaging/mod.rs:952`(함수)·`4674`(구조체)·`5008-5016`(`MIIM_*` 마스크 전종) | ✅ |
| 3 | 셸이 만든 `IDataObject`를 클립보드에 그대로 올릴 수 있다 | `Ole/mod.rs:776` `OleSetClipboard(IDataObject)`, 기존 획득 경로는 `src/fs/drag_source.rs:62-66` | ✅ |
| 4 | `IFileOperation`으로 이름 바꾸기·삭제를 할 수 있고 대화는 셸이 띄운다 | `Shell/mod.rs:23187`(`RenameItem`)·`23234`(`DeleteItem`), 기존 사용 경로 `src/fs/file_op.rs:122-155` | ✅ |
| 5 | 요청 단축키 11종 중 Alt+←/→/↑·F5는 이미 동작한다 | `src/ui/menu.rs:319-363` `shortcut_table()`(그 넷은 `354-361`) | ✅ |
| 6 | 사이드바가 F2를 전역으로 가로채 파일 목록의 F2와 충돌한다 | `src/ui/sidebar.rs:272-277` — 포커스 판정 없이 `ui.input(\|i\| i.key_pressed(F2))` | ✅ |
| 7 | 로컬 목록 렌더러는 둘뿐이라 인라인 편집을 두 곳에만 넣으면 된다 | `src/ui/file_list.rs:626`(`list_details::show`)·`656`(`list_grid::show`) — 분기가 그 둘뿐 | ✅ |
| 8 | 새 crate 없이 구현할 수 있다 | 위 Investigation Log의 `windows` 심볼 전건이 이미 링크된 `windows` crate 안에 있다 | ✅ |
| 8-1 | 새 feature 플래그 없이 구현할 수 있다 | **틀렸다** — T3 착수 시 실측: `RegisterClipboardFormatW`는 `Win32_System_DataExchange`, `GlobalAlloc`/`GlobalLock`은 `Win32_System_Memory`에 있고 둘 다 `Cargo.toml`에 없다 | ❌ 정정 — T3이 두 feature를 켠다(패키지 집합은 그대로라 라이선스 자산 재생성은 불요) |
| 9 | 셸의 `rename` verb는 다른 호스트에서 부르면 동작하지 않는다 | 그 verb는 탐색기 자신의 뷰(`IShellView`)가 편집을 시작하는 통지로 처리한다 — 우리 앱에는 받을 뷰가 없다 | ⚠ 미확인(실행 확인 불가) — **성립을 좌우하지 않는다**: 설계가 `IFileOperation::RenameItem`을 쓰므로 verb 동작 여부와 무관하다 |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| owner-draw(`MFT_OWNERDRAW`) 확장 항목은 문자열이 없어 이름을 읽을 수 없다 | 그 줄이 우리 메뉴에서 빠진다 | ① `GetCommandString(GCS_VERBW)`로 verb 이름이라도 얻어 보고 ② 그래도 없으면 그 줄을 **빼고** 메뉴 하단 `추가 옵션 표시`에서 정상으로 보이게 둔다(T4 Edge). 실제 누락 여부는 HUMAN-VERIFY 3 |
| 하위 메뉴(`보내기` 등)는 `WM_INITMENUPOPUP`을 받아야 채워진다 | 하위 메뉴가 빈 채로 열린다 | 하위 메뉴를 **처음 펼칠 때** `IContextMenu2/3::HandleMenuMsg(WM_INITMENUPOPUP, …)`를 직접 부른 뒤 그 HMENU를 다시 읽는다(T4). 기존 `forward_menu_msg`와 같은 인터페이스를 쓴다 |
| `IContextMenu` 수명이 메뉴가 닫힐 때까지 살아 있어야 `InvokeCommand`가 된다 | 항목을 눌러도 아무 일도 일어나지 않는다 | 메뉴가 열려 있는 동안 인터페이스와 HMENU를 함께 쥐는 핸들 타입을 두고, 닫힐 때 `DestroyMenu`까지 한 자리에서 한다(T4 Design) |
| `IFileOperation::PerformOperations`는 끝날 때까지 돌아오지 않는다 | UI 스레드에서 부르면 대용량 삭제 내내 앱이 굳는다 | 기존 `copy_into`와 같이 **워커 스레드 + 채널**로 돌린다(T2). AGENTS의 UI 스레드 블로킹 금지 규약과 같은 처리 |
| 잘라내기 후 원본이 지워지기 전에 클립보드가 다른 것으로 덮이면 이동이 사라진다 | 사용자가 잘라낸 것을 붙여넣지 못한다 | 탐색기와 같은 동작이다 — 잘라내기 표시만 풀고 파일은 그대로 둔다(데이터 손실 없음). 표시 상태와 세 해제 조건은 T8 Design ③ |
| 셸 확장(반디집·PowerRename·Zed 등)의 DLL 로딩·하위 메뉴 채움이 UI 스레드에서 돈다 | 확장이 많이 깔린 PC에서 첫 우클릭·하위 메뉴 펼치기가 눈에 띄게 늦다 | `IContextMenu`가 아파트(STA)에 묶여 워커로 옮길 수 없다(D10). 종전 `TrackPopupMenuEx` 경로도 같았으므로 **새 지연이 아니다**. AGENTS DO NOT의 예외 열거에 적어 규약과 코드를 맞춘다(T12) |
| 새 메뉴 파일이 소스 훑기 시험(`theme.rs`·`widgets.rs`·`i18n`)에 걸린다 | `cargo test` 실패 | 설계 단계에서 규약을 지킨다 — 팝업을 여는 자리에서 `theme::menu_style` 1회, 줄 그리기는 `ui::widgets`의 새 함수, 문구는 `i18n` 카탈로그, 아이콘은 phosphor만(T5 Design) |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `fs::shell_menu::show_context_menu` | `src/ui/shell_host.rs:56` · **`src/panel/panel.rs:760`(Win32 이식 이전 판)** | 시그니처 유지 — `추가 옵션 표시` 폴백 경로가 된다. Win32 판은 손대지 않는다 |
| `fs::shell_menu::forward_menu_msg` | `src/ui/shell_host.rs:143` · **`src/panel/panel.rs:784,821`** | 시그니처 유지 + 신규 하위 메뉴 채움이 같은 인터페이스를 쓴다 |
| `ui::panel::MenuRequest` | `src/ui/panel.rs:187`(정의)·`1162`(생성) · **`src/ui/splitter.rs:73`(필드 통과)·`423`·`477`(시험 안 구조체 리터럴 2개)** · `src/ui/app.rs:2413-2419`(소비) | 필드 추가 — **리터럴 생성처가 3곳**이라 `splitter.rs`를 함께 고치지 않으면 컴파일 실패 |
| `ui::menu::Command` | `src/ui/menu.rs:52`(정의)·`130-170`(패널 메뉴)·**`src/ui/app.rs:1736-1802`(유일한 망라 `match`)** | variant 추가 — 그 `match`가 망라적이라 컴파일러가 누락을 잡는다. `tabs.rs`·`titlebar.rs`는 **생성만** 하고 `match`하지 않아 파급이 없다 |
| `ui::menu::shortcut_table` | `src/ui/menu.rs:319`(정의)·**`404`**(`기존_분할_단축키는_뜻을_그대로_잇는다`)·`474`(F5 확인) | 길이 상수 `[…; 14]` → 신규 개수로 변경. 시험 두 개가 이 배열을 읽는다 |
| `ui::menu::clamp_menu_pos` | `src/ui/menu.rs:371`(정의)·`src/ui/panel.rs:1588`·`src/ui/tree.rs:291`(호출) | T7이 보정 근거를 실측으로 바꾼다 — 시그니처는 유지 |
| `ui::remote_menu::FRAME_PAD` | `src/ui/remote_menu.rs:203`(정의)·`197`·`198`(사용) | T7 대상 — 어림값을 egui 스타일에서 읽는 값으로 |
| **`ui::tree::MENU_FRAME_PAD`** | `src/ui/tree.rs:108`(정의)·`287`·`288`(사용) | **같은 어림값의 두 번째 사본** — T7이 함께 지운다(acceptance의 `FRAME_PAD` 0 hit가 이 이름도 잡는다) |
| `ui::widgets::menu_row` | `src/ui/widgets.rs:734`(정의)·`374`·`src/ui/remote_menu.rs:93` | 유지 — 신규 함수를 옆에 더한다(기존 호출부 불변) |
| `ui::sidebar` F2 처리 | `src/ui/sidebar.rs:272-277` | 조건에 키 소유 판정 추가 |
| `fs::drag_image::read_bgra` | `src/fs/drag_image.rs:91`(정의·유일 호출은 같은 파일) | T1이 공용 모듈로 옮긴다 — 호출부 1곳 갱신 |
| `fs::thumbnail::bitmap_to_rgba` | `src/fs/thumbnail.rs:326`(정의)·같은 파일 내 호출 | T1이 공용 함수를 쓰도록 바꾼다 |
| `ui::icon_tex`의 HBITMAP 읽기 | `src/ui/icon_tex.rs`(HICON→마스크/컬러 비트맵 경로) | T1이 공용 함수를 쓰도록 바꾼다 |
| `PanelState::selected_local` | `src/ui/panel.rs:711`·`src/ui/app.rs` 드래그 경로 | 유지 — 신규 명령이 같은 값을 읽는다 |

### 4-B. 계약·직렬화 변경

- **세션·설정 스키마 변경 없음.** 이름 편집 상태·클립보드 상태·키 소유 영역은 전부 **런타임 값**이며 `settings.json`에 담지 않는다(앱을 다시 띄우면 편집 중이 아닌 것이 옳다).
- `ui::menu::Command`는 앱 내부 열거형이라 외부 계약이 아니다.
- 클립보드 형식은 **Windows 표준**(`CF_HDROP` + `Preferred DropEffect`)을 그대로 쓴다 — 자체 형식을 만들지 않으므로 탐색기·다른 앱과 상호 운용된다.

### 4-C. 테스트 파일

- `src/ui/menu.rs` `mod tests` — `shortcut_table` 길이·내용을 단언하는 시험 2개(`기존_분할_단축키는_뜻을_그대로_잇는다`·F5 확인)가 배열 변경에 함께 걸린다.
- `src/ui/theme.rs` `mod tests` — `팝업_메뉴는_항목_스타일을_거친다`·`팝업_메뉴는_모서리를_따로_적지_않는다`가 `src/ui` **재귀 훑기**라 신규 메뉴 파일을 검사한다.
- `src/ui/widgets.rs` `mod tests` — `화면_코드에_원본_아이콘_기호가_남아_있지_않다`(재귀).
- `src/i18n/mod.rs` `mod tests` — `화면_문구가_카탈로그를_거치지_않은_곳이_없다`.
- `src/ui/panel/tests.rs` — 패널 조작 시험. `MenuRequest` 필드 추가가 이곳 픽스처에 닿을 수 있다.
- 신규: `fs::bitmap`·`fs::file_op`·`fs::clipboard`·`fs::shell_menu`의 순수 로직 시험(모듈 내 `#[cfg(test)] mod tests`).

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `fs::bitmap::bgra_from_hbitmap` | `fs::thumbnail::bitmap_to_rgba:326` · `fs::drag_image::read_bgra:91` · `ui::icon_tex`의 DIB 읽기 — **세 곳이 같은 절차**(`GetObjectW` → 헤더 세우기 → `CreateCompatibleDC` → `GetDIBits`) | **기존 셋을 이 하나로 합친다**(중복 3회 확인 — 공통화 기준 충족). 메뉴 아이콘이 네 번째가 되는 것을 막는 것이 이번 계기 |
| `fs::file_op::rename_item` · `delete_items` | `fs::file_op::copy_into:66` — 같은 모듈, 같은 `IFileOperation`·워커·채널 구조 | **같은 모듈에 나란히 추가**. `perform`의 COM 셋업·`shell_item` 헬퍼를 그대로 재사용 |
| `fs::clipboard` (신규 모듈) | 없음 — 레포에 클립보드 코드는 `ctx.copy_text`(egui 텍스트)뿐(`ui/app.rs:2408`) | 신규. 단 `IDataObject` 획득은 `fs::drag_source`의 경로(`SHCreateShellItemArrayFromIDLists`→`BindToHandler`)를 **공용 함수로 뽑아 재사용** |
| `fs::shell_menu::ShellMenuModel` 외 읽기 함수 | 없음 — 기존 모듈은 HMENU를 만들어 `TrackPopupMenuEx`에 곧바로 넘긴다 | 같은 모듈에 추가(PIDL·COM 수명 규약이 이미 그 파일에 격리돼 있다) |
| `ui::shell_context_menu` (신규 모듈) | `ui::remote_menu`(원격 자체 메뉴) — **구조는 참고하되 합치지 않는다** | 신규. `remote_menu`의 비추상화 선언(*"한쪽은 OS가 그리고 한쪽은 우리가 그린다"*)이 이번에도 유효하다 — 이쪽은 셸 항목·아이콘·하위 메뉴를 다루고 저쪽은 고정 7줄이다 |
| `ui::widgets::menu_row_rich` | `ui::widgets::menu_row:734` — 아이콘·우측 텍스트·화살표를 그리지 못한다 | **같은 모듈에 확장판 추가**. 토큰(`MENU_ITEM_*`)이 갈리지 않게 옆에 둔다. 기존 `menu_row`는 호출부 2곳이 그대로 쓴다 |
| `ui::app::KeyOwner` | 없음 — 지금은 어느 영역이 키를 갖는지 개념 자체가 없다 | 신규(작은 열거형 1개 + 갱신 지점 2곳). 별도 모듈을 만들지 않고 `ui::app`에 둔다 |

### Verified by

- `grep -rn "IContextMenu\|ShellExecute\|TrackPopupMenu" src` → 38건(실측). 셸 메뉴 관련 전건을 위 표에 반영(나머지는 트레이·`ShellExecuteExW` 실행 경로로 이번 변경과 무관).
- `grep -rn "show_context_menu\|forward_menu_msg" src` → 11건(실측): 정의 2 · `ui/shell_host.rs` 4(주석 2 포함) · **`panel/panel.rs` 3** · 모듈 doc 2. Win32 이식 이전 판의 호출 3건을 위 표에 반영.
- `grep -rn "clamp_menu_pos\|FRAME_PAD" src/ui` → 15건(실측): `panel.rs:28,1588` · `menu.rs:371,648,653,659` · `remote_menu.rs:197,198,203` · `tree.rs:23,108,276,287,288,291`. 전건 위 표 반영(어림 상수 사본이 **둘**임을 여기서 확인했다).
- `grep -n "MenuRequest" src/ui/splitter.rs` → 4건(실측): `16`(use) · `73`(필드) · `423`·`477`(구조체 리터럴). 리터럴 2건이 4-A에 빠져 있던 것을 채웠다.
- `grep -n "menu_row" src/ui/widgets.rs src/ui/remote_menu.rs` → 정의 1 + 호출 2(실측). 전건 확인.
- `grep -rn "SHFileOperation\|IFileOperation\|rename\|clipboard" src/fs src/ui` → 로컬 이름 바꾸기·삭제·클립보드 구현이 **한 건도 없음**을 확인(원격 `RemoteMenuAction::Rename` 계열만 hit).
- `src/ui/theme.rs:363-377` `style_calls` 정독 — 인정하는 문자열은 `theme::menu_style(`·`widgets::menu_row(`와 무자격 `menu_style(`·`menu_row(` **넷뿐**이다. `menu_row_rich(`는 `menu_row(`를 부분 문자열로 갖지 않아 **0으로 센다** — T5 Design이 이것을 전제로 `theme::menu_style` 직접 호출을 못 박았다.

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md` FR-8·FR-12·FR-64·Out of Scope 2줄 | PRD가 인라인 편집·클립보드를 **명시적 제외**로 적고 있어, 개정 없이는 코드와 요구가 정면으로 어긋난다 | **이미 완료** — 2026-08-22 사용자 승인 후 이 plan 작성 전에 반영(PRD → plan → 코드 순서) |
| 필수 | `README.md` 42행(셸 컨텍스트 메뉴)·24행(단축키 서술)·210행(아키텍처 플로우)·소스 트리 | 메뉴가 자체 렌더링으로 바뀌고 단축키·신규 모듈이 늘어 현재 서술이 사실과 어긋나게 된다 | T12에 편입 |
| 필수 | `src/ui/menu.rs` 자체 시험 2건 | `shortcut_table` 배열 길이가 바뀌면 그 시험이 깨진다 | T10에 편입 |
| 필수 | `src/ui/menu.rs:295-303` `poll_shortcuts` 문서 주석 | *"삭제는 키를 배정하지 않았다 … 별도 작업으로 미뤘다(Deferred)"* 가 이번 변경으로 거짓이 된다 | T9·T10에 편입(주석 규약 — 코드를 고치면 딸린 주석을 함께 고친다) |
| 필수 | `src/fs/shell_menu.rs` 모듈 doc | *"복사·삭제 등 실제 파일 작업 UI는 전부 셸이 제공한다"* 와 `TrackPopupMenuEx` 중심 서술이 바뀐다 | T4에 편입 |
| 필수 | `AGENTS.md` 「DO NOT」의 UI 스레드 블로킹 **예외 열거**(*"지금 셋뿐이다"*) | 셸 메뉴 열기·하위 메뉴 채움이 UI 스레드에서 돈다(D10 — STA 제약상 워커로 못 옮긴다). 열거를 그대로 두면 규약과 코드가 어긋나 다음 사람이 위반으로 읽는다 | T12에 편입 |
| 필수 | `docs/plans/deferred.md`의 2026-08-21 「32bpp 중복」·2026-08-20 「메뉴 보정 실측」 항목 | T1·T7이 이 둘을 해소하므로 대장에 남겨 두면 이미 끝난 일이 다음 회차에 다시 잡힌다 | T12에 편입(종결 처리) |
| 선택 | 워크스페이스 Delete 키 되살리기 | T9이 키 소유를 도입하면 막고 있던 이유가 사라진다 | **사용자가 거절**(2026-08-22) — Out of Scope에 명시, 대장 항목 유지 |
| 선택 | 32bpp 비트맵 읽기 3곳 통합 | 신규 4번째 중복을 막는 최소 재사용과 별개로, 기존 셋을 합치는 것은 이번 요청 밖 | **사용자가 채택**(2026-08-22) → T1 |
| 선택 | 메뉴 화면 밖 보정 실측화 | 새 메뉴도 같은 보정이 필요해 닿지만, 기존 두 메뉴의 어림값 교체는 요청 밖 | **사용자가 채택**(2026-08-22) → T7 |
| 무관 | 원격 전송 큐·자동 업데이트·설치 파일 생성기 | 이번 변경이 그 경로의 어떤 심볼·문서도 건드리지 않는다(4-A 전건이 UI 메뉴·로컬 파일 작업에 한정) | 건드리지 않음 |
| 무관 | `assets/licenses.json`·`THIRD-PARTY-NOTICES.md` | 의존성이 늘지 않는다(전제 검증 8) — 지문이 바뀌지 않으므로 재생성 불요 | 건드리지 않음 |

## Decisions

### D1. Windows 11 메뉴를 얻는 방법
- **Options**: A) 항목까지 셸 HMENU에서 읽어 우리가 Win11 모양으로 그린다 / B) 앱 기본 항목만 Win11 모양으로 그리고 확장은 `추가 옵션 표시` 안에만 / C) 종전 표준 메뉴 유지
- **Chosen**: A
- **Rationale**: 사용자가 2번 이미지(반디집·PowerRename·Zed가 보이는 화면)를 기준으로 제시했다. B는 그 확장들이 1차 메뉴에서 사라져 요구를 충족하지 못한다.
- **Source**: 2026-08-22 사용자 선택(「셸 항목까지 전부 담기」).

### D2. 상단 아이콘 줄의 다섯 항목을 무엇이 수행하는가
- **Options**: A) 다섯 개 모두 셸 verb를 `InvokeCommand` / B) 잘라내기·복사·이름 바꾸기·삭제는 앱 자체 기능(T2·T3·T8), 공유만 셸 verb
- **Chosen**: B
- **Rationale**: 셸의 `rename` verb는 탐색기 자신의 목록 뷰가 받아 편집을 시작하는 것이라 다른 호스트에서는 아무 일도 일어나지 않는다(전제 9). 잘라내기·복사도 verb 경로는 셸이 자기 클립보드 상태를 쥐어 우리 화면의 「잘라내기 표시」와 어긋난다. `공유`는 대응 기능이 없어 verb를 그대로 부른다.
- **Source**: 전제 검증 9 · FR-64 문면.

### D3. F2·Delete의 키 소유
- **Options**: A) 마지막으로 조작한 영역이 갖는다 / B) 사이드바에서 F2를 뗀다 / C) 마우스가 올라간 영역이 갖는다
- **Chosen**: A
- **Rationale**: 탐색기와 같은 방식이고, 사이드바 조작을 줄이지 않으면서 충돌을 푼다. C는 클릭 없이도 대상이 바뀌어 예측이 어렵다.
- **Source**: 2026-08-22 사용자 선택 · `docs/plans/deferred.md` 2026-07-28 항목.

### D4. 이름 변경 입력 방식
- **Options**: A) 목록에서 바로 편집 / B) 이름 입력 대화상자
- **Chosen**: A
- **Rationale**: 「탐색기와 동일하게」가 이번 요청의 기준이다. 원격 탭이 B를 쓰는 것은 원격에 인라인 편집을 얹을 목록이 셸 항목과 다르기 때문이며, 두 경로가 달라지는 것은 FR-64가 문면으로 인정한다.
- **Source**: 2026-08-22 사용자 선택 · PRD Out of Scope 재한정 문면.

### D5. 원격 탭 단축키 범위
- **Options**: A) 원격에 있는 기능만 잇는다(F2·Delete·Ctrl+Shift+N·F5) / B) 로컬 전용 / C) Ctrl+C/V를 전송으로 해석
- **Chosen**: A
- **Rationale**: 원격에는 `RemoteMenuAction::{Rename,Delete,NewFolder,Refresh}`가 이미 있어 이어 붙이기만 하면 된다. C는 클립보드에 원격 경로를 담는 새 형식이 필요하고 탐색기와 뜻이 달라진다.
- **Source**: 2026-08-22 사용자 선택 · `src/ui/remote_menu.rs:59-73`.

### D6. 새 메뉴의 줄 그리기 부품을 어디에 두는가
- **Options**: A) `ui::widgets`에 `menu_row`의 확장판을 더한다 / B) `ui::shell_context_menu` 안에 자체 그리기 / C) `menu_row` 자체를 아이콘·단축키를 받도록 고친다
- **Chosen**: A
- **Rationale**: 메뉴 한 줄의 토큰 정본이 `ui::theme`이고 그것을 읽는 자리가 `ui::widgets::menu_row` 하나라는 규약(AGENTS)이 이미 있다. B는 그 규약을 깨고 소스 훑기 시험에도 걸린다. C는 호출부 2곳의 시그니처를 바꿔 이득 없이 파급만 는다.
- **Source**: AGENTS.md 「팝업 메뉴 한 줄」 · `src/ui/theme.rs:431-465`.

### D7. 클립보드에 올릴 데이터 객체
- **Options**: A) 셸이 만든 `IDataObject`(`BHID_DataObject`)에 `Preferred DropEffect`만 더해 올린다 / B) `IDataObject`를 직접 구현한다
- **Chosen**: A
- **Rationale**: `CF_HDROP`·`CFSTR_SHELLIDLIST` 등 받는 쪽이 기대하는 형식을 셸이 전부 채운다. B는 `fs::drag_source`가 이미 「데이터 객체를 직접 구현하지 않는다」로 정한 방향과 어긋난다.
- **Source**: `src/fs/drag_source.rs:1-8` 모듈 doc · `windows` `Ole/mod.rs:776`.

### D8. 메뉴 화면 밖 보정을 무엇으로 재는가
- **Options**: A) egui 스타일(`Frame::menu`의 여백·테두리·그림자)에서 읽는다 / B) 어림 상수를 계속 쓴다
- **Chosen**: A
- **Rationale**: 사용자가 이 항목을 동반 처리로 채택했고, 새 메뉴가 세 번째 사용처가 되면서 어림값이 세 곳으로 퍼진다.
- **Source**: 2026-08-22 사용자 선택 · `src/ui/remote_menu.rs:203` 주석(「어림한 값」).

### D9. 셸에 `CMF_CANRENAME`을 줄 것인가
- **Options**: A) 주지 않는다(종전 `CMF_NORMAL` 유지) / B) 주고 그 항목을 모델에서 걸러 낸다 / C) 주고 그대로 보인다
- **Chosen**: A
- **Rationale**: C는 눌러도 아무 일이 없는 줄을 만든다(D2·전제 9 — 셸 `rename` verb는 다른 호스트에서 동작하지 않는다). B는 넣은 것을 도로 지우는 일이라 헛수고다. 이름 바꾸기는 **아이콘 줄이 자체 기능으로 제공**하므로 기능이 빠지지 않는다. `추가 옵션 표시`가 여는 표준 메뉴도 종전 그대로라 **이번 변경으로 없어지는 것이 없다**(그 메뉴에 이름 바꾸기가 없던 것은 종전과 같다 — 1번 이미지가 그 상태다).
- **Source**: `src/fs/shell_menu.rs:81` · D2.

### D10. 셸 확장 로딩·하위 메뉴 채움을 UI 스레드에서 도는 것으로 둘 것인가
- **Options**: A) UI 스레드에서 돌리고 AGENTS의 「블로킹 예외」 열거에 더한다 / B) 워커 스레드로 옮긴다
- **Chosen**: A
- **Rationale**: `IContextMenu`는 **아파트(STA)에 묶인 인터페이스**라 만든 스레드 밖에서 쓸 수 없다 — 워커로 옮기면 그 워커에서 열고 실행까지 마쳐야 하는데, 실행은 사용자가 항목을 고른 뒤라 그 스레드를 메뉴가 닫힐 때까지 붙잡아야 한다. 종전 `TrackPopupMenuEx` 경로도 같은 이유로 UI 스레드에서 돌았으므로 **새로 생기는 제약이 아니다**. 대신 그 사실을 AGENTS DO NOT의 예외 열거에 적어 다음 사람이 규약 위반으로 오해하지 않게 한다.
- **Source**: AGENTS.md 「DO NOT」의 UI 스레드 블로킹 예외 열거 · `src/fs/shell_menu.rs:26-31`(`ACTIVE_MENU`가 `thread_local`인 이유 — *"UI 스레드 전용(STA)"*).

## 시각 요소 분해

**기준**: 사용자가 제시한 Windows 11 탐색기 컨텍스트 메뉴 스크린샷 — `%USERPROFILE%\Desktop\2.png`(비교 대상인 현행 앱 메뉴는 `%USERPROFILE%\Desktop\1.png`).

> 이 표는 **시각 충실도**(Step 2.5 ①-a)의 분해다. `참조 정합 인벤토리`는 두지 않는다 — 메뉴의 **항목 목록은 셸이 실행 시점에 채우는 것**이라 원본에서 옮겨 적을 고정 항목이 없다(기준 이미지의 `반디집`·`PowerRename`은 그 PC에 그 확장이 깔려 있어 나온 것이지 우리가 정하는 목록이 아니다). 고정된 것은 **틀**뿐이며 그것이 아래 표다.
>
> `디자인 값`이 **기존 토큰**인 행은 그 토큰을 그대로 쓴다는 뜻이다 — 기준 이미지의 절대 픽셀을 그대로 옮기면 이 앱의 다른 메뉴(설정·원격·트리)와 어긋나 화면이 갈린다. 기준에서 가져오는 것은 **구조와 배치**(아이콘 줄의 존재·위치, 아이콘 열, 우측 단축키 정렬, `추가 옵션 표시`의 자리)이고, 치수·색은 앱 토큰을 따른다.

### 시각 속성

| 요소 | 속성 | 디자인 값 | 확인 방법 |
|------|------|----------|-----------|
| 메뉴 프레임 | 모서리 | `theme::MENU_CORNER_RADIUS`(6px) — `apply_dark`가 세운 값을 그대로 받는다 | 기준 이미지가 둥근 모서리 / 값은 `src/ui/theme.rs:93`·AGENTS 「팝업 메뉴」 규약 |
| 메뉴 프레임 | 배경·테두리 | `Frame::menu` 기본(egui 스타일) — 이 파일에 색을 새로 적지 않는다 | AGENTS 「팝업 메뉴」 — 모서리·색을 각 메뉴가 적지 않는다 |
| 메뉴 프레임 | 폭 | 260px(이 메뉴 고유 값 — 우측 단축키 열이 있어 원격 메뉴 180px보다 넓다) | 기준 이미지에서 라벨 + 단축키가 한 줄에 드는 폭 |
| **아이콘 줄** | 위치 | 메뉴 **맨 위**, 항목 목록 위 | 기준 이미지에서는 아래쪽에 있으나 그것은 메뉴가 화면 아래에서 위로 뒤집혀 열린 것이다 — 탐색기 기본은 맨 위 |
| 아이콘 줄 | 칸 수·구성 | 5칸 균등 분할 — 잘라내기·복사·이름 바꾸기·공유·삭제 | 기준 이미지 하단 5개 아이콘 |
| 아이콘 줄 | 아이콘 | `SCISSORS`·`COPY`·`PENCIL`·`SHARE`·`TRASH`(phosphor `regular`) | AGENTS 「아이콘」 규약 — phosphor에서만 가져온다. 존재 확인: `egui-phosphor` `variants/regular.rs` |
| 아이콘 줄 | 툴팁 | `i18n` 카탈로그 — 한국어 `잘라내기`·`복사`·`이름 바꾸기`·`공유`·`삭제` / 영어 `Cut`·`Copy`·`Rename`·`Share`·`Delete` | 라벨 없는 아이콘 줄이라 이름이 툴팁으로만 드러난다(기준 이미지도 그렇다). AGENTS 「화면 문구」 규약 대상 |
| 아이콘 줄 | 높이 | `theme::MENU_ITEM_HEIGHT`(28px) + 상하 4px | 기준 이미지에서 아이콘 줄이 일반 줄보다 조금 높다 |
| 아이콘 줄 | hover | `theme::MENU_HOT` 채움 + `MENU_ITEM_CORNER_RADIUS`(4px) — **삭제 칸만** `MENU_HOT_DANGER` | 앱 토큰(`src/ui/theme.rs:107-118`). 삭제의 파괴색은 사이드바 `삭제`가 이미 쓰는 규칙 |
| 아이콘 줄 | 비활성 | 선택 0개(폴더 배경 메뉴)면 줄 전체가 `theme::TEXT_DIM` | 기준 이미지에는 없는 상태 — 앱 규칙(`widgets::menu_row`의 비활성 색)을 따른다 |
| 구분선 | 위치 | 아이콘 줄 아래 1개 · `추가 옵션 표시` 위 1개 · 셸이 준 구분선은 모델 그대로 | 기준 이미지의 가로선 위치 |
| 항목 줄 | 높이·좌우 여백 | `theme::MENU_ITEM_HEIGHT`(28px) · `theme::MENU_ITEM_PAD_X`(12px) | AGENTS 「팝업 메뉴 한 줄」 — 값의 정본은 `ui::theme` |
| 항목 줄 | 아이콘 열 | 왼쪽 고정 폭 20px, 아이콘 16px 중앙 정렬. 아이콘이 없어도 열은 **자리를 지킨다** | 기준 이미지에서 아이콘 있는 줄과 없는 줄의 라벨이 같은 x에서 시작한다 |
| 항목 줄 | 라벨 | 아이콘 열 오른쪽, 왼쪽 정렬, `theme::TEXT` | 기준 이미지 |
| 항목 줄 | 단축키 | **오른쪽 정렬**, `theme::TEXT_MUTED`, 라벨보다 흐리게 | 기준 이미지의 `Enter`·`Ctrl+Shift+C`·`Alt+Enter` |
| 항목 줄 | 하위 메뉴 표시 | 오른쪽 끝 `CARET_RIGHT`(phosphor) | 기준 이미지의 `다음으로 공유`·`압축...` 우측 화살표 |
| 항목 줄 | hover | `theme::MENU_HOT` 채움 + 4px 모서리. 평상시 배경은 **투명** | AGENTS 「팝업 메뉴 한 줄」 — 칠하면 버튼 목록처럼 보인다 |
| `추가 옵션 표시` | 위치·구성 | 메뉴 **맨 아래**, 구분선 아래. `ARROW_SQUARE_OUT` 아이콘 + 라벨 | 기준 이미지의 같은 줄(`추가 옵션 표시`) |
| `추가 옵션 표시` | 문구 | `i18n` 카탈로그 — 한국어 `추가 옵션 표시` / 영어 `Show more options` | 기준 이미지가 한국어 `추가 옵션 표시`. AGENTS 「화면 문구」 규약 |
| 메뉴 전체 | 화면 밖 보정 | `menu_frame_pad`(T7 — egui 스타일 실측) | 기준 이미지가 화면 아래에서 위로 뒤집혀 열린 상태 |

## Tasks

<!-- T1~T3 (기반 계층) · T4~T7 (메뉴) · T8~T11 (편집·단축키) · T12 (문서) -->

- [x] T1. 32bpp 비트맵 → RGBA 읽기를 공용 모듈로 모은다
  - **Type**: C
  - **Design**: ① `src/fs/bitmap.rs` 신규(`fs` 계층 — `ui`를 모른다) ② `pub(crate) fn bgra_from_hbitmap(bitmap: HBITMAP) -> Option<(i32, i32, Vec<u8>)>` — GDI 32bpp DIB를 폭·높이·BGRA 바이트로 읽는다. `unsafe`는 이 함수 안에 격리하고 사유 주석을 단다 ③ `fs::thumbnail`·`fs::drag_image`·`ui::icon_tex`가 이 함수를 부른다(의존은 단방향 — 셋이 `fs::bitmap`을 참조) ④ **비추상화 선언**: 색 순서 변환(BGRA→RGBA)·알파 전처리는 부르는 쪽이 그대로 한다. 셋의 후처리가 서로 달라(썸네일은 `ThumbnailImage`, 아이콘은 마스크 합성, 드래그는 원본 BGRA) 그것까지 합치면 분기 인자가 늘어 읽기 어려워진다
  - **Acceptance**: Given 기존 세 경로, When `cargo test`·`cargo clippy --all-targets -- -D warnings`, Then 경고 0으로 통과하고 세 파일 어디에도 `GetDIBits` 직접 호출이 남지 않는다(`grep -c "GetDIBits" src/fs/thumbnail.rs src/fs/drag_image.rs src/ui/icon_tex.rs` → 각 0)
  - **Files**:
    - 주: `src/fs/bitmap.rs`(신규), `src/fs/mod.rs`
    - 동반: `src/fs/thumbnail.rs:326`, `src/fs/drag_image.rs:91`, `src/ui/icon_tex.rs:287`·`352`(**`GetDIBits` 호출이 둘 — 하나만 고치면 나머지가 중복으로 남는다**)
    - 테스트: `src/fs/bitmap.rs` 내 `mod tests`(널 핸들·0 크기 입력이 `None`)
  - **Edge Cases**: 널·무효 HBITMAP → `None` / 폭·높이 0 → `None` / **`GetDIBits`가 0줄을 돌려주면 `None`** — 원본 bpp를 판정 조건으로 쓰지 않는다(세 경로 모두 `biBitCount = 32`로 **청해서** 받으므로 8·24bpp 원본도 정상 변환된다. 원본 bpp로 거르면 아이콘·썸네일이 조용히 깨진다 — `drag_image.rs:88` 주석이 같은 사실을 적는다)
  - **Halt Forecast**:
    - (i) 세 경로의 픽셀 후처리가 미묘하게 달라 합치면 화면이 바뀔 수 있다 → Design ④가 후처리를 옮기지 않도록 못 박았다
  - **Depends on**: -

- [x] T2. 로컬 이름 바꾸기·삭제를 `fs::file_op`에 더한다
  - **Type**: C
  - **Design**: ① `src/fs/file_op.rs`(기존 모듈) ② `pub fn rename_item(path, new_name, owner, done, wake) -> bool` / `pub fn delete_items(paths, permanent: bool, owner, done, wake) -> bool` — 둘 다 `copy_into`와 같이 **곧바로 돌아오고 결과는 채널로** 온다. 결과 타입은 기존 `CopyOutcome`을 일반화하지 않고 `OpOutcome { requested, cancelled, error }`를 새로 두되 필드는 같게 한다 ③ 기존 `perform`의 COM 셋업·`shell_item` 헬퍼를 그대로 쓴다. `ui` 계층은 이 모듈을 모른다 ④ **비추상화 선언**: 세 작업(복사·이름·삭제)을 공통 `enum FileOp`로 묶지 않는다 — 인자가 서로 다르고(대상 폴더 vs 새 이름 vs 영구 여부) 묶으면 호출부가 매번 쓰지 않는 필드를 채우게 된다
  - **Acceptance**: Given 빈 대상 목록, When `delete_items`를 부르면, Then 워커를 띄우지 않고 `false`를 돌려준다(기존 `원본이_없으면_워커를_띄우지_않는다`와 같은 계약). Given `permanent = false`, When 삭제 플래그를 만들면, Then `FOF_ALLOWUNDO`가 켜지고 `permanent = true`면 꺼진다(플래그 산출 함수 단위 시험)
  - **Files**:
    - 주: `src/fs/file_op.rs`
    - 동반: `src/i18n/mod.rs`(이름 거부 사유 문구 — `i18n` 훑기 시험의 `ROOTS`에 `src/fs`가 들어 있어(`src/i18n/mod.rs:1308`) 리터럴로 적으면 `cargo test`가 잡는다. `fs::create`·`fs::file_op`이 이미 같은 방식을 쓴다)
    - 테스트: 같은 파일 `mod tests`(빈 입력 계약 · 삭제 플래그 산출 · 이름 검증)
  - **Edge Cases**: 빈 이름·기존과 같은 이름 → 셸에 걸지 않고 `error` 없이 `requested = 0`으로 끝낸다 / 이름에 `\ / : * ? " < > |` 포함 → 걸기 전에 거부(사유 문구는 `i18n`) / 대상이 그 사이 사라짐 → 그 항목만 건너뛴다(복사와 같은 규칙) / 읽기 전용·권한 없음 → 셸이 자기 대화로 승격을 묻는다
  - **Halt Forecast**:
    - (i) 이름 검증 규칙이 어디까지인가 → Edge Cases가 금지 문자 집합과 처리를 확정했다
  - **Depends on**: -

- [x] T3. 클립보드 계층을 만든다
  - **Type**: D
  - **Design**: ① `src/fs/clipboard.rs` 신규 + `src/fs/drag_source.rs`에서 `IDataObject` 획득 경로를 `pub(crate) fn data_object(paths) -> Option<IDataObject>`로 뽑아 공유 ② `pub fn put(paths: &[PathBuf], cut: bool) -> bool` — 셸 `IDataObject`에 `CFSTR_PREFERREDDROPEFFECT`(`DROPEFFECT_MOVE`/`COPY`)를 `SetData`로 얹고 `OleSetClipboard` / `pub fn take() -> Option<ClipboardFiles>` — `OleGetClipboard` → `CF_HDROP`(`DragQueryFileW`)로 경로를, `Preferred DropEffect`로 이동 여부를 읽는다 ③ `fs` 계층 — `ui`를 모른다. 붙여넣기 실행은 `ui`가 이 모듈로 읽어 `fs::file_op`에 넘긴다(두 모듈을 서로 참조시키지 않는다) ④ **비추상화 선언**: 텍스트·이미지 등 파일 아닌 클립보드 형식은 다루지 않는다. `arboard`(egui의 텍스트 클립보드)와 합치지 않는다 — 그쪽은 OLE를 쓰지 않아 데이터 객체를 얹을 수 없다
  - **Acceptance**: Given 경로 2개를 `put(paths, cut = true)`로 담고, When 곧바로 `take()`를 부르면, Then 같은 경로 2개와 `cut = true`가 돌아온다(왕복 시험 — 실제 클립보드를 쓰므로 `#[ignore]`가 아닌 직렬 시험 1건). Given 파일이 담기지 않은 클립보드, When `take()`, Then `None`
  - **Files**:
    - 주: `src/fs/clipboard.rs`(신규), `src/fs/mod.rs`
    - 동반: `src/fs/drag_source.rs`(데이터 객체 획득 경로 추출 — 기존 `start_copy_drag` 동작 불변)
    - 테스트: `src/fs/clipboard.rs` 내 `mod tests`(왕복 · 빈 클립보드 · 잘못된 형식)
  - **Edge Cases**: 다른 앱이 클립보드를 쥐고 있어 열지 못함 → `false`/`None`(조용히) / 담긴 경로가 그 사이 사라짐 → 붙여넣을 때 `file_op`이 그 항목만 건너뛴다 / 클립보드에 파일이 아니라 텍스트만 있음 → `None` / 100개 넘는 경로 → 상한을 두지 않는다(셸이 처리)
  - **Halt Forecast**:
    - (i) 셸 `IDataObject`가 `SetData`를 거부할 수 있다 → 거부하면 `Preferred DropEffect` 없이 올리고 **복사로 간주**한다(붙여넣기 기본값). 데이터가 사라지지 않는 안전한 폴백
  - **Depends on**: - (Design ③대로 `fs::clipboard`는 `fs::file_op`을 부르지 않는다 — 둘을 잇는 것은 `ui`다)

- [ ] T4. 셸 컨텍스트 메뉴를 읽어 모델로 바꾼다
  - **Type**: D
  - **Design**: ① `src/fs/shell_menu.rs`(기존 모듈 — PIDL·COM 수명 규약이 이미 격리돼 있다) ② `pub struct ShellMenu` — `IContextMenu`(+ `IContextMenu2/3` 캐스트)와 HMENU를 함께 쥐는 핸들. `Drop`에서 `DestroyMenu`. `pub fn open(owner, folder, items) -> Option<ShellMenu>` / `pub fn model(&self) -> Vec<ShellMenuItem>` / `pub fn submenu(&self, id) -> Vec<ShellMenuItem>`(펼칠 때 `HandleMenuMsg(WM_INITMENUPOPUP)` 후 다시 읽는다) / `pub fn invoke(&self, id, owner)` / `pub fn verb(&self, id) -> Option<String>`. `ShellMenuItem { id, label, icon: Option<(i32,i32,Vec<u8>)>, enabled, checked, separator, has_submenu }` ③ `fs` 계층 — `ui`를 모르고 egui 타입을 쓰지 않는다(아이콘은 픽셀 바이트로 넘긴다). 아이콘 읽기는 T1의 `fs::bitmap` ④ **비추상화 선언**: 「메뉴 공급자」 추상 trait을 만들지 않는다 — 원격 메뉴(`ui::remote_menu`)와 합치지 않는다는 기존 선언(`remote_menu.rs:1-6`)을 그대로 잇는다
  - **Acceptance**: Given `&` 액셀러레이터·탭 단축키가 섞인 메뉴 문자열, When 라벨 정규화 함수를 통과시키면, Then `&`가 제거되고 탭 뒤 단축키가 별도 필드로 갈린다(순수 함수 단위 시험). Given 로컬 파일 하나, When 메뉴를 열면, Then 모델에 항목이 1개 이상 담긴다(HUMAN-VERIFY 1 — HWND가 필요해 자동 시험 비대상). Given `grep -n "QueryContextMenu" -A 2 src/fs/shell_menu.rs`, When 그 **호출부 전건**을 보면, Then 넘기는 플래그가 `CMF_NORMAL`뿐이다(D9 — `CMF_CANRENAME`을 넣지 않는다. **주석에 그 이름이 근거로 적히는 것은 무방하다** — 재는 것은 실제로 넘기는 값이다)
  - **Files**:
    - 주: `src/fs/shell_menu.rs`
    - 동반: `src/fs/bitmap.rs`(T1 산출물 사용), `src/ui/shell_host.rs`(핸들을 여는 진입점 추가 — 기존 `popup`은 유지)
    - 테스트: `src/fs/shell_menu.rs` 내 `mod tests`(라벨 정규화 · 단축키 분리 · 빈 모델 처리)
  - **Edge Cases**: `MFT_OWNERDRAW` 항목(문자열 없음) → `verb()`로 이름을 시도하고 실패하면 **그 줄을 뺀다** / 라벨이 빈 문자열 → 뺀다 / 구분선이 맨 앞·맨 뒤·연속으로 옴 → 접어서 하나로 / `hbmpItem`이 `HBMMENU_*` 시스템 값 → 아이콘 없음으로 / 하위 메뉴가 비어 돌아옴 → 화살표는 그리되 펼치면 「항목 없음」 대신 아무것도 그리지 않는다 / `QueryContextMenu` 자체 실패 → `None`(호출부가 종전 표준 메뉴로 폴백) / **확장 DLL 로딩이 오래 걸림**(반디집·PowerRename 등이 깔린 환경에서 첫 우클릭이 수백 ms) → **UI 스레드에서 그대로 기다린다**(D10 — STA 제약상 워커로 옮길 수 없고, 종전 `TrackPopupMenuEx` 경로도 같았다). 중간 취소·진행 표시는 두지 않는다
  - **Halt Forecast**:
    - (i) 하위 메뉴 채움에 창 핸들이 필요하다 → `ShellHost::hwnd()`가 이미 있다(`shell_host.rs:49`)
    - (i) UI 스레드 블로킹이 AGENTS DO NOT의 예외 열거를 넘는다 → D10이 판정했고, 열거 갱신은 「동반 변경 판정」 필수로 T12에 편입
    - (ii-a) `fs::shell_menu`의 공개 API가 늘어난다(구조 변경) → `## 사전 승인 항목`에 등록
  - **Depends on**: T1

- [ ] T5. Windows 11 모양 메뉴를 그린다
  - **Type**: D
  - **Design**: ① `src/ui/shell_context_menu.rs` 신규 + `src/ui/widgets.rs`에 `pub(crate) fn menu_row_rich(ui, icon, label, shortcut, has_submenu, enabled) -> bool`. **팝업을 여는 자리(`Frame::menu`를 여는 함수)의 첫 줄에서 `theme::menu_style(ui)`를 1회 부른다** — `theme.rs:363-377`의 `style_calls`가 인정하는 문자열은 `theme::menu_style(`·`widgets::menu_row(`와 그 무자격 형태 넷뿐이라 `menu_row_rich(`는 **0으로 세어져** 부등식(열림 ≤ 호출)이 깨진다. 하위 메뉴는 별도 `Area`라 **거기서도 따로 부른다** ② 구성은 위에서부터 **아이콘 줄**(잘라내기 `SCISSORS`·복사 `COPY`·이름 바꾸기 `PENCIL`·공유 `SHARE`·삭제 `TRASH` — 균등 분할 5칸) → 구분선 → **항목 줄들**(아이콘 열·라벨·우측 단축키·하위 메뉴 `CARET_RIGHT`) → 구분선 → **`추가 옵션 표시`**(`ARROW_SQUARE_OUT`). 고른 것을 `ShellMenuChoice` 값으로 돌려주고 **실행하지 않는다**(`remote_menu`와 같은 규칙) ③ `ui` 계층 — `fs::shell_menu`의 모델을 읽고 아이콘 픽셀을 egui 텍스처로 올린다(`ui::icon_tex`의 캐시 방식을 따르되 메뉴 전용 캐시를 이 모듈에 둔다 — 메뉴가 닫히면 버린다) ④ **비추상화 선언**: 「메뉴 뷰」 공통 컴포넌트를 만들지 않는다. 색·행 높이는 `ui::theme` 토큰만 쓰고, **모서리는 아예 적지 않는다** — `apply_dark`가 `visuals.menu_corner_radius`에 세운 값을 `Frame::menu`가 그대로 받으므로, `.corner_radius(...)`를 쓰면 값이 같아도 `팝업_메뉴는_모서리를_따로_적지_않는다`(`theme.rs:279-314` — 값을 가리지 않고 잡는다)에 걸린다. 이 파일에 새 상수를 만들지 않는다(폭·아이콘 열 너비만 예외 — 이 메뉴 고유 값). **화면 밖 보정은 이 task가 하지 않는다** — 메뉴 크기 계산만 여기서 내고, 그 값을 클램프에 넣는 것은 T6, 여백을 실측으로 바꾸는 것은 T7이다
  - **Acceptance**: Given 모델 항목 N개, When 메뉴 높이를 계산하면, Then `아이콘 줄 + 구분선 + N × MENU_ITEM_HEIGHT + 추가 옵션 줄`과 같다(순수 함수 시험). Given `cargo test`, When 소스 훑기 시험 3종(`팝업_메뉴는_항목_스타일을_거친다` · `팝업_메뉴는_모서리를_따로_적지_않는다` · `화면_코드에_원본_아이콘_기호가_남아_있지_않다`)을 돌리면, Then 신규 파일을 포함해 전부 통과한다
  - **Files**:
    - 주: `src/ui/shell_context_menu.rs`(신규), `src/ui/mod.rs`, `src/ui/widgets.rs`
    - 동반: `src/i18n/mod.rs`(`추가 옵션 표시` 등 신규 문구), `src/ui/theme.rs`(토큰 추가가 필요할 때만)
    - 테스트: `src/ui/shell_context_menu.rs` 내 `mod tests`(높이 계산 · 아이콘 줄 활성 규칙 · 빈 모델)
  - **Edge Cases**: 항목이 화면 높이를 넘음 → 세로 스크롤(egui `ScrollArea`) / 라벨이 메뉴 폭을 넘음 → 말줄임 / 아이콘 텍스처를 만들지 못함 → 아이콘 자리를 비워 두고 라벨은 그대로 / 선택이 0개(배경 메뉴) → 아이콘 줄 전체가 비활성 / 아이콘 줄의 다섯 중 대응 기능이 없는 것(공유 verb 부재) → 그 칸만 비활성
  - **Halt Forecast**:
    - (i) 소스 훑기 시험(`팝업_메뉴는_항목_스타일을_거친다`)이 신규 파일을 잡을 수 있다 → Design ①이 **팝업을 여는 자리마다 `theme::menu_style(ui)` 1회 호출**을 못 박았다. 예외 목록은 경로 전체 비교(`theme.rs`·`panel.rs`)라 신규 파일은 면제되지 않으므로, 예외에 추가하지 않고 부등식을 만족시키는 쪽으로 푼다
  - **Depends on**: T4

- [ ] T6. 새 메뉴를 배선하고 고른 것을 실행한다
  - **Type**: D
  - **Design**: ① `src/ui/app.rs`(메뉴 상태 보유·실행)·`src/ui/panel.rs`(`MenuRequest` 생성) ② `MenuRequest`에 선택 항목의 폴더 여부를 더하고, `ExplorerApp`에 `open_menu: Option<OpenShellMenu>`(핸들 + 화면 위치 + 대상 경로)를 둔다. 그리기는 프레임 **마지막**에 — `TrackPopupMenuEx`와 달리 재진입은 없지만, 모든 패널을 그린 뒤 최상위에 떠야 한다 ③ 실행 분기: 아이콘 줄 → T2·T3의 자체 기능(공유만 `invoke`) / 일반 항목 → `ShellMenu::invoke` / `추가 옵션 표시` → **기존 `ShellHost::popup`**(그 호출만 종전대로 프레임 밖에서) ④ 화면 밖 보정은 **기존 `ui::menu::clamp_menu_pos`를 그대로** 부른다(T5가 낸 메뉴 크기를 넘긴다). 프레임 여백은 이 시점에 아직 어림이며 **T7이 실측으로 바꾼다** — 그 사이 화면 끝에서 몇 px 어긋날 수 있으나 계획된 중간 상태다 ⑤ **비추상화 선언**: 메뉴 실행 결과를 명령 열거형(`ui::menu::Command`)에 합치지 않는다 — 셸 항목 id는 그 메뉴 인스턴스에서만 뜻이 있어 저장·재생할 수 있는 값이 아니다
  - **Acceptance**: Given 로컬 목록에서 우클릭, When 메뉴가 뜨면, Then Win11 모양 메뉴가 뜨고 `추가 옵션 표시`를 누르면 종전 표준 메뉴가 같은 자리에 뜬다(HUMAN-VERIFY 2·3). Given 메뉴가 열린 프레임, When 파일 대화·OS 드래그 요청이 함께 들어오면, Then 종전과 같이 그 둘을 다음 프레임으로 미룬다(`app.rs:2422-2431`의 기존 규칙 유지 — 단언은 코드 검토)
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/panel.rs`
    - 동반: **`src/ui/splitter.rs:73,423,477`(`MenuRequest` 필드 통과 + 시험 안 구조체 리터럴 2개 — 고치지 않으면 컴파일 실패)**, `src/ui/shell_host.rs`, `src/ui/list_details.rs`, `src/ui/list_grid.rs`(우클릭 시 폴더 여부 전달)
    - 테스트: `src/ui/panel/tests.rs`(`MenuRequest` 생성 규칙 — 선택 밖 우클릭이 단독 선택으로 바뀌는 기존 규칙 유지), `src/ui/splitter.rs` 내 `mod tests`(`여러_패널의_서로_다른_결과가_함께_살아남는다` 등 리터럴을 든 시험)
  - **Edge Cases**: 메뉴가 열린 채 폴더가 바뀜(감시 갱신) → 메뉴를 닫는다 / 메뉴가 열린 채 앱이 트레이로 숨음 → 닫는다 / 메뉴 밖 클릭·`Esc` → 닫는다 / COM STA를 못 잡은 환경 → 종전과 같이 안내를 띄우고 메뉴를 열지 않는다(`app.rs:2186`) / 셸 항목 실행이 새 창을 띄우는 동안 → 메뉴는 먼저 닫는다 / **하위 메뉴를 처음 펼칠 때 확장이 늦게 응답** → 그 프레임이 늘어지는 것을 그대로 둔다(D10 — UI 스레드에서 기다린다). 펼치는 동안 다른 항목의 hover는 반응하지 않는다
  - **Halt Forecast**:
    - (i) 두 메뉴(자체·표준)가 겹칠 수 있다 → `추가 옵션 표시`는 **자체 메뉴를 닫은 뒤** 프레임 밖에서 표준 메뉴를 띄운다
    - (ii-a) `MenuRequest` 구조체 필드 변경(공개 계약) → `## 사전 승인 항목`에 등록
  - **Depends on**: T5, T2, T3

- [ ] T7. 메뉴 화면 밖 보정을 실측 값으로 바꾼다
  - **Type**: C
  - **Design**: ① `src/ui/menu.rs`(`clamp_menu_pos` 옆) ② `pub(crate) fn menu_frame_pad(ui: &egui::Ui) -> egui::Vec2` — `Frame::menu`가 쓰는 여백·테두리 굵기·그림자 확장을 egui 스타일에서 읽어 더한다 ③ **이 함수를 쓰는 메뉴는 셋이며 이 task가 셋 다 붙인다** — ⓐ `ui::remote_menu::FRAME_PAD`(`remote_menu.rs:203`, 사용 `197`·`198`) ⓑ `ui::tree::MENU_FRAME_PAD`(`tree.rs:108`, 사용 `287`·`288`) 두 어림 상수를 지우고 호출로 바꾸며, ⓒ **T5·T6이 만든 새 메뉴(`ui::shell_context_menu`)의 크기 산출에도 이 여백을 더한다**. T7이 T5·T6 뒤에 오는 이유가 ⓒ다 — 그 파일이 있어야 붙일 수 있다 ④ **비추상화 선언**: 「메뉴 배치기」 타입을 만들지 않는다 — 필요한 것은 함수 하나다
  - **Acceptance**: Given 기본 다크 스타일, When `menu_frame_pad`를 부르면, Then `Frame::menu`의 실제 여백·테두리 합과 같다(egui 스타일 값으로 단언하는 단위 시험). Given `grep -rn "FRAME_PAD" src/ui`, When 검색하면, Then **0 hit**이다(`MENU_FRAME_PAD`도 이 문자열을 포함하므로 `tree.rs`의 사본까지 함께 지워야 통과한다)
  - **Files**:
    - 주: `src/ui/menu.rs`, `src/ui/remote_menu.rs`, `src/ui/tree.rs`
    - 동반: `src/ui/shell_context_menu.rs`(T5 산출물 — 새 메뉴의 크기 산출에 여백을 더한다. Design ③ⓒ)
    - 테스트: `src/ui/menu.rs` 내 `mod tests`(보정 값 산출 · 기존 `clamp_menu_pos` 시험 3건 유지)
  - **Edge Cases**: 그림자가 꺼진 스타일 → 0을 더한다 / 메뉴가 화면보다 큼 → 기존 규칙대로 왼쪽·위 모서리를 우선한다 / `tree.rs`의 메뉴 크기 계산이 `remote_menu`와 행 수 산출이 다르다(트리는 1줄) → 크기 계산은 각자 두고 **여백만** 공용 함수로 받는다
  - **Halt Forecast**:
    - (i) egui가 그림자 크기를 노출하지 않을 수 있다 → 노출하지 않으면 그 항만 종전 어림값을 쓰고 **그 사실을 주석에 적는다**(전부 실측이라고 적어 두면 다음 사람이 오해한다). 그 경우에도 상수는 `menu_frame_pad` 안 한 곳에만 남으므로 acceptance의 0 hit는 만족한다
  - **Depends on**: T5, T6 (Design ③ⓒ — `ui::shell_context_menu`가 있어야 그 메뉴에 붙일 수 있다. ⓐ·ⓑ만 보면 독립이지만 셋을 한 task로 묶어 여백 정본이 한 번에 서게 한다)

- [ ] T8. 목록에서 바로 이름을 고치고, 잘라낸 항목을 흐리게 보인다
  - **Type**: D
  - **Design**: ① 상태 둘 다 `src/ui/file_list.rs`(`FileListView`)가 소유하고, 그리기는 `list_details`·`list_grid` 둘이 한다 ② **이름 편집** — `RenameEdit { index: usize, text: String, first_frame: bool }`. 이름 칸 자리에 `egui::TextEdit`을 얹는다. `Enter` 확정 → `FileListAction::Rename { index, new_name }` / `Esc`·포커스 상실 → 취소 ③ **잘라내기 표시** — `cut_marks: HashSet<PathBuf>`(경로 기준 — 목록이 갱신돼도 행 번호와 달리 어긋나지 않는다). 표시된 행은 라벨·아이콘을 `theme::TEXT_DIM`과 반투명(alpha 0.5)으로 그린다. **해제 조건 셋**: 붙여넣기 성공 / 다른 것을 클립보드에 담기(`put` 호출) / 다른 앱이 클립보드를 가져가 `fs::clipboard::take()`의 경로 집합이 달라짐(프레임마다 묻지 않고 **붙여넣기·담기 시점에만** 확인한다) ④ `ui::panel`이 `Rename` 액션을 받아 `fs::file_op::rename_item`을 걸고 결과 채널을 기존 `poll_create`와 같은 자리에서 거둔다 ⑤ **비추상화 선언**: 원격 목록의 이름 바꾸기(대화 — FR-39)와 합치지 않는다. 두 상태를 하나의 「행 상태」 타입으로 묶지 않는다 — 수명과 해제 조건이 전혀 다르다
  - **Acceptance**: Given `report.tar.gz`를 편집 시작, When 첫 프레임에 선택 범위를 잡으면, Then **마지막 점 앞**(`report.tar`)이 선택된다 — 확장자 판정은 마지막 점 기준이며 폴더는 이름 전체가 선택된다(순수 함수 시험). Given 편집 중 `Esc`, When 누르면, Then 이름이 원래대로 돌아가고 목록 선택이 유지된다. Given 경로 2개가 `cut_marks`에 있고, When 붙여넣기가 성공하면, Then 집합이 비고 그 행이 다시 정상 색으로 그려진다(상태 전이 시험 — FR-64의 *"붙여넣거나 다른 것을 담으면 그 표시가 풀린다"*)
  - **Files**:
    - 주: `src/ui/file_list.rs`, `src/ui/list_details.rs`, `src/ui/list_grid.rs`
    - 동반: `src/ui/list_common.rs`(`FileListAction::Rename` 추가), `src/ui/panel.rs`, `src/ui/panel/workers.rs`, `src/i18n/mod.rs`
    - 테스트: `src/ui/panel/tests.rs`, `src/ui/file_list.rs` 내 `mod tests`(선택 범위 산출 · 편집 상태 전이 · 목록 갱신 중 편집 유지 · 잘라내기 표시의 세 해제 조건)
  - **Edge Cases**: 편집 중 폴더가 갱신됨(감시) → 편집을 유지하고 **행 위치는 이름으로 다시 찾는다**(기존 선택 복원과 같은 규칙) / **편집 중 그 행이 뷰포트 밖으로 스크롤됨** → 편집을 유지한다. `list_details.rs:385`가 `if !ui.is_rect_visible(rect) { continue; }`로 보이는 행만 그리므로, 그대로 두면 `TextEdit`이 사라져 포커스가 풀리고 취소로 처리된다(탐색기는 유지한다) — **편집 중인 행만 컬링에서 빼거나 스크롤을 그 행에 고정**한다 / 편집 중 탭·패널 전환 → 취소 / 빈 이름·같은 이름으로 확정 → 조용히 취소(셸에 걸지 않는다) / 이름이 칸보다 긺 → `TextEdit`이 스크롤 / `..` 줄 → 편집 불가 / 여러 개 선택 상태에서 F2 → **첫 항목만** 편집(탐색기와 같다) / 잘라낸 항목이 다른 폴더에 있어 지금 목록에 없음 → 표시할 행이 없을 뿐 집합은 유지 / 잘라낸 뒤 붙여넣지 않고 앱 종료 → 원본은 그대로 남는다(집합은 저장하지 않는다 — 4-B)
  - **Halt Forecast**:
    - (i) 아이콘 보기에서 이름 칸이 2줄일 수 있다 → `list_grid`는 그 셀 크기를 이미 알고 있어 그 사각형을 그대로 쓴다
    - (ii-a) `FileListAction` 열거형 확장(두 렌더러가 쓰는 공개 계약) → `## 사전 승인 항목`에 등록
  - **Depends on**: T2, T3

- [ ] T9. 키를 받는 영역을 정한다
  - **Type**: C
  - **Design**: ① `src/ui/app.rs`에 `#[derive(Clone, Copy, PartialEq)] enum KeyOwner { FileList, Sidebar }` — 기본은 `FileList` ② 파일 목록·사이드바 카드에서 클릭·우클릭이 일어난 프레임에 소유를 그쪽으로 옮긴다 ③ `ui::sidebar`는 소유 값을 인자로 받아 F2 판정에 더한다(사이드바가 `ui::app`을 참조하지 않게 값으로 내려준다). `ui::menu::poll_shortcuts`도 소유를 인자로 받아 파일 대상 키를 거른다 ④ **비추상화 선언**: 일반 포커스 시스템(위젯별 포커스 링·Tab 순회)을 만들지 않는다 — 지금 갈라야 하는 것은 두 영역뿐이다
  - **Acceptance**: Given 사이드바 카드를 누른 뒤, When `F2`를 누르면, Then 워크스페이스 이름 편집이 시작되고 파일 이름 편집은 시작되지 않는다. Given 파일 목록을 누른 뒤, When `F2`를 누르면, Then 그 반대다(소유 전이 규칙의 순수 함수 시험 + HUMAN-VERIFY 5)
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/sidebar.rs`, `src/ui/menu.rs`
    - 동반: `src/ui/panel.rs`, `src/ui/list_details.rs`, `src/ui/list_grid.rs`(클릭 시 소유 전이 신호)
    - 테스트: `src/ui/app.rs` 또는 `src/ui/menu.rs` 내 `mod tests`(전이 규칙)
  - **Edge Cases**: 앱이 막 떴을 때(아무것도 누르지 않음) → `FileList` / 주소창·이름 편집 중 → 종전대로 `egui_wants_keyboard_input()`이 먼저 걸러 단축키가 돌지 않는다 / 사이드바가 접혀 있음 → 소유가 `Sidebar`로 갈 길이 없어 자연히 `FileList` / 전송 큐·설정 대화가 떠 있음 → 모달이 키를 가져간다(기존 동작)
  - **Halt Forecast**:
    - (i) `menu.rs:295-303`의 주석이 「삭제는 키를 배정하지 않았다」고 적고 있다 → 이 task에서 함께 고친다(동반 변경 판정 — 필수)
  - **Depends on**: -

- [ ] T10. 단축키를 명령에 잇는다
  - **Type**: C
  - **Design**: ① `src/ui/menu.rs`(`Command`·`shortcut_table`)·`src/ui/app.rs`(`apply_command`) ② 신규 variant: `Rename` · `Delete { permanent: bool }` · `ClipboardCopy` · `ClipboardCut` · `ClipboardPaste`. `NewFolder`에는 `Ctrl+Shift+N`을 더한다(variant 추가 없음) ③ `shortcut_table`은 **수식 키가 많은 조합을 앞에** 두는 기존 규칙을 지킨다 — `Shift+Delete`가 `Delete`보다 앞 ④ **비추상화 선언**: 사용자 정의 키 매핑·설정 화면 항목을 만들지 않는다(요청에 없다)
  - **Acceptance**: Given `shortcut_table()`, When 조회하면, Then 요청 11종이 모두 있고 `Shift+Delete`가 `Delete`보다 앞선다(단위 시험). Given 아무 선택 없이 `F2`·`Delete`·`Ctrl+C`, When 누르면, Then 아무 일도 일어나지 않는다(대상 없음 — 단위 시험)
  - **Files**:
    - 주: `src/ui/menu.rs`, `src/ui/app.rs`
    - 동반: `src/ui/panel.rs`, `src/ui/tabs.rs`, `src/ui/titlebar.rs`(`Command` match 망라)
    - 테스트: `src/ui/menu.rs` 내 `mod tests`(기존 2건 갱신 + 신규 대응표·순서 시험)
  - **Edge Cases**: 선택 0개에서 파일 대상 키 → 무시 / 선택 여럿에서 `F2` → 첫 항목 / `Ctrl+V`인데 클립보드가 비었거나 파일이 아님 → 무시 / `..` 줄만 선택 → 무시 / 붙여넣을 폴더가 원본과 같음 → 셸이 「- 복사본」을 만든다(탐색기와 같다 — 드래그의 같은 폴더 취소 규칙(FR-60)은 드래그 전용이다)
  - **Halt Forecast**:
    - (i) `Command`가 `Copy`를 파생하고 있어 새 variant도 `Copy`여야 한다 → 신규 variant는 전부 `Copy` 가능한 값만 담는다(`bool`)
  - **Depends on**: T9, T2, T3, T8

- [ ] T11. 원격 탭에서는 원격 기능으로 잇는다
  - **Type**: C
  - **Design**: ① `src/ui/app.rs`(`apply_command`의 분기) ② 활성 탭이 원격이면 `Rename`·`Delete`·`NewFolder`·`Refresh`를 `RemoteMenuAction::{Rename, Delete, NewFolder, Refresh}` 경로로 보내고, `Clipboard*`는 아무 일도 하지 않는다 ③ 기존 `handle_remote_menu_action`(`src/ui/app/remote.rs:129-145`)을 그대로 부른다 — 새 실행 경로를 만들지 않는다 ④ **비추상화 선언**: 로컬·원격 명령을 하나의 추상 「파일 작업」으로 묶지 않는다(대화 방식·비동기 성질이 다르다)
  - **Acceptance**: Given 원격 탭이 활성, When `F2`를 누르면, Then 원격 이름 바꾸기 대화가 뜬다(기존 메뉴 항목과 같은 대화). Given 원격 탭, When `Ctrl+C`, Then 아무 일도 일어나지 않는다(라우팅 판정의 단위 시험 + HUMAN-VERIFY 7)
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/app/remote.rs`
    - 테스트: `src/ui/app/remote.rs` 내 `mod tests`(탭 종류별 라우팅 판정)
  - **Edge Cases**: 연결이 끊긴 원격 탭 → 원격 메뉴와 같은 비활성 규칙(아무 일도 하지 않는다) / 원격 선택 0개에서 `F2`·`Delete` → 무시 / 원격 선택 2개 이상에서 `F2` → 무시(원격 메뉴와 같은 규칙 — 새 이름은 하나뿐)
  - **Halt Forecast**:
    - (i) 원격 메뉴의 활성 규칙이 이미 `menu_rows`에 있다 → 그 판정을 그대로 재사용한다(`remote_menu.rs:118-160`)
  - **Depends on**: T10

- [ ] T12. 문서를 갱신한다
  - **Type**: A
  - **Acceptance**: Given `README.md`, When 42행 셸 메뉴 서술·24행 단축키 서술·88행 아키텍처 절의 소스 트리와 플로우를 읽으면, Then 신규 모듈 4개(`fs::bitmap`·`fs::clipboard`·`fs::shell_menu` 확장·`ui::shell_context_menu`)와 단축키 11종·인라인 이름 편집·잘라내기 표시가 실제 구현과 일치한다. Given `AGENTS.md`의 「DO NOT」 UI 스레드 블로킹 예외 열거, When 읽으면, Then 셸 메뉴 열기·하위 메뉴 채움이 그 예외에 **이름으로 적혀 있다**(D10). Given `AGENTS.md`의 「원격 기능 테스트」 절, When 읽으면, Then 진짜 클립보드를 쓰는 시험을 켜는 환경변수(`MOA_TEST_CLIPBOARD`)가 적혀 있다 — 기본은 꺼짐이라 적어 두지 않으면 그 시험이 있다는 것을 아무도 모른다. Given `docs/plans/deferred.md`, When 「32bpp 중복」(2026-08-21 등재)·「메뉴 보정 실측」(2026-08-20 등재) 두 항목을 보면, Then 해소로 종결 처리돼 `## 대기`에 남지 않는다
  - **Files**:
    - 주: `README.md`, `docs/plans/deferred.md`
    - 동반: `AGENTS.md`(두 곳만 — ⓐ 「DO NOT」의 UI 스레드 예외 열거 ⓑ 「원격 기능 테스트」 절 곁에 **클립보드 시험 스위치**(`MOA_TEST_CLIPBOARD=1`) 한 줄. **절을 늘리지 않는다** — 이 파일은 이미 주입 상한(16KB)을 넘어 21KB이므로 기존 문장·절 안에서 고친다)
  - **Edge Cases**: README에 없던 절을 새로 만들지 않는다(공통 지침) / PRD는 이 회차 시작 시 이미 개정됐으므로 다시 손대지 않는다 / AGENTS.md가 더 커지면 주입 상한 초과가 심해진다 → 예외 열거 문장 안에서만 고쳐 증가를 한 줄 이내로 묶는다(대장의 2026-08-22 「AGENTS.md 주입 상한 초과」 항목은 이번에 해소하지 않는다 — 별건이다)
  - **Halt Forecast**:
    - 없음 — 문서 수정만이고 파괴적·외부 요소가 없다
  - **Depends on**: T11

## 사전 승인 항목 (일괄 승인 대상)

- **T4 — `fs::shell_menu`의 공개 API 확장**: `ShellMenu` 핸들 타입과 읽기·실행 함수를 더한다. 기존 `show_context_menu`·`forward_menu_msg`는 그대로 남아 `추가 옵션 표시` 경로가 된다.
- **T6 — `ui::panel::MenuRequest` 구조체 필드 추가**: 리터럴 생성처가 **3곳**(`panel.rs:1162` · `splitter.rs:423`·`477`)이고 전부 T6 Files에 있다. 컴파일러가 누락을 잡는다.
- **T8 — `ui::list_common::FileListAction` 열거형 확장**: `Rename` variant 추가. 두 렌더러와 `panel`이 이 열거형을 쓰며 `match`가 비망라라 컴파일러가 누락을 잡는다.
- **T10 — `ui::menu::Command` 열거형 확장**: variant 5종 추가와 `shortcut_table` 배열 길이 변경.
- **T1·T3 — 기존 함수를 공용 모듈로 이동**: `fs::drag_image::read_bgra` → `fs::bitmap`, `fs::drag_source`의 데이터 객체 획득 경로 추출. 파일 삭제는 없고 호출부는 각각 1곳이다.
- **신규 파일 3개 생성**: `src/fs/bitmap.rs` · `src/fs/clipboard.rs` · `src/ui/shell_context_menu.rs`.

## 불가피한 Halt (위임 불가)

- **commit·push·태그·GitHub 릴리즈** — 구현·검증이 끝난 뒤 최종 보고에서 따로 승인받는다. 특히 이번 변경은 사용자 눈에 보이는 기능이 크게 늘어 **버전을 올려 릴리즈할지**가 별도 결정이며, 올린다면 AGENTS.md 「릴리즈 발행」 6단계(라이선스 자산 재생성 포함)를 그대로 따라야 한다.
- **plan에 없던 돌발 결정** — 예컨대 owner-draw 항목이 실측에서 대량으로 빠져 설계를 바꿔야 하는 경우.

## Known Workarounds

- **owner-draw 확장 항목** — 이름을 읽을 수 없는 줄은 우리 메뉴에서 빼고 `추가 옵션 표시`의 표준 메뉴에 맡긴다. 근본 해결(그림째 중계)은 비용이 맞지 않아 `## Deferred / Follow-up`에 남겼다.

## Verification Strategy

- 빌드: `cargo build`
- 린트: `cargo clippy --all-targets -- -D warnings` (경고 0)
- 서식: `cargo fmt --check`
- 단위·통합 테스트: `cargo test`
- 수동 검증 (HUMAN-VERIFY — `cargo run --release` 후):
  1. 로컬 파일 하나를 우클릭 → Win11 모양 메뉴가 뜨고 설치된 확장(반디집·PowerRename·Zed 등)이 목록에 보인다
  2. 폴더를 우클릭 → 하위 메뉴(`보내기` 등)를 펼치면 항목이 채워진다
  3. `추가 옵션 표시` → 종전 Windows 표준 메뉴가 뜨고, 1에서 빠진 확장이 있었다면 여기서는 보인다
  4. 빈 영역 우클릭 → 폴더 배경 메뉴(`새로 만들기` 포함)가 Win11 모양으로 뜬다
  5. 파일 목록을 누른 뒤 `F2` → 이름 칸이 그 자리에서 편집되고 확장자 앞부분만 선택돼 있다. 사이드바 카드를 누른 뒤 `F2` → 워크스페이스 이름이 편집된다
  6. `Ctrl+C` → 탐색기에 붙여넣기 / 탐색기에서 `Ctrl+C` → 앱에 `Ctrl+V`. `Ctrl+X` 후 원본이 흐려지고 붙여넣으면 옮겨진다
  7. `Delete`(휴지통 확인) · `Shift+Delete`(영구 삭제 확인 대화) · `Ctrl+Shift+N`(새 폴더). 원격 탭에서 `F2`·`Delete`·`Ctrl+Shift+N`이 원격 대화로 간다

## Phase Ledger

## Retry Ledger

- T2: 리뷰 지적 수정 사이클 1/5 (품질 MINOR 1 + SUGGEST 2 반영 — 구조 변경이라 전량 재리뷰)
- T3: 리뷰 지적 수정 사이클 2/5 · 재호출 2/2(상한 도달) — 1라운드 BLOCKER 1·MAJOR 3·MINOR 3, 2라운드 MAJOR 2. **동일 지적 반복 0** (매 라운드 새 지적이라 3회 연속 조건에 닿지 않았다)

## Progress Log

- **T1-T2 완료** (커밋 `b22be28`, 진행 중): 32bpp 비트맵 읽기를 `fs::bitmap`으로 모으고, `fs::file_op`에 이름 바꾸기·삭제를 더했다. 빌드·clippy 경고 0, 테스트 1027건 통과.
  - **결정**: 결과 타입 이름을 plan의 `OpOutcome` 대신 **`FileOpOutcome`**으로 했다 — `ui::app::remote::OpOutcome`(원격 명령 뒤 재조회 여부를 정하는 열거형)이 이미 있어 뜻이 겹친다. impact-warn hook이 이 충돌을 잡았다.
  - **결정**: `CopyOutcome`과 `FileOpOutcome`을 합치지 않는다(필드는 같다) — 바뀌는 이유가 다르다(드래그 복사 FR-60·61 / 메뉴·단축키 FR-64). 대신 **워커 껍데기**(COM 초기화·해제·송신)는 `spawn_shell_op<T, F>`로 공통화했다(3회 반복 확인 — 품질 리뷰 SUGGEST 수용).
  - **계획 정정**: 전제 검증 8을 8·8-1로 갈랐다 — T3의 `RegisterClipboardFormatW`·`GlobalAlloc`이 아직 켜지지 않은 feature(`Win32_System_DataExchange`·`Win32_System_Memory`)에 있어 「새 feature 없이 가능」이 틀렸다. T3이 두 feature를 켠다(새 crate는 없으므로 라이선스 자산 재생성은 불요).
- **T3 완료** (진행 중): `fs::clipboard` 신규 — 셸 표준 형식(`CF_HDROP` + `Preferred DropEffect`)으로 담고 읽는다. `fs::drag_source`의 데이터 객체 획득을 `data_object`로 뽑아 끌기와 공유.
  - **결정**: 진짜 클립보드를 쓰는 시험은 **환경변수 `MOA_TEST_CLIPBOARD`로 연다**(기본 꺼짐). AGENTS 「원격 기능 테스트」의 실서버 게이트와 같은 관례 — `cargo test`마다 사용자 클립보드를 덮으면 안 된다. 이 회차에 게이트를 켜고 돌려 통과를 확인했다(전체 1020건).
  - **결정**: 그 시험은 **전용 스레드**에서 돈다. COM 아파트는 스레드마다 하나라, 다른 시험이 먼저 그 스레드를 잡으면 `OleInitialize`가 실패한다 — 단독 실행은 통과하는데 전체 실행에서만 깨지는 형태로 **실제 관측됐다**.
  - **결정**: `GetData`가 준 매체의 `tymed`를 `hGlobal`로 읽기 **전에** 검사한다(`is_global_medium`). 클립보드는 외부 입력이라 규격을 지키지 않는 앱이 다른 매체를 줄 수 있고, 그것을 핸들로 오독하면 임의 포인터를 Win32에 넘기게 된다.
  - **T3의 신규 심볼도 미연결**(`put`·`take`·`ClipboardFiles`) — **T8**(잘라내기 표시)과 **T10**(단축키)이 잇는다.
  - **T2의 신규 심볼은 아직 미연결**(`rename_item`·`delete_items`·`invalid_name_reason`·`FileOpOutcome`) — **T8**(인라인 이름 편집)과 **T10**(단축키)이 실행 경로에 잇는다.

## Next Steps

- 권장 다음 액션: 승인 후 `pjc:implement-task`로 T1부터 실행

## Open Questions

- (없음 — 2026-08-22 4라운드로 전부 해소: 메뉴 범위·키 충돌·이름 변경 방식·원격 범위 / PRD 개정 / 동반 변경 채택)
