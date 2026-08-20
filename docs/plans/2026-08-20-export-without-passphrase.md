# Plan: 내보내기에서 암호 입력을 없앤다 — 앱 내장 키로 봉인

**PRD**: docs/prd.md

## 요구 이해

- **원문 요청**: "사용이 더 불편한데 그냥 내보내기 하면 암호 입력없이 바로 저장할 폴더 선택하고 저장 하도록 해 사이트 암호까지 모든 정보 저장"
- **이해한 요구**: 방금 만든 내보내기가 암호를 두 칸 받고 확인 대화까지 거쳐 번거롭다. `내보내기`를 누르면 **곧바로 파일 저장 대화**가 뜨고 고르면 끝나야 한다. 그러면서도 **비밀번호를 포함해 모든 정보가 담겨야** 한다. 비밀번호를 무엇으로 보호할지는 사용자가 **앱 내장 키**로 정했다(2026-08-20 답변) — 파일을 열어도 평문이 보이지 않고 다른 PC에서도 복원되지만, MOA를 가진 사람은 풀 수 있다는 대가를 알고 택한 것이다.
- **포함하지 않는 것으로 이해**: 가져오기 쪽 흐름은 그대로다(파일 고르기 → 겹치면 묻기). **암호로 보호해 내보내는 선택지를 따로 남기지 않는다** — 사용자가 "그냥 바로 저장"을 요구했고, 선택지를 남기면 버튼·메뉴가 늘어 그 요구와 어긋난다.

## Goal

`내보내기` → 저장 위치 고르기 → 끝. 비밀번호까지 담기며 파일에 평문은 없다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-59 (문면 개정 — 암호 입력 제거, 앱 내장 키) | Must | T1·T2·T3·T4 | ✅ 커버 |
| FR-28 (문면 개정 — 예외 서술이 「사용자 암호」에서 「앱 내장 키」로) | Must | T4 | ✅ 커버 |
| 그 밖의 active Must FR | Must | — | 이번 범위 외 (기구현 — 이번 변경이 닿지 않는다) |

## Out of Scope

- **암호로 보호해 내보내는 선택지를 UI에 남기는 것** — 사용자가 "그냥 바로 저장"을 요구했다. 코드에는 `open_with_passphrase`가 남지만(구버전 파일을 읽기 위해) **새로 그런 파일을 만드는 길은 화면에 두지 않는다**.
- 내보내기 파일을 사용자가 직접 고른 암호로 다시 잠그는 기능(향후 요구가 생기면 그때).
- 앱 **설정 값**의 가져오기/내보내기·프로파일(`docs/prd.md` Out of Scope) — 사이트 목록과 다른 대상이라 이번 변경과 무관하다.

## Deferred / Follow-up

- 내보내기 진행 표시(사이트가 아주 많을 때의 진행률) — 직전 회차에서 이월. 실측상 수십 건은 즉시 끝난다.
- **내보내기 기본 파일 이름의 앱 이름 표기** — 한국어 화면에서도 `MOA 사이트.moasites`다. FR-53이 적용처를 창 제목·트레이 툴팁으로 한정해 파일 이름은 미정이다(직전 회차 F-7 m2 — 사용자 판단 사항).
- **`src/ui/site_manager.rs`의 모듈 주석이 버튼 수를 옛 값으로 적고 있다** — 「좌측 400px(사이트 목록 + 버튼 3개)」인데 실제로는 윗줄 3개(`show_list_buttons`) + 아랫줄 2개(`show_exchange_buttons`)다. 직전 회차가 아랫줄을 더하면서 주석을 함께 고치지 않은 것으로, 이번 회차의 변경과는 인과가 없다(그 파일은 이 회차 어느 task의 Files에도 없다). Phase F의 참조 정합에서 함께 본다.
- **간헐 실패 시험의 이름을 확보했다**: `remote::connection::tests::늦게_도착한_이전_세대의_목록은_버려진다`. 직전 회차에 「899 passed; 1 failed」로 한 번 나왔으나 이름을 못 잡았던 그것이며, T2 리뷰 중 다시 관찰돼 이번에 특정했다. `wait_events(&mut connection, 4, Duration::from_secs(2))`가 병렬 실행 부하에서 2초를 넘겨 이벤트 4개를 못 모으는 것으로 **추정**한다(격리 재실행은 통과). 이번 변경과 인과가 없어(`connection.rs`는 이 회차 어느 task의 Files에도 없다) 이연하며, **고칠 때는 타임아웃을 늘리기 전에 이벤트가 정말 4개 오는지부터 확인한다** — 상수만 키우면 원인을 덮는다.

## Investigation Log

- **위키 참조**: `20_projects/personal/moa/decisions.md` [2026-08-04] 자격증명 보관 — DPAPI 채택, **평문 저장은 기각**. 이번 변경은 그 기각과 충돌하지 않는다(내장 키 암호화는 평문이 아니다) — 다만 **보호 강도는 사용자 암호보다 낮다**는 점을 plan D1에 명시한다.
- **위키 참조**: `20_projects/personal/moa/feat-remote-sites.md` — DPAPI가 "다른 계정·다른 PC에서는 풀리지 않는다"는 설계 전제. 내보내기가 그 전제를 우회하는 별도 봉투를 쓰는 구조는 직전 회차에 이미 세웠고 이번에는 **그 봉투의 열쇠만 바뀐다**.
- **위키 참조**: `20_projects/personal/moa/conventions.md` — ⓐ 카탈로그 값 단언 시험은 `LanguageGuard::lock`을 든다(직전 회차에 `toast.rs`를 고쳐 해소) ⓑ 함수를 끼워 넣을 때 앞뒤 빈 줄이 없으면 doc 주석이 딸려 붙는다 ⓒ `ExplorerApp`은 단위 시험에서 만들 수 없다.
- **Deferred 대장 조회**: `docs/plans/deferred.md` `## 대기` 67건 제목 스캔. 이번 계획의 전제를 부정하는 항목 없음. 주제 매칭 항목 없음(내보내기 관련 항목은 직전 회차에 `## 종결`로 옮겼다). 잔량 100건 미만이라 소진 batch 미착수.
- **현행 구조 (직전 회차 산출물 — 실측)**:
  - `remote::envelope`: `KDF_NAME`(`"PBKDF2-HMAC-SHA256"`)·`PBKDF2_ITERATIONS`(600,000)·`SALT_LEN`(16)·`NONCE_LEN`(12)·`TAG_LEN`(16)·`KEY_LEN`(32), 공개 함수 `seal_with_passphrase`·`open_with_passphrase`·`to_hex`·`from_hex`, 내부 `derive_key`·`encrypt`·`decrypt`·`AlgHandle`·`KeyHandle`.
  - `remote::site_export`: `build(store, passphrase)`가 `passphrase.is_empty()`면 `secret = None`. `needs_passphrase(document)`는 `document.secret.is_some()`. `plan_import(document, store, passphrase)`가 `open_with_passphrase`로 푼다.
  - `ui::site_manager::exchange`: `Exchange` 7상태, 대화 4개(`show_export_ask`·`show_export_empty_confirm`·`show_import_ask`·`show_import_conflict`), 보조 `request_export_file`.
