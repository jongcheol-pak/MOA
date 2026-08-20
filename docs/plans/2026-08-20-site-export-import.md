# Plan: 사이트 목록 내보내기·가져오기 + 연결 메뉴 문구 변경

**PRD**: docs/prd.md

## 요구 이해

- **원문 요청**: "사이트 관리자 화면에 … '이름 바꾸기' 버튼 아래 라인에 '내보내기', '가져오기' 버튼 추가 / '내보내기' — 등록된 원격 사이트 목록과 항목별로 설정된 정보, 아이디, 패스워드까지 모든 정보를 저장.(패스워드는 암호화로 저장) / '가져오기' — '내보내기'로 생성된 파일을 선택하면 파일에 있는 목록 정보를 목록에 추가.(기존 항목에 동일한 호스트가 있는 경우 덮어쓰기 할건지 팝업으로 표시) / 연결 목록에서 + 버튼을 클릭하면 표시되는 메뉴의 '새 사이트 추가...' 문구를 '사이트 관리자'로 변경"
- **이해한 요구**: 사이트 관리자 좌측 열의 `이름 바꾸기(R)`·`삭제(D)`·`복제(I)` 줄 **아래에 새 줄**을 만들어 `내보내기`·`가져오기` 두 버튼을 둔다. 내보내기는 등록된 **모든** 사이트의 설정 전부(프로토콜·호스트·포트·암호화·로그온 유형·사용자·전송 모드·동시 연결 수·문자셋·사이드바 숨김 여부)와 비밀번호를 한 파일로 저장하되, 비밀번호는 **사용자가 정한 암호로 암호화**해 담아 다른 PC에서도 복원되게 한다. 가져오기는 그 파일을 골라 목록에 합치며, 이미 있는 사이트와 겹치면 덮어쓸지 팝업으로 묻는다. 함께 사이드바 `+` 메뉴의 마지막 항목 문구를 `사이트 관리자`로 바꾼다.
- **포함하지 않는 것으로 이해**: 사이트 관리자에서 **고른 사이트 하나만** 내보내는 기능은 만들지 않는다(요청이 "등록된 원격 사이트 목록 … 모든 정보"이므로 언제나 전체를 내보낸다). 호스트 키(`known_hosts.json`)·전송 큐·앱 설정은 내보내기 대상이 아니다.

## Goal

사이트 관리자에서 등록된 사이트 전부를 암호로 보호된 파일 하나로 내보내고, 그 파일을 다른 PC에서도 그대로 되불러올 수 있다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-59 (신설 — 사이트 목록 내보내기·가져오기) | Must | T1·T2·T3·T4·T5·T7 | ✅ 커버 |
| FR-28 (비밀번호 DPAPI 봉인) | Must | T7 (문면 보완 — 내보내기 파일은 암호 기반 봉투) | ✅ 커버 |
| FR-27 (사이트 관리자) | Must | T4 (좌측 버튼 줄 추가·문면 보완) | ✅ 커버 |
| 그 밖의 active Must FR (FR-1~FR-8·FR-13·FR-15~FR-17·FR-29~FR-32·FR-36·FR-37·FR-47~FR-53·FR-55) | Must | — | 이번 범위 외 (기구현) |

## Out of Scope

- 사이트 **하나만** 골라 내보내기 / 가져올 사이트를 목록에서 체크해 고르기 — 이번 요청은 목록 전체다.
- 내보내기 파일에 호스트 키·전송 큐·앱 설정·워크스페이스를 함께 담기.
- 다른 FTP 클라이언트(FileZilla `sitemanager.xml` 등)의 파일 가져오기.
- 이름 충돌만으로 덮어쓰기를 묻기 — 판정 기준은 접속 대상(D3)이고, 이름 겹침은 `SiteStore::insert`가 `(2)`를 붙여 자동으로 가른다(D14).

## Deferred / Follow-up

- 팝업 본문 폭이 대화마다 제각각이다(360·420·460·480·1080) — 이번에 대화가 셋 늘어 그 편차가 커진다. 대장 2026-08-15 항목이 그대로 유효하며, 신규 대화는 기존 값 중 하나를 골라 쓴다(새 폭을 만들지 않는다).
- 내보내기 진행 표시(사이트가 아주 많을 때의 진행률) — 실측상 수십 건은 즉시 끝나 이번엔 두지 않는다.

## Investigation Log

