# Plan: 앱 설정 화면 — part2 (앱 언어 전환 · 전면 영문화)

**PRD**: docs/prd.md

**이전 plan**: docs/plans/2026-08-13-app-settings-part1.md

## 요구 이해

- **원문 요청**: "5. 앱 언어 변경 — 시스템 기본, 한글, 영문 선택해서 앱의 언어 변경"
- **이해한 요구**: 설정 화면의 `언어` 그룹에서 `시스템 기본`·`한국어`·`English`를 골라 앱 전체 문구를 재시작 없이 바꾼다. 부분 번역이 아니라 **화면에 보이는 문구 전부**가 대상이다 — 실행 경로의 UI 한글 문자열 약 266개와 `format!`로 조립되는 문구 37곳이며, 후자는 조사(`을(를)`)가 영어에 없어 문장 틀째로 갈라야 한다. 지금은 문자열이 파일마다 흩어져 있어(중앙 카탈로그 없음) 먼저 카탈로그를 만들고 파일 그룹별로 옮긴다.
- **포함하지 않는 것으로 이해**: 한국어·영어 외의 언어, 외부 번역 파일(.po/.ftl) 형식, 날짜·숫자·통화의 지역화 서식, 실행 파일에서 쓰이지 않는 구 Win32 UI 코드(`src/app/menu.rs` 등 30개 문자열)의 번역.

## Goal

설정 화면에서 앱 언어를 고르면 재시작 없이 모든 화면 문구가 한국어·영어로 바뀐다.

**전체 목표**: 앱 설정 화면에서 글꼴·자동 실행·트레이 전환·파일 표시 방식·언어를 바꾸고 그 값이 즉시 반영·유지된다(part1 + part2).

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-53 | Must | T1, T2, T3~T7 | ✅ 커버 |
| NFR-6 | — | T1, T8 | ✅ 커버 |
| FR-47 | Must | T2 (`언어` 그룹 추가) | ✅ 이전 part 기구현 + 이 part가 그룹 하나를 채움 |
| FR-13, FR-48~FR-52 | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-1~FR-12, FR-14~FR-46 | Must/Should/Could | (기구현) | 이번 범위 외 (기구현) |
| NFR-1~NFR-5, NFR-7~NFR-13 | — | (기구현) | 이번 범위 외 (기구현) |

## Out of Scope

- 한국어·영어 외의 언어
- 외부 번역 리소스 파일(.po·.ftl·.json) — 카탈로그는 소스 안에 둔다(빌드·배포가 단일 exe라 외부 파일을 들이면 배포 단위가 늘어난다)
- 날짜·숫자·크기 단위·통화의 지역화 서식 — 표시 형식은 지금 것을 그대로 쓴다
- 구 Win32 UI 코드의 문구(`src/app/menu.rs` 18개·`src/panel/panel.rs` 5개·`src/panel/file_list.rs` 4개·`src/app/window.rs` 2개·`src/app/sidebar.rs` 1개) — 실행 파일에서 쓰이지 않는 사문이라 번역해도 화면에 나오지 않는다(AGENTS "egui 이식 이전 구현")
- 테스트 코드 안의 한글 문자열(약 1,000개) — 화면에 나오지 않는다
- 사용자 데이터에서 온 문자열(워크스페이스 이름·사이트 이름·파일명·서버 응답 원문)

## Deferred / Follow-up

- 구 Win32 UI 코드 제거 — 대장에 이미 있는 항목(`[2026-07-28] 구 Win32 UI 코드 제거`)과 같은 뿌리다. 지우면 위 Out of Scope 항목도 함께 사라진다
- 서버 응답 원문(`530 Login incorrect` 등)에 붙는 우리 설명 문구는 번역하되 **응답 원문은 그대로 둔다** — 원문까지 손대려면 서버 코드 사전이 필요하고, 사용자가 서버 관리자에게 전할 값이라 원문이 오히려 유용하다. 코드별 사용자 설명 사전이 필요해지면 별도 작업

## Investigation Log

