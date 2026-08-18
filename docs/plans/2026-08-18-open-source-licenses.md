# Plan: 오픈소스 라이선스 화면 (FR-57)

**PRD**: docs/prd.md

## 요구 이해

- **원문 요청**: "현재 사용중인 오픈소스 라이선스가 뭐가 있지?" → (조사 결과 제시 후) "진행 해줘" — 제안한 것은 *"`cargo-about` 등으로 라이선스 전문을 수집해 빌드 자산으로 만들고, 그것을 앱에 담아 화면에 보인다"*였다.
- **이해한 요구**: 타이틀바 설정 메뉴에서 표시만 되고 눌리지 않던 `오픈소스 라이선스`를 실제로 여는 화면으로 만든다. 앱이 정적 링크하는 오픈소스 155개(실측)의 라이선스 고지를 오프라인에서 볼 수 있게 하며, 고지 데이터는 레포에 커밋된 자산에서 온다(빌드가 네트워크·외부 도구를 타지 않는다). 의존성이 바뀌었는데 자산을 다시 만들지 않으면 시험이 잡는다.
- **포함하지 않는 것으로 이해**: 같은 메뉴의 나머지 세 항목(`업데이트`·`릴리즈 노트`·`정보`)은 이번에 건드리지 않는다 — 비활성 그대로 남는다(사용자 결정).

## Goal

설정 메뉴의 `오픈소스 라이선스`가 열리고, 앱이 쓰는 오픈소스 구성 요소의 이름·버전·SPDX 식별자와 라이선스 전문을 좌우 2단 대화에서 읽을 수 있다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-57 (신설) | Should | T1~T5 (신설은 T5) | ✅ 커버 |
| FR-22 (문면 개정) | Should | T5 | ✅ 커버 — 「각 기능은 미구현」 서술이 실제와 어긋난다 |
| Out of Scope 조항(`prd.md:119`) 재한정 | — | T5 | ✅ 커버 — 「오픈소스 라이선스는 표시만 유지」가 이번 구현과 모순된다 |
| 성공 기준 Should 목록(`prd.md:125`) | — | T5 | ✅ 커버 — FR-57이 빠지면 80% 판정 모수가 실제와 어긋난다 |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- 설정 메뉴의 `업데이트`·`릴리즈 노트`·`정보` 활성화 — 사용자가 이번 범위를 라이선스 하나로 좁혔다. 셋은 `docs/plans/deferred.md` 대기 항목으로 남는다.
- 라이선스 고지를 파일로 내보내기(저장·인쇄) — 요청에 없다.
- 라이선스 위반·호환성 자동 판정(정책 검사) — 고지 표시가 요구이며 판정은 사람 몫이다.

## Deferred / Follow-up

- 목록 검색·필터 — 155개라 이름으로 좁히면 편하지만, 이번 요구는 「고지를 볼 수 있게」다. 좌측 목록이 정렬돼 있고 스크롤로 닿으므로 없이도 성립한다.
- 라이선스 종류별 묶어 보기(같은 전문을 쓰는 크레이트를 한 항목으로) — 자산이 이미 전문을 중복 제거해 담으므로 화면만 바꾸면 되는 확장 지점이다.
- [SUGGEST] `examples/gen_licenses.rs`의 `main`에서 원문 있는 갈래와 없는 갈래가 `CrateEntry` 필드 넷(`name`·`version`·`spdx`·`authors`)을 똑같이 채운다 — 공통 필드를 먼저 만들고 `text_indices`·`standard_text` 둘만 분기하면 중복이 준다 (출처: T2 quality S1)

## Investigation Log

- 위키 참조: `20_projects/personal/moa/feat-titlebar-tray` — 설정 메뉴의 정본. **다섯 항목 중 `설정`만 동작하고 넷은 비활성**이며, 그 이유가 "활성처럼 보이면서 눌러도 반응이 없으면 고장으로 오인된다"임을 확인. 이번에 하나가 활성으로 바뀌므로 그 페이지 서술도 갱신 대상이 된다(위키 갱신은 별도 세션).
- 위키 참조: `20_projects/personal/moa/feat-dialog-shell` — 모달 여덟이 `ui::dialog`를 거치고 규약을 소스 훑기 시험이 지킨다. **`show`(본문 폭)와 `show_fixed`(프레임 크기)의 구분**과 사이트 관리자가 `show_fixed`로 1080×680을 쓰는 것을 확인 — 이번 대화가 아홉 번째다.
- 위키 참조: `20_projects/personal/moa/decisions` — 라이선스 화면·고지에 관한 과거 보류·기각 결정 없음. 상충 항목 0건.
- Deferred 대장(`docs/plans/deferred.md`, 대기 68건(실측)): 관련 항목은 `[2026-07-29] 설정 팝업 **네 항목**의 실제 기능 — 업데이트·릴리즈 노트·오픈소스 라이선스·정보`. 이번에 **넷 중 하나가 해소**되므로 그 항목은 삭제가 아니라 셋으로 좁히는 **정정** 대상이다(T5). 잔량 68건은 소진 batch 임계(100건)에 미달하고 최근 재확인이 2026-08-18이라 이번 회차에 batch를 열지 않는다.
- 대상 집합은 **155개(실측)** — `cargo tree --target x86_64-pc-windows-msvc -e normal --prefix none`의 중복 제거 결과(자기 자신 `moa` 제외). `cargo metadata --filter-platform`의 normal 전이 폐포는 **164개**로 9개 더 많고, 그 차이는 feature 미활성 optional(`tiff`·`zune-jpeg`·`fax`·`weezl`·`half`·`zune-core`·`glifo`·`quick-error`·`zerocopy-derive`)이다 — 실제로 링크되지 않는 것을 고지에 넣지 않기 위해 **대상 집합은 `cargo tree`가 정한다**.
- 라이선스 원문 파일 보유 현황(실측): 155개 중 **143개**에 `LICENSE*`·`COPYING*` 계열 파일이 있고 **12개**는 없다 — `accesskit`·`clipboard-win`·`ecolor`·`eframe`·`egui`·`egui_glow`·`egui-winit`·`emath`·`epaint`·`lazy-regex-proc_macros`·`profiling`·`suppaftp`. 12개 모두 `license` 필드는 있다(metadata 전수 확인: `license` 필드 없는 패키지 0건).
- 원문 총량(실측): 파일 225개 1,139,310B, **내용 중복을 걷어내면 95개 346,372B**. 현재 release exe는 8,222,208B이므로 전문을 그대로 담아도 4% 남짓 는다. **이 95는 각 크레이트 디렉터리 최상위만 훑은 값이라 번들 C 소스의 하위 경로 원문 2건은 빠져 있다** — 최종 개수는 그만큼 는다(T2 acceptance의 범위 근거).
- 번들 C 소스(실측): `libssh2-sys-0.3.2/libssh2/COPYING`(libssh2, BSD-3-Clause 계열)과 `libz-sys-1.1.29/src/zlib/LICENSE`(zlib)가 패키지 안에 실재하며, `cargo tree -i libz-sys`로 `libz-sys → libssh2-sys → ssh2 → moa` 경로를 확인했다. 이 둘은 크레이트의 SPDX 필드(`MIT OR Apache-2.0`)와 **다른 라이선스**라 필드만 긁으면 누락된다.
- 번들 글꼴(실측): `egui-phosphor-0.13.0/res/Phosphor*.ttf`가 exe에 정적으로 담기고, 그 README가 *"Phosphor Icons are licensed under MIT"*로 크레이트 라이선스와 별도임을 밝힌다. `res/` 아래에 라이선스 파일은 없다.
- `cargo tree --format "{p}|{l}"`가 이름·버전·SPDX를 한 줄로 준다(실측 156줄 = 155 + `moa`). 중복 행에는 ` (*)` 접미가 붙는다. 툴체인은 cargo 1.95.0 / rustc 1.95.0.
- `cargo metadata`의 각 패키지에 `manifest_path`가 있고 그 부모가 곧 크레이트 소스 디렉터리다(예: `%USERPROFILE%\.cargo\registry\src\index.crates.io-<hash>\accesskit-0.24.1\Cargo.toml`) — 레지스트리 경로를 손으로 조립하지 않아도 된다.
- `src/lib.rs`가 `app`·`i18n`·`ui` 등을 재수출하는 bin+lib 구성이라, `examples/`의 생성기가 `moa::app::licenses`의 지문 함수·자산 타입을 그대로 쓸 수 있다.
- 기존 대화 패턴: `ui::settings_dialog`가 `open`/`is_open`/`close` + `show(ctx, …)` 구조이고 `ui::app`이 `Command::OpenAppSettings`에서 `open()`을 부른다(`src/ui/app.rs:1926`·`:2061`·`:2864`). 이번 대화도 같은 모양이면 배선이 한 줄씩이다.
- 단축키 억제 게이트(`src/ui/app.rs:2740-2743`)는 `pending_remove || hostkey.is_open() || site_manager.is_open()`만 본다 — **`settings_dialog`도 그 목록에 없다.** T4는 그 선례대로 **라이선스 대화를 넣지 않는다**(T4 Edge Cases에서 확정).
- 표준 전문 3종의 출처(실측): MIT는 `bitflags-2.10.0/LICENSE-MIT`(1,071B — `Copyright (c) 2014 The Rust Project Developers`로 시작하는 표준형), Apache-2.0은 `serde-1.0.229/LICENSE-APACHE`(9,723B — SPDX 원문 그대로), BSL-1.0은 `error-code-3.3.2/LICENSE`(1,338B — 그대로). `serde`의 `LICENSE-MIT`는 저작권 줄이 없는 변형이라 MIT 원본으로 쓰지 않는다.
  - **MIT만 가공한다(4-D의 「저작권 줄을 SPDX 플레이스홀더로 대체」)** — 첫 줄을 `Copyright (c) <year> <copyright holders>`로 바꿔 담는다(1,071B → 1,065B). 그러지 않으면 **bitflags의 저작권자가 무관한 12개 크레이트의 고지에 오귀속**된다. Apache-2.0·BSL-1.0은 본문에 저작권자 자리가 없어 원본 그대로 쓴다(T2 실측: 두 파일은 레지스트리 원본과 바이트 단위로 같다).
