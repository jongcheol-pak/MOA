# 시스템 파일 표시 토글 분리 (FR-13)

**PRD**: `docs/prd.md`

## 요구 이해

> 원문 요청:
> - 설정 화면에서 '숨김 항목 표시' 아래 '시스템 파일 표시' on/off 토글 추가하고 종류가 시스템 파일 인 경우 on이면 표시, off 이면 표시하지 않음, 기본 값은 off
> - 설정 화면에서 '숨김 항목 표시' 문구 '숨김 파일 및 폴더 표시' 변경

이해한 요구:
1. 지금 하나인 숨김 토글(`숨김 항목 표시`)이 **숨김 속성과 시스템 속성을 함께** 걸러 낸다. 이를 **두 토글로 나눈다** — 숨김 속성은 기존 토글이, 시스템 속성(`FILE_ATTRIBUTE_SYSTEM`)은 새 토글이 맡는다.
2. 새 토글 `시스템 파일 표시`는 `파일 보기` 그룹의 **숨김 토글 바로 아래**에 서고 **기본값은 꺼짐**이다.
3. 두 속성이 함께 붙은 항목(pagefile.sys 등)은 **두 토글이 모두 켜져야** 보인다 (사용자 결정 — 탐색기와 같은 동작).
4. 기존 토글의 문구를 `숨김 항목 표시` → `숨김 파일 및 폴더 표시`로 바꾼다(영문도 함께).
5. 목록에서 **흐리게 그리는 기준은 지금 그대로**다 — 숨김이든 시스템이든 흐리게 그린다 (사용자 결정).

## Goal

`파일 보기` 설정의 숨김 토글 하나가 맡고 있던 두 속성(숨김·시스템)을 각자의 토글로 나누고, 시스템 파일은 기본으로 숨긴다.

## PRD Coverage

| FR | 요구 | 이번 커버 | 담당 task |
|---|---|---|---|
| FR-13 (Must) | 숨김·시스템 파일 표시를 토글할 수 있다 | ✅ 서술 갱신 + 구현 | T1~T4 |
| 그 밖의 active Must FR | — | 이번 범위 외 (기구현) | — |

FR-13 서술은 토글 하나를 전제로 쓰여 있어 이번 분리로 어긋난다 → T4에서 두 토글로 고쳐 쓰고 변경 이력에 한 줄 남긴다(사용자 승인).

## Tasks