- **위키 참조**: `20_projects/personal/moa/feat-remote-sites.md` — 자격증명은 DPAPI 사용자 범위로 봉인하며 "설정 파일이 통째로 복사돼도 다른 계정·다른 PC에서는 풀리지 않는다"가 설계 전제다. 이번 내보내기는 그 전제를 우회하는 **별도 봉투**를 새로 만드는 일이다.
- **위키 참조**: `20_projects/personal/moa/decisions.md` [2026-08-04] 자격증명 보관 — DPAPI 채택, 평문 저장은 기각. "대가로 설정 파일을 복사해도 다른 PC·계정에서는 비밀번호가 안 풀린다"가 명시된 수용 사항. 내보내기 파일에 평문을 담는 길은 이 결정과 정면으로 어긋나므로 두지 않는다.
- **위키 참조**: `20_projects/personal/moa/conventions.md` — ⓐ `ExplorerApp`은 단위 시험에서 만들 수 없다(생성자가 `eframe::CreationContext`를 요구) → 판정·병합 로직을 `ExplorerApp` 밖 순수 함수로 내려야 시험으로 덮인다. ⓑ `i18n::LanguageGuard::lock`은 비재진입이라 한 시험 함수에서 두 언어를 잇달아 잠그면 스위트가 멎는다. ⓒ 함수를 기존 함수 사이에 끼워 넣을 때 앞뒤 빈 줄이 없으면 doc 주석이 새 함수에 딸려 붙는다.
- **Deferred 대장 조회**: `docs/plans/deferred.md` `## 대기` 제목 스캔(전 항목). 주제 매칭 1건(팝업 본문 폭 편차 — 위 Deferred에 반영). 전제 반증 스캔에서 이 계획의 전제를 부정하는 항목 없음. 잔량 100건 미만이고 등록일 최솟값이 2026-07-23(28일)이라 소진 batch 미착수.
- **DPAPI 봉인 범위**: `src/remote/secret.rs:1-8`·`:50-51` — `CryptProtectData`(사용자 범위)이며 "다른 사용자·다른 PC에서 만든 것이면 `None`". `SiteStore::password`(`sites.rs:140-148`)도 같은 주석. PRD FR-28(`docs/prd.md:51`)이 같은 내용을 요구로 못 박는다.
- **저장 대상 구조**: `SiteRecord`(`src/remote/types.rs:112-136`) 12개 필드 전부 `Serialize`/`Deserialize`. `SiteStore`(`src/remote/sites.rs:16-26`)는 `sites`·`hidden`·`next_id`를 갖고 비공개다 — 외부에서 목록을 만들려면 `add`/`insert`/`get_mut`/`hide`/`unhide`/`set_password` 공개 API를 거쳐야 한다.
- **windows crate 가용성 (실측)**: `windows-0.62.2` 레지스트리 소스에서 `IFileOpenDialog`·`IFileSaveDialog`·`FileOpenDialog`/`FileSaveDialog` CLSID(`UI/Shell/mod.rs:9015`·`9017`·`23086`·`23832`), `COMDLG_FILTERSPEC`(`UI/Shell/Common/mod.rs:3`), `BCryptDeriveKeyPBKDF2`·`BCryptOpenAlgorithmProvider`·`BCryptGenerateSymmetricKey`·`BCryptEncrypt`·`BCryptDecrypt`·`BCryptGenRandom`·`BCryptSetProperty`·`BCryptDestroyKey`·`BCryptCloseAlgorithmProvider`, `BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO`·`BCRYPT_CHAIN_MODE_GCM`·`BCRYPT_AES_ALGORITHM`·`BCRYPT_ALG_HANDLE_HMAC_FLAG` 전부 확인. 필요한 feature(`Win32_UI_Shell`·`Win32_UI_Shell_Common`·`Win32_System_Com`·`Win32_Security_Cryptography`)는 `Cargo.toml`에 **이미 켜져 있다** — 신규 크레이트도 feature 추가도 필요 없다.
- **파일 대화를 부르는 자리**: `src/ui/app.rs:1936-1943` — 셸 컨텍스트 메뉴를 "그리기가 **모두 끝난 뒤**" 띄운다. 사유 주석: "TrackPopupMenuEx가 자체 메시지 루프를 돌려 이벤트 루프를 재진입시키므로, 위젯 트리가 절반만 구성된 상태로 들어가면 안 된다". `IFileDialog::Show`도 자체 메시지 루프를 돌리므로 같은 자리를 쓴다.
- **HWND 확보**: `ShellHost::hwnd()`(`src/ui/shell_host.rs:47-49`) 공개 접근자가 있고 `ExplorerApp.shell: Option<ShellHost>`(`app.rs:453`)가 그것을 든다.
- **대화 계약**: `SiteManager::show`는 `outcome`이 `None`이 아니면 곧바로 `self.close(store)`를 부른다(`site_manager.rs:600-604`) — 즉 `SiteManagerOutcome`에 새 variant를 더하면 그 조작이 대화를 닫아 버린다. 내보내기·가져오기는 대화를 **열어 둔 채** 진행해야 하므로 outcome 열거형을 쓰지 않는다.
- **재사용 후보**: 확인 팝업 3종(`site_manager::show_delete_confirm`·`remote_menu::show_conflict_dialog`·`remote_menu::show_name_dialog`)이 모두 `dialog::show` + `ButtonSpec` + `Shell.clicked` 패턴이다. 새 대화도 같은 셸을 거친다(`ui::dialog` 소스 훑기 시험이 강제).
- **문구 사용처 전수**: `sidebar_add_site` — 정의 `src/i18n/mod.rs:196`, 호출 `src/ui/sidebar.rs:643`, 단언 `src/ui/sidebar.rs:847`. 그 문구를 **주석으로 언급**하는 곳 5개(`src/remote/types.rs:142`·`src/ui/app.rs:532`·`:889`·`src/ui/sidebar.rs:106`·`src/ui/site_manager.rs:411`).
- **소스 훑기 규약 시험 4종**: `i18n::화면_문구가_카탈로그를_거치지_않은_곳이_없다`(`src/ui`·`src/remote`·`src/fs` 재귀 — 위젯 Id 문자열은 `EXEMPT_LITERALS` 28건에 등재해야 통과), `ui::dialog::대화는_모두_이_모듈을_거친다`(재귀), `ui::widgets::화면_코드에_원본_아이콘_기호가_남아_있지_않다`(재귀), `ui::theme`의 메뉴 규약 2종. 이번 신규 파일 셋이 모두 그 훑기 범위에 들어온다.
- **치수 시험 영향 없음**: `site_manager::대화_치수는_원본과_같다`(`:1403-1419`)는 상수 값만 단언하고 목록 웰 높이(계산식)는 보지 않는다 — 버튼 줄이 하나 늘어도 이 시험은 깨지지 않는다.
- **파일 I/O 위치 선례**: `app::settings::save_session`/`load_session`(`src/app/settings.rs:484`·`:498`)이 `%APPDATA%` JSON을 다루며 UI 스레드(`persist_session`)에서 불린다 — 사용자 조작 직후의 작은 파일 I/O는 워커로 내리지 않는 것이 이 레포의 기존 방식이다.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| P1 | DPAPI 봉인 바이트는 다른 PC·다른 계정에서 풀리지 않는다 — 그대로 내보내면 비밀번호 이전이 성립하지 않는다 | `remote/secret.rs:1-8`·`:50-51`, `remote/sites.rs:140-148`, `docs/prd.md:51`, 위키 decisions [2026-08-04] | ✅ |
| P2 | `windows` crate에 CNG(PBKDF2·AES-GCM)와 IFileDialog가 있고 필요한 feature가 이미 켜져 있다 | Investigation Log "windows crate 가용성 (실측)" 행 | ✅ |
| P3 | `SiteRecord`의 12개 필드가 전부 serde로 왕복한다 | `remote/types.rs:112-136` (`#[derive(Serialize, Deserialize)]`) | ✅ |
| P4 | `SiteManagerOutcome`에 새 variant를 더하면 그 조작이 대화를 닫는다 → outcome 경로를 쓸 수 없다 | `ui/site_manager.rs:600-604` | ✅ |
| P5 | 파일 대화는 자체 메시지 루프를 돌리므로 egui 그리기 도중에 띄우면 안 된다 | `ui/app.rs:1936-1943`의 셸 메뉴 선례와 그 사유 주석 | ✅ |
| P6 | `ExplorerApp`은 단위 시험에서 만들 수 없다 → 병합·충돌 판정은 순수 함수여야 시험으로 덮인다 | 위키 conventions [2026-08-15], `ui/app.rs`의 생성자가 `eframe::CreationContext` 요구 | ✅ |
| P7 | 좌측 버튼 줄을 하나 더해도 기존 치수 시험이 깨지지 않는다 | `ui/site_manager.rs:1403-1419` (상수만 단언) | ✅ |
| P8 | 새 위젯 Id·화면 문구는 i18n 소스 훑기 시험에 걸린다 | `src/i18n/mod.rs:1069-1110` (ROOTS 5·EXEMPT_LITERALS 28) | ✅ |
| P9 | `SiteId`를 유지한 채 덮어써야 세션·탭·전송 큐의 참조가 끊기지 않는다 | `remote/types.rs:104-108` (`SiteId` 주석 — 이름으로 잡지 않는 이유) | ✅ |
| P10 | AES-GCM 인증 태그는 암호가 틀리거나 내용이 변조되면 복호를 실패시킨다 | CNG `BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO`의 `pbTag` 계약 (`Security/Cryptography/mod.rs:3143`) — 동작 확인은 T1 시험이 실측한다 | ✅ (실측은 T1) |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| CNG unsafe FFI 신규 — 알고리즘 핸들·키 핸들·키 오브젝트 버퍼 누수 | 내보내기를 반복할수록 메모리가 샌다 | 핸들을 든 구조체에 `Drop`을 붙여 `BCryptDestroyKey`·`BCryptCloseAlgorithmProvider`를 반드시 부른다. 시험에서 1,000회 왕복해 누수 없이 끝나는지 본다 |
| 평문 비밀번호가 내보내기 도중 메모리에 모인다 | 덤프에 평문이 남을 수 있다 | 모은 평문 버퍼는 암호화 직후 `secret::zeroize`와 같은 volatile 소거로 지운다(그 함수를 `pub(crate)`로 올려 재사용) |
| 파일 대화를 프레임 도중에 띄우면 이벤트 루프가 재진입한다 | 위젯 트리가 절반만 구성된 상태로 들어가 화면이 깨진다 | P5대로 `update` 말미(셸 메뉴 호출 자리 옆)에서만 띄운다 |
| 덮어쓰기가 사이트 식별자를 새로 발급하면 열려 있던 원격 탭·전송 큐의 참조가 끊긴다 | 사용자가 보던 탭이 "연결 없음"이 된다 | P9대로 **기존 `SiteId`를 유지**한다. 그 일을 하는 API는 `SiteStore::insert`다(D14) — `get_mut`은 이름 유일화를 하지 않아 덮어쓴 이름이 다른 사이트와 그대로 겹친다 |
| 키 파생(PBKDF2 600,000회)이 UI 스레드를 1초 안팎 멈춘다 | 내보내기·가져오기를 누른 직후 창이 잠깐 멎은 것처럼 보인다 | D13대로 그 자리를 UI 스레드에 두되 T1이 릴리즈 빌드에서 실측해 1.0초를 넘으면 반복을 200,000회로 낮춘다. 앞뒤가 모두 모달 대화라 그 구간에는 사용자 조작이 없다 |
| 암호를 잊으면 그 파일은 영영 열 수 없다 | 사용자가 백업을 잃는다 | 암호 입력 대화에 그 사실을 문구로 못 박는다(T4). 복구 경로는 두지 않는다 |
| 사용자가 암호를 비운 채 내보내 비밀번호가 빠진 것을 모른다 | 다른 PC에서 전부 다시 입력해야 한다 | 암호가 비었으면 **확인을 한 번 더 받는다**(D6) |
| DPAPI 봉인·해제가 실패해 비밀번호만 조용히 빠진다 | "가져왔습니다"로 보고됐는데 연결할 때야 비밀번호가 없는 것을 안다 | `SiteStore::password`가 `None`인 사이트와 `set_password`가 `false`를 준 사이트를 세어 결과 문구에 함께 알린다(D15) |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `i18n::sidebar_add_site` (값 변경 + 개명 → `sidebar_site_manager`) | `src/i18n/mod.rs:196`(정의) · `src/ui/sidebar.rs:643`(호출) · `src/ui/sidebar.rs:847`(단언) | 값·이름 변경 — 호출부 전수 2곳 |
| `새 사이트 추가…`를 언급하는 주석 | `src/remote/types.rs:142` · `src/ui/app.rs:532` · `src/ui/app.rs:889` · `src/ui/sidebar.rs:106` · `src/ui/site_manager.rs:411` | 문구 변경으로 어긋나는 서술 — 함께 갱신 |
| `SiteManager` (필드·메서드 추가) | `src/ui/site_manager.rs` · `src/ui/app.rs:1473-1501`(유일 호출부) | 신규 공개 메서드 3개(`take_file_request`·`supply_file`·`take_notice`)와 공개 타입 `FileRequest` 추가. 기존 `show` 시그니처는 그대로 |
| `SiteStore` (공개 API 사용만) | `src/remote/sites.rs` | 변경 없음 — `add`·`get_mut`·`set_password`·`hide`·`unhide`·`sites`를 그대로 쓴다 |
| `secret::zeroize` (비공개 → `pub(crate)`) | `src/remote/secret.rs:89` · 신규 `src/remote/envelope.rs` | 가시성 확대 1곳 |
| `i18n` 카탈로그 신규 키 (약 21건(추정 — T4가 확정) · 내역: T3 필터·기본 이름 2건, T4 대화 문구 약 18건, T5 사유 문구 1건) | `src/i18n/mod.rs` (`strings!`·`dynamic`) | 추가만 — 기존 키 불변 |
| `EXEMPT_LITERALS` (리터럴 7건 추가 — 대화 Id 4 + 입력칸 salt 3) | `src/i18n/mod.rs:1102` | 배열 길이 상수 28 → 35 |

### 4-B. 계약·직렬화 변경

- **신규 파일 형식** `.moasites`(JSON, `"format": "moa-sites"`·`"version": 1`) — 앱 밖으로 나가는 형식이라 한 번 나가면 되돌릴 수 없다. 알 수 없는 `format`·`version`은 **거부**하고 사유를 보인다(조용히 부분 해석하지 않는다).
- **기존 `settings.json` 스키마는 건드리지 않는다** — `SiteStore`의 필드·`SiteRecord`의 필드 모두 불변이다. 따라서 세션 승격(`promote_v2`)·저장 경로에 영향이 없다.

### 4-C. 테스트 파일

