# Plan: 정보 화면 (FR-58)

**PRD**: docs/prd.md

## 요구 이해

- **원문 요청**: (참고 이미지 첨부와 함께) *"이미지처럼 정보 화면 구현"* → 이어진 정정: *"이미지하고 동일하게 만드는게 아니고 이미지와 같은 형식으로 팝업에 아이콘 표시하고 아이콘 아래 앱 이름 및 버전을 표시하는 것임. 그리고 이미지는 축소하면 자글자글하게 보일수 있는데 깔끔하게 표시 하도록 하고. 이미지 : docs/AppIcon.png"*
- **이해한 요구**: 타이틀바 설정 메뉴에서 표시만 되고 눌리지 않던 `정보`를 실제로 여는 팝업으로 만든다. 팝업은 참고 이미지와 **같은 구성**(가운데 앱 아이콘, 그 아래 앱 이름과 버전 한 줄)이며, 참고 이미지의 색·간격·글자 크기를 픽셀 단위로 베끼는 작업이 아니다. 아이콘은 `docs/AppIcon.png`에서 오고, **축소 과정에서 계단·자글거림이 보이지 않아야** 한다.
- **포함하지 않는 것으로 이해**: 같은 메뉴의 `업데이트`·`릴리즈 노트` 둘은 이번에 건드리지 않는다 — 비활성 그대로 남는다.

## Goal

설정 메뉴의 `정보`가 팝업을 열고, 그 안에 96px 앱 아이콘과 `MOA 0.1.0` 한 줄이 가운데 서며, 아이콘은 어떤 화면 배율에서도 축소 자글거림 없이 그려진다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-58 (신설) | Should | T2·T3 (신설 문면은 T4) | ✅ 커버 |
| FR-22 (문면 개정) | Should | T4 | ✅ 커버 — 「나머지 셋은 항목 표시만」이 실제와 어긋나게 된다 |
| Out of Scope 조항(`prd.md:120`) 재한정 | — | T4 | ✅ 커버 — 「`정보`는 계속 제외」가 이번 구현과 모순된다 |
| 성공 기준 Should 목록(`prd.md:126`) | — | T4 | ✅ 커버 — FR-58이 빠지면 80% 판정 모수가 실제와 어긋난다 |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- 설정 메뉴의 `업데이트`·`릴리즈 노트` 활성화 — 사용자가 이번 범위를 `정보` 하나로 좁혔다. 둘은 `docs/plans/deferred.md` 대기 항목으로 남는다.
- 저작권 줄·라이선스 화면으로 가는 버튼 — 사용자가 「아이콘 + 이름·버전만」을 골랐다.
- 빌드 날짜·커밋 해시·시스템 정보 표시 — 요청에 없다.
- 앱 이름 상수를 공용 자리로 통합 — 지금 네 모듈(`autostart`·`settings`·`tray`·`main`)이 각자 `"MOA"`를 든다. 이번 요청 범위 밖이며 호출부 넷을 건드린다.

## Deferred / Follow-up

- 정보 팝업에 저작권·홈페이지 링크 더하기 — 이번엔 사용자가 최소 구성을 골랐다. 나중에 줄만 더하면 되는 자리다.
- 앱 이름 `"MOA"` 상수 다섯 곳(이번 대화 포함) 통합 — 값이 바뀔 일이 드물어 급하지 않으나, 바꿀 때 다섯 곳을 찾아야 한다.
- (직전 plan에서 이관) 라이선스 목록 검색·필터 · 라이선스 종류별 묶어 보기 · `examples/gen_licenses.rs`의 `main` 중복 — 이번 작업과 무관하므로 T4가 `docs/plans/deferred.md` 대장으로 옮긴다.

## Investigation Log