- [x] **T1. 판정 기준을 숨김·시스템으로 가른다 (필터·표시 동작 불변)** — Type C
  - **Files**:
    - `src/panel/file_list.rs` — `ListRow` 트레이트 선언(`is_hidden` doc 포함) · `FileEntry` impl · `RemoteEntry` impl · **시험 `로컬_숨김은_숨김_또는_시스템_속성이다`(L979~987) 개정** · **`name_sort_key` 시험 리터럴의 raw NUL 바이트를 `\0` 이스케이프로 교체**(grep이 이 파일을 건너뛰던 원인 — 계획 당시 Deferred로 미뤘으나, 이 task의 Files 안이고 같은 검증 명령으로 확인되며 이 파일의 조사 자체가 그 함정에 걸려 있어 그 자리에서 고친다. 값은 U+0000 그대로라 시험 단언은 불변)
    - `src/ui/file_list.rs` — 필터 2곳(`:166`·`:204`)
    - `src/ui/tree.rs` — 필터 1곳(`:657`)
    - `src/ui/list_details.rs`(`:397`) · `src/ui/list_grid.rs`(`:268`·`:344`) — 흐리게 그리기
    - `src/ui/list_common.rs` — `dim_if_hidden`의 doc 주석을 “숨김이거나 시스템이면 흐리게”로 갱신(**함수 이름은 그대로 둔다** — 색 변환의 역할이 바뀌지 않는다)
  - **Design**:
    - 배치: 판정은 지금 `is_hidden`이 사는 `panel::file_list`의 `ListRow` 트레이트 그대로다 — 화면 규칙이 아니라 항목의 성질이라 로컬·원격 두 impl이 각자 답해야 한다.
    - 신규 심볼: `ListRow::is_system()`(시스템 속성인가 — 로컬은 `FILE_ATTRIBUTE_SYSTEM`, 원격은 언제나 `false`) · `ListRow::is_dimmed()`(흐리게 그릴 항목인가 — 기본 구현 `is_hidden() || is_system()`). `is_hidden()`은 **숨김 속성만** 보도록 좁힌다.
    - 의존 방향: 그대로다 — `ui::file_list`·`ui::tree`·`ui::list_*`가 트레이트를 부르고, 트레이트는 화면을 모른다.
    - 비추상화 선언: 필터 규칙을 담는 구조체·트레이트를 새로 만들지 않는다. 판정 함수 둘과 호출부의 논리식으로 충분하고, 규칙을 객체로 감싸면 어떤 항목이 왜 빠졌는지 추적이 한 겹 멀어진다.
  - **이 task에서 화면 동작은 바뀌지 않는다** — 필터 호출부는 `is_hidden() || is_system()`으로, dim 호출부는 `is_dimmed()`로 적어 지금과 같은 항목이 걸러지고 같은 항목이 흐려진다. 설정 값이 붙는 것은 T2다.
  - **개정할 시험 (같은 파일)**: `로컬_숨김은_숨김_또는_시스템_속성이다`는 `is_hidden()`이 SYSTEM을 본다고 단언한다(`assert!(entry_with("pagefile.sys", FILE_ATTRIBUTE_SYSTEM.0).is_hidden())`). 이 단언은 **반드시 뒤집힌다** — 시험을 `로컬_숨김과_시스템은_각자의_속성이다`로 고쳐 ① 숨김 속성만 → `is_hidden()`만 참 ② 시스템 속성만 → `is_system()`만 참 ③ 둘 다 → 둘 다 참 ④ 세 경우 모두 `is_dimmed()`가 참을 단언한다. 원격 시험(`원격_숨김은_점으로_시작하는_이름이다`)에는 `is_system()`이 언제나 거짓이라는 단언을 더한다.
  - **Acceptance**: ① 위 시험 개정을 마친 뒤 `cargo test` 전부 통과 ② `is_hidden()`이 `FILE_ATTRIBUTE_SYSTEM`을 보지 않는다(**Read로 확인** — 이 파일은 grep이 건너뛴다, Investigation Log 참조) ③ `cargo clippy --all-targets -- -D warnings` 통과 ④ 트레이트 doc 주석이 새 판정과 일치한다(`is_hidden`에서 “시스템” 서술 제거, `is_system`·`is_dimmed`에 각자의 doc)
  - **Edge Cases**: 두 속성이 함께 붙은 항목 → 두 판정이 모두 참(결과 같음) / 원격 항목의 `is_system()`은 거짓이라 `is_dimmed()`가 `is_hidden()`과 같아진다 / `..` 줄은 로컬에서 `attributes: 0`(`src/ui/panel.rs:1634`)이라 어느 쪽도 아니다
  - **Halt Forecast**: 없음 — 트레이트 구현체가 `FileEntry`·`RemoteEntry` 둘뿐이고 둘 다 이 task에서 함께 고친다(사전 승인 항목에 등록). 외부·파괴적 작업 없음