- 신규: `src/remote/envelope.rs`의 `#[cfg(test)] mod tests` · `src/remote/site_export.rs`의 `#[cfg(test)] mod tests`
- 갱신: `src/ui/sidebar.rs:840-851`(`연결_섹션_문구는_인벤토리_원문_그대로다` — 문구 단언) · `src/ui/site_manager.rs`의 `문구는_인벤토리_원문_그대로다`(신규 버튼 두 개 문구 추가)
- 영향 확인만: `src/i18n/mod.rs`의 소스 훑기 시험 · `src/ui/dialog.rs`의 `대화는_모두_이_모듈을_거친다` · `src/ui/widgets.rs`의 아이콘 리터럴 시험 — 신규 파일 셋이 훑기 범위에 들어오므로 통과 여부를 T1·T3·T4에서 확인한다
- `src/ui/panel/tests.rs`·`tests/remote_concurrency.rs`: 영향 없음(`SiteStore` 계약 불변)

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `remote::envelope::{seal_with_passphrase, open_with_passphrase}` | `remote::secret::{seal, unseal}`(DPAPI). grep `CryptProtect|BCrypt` → `secret.rs` 한 곳뿐 | **신규** — DPAPI는 원리상 기계·계정에 묶여 다른 PC로 옮길 수 없다(P1). 같은 파일에 두지 않는 것은 두 봉투의 키 출처·수명이 완전히 다르기 때문이다 |
| `remote::envelope::{to_hex, from_hex}` | grep `hex|base64` → 0건 | **신규** — 표준 라이브러리에 없고 바이트→문자 변환 20줄이면 끝나므로 크레이트를 들이지 않는다(AGENTS 최소 의존) |
| `remote::site_export::{SiteExport, ExportedSite, build, plan_import, apply_import, conflict_key}` | grep `export|import` → 0건 | **신규** — 같은 일을 하는 구현이 없다 |
| `fs::file_dialog::{pick_save, pick_open}` | grep `IFileDialog|GetOpenFileName|SHBrowseForFolder|rfd` → 0건 | **신규** — 이 앱에 파일 선택 대화가 아직 없다. `windows` crate가 이미 있어 패키지를 더하지 않는다 |
| 사이트 관리자 좌측 버튼 2개 | `widgets::design_button` + `site_manager::show_list_buttons`의 3열 그리드 | **재사용** — 기존 버튼 그리는 코드를 그대로 쓰고 줄만 하나 더한다 |
| 암호 입력 대화 · 가져오기 충돌 대화 | `dialog::show` + `ButtonSpec` + `Shell.clicked`(확인 팝업 3종의 공통 형태), `widgets::text_field(masked)` | **재사용** — 공통 셸과 마스킹 입력 필드를 그대로 쓴다. 본문 폭도 기존 값(360·460) 중에서 고른다 |
| 결과 알림 | `ui::toast`(`ExplorerApp.toast`) | **재사용** — 등록 토스트와 같은 통로 |

### Verified by

