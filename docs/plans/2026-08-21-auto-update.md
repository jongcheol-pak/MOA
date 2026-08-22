# Plan: GitHub 릴리즈 기반 자동 업데이트 + 릴리즈 노트 열기

**PRD**: `docs/prd.md` (FR-62·FR-63 신설과 Out of Scope 재한정을 포함한다 — T1)

## 요구 이해

- **원문 요청**:
  > 1. 업데이트 기능 구현
  > - 앱이 실행되면 업데이트를 확인. 업데이트는 비동기 식으로 확인해서 앱 실행에 영향을 주면 안됨. (업데이트 확인은 앱 실행시 한번만 확인)
  > - 이미지처럼 타이틀바 오른쪽에 업데이트가 있는경우 표시. 업데이트가 있으면 '업데이트' 표시, '업데이트' 클릭하면 '다운로드 중...' 으로 표시하고 설치 파일을 다운로드
  > - 설치 파일 다운로드는 설치 폴더에 'update' 폴데이트 폴더를 생성하고 다운로드 함.
  > - 다운로드가 완료되면 설치 화면이 표시 되고 자동으로 실행 중인 앱을 종료하고 설치를 진행, 설치가 완료되면 앱을 다시 실행 함.
  > - 업데이트가 완료되면 다운로드 받은 파일은 삭제 함.
  > - 업데이트 확인 및 파일은 git에 릴리즈된 파일 확인.
  > - 메뉴에서 업데이트 메뉴클릭시 업데이트 체크
  > 2. 릴리즈 노트 메뉴 클릭시 git 릴리즈 페이지 연결

- **이해한 요구**: 지금 타이틀바 설정 메뉴의 `업데이트`·`릴리즈 노트` 두 항목은 비활성으로 표시만 되어 있다(FR-22·`titlebar.rs`의 `pending_item`). 그 둘에 실제 기능을 넣는다. ① 앱이 뜰 때 **워커 스레드에서 한 번** GitHub Releases API를 물어 최신 버전을 확인하고, 앱 시작은 그 결과를 기다리지 않는다 ② 새 버전이 있으면 타이틀바 오른쪽(설정 기어 왼쪽)에 아이콘 + `업데이트`를 띄우고, 누르면 `다운로드 중...`으로 바뀌며 설치 파일을 **exe 옆 `update\` 폴더**로 받는다 ③ 다 받으면 설치 화면을 띄우고, 설치 프로그램이 실행 중인 앱을 닫고 덮어쓴 뒤 앱을 다시 띄운다 ④ 다시 뜬 앱이 `update\` 폴더를 지운다 ⑤ 설정 메뉴의 `업데이트`를 누르면 그 자리에서 다시 확인한다 ⑥ `릴리즈 노트`를 누르면 기본 브라우저로 GitHub 릴리즈 페이지를 연다.
- **추가 요청(2026-08-22)**:
  > "릴리즈 발행시 릴리즈 노트에 요약 정리해서 간단하게 표시 항목당 1~2 라인 정도, 불필요한 내용은 제외, 릴리즈 노트는 일반 사용자들이 보기 때문에 내용이 너무 길거나 내용이 너무 어려우면 안됨, 이 내용 확인해서 앞으로 릴리즈 발행시 릴리즈 노트 작성 하도록 함"

  릴리즈 노트를 **일반 사용자가 읽는 글**로 못 박는다 — 항목당 1~2줄, 불필요한 것은 빼고, 길거나 어렵게 쓰지 않는다. 이번 한 번이 아니라 **앞으로의 모든 릴리즈**에 적용되는 규약이므로 `AGENTS.md`에 남긴다(T9). 첫 릴리즈(v0.1.0) 노트도 이 규칙으로 쓴다.
- **함께 정하고 넣는 것(사용자 결정)**: 받은 파일의 **SHA256 무결성 검증**(릴리즈 노트 본문에 적힌 값과 대조, 어긋나면 지우고 중단), 설치 프로그램의 **업데이트 모드 인자**(`/UPDATE` — 언어·환영·라이선스·폴더 페이지를 건너뛰고 진행 화면만 보인 뒤 자동으로 앱 실행), **설치본에서만** 이 기능을 켜는 것, **전송이 도는 중이면 확인 대화**를 띄우는 것.
- **포함하지 않는 것으로 이해**: 다운로드 진행률(퍼센트·막대) 표시, 업데이트 채널·베타 구분, 코드 서명, 델타(차분) 업데이트, 자동 설치(사용자가 누르지 않으면 받지 않는다), 이전 버전으로 되돌리기.

## Goal

앱을 띄우면 백그라운드로 최신 릴리즈를 확인하고, 새 버전이 있으면 타이틀바에서 한 번 눌러 받아 설치까지 끝난 뒤 앱이 새 버전으로 다시 뜬다. 설정 메뉴의 `업데이트`·`릴리즈 노트` 두 항목이 비활성을 벗는다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-62 (자동 업데이트) **신설** | Should | T1(문면)·T2~T7·T8 | 커버 |
| FR-63 (릴리즈 노트 열기) **신설** | Should | T1(문면)·T7 | 커버 |
| FR-22 (타이틀바 설정 메뉴) **개정** | Should | T1(문면)·T7 | 커버 — "나머지 둘은 항목 표시만 한다"가 거짓이 된다 |
| Out of Scope 「`업데이트`·`릴리즈 노트` 계속 제외」 **재한정** | — | T1 | 커버 — 이번 요청이 그 제외를 뒤집는다 |
| NFR-1 (시작 1초) | — | T6 | 커버 — 확인이 워커라 시작 경로를 막지 않는다 |
| 그 밖의 active Must FR (FR-1~FR-8·FR-13·FR-15~FR-17·FR-27~FR-32·FR-36·FR-37·FR-47~FR-53·FR-55·FR-59) | Must | — | **이번 범위 외 (기구현)** — 이 변경이 닿지 않는다 |

## Out of Scope

- 다운로드 **진행률** 표시(퍼센트·막대) — 요청은 `다운로드 중...` 문구만 요구한다
- 업데이트 채널·프리릴리즈 구분 — `releases/latest` 하나만 본다
- 코드 서명·델타 업데이트·자동 설치·롤백
- 개발 빌드(설치본이 아닌 실행)의 업데이트 — 사용자 결정, 확인 자체를 하지 않는다

## Deferred / Follow-up

- **다운로드 진행률 표시** — 지금은 `다운로드 중...` 문구뿐이라 몇 MB 중 얼마를 받았는지 알 수 없다. WinHTTP는 `Content-Length`와 읽은 바이트를 다 쥐고 있어 값 자체는 있다(T2가 그것을 버린다). 요청 범위 밖이라 다음 회차로.
- **업데이트 확인 실패의 조용한 처리** — 시작 시 확인이 실패하면(네트워크 없음·API 한도) 아무것도 보이지 않는다. 메뉴에서 손으로 눌렀을 때만 토스트로 알린다(T7). 실패를 늘 알리는 편이 나은지는 써 보고 정한다.
- ~~**릴리즈·태그 규약 문서화**~~ → **이번 회차 T9로 재수용**. SHA256을 릴리즈 노트에 적는 절차가 이 기능의 전제라 미룰 수 없다(동반 변경 판정 「필수」).

## Investigation Log

- **타이틀바 설정 메뉴** — `src/ui/titlebar.rs:262~296`의 `show_settings_menu`가 다섯 항목을 그리고, `업데이트`·`릴리즈 노트`는 `pending_item`(`ui.add_enabled(false, …)`)으로 비활성이다. 메뉴 폭은 `settings_menu_width`가 다섯 라벨을 재서 정한다.
- **우측 버튼군 폭** — `RIGHT_GROUP_WIDTH = BUTTON_SIZE(36) + CAPTION_WIDTH(46) × 3 = 174`. 이 값이 **셋을** 함께 정한다 — **창 끌기 영역**(`drag_right`, 106)·**제목 최대 폭**(`title_width`, 142)·**위쪽 변 크기 조절 비켜주기**(`over_titlebar_button`, 387). 배지를 넣으면 세 곳이 같이 움직인다. **셋째를 빠뜨리면 배지 상단 4px에서 크기 조절 루프가 열려 그 클릭이 삼켜진다**(titlebar.rs:343~344 주석이 막으려던 바로 그 버그).
- **명령 경로** — `ui::menu::Command`(`src/ui/menu.rs:52~85`)가 타이틀바의 요청을 값으로 나르고, `ui::app::apply_command`(`src/ui/app.rs:1694~1746`)가 실행한다. `OpenLicenses`·`OpenAbout`가 그 본보기다(1731~1733).
- **HTTP 수단이 없다** — `Cargo.toml`에 HTTP 크레이트가 없고(`reqwest`·`ureq`·`curl` 0건), `suppaftp`·`ssh2`는 각자 프로토콜 전용이다. `std::process::Command`로 외부 프로세스를 띄우는 곳도 `src/`에 0건이다(`process::id`만 쓴다).
- **WinHTTP는 이미 쓰는 크레이트 안에 있다** — `windows` 0.62.2의 `Win32_Networking_WinHttp` feature(레지스트리 캐시 `Cargo.toml:505`)를 켜면 `WinHttpOpen`·`WinHttpConnect`·`WinHttpOpenRequest`·`WinHttpSendRequest`·`WinHttpReceiveResponse`·`WinHttpQueryHeaders`·`WinHttpReadData`·`WinHttpCloseHandle`을 쓸 수 있다(모듈 소스에서 시그니처 직접 확인). **신규 패키지 0건.**
- **SHA256도 이미 켠 feature 안에 있다** — `Win32_Security_Cryptography`(DPAPI 때문에 이미 켜져 있다)에 `BCryptCreateHash`·`BCryptHashData`·`BCryptFinishHash`가 있다(`Cryptography/mod.rs:59`). `remote::envelope`가 같은 CNG 계열로 PBKDF2-HMAC-SHA256·AES-GCM을 쓰고 있어 핸들 RAII·`NTSTATUS` 판정 관례가 그 파일에 이미 있다(`envelope.rs:214~300`).
- **설치 스크립트에 이미 있는 것** — `installer/moa.nsi`의 `Section "Install"`이 `taskkill`로 실행 중인 앱을 닫고(정상 종료 → 2초 → 강제 → 0.5초), `MUI_FINISHPAGE_RUN`이 마침 페이지에 앱 실행 체크박스를 둔다. 설치 폴더는 `$LOCALAPPDATA\Programs\MOA` 고정이고 `RequestExecutionLevel user`라 UAC가 없다.
- **NSIS 인자 읽기 수단 확인** — `C:\Program Files (x86)\NSIS\Include\FileFunc.nsh:1436`에 `!define GetParameters`가 있다. makensis도 그 자리에 있다.
- **전송 진행 판정 수단** — `remote::queue::TransferQueue::count(QueueFilter)`(`queue.rs:307`)와 `TransferState::is_active`(`queue.rs:40`)로 셀 수 있다. 새로 만들 것이 없다.
- **GitHub 릴리즈가 0건이다** — `gh release list`가 빈 결과, `git tag`도 0건. 원격은 `https://github.com/jongcheol-pak/MOA.git`이고 `gh`는 `jongcheol-pak`으로 로그인돼 있다.
- **위키 참조: `20_projects/personal/taskmon/feat-auto-update.md`** — 같은 사용자의 TaskMon이 같은 기능을 구현했다. 얻은 것 넷: ⓐ GitHub API는 **`User-Agent` 헤더가 없으면 403**이다 ⓑ 릴리즈 자산 URL은 **S3로 리다이렉트**되므로 추종이 필수다 ⓒ 버전 비교는 `v` 접두사를 떼고 `.`으로 갈라 수로 견준다 ⓓ 자기 갱신은 설치 프로그램을 띄운 뒤 **짧은 지연을 두고** 종료해야 파일 잠금이 풀린다.
- **위키 참조: `40_guides/recipes/rust/installer-sha256-verify.md`** — 릴리즈 노트 본문에서 체크섬을 뽑는 방법이 정규식 없이 「비-hex 문자로 토큰을 가르고 길이 64짜리를 고른다」다. 불일치 시 파일을 즉시 지운다.
- **위키 참조: `20_projects/personal/moa/decisions.md`** — 2026-08-21 결정에 *"코드 서명·자동 업데이트·MSI는 범위 밖으로 **기각**"*이 있다. **이번 요청이 그 기각을 뒤집는다** — PRD Out of Scope와 함께 고쳐야 할 자리다(T1).
- **위키 참조: 관련 recipe 외 MOA 쪽 자동 업데이트 자료 없음** — `20_projects/personal/moa/`에 그 주제 feature 페이지가 없다(목록 확인).
- **Deferred 대장** — `docs/plans/deferred.md` `## 대기` 81건. 이번 요청과 걸리는 것은 **「릴리즈·태그 규약 문서화」(2026-08-21)** 하나이며 T9로 재수용한다. 나머지 80건은 주제가 닿지 않고, 제목 스캔에서 이 plan의 전제를 부정하는 항목도 없다. 잔량 81건 < 100건, 절대 상한 130건 미만, 최고령 판정일이 2026-07-23이나 신규 등재분 30건 조건과 AND라 **소진 batch는 착수하지 않는다**.