- **표준 전문 3종이 원문 없는 12건을 전부 덮는다(실측)** — 그 12건의 SPDX는 `MIT OR Apache-2.0` 10건(`accesskit`·`ecolor`·`eframe`·`egui`·`egui_glow`·`egui-winit`·`emath`·`epaint`·`profiling`·`suppaftp`) · `BSL-1.0` 1건(`clipboard-win`) · `MIT` 1건(`lazy-regex-proc_macros`)뿐이고 넷째 식별자는 없다. 그래도 의존성이 바뀌면 새 식별자가 들어올 수 있어 T2가 그 경우를 오류로 막는다(T2 Edge Cases).
- 이번 변경이 stale로 만드는 doc 주석은 **둘**이다(실측) — `src/ui/titlebar.rs:244`의 `show_settings_menu`(「`설정`만 동작하고 나머지 넷은 아직 표시만 한다」)와 `src/i18n/mod.rs:136`의 `/// 설정 메뉴의 나머지 넷 — 아직 비활성이다`(덮는 키가 `:137~:140`이고 그중 `titlebar_licenses`가 `:139`). 둘 다 T4가 함께 고친다.
- `assets/`·`examples/` 디렉터리는 현재 레포에 없다 — 둘 다 이번에 생긴다. `.gitignore`는 `/target`만 걸어 두 디렉터리가 커밋된다.
- PRD의 관련 문면 3곳(실측): `docs/prd.md:37`(FR-22), `:119`(Out of Scope — 「오픈소스 라이선스…표시만 유지」), `:125`(성공 기준 Should 목록). 현재 FR 최대 번호는 FR-56.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | `cargo tree`가 실제 링크 대상만 주고 metadata는 과대 포함한다 | 위 Log 5번째 항목 — 두 집합의 차분 9건이 전부 feature 미활성 optional | ✅ |
| 2 | `cargo tree --format "{p}\|{l}"`로 이름·버전·SPDX를 함께 얻는다 | 실행 결과 156줄, `(*)` 접미 처리 필요 | ✅ |
| 3 | metadata의 `manifest_path` 부모에 라이선스 파일이 있다 | 143/155에서 실측, 나머지 12는 파일 자체가 없음(SPDX만) | ✅ |
| 4 | 전문을 exe에 담아도 크기가 감당된다 | 중복 제거 346,372B 대 현재 exe 8,222,208B | ✅ |
| 5 | 번들 C 소스 둘이 실제로 이 exe에 링크된다 | `cargo tree -i libz-sys` 경로 확인 + `libssh2-sys`에 `libssh2/` 소스 실재. **libz-sys가 시스템 zlib을 찾아 쓰는 경우까지 가려내지는 않았다** — 고지에 넣는 쪽이 보수적으로 안전하므로 판정 없이 등재한다 | ✅ (등재 판정) |
| 6 | 새 모달이 `ui::dialog`를 거치지 않으면 시험이 잡는다 | `src/ui/dialog.rs`의 `대화는_모두_이_모듈을_거친다`(`src/ui` 바로 아래 훑기, 대화 개수 하드코딩 없음) | ✅ |
| 7 | 화면 문구를 카탈로그 밖에 두면 시험이 잡는다 | `src/i18n/mod.rs:1025 화면_문구가_카탈로그를_거치지_않은_곳이_없다`, ROOTS에 `src/ui` 포함 | ✅ |
| 8 | 자산 JSON을 `serde_json`으로 읽을 수 있다 | `Cargo.toml`에 `serde`(derive)·`serde_json` 이미 있음 | ✅ |
| 9 | `examples/`는 `cargo build`의 산출물에 들어가지 않는다 | **T2에서 실측 확정** — `cargo build --release` 후 `target/release/*.exe`에 `gen_licenses.exe`가 없고 `target/release/examples/`도 비어 있다(2026-08-18) | ✅ |
| 10 | `Command`를 `match`하는 곳이 `src/ui/app.rs` 한 곳뿐이고 wildcard arm이 없다 | `src/ui/app.rs:2031`(중첩 `:2076`)이 유일하며 `_ =>` 없음. 나머지 참조(`ui/panel.rs:1273`·`ui/titlebar.rs:254`·`ui/menu.rs`·`ui/splitter.rs`·`ui/tabs.rs`)는 값 생성·비교뿐. `src/app/window.rs:783`의 `match command`는 Win32 `IDM_*` u32라 무관 | ✅ |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 생성기가 레지스트리 캐시(`~/.cargo/registry/src`)에 의존한다 | clean checkout에서 캐시가 없으면 원문을 못 읽는다 | 생성기는 **개발 시에만** 돌고 결과는 커밋된다 — 빌드·시험은 캐시를 보지 않는다. 캐시가 없으면 `cargo fetch` 후 실행하라는 안내를 생성기가 오류 문구로 낸다 |
| `cargo tree`의 출력 형식에 기댄다 | cargo가 형식을 바꾸면 생성기가 오작동한다 | `--format "{p}\|{l}"`는 공개 옵션이고 파싱 규칙이 단순하다(마지막 ` (*)` 제거 → `name vX.Y.Z\|spdx`). 형식이 깨지면 파싱 실패로 **오류를 내고 멈춘다** — 조용히 빈 자산을 쓰지 않는다 |
| 자산이 커져(≈400KB JSON) 파싱이 프레임을 막는다 | 대화를 처음 열 때 끊긴다 | 파싱은 **대화를 처음 열 때 1회**만 하고 결과를 들고 있는다(`OnceLock`). 346KB JSON 파싱은 밀리초 단위이며, 시작 시점에는 하지 않아 NFR-1(시작 1초)에 닿지 않는다 |
| 12건의 표준 전문이 그 크레이트의 실제 고지와 다를 수 있다 | 고지가 부정확해 보인다 | 표준 전문임을 화면에 **명시**하고(사유 표기) 저작권자는 metadata의 `authors`를 함께 보인다. 원문을 지어내지 않는다 |
| 지문 시험이 사소한 버전 갱신마다 붉어진다 | 의존성을 올릴 때마다 생성기를 돌려야 한다 | 그것이 의도다 — 버전이 바뀌면 라이선스 원문도 바뀔 수 있다. 재생성은 명령 하나(`cargo run --example gen_licenses`)다 |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `ui::menu::Command` (열거형) | `src/ui/menu.rs:54`(정의), `src/ui/app.rs:2031`(유일한 `match`, 중첩 `:2076`), `src/ui/titlebar.rs:254` 부근(발생지) | variant `OpenLicenses` 추가 — wildcard arm이 없어 처리 누락 시 빌드가 실패한다(전제 검증 #10) |
| `ui::titlebar::pending_item` | `src/ui/titlebar.rs:267`(정의), 같은 파일 호출 4곳(`:257`·`:258`·`:260`·`:261`) | 호출이 4 → 3으로 준다. 정의는 남는다(나머지 셋이 계속 쓴다) |
| `ui::titlebar::show_settings_menu` | `src/ui/titlebar.rs:244` | `오픈소스 라이선스` 줄이 `pending_item`에서 `ui.button` + `Command` 방출로 바뀐다. 함수 doc 주석(「`설정`만 동작하고 나머지 넷은 아직 표시만 한다」)이 어긋나므로 함께 고친다 |
| `ExplorerApp` (구조체) | `src/ui/app.rs:493`(필드 선언), `:664`(초기화), `:2864` 부근(프레임마다 show) | 필드 `license_dialog: LicenseDialog` 추가 — 이 구조체는 `ui::app` 안에서만 만들어진다 |
| `i18n::titlebar_licenses` | `src/i18n/mod.rs:139`, `src/ui/titlebar.rs:260` | 재사용(문구 동일) — 대화 제목으로도 쓴다 |

### 4-B. 계약·직렬화 변경

- **세션 스키마 변경 없음** — 대화 열림 상태·선택 항목은 저장하지 않는다(설정 대화와 같다). `app/settings.rs`는 건드리지 않는다.
- **신규 직렬화 형식** `assets/licenses.json` — 앱이 **읽기만** 하는 생성물이며 사용자 데이터가 아니다. 형식이 바뀌면 생성기와 모델을 함께 고친다(둘 다 이번에 만든다). 버전 필드(`schema`)를 두어 이후 변경을 구분할 수 있게 한다.

### 4-C. 테스트 파일

- 신규: `src/app/licenses.rs`의 `#[cfg(test)] mod tests` — 지문 함수 단위 시험(T1) + 자산 구조·지문 대조 시험(T2)
- 신규: `src/ui/license_dialog.rs`의 `#[cfg(test)] mod tests` — 선택 상태 전이 등 순수 로직
- 영향: `src/ui/dialog.rs`의 `대화는_모두_이_모듈을_거친다`, `src/i18n/mod.rs`의 `화면_문구가_카탈로그를_거치지_않은_곳이_없다`, `src/ui/widgets.rs`의 `화면_코드에_원본_아이콘_기호가_남아_있지_않다` — 셋 다 새 파일이 생기면 자동으로 대상에 든다

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `app::licenses::LicenseData`(자산 모델) | `grep -rn "licens" src/ -i` → i18n 키 1건·`ui/titlebar.rs` 1건뿐. 라이선스 데이터 구조 없음 | 신규 — 같은 일을 하는 것이 없다 |
| `app::licenses::lockfile_fingerprint` | `grep -rn "fnv\|Hasher\|hash(" src/` → `remote/sftp.rs:123`의 SSH 호스트 키 해시뿐(용도가 다르고 libssh2 API다) | 신규 — std에 해시 함수가 없고 의존성을 더하지 않기 위해 FNV-1a 64비트를 직접 쓴다(20줄) |
| `ui::license_dialog::LicenseDialog` | `ui::settings_dialog::SettingsDialog`·`ui::site_manager` | **셸은 재사용**(`dialog::show_fixed`), 좌우 2단 배치는 `site_manager::show_body`의 **계산 방식**(좌 열 rect → `BODY_GAP` → 나머지가 우 열)을 그대로 다시 쓴다(그 함수는 사이트 폼 전용 private이라 호출할 수 없다). `BODY_GAP`·`BODY_PAD_X`·`HEADER_HEIGHT`는 같은 값이고, **좌 열 폭만 400 → 320으로 좁혔다**(T3 구현 시 정정 — 담기는 것이 이름 한 줄뿐이라 400은 과하고, 남는 자리는 9KB에 이르는 전문을 넓게 읽는 데 쓰는 편이 낫다). 목록 행·전문 렌더는 신규 |
| 목록 행 렌더 | `ui::widgets`의 공개 위젯 15종에 "선택 가능한 목록 행"이 없다(`radio_row`·`check_row`·`toggle_row`는 각각 라디오·체크·토글 전용) | 신규 — 다만 hover 배경은 `widgets::hover_backdrop`를, 색은 `theme::ROW_HOT`·`theme::TEXT_*`를 재사용한다 |
| SPDX 표준 전문 3종(`assets/spdx/*.txt`) | 레지스트리에서 확보 — MIT는 `bitflags-2.10.0/LICENSE-MIT`(저작권 줄을 SPDX 플레이스홀더로 대체), Apache-2.0은 `serde-1.0.229/LICENSE-APACHE`(그대로), BSL-1.0은 `error-code-3.3.2/LICENSE`(그대로) | 재사용(레지스트리에서 가져와 커밋) — 네트워크를 타지 않는다 |

### Verified by

- `grep -rn "오픈소스\|라이선스\|licens" src/ --include=*.rs -i` → 4 hits, 전부 위 표에 포함(i18n 키 1·titlebar 1·무관한 `remote/transfer.rs` 시험 문자열 2)
- `Command` 참조 전수 → `match`는 `src/ui/app.rs` 1곳(전제 검증 #10), 나머지는 값 생성·비교
- `grep -n "SettingsDialog\|settings_dialog\|OpenAppSettings" src/ui/app.rs` → 6 hits, 배선 지점 3곳 확정(선언·초기화·프레임 호출)

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | PRD FR-22 문면(`prd.md:37`) | 「v1은 항목 표시만 하고 각 기능은 미구현」이 라이선스 활성화와 정면으로 어긋난다 | T5에 편입 |
| 필수 | PRD Out of Scope 조항(`prd.md:119`) | 「`오픈소스 라이선스`…는 FR-22대로 표시만 유지한다」가 남으면 요구 정본이 자기 모순(FR-57=구현 / 제외=표시만)이 된다 | T5에 편입 |
| 필수 | PRD 성공 기준 Should 목록(`prd.md:125`) | FR-57이 빠지면 「Should 80% 이상」의 모수가 실제 요구 집합과 어긋난다 | T5에 편입 |
| 필수 | `README.md` 핵심 기능 | 새 화면이 기능 목록에 없으면 문서가 실제와 어긋난다 | T5에 편입 |
| 필수 | `AGENTS.md` — Repository Structure·Build & Test·산출물·Conventions | `assets/`·`examples/`가 생기고 생성 명령이 새로 필요하다. 예제 타깃의 출력·오류 규약(아래 m2)도 여기 적는다 | T5에 편입 |
| 필수 | `docs/plans/deferred.md`의 `[2026-07-29] 설정 팝업 네 항목` | 넷 중 하나가 해소돼 「네 항목」이 사실과 달라진다 | T5에 편입(항목을 셋으로 정정, 해소분은 종결 처리) |
| 필수 | `src/ui/titlebar.rs:244`의 `show_settings_menu` doc 주석 | 「`설정`만 동작하고 나머지 넷은 아직 표시만 한다」가 틀린 서술이 된다 | T4에 편입(자기 유발 — 규칙 4-1) |
| 필수 | `src/i18n/mod.rs:136`의 `/// 설정 메뉴의 나머지 넷 — 아직 비활성이다` | 넷 중 하나가 활성이 되어 이 주석도 틀린 서술이 된다(위 주석과 같은 성질인데 1라운드 판정에서 빠졌다) | T4에 편입(자기 유발 — 규칙 4-1·7-1) |
| 무관 | 세션 스키마(`app/settings.rs`) | 대화가 아무 상태도 저장하지 않는다(4-B) | 건드리지 않음 |
| 무관 | 위키 `feat-titlebar-tray`·`feat-dialog-shell` | 갱신 대상이지만 위키는 별도 세션 소관이며 이 레포 밖이다 | 건드리지 않음(Next Steps에 남긴다) |

## Decisions

### D1. 라이선스 데이터 수집 도구
- **Options**: A) 레포 안 자체 생성기 / B) `cargo-about` 도입 / C) build.rs가 매 빌드 생성
- **Chosen**: A
- **Rationale**: AGENTS의 최소 의존 원칙에 맞고, SPDX 필드만 있는 12건·번들 C 소스 2건·번들 글꼴 1건 같은 **이 레포 고유 사정**을 코드로 다룰 수 있다. C는 clean checkout·오프라인에서 빌드가 깨진다.
- **Source**: 사용자 결정(2026-08-18), `AGENTS.md` DO NOT·Conventions

