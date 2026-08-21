# 라이선스 고지·정보 화면 라이선스 줄·의존성 갱신

**PRD**: docs/prd.md (FR-58 문면 개정을 포함한다 — T5)

## 요구 이해

원문 요청:

> - 아이콘 직접 구현, 스크린샷은 임의로 만든거라 괜찮음, LICENSE 파일 생성
> - 정보 화면의 버전 정보 밑에 mit 라이선스 추가
> - 현재 사용중인 오픈소스라이선스도 github에 있어야 하면 파일로 만들어줘
> - readme에 docs/design/screenshots/01-overview.png 화면 이미지 추가하고 마지막에 현재 프로젝트는 mit 라이선스라고 표시하고 사용시 출처 표시하라는 내용 추가.
> - 현재 사용하고 있는 오픈소스가 최신 버전인지 확인해서 최신 버전이 아닌경우 업데이트해도 문제가 없으면 업데이트하고 , 오픈소스라이선스도 같이 업데이트.

이해한 요구:

1. 이 프로젝트를 **MIT 라이선스로 공개**한다 — 레포 루트에 `LICENSE`(MIT, 저작권자 `jongcheol-pak`)를 두고, README 끝에 라이선스와 **출처 표시 요구**를 적는다.
2. **정보 화면**(FR-58)의 버전 줄 아래에 저작권·라이선스·저장소 링크 세 줄을 더한다.
3. 앱이 쓰는 **오픈소스 라이선스 고지를 레포 파일로도** 낸다 — 전문까지 담고 기존 생성기가 함께 만들게 한다(손으로 쓰지 않는다).
4. README 상단에 **화면 이미지**(`docs/design/screenshots/01-overview.png`)를 넣는다.
5. **의존성을 최신으로** 올리되 문제가 없는 범위까지만 올리고, 그에 맞춰 라이선스 자산을 다시 만든다.

앞선 세션에서 확정된 것: 아이콘은 사용자가 직접 만든 것이고 스크린샷은 임의로 만든 화면이라 공개해도 된다(계정명처럼 보이는 경로는 실제 개인 정보가 아니다).

## Goal

MOA를 MIT로 공개할 수 있는 상태로 만든다 — 라이선스 파일·고지 파일·README 표기·정보 화면 표시를 갖추고, 의존성은 안전한 범위까지 올려 라이선스 자산을 그 상태에 맞춘다.

## Tasks

- [x] **T1. `LICENSE` 파일 생성** — Type A
  - **Files**: `LICENSE`(신규)
  - MIT 표준 전문. 저작권 줄은 `Copyright (c) 2026 jongcheol-pak`(D1).
  - **Acceptance**: 파일이 레포 루트에 있고, 첫 줄이 `MIT License`, 셋째 줄이 `Copyright (c) 2026 jongcheol-pak`이며, 본문 세 문단(권한 부여·고지 유지·무보증)이 `assets/spdx/MIT.txt`의 문장과 같다 — **그 파일은 SPDX 원문이라 줄바꿈이 좁게 접혀 있고 `MIT License` 머리도 없으므로 글자 단위 대조가 아니라 문장 대조다**(LICENSE 파일은 널리 쓰이는 형식을 따른다).
  - **Edge Cases**: 이미 `LICENSE`가 있으면 덮어쓰지 않고 멈춘다(현재 없음 — Investigation Log에서 확인).
  - **Halt Forecast**: 없음 — 신규 문서 파일이며 덮어쓸 대상이 없다.

- [x] **T2. 의존성 패치 갱신과 라이선스 자산 재생성** — Type C
  - **Files**: `Cargo.lock`, `assets/licenses.json`
  - `cargo update`로 semver 호환 갱신 47건을 반영한다(D4). **`eframe` 0.36은 올리지 않는다** — `egui-phosphor`가 `egui = "0.35"`에 묶여 있다(전제 검증 P2).
  - 그 뒤 `cargo run --example gen_licenses`로 `assets/licenses.json`을 다시 만든다 — 안 만들면 lock 지문 대조 시험이 실패한다.
  - **Acceptance**: `cargo build` 경고 0 · `cargo test` 전건 통과(특히 `app::licenses`의 지문 대조) · `cargo clippy --all-targets -- -D warnings` 통과 · `cargo fmt --check` 통과. `Cargo.toml`의 버전 지정 줄은 하나도 바뀌지 않는다(패치 갱신은 lock만 움직인다).
  - **Edge Cases**: 갱신 후 빌드·시험이 깨지면 lock을 되돌리고(HEAD의 `Cargo.lock` 복원) 어느 크레이트가 문제인지 적는다 — 그때는 T2를 「갱신 없음」으로 마감하고 T3 이후를 그대로 진행한다(나머지 task는 lock에 의존하지 않는다).
  - **Halt Forecast**: 되돌려도 시험이 깨진 채면 멈추고 보고한다(위임 불가 — 원인이 이번 변경 밖이다).

