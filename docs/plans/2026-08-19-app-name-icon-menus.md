# 앱 이름 이중화 · 작업 표시줄 아이콘 · 새 탭 메뉴 · 메뉴 폭과 모서리

**PRD**: docs/prd.md

## 요구 이해

원문 요청(사용자, 2026-08-19 · 화면 4장 첨부):

> 1. 앱 이름이 영문/한글 모두 동일한데 영문 'MOA', 한글 '모아' 적용
> 2. 1번 이미지처럼 작업 표시줄에 앱 아이콘 표시 안되는 문제 수정.
> 3. 2번 이미지처럼 '새 탭', '다른 사이트로 새 탭 열기' 버튼이 있는데 '다른 사이트로 새 탭 열기' 버튼은 삭제하고 새 탭 아이콘을 누르면 컨텍스트 메뉴가 표시되고 메뉴에 '새 탭' 메뉴와 '다른 사이트로 새 탭 열기'에 표시되던 연결 목록을 표시해서 클릭시 탭이 표시될 수 있도록 함.
> 4. 3번 이미지처럼 설정버튼의 메뉴에서 'Open source licenses' 메뉴가 2줄로 표시가 되는데 메뉴의 가로 크기를 늘려서 1줄로 표시 되도록 함. - 한글에서 영문으로 언어를 변경하면 이런 문제가 발생. 앱을 다시 실행해서 처음부터 영문이었을 경우에는 1줄로 표시됨.
> 5. 4번 이미지처럼 원격 목록에서 컨텍스트 메뉴가 모서리가 각지게 표시 되는데 3번 이미지처럼 모서리가 둥글게 표시 되도록함. 일부 컨텍스트 메뉴가 각지게 표시되는 항목이 있는거 같은데 모두 확인해서 둥글게 수정. 이 부분은 공통 디자인으로 적용.

이해한 요구:

- **화면에 보이는 앱 이름**을 언어에 따라 가른다 — 한국어면 `모아`, 영어면 `MOA`. 적용처는 사용자가 고른 셋: 정보 대화 · 창 제목(작업 표시줄·Alt+Tab) · 트레이 아이콘 툴팁. 언어를 바꾸면 셋 다 즉시 따라간다. **데이터가 걸린 이름(레지스트리 값 `MOA`·설정 폴더 `%APPDATA%\MOA`·단일 인스턴스 뮤텍스)은 건드리지 않는다** — 바꾸면 자동 실행 등록과 기존 설정 파일을 잃는다.
- **작업 표시줄에 앱 아이콘이 뜨게 한다.** 실측으로 원인을 확정했다(아래 Investigation Log) — 창 아이콘은 정상 설정돼 있으나 작업 표시줄이 그 값을 반영하지 않으며, 창이 뜬 뒤 아이콘을 **다시 한 번** 설정하면 갱신된다.
- **탭 스트립의 `▾`(다른 사이트로 새 탭 열기) 버튼을 없애고**, `+`(새 탭)를 누르면 메뉴가 열리게 한다. 메뉴는 `새 탭` · 구분선 · 등록된 사이트 목록 순이고, 사이트 줄을 누르면 종전 `▾`와 같이 그 패널의 새 원격 탭으로 열린다.
- **설정 메뉴에 가로 폭을 지정해** 언어를 바꾼 뒤에도 `Open source licenses`가 한 줄로 보이게 한다.
- **팝업 메뉴 모서리를 6px로 통일한다** — 지금 각진 곳(원격 목록 메뉴·트리 메뉴·설정 대화 폼 드롭다운)을 이미 둥근 메뉴들과 같은 값으로 맞추고, 그 값을 공통 정본(테마)에 둔다.

## Goal

화면에 보이는 앱 이름을 언어별로 가르고, 작업 표시줄 아이콘을 실제로 뜨게 하고, 새 탭 진입점을 메뉴 하나로 합치고, 설정 메뉴 폭과 팝업 메뉴 모서리를 고친다.

## PRD Coverage

| FR | 요구 | 이번 처리 | 담당 task |
|---|---|---|---|
| FR-33 | 탭 스트립에 사이트 드롭다운 `▾`를 보이고 고른 사이트를 새 탭으로 연다 | **문면 개정** — 진입점이 `▾` 드롭다운에서 `+` 새 탭 메뉴로 바뀐다. 여는 동작(그 패널의 새 탭)은 그대로다 | T3·T6 |
| FR-53 | 고르면 재시작 없이 즉시 모든 화면 문구가 바뀐다 | **문면 개정** — 적용 범위에 창 제목·트레이 툴팁을 명시한다(종전에는 창 안 문구만 암묵 대상이었다) | T1·T6 |
| FR-58 | 정보 화면이 앱 이름과 버전을 한 줄(`MOA 0.1.0`)로 보인다 | **문면 개정** — 한국어에서는 `모아 0.1.0`이 된다 | T1·T6 |
| FR-22 | 타이틀바와 설정 메뉴 다섯 항목 | 문면 불변 — 메뉴 **폭**은 FR 문면에 없다(T4는 표시 결함 수정) | 해당 없음 |
| FR-21 | 팝업 메뉴를 고정 다크 스타일로 표시 | 문면 불변 — 모서리 반경은 FR 문면에 없다(T5는 값 통일) | 해당 없음 |
| (대응 FR 없음) | — | **T2(작업 표시줄 아이콘)는 요구 신설이 아니라 결함 수정이다** — PRD 어느 FR도 작업 표시줄 아이콘을 요구하지 않으며(FR-22는 앱이 그리는 타이틀바가 대상이다), 앱 아이콘 자산·exe 리소스는 이미 있는데 OS에 반영되지 않던 것을 고치는 일이라 문면 개정이 없다 | T2 |
| 그 밖의 active Must/Should FR | — | **이번 범위 외 (기구현)** — 이번 변경이 닿지 않는다 | — |

## Out of Scope

- **데이터가 걸린 앱 이름 문자열** — `app/autostart.rs`의 레지스트리 값 이름 `MOA`, `app/settings.rs`·`remote/hostkey.rs`의 폴더 이름 `MOA`, `app/single_instance.rs`의 뮤텍스·메시지 이름. 화면에 보이지 않으며 바꾸면 자동 실행 등록과 기존 설정·호스트 키를 잃는다.
- `app/window.rs:166`의 `w!("MOA")` — egui 이식 이전 Win32 창 제목이고 실행 파일에서 쓰이지 않는다(AGENTS.md Repository Structure 주석).
- 설정 메뉴의 `업데이트`·`릴리즈 노트` 활성화 — 이번 요청에 없다.
- 팝업 메뉴에 `ui::dialog` 셸(모서리 12px + 하단 버튼 줄)을 적용하는 것 — 2026-08-15 결정에서 **영구 제외**됐다(버튼 줄이 없어 규칙이 성립하지 않는다). 이번 T5는 그 셸이 아니라 **메뉴 모서리 값만** 통일한다.
- 사이트 드롭다운 고유 팔레트(배경 `#252525`·hover `#333333`·캡션 11px)를 다른 팝업에 퍼뜨리거나 반대로 없애는 일 — T3에서 그 팝업 자체가 사라지므로 논점이 소멸한다.

## Deferred / Follow-up

- 팝업 본문 폭이 대화마다 제각각(360·420·460·480·1080) — 이번에 손대는 것은 **메뉴** 폭이지 대화 본문 폭이 아니다. 대장에 이미 있는 항목이라 그대로 둔다.
- 트리 메뉴의 화면 밖 보정 크기가 실측이 아니라 어림값 — 이번 T5는 모서리만 만지고 크기 계산은 건드리지 않는다.
- `ui/tree.rs::menu_row`와 `ui/remote_menu.rs::menu_row`가 본문·상수까지 같다 — 이번에 두 파일을 모두 열지만 통합은 요청 밖이다.

## Investigation Log

