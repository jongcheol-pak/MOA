# Plan: 릴리즈 빌드로 NSIS 설치파일 만들기 (설정을 설치 폴더로)

**PRD**: `docs/prd.md` (NFR-7 개정을 포함한다 — T5)

## 요구 이해

- **원문 요청**: "릴리즈 빌드시 nsis 설치파일을 만들수 있게 해줘"
- **추가 요청(2026-08-21)**: "설정 파일 위치를 설치 폴더로 변경 / 설치 시 설정 파일 삭제 체크 기능 삭제하고 무족건 삭제 / 바로가기 및 시작 메뉴에 바로가기의 이름을 한글 '모아', 영문 'MOA' 로 표시"
- **이해한 요구**: 지금 배포 수단은 `cargo build --release`가 만드는 `moa.exe` 하나뿐이다. 여기에 **NSIS 설치 프로그램을 만드는 길**을 더한다 — 릴리즈 exe를 담아 `%LOCALAPPDATA%\Programs\MOA`에 **사용자 단위로** 설치하고, 시작 메뉴·바탕화면 바로가기를 만들며, 제거할 때 자동 실행 등록을 지우고 설정을 지울지 묻는 설치 파일이다. 진입점은 기존 생성기 관례를 따라 `cargo run --example gen_installer`. **추가로 셋을 함께 바꾼다** — ① 앱이 설정을 `%APPDATA%\MOA`가 아니라 **exe 옆**(설치 폴더)에 두게 하고 ② 제거할 때 설정 삭제를 **묻지 않고 폴더째 지우며** ③ 바로가기 이름을 설치 언어에 따라 **'모아'(한국어)·'MOA'(영어)**로 만든다.
- **포함하지 않는 것으로 이해**: 이 PC에 NSIS를 설치하는 것(사용자 결정 — 안내만 한다), 코드 서명, 자동 업데이트, **기존 `%APPDATA%\MOA` 설정의 이사**(사용자 결정 — 무시하고 새로 시작).

## Goal

`cargo build --release` 뒤에 `cargo run --example gen_installer`를 돌리면 사용자 단위 설치 프로그램(`MOA-Setup-<버전>.exe`)이 만들어지고, 그 설치본은 **설정을 자기 폴더 안에** 두어 제거하면 흔적이 남지 않는다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| NFR-7 (설정·세션 저장 위치) | — | T1·T5 | ✅ 커버 (문면 개정 포함 — 경로가 exe 옆으로 바뀐다) |
| FR-47 (앱 설정 — 설명줄이 저장 경로를 인용) | Should | T5 | ✅ 커버 (경로 서술만 개정 — 기능은 그대로) |
| 그 밖의 active Must FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- **코드 서명(Authenticode)** — 인증서가 없다. 서명 없는 설치 파일은 SmartScreen 경고가 뜨며, 그것은 이번 요청 밖이다.
- **자동 업데이트·설치 후 실행 중 교체** — 설치 프로그램은 파일을 놓기만 한다.
- **MSI·Store 패키지** — 요청이 NSIS다.
- **NSIS 자체 설치** — 사용자가 `winget install NSIS.NSIS`로 직접 설치한다(2026-08-21 사용자 결정).
- **기존 `%APPDATA%\MOA` 설정을 새 위치로 옮겨 오기** — 사용자가 「무시하고 새로 시작」을 골랐다(2026-08-21). 옛 파일은 지우지도 읽지도 않고 그 자리에 남는다.
- **설정 위치를 사용자가 고르게 하는 옵션**(포터블/설치 모드 전환) — 요청에 없다.

## Deferred / Follow-up

- **설치 파일 실제 생성·검증** — 이 PC에 `makensis`가 없어 이번 회차에서는 설치 파일을 만들어 볼 수 없다. NSIS 설치 후 `cargo run --example gen_installer`를 돌리는 것이 첫 실검증이며, 아래 HUMAN-VERIFY가 그 목록이다.
- **릴리즈·태그 규약 문서화** — 지금 이 저장소에는 "버전을 올리고 태그를 달고 GitHub 릴리즈에 설치 파일을 올린다"는 절차가 문서화돼 있지 않다. 설치 파일이 생기면 그 자리가 필요해지지만, 이번 요청 범위 밖이라 다음 회차로 남긴다.

## Investigation Log

- 위키 참조: 관련 위키 자료 없음 — `20_projects/personal/moa/`에서 `NSIS`·`설치 파일`·`installer` 무매칭(한/영 양방향). 코드 1차 출처로 진행.
- 위키 참조: `20_projects/personal/moa/decisions.md` — 배포·패키징에 관한 과거 결정 없음(가장 가까운 것이 2026-08-21 「레포 오픈소스 고지 파일은 생성기가 만든다」이며 이번 설계의 관례 근거로만 쓴다).
- `makensis` 부재 실측: `where makensis` 결과 없음 · `C:\Program Files\NSIS`·`C:\Program Files (x86)\NSIS` 부재 · `winget list --id NSIS.NSIS` → "설치된 패키지를 찾을 수 없습니다".
- 빌드 산출물: `target/release/moa.exe` 실재(8,946,176바이트 실측). `[profile.release]`는 `opt-level="s"`·`lto=true`·`strip=true`(`Cargo.toml:92-96`).
- 아이콘·매니페스트: `build.rs`가 링커 인자로 exe에 직접 담는다(build-dependency 없음). 설치 프로그램·바로가기가 쓸 아이콘 원본은 `docs/AppIcon.ico` 실재.
- 동봉 후보 문서: 레포 루트에 `LICENSE`(MIT)와 `THIRD-PARTY-NOTICES.md` 실재.
- 생성기 관례: `examples/gen_licenses.rs`·`examples/gen_app_icon.rs` 둘 다 `fn main() -> Result<(), String>` + `env!("CARGO_MANIFEST_DIR")`로 레포 루트를 잡고 stdout으로 결과를 알린다(AGENTS 「예제 타깃」 규약).
- 런타임 상태의 자리(**추가 요구 전 상태**): 설정은 `%APPDATA%\MOA\settings.json`, 지문은 같은 폴더의 `known_hosts.json`(AGENTS 「데이터 접근」) — T1이 이 둘을 exe 옆으로 옮긴다(D10). 자동 실행 정본은 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`의 `MOA` 값.
- `.gitignore`는 `/target` 한 줄뿐 — 산출물을 `target/` 아래 두면 추가 등재가 필요 없다.
- 시험 배치: `tests/`에 통합 시험 4개(`layout_flow`·`remote_concurrency`·`transfer_memory`·`watcher`)가 있어 레포 파일을 읽는 시험을 둘 자리가 있다.
- PRD 대조: `docs/prd.md`에 배포·설치·패키징을 다루는 FR·NFR이 없다(`배포`·`설치`·`단일 exe` 검색 — 걸리는 것은 라이선스 고지 서술과 NFR-2 메모리뿐). **설치 프로그램만 놓고 보면** active FR/NFR에 닿지 않았다 — 그러나 추가 요구(설정 위치)가 NFR-7·FR-47 설명줄에 닿아 이 판단은 D10에서 뒤집혔고 `**PRD**:` 줄을 달았다.
- 설정 경로 구현 실측: `src/app/settings.rs:455-457`(`settings_path`)·`src/remote/hostkey.rs:113-115`(`known_hosts_path`) 둘 다 `std::env::var_os("APPDATA")`로 `%APPDATA%\MOA\<파일>`을 만든다. **두 파일 모두 옛 앱 이름 폴더에서 복사해 오는 `migrate_from_legacy_dir`**를 갖고 있다(`settings.rs:464-481`·`hostkey.rs:121-135`, 상수 `LEGACY_APP_DIR = "FileExplorer"`).
- PRD 대조(추가 요구 뒤): **NFR-7이 경로를 명시한다** — 「설정·세션은 `%APPDATA%\MOA\settings.json`에 저장」(`docs/prd.md:104`)이고 FR-47 설명줄(`:72`)도 같은 경로를 인용한다. 설정을 exe 옆으로 옮기면 이 둘이 거짓이 되므로 **PRD 개정이 필요하고**, 그래서 `**PRD**:` 줄을 연결했다(D10).
- `%APPDATA%` 참조 전수(`grep -rn APPDATA src`): `settings.rs` 4곳·`hostkey.rs` 3곳·`i18n/mod.rs` 주석 2곳(문구 규약 설명)·`fonts.rs`는 `LOCALAPPDATA`(글꼴 캐시, 무관). 실제로 고칠 곳은 앞의 둘이다.
- Deferred 대장 조회: `docs/plans/deferred.md` `## 대기` 107건(실측) — 설치·배포·패키징 관련 항목 없음(`설치`·`installer`·`NSIS`·`배포`·`패키징`·`msi`·`zip` 검색). 소진 batch 임계(잔량 100 초과 + 신규 30건 / 최소 판정일 30일 초과 / 절대 상한 130) 미달.