- [x] **T2. 설정 값을 배선하고 두 토글로 거른다** — Type D
  - **Files**:
    - `src/app/settings.rs` — `show_system` 필드 + 기본값
    - `src/ui/panel.rs` — `DisplayRules::show_system` + `apply_display_rules`
    - `src/ui/file_list.rs` — 필드·setter·필터 + **시험 `숨김을_끄면_목록에서_빠지고_개수도_거른_뒤_기준이다` 개정**
    - `src/ui/tree.rs` — 필드·setter·`child_dirs` 시그니처·필터 + **시험 2건 개정**(`숨김을_끄면_트리에서도_숨김_폴더가_빠진다` · `트리_설정이_바뀌면_읽어_둔_하위를_버린다`)
    - `src/ui/app.rs` — `DisplayRules` 조립 1곳(`:2747`)
    - `src/ui/session.rs` — `앱_설정이_왕복한다`의 `AppSettings` 구조체 리터럴(`:666`)
  - **Design**:
    - 배치: 값은 `AppSettings::show_system`(기본 `false`)에서 나고 `DisplayRules`에 실려 지금 `show_hidden`이 가는 길을 그대로 탄다 — 새 통로를 만들지 않는다.
    - 신규 심볼: `AppSettings::show_system` 필드 · `DisplayRules::show_system` 필드. 목록·트리의 setter는 두 값을 함께 받도록 `set_hidden_rules(show_hidden, show_system) -> bool`로 바꾼다(둘 중 하나만 바뀌어도 다시 읽어야 하므로 “바뀌었나” 판정이 한 자리에 있어야 한다). `child_dirs`는 `show_system` 인자를 하나 더 받는다.
    - 의존 방향: `app::settings` → `ui::app` → `ui::panel` → `ui::file_list`/`ui::tree` (지금과 같다).
    - 비추상화 선언: 설정 항목별 옵저버·트레이트를 두지 않는다(`AppSettings` 주석의 기존 선언 유지).
  - **필터 규칙**: `(show_hidden || !is_hidden()) && (show_system || !is_system())`. 두 값이 모두 켜져 있으면 순회 자체를 건너뛴다(10만 항목 폴더 — NFR-3).
  - **스키마 버전은 올리지 않는다** — `settings` 객체는 `Value`로 받아 변환 실패를 그 자리에서 삼키고 `#[serde(default)]`가 없는 키를 기본값으로 채운다(위키 `feat-settings.md` 근거).
  - **개정할 시험 4건과 새 기대값**:
    1. `ui/file_list.rs`의 `숨김을_끄면_목록에서_빠지고_개수도_거른_뒤_기준이다` — 지금은 `show_hidden=false` 하나로 `pagefile.sys`(시스템 속성)까지 빠진다고 단언한다. **네 조합**으로 고친다: (참,참)→넷 다 보임 / (거짓,참)→숨긴폴더만 빠짐 / (참,거짓)→pagefile.sys만 빠짐 / (거짓,거짓)→둘 다 빠지고 개수 `(1,1)`. 같은 값을 다시 줄 때 `false`를 돌려주는 단언은 유지한다.
    2. `ui/tree.rs`의 `숨김을_끄면_트리에서도_숨김_폴더가_빠진다` — `child_dirs`가 인자를 하나 더 받으므로 호출을 고치고, `System Volume Information`이 **`show_system`으로** 판정된다고 단언한다.
    3. `ui/tree.rs`의 `트리_설정이_바뀌면_읽어_둔_하위를_버린다` — `set_hidden_rules(기본값과 같은 두 값)`이 `false`를, 어느 한쪽만 바꿔도 `true`를 돌려주고 캐시가 비워진다고 단언한다(두 방향 각각).
    4. `ui/file_list.rs`의 `숨김을_끄면_그_항목은_선택에서도_빠진다`(`:1042`) — 단언의 뜻은 그대로이고 **호출 형태만** 새 setter로 고친다(고치지 않으면 컴파일이 깨진다).
  - **Acceptance**: ① `AppSettings::default().show_system == false` ② 위 시험 4건 개정 + 새 조합 단언을 마친 뒤 **`cargo test` 전부 통과** ③ 시스템 속성 항목이 `show_system=false`에서 빠지고 `true`에서 보인다(위 1번 시험이 판정) ④ 숨김+시스템 항목은 두 값이 모두 참일 때만 보인다(같은 시험) ⑤ `cargo build` 경고 0 · `cargo clippy --all-targets -- -D warnings` 통과
  - **Edge Cases**: 기존 설정 파일에 `show_system` 키 없음 → `false`로 읽혀 **업데이트 직후 시스템 파일이 사라진다**(요청대로) / 원격 패널은 `is_system()`이 늘 거짓이라 새 토글에 반응하지 않지만, 토글을 바꾼 프레임에는 지금 규칙대로 재조회가 돈다(기존 Deferred 항목과 같은 자리 — 이번에 손대지 않는다) / 트리는 폴더만 보므로 시스템 속성 폴더(`System Volume Information`)가 새 토글의 영향을 받는다 / 뷰 내부 기본값을 설정 기본값과 어긋나게 두면 **시작 첫 프레임마다 폴더를 다시 읽는다**(D8)
  - **Halt Forecast**: 공개 시그니처 변경(`set_show_hidden` → `set_hidden_rules`) · 설정 스키마 필드 추가 — 둘 다 사전 승인 항목에 등록, 멈추지 않는다