### 전제 검증

| # | 이 plan이 참으로 삼는 것 | 확인 근거 | 상태 |
|---|---|---|---|
| 1 | `windows` 0.62.2에 `Win32_Networking_WinHttp` feature가 있다 | 크레이트 `Cargo.toml:505` 직접 확인 | ✅ |
| 2 | 그 feature의 WinHTTP 함수 8종을 쓸 수 있다 | `Networking/WinHttp/mod.rs`에서 시그니처 8건 확인 | ✅ |
| 3 | `BCryptCreateHash`가 **이미 켠** `Win32_Security_Cryptography`에 있다 | `Security/Cryptography/mod.rs:59` 확인 | ✅ |
| 4 | feature만 더하면 `Cargo.lock`이 바뀌지 않아 라이선스 자산을 다시 만들 필요가 없다 | `Cargo.lock`의 `windows` 항목에 feature 목록이 없다(이름·버전·checksum·dependencies뿐) — 지문(`lockfile_fingerprint`)은 이름·버전 줄만 접는다(`app/licenses.rs:114`) | ✅ |
| 5 | NSIS로 명령행 인자를 읽을 수 있다 | `FileFunc.nsh:1436`의 `GetParameters` 확인, makensis 설치 확인 | ✅ |
| 6 | `moa.nsi`가 이미 실행 중인 앱을 닫는다 | `Section "Install"`의 `taskkill` 2회 + 대기 확인 | ✅ |
| 7 | 전송 진행 건수를 셀 수 있다 | `queue.rs:307`의 `count(QueueFilter)` 확인 | ✅ |
| 8 | 설치본과 개발 빌드를 가릴 수 있다 | 설치 시 `WriteUninstaller "$INSTDIR\uninstall.exe"`가 그 폴더에만 생긴다(`moa.nsi`) — exe 옆 `uninstall.exe` 존재로 가른다 | ✅ |
| 9 | 릴리즈 자산 이름이 `MOA-Setup-<버전>.exe`다 | `moa.nsi`의 `OutFile "..\target\installer\MOA-Setup-${VERSION}.exe"` 확인 | ✅ |
| 10 | egui `Context::open_url`로 기본 브라우저를 연다 | `about_dialog.rs:155`의 `hyperlink_to`가 이미 그 경로로 저장소 주소를 연다(같은 앱에서 동작 중) | ✅ |
| 11 | GitHub API 비인증 한도는 시간당 60회다 | ⚠ **미확인** — 문서 대조 못 함. **성립을 좌우하지 않는다**: 앱 실행당 1회 + 손으로 누를 때만이라 한도에 닿기 어렵고, 닿으면 확인 실패로 떨어져 조용히 넘어간다(T6 Edge) | ⚠ 미확인 (비결정적) |
| 12 | 설치 프로그램이 앱을 닫는 사이에 앱이 세션을 저장할 틈이 있다 | `taskkill` 무옵션(정상 종료 요청) 뒤 2초 대기 — `moa.nsi` 확인. 앱은 0.4초 조용해지면 저장한다(FR-11, PRD 결정 이력 2026-08-21) | ✅ |

## Risks & Unknowns

- **R1. 실제 릴리즈로 끝까지 확인하려면 릴리즈가 있어야 한다.** 지금 0건이라 T2~T7은 릴리즈가 생기기 전까지 **가짜 응답(JSON 문자열)으로만** 검증된다. 첫 릴리즈 발행(별도 승인)이 붙어야 전 구간 확인이 끝난다.
- **R2. WinHTTP FFI는 이 레포에 처음 들어오는 계열이다.** 핸들 4종(session·connect·request)의 수명·해제 순서를 틀리면 누수가 조용히 쌓인다 — `remote::envelope`의 RAII 관례(`AlgHandle`·`KeyHandle`)를 그대로 본떠 막는다.
- **R3. 자기 자신을 덮어쓰는 설치다.** 설치 프로그램이 앱을 닫기 전에 앱이 먼저 죽으면 재실행이 빠지고, 늦게 죽으면 파일 잠금으로 덮어쓰기가 실패한다. 앱은 설치 프로그램을 띄우고 **스스로 정상 종료**하며(세션 저장 경로를 탄다), 설치 프로그램은 종전대로 `taskkill`로 한 번 더 확인한다 — 어느 쪽이 먼저여도 성립한다.
- **R4. `update\` 폴더가 설치 폴더 안에 생긴다.** 제거 시 `RMDir "$INSTDIR"`은 비재귀라 그 폴더가 남으면 설치 폴더가 지워지지 않는다 — 앱이 시작할 때마다 비우고(T5), 제거 스크립트도 그 폴더를 지운다(T8).

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 변경 대상 | 사용처 (grep 전수) | 처리 |
|---|---|---|
| `titlebar::RIGHT_GROUP_WIDTH` (`const f32`) | 정의 `titlebar.rs:23` + **실사용 3곳** — `show_titlebar`의 `drag_right`(106)·`title_width`(142), **`over_titlebar_button`(387)**. 파일 밖 0건 | **상수를 런타임 값으로 바꾼다**(D13) — 배지 폭이 문구·글꼴에 따라 변해 컴파일 타임에 정해지지 않는다. 상수는 「배지가 없을 때의 기본 폭」으로 남고, 세 곳 모두 그 프레임에 계산된 값을 쓴다(T7) |
| `titlebar::over_titlebar_button` (387) | 프로덕션 호출 `titlebar.rs:346`(`show_resize_handles` 안) 1곳 + **시험이 직접 3회**(`titlebar.rs:496·497·498` — 4-C가 그 시험을 따로 잡는다) | 우측 폭을 인자로 받는다 — **이 판정이 배지를 모르면 배지 상단 4px에서 크기 조절 루프가 열려 그 클릭이 삼켜진다**(titlebar.rs:343~344 주석이 막으려던 바로 그 버그) |
| `titlebar::show_resize_handles` (**pub, 시그니처 변경**) | 호출 `src/ui/app.rs:1041` 1곳 + 시험 `titlebar.rs:488` 1곳 | 우측 그룹 폭 인자 추가. **같은 프레임에서 `show_titlebar`(1036)가 먼저 불리므로** 그 결과의 폭을 그대로 넘길 수 있다(보관 불요) |
| `titlebar::show_titlebar` (**pub, 시그니처 변경**) | 호출 `src/ui/app.rs:1036` 1곳 + 시험 `titlebar.rs:535` 1곳 | 배지 상태 인자 추가, `TitlebarOutcome`에 그 프레임의 우측 폭을 실어 돌려준다 |
| `titlebar::TitlebarState` (**pub struct**) | 생성 `src/ui/app.rs:1024` 1곳 + 시험 `titlebar.rs:538` 1곳 | 배지 상태는 **이 구조체에 넣지 않고 별도 인자**로 받는다 — 이것은 창 상태(최대화·사이드바)를 나르는 값이고 배지는 성격이 다르다 |
| `titlebar::pending_item` | `titlebar.rs:278·279` 2곳뿐 | 두 항목이 모두 활성이 되어 **쓰임이 사라진다 → 함수째 삭제**(남기면 `dead_code`로 `-D warnings`가 깨진다) |
| `titlebar::settings_menu_width` | `titlebar.rs:300~318` 1곳 | 라벨 배열은 그대로(항목 수·문구 불변) |
| `ui::menu::Command` | `menu.rs` 정의 + `titlebar.rs`·`tabs.rs`·`panel.rs`·`sidebar.rs`·`app.rs`가 값으로 나르고 **`app.rs`에 `match`가 둘**(1701 본체 · 1748 안쪽 — 안쪽은 `_ => {}`로 받아 넘겨 실해가 없다) | **변형 3개 추가** — 실제로 넓힐 곳은 1701 하나이며, 1748은 `_` 팔이 있어 컴파일이 깨지지 않는다(그래서 **넓히는 것을 잊어도 조용히 무동작이 된다** — T7 acceptance ⑤가 그것을 잡는다) |
| `i18n` 카탈로그 | `strings!` 매크로 정의부 1곳 + 부르는 쪽 | 키 추가는 기존 규약대로(두 언어 모두 적지 않으면 컴파일 오류) |
| `app` 모듈 트리 | `src/app/mod.rs` | `pub mod update;` 1줄 추가 |

### 4-B. 계약·직렬화 변경

- **없다.** `settings.json` 스키마(v3)를 건드리지 않는다 — 업데이트 상태는 전부 휘발성이라 저장하지 않는다.
- 새로 읽는 외부 형식은 **GitHub Releases API의 JSON**이며, 우리가 쓰는 필드는 `tag_name`·`body`·`assets[].name`·`assets[].browser_download_url` 넷이다. `serde`의 미지 필드 무시 기본값으로 나머지는 건너뛴다.

### 4-C. 테스트 파일

- 영향 받는 기존 시험:
  - `src/ui/titlebar.rs:483~489` `최대화_중에는_크기_조절을_받지_않는다` — `show_resize_handles`를 직접 부른다. **시그니처가 바뀌므로 호출을 고친다**(인자 추가).
  - `src/ui/titlebar.rs:491~498` `위쪽_변은_버튼_구간을_비켜준다` — **우측 174px를 전제로 단언한다.** 배지 없는 상태의 폭이 그대로 174여야 이 시험이 통과한다(T7 acceptance ⑦의 「배지가 없으면 종전과 같다」가 그것을 요구한다). 배지 있는 경우의 단언을 여기 **추가**한다.
  - `src/ui/titlebar.rs:528~547` `show_titlebar` 호출 헬퍼 — 시그니처 변경에 맞춰 인자를 더한다.
  - `src/i18n/mod.rs`의 `화면_문구가_카탈로그를_거치지_않은_곳이_없다`(새 UI 문구가 카탈로그를 거치는지 본다), `src/ui/theme.rs`의 `팝업_메뉴는_항목_스타일을_거친다`·`팝업_메뉴는_모서리를_따로_적지_않는다`(타이틀바 메뉴가 대상), `src/ui/widgets.rs`의 `화면_코드에_원본_아이콘_기호가_남아_있지_않다`(배지 아이콘이 phosphor여야 한다), `src/app/licenses.rs`의 `Cargo.lock` 지문 대조(전제 4로 불변 확인).
- 신규 시험: T2~T6의 각 모듈 `#[cfg(test)] mod tests`(순수 로직 — 버전 비교·에셋 선택·체크섬 추출·SHA256 벡터·경로 조립·상태 전이·URL 분해).
- **네트워크가 필요한 경로는 시험 대상이 아니다** — 실제 HTTP는 UI(HWND)와 같은 취급으로 두고, 그 위의 순수 로직만 시험한다(AGENTS 테스트 규약).

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `app::update::http`(WinHTTP GET) | `reqwest`·`ureq`·`curl`·`HttpClient` 0건. `suppaftp`·`ssh2`는 각자 프로토콜 전용 | **신규** — 대체할 것이 없다. 신규 패키지 없이 이미 있는 `windows` 크레이트의 feature만 켠다(최소 의존 ④ 플랫폼 기본 기능) |
| `app::update::sha256` | `remote::envelope`가 CNG를 쓰지만 PBKDF2·AES-GCM뿐, 해시 함수는 없다 | **신규**(CNG 호출), 단 **`envelope::to_hex`는 재사용**(`pub fn`, 바이트→소문자 hex — 같은 일을 두 번 쓰지 않는다) |
| `app::update::release`(버전 비교·에셋 선택·체크섬 추출) | 버전 비교 함수 0건. JSON은 `serde_json` **재사용** | **신규** — 순수 로직 |
| `app::update::install`(설치 실행·정리) | `std::process::Command` 사용처 0건 | **신규** |
| 업데이트 상태·워커 | 워커+채널 패턴이 `ui::app::spawn_os_drop_scan`·`ui::font_scan`에 있다 | **패턴 재사용**(구조를 본뜬다, 코드 공유는 하지 않는다 — 나르는 값이 다르다) |
| 전송 진행 판정 | `TransferQueue::count(QueueFilter)`·`TransferState::is_active` | **재사용** — 새로 만들지 않는다 |
| 확인 대화 | `ui::dialog::show` | **재사용** — 모달 규약(AGENTS) |
| 안내 토스트 | `ui::toast::Toast` | **재사용** |
| 브라우저 열기 | `egui::Context::open_url`(about 대화가 같은 경로를 쓴다) | **재사용** |
| 배지 아이콘 | `egui_phosphor::regular::*` | **재사용** — 아이콘 규약(AGENTS) |