### 전제 검증

| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 릴리즈 exe가 `target/release/moa.exe`에 나온다 | `ls -la target/release/moa.exe` — 실재(8.9MB) · `Cargo.toml` name=`moa` | ✅ |
| 2 | 예제 타깃은 stdout 출력과 `main -> Result<_, String>`을 쓸 수 있다 | AGENTS.md 「예제 타깃(`examples/`)」 · `examples/gen_app_icon.rs:18` | ✅ |
| 3 | 설치 프로그램·바로가기가 쓸 아이콘 파일이 있다 | `ls docs/AppIcon.ico` — 실재 | ✅ |
| 4 | 이 PC에 `makensis`가 없어 이번 회차에 설치 파일을 만들어 볼 수 없다 | 위 Log의 3중 확인(PATH·Program Files 2곳·winget) | ✅ |
| 5 | 예제는 `CARGO_PKG_VERSION`으로 `Cargo.toml` 버전을 읽을 수 있다 | Cargo가 모든 타깃에 주는 환경변수 — `examples/gen_licenses.rs`가 `CARGO_MANIFEST_DIR`을 같은 방식(`env!`)으로 쓴다 | ✅ |
| 6 | NSIS는 명령줄 `/D<이름>=<값>`으로 스크립트에 값을 넘길 수 있고 스크립트는 `!ifndef`로 기본값을 둘 수 있다 | NSIS 사용법 — **이 PC에 makensis가 없어 실행으로 확인하지 못했다.** | ⚠ 미확인 — **빌드 성립은 좌우하지 않는다**(`!ifndef VERSION` 기본값이 있어 값이 안 와도 스크립트가 돈다). 다만 **버전이 실제로 반영되는지는 좌우한다** — 전달이 실패하면 산출물이 `MOA-Setup-0.0.0-dev.exe`가 되어 Goal 문면과 어긋나므로, 그 확정은 HUMAN-VERIFY 1이다 |
| 7 | `/target`이 gitignore돼 있어 산출물을 그 아래 두면 커밋되지 않는다 | `.gitignore` 전문 1줄 | ✅ |
| 8 | **T1 이후 앱은 설치 폴더에 `settings.json`·`known_hosts.json`을 쓴다**(추가 요구로 뒤집혔다 — 종전 전제는 「설치 경로에 아무것도 쓰지 않는다」였다) | T1 Design ② · D10 · `src/app/settings.rs:455`·`src/remote/hostkey.rs:113` | ✅(변경됨) — 제거 목록이 그 둘을 포함해야 한다(T2 ⓓ) |
| 9 | `makensis`는 스크립트가 있는 폴더를 작업 디렉터리로 삼는다(`/NOCD`를 주지 않는 한) | **미확인** — 그래서 T2가 `Command::current_dir(installer/)`로 **명시적으로 맞춘다**(D8). 두 동작 중 어느 쪽이든 `.nsi`의 상대경로가 같은 자리를 가리키게 만드는 설계라 이 전제에 기대지 않는다 | ⚠ 미확인(설계로 우회 — D8) |
| 11 | 설정·지문 파일 경로를 만드는 자리가 두 함수뿐이다 | `settings_path`(`settings.rs:455`)·`known_hosts_path`(`hostkey.rs:113`) — 다른 곳은 이 둘을 부른다(`:486`·`:500`·`:56`·`:94`) | ✅ |
| 12 | 앱은 `current_exe()`로 자기 위치를 알 수 있고 그 폴더는 per-user 설치에서 쓰기 가능하다 | `src/app/autostart.rs:94`가 이미 `std::env::current_exe()`를 쓴다 · 설치 위치가 `%LOCALAPPDATA%\Programs\MOA`(D4)라 사용자 권한으로 쓸 수 있다 | ✅ |
| 13 | PRD NFR-7이 설정 경로를 명시해 이번 변경이 그것을 거짓으로 만든다 | `docs/prd.md:104`·`:72` | ✅ |
| 10 | 자동 실행 레지스트리 값은 `"<exe 경로>" --tray` 형식이다 | `src/app/autostart.rs:89-95`(`command_line`) · 값 이름 `MOA`(`:21`) | ✅ |

## Risks & Unknowns