- **위키 참조**: `20_projects/personal/moa/feat-titlebar-tray` — 타이틀바·트레이·앱 아이콘의 정본. 트레이 툴팁이 `MOA` 고정이고(72~76행 표), `app_icon`이 창 아이콘·타이틀바 텍스처·트레이 아이콘 셋에 공용으로 쓰인다고 적혀 있다. 이번에 툴팁이 언어를 따르게 되므로 이 표도 갱신 대상이 된다(위키 갱신은 별도 세션).
- **위키 참조**: `20_projects/personal/moa/feat-theme-i18n` — 언어 즉시 전환이 배선된 두 지점이 `main.rs`(창을 만들기 전 적용)와 설정 대화의 `language_changed`라고 적혀 있다. 창 제목·트레이 툴팁은 그 두 지점 어디에도 없다 — 이번에 후자에 더한다.
- **위키 참조**: `20_projects/personal/moa/feat-remote-sites` — `▾` 드롭다운이 원격 연결 네 진입점 중 하나이고 넷이 모두 같은 경로(`Command::OpenSiteTab`)로 착지한다. T3은 그 착지 경로를 바꾸지 않고 **누르는 자리만** 옮긴다.
- **위키 참조**: `20_projects/personal/moa/decisions.md` [2026-08-15] — 팝업 공통 디자인에서 **우클릭 메뉴·토스트는 영구 제외**됐다. 그 제외의 대상은 `dialog.rs` 셸(모서리 12px + 전폭 버튼 줄)이며 "버튼이 없어 규칙이 성립하지 않는다"가 사유다. 이번 T5는 셸을 씌우는 것이 아니라 메뉴 모서리 값을 6px로 맞추는 것이라 그 결정과 충돌하지 않는다.
- **위키 참조**: `20_projects/personal/moa/feat-dialog-shell` — 대화 셸의 모서리는 12px(`CORNER_RADIUS`)다. 사용자가 이번에 고른 메뉴 반경 6px과 다른 값이며, 둘은 서로 다른 부품이다.
- **Deferred 대장**(`docs/plans/deferred.md`, 대기 88건 — 잔량 임계 100건 미만이고 최고령 등록일이 2026-07-23(27일)이라 소진 batch는 열지 않는다): 「앱 이름 `"MOA"` 상수 통합」 항목이 이번 요청 1과 겹친다 — 다만 그 항목이 겨냥한 것은 **데이터 이름 네 곳의 상수 통합**이고 이번 요청은 **화면 이름의 언어별 분기**라 대상이 다르다. 화면 이름은 i18n 카탈로그가 정본이 되므로 그 항목의 대상에서 빠지고, 나머지(레지스트리·폴더·뮤텍스)는 대기에 남는다.
- **앱 이름 리터럴 전수**(`grep '"MOA"' src/ examples/ Cargo.toml`, 11건): `autostart.rs:21`(레지스트리 값) · `settings.rs:37`(폴더) · `hostkey.rs:15`(폴더) · `single_instance.rs:7`(주석) · `app/window.rs:166`(구 Win32, 미사용) · `i18n/mod.rs:147`(정보 대화) · `main.rs:53`(창 제목) · `tray.rs:36`(툴팁) · `titlebar.rs:72`(주석) · `about_dialog.rs:256·257`(시험). 화면에 보이는 것은 넷(i18n·창 제목·툴팁, 그리고 그 시험)이다.
- **작업 표시줄 아이콘 — 원인 실측(가장 중요)**. `target/release/moa.exe`를 띄워 Win32로 직접 물어본 결과:
  - `WM_GETICON` → **ICON_BIG(32×32)·ICON_SMALL(16×16)이 둘 다 설정돼 있고**, 그 HICON을 비트맵으로 꺼내 보니 **정상 MOA 아이콘**이었다. 창 클래스 아이콘은 0(winit이 `WNDCLASSEXW`에 `hIcon: 0`으로 등록한다 — winit 0.30.13 `window.rs:1417`).
  - exe 리소스 아이콘도 정상이다(`ExtractAssociatedIcon`으로 꺼내 확인 — `build.rs`가 담은 것).
  - 그럼에도 작업 표시줄에는 **Windows 기본 앱 아이콘**이 나온다(캡처 대조: MOA 미실행/실행 두 상태를 찍어 늘어난 버튼이 그것임을 확인).
  - **아이콘 캐시 가설은 기각**했다 — exe를 새 경로로 복사해 실행해도(캐시가 있을 수 없다) 같은 기본 아이콘이 나왔다.
  - **결정적 실험**: 실행 중인 창에 `WM_SETICON`(ICON_BIG·ICON_SMALL)을 **다시** 보내자 작업 표시줄 아이콘이 **즉시 MOA 아이콘으로 바뀌었다**. → 작업 표시줄은 창 아이콘을 보지만, eframe이 그것을 설정하는 시점(창이 막 보이며 활성이 된 프레임 — `eframe/src/native/app_icon.rs`가 `GetActiveWindow()`가 non-null이 될 때까지 매 프레임 재시도한다)에 **작업 표시줄이 그 갱신을 집어가지 못한다**. 이 앱은 첫 프레임을 그린 뒤에야 창을 보이므로(`ui/app.rs:943` 주석) 설정과 버튼 생성이 같은 순간에 겹친다.
  - 따라서 고치는 방법은 **창이 보인 뒤 우리가 한 번 더 `WM_SETICON`을 보내는 것**이다. HWND는 이미 `ShellHost::hwnd()`로 들고 있다.
- **설정 메뉴 2줄 — 원인 실측**. `egui::Area`가 직전 프레임의 크기를 `AreaState`에 저장하고(`egui-0.35.0/src/containers/area.rs:666`) 다음 프레임에 **그 크기를 `Ui::max_rect`로 준다**(`:466`·`:610`). 언어를 한국어→영어로 바꾸면 기억된 한국어 폭(짧다) 안에서 `Open source licenses`가 줄바꿈되고, 2줄이 된 상태로 크기가 다시 굳는다. 처음부터 영어면 첫 프레임에 기억된 크기가 없어 한 줄로 잡힌다 — 사용자가 관측한 그대로다. `Popup::menu`는 폭을 지정하지 않으면 `Area::default_width`조차 주지 않으며(`popup.rs:583`), `default_width`는 **저장된 크기가 있으면 무시**된다(`area.rs:466`의 `get_or_insert_with`). 그래서 폭은 `Popup` 인자가 아니라 **본문에서 `ui.set_width`로** 줘야 그 프레임부터 먹는다.
- **각진 팝업 전수**(`grep 'corner_radius' src/`, 6건): `dialog.rs:175`(대화 셸 12px — 대상 아님) · `widgets.rs:137`(위젯 상태 값 — 대상 아님) · **`panel.rs:1543`(원격 목록 메뉴)** · **`tree.rs:306`(트리 메뉴)** · **`widgets.rs:353`(설정 대화 폼 드롭다운)** · `site_dropdown.rs:75`(T3에서 파일째 사라진다). 셋 다 `Frame::menu(ui.style())`에 `.corner_radius(0)`을 덧붙인 형태다.
- **둥근 팝업**(기본 `Frame::menu`/`Frame::popup`을 그대로 쓰는 곳): 설정 메뉴(`titlebar.rs:253`) · 패널 메뉴(`tabs.rs:468`) · 사이드바 `+` 메뉴와 사이트 우클릭(`sidebar.rs:620·660`) · 파일 목록 컨텍스트 메뉴(`list_details.rs:649`) · 전송 큐 메뉴(`queue_panel.rs:669`). 이들이 쓰는 값은 `style.visuals.menu_corner_radius`이고 egui 0.35 기본은 **6**이다(`style.rs:1522`). 사용자가 고른 값과 같다.
- **`site_dropdown` 사용처 전수**: `ui/mod.rs:28`(모듈 선언) · `ui/tabs.rs:14`(use) · `ui/tabs.rs:243`(호출) 셋뿐이다. i18n 키 둘(`site_dropdown_open`·`site_dropdown_other`)이 그 모듈 안에서만 쓰인다.
- **`about_app_name` 사용처 전수**: `i18n/mod.rs:624`(`dynamic::about_version_line`) · `about_dialog.rs:250·254`(시험) 셋.
- **탭 스트립 호출부**: `panel.rs:1172`가 `tabs::show_tab_strip`을 부르고 `:1275`가 `strip.open_site`를 `Command::OpenSiteTab`으로 올린다. 이 계약은 T3에서 그대로 유지된다.
- **트레이 툴팁 갱신 경로**: `tray.rs:138`이 `NOTIFYICONDATAW.szTip`을 채우고 `Tray::add`가 `NIM_ADD`로 올린다. 툴팁만 바꾸려면 같은 구조체에 `NIM_MODIFY`를 보내면 된다(`Shell_NotifyIconW`는 이미 import돼 있다).
- **언어 변경 지점**: `ui/app.rs:1950`의 `if outcome.language_changed` 한 곳이다. 창 제목·트레이 툴팁 갱신을 여기 잇는다.
- **글꼴 크기**: 앱이 `text_styles`를 덮어쓰지 않아 `TextStyle::Button`은 egui 기본 12.5px이다(맑은 고딕). 설정 메뉴 최장 문구는 영어 `Open source licenses`(20자)다.

