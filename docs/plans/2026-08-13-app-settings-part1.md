# Plan: 앱 설정 화면 — part1 (설정 모달 · 글꼴 · 자동 실행 · 트레이 · 파일 보기)

**PRD**: docs/prd.md

**다음 plan**: docs/plans/2026-08-13-app-settings-part2.md

## 요구 이해

- **원문 요청**: "설정화면 구현 / 1. 폰트 설정 — os 설치된 폰트 목록을 표시하고 폰트를 선택해서 변경할 수 있음 / 2. 자동 실행 — 윈도우 부팅 후 앱을 자동 실행 함, on/off 토글 버튼으로 ui 구현 / 3. 앱 종료 시 트레이 표시 — 앱을 종료하면 트레이로 전환, on/off 토글, on이면 트레이 아이콘 표시, 더블 클릭하면 메인 화면 실행 이미 실행되어 있는 경우 탑으로 화면을 표시, 컨텍스트 메뉴에 실행·종료 / 4. 파일 보기 — 파일 확장명(off 설정시 확장자 표시하지 않음), 숨김 항목, on/off 토글 / 5. 앱 언어 변경 — 시스템 기본, 한글, 영문"
- **이해한 요구**: 타이틀바 `⚙` 메뉴의 `설정`(지금은 비활성)을 눌러 열리는 모달을 만들고, 그 안에 다섯 그룹(모양·시작·종료·파일 보기·언어)을 단일 스크롤로 세로 배치한다. 값은 바꾸는 즉시 화면에 반영되고 `settings.json`에 저장된다. **이 part는 언어를 뺀 네 기능**을 구현한다 — 글꼴 교체, 부팅 자동 실행, 닫기 시 트레이 전환(아이콘·더블클릭 복원·컨텍스트 메뉴·중복 실행 방지 포함), 확장자·숨김 항목 토글. 확장자·숨김 토글은 로컬·원격 목록 모두에 적용한다.
- **포함하지 않는 것으로 이해**: 언어 전환(part2에서 구현 — 이 part는 설정 값 자리만 만들고 `언어` 그룹은 화면에 두지 않는다), 글꼴 **크기**·굵기 조절, `업데이트`·`릴리즈 노트`·`오픈소스 라이선스`·`정보` 항목의 실기능.

## Goal

타이틀바 설정 메뉴에서 앱 설정 화면을 열어 글꼴·자동 실행·트레이 전환·파일 표시 방식을 바꾸고, 그 값이 즉시 반영되며 재시작 후에도 유지된다.

**전체 목표**: 위에 더해 앱 언어를 시스템 기본·한국어·영어로 전환할 수 있다(part2).

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-47 | Must | T2, T3 | ✅ 커버 (언어 그룹은 part2) |
| FR-48 | Must | T4, T5 | ✅ 커버 |
| FR-49 | Must | T6, T10 | ✅ 커버 |
| FR-50 | Must | T7, T8 | ✅ 커버 |
| FR-51 | Must | T9 | ✅ 커버 |
| FR-52 | Must | T11 | ✅ 커버 |
| FR-13 | Must | T12 | ✅ 커버 |
| FR-53 | Must | (part2 담당) | ⏭️ 다음 part |
| NFR-6 | — | (part2 담당) | ⏭️ 다음 part |
| NFR-7 | — | T1 | ✅ 커버 (설정을 같은 파일에 담는다) |
| FR-1~FR-12, FR-14~FR-46 | Must/Should/Could | (기구현) | 이번 범위 외 (기구현) |
| NFR-1~NFR-5, NFR-8~NFR-13 | — | (기구현) | 이번 범위 외 (기구현) |

> NFR-1(콜드 스타트 1초)·NFR-2(유휴 150MB)는 이번 변경이 시작 경로(뮤텍스·트레이 등록·글꼴 읽기)와 상주 자원(트레이 아이콘·글꼴 바이트)을 건드리므로 T13에서 재측정한다.

## Out of Scope

- 글꼴 **크기**·굵기·자간 조절 — 요청이 "폰트를 선택해서 변경"이라 종류만 다룬다
- 설정 값의 가져오기/내보내기·프로파일
- `업데이트`·`릴리즈 노트`·`오픈소스 라이선스`·`정보` 항목의 실기능 — FR-22대로 표시만 유지
- 트레이 아이콘의 풍선 알림(balloon tip)·전송 진행률 표시
- 설정 화면의 글꼴 미리보기 영역 — 앱 전체가 즉시 바뀌므로 화면 자체가 미리보기다

## Deferred / Follow-up

- **다음 분할 plan**: `docs/plans/2026-08-13-app-settings-part2.md` — T1~T8 (언어 전환 전면 적용, 미실행)
- 트레이 아이콘의 상태 표시(전송 중 배지 등) — FR-36 전송 큐와 이어 붙일 여지가 있으나 이번 요구에 없다
- 설정 화면에 키보드 단축키 배정(`Ctrl+,` 등) — 진입점은 타이틀바 메뉴 하나로 충분하고, 단축키 표를 손대는 작업이 따로 있다(대장의 `사이트 관리자 단축키 Ctrl+S` 항목과 같은 성격)
- **[SUGGEST] 모달 크롬 상수가 두 파일에 같은 값으로 있다** (T3 quality 리뷰 m1) — `SCRIM_ALPHA`(140)·`SHADOW_OFFSET_Y/BLUR/ALPHA`(18/60/153)·`HEADER_HEIGHT`(40)·`FOOTER_HEIGHT`(58)가 `ui/site_manager.rs`와 `ui/settings_dialog.rs`에 소수점까지 같다. 대화 크기 자체는 다르므로(1080×680 vs 420×400) `DIALOG_*`는 해당 없다. 공통화 문턱(2회)에 닿았으나 `site_manager.rs`는 이 task의 Files 밖이라 이번엔 두었다 — 모달을 하나 더 만들거나 사이트 관리자를 손볼 때 `widgets.rs`로 모아 정본을 하나로 한다
- 숨김 항목 토글의 원격 판정을 서버 종류별로 정교화 — 지금은 이름이 `.`으로 시작하는지만 본다. SFTP에서 서버가 주는 속성을 더 볼 여지가 있다

## Investigation Log