| 위험 | 영향 | 완화책 |
|---|---|---|
| `makensis`가 없어 이번 회차에 산출물을 만들어 볼 수 없다 | 문법 오류가 있어도 이 회차에서 드러나지 않는다 | T4의 훑기 시험이 T2 Acceptance ⓐ~ⓖ **전부**를 1:1로 단언해 요구 항목 누락을 기계로 막고, 문법·실동작은 HUMAN-VERIFY로 분리해 보고한다(빌드 통과를 동작 확인으로 단정하지 않는다) |
| 설치 중 앱이 실행 중이면 exe를 덮어쓸 수 없다 | 설치가 실패하거나 파일이 잠긴 채 남는다 | 설치 시작 시 안내 문구를 띄운다(D5) — 프로세스 감지 플러그인(`nsProcess`)은 외부 의존이라 들이지 않는다 |
| 제거하면 사이트 목록·봉인된 비밀번호·지문이 함께 사라진다 | 되돌릴 수 없는 손실 | **묻지 않는 것이 사용자 결정**(D11)이라 막지 않는다 — 대신 그 사실을 README와 설치 마지막 페이지에 적어 사전에 알린다 |
| 버전을 손으로 적으면 `Cargo.toml`과 어긋난다 | 설치 파일 버전이 앱 버전과 다르게 배포된다 | 예제가 `CARGO_PKG_VERSION`을 읽어 `/DVERSION`으로 넘긴다(D2) |

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| (신규 파일) `installer/moa.nsi` | — | 새 파일. 기존 코드가 참조하지 않는다 |
| (신규 파일) `examples/gen_installer.rs` | Cargo가 자동 인식(`[[example]]` 선언 불요 — 기존 두 예제도 선언이 없다) | 새 개발용 타깃 |
| (신규 파일) `tests/installer.rs` | Cargo가 자동 인식 | 새 통합 시험 |
| `settings_path`(`src/app/settings.rs:455`) | 같은 파일 `:486`·`:500`(호출자 둘) | 경로 계산이 `%APPDATA%` → `current_exe()` 부모로 바뀐다 |
| `known_hosts_path`(`src/remote/hostkey.rs:113`) | 같은 파일 `:56`·`:94`(호출자 둘) | 같은 변경 |
| `migrate_from_legacy_dir` 둘(`settings.rs:464`·`hostkey.rs:121`) | 호출자 `settings.rs:501`·`hostkey.rs:59`(둘 다 같은 파일) | **제거**(D12) — 시험 참조 0건(실측) |
| `LEGACY_APP_DIR`·`APP_DIR` 상수 각 둘 | 위 함수들이 유일 사용처 | **제거** — 남기면 `dead_code`로 `-D warnings`가 실패한다 |
| 그 밖의 앱 코드(`src/**`) | — | **변경 없음** — UI·전송·원격 동작은 그대로다 |
| `README.md`·`AGENTS.md` | 문서 | 빌드 명령·산출물 위치 서술 추가 |

### 4-B. 계약·직렬화 변경

- **형식은 그대로, 위치가 바뀐다.** `settings.json` 스키마(v3)·`known_hosts.json` 형식·레지스트리 값 형식은 손대지 않는다. 다만 **읽고 쓰는 자리가 `%APPDATA%\MOA`에서 exe 옆으로 옮겨져 기존 사용자의 설정이 읽히지 않는다**(이사하지 않는 것이 사용자 결정 — Out of Scope·D12). 옛 파일은 그 자리에 남으므로 되돌리려면 손으로 복사하면 된다. 설치 프로그램은 HKCU Run 값을 **제거할 때만** 건드린다(D9).

### 4-C. 테스트 파일

- `tests/installer.rs` — 신규. `.nsi`가 요구 항목을 담고 있는지 훑는다(T4).
- `src/app/settings.rs`·`src/remote/hostkey.rs`의 `#[cfg(test)] mod tests` — **경로 함수·마이그레이션을 참조하는 시험은 0건**(실측: 직렬화만 다룬다). T1이 경로 단언 시험을 새로 더한다.

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `examples/gen_installer.rs`의 `main` | `gen_licenses.rs`·`gen_app_icon.rs`가 같은 골격(`CARGO_MANIFEST_DIR` + `Result<(), String>` + stdout) | **관례를 재사용**한다 — 골격을 공통 모듈로 빼지 않는다(예제 셋이 서로 다른 일을 하고 공통부는 3줄뿐이다) |
| `makensis` 탐색 함수 | 레포에 외부 실행 파일을 찾는 코드 없음(`Command::new` 검색 결과 0건) | 신규. NSIS 설치 위치가 PATH·`Program Files`·`Program Files (x86)` 셋으로 갈려 그 자리에서만 쓴다 |
| 설치 프로그램 자체 | 없음 | 신규 |
| 아이콘·라이선스 자산 | `docs/AppIcon.ico`·`LICENSE`·`THIRD-PARTY-NOTICES.md` | **그대로 재사용** — 설치 파일용 자산을 새로 만들지 않는다 |

### Verified by

- `grep -rn "makensis\|NSIS\|installer" src examples tests build.rs` → 0 hits (신규 도입임을 확인)
- `grep -rn "Command::new" src examples build.rs` → 0 hits (외부 프로세스 호출 선례 없음 — T2가 첫 사례)
- `ls examples/ tests/` → 기존 예제 2개·통합 시험 4개, 이름 충돌 없음
- `cat .gitignore` → `/target` 1줄, 산출물 경로가 이미 제외됨

## 동반 변경 판정

| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `AGENTS.md`의 「Build & Test」 | 새 빌드 명령이 생기는데 그 문서가 빌드 명령의 정본이다 — 적지 않으면 다음 세션이 이 길을 모른다 | T5 |
| 필수 | `README.md` | 설치 방법이 "exe를 직접 실행"뿐인 서술로 남으면 실제와 어긋난다 | T5 |
| 필수 | `AGENTS.md`의 「산출물·파일 관리」 | 새 산출물(`target/installer/`)과 **커밋되지 않는다**는 사실을 그 절이 다룬다 | T5 |
| 필수 | `AGENTS.md:112`의 「배포: 단일 exe (cargo build --release)」 | 배포 수단이 둘이 되는데 그 줄이 하나라고 말한다 — 이번 변경이 거짓으로 만든다 | T5 |
| 필수 | `AGENTS.md`의 Repository Structure 트리 | 새 폴더 `installer/`와 새 파일 `examples/gen_installer.rs`가 트리에 없다 | T5 |
| 필수 | `README.md`의 구조 트리 | 같은 이유 | T5 |
| 필수 | `docs/prd.md` NFR-7·FR-47 설명줄(`:104`·`:72`) | 설정 경로를 `%APPDATA%\MOA\settings.json`으로 명시한다 — 이번 변경이 그것을 거짓으로 만든다 | T5 (D10) |
| 필수 | `AGENTS.md`의 「데이터 접근」 절 | 그 절이 설정 파일 위치의 정본이다 | T5 |
| 필수 | `README.md`의 세션 저장 설명 | 사용자용 서술이 실제와 어긋난다 | T5 |
| 필수 | `src/app/settings.rs`·`src/remote/hostkey.rs`의 모듈·doc 주석 | 경로를 서술하는 주석이 그 파일 안에 있다(자기 유발 stale) | T1 |
| 무관 | `assets/licenses.json`·`THIRD-PARTY-NOTICES.md` | 의존성이 늘지 않는다(NSIS는 빌드 도구이지 링크되는 구성 요소가 아니다) | 건드리지 않음 |
| 필수 | `src/app/settings.rs`·`src/remote/hostkey.rs`의 경로 함수와 `migrate_from_legacy_dir`·`LEGACY_APP_DIR`·`APP_DIR` | 설정 위치를 옮기는 요구 자체가 이 코드를 바꾸는 일이다 | T1 |
| 무관 | 그 밖의 `src/**` | 설치 수단만 더하고 앱 동작(UI·전송·원격)은 바꾸지 않는다 | 건드리지 않음 |