- **제거·유지 대상 i18n 키 사용처 (실측 — `grep -rn <키> src/ --include=*.rs` 에서 카탈로그 자신을 뺀 수)**:
  - 제거 대상 9건: `site_export_title`(1)·`site_export_hint`(1)·`site_export_passphrase_again`(1)·`site_export_empty_hint`(1)·`site_export_forget_warning`(1)·`site_export_mismatch`(1)·`site_export_empty_title`(1)·`site_export_empty_detail`(1)·`site_export_save`(2 — 없어지는 대화 둘이 함께 쓴다).
  - **개명 대상 1건**: `site_export_passphrase`(3 — 내보내기 2 + 가져오기 1). 앞의 둘이 사라지면 이름만 `export`인 채 가져오기에서만 쓰이므로 `site_import_passphrase`로 옮긴다.
  - 유지: `site_export_seal_failed`(1)·`site_export_write_failed`·`site_import_*` 전부·`site_conflict_*` 전부.
- **`EXEMPT_LITERALS` 영향**: 없어지는 위젯 Id·salt 4건(`"사이트 내보내기"`·`"사이트 내보내기 확인"`·`"내보내기 암호"`·`"내보내기 암호 확인"`)을 함께 뺀다. 배열 길이 상수 `35 → 31`.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| P1 | 앱 내장 키는 실행 파일에서 추출 가능하다 — 「잠금」이 아니라 「눈에 안 보임」 수준이다 | 키가 바이너리에 상수로 실린다는 것은 정의상 자명하다. **사용자가 그 대가를 알고 택했다**(2026-08-20 답변의 선택지 설명에 명시) | ✅ |
| P2 | 구버전(사용자 암호) 파일과의 호환을 `kdf` 필드로 가를 수 있다 | `envelope.rs:28`(`KDF_NAME`)·`:52-60`(`Envelope`에 `kdf` 필드)·`:93-95`(`open_with_passphrase`가 `kdf`를 검사해 거부) | ✅ |
| P3 | 내장 키에는 PBKDF2 반복이 방어를 더하지 않는다 | PBKDF2 반복의 목적은 **약한 비밀**의 사전 공격을 늦추는 것인데, 내장 키는 고정 고엔트로피 값이라 그 공격의 대상이 아니다(공격자는 키를 추측하는 대신 바이너리에서 읽는다) | ✅ |
| P4 | 제거·개명 대상 i18n 키의 사용처가 각각 특정됐다 | Investigation Log의 실측 표(키별 grep 카운트) | ✅ |
| P5 | 내보내기 대화 둘을 없애도 나머지 흐름(파일 요청 → `supply_file` → 알림)은 그대로 선다 | `exchange.rs:87-118`(공개 메서드 넷)·`:222-241`(`show_exchange` 분기) — 파일 대기 상태(`ExportWaitFile`)와 그 뒤 경로는 대화와 독립이다 | ✅ |
| P6 | 이 변경이 FR-28의 「평문은 어떤 파일·로그에도 남기지 않는다」를 깨지 않는다 | 내장 키 봉투도 AES-GCM 암호문이라 평문이 아니다. 직전 회차의 평문 부재 시험이 그대로 유효하다(`site_export.rs`의 `암호를_넣어도_문서에_봉인_바이트가_없다` 계열) | ✅ |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| 파일 하나만 얻으면 그 서버들에 접속할 수 있게 된다 | 유출 시 자격증명이 통째로 넘어간다 | 사용자가 대가를 알고 택했다(P1). **README·PRD에 그 성질을 명시**해 보관 주의를 남긴다(T4) — 코드로 막을 수 있는 위험이 아니다 |
| 구버전(사용자 암호) 파일을 못 열게 되면 이미 만든 백업이 무용해진다 | 사용자가 시험 삼아 만든 파일이 열리지 않는다 | `open_with_passphrase` 경로와 가져오기 암호 대화를 **그대로 남긴다** — 그런 파일을 골랐을 때만 암호를 묻는다 |
| 대화 둘을 들어내며 상태 기계가 어긋난다 | 내보내기가 중간에 멎거나 두 번 진행된다 | `Exchange` variant를 함께 지워 **컴파일러가 남은 분기를 강제로 드러내게** 한다(문자열·플래그로 우회하지 않는다). 상태 전이 시험을 새 흐름에 맞춰 고친다 |
| 안 쓰이게 된 i18n 키가 남는다 | 화면에 없는 문구가 카탈로그에 쌓인다 | 제거 9건·개명 1건을 실측 목록대로 처리하고, `EXEMPT_LITERALS`도 함께 줄인다(T3 acceptance) |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `envelope::seal_with_passphrase` | `src/remote/envelope.rs`(정의) · `src/remote/site_export.rs:205`(유일 호출부) | 호출부가 새 `seal_with_app_key`로 바뀐다. 함수 자체는 **남긴다**(시험이 쓰고, 향후 암호 보호 요구가 오면 그대로 쓴다) |
| `envelope::open_with_passphrase` | `src/remote/envelope.rs`(정의) · `src/remote/site_export.rs:266`(유일 호출부) | 구버전 파일 경로로 **유지**. 새 분기가 그 옆에 선다 |
| `envelope::KDF_NAME` | `src/remote/envelope.rs` 내부 2곳(봉인·검사) | 유지 + `KDF_APP_KEY` 신설 |
| `site_export::build(store, passphrase)` | `src/remote/site_export.rs:170`(정의) · `src/ui/site_manager/exchange.rs:138`(유일 호출부) · 시험 다수 | **시그니처 변경** — `passphrase` 인자를 없앤다(언제나 내장 키로 봉한다) |
| `site_export::needs_passphrase` | `src/remote/site_export.rs:250`(정의) · `src/ui/site_manager/exchange.rs:168`(유일 호출부) · 시험 1곳 | 판정 기준 변경 — `secret.is_some()`에서 **「사용자 암호 봉투인가」**로 |
| `site_export::plan_import(document, store, passphrase)` | `src/remote/site_export.rs:258`(정의) · `exchange.rs:184`(유일 호출부) · 시험 다수 | 시그니처 유지(구버전 파일에 여전히 암호가 필요하다). 내부에서 `kdf`로 갈라 푼다 |
| `Exchange::ExportAsk`·`Exchange::ExportConfirmEmpty` | `src/ui/site_manager/exchange.rs` — 정의 2(`:60`·`:66`) · 프로덕션 7(`:124`·`:225`·`:230`·`:302`·`:311`·`:320`·`:360`) · **시험 6**(`:618`·`:770`·`:801`·`:805`·`:827`·`:832`) | **제거** — 컴파일러가 남은 분기를 드러낸다 |
| `show_export_ask`·`show_export_empty_confirm` | `src/ui/site_manager/exchange.rs` — 각 정의 1 + `show_exchange` 분기 1 | **제거** |
| `request_export_file` | `src/ui/site_manager/exchange.rs` — 정의 1(`:327`) · **프로덕션 2**(`:323`·`:356`) · **시험 5**(`:628`·`:653`·`:666`·`:701`·`:754`) | **제거**. 프로덕션 두 호출부는 없어지는 두 대화 안이고, 시험 5곳은 새 흐름·픽스처로 다시 쓴다(4-C) |
| `Exchange::ExportWaitFile { pass }` | `src/ui/site_manager/exchange.rs:68`(정의) · `:100`(`supply_file` 매칭) · `:331` · 시험 `:631` | **`pass` 필드 제거** — 더 이상 들고 다닐 암호가 없다. 필드가 남으면 `finish_export`가 쓰지 않는 값을 받아 clippy가 막는다 |
| `finish_export(path, passphrase, store)` | `src/ui/site_manager/exchange.rs:137`(정의) · `:100`(유일 호출부) | **`passphrase` 인자 제거** — `build`가 인자를 잃으면 이 값이 미사용이 되어 `-D warnings`에 걸린다 |
| i18n 키 9건 제거 + 1건 개명 | `src/i18n/mod.rs` · `src/ui/site_manager/exchange.rs` | Investigation Log 실측 표대로 |
| `EXEMPT_LITERALS` | `src/i18n/mod.rs` | 4건 제거, 배열 길이 `35 → 31` |