- 위키 참조: 관련 위키 자료 없음 — vault(`D:/Personal Project/Obsidian Vault/LLM WIKI`)는 설정돼 있으나 이 프로젝트의 설정 화면·트레이·i18n 관련 페이지가 없어 코드를 1차 출처로 진행
- `docs/plans/deferred.md` `## 대기` 조회 — 이번 작업과 걸리는 항목 2건: ① `[2026-07-23] FR-13 숨김·시스템 파일 표시 토글 (Could)` → **T12로 재수용** ② `[2026-07-29] 설정 팝업 5개 항목의 실제 기능 — v1은 항목 표시만 하고 전부 비활성` → **`설정` 하나만 T2에서 채우고 나머지 4개는 그대로 둔다**(Out of Scope에 명시). 전제 반증 역질의 결과 이 plan의 전제를 부정하는 항목은 없음
- 타이틀바 설정 메뉴는 `src/ui/titlebar.rs:234-250` `show_settings_menu` — 다섯 항목이 `pending_item`(`add_enabled(false, Button)`)으로 비활성이며 반환값이 없다(`fn show_settings_menu(ui: &mut egui::Ui)`)
- `show_titlebar`는 이미 `TitlebarOutcome { command: Option<Command>, window: Option<WindowRequest> }`를 반환한다(`titlebar.rs:65-79`) — 설정 열기는 `Command`(`src/ui/menu.rs:56`)에 variant를 더해 실어 보낼 수 있다
- 공용 폼 위젯이 이미 있다(`src/ui/widgets.rs`) — `form_label:205`·`dropdown_field:302`·`check_row:454`·`radio_row:413`·`design_button:101`·`primary_button:119`와 `FORM_LABEL_WIDTH:184`·`FORM_FIELD_HEIGHT:186`·`FORM_GAP:188`·`FORM_FONT_PX:192`. **on/off 토글 스위치만 없다**
- 모달 골격의 선례는 `src/ui/site_manager.rs:535-560` — `egui::Modal::new(Id)` + `backdrop_color(SCRIM_ALPHA)` + `Frame{fill: SURFACE_BG, stroke: BORDER_CONTROL, corner_radius: 0, shadow}` + `allocate_exact_size(DIALOG_WIDTH, DIALOG_HEIGHT)`로 헤더/본문/푸터를 나눈다
- 숨김 파일 필터가 **아예 없다** — `src/fs/enumerate.rs:143-166` `push_entry`가 `.`·`..`만 거르고 `WIN32_FIND_DATAW.dwFileAttributes`에서 `FILE_ATTRIBUTE_DIRECTORY`만 읽는다. `FileEntry`(`enumerate.rs:20-27`)에 속성 필드가 없어 숨김·시스템 판정을 나중에 할 수 없다
- 표시 이름은 `ListRow::name()`(`src/panel/file_list.rs:521`)이 내주고 `FileEntry`·`RemoteEntry`가 구현한다(`:575`·`:625`). 렌더가 이름을 쓰는 곳은 `list_details.rs:476-477`·`list_grid.rs:256,279,324,340`이고, **`list_details.rs:399`·`list_grid.rs:154,218`은 `dir.join(entry.name())`으로 경로를 만든다** — 이 셋에 확장자 숨김을 적용하면 파일 실행·셸 메뉴가 깨진다
- 종료 경로: 타이틀바 `✕` → `WindowRequest::Close`(`titlebar.rs:200-206`) → `ViewportCommand::Close`(`ui/app.rs:734`) → eframe 종료 → `on_exit`(`ui/app.rs:1932-1937`)가 `runner.shutdown` + `save_session(collect_session())`. **닫기를 가로챌 코드가 없다** — `close_requested()` 읽기·`CancelClose` 사용 0건
- 창 상태(위치·크기·최대화)를 저장하는 지점은 `on_exit` **하나뿐**이다. `persist_session`(`ui/app.rs:1777-1784`)은 사이트 목록이 바뀔 때만 불린다(`:1329`·`:1332`·`:1342`·`:1770`)
- HWND는 `src/ui/shell_host.rs:31-43`에서 한 번 얻어 보관하고(`hwnd():47`), **이미 창 서브클래스가 설치돼 있다**(`SetWindowSubclass(hwnd, shell_menu_proc, SUBCLASS_ID=1, 0)`, 프로시저는 `:91-104`로 셸 메뉴만 처리하고 나머지는 `DefSubclassProc`에 넘긴다)
- 단일 인스턴스 처리 없음 — `CreateMutex`·`already_running` 검색 0건. 트레이 관련 코드도 0건(`Shell_NotifyIcon`·`NOTIFYICONDATA` 없음)
- `install_fonts`(`ui/app.rs:137-166`)는 `C:\Windows\Fonts\malgun.ttf`(`:55` 상수)를 읽어 `Proportional` 맨 앞·`Monospace` 뒤에 넣고 phosphor를 더한 뒤 `ctx.set_fonts(fonts)`를 **한 번** 부른다. 호출부는 `ui/app.rs:402`(앱 시작)와 테스트 5곳(`list_common.rs:254`·`menu.rs:438`·`list_grid.rs:405,489,634`)
- exe에 아이콘 리소스가 이미 담겨 있다 — `build.rs`가 `docs/AppIcon.ico`를 `RT_GROUP_ICON` **ID 1**(`build.rs:34 GROUP_ID`)로 넣는다. 트레이 아이콘은 이 리소스를 `LoadImageW`로 16px 로드해 쓸 수 있다(새 아이콘 파일 불필요)
- 세션 스키마는 v3이고 `promote_v2`로 v2를 승격한다(`src/app/settings.rs:1-8`, `SESSION_VERSION=3`). 종전 확장(열 폭·보기 모드)은 **버전을 올리지 않고 `#[serde(default)]`로** 더했다

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 닫기 요청을 취소하고 창만 숨길 수 있다 | `egui-0.35.0/src/viewport.rs:1080-1097`에 `ViewportCommand::CancelClose`·`Visible(bool)` 존재. 요청 감지는 `egui-0.35.0/src/data/input/viewport_info.rs:111 close_requested()` | ✅ |
| 2 | 트레이 아이콘 API를 새 의존성 없이 쓸 수 있다 | `windows-0.62.2` `UI/Shell/mod.rs:4450 Shell_NotifyIconW`·`:52111 NOTIFYICONDATAW`. feature `Win32_UI_Shell`은 `Cargo.toml`에 이미 켜져 있다 | ✅ |
| 3 | 트레이 우클릭 메뉴를 만들 수 있다 | 같은 crate `UI/WindowsAndMessaging/mod.rs:405 CreatePopupMenu`·`:35 AppendMenuW`·`:2377 TrackPopupMenu`·`:2142 SetForegroundWindow`. feature `Win32_UI_WindowsAndMessaging` 활성 | ✅ |
| 4 | 자동 실행을 관리자 권한 없이 등록할 수 있다 | 같은 crate `System/Registry/mod.rs:84 RegCreateKeyExW`·`:626 RegSetValueExW`·`:211 RegDeleteValueW`·`:489 RegQueryValueExW`. feature `Win32_System_Registry`가 이미 켜져 있다(현재는 구조체 정의 용도로만 쓰인다는 주석 — `Cargo.toml`) | ✅ |
| 5 | 설치된 글꼴 중 **한글 지원분만** 열거할 수 있다 | 같은 crate `Graphics/Gdi/mod.rs:614 EnumFontFamiliesExW`·`:5653 LOGFONTW`·`:4569 ENUMLOGFONTEXW`. `LOGFONTW.lfCharSet`에 `HANGUL_CHARSET`을 넣으면 그 문자셋을 가진 글꼴만 콜백된다. feature `Win32_Graphics_Gdi` 활성 | ✅ |
| 6 | ~~글꼴 **이름만 알아도 바이트를 얻을 수 있다**~~ **→ 모음 글꼴에서 거짓 (2026-08-13 실측)** | 같은 crate `Graphics/Gdi/mod.rs:207 CreateCompatibleDC`·`:309 CreateFontIndirectW`·`:1756 SelectObject`·`:1000 GetFontData`. `GetFontData(hdc, 0, 0, None, 0)`로 크기를 얻고 다시 불러 버퍼를 채우면 TTF/OTF 원본 바이트가 나온다 — 레지스트리에서 파일 경로를 뒤질 필요가 없다. GDI가 준 바이트는 **단일 글꼴에서만** 유효하다 — 모음 글꼴(TTC)에서는 40바이트 어긋난 데이터가 와 파서가 읽지 못한다. 그래서 D3을 C(파일 직접 읽기)로 바꿨다 | ❌ 실측으로 부정됨 |
| 7 | 글꼴을 런타임에 바꿀 수 있다 | `egui-0.35.0/src/context.rs:2038 set_fonts` — `&self`라 프레임 중에도 부를 수 있고, 기존 주석("두 번 부르면 뒤엣것이 앞엣것을 덮어쓴다", `ui/app.rs:135`)대로 **덮어쓰기가 곧 교체 수단**이다. **단 반영은 다음 pass부터다** — 같은 파일 `:2036` 문서주석이 "The new fonts will become active at the start of the next pass"라고 못박고, 구현(`:2049-2051`)도 `mem.new_font_definitions`에 넣어 둘 뿐이다 | ✅ (반영 시점 = 다음 프레임) |
| 8 | 단일 인스턴스를 이름 있는 뮤텍스로 판정할 수 있다 | 같은 crate `System/Threading/mod.rs:301 CreateMutexW`. feature `Win32_System_Threading` 활성(변경 감시가 이미 쓴다) | ✅ |
| 9 | 트레이 콜백 메시지를 받을 창 프로시저가 이미 있다 | `src/ui/shell_host.rs:91-104` `shell_menu_proc`가 서브클래스로 설치돼 있고 미처리 메시지를 `DefSubclassProc`에 넘긴다 — 메시지 번호를 하나 더 처리하면 된다. **번호는 `WM_APP+2` 이상을 쓴다** — `WM_APP+1`은 `fs/enumerate.rs:16 WM_APP_ENUM_DONE`이 이미 쓴다 | ✅ |
| 9-a | 숨김 상태에서 **프레임이 도는지 코드만으로 단정할 수 없다** | `eframe-0.35.0/src/native/glow_integration.rs:571-580`이 `run_ui = is_visible \|\| …`로 UI 콜백을 가르고 `is_visible = viewport.info.visible().unwrap_or(true)`다. 그런데 `visible()`(`egui .../viewport_info.rs:95-100`)은 **`(minimized, occluded)` 파생값**이고, `egui-winit .../lib.rs:1363`은 `minimized`만 채우며 **winit Windows 백엔드는 `Occluded` 이벤트를 아예 보내지 않는다**(`winit-0.30.13/src/platform_impl/windows/*.rs`에 `Occluded` 0건). 따라서 `occluded = None` → `visible() = None` → `unwrap_or(true)`로 **Windows에서는 `run_ui`가 참일 공산이 크다** | ⚠ 미확인(실측 필요) — **그래서 복원을 프레임에 기대지 않는다**(D5) |
| 9-c | 창을 숨기면 winit의 **가시성 캐시**가 false로 남는다 | `winit-0.30.13/src/platform_impl/windows/window_state.rs:90 WindowFlags::VISIBLE`·`:392-395` — `apply_diff` 끝에서 `if !new.contains(VISIBLE) { ShowWindow(window, SW_HIDE) }`를 **조건 없이** 실행한다. 반면 `window.rs:147-149 is_visible()`은 `IsWindowVisible` 직조회라 캐시와 어긋날 수 있다 | ✅ (그래서 복원 후 `Visible(true)`로 캐시를 맞춘다 — D5) |
| 9-b | `shell_menu_proc`가 상태를 들고 있지 않다 | `src/ui/shell_host.rs:31-43`이 `SetWindowSubclass(.., SUBCLASS_ID, 0)`로 참조 데이터 `0`을 넘기고, `:37-38` 주석대로 **해제 경로가 없다**(`ShellHost`에 `Drop` 미구현) | ✅ — 그래서 `dwRefData`에 원시 포인터를 싣지 않고 `static OnceLock`을 쓴다(T7 Design ③) |
| 10 | 세션에 필드를 더해도 기존 파일이 버려지지 않는다 | `src/app/settings.rs`의 `parse_session`은 `version != SESSION_VERSION`이면 통째로 폴백하지만, **버전을 유지한 채 `#[serde(default)]` 필드를 더하면** 기존 v3 파일이 그대로 읽힌다(열 폭·보기 모드가 그렇게 들어갔다) | ✅ |
| 11 | exe 아이콘 리소스를 트레이가 쓸 수 있다 | `build.rs:34 GROUP_ID=1`·`:83-88`이 `RT_ICON`(1..n)과 `RT_GROUP_ICON`(1)을 넣는다. `LoadImageW(hinst, PCWSTR(1), IMAGE_ICON, 16, 16, LR_DEFAULTCOLOR)`로 16px을 얻는다 | ✅ |
| 12 | 원격 항목에는 Windows 파일 속성이 없다 | `src/remote/types.rs:285-298 RemoteEntry`에 `mode: Option<u32>`(POSIX)·`owner`만 있고 `dwFileAttributes`류가 없다 — 원격 숨김 판정은 **이름이 `.`으로 시작하는지**로 한다 | ✅ |
| 13 | 확장자를 뗀 이름이 경로·정렬·선택을 오염시키지 않는다 | 경로 조립은 `list_details.rs:399`·`list_grid.rs:154,218`, 정렬 키는 `ListRow::name_sort_key`(`panel/file_list.rs:527`), 선택 복원은 `ui/file_list.rs:233,470`이 각각 **원본 이름**을 쓴다 — 표시 함수를 따로 두면 이 셋이 영향받지 않는다 | ✅ |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| TTC(글꼴 모음)를 제대로 다루지 못하면 **굴림·굴림체·돋움·돋움체·바탕·궁서가 목록에서 사라진다** | 선택지가 맑은 고딕 계열만 남아 FR-48이 사실상 무의미해진다 | **T4에서 한 번 "위험 없음"으로 오판했다가 T5에서 되살아난 위험이다.** GDI가 모음에서 매직만 단일 sfnt로 바꾼 **40바이트 부족한 깨진 데이터**를 주기 때문이다(실측 표는 `## Next Steps`의 「T4·T5 실측 기록」). 해소: 글꼴 파일을 직접 읽고 face 인덱스를 `egui::FontData.index`에 넣는다(D3 A→C) |
| **GDI 폰트 매퍼가 없는 글꼴 이름을 조용히 대체한다** (실측: `없는글꼴이름XYZ` → `굴림`, 크기까지 동일) | 저장된 글꼴이 삭제된 뒤 **엉뚱한 글꼴이 성공처럼 적용**되고 FR-48의 폴백이 무증상으로 죽는다 | `load_font`가 `SelectObject` 뒤 `GetTextFaceW`로 실제 선택된 이름을 대조하고 다르면 `None`(T4 Design ⓐ). 이 경로를 검증하는 시험이 T4 Acceptance ③이다 |
| 숨긴 동안 eframe이 프레임을 도는지 **코드만으로 단정할 수 없다**(전제 9-a) | 프레임 폴링에 기대면 더블클릭해도 창이 안 돌아와 작업 관리자로 죽여야 한다 | **폴링에 기대지 않는다** — 창 복원은 창 프로시저가 `ShowWindow`+`SetForegroundWindow`로 직접 하고, 앱은 사후 통지(`TrayEvent::Shown`)만 받는다(D5). 이 설계는 프레임이 돌든 안 돌든 작동한다. T7·T8 Acceptance에 "숨긴 상태에서 더블클릭 → 복원"을 넣어 실측한다 |
| 프로시저의 `ShowWindow`와 winit의 가시성 캐시가 어긋난다(전제 9-c) | 복원 뒤 최대화 등 창 조작 때 `apply_diff`가 `SW_HIDE`를 재적용해 **창이 갑자기 사라진다** | 복원 통지를 받은 프레임에 `ViewportCommand::Visible(true)`를 한 번 보내 캐시를 맞춘다(T8 Design ③). 이미 보이는 창에 대한 멱등 호출이라 부작용이 없다 |
| 뮤텍스로 중복을 막을 때 기존 창을 **찾는** 수단이 없음(창 제목은 사용자가 바꾸는 워크스페이스 이름이 아니라 "MOA" 고정이지만 다른 앱과 겹칠 수 있다) | 두 번째 프로세스가 조용히 죽기만 하고 창이 안 뜸 | `RegisterWindowMessageW`로 앱 고유 메시지를 만들어 `HWND_BROADCAST`로 쏘고, 기존 프로세스의 서브클래스가 그것을 받아 자기 창을 띄운다(창 찾기 불필요). T9 Design에 확정 |
| 시작 경로에 뮤텍스·트레이·글꼴 읽기가 더해져 콜드 스타트가 늘어남 | NFR-1(1초) 위반 | 대장 기록상 현재 0.4~0.6초이고 대부분이 glutin Display 생성(260~420ms)이다. 추가분은 뮤텍스(µs)·글꼴 읽기(맑은 고딕 12.84MB를 이미 36ms에 읽는다)라 여유가 있다. T13에서 실측 |
| `install_fonts` 시그니처 변경이 호출부 6곳(테스트 5 + 앱 시작 1)을 깨뜨림 | 빌드 실패 | T5 Files에 6곳을 모두 적었다. 기본 인자(`None`)로 부르면 현행과 같게 동작하도록 설계한다 |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `Session`(구조체) | `src/app/settings.rs`(정의·`parse_session`·`promote_v2`), `src/ui/app.rs:514 collect_session`, `src/ui/session.rs`(왕복 테스트) | 필드 추가(`settings`, `#[serde(default)]`) — 버전 불변 |
| `install_fonts` | `src/ui/app.rs:137`(정의)·`:402`(호출), `src/ui/list_common.rs:254`, `src/ui/menu.rs:438`, `src/ui/list_grid.rs:405,489,634` | 시그니처에 글꼴 이름 인자 추가 — **호출부 6곳** 전부 갱신 |
| `FileEntry`(구조체) | `src/fs/enumerate.rs:20`(정의), `src/panel/file_list.rs:575 impl ListRow` | 필드 추가(`attributes: u32`) — **생성 지점 8곳 전부** 갱신: 프로덕션 2곳(`fs/enumerate.rs:153 push_entry`, **`ui/panel.rs:1381 with_local_parent_first`의 `..` 항목**)과 테스트 6곳(`fs/enumerate.rs:305`, `panel/file_list.rs:769`, `ui/file_list.rs:630`, `ui/list_grid.rs:392`, `ui/panel/tests.rs:313`, `ui/tree.rs:372`) |
| `ListRow`(트레이트) | `src/panel/file_list.rs:520`(정의)·`:575 FileEntry`·`:625 RemoteEntry` | 메서드 추가(`display_name(show_extensions)`·`is_hidden()`) — 구현체 2개 모두 |
| `Command`(enum) | `src/ui/menu.rs:56`(정의), `src/ui/titlebar.rs:10`(import)·`:234 show_settings_menu`, `src/ui/app.rs:1384`(전수 match — 갈래를 더할 유일한 지점) | variant 추가(`OpenAppSettings`) |
| `TitlebarOutcome` | `src/ui/titlebar.rs:65`, `src/ui/app.rs:726` | 변경 없음(기존 `command` 필드를 그대로 쓴다) |
| `shell_menu_proc` | `src/ui/shell_host.rs:91` | 트레이 콜백 메시지 갈래 추가 |
| `on_exit` | `src/ui/app.rs:1932` | 트레이 숨김 경로에서도 세션이 저장되도록 저장 지점 추가(T8) |