- 위키 참조: 관련 위키 자료 없음 — vault는 설정돼 있으나 i18n 관련 페이지가 없어 코드를 1차 출처로 진행
- **중앙 문자열 모듈이 없다** — `src/`에 `strings.rs`·`i18n`·`locale`·`lang` 모듈이 없고 `.ftl`/`.po`/`.json` 리소스도 없다(`grep -rniE "i18n|locale|lang|translat"` 유효 히트 0건). 대신 **파일별 사설 `const` 블록 89개**가 사실상의 리소스 층이다
- 실행 경로의 UI 한글 문자열 **약 266개**(레거시 30개 제외). 상위 분포 — `ui/site_manager.rs` 42, `remote/sftp.rs` 29, `ui/remote_menu.rs` 27, `ui/remote_states.rs` 18, `ui/app.rs` 17, `ui/queue_panel.rs` 15, `remote/ftp.rs` 12, `ui/titlebar.rs` 12, `remote/connection.rs` 11, `ui/menu.rs` 10, `ui/panel.rs` 10, `ui/sidebar.rs` 10, `remote/types.rs` 9, `ui/view_mode.rs` 8
- `format!`/`write!`로 **동적 조립되는 한글 문구 약 37곳**. 대표 — `remote/types.rs:382-404`(`impl Display for RemoteError` 8개, `'{path}'을(를) 찾을 수 없습니다 — {detail}` 등), `remote/sftp.rs:380`·`:471`, `remote/connection.rs:430`·`:574`, `ui/panel.rs:843`·`:1184`, `ui/app.rs:1958`·`:2229`, `ui/remote_menu.rs:399`, `ui/toast.rs:27`
- **어순이 굳은 결합이 있다** — `ui/status_bar.rs:64`가 `format!("{}건 대기", …)`를 만들고 `:73`이 `out.push_str(" 남음")`으로 덧붙인다. 영어로는 조각 순서가 달라져 **문장 단위로 다시 짜야 한다**
- **번역하면 안 되는 한글 문자열 8곳** — 위젯 상태 키다: `ui/panel.rs:1289 egui::Id::new("원격 메뉴")`, `ui/remote_menu.rs:277 ("원격 이름 대화", title)`·`:322 "원격 권한 변경"`·`:391 "원격 삭제 확인"`, `ui/remote_states.rs:429 "원격 호스트 키 확인"`, `ui/site_manager.rs:535 "사이트 관리자"`·`:1266 .id_salt("사이트 이름 바꾸기")`, `ui/toast.rs:80 Area::new(egui::Id::new("원격 알림"))`. 바꾸면 위젯 상태가 초기화된다
- 조사 결합(`을(를)`·`이(가)`)이 문구에 직접 박혀 있다 — `remote/types.rs:388`·`remote/sftp.rs:380`·`ui/panel.rs:843`. 영어에는 대응이 없어 문장 틀 자체가 갈린다
- 오류 문구가 **동사 조각을 인자로 받는 구조**다 — `remote/sftp.rs`·`ftp.rs`의 `classify(err, "이름 바꾸기", path)`가 `format!("{operation}: {err}")`로 합친다. 조각과 틀을 함께 옮겨야 한다
- 선례 — AGENTS 규약 `아이콘은 egui_phosphor에서만`을 `ui::widgets::is_icon_font`와 테스트(`화면_코드에_원본_아이콘_기호가_남아_있지_않다`)가 지킨다. **소스를 읽어 규약 위반을 잡는 테스트가 이 레포에 이미 있다** — 같은 방식으로 미등록 한글 리터럴을 막을 수 있다
- 시스템 UI 언어 판정: `windows-0.62.2` `Globalization/mod.rs:585 GetUserDefaultUILanguage() -> u16`. feature `Win32_Globalization`은 이미 켜져 있다(`Cargo.toml` — CP949 변환 용도)
- part1이 `AppSettings.language: LanguageSetting`(`System`/`Korean`/`English`)을 이미 스키마에 넣어 두었다 — 이 part는 **값을 쓰기만** 하면 되고 세션 스키마를 건드리지 않는다

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 언어를 바꾸면 재시작 없이 화면이 갱신된다 | egui는 즉시 모드라 매 프레임 문자열을 다시 만든다 — 캐시된 텍스트 위젯이 없다(`ui.label(…)`이 프레임마다 인자를 받는다). 갱신은 `ctx.request_repaint()` 없이도 다음 프레임에 반영된다 | ✅ |
| 2 | 시스템 UI 언어를 판정할 수 있다 | `windows-0.62.2` `Globalization/mod.rs:585 GetUserDefaultUILanguage`. 반환값 하위 10비트가 `LANG_KOREAN`(0x12)이면 한국어 | ✅ |
| 3 | 세션 스키마를 다시 건드리지 않아도 된다 | part1 T1이 `language` 필드를 `AppSettings`에 이미 넣는다(part1 T1 Design·Acceptance) | ✅ |
| 3-a | 모듈 선언의 정본은 `src/lib.rs` 하나다 | `src/lib.rs:5-9`가 `app`·`fs`·`panel`·`remote`·`ui`를 선언하고, `src/main.rs:11-15`는 `use moa::…`만 쓴다(`mod` 선언 0건) — `i18n`도 `lib.rs`에만 넣어야 전역 상태가 하나로 유지된다 | ✅ |
| 3-b | 언어를 바꾸면 **다음 프레임**에 반영된다 | egui는 즉시 모드라 다음 pass에 문자열을 다시 만든다. 다만 이미 그려진 프레임은 바뀌지 않으므로 전환 직후 `ctx.request_repaint()`가 필요하다 | ✅ (같은 프레임이 아니라 다음 프레임) |
| 4 | 소스를 읽어 규약 위반을 잡는 테스트가 이 레포에서 동작한다 | `src/ui/widgets.rs`의 `화면_코드에_원본_아이콘_기호가_남아_있지_않다`가 같은 기법을 쓴다(AGENTS 아이콘 규약) | ✅ |
| 5 | 번역 대상에서 빼야 할 문자열의 목록이 확정돼 있다 | 위젯 ID 8곳(Investigation Log) + 레거시 30개 + 테스트 코드 — 세 부류 모두 위치가 특정됐다 | ✅ |
| 6 | 전역 현재 언어를 안전하게 읽을 수 있다 | UI는 단일 스레드지만 `remote` 워커도 오류 문구를 만든다 — `AtomicU8`이면 스레드 경계를 넘어도 안전하고 잠금이 없다(`std::sync::atomic`, 새 의존성 없음) | ✅ |
| 7 | 매크로로 함수를 펼쳐도 컴파일 타임에 한·영 누락을 잡는다 | `macro_rules!`가 두 리터럴을 모두 요구하면 하나만 적은 항목은 **컴파일 오류**가 된다(런타임 검사 불필요) | ✅ |
| 8 | 문구 이관이 화면 배치를 바꾸지 않는다 | ⚠ 미확인 — 영어 문구가 한글보다 길어지는 곳(예: `연결 없음` → `Not connected`)이 있고, 고정 폭 필드(`widgets::FORM_LABEL_WIDTH:184` = 96px)나 고정 폭 대화(`site_manager` `DIALOG_WIDTH`)에서 잘릴 수 있다. **성립을 좌우하지는 않는다**(문구는 바뀌고 기능은 동작한다) — T8 수동 검증에서 잘림을 확인해 필요한 곳만 넓힌다 |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 영어 문구가 길어 고정 폭 UI에서 잘림 | 화면이 읽히지 않음 | 전제 8 — T8 수동 검증 항목에 "영어로 전환한 채 전 화면 훑기"를 넣고, 잘리는 곳만 폭을 넓히거나 문구를 줄인다. 폭 상수를 미리 손대지 않는다(추측 수정 방지) |
| 위젯 ID로 쓰이는 한글 문자열을 함께 옮김 | 대화 상태·팝업 위치가 초기화되고 원인이 화면에서 안 보임 | Investigation Log에 8곳을 파일:라인으로 특정했다. 각 이관 task의 Edge Cases에 "그 파일의 `Id::new`·`id_salt` 인자는 건드리지 않는다"를 명시하고, T8 회귀 테스트가 `i18n` 호출이 `Id::new` 인자에 들어가지 않았는지 본다 |
| 이관 중 문구가 조용히 바뀜(의역·오타) | 사용자가 보던 문구가 달라짐 | 한국어 값은 **원문 그대로 복사**한다(의역 금지). T8 회귀 테스트가 "카탈로그의 한국어 값 집합"과 "이관 전 커밋의 문자열 집합"을 대조할 수는 없으므로, 각 이관 task의 Acceptance에 "한국어로 두고 실행하면 이관 전과 문구가 같다"를 넣는다 |
| 동적 조립 문구의 인자 순서가 언어별로 다름 | 영어 문장이 어색하거나 뜻이 뒤바뀜 | 정적 문구는 매크로, **동적 문구는 인자를 받는 손수 쓴 함수**로 분리한다(D2) — 언어별로 `format!`을 따로 쓰므로 어순이 자유롭다 |
| 266개를 옮기다 일부를 빠뜨림 | 한 화면에 한·영이 섞임 | T8의 회귀 테스트가 소스를 훑어 미등록 한글 UI 리터럴을 잡는다(전제 4). 이것이 이 part의 완료 판정 장치다 |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| 파일별 문구 `const` 89개 | `ui/site_manager.rs:117-189`, `ui/remote_menu.rs:12-22`, `ui/remote_states.rs:64-85`, `ui/queue_panel.rs:43-67`, `ui/dock.rs:31-35`, `ui/panel.rs:47-48`, `ui/sidebar.rs:21-85`, `remote/connection.rs:603-607`, `remote/log.rs:32-33` 외 | `i18n` 호출로 대체 — 상수 선언 삭제 |
| 인라인 문자열 인자 약 140개 | `ui/titlebar.rs:181-248`, `ui/app.rs:764-773`·`:2200-2203`, `ui/menu.rs:114-127`, `ui/sidebar.rs:216-472`, `ui/tabs.rs:206-409`, `ui/view_mode.rs:56-63`, `ui/list_details.rs:49-54`, `remote/log.rs:30-33` 외 | `i18n` 호출로 대체 |
| `format!`/`write!` 조립 37곳 | `remote/types.rs:382-404`, `remote/sftp.rs:380,471`, `remote/connection.rs:430,574`, `ui/panel.rs:843,1184`, `ui/app.rs:1958,2229`, `ui/remote_menu.rs:399`, `ui/toast.rs:27`, `ui/status_bar.rs:64,73` 외 | 인자를 받는 `i18n` 함수로 대체 |
| `classify(err, operation, path)` | `remote/sftp.rs`, `remote/ftp.rs` | 동사 조각 인자가 `&str`에서 언어 중립 키(enum)로 바뀐다 |
| `impl Display for RemoteError` | `remote/types.rs:382-404` | 언어별 문장으로 갈라진다 |
| `AppSettings.language` | `src/app/settings.rs`(part1), `src/ui/settings_dialog.rs`, `src/ui/app.rs` | 값 사용 — 스키마 변경 없음 |