### 4-B. 계약·직렬화 변경

- **`.moasites` v1 형식은 유지한다** — 필드 구성이 그대로이고 `Envelope.kdf` 값만 새로 생긴다(`"PBKDF2-HMAC-SHA256-appkey"`). 판 번호를 올리지 않는 이유: 구버전 앱이 새 파일을 열면 `open_with_passphrase`의 `kdf` 검사에 걸려 **"암호가 맞지 않는다"**로 거부되는데, 이는 안전한 실패이고 새 앱은 두 형식을 모두 읽는다. 판을 올리면 오히려 구버전이 `Unsupported`로 거부해 결과가 같으면서 새 앱의 호환 코드만 복잡해진다.
- **`build`의 공개 시그니처가 바뀐다**(인자 1개 제거) — 호출부는 1곳(실측).

### 4-C. 영향 받는 테스트

> **목록 작성 방식 (2라운드 RECURRING 대응)**: 아래는 손으로 센 것이 아니라
> `grep -rn "ExportAsk\|ExportConfirmEmpty\|request_export_file\|show_export_ask\|show_export_empty_confirm\|site_export::build\|build(&store\|build(&source" src/ --include=*.rs`
> **결과를 행 단위로 옮겨 적은 것**이다. 1·2라운드에 목록이 연달아 한 건씩 빠진 것은 열거를 기억에 맡겼기 때문이라, 방식을 바꿨다.

- 갱신: **`src/remote/site_export.rs`의 시험 15곳** — `build(&source, …)` 호출이 든 전부(`:443`·`:489`·`:516`·`:527`·`:546`·`:601`·`:649`·`:668`·`:688`·`:700`·`:738`·`:756`·`:764`·`:819`·`:834`). 인자를 빼는 기계적 수정이며, 그중 **둘만 뜻이 바뀐다**:
  - `암호를_비우면_비밀번호가_담기지_않는다`(`:484`) → **명제가 폐기된다.** 「암호 없이도 비밀번호가 담긴다」로 다시 쓴다(이름도 함께).
  - `틀린_암호로는_계획을_세우지_못한다`(`:523`) → 새 문서에는 틀릴 암호가 없다. **구버전 픽스처**(`legacy_document`)로 세워 회귀 자산을 지킨다.
- 갱신: **`src/ui/site_manager/exchange.rs`의 시험 8곳** (제거 대상 심볼을 쓰는 전부):
  - `내보내기는_암호를_받고_파일을_청한다`(`:612`, 쓰는 것 `:618`·`:628`) → **암호 단계 없이 곧바로 파일을 청하는지**로 다시 쓴다(이름도 `내보내기는_곧바로_파일을_청한다`).
  - `내보내기를_취소하면_아무_일도_없다`(`:651`, 쓰는 것 `:653`) → `apply_exchange_action(Export)`로 시작한다.
  - `가져오기는_파일을_청하고_겹치면_묻는다`(`:662`, 쓰는 것 `:666`) → 같은 방식으로 파일을 먼저 만든다.
  - `암호로_보호된_파일은_암호를_묻는다`(`:698`, 쓰는 것 `:701`) → **`legacy_document`로 픽스처를 만든다**(M3 — 「유지」가 아니라 재작성).
  - `사이트가_없으면_가져올_것도_없다고_알린다`(`:748`, 쓰는 것 `:754`) → 같은 방식.
  - `대화를_닫으면_적던_암호가_함께_사라진다`(`:768`, 쓰는 것 `:770`) → `ExportAsk` 대신 **`ImportAsk`**(암호를 들고 있는 유일한 남은 상태)를 조립한다. 명제 자체는 그대로 유효하다.
  - `파일을_기다리던_중이_아니면_받을_것이_없다`(`:799`, 쓰는 것 `:801`·`:805`) → `ExportConfirmEmpty` 대신 **`ImportConflict`**를 세운다.
  - `내보내기_대화가_한_프레임을_그린다`(`:822`, 쓰는 것 `:827`·`:832`·`:842`) → **남은 대화 셋**(`ImportAsk`·`ImportConflict` + 삭제 확인은 부모 소관이므로 둘)만 그리는 형태로 재작성하고 **이름과 주석(`:823` 「대화 넷의 그리기 경로」)을 함께 고친다**.