### 4-B. 계약·직렬화 변경

- `settings.json`에 `settings` 객체 추가. **스키마 버전은 v3 유지** — `#[serde(default)]`라 기존 v3 파일은 그대로 읽히고, 새 필드는 기본값이 된다(전제 10). 마이그레이션 불필요
- `Session`·`AppSettings`는 앱 내부 타입이라 외부 계약 없음

### 4-C. 영향 받는 테스트

- `src/ui/session.rs` — 세션 왕복·v2 승격 테스트. `AppSettings` 기본값 왕복 케이스 추가(T1)
- `src/ui/list_common.rs:254`, `src/ui/menu.rs:438`, `src/ui/list_grid.rs:405,489,634` — `install_fonts` 호출부(T5)
- `src/fs/enumerate.rs` `mod tests` — `FileEntry` 생성 헬퍼(T12)
- `src/panel/file_list.rs:895-912` — `ListRow` 트레이트 테스트(T11·T12)

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `widgets::toggle_row` | `check_row:454`(체크박스)·`radio_row:413`(라디오)만 있고 on/off 스위치는 없음 | **신규** — 요구가 "on/off 토글 버튼"이고 체크박스로 대체하면 요구 문구와 어긋난다. 기존 `check_row`의 배치·색 규약을 그대로 따른다 |
| `SettingsDialog`(모달) | `SiteManager`(`site_manager.rs:535`)가 같은 `egui::Modal` 골격 | **신규 타입, 골격은 재사용** — 사이트 관리자는 목록·초안 편집·3탭을 가진 다른 화면이라 확장할 수 없다. `Modal`+`Frame`+헤더/본문/푸터 구성과 `widgets::*`는 그대로 쓴다 |
| `app::fonts::installed_korean_fonts` / `load_font`(+`LoadedFont`) | `install_fonts`가 경로 상수 하나를 읽을 뿐 열거·추출 기능 없음 | **신규** — 기존에 없다 |
| `app::autostart::{is_enabled, set_enabled}` | 레지스트리 접근 코드 0건(feature만 켜져 있음) | **신규** |
| `ui::tray::Tray` | 트레이 코드 0건 | **신규** |
| `app::single_instance::acquire` | `CreateMutex` 검색 0건 | **신규** |
| `ListRow::display_name` / `is_hidden` | `ListRow::name`·`extension`이 있으나 표시 전용 이름·숨김 판정은 없음 | **신규 메서드** — 트레이트에 두어야 로컬·원격이 같은 규칙을 쓴다(기존 `name_sort_key`가 같은 이유로 트레이트에 있다) |

### Verified by

- `grep -rn "install_fonts" src/` → **7 hits (정의 1 + 호출 6)**, 모두 위 표에 포함
- `grep -rn "ListRow" src/ --include=*.rs` → 정의 1 + 구현체 2 + 사용 8, 구현체 2개 모두 위 표에 포함
- `grep -rn "\.name()" src/ui/list_details.rs src/ui/list_grid.rs` → 9 hits, 표시용 6곳·경로용 3곳으로 분류 완료(Investigation Log)
- `grep -rn "FileEntry {" src/` → 16 hits 중 **구조체 리터럴 생성 8곳**(프로덕션 2 — `fs/enumerate.rs:153`·`ui/panel.rs:1381`, 테스트 6 — `fs/enumerate.rs:305`·`panel/file_list.rs:769`·`ui/file_list.rs:630`·`ui/list_grid.rs:392`·`ui/panel/tests.rs:313`·`ui/tree.rs:372`), 나머지는 정의·`impl`·함수 시그니처. 생성 8곳 모두 T12 Files에 포함

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md` — FR-47~53 추가·FR-13 승격·NFR-6 개정·Out of Scope 재한정 | 요구가 PRD의 "다국어 리소스 제외"·NFR-6 "한국어 고정"과 정면으로 어긋난다. 고치지 않으면 Phase G 재검증의 기준 자체가 틀린다 | **계획 단계에서 이미 반영**(승인 대상) |
| 필수 | `README.md` — 설정 화면·네 기능 추가 | 기능·UI 추가라 문서 갱신 기준에 해당한다 | T13에 편입 |
| 필수 | `AGENTS.md` — "설정은 `%APPDATA%\MOA\settings.json` 로컬 파일 — 스키마 v3" 서술 | 앱 설정이 같은 파일에 들어가지만 **버전은 v3 그대로**라 이 문장이 여전히 맞다. 다만 담기는 것이 세션뿐이 아니게 되므로 한 줄 보강이 필요하다 | T13에 편입 |
| 무관 | `docs/design/README.md`·`*.dc.html` | 설정 화면 디자인이 없다(`README.md:121`은 FTP 화면에서 설정 항목을 뺐다는 서술). 이번 변경이 FTP 화면 규격을 바꾸지 않는다 | 건드리지 않음 |
| 무관 | `Cargo.toml` | 필요한 Win32 feature 5종이 모두 이미 켜져 있다(전제 2~5·8). 새 의존성 0건 | 건드리지 않음 |
| 무관 | `build.rs` | 트레이가 기존 아이콘 리소스를 재사용한다(전제 11) | 건드리지 않음 |

## Decisions

### D1. 설정 값을 어디에 담는가
- **Options**: A) `settings.json`의 `Session`에 `settings` 필드 / B) 별도 `config.json`
- **Chosen**: A
- **Rationale**: NFR-7이 "설정·세션은 `%APPDATA%\MOA\settings.json`"으로 이미 규정한다. 파일을 나누면 저장 시점이 둘이 되고 트레이 숨김 경로에서 한쪽만 저장되는 사고가 생긴다.
- **Source**: `docs/prd.md` NFR-7, `src/app/settings.rs:328 save_session`

### D2. 세션 스키마 버전을 올리는가
- **Options**: A) v3 유지 + `#[serde(default)]` / B) v4로 올리고 `promote_v3` 작성
- **Chosen**: A
- **Rationale**: `parse_session`은 버전이 다르면 **통째로 폴백**한다(워크스페이스·탭이 전부 날아간다). 필드 추가만으로는 기존 파일을 읽는 데 문제가 없어 종전 확장(열 폭·보기 모드)도 버전을 올리지 않았다. 올리면 승격 코드를 쓰는 값 없는 비용만 든다.
- **Source**: `src/app/settings.rs:1-8`(모듈 주석), 전제 10

### D3. 글꼴 바이트를 어떻게 얻는가 — **2026-08-13 실측으로 A → C 변경 (사용자 승인)**
- **Options**: A) `GetFontData`(GDI에 글꼴을 선택하고 원본 테이블을 뽑음) / B) 레지스트리 `HKLM\...\Fonts`에서 이름→파일명을 찾아 읽음 / C) 글꼴 폴더를 훑어 파일 안의 `name` 테이블에서 이름을 읽음
- **Chosen**: **C** (처음엔 A였다)
- **Rationale**: **A는 모음 글꼴(TTC)에서 깨진 데이터를 준다** — 굴림은 파일이 13,533,424바이트인데 `GetFontData`는 13,533,384바이트(40바이트 적다)를 주고, 헤더만 단일 글꼴 모양으로 바꾸면서 내부 테이블 오프셋은 원본 기준으로 남겨 글꼴 파서가 읽지 못한다. 그 결과 **굴림·굴림체·돋움·돋움체·바탕·궁서가 전부 목록에서 빠졌다**(93개 중 58개만 남음). 매직 넘버(`0x00010000`)만 보고 파싱까지 확인하지 않은 것이 T4의 오판이었다.
  **B는 여전히 불가**하다 — 레지스트리 값 이름이 실제로 **영문**(`Gulim & GulimChe & Dotum & DotumChe (TrueType)` → `gulim.ttc`)인데 화면에 보여야 하는 이름은 한글(`굴림`)이라 짝지을 수 없다(실측 확인).
  **C를 택한다**: 글꼴 폴더(시스템 + 사용자)를 훑어 각 파일의 `name` 테이블에서 그 글꼴이 스스로 밝히는 한국어 이름(languageID 0x0412, 없으면 영어 0x0409)을 읽는다. 모음 글꼴은 face 인덱스를 함께 얻어 `egui::FontData.index`에 넣는다. 파일마다 이름표만 seek해 읽으므로 전체를 메모리에 올리지 않는다. **한글 지원 판정은 GDI 열거(`HANGUL_CHARSET`, 1ms)에 그대로 맡긴다** — OS가 이미 판정한 것을 다시 계산할 이유가 없다.
- **Source**: 2026-08-13 실측(파일 크기 대조·레지스트리 값 확인), 사용자 승인

### D4. 닫기를 어떻게 가로채는가
- **Options**: A) `close_requested()` 감지 → `CancelClose` + `Visible(false)` / B) 타이틀바 `✕` 핸들러에서 `ViewportCommand::Close`를 안 보내고 숨김만
- **Chosen**: A
- **Rationale**: B는 타이틀바 버튼만 막는다 — `Alt+F4`·작업 표시줄 닫기·시스템 메뉴로 들어오는 종료를 놓친다. A는 경로가 하나로 모인다.
- **Source**: 전제 1, `src/ui/app.rs:726-738`