- [x] **T3. 라이선스 고지 파일 생성기 확장** — Type C
  - **Files**: `examples/gen_licenses.rs`, `THIRD-PARTY-NOTICES.md`(신규 생성물)
  - **Design**: ① 배치 — 생성 로직은 `examples/gen_licenses.rs` 안에 둔다(앱은 이 파일을 읽지 않으므로 `src/`에 둘 이유가 없다). ② 신규 심볼 — `fn write_notices(data: &LicenseData, out: &Path) -> Result<(), String>`(이미 만든 `LicenseData`를 마크다운으로 옮겨 적는다). ③ 의존 방향 — `main`이 `LicenseData`를 만든 뒤 `write_notices`를 부른다. 앱 코드는 이 함수를 모른다. ④ 비추상화 선언 — 출력 형식을 고르는 트레이트·설정 구조체를 두지 않는다(마크다운 한 가지뿐이다).
  - 형식: 머리말(무엇인지·어떻게 만들어지는지·손으로 고치지 않는다) → 구성 요소 목록 표(이름·버전·SPDX·저작권자) → 라이선스 전문. 전문은 `texts`의 중복 제거를 그대로 살려 **같은 전문을 한 번만 싣고 어느 구성 요소가 그것을 쓰는지 적는다**(구성 요소마다 펼치면 파일이 몇 배가 된다).
  - **Acceptance**: `cargo run --example gen_licenses`가 `assets/licenses.json`과 `THIRD-PARTY-NOTICES.md`를 함께 만든다 · 만들어진 마크다운의 구성 요소 표 행 수가 `licenses.json`의 `crates` 수와 같고(현재 158개(실측) — T2 갱신 후 값이 달라지면 그 값) · 전문 절의 수가 `texts` 수와 같으며(현재 93종(실측)) · `cargo clippy --all-targets -- -D warnings`가 통과한다. 두 번 돌려도 결과가 같다(정렬이 고정돼 있다).
  - **Edge Cases**: 저작권자가 없는 크레이트(빈 배열)는 그 칸을 비운다 · 전문에 마크다운 특수문자가 있어도 코드 블록 안에 넣어 깨지지 않게 한다 · 파일 쓰기 실패는 `Err`로 알린다(기존 `main`의 처리와 같다).
  - **Halt Forecast**: 형식 판단으로 멈추지 않는다 — 표의 열 구성(이름·버전·SPDX·저작권자)과 전문 절의 배치(전문 하나 + 그것을 쓰는 구성 요소 목록)를 위 「형식」 문단이 확정했고, 마크다운 이스케이프는 Edge Cases가 「코드 블록 안」으로 정했다. 전문에 코드 블록 종료 표시(백틱 셋)가 들어 있어 감쌀 수 없는 경우만 판단이 필요한데, 그때는 **울타리 길이를 늘린다**(백틱 넷 이상 — 마크다운 표준 규칙).