- **신규 시험 헬퍼 `legacy_document`** — 구버전(사용자 암호) 문서를 손으로 조립한다. **배치와 가시성을 여기서 확정한다**(M2): `src/remote/site_export.rs`의 `mod tests` **밖**에 `#[cfg(test)] pub(crate) fn legacy_document(sites: &[ExportedSite], passwords: &[String], passphrase: &str) -> SiteExport`로 둔다 — 두 파일(`site_export.rs`·`exchange.rs`)의 시험이 함께 써야 하는데 `mod tests`는 private이라 그 안에 두면 건너 부를 수 없다. 조립에 필요한 것은 전부 `pub`이다(`SiteExport`·`ExportedSite`·`secret` 필드·`envelope::seal_with_passphrase`).
- 영향 확인만: `src/i18n/mod.rs`의 소스 훑기 시험(`EXEMPT_LITERALS` 축소 후에도 통과해야 한다).

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `envelope::seal_with_app_key` / `open_with_app_key` | `seal_with_passphrase`/`open_with_passphrase`(같은 파일) | **신규 — 단 내부는 전부 재사용**. 기존 `derive_key`·`encrypt`·`decrypt`를 그대로 부르고 다른 것은 「어떤 비밀을, 몇 번 파생하는가」뿐이다. 두 경로를 한 함수에 `Option<&str>`로 합치지 않는 이유는 호출부가 「무엇으로 봉했는지」를 이름으로 드러내는 편이 안전하기 때문이다(빈 문자열이 내장 키를 뜻하는 식의 암묵 규칙을 두지 않는다) |
| `envelope::KDF_APP_KEY` 상수 | `KDF_NAME`(같은 파일) | **신규** — 두 봉투를 가르는 유일한 표식이다 |
| `envelope::APP_KEY` 상수 | grep `APP_KEY|app_key` → 0건 | **신규** — 내장 비밀. 소스에 상수로 둔다(바이너리에 실리는 이상 은닉 수준은 어디에 두든 같다) |

### Verified by

- `grep -rn "seal_with_passphrase\|open_with_passphrase" src/` → 정의 2 + `site_export.rs` 2 + 시험. 프로덕션 호출부는 `site_export.rs` 두 곳뿐
- `grep -rn "site_export::build\|needs_passphrase\|plan_import" src/` → 각 정의 1 + `exchange.rs` 1 + 시험
- `grep -rn "ExportAsk\|ExportConfirmEmpty" src/` → `exchange.rs` 한 파일 안(정의·분기·시험)
- i18n 키별 카운트는 Investigation Log의 실측 표

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `docs/prd.md` FR-59·FR-28 문면 | FR-59가 「사용자가 정한 암호로 봉해 담고, 암호를 비우면 비밀번호를 빼고 저장하되 그 사실을 한 번 더 묻는다」를 명시한다 — 이번 변경으로 **그 서술이 통째로 거짓**이 된다 | T4에 편입 |
| 필수 | `README.md` 사이트 관리자 항목 | 「그때 정한 암호로 봉해」·「암호를 비우고 저장하면」·「암호를 잊으면 그 파일은 다시 열 수 없습니다」 세 문장이 모두 사실과 달라진다 | T4에 편입 |
| 필수 | `AGENTS.md` 「비밀번호」 항목 | 「사용자가 정한 암호에서 키를 파생해 봉한다」가 어긋난다 | T4에 편입 |
| 필수 | 직전 plan(`2026-08-20-site-export-import.md`)의 D1·D6 | 그 결정이 이번에 번복된다. 그 파일은 **완료된 회차의 기록**이라 고치지 않고, 이 plan의 D1이 번복 사실과 사유를 적어 잇는다(git 이력이 정본) | 이 plan의 D1·D2에 명시 |
| 필수 | 이번 변경이 **거짓으로 만드는 코드 주석 4곳** — `envelope.rs:1-12`(모듈 doc: "사용자가 정한 암호에서 키를 파생한다") · `envelope.rs:62-65`(`seal_with_passphrase` doc의 D6 서술) · `site_export.rs:166-169`(`build` doc: "`passphrase`가 비어 있으면 담지 않는다") · `exchange.rs:1-8`(모듈 doc: "대화 넷") · `exchange.rs:823`(시험 주석: "대화 넷의 그리기 경로가 패닉 없이 도는지") | 파일의 성격을 통째로 잘못 서술하게 된다. **특히 `envelope.rs` 모듈 doc은 「내장 키는 실행 파일에서 추출 가능하다」는 성질을 코드 곁에 남길 유일한 자리다** — 그것이 없으면 다음 세션이 이 봉투를 DPAPI급 보호로 오독한다 | T1(envelope 2곳)·T2(site_export 1곳)·T3(exchange 1곳)에 편입 |
| 필수 | `docs/prd.md` Out of Scope의 **「사이트 설정의 가져오기/내보내기」**(`:116`) | **PRD가 지금 자기모순이다** — FR-59가 Must로 그 기능을 요구하는데 같은 문서의 Out of Scope가 그것을 제외로 남겨 뒀다. 직전 회차가 FR-59를 신설하며 해제하지 않은 누락이고(이번 변경이 유발한 것은 아니다) 아직 아무도 고치지 않았다. T4가 같은 파일의 FR-59를 손대므로 여기서 함께 닫는다 — 한 줄이며, 남겨 두면 Phase G가 매번 이 모순을 다시 만난다 | T4에 편입 |
| 무관 | 위키 큐의 `[DECISION]` 항목(내보내기 관련 5건) | 아직 본문에 반영되지 않은 대기 항목이라 지금 고칠 대상이 아니다. 이번 회차의 번복도 F-6.5가 같은 큐에 넣는다 | 건드리지 않음 |

## Decisions

### D1. 비밀번호를 무엇으로 보호하는가 (직전 회차 D1의 번복)