### Verified by

- `cargo build --release` 경고 0 / `cargo clippy --all-targets -- -D warnings` 0 / `cargo fmt --check`
- `cargo test` 전건 통과
- `cargo run --example gen_installer` — 설치 파일 생성(makensis 경고 0)
- HUMAN-VERIFY: 아래 「Verification Strategy」

## 동반 변경 판정

| 판정 | 대상 | 근거 | 처리 |
|---|---|---|---|
| **필수** | `docs/prd.md` — Out of Scope의 「`업데이트`·`릴리즈 노트` 둘은 계속 제외」, FR-22 문면("나머지 둘은 항목 표시만 한다"), 성공 기준 Should 목록 | 이번 요청이 그 제외를 뒤집는다. 고치지 않으면 PRD가 산출물과 정반대를 말한다 | **T1** |
| **필수** | `AGENTS.md` 「추가 정보」 배포 줄 — *"코드 서명·자동 업데이트는 없다"* | 이 문장이 거짓이 된다 | **T9** |
| **필수** | `AGENTS.md` Build & Test — 릴리즈 발행 절차(버전 → 설치 파일 → SHA256 → 태그 → 릴리즈 본문) | SHA256 검증(D3)은 **릴리즈 노트에 체크섬이 적혀 있을 때만** 작동한다. 그 절차가 어디에도 없으면 다음 릴리즈에서 조용히 빠져 검증이 꺼진다 — 기능의 성립 조건이다 | **T9** (Deferred 「릴리즈·태그 규약 문서화」 재수용) |
| **필수** | `README.md` — 기능 목록·설치/업데이트 절 | 기능 추가 시 갱신(공통 지침 4단계) | **T9** |
| **필수** | `installer/moa.nsi` 제거 절 — `update\` 폴더 삭제 | 그 폴더가 남으면 비재귀 `RMDir "$INSTDIR"`이 실패해 **제거 후 설치 폴더가 남는다**(R4) | **T8** |
| **필수** | `src/ui/titlebar.rs`의 `pending_item` 함수와 그 doc 주석, `show_settings_menu` doc 주석("나머지 둘은 아직 표시만 한다"), `src/i18n/mod.rs:136~138`의 같은 뜻 주석 | 이번 변경이 그 서술을 거짓으로 만든다(자기 유발) | **T7** |
| **선택** | 위키 `20_projects/personal/moa/decisions.md`의 「자동 업데이트 기각」 항목 갱신 | 레포 밖 개인 지식 저장소라 이번 산출물의 성립과는 무관 | Phase F-6.5의 `[DECISION]` 큐로 넘긴다(별도 세션) |
| **무관** | `assets/licenses.json`·`THIRD-PARTY-NOTICES.md` | 전제 4 — feature만 더하면 패키지 집합·`Cargo.lock`이 그대로라 재생성이 필요 없다 | 건드리지 않는다 |
| **무관** | `settings.json` 스키마·`remote/` 전반 | 4-B — 저장 형식도 원격 경로도 닿지 않는다 | 건드리지 않는다 |

## 시각 요소 분해

> Step 2.5의 발동 조건 ①-a·①-b에는 해당하지 않는다 — 보내신 이미지는 **다른 앱(Zed)의 화면**을 위치 예시로 든 것이고 "똑같이 맞춰 달라"는 요구가 아니며, 대조할 원본 소스도 없다. 그래도 타이틀바에 **새 요소**가 들어가므로 값이 흔들리지 않게 시각 속성만 확정해 둔다(참조 정합 인벤토리는 원본이 없어 만들지 않는다).

### 시각 속성

| 요소 | 속성 | 값 | 출처 |
|---|---|---|---|
| 업데이트 배지 | 높이 | `TITLEBAR_HEIGHT`(36px) | 기존 타이틀바 토큰 |
| 업데이트 배지 | 좌우 여백 | 10px | 캡션 버튼(46px)보다 좁게, 글자가 붙지 않게 |
| 업데이트 배지 | 폭 | 아이콘 + 간격 4px + 글자 폭 + 여백 20px을 **그 자리에서 재서** 정한다 (배지가 없으면 0 — 우측 그룹 폭이 종전 174 그대로다) | `settings_menu_width`와 같은 방식(언어·글꼴이 바뀌어도 든다). 이 값이 상수가 아니라서 **우측 그룹 폭도 런타임 값이 된다** → D13 |
| 업데이트 배지 | 자리 | 설정 기어 버튼 **왼쪽** | 요청 이미지의 배치 |
| 배지 아이콘 (새 버전 있음) | 글리프 | `egui_phosphor::regular::ARROW_CIRCLE_DOWN` | 아이콘 규약(phosphor만) |
| 배지 아이콘 (받는 중) | 글리프 | `egui_phosphor::regular::CIRCLE_NOTCH` + 회전 | 같은 규약 — 도는 표시가 필요하다 |
| 배지 글자 | 크기 | `TITLE_FONT_PX`(14px) | 제목과 같은 단 |
| 배지 글자 | 색 | `theme::TEXT` | 기존 팔레트 |
| 배지 hover | 배경 | `theme::CONTROL_HOT` | 캡션·설정 버튼과 같은 반응 |
| 배지 (받는 중) | 상호작용 | 누를 수 없다(hover 없음) | 두 번 누르면 두 번 받는다 |
| 확인 대화 | 셸 | `ui::dialog::show`, 본문 폭 420px | 모달 규약, 기존 확인 대화와 같은 폭대 |

## Decisions

### D1. HTTP는 WinHTTP로 한다 (신규 패키지 0)

**Options**: (A) `curl.exe` 호출 (B) **WinHTTP FFI** (C) `ureq` 등 신규 크레이트
**Chosen**: **B** — 사용자 결정.
**Source**: 전제 1·2. AGENTS 최소 의존 원칙(④ 플랫폼 기본 기능). 외부 프로세스를 띄우지 않아 콘솔 깜빡임·`curl.exe` 부재(Windows 10 1803 미만) 걱정이 없고, 라이선스 자산 재생성도 없다(전제 4). 대가는 `unsafe` FFI 코드 약 250줄이며, AGENTS의 「함수 단위 격리 + 사유 주석 + 안전 래퍼」 규약으로 다룬다.

### D2. 업데이트 모드 인자 `/UPDATE`를 설치 스크립트에 신설한다

**Options**: (A) **`/UPDATE` 인자** (B) 설치 파일을 그대로 실행 (C) 완전 무음 `/S`
**Chosen**: **A** — 사용자 결정. 요청 문면("설치 화면이 표시되고 자동으로 … 진행 … 완료되면 앱을 다시 실행")과 1:1이다.
**Source**: 전제 5(`GetParameters`)·6(taskkill 기존). B는 클릭 4번을 요구해 "자동으로 진행"과 어긋나고, C는 "설치 화면이 표시되고"와 어긋난다.

### D3. 받은 파일은 SHA256으로 검증한다

**Options**: (A) **검증한다** (B) HTTPS 신뢰에 맡긴다
**Chosen**: **A** — 사용자 결정. 릴리즈 노트 본문의 64자 hex를 기대값으로 삼고, 어긋나면 **파일을 지우고** 중단한다.
**Source**: 위키 recipe `installer-sha256-verify`. 전제 3으로 신규 크레이트 없이 CNG로 구현한다. 체크섬이 본문에 **없으면**: 그 릴리즈는 검증을 건너뛰지 않고 **받기를 거부**한다 — 지금 릴리즈가 0건이라 "구버전 호환"을 배려할 대상이 없고, T9가 발행 절차에 체크섬을 못 박으므로 없는 편이 사고다(TaskMon은 이미 낸 릴리즈가 있어 동의 후 진행을 뒀다 — 우리는 그 부채가 없다).

### D4. 설치본에서만 켠다

**Options**: (A) **설치본에서만** (B) 확인만 하고 설치는 막는다 (C) 구분 없이
**Chosen**: **A** — 사용자 결정. exe 옆에 `uninstall.exe`가 있으면 설치본이다(전제 8).
**Source**: 개발 실행은 `target/debug\`에서 돌아 거기에 `update\` 폴더가 생기면 지저분하고, 설치는 어차피 다른 자리(`%LOCALAPPDATA%\Programs\MOA`)에 되어 지금 도는 exe와 갈린다. 설치본이 아니면 **확인 자체를 하지 않고** 두 메뉴 항목은 지금처럼 비활성으로 둔다.

### D5. 전송이 도는 중이면 확인 대화를 띄운다

**Options**: (A) **묻고 진행** (B) 그대로 설치 (C) 전송 중엔 막는다
**Chosen**: **A** — 사용자 결정. 활성 전송이 1건 이상이면 건수를 담은 확인 대화를 띄우고, 동의했을 때만 설치한다.
**Source**: 전제 7(`TransferQueue::count`). 설치는 앱을 강제 종료시키므로 올리던 파일이 중간에 끊긴다.

### D6. 상태는 저장하지 않는다 (전부 휘발성)

**Chosen**: 확인 결과·다운로드 진행·오류는 `settings.json`에 적지 않는다.
**Source**: 4-B. 앱을 다시 띄우면 어차피 확인을 다시 하고(요청: 실행 시 1회), 저장하면 스키마 v4가 되어 되돌리기 어려운 계약이 는다. 「나중에 필요할 것 같은 코드」를 넣지 않는다(공통 지침 2단계).

### D7. 확인은 앱 실행당 1회, 손으로 누르면 그때 또 한 번

**Chosen**: 시작 시 워커 1회. 설정 메뉴의 `업데이트`를 누르면 다시 확인한다(요청 명시). 주기적 재확인은 두지 않는다.
**Source**: 요청 원문("업데이트 확인은 앱 실행시 한번만 확인" + "메뉴에서 업데이트 메뉴클릭시 업데이트 체크").

### D8. `update\` 폴더는 앱이 시작할 때 지운다

**Options**: (A) 설치 직전 지운다 (B) **시작할 때 지운다** (C) 설치 프로그램이 지운다
**Chosen**: **B**(+ 받기 직전에도 비운다).
**Source**: 요청은 "업데이트가 완료되면 다운로드 받은 파일은 삭제"인데, **설치가 끝나는 시점에는 앱이 죽어 있어** 스스로 지울 수 없다. 새 버전으로 다시 뜬 앱이 지우는 것이 그 요구의 유일한 성립 방식이다. 설치가 중간에 취소돼 남은 파일도 같은 자리에서 함께 치워진다. C는 설치 프로그램이 자기 실행 파일을 지우는 꼴이라 안 된다.

### D9. 앱은 설치 프로그램을 띄운 뒤 **정상 종료 경로**로 닫는다

**Chosen**: `Command::spawn`으로 설치 프로그램을 분리 실행하고, 앱은 `quitting = true` + `ViewportCommand::Close`(트레이 `종료`와 같은 길 — 세션 저장이 그 길에 있다)로 닫는다.
**Source**: `ui/app.rs:1463~1469`. 설치 프로그램의 `taskkill`은 그대로 두어(R3) 어느 쪽이 먼저여도 성립하게 한다.

### D10. 릴리즈 노트는 `releases` 목록 페이지를 연다

**Chosen**: `https://github.com/jongcheol-pak/MOA/releases`를 `ctx.open_url`로 연다.
**Source**: 전제 10. `releases/latest`가 아니라 목록인 이유는 "릴리즈 노트"가 이력 전체를 뜻하고, 릴리즈가 0건이어도 페이지가 성립하기 때문이다. 주소는 카탈로그(`i18n`)에 둔다 — 기존 `about_repository_url`과 같은 취급(화면 문구 규약).