- [x] **T4. 정보 화면에 저작권·라이선스·저장소 줄 추가** — Type C
  - **Files**: `src/ui/about_dialog.rs`, `src/i18n/mod.rs`
  - **Design**: ① 배치 — 문구는 `i18n` 카탈로그(`strings!`)에, 그리기는 `about_dialog::show_body`에 둔다(지금 이름·버전 줄을 그리는 자리 바로 아래). ② 신규 심볼 — `i18n::about_copyright()`·`i18n::about_license()`·`i18n::about_repository_url()` 셋과, `show_body` 안의 링크 줄 그리기. ③ 의존 방향 — `about_dialog`가 `i18n`을 부른다(지금과 같다). ④ 비추상화 선언 — 「정보 화면 항목」을 목록으로 추상화하지 않는다(줄이 넷뿐이고 각 줄의 그리기가 다르다).
  - 세 줄의 내용(D3): `Copyright (c) 2026 jongcheol-pak` / `MIT License` / `github.com/jongcheol-pak/MOA`(누르면 브라우저로 연다). 앞의 둘은 두 언어에서 같은 값이고, 카탈로그를 거치는 것은 규약(AGENTS 「화면 문구」) 때문이다.
  - 링크는 `ui.hyperlink_to`를 쓴다 — 레포에 선례가 없어 신규이며(4-D), eframe이 `OutputCommand::OpenUrl`을 `webbrowser`로 처리한다(이미 의존 트리에 있다).
  - 본문 폭(`BODY_WIDTH` 248.0)이 세 줄을 담기에 좁으면 **폭을 넓히지 말고 글자 크기를 줄인다**(D5) — 대화 크기가 바뀌면 화면 인상이 달라진다.
  - **Acceptance**: 세 문구가 `i18n` 카탈로그를 거치는 것을 **새 시험이 직접 단언한다** — `LanguageGuard::lock`으로 한국어·영어를 각각 잠그고 `i18n::about_copyright()`·`about_license()`·`about_repository_url()`의 반환값을 원문 리터럴과 대조한다(AGENTS 「화면 문구 — 시험」 규약). **기존 소스 훑기 시험(`화면_문구가_카탈로그를_거치지_않은_곳이_없다`)에 기대지 않는다** — 그것은 한글(`'가'..='힣'`)이 든 리터럴만 잡아 이번 세 문구처럼 ASCII뿐인 것은 소스에 박아도 통과한다(`src/i18n/mod.rs:1331` 확인). 그 밖에 · 정보 대화를 열면 네 줄이 모두 그려지고(단위테스트로 확인) · 링크 문자열이 `https://github.com/jongcheol-pak/MOA`와 일치하며 · 기존 `about_dialog` 시험 13건(실측 2026-08-21)이 모두 통과한다 · 화면 배치는 HUMAN-VERIFY.
  - **Edge Cases**: 글꼴이 바뀌어도 줄이 겹치지 않게 각 줄의 실제 높이로 자리를 잡는다 · 아이콘 자산을 읽지 못하는 경로(`icon = None`)에서도 세 줄이 그대로 나온다 · 언어를 바꿔도 세 줄의 값이 같다.
  - **Halt Forecast**: 없음(기존 파일 안의 추가이며 호출부가 늘지 않는다).

- [x] **T5. PRD FR-58 문면 개정** — Type A
  - **Files**: `docs/prd.md`
  - FR-58 본문의 「이름과 버전을 한 줄로 보이며 `닫기` 버튼 하나를 둔다」를 저작권·라이선스·저장소 링크 세 줄이 그 아래 선다는 서술로 고치고, 개정 이력에 2026-08-21 줄을 더한다(기존 이력 형식 그대로).
  - **FR-57에도 한 구를 더한다**(D9) — 지금 생성기 산출물을 `assets/licenses.json` 하나로 적는데 같은 명령이 레포 고지 파일도 만들게 된다. PRD를 이미 여는 task라 비용이 없고, 두지 않으면 PRD가 고지 자료의 절반만 서술한다.
  - **Acceptance**: FR-58 본문이 T4의 실제 구성과 일치하고 · FR-57이 산출물 둘을 적고 있으며 · 개정 이력에 이번 회차 줄과 plan 경로가 있고 · **FR-57·FR-58 밖의 FR은 한 글자도 바뀌지 않는다**(`git diff`로 확인).
  - **Halt Forecast**: 없음 — 문서 수정이며 파괴적·외부 요소가 없다.