## Decisions

### D1. `.nsi`는 손으로 쓴 소스로 두고 예제는 그것을 호출만 한다
- **Options**: A) 예제가 `.nsi`를 통째로 생성 / B) `.nsi`를 레포에 두고 예제는 `makensis`를 호출
- **Chosen**: B
- **Rationale**: 설치 스크립트는 사람이 읽고 고치는 소스다 — 생성물로 만들면 diff가 통째로 흔들리고 손으로 실험하기도 어렵다. `gen_licenses`가 자산을 생성하는 것과 성격이 다르다(그쪽은 의존성 목록이라 손으로 유지할 수 없다).
- **Source**: AGENTS 「산출물·파일 관리」의 생성물 셋 정의(전부 기계가 만드는 자료다)

### D2. 버전은 `CARGO_PKG_VERSION`을 `/DVERSION`으로 넘긴다
- **Options**: A) `.nsi`에 손으로 적기 / B) 예제가 `Cargo.toml` 버전을 읽어 명령줄로 넘기기
- **Chosen**: B
- **Rationale**: 손으로 적으면 버전을 올릴 때마다 두 곳을 고쳐야 하고 곧 어긋난다. `.nsi`에는 `!ifndef VERSION`로 기본값(`0.0.0-dev`)을 둬 손으로 `makensis`를 돌려도 빌드는 성립하게 한다.
- **Source**: `Cargo.toml:3`(version = "0.1.0"), 예제가 이미 `env!`로 빌드 환경변수를 쓴다(`gen_app_icon.rs:19`)
- **넘기는 값은 `VERSION` 하나다** — 산출물 이름·경로는 `.nsi`가 그 값으로 스스로 조립한다(`OutFile ..\target\installer\MOA-Setup-${VERSION}.exe`). `/D`로 넘기는 값이 늘수록 미확인 전제 6에 얹히는 무게가 커지므로 최소로 둔다.

### D8. 경로 기준은 `makensis`의 작업 디렉터리로 못 박는다
- **Options**: A) `.nsi`가 `${__FILEDIR__}`로 자기 위치를 잡는다 / B) 예제가 `current_dir`을 `installer/`로 지정하고 `.nsi`는 그 기준의 상대경로만 쓴다 / C) 예제가 모든 경로를 `/D`로 넘긴다
- **Chosen**: B
- **Rationale**: `makensis`가 스크립트 폴더로 작업 디렉터리를 옮기는지(전제 9)를 이 PC에서 확인할 수 없다. B는 **예제가 그 디렉터리를 명시적으로 지정**하므로 makensis의 기본 동작이 어느 쪽이든 같은 자리를 가리킨다 — 미확인 전제에 기대지 않는 유일한 안이다. A는 `${__FILEDIR__}`(NSIS 3 내장)에 다시 기대야 하고, C는 넘기는 값을 늘려 전제 6의 무게를 키운다.
- **적용**: `.nsi`가 쓰는 경로는 전부 `installer/` 기준 상대경로다 — 담는 파일 `..\target\release\moa.exe`·`..\LICENSE`·`..\THIRD-PARTY-NOTICES.md`, 설치 프로그램 아이콘 `..\docs\AppIcon.ico`, 산출 `..\target\installer\MOA-Setup-${VERSION}.exe`.
- **Source**: 전제 9(미확인), `std::process::Command::current_dir`

### D9. 제거 시 자동 실행 값은 **이 설치본을 가리킬 때만** 지운다
- **Options**: A) 무조건 지운다 / B) 값이 이 설치 경로를 가리킬 때만 지운다
- **Chosen**: B
- **Rationale**: 이 레포는 `cargo run --release`로도 자동 실행을 켤 수 있어, 그 값이 개발 빌드를 가리키는 상태에서 설치본을 제거하면 A는 남의 설정을 지운다. 앱이 쓰는 값 형식이 `"<exe 경로>" --tray`로 확정돼 있어(전제 10) **정확히 `"$INSTDIR\moa.exe" --tray`와 같을 때만** 지우면 된다 — 부분 문자열 검사가 필요 없어 외부 헤더도 들이지 않는다.
- **Source**: `src/app/autostart.rs:89-95`·`:21`

### D3. `makensis`가 없으면 오류로 끝내되 설치 방법을 알린다
- **Options**: A) 조용히 건너뛴다 / B) 오류 종료 + `winget install NSIS.NSIS` 안내
- **Chosen**: B
- **Rationale**: 설치 파일을 만들라고 부른 명령이 아무것도 만들지 않고 성공으로 끝나면 그것이 더 나쁘다. 예제 타깃은 오류를 종료 코드에 실을 수 있다(AGENTS 규약).
- **Source**: AGENTS 「예제 타깃」, 사용자 결정(2026-08-21 — NSIS 설치는 안내만)

### D4. 사용자 단위 설치 (`%LOCALAPPDATA%\Programs\MOA`)
- **Options**: A) 사용자 단위 / B) 시스템 전체(Program Files) / C) 설치 시 선택
- **Chosen**: A (사용자 결정 2026-08-21)
- **Rationale**: 이 앱은 설정을 `%APPDATA%`에, 자동 실행을 `HKCU`에 쓴다 — 설치만 시스템 전체로 두면 권한 모델이 갈린다. UAC 없이 설치·제거가 끝나는 것도 이 앱의 성격(개인용 파일 탐색기)에 맞는다.
- **Source**: AGENTS 「데이터 접근」(설정·레지스트리 위치)

### D5. 실행 중 감지에 외부 플러그인을 쓰지 않는다
- **Options**: A) `nsProcess` 플러그인으로 프로세스 감지·자동 종료 / B) 안내 문구만 띄우고 NSIS 기본 처리에 맡긴다
- **Chosen**: B
- **Rationale**: 플러그인은 NSIS 기본 배포에 없어 빌드하는 사람이 따로 설치해야 한다 — 최소 의존 원칙(4-D)에 어긋난다. 파일이 잠겨 있으면 NSIS가 「다시 시도/무시」 대화를 스스로 띄우므로 진행이 막히지도 않는다.
- **Source**: 4-D 최소 의존 원칙, NSIS 기본 배포 구성