- [x] **T3. 설정 화면에 토글을 세우고 문구를 바꾼다** — Type C
  - **Files**: `src/i18n/mod.rs`(문구 2건), `src/ui/settings_dialog.rs`(토글 + 시험 + `파일 보기` 그룹을 “두 토글”이라 적은 주석 `:343`·`:186` 확인)
  - **Design**:
    - 배치: `show_file_group`의 둘째 줄 아래에 셋째 줄로 붙인다 — 그룹 하나를 떼어 그리는 기존 시험 구조를 그대로 쓴다.
    - 신규 심볼: `i18n::settings_show_system`(`시스템 파일 표시` / `Show system files`). 기존 `settings_show_hidden`의 값은 `숨김 파일 및 폴더 표시` / `Show hidden files and folders`로 바꾼다.
    - 의존 방향: 그대로(`settings_dialog` → `i18n`·`widgets`).
    - 비추상화 선언: 토글 목록을 배열로 돌리지 않는다 — 세 줄이 각자 다른 필드를 뒤집을 뿐이라 배열로 만들면 필드 접근이 클로저 뒤로 숨는다.
  - **Acceptance**: ① 설정 대화의 `파일 보기` 그룹이 세 줄(확장명·숨김·시스템)이다 ② 셋째 줄을 눌렀을 때 `show_system`만 뒤집히고 나머지 둘은 그대로인 시험(기존 그룹 시험에 이어 붙인다) ③ `cargo test` 전부 통과 — `i18n` 소스 훑기 시험(`화면_문구가_카탈로그를_거치지_않은_곳이_없다`) 포함 ④ 카탈로그 값 단언은 원문 리터럴로 쓰고 `i18n::LanguageGuard::lock`으로 언어를 잠근다
  - **Edge Cases**: 그룹 높이가 한 줄 늘어 그 아래 `언어` 그룹의 좌표가 밀린다 — 기존 시험이 그룹 하나만 떼어 그리므로 영향 없음(그 구조를 둔 이유가 이것이다) / 셋째 줄의 클릭 좌표는 줄 높이만으로 계산하면 안 된다 — 줄 사이에 `ui.spacing().item_spacing.y`가 끼므로 `2.0 * (FORM_FIELD_HEIGHT + item_spacing.y) + FORM_FIELD_HEIGHT / 2.0`로 잡는다(간격을 빼먹으면 줄 틈을 눌러 아무 일도 일어나지 않는다) / 문구가 길어져도 스위치를 밀지 않는다(`toggle_row`의 말줄임 규칙)
  - **Halt Forecast**: 없음 — 화면 문구 추가·변경은 카탈로그 안에서 끝나고 저장 형식·외부에 닿지 않는다

- [x] **T4. 문서를 실제와 맞춘다** — Type A
  - **Files**: `README.md`(설정 화면 `파일 보기` 서술), `docs/prd.md`(FR-13 + 변경 이력)
  - **Acceptance**: ① README의 `파일 보기` 줄이 토글 셋을 적고, 시스템 토글의 기본값이 꺼짐이라는 것과 두 속성이 함께 붙은 항목 규칙이 드러난다 ② PRD FR-13이 두 토글로 서술되고 `## 결정 이력`에 2026-08-18 줄이 선다(이 PRD의 이력 섹션 이름은 `결정 이력`이다 — 계획 당시 `변경 이력`으로 잘못 적었다) ③ 문서에 없는 기능을 적지 않는다(역대조)
  - **Halt Forecast**: 없음 — 문서 파일 둘만 고친다

## 동반 변경 판정

| # | 대상 | 축 | 판정 | 처리 |
|---|------|-----|------|------|
| 1 | `README.md` 「설정 화면」 `파일 보기` 서술 | ① 서술 문서 | **필수** — 토글이 둘이 되면 지금 문장이 사실과 어긋난다 | T4 |
| 2 | `docs/prd.md` FR-13 | ① 서술 문서 | **필수** — Must FR의 서술이 토글 하나를 전제한다(사용자 승인) | T4 |
| 3 | `src/ui/session.rs` 앱 설정 왕복 시험 | ③ 검증 자산 | **필수** — `AppSettings` 구조체 리터럴이라 필드가 늘면 컴파일이 깨진다 | T2 |
| 4 | `src/panel/file_list.rs` 숨김 판정 시험 2건 | ③ 검증 자산 | **필수** — `is_hidden()`이 시스템을 본다고 단언한다(반드시 깨진다) | T1 |
| 5 | `src/ui/file_list.rs`·`src/ui/tree.rs` 숨김 관련 시험 4건 | ③ 검증 자산 | **필수** — 시스템 속성 항목이 `show_hidden` 하나로 빠진다고 단언한다 | T2 |
| 6 | 판정을 서술하는 주석 — `ListRow` doc(`panel/file_list.rs:563-567`) · `dim_if_hidden` doc(`list_common.rs:125-133`) · `파일 보기` 그룹 주석(`settings_dialog.rs:343`, “두 토글”) | ① 서술 문서 | **필수** — 코드와 어긋난 주석은 후속 작업을 오도한다 | T1(앞 둘) · T3(셋째) |
| 7 | 위키 `feat-file-list.md`·`feat-settings.md` | ① 서술 문서 | **필수(위키 큐)** — 두 페이지가 토글 하나를 전제로 쓰여 있다. 코드 세션에서 위키를 직접 고치지 않으므로 F-6.5 대기 큐로 넘긴다 | Phase F-6.5 |
| 8 | `AGENTS.md` 데이터 접근 항목 | ① 서술 문서 | **무관** — 앱 설정을 `파일 보기`로 묶어 적어 항목이 하나 늘어도 어긋나지 않는다 | — |
| 9 | 세션 스키마 버전(`SESSION_VERSION`) | ④ 매니페스트 | **무관** — 앱 설정 필드 추가는 버전을 올리지 않는 것이 이 프로젝트의 기존 판단이다(위키 근거) | — |