### 전제 검증

| # | 이 plan이 참으로 삼는 것 | 확인 근거 | 판정 |
|---|---|---|---|
| 1 | 작업 표시줄은 창 아이콘(`WM_SETICON`)을 본다 — 안 보는 것이 아니라 갱신을 놓친다 | 실행 중 창에 `WM_SETICON`을 재전송하니 작업 표시줄 아이콘이 즉시 MOA 아이콘으로 바뀌었다(캡처 대조) | ✅ 확인 |
| 2 | 아이콘 캐시 문제가 아니다 | exe를 새 경로로 복사해 실행해도 같은 기본 아이콘이 나왔다 | ✅ 확인 |
| 3 | exe 리소스 아이콘과 ICO 자산은 정상이다 — 고칠 것은 설정 시점뿐이다 | `ExtractAssociatedIcon` 결과가 MOA 아이콘, ICO 9항목(16~256px) 모두 32bpp | ✅ 확인 |
| 4 | HWND를 앱이 이미 들고 있어 새 획득 경로가 필요 없다 | `ShellHost::new(cc)`가 `raw-window-handle`로 얻어 `hwnd()`로 내준다(`shell_host.rs:29·48`), `app.rs:614`가 이미 그 값으로 Win32를 부른다 | ✅ 확인 |
| 5 | 설정 메뉴 2줄은 `Area`의 크기 기억 때문이며 `ui.set_width`가 그 프레임부터 이긴다 | `area.rs:466`(저장된 크기 우선)·`:610`(그것이 `max_rect`)·`:666`(프레임 끝에 저장). `set_width`는 `max_width`를 함께 올려 줄바꿈 기준을 넓힌다 | ✅ 확인 |
| 6 | 지금 둥근 메뉴들이 쓰는 값은 6px이라 사용자가 고른 값과 같다 | `egui-0.35.0/src/style.rs:1522` `menu_corner_radius: CornerRadius::same(6)`, `frame.rs:208`이 그것을 읽는다 | ✅ 확인 |
| 7 | `site_dropdown` 모듈을 지워도 다른 호출부가 깨지지 않는다 | 전수 grep 결과 참조가 `ui/mod.rs`·`ui/tabs.rs` 둘뿐 | ✅ 확인 |
| 8 | 데이터 이름(`레지스트리`·폴더·뮤텍스)을 그대로 두면 자동 실행·설정이 유지된다 | AGENTS.md 「데이터 접근」이 `HKCU\...\Run`의 `MOA` 값을 자동 실행의 **정본**으로, `%APPDATA%\MOA\settings.json`을 설정 저장처로 명시 | ✅ 확인 |
| 9 | 창이 보인 **직후**(수 ms)에 보내도 작업 표시줄이 갱신된다 | **T2에서 확정 — 전제가 겨눈 변수 자체가 틀렸다.** 지연은 원인이 아니었다(0.3초·3초 모두 실패). 갱신을 막던 것은 **같은 핸들 재설정**이었고, 비웠다가 붙이도록 고치자 0.3~1.1초에서 통과했다(캡처 대조) | ✅ 확인 (T2) |

## 동반 변경 판정

| 구분 | 항목 | 판정 근거 | 처리 |
|---|---|---|---|
| **필수** | PRD FR-33 문면 — `▾` 드롭다운이 진입점이라고 적혀 있다 | T3이 그 버튼을 없앤다. 고치지 않으면 PRD가 화면과 어긋난다 | T6에 편입 |
| **필수** | PRD FR-58 문면 — 이름·버전이 `MOA 0.1.0`이라고 적혀 있다 | T1이 한국어에서 `모아 0.1.0`으로 바꾼다 | T6에 편입 |
| **필수** | PRD FR-53 문면 — 적용 범위에 창 제목·트레이 툴팁이 없다 | T1이 그 둘을 언어에 따르게 만든다 | T6에 편입 |
| **필수** | README — 디렉터리 트리의 `site_dropdown.rs` 줄, `tabs.rs` 설명, 정보 팝업의 `MOA 0.1.0` 예시 | T3이 파일을 지우고 T1이 이름을 가른다 | T6에 편입 |
| **필수** | AGENTS.md Conventions — 팝업 메뉴 모서리 규약이 없다 | T5가 규약 시험을 새로 만든다. 이 레포는 규약 시험 3종(아이콘·화면 문구·모달)을 모두 AGENTS.md에 한 줄씩 적어 두는 패턴이라, 시험만 두면 네 번째 규약이 문서에서 빠진다 | T6에 편입 |
| **필수** | `about_dialog.rs`의 시험 두 줄(`assert_eq!(korean, "MOA")`) | T1이 한국어 값을 바꾸면 그 시험이 깨진다 | T1에 편입(자기 유발) |
| **필수** | `site_dropdown.rs`의 시험 5건 | T3이 파일을 지운다 | T3에 편입(자기 유발) |
| **필수** | `feat-titlebar-tray`(트레이 툴팁 `MOA` 고정)·`feat-remote-sites`(`▾` 진입점) 위키 페이지의 어긋나는 서술 | T1·T3이 그 둘을 바꾼다 | **T6이 대기 큐에 등재한다**(아래) — 위키 본문 갱신 자체는 별도 세션이 정본(`pjc:llm-wiki`)이라 이 plan이 수행하지 않는다 |
| **선택** | `ui/tree.rs::menu_row`와 `ui/remote_menu.rs::menu_row` 통합 | T5가 두 파일을 열지만 이 중복은 이번 변경이 만든 것이 아니다 | Deferred 유지 |
| **무관** | `ui/dialog.rs`의 12px 모서리 | 대화 셸은 메뉴가 아니며 사용자가 메뉴에 6px를 골랐다 | 건드리지 않음 |
| **무관** | `app/window.rs`의 구 Win32 창 제목 | 실행 파일에서 쓰이지 않는다(AGENTS.md 명시) | 건드리지 않음 |

## Impact Analysis

### 4-A. 심볼 추적

| 변경 대상 | 사용처(전수 grep + Read) | 담당 task |
|---|---|---|
| `i18n::about_app_name` → `i18n::app_name`(개명 + 값 변경) | `i18n/mod.rs:624`(`dynamic::about_version_line`) · `about_dialog.rs:250·254`(시험) | T1 |
| `tray::TOOLTIP`(상수 제거) | `tray.rs:138`(`icon_data`) 한 곳 | T1 |
| `main.rs:53` `with_title("MOA")` | 호출부 없음(진입점) | T1 |
| `ui::site_dropdown`(모듈 삭제) | `ui/mod.rs:28` · `ui/tabs.rs:14·243` | T3 |
| `i18n::site_dropdown_open`·`site_dropdown_other`(키 삭제) | `site_dropdown.rs` 안 3곳(본문 2 + 시험 1) | T3 |
| `tabs::show_tab_strip`의 `open_site` 출력 | `panel.rs:1172`(호출) · `:1275`(소비 → `Command::OpenSiteTab`) — **계약 불변** | T3 |
| `.corner_radius(0)` 3곳 | `panel.rs:1543` · `tree.rs:306` · `widgets.rs:353` | T5 |

### 4-B. 계약·직렬화

- 직렬화 형식 변경 없음 — 세션·설정 스키마를 건드리지 않는다.
- 공개 계약 변경은 `i18n::about_app_name` 개명 하나이며 crate 내부 호출부 셋을 함께 고친다.
- **레지스트리 값 이름·설정 폴더 이름 불변** — 자동 실행 등록과 기존 설정 파일이 그대로 읽힌다(전제 8).

### 4-C. 영향 받는 테스트