- [x] **T6. README 화면 이미지·라이선스 절과 어긋나는 서술 정정** — Type A
  - **Files**: `README.md`
  - ⓐ 상단 개요 문단 뒤에 `docs/design/screenshots/01-overview.png`를 넣는다(D6 — 아이콘·제목·개요 다음, `## 핵심 기능` 앞).
  - ⓑ 문서 끝에 `## 라이선스` 절을 새로 만든다 — 이 프로젝트가 MIT임(LICENSE 참조)·**사용 시 출처를 표시할 것**·앱이 쓰는 오픈소스 고지는 `THIRD-PARTY-NOTICES.md`와 앱 내 고지 화면에 있다는 세 가지를 적는다.
  - ⓒ **「앱 정보」 불릿**(현재 `README.md:19` — ⓐ를 먼저 넣으면 줄이 밀리므로 번호가 아니라 이 제목으로 찾는다)**의 서술을 T4의 실제 구성에 맞춘다**(동반 변경 필수) — 지금 *"그 아래에 이름과 버전이 한 줄로 섭니다 … `닫기` 버튼 하나가 있습니다"*로 못박고 있어 세 줄을 더하면 곧바로 거짓이 된다. FR-58 문면을 T5로 고치면서 같은 내용을 말하는 이 줄을 두면 문서가 갈린다.
  - ⓓ **생성물 서술 두 자리를 새 산출물에 맞춘다**(동반 변경 필수) — **「오픈소스 라이선스 고지」 불릿**(현재 `README.md:18`)의 고지 자료 재생성 문장과 **`examples/`·`assets/` 저장소 트리**(현재 `README.md:162~169`)가 산출물을 `licenses.json` 하나로만 적고 있다. `THIRD-PARTY-NOTICES.md`(레포 루트)와 `LICENSE`를 그 서술에 넣는다. 여기서도 줄 번호가 아니라 제목·트리로 찾는다(ⓐ가 줄을 민다).
  - **Acceptance**: 이미지가 **상대 경로로 들어가고 그 경로에 파일이 실재하며**(자동 확인 — GitHub에서의 실제 렌더는 push 후에만 보이므로 HUMAN-VERIFY로 가른다) · `## 라이선스` 절에 MIT·출처 표시 요구·고지 파일 링크가 모두 있으며 · 「앱 정보」 줄이 T4의 네 줄 구성과 일치하고 · 생성기 산출물 둘과 `LICENSE`가 README에 나타나며 · **위 ⓐ~ⓓ가 건드리는 자리와 추가분 밖에서는 기존 내용이 바뀌지 않았다**(`git diff`로 확인).
  - **Halt Forecast**: 없음 — 문서 수정이며 파괴적·외부 요소가 없다.

- [x] **T7. AGENTS.md 생성물 규약 갱신** — Type A
  - **Files**: `AGENTS.md`
  - 「라이선스 자산 재생성」 항목과 「커밋되는 생성물」 목록이 지금은 `assets/licenses.json` 하나만 말한다 — 같은 명령이 `THIRD-PARTY-NOTICES.md`도 만들게 되므로 두 곳을 함께 고친다(4-E 필수).
  - **`AGENTS.md:70`의 「커밋되는 생성물: **둘 다** 손으로 고치지 않는다」 리터럴도 함께 고친다** — 그 목록이 셋이 되므로 「둘 다」가 틀린 수가 된다.
  - **Acceptance**: 「라이선스 자산 재생성」 항목이 산출물 둘을 적고 · 「커밋되는 생성물」 목록이 항목 셋을 담으며 그 머리말의 수 표현이 셋에 맞고 · 그 두 자리 밖의 항목은 바뀌지 않았다(`git diff`로 확인).
  - **Halt Forecast**: 없음 — 문서 수정이며 파괴적·외부 요소가 없다.

## Decisions