## Investigation Log

- **`src/panel/file_list.rs`는 grep이 건너뛴다** — 파일에 NUL 바이트가 있어 ripgrep이 binary로 보고 디렉터리 검색에서 조용히 제외한다(`Grep("is_hidden", src/panel/)` → 0건, 같은 파일을 직접 지정하면 “binary file matches”). **이 파일의 확인은 언제나 Read로 한다.** 첫 조사에서 이 함정 때문에 파일 안의 숨김 판정 시험 2건을 놓쳤다(plan-reviewer B1 지적으로 회수).
- 위키 참조: `20_projects/personal/moa/feat-file-list.md` — ⓐ 숨김은 **항목을 받는 자리에서 한 번만** 거른다(그리는 자리에서 거르면 `type_names`·`icon_indices`와 짝이 어긋난다) ⓑ 되돌릴 수 없어 설정이 바뀐 프레임에는 폴더를 다시 읽는다 ⓒ `ListRow` 트레이트가 숨김 판정의 정본이다.
- 위키 참조: `20_projects/personal/moa/feat-settings.md` — ⓐ 대화는 값을 복사해 두지 않고 `AppSettings`를 빌려 쓴다(즉시 반영·즉시 저장) ⓑ **앱 설정을 더할 때 스키마 버전을 올리지 않는다** — `settings` 객체만 `Value`로 받아 변환 실패를 그 자리에서 삼키기 때문이다.
- 위키 참조: `20_projects/personal/moa/decisions.md` — 숨김·시스템 항목 표시와 상충하는 기각·보류 결정 없음(최근 결정은 자격증명·원격 정책·전송 대상 탭이다).
- Deferred 대장(`docs/plans/deferred.md`) 확인: 대기 63건 · 최고령 2026-07-23(26일) → 소진 batch 임계(100건 / 30일) 미달, 이번 회차에 batch를 넣지 않는다. 관련 항목 1건 — “숨김 항목 토글이 원격 패널에서도 재조회를 부른다”(2026-08-14)는 이번 분리로 **닿는 자리가 같지만** 이번에도 Deferred 유지(아래 Deferred 절).
- `src/panel/file_list.rs:622`(Read 확인) — `is_hidden()`이 `FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM`을 함께 본다. 이 한 곳이 두 속성을 묶고 있는 지점이다.
- `is_hidden()` **외부** 호출부 전수(6곳, 전건 Read): `src/ui/file_list.rs:166`·`:204`(로컬·원격 필터), `src/ui/tree.rs:657`(트리 하위 폴더 필터), `src/ui/list_details.rs:397`·`src/ui/list_grid.rs:268`·`:344`(흐리게 그리기). 앞의 셋은 T2에서 두 토글을 보고, 뒤의 셋은 T1에서 `is_dimmed()`로 바뀐다.
- `is_hidden()`을 **단언하는 시험** 전수(Read 확인): `src/panel/file_list.rs:979`(로컬 — pagefile.sys가 숨김이라 단언)·`:990`(원격), `src/ui/file_list.rs:991`(목록 필터), `src/ui/tree.rs:1023`(트리 필터). 그 밖에 `set_show_hidden`을 부르는 시험이 `src/ui/file_list.rs:1015`·`:1021`·`:1042`, `src/ui/tree.rs:1042`·`:1047`에 있다.
- `FILE_ATTRIBUTE_SYSTEM` 사용처 전수: 판정 함수 1곳(`panel/file_list.rs:623`) + 시험 안의 단언·대입 3곳(`panel/file_list.rs:984`·`ui/file_list.rs:997`·`ui/tree.rs:1029`) + `use` 4곳(파일 상단 `panel/file_list.rs:11`, 시험 안 `panel/file_list.rs:980`·`ui/file_list.rs:992`·`ui/tree.rs:1025`).
- `show_hidden` 전수(42 hit, 전건 확인): 설정 필드 2 · i18n 1 · `DisplayRules` 경유 3 · 목록 6 · 트리 6 · 대화 3 · 세션 시험 5 · 그 밖은 시험 단언.
- `src/ui/splitter.rs:178`·`:226` — `DisplayRules`를 받아 그대로 넘기기만 한다(필드를 읽지 않아 수정 불필요). 조립 지점은 `src/ui/app.rs:2747` 하나뿐이다.
- `src/remote/types.rs:285` — `RemoteEntry`에 속성 필드가 없다(`name`·`is_dir`·`is_symlink`·`link_target`·`size`·`modified`·`mode`·`owner`). 원격의 시스템 판정은 만들 수 없다.
- `src/ui/session.rs:666` — `앱_설정이_왕복한다`가 `AppSettings` 구조체 리터럴을 쓴다(필드 추가 시 컴파일 오류). 나머지 두 곳은 `..Default::default()`라 영향 없다.
- `src/ui/panel.rs:1634` — 로컬 `..` 줄은 `attributes: 0`으로 만들어진다(숨김·시스템 어느 쪽도 아니다).
- `docs/prd.md:28` — FR-13은 Must이며 검증 방법이 “단위테스트: 필터 로직 검증”이다.
- 착수 전 작업 트리 정리: 미커밋이던 “숨김 항목 흐리게 표시” 작업을 **`07ef515`로 커밋했다**(`git log`·`git status` 확인 — 지금 트리에는 이 plan 파일만 untracked). `dim_if_hidden`이 `is_hidden()`을 쓰고 있어 T1의 판정 분리와 직접 닿는다. (plan-reviewer M4는 **세션 시작 시점의 git 스냅샷**을 근거로 한 지적이라 기각한다 — 그 스냅샷이 찍힌 뒤, 이 계획을 쓰기 전에 커밋했다. 두 기록의 시점은 같다.)