- `about_dialog.rs`의 `앱_이름은_두_언어에서_같다` 성격의 시험 2줄 → T1이 새 기대값으로 고친다.
- `site_dropdown.rs`의 시험 5건(`드롭다운_치수는_원본과_같다`·`캡션은_인벤토리_원문_그대로다`·`사이트가_없으면_버튼을_그리지_않는다`·`사이트가_있으면_버튼이_자리를_잡는다`·헬퍼) → T3이 파일과 함께 지우고, **사이트 목록이 메뉴에 실리는지 보는 새 시험**으로 대체한다.
- `i18n`의 소스 훑기 시험(`화면_문구가_카탈로그를_거치지_않은_곳이_없다`) → T1이 `tray.rs`·`main.rs`에서 리터럴을 없애는 방향이라 통과가 유지된다.
- `tests/` 통합 시험에 위 심볼을 쓰는 것은 없다(grep 확인).

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 | 재사용/신규 사유 |
|---|---|---|
| `app_icon::apply_to_window(hwnd)` | `grep 'WM_SETICON\|LoadImageW' src/` → 0건. `tray.rs::load_icon`이 ICO에서 HICON을 만들지만 **16px 고정 트레이용**이고 `CreateIconIndirect` 경로다 | **신규** — 작업 표시줄용은 OS가 크기를 골라 주는 `LoadImageW`(exe 리소스)가 맞다. 같은 모듈(`app_icon`)에 두어 아이콘 관련 코드를 한 자리에 유지한다 |
| `Tray::update_tooltip(&self)` | `tray.rs`에 `NIM_MODIFY` 사용 0건. `icon_data(hwnd, icon)`가 이미 `szTip`을 채우므로 그것을 재사용한다 | **신규(재사용 기반)** — 기존 `icon_data`를 그대로 부르고 `NIM_ADD` 대신 `NIM_MODIFY`만 보낸다 |
| `tabs::show_new_tab_menu`(내부 함수) | `site_dropdown::show_row`가 사이트 한 줄(상태 점·이름·프로토콜)을 이미 그린다 | **이전 재사용** — 그 그리기 로직을 `tabs.rs`로 옮겨 쓴다. 새로 짜지 않는다 |
| `theme::MENU_CORNER_RADIUS` | `dialog::CORNER_RADIUS`(12)가 있으나 **대화 셸 전용**이고 값이 다르다 | **신규 상수** — 메뉴 계열의 정본이 없어 각 파일이 제각각 `.corner_radius(0)`을 적었다 |
| `titlebar::SETTINGS_MENU_WIDTH` | `tree::MENU_WIDTH`·`menu::COLUMN_MENU_WIDTH` 등 메뉴별 폭 상수가 이미 관례다 | **신규 상수(관례 준수)** — 파일마다 자기 메뉴 폭을 상수로 둔다 |

### 4-E. 동반 변경 판정

위 `## 동반 변경 판정` 표 참조.

## Decisions

- **D1 (이름·값)**: 화면 앱 이름 키를 `about_app_name` → **`app_name`**으로 개명하고 값을 `"모아" / "MOA"`로 둔다. `Source`: 정보 대화 전용이 아니게 되므로 `about_` 접두사가 어긋난다. 키가 곧 함수 이름이라 개명 누락은 컴파일 오류다(`feat-theme-i18n` 위키).
- **D2 (범위)**: 앱 이름 적용처는 정보 대화·창 제목·트레이 툴팁 셋(사용자 결정). 데이터 이름 넷은 제외(Out of Scope).
- **D3 (창 제목 갱신)**: 시작 시엔 `main.rs`가 `set_language` **뒤에** `with_title(i18n::app_name())`을 부르고, 전환 시엔 `app.rs:1950`에서 `ViewportCommand::Title`을 보낸다. `Source`: `main.rs:46`이 이미 창 생성 전에 언어를 적용한다.
- **D4 (아이콘 소스)**: 작업 표시줄용 HICON은 **exe 리소스**에서 `LoadImageW(hinst, MAKEINTRESOURCE(1), IMAGE_ICON, 0, 0, LR_DEFAULTSIZE|LR_SHARED)`로 얻는다. `Source`: `build.rs`가 `GROUP_ID = 1`로 담고, `ExtractAssociatedIcon`으로 그 그룹이 읽힘을 실측했다. `LR_SHARED`면 시스템이 수명을 쥐어 `DestroyIcon`이 필요 없다(공유 아이콘 규약). ICO를 다시 파싱하는 `app_icon::decode` 경로를 쓰지 않는 이유: OS가 요청 크기에 맞는 항목을 스스로 고른다.
- **D5 (아이콘 설정 시점)**: **창이 보이는 상태가 된 뒤 잠깐 지나 1회** 보내고, 트레이에서 창을 되살릴 때(`Visible(true)`)마다 다시 보낸다. `Source`: 실측에서 창이 뜬 뒤 재전송이 먹혔고(전제 1), 작업 표시줄 버튼은 창이 보일 때 만들어진다. 지연 값은 T2가 실측으로 정한다(전제 9).
- **D6 (새 탭 메뉴 구성)**: `새 탭` · 구분선 · 사이트 목록(캡션 없음). 사용자 결정. 사이트가 하나도 없으면 `새 탭`만 남는다.
- **D7 (`+` 동작 변경)**: `+`는 이제 즉시 새 탭을 만들지 않고 메뉴를 연다. 사용자가 명시로 요청한 변경이다.
- **D8 (드롭다운 팔레트)**: `site_dropdown`의 고유 색(`#252525`/`#333333`·캡션 11px)은 그 팝업과 함께 사라지고, 새 메뉴는 **다른 메뉴와 같은 팔레트**를 쓴다. `Source`: 사용자가 "컨텍스트 메뉴"를 요청했고 요청 5가 메뉴 공통 디자인을 지시한다. 고유 색을 지키던 시험(`드롭다운_치수는_원본과_같다`)도 함께 사라진다.
- **D9 (메뉴 폭 해법)**: 설정 메뉴에 **고정 폭을 지정**한다(`ui.set_width`). 사용자 결정. 줄바꿈을 끄는 대안은 고르지 않았다.
- **D10 (모서리 정본)**: `theme::apply_dark`가 `visuals.menu_corner_radius`를 명시로 세우고, 각진 3곳은 `.corner_radius(0)`을 **지우기만** 한다(그러면 `Frame::menu`가 그 값을 읽는다). `Source`: `frame.rs:208`. 값은 6px(사용자 결정, egui 기본과 같아 지금 둥근 메뉴 5곳은 보이는 변화가 없다).

## Tasks

- [x] **T1. 화면 앱 이름을 언어별로 가른다** — Type C
  - **Design**: ① 배치 — 값은 `src/i18n/mod.rs` 카탈로그, 소비는 `about_dialog`·`main`·`tray`·`app` 넷. ② 신규 심볼 — `i18n::app_name`(개명), `Tray::update_tooltip(&self)`(툴팁만 `NIM_MODIFY`로 다시 올린다). ③ 의존 — `tray`가 `i18n`을 참조(단방향 유지, `i18n`은 아무것도 모른다). ④ 비추상화 — "앱 이름을 쓰는 곳"을 묶는 헬퍼나 트레이트를 만들지 않는다. 넷은 각자 다른 API(egui 명령·Win32 구조체·문자열)를 쓰므로 묶으면 오히려 늘어난다.
  - `i18n/mod.rs`: `about_app_name` → `app_name`, 값 `"모아" / "MOA"`. doc 주석의 "두 언어에서 값이 같아도" 서술을 새 사실로 고친다.
  - `main.rs:53`: `with_title(moa::i18n::app_name())` — `set_language` 뒤에 있음을 확인하고 둔다.
  - `tray.rs`: `TOOLTIP` 상수 제거, `icon_data`가 `i18n::app_name()`을 쓴다. `Tray::update_tooltip(&self)` 추가(`NIM_MODIFY`).
  - `app.rs:1950` `language_changed` 분기: `ctx.send_viewport_cmd(ViewportCommand::Title(...))` + `if let Some(tray) = &self.tray { tray.update_tooltip(); }`.
  - `about_dialog.rs:256·257` 시험 기대값을 `"모아"`/`"MOA"`로 고친다(**기대값은 원문 리터럴로 둔다** — AGENTS.md 「화면 문구 · 시험」 규약).
  - `app/single_instance.rs:7` 주석의 `창 제목("MOA")` 서술을 새 사실로 고친다 — 제목이 언어를 따라 갈리므로 그 표현이 어긋난다(주장 자체인 "제목으로 창을 찾지 않는다"는 그대로 유효하다).
  - **Files**: `src/i18n/mod.rs` · `src/main.rs` · `src/ui/tray.rs` · `src/ui/app.rs` · `src/ui/about_dialog.rs` · `src/app/single_instance.rs`(주석만).
  - **Edge Cases**: 트레이 아이콘이 없을 때(`tray: None`) 갱신을 건너뛴다 · `NIM_MODIFY`가 실패해도(아이콘이 이미 사라진 경우) 앱은 계속 돈다(`Drop`과 같은 취급) · `szTip`은 128 UTF-16 단위 고정이라 `모아`·`MOA` 둘 다 넉넉히 든다 · 언어를 `시스템 기본`으로 두고 시작했을 때도 `set_language`가 이미 실제 언어로 풀어 두므로 제목이 맞는다 · 창 제목이 바뀌어도 중복 실행 감지(FR-51)는 깨지지 않는다 — `single_instance`는 이름 있는 뮤텍스로 판정하고 제목으로 창을 찾지 않는다(`single_instance.rs:7` 명시).
  - **Halt Forecast**: 없음 — 파일 삭제·외부 호출·의존성 변경이 모두 없고 Files의 여섯 파일이 전부 편집 대상이다.
  - **Acceptance**: ① `cargo test` 통과 — `LanguageGuard`로 언어를 잠근 새 시험이 `app_name()`이 한국어에서 `"모아"`, 영어에서 `"MOA"`임을 단언한다. ② `dynamic::about_version_line()`이 한국어에서 `"모아 0.1.0"`을 준다(시험). ③ **화면을 그리는 코드 경로에 앱 이름 리터럴이 없다** — `grep '"MOA"' src/main.rs src/ui/tray.rs` 결과 0건(둘 다 `i18n::app_name()`을 부른다). 이 항목은 `src/` 전체를 재지 않는다: 카탈로그 정의(`i18n/mod.rs`의 영어 값)와 시험 기대값(`about_dialog.rs`·새 시험)은 **규약이 원문 리터럴을 요구**하므로 남는 것이 정상이고, 데이터용 넷(`autostart`·`settings`·`hostkey`·`single_instance`)과 구 Win32(`app/window.rs`)는 Out of Scope다. ④ `cargo clippy --all-targets -- -D warnings` 경고 0. ⑤ 창 제목·트레이 툴팁이 언어를 따라 바뀌는 것은 화면 축이라 **⏳ HUMAN-VERIFY**.

