# Plan: 아이콘 변환의 셸 잠금과 실패 복원력

## 요구 이해
- **원문 요청**: "IconTextures의 아이콘 변환이 셸 잠금 밖이라 병렬 시험에서 경합 수정"
- **이해한 요구**: 직전 회차가 Deferred에 남긴 그 항목을 지금 착수한다. 사용자와 확인한 결과 **셋을 함께** 한다 — ① 병렬에서 텍스처 변환이 실제로 실패하는지 **원인을 규명**하고 ② 실패를 `None`으로 영구 기억해 재시도하지 않는 것을 **고쳐 복원력을 주고** ③ 셸 호출 직렬화 잠금의 **소유를 "시험이 잡는다"에서 "자원을 만지는 함수가 잡는다"로 옮긴다**(그러지 않고 변환 함수에만 잠금을 넣으면 이미 잠금을 쥔 시험 5곳에서 재진입 데드락이 난다).
- **포함하지 않는 것으로 이해**: 화면에 보이는 동작·아이콘 모양은 바꾸지 않는다(복원력은 "한 번 실패한 아이콘이 영영 안 보이던 것"만 고친다). 잠금을 실행 파일에까지 넣지 않는다 — UI 스레드 하나뿐이라 경합할 상대가 없다.

## Goal
`SHELL_LOCK`이 겨냥하는 네 셸 API(`SHGetFileInfoW`·`SHGetKnownFolderPath`·`SHGetImageList`·`ImageList_GetIcon`)가 모두 같은 잠금 안에서 직렬화되고, **일시적으로** 변환에 실패한 아이콘이 다음 프레임에 다시 시도된다(**키당 3회까지** — 늘 실패하는 아이콘은 그 뒤 포기해 예산과 프레임 비용을 놓는다. D6).

> **PRD는 연결하지 않는다** — 이 plan은 `docs/prd.md`의 FR-5·NFR-3을 **근거로 인용**하지만(D6·동반 변경 판정) 그 문면을 바꾸지도, 새 FR을 구현하지도 않는다. `**PRD**:` 줄을 달면 Phase G가 이번 범위 밖의 Must FR까지 재검증하므로 달지 않는다(2026-08-17 리뷰 1라운드에서 이 판정이 타당함을 확인).

> **"셸을 부르는 모든 자리"가 아니다** — `fs::thumbnail`(`SHCreateItemFromParsingName`)·`fs::shell_menu`(`SHParseDisplayName`·`SHGetDesktopFolder`)는 이 잠금의 대상이 아니다(아래 동반 변경 판정의 `무관` 행). 그 둘까지 포함하는 것으로 읽히면 다음 세션이 "셸 직렬화는 끝났다"고 오인한다.

## Out of Scope
- **실행 파일에서의 잠금** — UI 스레드 하나가 그리므로 직렬화할 상대가 없다. 잠금은 `#[cfg(test)]`로만 존재한다(지금 구조 유지).
- **재진입 가능 잠금 도입**(`parking_lot::ReentrantMutex` 등) — 새 의존성이 필요하고, 잠금 소유를 한 층으로 모으면 재진입 자체가 생기지 않는다.
- **`fs::thumbnail`·`fs::shell_menu`의 셸 호출 직렬화** — 그 API들은 `SHELL_LOCK` doc이 든 네 API에 없고, 실패 양상(16px 폴백)도 다르다. 필요해지면 별도 회차.
- **`IconTextures`의 축출·상한 정책 변경** — 프레임당 생성 상한(8개)과 썸네일 LRU는 이번 대상이 아니다.

## Deferred / Follow-up
- `[SUGGEST]` `icon_index_for_path`·`shell_display_name`의 잠금도 `icon_index`·`type_name`처럼 블록 스코프로 좁히면 네 곳의 해제 시점이 통일된다(T3 quality 리뷰 S1 — 기능 차이는 없고, 두 곳은 여러 지역 변수를 써서 블록으로 감싸면 오히려 길어진다)

## Investigation Log
- 위키 참조: `20_projects/personal/moa/conventions.md` — 직전 회차가 넣은 3항목이 이 작업의 배경이다. 그중 **"`IconTextures`의 아이콘 변환은 셸 잠금 밖이라 병렬 시험에서 경합한다"** 항목이 미검증 가설을 사실처럼 적었다(아래 전제 4) — 이번 조사로 재현되지 않음을 확인했으므로 정정 대상이다(동반 변경).
- 위키 참조: `20_projects/personal/moa/decisions.md` — 이 작업과 상충하는 기각·보류 결정 없음(최신 항목은 2026-08-17 네트워크 드라이브 상태 표시 채택·영구 제외 3건이며 잠금·시험 인프라와 무관).
- Deferred 대장(`docs/plans/deferred.md`) ①: 이번 착수 대상 항목이 `## 대기`에 있다(`[2026-08-17] IconTextures의 아이콘 변환이 셸 잠금 밖이다`) — F-6.5에서 종결(반영) 처리한다. ② 전제 반증: 전 그룹을 훑어 이 plan의 전제를 부정하는 항목 없음. 관련 항목으로 `[2026-08-15] CPU가 붐빌 때 lib 시험 1건이 간헐 실패`가 있으나 그것은 **시간 마감(2초)에 기대는 시험** 목록이고 이번 경합과 다른 원인이다. ③ 소진 batch: 잔량 **70건**(임계 100 미달) · 최고령 `2026-07-23`(25일, 임계 30일 미달) → batch 미착수.
- `SHELL_LOCK`의 목적이 doc 주석에 명시돼 있다(`src/fs/icons.rs:317-325`): "`SHGetFileInfoW`·`SHGetKnownFolderPath`·`SHGetImageList`는 프로세스 전역 셸 상태를 함께 쓰는데 … 동시에 부르면 `SHGetImageList`가 실패해 **16px로 폴백**한다". **`ImageList_GetIcon`은 그 목록에 없다** — 이번에 그 자리를 더한다.
- `icon_to_image`(`src/ui/icon_tex.rs:166`)는 `IconTextures::get`(`:70`)에서만 불린다 — 호출 경로가 하나뿐이라 잠금을 넣을 지점이 명확하다.
- `IconTextures::get`(`src/ui/icon_tex.rs:58-82`)은 변환 결과를 `by_key.insert(key, handle)`로 담는데 **`handle`이 `None`이어도 담는다** — 그래서 한 번 실패한 인덱스는 `contains_key`가 참이 되어 **다시 시도되지 않는다**. 프레임 상한에 걸린 경우만 캐시에 넣지 않아 재시도된다(`:67-69`).
- **캐시 조회가 셸 호출보다 앞에 있다** — `icon_index`(`:148` `is_dir` 조기 반환 · `:156` 경로 캐시), `icon_index_for_path`(`:189`), `shell_display_name`(`:225`), `type_name`(`:263`·`:266`). 즉 **잠금을 함수 첫 줄에 두면 캐시 히트까지 전역 직렬화된다**(설계 근거 — D5).
- **경합 재현 시도(2026-08-17)**: `cargo test --lib` **5회 연속 808건 전부 통과**. 아울러 `트리_줄에는_셸_아이콘이_붙는다`(`src/ui/panel/tests.rs:2506`)는 **이미 `shell_test_guard()`를 잡은 채 아이콘 수를 단언**하는데(`:2511`·`:2523`) 병렬에서 통과한다 — 즉 현재 코드에서 텍스처 변환 실패는 관측되지 않는다.
- **T5 실패의 원인은 미확정이다**: 직전 회차에서 `드라이브_갈래는_끊긴_상태를_배지로_그린다`가 병렬에서만 실패하고 `--test-threads=1`에서 통과한 것은 실측이지만, 그 원인을 `ImageList_GetIcon` 경합으로 적은 것은 **가설이었다**(당시 텍스처 실패 여부를 직접 관측하지 않았다). 배지를 텍스처와 분리한 뒤로는 그 시험이 텍스처 실패를 감지하지 않으므로 **지금 코드로는 재현 여부를 알 수 없다** — T1이 그것을 관측한다.
- **잠금 획득 지점을 호출 관계로 갈랐다** (전제 6·7이 정본):
  - 셸을 직접 부르는 **바깥 함수 7곳** — `IconCache::new`(`icons.rs:91`) · `icon_index`(`:147`) · `icon_index_for_path`(`:188`) · `shell_display_name`(`:224`) · `type_name`(`:262`) · `known_folders::known_folder`(`known_folders.rs:37` — 본문이 곧 `SHGetKnownFolderPath`(`:41`)) · `icon_to_image`(`icon_tex.rs:166`).
  - **잠그지 않는 private 내부 2곳** — `system_image_list`(`icons.rs:281`, `IconCache::new`의 `:110`에서만 불린다) · `lookup_by_attributes`(`:292`, `icon_index`의 `:175`와 `type_name`의 `:269`에서만 불린다).
  - **잠그지 않는 조합 함수 2곳** — `fs::drives::list_drives`(`drives.rs:129-133`에서 `shell_display_name`·`icon_index_for_path`를 순차 호출) · `fs::known_folders::default_favorites`(`known_folders.rs:19-31`에서 `known_folder`(`:22`)와 `shell_display_name`(`:26`)을 호출). **둘은 같은 형태이며 잠그면 재진입 데드락이다.**