### D6. PRD는 **설정 위치 때문에** 건드린다 (추가 요구로 뒤집힌 결정)
- **Options**: A) 배포·설치 FR을 새로 추가 / B) 건드리지 않는다 / C) 설정 위치 서술만 고친다
- **Chosen**: C
- **Rationale**: 설치 프로그램 자체는 여전히 PRD의 FR이 아니다(그 문서는 앱 동작을 서술하고 설치는 전달 수단이다 — A를 택하지 않는 근거). 그러나 **설정 위치를 옮기는 추가 요구**(D10)가 NFR-7(`:104`)과 FR-47 설명줄(`:72`)을 직접 거짓으로 만든다 — 그래서 B(처음 판단)를 뒤집고 그 두 자리만 고친다. 배포·패키징 절차는 종전대로 AGENTS·README가 맡는다.
- **Source**: `docs/prd.md:104`·`:72`(경로 명시), 2026-08-21 추가 요구

### D7. 산출물은 `target/installer/`에 둔다
- **Options**: A) 레포 루트 / B) `dist/` 신설 / C) `target/installer/`
- **Chosen**: C
- **Rationale**: `.gitignore`가 `/target` 한 줄뿐이라 그 아래면 추가 등재 없이 커밋에서 빠진다. `dist/`를 새로 만들면 gitignore도 함께 고쳐야 하고, 빌드 산출물이 두 곳으로 갈린다.
- **Source**: `.gitignore` 전문, AGENTS 「산출물·파일 관리」(빌드 산출물은 `target/`)

### D10. 설정을 exe 옆에 두고 PRD NFR-7을 고친다
- **Options**: A) `%APPDATA%` 유지 / B) exe 옆(설치 폴더) / C) 설치 시 선택
- **Chosen**: B (사용자 결정 2026-08-21)
- **Rationale**: 요청이 그것이다. 부수 효과가 둘 — ⓐ 제거하면 설정이 함께 사라져 「무조건 삭제」(D11)가 자연히 성립하고 ⓑ 개발 실행과 설치본이 서로 다른 설정을 갖는다(사용자가 그대로 두기로 했다). PRD NFR-7이 경로를 명시하므로 그 문면도 함께 고친다 — 고치지 않으면 승인된 요구 문서가 거짓이 된다.
- **Source**: `docs/prd.md:104`·`:72`, `src/app/settings.rs:455`, `src/remote/hostkey.rs:113`

### D11. 제거 시 설정을 묻지 않고 지운다
- **Options**: A) 묻는다(직전 계획) / B) 묻지 않고 지운다
- **Chosen**: B (사용자 결정 2026-08-21)
- **Rationale**: 설정이 `$INSTDIR` 안에 있으므로 폴더를 지우는 것이 곧 설정을 지우는 것이다 — 묻는 대화를 남기면 「예」와 「아니오」가 같은 결과를 내거나(폴더째 삭제) 폴더만 남는 어중간한 상태가 생긴다. 다만 그 파일 둘은 삭제 목록에 **이름으로 명시**해 `RMDir`가 실패하지 않게 한다.
- **Source**: 사용자 결정, T1의 새 경로

### D12. 기존 `%APPDATA%` 설정을 읽어 오지 않고 마이그레이션 코드를 지운다
- **Options**: A) 첫 실행 때 옮겨 온다 / B) 무시한다 / C) 옮기고 원본도 남긴다
- **Chosen**: B (사용자 결정 2026-08-21)
- **Rationale**: 사용자가 「무시하고 새로 시작」을 골랐다. 그러면 두 파일에 있는 `migrate_from_legacy_dir`(옛 앱 이름 폴더에서 복사)도 함께 지워야 한다 — 남겨 두면 새 위치에 파일이 없을 때 `%APPDATA%\FileExplorer`에서 읽어 와 그 결정과 어긋난다. 옛 파일은 지우지 않는다(사용자가 직접 지울 수 있게 그 자리에 둔다).
- **Source**: 사용자 결정, `src/app/settings.rs:464-481`·`src/remote/hostkey.rs:121-135`

## Tasks