### 전제 검증

| # | 전제 | 확인 근거 | 결과 |
|---|------|----------|------|
| 1 | 숨김·시스템을 묶는 **판정 지점**은 `is_hidden()` 한 곳이고, 그 사실을 **단언하는 시험**은 4곳이다 | `src/panel/file_list.rs:622-625`(판정) + 시험 4곳 Read(`panel/file_list.rs:979`·`:990`, `ui/file_list.rs:991`, `ui/tree.rs:1023`) | ✅ |
| 2 | 원격 항목에는 시스템 속성 개념이 없다 | `src/remote/types.rs:285-298`(속성 필드 없음) + `panel/file_list.rs:673`(이름 `.` 판정) | ✅ |
| 3 | 설정 필드 추가는 세션 스키마 버전을 올리지 않아도 된다 | 위키 `feat-settings.md` 「설정 하나가 깨져 세션 전체를 잃지 않게 한다」 + `src/app/settings.rs:109`(`#[serde(default)]`) | ✅ |
| 4 | 필터는 항목을 받는 자리에서만 돈다(그리는 자리에는 없다) | `src/ui/file_list.rs:165`·`:203`이 유일한 `retain` 지점, 그리기 경로(`list_details`·`list_grid`)에는 필터 없음 | ✅ |
| 5 | 새 화면 문구는 카탈로그를 거쳐야 하고 시험이 그것을 지킨다 | AGENTS.md 「화면 문구」 규약 + `src/i18n/mod.rs`의 소스 훑기 시험 | ✅ |
| 6 | 설정 대화의 그룹별 시험은 줄이 늘어도 좌표가 밀리지 않는다 | `src/ui/settings_dialog.rs:347-350` 주석 + `:530` 시험이 `show_file_group`만 떼어 그린다 | ✅ |
| 7 | `DisplayRules`를 조립하는 곳은 `ui::app` 한 곳뿐이다 | `src/ui/app.rs:2747`(유일한 리터럴) + `splitter.rs:178`·`:226`은 통과만 시킨다 | ✅ |

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `ListRow::is_system()` | `FILE_ATTRIBUTE_SYSTEM`을 보는 곳은 `is_hidden()` 하나뿐(시험 제외) — 시스템 전용 판정 없음 | 신규. 기존 판정에서 갈라 나오는 것이라 같은 트레이트에 둔다 |
| `ListRow::is_dimmed()` | `dim_if_hidden`(`list_common.rs:131`)이 **색 변환**만 하고 판정은 호출부가 한다 | 신규. 색 변환은 그대로 재사용하고 “흐리게 그릴 항목인가”라는 판정만 트레이트로 올린다(호출부 3곳이 같은 논리식을 되풀이하지 않게) |
| `AppSettings::show_system` | `show_hidden`·`show_extensions`와 같은 자리 | 신규 필드. 기존 구조 그대로 확장 |
| `DisplayRules::show_system` | 같은 구조체의 `show_hidden` | 신규 필드. 통로를 새로 만들지 않는다 |
| `set_hidden_rules` | 기존 `set_show_hidden`(목록·트리) | **기존 함수의 확장**(이름 변경 + 인자 추가). 값별 setter를 하나 더 만들지 않는 이유는 D5 |
| `i18n::settings_show_system` | 카탈로그의 `settings_show_hidden`·`settings_show_extensions` | 신규 키. `strings!` 매크로 그대로 |