- `shell_test_guard` grep 내역 **19줄 = 호출 16 + `use` 2(`drives.rs:169`·`known_folders.rs:58`) + 정의 1(`icons.rs:332`)**. 호출 16곳: `fs/drives.rs` 4(`:174,185,203,215`) · `fs/icons.rs` 5(`:367,419,434,454,468`) · `fs/known_folders.rs` 2(`:64,87`) · `ui/panel/tests.rs` 2(`:123,2511`) · `ui/tree.rs` 3(`:945,962,981`).
- `std::sync::Mutex`는 **재진입 불가**다(`SHELL_LOCK`이 그 타입 — `src/fs/icons.rs:327`). 같은 스레드가 두 번 `lock()`하면 그 자리에서 멎는다(타임아웃 없음).
- 현행 `shell_test_guard`는 **poison을 관용한다**(`:333-335` `unwrap_or_else(|poisoned| poisoned.into_inner())`) — 그 사유가 주석에 있다("앞선 시험이 패닉해 독이 올랐어도 이어서 쓴다 … 여기서 또 패닉하면 원인이 가려진다"). 새 획득 함수도 이 관용을 그대로 이어받아야 한다(전제 9).
- **T1 관측 결과 (2026-08-17 실측)**: `icon_to_image`를 여러 스레드에서 동시에 부르며 `None` 횟수를 셌다. **① 단독 실행** 8스레드 × 50회 = 400회 → 실패 **0**. **② 전체 스위트(809건)와 병렬** 같은 조건 3회 → 매회 실패 **0**. **③ 부하 4배** 16스레드 × 200회 = 3200회를 전체 스위트와 병렬로 여러 회 → 매회 실패 **0**. 즉 **이 PC·이 부하에서는 `ImageList_GetIcon` 경합이 재현되지 않는다**(다른 환경·부하에서의 부재를 증명한 것은 아니다). **진단을 넣은 상태와 제거한 상태 양쪽에서 `cargo build`·`cargo test --lib`·`cargo clippy --all-targets -- -D warnings`가 모두 통과했다**(acceptance 4 — 넣은 상태의 clippy는 리뷰 지적으로 뒤늦게 확인).
  - **곁가지 관측**: ③의 부하를 건 상태로 스위트를 **23회** 돌리는 동안 **1회**, 진단이 아닌 다른 시험 1건이 실패했다(재현율 ~4%, 이름은 포착하지 못했다 — 필터·반복·로그 캡처 세 방법으로 시도). 대장의 기존 항목 `[2026-08-15] CPU가 붐빌 때 lib 시험 1건이 간헐 실패`와 같은 현상으로 보여 새로 등재하지 않는다 — 다만 **그 항목의 범위는 시간 마감(2초)에 기대는 시험 3파일**이고 이번 실패가 거기 드는지는 이름을 못 잡아 확인하지 못했다. **재발하면 이름을 반드시 포착해 그 범위에 실제로 드는지 확인한다**(들지 않으면 전제 4를 다시 연다).
- `IconTextures::get`의 반환은 마지막 줄 `by_key.get(&key).and_then(|h| h.as_ref())`가 주는 `Option<&TextureHandle>`이다(`:81`) — **재시도 갈래를 더해도 이 줄이 그대로면 반환 타입이 바뀌지 않으므로** 호출부 4곳(`list_details.rs:409`·`list_grid.rs:173`·`sidebar.rs:722`·`tree.rs:720`)은 손대지 않는다.