### 4-B. 계약·직렬화 변경

- **없다** — `LanguageSetting`은 part1이 이미 직렬화 형식을 정했고 이 part는 값을 읽어 쓸 뿐이다
- `RemoteError`의 `Display` 출력 문자열이 언어에 따라 달라진다. 이 값은 화면·로그에만 쓰이고 저장·비교 대상이 아니다(`grep`으로 확인 — T1에서 재확인)

### 4-C. 영향 받는 테스트

- 한글 문구를 단언하는 기존 테스트 — `remote/*`·`ui/*`의 오류 문구 단언이 있으면 언어를 한국어로 고정한 뒤 비교하도록 고친다(T3~T7 각 task가 자기 그룹의 테스트를 함께 처리)
- 신규: `src/i18n/mod.rs`(키 왕복·시스템 판정), 소스 훑기 회귀 테스트(T8)

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `i18n` 모듈·`strings!` 매크로 | 중앙 문자열 모듈 0건(Investigation Log) | **신규** — 없다. 외부 crate(`fluent`·`rust-i18n`)는 AGENTS 최소 의존 원칙에 따라 기각(D3) |
| `i18n::current()` / `set_language()` | 전역 상태 관리 코드 없음 | **신규** — `AtomicU8` 하나 |
| `i18n::system_language()` | `Win32_Globalization`은 CP949 변환에만 쓰인다(`remote/charset`) | **신규 함수** — 같은 feature를 쓰지만 하는 일이 다르다 |
| 소스 훑기 회귀 테스트 | `ui/widgets.rs`의 `화면_코드에_원본_아이콘_기호가_남아_있지_않다` | **기법 재사용, 검사 대상 신규** — 파일을 읽어 규약 위반을 잡는 골격을 그대로 따른다 |

### Verified by

- `grep -rnP "\"[^\"]*[가-힣][^\"]*\"" src/ --include=*.rs` → 실행 경로 266개 + 레거시 30개 + 테스트 약 1,000개로 분류 완료(Investigation Log). 이관 대상 266개는 T3~T7의 파일 그룹에 빠짐없이 배정됐다(아래 배정표)
- `grep -rn "Id::new\|id_salt" src/ --include=*.rs | grep -P "[가-힣]"` → 8 hits, 전부 "번역 금지" 목록에 포함
- `grep -rn "format!\|write!" src/ --include=*.rs | grep -P "[가-힣]"` → 37곳, 전부 T3~T7에 배정

**파일 그룹 배정표** (266개 전수 — 합계가 맞지 않으면 누락이다)

| task | 대상 파일 | 개수(추정) |
|---|---|---|
| T3 | `ui/titlebar.rs` 12, `ui/menu.rs` 10, `ui/sidebar.rs` 10, `ui/view_mode.rs` 8, `ui/tabs.rs` 3, `ui/address_bar.rs` 3, `ui/site_dropdown.rs` 2, `ui/tree.rs` 1, `ui/toast.rs` 1 | 50 |
| T3 (part1 신규분) | **`ui/settings_dialog.rs`**(그룹 제목 5·항목 라벨 7·`닫기`·언어 선택지 3 등), **`ui/tray.rs`**(툴팁 `MOA`·메뉴 `실행`·`종료`), `ui/widgets.rs`(T2 `toggle_row`에 문구가 생기면) | part1 완료 후 실측해 채운다 |
| T4 | `ui/panel.rs` 10, `ui/list_details.rs` 6, `ui/status_bar.rs` 5, `fs/create.rs` 3, `app/workspace.rs` 1, `panel/tabs.rs` 1 | 26 |
| T5 | `ui/site_manager.rs` 42, `ui/remote_menu.rs` 27, `ui/remote_states.rs` 18 | 87 |
| T6 | `ui/app.rs` 17, `ui/queue_panel.rs` 15, `ui/dock.rs` 4, `remote/sites.rs` 1, `remote/transfer.rs` 1 | 38 |
| T7 | `remote/sftp.rs` 29, `remote/ftp.rs` 12, `remote/connection.rs` 11, `remote/types.rs` 9, `remote/log.rs` 4 | 65 |
| — | 합계 | **266 + part1 신규분** |