### D2. 생성기 위치
- **Options**: A) `examples/gen_licenses.rs` / B) `src/bin/` / C) `#[ignore]` 시험
- **Chosen**: A — `cargo run --example gen_licenses`
- **Rationale**: `cargo build --release`가 예제를 빌드하지 않아 배포 산출물이 늘지 않고, `cargo test`·`clippy --all-targets`가 컴파일 검사를 해 썩지 않는다. Cargo.toml 구조도 그대로다.
- **Source**: 사용자 결정(2026-08-18)

### D3. 대상 집합의 정본
- **Options**: A) `cargo tree -e normal` / B) `cargo metadata`의 normal 폐포
- **Chosen**: A(대상 집합) + B(패키지 경로·저작자)
- **Rationale**: B만 쓰면 실제로 링크되지 않는 9건이 화면에 뜬다(전제 검증 #1). A는 경로를 주지 않으므로 둘을 함께 쓴다.
- **Source**: 실측 차분(Investigation Log)

### D4. 이중 라이선스 표기
- **Options**: A) 선언 그대로 + 전문 둘 다 / B) 선택을 명시하고 그 전문만
- **Chosen**: A
- **Rationale**: 사실을 그대로 옮겨 틀릴 여지가 없고 선택권을 나중에 행사할 수 있다. 대가인 자산 증가는 중복 제거로 흡수된다(같은 Apache-2.0 전문을 101번 담지 않는다).
- **Source**: 사용자 결정(2026-08-18)