### D11. 모듈은 `src/app/update/`에 넷으로 나눈다

**Chosen**: `mod.rs`(상태·워커·공개 API) / `http.rs`(WinHTTP 래퍼) / `release.rs`(릴리즈 조회·버전·에셋·체크섬) / `install.rs`(설치본 판정·폴더·실행·정리).
**Source**: AGENTS 파일 분할 판정 — ① 변경 이유가 넷으로 갈린다(FFI 사정·GitHub 응답 형식·설치 절차·상태 흐름) ② FFI를 고치려고 상태 기계를 읽을 이유가 없다 ④ 나눠도 흩어지지 않는다(한 폴더 안). `app/`에 두는 것은 `ui`를 몰라야 하기 때문이다(의존 단방향) — `autostart.rs`(레지스트리)·`single_instance.rs`(뮤텍스)가 이미 같은 성격으로 그 자리에 있다.

### D13. 우측 버튼군 폭을 **상수에서 런타임 값으로** 바꾼다

**문제**: `RIGHT_GROUP_WIDTH`는 `const f32`(174 = 36 + 46×3)인데 배지 폭은 문구·글꼴·언어에 따라 변해 컴파일 타임에 정해지지 않는다. 그 상수를 쓰는 세 곳(창 끌기 영역·제목 최대 폭·**위쪽 변 크기 조절 비켜주기**)이 배지를 모르면, 배지 위에서 창이 끌리고 배지 상단 4px에서 크기 조절 루프가 열려 클릭이 삼켜진다.

**Chosen**: 상수는 `RIGHT_GROUP_BASE`(배지 없을 때의 폭)로 남기고, `show_titlebar`가 그 프레임의 배지 폭을 재서 `RIGHT_GROUP_BASE + badge_width`를 계산해 ⓐ 자기 안의 두 곳에 쓰고 ⓑ `TitlebarOutcome.right_group_width`로 실어 돌려준다. `ui::app`은 그 값을 **같은 프레임에** `show_resize_handles(ctx, maximized, right_group_width)`로 넘긴다.

**Source**: `src/ui/app.rs:1036`이 `show_titlebar`를, 1041이 `show_resize_handles`를 부른다 — **호출 순서가 이미 맞아** 값을 프레임 사이에 보관할 필요가 없다. 직전 프레임 값을 기억하는 방식(`Cell`·필드)은 언어를 바꾼 첫 프레임에 한 프레임 어긋나므로 택하지 않는다. 배지가 없으면 폭이 정확히 종전 174라 기존 시험(`위쪽_변은_버튼_구간을_비켜준다`)이 그대로 통과한다.

### D14. 개발 빌드에서 업데이트 화면을 볼 수 있는 스위치를 둔다

**문제**: D4로 설치본에서만 기능이 켜지므로, 개발 빌드로는 배지도 `다운로드 중...`도 확인 대화도 **한 번도 볼 수 없다**. 그대로 두면 화면을 못 본 채 첫 릴리즈를 내야 한다.

**Chosen**: **`#[cfg(debug_assertions)]`에서만** 환경변수 `MOA_UPDATE_DEV`가 **둘을 함께** 뚫는다. 릴리즈 빌드에는 그 분기가 **컴파일되지 않는다**(`cfg`라 코드 자체가 없다).
- `MOA_UPDATE_DEV=1` → `is_installed_build()`가 참(설치본 판정 우회)
- `MOA_UPDATE_DEV=fake` → 그에 더해 **확인 결과를 가짜 `ReleaseInfo`로 대체한다**(버전 `99.0.0`, 자산 이름·URL은 더미, sha256 `None`). 네트워크를 치지 않는다.

**Source**: **판정만 뚫으면 A 구간이 헛돈다** — 저장소에 릴리즈가 0건이라 확인이 `Ok(None)`으로 떨어져 배지가 끝내 뜨지 않고, 그러면 「배지를 릴리즈 발행 전에 본다」는 목적 자체가 성립하지 않는다(실제로 이 결함이 리뷰에서 잡혔다). `fake`는 확인 경로만 대체하고 **다운로드 이후는 대체하지 않는다** — 그 구간은 실제 릴리즈로만 확인할 수 있고(HUMAN-VERIFY B), 가짜 URL을 받으러 가면 오류 표시 경로가 함께 확인된다. 환경변수를 고른 것은 설정 파일·명령행 인자와 달리 **사용자에게 노출되는 표면이 늘지 않기** 때문이다.

### D15. 릴리즈 노트는 사용자용 요약이고, 체크섬은 접어서 맨 아래 둔다

**요구**: 항목당 1~2줄 · 불필요한 것 제외 · 일반 사용자가 읽으므로 길거나 어렵지 않게(2026-08-22 사용자 요청).

**충돌과 그 해결**: D3이 **릴리즈 본문에 SHA256을 적을 것**을 요구하는데, 64자 hex는 이 요구가 말하는 「불필요하고 어려운 내용」의 전형이다. 둘을 함께 만족시키려면 **자리를 가른다** — 사용자용 요약을 위에 두고, 체크섬은 본문 **맨 아래 `<details>` 접기** 안에 넣는다. GitHub 릴리즈 본문은 마크다운이라 접기가 그대로 동작하고, `extract_sha256`은 **본문 어느 자리에 있든 64자 hex 토큰을 뽑으므로**(T4) 접혀 있어도 읽힌다. 사용자는 펴지 않는 한 보지 않는다.

**본문 형식** (T9가 `AGENTS.md`에 이 틀과 예시를 함께 적는다):
```markdown
### 새로워진 것
- 자동 업데이트 — 새 버전이 나오면 제목 줄에 알리고, 눌러 두면 알아서 받아 설치합니다.
- 릴리즈 노트 — 설정 메뉴에서 이 페이지를 바로 열 수 있습니다.

### 고친 것
- (없으면 이 절을 통째로 뺀다)

<details><summary>파일 무결성 확인용 (SHA256)</summary>

MOA-Setup-0.1.0.exe
`<64자 hex>`
</details>
```

**쓰는 규칙**: ⓐ 한 항목은 **1~2줄**, 사용자가 화면에서 겪는 변화로 적는다 ⓑ **내부 사정은 빼거나 사용자 말로 옮긴다** — 모듈 이름·함수 이름·리팩토링·시험 추가·의존성 조정은 적지 않는다 ⓒ 절은 `새로워진 것` / `고친 것` 둘뿐이고 **빈 절은 뺀다** ⓓ 커밋 목록·비교 링크를 붙이지 않는다(GitHub이 자동으로 붙인다) ⓔ 사용자에게 **행동이 필요한 것**(설정이 초기화된다 등)이 있으면 맨 위에 한 줄로 알린다.

**Source**: 요청 원문. 체크섬 자리를 접기로 정한 것은 D3과 이 요구가 **본문 하나를 두고 겹치기 때문**이며, 어느 한쪽을 포기하지 않는 유일한 배치다.

### D12. PRD는 FR을 **둘** 신설한다

**Chosen**: FR-62(자동 업데이트)·FR-63(릴리즈 노트 열기), 둘 다 Should. FR-22는 문면만 고친다.
**Source**: 요청이 서로 다른 두 기능(요청 1·2)이고 진입점·동작·실패 양상이 갈린다. 다음 번호가 62·63이다(현재 최대 FR-61 확인).

## Tasks

- [x] T1. PRD 개정 — FR-62·FR-63 신설, Out of Scope 재한정, FR-22 문면 갱신
  - **Type**: A
  - **Acceptance**: Given `docs/prd.md`, When 읽으면, Then ① FR-62·FR-63 두 행이 기능 요구사항 표에 있고 각각 우선순위 Should·검증 방법이 적혀 있다 ② Out of Scope의 「`업데이트`·`릴리즈 노트` 둘은 계속 제외하며 FR-22대로 표시만 유지한다」가 **취소선 + 2026-08-22 재한정** 형식으로 고쳐져 그 둘이 채택됐음을 말한다(기존 문서의 재한정 관례 그대로) ③ FR-22 행의 「나머지 둘은 항목 표시만 한다」가 사라지고 다섯 항목이 모두 동작한다고 적힌다 ④ 성공 기준의 Should 목록에 FR-62·FR-63이 든다 ⑤ 결정 이력에 이번 채택이 한 줄 남고, 그 줄이 **2026-08-21 결정 이력의 「코드 서명·자동 업데이트는 범위 밖이다」(`prd.md:154`)를 뒤집는다는 것을 이름으로 적는다** — 그 줄 자체는 날짜가 붙은 그 시점의 기록이라 **원문을 고치지 않는다**(문서의 기존 관례). `grep -c "항목 표시만" docs/prd.md` → 0.
  - **Files**: 주: `docs/prd.md`
  - **Halt Forecast**: (i) FR 번호 충돌 → 현재 최대가 FR-61임을 실측했다(Investigation Log). 착수 시점에 다른 세션이 번호를 쓰지 않는다(단독 작업) — 사전 해소 / (ii-b) 없음 — 문서 한 파일이라 파괴적·외부 작업이 없다
  - **Depends on**: -