- `grep -rn "sidebar_add_site" src/` → 3 hits(정의 1·호출 1·단언 1), 모두 위 표에 포함
- `grep -rn "새 사이트 추가" src/` → 7 hits(카탈로그 값 1·주석 5·단언 1), 모두 위 표에 포함
- `grep -rn "SiteManagerOutcome" src/` → `site_manager.rs`(정의·생성)와 `app.rs:1473-1501`(유일 소비처). 호출부 1곳
- `grep -rn "IFileOpenDialog\|IFileSaveDialog\|GetOpenFileName\|FileDialog\|rfd::" src/` → 0 hits (파일 대화 없음 확인)
- `grep -rn "hex\|base64" src/` → 0 hits (인코더 없음 확인)
- `grep -rn "SiteStore" src/ -l` → 11개 파일. 이번 변경은 `SiteStore`의 **공개 API를 쓰기만** 하고 타입·필드·시그니처를 바꾸지 않으므로 그 11곳은 영향 없음(전건 확인: 모두 `sites()`·`get()`·`visible()` 등 읽기 또는 기존 변경 API 호출)

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md`에 FR-59 신설 + FR-27·FR-28 문면 보완 | 이 레포는 PRD가 요구 정본이고 Must 기능이 늘면 그쪽이 먼저 어긋난다. 사용자가 반영을 택했다 | T7에 편입 |
| 필수 | `README.md` §핵심 기능의 사이트 관리자 서술 | 현재 "다른 PC로 설정을 옮기면 다시 입력받습니다"라고 적혀 있어, 내보내기로 옮길 수 있게 되면 그 문장이 틀린 서술이 된다 | T7에 편입 |
| 필수 | `AGENTS.md`의 「데이터 접근 — 비밀번호」 항목 | "`%APPDATA%` 파일에 DPAPI로 봉인해서만 담는다"가 정본인데 암호 기반 봉투가 두 번째 통로로 생긴다 | T7에 편입 |
| 무관 | `docs/design/design-files/FileExplorer-FTP.dc.html:378` · `docs/design/README.md:119`의 `새 사이트 추가…` | 디자인 **원본**이라 그때 무엇이었는지를 기록하는 문서다. 앱 문구를 바꿨다고 원본을 고치면 이후 대조의 기준이 사라진다 | 건드리지 않음 |
| 무관 | `docs/plans/2026-08-04-ftp-integration.md`의 인벤토리 #8 | 지난 회차의 실행 기록이다 | 건드리지 않음 |

> 이번 변경이 **유발한** 어긋남(문구 변경으로 낡아지는 주석 5곳·시험 단언 2곳, 신규 위젯 Id의 `EXEMPT_LITERALS` 등재)은 3분류 대상이 아니라 각 task에서 함께 고친다(T4·T6).

## Decisions

### D1. 내보내기 파일의 비밀번호 보호 방식
- **Options**: A) 사용자 암호로 재암호화 / B) DPAPI 봉인 바이트를 그대로 담기 / C) 비밀번호 제외 / D) 내보낼 때 사용자가 고르게
- **Chosen**: A (사용자 선택)
- **Rationale**: 내보내기의 쓰임이 다른 PC로의 이전인데 B·C는 그 자리에서 비밀번호를 잃는다. Windows CNG로 직접 구현하므로 신규 패키지가 없다.
- **Source**: 사용자 답변 2026-08-20 · P1(DPAPI 범위) · P2(CNG 가용성)

### D2. 암호화 파라미터
- **Options**: A) PBKDF2-HMAC-SHA256 600,000회 + AES-256-GCM / B) 더 적은 반복 / C) 다른 KDF(scrypt·Argon2 — CNG에 없어 크레이트 필요)
- **Chosen**: A
- **Rationale**: CNG가 그대로 제공하는 조합이라 패키지가 늘지 않는다. 반복 600,000회는 OWASP가 PBKDF2-SHA256에 권하는 값이고, 내보내기·가져오기는 사용자가 명시적으로 누르는 드문 조작이라 1초 안팎의 지연을 감당한다. 소금 16바이트·nonce 12바이트·태그 16바이트는 `BCryptGenRandom`으로 매번 새로 뽑는다.
- **Source**: `windows-0.62.2` CNG 심볼 실측(P2)
- **주의**: 반복 횟수는 **D13의 상한 규칙에 걸린다** — T1이 릴리즈 빌드에서 실측해 1회 파생이 1.0초를 넘으면 200,000회로 낮춘다.

### D3. 덮어쓰기 판정 키
- **Options**: A) 호스트만 / B) 호스트+포트+프로토콜+사용자 / C) 호스트+포트 / D) 사이트 이름
- **Chosen**: B (사용자 선택)
- **Rationale**: 같은 서버에 계정·포트를 달리해 여러 항목을 둔 경우 A·C는 서로를 덮어쓴다. 키는 `(호스트 소문자·공백 제거, 포트, 프로토콜, effective_user)`이며 호스트만 대소문자를 무시한다(DNS는 대소문자를 가리지 않고 사용자 이름은 서버가 가릴 수 있다). 익명 로그온은 `effective_user()`가 `anonymous`를 주므로 일반 로그온의 같은 이름과 자연히 갈린다.
- **Source**: 사용자 답변 2026-08-20 · `remote/types.rs:159-164`(`effective_user`)

### D4. 겹치는 사이트를 묻는 방식
- **Options**: A) 겹치는 것 전부에 한 번에(덮어쓰기/건너뛰기/취소) / B) 사이트마다 개별 / C) 체크박스 목록
- **Chosen**: A (사용자 선택)
- **Rationale**: 파일 전송의 같은 이름 확인(`remote_menu::show_conflict_dialog`, FR-55)과 같은 규칙·같은 모양이라 사용자가 새로 읽을 것이 없다. 겹치는 목록은 앞의 5건만 미리 보이고 나머지는 `…`로 접는다(그 대화와 같은 방식).
- **Source**: 사용자 답변 2026-08-20 · `ui/remote_menu.rs:445-500`

### D5. 파일 형식과 확장자
- **Options**: A) `.moasites`(내용은 JSON) / B) `.json` / C) 통째로 암호화한 바이너리
- **Chosen**: A (사용자 선택)
- **Rationale**: 파일 대화 필터가 하나로 좁혀져 엉뚱한 JSON을 고르기 어렵다. 내용이 JSON이라 사람이 열어 무엇이 담겼는지 확인할 수 있고, 그 안에서 비밀번호만 봉투로 감싼다. 이진 바이트는 hex 문자열로 담는다(D9).
- **Source**: 사용자 답변 2026-08-20

### D6. 암호를 비운 채 내보내기
- **Options**: A) 암호 필수 / B) 비우면 비밀번호 제외 / C) 비우면 확인을 한 번 더 받고 제외
- **Chosen**: C (사용자가 B를 선택 — 그 대가인 "깜빡한 빈칸과 일부러 비운 것을 구분할 수 없다"를 확인 한 번으로 막는다)
- **Rationale**: B의 유연함은 그대로 두면서, 사용자가 의도치 않게 비밀번호 없는 파일을 만드는 경우만 걸러 낸다. 암호를 적었으면 확인 칸과 일치해야 진행한다(오타로 영영 못 여는 파일을 막는다).
- **Source**: 사용자 답변 2026-08-20

### D7. 내보내기·가져오기를 사이트 관리자 밖으로 알리는 통로
- **Options**: A) `SiteManagerOutcome`에 variant 추가 / B) `SiteManager`가 파일 요청을 내부 상태로 들고 앱이 꺼내 간다 / C) `SiteManager`가 파일 대화를 직접 부른다
- **Chosen**: B
- **Rationale**: A는 outcome이 `None`이 아니면 대화가 닫히는 기존 계약(P4)을 깨야 하고, C는 파일 대화가 이벤트 루프를 재진입시켜 그리기 도중에 부를 수 없다(P5). B는 두 계약을 모두 지킨다 — 대화는 `pending_file: Option<FileRequest>`를 세우고, 앱은 그리기를 마친 뒤 그것을 `take`해 Win32 대화를 띄운 다음 `supply_file(...)`로 결과를 돌려준다.
- **Source**: `ui/site_manager.rs:600-604` · `ui/app.rs:1936-1943`

### D8. 병합 로직을 두는 곳
- **Options**: A) `SiteManager`(ui) 안 / B) `remote::site_export`(순수 모듈) / C) `ExplorerApp` 안
- **Chosen**: B
- **Rationale**: `ExplorerApp`은 단위 시험에서 만들 수 없고(P6) `SiteManager`도 프레임을 그려야 검증되는 부분이 많다. 충돌 판정·병합·요약 집계를 `remote`의 순수 함수로 두면 시험이 그것을 직접 부른다. `remote`는 `ui`를 모르므로 계층 방향(AGENTS)도 지켜진다.
- **Source**: AGENTS 「Conventions — 아키텍처」 · 위키 conventions [2026-08-15]

### D9. 이진 바이트의 문자 표현
- **Options**: A) hex 직접 구현 / B) base64 직접 구현 / C) base64 크레이트 추가 / D) JSON 숫자 배열
- **Chosen**: A
- **Rationale**: 봉투에 담기는 바이트는 소금 16 + nonce 12 + 태그 16 + 비밀번호 묶음(수백 바이트)뿐이라 hex의 2배 크기가 문제가 되지 않는다. 구현·검증이 base64보다 훨씬 짧고(변환표·패딩 없음) 크레이트를 더하지 않는다. D는 JSON이 장황해지고 사람이 읽기도 나쁘다.
- **Source**: AGENTS 「최소 의존」 · `grep hex|base64 src/` → 0건

### D10. 좌측 버튼 두 줄의 배치
- **Options**: A) 아랫줄을 2등분 / B) 윗줄과 같은 3등분 중 앞 두 칸
- **Chosen**: A (사용자 선택)
- **Rationale**: 두 줄의 좌우 끝이 맞아떨어져 한 덩어리로 보인다. 좌우 여백(`GRID_PAD_X` 30px)·줄 간격(`GRID_GAP` 8px)·버튼 높이(`GRID_BUTTON_HEIGHT` 28px)는 윗줄 값을 그대로 쓰고, 목록 웰은 늘어난 줄(28 + 8 = 36px)만큼 짧아진다.
- **Source**: 사용자 답변 2026-08-20 · `ui/site_manager.rs:65-69`·`:785-788`

### D11. 카탈로그 키 개명 (`sidebar_add_site` → `sidebar_site_manager`)
- **Options**: A) 키 이름을 값에 맞춰 바꾼다 / B) 키는 두고 값만 바꾼다 / C) 기존 `site_title` 키를 재사용한다
- **Chosen**: A
- **Rationale**: `sidebar_add_site`는 "추가"라는 뜻을 이름에 담고 있어 값이 `사이트 관리자`가 되면 이름이 거짓이 된다. C는 대화 제목과 메뉴 항목이 한 키를 나눠 쓰게 되어, 나중에 한쪽만 바꾸려 할 때 다시 갈라야 한다. 호출부가 2곳뿐이라 개명 비용이 작다.
- **Source**: `grep -rn "sidebar_add_site" src/` → 3 hits

### D12. 가져오기가 사이드바 숨김 상태를 어떻게 다루는가
- **Options**: A) 파일에 담긴 숨김 상태를 그대로 적용 / B) 가져온 것은 모두 보이게 / C) 가져온 것은 모두 숨기게
- **Chosen**: A
- **Rationale**: 요청이 "항목별로 설정된 정보 … 모든 정보"이고 숨김 여부도 그 사이트의 설정이다. 주소창으로 한 번 열어 숨겨 둔 사이트가 다른 PC에서 갑자기 사이드바에 나타나지 않는다.
- **Source**: `remote/sites.rs:20-22`·`:110-119`

### D13. 봉투 연산과 파일 I/O를 UI 스레드에서 수행한다
- **Options**: A) UI 스레드에서 그대로 / B) 워커 스레드 + 채널로 내린다 / C) 반복 횟수를 낮춰 지연을 줄인다
- **Chosen**: A + 상한 규칙
- **Rationale**: AGENTS의 「UI 스레드 블로킹 금지」가 겨냥하는 것은 **매 프레임 도는 렌더·탐색 경로**다(디렉터리 열거·감시). 내보내기·가져오기는 사용자가 직접 누르는 드문 조작이고, 그 직전·직후가 모두 모달 대화라 그 구간에는 사용자가 앱과 상호작용하지 않는다. B는 상태 기계에 비동기 홉이 하나 더 생겨(요청→채널→다음 프레임 반영) 이번 작업에서 가장 복잡한 부분을 한 번 더 복잡하게 만드는데, 얻는 것은 1초 미만의 정지를 없애는 것뿐이다. **다만 정지 시간에 상한을 둔다** — T1이 PBKDF2 1회 파생 시간을 실측해 릴리즈 빌드에서 **1.0초를 넘으면 반복을 200,000회로 낮추고** 그 실측값과 사유를 코드 주석에 적는다(추측으로 고정하지 않는다).
- **Source**: `AGENTS.md`의 UI 스레드 원칙과 DO NOT 항목 · `app/settings.rs:484`(세션 저장이 UI 스레드에서 도는 선례)

### D14. 덮어쓰기에 쓸 `SiteStore` API
- **Options**: A) `get_mut`으로 필드를 갈아 끼운다 / B) `insert(기존 SiteId를 담은 레코드)` / C) `remove` 후 `add`
- **Chosen**: B
- **Rationale**: C는 `SiteId`가 새로 발급돼 열려 있던 원격 탭·전송 큐의 참조가 끊긴다(P9). A는 식별자는 지키지만 **이름 유일화를 하지 않아**(`sites.rs:45-47`은 `iter_mut().find()`뿐이다) 덮어쓴 이름이 다른 사이트와 그대로 겹친다. `insert`(`sites.rs:64-72`)만이 `unique_name(name, Some(record.id))`로 이름을 가르면서 같은 식별자의 항목을 그 자리에서 교체한다. 비밀번호는 레코드에 직접 담지 않고 `insert` 직후 `set_password(id, 평문)`으로 봉인한다(봉인 경로가 그것 하나뿐이다).
- **Source**: `src/remote/sites.rs:45-47`·`:64-72`·`:125-137`

### D15. 봉인·해제 실패를 어떻게 알리는가
- **Options**: A) 무시하고 성공으로 보고 / B) 통째로 실패 처리 / C) 나머지는 반영하고 실패한 건수를 함께 알린다
- **Chosen**: C
- **Rationale**: `SiteStore::set_password`는 봉인 실패 시 `false`를 주고 **호출부가 알릴 의무**를 주석에 못 박고 있으며(`sites.rs:125-137`), `password`도 다른 계정에서 온 설정이면 `None`이다(`:140-147`). A는 "가져왔습니다"라고 해 놓고 연결할 때야 비밀번호가 없는 것을 알게 한다. B는 사이트 20개 중 하나가 실패했다고 나머지 19개를 버린다. 그래서 내보내기는 `password()`가 `None`인 사이트를 비밀번호 없이 담고 그 건수를 돌려주며, 가져오기는 `set_password`가 실패한 건수를 `ImportSummary`에 담아 결과 문구가 함께 알린다.
- **Source**: `src/remote/sites.rs:125-137`·`:140-148`

### D16. 문서에 비밀번호가 없을 때 기존 비밀번호를 어떻게 하는가

- **Options**: A) 문서에 없으면 기존 것도 지운다(파일 내용으로 통째 교체) / B) 문서에 있을 때만 갈아 끼우고 없으면 기존 것을 지킨다
- **Chosen**: B
- **Rationale**: 「비밀번호를 빼고 내보냈다」(D6)는 것은 「비밀번호를 지우겠다」는 뜻이 아니다. A를 택하면 암호 없이 내보낸 파일을 같은 PC에서 되가져오는 것만으로 저장된 로그인 정보가 통째로 사라지는데, 사용자는 그것을 예상하지 않고 되돌릴 수도 없다(절대 규칙 10이 막는 데이터 손실과 같은 성질). 구현은 `write_record`가 기존 `password_sealed`를 옮겨 담고 `apply_password`가 문서에 값이 있을 때만 덮는 형태다.
- **Source**: 구현 중 확정(T2) — plan D14가 「필드만 갈아 끼운다」까지만 정해 이 갈래가 열려 있었다. `remote/sites.rs:125-137`(`set_password`)·시험 `문서에_없는_비밀번호는_있던_것을_지우지_않는다`

## Tasks

<!-- T1~T2 (순수 계층) → T3 (Win32) → T4~T5 (화면·배선) → T6~T7 (문구·문서) -->

- [x] T1. 암호 기반 봉투(`remote::envelope`)를 만든다 — CNG PBKDF2 + AES-256-GCM
  - **Type**: D
  - **Design**: ① 배치 — 신규 `src/remote/envelope.rs`, `remote/mod.rs`에 모듈 선언. ② 신규 심볼 — `Envelope`(직렬화되는 봉투: `kdf`·`iterations`·`salt`·`nonce`·`tag`·`ciphertext`, 이진 값은 hex 문자열), `seal_with_passphrase(plain: &[u8], passphrase: &str) -> Option<Envelope>`(암호로 봉한다), `open_with_passphrase(env: &Envelope, passphrase: &str) -> Option<Vec<u8>>`(풀지 못하면 `None` — 틀린 암호와 변조를 구분하지 않는다), `to_hex`/`from_hex`(바이트↔소문자 hex), 내부 `AlgHandle`·`KeyHandle`(각각 `Drop`에서 `BCryptCloseAlgorithmProvider`·`BCryptDestroyKey`). ③ 의존 — `windows` CNG와 `serde`만 참조하고 `ui`·`remote`의 다른 모듈을 모른다(`secret::zeroize`만 `pub(crate)`로 빌려 쓴다). ④ 비추상화 — 「키 저장소 트레이트」·「알고리즘 선택 열거형」을 두지 않는다(쓰는 조합이 하나뿐이라 `secret.rs`가 DPAPI 하나에 추상화를 두지 않은 것과 같은 판단이다).
  - **Acceptance**:
    - Given 임의의 평문과 암호, When 봉한 뒤 같은 암호로 열면, Then 원문 바이트가 그대로 돌아온다(빈 평문·1KB 평문·비ASCII 암호 포함)
    - Given 봉한 봉투, When **다른 암호**로 열면, Then `None`이며 패닉하지 않는다
    - Given 봉한 봉투, When `ciphertext` 또는 `tag`의 한 바이트를 뒤집어 열면, Then `None`이다
    - Given 같은 평문·같은 암호로 두 번 봉하면, Then `salt`와 `nonce`가 서로 다르고 `ciphertext`도 다르다
    - Given 봉투를 JSON으로 왕복시키면, Then 같은 값이 돌아오고 그 JSON에 평문이 부분 문자열로 존재하지 않는다
    - `to_hex`/`from_hex`가 왕복하고, 홀수 길이·비hex 문자는 `None`이다
    - Given 1,000회 연속 왕복, Then 모두 성공하고 시험이 끝난다(핸들 누수로 실패하지 않는다)
    - **D13 상한**: `cargo build --release`로 만든 실행 파일에서 PBKDF2 1회 파생 시간을 실측해 그 값을 코드 주석에 적는다. **1.0초를 넘으면 반복을 200,000회로 낮추고** 낮춘 사실과 실측값을 함께 적는다(UI 스레드가 그만큼 멈추므로 추측으로 고정하지 않는다)
    - `cargo clippy --all-targets -- -D warnings` 통과 — 모든 `unsafe` 블록에 사유 주석이 있다
  - **Files**:
    - 주: `src/remote/envelope.rs`(신규)
    - 동반: `src/remote/mod.rs`(모듈 선언) · `src/remote/secret.rs`(`zeroize`를 `pub(crate)`로)
    - 테스트: `src/remote/envelope.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 빈 평문 / 빈 암호(호출부가 걸러야 하지만 이 함수는 그대로 봉한다) / 1KB 넘는 평문 / 비ASCII·이모지 암호(UTF-8 바이트로 넘긴다)
    - CNG 호출이 실패(NTSTATUS < 0)하면 `None` — `unwrap`·`expect`로 패닉하지 않는다
    - `BCryptEncrypt`·`BCryptDecrypt`는 **한 번만 부른다** — GCM은 스트림 모드라 출력 길이가 입력과 같아 길이를 먼저 물을 것이 없다(길이 조회 왕복이 필요한 것은 블록 모드다). 대신 반환된 기록 길이가 잡아 둔 버퍼와 다르면 실패로 본다
    - hex 문자열에 대문자가 섞여 들어와도 읽는다(다른 도구가 만든 파일 대비)
  - **Halt Forecast**:
    - (i) GCM 태그를 `BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO`로 주고받는 구체 절차가 막힘 → Investigation Log의 심볼 실측과 이 task의 Design이 쓸 함수를 이미 지목했다. 그래도 막히면 `BCryptEncrypt` 호출 규약을 MS 문서로 대조한다(외부 서비스 아님 — Halt 아님)
  - **Depends on**: -