### D5. 원문이 없는 12건의 처리
- **Options**: A) 표준 전문 + 사유 표기 / B) 식별자만 / C) GitHub에서 내려받기
- **Chosen**: A
- **Rationale**: MIT는 전문 동봉이 의무라 B는 고지가 불완전하다. C는 생성기가 네트워크를 타 오프라인 재생성이 불가능해진다.
- **Source**: 사용자 결정(2026-08-18)

### D6. stale 검출 방식
- **Options**: A) `Cargo.lock` 지문 대조 / B) 크레이트 목록 포함 관계 / C) 없음
- **Chosen**: A — `Cargo.lock`에서 `name`·`version` 줄만 뽑아 이어 붙인 문자열의 FNV-1a 64비트
- **Rationale**: 버전 하나만 올라도 붉어져야 한다(라이선스 원문이 바뀔 수 있다). 공백·주석 변화에는 반응하지 않도록 이름·버전 줄만 본다. std에 해시가 없어 FNV-1a를 직접 쓴다(의존성 추가 대신).
- **Source**: 사용자 결정(2026-08-18) + `grep`으로 기존 해시 구현 부재 확인(4-D)

### D7. 화면 구조
- **Options**: A) 좌우 2단 / B) 한 면 전환 / C) 한 장 스크롤
- **Chosen**: A — `dialog::show_fixed`, 프레임 1080×680(사이트 관리자와 같은 값)
- **Rationale**: 155개를 훑으면서 전문을 넓게 읽을 수 있다. 같은 창에서 뜨는 큰 대화가 이미 1080×680이라 판이 흔들리지 않는다(설정 대화 주석이 같은 이유로 헤더 높이를 맞췄다).
- **Source**: 사용자 결정(2026-08-18), `src/ui/site_manager.rs:23`·`:24`·`:41`·`:43`

### D8. 자산 형식
- **Options**: A) JSON(`serde_json`) / B) 자체 길이-접두 텍스트 형식
- **Chosen**: A — `assets/licenses.json`
- **Rationale**: `serde`·`serde_json`이 이미 의존성이라 파서를 새로 만들 이유가 없다(4-D 재사용). B는 40줄짜리 파서와 그 시험이 새로 생긴다.
- **Source**: `Cargo.toml`

### D9. 전문 중복 제거
- **Chosen**: 같은 내용의 전문은 자산에 한 번만 담고 크레이트는 인덱스로 가리킨다
- **Rationale**: 실측 1,139,310B → 346,372B(3.3배 감소). 화면 동작은 같다.
- **Source**: 실측(Investigation Log)

### D10. 지문 시험을 두 task로 가른다
- **Options**: A) 모델 task에서 실제 `Cargo.lock`을 대조 / B) 모델 task는 fixture만, 실제 대조는 자산이 생기는 task에서
- **Chosen**: B
- **Rationale**: 모델 task 시점의 자산은 손으로 둔 **빈 스켈레톤**이라 그 `lock_fingerprint`가 실제 lock과 맞을 수 없다 — A를 택하면 acceptance가 자기 시점에 만족 불가능해지고, 실패 안내가 가리키는 생성기도 아직 없다. 계획에 없는 부트스트랩을 구현자가 지어내게 두지 않는다.
- **Source**: `plan-reviewer` 1라운드 M2

### D11. 예제 타깃의 출력·오류 규약
- **Chosen**: 예제(`examples/`)는 **stdout/stderr 출력을 허용**하고 `fn main() -> Result<(), String>`으로 오류를 종료 코드에 싣는다. `unwrap`·`expect`는 쓰지 않는다.
- **Rationale**: AGENTS의 `println!` 금지는 **GUI 프로덕션 코드**를 겨냥한 것이고(콘솔 창이 없다), 개발용 CLI 예제에는 출력 수단이 필요하다. 규약을 그대로 읽으면 오류를 알릴 방법이 사라져 halt 요인이 된다. 이 예외를 `AGENTS.md`에 한 줄로 적어 다음 회차가 다시 판단하지 않게 한다(T5).
- **Source**: `plan-reviewer` 1라운드 m2, `AGENTS.md` DO NOT

## Tasks

<!-- 순서 주의: 생성기(T2)가 모델(T1)의 타입·지문 함수를 쓰므로 모델이 먼저다. -->

- [x] T1. 자산 모델과 지문 함수를 만든다
  - **Type**: C
  - **Design**: ① 배치 — `src/app/licenses.rs`(순수 로직 계층, `ui`를 모른다), `src/app/mod.rs`에 모듈 선언, `assets/licenses.json`에 **빈 스켈레톤**(`schema`·`lock_fingerprint: 0`·빈 배열 둘)을 손으로 둔다(`include_str!`이 컴파일 시점에 파일을 요구한다). ② 신규 심볼과 책임 — `LicenseData`(`schema`·`lock_fingerprint`·`crates`·`texts`), `CrateEntry`(`name`·`version`·`spdx`·`authors`·`text_indices`·`standard_text`·`bundled`), `LicenseText`(`spdx`·`body`), `parse(json: &str) -> LicenseData`(문자열을 받는 파싱 진입점 — 실패하면 패닉하지 않고 빈 데이터를 준다. 시험이 깨진 fixture를 여기 넣는다), `load()`(`include_str!`한 자산을 `parse`에 넘기고 결과를 `OnceLock`에 담는다), `CrateEntry::texts<'a>(&self, data: &'a LicenseData) -> Vec<&'a LicenseText>`(`text_indices`가 범위를 벗어난 자리는 건너뛴다 — Edge Case의 담당 심볼), `lockfile_fingerprint(lock: &str) -> u64`(`name = "…"`·`version = "…"` 줄만 뽑아 이어 붙인 뒤 FNV-1a 64). 세 자료형에 `Serialize`·`Deserialize`를 **둘 다** 파생한다 — T2가 같은 타입으로 자산을 쓴다. ③ 의존 방향 — `serde`·`serde_json`만 참조하고 `ui`를 모른다. `ui::license_dialog`(T3)와 `examples/gen_licenses.rs`(T2)가 이것을 참조한다. ④ 비추상화 — 자산 소스를 갈아 끼우는 트레이트를 두지 않는다(`include_str!` 한 곳뿐). 검색·필터·정렬 API를 미리 만들지 않는다(정렬은 생성기가 이미 해서 담는다).
  - **Acceptance**:
    - Given 빈 스켈레톤 자산, When `licenses::load()`, Then 패닉 없이 `LicenseData`를 돌려주고 `crates`가 빈 배열이다.
    - `lockfile_fingerprint` 단위 시험 3건이 통과한다 — ⓐ 공백·주석만 다른 두 fixture가 **같은 값** ⓑ 버전 1건이 다른 두 fixture가 **다른 값** ⓒ 이름만 있고 버전이 없는 항목은 이름만 반영된다.
    - 깨진 JSON fixture를 `parse`에 넣으면 빈 데이터를 돌려준다(`unwrap`·`expect` 없음 — AGENTS).
    - `text_indices`에 범위를 벗어난 값이 든 fixture에서 `CrateEntry::texts`가 그 자리를 건너뛰고 나머지를 돌려준다.
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0.
    - **실제 `Cargo.lock` 대조는 이 task의 기준이 아니다**(D10 — T2가 자산을 실제로 만든 뒤에 판정한다).
  - **Files**:
    - 주: `src/app/licenses.rs`(신규)
    - 동반: `src/app/mod.rs`(모듈 선언), `assets/licenses.json`(빈 스켈레톤 — T2가 전량으로 덮어쓴다)
    - 테스트: `src/app/licenses.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 자산 JSON이 깨져 있다 → 빈 데이터로 폴백(위 acceptance)
    - `text_indices`가 범위를 벗어난다 → 그 항목의 전문을 비우고 넘어간다
    - `Cargo.lock`에 이름만 있고 버전이 없는 항목(경로 의존) → 이름만 지문에 넣는다
  - **Halt Forecast**:
    - (i) 자산 형식 확정 → D8
    - (ii-a) 신규 공개 모듈 `app::licenses` · `assets/` 디렉터리 신설 → `## 사전 승인 항목`
  - **Depends on**: -