| # | 항목 | 선택 | 근거 |
|---|---|---|---|
| D1 | 저작권자 표기 | `jongcheol-pak` | 사용자 결정(2026-08-21). GitHub 계정명과 일치하고 실명이 노출되지 않는다 |
| D2 | 고지 파일 형식 | 전문 포함 + 생성기 자동 생성(`THIRD-PARTY-NOTICES.md`) | 사용자 결정. 배포 시 고지 의무를 파일 하나로 충족하고, 손으로 쓰면 곧 실제와 어긋난다 |
| D3 | 정보 화면 구성 | 저작권·라이선스·저장소 링크 세 줄 | 사용자 결정. 대장의 2026-08-19 항목(「정보 팝업에 저작권 줄·홈페이지 링크 더하기」)이 함께 해소된다 |
| D4 | 의존성 범위 | `cargo update` 패치 47건만, `eframe` 0.36 보류 | 사용자 결정(사실 제시 후 재확인). `egui-phosphor`가 `egui 0.35`에 묶여 있어 0.36으로 가려면 아이콘 글꼴을 자체 관리해야 한다(전제 검증 P2) |
| D5 | 정보 대화 폭 | 지금 값(248.0) 유지 | 폭을 바꾸면 대화 크기가 달라져 화면 인상이 바뀐다. 세 줄이 넘치면 글자 크기로 맞춘다 |
| D6 | README 이미지 위치 | 개요 문단 뒤, `## 핵심 기능` 앞 | 개요를 읽고 바로 화면을 보는 흐름. 사용자가 위치를 지정하지 않아 자체 확정 |
| D7 | 고지 파일 이름 | `THIRD-PARTY-NOTICES.md` | 널리 쓰이는 이름이라 공개 저장소에서 무엇인지 바로 읽힌다 |
| D8 | 정보 화면 링크 색 | egui 기본값을 그대로 쓴다 | `src/ui/theme.rs`에 `hyperlink`·`link_color` 정의가 없다(hit 0). 링크 하나를 위해 팔레트에 새 색을 만들지 않는다 — 다크 배경에서 egui 기본 링크 색이 이미 읽힌다. 눈에 거슬리면 그때 팔레트에 넣는다(리뷰 m3) |
| D9 | PRD FR-57 한 구 추가 | T5에 편입 | 생성기 산출물이 둘이 되는데 FR-57은 하나만 적는다. T5가 이미 PRD를 여므로 추가 비용이 없고, 빼면 PRD가 고지 자료의 절반만 서술한다(리뷰 m4) |

## Investigation Log

- 위키 참조: `20_projects/personal/moa/feat-license-notice.md` — 고지 자산은 `cargo tree`가 대상 집합을 정하고 lock 지문 시험이 낡은 자산을 잡는다. 전문은 중복 제거해 담는다(158 구성 요소 · 93 전문)
- 위키 참조: `20_projects/personal/moa/feat-about-dialog.md` — 정보 화면은 「저작권 줄·홈페이지 링크는 두지 않았다」로 적혀 있다. T4가 이 서술을 뒤집으므로 위키 갱신이 필요하다(Deferred)
- Deferred 대장: 2026-08-19 「정보 팝업에 저작권 줄·홈페이지 링크 더하기」가 **이번 T4로 해소**된다 — 그 항목의 해소 표시(`docs/plans/deferred.md:19`)는 `implement-task` Phase F-6.5가 맡는다(위키 큐와 같은 자리). 대장 소진 batch는 열지 않는다 — 잔량 102건(>100)이나 최고령 항목이 2026-07-23으로 29일(임계 30일 미만)이고 절대 상한(130) 미만이며, 「직전 batch 이후 신규 등재 30건 이상」을 만족한다는 근거가 없다(세 축 모두 미달)
- 하이퍼링크 선례: `grep -rn "hyperlink|open_url|OpenUrl" src/` hit 0 — 신규
- `LICENSE` 파일: 레포 루트에 없음(`git ls-files`로 확인 — 덮어쓸 것이 없다)
- 아이콘 규약 대상: `egui_phosphor::regular::*` 상수 23종(실측, `grep -rhno`로 중복 제거해 셈) — D4로 이번엔 손대지 않는다

### 전제 검증