> **배정표는 part1 착수 *전* 코드 기준이다.** part1이 새로 만드는 `ui/settings_dialog.rs`·`ui/tray.rs`(그리고 `app/fonts.rs`·`app/autostart.rs`·`app/single_instance.rs`에 사용자 노출 문구가 생기면 그것까지)의 문구는 이 표에 수치로 잡혀 있지 않다. **T3 착수 시점에 그 파일들의 실제 한글 리터럴 수를 세어 위 행을 채우고 합계를 다시 닫는다** — 그러지 않으면 T8의 소스 훑기 테스트가 `ui/tray.rs`에서 잔여 문구를 잡았을 때 「불가피한 Halt」의 "배정표에 없는 파일" 조항이 발동해 자율 루프가 선다. part1의 신규 파일은 **배정표에 이미 등재된 대상**이므로 그 Halt 조항의 대상이 아니다.

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md` NFR-6·Out of Scope | 이 part가 "한국어 고정"을 뒤집는다 | **계획 단계에서 이미 반영**(part1과 함께 승인) |
| 필수 | `AGENTS.md` `## Conventions` — 새 화면 문구는 카탈로그를 거친다는 규약 | 카탈로그를 만들어도 규약이 없으면 다음 작업이 문자열을 다시 소스에 박고, 그러면 화면이 다시 섞인다. 아이콘 규약(`egui_phosphor에서만`)이 같은 자리에 있다 | T8에 편입 |
| 필수 | `README.md` — 언어 전환 기능 | 기능 추가라 문서 갱신 기준에 해당 | T8에 편입 |
| 선택 | 구 Win32 UI 코드 제거(레거시 문구 30개가 함께 사라진다) | 이 part 없이도 성립하지만, 남겨 두면 회귀 테스트의 예외 목록에 레거시 파일을 계속 두어야 한다 | **Deferred로 처리**(대장에 이미 있는 항목과 한 뿌리 — 여기서 함께 지우면 이 plan의 범위가 크게 넘친다) |
| 무관 | `docs/design/*` | 디자인 문서는 한국어 화면 기준이고 이 변경이 배치·색을 바꾸지 않는다 | 건드리지 않음 |
| 무관 | `Cargo.toml` | `Win32_Globalization`이 이미 켜져 있고 새 crate를 들이지 않는다(D3) | 건드리지 않음 |

## Decisions

### D1. 카탈로그를 어디에 어떤 형태로 두는가
- **Options**: A) `src/i18n/mod.rs`에 `macro_rules!`로 선언 → 함수 생성 / B) 언어별 `HashMap<&str, &str>` + `t("key")` 문자열 조회 / C) 파일별 `const`를 언어별로 둘씩 두고 `match`
- **Chosen**: A
- **Rationale**: B는 키 오타가 **런타임에** 빈 문자열로 나타나고(화면에서야 발견) 해시 조회가 매 프레임 수백 번 돈다. C는 파일마다 `match`가 흩어져 "다 옮겼는가"를 셀 수 없다. A는 키가 곧 함수명이라 오타가 컴파일 오류이고, 한·영 둘 다 적지 않으면 역시 컴파일 오류다(전제 7). 매크로가 하는 일은 **선언 목록을 함수로 펼치는 것 하나**이며 조건·분기·타입 조작이 없다.
- **Source**: 전제 7, AGENTS "영리한 추상화보다 명시적·직접적 코드"(이 매크로는 간접화가 아니라 반복 제거)

### D2. 동적 조립 문구를 어떻게 다루는가
- **Options**: A) 인자를 받는 손수 쓴 함수(언어별 `format!`) / B) 자리표시자(`{0}`) 문자열을 카탈로그에 두고 런타임 치환 / C) 정적 조각을 이어 붙이기(현행 `push_str` 방식 유지)
- **Chosen**: A
- **Rationale**: B는 인자 개수·순서가 타입으로 보장되지 않고, 조사(`을(를)`)를 다루려면 결국 언어별 분기가 필요하다. C는 `status_bar.rs:64,73`처럼 **어순이 굳어** 영어에서 문장이 깨진다(Investigation Log). A는 언어별로 문장을 통째로 쓰므로 어순·조사·복수형이 자유롭고, 인자는 컴파일러가 검사한다. 37곳뿐이라 손으로 쓸 만하다.
- **Source**: Investigation Log(어순 결합·조사 결합), 전제 7

### D3. 외부 i18n crate를 쓰는가
- **Options**: A) 직접 구현 / B) `fluent` / C) `rust-i18n`
- **Chosen**: A
- **Rationale**: 최소 의존 원칙의 순차 체크 — ① 필요한가: 언어 2개·문구 266개로 확정 ② 코드베이스에 있는가: 없다 ③ 표준 라이브러리로 되는가: `std::sync::atomic` + `macro_rules!`로 된다 ④⑤ 플랫폼·기설치 의존성: 해당 없음 ⑥ 최소 구현: 약 30줄의 매크로 + 함수. B·C는 복수형 규칙·성별·외부 파일 로딩 등 이 앱에 없는 문제를 위한 것이고, 외부 파일을 쓰면 단일 exe 배포가 깨진다.
- **Source**: AGENTS `## Stack`(단일 exe 배포), plan-feature 4-D 최소 의존 원칙

### D4. 현재 언어를 어떻게 읽는가
- **Options**: A) `static AtomicU8` 전역 / B) `Context`에 실어 인자로 전달 / C) `thread_local!`
- **Chosen**: A
- **Rationale**: B는 문구를 쓰는 함수 수백 곳의 시그니처가 바뀐다(오류 문구를 만드는 `remote` 워커까지). C는 워커 스레드가 자기 값을 따로 가져 UI와 어긋난다 — `remote`의 오류 문구가 다른 언어로 나온다. A는 잠금 없이 스레드 경계를 넘는다.
- **Source**: 전제 6, Investigation Log(`remote/*`가 오류 문구를 만든다)

### D5. `시스템 기본`을 언제 판정하는가
- **Options**: A) 설정이 `System`이면 **호출 시마다** 시스템 값을 반영한 결과를 `AtomicU8`에 이미 풀어 둔다(설정이 바뀔 때·시작할 때 1회 계산) / B) 문구를 만들 때마다 `GetUserDefaultUILanguage` 호출
- **Chosen**: A
- **Rationale**: B는 매 프레임 수백 번 Win32를 부른다. 시스템 UI 언어는 앱 실행 중에 바뀌지 않는다(바뀌면 로그아웃이 필요하다).
- **Source**: 전제 2

### D6. 이관 순서
- **Options**: A) 파일 그룹별로 나눠 순차 이관(T3→T7) / B) 화면별 / C) 한 번에 전부
- **Chosen**: A
- **Rationale**: C는 커밋 하나가 40파일을 건드려 리뷰가 불가능하다. B는 한 화면이 여러 파일에 걸쳐 있어(예: 원격 패널 = `panel.rs` + `remote_states.rs` + `tabs.rs`) 경계가 흐리다. A는 파일 단위라 "이 파일에 한글 리터럴이 남았는가"로 완료를 셀 수 있고, 배정표 합계가 266이라 누락이 드러난다.
- **Source**: 4-D 배정표