### D5. 트레이 이벤트를 UI로 어떻게 나르는가 — **창을 되살리는 경로는 프레임을 거치지 않는다**
- **Options**: A) 프로시저가 채널로 보내고 `ctx.request_repaint()`로 깨워 **프레임에서** 처리 / B) 전역 `AtomicU8` 플래그 / C) **창 표시는 프로시저가 Win32로 직접 하고**, 그 밖의 이벤트만 채널로 프레임에 넘김
- **Chosen**: C
- **Rationale**: **A는 숨김 상태에서 돈다는 보장이 없다.** eframe은 `run_ui = is_visible || …`로 UI 콜백을 가르는데(`glow_integration.rs:571-580`), 그 `is_visible`이 참이 될지는 winit/egui 내부 구현에 달려 있다(전제 9-a — Windows에서는 참일 공산이 크지만 `occluded`를 채우지 않는 현재 구현에 기댄 결과이고, 숨은 창이 이벤트 루프를 깨우는지는 별개 문제다). **A를 택하면 이 두 가지가 모두 참이어야만 창을 되살릴 수 있고, 하나라도 어긋나면 사용자는 작업 관리자로 앱을 죽여야 한다**(FR-50 ③이 통째로 죽는다). C는 **프레임이 돌든 안 돌든 작동한다** — 창 프로시저는 eframe의 프레임 루프와 무관하게 메시지를 받으므로, A가 성립하는 경우까지 포함하는 상위집합이다.
  **복원 절차(프로시저 안에서)**: ⓐ `IsIconic`이면 `ShowWindow(SW_RESTORE)`, 아니면 `ShowWindow(SW_SHOW)` ⓑ `SetForegroundWindow` ⓒ `TrayEvent::Shown`을 채널로 보내고 `ctx.request_repaint()`.
  **ⓒ가 필요한 이유가 둘이다.** ① 앱이 `hidden`을 내릴 신호가 그것뿐이다 — `ctx.input(|i| i.viewport().visible())`로 파생시키는 방법은 **Windows에서 쓸 수 없다**(전제 9-a: `visible()`이 늘 `None`이라 숨김·표시를 구분하지 못한다). ② 앱이 그 프레임에 **`ViewportCommand::Visible(true)`를 한 번 보내 winit의 가시성 캐시를 맞춰야 한다**(전제 9-c) — 프로시저의 `ShowWindow`는 OS 창만 바꾸고 winit 캐시는 `VISIBLE=false`로 남는데, 그 뒤 winit을 거치는 창 플래그 변경이 한 번이라도 일어나면(`app.rs:670`의 `Maximized(true)`, 사용자의 최대화 버튼 등) `apply_diff`가 **`SW_HIDE`를 다시 적용해 창이 갑자기 사라진다**. 이미 보이는 창에 대한 멱등 호출이라 부작용은 없다.
  메뉴 `종료`도 같은 복원 절차를 먼저 거친 뒤 `TrayEvent::Quit`을 보낸다 — 그래야 프레임이 돌아 정상 종료 절차와 세션 저장을 지난다.
  C가 "창 프로시저에서 egui 상태를 만진다"는 우려에 해당하지 않는 이유: `ShowWindow`·`SetForegroundWindow`는 **OS 창 상태**만 바꾸고 egui 자료구조를 건드리지 않는다.
- **Source**: `eframe-0.35.0/src/native/glow_integration.rs:571-580`, `egui .../data/input/viewport_info.rs:95-100`, `egui-winit .../lib.rs:1363`·`:1778`, `winit-0.30.13/src/platform_impl/windows/window_state.rs:90,392-395`·`window.rs:147-149`(전부 직접 확인), `windows-0.62.2` `UI/WindowsAndMessaging/mod.rs:2342 ShowWindow`·`:2142 SetForegroundWindow`·`:6421 SW_SHOW`·`:6409 SW_HIDE`·`:6419 SW_RESTORE`, `src/ui/shell_host.rs:91-104`

### D6. 중복 실행 시 기존 창을 어떻게 부르는가
- **Options**: A) 이름 있는 뮤텍스로 판정 + `RegisterWindowMessageW` 브로드캐스트로 기존 창 호출 / B) `FindWindowW`로 창 제목을 찾아 메시지 전송 / C) 파일 락 + 감시
- **Chosen**: A
- **Rationale**: B는 창이 숨겨져 있어도 `FindWindowW`로 찾히지만 제목("MOA")이 다른 앱과 겹칠 수 있고, 커스텀 타이틀바라 클래스 이름도 winit 내부값이라 안정적이지 않다. A는 등록 메시지가 시스템 전역에서 고유해 오인이 없다.
- **Source**: 전제 8, `windows-0.62.2` `UI/WindowsAndMessaging`

### D7. 확장자 숨김을 어디에 적용하는가
- **Options**: A) `ListRow`에 `display_name(show_extensions)`를 더해 렌더 6곳만 바꿈 / B) `ListRow::name()` 자체를 바꿈 / C) 열거 단계에서 이름을 잘라 저장
- **Chosen**: A
- **Rationale**: B·C는 경로 조립(`dir.join(name)`)·정렬 키·선택 복원까지 잘린 이름을 쓰게 되어 파일 실행과 셸 메뉴가 깨진다(전제 13). A는 표시 경로에만 닿는다.
- **Source**: 전제 13, `src/ui/list_details.rs:399`·`list_grid.rs:154,218`

### D8. 숨김 판정 기준
- **Options**: A) 로컬은 `FILE_ATTRIBUTE_HIDDEN|SYSTEM`, 원격은 이름이 `.`으로 시작 / B) 로컬만 적용 / C) 원격도 POSIX `mode`로 판정
- **Chosen**: A
- **Rationale**: C는 POSIX에 "숨김" 비트가 없다 — Unix에서 숨김은 이름 관례다. 사용자 확정 사항이기도 하다.
- **Source**: 전제 12, 사용자 2·3라운드 답변

### D9. 자동 실행으로 시작했음을 어떻게 아는가
- **Options**: A) Run 키에 등록하는 명령줄에 `--tray` 인자를 붙이고 `std::env::args()`로 판정 / B) 부팅 후 경과 시간 추정 / C) 별도 플래그 파일
- **Chosen**: A
- **Rationale**: 명시적이고 되돌리기 쉽다. B는 추정이라 틀린다. C는 정리 책임이 생긴다.
- **Source**: FR-49

### D10. 토글 위젯의 형태
- **Options**: A) `widgets.rs`에 `toggle_row`(스위치) 신규 / B) 기존 `check_row` 재사용
- **Chosen**: A
- **Rationale**: 요구 원문이 "on/off 토글 버튼"이다. 다만 색·크기·라벨 배치는 `check_row`의 규약(`FORM_FIELD_HEIGHT`·`FORM_FONT_PX`·`theme::TEXT`)을 그대로 따라 화면이 갈리지 않게 한다.
- **Source**: 사용자 요청 원문, `src/ui/widgets.rs:454`

### D11. 설정 화면을 여는 신호
- **Options**: A) `Command`에 `OpenAppSettings` 추가(기존 `TitlebarOutcome.command` 경유) / B) `TitlebarOutcome`에 필드 추가 / C) 전역 상태
- **Chosen**: A
- **Rationale**: 타이틀바는 이미 `Command`를 실어 보내는 통로를 갖고 있고(`TitlebarOutcome.command`), 앱이 그것을 한 곳에서 처리한다. 필드를 늘리면 같은 일을 하는 경로가 둘이 된다. **이름을 `OpenSettings`가 아니라 `OpenAppSettings`로 하는 이유**: 이 코드베이스에는 이미 `RemoteAction::OpenSettings`(`ui/panel.rs:129`)와 `FailedAction::OpenSettings`(`ui/remote_states.rs:113`)가 있고 **둘 다 "사이트 관리자 열기"를 뜻한다**. 컴파일 충돌은 없지만 같은 파일군에서 같은 이름이 다른 화면을 가리키면 읽는 쪽이 매번 어느 것인지 되짚어야 한다.
- **Source**: `src/ui/titlebar.rs:65-68`, `src/ui/menu.rs:56`, `src/ui/panel.rs:129`, `src/ui/remote_states.rs:113`

## Tasks

<!-- T1~T3 (설정 기반·화면 뼈대) -->

- [x] **T1. 앱 설정 모델과 세션 저장·복원**
  - **Type**: C
  - **Design**: ① `src/app/settings.rs`에 둔다(세션 직렬화가 이미 사는 곳). ② 신규 심볼 — `AppSettings`(앱 전역 설정 한 벌), `LanguageSetting`(`System`/`Korean`/`English` — **이 part에서는 저장만 하고 화면에 노출하지 않는다**, part2가 쓴다). ③ `Session`이 `AppSettings`를 소유하고, `ui::app`이 읽어 각 기능에 나눠 준다 — `settings`는 `ui`를 모른다. ④ 이번에 추상화하지 않을 것: 설정 항목별 트레이트·옵저버·변경 이벤트 버스를 두지 않는다(값 7개를 매 프레임 읽는 것으로 충분하다).
  - **Acceptance**: Given 필드가 없는 기존 v3 `settings.json`, When 앱이 읽으면, Then 폴백 없이 세션이 복원되고 `AppSettings`는 기본값(`font_family: None`, `auto_start: false`, `tray_on_close: false`, `show_extensions: true`, `show_hidden: true`, `language: System`)이 된다. 값을 채워 저장한 뒤 다시 읽으면 왕복이 일치한다. `SESSION_VERSION`은 3 그대로다.
  - **Files**:
    - 주: `src/app/settings.rs`
    - 동반: `src/ui/app.rs`(`collect_session:514` — `AppSettings`를 실어 보냄)
    - 테스트: `src/ui/session.rs`(왕복 + "필드 없는 v3 파일" 케이스)
  - **Edge Cases**: 알 수 없는 `language` 문자열 → 기본값으로 폴백(`#[serde(other)]` 또는 `Option` + `unwrap_or_default`) / `font_family`가 빈 문자열 → `None`과 같게 취급 / 손상된 `settings` 객체 → 그 객체만 기본값이 되고 세션 전체는 살아남아야 한다
  - **Halt Forecast**:
    - (i) "버전을 올려야 하는가" → D2에서 확정(올리지 않는다)
  - **Depends on**: -

- [x] **T2. 토글 스위치 위젯**
  - **Type**: C
  - **Design**: ① `src/ui/widgets.rs`에 더한다(폼 부품이 모여 있는 곳). ② 신규 심볼 — `toggle_row(ui, label, on) -> bool`(라벨 + 우측 정렬 스위치 한 줄, 눌리면 `true`). ③ `theme`·기존 `FORM_*` 상수를 참조하고, 설정 화면이 이것을 쓴다. ④ 이번에 추상화하지 않을 것: 애니메이션 상태 머신·제네릭 값 바인딩을 두지 않는다(`check_row`와 같이 즉시 값·즉시 반환).
  - **Acceptance**: Given `toggle_row(ui, "라벨", false)`, When 스위치를 클릭하면, Then `true`를 반환한다. 높이는 `FORM_FIELD_HEIGHT`, 글꼴 크기는 `FORM_FONT_PX`, 글자색은 `theme::TEXT`로 `check_row`와 일치한다. 아이콘 글꼴 규약 테스트(`화면_코드에_원본_아이콘_기호가_남아_있지_않다`)가 그대로 통과한다.
  - **Files**:
    - 주: `src/ui/widgets.rs`
    - 테스트: `src/ui/widgets.rs`(`mod tests` — 반환값·크기 계산)
  - **Edge Cases**: 비활성 상태 표현(이번엔 전 항목이 항상 활성이므로 `enabled` 인자를 두지 않는다 — 필요해지면 그때 더한다) / 라벨이 폭을 넘칠 때 말줄임
  - **Halt Forecast**:
    - (i) "체크박스로 대체할까" → D10에서 확정(신규 토글)
  - **Depends on**: -

- [x] **T3. 설정 모달 뼈대와 타이틀바 진입점**
  - **Type**: C
  - **Design**: ① `src/ui/settings_dialog.rs` 신규(모듈 등록은 `src/ui/mod.rs`). ② 신규 심볼 — `SettingsDialog`(열림 상태 + 그리기), `SettingsOutcome`(이번 프레임에 바뀐 값 — `Option<AppSettings>` 또는 변경 종류 enum). ③ `ui::app`이 소유하고 `AppSettings`를 빌려주며, 결과를 받아 반영·저장한다. `settings_dialog`는 `widgets`·`theme`을 참조하고 `app`을 모른다. ④ 이번에 추상화하지 않을 것: 설정 항목을 배열+반복으로 묶지 않는다 — 항목마다 컨트롤 종류와 부수 효과(레지스트리 쓰기·글꼴 재등록)가 달라 묶으면 채우는 순간 다시 풀어야 한다(`show_settings_menu`의 기존 주석과 같은 판단).
  - **Acceptance**: Given 앱 실행 상태, When 타이틀바 `⚙` → `설정`을 누르면, Then 모달이 열리고 `모양`·`시작`·`종료`·`파일 보기` 네 그룹이 구분선과 그룹 제목으로 나뉘어 세로로 나열된다. `닫기` 또는 배경 클릭·`Esc`로 닫힌다. 나머지 네 메뉴 항목(`업데이트`·`릴리즈 노트`·`오픈소스 라이선스`·`정보`)은 비활성 그대로다.
  - **Files**:
    - 주: `src/ui/settings_dialog.rs`(신규), `src/ui/mod.rs`
    - 동반: `src/ui/titlebar.rs`(`show_settings_menu:234` — `설정`만 활성화하고 `Command::OpenAppSettings` 반환), `src/ui/menu.rs`(`Command:56` variant 추가), `src/ui/app.rs`(명령 처리·모달 그리기)
    - 테스트: `src/ui/settings_dialog.rs`(`mod tests` — 열림/닫힘 상태 전이)
  - **Edge Cases**: 모달이 열린 채 창 크기가 줄어들 때(고정 크기 대화가 창보다 커짐) → 사이트 관리자와 같은 처리를 따른다 / 다른 모달(사이트 관리자·삭제 확인)과 동시에 열리는 경우 → 설정은 타이틀바에서만 열리므로 겹칠 수 있는지 확인하고, 겹치면 뒤에 그린다
  - **Halt Forecast**:
    - (i) "어떤 신호로 여는가" → D11에서 확정
  - **Depends on**: T1, T2