## Decisions

| # | 결정 | 선택 | 근거 |
|---|------|------|------|
| D1 | 숨김+시스템이 함께 붙은 항목 | 두 토글이 모두 켜져야 표시 | 사용자 결정(속성별로 토글이 대응 — 탐색기와 같은 동작) |
| D2 | 흐리게 그리는 기준 | 숨김이거나 시스템이면 흐리게(현행 유지) | 사용자 결정 |
| D3 | PRD 갱신 | FR-13 서술 갱신 + 변경 이력 | 사용자 결정 |
| D4 | 시스템 판정 기준 | `FILE_ATTRIBUTE_SYSTEM` 속성 (셸 `종류` 문자열 아님) | 지금 `is_hidden()`이 이미 속성으로 판정한다(`panel/file_list.rs:622`). 셸 종류 문자열(`.sys` → “시스템 파일”)은 언어·연결 프로그램에 따라 바뀌어 필터 기준이 될 수 없다 |
| D5 | setter 시그니처 | `set_show_hidden(bool)` → `set_hidden_rules(bool, bool)` | 어느 한쪽만 바뀌어도 폴더를 다시 읽어야 하므로 “바뀌었나” 판정이 한 자리에 있어야 한다. 값별 setter 둘이면 호출부가 두 반환값을 `\|\|`로 합치는 일을 매번 기억해야 한다 |
| D6 | 새 토글의 자리 | `파일 보기` 그룹의 셋째 줄 | 요청(“'숨김 항목 표시' 아래”) |
| D7 | 원격 재조회 억제 | 이번에 하지 않음 | 기존 Deferred 항목(2026-08-14)과 같은 자리이며 이번 요청의 성립과 무관하다 |
| D8 | 뷰 내부 `show_system` 기본값 | `false` (= `AppSettings::default()`와 같게) | 뷰 기본값이 설정 기본값과 어긋나면 **첫 프레임마다 “바뀜”으로 판정돼 폴더·트리를 다시 읽는다**. 지금 `show_hidden`이 양쪽 다 `true`인 것도 같은 이유다(`ui/file_list.rs:110`·`ui/tree.rs:173` ↔ `app/settings.rs:139`) |

## Open Questions

- [x] Q1. 숨김+시스템이 함께 붙은 항목 규칙 → **두 토글 모두 on이어야 표시**(D1)
- [x] Q2. 흐리게 표시 기준 → **현행 유지, 둘 다 흐리게**(D2)
- [x] Q3. PRD FR-13 갱신 여부 → **함께 갱신**(D3)

## Deferred / Follow-up

- 없음 — 두 항목 모두 `docs/plans/deferred.md` 대기로 이관했다 (2026-08-18).

## Out of Scope

- 탐색기의 “보호된 운영 체제 파일 숨기기”처럼 시스템 토글을 켤 때 확인 대화를 띄우는 것 — 요청에 없다.
- 시스템 파일을 다른 색·아이콘으로 구분해 그리는 것 — 흐리게 표시는 현행 그대로다(D2).

## 사전 승인 항목 (일괄 승인 대상)