## Tasks

<!-- T1~T2 (기반) -->

- [x] **T1. i18n 카탈로그 기반과 언어 전환**
  - **Type**: D
  - **Design**: ① `src/i18n/mod.rs` 신규. **모듈 선언은 `src/lib.rs`에만 둔다** — 이 crate는 bin+lib 구성이고 `lib.rs:5-9`가 모든 모듈을 선언하며 `main.rs:11-15`는 `moa::`로 쓰기만 한다(`main.rs`에 `mod` 선언 0건). `main.rs`에도 `mod i18n;`을 두면 **같은 파일이 두 모듈로 컴파일되어 전역 `AtomicU8`이 둘이 되고**, main이 부른 `set_language`가 lib 쪽 화면에 반영되지 않는 무증상 결함이 된다. `main.rs`는 `moa::i18n::set_language`를 호출만 한다. `ui`·`remote`·`fs` 어디서나 부르므로 최상위에 둔다 — `i18n`은 **화면 계층(`ui`·`remote`·`fs`)을 참조하지 않는다**(단방향 유지 — 저장 값 타입 `app::settings::LanguageSetting`만 받는다). ② 신규 심볼 — `Language`(`Korean`/`English` — 실제로 쓰이는 값), `set_language(LanguageSetting)`(part1의 설정 값을 받아 `System`을 풀어 저장), `current() -> Language`, `system_language() -> Language`, `strings!` 매크로와 그것이 펼치는 정적 문구 함수들. ③ 설정을 읽는 `ui::app`이 `set_language`를 부르고, 문구를 쓰는 모든 모듈이 `i18n::*`를 부른다. ④ 이번에 추상화하지 않을 것: 언어별 리소스 로더·복수형 규칙 엔진·서식 지역화를 두지 않는다. 매크로도 조건·분기 없이 선언을 함수로 펼치는 것만 한다.
  - **Acceptance**: Given `strings!`에 `settings => "설정" / "Settings"`를 선언, When `set_language(Korean)` 후 `i18n::settings()`를 부르면 `"설정"`, `English`면 `"Settings"`를 반환한다. 한쪽 값을 빠뜨린 선언은 **컴파일되지 않는다**. `LanguageSetting::System`은 Windows UI 언어가 한국어면 `Korean`, 아니면 `English`로 풀린다. `set_language`를 부르지 않은 초기 상태의 기본값은 `Korean`이다(이관 중간에도 화면이 지금과 같게 보인다).
  - **Files**:
    - 주: `src/i18n/mod.rs`(신규), `src/lib.rs`(모듈 선언 — 여기에만), `src/main.rs`(시작 시 `moa::i18n::set_language` 호출)
    - 동반: (없음 — 설정 변경 시 `set_language` 배선은 드롭다운과 함께 T2가 넣는다)
    - 테스트: `src/i18n/mod.rs`(`mod tests` — 두 언어 반환값, `System` 판정, 초기 기본값)
  - **Edge Cases**: `GetUserDefaultUILanguage`가 0을 반환(실패) → `English`가 아니라 `Korean`으로 폴백(이 앱의 기존 화면이 한국어다) / 한국어 변종(`ko-KR` 외) → 하위 10비트가 `LANG_KOREAN`이면 전부 한국어 / `AtomicU8`에 알 수 없는 값 → `Korean`
  - **Halt Forecast**:
    - (i) "외부 crate를 쓸까" → D3에서 확정(직접 구현, 의존성 추가 없음)
    - (i) "전역 상태가 워커에서 안전한가" → 전제 6에서 확인
  - **Depends on**: part1 T1(`LanguageSetting` 정의)

- [ ] **T2. 설정 화면 `언어` 그룹과 즉시 반영**
  - **Type**: C
  - **Design**: ① `src/ui/settings_dialog.rs`(part1 T3의 산출물)에 `언어` 그룹을 더한다. ② 신규 심볼 없음 — `widgets::dropdown_field`를 재사용한다. ③ 고른 값을 `AppSettings.language`에 넣고 `i18n::set_language`를 부른 뒤 저장한다. ④ 이번에 추상화하지 않을 것: 언어 목록을 동적으로 만들지 않는다(세 항목 고정).
  - **Acceptance**: Given 설정 화면, When `언어` 드롭다운에서 `English`를 고르면, Then **다음 프레임에** 설정 화면 자신을 포함한 모든 문구가 영어로 바뀌고(전환 직후 `ctx.request_repaint()`로 그 프레임을 보장한다 — 전제 3-b) 값이 저장된다. `시스템 기본`을 고르면 Windows UI 언어에 따라 갈린다. 앱을 다시 켜도 고른 언어가 유지된다. 드롭다운 항목 이름은 현재 언어를 따른다(한국어일 때 `시스템 기본`·`한국어`·`English`, 영어일 때 `System default`·`Korean`·`English`).
  - **Files**:
    - 주: `src/ui/settings_dialog.rs`
    - 동반: `src/ui/app.rs`(설정 반영 경로)
    - 테스트: `src/ui/settings_dialog.rs`(`mod tests` — 선택 → `LanguageSetting` 매핑)
  - **Edge Cases**: 언어를 바꾸는 순간 드롭다운이 열려 있다 → 항목 문구가 바뀌어도 팝업이 닫히거나 선택이 튀지 않아야 한다(`dropdown_field`는 번호로 주고받으므로 안전) / `English` 이름은 두 언어에서 같다
  - **Halt Forecast**:
    - (i) "즉시 반영되는가" → 전제 1에서 확인
  - **Depends on**: T1, part1 T3

<!-- T3~T7 (문구 이관 — 배정표 순서) -->