<!-- T4~T5 (글꼴) -->

- [x] **T4. 시스템 글꼴 열거와 글꼴 바이트 읽기**
  - **Type**: C
  - **Design**: ① `src/app/fonts.rs` 신규 — Win32 GDI를 쓰지만 파일시스템 열거(`fs`)도 UI(`ui`)도 아닌 "앱 환경 조회"라 `app`에 둔다(`app::theme`이 `DwmSetWindowAttribute`를 쓰는 것과 같은 자리). ② 신규 심볼 — `installed_korean_fonts() -> Vec<String>`(한글 지원 글꼴 이름을 가나다순 중복 없이), `load_font(name: &str) -> Option<Vec<u8>>`. ③ `ui`가 이것을 부르고, `fonts`는 아무것도 참조하지 않는다. **`installed_korean_fonts()`는 1.5초쯤 걸린다**(실측: 이름 열거 1ms + 93개 전수 읽기 1,525ms) — 이름만 세지 않고 실제로 읽어 확인하기 때문이다. 그래서 **UI 스레드에서 부르면 안 되고**, T5가 워커에서 부른다(AGENTS "UI 스레드에서 블로킹 I/O 금지"). ④ 이번에 추상화하지 않을 것: 글꼴 캐시·메타데이터 구조체를 두지 않는다(이름과 바이트면 충분하다).
    **`load_font`의 세 단계**: ⓐ `CreateFontIndirectW`+`SelectObject` 뒤 **`GetTextFaceW`로 실제 선택된 face 이름이 요청 이름과 같은지 확인**한다 — GDI 폰트 매퍼는 없는 이름을 오류 없이 비슷한 글꼴로 대체하므로, 이 확인이 없으면 삭제된 글꼴에 대해 **엉뚱한 글꼴 바이트가 성공처럼 반환된다**(FR-48의 폴백 요구가 무증상으로 죽는다). 다르면 `None`. ⓑ `GetFontData(hdc, 0, 0, ..)`로 원본 바이트를 받는다.
    ~~**face 인덱스는 다루지 않는다 (2026-08-13 실측으로 확정)**~~ **→ T5에서 재실측으로 뒤집힘.** T4는 모음 글꼴의 `ttcf` 테이블이 없고 매직이 `0x00010000`이라는 것만 보고 "GDI가 단일 sfnt를 뽑아 준다"고 결론냈는데, **매직만 보고 파싱까지 확인하지 않은 오판**이었다. 실제로는 40바이트 부족한 깨진 데이터라 글꼴 파서가 읽지 못하고, 그 결과 굴림·바탕 등이 전부 걸러졌다. **face 인덱스는 필요하다** — 파일을 직접 읽고 `egui::FontData.index`에 넣는다(D3 A→C, 상세는 `## Next Steps`의 「T4·T5 실측 기록」).
  - **Acceptance**: Given 한국어 Windows, When `installed_korean_fonts()`를 부르면, Then 결과에 `맑은 고딕`과 **글꼴 모음에 든 `굴림`·`바탕`이 함께** 있고 한글 글리프가 없는 `Wingdings`·`Webdings`는 없다. **목록의 모든 이름에 대해** `load_font(name)`이 `Some`이다(바이트를 얻을 수 있다). 목록에 없는 이름(`load_font("없는글꼴")`)은 **GDI가 굴림으로 조용히 대체하더라도** `None`을 반환한다(실측: `없는글꼴이름XYZ` → 실제 선택 `굴림`, 데이터 크기까지 굴림과 동일).
    **글꼴이 실제로 파싱되는지(한글 폭 > 0)는 T5가 확인한다** — 그 검증은 egui에 등록해 봐야 알 수 있는데 `app` 계층은 `ui`를 모르므로(AGENTS 의존 방향) 여기 둘 수 없다. 실측에서 `D2Coding`이 그 경계였다: 바이트는 읽히는데 등록하면 한글 폭이 0이다.
  - **Files**:
    - 주: `src/app/fonts.rs`(신규), `src/app/mod.rs`
    - 테스트: `src/app/fonts.rs`(`mod tests` — ① 목록에 `맑은 고딕`·`굴림` 포함, `Wingdings`·세로쓰기(`@`) 부재 ② 목록이 정렬·중복 없음 ③ 목록의 모든 이름이 `load_font`로 읽힘 ④ 없는 이름은 `None`)
  - **Edge Cases**: `EnumFontFamiliesExW` 콜백이 같은 이름을 여러 번 준다(굵기·기울임별) → 중복 제거 / 이름이 `@`로 시작하는 세로쓰기 글꼴 → 제외 / GDI 핸들 누수 → `CreateCompatibleDC`·`CreateFontIndirectW`를 반드시 짝지어 해제(`unsafe` 격리 함수 안에서) / 글꼴이 하나도 없는 환경 → 빈 목록이어도 패닉하지 않고 현재 글꼴 유지 / **`CreateFontIndirectW`는 요청한 이름의 글꼴이 없으면 오류를 내지 않고 가장 비슷한 글꼴로 조용히 대체한다** — `load_font`는 `SelectObject` 뒤 `GetTextFaceW`(`Graphics/Gdi/mod.rs:1293`)로 **실제 선택된 이름이 요청한 이름과 같은지 확인**하고, 다르면 `None`을 돌려준다(엉뚱한 글꼴 바이트가 적용되는 것을 막는다. 저장된 글꼴이 나중에 삭제된 경우가 이 경로다)
  - **Halt Forecast**:
    - (i) "이름만으로 바이트를 얻을 수 있는가" → 전제 6에서 확인 완료
    - (i) "TTC(글꼴 모음)를 어떻게 다루는가" → ~~실측으로 해소(특별 취급 불필요)~~ **→ T5 재실측으로 부정**. GDI가 준 데이터가 깨져 있어 모음 글꼴이 전부 걸러졌고, 파일 직접 읽기 + face 인덱스로 전환했다(D3 A→C, 상세는 「T4·T5 실측 기록」)
  - **Depends on**: -

- [x] **T5. 글꼴 선택 UI와 런타임 적용**
  - **Type**: C
  - **Design**: ① 선택 UI는 `settings_dialog.rs`의 `모양` 그룹(`widgets::dropdown_field` 재사용), 적용은 `ui/app.rs`의 `install_fonts`를 확장한다. **목록은 워커 스레드가 만든다**(사용자 결정 A, 2026-08-13). 그 상태·스레드는 **`src/ui/font_scan.rs`(신규)** 에 둔다 — 계획은 `ui/app.rs`라고 적었으나 그 파일이 이미 2,700줄이라 워커를 더 얹으면 분리 검토선에서 더 멀어진다(AGENTS 파일 책임 규약). `ui/panel/workers.rs`가 패널 워커를 따로 둔 것과 같은 자리다. 대화를 열 때 스레드를 하나 띄워 `app::fonts::installed_korean_fonts()`(1.5초)를 부르고 **각 이름을 egui에 실제로 등록해 한글 폭 > 0인 것만 남긴 뒤** 1회용 채널로 돌려준다(`ui/panel.rs`의 `DirLoad`와 같은 방식). 준비될 때까지 드롭다운은 현재 글꼴만 보이고 `글꼴 목록을 읽는 중`을 알린다. 이렇게 해야 **목록에 있는데 고르면 깨지는 글꼴이 없다**(실측 `D2Coding`). ② 신규 심볼 없음 — `install_fonts(ctx, family: Option<&str>) -> bool`로 시그니처만 넓힌다(`None`이면 지금처럼 맑은 고딕). 고른 글꼴은 `app::fonts::load_font`가 준 바이트와 **face 인덱스**를 `egui::FontData { font, index, tweak }`로 등록한다 — 모음 글꼴(굴림·바탕 등)은 파일 하나에 여러 글꼴이 들어 있어 인덱스가 없으면 언제나 첫 번째만 나온다(D3 변경 참조. `from_owned`는 인덱스를 0으로 고정하므로 쓰지 않는다). ③ `ui::app`이 `app::fonts`를 부른다. ④ 이번에 추상화하지 않을 것: 글꼴 폴백 체인을 설정으로 만들지 않는다(고른 글꼴 → 맑은 고딕 → egui 기본, 2단 폴백 고정).
  - **Acceptance**: 대화를 열면 창이 멈추지 않고(목록 조회가 UI 스레드를 막지 않는다) 잠시 뒤 목록이 채워지며, **그 목록의 모든 글꼴은 골랐을 때 한글이 두부(□)로 깨지지 않는다**(등록해 폭 > 0인 것만 담기 때문 — `D2Coding`처럼 읽히지만 파싱되지 않는 글꼴은 목록에 없다). Given 설정 화면, When `모양` 그룹의 글꼴 드롭다운에서 다른 글꼴을 고르면, Then **다음 프레임에** 파일 목록·타이틀바·메뉴 글꼴이 바뀌고(전제 7 — `set_fonts`는 다음 pass부터 적용되므로 호출과 함께 `ctx.request_repaint()`로 그 프레임을 보장한다) 아이콘(phosphor)은 그대로 보이며 값이 저장된다. 앱을 다시 켜도 그 글꼴이 유지된다. 저장된 글꼴을 읽지 못하면(글꼴 삭제 등) 맑은 고딕으로 시작하고 **설정 값은 그대로 둔다**(사용자가 글꼴을 다시 설치하면 되살아난다).
  - **Files**:
    - 주: `src/ui/font_scan.rs`(신규 — 목록 워커), `src/ui/app.rs`(`install_fonts` 확장·시작 시 적용·워커 배선), `src/ui/settings_dialog.rs`(`모양` 그룹)
    - 동반: `src/ui/list_common.rs:254`, `src/ui/menu.rs:438`, `src/ui/list_grid.rs:405,489,634`(테스트 호출부 5곳)
    - 테스트: `src/ui/font_scan.rs`(`mod tests` — 목록 준비 상태 전이·목록의 모든 글꼴이 한글을 그림·고른 글꼴 등록·없는 글꼴 → 폴백)
  - **Edge Cases**: 매 프레임 `set_fonts`를 부르면 글꼴 아틀라스를 다시 만들어 느려진다 → **값이 바뀐 프레임에만** 부른다 / 목록 조회는 대화를 열 때 워커에서 한 번만 — 이미 받아 둔 목록이 있으면 다시 부르지 않는다 / 목록이 오기 전에 대화를 닫으면 결과를 버린다(수신 측이 사라져도 `send` 실패는 무해) / 워커가 만드는 검증용 `egui::Context`는 화면 컨텍스트와 별개다 — 화면 글꼴을 건드리지 않는다 / 12MB급 글꼴을 읽는 동안 한 프레임 멈춤 → 허용(설정 조작 중 1회, NFR-3의 목록 스크롤과 성격이 다르다)
  - **Halt Forecast**:
    - (i) "런타임 교체가 되는가" → 전제 7에서 확인 완료
    - (ii-a) `install_fonts` 공개 시그니처 변경(호출부 6곳) → `## 사전 승인 항목`에 등록
  - **Depends on**: T3, T4

<!-- T6 (자동 실행) -->