### 전제 검증
| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | `IconTextures::get`이 변환 실패를 영구 기억한다 | `src/ui/icon_tex.rs:65-81` — `image.map(...)`이 `None`이어도 `by_key.insert(key, handle)`가 담고, 다음 호출은 `contains_key`가 참이라 변환을 건너뛴다 | ✅ |
| 2 | `icon_to_image`는 호출 경로가 하나다 | `grep -rn "icon_to_image" src/` → 정의 1 + 호출 1(`icon_tex.rs:70`, `IconTextures::get` 안) | ✅ |
| 3 | 잠금을 쥔 채 잠금 대상 함수를 쓰는 시험이 **15곳** 있어, 자원 함수에 잠금을 넣으면서 시험 guard를 그대로 두면 데드락이다 | **`IconTextures` 경로 5곳** — `src/ui/tree.rs:945,962,981`(배지 시험 → `badges_of_node` → `IconTextures::get`) · `src/ui/panel/tests.rs:2511`(→ `panel.show` → `IconTextures::get`) · `src/ui/panel/tests.rs:123`(`drive_rows()`의 `OnceLock`이 guard를 쥔 채 `list_drives`). **`IconCache::new`를 부르는 10곳** — `fs/icons.rs` 5 · `fs/drives.rs` 3 · `fs/known_folders.rs` 2. (guard 16곳 중 `drives.rs:215`만 예외 — `is_network_drive`·`is_reachable`만 부른다.) `SHELL_LOCK`은 `std::sync::Mutex`라 재진입 불가(`src/fs/icons.rs:327`) | ✅ (리뷰 2라운드 m2 — 첫 판의 "5곳"은 `IconTextures` 경로만 센 값) |
| 4 | **`ImageList_GetIcon` 경합이 실제로 일어나는가** | **T1이 직접 관측했다**(위 Log 「T1 관측 결과」) — 최대 16스레드 × 200회를 전체 스위트와 병렬로 돌려도 `icon_to_image` 실패 **0건**. 종전 근거(`cargo test --lib` 5회 통과 · 잠금을 잡고 아이콘 수를 단언하는 기존 시험 통과)와 일치한다 | ✅ **재현되지 않음으로 확정**(이 PC·이 부하 한정). 전제 5대로 T2·T3의 성립에는 영향 없다 |
| 5 | 전제 4가 부정되어도 이 plan은 성립한다 | T2(복원력)는 전제 1만 근거로 하고, T3(잠금 정리)는 **이미 있는 잠금의 소유를 옮기는 일**이라 경합 실측이 성립 조건이 아니다. 전제 4가 좌우하는 것은 `icon_to_image`를 잠금 집합에 넣는 1줄의 값어치뿐이며 그것도 `#[cfg(test)]`라 실행 파일에 비용이 없다. 사용자가 셋을 함께 하기로 결정(2026-08-17) | ✅ |
| 6 | 잠금을 자원 함수로 옮길 때 재진입이 생기지 않는다 | 위 Log의 「잠금 획득 지점」 — 바깥 7곳만 잠그고 private 내부 2곳·조합 함수 2곳은 잠그지 않는다 | ✅ |
| 7 | **`default_favorites`도 `list_drives`와 같은 조합 함수다** | `src/fs/known_folders.rs:19-31` — `known_folder(&id)`(`:22`)와 `icons.shell_display_name(...)`(`:26`)을 함께 부른다. 이 함수를 잠그면 `기본_즐겨찾기는_실재하는_폴더만_돌려준다`(`:64`)가 그 자리에서 멎는다 | ✅ (리뷰 1라운드 B1 — 첫 판에서 빠뜨렸다) |
| 8 | **잠금을 함수 첫 줄에 두면 캐시 히트까지 직렬화된다** | 위 Log의 캐시 위치 4곳. 그 병리가 이 레포에서 이미 실측됐다 — `src/ui/panel/tests.rs:116-120`("셸 잠금을 프레임마다 잡아 … 실측으로 전체 스위트가 10분을 넘겼다") | ✅ (리뷰 1라운드 M1) |
| 9 | 새 획득 함수는 poison을 관용해야 한다 | `src/fs/icons.rs:329-336`의 현행 구현과 그 사유 주석. `.unwrap()`은 AGENTS.md 「에러 처리」 금지 대상이기도 하다 | ✅ |
| 10 | 실행 파일에는 잠금이 필요 없다 | `SHELL_LOCK`·`shell_test_guard`가 이미 `#[cfg(test)]`이고(`:326,331`) AGENTS의 UI 스레드 원칙상 그리기는 한 스레드다 | ✅ |
| 11 | **현재 스위트 소요 시간** | `cargo test --lib` **2.0초**(808건, 2026-08-17 재실측 — 프로세스 전체 2.4초). 직전 plan이 적은 `10.6초`는 `cargo test` **전체**(lib + 통합 + doc) 값이다. 병리 때는 **10분+**였다(전제 8) — 정상과 병리가 300배 떨어져 있어 기준선을 촘촘히 잡을 수 있다 | ✅ (리뷰 2라운드 M1 — 첫 판의 "2~3초"는 근거 없이 적었다가 실측으로 확인) |

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| 잠금을 자원 함수로 옮기면서 **바깥/내부/조합 판정을 틀리면 데드락**이다 | 그 시험이 그 자리에서 멎어 `cargo test`가 끝나지 않는다(타임아웃도 없다) | 전제 6·7의 목록을 T3 Design에 **심볼명으로** 박고 acceptance에 열거한다. T3 검증에 `cargo test --lib` **완주**를 넣어 멎음을 곧바로 드러낸다 |
| 잠금 획득이 **캐시 히트 앞**에 오면 스위트가 통째로 늘어진다 | 직전 회차의 10분 병리 재발(전제 8) | 획득 지점을 **셸 호출 직전**(캐시 조회·조기 반환 뒤)으로 못 박고(D5), T3 acceptance에 **스위트 소요 시간 기준선**을 둔다 |
| 시험 16곳 + `use` 2 + 정의 1을 걷어내다 **한 곳을 빠뜨리면** 데드락이거나 `-D warnings`가 깨진다 | 위와 같다 | `grep -rn "shell_test_guard" src/` → **0건**(호출·`use`·정의 전부)을 acceptance로 둔다 |
| 재시도 제한을 잘못 걸면 **영구 아이콘 기아**·**무제한 GDI 반복**·**목표 무효화** 중 하나에 걸린다 | 첫 시도와 같은 예산을 쓰면 처음 보는 키가 영영 올라오지 못하고, 제한이 없으면 매 프레임 셸·GDI를 무제한 호출하며, 프레임 예산만 두면 늘 실패하는 키가 그 예산을 영구 점유해 일시적 실패 키에 차례가 오지 않는다 | **재시도 전용 프레임 예산(2) + 키당 재시도 상한(3)** 을 함께 둔다(D6) — 처음 보는 키의 몫 8이 온전하고, 늘 실패하는 키는 3회 뒤 예산과 프레임 비용을 놓는다. T2 acceptance의 4·5번째 항목이 두 갈래를 각각 지킨다 |
| T3의 데드락이 **타임아웃 없는 멎음**으로 나타난다 | `cargo test`에 자체 상한이 없어 자율 실행 중에는 세션이 그대로 매달린다(Halt보다 나쁘다 — 사용자 개입 없이 벗어날 수 없다) | 시험 실행에 **바깥에서 시간 제한을 씌운다**(T3 Verification — 빌드는 `--no-run`으로 분리하고 실행에만 60초 상한). 잠금 추가와 시험 guard 제거를 **한 편집으로 끝내** 중간 상태에서 검증하지 않는다 |
| T1의 진단이 **경합 없음**으로 나오면 T3의 근거가 약해진다 | 구조 개선의 값어치만 남는다 | 전제 5대로 T3는 구조 정합을 목표로 하며 실측에 의존하지 않는다. **T1 결과를 그대로 기록해** 위키·대장의 미검증 서술을 정정한다(T4) |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `IconTextures::get` | 정의 `src/ui/icon_tex.rs:58` · 호출 4곳(`ui/list_details.rs:409` · `ui/list_grid.rs:173` · `ui/sidebar.rs:722` · `ui/tree.rs:720`) | 재시도 갈래가 는다 — **시그니처·반환 타입 불변**이라 호출부는 손대지 않는다 |
| `IconTextures`의 필드 | `src/ui/icon_tex.rs:22-26`(`by_key`·`created_this_frame`) · `:65-81`(유일 사용처) | **`by_key`의 값 타입은 그대로**(D3) · `retried_this_frame`·`retries`가 는다(둘 다 비공개) |
| `icon_to_image` | `src/ui/icon_tex.rs:166`(정의) · `:70`(유일 호출부) | 잠금 획득 1줄 추가(`index < 0` 조기 반환 뒤) |
| `shell_test_guard` / `SHELL_LOCK` | 정의 `src/fs/icons.rs:326-336` · `use` 2(`fs/drives.rs:169`·`fs/known_folders.rs:58`) · 호출 **16곳**(`fs/drives.rs` 4 · `fs/icons.rs` 5 · `fs/known_folders.rs` 2 · `ui/panel/tests.rs` 2 · `ui/tree.rs` 3) | `shell_test_guard`와 `use` 2줄은 **제거**하고 `SHELL_LOCK`은 남겨 새 `shell_guard()`가 재사용한다 |
| `IconCache::new` · `icon_index` · `icon_index_for_path` · `shell_display_name` · `type_name` | `src/fs/icons.rs:91,147,188,224,262` | 각자 **셸 호출 직전**에 잠금을 잡는다(캐시 조회 뒤 — D5) · 시그니처 불변 |
| `known_folders::known_folder` | `src/fs/known_folders.rs:37` | 잠금을 잡는다(본문이 곧 셸 호출) |
| `system_image_list` · `lookup_by_attributes` | `src/fs/icons.rs:281,292` | **잠그지 않는다**(바깥 함수에서만 불리는 private — 잠그면 재진입) |
| `fs::drives::list_drives` · `fs::known_folders::default_favorites` | `src/fs/drives.rs:120` · `src/fs/known_folders.rs:19` | **잠그지 않는다**(안에서 잠금 함수를 부르는 조합 함수 — 전제 6·7) |

### 4-B. 계약·직렬화 변경
- 없다. `IconTextures::get`의 시그니처·반환 타입이 그대로이고, 잠금은 `#[cfg(test)]`라 실행 파일의 동작·저장 형식에 영향이 없다.