- [x] T2. 내보내기 문서 모델과 병합 로직(`remote::site_export`)을 만든다
  - **Type**: D
  - **Design**: ① 배치 — 신규 `src/remote/site_export.rs`, `remote/mod.rs`에 선언. ② 신규 심볼 — `SiteExport { format, version, sites: Vec<ExportedSite>, secret: Option<Envelope> }`, `ExportedSite`(`SiteRecord`의 12개 필드에서 **`id`와 `password_sealed`를 함께 뺀 10개** + `hidden: bool`. **`password_sealed`를 빼는 것이 이 형식의 핵심 제약이다** — 그것을 담으면 같은 PC·같은 계정에서는 DPAPI가 그대로 풀려, 암호를 비워 "비밀번호 제외"로 내보낸 파일에서도 비밀번호가 복원된다(D6가 깨진다). 비밀번호가 문서로 나가는 통로는 `secret` 봉투 하나뿐이다), `build(store: &SiteStore, passphrase: &str) -> Result<ExportOutcome, ExportError>`(암호가 비면 `secret`을 `None`으로 두고 비밀번호를 담지 않는다. `ExportOutcome { document, password_unreadable }` — D15가 요구하는 「풀지 못한 비밀번호 수」를 문서와 함께 돌려준다), `write_file(path, &SiteExport) -> Result<(), ExportError>`, `read_file(path) -> Result<SiteExport, ImportError>`, `needs_passphrase(&SiteExport) -> bool`, `conflict_key(host, port, protocol, user) -> ConflictKey`, `plan_import(doc, store, passphrase) -> Result<ImportPlan, ImportError>`(`ImportPlan { fresh: Vec<PreparedSite>, conflicts: Vec<(String, SiteId, PreparedSite)> }`), `apply_import(store, plan, overwrite: bool) -> ImportSummary { added, replaced, skipped, password_failed }`. **보조 타입 정의** — `ConflictKey { host, port, protocol, user }`(D3의 네 값, 호스트만 소문자·trim), `PreparedSite { site: ExportedSite, password: Option<String> }`(문서의 한 항목과 그에 대응하는 평문 비밀번호를 묶은 것), `ExportError { Seal, Io(String), Serialize }`, `ImportError { Broken, Unsupported, WrongPassphrase, Io(String) }`. **비밀번호 대응 규칙** — 봉투의 평문은 `Vec<String>`을 JSON으로 직렬화한 것이고 **`sites` 배열과 같은 순서·같은 길이**다(사이트에 `id`가 없으므로 순서가 유일한 연결 고리다). 길이가 다르면 `ImportError::Broken`이다. ③ 의존 — `remote::{types, sites, envelope, secret}`과 `serde_json`만. `ui`를 모른다. ④ 비추상화 — 형식 버전이 하나뿐이므로 「마이그레이션 트레이트」를 두지 않고, `version != 1`이면 오류로 거부한다.
  - **Acceptance**:
    - Given 사이트 3개(일반·익명·숨김 각 하나)가 든 `SiteStore`와 암호, When `build` → `write_file` → `read_file` → `plan_import` → `apply_import(overwrite=true)`를 빈 저장소에 적용하면, Then **`id`·`password_sealed`를 뺀 10개 필드와 숨김 여부**가 원본과 같고, 비밀번호는 `SiteStore::password`로 푼 평문이 원본 평문과 같다(`id`는 새로 발급되므로 대조 대상이 아니다)
    - Given 암호가 빈 문자열, When `build`하면, Then `secret`이 `None`이고 **직렬화된 JSON에 `password_sealed` 키가 한 번도 나타나지 않으며**, 그 문서를 **같은 PC·같은 계정에서** 가져와도 비밀번호는 빈 채로 들어온다(나머지 설정은 그대로)
    - Given 파일에 담긴 봉투를 **틀린 암호**로 `plan_import`하면, Then `ImportError::WrongPassphrase`이며 저장소는 바뀌지 않는다
    - Given 저장소에 `ftp://example.test:21` 사용자 `deploy`가 이미 있고 같은 키의 사이트가 든 문서를 가져오면, Then `conflicts`에 1건이 잡히고 `fresh`는 0건이다. 포트·사용자·프로토콜 중 하나만 달라도 `fresh`로 간다
    - Given 충돌 1건 + 신규 1건, When `apply_import(overwrite=false)`, Then `added=1 replaced=0 skipped=1`이고 기존 사이트의 값이 그대로다. `overwrite=true`면 `added=1 replaced=1 skipped=0`이고 **기존 `SiteId`가 유지된** 채 값만 바뀐다
    - Given 덮어쓸 사이트의 새 이름이 **다른 기존 사이트의 이름과 겹치면**, When `apply_import(overwrite=true)`, Then 그 이름에 `(2)`가 붙고 식별자는 그대로다(D14 — `insert` 경로를 쓴다는 것이 이 단언으로 판정된다)
    - Given 비밀번호를 풀 수 없는 사이트(다른 계정에서 온 봉인)가 든 저장소, When `build`, Then 그 사이트는 비밀번호 없이 담기고 그 건수가 반환값에 담긴다. Given `set_password`가 실패하는 상황, When `apply_import`, Then `password_failed`가 그만큼 세어지고 **나머지 사이트는 정상 반영된다**(D15)
    - Given `format`이 다르거나 `version`이 1이 아닌 파일, When `read_file`, Then `ImportError::Unsupported`다. 깨진 JSON·빈 파일은 `ImportError::Broken`이다
    - Given 사이트가 0개인 저장소, When `build`, Then 사이트 0건 문서가 만들어지고 그것을 가져오면 `added=0`이다
    - 저장한 `.moasites` 텍스트에 평문 비밀번호가 부분 문자열로 존재하지 않고, `password_sealed` 키도 존재하지 않는다(암호를 넣은 경우·비운 경우 둘 다)
  - **Files**:
    - 주: `src/remote/site_export.rs`(신규)
    - 동반: `src/remote/mod.rs`
    - 테스트: `src/remote/site_export.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 사이트 0개 / 같은 키를 가진 항목이 파일 안에 둘 이상(뒤엣것이 이긴다 — 그 규칙을 주석과 시험에 못 박는다)
    - 호스트에 대소문자·앞뒤 공백이 섞인 경우(키 정규화로 같은 것으로 본다)
    - 봉투의 비밀번호 배열 길이가 사이트 수와 다른 파일(손으로 고친 파일) → `ImportError::Broken`
    - 파일 쓰기 실패(경로 권한·디스크 부족) → `ExportError::Io`로 사유를 올린다
    - DPAPI 해제 실패(`SiteStore::password`가 `None`) — 저장된 것이 없는 경우와 풀지 못한 경우를 구분하지 않고 둘 다 "비밀번호 없음"으로 담되, **풀지 못한 경우만 건수에 센다**(빈 비밀번호를 실패로 세면 익명 사이트마다 경고가 뜬다)
    - DPAPI 봉인 실패(`set_password`가 `false`) → `password_failed`를 올리고 그 사이트의 나머지 설정은 그대로 반영한다
    - 아주 긴 이름·비ASCII 이름 사이트
  - **Halt Forecast**:
    - (i) `SiteStore`가 비공개 필드라 목록을 직접 만들 수 없음 → `add`·`get_mut`·`set_password`·`hide`/`unhide` 공개 API로 채우는 것이 이 task의 방식이다(Investigation Log에 확인 완료)
  - **Depends on**: T1

- [x] T3. 파일 저장·열기 대화(`fs::file_dialog`)를 만든다
  - **Type**: C
  - **Design**: ① 배치 — 신규 `src/fs/file_dialog.rs`, `fs/mod.rs`에 선언(셸 API를 감싸는 다른 모듈들과 같은 자리다). ② 신규 심볼 — `pick_save(hwnd: HWND, suggested_name: &str) -> Option<PathBuf>`, `pick_open(hwnd: HWND) -> Option<PathBuf>`. 둘 다 `IFileSaveDialog`/`IFileOpenDialog`를 `CoCreateInstance`로 만들고 `.moasites` 필터 하나와 기본 확장자를 세운 뒤 `Show(hwnd)`를 부른다. 사용자가 취소하면 `None`. ③ 의존 — `windows` crate와 `i18n`(필터 문구)만. `ui`를 모른다. ④ 비추상화 — 「대화 빌더」·「필터 목록 타입」을 두지 않는다(필터가 하나뿐이다).
  - **Acceptance**:
    - `cargo build`·`cargo clippy --all-targets -- -D warnings` 통과, 모든 `unsafe`에 사유 주석
    - 필터 문구·기본 파일 이름이 `i18n` 카탈로그를 거친다(소스 훑기 시험 통과)
    - COM 인터페이스 참조가 스코프 종료 시 해제된다(`windows` crate의 `Drop`에 맡기고 수동 `Release`를 부르지 않는다)
    - 수동 검증: `내보내기`에서 저장 대화가 뜨고 기본 이름이 `MOA 사이트.moasites`이며 취소가 아무 일도 하지 않는다 / `가져오기`에서 열기 대화가 `.moasites`만 보인다
  - **Files**:
    - 주: `src/fs/file_dialog.rs`(신규)
    - 동반: `src/fs/mod.rs` · `src/i18n/mod.rs`(필터 문구·기본 파일 이름 키)
    - 테스트: 없음 — 실제 대화를 띄워야 해 자동 검증 대상이 아니다(AGENTS: UI 로직 비대상). 그 사실을 모듈 주석에 적는다
  - **Edge Cases**:
    - 사용자가 취소·`Esc`·창 닫기 → `None`(`HRESULT`가 `ERROR_CANCELLED`인 경우를 오류로 올리지 않는다)
    - 사용자가 확장자를 손수 지우고 저장 → `IFileDialog::SetDefaultExtension`이 `.moasites`를 붙인다
    - COM이 초기화되지 않은 스레드에서 불림 → UI 스레드는 `app.rs:110`에서 이미 STA로 초기화돼 있다. 그럼에도 `CoCreateInstance`가 실패하면 `None`으로 물러선다
    - HWND가 없는 환경(`ExplorerApp.shell`이 `None`) → 호출부(T5)가 소유자 없이 띄우지 않고 오류 문구를 보인다
  - **Halt Forecast**:
    - (i) `IFileDialog`의 Rust 바인딩 형태(`IShellItem`→경로 추출)가 막힘 → `GetResult` → `GetDisplayName(SIGDN_FILESYSPATH)` → `PWSTR` 해제까지가 이 task의 정해진 경로다. 그 절차는 `fs::shell_menu`가 이미 쓰는 PIDL·PWSTR 해제 방식과 같다
  - **Depends on**: -

- [x] T4. 사이트 관리자에 버튼 두 개와 대화 네 개를 더한다
  - **Type**: D
  - **Design**: ① 배치 — `src/ui/site_manager.rs` 안. ② 신규 심볼 — `FileRequest { Save { suggested: String }, Open }`(공개 — 앱이 받는다), `SiteManager::take_file_request() -> Option<FileRequest>`, `SiteManager::supply_file(&mut self, path: Option<PathBuf>, store: &mut SiteStore)`, `SiteManager::take_notice() -> Option<String>`(가져오기·내보내기 결과 문구를 앱이 토스트로 띄운다 — **봉인·해제 실패 건수가 0이 아니면 그 사실을 문구에 함께 담는다**, D15), 내부 `Exchange`(진행 상태: `Idle` / `ExportAsk { pass, confirm, error }` / `ExportConfirmEmpty { pass }` / `ExportWaitFile { pass }` / `ImportWaitFile` / `ImportAsk { doc, pass, error }` / `ImportConflict { plan }`), `show_export_ask`·`show_export_empty_confirm`·`show_import_ask`·`show_import_conflict`(넷 전부 `dialog::show`를 거친다 — `ExportConfirmEmpty` 상태가 D6의 되물음을 별도 대화로 요구하므로 셋이 아니라 넷이다), 좌측 아랫줄을 그리는 `show_export_buttons`. ③ 의존 — `remote::site_export`·`remote::sites`·`ui::dialog`·`ui::widgets`·`i18n`. 파일 대화(`fs::file_dialog`)는 **직접 부르지 않는다**(D7). ④ 비추상화 — 세 대화를 하나의 「단계 대화」 부품으로 합치지 않는다(본문 구성이 서로 달라 합치면 분기만 늘어난다).
  - **Acceptance**:
    - Given 사이트 관리자가 열려 있고 사이트가 1개 이상, When 좌측을 보면, Then 기존 세 버튼 아래 줄에 `내보내기`·`가져오기`가 좌우 균등 폭으로 서고 목록 웰이 그만큼(36px) 짧아진다
    - Given 사이트가 0개, When 좌측을 보면, Then `내보내기`는 비활성이고 `가져오기`는 활성이다(가져올 것은 있다)
    - Given `내보내기` 클릭, When 암호 대화에서 암호와 확인이 다르면, Then 대화 안에 사유가 뜨고 진행되지 않는다
    - Given 암호를 비운 채 `내보내기`, Then "비밀번호를 제외하고 내보냅니다" 확인을 한 번 더 받는다(D6)
    - Given 암호 확인을 마치면, Then `take_file_request()`가 `Save`를 한 번만 돌려주고 대화는 **열린 채로** 남는다
    - Given 앱이 `supply_file(Some(경로))`를 주면, Then 파일이 쓰이고 결과 문구가 `take_notice()`로 나온다. `None`(취소)이면 아무 일도 없이 `Idle`로 돌아간다
    - Given `가져오기`로 고른 파일에 봉투가 있으면, Then 암호 대화가 뜨고 틀린 암호는 대화 안에 사유를 남긴 채 다시 묻는다
    - Given 겹치는 사이트가 있으면, Then 그 이름 목록(앞 5건 + `…`)과 `덮어쓰기`·`건너뛰기`·`취소` 세 버튼이 뜨고, 각 선택이 T2의 요약대로 목록에 반영된다
    - Given `ImportSummary.password_failed`가 0이 아니면, Then 결과 문구가 "N개는 비밀번호를 저장하지 못했습니다"를 함께 알린다(D15)
    - Given 사이트 관리자를 닫으면, Then 진행 중이던 `Exchange` 상태와 입력한 암호가 함께 버려진다(`close`에서 지운다)
    - 화면 문구가 모두 `i18n` 카탈로그를 거치고, 새 리터럴 7건(대화 Id 4 + 입력칸 salt 3)이 `EXEMPT_LITERALS`에 등재돼 소스 훑기 시험이 통과한다
    - `ui::dialog::대화는_모두_이_모듈을_거친다` 통과(네 대화 모두 `dialog::show` 경유)
    - `site_manager::문구는_인벤토리_원문_그대로다`(`:1370`)에 신규 버튼 두 개의 문구 단언이 더해진다 — 그 시험은 새 문구를 넣지 않아도 통과하므로 acceptance로 못 박지 않으면 조용히 빠진다. 그 시험 주석(인벤토리 #60~75·#88~90)에 **신규 버튼 두 개는 원본 인벤토리에 없는 항목**이라는 사실과 사유(사용자 요청 2026-08-20)를 함께 적는다 — 적지 않으면 시험 이름("인벤토리 원문 그대로")과 내용이 어긋난 채 남는다
    - 시험: 상태 기계가 `Idle → ExportAsk → ExportWaitFile → Idle`, `Idle → ImportWaitFile → ImportAsk → ImportConflict → Idle`로 도는 것을 프레임 없이 직접 부르는 단위 시험으로 덮는다(대화 그리기는 기존 `대화가_한_프레임을_그린다` 방식으로 1프레임 확인)
  - **Files**:
    - 주: `src/ui/site_manager.rs` · `src/ui/site_manager/exchange.rs`(신규 — 구현 중 분할, 아래 사유)
    - 동반: `src/i18n/mod.rs`(문구 키 32건(실측) + `EXEMPT_LITERALS` 7건) · `src/ui/toast.rs`(범위 밖 1줄 — 아래 사유)
    - 테스트: `src/ui/site_manager/exchange.rs`의 `#[cfg(test)] mod tests`(교환 시험 10건) · `src/ui/site_manager.rs`의 기존 시험
  - **구현 중 판정 2건**:
    - **파일 분할** — 이 흐름을 더하자 `site_manager.rs`가 2,699줄이 됐고 AGENTS 「파일」 네 질문에서 ①(변경 이유가 둘: 사이트 CRUD ↔ 파일로 주고받기)·③이 「예」, ④가 「아니오」로 나와 자식 모듈 `site_manager/exchange.rs`로 나눴다(1,915 + 837줄). `ui::app` ↔ `ui::app::transfer_conflict`와 같은 배치라 부모의 private 필드를 그대로 만진다.
    - **`src/ui/toast.rs` 1줄** — `문구는_인벤토리_원문_그대로다`가 언어를 잠그지 않아 병렬 실행에서 간헐 실패했다(2026-08-20 실측 3회 중 1회). 이 task가 만든 결함은 아니고 대장에 이미 있던 항목이지만, 남겨 두면 **이후 모든 task의 `cargo test` 판정이 흔들리므로** 그 자리에서 `LanguageGuard::lock`을 더해 해소했다(4회 연속 통과 확인).
  - **Edge Cases**:
    - 암호에 비ASCII·아주 긴 문자열 / 확인 칸만 채운 경우
    - 가져온 파일에 사이트가 0건 → "가져올 사이트가 없습니다"
    - 봉투가 없는 파일을 가져올 때는 암호를 묻지 않는다
    - 겹치는 것이 0건이면 충돌 대화 없이 곧바로 반영한다
    - 내보내기·가져오기 중에 `삭제` 확인이 함께 뜨지 않게 한다(`pending_delete`와 같은 프레임 억제 규칙을 따른다)
    - 이름 바꾸기 편집 중에 두 버튼을 누르면 편집을 먼저 확정한다
  - **Halt Forecast**:
    - (i) 상태가 많아 `show`가 길어짐 → 대화 셋을 별도 메서드로 나누고 `show` 말미에서 차례로 부른다(기존 `show_delete_confirm` 배치와 같다)
  - **Depends on**: T2, T3