- 위키 참조: `20_projects/personal/moa/feat-titlebar-tray` — 설정 메뉴의 정본. **다섯 항목 중 비활성인 것에 대해 "눌러도 반응이 없으면 고장으로 오인된다"**는 이유로 `add_enabled(false)`를 쓴다고 적혀 있다(`feat-titlebar-tray.md:48`). 이번에 하나가 더 활성이 되므로 그 페이지 서술도 갱신 대상이 된다(위키 갱신은 별도 세션).
- 위키 참조: `20_projects/personal/moa/feat-dialog-shell` — 셸의 공개 표면이 `show`(본문이 높이를 정한다 · `body_width`를 받는다)와 `show_fixed`(대화가 크기를 잡는다 · 프레임 크기를 받는다) 둘임을 확인(`feat-dialog-shell.md:50`). 본문 안쪽 여백 18px은 `show`만 입힌다(`:68`).
- 위키 참조: `20_projects/personal/moa/decisions` — `정보`·about 화면에 관한 과거 보류·기각 결정 없음(문자열 검색 무매칭). 상충 항목 0건.
- Deferred 대장(`docs/plans/deferred.md`, 대기 73건(실측)): 관련 항목은 `[2026-07-29] 설정 팝업 **세 항목**의 실제 기능 — 업데이트·릴리즈 노트·정보`. 이번에 **셋 중 하나가 해소**되므로 삭제가 아니라 둘로 좁히는 **정정** 대상이다(T4). 잔량 73건은 소진 batch 임계(100건)에 미달하고 직전 batch 재확인이 2026-08-18이라 이번 회차에 batch를 열지 않는다. 전제 반증 항목은 제목 스캔 결과 0건.
- `docs/AppIcon.png`는 **1083×1105**(실측, 32bpp ARGB, 1,331,154B) — 정사각이 아니다. 네 모서리 알파는 0이다.
- `docs/AppIcon.ico`에는 16·24·32·48·64·72·96·128px의 32bpp DIB와 256px PNG 항목이 들어 있다(실측 9항목). 현재 `ui::app_icon`은 **PNG 항목을 건너뛰므로** 그 경로로 얻을 수 있는 최대는 128px이다.
- **품질 실측**(방법 포함): PowerShell `System.Drawing`으로 ⓐ `AppIcon.ico`에서 96px 항목을 뽑은 것과 ⓑ `AppIcon.png`(1083×1105)를 `InterpolationMode = HighQualityBicubic`으로 96×96까지 줄인 것을 각각 PNG로 저장해(스크래치패드 `ico96.png`·`png96.png`) 나란히 띄워 확인했다 — 육안 차이가 없었다. 즉 원본 PNG를 고품질 필터로 줄이면 ICO에 담긴 것과 같은 품질이 나오며, 자글거림의 원인은 원본이 아니라 **축소 방법**이다(참고: 같은 원본을 `NearestNeighbor`로 줄인 것은 가장자리가 확연히 거칠다).
- **참고 이미지 실측**(구성 비율의 근거): 사용자가 준 이미지(772×226)에서 아이콘은 96×96, 그 아래 이름·버전 줄의 대문자 높이는 19px, 텍스트 색은 가장 밝은 곳이 `#E0E1E4`, 카드 테두리는 `#3A3A3B` 2px, 아이콘과 텍스트 모두 카드 가로 중앙(376.5px)에 정렬돼 있었다. 아이콘 아래 끝(151)과 대문자 위 끝(180) 사이는 29px이다. **테두리가 2px이고 우리 셸이 1px인 점으로 보아 그 스크린샷은 200% 배율 화면에서 찍힌 것으로 보이며**, 논리 픽셀로 환산하면 아이콘 48·글자 크기 약 13.5·간격 약 14.5가 된다 — 사용자가 표시 크기를 96px로 정했으므로 **간격·글자 크기는 그 비율(아이콘의 약 1/6·1/7)에 맞춰 D12에서 확정한다**.
- `image` 0.25.10과 `png` 0.18.1은 **이미 실제로 링크되는 의존성**이다 — `cargo tree -i image`가 `arboard → egui-winit → eframe → moa`와 `eframe → moa` 두 경로를 보인다. `cargo tree -e features`에서 **`image` feature `png`도 이미 켜져 있다**. 따라서 `image`를 명시 의존으로 올려도 패키지 집합·feature 집합이 변하지 않는다.
- 라이선스 자산 지문(`app::licenses::lockfile_fingerprint`)은 `Cargo.lock`의 `name = `·`version = ` 줄만 FNV로 해싱한다(`src/app/licenses.rs:122-139`). 위와 같이 패키지 집합이 그대로이므로 **지문이 변하지 않고 `assets/licenses.json` 재생성도 불필요하다**.
- `image` 0.25.10의 `[features]`에 `png`가 있다(레지스트리 원본 `Cargo.toml:87` 실측) — `default-features = false, features = ["png"]`로 쓸 수 있다. `imageops`는 feature 없이 딸려 온다.
- `dialog::show`는 버튼 배열이 비면 하단 44px 자리를 아예 잡지 않고, 버튼이 있으면 프레임 전폭을 나눈다(`src/ui/dialog.rs:233-262`·`:107-124`). 본문 Frame에 `BODY_MARGIN`(18) 여백을 셸이 입히므로 **본문과 버튼 줄 사이 여백도 셸이 이미 만든다**.
- `dialog::frame()`은 `SURFACE_BG`(#1E1E1E) 채움 + `BORDER_CONTROL`(#3A3A3A) 1px 테두리 + 모서리 12px + 그림자다(`src/ui/dialog.rs:169-183`). 참고 이미지의 카드(테두리 #3A3A3B·둥근 모서리·어두운 채움, 실측)와 사실상 같은 모양이라 **팝업 껍데기를 새로 만들 이유가 없다**.
- 기존 대화 배선: `Command::OpenLicenses => self.license_dialog.open()`(`src/ui/app.rs:2066`)과 프레임마다 `self.license_dialog.show(&ctx)`(`:2871`) 두 곳뿐이다. `LicenseDialog`의 표면은 `new`/`open`/`is_open`/`close`/`show`다(`src/ui/license_dialog.rs:69-89`).
- 설정 메뉴에서 `정보`는 지금 `pending_item(ui, crate::i18n::titlebar_about())` 한 줄이다(`src/ui/titlebar.rs:265`). `pending_item`은 `add_enabled(false, Button::new(label))`이며 나머지 두 항목(`titlebar_updates`·`titlebar_release_notes`)이 계속 쓴다 — **함수는 남는다**.
- 텍스처 로드 선례: `app_icon::load_texture(ctx, 48)`을 앱이 시작할 때 한 번 불러 `Option<TextureHandle>`로 든다(`src/ui/app.rs:635`·`:66`). 타이틀바는 그것을 20px 자리에 그린다.
- `i18n` 소스 훑기 시험(`화면_문구가_카탈로그를_거치지_않은_곳이_없다`)이 훑는 뿌리에 `src/ui`가 들어 있다(`src/i18n/mod.rs:1058-1064`) — 새 모듈도 검사 대상이다.
- `i18n::close()`가 `"닫기" / "Close"`로 이미 있다(`src/i18n/mod.rs:125`).

### 전제 검증

| 전제 | 확인 근거 | 판정 |
|---|---|---|
| `image` 크레이트를 써도 라이선스 자산이 낡지 않는다 | `cargo tree -i image`·`-e features`로 image·png와 `png` feature가 이미 활성임을 확인 + 지문 함수가 `name`/`version` 줄만 해싱(`licenses.rs:122`) | ✅ 확인 |
| PNG를 `image`로 디코드·축소할 수 있다 | 레지스트리 `image-0.25.10/Cargo.toml`의 `[features]`에 `png` 존재 | ✅ 확인 |
| 고품질 축소면 자글거림이 사라진다 | ICO 96px 항목 vs PNG 고품질 축소 96px 실측 비교 — 육안 차이 없음 | ✅ 확인 |
| `dialog::show`로 버튼 하나짜리 작은 팝업을 만들 수 있다 | `dialog.rs:233-262` 코드 확인(빈 배열이면 자리도 안 잡는다) | ✅ 확인 |
| 새 대화·문구가 기존 소스 훑기 시험 3종에 걸린다 | `dialog.rs:474`(Modal 직접 사용 금지)·`i18n/mod.rs:1054`(한글 리터럴)·아이콘 기호 시험의 훑는 뿌리에 `src/ui` 포함 | ✅ 확인 |
| `Command`에 변형을 더하면 매치 누락을 컴파일러가 잡는다 | `src/ui/app.rs:2065` 부근이 `Command`를 매치하며 `_` 갈래가 없음(줄 단위 확인) | ✅ 확인 |
| 원본 PNG를 정사각으로 리샘플해도 그림이 어긋나 보이지 않는다 | 배포 중인 `AppIcon.ico`가 이미 정사각(96×96 등)이고, 그것과 PNG 정사각 축소본의 육안 비교에서 차이 없음 | ✅ 확인 |

## 동반 변경 판정

| 대상 | 구분 | 처리 |
|---|---|---|
| `src/ui/titlebar.rs:240` doc 주석 「`설정`과 `오픈소스 라이선스`가 동작하고 나머지 셋은 아직 표시만 한다」 | **필수** (이번 변경이 어긋나게 한다) | T3에 편입 — 「나머지 둘」로 고친다 |
| `src/i18n/mod.rs:136` doc 주석 「설정 메뉴의 나머지 셋(…·`정보`)은 아직 비활성이다」 | **필수** | T3에 편입 |
| PRD FR-22 문면·Out of Scope 조항(`prd.md:120`)·성공 기준 Should 목록 | **필수** (구현과 모순된 채로 남는다) | T4에 편입 (FR-58 신설 포함) |
| `README.md` — 기능·화면 목록 | **필수** (기능 추가) | T4에 편입 |
| `AGENTS.md` — 새 자산(`assets/app_icon_256.png`)과 그 생성기, `image` 의존 | **필수** (Repository Structure·Build & Test가 실제와 어긋난다) | T4에 편입 |
| `docs/plans/deferred.md`의 「설정 팝업 세 항목」 항목 | **필수** (셋 중 하나가 해소돼 서술이 틀린다) | T4에 편입 — 둘로 좁힌다 |
| 직전 plan의 Deferred 3건 | **필수** (plan 교체로 사라진다) | T4에 편입 — 대장 `## 대기`로 이관 |
| 위키 `feat-titlebar-tray`·`feat-dialog-shell` 페이지 | 무관 (이번 diff 대상 아님) | 코드 레포 밖이며 위키 갱신은 별도 세션 규약 — 건드리지 않는다 |
| `ui::app_icon`의 ICO 경로 | 무관 | 타이틀바·트레이·창 아이콘이 계속 쓴다. 이번 팝업만 PNG 경로를 쓴다(사유는 D2) |

## Impact Analysis

### 4-A. 심볼/타입 추적

| 대상 | 사용처(전수) | 처리 |
|---|---|---|
| `menu::Command` (열거형) | 매치 1곳 `src/ui/app.rs:2065-2072`, 생성 `src/ui/titlebar.rs:255·262` 외 패널 메뉴 | `OpenAbout` 변형 추가 — 매치 누락은 컴파일 오류로 드러난다 |
| `titlebar::pending_item` | `titlebar.rs:258·259·265` 3곳 | `:265`만 활성 버튼으로 바꾼다. 나머지 둘이 남아 함수는 존치 |
| `i18n::titlebar_about()` | `titlebar.rs:265` 1곳 | 호출 그대로, 감싸는 위젯만 바뀐다 |
| `ui::mod`의 모듈 선언 | `src/ui/mod.rs:5-37` | `pub mod about_dialog;` 한 줄 추가(알파벳 순 — `address_bar` 앞) |
| `App` 구조체 필드·생성 | `src/ui/app.rs:495-496`(라이선스 선례)·`:668` | `about_dialog: AboutDialog` 필드와 초기화 추가 |

### 4-B. 계약·직렬화 변경

없다. 세션 파일(`settings.json`) 스키마·저장 대상이 바뀌지 않는다 — 팝업은 열림 상태를 저장하지 않는다(라이선스 대화와 같다).

### 4-C. 영향 받는 테스트

- 기존 소스 훑기 시험 3종(`대화는_모두_이_모듈을_거친다`·`화면_문구가_카탈로그를_거치지_않은_곳이_없다`·`화면_코드에_원본_아이콘_기호가_남아_있지_않다`)이 새 모듈을 자동으로 검사 대상에 넣는다. 별도 수정 불필요.
- 라이선스 지문 대조 시험은 위 Investigation Log대로 영향 없다.
- 신규: 아이콘 자산 디코드·축소 결과에 대한 단위 시험, 표시 물리 크기 계산 시험, 이름·버전 문구 조합 시험(T1·T2).

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `ui::about_dialog::AboutDialog` | `LicenseDialog`·`SettingsDialog`가 `new`/`open`/`is_open`/`close`/`show` 표면을 갖는다 | **표면을 그대로 따른다**(구조 신규, 규약 재사용) — 두 대화와 같은 모양이어야 배선이 한 줄로 끝난다 |
| 팝업 프레임·하단 버튼 | `ui::dialog::show` | **재사용** — 새로 만들지 않는다(규약이 소스 훑기 시험으로 강제) |
| PNG 디코드·축소 | `ui::app_icon`은 ICO 32bpp DIB 전용이고 PNG 항목을 일부러 건너뛴다(`app_icon.rs:5-7`) | **신규** — 기존 경로로는 96px 넘는 원본을 못 읽는다. `image` 크레이트로 디코드·리샘플 |
| 텍스처 업로드 | `ctx.load_texture` + `ColorImage::from_rgba_unmultiplied` (`app_icon.rs:41-44`) | **같은 방식을 따른다** |
| 가운데 정렬 텍스트 그리기 | `license_dialog::show_header`의 `painter().text` + `Align2` | **같은 방식을 따른다** |
| 아이콘 자산 생성기 | `examples/gen_licenses.rs`(개발용 CLI, `fn main() -> Result<_, String>`) | **같은 형태로 신규** — 커밋되는 생성물은 생성기가 만든다는 규약(AGENTS.md) |

## Decisions

| # | 항목 | 선택 | 근거 |
|---|---|---|---|
| D1 | 아이콘 원본 | `docs/AppIcon.png`를 **256×256으로 줄인 `assets/app_icon_256.png`**를 exe에 담는다 | 사용자 결정. 원본 1.3MB를 통째로 담지 않아 exe 증가가 **60KB 안팎(추정 — T1이 생성 후 실측해 확정한다)** 에 그치고, 화면 배율 266%까지 축소만 일어나 무손실 |
| D2 | 축소 방법 | 표시 물리 크기(`96 × pixels_per_point`)를 구해 **CPU에서 `image::imageops::resize`(Lanczos3)** 로 줄인 뒤 텍스처로 올린다 | 자글거림의 원인은 GPU 선형 필터의 큰 배율 축소다(인접 4텍셀만 본다). 실측에서 고품질 필터 결과는 ICO 원본과 육안 동일 |
| D3 | 표시 크기 | 96 논리 px 정사각 | 사용자 결정 |
| D4 | 팝업 내용 | 아이콘 + `MOA 0.1.0` 한 줄 + `닫기` 버튼 | 사용자 결정(최소 구성) |
| D5 | 셸 함수 | `dialog::show`(본문이 높이를 정한다), 본문 폭 **248px** | 내용이 아이콘과 한 줄뿐이라 고정 크기를 잡을 이유가 없다. 96px 아이콘 좌우로 넉넉한 여백이 남고 프레임 폭은 `248 + 18×2 = 284`가 된다 |
| D6 | 종횡비 | 원본 1083×1105를 **256×256 정사각으로 리샘플**한다(가로 약 2% 확대) | 배포 중인 `AppIcon.ico`가 이미 정사각이며 그것과 정사각 축소본의 육안 비교에서 차이가 없었다. 여백을 덧대 종횡비를 지키면 아이콘이 그만큼 작아 보인다 |
| D7 | 의존성 | `image = { version = "0.25", default-features = false, features = ["png"] }`를 **명시 의존으로 올린다** | 이미 `eframe`·`arboard` 경유로 링크되고 `png` feature도 켜져 있다(실측) — 패키지·feature 집합이 변하지 않아 라이선스 자산 재생성이 없다. **exe 코드 증가는 이 판단이 틀렸다** — 우리가 새로 부르는 디코드·리샘플 경로가 링크돼 약 60KB 늘었다(Phase F 실측, `Cargo.toml` 주석도 함께 정정). 전이 의존에 기대어 쓰면 상위가 그것을 끊는 순간 조용히 깨진다 |
| D8 | 앱 이름 | 화면에 보이는 이름은 **i18n 카탈로그의 `about_app_name`** 이 정본이다(D9). 다른 네 곳(`autostart`·`settings`·`tray`·`main`)의 `"MOA"` 상수는 건드리지 않는다 | 그 넷은 레지스트리 값 이름·폴더 이름·창 제목이라 **화면 문구가 아니고** 언어를 따라가서도 안 된다. 통합은 호출부 넷을 건드려 범위를 넘는다(Deferred에 등재) |
| D9 | 문구 카탈로그 | **이름도 카탈로그를 거친다** — `strings!`에 `about_app_name => "MOA" / "MOA"`를 더하고, 이름과 버전을 잇는 자리는 값이 끼어들므로 `i18n::dynamic`에 `about_version_line()` 함수로 둔다. `닫기`는 기존 `i18n::close()` | **두 언어에서 값이 같아도 카탈로그에 넣는 것이 이 레포의 선례다** — `settings_language_english => "English" / "English"`가 주석까지 달고 그렇게 돼 있다(`src/i18n/mod.rs:121-124`). NFR-6(`prd.md:99`)은 "새로 넣는 화면 문구는 카탈로그를 거치며 소스에 직접 박지 않는다"에 예외를 두지 않았고, `AGENTS.md:78`의 예외 3종(위젯 열쇠·외부에서 온 문자열·개발자용 메시지)에도 해당하지 않는다. 소스 훑기 시험이 한글 리터럴만 잡아 **어긋나도 아무 시험에 걸리지 않는 자리**라 더욱 규약을 따른다 |
| D10 | 텍스처 수명 | 대화가 `(TextureHandle, 만든 물리 크기)`를 들고, 필요한 물리 크기가 달라졌을 때만 다시 만든다 | 앱 시작 시 만들면 한 번도 열지 않는 팝업 때문에 콜드 스타트(NFR-1)에 디코드 비용이 붙는다. 배율은 창을 다른 모니터로 옮기면 바뀐다 |
| D11 | 버전 출처 | `env!("CARGO_PKG_VERSION")` | `Cargo.toml`이 정본이며 컴파일 시점에 박힌다. 따로 상수를 두면 둘이 갈린다 |
| D12 | 이름·버전 줄의 시각 값 | **글자 크기 16px · 색 `theme::TEXT` · 아이콘과의 간격 16px · 아이콘 위와 텍스트 아래 여백은 셸의 `BODY_MARGIN`(18px) 그대로** | 참고 이미지 실측을 96px 아이콘 기준으로 환산한 값이다(간격 29px → 아이콘의 약 1/6, 글자 대문자 높이 19px → 본문 13px보다 한 단 큰 자리). 16px은 `license_dialog`·`settings_dialog`의 제목 크기와 같아 이 앱 안에서 새 치수를 만들지 않는다. 색은 실측 `#E0E1E4`에 가장 가까운 기존 상수가 `TEXT`(#E8E8E8)다 — 새 색을 만들지 않는다. **구현자가 임의로 정하지 않도록 값으로 못 박는다** |
| D13 | 텍스처 옵션과 픽셀 정렬 | `TextureOptions::LINEAR`로 올리되(선례 `app_icon.rs:44`), **텍스처를 표시 물리 크기와 같은 크기로 만들고 아이콘 사각형의 좌상단을 물리 픽셀 격자에 맞춘 정수 좌표로 잡는다** | 물리 크기가 일치하면 GPU가 확대·축소를 하지 않아 필터 종류가 결과를 바꾸지 않는다. 다만 사각형이 반픽셀 어긋난 자리에 놓이면 그 지점에서 다시 흐려지므로 좌표를 격자에 맞춘다(수단은 구현자 재량 — egui의 픽셀 반올림 헬퍼 또는 직접 계산) |

## Tasks

- [x] **T1. 아이콘 자산과 생성기** — Type C
  - **Design**: ① 배치 — `examples/gen_app_icon.rs`(신규 개발용 CLI), 산출물 `assets/app_icon_256.png`(커밋). ② 신규 심볼과 책임 — `fn main() -> Result<(), String>`(원본을 읽어 256×256 Lanczos3로 줄여 PNG로 쓴다). ③ 의존 방향 — `image`만 참조하고 앱 코드를 참조하지 않는다. ④ 비추상화 — 크기·필터를 인자로 받는 옵션 파서를 만들지 않는다(부를 일이 한 가지다).
  - **Acceptance**:
    - `Cargo.toml`에 `image`가 D7의 형태로 추가되고 `cargo tree --target x86_64-pc-windows-msvc -e normal`의 패키지 수가 **변하지 않는다**(실측 155 유지 — 변하면 라이선스 자산 재생성이 필요하다는 신호다).
    - `cargo run --example gen_app_icon`이 `assets/app_icon_256.png`를 만들고, 생성기가 산출물의 크기(픽셀·바이트)를 stdout에 적는다. **자산이 256×256 RGBA로 디코드되는지를 재는 단위 시험은 T2가 갖는다** — 이 레포에서 자산 시험은 그 자산을 `include_bytes!`로 담는 src 모듈 안에 살고(`src/app/licenses.rs:18`·`:200` 선례), 그 모듈이 T2에서 생기기 때문이다. 예제 타깃의 `#[cfg(test)]`는 `cargo test`가 돌리지 않는다.
    - 같은 명령을 두 번 돌려도 결과 파일이 바이트 단위로 같다(재현 가능).
    - 생성물의 **실제 크기(바이트)를 Progress Log에 적는다** — D1의 `60KB 안팎(추정)`을 확정하는 자리다.
    - **`cargo run --example gen_licenses`를 돌려 `assets/licenses.json`에 diff가 없음을 확인한다** — `AGENTS.md:20`이 "의존성을 더한 뒤 반드시 돌린다"고 무조건으로 적고 있어, 지문이 안 바뀐다는 판단(Investigation Log)을 실제로 한 번 확인해 남긴다. diff가 나오면 그 결과를 커밋하고 Progress Log에 적는다.
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0.
  - **Edge Cases**: 원본 `docs/AppIcon.png`가 없거나 PNG가 아니면 → 생성기가 사유를 stderr에 적고 종료 코드로 알린다(`unwrap` 금지). `assets/`가 없으면 만든다.
  - **Halt Forecast**: 없음(파괴적·외부 호출 없음. `assets/`에 새 파일을 더할 뿐 기존 `licenses.json`을 건드리지 않는다).
  - **Files**: 주 — `examples/gen_app_icon.rs`(신규), `assets/app_icon_256.png`(신규 생성물), `Cargo.toml`, `Cargo.lock`

- [x] **T2. 정보 대화 모듈** — Type D
  - **Design**: ① 배치 — `src/ui/about_dialog.rs`(신규), `src/ui/mod.rs`에 선언. ② 신규 심볼과 책임 — `AboutDialog`(열림 상태와 아이콘 텍스처 캐시만 든다) · `new`/`open`/`is_open`/`close`(라이선스·설정 대화와 같은 표면) · `show(&mut self, ctx)`(닫기 판정을 안에서 하고 스스로 `close()`를 부른다 — 라이선스 대화가 그렇게 정정된 선례) · private `icon_texture(ctx, physical_px)`(캐시가 맞으면 그대로, 아니면 디코드·축소해 새로 만든다). 이름·버전 줄은 `i18n::dynamic::about_version_line()`이 만든다(D9 — 대화 모듈이 아니라 카탈로그 쪽에 둔다). ③ 의존 방향 — `ui::dialog`(셸)·`ui::theme`·`i18n`·`image`를 참조하고 `ui::app`이 이것을 참조한다. `ui::app_icon`은 참조하지 않는다(ICO 경로와 독립). ④ 비추상화 — 아이콘 로더를 공용 위젯으로 승격하지 않는다(쓰는 곳이 하나다). 팝업 안에 스크롤·레이아웃 헬퍼를 두지 않는다.
  - **Acceptance**:
    - `AboutDialog`가 `open`/`is_open`/`close`를 갖고, 닫힌 상태에서 `show`를 불러도 아무것도 그리지 않는다(단위 시험).
    - `i18n`에 `about_app_name`(`"MOA" / "MOA"`)이 서고, `i18n::dynamic::about_version_line()`이 그 이름과 `env!("CARGO_PKG_VERSION")`을 이어 돌려준다(단위 시험 — 기대값은 원문 리터럴로 적고 `LanguageGuard::lock`으로 언어를 잠근다. AGENTS.md 시험 규약).
    - 자산 디코드 함수가 `assets/app_icon_256.png`를 **256×256 RGBA로 읽고**(T1이 만든 자산을 재는 자리다), 96·192 두 크기로 줄였을 때 각각 그 크기의 RGBA를 돌려준다(단위 시험).
    - 텍스처 캐시가 **같은 물리 크기를 두 번 요청하면 다시 만들지 않고**, 다른 크기를 요청하면 다시 만든다(단위 시험 — 캐시 판정 함수를 순수 함수로 두어 잰다).
    - 새 대화가 `ui::dialog`를 거치므로 `대화는_모두_이_모듈을_거친다`가 통과한다. 화면 문구가 `화면_문구가_카탈로그를_거치지_않은_곳이_없다`를 통과한다(D9대로 한글 리터럴 0).
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0.
    - 아이콘·텍스트의 시각 값이 **D12·D13대로** 코드에 상수로 서 있다(글자 16px · `theme::TEXT` · 간격 16px · 텍스처를 표시 물리 크기와 같게).
    - **화면 축(T3 이후 수동 검증에서 판정)**: 아이콘이 팝업 가운데 96px로 서고 그 아래에 `MOA 0.1.0`이 가운데 정렬되며, 아이콘 가장자리에 계단·자글거림이 보이지 않는다. UI는 이 레포에서 시험 비대상이다(AGENTS.md Conventions).
  - **Edge Cases**: 자산 디코드 실패 → 아이콘 자리를 비우고 이름·버전 줄만 그린다(타이틀바가 아이콘 없이도 자리를 잡는 선례). 물리 크기가 0 이하로 계산되는 경우(비정상 배율) → 1px 하한으로 막는다. 요청 물리 크기가 원본 256보다 크면(배율 267% 이상) → 확대가 되며 그대로 진행한다(자글거림은 축소에서 생긴다). `close()` 시 텍스처를 버리지 않는다(다시 열 때 재사용).
  - **Halt Forecast**: 없음 — 자산이 exe에 내장돼 런타임 파일·네트워크 접근이 없고, 디코드 실패는 위 Edge Cases가 흡수한다. 새 파일 하나를 더할 뿐 기존 파일을 지우거나 옮기지 않는다.
  - **Files**: 주 — `src/ui/about_dialog.rs`(신규), `src/ui/mod.rs`, `src/i18n/mod.rs`(`about_app_name` 키와 `dynamic::about_version_line`)

- [x] **T3. 메뉴 배선과 어긋난 주석 정정** — Type C
  - **Design**: ① 배치 — `src/ui/menu.rs`(`Command`), `src/ui/titlebar.rs`(메뉴 항목), `src/ui/app.rs`(필드·매치·프레임 호출). ② 신규 심볼과 책임 — `Command::OpenAbout`(정보 대화를 연다). ③ 의존 방향 — `ui::app`이 `ui::about_dialog`를 참조한다(단방향 유지). ④ 비추상화 — 다섯 메뉴 항목을 배열+반복으로 묶지 않는다(`titlebar.rs:243`의 기존 사유를 그대로 지킨다).
  - **Acceptance**:
    - 설정 메뉴의 `정보`가 활성 버튼이 되고 누르면 `Command::OpenAbout`이 나온다. `업데이트`·`릴리즈 노트`는 `pending_item` 그대로다.
    - `App`이 `about_dialog`를 들고 매 프레임 `show`를 부르며, `Command::OpenAbout`에서 `open()`을 부른다.
    - `src/ui/titlebar.rs:240`·`src/i18n/mod.rs:136`의 doc 주석이 **「나머지 둘(`업데이트`·`릴리즈 노트`)」** 로 고쳐진다(옛 표현 `나머지 셋`으로 검색해 잔존 0을 확인).
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0.
    - **화면 축(수동 검증)**: 메뉴에서 `정보`를 눌러 팝업이 뜨고, `닫기`·`Esc`·배경 클릭으로 닫힌다.
  - **Edge Cases**: 정보 팝업이 열린 채 다른 대화를 여는 경로 — 라이선스 대화 선례대로 단축키 억제 게이트(`app.rs:2740` 부근)에 넣지 않는다(설정·라이선스 대화도 들어 있지 않다).
  - **Halt Forecast**: 없음 — `Command` 변형 추가는 사전 승인 항목이고, 매치 누락은 `_` 갈래가 없어 컴파일 오류로 즉시 드러난다. 파일을 지우거나 옮기지 않는다.
  - **Files**: 주 — `src/ui/menu.rs`, `src/ui/titlebar.rs`, `src/ui/app.rs`, `src/i18n/mod.rs`

- [x] **T4. 문서·기록 정합** — Type A
  - **Acceptance**:
    - `docs/prd.md`에 **FR-58**(정보 화면)이 신설되고, FR-22 문면의 「나머지 셋은 항목 표시만」이 둘로 좁혀지며, Out of Scope 조항(`:120`)과 성공 기준 Should 목록(`:126`)이 그에 맞게 개정된다. 변경 이력에 2026-08-19 항목을 더한다.
    - `README.md`에 정보 화면이 현재 기능으로 기재된다(존재하지 않는 기능 추가 금지, 새 `##` 절 신설 금지).
    - `AGENTS.md`의 **네 자리**가 함께 갱신된다 — Repository Structure(`assets/app_icon_256.png`·`examples/gen_app_icon.rs`), Build & Test(아이콘 자산 재생성 명령), Stack의 주요 crates(`image`), **「산출물·파일 관리」의 "커밋되는 생성물"**(지금은 `assets/licenses.json` 하나만 적혀 있다 — 둘이 된다).
    - `docs/plans/deferred.md`의 「설정 팝업 **세 항목**」이 **둘**(업데이트·릴리즈 노트)로 정정되고, 직전 plan(`2026-08-18-open-source-licenses.md`)의 Deferred 3건이 `## 대기`로 이관된다.
    - 문서에 실제 IP·계정·비밀번호·토큰이 없다.
  - **Edge Cases**: PRD 번호가 이미 쓰이고 있으면(FR-58 충돌) 다음 빈 번호를 쓴다 — 착수 시 `grep "FR-5[0-9]" docs/prd.md`로 확인한다.
  - **Halt Forecast**: 없음(문서 편집만).
  - **Files**: 주 — `docs/prd.md`, `README.md`, `AGENTS.md`, `docs/plans/deferred.md`

## 사전 승인 항목 (일괄 승인 대상)

- **`Cargo.toml`에 `image` 의존 추가**(D7) — 이미 링크되는 크레이트를 명시로 올릴 뿐이라 패키지·feature 집합이 변하지 않는다. `Cargo.lock`의 `moa` 항목 의존 목록만 바뀐다.
- **`assets/app_icon_256.png` 신규 커밋**(60KB 안팎 — 추정, T1이 실측해 적는다) — 새 파일 추가이며 기존 자산을 덮어쓰지 않는다.
- **`Command` 열거형에 변형 추가**(공개 API 변경) — 크레이트 내부에서만 쓰이고 매치 누락은 컴파일러가 잡는다.
- **PRD 개정**(FR-58 신설·FR-22 문면·Out of Scope·성공 기준) — 이번 구현과 모순된 채로 둘 수 없다.

## 불가피한 Halt (위임 불가)

- commit / push / 태그 / 릴리즈 — 구현·검증이 끝난 뒤 별도로 승인받는다.
- 위 사전 승인 항목 밖에서 파일을 지우거나 이름을 바꿔야 하는 상황이 생기면 그 지점에서 멈춘다.

## Open Questions

- [x] Q1. 아이콘 원본과 축소 방법 → **PNG를 256px로 줄여 담고 표시 크기에 맞춰 고품질 축소**(D1·D2).
- [x] Q2. 팝업에 담을 내용 → **아이콘 + 이름·버전만**(D4).
- [x] Q3. 아이콘 표시 크기 → **96px**(D3).

## 리뷰 이력

- **1라운드** (BLOCKER 0 / MAJOR 2 / MINOR 4) — 전건 반영. M1 시각 값 부재 → D12 신설, M2 Halt Forecast 근거 → T2·T3 부기, m1 exe 크기 라벨, m2 텍스처 필터·정렬 → D13 신설, m3 AGENTS.md 네 번째 자리, m4 실측 방법 명시.
- **2라운드** (BLOCKER 0 / MAJOR 2 / MINOR 1) — 1라운드 지적 6건 전건 해소 확인. **동일 지적 잔존 0이고 신규 지적만 남아, 재호출 상한을 수렴이 아니라 예산 소진으로 끝냈다.** 신규 3건은 메인이 근거를 직접 대조해 아래와 같이 처리했다:
  - **M1 (수용)** — D9의 "두 언어에서 같으면 카탈로그 제외" 논리가 레포 선례와 어긋난다는 지적. `src/i18n/mod.rs:121-124`의 `settings_language_english => "English" / "English"`(주석까지 달려 있다)와 `prd.md:99`(NFR-6, 예외 없음), `AGENTS.md:78`의 예외 3종을 직접 읽어 **지적이 옳음을 확인**했다. D9를 카탈로그 등재로 뒤집고 D8·T2 Design·T2 acceptance·T2 Files를 그에 맞췄다.
  - **M2 (수용)** — T1 acceptance의 자산 디코드 시험이 살 파일이 T1 Files에 없다는 지적. `src/app/licenses.rs:18`·`:200`이 그 선례이며 예제 타깃의 시험은 `cargo test`가 돌리지 않는 것도 맞다. 그 시험을 **T2로 이관**하고 T1은 생성·재현성·크기 기록만 갖게 했다.
  - **m1 (수용)** — `AGENTS.md:20`의 "의존성을 더한 뒤 반드시 돌린다"가 무조건 문면이라는 지적. T1 acceptance에 `gen_licenses` 실행 후 diff 0 확인을 넣었다(비용이 거의 없고 판단을 실물로 남긴다).

## Phase Ledger

- Phase G 통과 (Must 100% — 이번 회차가 커버한 것은 Should FR-58이고, F-7의 PRD 전수 대조에서 기존 Must FR 무회귀를 확인했다. 갭 0이라 재루프 없음. 화면 축 5항목은 ⏳ HUMAN-VERIFY로 아래에 등재)
- Phase F 통과 (HEAD 8e56de1 — F-7: BLOCKER 0 / MAJOR 0 / MINOR 2. m2(자기 유발 — `Cargo.toml` 주석의 「exe 변화 없음」이 실측과 어긋남)는 규칙 4-1대로 그 자리에서 정정했고, m1(화면 축 기록 부재)은 아래 HUMAN-VERIFY 목록으로 남겼다)

### ⏳ HUMAN-VERIFY (사용자 확인 필요 — 빌드·시험으로 판정할 수 없는 축)

1. 설정 메뉴(⚙)의 `정보`를 눌러 팝업이 뜨는가.
2. 팝업 가운데에 아이콘이 서고 그 아래에 `MOA 0.1.0`이 가운데 정렬되는가.
3. **아이콘 가장자리에 계단·자글거림이 없는가** — 이번 요구의 핵심 축이다.
4. `닫기` 버튼·`Esc`·배경 클릭으로 닫히는가.
5. 같은 메뉴의 `업데이트`·`릴리즈 노트`는 여전히 비활성인가.

## Retry Ledger

## Progress Log

- T3-T4 완료: 설정 메뉴의 `정보`가 활성 버튼이 되어 `Command::OpenAbout`으로 대화를 연다(라이선스 대화와 같은 배선). 어긋나게 되는 doc 주석 둘을 「나머지 둘」로 고쳤고(옛 표현 잔존 0), PRD에 FR-58을 신설하며 FR-22·Out of Scope·성공 기준·변경 이력을 함께 개정했다. README·AGENTS.md 네 자리·Deferred 대장(설정 팝업 항목 정정 + 직전 회차 3건 이관)도 맞췄다.
- **exe 크기 실측(Phase F)**: master 기준 8,634,368B → 이번 브랜치 8,770,048B, **+135,680B**. 자산이 75,943B이고 나머지 약 60KB는 `image`의 디코드·리샘플 경로가 새로 링크된 몫이다 — **D1이 적은 「60KB 안팎(추정)」은 자산 크기만 본 값이었고 실제 exe 증가는 그 두 배가 조금 넘는다**(패키지 집합은 그대로여도 우리가 부르는 함수가 늘면 링크되는 코드가 는다).

- T2 완료: `src/ui/about_dialog.rs`가 96px 아이콘과 `MOA 0.1.0` 한 줄을 `dialog::show` 셸 안에 세운다. 아이콘은 표시 물리 크기(`96 × 화면 배율`)로 CPU에서 Lanczos3 축소해 텍스처로 올리고, 만든 크기를 함께 들어 배율이 바뀔 때만 다시 만든다. 신규 시험 13건.
  - 결정: `i18n` 소스 훑기 시험이 위젯 열쇠 `Id::new("정보 대화")`를 잡아 `EXEMPT_LITERALS`(위젯 ID 목록)에 등재했다 — `"라이선스 대화"`와 같은 자리이며 AGENTS.md의 예외 조항("위젯 상태를 잇는 열쇠")에 해당한다.
  - 리뷰 반영: 시험이 `show()` 실제 렌더 경로를 한 번도 돌지 않는다는 지적(MAJOR)에 따라 라이선스 대화의 `ctx.run_ui` 기법을 옮겨 왔다. 열린 상태 시험은 **두 프레임**을 돈다 — egui의 떠 있는 영역은 첫 프레임에 크기만 재고 그리지 않는다(실측).
  - 리뷰 반영: `snap_to_pixels`의 가드가 NaN을 통과시키던 것(MINOR)을 `is_finite` 조합으로 고쳤다. `!(x > 0.0)` 형태는 clippy가 거부한다.

- T1 완료: `examples/gen_app_icon.rs`가 `docs/AppIcon.png`(1083×1105)를 256×256 Lanczos3로 줄여 `assets/app_icon_256.png`를 만든다. **생성물 실측 75,943B**(D1의 `60KB 안팎(추정)`을 이 값으로 확정 — 추정보다 약 16KB 크다). 두 번 실행해 md5가 같아 재현성을 확인했다.
  - 확인: `cargo tree --target x86_64-pc-windows-msvc -e normal` 중복 제거 **155개로 불변**, `cargo run --example gen_licenses` 후 `assets/licenses.json` **md5 불변(diff 0)** — D7의 "패키지·feature 집합이 변하지 않아 라이선스 자산 재생성 불요"가 실측으로 확인됐다(`git status`가 M으로 표시한 것은 CRLF 정규화 때문이며 내용 변화는 0이다).