- [x] T2. 라이선스 자산 생성기를 만들고 자산을 채운다
  - **Type**: D
  - **Design**: ① 배치 — `examples/gen_licenses.rs`(생성기), `assets/spdx/{MIT,Apache-2.0,BSL-1.0}.txt`(표준 전문, 4-D가 지목한 출처에서 손으로 배치), `assets/licenses.json`(T1의 스켈레톤을 덮어쓴다). ② 신규 심볼과 책임 — `main() -> Result<(), String>`(절차 전체), `collect_targets`(`cargo tree --target x86_64-pc-windows-msvc -e normal --prefix none --format "{p}|{l}"` → ` (*)` 접미 제거 → `(이름, 버전, SPDX)` 목록, `moa` 제외), `read_package_info`(`cargo metadata --filter-platform … --format-version 1` → 이름+버전별 `manifest_path`·`authors`), `read_license_files`(크레이트 디렉터리 최상위의 `LICENSE*`·`LICENCE*`·`COPYING*`·`NOTICE*`·`UNLICENSE*` 수집), `dedupe_texts`(내용이 같은 전문을 하나로 접고 인덱스를 돌려준다), `bundled_entries`(수동 등재 3건 — libssh2·zlib·Phosphor 글꼴). ③ 의존 방향 — `moa::app::licenses`의 타입과 `lockfile_fingerprint`를 참조한다(역방향 없음). ④ 비추상화 — 수집 단계를 트레이트로 추상화하지 않는다(구현이 하나뿐이다). 도구 실행은 `std::process::Command` 직접 호출이고 래퍼를 두지 않는다. 출력·오류는 D11의 규약을 따른다.
  - **Acceptance**:
    - Given 레지스트리 캐시가 있는 개발 환경, When `cargo run --example gen_licenses`, Then `assets/licenses.json`이 채워지고 crate 항목이 **158개**(대상 155 + 수동 등재 3)이며 전문 항목이 **90~100개**다(실측 95 기준 — 번들 C 소스의 하위 경로 원문 2건과 표준 전문 3종이 더해지고 중복이 접힌다. 범위를 벗어나면 중복 제거 로직을 재확인한다).
    - 자산의 12개 항목(`accesskit`·`clipboard-win`·`ecolor`·`eframe`·`egui`·`egui_glow`·`egui-winit`·`emath`·`epaint`·`lazy-regex-proc_macros`·`profiling`·`suppaftp`)에 `standard_text: true`가 붙고 SPDX에 맞는 표준 전문을 가리킨다.
    - 수동 등재 3건이 `bundled: true`이며 각각 실제 원문(libssh2·zlib)과 표준 MIT 전문(Phosphor 글꼴)을 가리킨다.
    - Given 채워진 자산, When `cargo test`, Then **지문 대조 시험이 통과한다**(현재 `Cargo.lock`의 `lockfile_fingerprint` == 자산의 `lock_fingerprint`). 어긋나면 실패 문구가 `cargo run --example gen_licenses`로 다시 만들라고 알린다.
    - `cargo tree`·`cargo metadata` 실행이 실패하거나 출력이 예상 형식이 아니면 **오류 문구를 내고 0이 아닌 코드로 종료**한다(빈 자산·부분 자산을 쓰지 않는다).
    - Given 자산 생성 후, When `cargo build --release`, Then `target/release`에 `gen_licenses` 실행 파일이 없다(전제 검증 #9 확정).
    - `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0(예제 타깃 포함).
  - **Files**:
    - 주: `examples/gen_licenses.rs`(신규)
    - 동반: `assets/licenses.json`(전량 갱신), `assets/spdx/MIT.txt`·`assets/spdx/Apache-2.0.txt`·`assets/spdx/BSL-1.0.txt`(신규), `src/app/licenses.rs`(지문 대조 시험 추가)
    - 테스트: `src/app/licenses.rs`의 `#[cfg(test)] mod tests`에 지문 대조·구조 시험 추가
  - **Edge Cases**:
    - 레지스트리 캐시에 크레이트 디렉터리가 없다 → 어느 크레이트가 없는지 적어 오류로 멈춘다(`cargo fetch` 안내)
    - 라이선스 파일이 UTF-8이 아니다 → 손실 없이 읽을 수 없으면 그 파일을 건너뛰고 어느 것을 건너뛰었는지 알린다
    - 같은 이름·버전이 두 번 나온다(`cargo tree`의 `(*)` 행) → 접미 제거 후 중복 제거
    - `authors`가 비어 있다 → 저작권 줄을 생략한다(빈 괄호를 만들지 않는다)
    - 자산이 이미 채워져 있다 → 덮어쓴다(생성기는 멱등이다)
    - **원문이 없는 크레이트의 SPDX에 대응하는 표준 전문이 `assets/spdx/`에 없다** → 어느 크레이트의 어느 식별자인지 적어 **오류로 멈춘다**(추측해 다른 전문을 붙이지 않는다). 지금은 세 파일이 12건을 전부 덮지만(Investigation Log 실측) 의존성이 바뀌면 넷째 식별자가 들어올 수 있다
  - **Halt Forecast**:
    - (i) 표준 전문 3종의 출처 → 4-D에서 크레이트·버전까지 확정
    - (i) 예제에서 오류를 어떻게 알리는가 → D11
    - (ii-a) `examples/` 디렉터리 신설 · `assets/spdx/` 파일 4개 커밋 → `## 사전 승인 항목`
  - **Depends on**: T1

- [x] T3. 라이선스 대화를 그린다
  - **Type**: D
  - **Design**: ① 배치 — `src/ui/license_dialog.rs`(신규), `src/ui/mod.rs`에 선언. ② 신규 심볼과 책임 — `LicenseDialog`(열림 상태 + 고른 항목 인덱스만 든다), `open`/`is_open`/`close`(설정 대화와 같은 표면), `show(&mut self, ctx)`(**구현 시 정정**: 계획은 `-> bool`이었으나 설정 대화와 같이 **닫기 판정을 안에서 하고 `close()`를 부르는** 쪽으로 뒀다 — 호출부가 반환값을 받아 다시 `close()`를 부르면 같은 판정이 두 곳에 갈린다. 바깥은 `is_open()`으로 읽는다), private `show_list`(좌측 목록)·`show_detail`(우측 전문)·`show_header`(제목 줄). ③ 의존 방향 — `ui::dialog`(셸)·`ui::theme`·`ui::widgets`·`app::licenses`(데이터)·`i18n`을 참조하고, `ui::app`이 이것을 참조한다. ④ 비추상화 — 목록 행을 공용 위젯으로 승격하지 않는다(쓰는 곳이 하나다 — 셋째 지점이 생기면 그때 `widgets`로 올린다). 검색·정렬 UI를 만들지 않는다(Deferred).
  - **Acceptance**:
    <!-- 화면에 실제로 무엇이 보이는가는 T3 시점에 판정할 수 없다 — 대화를 여는 배선이 T4다.
         그 축은 아래 「화면 축(T4 이후 수동 검증)」으로 이관했고, 여기에는 T3에서 기계로 판정되는 것만 둔다. -->
    - `LicenseDialog`가 `open`/`is_open`/`close`를 갖고, 닫힌 상태에서 `show`를 불러도 아무것도 그리지 않는다(설정 대화와 같은 표면 — 단위 시험).
    - 선택 인덱스가 항목 수보다 크거나 목록이 비었을 때 0으로 클램프된다(단위 시험).
    - 이중 라이선스 항목의 전문 목록을 만드는 순수 함수가 **`text_indices` 순서대로 전부** 돌려준다(D4 — 단위 시험. 그리는 것이 아니라 무엇을 그릴지 고르는 로직을 잰다).
    - 새 대화가 `ui::dialog`를 거치므로 `대화는_모두_이_모듈을_거친다`가 통과한다. 화면 문구가 전부 카탈로그를 거쳐 `화면_문구가_카탈로그를_거치지_않은_곳이_없다`가 통과한다. 아이콘을 쓴다면 phosphor에서만 가져와 `화면_코드에_원본_아이콘_기호가_남아_있지_않다`가 통과한다.
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0.
    - **화면 축(T4 이후 수동 검증 1~5에서 판정)**: 좌측 목록(이름 + 흐린 버전)과 우측 전문의 배치, 처음 열었을 때 첫 항목 선택, 이중 라이선스의 두 전문 표시, `standard_text` 사유 문구, 목록·전문의 독립 스크롤. UI는 이 레포에서 시험 비대상이다(`AGENTS.md` Conventions).
  - **Files**:
    - 주: `src/ui/license_dialog.rs`(신규)
    - 동반: `src/ui/mod.rs`(선언), `src/i18n/mod.rs`(신규 문구 — 안내문·저작권 라벨·표준 전문 사유·목록 머리글. 개수가 끼어드는 문구는 `i18n::dynamic`에 함수로)
    - 테스트: `src/ui/license_dialog.rs`의 `#[cfg(test)] mod tests`(선택 인덱스 전이·범위 클램프 등 순수 로직)
  - **Edge Cases**:
    - 자산이 비어 있다(파싱 실패) → 목록 자리에 안내 문구를 적고 빈 화면을 보이지 않는다
    - 전문이 매우 길다(Apache-2.0 9,723B) → 우측이 스크롤한다
    - 고른 인덱스가 범위를 벗어난다 → 0으로 되돌린다
    - 좁은 창(1080보다 좁은 창)에서 대화가 화면을 넘는다 → 프레임 크기가 고정이라 egui가 중앙에 놓는다. 사이트 관리자와 같은 성질이며 이번에 바꾸지 않는다
  - **Halt Forecast**:
    - (i) 셸 사용법·좌우 2단 계산 → D7·4-D에서 확정
    - (ii-a) 신규 공개 모듈 `ui::license_dialog` → `## 사전 승인 항목`
  - **Depends on**: T1, T2 (자산이 실제로 채워져야 전문 표시 로직을 눈으로 확인할 수 있다)

- [x] T4. 설정 메뉴에서 대화를 연다
  - **Type**: C
  - **Design**: ① 배치 — `src/ui/menu.rs`(`Command::OpenLicenses` 추가), `src/ui/titlebar.rs`(`show_settings_menu`의 라이선스 줄), `src/ui/app.rs`(필드·초기화·명령 처리·프레임 호출). ② 신규 심볼과 책임 — `Command::OpenLicenses`(라이선스 대화를 연다). ③ 의존 방향 — `titlebar`가 명령을 값으로 내고 `ui::app`이 받아 `LicenseDialog::open`을 부른다(타이틀바는 상태를 바꾸지 않는다 — 기존 규약). ④ 비추상화 — 나머지 세 항목(`업데이트`·`릴리즈 노트`·`정보`)을 위한 공용 배열·반복을 만들지 않는다(`show_settings_menu` 주석이 밝힌 이유 그대로 각 항목이 서로 다른 화면으로 갈라진다).
  - **Acceptance**:
    - Given 앱 실행, When 타이틀바 ⚙ → `오픈소스 라이선스`, Then 라이선스 대화가 열린다(비활성이 아니다).
    - Given 대화가 열림, When `닫기`·`Esc`·배경 클릭, Then 닫힌다.
    - stale이 되는 doc 주석 **둘**이 실제로 고쳐진다 — `src/ui/titlebar.rs:244`의 「`설정`만 동작하고 나머지 넷은…」과 `src/i18n/mod.rs:136`의 「설정 메뉴의 나머지 넷 — 아직 비활성이다」. 둘 다 셋을 가리키게 된다.
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0(`Command`에 wildcard arm이 없어 처리 누락은 빌드가 잡는다 — 전제 검증 #10).
  - **Files**:
    - 주: `src/ui/titlebar.rs`, `src/ui/app.rs`, `src/ui/menu.rs`
    - 동반: `src/i18n/mod.rs`(`:136` doc 주석 1줄 수정 — 문구 추가는 T3 몫이라 충돌하지 않는다)
    - 테스트: 기존 시험 통과 확인(신규 시험 없음 — 배선은 화면 확인 대상)
  - **Edge Cases**:
    - **단축키 억제 게이트**(`src/ui/app.rs:2740-2743`)에 이 대화를 넣을지 — 현재 그 목록은 `pending_remove`·호스트 키·사이트 관리자 셋뿐이고 **설정 대화도 빠져 있다**. 선례를 따라 **넣지 않는다**(대화가 열린 채 Ctrl+T 등이 통과하는 것은 설정 대화와 같은 성질이다). 바꾸려면 설정 대화까지 함께 넣어야 하므로 이번 범위 밖이다.
    - 설정 대화와 라이선스 대화가 동시에 열린다 → 둘 다 모달이라 나중에 그린 것이 위에 온다. 메뉴에서 하나씩만 열리므로 실제로 겹치지 않는다
    - 대화가 열린 채 언어를 바꾼다 → 문구가 다음 프레임에 바뀐다(다른 대화와 같다)
  - **Halt Forecast**:
    - (i) 명령 배선 지점 → 4-A에서 3곳 확정
    - (ii-a) 공개 열거형 `Command`에 variant 추가 → `## 사전 승인 항목`
  - **Depends on**: T3

- [x] T5. 문서를 실제와 맞춘다
  - **Type**: A
  - **Acceptance**:
    - `docs/prd.md`에 **FR-57**(오픈소스 라이선스 화면, Should)이 신설된다.
    - `docs/prd.md:37`(FR-22)의 「설정·업데이트·릴리즈 노트·오픈소스 라이선스·정보 — v1은 항목 표시만 하고 각 기능은 미구현」이 실제(설정·라이선스는 동작, 나머지 셋은 미구현)로 개정된다.
    - `docs/prd.md:119`의 Out of Scope 조항이 같은 파일 115·117행의 기존 표기 관례(`~~…~~ → **날짜 채택/재한정**: …`)를 따라 **재한정**된다 — 오픈소스 라이선스는 채택(FR-57), 나머지 셋은 계속 제외.
    - `docs/prd.md:125`의 성공 기준 Should 목록에 **FR-57이 더해진다**.
    - PRD 결정 이력에 2026-08-18 항목이 더해진다.
    - `README.md` 핵심 기능에 라이선스 화면이 더해지고, 아키텍처 절의 디렉터리 서술에 `assets/`·`examples/`가 반영된다.
    - `AGENTS.md`의 Repository Structure에 `assets/`·`examples/`가, Build & Test에 `cargo run --example gen_licenses`(라이선스 자산 재생성)가, 산출물 절에 `assets/licenses.json`이 커밋되는 생성물임이, Conventions(또는 DO NOT의 단서)에 **예제 타깃의 출력·오류 규약**(D11)이 적힌다.
    - `docs/plans/deferred.md`의 `[2026-07-29] 설정 팝업 네 항목` 항목이 **셋**으로 정정되고(업데이트·릴리즈 노트·정보), 해소분은 `## 종결`로 옮겨진다.
    - 문서에 실제 IP·계정·비밀번호·개인 경로가 없다.
  - **Files**:
    - 주: `docs/prd.md`, `README.md`, `AGENTS.md`
    - 동반: `docs/plans/deferred.md`
    - 테스트: 없음(문서)
  - **Edge Cases**:
    - FR 번호 충돌 → 현재 최대가 FR-56(실측)이라 FR-57이 비어 있다
  - **Halt Forecast**:
    - (ii-a) PRD 문면 개정·`AGENTS.md` 수정 → `## 사전 승인 항목`
  - **Depends on**: T4

- [x] T6. 전문이 아닌 파일을 걸러 내고 라벨을 정돈한다 <!-- Phase F-7이 낸 자기 유발 결함 — 규칙 4-1에 따라 같은 루프에서 고친다 -->
  - **Type**: C
  - **Design**: ① 배치 — `examples/gen_licenses.rs`(수집·라벨)와 `src/app/licenses.rs`(시험·필드 doc). ② 신규 심볼과 책임 — `MIN_LICENSE_BYTES`(원문으로 볼 최소 길이), `label_of(file_name, spdx)`(인자가 하나 늘어 크레이트 선언을 폴백으로 쓴다), `normalize_label`(파일명 조각 → 표준 SPDX 표기). ③ 의존 방향 — 그대로(생성기 → `moa::app::licenses`). ④ 비추상화 — 라이선스 텍스트를 실제로 파싱해 종류를 알아내지 않는다(길이 하나로 스텁만 거른다).
  - **Acceptance**:
    - Given `harfrust`처럼 패키지의 `LICENSE`가 상위를 가리키는 **스텁**(`../LICENSE`, 10B)인 크레이트, When 자산을 다시 만든다, Then 그 항목은 **원문 없음**으로 판정돼 SPDX 표준 전문을 가리키고 `standard_text: true`가 된다.
    - 자산의 모든 전문이 `MIN_LICENSE_BYTES` 이상이다(시험이 단언 — 개수만 세던 기존 시험은 길이도 함께 본다).
    - 화면 라벨에 `apache-2`·`mit` 같은 파일명 조각이 남지 않는다 — 알려진 것은 표준 표기(`Apache-2.0`·`MIT`)로, `LICENSE`·`COPYING`처럼 종류를 알 수 없는 이름은 그 크레이트의 SPDX 선언으로 바뀐다.
    - `LicenseData`의 필드 doc이 실제 내용과 맞는다(라벨이 늘 SPDX 식별자인 것은 아니다).
    - 닫힌 상태에서 `LicenseDialog::show`를 불러도 아무것도 그리지 않는 것을 단위 시험이 잰다(T3 acceptance가 요구했으나 시험이 없었다 — F-7 m2).
    - `cargo build` · `cargo test` · `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check` 전부 경고 0.
  - **Files**:
    - 주: `examples/gen_licenses.rs`, `src/app/licenses.rs`
    - 동반: `assets/licenses.json`(재생성), `src/ui/license_dialog.rs`(시험 추가)
    - 테스트: `src/app/licenses.rs`·`src/ui/license_dialog.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 짧지만 진짜인 전문 — 실측 세 번째로 짧은 것이 553B라 임계 300B에 걸리지 않는다
    - 이중 라이선스 안내문(`aho-corasick`의 125B "dual-licensed under…")도 걸러지지만 그 크레이트는 실제 전문 둘을 따로 갖고 있어 손실이 없다
    - 걸러낸 뒤 표준 전문도 없는 SPDX면 T2가 이미 오류로 멈춘다
  - **Halt Forecast**:
    - (i) 임계값 근거 → 위 Edge Cases의 실측
  - **Depends on**: T5

## 사전 승인 항목 (일괄 승인 대상)

- **T1·T2 — `assets/`·`examples/` 디렉터리 신설**(구조 변경): 레포에 없던 두 디렉터리가 생기고 `assets/licenses.json`(≈400KB)·`assets/spdx/*.txt` 3개가 커밋된다. 되돌리려면 디렉터리 삭제 + `AGENTS.md`·`README.md` 되돌리기.
- **T1·T3 — 신규 공개 모듈 추가**: `app::licenses`(`src/app/mod.rs`에 선언)·`ui::license_dialog`(`src/ui/mod.rs`에 선언). 기존 심볼의 시그니처는 바뀌지 않는다(추가만).
- **T4 — 공개 열거형 `ui::menu::Command`에 variant 추가**: `OpenLicenses`. `match` 처리부는 `src/ui/app.rs` 한 곳이며 wildcard arm이 없어 누락 시 빌드가 실패한다.
- **T5 — PRD 문면 개정**: FR-57 신설 + FR-22 개정 + Out of Scope 재한정 + 성공 기준 Should 목록 갱신 + 결정 이력 추가. **PRD는 요구사항 정본이므로 개정 자체가 승인 대상**이다.
- **T5 — `AGENTS.md` 수정**: Repository Structure·Build & Test·산출물·예제 규약 네 곳.
- **각 task 완료 시 로컬 작업 브랜치 커밋** — push·병합은 포함하지 않는다.

## 불가피한 Halt (위임 불가)

- push · `master` 병합 · 태그 · 릴리즈 · PR
- 계획에 없던 파괴적 작업(파일 재귀 삭제·history rewrite 등)
- 인증정보가 필요한 신규 외부 서비스 도입 — 이번 회차에 예정 없음(생성기는 네트워크를 타지 않는다)

## Verification Strategy

- 빌드: `cargo build` · `cargo build --release`(T2 acceptance의 예제 미포함 확인)
- 단위·통합 테스트: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings`
- 형식: `cargo fmt --check`
- 자산 재생성: `cargo run --example gen_licenses`
- 수동 검증 (빌드로 못 재는 축):
  1. 타이틀바 ⚙ → `오픈소스 라이선스`가 눌리고 대화가 열린다
  2. 좌측 목록을 스크롤해 임의 항목을 고르면 우측 전문이 바뀐다
  3. 이중 라이선스 항목에서 전문이 둘 다 보인다
  4. 12건 중 하나(`egui` 등)에서 표준 전문 사유 문구가 보인다
  5. 수동 등재 3건(libssh2·zlib·Phosphor 글꼴)이 목록에 있고 전문이 보인다
  6. 언어를 영어로 바꾸면 화면 문구가 바뀌고 라이선스 전문은 원문 그대로다
  7. `닫기`·`Esc`·배경 클릭으로 닫힌다

## 리뷰 이력

| 라운드 | 지적 | 심각도 | 반영 방식 |
|---|---|---|---|
| 1 | B1 task 번호와 의존 순서가 반대 — 자율 루프가 T1부터 도는데 T1이 T2 산출을 쓴다 | BLOCKER | 수용 — 번호 교체(T1=모델, T2=생성기). `Depends on`·PRD Coverage·사전 승인·Next Steps 표기를 함께 정정 |
| 1 | M1 PRD Out of Scope 조항(`:119`)·성공 기준 Should 목록(`:125`) 미개정 | MAJOR | 수용 — T5 acceptance 2줄 추가, `동반 변경 판정`에 필수 2행 추가, PRD Coverage에 2행 추가 |
| 1 | M2 T2(현 T1) 지문 acceptance가 자기 시점에 만족 불가능 | MAJOR | 수용(㉠) — 모델 task는 fixture 단언만, 실제 `Cargo.lock` 대조는 자산이 채워지는 T2로 이동. 근거를 D10으로 명문화 |
| 1 | M3 "전문 항목 95개 안팎"이 기계 판정 불가 | MAJOR | 수용 — `90~100개`로 범위를 못 박고 그 근거(하위 경로 원문 2건 + 표준 전문 3종의 가감)를 Investigation Log에 추가 |
| 1 | m1 생성기 task에 clippy·fmt 없음 | MINOR | 수용 — T2 acceptance에 추가(T1·T3·T4도 같은 줄로 통일) |
| 1 | m2 예제의 `println!`·`unwrap` 방침 부재 | MINOR | 수용 — D11 신설 + T5의 `AGENTS.md` 갱신 항목에 편입 |
| 1 | m3 표준 전문 3종의 출처 미특정 | MINOR | 수용 — 크레이트·버전·바이트까지 실측해 4-D와 Investigation Log에 기재 |
| 1 | m4 단축키 억제 게이트 판단 부재 | MINOR | 수용 — T4 Edge Cases에 판단과 근거(설정 대화도 빠져 있다)를 명시 |
| 1 | m5 PRD Coverage의 FR-57 대응 task가 `T1~T4`인데 신설은 T5 | MINOR | 수용 — `T1~T5 (신설은 T5)` |
| 1 | (참고) plan의 라인 인용 2~3줄 오차 | — | 수용 — `show_settings_menu` 244, `pending_item` 267, `Command` match 2031로 정정 |
| 2 | M1 T3 acceptance 4건이 T3 시점에 판정 불가(대화를 여는 배선은 T4, 자산은 T2) | MAJOR | **수용** — `Depends on`을 `T1, T2`로 고치고, 시각 4항목을 「화면 축(T4 이후 수동 검증 1~5)」으로 명시 이관. T3에는 기계 판정되는 축(표면·클램프·전문 선택 로직·규약 시험·빌드)만 남겼다 |
| 2 | M2 `src/i18n/mod.rs:136`의 doc 주석이 stale이 되는데 판정에서 빠졌다 | MAJOR | **수용** — 실물 확인(주석이 `:136`, 덮는 키 `:137~:140`). `동반 변경 판정`에 필수 행 추가, T4 Files 동반과 acceptance에 편입 |
| 2 | m1 Investigation Log의 단축키 게이트 서술이 T4 결정과 시제가 어긋남 | MINOR | 수용 — 「T4가 판단한다」 → 「T4는 선례대로 넣지 않는다」 |
| 2 | m2 T1 Design ②에 파싱 진입점·범위 초과 처리·`Serialize`가 빠짐 | MINOR | 수용 — `parse(&str)`·`CrateEntry::texts`·`Serialize`+`Deserialize` 파생을 Design ②에 추가하고 acceptance 2줄로 잰다 |
| 2 | m3 표준 전문 3종이 12건의 SPDX를 전부 덮는지 미확인 | MINOR | **수용 — 실측으로 해소**: 12건의 SPDX가 `MIT OR Apache-2.0` 10 · `BSL-1.0` 1 · `MIT` 1뿐이라 셋이 전부 덮는다(Investigation Log 기재). 의존성 변경 대비로 「표준 전문 없는 식별자 → 오류로 멈춘다」를 T2 Edge Cases에 추가 |

**기각한 지적은 없다.** 리뷰어가 "결함이 아니다"로 판정한 것(1라운드: 심볼 추적 정확·전제 #9는 Open Question 대상 아님·AGENTS 규약 위반 없음 / 2라운드: 번호 교체가 어긋난 곳 없음·T1의 clippy `-D warnings`는 `lib.rs`의 재수출 때문에 `dead_code`가 발동하지 않음)은 그대로 두었다.

**이 리뷰는 수렴이 아니라 예산 소진으로 끝났다.** 재호출 상한(2회)을 다 쓴 시점에 **동일 지적 잔존은 0건**이고(1라운드 10건은 2라운드에서 실물 대조로 전건 해소 확인) 2라운드 신규 지적 5건은 메인이 실물에서 직접 대조해 처리했다 — 그 판정이 위 표의 2라운드 행이다. 3라운드를 열지 않은 이유는 상한 규정이며, 남은 5건이 모두 **문면·범위 조정으로 끝나고 구조 재설계를 요구하지 않는다**는 점도 같다.

## Phase Ledger

- Phase F 2회차 통과 (HEAD 기준 — F-7 2회차: 1회차 지적 4건 전건 해소 확인, 신규 MAJOR 1(대장의 전문 수 stale)·MINOR 3(라벨 폭·README 트리 위치·exe 크기)은 전부 문서라 그 자리에서 정정)
- Phase F 1회차: F-7이 자기 유발 MAJOR 1건(`harfrust`의 전문이 `../LICENSE` 스텁)을 잡아 **T6를 추가**해 고쳤다 — 규칙 4-1에 따라 이연하지 않았다

## Retry Ledger

## Progress Log

- T1-T2 완료 (커밋 653cdd1, 다음): `app::licenses`가 자산을 읽고 `examples/gen_licenses.rs`가 그것을 채운다. 구성 요소 **158개**(대상 155 + 번들 3) · 전문 **95개**(중복 제거) · 자산 369KB. 시험 836건 통과.
  - 결정: **저작자 표기에서 `@`가 든 낱말을 통째로 뺀다** — crates.io에 공개된 값이어도 커밋되는 파일에 메일 주소·핸들을 담지 않는다(규칙 6-1). `<>`로 감싼 것뿐 아니라 맨몸으로 붙은 것도 있었다(`Rich Geldreich rich@…`·`Amod Malviya @amodm` 실측). 시험이 `@` 잔존 0을 지킨다.
  - 결정: `assets/spdx/MIT.txt`만 원본을 가공한다(첫 줄을 SPDX 플레이스홀더로) — 그러지 않으면 `bitflags`의 저작권자가 무관한 12개 크레이트의 고지에 오귀속된다. Apache-2.0·BSL-1.0은 본문에 저작권자 자리가 없어 그대로 쓴다.
  - 관측: **자산을 담아도 release exe가 2KB만 늘었다**(8,222,208 → 8,224,256B) — 아직 `load()`를 부르는 코드가 없어 `lto` + `strip`이 `include_str!` 데이터를 죽은 것으로 걷어낸 것이다. T3가 대화를 붙이면 실제 크기가 드러난다.
  - 관측: 생성기는 멱등이다 — 두 번 돌려도 `assets/licenses.json`이 바이트 단위로 같다(디렉터리 열거 순서를 이름으로 세우고 전문을 내용으로 중복 제거하기 때문).

- T3-T4 완료 (커밋 0a35c79, 다음): `ui::license_dialog`가 좌우 2단 대화를 그리고 설정 메뉴의 `오픈소스 라이선스`가 그것을 연다. 시험 839건 통과.
  - 계획 정정 2건: `show`는 `-> bool` 대신 **닫기 판정을 안에서 하고 `close()`를 부른다**(설정 대화와 같은 모양 — 판정이 두 곳에 갈리지 않게), 좌 열 폭은 사이트 관리자의 400이 아니라 **320**(담기는 것이 이름 한 줄뿐이라 남는 자리를 전문에 준다). 둘 다 plan Design·4-D에 사유를 적었다.
  - 함정: **위젯 id의 한글은 `i18n` 소스 훑기 시험에 걸린다** — `Id::new("라이선스 대화")`·`id_salt` 둘이 그랬다. AGENTS의 「위젯 상태를 잇는 열쇠」 예외에 해당하므로 `EXEMPT_LITERALS`에 3건을 등재해야 통과한다(문구를 카탈로그로 옮기면 화면 언어를 바꿀 때 위젯 상태가 초기화된다).
  - 함정: **안내 문구의 높이를 상수로 박으면 언어를 바꿀 때 어긋난다** — 한국어는 한 줄, 영어는 두 줄이 될 수 있다. 그려진 갤리를 재어 아래 요소의 자리를 잡는다.

- T5-T6 완료 (커밋 38e1ac0, 49bd9fd): 문서 넷(PRD·README·AGENTS·deferred 대장)을 실제와 맞추고, Phase F-7이 잡은 스텁 전문 결함을 고쳤다. 시험 850건 통과.
  - 함정: **이름이 `LICENSE`인데 내용은 원문이 아닌 파일이 있다** — `harfrust`는 `../LICENSE` 열 바이트, `aho-corasick`은 125바이트 이중 라이선스 안내다. 파일 존재만 보고 담으면 화면의 전문 자리에 그 한 줄만 뜬다. 길이(300B)로 거르고 그 규칙을 시험이 지킨다.
  - 관측: release exe 8,222,208 → 8,634,368B(+5.0%). 자산을 참조하는 코드가 생기기 전(T2 시점)에는 `lto`+`strip`이 `include_str!` 데이터를 걷어내 2KB만 늘었다.

## Next Steps

- 권장 다음 액션: T1부터 `pjc:implement-task` 실행
- Suggested skills: `pjc:llm-wiki`(위키 반영 — `feat-titlebar-tray`의 「넷은 비활성」, `feat-dialog-shell`의 「모달은 여덟」이 이번에 바뀐다)

## Open Questions

- [x] Q1. 수집 도구 → **레포 안 자체 생성기**(D1)
- [x] Q2. 생성기 위치 → **`examples/gen_licenses.rs`**(D2)
- [x] Q3. 화면 구조 → **좌우 2단 모달**(D7)
- [x] Q4. stale 검출 → **`Cargo.lock` 지문 대조 시험**(D6)
- [x] Q5. 이번 범위 → **오픈소스 라이선스만**(Out of Scope)
- [x] Q6. 이중 라이선스 표기 → **선언 그대로 + 전문 둘 다**(D4)
- [x] Q7. 원문 없는 12건 → **표준 전문 + 사유 표기**(D5)