### 4-C. 영향 받는 테스트
- `src/ui/icon_tex.rs` — 현재 자체 시험 없음. T2가 재시도·성공 캐시 시험 2건을 신설한다(같은 파일 `mod tests`). **그 시험은 실제 이미지 리스트가 필요해 T2 시점에는 `shell_test_guard()`를 잡으므로, T3의 걷어내기 대상이 16 → 18곳이 된다**(신설 2건 포함).
- `src/fs/icons.rs`(5곳) · `src/fs/drives.rs`(4곳 + `use` 1) · `src/fs/known_folders.rs`(2곳 + `use` 1) · `src/ui/panel/tests.rs`(2곳) · `src/ui/tree.rs`(3곳) — 잠금 획득 줄 제거.
- `src/ui/tree.rs:955-962`(`아이콘_텍스처가_없어도_배지는_그려진다`) — 그 시험의 **존재 이유 주석이 T2로 거짓이 된다**(동반 변경). 시험 자체는 유효하다(텍스처 없이도 배지가 그려지는 것을 계속 본다).

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `fs::icons::shell_guard()` + `pub(crate) struct ShellGuard`(빌드별) | `grep -rn "shell_test_guard\|SHELL_LOCK" src/` → 그 둘뿐이고 **시험 전용(`#[cfg(test)]`)**이라 프로덕션 코드에서 부를 수 없다 | 신규. 다만 **`SHELL_LOCK`을 그대로 재사용**하고 획득 함수만 빌드 양쪽에 두는 얇은 껍데기다(잠금을 새로 만들면 직렬화가 둘로 쪼개진다). `shell_test_guard`는 이 함수로 **대체·제거**된다 |
| `retried_this_frame`·`retries` + 상수 둘 | `grep -rn "this_frame" src/ui/icon_tex.rs` → 아이콘·썸네일의 `created_this_frame` 두 카운터가 이미 같은 꼴로 있다 | 신규 — **기존 카운터를 재사용할 수 없다**(첫 시도와 재시도에 각각 다른 상한을 물려야 갈림이 성립하고, 키당 상한은 프레임이 아니라 키의 이력이라 프레임 카운터로 표현할 수 없다 — D6). 프레임 카운터의 이름·리셋 방식은 기존 `created_this_frame`의 꼴을 따른다 |

### Verified by
- `grep -rn "icon_to_image" src/` → 4 hits(정의 1·호출 1·`hicon_to_image` 2), 전부 반영
- `grep -rn "shell_test_guard" src/` → **19줄 = 호출 16 + `use` 2 + 정의 1**, 전부 반영(첫 판의 "17곳"은 정의·`use`를 뭉갠 오산 — 리뷰 1라운드 M3)
- `grep -rn "textures.get(\|\.get(row.ctx\|\.get(ctx" src/` → 4 hits(`IconTextures::get` 호출부), 반환 타입 불변이라 무영향
- `grep -n "system_image_list(\|lookup_by_attributes(" src/fs/icons.rs` → 정의 2 + 호출 3, 전부 바깥 함수 안이라 재진입 판정에 반영
- `grep -rn "SHCreateItemFromParsingName\|SHParseDisplayName\|SHGetDesktopFolder" src/fs/` → `thumbnail.rs:303` · `shell_menu.rs:136,165,173` — **이 잠금의 대상이 아님**을 동반 변경 판정에 근거와 함께 남김

## 동반 변경 판정
| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `src/fs/icons.rs:317-325`의 `SHELL_LOCK` doc 주석 | "셸을 실제로 부르는 **시험들**의 직렬화 잠금"이라 적혀 있는데 소유가 자원 함수로 옮겨간다. 겨냥 API 목록에 `ImageList_GetIcon`도 없다 | T3에 편입 |
| 필수 | `src/ui/icon_tex.rs:57`의 `get` doc 주석 | "변환 실패한 인덱스는 `None`으로 기억해 재시도하지 않는다" — T2가 이 문장을 직접 거짓으로 만든다 | T2에 편입 |
| 필수 | `src/ui/tree.rs:731-735`의 **프로덕션 주석** | 배지를 텍스처와 묶지 않는 **설계 근거**로 "아이콘 변환이 실패한 인덱스는 `IconTextures`가 `None`으로 기억해 다시 시도하지 않으므로"를 든다 — T2가 그 전제를 무효화한다(결론은 유효하니 근거만 고친다) | T2에 편입 |
| 필수 | `src/ui/tree.rs:957-959`의 시험 주석 | 그 시험의 **존재 이유**로 "`IconTextures`는 변환에 실패한 인덱스를 `None`으로 기억해 다시 시도하지 않는다"를 든다 — T2 이후 거짓이 되고, 어긋난 채 두면 다음 세션이 그 시험을 잘못 읽는다 | T2에 편입(`tree.rs`를 T2 Files에 넣는다) |
| 필수 | `src/ui/panel/tests.rs:103`의 `drive_labels` doc 주석 | "**잠금**·조회는 `drive_rows`가 한다"고 적혀 있는데 T3가 `drive_rows()`의 guard(`:123`)를 걷어낸다 — 1라운드 M6과 같은 갈래의 누락이다(`:116-120`의 ⓒ는 잠금이 함수 안으로 옮겨도 취지가 유효해 대상이 아니다) | T3에 편입 |
| 필수 | 위키 `moa/conventions.md`의 "`IconTextures`의 아이콘 변환은 셸 잠금 밖이라 병렬 시험에서 경합한다" 항목 | **미검증 가설을 사실처럼 적었다**(전제 4). T1의 실측 결과로 정정해야 한다 — 구현 세션은 위키 본문을 고치지 않으므로 `[K-DRIFT]` 큐 1줄 | T4에서 큐잉 |
| 필수 | Deferred 대장의 그 항목 | 이번에 착수해 해소한다 | F-6.5 종결(반영) 처리 |
| 무관 | `fs::thumbnail`의 `SHCreateItemFromParsingName`(`:303`) | `SHELL_LOCK` doc이 든 네 API에 없고, 그 워커는 **자기 COM 아파트먼트**에서 돌며(`:277` `CoInitializeEx`) 실패 양상도 16px 폴백이 아니다. 이번 잠금이 겨냥하는 자원(시스템 이미지 리스트)을 건드리지 않는다 | 건드리지 않음 |
| 무관 | `fs::shell_menu`의 `SHParseDisplayName`·`SHGetDesktopFolder`(`:136,165,173`) | 같은 이유 — 셸 네임스페이스 조회이지 이미지 리스트 자원이 아니다. 그 모듈에는 시험도 없다(HWND 필요) | 건드리지 않음 |
| 무관 | PRD·README | 화면 동작·아이콘 모양이 바뀌지 않는다. 복원력은 FR-5("파일·폴더에 시스템 아이콘을 표시한다")에 어긋나지 않고 오히려 충실해진다 — 문면에 담을 새 규칙이 없다 | 건드리지 않음 |
| 무관 | 세션 스키마·i18n | 저장 형식·화면 문구와 무관하다 | 건드리지 않음 |

## Decisions
### D1. 잠금 소유를 어디에 두는가
- **Options**: A) 자원을 만지는 함수가 잡는다(시험은 안 잡는다) / B) 지금처럼 시험이 잡고 변환 함수에도 넣는다 / C) 재진입 가능 잠금을 도입한다
- **Chosen**: A
- **Rationale**: B는 전제 3의 시험 5곳에서 곧바로 데드락이다(`std::sync::Mutex` 재진입 불가). C는 새 의존성이 필요하고, A로 소유를 한 층에 모으면 재진입 자체가 생기지 않는다. 잠금은 "자원을 아는 곳"에 있어야 하며 호출부가 잡으면 계층마다 재진입 위험이 생긴다.
- **Source**: `src/fs/icons.rs:327`(Mutex 타입) · 전제 3·6·7의 호출 관계.

### D2. 실행 파일에도 잠금을 넣는가
- **Options**: A) `#[cfg(test)]`로만 존재(현행 유지) / B) 언제나 잠근다
- **Chosen**: A
- **Rationale**: AGENTS의 UI 스레드 원칙상 그리기는 한 스레드이고 워커도 각자 자기 `IconCache`를 만든다 — 실행 파일에는 경합할 상대가 없다. B는 매 프레임 수백 번 잡는 잠금을 공짜가 아닌 값으로 넣는다.
- **Source**: `src/fs/icons.rs:326,331`(이미 `#[cfg(test)]`) · AGENTS.md 「UI 스레드 원칙」.