- [x] **T6. 윈도우 시작 시 자동 실행 등록·해제**
  - **Type**: C
  - **Design**: ① `src/app/autostart.rs` 신규(레지스트리 접근은 앱 환경 조작이라 `app`). ② 신규 심볼 — `is_enabled() -> bool`, `set_enabled(on: bool) -> std::io::Result<()>`. 값 이름은 `MOA`, 데이터는 `"<exe 경로>" --tray`(D9). ③ `settings_dialog`가 토글에서 부르고, `autostart`는 아무것도 참조하지 않는다. ④ 이번에 추상화하지 않을 것: 레지스트리 래퍼 타입을 만들지 않는다(키 하나·값 하나를 읽고 쓸 뿐이다).
  - **Acceptance**: Given 설정 화면, When `시작` 그룹의 토글을 켜면, Then `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`에 `MOA` 값이 현재 exe 절대 경로(따옴표로 감싸고 `--tray` 인자 포함)로 생기고, 끄면 그 값이 사라진다. 앱을 다시 켜면 **레지스트리의 실제 상태**가 토글에 반영된다(설정 파일 값이 아니라 레지스트리가 정본).
  - **Files**:
    - 주: `src/app/autostart.rs`(신규), `src/app/mod.rs`
    - 동반: `src/ui/settings_dialog.rs`(`시작` 그룹), `src/ui/app.rs`(등록 실패 알림을 토스트로 — `SettingsOutcome.notice`를 소비하는 유일한 지점)
    - 테스트: `src/app/autostart.rs`(`mod tests` — 켜기→읽기→끄기→읽기 왕복. 실제 HKCU를 쓰되 테스트 종료 시 반드시 원상 복구)
  - **Edge Cases**: 경로에 공백이 있는 exe → 따옴표 필수 / 이전에 다른 경로로 등록돼 있음(앱을 옮긴 경우) → 켜져 있으면 현재 경로로 덮어쓴다 / 레지스트리 쓰기 실패(정책 차단) → 토글을 되돌리고 조용히 무시하지 않는다(상태가 어긋나면 안 된다) / 값 이름 충돌 → `MOA` 고정
  - **Halt Forecast**:
    - (i) "관리자 권한이 필요한가" → 전제 4에서 확인(HKCU라 불필요)
  - **Depends on**: T3

<!-- T7~T9 (트레이·단일 인스턴스) -->

- [x] **T7. 트레이 아이콘 등록과 이벤트 배선**
  - **Type**: D
  - **Design**: ① `src/ui/tray.rs` 신규 — HWND·창 프로시저와 붙어 있어 `ui`에 둔다(`shell_host`와 같은 층). ② 신규 심볼 — `Tray`(아이콘 등록·해제를 `Drop`까지 책임지는 소유 타입), `TrayEvent`(`Shown`/`Quit` — **창을 보이게 하는 동작 자체는 프로시저가 이미 끝냈고, `Shown`은 "되살렸다"는 사후 통지다**), `TRAY_CALLBACK: u32 = WM_APP + 2`(`WM_APP+1`은 `fs/enumerate.rs:16`이 이미 쓴다), `TrayContext { tx: Sender<TrayEvent>, ctx: egui::Context }`. ③ `TrayContext`는 **`static TRAY: OnceLock<TrayContext>`**에 둔다 — `SetWindowSubclass`의 `dwRefData`에 `Box::into_raw`를 실으면 `RemoveWindowSubclass`보다 먼저 `Box::from_raw`를 하는 순간 이후 도착하는 트레이·셸 메시지가 해제된 메모리를 읽고(UAF), 현재 `shell_host.rs:37-38`은 "창이 파괴되면 서브클래스도 함께 사라지므로 별도 해제는 필요 없다"는 전제로 짜여 `ShellHost`에 `Drop`이 없다. `OnceLock`은 프로세스 수명과 같아 해제 순서 문제가 아예 없고(앱 인스턴스가 하나뿐이라 누수가 아니다) `unsafe` 표면을 늘리지 않는다. 프로시저는 `TRAY.get()`으로 채널에 닿는다. 더블클릭·메뉴 `실행`은 프로시저가 D5의 복원 절차 ⓐⓑⓒ를 즉시 수행하고, 메뉴 `종료`는 같은 복원을 거친 뒤 `TrayEvent::Quit`을 보낸다. ④ 이번에 추상화하지 않을 것: 트레이 아이콘 여러 개·풍선 알림·상태별 아이콘 교체를 다루지 않는다(아이콘 하나·툴팁 하나).
  - **Acceptance**: Given `종료` 그룹의 토글이 켜진 상태, When 앱이 실행 중이면, Then 트레이에 MOA 아이콘이 **창이 떠 있는 동안에도** 보이고 마우스를 올리면 `MOA` 툴팁이 뜬다. 토글을 끄면 아이콘이 즉시 사라진다. **정상 종료 시** `Drop`이 `NIM_DELETE`를 보내 아이콘이 즉시 사라진다(강제 종료 시 남는 유령 아이콘은 OS가 마우스 접촉 때 정리하는 영역이라 이 task의 판정 대상이 아니다). 아이콘 우클릭 시 `실행`·`종료` 두 항목의 메뉴가 뜬다.
  - **Files**:
    - 주: `src/ui/tray.rs`(신규), `src/ui/mod.rs`
    - 동반: `src/ui/app_icon.rs`(`ICO_BYTES`를 `pub`으로 — 트레이가 같은 그림을 쓴다), `src/ui/shell_host.rs`(`shell_menu_proc:91` — 트레이 콜백 갈래 + `new:31`의 `dwRefData`에 `TrayContext` 싣기 + `Drop`에서 회수), `src/ui/app.rs`(`Tray` 소유·`Quit` 폴링)
    - 테스트: `src/ui/tray.rs`(`mod tests` — 마우스 메시지 코드(`WM_LBUTTONDBLCLK`·`WM_RBUTTONUP`)→동작 분류, 메뉴 항목 ID→`TrayEvent` 매핑. 실제 아이콘 등록은 HWND가 필요해 테스트 비대상)
  - **Edge Cases**: 탐색기(explorer.exe) 재시작 → `TaskbarCreated` 등록 메시지를 받아 아이콘을 다시 등록한다 / 프로세스가 비정상 종료해 유령 아이콘이 남음 → `NOTIFYICONDATAW`에 `uVersion`을 설정하고 `Drop`에서 `NIM_DELETE`, 그래도 남는 경우는 OS가 마우스 접촉 시 정리한다 / 우클릭 메뉴가 뜬 채 포커스를 잃음 → `SetForegroundWindow`를 메뉴 표시 전에 부른다(Win32 관례) / 아이콘 리소스를 못 읽음 → 트레이 기능을 조용히 끄지 말고 토글을 되돌린다
  - **Halt Forecast**:
    - (i) "콜백을 받을 창 프로시저가 있는가" → 전제 9에서 확인 완료
    - (i) "아이콘 리소스를 새로 만들어야 하는가" → 전제 11에서 확인(기존 재사용)
  - **Depends on**: T3

- [x] **T8. 닫기 가로채기와 창 숨김·복원**
  - **Type**: D
  - **Design**: ① `ui/app.rs`의 프레임 처리(`logic`/`ui`)·`track_window:649`·`on_exit:1932`. ② 신규 심볼 없음 — `ExplorerApp`에 `hidden: bool`과 분기를 더한다. ③ 숨기기는 `ViewportCommand::Visible(false)`로 보내고(그 시점엔 창이 보여 프레임이 돈다), 되살리기는 프로시저가 이미 끝낸 뒤이므로 앱은 **`TrayEvent::Shown`을 받은 프레임에** ⓐ `hidden = false`로 내리고 ⓑ `ViewportCommand::Visible(true)`를 한 번 보내 winit 가시성 캐시를 맞춘다(전제 9-c — 이걸 빼면 이후 최대화 등 창 플래그 변경 때 `apply_diff`가 `SW_HIDE`를 재적용해 창이 갑자기 사라진다). **`ctx.input(|i| i.viewport().visible())`로 파생시키지 않는다** — Windows에서 그 값은 늘 `None`이라 숨김·표시를 구분하지 못한다(전제 9-a). ④ 이번에 추상화하지 않을 것: 창 표시 상태 머신 타입을 만들지 않는다(`hidden`·`quitting` 불리언 둘).
    **숨김 판정에 "트레이 아이콘이 실제로 올라가 있는가"를 함께 본다**(구현 중 추가): `sync_tray`가 등록 실패 시 토글을 되돌리지만 `intercept_close`가 `sync_tray`보다 먼저 도는 프레임이 있어, 그 사이에 닫으면 **아이콘 없이 창만 사라져 되부를 방법이 없어진다**.
    **`track_window`를 숨김 중에 멈춘다**: `track_window:649-682`는 매 프레임 viewport 정보로 `self.window`(위치·크기·최대화)를 덮는다. 숨기는 순간의 좋은 값을 저장해 두어도, 숨김 전후에 이 함수가 한 번이라도 더 돌면 그 값이 덮이고 `on_exit:1936`이 덮인 값을 저장한다. 따라서 **`hidden`이면 `track_window`를 즉시 반환**시키고, 숨기기 직전에 세션을 저장한다.
  - **Acceptance**: Given `종료` 토글이 켜진 상태, When 타이틀바 `✕`나 `Alt+F4`로 닫으면, Then `close_requested()`를 감지해 `CancelClose`+`Visible(false)`를 보내 **프로세스는 살아 있고 창만 사라지며, 숨기기 직전에 세션(창 위치·크기·워크스페이스)이 저장된다**. 트레이 아이콘 더블클릭 또는 메뉴 `실행`을 하면 창이 **숨기기 전과 같은 위치·크기(최대화였으면 최대화 상태)로** 다시 뜨고 최상위로 올라온다 — 즉 숨김·복원을 반복해도 창 지오메트리가 흘러가지 않는다. 메뉴 `종료`는 앱을 실제로 끝내며 이때도 세션이 저장된다. 토글이 꺼져 있으면 `✕`가 종전대로 종료한다.
  - **Files**:
    - 주: `src/ui/app.rs`(`logic:1939` 부근 닫기 감지, `on_exit:1932`, `persist_session:1777`)
    - 동반: `src/ui/tray.rs`(이벤트 소비)
    - 테스트: `src/ui/app.rs`(`mod tests` — 토글 on/off × 닫기 요청 → 숨김/종료 판정 함수. `ViewportCommand` 전송은 순수 함수로 뽑아 검증)
  - **Edge Cases**: **숨긴 상태에서는 UI 코드가 아예 돌지 않는다**(전제 9-a) → 복원은 프레임이 아니라 창 프로시저가 한다(D5) / 숨긴 상태에서 메뉴 `종료`를 누름 → 프로시저가 창을 먼저 보이게 해 프레임을 되살린 뒤 종료 신호를 보낸다(그래야 `on_exit`가 돌아 세션이 저장된다) / **최대화 상태에서 숨겼다 복원** → `ShowWindow(SW_SHOW)`는 최대화를 유지하지만 최소화돼 있었다면 `SW_RESTORE`가 필요하다 — 프로시저가 `IsIconic`으로 갈라 부른다 / `track_window`가 숨김 중 값을 덮지 않는지 확인(Design) / 시작 시부터 숨김인 경우(T10)와 `restoring_maximized:660-673` 복원 로직이 만나는 지점 → T10이 다룬다 / **트레이 토글은 설정 모달에서만 바뀌고 모달은 창이 보일 때만 열리므로**, "숨은 상태에서 토글이 꺼지는" 분기는 존재하지 않는다(트레이 메뉴에 토글을 두지 않는다)
  - **Halt Forecast**:
    - (i) "닫기를 취소할 수 있는가" → 전제 1에서 확인 완료
    - (i) "숨김 시 세션이 날아가는가" → 이 task Acceptance에서 저장 지점을 추가해 해소
  - **Depends on**: T7