- **Options**: A) 앱 내장 키로 암호화 / B) 사용자 암호(직전 회차의 선택) / C) DPAPI / D) 평문
- **Chosen**: A (사용자 선택 2026-08-20)
- **Rationale**: B는 보호가 가장 세지만 매번 암호를 두 칸 받고 확인까지 거쳐 **사용자가 불편하다고 판단했다**. A는 클릭 두 번으로 끝나면서 다른 PC 복원도 된다. **대가는 명확하다 — 키가 실행 파일에 실리므로 MOA를 가진 사람은 누구나 풀 수 있어, 그 파일 자체가 자격증명이 된다**(P1). 사용자가 그 설명을 받고 택했다. D는 위키 decisions [2026-08-04]가 기각한 것이고 PRD FR-28과도 어긋나 후보가 아니다. C는 다른 PC 복원이라는 이 기능의 목적을 없앤다.
- **Source**: 사용자 답변 2026-08-20 · 위키 `decisions.md` [2026-08-04] · 직전 plan D1

### D2. 암호 입력 대화 둘을 없앤다 (직전 회차 D6의 번복)

- **Options**: A) 둘 다 제거하고 곧바로 파일 대화 / B) 암호 칸 하나만 남긴다 / C) 「암호로 보호」 체크박스를 파일 대화에 붙인다
- **Chosen**: A
- **Rationale**: 사용자 요구가 "그냥 바로 저장"이다. B는 여전히 한 단계이고, C는 `IFileDialogCustomize`로 커스텀 컨트롤을 붙여야 해 코드가 늘면서 얻는 것은 거의 없다(내장 키로 이미 보호되므로 추가 암호는 선택적 강화일 뿐이다). **암호 보호가 다시 필요해지면 그때 별도 진입점으로 되살린다** — `seal_with_passphrase`를 지우지 않는 이유가 그것이다.
- **Source**: 사용자 요구 원문 · 직전 plan D6

### D3. 구버전(사용자 암호) 파일을 계속 읽는다

- **Options**: A) 읽는다(`kdf`로 분기) / B) 안 읽는다(형식을 하나로)
- **Chosen**: A
- **Rationale**: 직전 버전이 이미 master에 병합·push돼 사용자가 그 빌드로 파일을 만들었을 수 있다. **비용이 거의 없다** — `Envelope.kdf`가 이미 형식을 밝히고 있어 분기 하나면 되고, 가져오기 암호 대화도 이미 있다. 그 파일을 만나지 않으면 대화는 영영 뜨지 않으므로 "암호를 없앤다"는 요구와 충돌하지 않는다.
- **Source**: `envelope.rs:52-60`·`:93-95`(kdf 필드와 검사) · 직전 회차 커밋이 master에 있음

### D4. 내장 키 봉투의 파생 반복 횟수

- **Options**: A) 1회 / B) 600,000회(사용자 암호와 같게) / C) 파생을 아예 생략하고 키 상수를 그대로 쓴다
- **Chosen**: A
- **Rationale**: PBKDF2 반복은 **약한 비밀**의 사전 공격을 늦추려는 것인데 내장 키는 고정 고엔트로피 값이라 그 공격 대상이 아니다(P3) — B는 0.126초를 값 없이 쓴다. C가 이론상 가장 단순하지만 `Envelope`의 `salt`·`iterations` 필드가 뜻을 잃어 형식이 어긋나고, 파생 경로가 둘로 갈려 코드가 늘어난다. A는 **기존 경로를 그대로 재사용하면서**(salt는 여전히 매번 랜덤이라 같은 목록도 매번 다른 암호문이 된다) 비용만 없앤다.
- **Source**: `envelope.rs:196-212`(`derive_key`가 반복을 인자로 받는다)

### D5. `.moasites` 판 번호를 올리지 않는다

- **Options**: A) v1 유지 / B) v2로 올린다
- **Chosen**: A
- **Rationale**: 필드 구성이 그대로이고 `kdf` 값만 는다. 구버전 앱이 새 파일을 만나면 `kdf` 검사에 걸려 "암호가 맞지 않는다"로 거부되는데 이는 **안전한 실패**다. B로 올리면 구버전이 "더 새로운 버전"으로 거부해 결과가 같으면서, 새 앱은 v1·v2를 모두 읽는 코드를 져야 한다.
- **Source**: 4-B · `site_export.rs:233-248`(`parse`의 format·version 검사)

### D6. `site_export_passphrase` 키를 개명한다

- **Options**: A) `site_import_passphrase`로 옮긴다 / B) 이름을 그대로 둔다
- **Chosen**: A
- **Rationale**: 내보내기 쪽 두 사용처가 사라지면 **이름만 `export`인 채 가져오기에서만 쓰인다**(실측 3 → 1). 이름이 거짓이 되는 것은 직전 회차에 `sidebar_add_site`를 개명한 것과 같은 사유이며, 호출부가 1곳이라 비용도 같다.
- **Source**: Investigation Log 실측 표 · 직전 plan D11(같은 판단의 선례)

## Tasks

<!-- T1~T2 (봉인 계층) → T3 (화면·문구) → T4 (문서) -->