| 전제 | 확인 근거 | 판정 |
|---|---|---|
| P1. 직접 의존성 중 뒤처진 것은 `eframe` 하나뿐이다 | `cargo update --dry-run --verbose` → `Unchanged eframe v0.35.0 (available: v0.36.1)` 한 줄. `cargo search`로 나머지 9개(egui-phosphor 0.13.0 · image 0.25.10 · ssh2 0.9.6 · suppaftp 10.0.2 · windows 0.62.2 · windows-core 0.62.2 · raw-window-handle 0.6.2 · serde 1.0.229 · serde_json 1.0.151)가 최신임을 대조 | 확인 |
| P2. `eframe` 0.36으로 올리면 `egui-phosphor`가 따라오지 못한다 | 레지스트리 캐시의 `egui-phosphor-0.13.0/Cargo.toml` → `[dependencies.egui] version = "0.35"`. git main의 `Cargo.toml`도 같은 값이고 최근 커밋 5건에 egui 0.36 대응이 없다(마지막 `bump to 0.13.0 for egui 0.35`, 2026-07-22) | 확인 |
| P3. `cargo update`가 바꾸는 것은 lock뿐이라 `Cargo.toml`은 그대로다 | `--dry-run` 출력이 전이 의존성 47건의 패치 갱신만 나열한다. 갱신 대상 중 직접 의존성은 `suppaftp` 10.0.1→10.0.2 하나이며 그 요구 줄(`version = "10.0.1"`)은 semver 범위 안이라 손댈 필요가 없다 | 확인 |
| P4. lock이 바뀌면 라이선스 자산 재생성이 필요하다 | `src/app/licenses.rs:122` `lockfile_fingerprint`가 lock의 `name`·`version` 줄만 접어 지문을 만들고, `licenses.rs:202` 시험이 자산의 지문과 현재 lock의 지문을 대조한다 | 확인 |
| P5. egui 0.36의 제거된 API를 이 앱이 거의 쓰지 않는다 | egui CHANGELOG 0.36.0 「Removed」 2건 — `clip_rect_margin`은 `grep` hit 0, `RawInput`의 `Modifiers`는 시험 코드 1곳. rustc 1.95.0으로 MSRV(1.95) 충족 | 확인 — **다만 이번에 올리지 않으므로 이 plan의 성립과 무관하다**(D4 재검토 시의 근거로만 남긴다) |
| P6. `ui.hyperlink_to`가 이 앱에서 실제로 브라우저를 연다 | `webbrowser 1.2.1`이 의존 트리에 있음(`cargo update --dry-run` 출력에 등장). eframe(winit) 백엔드가 `OutputCommand::OpenUrl`을 그것으로 처리한다 | ⚠ 미확인 — **성립을 좌우하지 않는다**(링크가 열리지 않아도 문구는 그대로 보인다). T4 구현 중 실제 클릭으로 확인하고, 열리지 않으면 셸 실행 경로로 대체한다 |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `write_notices`(gen_licenses.rs) | `grep -rn "NOTICES|notices" .` hit 0 — 마크다운을 만드는 코드가 레포에 없다 | 신규. 다만 **자료는 재사용**한다 — 이미 만든 `LicenseData`를 그대로 받아 적으므로 대상 집합·전문 수집 로직이 중복되지 않는다 |
| `i18n::about_copyright` 외 2 | `src/i18n/mod.rs`의 `strings!` 블록 — 같은 형식의 문구가 이미 다수 | 신규 항목이지만 **기존 매크로를 그대로 쓴다**(새 구조 없음) |
| 정보 화면 링크 줄 | `hyperlink`·`open_url` hit 0 | 신규. egui 내장 위젯(`ui.hyperlink_to`)을 쓰고 자체 링크 위젯을 만들지 않는다 |

## 동반 변경 판정