### D3. 복원력을 어떤 방식으로 주는가
- **Options**: A) `by_key`의 값에서 `Option`을 걷어내 실패를 담지 않는다 / B) **실패를 담되(현행 `Option` 유지) 다음 프레임에 별도 예산으로 재시도한다** / C) 일정 프레임마다 실패 항목을 비운다
- **Chosen**: B
- **Rationale**: A는 **성립하지 않는다** — 실패를 담지 않으면 *"이 키는 이미 실패했다"* 를 알 수 없고, 성공·실패는 **시도한 뒤에만** 알 수 있으므로 첫 시도와 재시도를 갈라 예산을 줄 수단이 사라진다. 그러면 재시도를 막으려는 어떤 개수 게이트도 **처음 보는 키까지 막아** 성공 텍스처를 영구히 굶긴다(D6에 상술). 실패 표시를 남기는 B는 그 갈림을 가능하게 하고, 값 타입을 안 바꾸므로 변경도 작다. C는 주기라는 새 상태를 들이면서 B보다 나은 점이 없다.
- **Source**: `src/ui/icon_tex.rs:22`(현행 `HashMap<(isize, i32), Option<TextureHandle>>`) · `:65-81`(현재 구조 — 실패를 담고 재시도하지 않는다) · 리뷰 3라운드 B1(첫 판은 A를 골랐다가 이 모순으로 되돌렸다).

### D4. 진단(T1)의 산출물을 남기는가
- **Options**: A) 관측 결과를 plan에 적고 **진단 코드는 남기지 않는다** / B) 회귀 시험으로 승격해 남긴다
- **Chosen**: A
- **Rationale**: 이 진단은 "지금 경합이 일어나는가"를 한 번 재는 것이고, 그 답이 무엇이든 T2·T3가 진행된다(전제 5). 상시 시험으로 남기면 **부하에 따라 결과가 달라지는 시험**이 되어 간헐 실패의 새 원인이 된다. 복원력의 회귀는 T2가 만드는 재시도 시험이 상시로 지킨다.
- **Source**: 대장의 `[2026-08-15] CPU가 붐빌 때 lib 시험 1건이 간헐 실패` — 부하 의존 시험이 이미 이 레포의 알려진 마찰이다.

### D5. 잠금을 함수의 어디에서 잡는가
- **Options**: A) **셸 호출 직전**(캐시 조회·조기 반환 뒤) / B) 함수 첫 줄
- **Chosen**: A
- **Rationale**: 이 함수들은 캐시 히트와 `is_dir` 조기 반환이 셸 호출보다 앞에 있고(전제 8), 렌더 경로가 **행마다·프레임마다** 부른다(`list_details.rs:404`·`list_grid.rs:225`·`tree.rs:403`). B는 캐시 히트까지 전역 뮤텍스로 직렬화해 **직전 회차에 실측된 10분 병리**를 재발시킨다(`ui/panel/tests.rs:116-120`).
- **Source**: 전제 8 · 리뷰 1라운드 M1.

### D6. 재시도를 어떻게 제한하는가
- **Options**: A) 첫 시도와 같은 예산(`created_this_frame`)을 쓴다 / B) 아무 제한도 두지 않는다 / C) 재시도 전용 프레임 예산만 둔다 / D) **재시도 전용 프레임 예산 + 키당 재시도 횟수 상한**
- **Chosen**: D
- **Rationale**: B는 매 프레임 `ImageList_GetIcon`+GDI를 무제한 반복해 현재보다 나쁜 성능 회귀다. A는 **처음 보는 키를 영구히 굶긴다** — 요청 순서 = 행 그리기 순서라 프레임마다 같은데, 앞선 실패 키들이 매 프레임 그 예산을 소진하면 뒤쪽의 처음 보는 키가 `return None`으로 막혀(`icon_tex.rs:65-69` — 캐시에 담지도 않는다) 다음 프레임에도 같은 자리에서 막힌다. C는 그 기아를 없애지만 **두 결함이 남는다**: ⓐ 늘 실패하는 키(1bpp 마스크만 있는 흑백 아이콘 — `color_bitmap_to_image`의 `:205-210`에서 **결정론적으로** `None`)가 요청 순서 앞쪽에 2개 있으면 뒤쪽의 *일시적* 실패 키에 차례가 영영 오지 않아 **이 plan의 목표가 프로덕션에서 통째로 무효화되고** ⓑ 성공할 수 없는 변환이 **매 프레임 2회씩 영구히** GDI 개체를 만들고 버린다(60fps면 초당 120회 — 현재는 0회다. B를 물리친 사유가 크기만 줄어 그대로 남는 셈이다). D는 키당 상한(3회)으로 둘을 함께 없앤다 — 늘 실패하는 키는 3회 뒤 예산을 놓고 프레임 비용이 0으로 돌아가며, 그 자리를 일시적 실패 키가 물려받는다.
- **정하는 값**: 재시도 프레임 예산 `MAX_FAILED_RETRIES_PER_FRAME = 2` · 키당 재시도 상한 `MAX_RETRIES_PER_KEY = 3`. 3회는 "일시적 경합이라면 그 사이에 풀린다"는 어림이며, 늘 실패하는 키에 무는 총 비용은 **키당 3회로 끝난다**(무한이 아니다).
- **비용**: 비공개 필드 둘(`retried_this_frame: usize` · `retries: HashMap<(isize, i32), u8>`)과 상수 둘. `retries`는 **실패한 키만** 담으므로 작고, 성공하면 그 키를 지운다. 리셋 자리는 이미 있다 — `begin_frame`(`icon_tex.rs:52-54`)이 프로덕션(`ui/app.rs:2592`)과 시험 하네스에서 프레임마다 불린다(프레임 카운터만 되돌리고 `retries`는 유지한다 — 그것이 키의 이력이다).
- **남는 한계(정직하게)**: 실패 키가 3개 이상이면 한 프레임에는 앞쪽 2개만 차례가 온다 — 다만 상한 덕에 **앞쪽이 3회로 비켜 주므로** 뒤쪽도 몇 프레임 뒤 반드시 차례를 받는다(C에서 영구였던 것이 유한이 됐다).
- **Source**: `src/ui/icon_tex.rs:31-36`(상한과 사유 "3096ms 스파이크") · `:65-69`(예산 소진 시 캐시 미삽입) · `:71-78`(현행은 **성공할 때만** `created_this_frame`을 올린다 — `image.map` 안) · `:205-210`(결정론적 실패 경로) · 리뷰 2R M2 · 3R B1 · 4R M1.
- **NFR-3과의 관계**: NFR-3(`docs/prd.md:95` — 10만 파일 폴더에서 UI 무정지)은 프레임당 작업량 상한에 기댄다. **재시도가 성공하면 `created_this_frame`도 올려** 프레임당 실제 텍스처 업로드를 8로 묶는다(업로드 비용은 첫 시도와 같으므로 같은 예산으로 센다). 즉 프레임당 최대치는 업로드 8회 + 실패 재시도 2회이며, 후자는 키당 3회로 끝난다.

## Tasks