- [x] **T9. 중복 실행 방지와 기존 창 활성화**
  - **Type**: C
  - **Design**: ① 판정은 `src/app/single_instance.rs` 신규(창 생성 **전**에 해야 해서 `main.rs`가 부른다), 기존 창을 깨우는 수신은 `ui/tray.rs`의 `handle_callback`에 둔다 — `shell_host.rs`의 창 프로시저가 커스텀 등록 메시지를 전부 그 함수에 위임하는 구조라(트레이 콜백·탐색기 재시작도 같은 자리), 여기만 따로 떼면 같은 성격의 분기가 두 파일로 갈린다. ② 신규 심볼 — `acquire() -> Instance`(뮤텍스를 든 가드. `Instance::IsFirst`/`AlreadyRunning`), `WAKE_MESSAGE`(등록 메시지 이름 상수). ③ `main.rs`가 부르고, 수신은 `shell_menu_proc`가 **트레이 더블클릭과 같은 복원 루틴**(D5의 ⓐⓑⓒ)을 그대로 부른다 — 하는 일이 "창을 보이게 하고 최상위로 올린다"로 동일하므로 경로를 나누지 않는다. ④ 이번에 추상화하지 않을 것: IPC 계층·명령줄 전달(두 번째 인스턴스가 연 경로를 기존 창에 전달하는 등)을 만들지 않는다 — 이번 요구는 "창을 띄운다"뿐이다.
  - **Acceptance**: Given 앱이 실행 중(창이 떠 있거나 트레이에 숨은 상태), When exe를 다시 실행하면, Then 새 프로세스는 **창을 만들지 않고** 끝나고 기존 창이 화면 최상위로 올라온다. 실행 중이 아니면 정상적으로 새 창이 뜬다.
  - **Files**:
    - 주: `src/app/single_instance.rs`(신규), `src/app/mod.rs`, `src/main.rs`
    - 동반: `src/ui/tray.rs`(깨우기 메시지 수신 갈래 — D5 복원 루틴 재사용), `src/ui/app.rs`(`TrayEvent::Shown` 처리 — T8과 같은 경로)
    - 테스트: `src/app/single_instance.rs`(`mod tests` — 같은 프로세스에서 두 번 `acquire` 시 두 번째가 `AlreadyRunning`)
  - **Edge Cases**: 앱이 죽어 뮤텍스가 해제됨 → OS가 핸들을 닫으면 자동 해제되므로 다음 실행이 정상 시작 / 다른 사용자 세션에서 실행 → 뮤텍스 이름에 `Local\` 접두를 붙여 세션 단위로 가른다 / 브로드캐스트를 받는 창이 아직 안 만들어짐(시작 경합) → 뮤텍스를 얻은 쪽만 창을 만드므로 경합해도 창은 하나다 / COM 초기화·세션 로드보다 **먼저** 판정해 불필요한 작업을 막는다
  - **Halt Forecast**:
    - (i) "기존 창을 어떻게 찾는가" → D6에서 확정(브로드캐스트 — 창 찾기 불필요)
  - **Depends on**: T8

- [ ] **T10. 자동 실행으로 시작할 때 트레이로만 올라오기**
  - **Type**: B
  - **Design**: 해당 없음 — 신규 심볼 없이 `main.rs`의 시작 분기에 조건 한 줄과 `ExplorerApp` 초기 `hidden` 값 전달만 더한다.
  - **Acceptance**: Given Run 키에 `--tray` 인자로 등록된 상태에서 부팅, When 앱이 자동 실행되면, Then `종료` 토글이 켜져 있으면 **최대화 복원(`restoring_maximized`)이 끝난 직후** 창이 숨겨져 트레이 아이콘만 남고, 꺼져 있으면 창이 정상으로 뜬다. 사용자가 직접 실행할 때는(인자 없음) 항상 창이 뜬다. 숨겨진 뒤 트레이 더블클릭으로 창을 부르면 세션의 위치·크기(최대화 포함)가 그대로다.
  - **Files**:
    - 주: `src/main.rs`, `src/ui/app.rs`(초기 숨김 상태 수용)
    - 테스트: `src/app/settings.rs` 또는 `src/main.rs`(인자 판정 순수 함수 — `args`에 `--tray`가 있고 설정이 켜져 있을 때만 `true`)
  - **Edge Cases**: `--tray`인데 트레이 토글이 꺼져 있음(사용자가 나중에 껐다) → 창을 띄운다(부를 방법이 없어지는 것을 막는다) / eframe은 창 없이 돌 수 없으므로 창은 정상으로 만들고 **아래 시점에** 숨긴다 / **숨김 시작과 최대화 복원이 겹친다** — 세션이 최대화 상태였으면 `track_window:660-673`의 `restoring_maximized`가 여러 프레임에 걸쳐 `Maximized(true)`를 재시도하는데, 그 전에 창을 숨기면 프레임이 멈춰(전제 9-a) 복원이 **영영 끝나지 않는다**. 따라서 시작 숨김은 **`restoring_maximized`가 0이 된 뒤**(복원 완료) 적용하고, 그때까지는 창이 잠깐 보인다(부팅 직후라 사용자가 보고 있을 가능성이 낮다). 이 순서를 T10 Acceptance의 판정에 포함한다
  - **Halt Forecast**:
    - (i) "자동 실행을 어떻게 판정하는가" → D9에서 확정
  - **Depends on**: T6, T8

<!-- T11~T12 (파일 보기) -->

- [ ] **T11. 파일 확장명 표시 토글**
  - **Type**: C
  - **Design**: ① `src/panel/file_list.rs`의 `ListRow`에 표시 전용 메서드를 더하고, 렌더 6곳이 그것을 쓴다. ② 신규 심볼 — `ListRow::display_name(&self, show_extensions: bool) -> String`(기본 구현: `false`이고 폴더·`..`·확장자 없음이 아니면 마지막 `.` 앞까지). ③ 렌더 모듈(`list_details`·`list_grid`)이 `DetailsInput`/격자 입력에 실린 플래그를 받아 부른다 — `panel`은 `ui`를 모르는 방향 그대로. ④ 이번에 추상화하지 않을 것: "알려진 형식만 숨기기"(탐색기식) 규칙을 넣지 않는다 — 전부 숨긴다.
  - **Acceptance**: Given `파일 보기` 그룹의 `파일 확장명`이 off, When 목록을 보면, Then `보고서.hwp`가 `보고서`로 보이고 폴더·`..`·`.gitignore`(앞이 빈 이름)는 그대로다. **원격 패널도 같게** 보인다. 이 상태에서 파일을 더블클릭하면 정상 실행되고, 이름 정렬 순서·선택 유지·셸 컨텍스트 메뉴가 확장자 표시 때와 **똑같다**. on으로 되돌리면 확장자가 다시 보인다.
  - **Files**:
    - 주: `src/panel/file_list.rs`(`ListRow:520`·`impl FileEntry:575`·`impl RemoteEntry:625`)
    - 동반: `src/ui/list_details.rs:476-477`, `src/ui/list_grid.rs:256,279,324,340`, `src/ui/file_list.rs`(플래그 전달), `src/ui/panel.rs`(설정 값 전달)
    - 테스트: `src/panel/file_list.rs`(`mod tests` — 로컬·원격 각각 폴더/`..`/확장자 없음/일반 파일 4케이스), `src/ui/file_list.rs`(경로 조립·정렬이 원본을 쓰는지)
  - **Edge Cases**: 이름이 `.`으로만 됨 / 확장자가 여러 개(`a.tar.gz` → `a.tar`) / 이름 끝이 `.`(`a.` → `a.` 그대로 — 잘라도 얻는 게 없다) / 심볼릭 링크의 `이름 → 대상` 표기에서 이름 부분만 잘라야 한다(`list_details.rs:476`) / 격자 보기의 `내용` 모드는 이름·종류·크기 3줄이라 이름 줄만 대상(`list_grid.rs:340`)
  - **Halt Forecast**:
    - (i) "어디에 적용하는가(경로 오염 위험)" → D7·전제 13에서 확정
  - **Depends on**: T3

- [ ] **T12. 숨김·시스템 항목 표시 토글**
  - **Type**: C
  - **Design**: ① 로컬 속성 보존은 `src/fs/enumerate.rs`, 판정은 `ListRow`, 거르기는 목록 모델(`src/ui/file_list.rs`). ② 신규 심볼 — `FileEntry.attributes: u32`(필드), `ListRow::is_hidden(&self) -> bool`(로컬: `FILE_ATTRIBUTE_HIDDEN|FILE_ATTRIBUTE_SYSTEM`, 원격: 이름이 `.`으로 시작). ③ `fs`가 속성을 실어 주고 `panel`이 판정하며 `ui`가 거른다. ④ 이번에 추상화하지 않을 것: 필터 체인·사용자 정의 규칙을 두지 않는다(불리언 하나).
    **`..` 항목의 속성값**: `ui/panel.rs:1381 with_local_parent_first`가 만드는 상위 이동 줄은 실제 파일이 아니라 화면 장치이므로 `attributes: 0`으로 둔다 — 어떤 필터에도 걸리지 않아 FR-31의 "`..`는 항상 첫 줄"이 유지된다. 테스트 헬퍼 6곳도 같은 이유로 `0`을 기본값으로 쓴다.
  - **Acceptance**: Given `파일 보기` 그룹의 `숨김 항목`이 off, When 목록을 보면, Then `FILE_ATTRIBUTE_HIDDEN` 또는 `FILE_ATTRIBUTE_SYSTEM`이 붙은 로컬 항목과 이름이 `.`으로 시작하는 원격 항목이 보이지 않고, **폴더·파일 개수(상태 줄)도 거른 뒤 기준**으로 센다. on이면 전부 보인다(현재 동작과 같다). `..` 항목은 토글과 무관하게 항상 첫 줄에 남는다. 토글을 바꾸면 새로 고침 없이 즉시 반영된다.
  - **Files**:
    - 주: `src/fs/enumerate.rs`(`FileEntry:20`·`push_entry:153`), `src/panel/file_list.rs`(`ListRow` 구현 2곳 — `:575`·`:625`)
    - 동반: `src/ui/panel.rs`(`with_local_parent_first:1381`의 `..` 생성 + 설정 값 전달·변경 시 재정렬), `src/ui/file_list.rs`(거르기·`dir_count`/`file_count` 재계산)
    - 테스트(**`FileEntry` 생성 헬퍼가 있어 필드 추가로 전부 깨진다 — 6곳 모두 갱신**): `src/fs/enumerate.rs:305`, `src/panel/file_list.rs:769`, `src/ui/file_list.rs:630`, `src/ui/list_grid.rs:392`, `src/ui/panel/tests.rs:313`, `src/ui/tree.rs:372`
    - 신규 테스트: `src/fs/enumerate.rs`(숨김 속성 파일을 만들어 `attributes`가 실리는지), `src/panel/file_list.rs`(`is_hidden` 로컬·원격 판정), `src/ui/file_list.rs`(거른 뒤 개수)
  - **Edge Cases**: 숨김 폴더 안으로 직접 들어감(주소창 입력) → 목록은 비어 보일 수 있어도 진입 자체는 막지 않는다 / 숨김 항목이 선택된 상태에서 토글을 끔 → 선택에서 빠져야 한다(보이지 않는 항목이 선택돼 있으면 삭제·복사가 예상 밖으로 동작한다) / `FILE_ATTRIBUTE_SYSTEM`만 붙은 항목(`pagefile.sys`) → 함께 숨긴다 / 변경 감시(`FR-10`)로 새 항목이 들어올 때도 같은 필터를 지난다
  - **Halt Forecast**:
    - (i) "원격 숨김을 무엇으로 판정하는가" → D8·전제 12에서 확정
    - (ii-a) `FileEntry`에 필드 추가(공개 구조체 변경) → `## 사전 승인 항목`에 등록
  - **Depends on**: T3

<!-- T13 (마무리) -->