- [x] T2. WinHTTP GET 래퍼 — `src/app/update/http.rs`
  - **Type**: C
  - **Design**: ① `src/app/update/http.rs` 신설, `app` 계층(ui를 모른다) ② 신규 심볼 — `get_bytes(url, accept) -> Result<Vec<u8>, HttpError>`(작은 응답을 메모리로), `download_to_file(url, dest) -> Result<(), HttpError>`(64KB 청크로 파일에 흘린다), 내부에 `Session`·`Connect`·`Request` RAII 래퍼 3종과 `split_url(url) -> Option<UrlParts>`(순수 — 시험 대상. 포트를 함께 다뤄야 해 3튜플이 아니라 `host`·`port`·`path`·`secure` 네 칸의 구조체다) ③ 의존 방향 — `windows` 크레이트만 본다. `release.rs`·`mod.rs`가 이것을 부르고, 이 파일은 그 둘을 모른다 ④ **비추상화 선언** — HTTP 클라이언트 트레이트·미들웨어·리트라이 정책을 만들지 않는다. GET 둘뿐이고 재시도는 사용자가 다시 누르는 것으로 갈음한다
  - **Acceptance**: Given 이 모듈, When `cargo test`를 돌리면, Then `split_url`이 `https://api.github.com/repos/a/b/releases/latest`를 (`api.github.com`, `/repos/a/b/releases/latest`, https=true)로 가르고, 포트·쿼리·비-https·빈 문자열·스킴 없는 문자열을 각각 정해진 대로 다루는 시험이 통과한다. `unsafe` 블록마다 사유 주석이 있고 핸들 3종이 `Drop`에서 닫힌다. `cargo clippy --all-targets -- -D warnings` 0.
  - **Files**: 주: `src/app/update/http.rs`(신규), `src/app/update/mod.rs`(신규 — 모듈 선언만 먼저), `src/app/mod.rs`(`pub mod update;`), `Cargo.toml`(`Win32_Networking_WinHttp` feature + 사유 주석) / 테스트: 같은 파일 `#[cfg(test)] mod tests`
  - **Edge Cases**: 네트워크 없음 → `WinHttpConnect`/`SendRequest` 실패를 `HttpError`로 / HTTP 상태가 200이 아님(403 한도·404) → 상태 코드를 `WinHttpQueryHeaders`로 읽어 오류로 / 리다이렉트 → WinHTTP 기본이 추종한다(자산 URL의 S3 리다이렉트 — 위키 ⓑ) / 응답이 비정상적으로 큼 → 메모리 경로에 상한(16MB)을 두고 넘으면 오류 / 쓰기 실패(디스크 풀·권한) → 파일을 지우고 오류
  - **Halt Forecast**: (i) `User-Agent` 없이 GitHub이 403을 준다(위키 ⓐ) → `WinHttpOpen`의 agent 인자에 `MOA/<버전>`을 넣어 사전 해소 / (ii-a) `Cargo.toml` feature 추가 — 사전 승인 항목
  - **Depends on**: -

- [x] T3. SHA256 (CNG) — `src/app/update/sha256.rs`
  - **Type**: C
  - **Design**: ① `src/app/update/sha256.rs` ② 신규 심볼 — `file_sha256(path) -> Option<String>`(64KB 청크 스트리밍, 소문자 hex)과 `matches(expected, actual) -> bool`(**T5의 `verify_downloaded`가 부른다** — 릴리즈 노트는 사람이 쓰는 글이라 값이 대문자로 적히거나 줄 끝에 공백이 붙어, 그 관용을 대조하는 쪽마다 되풀이하지 않고 해시를 아는 이 모듈에 둔다). 내부에 `AlgHandle`·`HashHandle` RAII **2종**(제공자 핸들도 닫아야 하므로 하나로는 새는 자리가 생긴다) ③ 의존 — `windows`(BCrypt)와 **`crate::remote::envelope::to_hex` 재사용**(4-D). `install.rs`가 이것을 부른다. **`app` → `remote` 참조는 새로 만드는 것이 아니다** — `src/app/settings.rs:212`가 이미 `crate::remote::sites::SiteStore`를 쓴다(AGENTS 계층 규약은 「`ui`만 상위」를 말하고 그 둘의 관계는 정하지 않았다) ④ **비추상화 선언** — `Digest` 트레이트·해시 알고리즘 선택 인자를 두지 않는다. SHA256 하나뿐이다
  - **Acceptance**: Given 알려진 시험 벡터, When `file_sha256`을 부르면, Then 빈 파일이 `e3b0c442...b855`, `"abc"`가 `ba7816bf...ad15`를 돌려주고, 64KB 경계를 넘는 파일(예: 100KB)도 한 번에 읽은 것과 같은 값을 낸다. 없는 경로는 `None`. `cargo test` 통과, clippy 0.
  - **Files**: 주: `src/app/update/sha256.rs`(신규), `src/app/update/mod.rs`(모듈 선언) / 테스트: 같은 파일 `#[cfg(test)] mod tests`(임시 폴더 사용 — 기존 시험 관례 `std::env::temp_dir().join(format!("…{}", std::process::id()))`)
  - **Edge Cases**: 빈 파일 → 빈 입력의 정해진 해시 / 읽는 중 파일이 사라짐 → `None` / 큰 파일(수십 MB) → 청크라 메모리가 일정 / `BCrypt` 호출 실패 → `None`(패닉하지 않는다)
  - **Halt Forecast**: (i) `BCryptFinishHash`가 요구하는 해시 오브젝트 버퍼 크기 → `BCryptGetProperty(BCRYPT_OBJECT_LENGTH)`로 묻거나 Windows 8+ 관례대로 CNG에 맡긴다(`envelope.rs:238~240`이 같은 판단을 이미 적어 뒀다) — 사전 해소
  - **Depends on**: -

- [ ] T4. 릴리즈 조회·버전 비교·에셋 선택·체크섬 추출 — `src/app/update/release.rs`
  - **Type**: C
  - **Design**: ① `src/app/update/release.rs` ② 신규 심볼 — `ReleaseInfo { version, asset_name, asset_url, sha256 }`, `fetch_latest() -> Result<Option<ReleaseInfo>, UpdateError>`(HTTP + 파싱 + 비교를 엮는다), 순수 함수 넷: **`parse_release(json, current) -> Result<Option<ReleaseInfo>, UpdateError>`**·`is_newer(latest, current) -> bool`·`pick_asset(names) -> Option<…>`·`extract_sha256(body) -> Option<String>`. **`parse_release`가 `Option`이 아니라 `Result`인 이유가 둘이다** — ⓐ 실패 사유를 갈라야 화면이 「최신입니다」와 「릴리즈가 깨졌습니다」를 구분해 알릴 수 있다(FR-62가 결과를 알리라고 요구한다) ⓑ **체크섬 없는 릴리즈를 타입으로 막는다**: `ReleaseInfo.sha256`이 `Option<String>`이 아니라 `String`이라 값이 없으면 그 구조체가 애초에 만들어지지 않는다(D3을 T5의 성실함에 기대지 않는다). `current` 인자는 T6의 `new(enabled)`와 같은 시험 seam이다 — 박아 두면 판정을 `CARGO_PKG_VERSION` 하나로만 시험할 수 있다 ③ 의존 — `http.rs`·`serde_json`. `mod.rs`가 부른다 ④ **비추상화 선언** — 릴리즈 제공자 트레이트(GitHub/GitLab 추상화)를 만들지 않는다. 저장소는 하나다
  - **Acceptance**: Given 실제 GitHub 응답 형태의 JSON 문자열(시험에 박아 둔 표본), When 순수 함수들을 부르면, Then ① `parse_release`가 `tag_name`·`body`·`assets`에서 넷을 뽑고, **빠진 것에 따라 사유를 갈라 `Err`를 낸다**(형태가 아니면 `BadResponse` · 설치 파일이 없으면 `NoAsset` · 체크섬이 없으면 `NoChecksum`). 새 판이 아니면 `Ok(None)` ② `is_newer`가 `v0.2.0 > 0.1.0`, `0.1.10 > 0.1.9`(사전식 문자열 비교였다면 뒤집혔을 값), `0.1.0 == 0.1.0` → false, `v` 접두사 유무 무관, 형식이 깨진 값 → false를 낸다 ③ `pick_asset`이 `MOA-Setup-0.2.0.exe`를 고르고 `MOA-Setup-0.2.0.exe.sha256`·`source.zip`은 거르며 후보가 없으면 `None` ④ `extract_sha256`이 본문 어느 자리에 있든 64자 hex를 소문자로 뽑고, 63·65자·비-hex는 무시하며 없으면 `None`. **표본에 D15의 실제 릴리즈 노트 형식을 쓴다** — 사용자용 요약(한글 문장·목록) 아래 `<details>` 접기 안에 백틱으로 감싼 체크섬이 있는 본문에서 그 값을 뽑아야 한다(태그·백틱·파일명은 전부 비-hex 문자라 구분자로 잘린다. `<details>`·`summary`·`MOA-Setup-0.1.0.exe` 어느 것도 64자 hex가 아니다). `cargo test` 통과, clippy 0.
  - **Files**: 주: `src/app/update/release.rs`(신규), `src/app/update/mod.rs`(모듈 선언) / 테스트: 같은 파일 `#[cfg(test)] mod tests`
  - **Edge Cases**: 릴리즈 0건(404 또는 빈 응답) → `Ok(None)`(오류가 아니다 — 지금 저장소의 실제 상태다) / 자산이 없는 릴리즈 → `None` / 프리릴리즈·초안 → `releases/latest`가 애초에 주지 않는다 / 본문에 체크섬이 없다 → **T4가 그 자리에서 `Err(NoChecksum)`으로 거부**(D3 — T5까지 미루지 않는다. `ReleaseInfo.sha256`이 `String`이라 값 없는 릴리즈는 그 구조체가 만들어지지 않는다) / 최신이 현재보다 **낮다**(강등된 릴리즈) → `is_newer` false라 아무것도 하지 않는다 / 버전 문자열에 `-rc1` 같은 꼬리 → 수로 못 읽는 마디는 0으로 보고 비교
  - **Halt Forecast**: (i) `latest`가 초안·프리릴리즈를 어떻게 다루는지 → 그 엔드포인트가 정식 릴리즈만 준다는 전제에 기대지 않고, 자산 이름 매칭이 실패하면 조용히 없는 것으로 다룬다 — 사전 해소
  - **Depends on**: T2