- [x] T1. 병렬에서 아이콘 변환이 실패하는지 관측한다
  - **Type**: C
  - **Design**: ① 진단은 `src/ui/icon_tex.rs`의 `#[cfg(test)] mod tests`에 둔다(변환 함수와 같은 파일이라 `icon_to_image`를 직접 부를 수 있다). ② 신규 심볼 — 진단 시험 함수 1개(D4에 따라 **이 task 안에서 제거**한다). `std::thread::scope`로 스레드 몇 개를 띄워 같은 이미지 리스트·인덱스로 `icon_to_image`를 동시에 여러 번 부르고 `None` 횟수를 센다. ③ 의존 방향 — `ui::icon_tex`가 `fs::icons`를 참조한다(이미 그렇다). ④ 비추상화 선언 — 부하 생성기·통계 유틸을 만들지 않는다. ⑤ **이 task가 T3보다 앞이어야 하는 이유** — T3가 `icon_to_image`에 잠금을 넣으면 진단 스레드가 직렬화되어 `None` 관측 자체가 불가능해진다(아래 Edge 3과 같은 이유). 순서는 취향이 아니라 성립 조건이다.
  - **Acceptance**:
    - Given 유효한 이미지 리스트와 아이콘 인덱스, When 여러 스레드가 동시에 `icon_to_image`를 각각 여러 번 호출, Then **`None`이 몇 번 나왔는지 수치로 관측**된다(0이어도 관측이다)
    - 그 수치와 실행 조건(스레드 수·반복 수)을 **이 plan의 `## Investigation Log`에 기록**하고 전제 4의 `⚠ 미확인`을 그 결과로 갱신한다
    - **관측이 끝나면 진단 시험을 제거**한다(D4) — task 종료 시 `grep -c "thread::scope" src/ui/icon_tex.rs` → 0
    - `cargo build` · `cargo test --lib` · `cargo clippy --all-targets -- -D warnings` 전부 통과(진단을 넣은 상태와 제거한 상태 양쪽)
  - **Files**:
    - 주: `src/ui/icon_tex.rs`(임시 시험 추가 후 제거)
    - 동반: `docs/plans/2026-08-17-icon-texture-lock.md`(관측 결과 기록)
  - **Edge Cases**:
    - 이미지 리스트를 얻지 못하면(`IconCache::new` 실패) 진단이 성립하지 않는다 — "리스트 획득 실패로 관측 불가"를 기록하고 넘어간다(경합 없음으로 단정하지 않는다)
    - 스레드를 많이 띄워도 실패가 0이면 **"이 PC·이 부하에서는 재현되지 않는다"**로 적는다 — "경합이 없다"고 단정하지 않는다(다른 환경·부하에서의 부재를 증명한 것이 아니다)
    - 진단이 `shell_test_guard()`를 잡지 않게 한다 — 잠그면 직렬화되어 경합을 볼 수 없다(관측 목적과 반대)
    - `IconCache`는 `Send`가 아닐 수 있다 — 스레드마다 각자 만들거나, 이미지 리스트 핸들(`isize`)만 넘겨 스레드 안에서 쓴다(핸들은 프로세스 전역이라 유효 — 직전 회차 전제 4에서 확인됨)
  - **Halt Forecast**:
    - (i) 관측 결과가 무엇이든 T2·T3는 진행된다 → 전제 5에서 확정(사용자 결정)
  - **Depends on**: -

- [x] T2. 변환에 한 번 실패한 아이콘을 다음 프레임에 다시 시도한다
  - **Type**: C
  - **Design**: (1) `src/ui/icon_tex.rs`의 `IconTextures`. (2) 신규 심볼 — 필드 `retried_this_frame: usize` · `retries: HashMap<(isize, i32), u8>`와 모듈 상수 `MAX_FAILED_RETRIES_PER_FRAME = 2` · `MAX_RETRIES_PER_KEY = 3`. `by_key`의 **값 타입은 그대로 `Option<TextureHandle>`**(D3 — 실패 표시가 있어야 첫 시도와 재시도를 가를 수 있다). `get`을 세 갈래로 만든다 — **성공 캐시**면 곧바로 돌려주고(카운터를 건드리지 않는다), **처음 보는 키**면 지금처럼 `created_this_frame` 상한(8) 안에서 시도하고, **실패로 아는 키**면 `retries[key] < 3`이고 `retried_this_frame < 2`일 때만 다시 시도해 결과를 덮어쓴다. **카운터 규칙**: `retried_this_frame`과 `retries[key]`는 **시도할 때** 올린다(성공 여부와 무관 — 비용이 이미 발생했다), 변환이 **성공하면** `created_this_frame`도 올리고 `retries`에서 그 키를 지운다(D6의 NFR-3 항). `begin_frame`에서 **두 프레임 카운터만** 0으로 되돌린다(`retries`는 키의 이력이라 유지). (3) 의존 방향 — 변화 없다(`get`의 시그니처·반환 타입 불변이라 호출부 4곳은 손대지 않는다). (4) 비추상화 선언 — 재시도 커서·백오프·주기를 두지 않는다(D6의 한계를 그대로 받아들인다).
  - **Acceptance**:
    - **복원력(이 task의 핵심)**: Given 변환이 실패하는 인덱스, When 두 프레임에 걸쳐 `get`을 부름, Then **두 번째 프레임에도 변환이 다시 시도된다** — `retried_this_frame == 1`로 확인한다(같은 파일 `mod tests`에서 비공개 필드 열람 — `use super::*`가 이미 있다)
    - **성공 캐시는 그대로 듣는다**: Given 변환이 성공하는 인덱스(`IconCache::new()`의 `himl()`+`dir_icon()`으로 얻는다 — `src/fs/icons.rs:129,142`), When `get`을 두 번 부름, Then 두 번째는 캐시 히트다 — `created_this_frame`이 한 번만 올랐음으로 확인한다
    - **재시도가 프레임당 2회로 묶인다**: Given 실패 키 8개가 이미 캐시된 상태, When 그 8개를 한 프레임에 요청, Then `retried_this_frame`이 **2**에서 멎는다
    - **재시도가 새 아이콘을 굶기지 않는다**(D6의 갈래 1 — 이 항목이 없으면 영구 기아 회귀를 시험이 잡지 못한다): Given 실패 키 8개가 이미 캐시된 상태, When 그 8개를 **먼저** 요청한 뒤 같은 프레임에 처음 보는 성공 키를 요청, Then **성공 텍스처가 그 프레임에 올라간다**(재시도가 별도 예산을 쓰므로 처음 보는 키의 몫 8이 온전하다)
    - **늘 실패하는 키는 3회 뒤 재시도를 멈춘다**(D6의 갈래 2 — 이 항목이 없으면 결정론적 실패가 예산을 영구 점유하고 GDI 호출이 영구히 남는다): Given 결정론적으로 실패하는 키 하나, When 프레임을 다섯 번 돌리며 매번 요청, Then **재시도는 3회에서 멎는다** — 4·5번째 프레임의 `retried_this_frame`이 **0**이고 `retries[key] == 3`이다
    - `IconTextures::get`의 시그니처·반환 타입이 그대로여서 호출부 4곳(`list_details`·`list_grid`·`sidebar`·`tree`)이 손대지 않고 빌드된다
    - **T2가 거짓으로 만드는 주석 3곳이 새 동작으로 갱신된다**(동반 변경) — `src/ui/icon_tex.rs:57`(`get` doc) · `src/ui/tree.rs:731-735`(배지를 텍스처와 묶지 않는 **설계 근거**) · `src/ui/tree.rs:957-959`(그 시험의 존재 이유). 세 곳 모두 종전 문면이 "실패를 `None`으로 기억해 다시 시도하지 않는다"에 기대는데, **배지를 묶지 않는다는 결론 자체는 유효하다**(D6의 한계대로 결정론적 실패는 여전히 재시도되지 않는다) — 근거 문장만 고친다
    - `cargo test --lib` 전건 통과 · `cargo clippy --all-targets -- -D warnings` 경고 0 · `cargo fmt --check` 무차이
  - **Files**:
    - 주: `src/ui/icon_tex.rs`
    - 동반: `src/ui/tree.rs`(주석 2곳 — `:731-735`·`:957-959`)
    - 테스트: `src/ui/icon_tex.rs`(같은 파일 `mod tests` — 위 acceptance 4건)
  - **Edge Cases**:
    - **실패 키가 3개 이상이면 한 프레임에는 앞쪽 2개만 차례가 온다**(D6의 남는 한계) — 다만 키당 상한 덕에 앞쪽이 3회로 비켜 주므로 뒤쪽도 몇 프레임 뒤 반드시 차례를 받는다. 그 사실을 `get`의 doc에 적는다
    - **성공 변환 전제를 얻지 못하는 환경**: `IconCache::new()`가 이미지 리스트를 못 얻거나 그 인덱스 변환이 실패하면 acceptance 2·4번째(성공이 필요한 둘)는 성립하지 않는다 → 그 사실을 Progress Log에 적고 **셸 없이 성립하는 1·3·5번째만으로 판정**한다(T1 Edge와 같은 처리)
    - `retried_this_frame` 예산이 소진된 뒤의 반환은 **캐시된 `None`을 그대로 돌려주는 것**이다 — 별도 분기가 필요 없다(마지막 줄 `by_key.get(&key).and_then(|h| h.as_ref())`이 그 값을 준다)
    - 현행은 **성공할 때만** `created_this_frame`을 올린다(`:75` — `image.map` 안). 처음 보는 키의 갈래에서는 그 동작을 바꾸지 않는다 — 바꾸면 실패한 새 키가 그 프레임의 성공 몫을 잠식한다
    - `by_key`의 크기는 여전히 "시도한 수"다(실패도 담으므로) — 그 값을 쓰는 곳은 없다(사용처는 `icon_tex.rs:24,47,65,79,81`뿐이고 `IconTextures`에는 `len()`이 없다. `len`은 `ThumbnailTextures`의 것)
    - 신설 시험은 실제 이미지 리스트가 필요해 **T2 시점에는 `shell_test_guard()`를 잡는다** — T3가 그것도 걷어낸다(4-C)
  - **Halt Forecast**:
    - (i) 세 갈래를 어떻게 가르는가 → 위 Design에서 확정(성공 캐시 / 처음 보는 키 / 실패로 아는 키)
  - **Depends on**: T1