- [x] T1. 설정·지문 파일을 **exe 옆**에 둔다
  - **Type**: C
  - **Design**: ① `src/app/settings.rs`와 `src/remote/hostkey.rs` 두 곳 — 경로를 만드는 함수가 그 둘뿐이다(전제 11) ② `settings_path`·`known_hosts_path`가 `std::env::current_exe()`의 부모 폴더에 파일명을 붙여 돌려준다(하위 폴더를 두지 않는다 — 설치 폴더가 이미 그 앱의 자리다). 실패(`current_exe` 오류)면 종전처럼 `None`이고 저장·읽기는 조용히 넘어간다 ③ **`migrate_from_legacy_dir`·`LEGACY_APP_DIR`·`APP_DIR`를 두 파일에서 지운다**(`APP_DIR`은 하위 폴더를 두지 않으면 쓰임이 없어져 `dead_code`로 `-D warnings`를 깨뜨린다) — 사용자가 「기존 `%APPDATA%` 설정은 무시하고 새로 시작」을 골랐으므로(Out of Scope) 옛 폴더를 읽어 오는 경로가 남아 있으면 그 결정과 어긋난다 ④ 「저장 위치 전략」 같은 추상화를 만들지 않는다 — 두 함수가 각자 세 줄이다
  - **Acceptance**: Given 앱, When 설정을 저장하면, Then `settings.json`과 `known_hosts.json`이 **exe와 같은 폴더**에 생긴다(개발 실행이면 `target\debug\`, 설치본이면 설치 폴더 — 사용자 결정). **그 사실을 시험이 단언한다** — `settings_path()`·`known_hosts_path()`가 돌려주는 경로의 **부모가 `std::env::current_exe()`의 부모와 같고** 파일명이 각각 `settings.json`·`known_hosts.json`이다(각 파일의 `#[cfg(test)] mod tests`에서 그 비공개 함수를 직접 부른다 — 시험 바이너리에서도 성립한다). 더해 `%APPDATA%`를 읽는 코드가 `src/`에서 사라지고(`grep -rn 'var_os("APPDATA")' src` → 0건), 기존 시험이 모두 통과하며 빌드·clippy 경고가 0이다
  - **Files**:
    - 주: `src/app/settings.rs`, `src/remote/hostkey.rs`
    - 동반: 두 파일의 모듈 주석·doc 주석(경로 서술이 그 안에 있다) · `src/ui/app.rs:2022`의 `on_exit` 주석(옛 경로 `%APPDATA%\FileExplorer\settings.json`을 가리킨다)
    - 테스트: 두 파일의 `#[cfg(test)] mod tests`(경로 단언 신규)
  - **Edge Cases**:
    - `current_exe()`가 실패하는 비정상 환경 → `None`을 돌려 종전의 「저장 실패는 조용히 넘긴다」 규약을 그대로 탄다
    - 설치 폴더가 읽기 전용인 경우(사용자가 Program Files에 손으로 옮긴 경우) → 저장이 조용히 실패한다. per-user 설치(D4)에서는 생기지 않는다
    - 옛 `%APPDATA%\MOA` 파일은 읽지도 지우지도 않는다 — 그 자리에 그대로 남는다(Out of Scope)
    - 같은 폴더에 설치본이 둘일 수 없다(설치 경로가 고정) — 설정이 섞이는 경로가 없다
    - 개발 실행의 설정은 `target/debug/`에 생기므로 `cargo clean`으로 함께 사라진다 — 개발 편의상 의도된 결과다(설치본과 무관)
  - **Halt Forecast**:
    - (i) 시험이 `%APPDATA%`를 전제로 쓰였을 가능성 → 실측으로 그 전제를 쓰는 시험이 없음을 확인했다(경로 함수는 비공개이고 시험은 직렬화만 다룬다). 그래도 어긋나면 그 시험을 함께 고친다(Files의 「동반」)
  - **Depends on**: -

- [x] T2. `installer/moa.nsi` — 사용자 단위 설치 스크립트
  - **Type**: C
  - **Design**: ① 새 폴더 `installer/`에 `moa.nsi` 하나 ② **설치 구역**: `!ifndef VERSION` 기본값(`0.0.0-dev`) · `RequestExecutionLevel user` · `InstallDir $LOCALAPPDATA\Programs\MOA` · MUI2(한국어 + 영어) · `MUI_ICON ..\docs\AppIcon.ico` · 담는 파일 `..\target\release\moa.exe`·`..\LICENSE`·`..\THIRD-PARTY-NOTICES.md`(경로 기준은 D8) · `OutFile ..\target\installer\MOA-Setup-${VERSION}.exe` · 시작 메뉴 바로가기 · 바탕화면 바로가기(마지막 페이지 체크박스) — **이름은 설치 언어를 따른다**: 한국어 `모아`, 영어 `MOA`(`LangString SHORTCUT_NAME`으로 두고 두 언어에 값을 준다). **제거 구역은 두 이름을 모두 지운다** — 언인스톨러는 자기 언어를 새로 정하므로 설치 때와 갈릴 수 있고, `Delete`는 없는 파일에 관대해 두 번 지워도 해가 없다(설치 언어를 레지스트리에 남겨 되읽는 길보다 단순하다) · 언인스톨러 생성과 `HKCU\...\CurrentVersion\Uninstall\MOA` 등록(DisplayName·DisplayVersion·DisplayIcon·UninstallString·NoModify·NoRepair) ③ **제거 구역**: 바로가기 둘 삭제(두 이름 모두) + 시작 메뉴 폴더 `RMDir` · `$INSTDIR`의 파일을 **이름으로 다섯 개 열거해 삭제**(`moa.exe`·`LICENSE`·`THIRD-PARTY-NOTICES.md`·`uninstall.exe`·런타임 생성물 `settings.json`·`known_hosts.json`) 후 `RMDir $INSTDIR` · `DeleteRegKey HKCU ...\Uninstall\MOA` · 자동 실행 값은 **이 설치본을 가리킬 때만** 삭제(D9 — `StrCmp $0 '\"$INSTDIR\moa.exe\" --tray'` 형태로 **작은따옴표로 감싸** 큰따옴표를 리터럴로 넣는다) · **설정은 묻지 않는다** — 설정·지문 파일이 `$INSTDIR` 안에 있으므로(T1) 폴더를 지우면 함께 사라진다(2026-08-21 사용자 결정 — 직전 계획의 「삭제 여부를 묻는 대화」는 없앤다). `$INSTDIR`의 파일 삭제 목록에 `settings.json`·`known_hosts.json`을 **명시**한다(우리가 놓지 않은 파일까지 지우지 않으면서 그 둘은 확실히 지운다) ④ `gen_installer`가 `/DVERSION`만 주며 부른다 · 다국어 문구를 별도 `.nsh`로 쪼개지 않는다(문구가 열 줄 안팎이다)
  - **Acceptance**: **T4가 기계로 단언하는 일곱 가지와 같다** — ⓐ `RequestExecutionLevel user`·`$LOCALAPPDATA\Programs\MOA` ⓑ 시작 메뉴·바탕화면 바로가기의 **생성과 제거 양쪽** ⓒ 자동 실행 값 삭제(`$INSTDIR\moa.exe` 비교 포함) ⓓ 제거 구역이 `$INSTDIR\settings.json`·`$INSTDIR\known_hosts.json`을 지우고 **설정 삭제를 묻는 대화가 없다** ⓔ `!ifndef VERSION` 기본값 ⓕ `Uninstall\MOA` 키 삭제와 `RMDir $INSTDIR`·**시작 메뉴 폴더 `RMDir`** ⓖ **D8이 정한 경로 다섯**(`..\target\release\moa.exe`·`..\LICENSE`·`..\THIRD-PARTY-NOTICES.md`·`..\docs\AppIcon.ico`·`OutFile ..\target\installer\MOA-Setup-${VERSION}.exe`) ⓗ 바로가기 이름이 `LangString`으로 한국어 `모아`·영어 `MOA` 두 값을 갖는다. 이 여덟이 `cargo test`로 확인된다(하나를 지우면 T4가 실패한다)
  - **Files**:
    - 주: `installer/moa.nsi` (신규)
    - 참조(담기는 자산): `docs/AppIcon.ico`·`LICENSE`·`THIRD-PARTY-NOTICES.md`·`target/release/moa.exe`
  - **Edge Cases**:
    - `/DVERSION` 없이 손으로 `makensis`를 돌려도 빌드가 성립한다(기본값 `0.0.0-dev`)
    - 이미 설치돼 있으면 같은 자리에 덮어쓴다 — 언인스톨 정보를 같은 키에 다시 쓴다
    - 자동 실행 값이 **다른 경로**(개발 빌드 등)를 가리키면 제거해도 남긴다(D9)
    - 설정이 `$INSTDIR` 안에 있어 **제거하면 사이트 목록·봉인된 비밀번호·알려진 호스트 지문이 함께 사라진다** — 묻지 않는 것이 사용자 결정이다(D11)
    - 재설치·업그레이드는 같은 폴더를 덮어쓰므로 설정이 유지된다(제거를 거치지 않는 한 지워지지 않는다)
    - **앱이 실행 중일 때 제거하면** 종료 시 `settings.json`이 다시 쓰여 폴더가 남을 수 있다 — 제거 전에 앱을 닫으라는 안내를 띄운다(D5와 같은 자리)
    - `$INSTDIR`에 사용자가 넣어 둔 파일이 있으면 `RMDir`(재귀 아님)가 조용히 실패해 폴더가 남는다 — 우리가 놓은 파일만 지우는 것이 의도다
    - 바탕화면 바로가기를 만들지 않은 설치에서도 제거가 실패하지 않는다(`Delete`는 없는 파일에 관대하다)
  - **Halt Forecast**:
    - (i) 문법 오류를 이 회차에서 잡을 수 없다 → T4의 훑기 시험으로 **요구 항목 누락**만 기계로 막고, 문법은 HUMAN-VERIFY 1로 분리한다(Risks 표와 같은 판단)
  - **Depends on**: T1

- [x] T3. `examples/gen_installer.rs` — 릴리즈 exe 확인·`makensis` 탐색·호출
  - **Type**: C
  - **Design**: ① `examples/gen_installer.rs` 하나 ② 신규 심볼: `main`(절차) · `find_makensis() -> Option<PathBuf>`(PATH → `%ProgramFiles%\NSIS` → `%ProgramFiles(x86)%\NSIS` 순서) · `run(makensis, args) -> Result<(), String>` ③ 표준 라이브러리만 쓴다(`std::process::Command`·`std::env`·`std::path`) — 새 의존성 0. 넘기는 것은 `/DVERSION=<CARGO_PKG_VERSION>`과 `/INPUTCHARSET UTF8` 둘이다(뒤엣것은 값 정의가 아니라 **소스 인코딩 지정** — 이 레포는 BOM 없는 UTF-8이고 makensis는 BOM이 없으면 시스템 코드페이지로 읽어 `.nsi`의 한글 문구가 깨진다. T2 quality 리뷰 m1)이고, 작업 디렉터리를 `installer/`로 지정하며(D8), 산출 폴더(`target/installer/`)는 부르기 전에 만든다 ④ 「빌드 파이프라인」 추상화를 만들지 않는다: 이 예제 하나가 전부다
  - **Acceptance**: 두 실패 경로를 **이번 회차에 실제로 실행해** 확인한다. ① Given `target/release/moa.exe`를 같은 폴더에서 `moa.exe.bak`으로 **임시 rename**한 상태(삭제 금지 — `lto=true` 릴리즈 재빌드가 필요해진다), When `cargo run --example gen_installer`, Then exe가 없다는 것과 `cargo build --release`를 먼저 돌리라는 안내가 나오고 **종료 코드가 0이 아니다**. 확인 후 원래 이름으로 되돌리고 **`target/release/moa.exe`가 있고 `moa.exe.bak`이 남아 있지 않음을 확인한다**. ② Given exe는 있고 `makensis`가 없는 상태(이 PC 그대로), When 같은 명령, Then `winget install NSIS.NSIS` 안내와 함께 실패로 끝난다. 성공 경로(설치 파일 생성)는 makensis가 없어 이번 회차에서 확인할 수 없다 — HUMAN-VERIFY 1로 분리한다
  - **Files**:
    - 주: `examples/gen_installer.rs` (신규)
  - **Edge Cases**:
    - `%ProgramFiles(x86)%`가 없는 환경(순수 ARM64 등) → 그 후보를 건너뛰고 다음으로
    - `makensis`가 PATH에 있으나 실행이 실패(권한·손상) → 종료 코드와 함께 그 사실을 알린다
    - `target/installer/`가 없으면 만든다
    - 경로에 공백이 있다(`D:\Personal Project\...`) → 인자를 문자열로 조립하지 않고 `Command::arg`로 하나씩 넘긴다
    - `makensis`의 작업 디렉터리를 `installer/`로 **명시 지정**한다(D8) — `.nsi`의 상대경로가 그 기준이다
    - 확인용 rename(`moa.exe.bak`)을 되돌리지 않은 채 끝내지 않는다 — 되돌리기까지가 그 검증의 일부다
  - **Halt Forecast**:
    - (i) `makensis` 부재로 성공 경로를 못 밟는다 → 실패 경로 두 갈래는 실제로 실행해 검증하고, 성공 경로는 HUMAN-VERIFY로 분리(Acceptance가 그렇게 쓰여 있다)
  - **Depends on**: T2

- [x] T4. `tests/installer.rs` — `.nsi`가 요구 항목을 담고 있는지 훑는 시험
  - **Type**: C
  - **Design**: ① `tests/installer.rs`(통합 시험 관례 — 기존 4개와 같은 자리) ② 신규 심볼: 시험 함수 2개(`설치_스크립트는_사용자_단위로_설치한다`·`제거는_자동_실행과_설정_처리를_모두_담는다`) ③ `CARGO_MANIFEST_DIR`로 `installer/moa.nsi`를 읽어 문자열을 단언한다 — 앱 코드를 참조하지 않는다. 단언 목록은 T2 Acceptance ⓐ~ⓗ와 **1:1로 대응**한다(그 표가 정본이고 여기가 그 기계 판정이다) ④ NSIS 파서를 만들지 않는다(문자열 포함 검사로 충분하다 — 문법 검증은 `makensis`의 몫이다)
  - **Acceptance**: Given `installer/moa.nsi`, When `cargo test`, Then 두 시험이 통과한다 — ⓐ(설치) `RequestExecutionLevel user`·`$LOCALAPPDATA\Programs\MOA`·시작 메뉴/바탕화면 바로가기 생성·`!ifndef VERSION` 기본값·**D8 경로 다섯**(`..\target\release\moa.exe`·`..\LICENSE`·`..\THIRD-PARTY-NOTICES.md`·`..\docs\AppIcon.ico`·`OutFile ..\target\installer\MOA-Setup-${VERSION}.exe`) ⓑ(제거) 바로가기 삭제가 **두 이름(`모아`·`MOA`) 모두**·시작 메뉴 폴더 `RMDir`·삭제 목록에 다섯 파일(`moa.exe`·`LICENSE`·`THIRD-PARTY-NOTICES.md`·`uninstall.exe`)과 런타임 생성물 둘(`settings.json`·`known_hosts.json`)·`RMDir $INSTDIR`·`Uninstall\MOA` 키 삭제·자동 실행 값 삭제와 비교 문자열(`'\"$INSTDIR\moa.exe\" --tray'`)·**설정 삭제를 묻는 `MessageBox`가 없음**·`LangString`의 `모아`와 `MOA`. `.nsi`에서 그중 하나를 지우면 시험이 **실패한다**(임시로 지웠다 되돌려 회귀 검출을 실제로 확인한다)
  - **Files**:
    - 주: `tests/installer.rs` (신규)
    - 참조: `installer/moa.nsi`
  - **Edge Cases**:
    - 파일이 없으면 시험이 그 사실을 명확히 알리며 실패한다(`expect` 메시지 — 시험은 `unwrap`/`expect` 금지 예외다)
    - 줄바꿈(CRLF/LF)에 의존하지 않는 검사로 쓴다
  - **Halt Forecast**: 없음 — 파일을 읽어 단언하는 시험이다
  - **Depends on**: T2

- [ ] T5. 문서 갱신 — README·AGENTS
  - **Type**: A
  - **Acceptance**: ⓐ `README.md`에 설치 파일로 설치하는 방법과 만드는 방법(`cargo build --release` → `cargo run --example gen_installer`), NSIS 선행 설치(`winget install NSIS.NSIS`), 설치 위치(`%LOCALAPPDATA%\Programs\MOA`)와 **설정이 그 폴더 안에 생긴다는 것**, 제거 시 동작(자동 실행 정리 · **묻지 않고 폴더째 삭제 — 설정·지문 파일도 함께 사라진다**)이 적힌다 ⓑ `AGENTS.md`의 「Build & Test」에 그 명령이, 「산출물·파일 관리」에 `target/installer/`가 **커밋되지 않는 산출물**로 적힌다 ⓒ **`AGENTS.md`의 「배포: 단일 exe (cargo build --release)」 줄이 설치 파일까지 포함하도록 고쳐진다** ⓓ **양쪽 구조 트리(`AGENTS.md` Repository Structure·`README.md`)에 `installer/`·`examples/gen_installer.rs`·`tests/installer.rs`가 반영된다** ⓔ **`docs/prd.md` NFR-7과 FR-47 설명줄(`:72`)의 `%APPDATA%\MOA\settings.json` 서술이 새 위치(exe 옆)로 고쳐지고 `## 결정 이력`에 이번 항목이 더해진다** ⓕ **`AGENTS.md`의 「데이터 접근」 절과 「DO NOT」의 관련 서술, `README.md`의 세션 저장 설명이 새 위치로 고쳐진다** ⓖ 존재하지 않는 기능(코드 서명·자동 업데이트)은 적지 않는다
  - **Files**:
    - 주: `README.md`, `AGENTS.md`, `docs/prd.md`
  - **Edge Cases**: 없음 — 문서만 고친다
  - **Halt Forecast**: 없음 — 문서 수정이라 파괴적·외부·의존성 요소가 없다
  - **Depends on**: T3

## 사전 승인 항목 (일괄 승인 대상)

- T2·T3·T4 — 새 파일 3개(`installer/moa.nsi`·`examples/gen_installer.rs`·`tests/installer.rs`)와 새 폴더 `installer/` 생성.
- T1 — **앱의 설정 저장 위치 변경**(`settings.rs`·`hostkey.rs`의 비공개 경로 함수 2개)과 `migrate_from_legacy_dir`·`LEGACY_APP_DIR` **제거**. 계획된 동작 변경이다.
- T5 — 승인된 PRD의 NFR-7·FR-47 설명줄 문면 개정.
- T3 — `std::process::Command`로 외부 실행 파일(`makensis`)을 부르는 첫 사례. 새 crate 의존성은 없다.

## 불가피한 Halt (위임 불가)

- **이 PC에 NSIS를 설치하는 것** — 사용자가 안내만 받기로 했다(Out of Scope). 구현 중 필요해 보여도 설치하지 않는다.
- commit 이후의 push·병합·태그·릴리즈 — 각 지점에서 따로 승인받는다.

## Verification Strategy

- 빌드: `cargo build`
- 시험: `cargo test` (T1·T4의 신규 시험 포함)
- 린트·형식: `cargo clippy --all-targets -- -D warnings` · `cargo fmt --check`
- 예제 실패 경로 실행: `cargo run --example gen_installer` — 이 PC에서는 `makensis` 부재 경로가 돈다(T3 Acceptance의 검증 대상)
- 수동 검증(HUMAN-VERIFY — NSIS가 있어야 한다):
  1. `winget install NSIS.NSIS` 후 `cargo build --release` → `cargo run --example gen_installer` → `target/installer/MOA-Setup-0.1.0.exe`가 만들어지는가.
  2. 그 설치 파일을 실행 → UAC 없이 `%LOCALAPPDATA%\Programs\MOA`에 설치되고 시작 메뉴에 등록되는가(바탕화면 체크박스 동작 포함).
  3. 설치된 앱이 정상 실행되고 아이콘이 제대로 보이는가.
  4. 제거 → 자동 실행을 켜 뒀다면 HKCU Run의 `MOA` 값이 사라지는가.
  5. 제거 → **묻지 않고** 설치 폴더가 통째로 사라지는가(설정·지문 파일 포함).
  6. 설치본을 실행해 설정을 바꾼 뒤 다시 켜면 그 설정이 유지되는가 — 그리고 그 파일이 **설치 폴더 안에** 생겼는가.
  7. 바로가기 이름이 설치 언어에 따라 `모아`/`MOA`로 만들어지는가(시작 메뉴·바탕화면 둘 다).

## Phase Ledger

## Retry Ledger

## Progress Log

- T1 완료 (커밋 e132ba3): 설정·지문 파일을 exe 옆으로, 마이그레이션·상수 제거, 경로 단언 시험 2건.
  - spec MAJOR 1(plan이 필수로 지목한 `ui/app.rs:2022` stale 주석 누락)은 자기 유발이라 그 자리에서 고쳤다.
- T2 진행 중 결정: makensis에 `/INPUTCHARSET UTF8`을 함께 넘긴다 — `.nsi`를 BOM 없는 UTF-8로 두면서 한글 문구가 깨지지 않게 하는 길이다(D8의 「/D는 VERSION 하나」는 **값 정의**에 대한 것이고 이 인자는 인코딩 지정이라 그 규정과 충돌하지 않는다).
- T4 완료: `.nsi`의 요구 항목 18개를 훑는 통합 시험 2개. 리뷰 지적 둘을 함께 반영했다 —
  ⓐ 설치·제거가 같은 토큰을 쓰므로 `section()`으로 구역을 잘라 단언한다(파일 전체에서 찾으면
  설치 코드가 사라져도 제거 쪽 문자열이 통과시킨다) ⓑ 삭제 목록은 `Delete "..."` 전체를 needle로
  삼는다(경로만 찾으면 같은 구역의 자동 실행 비교 문자열에 걸린다). 18개 축 전부를 `.nsi`에서
  한 줄씩 지워 시험이 실패하는 것을 실제로 확인했다.

## Next Steps

- 권장 다음 액션: 승인 후 `pjc:implement-task`로 T1부터 실행

## Open Questions

- [x] Q1: NSIS 미설치 상태의 범위 → **스크립트만 두고 설치는 안내**(Out of Scope + D3)
- [x] Q2: 설치 범위 → **사용자 단위**(D4)
- [x] Q3: 설치 항목 → **시작 메뉴·바탕화면 바로가기 + 제거 시 자동 실행 정리**(T2). 「제거 시 설정 삭제 질문」은 2026-08-21 추가 요구로 없앴다(D11)
- [x] Q4: 진입점 → **`cargo run --example gen_installer`**(D1·D2)