- [ ] **T3. 문구 이관 ① 창 껍데기 (50개 + 설정 화면)**
  - **Type**: C
  - **Acceptance**: 배정표 T3 두 행의 파일들(**part1 신규 파일 포함**)에 UI 한글 문자열 리터럴이 **0개** 남는다(위젯 ID 예외는 아래 Edge Cases). 한국어로 두고 실행하면 타이틀바·메뉴·사이드바·탭·주소창·보기 모드·토스트·설정 화면·**트레이 메뉴와 툴팁**의 문구가 이관 전과 **글자 그대로 같다**. `English`로 바꾸면 전부 영어로 바뀐다. **배정표의 `part1 신규분` 칸이 실측 수치로 채워져 합계가 닫힌다**. `cargo test`가 통과한다.
  - **Files**:
    - 주: `src/ui/titlebar.rs`, `src/ui/menu.rs`, `src/ui/sidebar.rs`, `src/ui/view_mode.rs`, `src/ui/tabs.rs`, `src/ui/address_bar.rs`, `src/ui/site_dropdown.rs`, `src/ui/tree.rs`, `src/ui/toast.rs`
    - 주(part1 신규 파일 — 착수 시 실측해 배정표를 닫는다): `src/ui/settings_dialog.rs`, `src/ui/tray.rs`, `src/ui/widgets.rs`(`toggle_row`에 문구가 있으면), `src/app/fonts.rs`·`src/app/autostart.rs`·`src/app/single_instance.rs`(사용자 노출 문구가 있으면)
    - 동반: `src/i18n/mod.rs`(키 추가)
    - 테스트: 각 파일의 `mod tests` 중 한글 문구를 단언하는 것
  - **Edge Cases**: **`ui/toast.rs:80 Area::new(egui::Id::new("원격 알림"))`은 위젯 ID다 — 건드리지 않는다** / `ui/toast.rs:27 format!("{host} 등록됨 · 더블클릭하여 연결")`은 동적 조립이라 D2의 함수 형태로 / `ui/view_mode.rs:56-63`·`ui/list_details.rs`의 `match` 라벨은 `enum → i18n 함수` 매핑으로 / 단축키 표기(`F5`·`Ctrl+T`)는 번역 대상이 아니다 / **`ui/tray.rs`의 메뉴 문구는 Win32 `AppendMenuW`에 넘기는 UTF-16 문자열**이라 egui 라벨과 경로가 다르다 — `i18n` 함수가 준 `&str`을 그 자리에서 `HSTRING`으로 바꿔 넘긴다 / **트레이 메뉴는 언어를 바꾼 뒤 다시 열 때 새 문구가 나오면 된다**(메뉴는 열 때마다 `CreatePopupMenu`로 새로 만든다)
  - **Halt Forecast**:
    - (i) "위젯 ID를 어떻게 가리는가" → Investigation Log의 8곳 목록으로 확정
  - **Depends on**: T2

- [ ] **T4. 문구 이관 ② 목록·패널·상태 줄 (26개)**
  - **Type**: C
  - **Acceptance**: 배정표 T4 행의 파일들에 UI 한글 문자열 리터럴이 0개 남는다. 한국어 실행 시 문구가 이관 전과 같고, `English`로 전부 바뀐다. **`ui/status_bar.rs:64,73`의 조각 결합(`"{}건 대기"` + `" 남음"`)이 언어별 완성 문장으로 바뀌어** 영어에서 어순이 깨지지 않는다. `cargo test` 통과.
  - **Files**:
    - 주: `src/ui/panel.rs`, `src/ui/list_details.rs`, `src/ui/status_bar.rs`, `src/fs/create.rs`, `src/app/workspace.rs`, `src/panel/tabs.rs`
    - 동반: `src/i18n/mod.rs`
    - 테스트: 각 파일의 `mod tests`
  - **Edge Cases**: **`ui/panel.rs:1289 egui::Id::new("원격 메뉴")`는 위젯 ID — 건드리지 않는다** / `ui/panel.rs:843 format!("새 {kind}을(를) 만들지 못했습니다 — {error}")`은 `kind`(`"폴더"`/`"파일"`, `:657`·`:666`)까지 함께 옮겨야 한다 — 영어는 조사가 없으므로 `format!("Could not create the {kind} — {error}")` 형태 / `ui/panel.rs:1184 format!("폴더 {dirs} 파일 {files}")`은 어순이 갈린다 / `fs/create.rs`의 기본 이름(`"새 폴더"`·`"새 텍스트 문서.txt"`)은 **실제로 만들어지는 파일 이름**이다 — 언어에 따라 만들어지는 이름이 달라져도 되는지 판단이 필요하다: **번역한다**(사용자 화면 언어를 따르는 것이 자연스럽고, 이미 만들어진 파일 이름은 바뀌지 않는다)
  - **Halt Forecast**:
    - (i) "새로 만드는 파일 이름을 번역하는가" → 위 Edge Cases에서 확정(번역한다)
  - **Depends on**: T3

- [ ] **T5. 문구 이관 ③ 사이트 관리자·원격 메뉴·원격 상태 (87개)**
  - **Type**: C
  - **Acceptance**: 배정표 T5 행의 파일들에 UI 한글 문자열 리터럴이 0개 남는다(위젯 ID 5곳 제외). 한국어 실행 시 문구가 이관 전과 같고 `English`로 전부 바뀐다. **사이트 관리자를 열었다 닫았다 해도 대화 상태·이름 바꾸기 편집 상태가 초기화되지 않는다**(위젯 ID를 보존했다는 증거). `cargo test` 통과.
  - **Files**:
    - 주: `src/ui/site_manager.rs`, `src/ui/remote_menu.rs`, `src/ui/remote_states.rs`
    - 동반: `src/i18n/mod.rs`
    - 테스트: 각 파일의 `mod tests`(`site_manager.rs:1325` 이후 433줄)
  - **Edge Cases**: **위젯 ID 5곳을 건드리지 않는다** — `site_manager.rs:535 Modal::new(Id::new("사이트 관리자"))`, `:1266 .id_salt("사이트 이름 바꾸기")`, `remote_menu.rs:277 ("원격 이름 대화", title)`·`:322 "원격 권한 변경"`·`:391 "원격 삭제 확인"`, `remote_states.rs:429 "원격 호스트 키 확인"` / 접근 키 표기(`프로토콜(T)`·`호스트(H)` 등 FR-27의 알파벳)는 영어 문구에서도 유지하되 영어 라벨의 첫 글자와 어긋날 수 있다 — **원래 알파벳을 그대로 둔다**(키 배정이 바뀌면 사용자가 익힌 조작이 깨진다) / `remote_menu.rs:399 format!("{}개 항목을 서버에서 지웁니다.")`는 영어 복수형(`1 item`/`N items`) 분기가 필요하다 / `remote_menu.rs:318-319 GROUPS/BITS` 배열(권한 그룹 이름)
  - **Halt Forecast**:
    - (i) "접근 키 알파벳을 영어에 맞춰 바꾸는가" → 위 Edge Cases에서 확정(그대로 둔다)
  - **Depends on**: T4