| 대상 | 축 | 판정 | 처리 |
|---|---|---|---|
| `docs/prd.md` FR-58 | ① 문서 | **필수** | 정보 화면 구성이 바뀌므로 FR 문면이 실제와 어긋난다 → T5 |
| `AGENTS.md`(라이선스 자산 재생성·커밋되는 생성물) | ① 문서 | **필수** | 같은 명령의 산출물이 둘이 된다 → T7 |
| `assets/licenses.json` | ③ 검증 자산 | **필수** | lock이 바뀌면 지문 대조 시험이 깨진다 → T2 |
| `README.md`(이미지·`## 라이선스` 절) | ① 문서 | **필수**(요청 자체) | T6 ⓐⓑ |
| `README.md:19`(앱 정보 서술) | ① 문서 | **필수** | T4가 세 줄을 더하면 「이름과 버전이 한 줄 … `닫기` 버튼 하나」가 거짓이 된다. FR-58을 T5로 고치면서 같은 내용의 README를 두면 문서가 갈린다 → T6 ⓒ (리뷰 B1) |
| `README.md:18`·`README.md:162~169`(생성물 서술·저장소 트리) | ① 문서 | **필수** | 같은 명령의 산출물이 둘이 되는 것을 근거로 `AGENTS.md`를 필수로 판정했는데, 같은 사실을 서술하는 README도 같은 축이다. 새 커밋 생성물 `THIRD-PARTY-NOTICES.md`와 `LICENSE`가 README 어디에도 없다 → T6 ⓓ (리뷰 M4) |
| `docs/prd.md` FR-57(생성기 산출물 서술) | ① 문서 | **선택 → 편입** | 「자산은 `gen_licenses`로 다시 만든다」는 여전히 참이라 거짓이 되지는 않는다(그래서 필수가 아니다). 다만 T5가 이미 PRD를 열고 있어 비용이 없다 → T5에 편입(D9, 리뷰 m4) |
| 위키 `feat-about-dialog.md`·`feat-license-notice.md` | ① 문서 | **필수이나 이 스킬 범위 밖** | 이 스킬은 위키를 고치지 않는다 → Deferred에 적어 F-6.5 위키 큐가 맡는다 |
| `Cargo.toml` 버전(0.1.0) | ④ 버전 | **무관** | 사용자가 버전 변경을 요청하지 않았고 기능 추가가 공개 API를 바꾸지 않는다 |
| `src/ui/license_dialog.rs`(앱 내 고지 화면) | ⑤ 기존 기능 | **무관** | 고지 파일은 레포용 산출물이고 화면은 종전대로 `licenses.json`을 읽는다 — 화면 동작이 바뀌지 않는다 |

## PRD Coverage

| FR | 이번 범위 | 담당 |
|---|---|---|
| FR-58 (정보 화면) | 커버 — 문면 개정 + 구현 | T4, T5 |
| FR-57 (오픈소스 라이선스 화면) | 커버 — 자산 재생성으로 최신 상태 유지(화면 동작 변경 없음) + 생성기 산출물을 적는 문면 한 구 추가 | T2, T3, T5 |
| 그 밖의 active Must/Should FR | 이번 범위 외 (기구현) | — |

## Open Questions

- [x] 저작권자 표기 → `jongcheol-pak` (D1)
- [x] 고지 파일 형식 → 전문 포함 + 자동 생성 (D2)
- [x] 정보 화면 구성 → 저작권·라이선스·링크 세 줄 (D3)
- [x] 의존성 범위 → 패치만, eframe 0.36 보류 (D4)

## 사전 승인 항목 (일괄 승인 대상)

- `LICENSE`·`THIRD-PARTY-NOTICES.md` 신규 파일 생성
- `Cargo.lock` 갱신(`cargo update` — 패치 범위, `Cargo.toml` 불변)
- `assets/licenses.json` 재생성(생성기 실행)
- `docs/prd.md` FR-57·FR-58 문면 개정과 개정 이력 추가
- `AGENTS.md`의 생성물 규약 두 자리 수정
- 각 task 완료 후의 로컬 작업 브랜치 커밋

## 불가피한 Halt (위임 불가)

- push · 태그 · 릴리즈 · PR (외부·비가역)
- `cargo update` 후 되돌려도 시험이 깨지는 경우(원인이 이번 변경 밖)
- `eframe` 0.36 상향(D4로 보류 — 하려면 별도 승인)

## Progress Log

- T1-T2 완료 (커밋 6401ee4, d9ff93f): MIT `LICENSE` 추가 · `cargo update` 47건 + 라이선스 자산 재생성. 빌드·시험 954건 통과, 리뷰 지적 0.
  - 확인된 사실: `Cargo.toml`은 실제로 불변이었고 lock만 움직였다(P3 검증). eframe은 0.35에 그대로 남았다.
- T3-T4 완료 (커밋 3ab1dae, +T4): 생성기가 `THIRD-PARTY-NOTICES.md`도 만들게 확장 · 정보 화면에 저작권·라이선스·저장소 세 줄 추가. 시험 948건(신규 2건 포함) 통과.
  - 결정: 표 머리를 `라이선스(SPDX)`로 적어 plan 문면과 실제를 함께 만족(spec M1). `escape_cell`에 개행 처리를 더했다(quality m1).
  - 결정: 신규 카탈로그 항목을 FR-57 섹션에서 빼내 `// ── 정보 대화 (FR-58) ──` 아래로 옮겼다(quality m1 — 섹션 관례 유지).