- [x] T1. `remote::envelope`에 앱 내장 키 경로를 낸다
  - **Type**: C
  - **Design**: ① 배치 — `src/remote/envelope.rs` 안(새 파일 없음). ② 신규 심볼 — `APP_KEY`(내장 비밀 문자열 상수, 비공개), `KDF_APP_KEY`(`"PBKDF2-HMAC-SHA256-appkey"` — 두 봉투를 가르는 표식, 공개), `seal_with_app_key(plain) -> Option<Envelope>`, `open_with_app_key(&Envelope) -> Option<Vec<u8>>`. ③ 의존 — 기존 `derive_key`·`encrypt`·`decrypt`·`random_bytes`를 그대로 재사용하며 새 FFI를 늘리지 않는다. ④ 비추상화 — 「봉인 방식 트레이트」·「키 공급자」를 두지 않는다. 두 경로가 갈리는 지점은 **비밀 값과 반복 횟수 둘뿐**이라 함수 두 개가 가장 단순하다.
  - **Acceptance**:
    - Given 임의 평문, When `seal_with_app_key` → `open_with_app_key`, Then 원문이 그대로 돌아온다(빈 평문·1KB 포함)
    - Given 내장 키로 봉한 봉투, When `open_with_passphrase`로 열면, Then `None`이다(`kdf`가 다르다). **반대도 마찬가지** — 사용자 암호 봉투를 `open_with_app_key`로 열면 `None`
    - Given 같은 평문을 두 번 봉하면, Then `salt`·`nonce`·`ciphertext`가 서로 다르다(내장 키여도 매번 다른 암호문)
    - Given 내장 키 봉투의 `ciphertext` 한 바이트를 뒤집으면, Then `None`이다
    - 봉투 JSON에 평문이 부분 문자열로 없다
    - `kdf` 필드 값이 `KDF_APP_KEY`이고 `iterations`가 1이다 (D4)
    - `cargo clippy --all-targets -- -D warnings` 통과 · 새 `unsafe` 0(기존 래퍼만 쓴다)
    - **모듈 doc이 두 봉투를 함께 서술하고, 내장 키의 성질을 못 박는다** — 「키가 실행 파일에 실리므로 MOA를 가진 사람은 풀 수 있다 = 잠금이 아니라 눈에 안 보임」. 지금 doc은 「사용자가 정한 암호에서 키를 파생한다」고만 적어 이번 변경으로 거짓이 된다. `seal_with_passphrase`의 doc에서도 폐기된 D6 서술을 걷어낸다
  - **Files**:
    - 주: `src/remote/envelope.rs`
    - 테스트: 같은 파일의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 빈 평문 / 1KB 평문 / 난수 실패 시 `None`
    - `open_with_app_key`에 길이가 어긋난 봉투(`salt`·`nonce`·`tag`) → `None`
    - 두 함수가 같은 `Envelope` 타입을 쓰므로 **kdf 검사를 빠뜨리면 교차 해제가 조용히 성공할 수 있다** — 위 acceptance 2번이 그것을 잡는다
  - **Halt Forecast**:
    - (i) 없음 — 기존 내부 함수를 재사용하는 추가라 새 API 조사가 필요 없다
  - **Depends on**: -

- [x] T2. `remote::site_export`가 내장 키로 봉하고, 구버전 파일도 읽게 한다
  - **Type**: C
  - **Design**: ① 배치 — `src/remote/site_export.rs` 안. ② 신규 심볼 없음 — 기존 `build`·`needs_passphrase`·`plan_import`의 **동작만** 바꾼다(`build`는 인자 하나가 줄어 시그니처가 바뀐다). ③ 의존 — `envelope`의 새 두 함수. ④ 비추상화 — 「봉인 전략」 열거형을 두지 않는다. 판정은 `Envelope.kdf` 값 비교 한 줄이면 끝난다.
  - **Acceptance**:
    - Given 사이트가 든 저장소, When `build(&store)`, Then **언제나 `secret`이 `Some`**이고 그 `kdf`가 `KDF_APP_KEY`다(비밀번호가 하나도 없어도 봉투는 만들어진다 — 빈 목록을 봉한 것이다)
    - Given 그 문서, When `needs_passphrase`, Then `false`다. **구버전(사용자 암호) 봉투가 든 문서**면 `true`다. **`secret`이 아예 없는 문서**(직전 버전에서 암호를 비우고 내보낸 것)도 `false`이며 비밀번호 없이 나머지 설정만 들어온다 — 판정이 세 갈래임을 잊고 `secret.is_none()`을 잘못 묶으면 이 경로가 깨진다
    - Given 그 문서, When `plan_import(document, &store, "")`, Then 암호 없이 계획이 서고 비밀번호가 그대로 풀린다
    - Given 구버전 봉투 문서, When 맞는 암호로 `plan_import`, Then 계획이 서고 **틀린 암호면 `WrongPassphrase`**다(회귀 방지)
    - **그 구버전 문서를 만드는 경로가 시험 안에 있다** — 앱에는 더 이상 사용자 암호 파일을 만드는 길이 없으므로(D2), 시험이 `SiteExport { secret: Some(envelope::seal_with_passphrase(&serde_json::to_vec(&passwords)?, "맞는 암호")?), .. }`를 **손으로 조립하는 헬퍼**(`legacy_document`)를 둔다. 이 헬퍼가 없으면 구버전 호환(D3)을 검증할 방법이 사라진다
    - 저장된 `.moasites` 텍스트에 평문 비밀번호가 없고 `password_sealed` 키도 없다
    - 사이트 0개 문서도 왕복한다
    - 기존 시험 중 「암호를 비우면 비밀번호가 담기지 않는다」는 **뜻이 바뀌었으므로 「암호 없이도 비밀번호가 담긴다」로 다시 쓴다**(삭제가 아니라 재작성 — 그 자리가 이번 변경의 핵심이다)
    - `cargo clippy --all-targets -- -D warnings` 통과 — 위 Files의 연쇄 두 곳(미사용 인자·필드)을 함께 고쳐야 여기에 닿는다
  - **Files**:
    - 주: `src/remote/site_export.rs`
    - 동반: `src/ui/site_manager/exchange.rs` — `build` 호출부(`:138`)**와 그 연쇄로 미사용이 되는 둘까지 이 task가 함께 고친다**: `finish_export`의 `passphrase` 인자(`:137`·유일 호출 `:100`)와 `Exchange::ExportWaitFile`의 `pass` 필드(`:68`·`:100`·`:331`·시험 `:631`). `build`가 인자를 잃는 순간 둘 다 쓰이지 않게 되어 `-D warnings`가 막으므로, 이것을 T3로 미루면 T2가 완료 판정을 받을 수 없다
    - 테스트: 같은 파일의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 비밀번호가 하나도 없는 목록 → 빈 문자열 배열을 봉한다(봉투는 만들어진다)
    - 구버전 봉투 + 빈 암호 → `WrongPassphrase`
    - `kdf`가 둘 중 어느 것도 아닌 문서 → 두 해제 모두 `None`이므로 `WrongPassphrase`로 떨어진다(그 판정을 주석에 적는다)
    - 봉투 평문 배열 길이가 사이트 수와 다름 → 기존대로 `Broken`
  - **Halt Forecast**:
    - (i) 없음 — 새 API·외부 의존이 없고, 갈림길이던 호환 분기와 판 번호는 D3·D5가 이미 확정했다. 시그니처 변경의 호출부도 1곳으로 실측됐다
  - **Depends on**: T1