- [ ] **T6. 문구 이관 ④ 앱 본체·전송 큐·도크 (38개)**
  - **Type**: C
  - **Acceptance**: 배정표 T6 행의 파일들에 UI 한글 문자열 리터럴이 0개 남는다. 한국어 실행 시 문구가 이관 전과 같고 `English`로 전부 바뀐다. `cargo test` 통과.
  - **Files**:
    - 주: `src/ui/app.rs`, `src/ui/queue_panel.rs`, `src/ui/dock.rs`, `src/remote/sites.rs`, `src/remote/transfer.rs`
    - 동반: `src/i18n/mod.rs`
    - 테스트: 각 파일의 `mod tests`
  - **Edge Cases**: `ui/app.rs:2200-2203`의 `OpKind` 라벨(`"새 폴더"`·`"삭제"`·`"이름 바꾸기"`·`"권한 바꾸기"`)이 `:2229 format!("{label} 실패 — {err}")`에 들어간다 — 조각과 틀을 함께 옮긴다 / `ui/app.rs:1958 format!("읽을 수 없는 폴더 {skipped}개는 건너뛰었습니다")` 복수형 / `ui/queue_panel.rs:47 HEADERS: [&str; 7]`은 배열이라 함수 7개를 부르는 형태로 바뀐다 / **`ui/app.rs:762 Modal::new(Id::new("workspace_remove_confirm"))`은 이미 영문 ID라 무관**
  - **Halt Forecast**:
    - (i) 없음 — T3~T5와 같은 성격
  - **Depends on**: T5

- [ ] **T7. 문구 이관 ⑤ 원격 계층 오류·로그 문구 (65개, 동적 조립 중심)**
  - **Type**: D
  - **Design**: ① 문구는 `src/i18n/mod.rs`의 손수 쓴 함수(D2), 호출은 `src/remote/*`. ② 신규 심볼 — `RemoteOp`(`Connect`/`Move`/`Rename`/… — 지금 `&str` 동사 조각으로 넘기는 것을 언어 중립 enum으로 바꾼다)와 그에 대응하는 `i18n` 함수들. ③ `remote`가 `i18n`을 참조한다(`i18n`은 아무것도 참조하지 않으므로 단방향 유지). ④ 이번에 추상화하지 않을 것: 오류 코드 사전·서버 응답 코드 번역표를 만들지 않는다 — 서버 원문은 그대로 싣는다.
  - **Acceptance**: 배정표 T7 행의 파일들에 UI 한글 문자열 리터럴이 0개 남는다. `impl Display for RemoteError`(`types.rs:382-404`)의 8개 문장이 언어별로 갈리고, 한국어 출력이 이관 전과 **글자 그대로 같다**(조사 `을(를)` 포함). `classify`가 `&str` 대신 `RemoteOp`를 받아 호출부가 전부 갱신된다. 영어로 두고 연결에 실패시키면 오류 문구가 영어로 나오고 **서버 응답 원문(`530 Login incorrect` 등)은 그대로 실린다**. `cargo test` 통과.
  - **Files**:
    - 주: `src/remote/sftp.rs`, `src/remote/ftp.rs`, `src/remote/connection.rs`, `src/remote/types.rs`, `src/remote/log.rs`
    - 동반: `src/i18n/mod.rs`, `src/remote/testing.rs`(가짜 서버가 문구를 단언하면)
    - 테스트: `src/remote/*`의 `mod tests`(문구 단언은 한국어 고정으로), 신규: 두 언어 각각에서 `RemoteError` 8종의 출력이 비어 있지 않은지
  - **Edge Cases**: `sftp.rs:380`의 `subject`(`:153 "서버의 시작 폴더 이름"`·`:163 "이 폴더의 파일 이름"`)가 조사와 함께 조립된다 — 조각까지 함께 옮긴다 / `sftp.rs:471`의 인증 방식 목록(`publickey,keyboard-interactive`)은 **SSH 프로토콜 식별자라 번역하지 않는다** / `connection.rs:574 "연결에 실패해 {}초 뒤 다시 시도합니다 — {err}"` 복수형 / `remote/log.rs:30-33`의 로그 종류 접두어(`상태:`·`명령:`·`응답:`·`오류:`)는 **로그 화면 표시용이라 번역 대상**이다(FR-40) / 비밀번호 마스킹 로직이 문구 변경으로 깨지지 않아야 한다(FR-40)
  - **Halt Forecast**:
    - (i) "동사 조각을 어떻게 다루는가" → D2·이 task Design에서 확정(`RemoteOp` enum)
    - (ii-a) `classify`의 시그니처 변경(`&str` → `RemoteOp`) → `## 사전 승인 항목`에 등록
  - **Depends on**: T6

<!-- T8 (완료 판정) -->

- [ ] **T8. 회귀 테스트·규약 기록·문서**
  - **Type**: C
  - **Design**: ① 테스트는 `src/i18n/mod.rs`의 `mod tests`(검사 대상이 i18n 규약이므로). ② 신규 심볼 — 소스를 훑어 미등록 한글 UI 리터럴을 찾는 테스트 함수 하나. ③ 파일을 읽기만 하고 아무것도 참조하지 않는다. ④ 이번에 추상화하지 않을 것: 범용 소스 린터를 만들지 않는다 — `is_icon_font` 테스트와 같은 크기의 단일 목적 검사다.
  - **Acceptance**: ① 소스 훑기 테스트가 `src/ui`·`src/remote`·`src/fs`·`src/panel/tabs.rs`·`src/app/workspace.rs`의 **주석·테스트 모듈·`i18n` 모듈·위젯 ID 예외 목록·레거시 Win32 파일**을 뺀 나머지에서 한글 문자열 리터럴을 찾으면 **실패**한다(현재 0건). ② 카탈로그의 모든 키가 한·영 값을 갖는다(매크로가 컴파일 타임에 보장하므로 이 항목은 컴파일 성공으로 갈음). ③ `AGENTS.md` `## Conventions`에 "화면 문구는 `i18n` 카탈로그를 거친다 — 소스에 직접 박지 않는다. 규약은 `i18n`의 소스 훑기 테스트가 지킨다"가 아이콘 규약과 같은 형식으로 적힌다. ④ `README.md`에 언어 전환 기능이 기재된다. ⑤ `cargo build`·`cargo test`·`cargo clippy --all-targets -- -D warnings`·`cargo fmt --check` 전부 통과.
  - **Files**:
    - 주: `src/i18n/mod.rs`, `AGENTS.md`, `README.md`
    - 테스트: `src/i18n/mod.rs`(`mod tests`)
  - **Edge Cases**: 예외 목록이 너무 넓어 검사가 무력해짐 → 예외는 **파일 단위가 아니라 위치(파일:라인 또는 정확한 리터럴) 단위**로 둔다(위젯 ID 8곳은 리터럴로 특정 가능) / **레거시 Win32 파일 5개만은 파일 단위 예외가 불가피하다**(문구가 수십 개라 리터럴로 열거할 수 없다) — 이 예외에는 `// 실행 경로에서 쓰이지 않는다: lib.rs가 선언하지만 main.rs가 부르지 않는 egui 이식 이전 구현` 근거 주석을 함께 달고, **그 파일이 다시 실행 경로에 들어오면 예외를 지우라**는 문장을 남긴다(그러지 않으면 부활 시 검사가 조용히 빈다) / 새 파일이 생기면 검사 대상에 자동으로 들어가야 한다(디렉터리를 훑고 화이트리스트를 쓰지 않는다) / 한글이 든 주석을 문자열로 오인 → 줄 단위로 `//` 이후를 잘라낸다. 여러 줄 주석(`/* */`)은 이 레포에서 쓰이지 않음을 먼저 확인한다
  - **Halt Forecast**:
    - (i) "완료를 어떻게 판정하는가" → 이 task의 소스 훑기 테스트가 장치다
    - (ii-b) 소스 훑기 테스트가 잔여 문구를 잡았는데 **그것이 이 plan의 배정표에 없는 파일**이면 → 배정표(266개 전수)가 틀렸다는 뜻이므로 범위를 다시 확인해야 한다(Halt)
  - **Depends on**: T7