- T5-T7 완료 (커밋 2edfca2, 0a580b9, 84b6821): PRD FR-57·FR-58 문면 개정과 결정 이력 · README 이미지·`## 라이선스` 절·어긋난 서술 2자리 정정 · AGENTS.md 생성물 규약(둘 → 셋). 문서 task 셋이라 빌드·시험 대상 없음.
  - F-7 지적 반영: `about_dialog.rs`의 doc 주석 두 자리가 네 줄 구성과 어긋난 채 남아 있어 고쳤다(M1 — 자기 유발이라 이연 불가). PRD FR-58의 「닫기 버튼」 구를 실제 순서대로 다시 썼다(m1 — T5가 원래 지시한 형태).
  - 이연 판정: AGENTS.md의 Repository Structure 트리에 새 두 파일을 넣지 않았다(m3) — 그 트리는 `README.md`조차 싣지 않는 선택 목록이고 T7 acceptance가 두 자리로 한정했다. `Cargo.toml`의 `license = "MIT"` 필드도 두지 않았다(m6) — 이 앱은 crates.io에 발행하지 않아 그 필드가 쓰이는 곳이 없다.

## Deferred / Follow-up

- **[SUGGEST] `Cargo.toml`에 `license = "MIT"` 필드 더하기** — 지금은 `LICENSE` 파일만 있고 매니페스트에는 선언이 없다. crates.io에 발행하지 않는 앱이라 쓰이는 곳이 없어 이번엔 두지 않았으나, 도구가 라이선스를 읽는 자리가 생기면 한 줄로 끝난다 (출처: F-7 m6)
- **[SUGGEST] 시험용 셰이프 텍스트 수집 헬퍼가 여러 파일에 흩어져 있다** — `collect_text`(`about_dialog.rs`)와 같은 패턴(`Shape::Vec` 재귀 + `Shape::Text` 수집)이 `ui/tabs.rs`·`ui/menu.rs`·`ui/sidebar.rs`·`ui/tree.rs`·`ui/panel/tests.rs`에 각각 손으로 재구현돼 있다. 각 파일이 독립 `#[cfg(test)] mod tests`라 3회 공통화 문턱이 파일 간에 그대로 적용되지는 않지만, 시험 전용 공용 모듈(`src/ui/test_support.rs`)로 뽑으면 반복이 준다. **이번 회차가 그 사본을 하나 늘렸다**(`about_dialog.rs`의 것이 여섯 번째 — F-7 m2). 대장의 2026-08-20 「`ui_sources` 재귀 헬퍼가 세 곳에 중복」과 **같은 해법을 요구하는 건**이라 함께 다루는 편이 낫다 (출처: T4 quality S1)
- **위키 갱신** — `20_projects/personal/moa/`의 `feat-about-dialog.md`(「저작권 줄·홈페이지 링크는 두지 않았다」가 뒤집힌다)·`feat-license-notice.md`(레포 고지 파일이 새로 생긴다)가 실제와 어긋나게 된다. F-6.5의 위키 큐가 맡는다.
- **`eframe` 0.36 상향** — `egui-phosphor` 0.14(egui 0.36 대응)가 나오면 둘을 함께 올린다. 대응판 없이 가려면 Phosphor 글꼴(488KB)을 `assets/`에 담고 쓰는 아이콘 23종의 상수를 직접 정의해야 하며, AGENTS 아이콘 규약·`is_icon_font` 시험·라이선스 자산이 함께 바뀐다. egui 0.36의 브레이킹은 작다(전제 검증 P5) — 걸리는 것은 글꼴 크레이트뿐이다.
- **정보 화면 링크의 브라우저 열기 방식** — `ui.hyperlink_to`로 열리지 않으면 셸 실행으로 대체한다(P6).

## Out of Scope

- 앱 버전 올리기·릴리즈 발행 (요청에 없다)
- 앱 내 고지 화면(FR-57)의 UI 변경 — 자산만 다시 만들고 화면은 손대지 않는다
- 스크린샷 다시 만들기 — 사용자가 「임의로 만든 것이라 괜찮다」고 확정했다