- [x] **T2. 작업 표시줄 아이콘이 뜨게 한다** — Type C
  - **Design**: ① 배치 — `src/ui/app_icon.rs`(아이콘을 다루는 기존 모듈)에 창 아이콘 적용 함수를 더하고, 호출 시점 판단은 `src/ui/app.rs`가 쥔다. ② 신규 심볼 — `app_icon::apply_to_window(hwnd: HWND)`(exe 리소스에서 큰·작은 아이콘을 얻어 `WM_SETICON` 둘을 보낸다), `ExplorerApp`에 적용 여부·시각을 들 필드. ③ 의존 — `app_icon`이 `windows` 크레이트를 새로 참조한다(같은 모듈의 기존 파서는 그대로 순수 함수로 둔다). ④ 비추상화 — "창 속성 설정" 같은 일반 계층을 만들지 않는다. `app/theme.rs`가 이미 창 Win32 호출을 개별 함수로 두는 방식과 같게 간다.
  - `app_icon.rs`: `apply_to_window` 추가 — `GetModuleHandleW(None)` + `LoadImageW(..., IMAGE_ICON, cx, cy, LR_SHARED)`로 `SM_CXICON`·`SM_CXSMICON` 크기를 각각 얻어 `SendMessageW(hwnd, WM_SETICON, ICON_BIG/ICON_SMALL, hicon)`. `unsafe`는 함수 안에 가두고 사유 주석을 단다(AGENTS.md 규약).
  - `app.rs`: 창이 보이는 상태(`!self.hidden`)가 된 뒤 **정해진 지연이 지나면 반복 전송 창(window)을 연다** — `Visible(true)`로 되살릴 때도 같은 창을 다시 연다.
  - **지연·반복은 계획에서 확정한다(추측 금지)**: 창이 보인 뒤 **300ms부터 시작해 200ms 간격으로 최대 5회**(≈1.1초까지) 보내고 멈춘다. 고정 지연 1회는 작업 표시줄 버튼 생성과의 경합에 취약하므로(리뷰 m5) 처음부터 짧은 반복으로 간다. `LR_SHARED` 아이콘이라 여러 번 보내도 핸들이 새지 않고, 같은 값을 다시 넣는 것이라 화면 깜박임도 없다. **이 값들로 ③이 통과하면 그대로 확정**하고, 통과하지 못하면 상한을 3초까지만 늘려 재시험한다(그 이상은 사용자가 아이콘이 늦게 바뀌는 것을 알아채므로 설계를 다시 본다 → 그때는 Halt).
  - **검증 수단(재현 가능하게 적어 둔다)**: ⓐ 앱을 띄우고 `Shell_TrayWnd`가 아니라 **가상 화면 전체를 `CopyFromScreen`으로 캡처**한 뒤 작업 표시줄 영역을 잘라 본다(조사 때 `Shell_TrayWnd`는 `FindWindowW`로 잡히지 않았다 — 전체 캡처 후 크롭이 확실하다). ⓑ 대조 기준은 `[System.Drawing.Icon]::ExtractAssociatedIcon(exe)`로 뽑은 exe 아이콘 그림이다. ⓒ 보조로 `SendMessageW(hwnd, WM_GETICON=0x7F, ICON_BIG=1, 0)`이 non-null이고 그 HICON을 `Icon::FromHandle(...).ToBitmap()`으로 꺼내 MOA 아이콘인지 본다. **조사 때 쓴 스크립트가 `<scratchpad>/probe_icon.ps1`·`probe_taskbar.ps1`·`probe_reset.ps1`에 남아 있으나 세션 임시 폴더라 사라질 수 있다** — 없으면 위 ⓐ~ⓒ를 그 자리에서 다시 짜면 된다(각 10줄 안팎).
  - **Files**: `src/ui/app_icon.rs`(신규 함수) · `src/ui/app.rs`(호출 시점·필드).
  - **Edge Cases**: `ShellHost`가 없으면(HWND 미획득) 조용히 건너뛴다 — 셸 메뉴와 같은 취급 · `LoadImageW`가 실패하면 아무것도 보내지 않는다(OS 기본 아이콘 유지, 앱은 계속 돈다) · `start_hidden`(자동 실행)이면 창이 보일 때까지 적용을 미룬다 · 트레이로 숨겼다 되살리면 반복 창을 다시 연다 · 반복 중에 창을 다시 숨기면 남은 회차를 버린다 · 여러 번 보내도 무해하다(`LR_SHARED`라 핸들 누수가 없다) · 반복이 도는 동안 프레임이 오지 않으면(유휴) 다음 회차가 늦어진다 — `request_repaint_after`로 그 시각에 프레임을 청한다.
  - **Halt Forecast**: 상한 3초까지 늘려도 작업 표시줄이 갱신되지 않으면 **Halt** — 그때는 창 아이콘 경로가 아니라 다른 원인(AppUserModelID 등)이라는 뜻이라 설계를 다시 세워야 한다. 그 밖에는 없다(파일 삭제·외부 호출·의존성 변경 없음).
  - **Acceptance**: ① `cargo build`·`cargo clippy --all-targets -- -D warnings` 경고 0. ② `cargo test` 통과. ③ **실측 검증** — `cargo build --release` 뒤 exe를 띄우고 위 ⓐ 방법으로 작업 표시줄을 캡처해, 그 자리의 아이콘이 ⓑ의 exe 아이콘(어두운 배경 + 노란 폴더)과 같은지 대조한다. 고치기 전 캡처는 Windows 기본 앱 아이콘(흰 창 + 청록 문서)이었다. ④ 자동 실행으로 시작(창 숨김) 후 트레이에서 창을 되살렸을 때도 아이콘이 맞는지는 **⏳ HUMAN-VERIFY**(자동 실행 등록이 필요해 캡처 절차로 재현하기 어렵다).