- [ ] **T13. 문서 갱신과 성능 재측정**
  - **Type**: C
  - **Acceptance**: `README.md`에 설정 화면과 네 기능이 현재 구현대로 기재된다(없는 기능 기재 0건). `AGENTS.md`의 데이터 접근 항목이 "세션 + 앱 설정"을 담는다는 사실을 반영한다. NFR-1(콜드 스타트 1초)·NFR-2(유휴 150MB)를 재측정해 수치를 plan Progress Log에 남기고, 기준을 넘으면 원인을 특정한다. `cargo build`·`cargo test`·`cargo clippy --all-targets -- -D warnings`·`cargo fmt --check`가 전부 통과한다.
  - **Files**:
    - 주: `README.md`, `AGENTS.md`
  - **Edge Cases**: 측정값이 기준을 넘으면 → 트레이·뮤텍스·글꼴 중 어느 것이 원인인지 하나씩 빼며 좁힌다(대장의 시작 시간 구간 내역이 기준선)
  - **Halt Forecast**:
    - (ii-b) NFR-1·NFR-2 실측이 기준을 넘고 원인이 설계에 있으면 → 설계 변경은 plan에 없던 결정이므로 Halt
  - **Depends on**: T1~T12

## 사전 승인 항목 (일괄 승인 대상)

- **T5 — `install_fonts` 공개 시그니처 변경** (`ui/app.rs:137`): 글꼴 이름 인자를 더한다. 호출부 **6곳**(앱 시작 1 + 테스트 5 — `list_common.rs:254`·`menu.rs:438`·`list_grid.rs:405,489,634`)을 함께 고친다. 계획된 변경이며 되돌리기는 인자 제거 1줄
- **T12 — `FileEntry` 구조체에 `attributes: u32` 필드 추가** (`fs/enumerate.rs:20`): 숨김·시스템 판정에 필요하다. **생성 지점 8곳**(프로덕션 2 + 테스트 6)을 함께 고친다 — 목록은 T12 Files에 있다
- **T1 — `Session` 구조체에 `settings` 필드 추가** (비파괴 — `#[serde(default)]`, 스키마 버전 불변)
- **T3·T5·T7·T9 — 신규 모듈 추가** (`ui/settings_dialog.rs`·`ui/font_scan.rs`·`ui/tray.rs`·`app/fonts.rs`·`app/autostart.rs`·`app/single_instance.rs`)와 그에 따른 `mod.rs` 등록
- **T3 — `Command` enum에 `OpenAppSettings` variant 추가** (`ui/menu.rs:56`)
- **T11 — `ListRow` 트레이트에 메서드 2개 추가** (`display_name`·`is_hidden`): 구현체 2개를 함께 고친다

> 신규 외부 의존성은 **없다** — 필요한 Win32 feature가 모두 이미 켜져 있다(동반 변경 판정의 `Cargo.toml` 행).

## 불가피한 Halt (위임 불가)

- commit 이후의 **push·master 병합·태그·릴리즈** — 이 plan의 작업은 `task/*` 브랜치 로컬 commit까지다
- **NFR-1·NFR-2 실측이 기준을 넘고 원인이 이 plan의 설계에 있는 경우**(T13) — 설계 변경은 plan에 없던 결정
- T6의 레지스트리 쓰기가 **정책으로 차단된 환경**이 확인되는 경우 — 우회(다른 자동 실행 수단: 작업 스케줄러·시작 프로그램 폴더)는 plan에 없는 방식이라 별도 승인

## Verification Strategy

- 빌드: `cargo build`
- 단위·통합 테스트: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- 서식: `cargo fmt --check`
- 수동 검증 (T13에서 한 번에): ① 설정 열기 → 다섯 그룹 표시 ② 글꼴 교체 즉시 반영·재시작 유지 ③ 자동 실행 토글 후 `regedit`으로 Run 키 확인 ④ 트레이 토글 on → **아이콘이 뒤집히지 않고 정상으로 보이는지**(코드로는 확인 불가 — GDI 비트맵 방향) → `✕` → 숨김 → 더블클릭 복원 → 우클릭 메뉴 `종료` → 탐색기를 작업 관리자에서 다시 시작한 뒤 아이콘이 되살아나는지 ⑤ 트레이 상태에서 exe 재실행 → 창이 하나만 ⑥ 확장자·숨김 토글을 로컬·원격 패널에서 각각 확인 ⑦ NFR-1·NFR-2 측정

## Phase Ledger

## Retry Ledger

## Progress Log

- T1-T2 완료 (커밋 cd9d661 + T2 완료 커밋): 앱 설정 스키마(`AppSettings`·`LanguageSetting`)를 세션에 더하고, 설정 화면이 쓸 on/off 토글 위젯(`widgets::toggle_row`)을 만들었다. 빌드·627 테스트·clippy·fmt 전부 통과.
  - 결정: 세션 스키마 버전은 **v3 유지** — 올리면 `parse_session`이 통째로 폴백해 기존 워크스페이스가 초기화된다 (D2 그대로).
  - 결정: 앱 설정은 `to_session`이 아니라 `ExplorerApp::collect_session`이 싣는다 — `to_session`에 인자를 더하면 그 함수의 책임이 흐려지고 테스트 호출부 7곳이 함께 바뀐다.
  - **함정(spec 리뷰가 실험으로 잡음)**: `#[serde(default)]`는 **키가 없을 때만** 기본값을 준다. 값이 있는데 타입이 어긋나면(`"auto_start": "yes"`) 그 오류가 `Session` 전체로 번져 워크스페이스까지 잃는다 → `settings_or_default`가 그 자리에서만 흡수한다. 회귀 4케이스 고정.
  - **함정(quality 리뷰가 잡음)**: 팔레트 색을 지역 상수로 다시 만들면 정본이 갈린다(`theme.rs`가 명시적으로 경계) → `theme::BORDER_CONTROL` 직접 참조.
  - 규약: 상수끼리의 단언은 clippy `assertions_on_constants`에 걸리므로 시험이 아니라 `const _: () = assert!(..)`로 둔다.
  - 규약: 접근자 이름을 필드와 같게 두면(`font_family` / `font_family()`) 정규화가 조용히 우회된다 → `selected_font()`처럼 구분되는 이름을 쓴다.

- T3-T4 완료 (커밋 `ba8614f` + T4 완료 커밋): 설정 대화 뼈대(그룹 4개·구분선·`파일 보기` 토글 배선)와 시스템 글꼴 조회(`app::fonts`).
  - **실측이 계획을 두 번 고쳤다** — ① 모음 글꼴(TTC) face 인덱스 탐색은 불필요했다(`GetFontData`가 모음에서도 단일 sfnt를 준다) ② GDI가 없는 글꼴 이름을 **굴림으로 조용히 대체**해 `GetTextFaceW` 대조가 실제로 필요했다.
  - **사용자 결정 (Halt 후)**: 글꼴 목록 만들기를 **워커 스레드로** 옮긴다(선택지 A). 목록 전수 읽기가 1,525ms라 UI 스레드에서 부르면 대화 열기가 그만큼 멈춘다. `D2Coding`처럼 **읽히지만 egui가 파싱하지 못하는** 글꼴이 있어, 등록해 폭 > 0인 것만 남기는 검증도 그 워커가 함께 한다(T5).
  - 규약: 설정 대화는 `취소`가 없어 초안(`Draft`) 사본을 두지 않고 `&mut AppSettings`를 빌려 쓴다 — 사이트 관리자와 다른 점.
  - 규약: 시험이 여러 그룹을 쌓은 좌표에 기대면 뒤 task가 앞 그룹을 채울 때 조용히 깨진다 → 그룹 하나만 그리는 함수로 떼어 시험한다.

- T5-T6 완료 (커밋 `2bc81ec` + T6 완료 커밋): 글꼴 선택·적용(워커 포함)과 윈도우 시작 시 자동 실행.
  - **T4의 결론이 T5에서 뒤집혔다** — GDI `GetFontData`가 모음 글꼴(TTC)에서 40바이트 부족한 깨진 데이터를 준다. 굴림·바탕 등이 전부 걸러져 파일 직접 읽기로 전환(D3 A→C, 사용자 승인). 상세는 아래 「T4·T5 실측 기록」.
  - **규약(리뷰가 두 번 잡음)**: 결론이 뒤집히면 plan의 **모든** 관련 자리를 함께 고쳐야 한다. T5 Design 한 곳만 고쳤다가 T4 Design·Halt Forecast·위험표 세 곳에 낡은 결론이 남은 것을 quality 리뷰가 잡았다 — 그대로 두면 다음 세션이 그중 하나만 보고 같은 오판을 반복한다.
  - 규약: 자동 실행의 **정본은 레지스트리**다(설정 파일 값은 사본). 다른 도구가 Run 키를 지웠을 수 있어 화면에 보일 때마다 다시 읽는다.
  - 규약: 레지스트리를 건드리는 시험은 `Drop`으로 원상 복구한다 — 단언이 실패해 패닉이 나도 복구가 돌아야 사용자의 실제 설정이 남지 않는다.

## Next Steps

- 권장 다음 액션: T6부터 `pjc:implement-task docs/plans/2026-08-13-app-settings-part1.md`로 이어서 실행
- 남은 분할 plan: `docs/plans/2026-08-13-app-settings-part2.md` — part1 완료 후 별도 실행

### T4·T5 실측 기록 (2026-08-13) — 글꼴 읽기 경로가 두 번 뒤집혔다

계획은 GDI(`GetFontData`)로 글꼴 바이트를 얻기로 했는데(D3 A안), 두 단계에 걸쳐 틀린 것이 드러났다.

**1차 (T4)**: 모음 글꼴(TTC)에서 `ttcf` 테이블이 없고 매직이 `0x00010000`이라 "GDI가 단일 sfnt를 뽑아 준다"고 결론냈다. → **매직만 보고 파싱까지 확인하지 않은 오판.**

**2차 (T5)**: 등록 검증을 붙이자 굴림·굴림체·돋움·돋움체·바탕·궁서가 **전부 걸러졌다**(93개 중 58개만 남음). 파일 크기를 재 보니 원인이 드러났다.

| 글꼴 | 파일 | `GetFontData` | 차이 | 파싱 |
|---|---|---|---|---|
| 맑은 고딕 (단일 TTF) | 13,459,196 | 13,459,196 | 0 | ✅ |
| 굴림 (`gulim.ttc`) | 13,533,424 | 13,533,384 | **-40** | ❌ |
| 바탕 (`batang.ttc`) | 16,273,348 | 16,273,308 | **-40** | ❌ |

GDI는 모음에서 **헤더만 단일 글꼴 모양으로 바꾼 데이터**를 주는데 내부 테이블 오프셋은 원본 파일 기준이라 40바이트씩 어긋난다.

**해소 (사용자 승인 A)**: 글꼴 파일을 직접 읽는다(D3 → C). 레지스트리 매핑은 값 이름이 **영문**(`Gulim & GulimChe & Dotum & DotumChe (TrueType)` → `gulim.ttc`)이고 열거는 한글(`굴림`)이라 짝지을 수 없어, 파일 안의 `name` 테이블에서 한국어 이름을 직접 읽는다. 모음 글꼴은 face 인덱스를 함께 얻어 `egui::FontData.index`에 넣는다.

**함께 잡은 것**: 글꼴마다 폴더를 다시 훑어 O(n²)가 되던 것(실측 90개에 63초)을 `FontCatalog::scan()` 한 번 재사용으로 바꿨다(20초).

## Open Questions

- [x] 언어 전환 범위 → **전면 영문화**(약 266개 + 동적 조립 37곳). part2에서 전담
- [x] 글꼴 목록 범위 → **한글 지원 글꼴만**(`HANGUL_CHARSET` 필터)
- [x] 숨김 항목에 시스템 파일 포함 → **포함**
- [x] 확장자 숨김의 원격 적용 → **적용**
- [x] 숨김 항목의 원격 적용 → **적용**(이름이 `.`으로 시작)
- [x] 두 토글의 기본값 → **현재 동작에 맞춤**(확장자 on, 숨김 on)
- [x] 중복 실행 처리 → **기존 창을 띄우고 새 프로세스는 종료**
- [x] 설정 화면 레이아웃 → **단일 스크롤 목록**
- [x] 트레이 아이콘 표시 시점 → **토글 on이면 항상 표시**
- [x] 자동 실행 시 시작 모드 → **트레이 토글이 on이면 조용히 트레이로**
- [x] 설정 저장 방식 → **바꾸는 즉시 반영·저장**, 바닥은 `닫기` 하나
- [x] plan 분할 → **둘로 나눔**(기능 / 언어)