- [x] T3. 내보내기 대화 둘을 들어내고 문구를 정리한다
  - **Type**: D
  - **Design**: ① 배치 — `src/ui/site_manager/exchange.rs`·`src/i18n/mod.rs`. ② 신규 심볼 없음 — `Exchange::ExportAsk`·`ExportConfirmEmpty` variant와 `show_export_ask`·`show_export_empty_confirm`·`request_export_file`을 **제거**하고, `apply_exchange_action`의 `Export` 갈래가 곧바로 `pending_file = Save` + `Exchange::ExportWaitFile`을 세운다. ③ 의존 — 변화 없음. ④ 비추상화 — 제거만 하고 남는 셋(가져오기 암호·충돌 확인)을 공통 부품으로 묶지 않는다(본문 구성이 서로 다르다).
  - **Acceptance**:
    - Given 사이트가 1개 이상, When `내보내기`를 누르면, Then **대화 없이** 곧바로 `FileRequest::Save`가 청해지고 상태가 `ExportWaitFile`이 된다
    - Given 그 뒤 `supply_file(Some(경로))`, Then 파일이 쓰이고 결과 알림이 나오며 상태가 `Idle`로 돌아간다. `None`(취소)이면 아무 일도 없다
    - Given 사이트가 0개, Then `내보내기`는 비활성이다(기존 동작 유지)
    - `Exchange`에 `ExportAsk`·`ExportConfirmEmpty`가 없고, 그 둘을 그리던 함수 셋이 소스에서 사라졌다(`grep -rn "ExportAsk\|ExportConfirmEmpty\|show_export_ask\|show_export_empty_confirm\|request_export_file" src/` → 0건)
    - 제거 대상 i18n 키 9건이 카탈로그와 소스 양쪽에서 사라지고(`grep` 0건), `site_export_passphrase`가 `site_import_passphrase`로 개명됐다(D6)
    - `EXEMPT_LITERALS`에서 4건이 빠지고 배열 길이가 31이다
    - **가져오기 암호 대화는 그대로 뜬다** — 구버전 파일을 골랐을 때만. 그 시험은 T2가 두는 `legacy_document` 헬퍼로 픽스처를 만들어 검증한다(M3 — 「유지」가 아니라 **재작성**이다)
    - `ui::dialog::대화는_모두_이_모듈을_거친다`·`i18n` 소스 훑기 시험 통과
    - 상태 전이 시험이 새 흐름(`Idle → ExportWaitFile → Idle`)을 덮는다
  - **Files**:
    - 주: `src/ui/site_manager/exchange.rs`
    - 동반: `src/i18n/mod.rs`(문구 9건 제거 + 1건 개명 + `EXEMPT_LITERALS` 4건 축소)
    - 테스트: `exchange.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 제거한 variant를 참조하던 분기가 남아 컴파일이 깨지는 경우 → **의도된 것이다**(컴파일러가 드러낸다)
    - 대화를 닫는 경로(`close`)가 여전히 `exchange`·`pending_file`을 비우는지 확인
    - 파일 대화를 기다리는 동안 관리자를 닫으면 요청이 버려진다(기존 동작 유지)
    - 개명한 키의 doc 주석이 엉뚱한 항목에 붙지 않게 한다(`strings!` 매크로 함정 — 위키 conventions)
  - **Halt Forecast**:
    - (i) 제거 후 남는 분기가 예상보다 많아 보임 → 컴파일 오류가 전수를 알려 주므로 추측이 필요 없다
  - **Depends on**: T2

- [x] T4. PRD·README·AGENTS.md 문면을 새 동작에 맞춘다
  - **Type**: B (문서 3종 + 화면 문구 1건 — 순수 문서만이 아니라 `site_export_done`이 화면에 나가므로 A로 두지 않는다)
  - **Acceptance**:
    - `docs/prd.md` FR-59에서 「사용자가 정한 암호로 봉해 담고」·「암호를 비우면 비밀번호를 빼고 저장하되 그 사실을 한 번 더 묻는다」가 **앱 내장 키로 언제나 담는다**로 바뀐다. **그 파일을 얻은 사람이 MOA로 풀 수 있다는 성질**을 요구사항 본문에 적는다(보관 주의가 요구의 일부다)
    - `docs/prd.md` FR-28의 예외 서술이 「사용자가 정한 암호」에서 「앱 내장 키」로 바뀐다
    - `docs/prd.md`의 **`## 결정 이력`**(그 문서의 실제 섹션명이다 — 새 절을 만들지 않는다)에 2026-08-20 항목이 더해진다 — **직전 항목의 번복임을 명시**하고 사유(사용자가 불편을 이유로 바꿨다)와 대가를 적는다
    - `README.md`의 세 문장(「그때 정한 암호로 봉해」·「암호를 비우고 저장하면」·「암호를 잊으면 다시 열 수 없습니다」)이 새 동작으로 바뀌고, **파일 보관 주의**가 사용자 언어로 한 줄 들어간다
    - `AGENTS.md` 「비밀번호」 항목의 두 번째 통로 서술이 내장 키로 바뀌고, **그 키가 실행 파일에서 추출 가능하다는 성질**을 함께 적는다 — 에이전트가 읽는 정본이라 이것이 없으면 이후 세션이 DPAPI급 보호로 오독한다
    - `docs/prd.md` Out of Scope(`:116`)의 **「사이트 설정의 가져오기/내보내기」가 해제**된다. 그 줄은 **여러 제외가 한 줄에 나열된 행**이므로 줄 전체가 아니라 **그 항목에만 취소선을 걸고 줄 끝에 재한정 주석**을 붙인다(줄 단위로 적용하면 SSH 키 인증·프록시 등 무관한 제외까지 취소된다). 해제하지 않으면 FR-59(Must)와 같은 문서 안에서 정면 충돌이 남는다
    - `docs/prd.md` FR-59의 **검증 방법 열**도 새 동작에 맞춘다 — 「틀린 암호 거부」가 이제 **직전 버전이 만든 암호 보호 파일** 경로 한정임을 적는다. **이 표현을 쓴다**(「사용자가 정한 암호」라고 쓰면 바로 아래 잔존 검사와 충돌한다)
    - **옛 문면 잔존 0** — `grep -n "암호를 비우면\|암호를 잊으면\|사용자가 정한 암호\|그때 정한 암호" docs/prd.md README.md AGENTS.md` → 0건
    - `site_export_done` 알림 문구에 **파일 보관 주의 한 조각**을 더한다 — 화면에서 그 파일의 성질을 알릴 유일한 자리이며, 대화를 늘리지 않고 이미 뜨는 알림에 얹는 것이라 D2의 「바로 저장」과 어긋나지 않는다. **한국어·영어 두 분기를 모두 채우고**(그 함수는 언어별로 갈린다), `unreadable > 0`이 겹칠 때의 문장도 정한다 — 권장 형태: 한국어 `사이트 N개를 저장했습니다 · 비밀번호가 함께 담겼습니다`(+ 기존 unreadable 꼬리), 영어 `Saved N sites · passwords included`(+ 기존 꼬리)
    - 세 문서 어디에도 실제 호스트·계정·비밀번호가 없고, **내장 키 값 자체를 문서에 적지 않는다**
  - **Files**:
    - 주: `docs/prd.md`
    - 동반: `README.md` · `AGENTS.md` · `src/i18n/mod.rs`(`site_export_done` 문구에 보관 주의 한 조각)
    - 테스트: 없음 — 문구·문서 수정이며 기존 소스 훑기 시험이 카탈로그 경유를 그대로 검사한다
  - **Edge Cases**:
    - `site_export_done`의 조합 — 사이트 0개 / `unreadable > 0`이 겹칠 때 문장이 어색하게 이어지지 않는지(두 언어 모두)
    - 취소선 처리가 같은 줄의 **다른 제외 항목**까지 걸지 않는지
    - 잔존 검사 grep이 **plan 문서 자신**을 대상에 넣지 않는지(대상은 `docs/prd.md`·`README.md`·`AGENTS.md` 셋뿐이다)
  - **Halt Forecast**:
    - (i) 없음 — 문서 문면 수정과 화면 문구 1건뿐이고 파괴적·외부 요소가 없다
  - **Depends on**: T1, T2, T3