- **공개 API 시그니처 변경**: `FileListView::set_show_hidden` · `FolderTreeView::set_show_hidden` → `set_hidden_rules(show_hidden, show_system)`. 실호출부는 `ui::panel::apply_display_rules` 한 곳이고, 그 밖에 시험 호출부 5곳(`ui/file_list.rs:1015`·`:1021`·`:1042`, `ui/tree.rs:1042`·`:1047`)이 함께 바뀐다. `child_dirs`도 인자가 하나 는다(모듈 내부 함수 + 시험 1곳)
- **트레이트 확장**: `ListRow`에 `is_system()`(필수 메서드)·`is_dimmed()`(기본 구현) 추가 — 구현체는 `FileEntry`·`RemoteEntry` 둘뿐이며 둘 다 이번에 함께 고친다
- **설정 스키마 필드 추가**: `AppSettings::show_system`(기본 `false`) — 비파괴 추가, 세션 버전 유지
- **`is_hidden()`의 의미 축소**: 시스템 속성을 보지 않도록 좁힘(외부 호출부 6곳 + 단언 시험 4곳을 T1·T2에서 함께 고친다)

## 불가피한 Halt (위임 불가)

- push · 태그 · 릴리즈 (이번 회차에 예정 없음)
- 계획에 없던 파괴적 작업

## Phase Ledger

- Phase F 통과 (HEAD cce9e8a 기준 — F-7 `plan-completion-reviewer`: BLOCKER 0 · MAJOR 0 · MINOR 3, 그중 둘은 정정하고 하나(원격 헛 재조회)는 D7대로 이연)
- Phase G 통과 (Must 100% — FR-13 하나가 이번 커버 대상이고 구현·PRD 문면 모두 일치, 재루프 사유 없음)

## Next Steps

- 권장 다음 액션: 앱을 띄워 설정 화면의 `파일 보기` 세 줄과 시스템 파일 필터 동작을 눈으로 확인 (빌드로는 못 재는 부분)
- Suggested skills: `pjc:llm-wiki`(위키 큐 소비 — `feat-file-list.md`·`feat-settings.md`가 아직 "토글 둘"을 전제한다)

## Progress Log
- T3-T4 완료: 설정 대화의 `파일 보기`가 세 줄이 되고(확장명·숨김·시스템) 문구를 바꿨다. README·PRD FR-13·PRD 결정 이력을 실제와 맞췄다. 815건 통과.
- T1-T2 완료 (커밋 8f8f47f, T2 커밋): 판정을 `is_hidden`/`is_system`/`is_dimmed`로 가르고(화면 동작 불변), `AppSettings::show_system`(기본 false)을 `DisplayRules` → 목록·트리로 배선했다. setter는 `set_hidden_rules(show_hidden, show_system)`로 확장. 시험 6건 개정(판정 2·필터 4) + `child_dirs` 호출부 2곳 추가 갱신. 814건 전부 통과.
  - 결정: 필터 논리식을 목록(`shows`)과 트리(`child_dirs`) 두 자리에 두되 공통화하지 않는다 — 정의 기준 2회라 공통화 문턱(3회) 미달이고, 트레이트로 올리면 어떤 항목이 왜 빠졌는지 추적이 한 겹 멀어진다(T2 quality 리뷰가 SUGGEST로만 남김).

- 2026-08-18: 계획을 쓰기 전에 미커밋이던 “숨김 항목 흐리게 표시” 작업을 `07ef515`로 커밋해 작업 트리를 비웠다(세션 시작 스냅샷에는 그 전 상태가 찍혀 있다). 그 뒤 이 계획을 작성했다.
- 2026-08-18: plan-reviewer 1라운드 — BLOCKER 2(시험 자산 누락)·MAJOR 4·MINOR 3. B1·B2·M1·M2·M3·m1·m2·m3 반영, M4는 시점 오해로 기각(근거는 Investigation Log 마지막 줄).
- 2026-08-18: plan-reviewer 2라운드 — BLOCKER 0·MAJOR 0·MINOR 5(기록 정확도 3 + 구현 중 드러나는 것 2). 다섯 건 모두 반영하고 통과.
- 2026-08-18: T1 구현 중 **Deferred 결정 하나를 뒤집었다** — `src/panel/file_list.rs`의 raw NUL 바이트를 T1에서 함께 고쳤다(위 Files 참조). 계획에서는 미루기로 했으나 그 NUL이 바로 이 task의 조사를 막는 함정이었고, 수정이 1글자·같은 파일·같은 검증 명령 범위였다. 두 리뷰어(spec MAJOR·quality MINOR)가 범위 이탈로 지적해 이 기록을 남긴다.
- 2026-08-18: T1 리뷰 — spec MAJOR 1(위 Deferred 불일치, 이 기록으로 해소) · quality MAJOR 1(doc 주석이 아직 없는 T3 라벨을 인용 → 화면 문구 인용을 걷어내고 역할로 서술) · quality MINOR 1(spec MAJOR와 같은 사안).