- [x] T3. 셸 잠금의 소유를 자원 함수로 옮긴다
  - **Type**: D
  - **Design**: ① 잠금 획득 함수는 `src/fs/icons.rs`(잠금이 사는 곳), 획득 지점은 `fs/icons.rs`·`fs/known_folders.rs`·`ui/icon_tex.rs`. ② 신규 심볼 — `pub(crate) fn shell_guard() -> ShellGuard`와 **`pub(crate)`** 로 공개된 빌드별 `ShellGuard`(`#[cfg(test)]`는 `MutexGuard<'static, ()>`를 감싸고, `#[cfg(not(test))]`는 빈 구조체 — 가시성을 맞추지 않으면 `private_interfaces` 경고로 `-D warnings`가 깨진다). **`SHELL_LOCK`은 그대로 재사용**하고 **poison을 관용한다**(`unwrap_or_else(|p| p.into_inner())` — 전제 9. `.unwrap()`은 AGENTS 금지). `shell_test_guard`와 `use` 2줄은 제거한다. ③ 의존 방향 — `ui::icon_tex`가 `fs::icons`를 참조한다(이미 `IconCache`로 그렇다). `fs::known_folders`도 이미 `fs::icons`를 쓴다. ④ 비추상화 선언 — 잠금을 트레이트·RAII 계층으로 감싸지 않는다. ⑤ **잠금 추가와 시험 guard 제거를 한 편집으로 끝낸다** — 중간 상태(잠금은 들어갔는데 guard가 남은 상태)에서 검증을 돌리면 그 자리에서 멎는다.
  - **Design (잠그는 곳 · 잠그지 않는 곳 · 잠그는 위치)**:
    - **잠근다 7곳** — `IconCache::new`(본문 첫 줄, 전체가 셸 호출) · `icon_index`(두 갈래 각각의 **셸 호출 직전** — 경로별 조회와 `lookup_by_attributes` 호출부) · `icon_index_for_path`(캐시 조회 뒤) · `shell_display_name`(캐시 조회 뒤) · `type_name`(캐시 조회 뒤) · `known_folders::known_folder`(본문 첫 줄) · `icon_to_image`(`index < 0` 조기 반환 뒤).
    - **잠그지 않는다 4곳** — `system_image_list`·`lookup_by_attributes`(private, 위 바깥 함수에서만 불린다) · `fs::drives::list_drives`·`fs::known_folders::default_favorites`(조합 함수 — 안에서 잠금 함수를 부른다).
    - **위치가 곧 성능이다**(D5) — 캐시 히트 앞에 잠그면 렌더 경로가 프레임마다 전역 뮤텍스를 잡아 스위트가 늘어진다(전제 8).
  - **Acceptance**:
    - `grep -rn "shell_test_guard" src/` → **0건**(호출 16 + `use` 2 + 정의 1 전부 사라진다)
    - 위 Design이 열거한 **7곳이 각각 잠금을 잡고**(잠그는 위치까지 그대로), `system_image_list`·`lookup_by_attributes`·`list_drives`·`default_favorites` **4곳은 잡지 않는다**
    - `cargo test --lib`가 **완주**한다(데드락으로 멎지 않는다) — 전건 통과
    - **스위트 소요 시간이 기준선 안이다**: 아래 「검증 순서」대로 lib 하네스만 돌려 **15초 이내**에 끝난다(전제 11 — 현재 실측 **2.0초**, 병리는 **10분+**. 정상의 7.5배까지 허용하면서 병리의 1/40에서 끊는다). **컴파일 시간을 재지 않는다** — T3 편집 직후 첫 `cargo test`는 windows-rs 링크까지 포함해 15초를 넘길 수 있고 그것은 성능 병리가 아니다
    - **그 실행이 시험을 실제로 돌렸다**: 보고된 시험 건수가 **0이 아니다**(현재 808건). 이 항목이 없으면 시험 0건짜리 하네스를 집고도 "완주·15초 이내"가 충족되어 **데드락 관문이 무증상으로 사라진다**(`target/debug/deps/moa-*.exe`가 실제로 **6개** 있고 그중 bin 하네스는 `#[test]`가 0건이다 — 2026-08-17 실측)
    - `cargo build` 경고 0 — 실행 파일 빌드에서 `ShellGuard`가 빈 구조체라 잠금 비용이 없다
    - `cargo clippy --all-targets -- -D warnings` 경고 0 · `cargo fmt --check` 무차이
    - `src/fs/icons.rs:317-325`의 `SHELL_LOCK` doc 주석이 새 소유("자원을 만지는 함수가 잡는다")를 서술하고 겨냥 API에 `ImageList_GetIcon`이 더해진다(동반 변경)
    - **재진입 불변식이 코드에 남는다**: `shell_guard` 또는 `SHELL_LOCK`의 doc에 **잡는 곳·잡지 않는 곳(조합 함수·private 내부)과 그 사유(재진입 데드락, 타임아웃 없음)**를 적는다. 이 불변식이 plan에만 있으면 **plan이 교체될 때 사라지고**, 다음 세션이 `list_drives`에 잠금을 더해 멎을 때 근거를 찾을 자리가 없다
    - `src/ui/panel/tests.rs:103`의 `drive_labels` doc 주석이 새 소유를 반영한다(동반 변경 — 종전 문면은 "잠금·조회는 `drive_rows`가 한다"였다)
  - **Files**:
    - 주: `src/fs/icons.rs` · `src/ui/icon_tex.rs` · `src/fs/known_folders.rs`
    - 동반: `src/fs/drives.rs`(시험 4곳 + `use` 1) · `src/ui/panel/tests.rs`(2곳) · `src/ui/tree.rs`(3곳) · `src/ui/icon_tex.rs`(T2가 신설한 시험 2곳)
  - **Edge Cases**:
    - **재진입 데드락** — 위 Design의 갈림이 유일한 방어다. `std::sync::Mutex`도 `cargo test`도 상한이 없어 **멎으면 그대로 매달린다**(자율 실행 중에는 Halt보다 나쁘다) — 그래서 아래 「검증 순서」를 지킨다. **상한을 씌우는 대상은 `cargo`가 아니라 시험 실행 파일 자체다**: `cargo`를 띄우면 `Kill()`이 죽이는 것은 래퍼뿐이고 **멎은 하네스가 고아로 남아** exe를 잠근 채 다음 링크를 `os error 5`로 깨뜨린다(PowerShell 5.1에는 프로세스 트리를 죽이는 `Kill(true)`가 없다 — 부득이 래퍼를 쓰면 `taskkill /T /F /PID`로 트리를 정리한다)
  - **T3의 검증 순서 (이 순서를 지킨다)**:
    1. `cargo test --lib --no-run --message-format=json`으로 빌드하고, 그 출력에서 **lib 하네스 경로를 취한다** — `reason == "compiler-artifact"`이고 `profile.test == true`이고 `target.kind`에 `"lib"`이 든 줄의 `executable`. **와일드카드로 고르지 않는다**(`moa-*.exe`가 6개이고 시험 0건짜리도 섞여 있다)
    2. 그 실행 파일을 `Start-Process -PassThru -NoNewWindow`로 띄우고 표준 출력·오류를 시스템 임시 폴더 파일로 돌린다(`-RedirectStandardOutput`·`-RedirectStandardError` — 돌리지 않으면 어느 시험이 왜 깨졌는지 볼 수 없어 재실행이 붙는다). `WaitForExit(60000)`이 거짓이면 `Kill()` → **곧 데드락이니 위 갈림을 되짚는다**
    3. 그 실행이 완주하고 시험 건수가 0이 아닌 것을 확인한 **뒤에야** `cargo test`(통합·doc 포함)를 상한 없이 돌린다 — **순서를 뒤집으면 완화책이 있는데도 세션이 매달린다**
    - **60초와 15초는 목적이 다르다** — 하나는 매달림을 끊는 탈출구, 하나는 성능 병리를 잡는 기준선이다
    - `#[cfg(not(test))]` 빈 구조체를 `let _guard = shell_guard();`로 받는다 — `let _ = ...`는 그 자리에서 drop되어 잠금이 무효가 된다(시험 빌드에서 조용히 직렬화가 풀린다)
    - 빈 구조체가 **미사용 필드·미사용 변수 경고**를 낼 수 있다 — 필드 없는 구조체로 두고 `_guard` 바인딩으로 받으면 걸리지 않는다
    - 시험에서 잠금을 걷어내면 **잠금 단위가 시험 전체에서 호출 하나로 좁아진다** — 한 시험이 여러 셸 호출을 하면 그 사이에 다른 시험이 끼어들 수 있다. `크기별로_서로_다른_이미지_리스트를_얻는다`(`icons.rs:367`)처럼 **연속 호출의 결과를 견주는 시험**이 영향을 받는지 확인한다(그 시험은 `IconCache::new` 한 번의 결과만 보므로 안전할 것으로 보이나 실행으로 확인한다)
    - poison 관용을 잃으면 assert 실패 1건이 `IconCache::new`를 쓰는 모든 시험을 패닉시켜 원인을 덮는다(전제 9)
  - **Halt Forecast**:
    - (i) 어느 함수를 어디서 잠그는가 → 위 Design에서 열거·위치까지 확정
    - (ii-a) `fs::icons`에 crate 범위 공개 심볼 2개(`shell_guard`·`ShellGuard`)가 생기고 `shell_test_guard`(기존 `pub(crate)` 심볼)가 제거되며, `IconCache`의 다섯 메서드와 `known_folder` 본문에 잠금 획득 줄이 들어간다 → `## 사전 승인 항목`에 등록
  - **Depends on**: T2