## 사전 승인 항목 (일괄 승인 대상)

- **T1 — 신규 모듈 `src/i18n/mod.rs` 추가**와 모듈 등록
- **T7 — `classify`의 시그니처 변경** (`remote/sftp.rs`·`remote/ftp.rs`, `&str` 동사 조각 → `RemoteOp` enum): 호출부를 함께 고친다. 계획된 변경이며 언어 중립성을 위해 필요하다
- **T3~T7 — 파일별 문구 `const` 89개 삭제와 인라인 문자열 약 177개 교체**: 계획된 대량 수정이며 배정표(266개 전수)가 범위다. 되돌리기는 커밋 단위 revert
- **T7 — `impl Display for RemoteError` 출력이 언어에 따라 달라짐** (`remote/types.rs:382-404`): 화면·로그 전용 값이라 저장·비교에 쓰이지 않음을 T1에서 재확인한 뒤 진행

> 신규 외부 의존성은 **없다**(D3).

## 불가피한 Halt (위임 불가)

- commit 이후의 **push·master 병합·태그·릴리즈**
- **T8의 소스 훑기 테스트가 배정표에 없는 파일에서 잔여 문구를 잡는 경우** — 266개 전수 조사가 틀렸다는 뜻이라 범위 재확인이 필요하다. **단 part1이 만든 신규 파일(`ui/settings_dialog.rs`·`ui/tray.rs` 등)은 배정표 `T3 (part1 신규분)` 행에 이미 등재돼 있으므로 이 조항의 대상이 아니다** — 그 파일들은 T3에서 실측해 수치를 채운다
- 영어 문구가 길어 **고정 폭 UI를 넓혀야 하는데 그 변경이 디자인 규격(`docs/design/README.md`의 px 값)을 벗어나는 경우** — 디자인 기준 변경은 plan에 없던 결정

## Verification Strategy

- 빌드: `cargo build`
- 단위·통합 테스트: `cargo test` (완료 판정은 T8의 소스 훑기 테스트)
- Lint: `cargo clippy --all-targets -- -D warnings`
- 서식: `cargo fmt --check`
- 수동 검증 (T8에서 한 번에): ① 한국어로 두고 전 화면을 훑어 이관 전과 문구가 같은지 ② `English`로 바꿔 같은 화면을 훑어 **잘리거나 겹치는 곳**을 찾기(전제 8 — 사이트 관리자 3탭·전송 큐 열 머리글·상태 표시줄·연결 실패 화면이 폭이 빡빡한 곳이다) ③ `시스템 기본` 선택 시 판정 확인 ④ 원격 연결을 일부러 실패시켜 오류 문구가 두 언어로 나오는지 + 서버 응답 원문 보존 확인 ⑤ 사이트 관리자를 열고 닫으며 상태 초기화가 없는지(위젯 ID 보존)

## Phase Ledger

## Retry Ledger

## Progress Log

- T1 완료: `src/i18n/mod.rs` 신설. `strings!` 매크로가 `키 => "한국어" / "English"` 한 줄을 함수 하나로 펼치고, 현재 언어는 `AtomicU8` 하나로 든다.
  - 설계: 모듈 선언은 **`lib.rs`에만** 뒀다. `main.rs`에도 두면 같은 파일이 두 모듈로 컴파일돼 전역 언어가 둘이 되고, main이 부른 `set_language`가 화면에 반영되지 않는 무증상 결함이 된다.
  - **규약(quality 리뷰 M2)**: 전역 상태를 건드리는 시험은 값을 되돌리는 것만으로 부족하다 — `cargo test`가 여러 스레드로 돌리므로 **본문이 도는 동안 잠가야** 한다. 되돌리기만 하면 한 시험이 단언하는 찰나에 다른 시험이 값을 바꾼다. `Mutex` 가드를 `Restore`가 함께 든다.
  - 규약: 주석이 **아직 없는 것**을 가리키지 않게 한다. "아래 `mod dynamic`"이 T7에서야 생길 모듈을 현재형으로 가리켜 리뷰가 잡았다.
  - 정정: 모듈 doc의 "아무 모듈도 참조하지 않는다"는 실제와 달랐다(`app::settings::LanguageSetting`을 받는다). plan Design ①의 같은 표현도 함께 고쳤다.

## Next Steps

- 권장 다음 액션: part1 완료 후 `pjc:implement-task docs/plans/2026-08-13-app-settings-part2.md`로 T1부터 실행

## Open Questions

- [x] 언어 전환 범위 → **전면 영문화**(실행 경로 266개 + 동적 조립 37곳, 레거시·테스트 제외)
- [x] 카탈로그 형태 → **`macro_rules!`로 선언 → 함수 생성**(D1), 외부 crate 없음(D3)
- [x] 동적 조립 문구 → **인자를 받는 손수 쓴 함수**(D2)
- [x] 현재 언어 접근 → **`AtomicU8` 전역**(D4)
- [x] 즉시 반영 여부 → **즉시**(전제 1)