- [x] T5. 앱에 파일 대화와 결과 알림을 배선한다
  - **Type**: C
  - **Design**: ① 배치 — `src/ui/app.rs`의 `show_site_manager`와 `update` 말미. ② 신규 심볼 — `ExplorerApp::pump_site_file_dialog`(파일 대화를 띄우고 결과를 되돌린다)·`ExplorerApp::flush_site_notice`(결과를 토스트로 알리고 목록을 적는다) 둘. 그 밖에는 기존 `self.shell`(HWND)·`self.toast`·`self.persist_session()`을 쓴다. **`SiteManager::fail_file_request`를 하나 더 열었다** — 창 핸들이 없을 때 사유를 관리자 바닥에 남기는 통로이며, plan T5 acceptance가 요구한 동작이라 T4의 공개 메서드 셋에 하나가 더해진다. ③ 의존 — `fs::file_dialog`·`ui::site_manager::FileRequest`. ④ 비추상화 — 파일 요청을 나르는 전용 큐·채널을 두지 않는다(요청은 프레임당 최대 하나다). 봉투 연산·파일 읽기·쓰기도 **워커로 내리지 않고 이 자리에서 그대로 돈다**(D13) — 그 결정을 코드 주석에 사유와 함께 적는다.
  - **Acceptance**:
    - Given 사이트 관리자가 파일 요청을 세웠을 때, When 그 프레임의 그리기가 끝나면, Then 셸 메뉴를 띄우는 자리(`update` 말미)에서 파일 대화가 뜨고 결과가 `supply_file`로 돌아간다 — 그리기 도중에는 띄우지 않는다
    - Given 가져오기·내보내기가 목록을 바꿨을 때, Then `persist_session()`이 불려 `settings.json`에 반영된다
    - Given `take_notice()`에 문구가 있으면, Then 등록 토스트와 같은 통로로 뜬다
    - Given `self.shell`이 `None`(HWND 없음), Then 파일 대화를 띄우지 않고 사이트 관리자에 사유를 남긴다
    - `cargo build`·`cargo clippy --all-targets -- -D warnings` 통과
  - **Files**:
    - 주: `src/ui/app.rs`
    - 동반: `src/i18n/mod.rs`(HWND 없음 사유 문구) · `src/ui/site_manager/exchange.rs`(`fail_file_request` 신설)
    - 테스트: `src/ui/site_manager/exchange.rs`에 `fail_file_request` 시험 1건. **`app.rs` 배선 자체는 시험 대상이 아니다** — `ExplorerApp`은 단위 시험에서 만들 수 없다(P6). **이 배선 줄은 리뷰가 지키는 자리**임을 코드 주석에 명시한다(위키 conventions [2026-08-17]의 처방)
  - **Edge Cases**:
    - 같은 프레임에 셸 컨텍스트 메뉴 요청과 파일 요청이 함께 있는 경우 → 셸 메뉴를 먼저 처리하고 파일 대화는 그 다음 프레임으로 미룬다(두 모달을 겹쳐 띄우지 않는다)
    - 파일 대화가 뜬 동안 앱이 그리기를 멈추는 것은 셸 메뉴와 같은 성질이라 그대로 둔다
  - **Halt Forecast**:
    - (i) 없음 — 부르는 자리와 순서가 이미 정해져 있다
  - **Depends on**: T4