- [ ] T5. 설치본 판정·`update\` 폴더·설치 실행·정리 — `src/app/update/install.rs`
  - **Type**: C
  - **Design**: ① `src/app/update/install.rs` ② 신규 심볼 — `is_installed_build() -> bool`(exe 옆 `uninstall.exe`), 그 판정부는 **경로를 인자로 받는 비공개 `is_installed_at(dir) -> bool`**로 두고 공개 함수가 `current_exe`의 부모를 넘긴다(시험 seam), `update_dir() -> Option<PathBuf>`(exe 옆 `update\`), `clear_update_dir()`(폴더째 지운다), `download_and_verify(info) -> Result<PathBuf, UpdateError>`(받기 → SHA256 대조 → 어긋나면 지우고 오류), `launch_installer(path) -> bool`(`/UPDATE` 인자로 분리 실행) ③ 의존 — `http.rs`·`sha256.rs`·`release.rs`의 타입. `mod.rs`가 부른다 ④ **비추상화 선언** — 설치 방식(msi/exe/zip) 분기나 플러그인 지점을 두지 않는다. NSIS exe 하나다
  - **Design(개발 스위치, D14)**: `is_installed_build()`는 **`#[cfg(debug_assertions)]`에서만** `MOA_UPDATE_DEV`가 `1` 또는 `fake`면 `true`를 돌려준다(가짜 릴리즈 주입은 T6이 같은 변수를 보고 한다). 릴리즈 빌드에는 그 분기가 컴파일되지 않는다.
  - **Design(검증부 분리)**: `download_and_verify`를 둘로 가른다 — 받기는 `http::download_to_file`이 하고, **대조는 `verify_downloaded(path, expected_hex) -> Result<(), UpdateError>`**(불일치면 그 자리에서 파일을 지운다)가 하며, 값 비교는 **T3의 `sha256::matches`를 재사용**한다(대소문자·앞뒤 공백 관용). 그래야 **네트워크 없이 대조 규칙을 시험할 수 있다**(합쳐 두면 다운로드가 선행돼 `cargo test`로 관측 불가하다).
  - **Acceptance**: Given 이 모듈, When `cargo test`를 돌리면, Then ① `update_dir()`의 부모가 `std::env::current_exe()`의 부모와 같고 이름이 `update`다 ② `clear_update_dir()`이 파일이 든 폴더를 지우고, 폴더가 없어도 오류 없이 끝난다 ③ `is_installed_at(dir)`이 `uninstall.exe` 유무로 갈린다(임시 폴더로 두 경우를 만들어 직접 시험) ④ **`verify_downloaded`가 임시 파일 + 틀린 hex에서 오류를 내고 그 파일을 지운다**(맞는 hex면 파일이 남는다) — 네트워크를 쓰지 않는다 ⑤ `MOA_UPDATE_DEV` 분기가 `#[cfg(debug_assertions)]` 안에 있어 릴리즈 빌드 소스에 남지 않는다(`cargo build --release` 경고 0으로 그 코드가 죽지 않았음을 함께 확인). clippy 0.
  - **Acceptance(화면 — HUMAN-VERIFY B-13)**: 실제 릴리즈에서 받은 파일이 체크섬 불일치로 거부되는 전 구간은 사람이 본다(다운로드가 선행되므로 자동 관측 불가).
  - **Files**: 주: `src/app/update/install.rs`(신규), `src/app/update/mod.rs`(모듈 선언) / 테스트: 같은 파일 `#[cfg(test)] mod tests`
  - **Edge Cases**: `update\`에 이전 파일이 남아 있다 → 받기 전에 비운다(D8) / 폴더를 만들 수 없다(권한·디스크 풀) → 오류로 알린다 / 받는 중 앱이 죽는다 → 다음 시작의 `clear_update_dir()`이 치운다 / 체크섬이 릴리즈에 없다 → 받지 않고 오류(D3) / 설치 프로그램 실행 실패 → `false`를 돌려 앱을 닫지 않는다(닫고 나서 실패하면 사용자가 앱을 잃는다) / 같은 파일을 두 번 받기 → 배지가 받는 중에는 눌리지 않는다(시각 속성)
  - **Halt Forecast**: (i) 설치 프로그램을 띄우고 앱을 언제 닫는가 → D9로 확정(띄우기 성공을 확인한 뒤에만 닫는다) / (ii-b) 없음
  - **Depends on**: T2, T3, T4

- [ ] T6. 업데이트 상태 기계 + 워커 — `src/app/update/mod.rs`
  - **Type**: C
  - **Design**: ① `src/app/update/mod.rs`(하위 넷을 선언하고 그 위에 상태를 얹는다) ② 신규 심볼 — `UpdateStatus { Idle, Checking, Available(ReleaseInfo), Downloading, Ready(PathBuf), UpToDate, Failed(UpdateError) }`, `UpdateService { enabled, status, rx }` + `new(enabled: bool)`·`start_check(&mut self, wake)`·`start_download(&mut self, wake)`·`pump(&mut self)`(채널을 비워 상태를 옮긴다)·`status()` ③ 의존 — 하위 넷 + `std::thread`·`std::sync::mpsc`. **`ui::app`이 이것을 소유해 프레임마다 `pump`한다**(단방향 — 이 모듈은 egui를 모른다) ④ **비추상화 선언** — 상태 기계 프레임워크·이벤트 버스를 만들지 않는다. `enum` 하나와 채널 하나다
  - **Design(시험 seam — 게이트와 작업 **둘 다** 주입한다)**:
    - **게이트**: 설치본 판정을 이 모듈이 스스로 하지 않는다 — `new(enabled)`로 받아 필드에 둔다(부르는 쪽 `ui::app`이 `install::is_installed_build()`를 넘긴다). `cargo test`는 `target/debug/deps`에서 돌아 판정이 늘 false이기 때문이다.
    - **작업**: 공개 진입점은 `start_check_with(&mut self, fetch: impl FnOnce() -> CheckResult + Send + 'static, wake)`이고, `start_check`는 **실제 `release::fetch_latest`를 넘기는 얇은 래퍼**다(`start_download`/`start_download_with`도 같은 꼴). 게이트만 주입하면 `new(true)`로 부른 순간 시험 프로세스가 `api.github.com`으로 스레드를 띄워, 단언은 통과하더라도 **4-C의 「네트워크 경로는 시험 대상이 아니다」와 정면으로 어긋나고 시험이 망에 따라 흔들린다**.
    - **가짜 릴리즈(D14)**: `#[cfg(debug_assertions)]`에서 `MOA_UPDATE_DEV=fake`면 `start_check`가 실제 fetch 대신 고정 `ReleaseInfo`를 돌려주는 클로저를 넘긴다 — **시험용 seam을 개발 확인에 그대로 재사용**한다(길을 둘 만들지 않는다).
  - **Acceptance**: Given `UpdateService::new(true)`, When **즉시 값을 돌려주는 클로저**로 `start_check_with`를 부르면, Then ① 시작 상태가 `Idle`이었다가 `Checking`이 되고 ② `Checking` 중에 또 불러도 워커가 둘로 늘지 않으며 ③ `Downloading` 중에 `start_download_with`를 또 불러도 마찬가지고 ④ `pump`가 결과를 `Available`/`UpToDate`/`Failed`로 옮긴다. Given `UpdateService::new(false)`, Then ⑤ `start_check`가 아무 워커도 띄우지 않고 `Idle`에 머문다. **전 구간이 네트워크를 치지 않는다**(그 사실을 시험이 실제로 보증한다 — 주입한 클로저만 불린다). `cargo test` 통과, clippy 0.
  - **Files**: 주: `src/app/update/mod.rs` / 테스트: 같은 파일 `#[cfg(test)] mod tests`
  - **Edge Cases**: 워커가 결과를 보내기 전에 앱이 닫힌다 → 채널이 끊겨 `send`가 실패할 뿐(패닉 없음) / 확인과 다운로드가 겹친다 → 상태가 하나뿐이라 겹칠 수 없다 / 실패 뒤 다시 누르기 → `Failed`에서 `start_check`가 다시 돈다 / 시작 시 확인이 실패 → 조용히 `Failed`, 배지는 뜨지 않는다(Deferred에 적어 둔 판단)
  - **Halt Forecast**: (i) 워커에서 UI를 깨우는 방법 → 기존 관례대로 `repaint` 콜백(`ctx.request_repaint`를 감싼 `Arc<dyn Fn()>`)을 넘긴다(`ui::app::spawn_os_drop_scan`이 `self.repaint.clone()`으로 같은 일을 한다) — 사전 해소
  - **Depends on**: T4, T5

- [x] T7. UI 배선 — 타이틀바 배지·메뉴 두 항목·확인 대화·문구
  - **Type**: D
  - **Design**: ① 배치 — 배지 그리기는 `src/ui/titlebar.rs`(`show_right`의 설정 버튼 왼쪽), 명령은 `src/ui/menu.rs`, 실행·상태 보유·확인 대화는 `src/ui/app.rs`, 문구는 `src/i18n/mod.rs` ② 신규 심볼 — `Command::CheckUpdate`·`Command::StartUpdate`·`Command::OpenReleaseNotes` 세 변형, `titlebar::UpdateBadge { visible, downloading }`(그리는 데 필요한 최소 상태만 나른다 — 타이틀바는 `app::update`를 모른다), `titlebar::update_badge_width(ui, badge)`, `ExplorerApp::update`(서비스 필드)·`show_update_confirm(ctx)` ③ 의존 — `ui::titlebar` → `ui::menu`(기존)만, `app::update`는 `ui::app`만 안다(계층 단방향 유지) ④ **비추상화 선언** — 타이틀바에 "부가 위젯" 슬롯 체계를 만들지 않는다. 배지 하나다
  - **Design(폭 전파, D13)**: `RIGHT_GROUP_WIDTH`(const)를 `RIGHT_GROUP_BASE`로 바꾸고, `show_titlebar`가 `RIGHT_GROUP_BASE + update_badge_width(ui, badge)`를 계산해 ⓐ `drag_right`·`title_width`에 쓰고 ⓑ `TitlebarOutcome.right_group_width`에 실어 돌려준다. `ui::app`이 **같은 프레임에** 그 값을 `show_resize_handles(ctx, maximized, right_group_width)`로 넘기고, 그것이 `over_titlebar_button(x, window, right_group_width)`에 닿는다. 배지가 없으면 폭이 정확히 종전 174다.
  - **Acceptance(자동 — `cargo test`로 관측)**: Given `UpdateBadge { visible: true, downloading: false }`, When 타이틀바를 그리면, Then ① 우측 폭이 `RIGHT_GROUP_BASE`보다 배지 폭만큼 크고 그 값이 `TitlebarOutcome.right_group_width`로 나온다 ② `downloading: true`면 문구가 `다운로드 중...`으로 갈리고 클릭이 나오지 않는다 ③ `visible: false`면 우측 폭이 **정확히 종전 174**라 기존 시험 `위쪽_변은_버튼_구간을_비켜준다`가 그대로 통과하고, 배지 있는 경우의 단언을 그 시험에 **추가**해도 통과한다 ④ **배지 자리에서 눌러도 창 끌기 요청이 나오지 않는다** — 기존 헤드리스 틀(`run_frame`+`press`, `titlebar.rs:517·549`)을 그대로 쓴다. `run_frame`이 `TitlebarOutcome` 전체를 돌려주게 고쳐 그 프레임의 `right_group_width`로 배지 중심 x를 계산하고, 그 자리에서 press→move→release해도 `WindowRequest::Drag`가 나오지 않으며, **같은 x의 창 위쪽 가장자리(y<4)에서 `over_titlebar_button`이 참**이라 크기 조절이 비켜준다(**B1이 겨냥한 회귀의 게이트 — 사람 확인에만 맡기지 않는다**) ⑤ `pending_item`이 소스에서 사라진다(`grep -c "fn pending_item" src/ui/titlebar.rs` → 0) ⑥ `app.rs`의 `apply_command` `match`(1701)에 세 변형의 팔이 실제로 있다(`grep -c "Command::CheckUpdate" src/ui/app.rs` ≥ 1 — 1748의 `_` 팔 때문에 빠뜨려도 컴파일이 깨지지 않으므로 이 확인이 필요하다) ⑦ 새 문구가 모두 카탈로그를 거쳐 `화면_문구가_카탈로그를_거치지_않은_곳이_없다`가 통과하고 ⑧ 배지 아이콘이 phosphor라 `화면_코드에_원본_아이콘_기호가_남아_있지_않다`가 통과한다. `cargo test` 전건 통과, clippy 0.
  - **Acceptance(화면 — HUMAN-VERIFY로 이관)**: 배지가 실제로 뜨는 모습(**A-1**), 설정 메뉴 두 항목을 눌렀을 때 실제로 동작하는 것(**A-4·A-5** — 팝업 메뉴를 헤드리스로 클릭하는 전례가 이 레포에 없어 `ui.button` 소스 대조 이상은 자동으로 덮지 못한다), `다운로드 중...` 전환(**B-8**), 전송 중 확인 대화(**B-9**)는 사람이 본다. D14의 `MOA_UPDATE_DEV=fake`로 A 구간은 **릴리즈 없이** 확인된다.
  - **Files**:
    - 주: `src/ui/titlebar.rs`, `src/ui/menu.rs`, `src/ui/app.rs`, `src/i18n/mod.rs`
    - 동반: `src/ui/titlebar.rs`의 `show_settings_menu` doc 주석과 `pending_item` 제거 · 같은 파일 `RIGHT_GROUP_WIDTH` 정의(23)·`drag_right`(106)·`title_width`(142)·`over_titlebar_button`(387)·`show_resize_handles`(336) · `src/i18n/mod.rs:136~138`의 "나머지 둘은 아직 비활성이다" 주석 · `src/ui/app.rs:1024`(`TitlebarState` 생성)·`1036`(`show_titlebar` 호출)·`1041`(`show_resize_handles` 호출)·`apply_command` `match`(1701)
    - 테스트: `src/ui/titlebar.rs`의 `#[cfg(test)] mod tests` — **기존 3건을 고친다**(`최대화_중에는_크기_조절을_받지_않는다`:488 · `위쪽_변은_버튼_구간을_비켜준다`:492 · `show_titlebar` 호출 헬퍼:535) + 신규(배지 폭 계산·상태별 문구·메뉴 명령). `src/i18n/mod.rs`(문구 왕복 — `LanguageGuard::lock`으로 언어를 잠그고 기대값은 원문 리터럴)
  - **Edge Cases**: 배지 문구가 언어에 따라 길어진다(`Downloading...`) → 폭을 그 자리에서 잰다(시각 속성) / 창이 아주 좁다 → 제목이 먼저 줄고(`truncate`) 배지는 유지 / 설치본이 아니다 → 배지도 활성 메뉴도 없다(D4, 지금 화면과 같다) / 확인 대화가 떠 있는 채 전송이 끝난다 → 대화는 그대로 두고 동의 시 설치를 진행한다(건수는 대화를 띄운 시점의 값) / 다운로드 중 앱을 닫는다 → 워커가 끊길 뿐, 다음 시작이 `update\`를 치운다
  - **Halt Forecast**: (i) `Sides` 레이아웃에서 오른쪽이 **오른쪽부터 채워진다**는 기존 규약(닫기를 먼저 추가) → 배지는 설정 버튼 **뒤에** 추가해야 왼쪽에 놓인다(`show_right` doc 주석이 그 순서를 이미 설명한다) — 사전 해소 / (ii-a) `Command` 열거형(공개 API) 변형 추가 — 사전 승인 항목
  - **Depends on**: T6

- [x] T8. 설치 스크립트 업데이트 모드 — `installer/moa.nsi`
  - **Type**: C
  - **Design**: ① `installer/moa.nsi` 한 파일 ② 신규 — `Var UpdateMode`, `.onInit`에서 `${GetParameters}`로 `/UPDATE` 판정, 페이지 넷(WELCOME·LICENSE·DIRECTORY·FINISH)에 `MUI_PAGE_CUSTOMFUNCTION_PRE` 콜백을 달아 업데이트 모드면 `Abort`(건너뛰기), `SetAutoClose`로 진행 화면 자동 닫기, `.onInstSuccess`에서 업데이트 모드면 `Exec`로 앱 실행 ③ 의존 — `FileFunc.nsh` 추가 include ④ **비추상화 선언** — 업데이트용 별도 스크립트를 만들지 않는다. 한 파일이 두 모드를 갖는다(설치 내용이 같은데 스크립트가 둘이면 한쪽만 고치는 사고가 난다)
  - **Acceptance(자동 — 빌드와 소스로 관측)**: Given `cargo run --example gen_installer`, When 돌리면, Then ① **makensis 경고 0**으로 `target/installer/MOA-Setup-<버전>.exe`가 만들어진다(문법·매크로 순서 오류가 여기서 잡힌다). Given `installer/moa.nsi`, When 읽으면, Then ② `Var UpdateMode`와 `${GetParameters}` 기반 `/UPDATE` 판정이 있고 ③ 페이지 넷(WELCOME·LICENSE·DIRECTORY·FINISH)에 각각 건너뛰기 `PRE` 콜백이 걸려 있으며 ④ `.onInstSuccess`에 업데이트 모드일 때의 `Exec`가 있고 ⑤ 제거 절에 `$INSTDIR\update` 삭제(`RMDir /r`)가 **`RMDir "$INSTDIR"`보다 앞에** 있다.
  - **Acceptance(화면 — HUMAN-VERIFY로 이관)**: `/UPDATE` 없이 실행했을 때 **지금과 똑같은 화면이 나오는 것**(회귀 없음 — **A-3**)과 `/UPDATE`로 실행했을 때 앞 페이지가 건너뛰어지고 끝나면 앱이 다시 뜨는 것(**B-10**), 제거 후 폴더가 남지 않는 것(**B-14**)은 사람이 본다. 설치 프로그램의 페이지 흐름은 실행해야만 관측된다.
  - **Files**: 주: `installer/moa.nsi` / 동반: 그 파일 머리 주석(모드 둘을 설명) · `AGENTS.md`의 설치 파일 생성 줄(T9에서 함께)
  - **Edge Cases**: `/UPDATE`와 `/S`를 함께 준다 → 무음이 이긴다(NSIS 기본) / 인자가 `/update` 소문자다 → 대소문자를 가리지 않고 본다 / 업데이트 모드인데 설치 폴더가 없다(사용자가 지웠다) → `InstallDir` 고정이라 새로 만들어 설치한다 / 언어 → 업데이트 모드는 대화를 띄우지 않고 시스템 표시 언어를 따른다(사용자가 볼 것은 진행 막대뿐) / 바탕화면 바로가기 체크박스 → 마침 페이지를 건너뛰므로 업데이트에서는 만들지 않는다(이미 있던 것은 그대로 남는다)
  - **Halt Forecast**: (i) `Abort`가 페이지를 건너뛴다는 NSIS 관례 → `MUI_PAGE_CUSTOMFUNCTION_PRE`의 정의된 동작이며 makensis로 실제 빌드해 확인한다(빌드가 이 task의 acceptance ①) / (ii-b) 없음
  - **Depends on**: -

- [ ] T9. 문서 — 릴리즈 발행 규약(AGENTS)·배포 줄 정정·README
  - **Type**: A
  - **Acceptance**: Given `AGENTS.md`, When 읽으면, Then ① Build & Test에 **릴리즈 발행 절차**가 순서대로 적힌다 — 버전 올리기(`Cargo.toml`) → `cargo build --release` → `cargo run --example gen_installer` → 설치 파일의 **SHA256 산출 명령**(`certutil -hashfile <파일> SHA256`) → 태그 → GitHub 릴리즈 생성 + 설치 파일 첨부 + **본문에 SHA256 적기**, 그리고 *"본문에 체크섬이 없으면 앱이 그 릴리즈를 받지 않는다"*는 한 줄 ② **릴리즈 노트 작성 규약(D15)이 그 절차 안에 적힌다** — 본문 형식(`새로워진 것`/`고친 것` + 접힌 체크섬 절)과 쓰는 규칙 ⓐ~ⓔ, 그리고 **베끼면 되는 예시 한 벌**. 규칙은 「항목당 1~2줄 · 내부 사정 제외 · 빈 절 제거 · 커밋 목록 금지 · 사용자 행동이 필요하면 맨 위 한 줄」이 드러나야 한다 ③ 「추가 정보」의 배포 줄에서 「자동 업데이트는 없다」가 사라지고 있는 대로 적힌다 ④ `README.md`에 업데이트·릴리즈 노트 기능이 현재 동작대로 실린다(없는 기능을 적지 않는다). `grep -c "자동 업데이트는 없다" AGENTS.md` → 0, `grep -c "새로워진 것" AGENTS.md` ≥ 1.
  - **Acceptance(규약이 실제로 지켜지는지)**: 이 task는 **문서를 쓰는 것까지**다. 그 규약대로 쓴 첫 결과물(v0.1.0 노트)은 릴리즈 발행 승인 시점에 함께 제시해 사용자가 보고 판정한다 — 문서에 적어 두는 것만으로는 지켜졌는지 알 수 없다.
  - **Files**: 주: `AGENTS.md`, `README.md`
  - **Halt Forecast**: (i) 릴리즈 발행 절차를 **적는 것**과 **실행하는 것**은 다르다 — 이 task는 문서만 쓰고, 실제 태그·릴리즈는 「불가피한 Halt」로 따로 승인받는다(그 경계를 흐리면 문서 task가 외부 작업을 끌고 들어온다) / (ii-b) 없음 — 문서 두 파일이라 이 task 안에는 파괴적·외부 작업이 없다
  - **Depends on**: T8

## 사전 승인 항목 (일괄 승인 대상)

- `Cargo.toml`에 `windows` 크레이트의 **`Win32_Networking_WinHttp` feature 추가**(신규 패키지 아님 — 이미 링크된 크레이트의 모듈 하나. `Cargo.lock` 불변, 라이선스 자산 재생성 불요 — 전제 4)
- `src/app/update/` 폴더와 그 아래 파일 다섯 신규 생성, `src/app/mod.rs`에 `pub mod update;` 한 줄
- `ui::menu::Command`에 변형 3개 추가(공개 열거형 — 실해가 있는 `match`가 `app.rs:1701` 한 곳이라 영향이 닫혀 있다)
- **`titlebar::show_titlebar`·`show_resize_handles`의 시그니처 변경**(둘 다 `pub`이지만 호출부가 각각 앱 1곳 + 시험 1곳뿐이다 — 4-A). `RIGHT_GROUP_WIDTH` 상수를 런타임 계산으로 바꾸는 것(D13)이 여기 포함된다
- `src/ui/titlebar.rs`의 `pending_item` 함수 **삭제**(쓰임이 사라져 `-D warnings`를 깨뜨린다 — 4-A)
- 기존 시험 3건(`titlebar.rs:488·492·535`)의 호출·단언 수정 — 시그니처 변경에 따른 것이며 검증을 약하게 하지 않는다(배지 있는 경우의 단언을 **더한다**)
- `docs/prd.md` 개정(T1의 acceptance에 적힌 문면 그대로)·`AGENTS.md`·`README.md` 갱신
- `installer/moa.nsi` 수정(기존 설치 경로의 동작은 보존 — T8 acceptance ②)
- 로컬 작업 브랜치 commit

## 불가피한 Halt (위임 불가)

- **첫 릴리즈 발행** — `v0.1.0` 태그 + GitHub 릴리즈 생성 + 설치 파일 업로드 + 본문(D15 형식의 사용자용 노트 + 접힌 SHA256). 외부·비가역이라 구현·검증이 끝난 뒤 그 지점에서 이름을 적어 따로 승인받으며, **그때 릴리즈 노트 초안을 함께 보여** 사용자가 문면을 보고 판정하게 한다(D15가 지켜졌는지는 결과물로만 알 수 있다).
- **push / 병합 / PR** — 같은 이유로 별도 승인.
- 계획에 없던 결정이 필요해지는 지점(예: GitHub API 응답이 전제와 다른 형태로 드러남).

## Verification Strategy

- 자동: `cargo build --release` → `cargo clippy --all-targets -- -D warnings` → `cargo test` → `cargo fmt --check` → `cargo run --example gen_installer`

**확인은 두 구간으로 갈린다** — 릴리즈가 없어도 되는 것(A)과 첫 릴리즈 발행 뒤에야 되는 것(B). A를 먼저 통과시킨 뒤에 발행 승인을 청하고, B는 그 뒤에 한다.

- **HUMAN-VERIFY A** (릴리즈 없이 지금 확인 — D14의 `MOA_UPDATE_DEV=fake`를 켜고 개발 빌드로. **가짜 릴리즈가 주입되므로 저장소에 릴리즈가 0건이어도 배지가 뜬다** — 버전을 낮춰 빌드할 필요가 없다):
  1. 배지가 설정 기어 왼쪽에 `↓ 업데이트`로 뜨고 글자가 잘리지 않는다. 언어를 영어로 바꿔도 `Update`가 잘리지 않는다
  2. **배지 위에서 창을 끌어도 창이 움직이지 않고**, 배지 바로 위 가장자리에서 크기 조절 커서가 뜨지 않으며, 그 자리를 누르면 배지가 눌린다 (B1이 겨냥한 회귀 — **자동 시험 T7-④가 같은 것을 먼저 잡는다**. 여기서는 눈으로 재확인)
  3. `/UPDATE` **없이** 설치 파일을 실행하면 지금과 똑같은 화면이 나온다 — 언어 선택 → 환영 → 라이선스 → 폴더(입력란 잠김) → 설치 → 마침(바탕화면 바로가기 체크박스) (**회귀 없음**)
  4. 설정 메뉴 → `릴리즈 노트`를 누르면 기본 브라우저에 GitHub 릴리즈 페이지가 열린다
  5. 설정 메뉴 → `업데이트`를 누르면(`MOA_UPDATE_DEV=1` — 가짜 주입 없이) 확인이 돌고 **`최신 버전입니다`** 안내가 뜬다 (릴리즈 0건이라 `Ok(None)` → `UpToDate`. 손으로 누른 확인의 결과 표시 경로가 여기서 덮인다)
  6. `MOA_UPDATE_DEV` 없이 개발 빌드를 실행하면 배지가 없고 설정 메뉴의 **`업데이트`가 비활성**이다 (D4). **`릴리즈 노트`는 그때도 활성이다** — 브라우저를 여는 것뿐이라 설치본과 무관하고, FR-63에도 설치본 조건이 없다(D4의 「이 항목도 비활성」은 `업데이트`를 가리킨다). `cargo build --release`로 만든 exe를 직접 실행해도 같다
- **HUMAN-VERIFY B** (첫 릴리즈 발행 뒤 — 별도 승인이 난 다음):
  7. 새 버전 릴리즈가 있을 때 설치본을 띄우면 **시작이 느려지지 않고**(체감) 잠시 뒤 배지가 뜬다
  8. 누르면 `다운로드 중...`으로 바뀌고, 설치 폴더에 `update\` 폴더와 설치 파일이 생긴다
  9. 전송을 큰 파일로 걸어 둔 채 업데이트를 누르면 **확인 대화가 먼저** 뜨고, 취소하면 받지 않는다
  10. 다 받으면 설치 화면이 뜨고 실행 중이던 앱이 닫히며, 끝나면 앱이 다시 뜬다 — 언어 선택·환영·라이선스·폴더 페이지가 **뜨지 않는다**
  11. 다시 뜬 앱의 정보 화면 버전이 새 버전이고 `update\` 폴더가 사라져 있다
  12. 설정 메뉴 → `업데이트`를 누르면(최신 상태에서) `최신 버전입니다` 안내가 뜬다
  13. 릴리즈 본문에서 SHA256을 지우거나 한 글자 고친 뒤 받아 보면 **설치가 시작되지 않고** 받은 파일이 남지 않는다 (D3)
  14. 제거하면 설치 폴더가 통째로 사라진다(`update\` 잔존 없음)

## 리뷰 이력

**2라운드로 재호출 상한을 소진했다 — 수렴이 아니라 예산 소진으로 끝냈다.** 1라운드 지적 9건은 2라운드에서 **전부 닫힌 것이 코드 대조로 확인**됐고(동일 지적 잔존 0), 2라운드가 낸 신규 7건은 **메인이 실물과 대조해 전건 처리**했다. 라운드를 더 열지 않은 이유는 그 구간의 수정이 새 결함을 만들기 때문이며(실제로 2라운드 신규 3건이 1라운드 수정의 산물이었다), 여기서의 정답은 라운드 추가가 아니라 대조와 공시다.

| 라운드 | 지적 | 심각도 | 처리 |
|---|---|---|---|
| 1 | B1 `over_titlebar_button` 누락 + 상수/런타임 폭 모순 | BLOCKER | 수용 — D13 신설, 4-A 재작성. **2라운드에서 폭 전파 경로가 닫힌 것을 코드 대조로 확인**(app.rs 1036→1041 같은 함수, 값이 프레임을 건너지 않는다) |
| 1 | M1 시그니처 변경 누락 / M2 T6 seam / M3 T8 검증 공백 / M4 T7 화면 확인 불가 / M5 Halt Forecast / m1 `match` 서술 / m2 PRD 이력 / m3 `app`→`remote` | MAJOR·MINOR | 전건 수용(m2는 부분 — 날짜 붙은 이력은 원문 보존, m3은 사실 정정) |
| 2 | **B1(new)** A 구간이 릴리즈 0건에서 실행 불가 — D14가 설치본 판정만 뚫고 릴리즈 존재를 뚫지 않아, 1라운드가 세운 회귀 게이트(A-2)가 첫 릴리즈까지 한 번도 발동하지 않는다 | BLOCKER | **수용(양쪽 다)** — ⓐ D14를 `MOA_UPDATE_DEV=fake`(가짜 릴리즈 주입)까지 넓혀 A 구간이 실제로 돌게 했다 ⓑ 그 회귀를 **자동 시험으로 승격**(T7 acceptance ④ — 기존 헤드리스 틀 `run_frame`+`press` 재사용, 전례 4건 실재). 사람 확인에만 맡기지 않는다 |
| 2 | **M1(new)** T5 ④가 자동 구간인데 다운로드가 선행돼 관측 불가 | MAJOR | 수용 — `verify_downloaded(path, hex)`를 분리해 대조 규칙만 시험하고, 전 구간은 B-13으로 이관 |
| 2 | **M2(new)** T6 ②③이 실제 HTTP를 침(게이트만 주입했고 워커 본문은 그대로) | MAJOR | 수용 — `start_check_with(fetch, wake)`로 **작업까지 주입**하고 `start_check`를 얇은 래퍼로. 같은 seam을 D14의 가짜 주입에 재사용해 길을 둘 만들지 않았다 |
| 2 | m1 Investigation Log가 아직 "둘을" / m2 `over_titlebar_button` 호출 수 / m3 HUMAN-VERIFY 상호참조 번호 어긋남 | MINOR | 전건 수용 — 발원 문장을 "셋"으로 고치고, 시험 호출 3회를 명시하고, 참조를 A-1~6·B-7~14 번호로 맞췄다 |
| 2 | m4 T7 ④(메뉴 항목 클릭)에 팝업 헤드리스 클릭 전례가 없다 | MINOR(유보) | **수용 — 자동에서 뺐다.** 소스 대조 이상을 자동으로 덮지 못한다는 지적이 맞다. 메뉴 클릭 동작은 A-4·A-5로 사람이 본다(그 둘은 릴리즈 없이 지금 확인된다) |

## Phase Ledger

## Retry Ledger

## Progress Log

- T1~T3 완료 (커밋 c5e57f3·063f5e2·9f7c8a4): PRD에 FR-62·FR-63 신설 → WinHTTP GET 둘 → CNG SHA256. 신규 크레이트 0건(feature 하나만 추가), `Cargo.lock` 불변이라 라이선스 자산 재생성도 불요(전제 4 실측 확인).
  - T2 리뷰: spec OK·quality OK. 주석 표기를 레포 관례(`// 안전성:`)로 통일하고 plan의 Design 문구를 실제 심볼명에 맞췄다.
  - T3 리뷰 spec MAJOR 1(`matches`가 계획에 없고 호출자 0) → **같은 회차 T5가 실제로 부르게 되어 해소**. 리뷰어가 제시한 첫 해법대로 T3·T5 Design에 재사용 관계를 명시.
- T4~T6 구현 (커밋 36d5388·804dda0·045d41b): 릴리즈 조회·판정 → 설치본 판정·내려받기·대조·설치 실행 → 상태 기계와 워커.
  - **T4 spec BLOCKER 1 → 구현 유지 + plan 개정으로 해소**(리뷰어가 판단을 메인에 위임). `parse_release`가 `Option`이 아니라 `Result`인 근거 셋을 Design에 적었다 — 실패 사유 구분(FR-62의 「결과를 알린다」)·`sha256: String`으로 D3을 타입 강제·`current` 인자는 시험 seam.
  - T5 리뷰 spec OK. T4 quality OK.
  - **결정(T6)**: 상태 기계를 `mod.rs`가 아니라 `service.rs`에 뒀다 — 모듈 구성과 상태 흐름은 변경 이유가 달라 한 파일에 섞으면 하위 모듈을 더할 때마다 300줄을 지나야 한다(AGENTS 분할 판정 ①②). `mod.rs`에서 재수출해 **밖에서 보는 이름은 `app::update::UpdateService` 그대로**다.
  - **결정(T6)**: 게이트(`enabled`)뿐 아니라 **작업 자체도 주입**한다(`start_check_with`·`start_download_with`). 게이트만 주입하면 `new(true)`로 부른 시험이 진짜 `api.github.com`을 두드려 4-C의 「네트워크 경로는 시험 대상이 아니다」와 어긋난다. 개발용 가짜 릴리즈(D14)도 같은 자리를 재사용해 길을 둘 만들지 않았다.

## Next Steps

## Open Questions

- [x] HTTP 수단 → **WinHTTP**(D1)
- [x] 설치 실행 방식 → **`/UPDATE` 모드 신설**(D2)
- [x] SHA256 검증 → **한다**(D3)
- [x] 개발 빌드에서의 동작 → **설치본에서만 켠다**(D4)
- [x] 이번 회차 범위 → **구현 + 발행 규약 문서화 + 첫 릴리즈 발행**(릴리즈는 별도 승인 — 「불가피한 Halt」)
- [x] 타이틀바 표시 모양 → **아이콘 + 글자**(시각 속성 표)
- [x] 전송 중 설치 → **묻고 진행**(D5)