- [x] T4. 위키 큐에 정정을 남긴다
  - **Type**: A
  - **Acceptance**:
    - 위키 vault 루트 `pending.md`에 `[K-DRIFT]` 1줄 — `moa/conventions.md`의 "`IconTextures`의 아이콘 변환은 셸 잠금 밖이라 병렬 시험에서 경합한다" 항목이 **미검증 가설을 사실처럼 적었고**, T1의 실측 결과와 이번 수정으로 정정이 필요하다는 것
    - 그 줄에 T1의 관측 수치를 담는다(재현됐는지·안 됐는지)
    - **재진입 불변식도 같은 큐에 1줄** — "셸 잠금은 자원 함수가 잡고, 조합 함수(`list_drives`·`default_favorites`)와 private 내부(`system_image_list`·`lookup_by_attributes`)는 잡지 않는다(잠그면 타임아웃 없는 재진입 데드락)". 코드 doc(T3)과 위키 양쪽에 남겨 plan 교체 후에도 보존한다
  - **Files**:
    - 동반: 위키 vault 루트 `pending.md`(큐 1줄 — 위키 본문은 고치지 않는다)
  - **Edge Cases**:
    - vault가 없거나 읽지 못하면 큐잉을 조용히 건너뛰고 이 plan에 폴백 기록한다
    - 같은 요지가 이미 큐에 있으면 append를 생략한다(중복 억제)
  - **Halt Forecast**:
    - 없음 — 큐 1줄이고 vault 부재는 위 Edge Case가 처리한다
  - **Depends on**: T1, T3

## 사전 승인 항목 (일괄 승인 대상)
- T3 — `fs::icons`에 `pub(crate) fn shell_guard()`와 `pub(crate) struct ShellGuard`(빌드별)가 생긴다.
- T3 — 기존 `pub(crate) fn shell_test_guard`와 `use` 2줄이 **제거**된다(새 함수로 대체).
- T3 — `IconCache`의 다섯 메서드(`new`·`icon_index`·`icon_index_for_path`·`shell_display_name`·`type_name`)와 `known_folders::known_folder` 본문에 잠금 획득 줄이 들어간다(시그니처 불변).
- T2 — `IconTextures`에 비공개 필드 둘(`retried_this_frame`·`retries`)이 늘고 모듈 상수 둘(`MAX_FAILED_RETRIES_PER_FRAME = 2` · `MAX_RETRIES_PER_KEY = 3`)이 신설된다(`by_key`의 값 타입과 `get`의 시그니처·반환 타입은 불변).
- T1 — 진단 시험을 임시로 넣고 그 task 안에서 제거한다.
- **새 외부 의존성은 없다** — 잠금은 `std::sync::Mutex`를 그대로 쓴다.

## 불가피한 Halt (위임 불가)
- push·master 병합·태그·릴리즈·PR — 구현·검증이 끝난 뒤 별도 승인.
- 위 사전 승인 목록에 없는 파일 삭제·이동.

## Verification Strategy
- 빌드: `cargo build`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 형식: `cargo fmt --check`
- 단위·통합 시험: `cargo test`
- **T3는 「검증 순서」가 정본이다**(T3 Edge Cases) — ① json으로 lib 하네스 경로 취득 → ② 60초 상한을 씌워 그것만 실행 → ③ 완주·시험 건수 0 아님을 확인한 뒤에야 `cargo test` 전체. **이 순서를 뒤집지 않는다**
- 반복 확인: 같은 실행 파일을 5회 돌려 간헐 실패가 없는지, 그리고 **매 회 15초 이내**인지 본다(T3 acceptance의 기준선 — 현재 실측 2.0초)

## Phase Ledger

## Retry Ledger

## Progress Log
- T1-T2 완료 (커밋 78103bd, 11797e2): 경합 관측(재현 안 됨) + 실패 재시도 구현. 빌드/시험/clippy OK.
  - **T1 결정**: 전제 4를 "재현되지 않음(이 PC·이 부하 한정)"으로 확정 — 최대 16스레드 × 200회에서 `icon_to_image` 실패 0건. T3는 전제 5대로 구조 정합을 근거로 계속한다.
  - **T2 결정**: D3를 A(값 타입에서 `Option` 제거) → **B(실패 표시 유지)** 로, D6을 **재시도 전용 예산 + 키당 상한(3회)** 으로 확정 — 리뷰 3·4라운드가 잡은 영구 기아 때문이다. 실패 표시가 없으면 첫 시도와 재시도를 가를 수 없어, 어떤 개수 게이트도 처음 보는 키까지 막는다.
  - 리뷰 MINOR 2건(spec M1 시험 커버리지 한계 · quality m1 `convert_into` 2곳 추출)은 모두 `(판정 유보)`라 대장에 등재하지 않는다(F-6.5 등재 게이트).

## Next Steps

## Open Questions
- [x] Q1: 어디까지 할지 → **원인 규명 + 복원력 + 잠금 정리 전부**(2026-08-17 사용자 결정). 원인이 미확정이라는 사실을 밝히고 물었고, 전제 5로 "경합이 재현되지 않아도 plan이 성립함"을 근거에 담았다