- [x] **T3. `+` 새 탭 버튼에 메뉴를 달고 `▾` 드롭다운을 없앤다** — Type D
  - **Design**: ① 배치 — 메뉴 그리기는 `src/ui/tabs.rs`(그 버튼이 있는 자리), 사이트 한 줄 그리기는 `site_dropdown.rs`에서 옮겨 온다. `src/ui/site_dropdown.rs`는 삭제하고 `ui/mod.rs`의 선언도 지운다. ② 신규 심볼 — `tabs::show_new_tab_menu(ui, response, remote) -> NewTabChoice`(`새 탭` 또는 사이트 하나), `tabs::show_site_row`(이전받은 그리기). ③ 의존 — `tabs`가 `site_dropdown`을 참조하던 자리가 사라지고 `remote_states::RemoteView`만 남는다. 출력 계약(`TabStripOutcome`의 `action`·`open_site`)은 **그대로**라 `panel.rs`는 손대지 않는다. ④ 비추상화 — 메뉴 항목을 데이터 배열로 돌리지 않는다. `새 탭`과 사이트 줄은 그리는 모양이 서로 달라(라벨 하나 vs 점·이름·프로토콜) 묶으면 분기가 생긴다.
  - `+` 버튼: `TabAction::New`을 즉시 내지 않고 `egui::Popup::menu(&response)`를 붙인다. 메뉴 첫 항목 `새 탭`이 `TabAction::New`을, 사이트 줄이 `open_site`를 낸다.
  - 사이트가 없으면 구분선과 목록을 그리지 않는다(`새 탭` 한 줄만).
  - i18n: `site_dropdown_open`·`site_dropdown_other` 키 삭제. `tabs_new`(`새 탭` / `New tab`)를 메뉴 라벨로 재사용한다.
  - 시험: `site_dropdown.rs`의 5건을 지우고, **사이트가 있으면 메뉴가 그 사이트를 싣고 고르면 `open_site`가 나온다** / **사이트가 없으면 `새 탭`만 남는다**를 `tabs.rs`에 새로 쓴다.
  - **Files**: `src/ui/tabs.rs` · `src/ui/site_dropdown.rs`(삭제) · `src/ui/mod.rs`(선언 제거) · `src/i18n/mod.rs`(키 2개 삭제). `src/ui/panel.rs`는 **손대지 않는다**(출력 계약 불변 — acceptance ④가 그것을 지킨다).
  - **Edge Cases**: 숨긴 사이트만 남으면 목록이 비어 `새 탭`만 보인다(`SiteStore::visible`) · 사이트 이름이 길면 종전처럼 프로토콜 라벨을 먼저 오른쪽에 붙이고 이름을 줄인다 · 메뉴를 연 채 사이트가 지워지면 다음 프레임에 목록에서 빠진다(값을 사본으로 들지 않는다) · 좁은 분할 패널에서 메뉴가 화면 밖으로 나가면 `Popup`이 스스로 자리를 고른다(`RectAlign::find_best_align`) · 이름이 지워진 사이트는 목록에 오지 않는다.
  - **Halt Forecast**: **파일 삭제**(`src/ui/site_dropdown.rs`) — 아래 `## 사전 승인 항목`에 등재한다(git 이력이 복구 경로라 되돌릴 수 있다).
  - **Acceptance**: ① `cargo test` 통과 — 새 시험 2건 포함. ② `cargo clippy --all-targets -- -D warnings` 경고 0. ③ `grep -rn 'site_dropdown' src/ README.md docs/prd.md AGENTS.md` 결과 0건 — 삭제된 파일이 README 구조 트리나 PRD에 남지 않게 한다(T6이 README를 고치므로 두 task가 끝난 뒤 이 조건이 성립한다). **과거 회차의 `docs/plans/*.md`는 대상이 아니다** — 그 시점의 기록이라 지금 사실에 맞춰 고치면 이력이 왜곡된다(구현 중 확인: `2026-08-04-ftp-integration.md` 등이 그 파일을 신설했다고 적고 있다). ④ `panel.rs`의 diff가 0줄(출력 계약 불변 확인). ⑤ 메뉴가 실제로 뜨고 사이트를 고르면 탭이 열리는 것은 **⏳ HUMAN-VERIFY**.

- [x] **T4. 설정 메뉴에 가로 폭을 준다** — Type C
  - **Design**: ① 배치 — `src/ui/titlebar.rs`의 `show_settings_menu` 한 곳. ② 신규 심볼 — `SETTINGS_MENU_WIDTH` 상수(다른 메뉴가 자기 폭 상수를 두는 관례와 같다). **구현 중 변경**: 고정 상수 대신 `settings_menu_width(ui)`가 다섯 라벨을 **실제 글꼴로 재** 최대값 + 여백을 돌려주고, `SETTINGS_MENU_MIN_WIDTH`가 하한을 잡는다 — quality 리뷰 SUGGEST 채택. 상수로 박으면 그 값이 맑은 고딕·사용자 글꼴(FR-48)에 맞는지 추정에 기대게 되는데, 재면 그 추정 자체가 사라진다(`remote_states::badge_width` 선례). ③ 의존 — 없음(파일 안에서 끝난다). ④ 비추상화 — 모든 메뉴의 폭을 공통 상수로 묶지 않는다. 메뉴마다 담는 문구 길이가 달라 한 값으로 묶으면 좁은 메뉴가 헐렁해진다.
  - `Popup::menu(&response).show(|ui| { ui.set_width(settings_menu_width(ui)); ... })`.
  - **폭은 그리는 자리에서 잰다** — 다섯 라벨을 지금 언어·지금 글꼴로 레이아웃해 가장 넓은 것에 여백을 더하고, `SETTINGS_MENU_MIN_WIDTH`로 하한을 잡는다. 고정 상수를 쓰지 않는 이유: 화면 글꼴은 맑은 고딕이고 사용자가 바꿀 수도 있어(FR-48) 상수가 맞는지 추정에 기대게 되는데, 재면 그 추정이 사라진다.
    - (계획 단계에서는 "실측해 정한 고정 상수 + 여유 비율 시험"으로 적었으나, T4 quality 리뷰의 SUGGEST를 받아 **원인을 없애는 쪽**으로 바꿨다. `remote_states::badge_width`가 같은 방식을 쓴다.)
  - 시험: 잰 폭을 줄바꿈 한계로 삼아 **두 언어의 다섯 항목이 실제로 몇 줄로 그려지는지** 본다 — 폭 계산을 되풀이해 견주면 늘 참이라 아무것도 지키지 못한다.
  - **Files**: `src/ui/titlebar.rs` 한 파일.
  - **Edge Cases**: 언어를 한국어→영어→한국어로 왕복해도 그때그때 다시 재므로 어긋나지 않는다 · 사용자가 글꼴을 바꾸면(FR-48) 그 글꼴로 재어 폭이 함께 따라간다 · 라벨이 짧아도 최소 폭 아래로는 좁아지지 않는다 · 폭을 준다고 메뉴가 화면 밖으로 나가지는 않는다(`Popup`이 자리를 고른다).
  - **Halt Forecast**: 없음 — 편집 대상이 `titlebar.rs` 한 파일이고 삭제·외부 호출·의존성 변경이 없다.
  - **Acceptance**: ① 새 시험이 두 언어 모두에서 통과한다(`LanguageGuard`로 잠근다). ② `cargo test`·`cargo clippy --all-targets -- -D warnings` 통과. ③ 한국어로 시작해 영어로 바꿨을 때 `Open source licenses`가 한 줄인 것은 **⏳ HUMAN-VERIFY**.

- [x] **T5. 팝업 메뉴 모서리를 6px로 통일한다** — Type C
  - **Design**: ① 배치 — 값의 정본은 `src/ui/theme.rs`(팔레트·스타일이 모이는 자리), 적용은 `apply_dark` 한 줄. ② 신규 심볼 — `theme::MENU_CORNER_RADIUS: u8 = 6`. ③ 의존 — 없음(egui `Visuals`에 세우면 `Frame::menu`가 알아서 읽는다). ④ 비추상화 — 메뉴 프레임을 만드는 공통 함수(`menu_frame()`)를 만들지 않는다. 각 메뉴가 채움·테두리를 조금씩 달리 쓰므로 함수로 묶으면 인자만 늘어난다 — 값만 정본화하는 것으로 충분하다.
  - `theme::apply_dark`: `visuals.menu_corner_radius = egui::CornerRadius::same(MENU_CORNER_RADIUS)`.
  - `panel.rs:1543` · `tree.rs:306` · `widgets.rs:353`의 `.corner_radius(0)`을 **지운다**(그러면 `Frame::menu` 기본이 위 값을 읽는다). 각 자리의 doc 주석이 각진 모양을 전제하면 함께 고친다.
  - 회귀 시험: `src/ui`의 `.rs`를 훑어 `Frame::menu` 뒤에 `.corner_radius(0)`을 붙인 곳이 없음을 단언한다(AGENTS.md가 세 번 쓰는 소스 훑기 규약과 같은 형태). **`src/ui` 바로 아래만 훑지 말고 하위 폴더(`src/ui/panel/`)까지 재귀로 훑는다** — 기존 대화 규약 시험이 비재귀라 하위를 놓치는 것이 이미 Deferred 대장에 올라 있다.
  - **Files**: `src/ui/theme.rs`(상수·`apply_dark`) · `src/ui/panel.rs` · `src/ui/tree.rs` · `src/ui/widgets.rs`.
  - **Edge Cases**: 지금 둥근 메뉴 5곳은 값이 그대로라 보이는 변화가 없다 · 대화 셸(12px)은 `Frame::menu`를 쓰지 않아 영향이 없다 · `widgets.rs:137`의 `state.corner_radius = ZERO`는 메뉴가 아니라 위젯 상태값이라 대상이 아니다 · 메뉴 안 hover 채움은 각진 채로 그려지는데, 항목이 프레임 안쪽 여백(`menu_margin` 6) 안에 있어 둥근 모서리 밖으로 새지 않는다.
  - **Halt Forecast**: 없음 — Files의 네 파일이 전부 편집 대상이고 삭제·외부 호출·의존성 변경이 없다.
  - **Acceptance**: ① 새 소스 훑기 시험이 통과한다. ② `cargo test`·`cargo clippy --all-targets -- -D warnings` 통과. ③ `grep -rn 'corner_radius(0)' src/ui/` 결과가 0건(`site_dropdown`은 T3에서 사라진다). ④ 원격 목록·트리 메뉴 모서리가 실제로 둥근 것은 **⏳ HUMAN-VERIFY**.