## 사전 승인 항목 (일괄 승인 대상)

- T1 — `remote::envelope`에 공개 심볼 2개(`seal_with_app_key`·`open_with_app_key`)와 공개 상수 1개(`KDF_APP_KEY`) 추가
- T2 — `site_export::build`의 공개 시그니처 변경(인자 1개 제거, 호출부 1곳)
- T3 — `Exchange` variant 2개와 비공개 메서드 3개 제거, i18n 키 9건 제거 + 1건 개명 (구조·공개 표면 변경)
- T4 — `docs/prd.md` FR-59·FR-28 문면 개정 (요구 정본 변경, 직전 회차 결정의 번복)
- 전 task — `.moasites` v1 형식에 `kdf` 값 하나를 더하는 것(판 번호는 올리지 않는다 — D5)

## 불가피한 Halt (위임 불가)

- commit 이후의 push·`master` 병합·태그·릴리즈 — 이 plan의 위임 범위는 로컬 작업 브랜치 commit까지다
- 신규 패키지(crate) 추가가 필요해지는 경우 — 이 plan은 의존성 0 추가를 전제로 세웠다
- 파괴적 작업 — 이 plan에는 없다

## Verification Strategy

- 빌드: `cargo build`
- 단위·통합 테스트: `cargo test`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 서식: `cargo fmt --check`
- 수동 검증 (사용자 확인 필요):
  1. `내보내기` 클릭 → **대화 없이 곧바로** 파일 저장 창이 뜨는가
  2. 저장된 `.moasites`를 메모장으로 열어 비밀번호 평문이 없는가
  3. 그 파일을 가져오기 → **암호를 묻지 않고** 목록에 들어오는가, 비밀번호까지 살아 있는가
  4. (있다면) 직전 버전으로 만든 암호 보호 파일을 가져올 때만 암호를 묻는가
  5. 다른 PC에서 가져와 비밀번호까지 연결에 쓰이는가

## Phase Ledger

## Retry Ledger

## Progress Log

- **T1 완료** (`4ddee68`) — `remote::envelope`에 앱 내장 키 봉인 경로를 더했다. 두 열쇠가 `kdf` 값(`KDF_APP_KEY` / `KDF_NAME`)으로 갈리고 서로의 봉투를 열지 못한다. 시험 6건 신규.
- **T2 완료** — `site_export::build`가 인자를 잃고 언제나 내장 키로 봉한다. `needs_passphrase`는 사용자 암호 봉투에만 참이고, `plan_import`은 `kdf`로 갈라 연다(구버전 파일 호환). 연쇄로 `finish_export`의 `passphrase` 인자·`ExportWaitFile.pass` 필드·`request_export_file` 인자가 사라졌다. 시험 픽스처 `legacy_document` 신설.
  - 리뷰: code-quality 지적 0. spec-compliance **MAJOR 1건**(`secret: None` 갈래를 통과시키는 시험 부재) → `봉투가_없는_구버전_파일은_설정만_들여온다`로 해소. 함께 Edge Cases 둘(구버전+빈 암호 / 모르는 `kdf`)의 단언도 더했다.
  - `cargo test --lib` 907 passed · `cargo clippy --all-targets -- -D warnings` 경고 0 · `cargo fmt --check` 통과.
- **T3 완료** — `Exchange::ExportAsk`·`ExportConfirmEmpty`와 `show_export_ask`·`show_export_empty_confirm`·`request_export_file`이 사라졌다. `내보내기` 버튼이 곧바로 `FileRequest::Save`를 청한다. i18n 9건 제거·`site_export_passphrase` → `site_import_passphrase` 개명·`EXEMPT_LITERALS` 35→31. 시험 8곳 재작성.
  - 리뷰: spec-compliance·code-quality 둘 다 지적 0.
  - `cargo test --lib` 907 passed · clippy 경고 0 · fmt --check 통과.
- **T4 완료** — PRD FR-59·FR-28 문면과 검증 방법 열, Out of Scope 취소선(해당 항목만), `## 결정 이력`의 번복 항목, README 사용자 문장, AGENTS.md의 「DPAPI급 보호가 아니다」 서술을 새 동작에 맞췄다. `site_export_done` 알림에 「비밀번호가 함께 담겼습니다」/「passwords included」를 두 언어로 얹고 조합 6가지를 고정하는 시험을 더했다.
  - 리뷰: spec-compliance·code-quality 둘 다 지적 0.
  - `cargo test` 916 passed(단위 908 + 통합 8) · clippy 경고 0 · fmt --check 통과 · `cargo build --release` 성공.

## Next Steps

- 권장 다음 액션: 사용자 수동 검증(Verification Strategy의 5항목) 후 master 병합·push 승인

## Open Questions

- [x] Q1. 암호를 없앤 뒤 비밀번호를 무엇으로 보호할까 — **앱 내장 키로 암호화**(D1). 파일을 얻은 사람이 MOA로 풀 수 있다는 대가를 사용자가 확인하고 택했다.