- [x] T6. 연결 메뉴 문구를 `사이트 관리자`로 바꾼다
  - **Type**: C
  - **Design**: 신규 심볼 없음 — 카탈로그 키 개명(D11)과 그 값 변경, 호출부·단언·주석 갱신뿐이다. **동작은 바꾸지 않는다** — 이 항목은 종전대로 `SidebarAction::OpenSiteManager` → `SiteManager::open_new()`(빈 초안)로 연다(`src/ui/app.rs:890`). 문구가 `사이트 관리자`가 됐다고 `open(store, None)`(첫 사이트를 고른 채 열기)으로 바꾸지 않는다 — 그러면 `확인(O)`이 고른 사이트를 덮어써 새 사이트를 더할 길이 사라진다(`site_manager.rs:411-414`의 사유).
  - **Acceptance**:
    - Given 사이드바 `+` 메뉴를 열면, Then 마지막 항목 문구가 `사이트 관리자`(영어 `Site Manager`)다
    - `sidebar_add_site`가 소스에서 0건이고 `sidebar_site_manager`로 바뀌어 있다
    - `sidebar.rs`의 `연결_섹션_문구는_인벤토리_원문_그대로다`가 새 문구를 단언하며, 원본 인벤토리(#8)와 갈린 사실과 그 사유(사용자 요청 2026-08-20)를 그 시험 주석에 적는다
    - `새 사이트 추가…`를 언급하는 주석 5곳이 새 문구를 가리키도록 갱신된다(`src/remote/types.rs:142` · `src/ui/app.rs:532`·`:889` · `src/ui/sidebar.rs:106` · `src/ui/site_manager.rs:411`)
    - `cargo test` 통과 · `grep -rn "새 사이트 추가" src/` → 0건
  - **Files**:
    - 주: `src/i18n/mod.rs`
    - 동반: `src/ui/sidebar.rs` · `src/ui/app.rs` · `src/ui/site_manager.rs` · `src/remote/types.rs`
    - 테스트: `src/ui/sidebar.rs`의 `연결_섹션_문구는_인벤토리_원문_그대로다`
  - **Edge Cases**:
    - 영어 문구도 함께 바꾼다(카탈로그가 한쪽만 빠지면 컴파일 오류다)
    - `site_title`(대화 제목)과 값이 같아지지만 키는 따로 둔다(D11)
  - **Halt Forecast**:
    - (i) 없음 — 개명 대상이 `grep -rn "sidebar_add_site" src/` 3 hits·`grep -rn "새 사이트 추가" src/` 7 hits로 전수 확정됐고, 카탈로그 매크로가 한·영 누락을 컴파일 오류로 잡는다
  - **Depends on**: -

- [x] T7. PRD·README·AGENTS.md를 갱신한다
  - **Type**: A
  - **Acceptance**:
    - `docs/prd.md`에 **FR-59**(사이트 목록 내보내기·가져오기, Must)가 신설되고 검증 방법 열에 단위 시험 항목이 적힌다. FR-27 문면에 좌측 버튼 두 줄이, FR-28 문면에 "내보내기 파일만은 사용자 암호로 봉한다(PBKDF2+AES-GCM)"가 더해진다. 개정 이력에 2026-08-20 줄이 붙는다
    - 위 `## PRD Coverage` 표의 FR-59 행이 실제 PRD 항목과 번호·우선순위가 일치한다
    - `README.md` §핵심 기능의 사이트 관리자 항목에 내보내기·가져오기 서술이 들어가고, "다른 PC로 설정을 옮기면 다시 입력받습니다"가 내보내기 파일을 쓰면 옮길 수 있다는 사실과 어긋나지 않게 고쳐진다
    - `AGENTS.md` 「데이터 접근 — 비밀번호」에 두 번째 통로(암호 기반 봉투 `remote::envelope`, 내보내기 파일 전용)가 한 줄 더해진다
    - `AGENTS.md`의 DO NOT 「UI 스레드에서 파일시스템 블로킹 호출」에 **그 금지가 겨냥하는 범위**(매 프레임 도는 렌더·탐색 경로)와 예외 둘(세션 저장 `persist_session`, 사이트 내보내기·가져오기)이 한 줄로 적힌다 — D13이 그 DO NOT을 좁혀 읽는 근거를 규약 쪽에도 남기지 않으면 다음 세션이 같은 판단을 다시 해야 한다
    - 세 문서 어디에도 실제 호스트·계정·비밀번호가 없다
  - **Files**:
    - 주: `docs/prd.md`
    - 동반: `README.md` · `AGENTS.md`
  - **Halt Forecast**:
    - (i) 없음 — 문서 3종의 문면 수정뿐이라 파괴적·외부 요소가 없다. FR-59는 미사용 번호이며(현재 최대 FR-58) PRD 개정은 이 plan 승인에 포함된다(사전 승인 항목)
  - **Depends on**: T1, T2, T4, T6

## 사전 승인 항목 (일괄 승인 대상)

- T1·T2·T3 — 신규 모듈 3개 추가(`src/remote/envelope.rs`·`src/remote/site_export.rs`·`src/fs/file_dialog.rs`)와 그에 따른 `mod.rs` 선언 추가 (구조 변경)
- T1 — `remote::secret::zeroize`의 가시성을 비공개에서 `pub(crate)`로 넓힘 (공개 표면 변경)
- T4 — `SiteManager`에 공개 메서드 3개(`take_file_request`·`supply_file`·`take_notice`)와 공개 타입 `FileRequest` 추가 (공개 API 추가)
- T6 — `i18n::sidebar_add_site` → `sidebar_site_manager` 개명 (공개 심볼 이름 변경, 호출부 2곳)
- T7 — `docs/prd.md`에 FR-59 신설 및 FR-27·FR-28 문면 개정 (요구 정본 변경)
- 전 task — 새 파일 형식 `.moasites` v1의 확정 (D5·D9 — 한 번 나가면 되돌리기 어려운 형식)

## 불가피한 Halt (위임 불가)

- commit 이후의 push·병합·태그·릴리즈 — 이 plan의 위임 범위는 로컬 작업 브랜치 commit까지다
- 신규 패키지(crate) 추가가 필요하다고 판단되는 경우 — 이 plan은 신규 의존성 0을 전제로 세웠다(P2). 그 전제가 깨지면 멈추고 승인받는다
- 파괴적 작업(기존 `settings.json` 삭제·초기화 등) — 이 plan에는 없다

## Verification Strategy

- 빌드: `cargo build`
- 단위·통합 테스트: `cargo test`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 서식: `cargo fmt --check`
- 수동 검증 (사용자 확인 필요 — 자동 판정 불가):
  1. 사이트 관리자 좌측에 두 줄 버튼이 서고 목록 웰이 그만큼 짧아진 모습
  2. 암호를 넣어 내보내기 → 파일 저장 대화 → 만들어진 `.moasites`를 메모장으로 열어 평문 비밀번호가 없음을 확인
  3. 그 파일을 같은 PC에서 가져와 겹치는 사이트에 대해 `덮어쓰기`·`건너뛰기`가 각각 목록에 어떻게 반영되는지
  4. 암호를 비운 채 내보내기 → 확인 대화가 한 번 더 뜨는지
  5. 틀린 암호로 가져오기 → 사유가 뜨고 다시 물어보는지
  6. 사이드바 `+` 메뉴 마지막 항목이 `사이트 관리자`인지 (영어 전환 시 `Site Manager`)
  7. 다른 PC(또는 다른 Windows 계정)에서 가져와 비밀번호까지 연결에 쓰이는지

## Phase Ledger

## Retry Ledger

## Progress Log

- T1-T2 완료 (커밋 7a86a66, dd6caa7 이후 완료 커밋): 순수 계층 둘을 세웠다.
  - T1 `remote::envelope` — CNG PBKDF2-SHA256(600,000회) + AES-256-GCM 봉투, hex 직접 구현. 신규 크레이트 0.
  - T2 `remote::site_export` — 문서 모델·충돌 판정·병합. `password_sealed`는 문서에 담지 않는다(D6 보호).
  - **D13 실측**: 릴리즈 빌드 PBKDF2 1회 파생 **0.126초**(상한 1.0초) → 반복 유지.
  - **D16 신설**(구현 중 확정): 문서에 비밀번호가 없으면 기존 것을 지우지 않는다. plan D14가 「필드만 갈아 끼운다」까지만 정해 열려 있던 갈래이며, 지우는 쪽은 되돌릴 수 없는 손실이라 지키는 쪽을 택했다.
  - 리뷰: T1 spec/quality 각 MINOR 1(판정 유보) · T2 quality OK, spec MAJOR 1(`password_failed` 분기 미검증) → 재현 가능한 실패 갈래로 시험 추가해 해소.
- T3-T4 완료: 파일 대화와 화면을 이었다.
  - T3 `fs::file_dialog` — `IFileSaveDialog`/`IFileOpenDialog` 래퍼. 시험 비대상(실제 대화가 동작의 전부)이라 그 사유를 모듈 주석에 적었다.
  - T4 사이트 관리자 — 좌측 아랫줄 버튼 둘 + 대화 넷(내보내기 암호 · 암호 없이 저장 확인 · 가져오기 암호 · 겹치는 사이트). 문구 32건·리터럴 예외 7건.
  - **파일 분할**: `site_manager.rs`가 2,699줄이 되어 AGENTS 네 질문 판정으로 `site_manager/exchange.rs`를 갈랐다(1,915 + 837).
  - **범위 밖 1줄**: `ui/toast.rs`의 간헐 실패(언어 미잠금)를 해소했다 — 대장에 있던 기존 항목이지만 이후 task의 `cargo test` 판정을 흔들어 그 자리에서 고쳤다(4회 연속 통과 확인).
  - 리뷰: T3 spec MINOR 1(doc 주석 오귀속) → 수정 · T4 spec BLOCKER 1(상태 경로 미완주) → 시험 확장으로 해소, 재리뷰 통과.
- T5-T6 완료: 배선과 문구.
  - T5 `ui::app` — 파일 대화를 `update` 말미(셸 메뉴 자리 옆)에서 띄우고 결과를 되돌린다. 셸 메뉴가 뜬 프레임에는 미룬다. 결과 알림은 두 자리에서 비워 관리자를 닫은 뒤에도 유실되지 않는다. 창 핸들이 없으면 `fail_file_request`로 사유를 남긴다(공개 메서드 4번째 — plan Design 갱신).
  - T6 문구 — `sidebar_add_site` → `sidebar_site_manager` 개명 + 값 변경. 옛 문구를 가리키던 주석 5곳과 시험 1곳을 함께 고쳤고, 소스에 옛 문구 잔존 0건.
  - 리뷰: T5 spec MAJOR 1(신규 공개 메서드 시험 부재) → 시험 추가로 해소, quality MINOR 2(주석) 반영 · T6 spec/quality 둘 다 OK.

## Next Steps

- 권장 다음 액션: `pjc:implement-task`로 T1부터 실행

## Open Questions

- [x] Q1. 내보내기 파일의 비밀번호 보호 방식 — **암호 입력 기반 재암호화**(D1)
- [x] Q2. 덮어쓰기 판정 기준 — **호스트+포트+프로토콜+사용자**(D3)
- [x] Q3. PRD·README 반영 범위 — **PRD FR 신설 + README 갱신**(D 없음 — T7)
- [x] Q4. 파일 확장자·형식 — **`.moasites`(내용은 JSON)**(D5)
- [x] Q5. 내보내기 암호 규칙 — **암호를 비우면 비밀번호 제외**(D6 — 비웠을 때 확인 한 번 추가)
- [x] Q6. 중복 사이트를 묻는 방식 — **겹치는 것 전부에 한 번에**(D4)
- [x] Q7. 새 버튼 배치 — **아랫줄 두 칸 균등 분할**(D10)