- [x] **T6. PRD·README·AGENTS.md 정합** — Type A
  - `docs/prd.md`: FR-33 문면을 새 진입점(`+` 새 탭 메뉴)으로 고친다 · FR-53 문면에 창 제목·트레이 툴팁이 언어를 따른다는 것을 더한다 · FR-58 문면의 `MOA 0.1.0` 예시에 한국어 `모아 0.1.0`을 병기한다 · 변경 이력에 2026-08-19 항목을 더한다.
  - `AGENTS.md` Conventions: **「팝업 메뉴」 규약 한 줄**을 더한다 — 메뉴 모서리의 정본이 `theme::MENU_CORNER_RADIUS`이고 `Frame::menu`에 `.corner_radius(0)`을 덧붙이지 않으며 T5의 소스 훑기 시험이 그것을 지킨다는 것. 이 레포는 같은 형태(규약 한 줄 + 시험 이름)를 아이콘·화면 문구·모달 대화 셋에 이미 쓰고 있어(85행 「모달 대화」가 그 예), 시험만 만들고 문서를 비워 두면 규약이 네 번째만 어긋난다.
  - `README.md`(고칠 세 자리를 실물에서 특정했다): **96행** 디렉터리 트리의 `site_dropdown.rs # 탭 스트립의 사이트 드롭다운·연결 메뉴` 줄을 지운다 · **84행** `tabs.rs # 탭 스트립 + 오른쪽 끝 패널 메뉴 버튼`에 새 탭 메뉴를 더한다 · **17행** 앱 정보 서술의 `MOA 0.1.0` 예시에 한국어 `모아 0.1.0`을 병기한다. (README 본문에 `▾` 표기는 없다 — 확인함.)
  - **위키 대기 큐 등재**(동반 변경 판정의 위키 행을 여기서 소진한다): `feat-titlebar-tray`의 트레이 툴팁 `MOA` 고정 서술과 `feat-remote-sites`의 `▾` 진입점 서술이 이번 변경으로 어긋난다는 것을 `pjc:llm-wiki` 절차가 정한 대기 큐에 1줄씩 적는다. **위키 본문은 이 plan이 고치지 않는다**(별도 세션 소관) — 적어 두지 않으면 그 어긋남을 아무도 회수하지 못한다.
  - **Files**: `docs/prd.md` · `AGENTS.md` · `README.md`(+ 위키 대기 큐 1줄 — 레포 밖).
  - **Edge Cases**: 없음(문서만).
  - **Halt Forecast**: 없음 — 문서 편집뿐이고 삭제·외부 발행이 없다(위키 큐 등재는 기록 1줄이며 위키 본문을 고치지 않는다).
  - **Acceptance**: ① `grep -rn '▾' docs/prd.md` 결과에 옛 진입점 서술이 남지 않는다(README에는 애초에 없다). ② `grep -rn 'site_dropdown' README.md` 결과 0건. ③ PRD의 FR-33·FR-53·FR-58 문면이 T1·T3 산출물과 한 줄씩 대조된다. ④ AGENTS.md Conventions에 팝업 메뉴 규약 줄이 있고 거기 적힌 시험 이름이 T5가 실제로 만든 시험 이름과 같다. ⑤ README에 존재하지 않는 기능이 남지 않는다.

## 사전 승인 항목 (일괄 승인 대상)

- **`src/ui/site_dropdown.rs` 파일 삭제**(T3) — 그 팝업이 새 탭 메뉴로 대체되므로 남기면 죽은 코드가 된다. 복구 경로는 git 이력이다. `ui/mod.rs`의 모듈 선언도 함께 지운다.
- **`i18n` 키 2개 삭제**(`site_dropdown_open`·`site_dropdown_other` — T3) — 쓰는 곳이 함께 사라진다.
- **`i18n::about_app_name` → `app_name` 개명**(T1) — crate 내부 심볼이며 호출부 셋을 같은 task에서 고친다.
- **로컬 작업 브랜치 commit** — task별로 나눠 커밋한다.

## 불가피한 Halt (위임 불가)

- push · master 병합 · 태그 · 릴리즈 · PR — 구현·검증을 마친 뒤 별도로 승인받는다.
- 위 사전 승인 항목 밖의 파일 삭제·이동·이름 변경.
- 의존성 추가·버전 변경 — **이번 계획에는 없다(확인함).** windows 0.62.2 소스에서 소재를 대조했다: `LoadImageW`·`SendMessageW`·`GetSystemMetrics`·`IMAGE_ICON`·`LR_SHARED`·`WM_SETICON`·`ICON_BIG`은 `Win32_UI_WindowsAndMessaging`, `GetModuleHandleW`는 `Win32_System_LibraryLoader`에 있고 둘 다 `Cargo.toml`에 이미 켜져 있다. `NIM_MODIFY`(T1)는 이미 쓰는 `Shell_NotifyIconW`와 같은 `Win32_UI_Shell`이다. 그럼에도 feature가 모자라면 그때 Halt.

## Open Questions

- [x] Q1. 앱 이름 이중화를 어디까지 적용하나 → **정보 대화 + 창 제목 + 트레이 툴팁 셋 전부**(D2).
- [x] Q2. `+` 메뉴 구성 → **`새 탭` + 구분선 + 사이트 목록**, 캡션 줄은 두지 않는다(D6).
- [x] Q3. 설정 메뉴 2줄 해법 → **메뉴에 고정 폭 지정**(D9).
- [x] Q4. 메뉴 모서리 반경 → **6px**(지금 둥근 메뉴와 같은 값, D10).

## 리뷰 이력

| 라운드 | 지적 | 심각도 | 반영 방식 |
|---|---|---|---|
| 1 | M1 — T1 acceptance ③이 자기 모순(카탈로그 영어 값·시험 기대값도 `"MOA"`라 "데이터용 넷만 남는다"가 성립 불가) | MAJOR | **수용** — ③을 "화면을 그리는 코드 경로(`main.rs`·`tray.rs`)에 리터럴 0건"으로 좁히고, 규약상 남아야 하는 것(카탈로그 정의·시험 기대값)을 명시 |
| 1 | M2 — T2의 실측 검증 방법이 역참조뿐이고 지연 값도 그 루프로 정하게 돼 있어 구현자가 추측한다 | MAJOR | **수용** — 검증 수단 ⓐ~ⓒ(전체 화면 캡처 후 크롭 / `ExtractAssociatedIcon` 대조 / `WM_GETICON` 확인)를 본문에 적고, 지연을 **300ms 시작·200ms 간격·5회**로 계획에서 확정. 상한(3초) 초과 시 Halt를 명시 |
| 1 | M3 — `README.md:96`의 `site_dropdown.rs` 구조 트리 줄을 어느 게이트도 잡지 못한다 | MAJOR | **수용**(일부는 리뷰 전 이미 반영) — T6에 그 줄 삭제를 명시하고, T3 acceptance ③의 grep 범위를 `src/ README.md docs/`로 넓혀 두 게이트가 함께 잡게 함 |
| 1 | m1 — T4·T5의 Halt Forecast가 근거 없는 "없음" | MINOR | **수용** — 두 task 모두 근거(편집 파일이 Files에 전부 있고 삭제·외부 호출 없음)를 한 줄로 적음. T1·T2·T6도 같이 보강 |
| 1 | m2 — `single_instance.rs:7` 주석의 `창 제목("MOA")`이 어긋난다 | MINOR | **수용** — T1에 주석 정정을 편입하고 Files에 그 파일을 더함. 아울러 "제목이 바뀌어도 FR-51은 안전"을 Edge Case에 기록 |
| 1 | m3 — task에 `Files:` 필드가 없어 파일 대조가 산문 해석에 의존 | MINOR | **수용** — 여섯 task 전부에 `Files` 추가 |
| 1 | m4 — 위키 갱신이 "큐로 넘긴다"인데 그 등록을 수행하는 task가 없다 | MINOR | **수용** — T6의 단계로 명시(등재만 하고 위키 본문은 별도 세션) |
| 1 | m5 — 고정 지연 1회 전송은 경합에 취약, 반복 전송 대안을 미리 적으면 실측이 짧아진다 | MINOR | **수용** — 처음부터 반복 전송(5회)으로 설계. M2 반영과 같은 자리 |
| 1 | m6 — PRD Coverage에 T2 행이 없다 | MINOR | **수용** — "대응 FR 없음(결함 수정)" 행을 표에 추가 |
| 1 | m7 — 요청 5에 `## 시각 요소 분해` 섹션이 없다(판정 유보) | MINOR | **유지** — 리뷰어도 "대상 속성이 모서리 반경 하나이고 전수(6건)·값(6px)이 Investigation Log에 확정돼 실질 분해가 이뤄졌다"고 판단했다. 별도 섹션을 만들면 한 줄짜리 표가 된다 |
| 1 | m8 — T4 폭 시험은 기본 글꼴로 재는데 실제는 맑은 고딕·사용자 글꼴 | MINOR | **수용** — 값에 여유를 두고, 시험이 `최대 폭 ≤ 폭 × 0.75`로 **여유가 남았는지까지** 단언하게 함 |

리뷰어 총평: BLOCKER 0. 코드 안 호출부 누락은 발견되지 않았고(`about_app_name`·`site_dropdown`·`corner_radius` 6건·`TOOLTIP`·`show_tab_strip` 모두 plan 표와 일치), Investigation Log의 외부 근거(egui `style.rs:1522`·`frame.rs:208`·`area.rs:466·666`)도 실물과 맞음을 확인했다. 전제 9의 ⚠ 미확인이 "성립이 아니라 지연 값만 좌우한다"는 판정에도 동의. Type 분류(T2가 C, T6이 A 포함)도 타당하다고 판정했다.

## Phase Ledger

- **Phase G 통과 (Must 100%)** — F-7의 PRD 전수 대조에서 기존 Must FR 무회귀를 확인했다. 이번 회차가 문면을 개정한 셋은 Should FR-33·FR-58과 Must FR-53이며, FR-53은 적용 범위를 넓힌 것(창 제목·트레이 툴팁 추가)이라 종전 요구를 그대로 포함한다. 갭 0이라 재루프 없음. 화면 축 6항목은 아래 ⏳ HUMAN-VERIFY로 등재.
- **Phase F 통과 (HEAD 116fc46)** — F-7: BLOCKER 0 / MAJOR 0 / MINOR 4. m1(규약 시험이 `.corner_radius(0)` 리터럴만 잡아 문면보다 좁음)과 m3(`+` 버튼 hover 문구가 바뀐 동작과 어긋남)은 이번 변경이 유발한 것이라 그 자리에서 고쳤다. m2(검토 입력 SHA가 실제 HEAD보다 2 커밋 뒤 — 둘 다 문서 전용)는 보고 기준을 실제 HEAD로 잡는 것으로 갈음. m4(새 탭 메뉴는 고정 폭 250px인데 설정 메뉴는 실측 — 판정 유보)는 원본 드롭다운 치수를 그대로 옮긴 결과이고 라벨이 짧아 접힘이 재현되지 않아 조치하지 않았다.

### ⏳ HUMAN-VERIFY (사용자 확인 필요 — 빌드·시험으로 판정할 수 없는 축)

작업 표시줄 아이콘(요청 2)은 캡처 대조로 **검증을 마쳤다**(release·debug 양쪽). 나머지 화면 축은 메뉴를 눌러 열어야 보이는 것이라 아래로 넘긴다 — 창 위치·포커스가 흔들려 UI 자동 조작으로는 신뢰할 만한 캡처를 얻지 못했다(2회 시도 후 중단).

1. **설정 메뉴(⚙)를 한국어로 열었다가 언어를 English로 바꾼 뒤 다시 연다** — `Open source licenses`가 한 줄로 서는가.
2. **원격 목록에서 우클릭** — 메뉴 네 모서리가 둥근가(설정 메뉴와 같은 정도).
3. **폴더 트리에서 우클릭 · 설정 대화의 글꼴 드롭다운** — 위와 같은 모서리인가.
4. **탭 줄의 `+`를 누른다** — `새 탭` 아래 구분선과 사이트 목록(상태 점·이름·프로토콜)이 뜨는가. 사이트를 고르면 그 패널에 원격 탭이 열리는가. 등록된 사이트가 없으면 `새 탭` 한 줄만 남는가.
5. **설정 → 언어를 바꾼다** — 창 제목(작업 표시줄에 마우스를 올리거나 Alt+Tab)과 트레이 아이콘 툴팁이 `모아`/`MOA`로 함께 바뀌는가. 설정 → 정보의 이름·버전 줄도 같이 바뀌는가.
6. **자동 실행으로 시작(창 숨김) 후 트레이에서 창을 되살린다** — 그때도 작업 표시줄 아이콘이 MOA 아이콘인가.

## Retry Ledger

## Progress Log

- **Phase F에서 T2가 한 번 더 드러났다(중요)**: 전 task를 마치고 release로 통합 검증하니 **release 빌드에서만** 작업 표시줄이 다시 기본 아이콘이었다(debug는 통과). 원인은 **eframe도 창 아이콘을 설정하는데 그 시점이 실행마다 다르다**는 것 — eframe은 `GetActiveWindow()`가 잡힐 때까지 매 프레임 재시도하므로 시작이 빠른 release에서는 우리 전송(0.3~1.1초)이 그보다 앞서고, 뒤이은 eframe의 설정이 다시 「같은 값이라 갱신이 감지되지 않는」 상태를 만든다. 전송 창을 **0.3초부터 0.4초 간격 여덟 번(3.1초까지)**으로 넓혀 어느 쪽이 먼저 오든 마지막 회차가 뒤에 오게 했고, release·debug 양쪽에서 캡처로 확인했다. — **task 단위 검증을 debug로만 하면 이 차이를 놓친다**는 것이 이번의 교훈이다.

- **T4-T6 완료**: 설정 메뉴 폭(실측), 메뉴 모서리 6px 통일(정본 + 규약 시험), 문서 정합. T4는 리뷰 SUGGEST를 받아 고정 상수를 버리고 그리는 자리에서 재는 방식으로 바꿨고, 그 과정에서 시험이 구현 계산을 되풀이해 늘 참이던 것을 **실제 줄 수 관측**으로 교체했다. T5의 규약 시험은 하위 폴더까지 재귀로 훑는다(모달 규약 시험의 비재귀 함정을 반복하지 않았다).

- **T1-T3 완료**: 앱 이름 언어 분기(정보 대화·창 제목·트레이 툴팁) · 작업 표시줄 아이콘 · `+` 새 탭 메뉴. `src/ui/site_dropdown.rs`를 지우고 그 그리기를 `tabs.rs`로 옮겼으며, 출력 계약(`TabStripOutcome`)이 그대로라 `panel.rs`는 한 줄도 바뀌지 않았다. 시험은 팝업을 눌러 여는 대신 메뉴 **내용 함수**(`new_tab_menu_items`)를 직접 그려 실린 글자를 단언한다 — 설정 대화가 같은 이유로 쓰던 형태다.

- **T2에서 원인이 계획과 달랐다(중요 — 전제 9 확정)**: 계획은 "eframe의 전송 시점이 작업 표시줄 버튼 생성과 겹쳐 갱신이 누락된다"고 보아 **지연**을 변수로 잡았으나, 실측 결과 지연은 원인이 아니었다. 0.3~1.1초도 3초까지 늘린 것도 실패했고, **바깥에서 다른 핸들을 보냈을 때만** 성공했다. 진짜 원인은 **같은 핸들을 다시 넣으면 값이 변하지 않아 작업 표시줄이 갱신을 감지하지 못한다**는 것이다. 그래서 `WM_SETICON`으로 **NULL을 먼저 보내 비운 뒤** 실제 아이콘을 붙이도록 고쳤고, 그 뒤에는 처음 계획했던 0.3~1.1초 범위에서 곧바로 통과했다(캡처 대조). 반복 전송은 남겨 두었다 — 작업 표시줄 버튼이 아직 없을 때 보낸 회차가 헛도는 것에 대한 안전망이다.
