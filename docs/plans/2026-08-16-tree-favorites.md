# Plan: 폴더 트리 즐겨찾기 (2026-08-16)

**PRD**: docs/prd.md

## 요구 이해
- **원문 요청**: "폴더 트리 위쪽(c:\) 에 가로 라인(구분선)을 추가 하고 위쪽에 즐겨 찾기 목록을 표시함 / 폴더 트리에서 항목의 오른쪽 마우스 버튼을 클릭해서 컨텍스트 메뉴를 만들고 메뉴에 '즐겨 찾기' 메뉴 표시, 클릭하면 즐겨 찾기 목록에 추가됨 / 이미 즐겨찾기로 등록된 폴더인 경우 즐겨 찾기 메뉴 비활성화 표시 / 즐겨 찾기 목록에서 마우스 오른쪽 버튼을 클릭해서 컨텍스트 메뉴에 '해제' 메뉴 추가, 클릭시 즐겨 찾기 항목에서 삭제됨 / 즐겨 찾기 목록은 모든 탭에 공통으로 표시해서 관리 / 원격 트리는 즐겨 찾기기능 제외"
- **이해한 요구**: 로컬 폴더 트리 맨 위에 **즐겨찾기 목록**을 두고 그 아래 가로 구분선으로 드라이브 트리와 나눈다.
  트리 항목 우클릭 메뉴의 `즐겨찾기`로 등록하고(이미 등록된 폴더면 비활성), 즐겨찾기 항목 우클릭 메뉴의 `해제`로 뺀다.
  즐겨찾기는 **앱 전역 하나**로 모든 워크스페이스·패널·탭이 같은 목록을 보며, 재시작해도 남는다(세션 파일에 저장).
  **원격 트리에는 즐겨찾기 영역도 컨텍스트 메뉴도 없다.**
- **포함하지 않는 것으로 이해**: 오른쪽 파일 목록의 우클릭(셸) 메뉴에는 즐겨찾기를 넣지 않는다 — 요청은 트리에 한정된다.

## Goal
로컬 폴더 트리에서 자주 쓰는 폴더를 즐겨찾기로 등록·해제하고, 트리 맨 위에서 한 번에 그리로 이동할 수 있다.

## PRD Coverage
| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-56 (신설 — 폴더 트리 즐겨찾기) | Should | T1·T2·T3 | ✅ 커버 (문면 추가는 T4) |
| FR-9 (폴더 트리 토글·지연 확장) | Should | T2 (상단 구역만 더하고 기존 동작 보존) | ✅ 커버 |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope
- 파일 목록(셸 컨텍스트 메뉴)에서의 즐겨찾기 등록
- 원격(FTP/SFTP) 폴더의 즐겨찾기 — 사용자 명시 제외
- 즐겨찾기 순서 바꾸기(드래그 정렬)·이름 바꾸기·항목별 아이콘
- 즐겨찾기 개수 상한·경로 정규화(대소문자·심볼릭 링크 해석)

## Deferred / Follow-up
- 즐겨찾기가 가리키는 폴더가 사라졌을 때의 표시(흐린 글씨·자동 정리) — 이번에는 눌렀을 때 기존 실패 안내로만 알린다(시작할 때 전건을 확인하면 네트워크 드라이브에서 창이 늦게 뜬다)
- 즐겨찾기 순서 바꾸기 — 항목이 많아지면 필요해진다
- 트리 메뉴의 화면 밖 보정에 넘기는 크기가 실측이 아니라 어림값이다(`MENU_WIDTH + PAD*2` 등) — 원격 메뉴는 `remote_menu::menu_size()`처럼 실측 상수를 쓴다. 창 하단에서 몇 px 넘칠 여지가 있으나 "열자마자 닫힘" 같은 동작 결함은 없다(보정은 포인터 쪽으로만 당긴다) (F-7 m4)
- [SUGGEST] `tree.rs::menu_row`가 `remote_menu.rs::menu_row`와 본문·상수까지 같다 — 도메인 지식이 없는 순수 렌더링 헬퍼라 `clamp_menu_pos`처럼 공용화할 여지가 있다. 지금은 2곳이라 문턱(3회) 미달 (T3 quality 리뷰 S1)
- [SUGGEST] `tree.rs`의 즐겨찾기 줄과 하위 없는 잎 노드가 "들여쓰기 + `selectable_label` + 클릭 시 `select`"로 거의 같은 모양이다 — 지금은 2곳이라 공통화 문턱(3회)에 못 미친다. 세 번째 유사 지점이 생기면 헬퍼로 뽑을지 재검토 (T2 quality 리뷰 S1)

## Investigation Log
- 위키 참조: `20_projects/personal/moa/conventions.md` — ① `ExplorerApp`은 단위 시험에서 만들 수 없다(생성자가 `eframe::CreationContext`를 받는다) → 판정 로직은 그 밖의 타입에 둬야 검증된다 ② `src/app/window.rs` 등 구 Win32 코드가 **여전히 컴파일 대상**이라 `Session` 같은 공용 타입을 바꾸면 그 파일도 함께 고쳐야 한다 ③ 함수를 사이에 끼워 넣을 때 앞 함수의 doc 주석이 딸려 붙지 않게 빈 줄을 확인한다
- 위키 참조: `20_projects/personal/moa/feat-navigation.md` — 트리는 로컬·원격이 같은 화면 코드를 쓰고 다른 것은 "무엇을 뿌리로 삼고 하위를 어디서 읽는가"뿐이다. 동기화는 단방향(트리→목록)
- 위키 참조: `20_projects/personal/moa/decisions.md` — 즐겨찾기 관련 과거 결정 없음(원격 시험 기반 결정만 확인)
- Deferred 대장(`docs/plans/deferred.md`) 확인 — 즐겨찾기 관련 항목 없음. `## 대기` 잔량이 소진 batch 임계(100건) 미만이라 이번 plan에 batch task를 넣지 않는다
- PRD `docs/prd.md:114` Out of Scope에 `즐겨찾기` 한 줄이 있다 → 사용자 승인(2026-08-16)으로 FR-56 신설·제외 해제
- `src/ui/tree.rs:139-155` `show`가 `TreeSource`로 로컬·원격을 갈라 그린다. 로컬은 `roots`(드라이브)부터 `show_node` 재귀
- `src/app/settings.rs:43-66` `Session`은 `sites`·`queue`·`dock`·`settings`를 `#[serde(default)]`로 담는다. `settings`만 `settings_or_default`로 **그 자리만** 기본값 처리해 세션 전체 폴백을 막는다
- `Session` 리터럴은 **5곳**이다 — 전체 나열 3곳(`src/ui/session.rs:79` to_session · `src/app/settings.rs:552` 시험 sample · `src/app/window.rs:1005` 구 Win32 저장 경로, 컴파일 대상)과 **스프레드 2곳**(`src/app/settings.rs:528` promote_v2 · `src/ui/app.rs:761` collect_session). **스프레드 쪽은 필드를 더해도 컴파일이 깨지지 않는다** — `collect_session`이 그 자리라, 거기서 즐겨찾기를 싣지 않으면 빈 목록이 조용히 저장된다(D7)
- `src/ui/app.rs:705-712` 복원부가 `sites`·`queue`·`dock`·`settings`를 세션에서 옮겨 담는다. `collect_session`(:732-777)은 `settings`만 따로 싣고 나머지는 `to_session`이 만든다
- `src/ui/panel.rs:1156-1170` 트리는 `scope_builder` 안에서 `self.tree.show(ui, source)` 한 곳에서만 그려지고, 결과(`TreeOutcome`)의 `chosen`·`requests`를 `:1186`에서 소비한다
- `src/ui/panel.rs`의 `show_remote_menu` — 우클릭 메뉴를 `egui::Area(Foreground)` + `Frame::menu` + `clamp_menu_pos`로 그리고 바깥 클릭·Esc로 닫는 선례가 있다
- `src/ui/splitter.rs:82-95` `merge_panel_outcome`이 `PanelOutcome`을 **구조 분해**로 받는다 → 필드를 더하면 컴파일 오류로 드러나 누락될 수 없다
- `grep -rn "즐겨찾기|favorite|bookmark" src/` → 0건(기존 구현 없음)

### 전제 검증
| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | `Session`에 `#[serde(default)]` 필드를 더하면 스키마 버전을 올리지 않아도 기존 파일이 그대로 살아난다 | `src/app/settings.rs:60-64`의 `settings` 선례와 그 주석(D2) | ✅ |
| 2 | `Session` 필드 추가로 손댈 곳은 리터럴 5곳(전체 나열 3 + 스프레드 2)이고, **스프레드 쪽은 컴파일러가 잡아 주지 않는다** | `grep -rn "Session {" src/` → 위 Log의 5곳. `src/ui/app.rs:761`(collect_session)·`src/app/settings.rs:528`(promote_v2)이 `..` 스프레드 | ✅ |
| 3 | 트리 그리기 진입점이 한 곳이라 시그니처를 바꿔도 호출부가 하나다 | `grep -rn "\.tree\.show(" src/` → `src/ui/panel.rs:1165` 1건 | ✅ |
| 4 | `PanelOutcome`에 필드를 더하면 `splitter`가 컴파일 오류로 알려 준다 | `src/ui/splitter.rs:82-95` 구조 분해 | ✅ |
| 4-b | **앞뒤 두 홉은 잡아 주지 않는다** — ① `TreeOutcome`은 `panel.rs`가 필드별로 소비하고 ② `LayoutOutcome`은 `default()`로 만들어져 `ui::app`이 필드별로 대입해 읽는다 | ① `src/ui/panel.rs:1186-1201` ② `src/ui/splitter.rs:158` · `src/ui/app.rs:2691-2698` | ✅ (①은 4-C의 `panel/tests.rs` 시험이 덮고, ②는 D8로 적용 규칙을 순수 함수에 두되 배선 한 줄은 수동 확인 항목) |
| 8 | `collect_session`은 비공개이고 `ExplorerApp`은 시험에서 만들 수 없어, "모은 세션"을 직접 시험할 수 없다 | `src/ui/app.rs:732`(비공개) · `src/ui/app.rs:578`(생성자가 `CreationContext`를 받는다) · `src/main.rs:80`(유일 생성 지점) | ✅ (D7의 `with_favorites` seam으로 우회) |
| 5 | 앱 전역 상태를 트리까지 내려보내는 길이 이미 있다(`RemoteView`가 같은 길로 내려간다) | `src/ui/app.rs:2674-2678` → `splitter::show_layout` → `panel.show` → `tree.show` | ✅ |
| 6 | 저장소 로직을 `ExplorerApp` 밖 타입에 두면 단위 시험으로 덮인다 | 위키 conventions 2026-08-15 항목 + `src/app/settings.rs`의 기존 순수 타입 시험 | ✅ |
| 7 | 우클릭은 `Response::secondary_clicked()`로 잡고 그 자리에 `Area`로 메뉴를 띄우는 선례가 있다 | `src/ui/panel.rs`의 `remote_menu_at` 설정부·`show_remote_menu` | ✅ |

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| 트리 항목은 `CollapsingState::show_header` 안에서 그려져 우클릭 응답을 어디서 잡을지 헷갈릴 수 있다 | 메뉴가 안 뜨거나 엉뚱한 항목이 대상이 된다 | 라벨을 그리는 `selectable_label`의 `Response`에서 `secondary_clicked()`를 본다 — 그 응답이 곧 그 항목이다 |
| 즐겨찾기 목록과 드라이브 트리에 같은 폴더가 동시에 보일 수 있다 | 선택 강조가 두 곳에 켜진다 | 같은 곳을 가리키므로 의도된 동작으로 둔다(Edge Case에 명시) |
| 메뉴가 트리의 `ScrollArea` 안에서 열린다 | 스크롤에 딸려가거나 잘린다 | 메뉴는 `ScrollArea` 밖 `egui::Area(Foreground)`에 그린다(원격 메뉴와 같은 방식) |
| `TreeOutcome`→`PanelOutcome`, `LayoutOutcome`→`ui::app` 두 홉은 필드별 소비라 한 줄을 빠뜨려도 컴파일이 통과한다(전제 4-b) | 메뉴는 뜨고 액션도 올라오는데 목록이 바뀌지 않는다 | ① 적용 규칙을 `FavoriteStore::apply`(순수)로 모아 시험한다(D8) ② 배선 한 줄은 `Verification Strategy`의 **수동 확인 항목**으로 명시한다(등록·해제가 화면에 반영되는지) |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `Session`(필드 `favorites` 추가) | `src/app/settings.rs`(정의·손상 방어·시험 sample) · `src/ui/session.rs`(to_session) · `src/app/window.rs:1005`(구 Win32 저장) · `src/ui/app.rs`(복원·collect_session) | 직렬화 계약 확장(비파괴) |
| `FolderTreeView::show`(인자 `favorites` 추가) | `src/ui/tree.rs`(정의) · `src/ui/panel.rs:1165`(유일 호출부) | 시그니처 변경 |
| `TreeOutcome`(필드 `favorite` 추가) | `src/ui/tree.rs` · `src/ui/panel.rs:1186`(소비) | 구조체 확장 |
| `PanelOutcome`(필드 `favorite` 추가) | `src/ui/panel.rs`(생성) · `src/ui/splitter.rs`(구조 분해·병합) | 구조체 확장 |
| `LayoutOutcome`(필드 `favorite` 추가) | `src/ui/splitter.rs:158`(정의·`default()`·병합) · `src/ui/app.rs:2691-2698`(소비 — 필드별 대입) | 구조체 확장 — **컴파일러가 잡아 주지 않는다**(`default()`로 채워지고 소비도 필드별이라, 읽는 줄을 빠뜨리면 조용히 지나간다). 방어는 D8의 순수 `apply`와 아래 위험 항목 |
| `PanelState::show`(인자 `favorites` 추가) | `src/ui/panel.rs`(정의) · `src/ui/splitter.rs`(호출) · `src/ui/panel/tests.rs`(시험 호출) | 시그니처 변경 |
| `splitter::show_layout`(인자 `favorites` 추가) | `src/ui/splitter.rs`(정의) · `src/ui/app.rs:2666`(유일 호출부) | 시그니처 변경 |
| `clamp_menu_pos`(`panel.rs` → `ui::menu`로 이동, D6) | `src/ui/panel.rs`(정의 `:99`·사용 `:1475`) · `src/ui/menu.rs`(새 위치) · `src/ui/tree.rs`(새 사용처) · `src/ui/panel/tests.rs:940·945·951`(그 시험 함수 하나를 `src/ui/menu.rs`의 기존 `mod tests`로 **함께 옮긴다** — 순수 위치 계산은 함수와 같은 곳에서 시험한다) | 구조 변경(비공개 → `pub(crate)`) |

### 4-B. 계약·직렬화 변경
- `Session.favorites: Vec<String>` 추가 — `#[serde(default)]`라 이 필드가 없는 기존 파일도 그대로 읽힌다(전제 1). 스키마 버전(v3)은 올리지 않는다
- 손상 방어: `settings`와 같은 방식으로 **그 자리만** 기본값 처리한다 — 즐겨찾기 하나가 깨져 워크스페이스·탭·큐를 통째로 잃지 않게 한다(D5)
- 타입은 `Vec<String>`이다(D9) — 기존 스키마가 경로를 문자열로 담고, `PathBuf`는 UTF-8이 아닌 경로에서 직렬화가 실패하는데 `save_session`이 그 실패를 삼켜 **저장 전체가 무산**된다(`src/app/settings.rs:449-451`)

### 4-C. 테스트 파일
- `src/app/favorites.rs` — 신규 모듈의 `#[cfg(test)] mod tests`(추가·중복·해제·순서)
- `src/app/settings.rs`의 `mod tests` — `sample()` 갱신 + `favorites` 없는 JSON·깨진 `favorites` JSON 복원 시험
- `src/ui/session.rs`의 `mod tests` — `with_favorites`의 직렬화 왕복(D7 seam)
- `src/ui/menu.rs`의 `mod tests` — `panel/tests.rs`에서 옮겨 온 `clamp_menu_pos` 시험(D6)
- ~~`src/ui/tree.rs`의 `mod tests`~~ → **실제 위치는 `src/ui/panel/tests.rs`** — 트리를 패널 경유로 실제로 그려야 좌표를 얻고 클릭·우클릭 이벤트를 주입할 수 있어, 계획한 자리보다 실물에 가까운 검증이 됐다(F-7 m1 정정)
- `src/ui/panel/tests.rs` — `draw_once` 헬퍼의 `PanelState::show` 호출 갱신, 액션이 `PanelOutcome`으로 올라오는지
- `src/ui/splitter.rs`의 `mod tests` — 액션 병합

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `app::favorites::FavoriteStore` | `grep -rn "즐겨찾기|favorite|bookmark" src/` → 0건. `remote::sites::SiteStore`가 "이름 붙은 목록을 세션에 담는" 같은 꼴 | 신규 — `SiteStore`는 접속 정보(비밀번호·프로토콜)를 담아 성질이 다르다. 다만 **API 모양은 `SiteStore`를 따른다**(생성·추가·제거·조회) |
| `app::favorites::FavoriteAction`(D8) | `ui::remote_menu::RemoteMenuAction`(원격 목록 메뉴의 같은 역할) | 신규 — 대상(로컬 경로 vs 원격 항목)과 항목이 달라 한 enum에 넣으면 양쪽 제약이 섞인다(`remote_menu.rs` 모듈 주석의 비추상화 선언과 같은 판단) |
| 트리 메뉴 그리기 | `panel.rs::show_remote_menu`의 `Area`+`Frame::menu`+`clamp_menu_pos` | **재사용** — `clamp_menu_pos`를 `ui::menu`로 올려(D6) 두 곳이 같은 함수를 쓰고, 프레임 스타일(배경·테두리·모서리)도 같은 값으로 맞춘다 |

### Verified by
- `grep -rn "\.tree\.show(" src/` → 1 hit(`panel.rs:1165`), 표에 포함
- `grep -rn "Session {" src/` → 리터럴 5곳(전체 나열 3 + 스프레드 2), 전부 표에 포함. `promote_v2`는 스프레드라 무영향이지만 `collect_session`은 **스프레드인데도 손대야 한다**(D7)
- `grep -rln "PanelOutcome" src/` → 2 파일(`panel.rs`·`splitter.rs`), 표에 포함
- `grep -rn "show_layout(" src/` → 정의 1 + 호출 1(`app.rs`), 표에 포함
- `grep -rn "clamp_menu_pos" src/` → 정의 1(`panel.rs:99`) + 사용 1(`panel.rs:1475`) + 시험 3(`panel/tests.rs:940·945·951`, 한 시험 함수 안), 전부 표에 포함

## 동반 변경 판정
| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | PRD `Out of Scope`의 `즐겨찾기` 줄 제거 + FR-56 신설 + 결정 이력 | 이 줄이 남으면 PRD가 "제외"라고 적은 기능이 앱에 존재하게 되어 문서가 코드와 어긋나고, Phase G 재검증의 기준 자체가 틀어진다 | T4에 편입 |
| 필수 | `README.md` 폴더 트리 항목에 즐겨찾기 서술 | README는 "현재 존재하는 기능"을 적는 문서다(공통 지침) | T4에 편입 |
| 필수 | 위키 `feat-navigation.md`의 트리 서술 | 위키는 세션 밖 정본이라 어긋난 채 두면 다음 세션이 옛 서술을 근거로 삼는다. 다만 위키 갱신은 별도 세션 규약이므로 **큐 1줄로 올린다**(구현 세션이 직접 쓰지 않는다) | T4에서 큐 기록 |
| 무관 | `AGENTS.md` | 빌드·시험 명령과 구조 규약이 바뀌지 않는다(새 파일 1개는 기존 `app/` 계층 안) | 건드리지 않음 |
| 무관 | 세션 스키마 버전(v3)·`promote_v2` | `#[serde(default)]` 확장이라 승격 경로가 그대로다(전제 1) | 건드리지 않음 |

## Decisions
### D1. 즐겨찾기를 어디에 저장하는가
- **Options**: A) `Session`의 새 최상위 필드 `favorites` / B) `AppSettings` 안 / C) 별도 파일(`favorites.json`)
- **Chosen**: A
- **Rationale**: 즐겨찾기는 "앱 설정"이 아니라 사용자 데이터라 `sites`·`queue`와 같은 층이 맞다. 별도 파일은 저장 지점이 둘로 늘어 종료 시 원자성이 갈린다
- **Source**: `src/app/settings.rs:43-66`(sites·queue·dock 선례)

### D2. 전역 상태를 트리까지 내려보내는 길
- **Options**: A) `ExplorerApp`이 소유하고 `show_layout → panel.show → tree.show`로 참조를 내려보낸다(`RemoteView`와 같은 길) / B) 트리마다 사본을 두고 앱이 동기화 / C) 전역 static
- **Chosen**: A
- **Rationale**: 사본을 두면 패널이 늘 때마다 동기화 지점이 늘고, 전역 static은 시험이 서로를 오염시킨다. A는 이미 `RemoteView`가 쓰는 길이라 새 구조가 없다
- **Source**: `src/ui/app.rs:2674-2678`

### D3. 즐겨찾기 변경을 앱까지 올리는 길
- **Options**: A) `TreeOutcome → PanelOutcome → LayoutOutcome`으로 값만 올리고 앱이 적용 / B) 트리가 `&mut FavoriteStore`를 받아 직접 고침
- **Chosen**: A
- **Rationale**: 트리는 "고른 것"을 값으로 올려보내는 구조로 이미 서 있다(`chosen`·`requests`). B는 그리는 도중 상태를 바꿔 같은 프레임의 다른 패널과 어긋난다
- **Source**: `src/ui/tree.rs:66-73`(TreeOutcome), `src/ui/panel.rs:1186`

### D4. 메뉴 문구
- **Options**: A) `즐겨찾기` / `해제` / B) `즐겨찾기에 추가` / `즐겨찾기 해제`
- **Chosen**: A (사용자 요청 표기 — 맞춤법상 `즐겨찾기`는 붙여 쓴다)
- **Rationale**: 메뉴가 한 줄뿐이라 무엇에 대한 것인지 분명하다
- **Source**: 사용자 요청 원문. 영어는 `Add to favorites` / `Remove from favorites`(영어에는 그런 축약 관용이 없다)

### D5. 손상된 `favorites` 값의 처리
- **Options**: A) `settings`처럼 그 자리만 빈 목록으로 삼킨다 / B) `sites`·`queue`처럼 `#[serde(default)]`만 둔다(타입이 어긋나면 세션 전체 폴백)
- **Chosen**: A
- **Rationale**: 즐겨찾기 하나가 깨져 워크스페이스·탭·큐를 통째로 잃는 것은 대가가 맞지 않는다. `settings_or_default`와 같은 판단이고 코드도 같은 꼴이다
- **Source**: `src/app/settings.rs:66-84`(settings_or_default 주석)

### D6. `clamp_menu_pos`를 어디로 옮기는가
- **Options**: A) `src/ui/menu.rs` / B) `src/ui/widgets.rs` / C) `src/ui/list_common.rs` / D) 옮기지 않고 `panel.rs`의 것을 `pub(crate)`로만 연다
- **Chosen**: A — `pub(crate) fn clamp_menu_pos`로 `src/ui/menu.rs`에 둔다
- **Rationale**: `menu.rs`는 "상태를 바꾸지 않고 명령만 값으로 돌려준다"는 순수 계산 모듈이라(모듈 주석) 위치 계산 함수의 성질과 같고, `panel`·`tree` 어느 쪽에도 치우치지 않는다. D는 `panel.rs`가 이미 1584줄로 분리 검토선(1500)을 넘겨 더 얹지 않는 편이 낫다
- **Source**: `src/ui/menu.rs:1-8`(모듈 주석) · `src/ui/panel.rs`(1584줄) · `AGENTS.md`의 "파일 1500라인 내외"

### D7. 즐겨찾기를 세션에 싣는 자리
- **Options**: A) `session::to_session`의 시그니처를 넓혀 받는다 / B) `to_session`은 빈 목록을 넣고 `ExplorerApp::collect_session`이 자기 값으로 덮는다
- **Chosen**: B — 단 덮는 일을 `ExplorerApp` 안에서 하지 않고 **순수 함수 `session::with_favorites(session, favorites) -> Session`** 을 거친다
- **Rationale**: `settings`가 이미 같은 방식이고 그 이유가 코드 주석에 적혀 있다 — `to_session`은 창·워크스페이스를 옮기는 자리라 인자를 더하면 책임이 흐려지고 호출부가 함께 는다(주석은 "테스트 7곳"이라 적지만 현재 실제 호출부는 `src/ui/session.rs`의 시험 12곳이다).
  **B는 컴파일러가 잡아 주지 않는 갈래인데**(`collect_session`이 스프레드다 — 전제 2), `collect_session` 자체는 비공개이고 `ExplorerApp`은 단위 시험에서 만들 수 없어(전제 8) "모은 세션"을 시험으로 덮을 수 없다.
  그래서 **덮는 규칙만 순수 함수로 떼어** 시험이 그 함수를 직접 부르게 한다 — `collect_session`은 `with_favorites(Session { settings, ..to_session(..) }, self.favorites.paths())` 한 줄이 되고, 시험은 그 함수의 입출력을 본다
- **Source**: `src/ui/app.rs:732`(collect_session — 비공개) · `:759-777`(그 주석과 스프레드) · `src/ui/session.rs:72-88`(to_session) · `src/main.rs:80`(`ExplorerApp` 생성이 여기 한 곳뿐)

### D8. 즐겨찾기 액션 타입을 어느 계층에 두는가
- **Options**: A) `ui::tree`에 `FavoriteAction`을 정의하고 `ui::app`이 해석 / B) `app::favorites`에 정의하고 `FavoriteStore::apply(action)`이 적용까지 맡는다(`ui::tree`는 그 타입을 만들기만)
- **Chosen**: B
- **Rationale**: A로 두면 **적용 규칙이 `ExplorerApp` 안에 남아 시험에 덮이지 않는다**(전제 6). B는 의존 방향과도 맞다 — `app`은 `ui`를 모르고 `ui`가 `app`을 참조한다(AGENTS.md의 계층 규칙). `ui::app`은 `self.favorites.apply(action)` 한 줄만 부르고, 무엇이 늘고 주는지는 순수 시험이 판정한다
- **Source**: `AGENTS.md`의 "의존은 단방향이며 `ui`만 상위다" · 위키 conventions 2026-08-15(ExplorerApp 시험 불가 → 판정 로직을 밖으로)

### D9. 세션에 담는 경로의 타입
- **Options**: A) `Vec<PathBuf>` / B) `Vec<String>`
- **Chosen**: B
- **Rationale**: 기존 세션 스키마가 경로를 문자열로 담는다(`TabSession.path: String`). `PathBuf`는 UTF-8이 아닌 경로에서 **직렬화 자체가 실패**하는데 `save_session`은 그 실패를 조용히 삼켜 **세션 저장이 통째로 무산**된다(D5의 방어는 읽는 쪽만 덮는다). 즐겨찾기 하나 때문에 창 위치·탭까지 잃지 않게 기존 선례를 따른다
- **Source**: `src/app/settings.rs:268-290`(TabSession.path) · `src/app/settings.rs:449-451`(save_session이 실패를 삼킨다)

## Tasks

- [x] T1. 즐겨찾기 저장소와 세션 스키마
  - **Type**: C
  - **Design**: ① `src/app/favorites.rs` 신규 — `app` 계층(순수 로직, `ui`를 모른다) ② 신규 심볼 — `FavoriteStore`(추가한 차례를 지키는 폴더 목록: `add`(이미 있으면 무시)·`remove`·`contains`·`paths`·`from_paths`/`into_paths`)와 **`FavoriteAction { Add(PathBuf), Remove(PathBuf) }` · `FavoriteStore::apply(action)`**(D8 — 적용 규칙을 이 계층에 둬야 시험에 덮인다), 그리고 `src/ui/session.rs`의 **`pub fn with_favorites(session, favorites) -> Session`**(D7 — 스프레드 사각지대를 덮는 순수 seam) ③ `app::settings::Session`이 `Vec<String>`으로 담고(D9) `ui::app`이 `FavoriteStore`를 소유한다. **경로↔문자열 변환은 `ui::session` 한 곳에서 한다** — 내보낼 때 `with_favorites`가 `to_string_lossy()`로 바꾸고(`to_tab_session`의 선례 `src/ui/session.rs:144`), 되살릴 때 `ui::app` 복원부가 `FavoriteStore::from_paths(session.favorites.iter().map(PathBuf::from))`로 받는다. `FavoriteStore`는 언제나 `PathBuf`만 다룬다. `app::favorites`는 아무것도 참조하지 않고, `ui::tree`가 `FavoriteAction`을 만들어 올린다 ④ 비추상화 선언: 트레이트·옵저버·정렬 옵션·개수 상한을 두지 않는다(`AppSettings`와 같은 판단 — 값 하나를 매 프레임 읽으면 충분하다)
  - **Acceptance**:
    - Given 빈 저장소, When 같은 경로를 두 번 `add`, Then 목록은 1건이고 순서는 추가한 차례 그대로다
    - Given 2건이 든 저장소, When 앞의 것을 `remove`, Then 나머지 순서가 그대로 유지된다
    - Given `favorites` 키가 **없는** v3 세션 JSON, When `parse_session`, Then 세션이 살아나고 즐겨찾기는 빈 목록이다
    - Given `favorites`가 문자열 등 **엉뚱한 타입**인 세션 JSON, When `parse_session`, Then 워크스페이스·탭·큐는 그대로 살아나고 즐겨찾기만 빈 목록이다 (D5)
    - Given `to_session`이 만든(즐겨찾기가 빈) 세션과 즐겨찾기 2건, When `session::with_favorites`를 거쳐 직렬화하고 다시 읽음, Then 두 건이 순서 그대로 살아난다 — **`collect_session`이 스프레드라 컴파일러가 잡지 않는 자리**(전제 2)를 이 순수 함수 시험이 대신 지킨다(D7). `collect_session` 자체는 비공개이고 `ExplorerApp`을 시험에서 만들 수 없어(전제 8) 직접 부르지 않는다
    - Given 즐겨찾기 1건이 든 저장소, When `apply(FavoriteAction::Add(다른 폴더))`·`apply(FavoriteAction::Remove(첫 폴더))`, Then 목록이 각각 늘고 준다 (D8 — 적용 규칙이 시험에 덮인다)
    - `cargo test` 통과, `cargo clippy --all-targets -- -D warnings` 경고 0
  - **Files**:
    - 주: `src/app/favorites.rs`(신규) · `src/app/settings.rs` · `src/app/mod.rs`(모듈 등록) · `src/ui/app.rs`(`ExplorerApp.favorites` 필드 · 복원 `:705-712` · `collect_session` `:761`에서 싣기 — D7)
    - 주(이어서): `src/ui/session.rs`(`to_session`은 빈 목록 · **신규 `with_favorites`** — D7)
    - 동반: `src/app/window.rs:1005`(구 Win32 저장 경로 — 컴파일 유지, 값은 기본값)
    - 테스트: `src/app/favorites.rs`의 `mod tests`(추가·중복·해제·순서·`apply`) · `src/app/settings.rs`의 `mod tests`(`favorites` 없는/깨진 JSON) · `src/ui/session.rs`의 `mod tests`(`with_favorites` 왕복)
  - **Edge Cases**:
    - 중복 경로 → `add`가 무시한다
    - 존재하지 않는 폴더 → **확인하지 않는다**(시작할 때 전건 확인은 네트워크 드라이브에서 창을 늦춘다 — Deferred)
    - 대소문자만 다른 경로 → 이번에는 문자열 그대로 비교한다(Out of Scope)
    - UTF-8로 표현되지 않는 경로 → 세션에는 문자열로 담으므로(D9) 저장이 통째로 무산되는 일이 없다. 그런 경로는 즐겨찾기에서 빠질 수 있고, 그것이 세션 전체를 잃는 것보다 낫다
  - **Halt Forecast**:
    - (i) 스키마 버전을 올릴지 → D1·전제 1에서 "버전 유지 + serde default"로 확정
    - (ii-a) `Session` 직렬화 계약 확장(비파괴) → `## 사전 승인 항목`
  - **Depends on**: -

- [x] T2. 트리 위쪽 즐겨찾기 목록과 구분선 + 전달 배선
  - **Type**: D
  - **Design**: ① `src/ui/tree.rs`의 `show`가 **로컬 소스일 때만** 즐겨찾기 줄들과 구분선을 드라이브 뿌리보다 먼저 그린다 ② 신규 심볼은 비공개 `show_favorites`(줄 그리기)와 `FolderTreeView::show`의 인자 `favorites: &[PathBuf]` ③ 의존은 단방향으로 내려간다 — `ui::app`(소유) → `splitter::show_layout` → `PanelState::show` → `FolderTreeView::show`. 트리는 저장소 타입을 모르고 `&[PathBuf]`만 본다 ④ 비추상화 선언: 즐겨찾기 전용 위젯·트레이트를 만들지 않고(기존 `selectable_label` 한 줄과 `ui.separator()` 그대로), 즐겨찾기 항목에 펼침 화살표를 두지 않는다(사용자 결정 — 평면 목록)
  - **Acceptance**:
    - Given 즐겨찾기 2건 + 로컬 패널이 트리를 켠 상태, When 한 프레임 그림, Then 두 이름이 드라이브 뿌리(`C:\`)보다 **위**에 그려지고 그 사이에 구분선이 하나 있다
    - Given 즐겨찾기 0건, When 그림, Then 즐겨찾기 줄도 구분선도 그려지지 않는다(지금 화면 그대로)
    - Given 원격 패널의 트리, When 그림, Then 즐겨찾기 줄이 하나도 그려지지 않는다
    - Given 즐겨찾기 줄, When 좌클릭, Then `TreeOutcome.chosen`이 `TreeChoice::Local(그 경로)`로 올라와 활성 탭이 그 폴더로 이동한다
    - 표시는 **폴더 이름만**이다(드라이브 뿌리는 `C:\`처럼 경로 자체가 이름이다) — 자동 시험은 "그려진 글이 이름뿐"까지 판정한다
    - 마우스를 올리면 전체 경로가 툴팁으로 뜬다 — **툴팁은 hover가 있어야 그려지므로 사용자 수동 확인 항목**이다(`Verification Strategy`)
    - `cargo test` 통과, clippy 경고 0
  - **Files**:
    - 주: `src/ui/tree.rs` · `src/ui/panel.rs` · `src/ui/splitter.rs` · `src/ui/app.rs`
    - 동반: 없음 — **T2에는 신규 화면 문구가 없다**(항목은 폴더 이름, 빈 목록이면 안내도 없다 — Q2·Q4). 문구는 T3의 메뉴에서만 는다
    - 테스트: `src/ui/tree.rs`의 `mod tests` · `src/ui/panel/tests.rs` · `src/ui/splitter.rs`의 `mod tests`
  - **Edge Cases**:
    - 이름이 트리 폭(200px)보다 길다 → 기존 트리 노드와 같이 잘려 그려진다
    - 같은 폴더가 즐겨찾기와 드라이브 트리 양쪽에 보인다 → 선택 강조가 둘 다 켜진다(같은 곳을 가리키므로 의도된 동작)
    - 즐겨찾기가 많아 트리를 넘긴다 → 기존 `ScrollArea` 안이라 함께 스크롤된다
    - 드라이브 루트(`C:\`)가 즐겨찾기다 → `file_name()`이 없어 경로 문자열을 그대로 이름으로 쓴다
  - **Halt Forecast**:
    - (i) 항목 모양·정렬·빈 상태 → 사용자 결정(요구 이해)으로 확정
    - (ii-a) `FolderTreeView::show`·`PanelState::show`·`splitter::show_layout` 시그니처 변경 → `## 사전 승인 항목`
  - **Depends on**: T1

- [x] T3. 트리 우클릭 컨텍스트 메뉴(`즐겨찾기`·`해제`)와 적용
  - **Type**: D
  - **Design**: ① 메뉴는 `src/ui/tree.rs`가 트리의 `ScrollArea` **밖** `egui::Area(Foreground)`에 그린다(원격 메뉴와 같은 방식). 메뉴는 **로컬 분기에서만** 열리고 그려지며, **원격 분기에 들어서면 `menu_at`을 비운다**(상태 누수 방지 — Edge Cases) ② 신규 심볼 — 비공개 `menu_at: Option<(egui::Pos2, MenuTarget)>` — `MenuTarget`은 `ui::tree`의 비공개 enum `{ Node(PathBuf), Favorite(PathBuf) }`이고, 이 둘이 곧 메뉴 두 종류(`즐겨찾기` / `해제`)를 가른다, `pub fn close_menu(&mut self)`(트리를 감출 때 패널이 부른다), `TreeOutcome.favorite: Option<FavoriteAction>`. **`FavoriteAction` 자체는 T1이 `app::favorites`에 둔다**(D8) — 트리는 그것을 만들어 올리기만 한다 ③ 값은 트리 → `PanelOutcome.favorite` → `LayoutOutcome.favorite` → `ui::app`이 **`self.favorites.apply(action)` 한 줄**로 반영하고 다음 저장 때 세션에 실린다(D3·D8). `LayoutOutcome` 단은 컴파일러가 잡지 않으므로(전제 4-b) 그 한 줄의 존재는 수동 확인 항목이다 ④ 비추상화 선언: `remote_menu.rs`와 공통 메뉴 부품으로 묶지 않는다(대상·항목이 다르다 — 그 모듈 주석의 판단과 같다). 다만 위치 보정 `clamp_menu_pos`는 공용으로 올려 **같은 함수**를 쓴다
  - **Acceptance**:
    - Given 로컬 트리 노드, When 우클릭, Then 그 자리에 `즐겨찾기` 한 줄짜리 메뉴가 뜬다
    - Given 이미 즐겨찾기로 등록된 폴더, When 우클릭, Then `즐겨찾기` 줄이 **비활성**으로 보이고 눌러도 아무 일이 없다
    - Given 미등록 폴더의 메뉴, When `즐겨찾기` 클릭, Then `FavoriteAction::Add`가 올라오고 그 폴더가 즐겨찾기 목록 **맨 아래**에 나타난다
    - Given 즐겨찾기 줄, When 우클릭, Then 메뉴가 `해제` 한 줄이고 클릭하면 목록에서 사라진다
    - Given 원격 트리 노드, When 우클릭, Then 메뉴가 뜨지 않는다
    - Given 로컬 트리에서 메뉴를 연 상태, When 활성 탭을 원격으로 바꿨다가 로컬로 되돌림, Then 메뉴가 떠 있지 않다
    - Given 메뉴가 열린 상태, When 바깥 클릭 또는 Esc, Then 메뉴가 닫힌다
    - Given 메뉴가 열린 상태, When 트리 토글을 껐다가 다시 켬, Then 메뉴가 떠 있지 않다(`close_menu`가 지운다)
    - Given 즐겨찾기를 등록한 뒤 `session::with_favorites`를 거쳐 저장·복원, Then 목록이 그대로 남는다(T1의 seam을 그대로 쓴다 — `collect_session`은 시험 대상이 아니다)
    - `cargo test` 통과, clippy 경고 0
  - **Files**:
    - 주: `src/ui/tree.rs` · `src/ui/panel.rs`(메뉴 상태 정리 호출 포함) · `src/ui/splitter.rs` · `src/ui/app.rs`
    - 동반: `src/i18n/mod.rs`(`즐겨찾기`·`해제` 문구) · `src/ui/menu.rs`(`clamp_menu_pos`의 새 위치와 그 시험 — D6) · `src/ui/panel/tests.rs`(그 시험 함수를 덜어 내고 `panel.rs`는 `use crate::ui::menu::clamp_menu_pos`로 받는다)
    - 테스트: `src/ui/tree.rs`의 `mod tests` · `src/ui/panel/tests.rs` · `src/ui/splitter.rs`의 `mod tests`
  - **Edge Cases**:
    - 화면 가장자리에서 우클릭 → `clamp_menu_pos`로 안으로 당긴다
    - **메뉴가 열린 채 트리 토글을 끈다 → 트리 코드가 아예 돌지 않아 스스로 닫지 못한다**(`panel.rs:1139`의 `if self.tree_visible`). 그래서 `PanelState`가 트리를 감출 때 `FolderTreeView::close_menu()`를 부른다 — 이것이 없으면 트리를 다시 켤 때 옛 메뉴가 되살아난다
    - 메뉴가 열린 채 패널을 바꾼다 → 그 패널의 트리는 계속 그려지므로 바깥 클릭 판정이 닫는다
    - **메뉴가 열린 채 활성 탭이 원격으로 바뀐다** → `FolderTreeView::show`의 **원격 분기에 들어서는 순간 `menu_at`을 비운다**. 로컬 분기에서만 다루면 원격 탭으로 갔다 돌아올 때 옛 메뉴가 되살아나고, 소스를 무시하고 그리면 원격 탭 위에 로컬 메뉴가 뜬다(acceptance ⑤ 위반). 기존 `show_remote_menu`가 원격 경로가 없을 때 `remote_menu_at`을 지우는 것과 **같은 규칙**이다(`src/ui/panel.rs:1465-1469`). 탭 전환은 트리 토글보다 훨씬 잦다
    - 같은 프레임에 두 패널에서 우클릭 → 트리는 패널마다 독립이라 각자 열리되, 위로 올라가는 액션은 `PanelOutcome` 병합에서 first-wins다(기존 필드와 같은 규칙)
    - 즐겨찾기 줄에서 여는 메뉴에는 `즐겨찾기`(추가)를 두지 않는다 — 이미 등록된 것이 자명하다
  - **Halt Forecast**:
    - (i) 메뉴 문구·비활성 규칙 → D4·요구 이해에서 확정
    - (ii-a) `TreeOutcome`·`PanelOutcome`·`LayoutOutcome` 구조체 확장, `clamp_menu_pos` 이동 → `## 사전 승인 항목`
  - **Depends on**: T2

- [x] T4. PRD·README 갱신과 위키 큐
  - **Type**: A
  - **Acceptance**:
    - `docs/prd.md`의 Out of Scope에서 `즐겨찾기` 줄이 사라지고, FR 표에 **FR-56(폴더 트리 즐겨찾기, Should)** 한 행이 생기며, `## 성공 기준`의 Should 목록과 `## 결정 이력`에 2026-08-16 한 줄이 함께 갱신된다
    - `README.md`의 폴더 트리 항목에 즐겨찾기 등록·해제·전역 공유·원격 제외가 적힌다
    - 위키는 이 세션이 직접 고치지 않고 `[PROJECT-FACT]` 큐 1줄로 남긴다(`feat-navigation.md`의 트리 서술 보강 필요)
    - 역대조: PRD·README에 적은 각 문장이 T1~T3의 실제 동작과 1:1로 맞는지 표로 대조한다
  - **Files**:
    - 주: `docs/prd.md` · `README.md`
    - 동반: 위키 vault 루트 `pending.md`(큐 1줄)
  - **Edge Cases**:
    - FR 번호 충돌 → 현재 최대가 FR-55이므로 FR-56을 쓴다(작성 시 재확인)
  - **Halt Forecast**:
    - (ii-a) PRD 문면 변경 → `## 사전 승인 항목`(사용자가 2026-08-16 승인)
  - **Depends on**: T3

## 사전 승인 항목 (일괄 승인 대상)
- T1 — `Session`에 `favorites` 필드 추가(비파괴 직렬화 확장, 스키마 버전 유지)
- T1 — `ui::session`에 `with_favorites` 추가(신규 공개 함수 — D7의 시험 seam)
- T2·T3 — `FolderTreeView::show` · `PanelState::show` · `splitter::show_layout` 시그니처에 인자 추가, `TreeOutcome`·`PanelOutcome`·**`LayoutOutcome`**에 필드 추가(계획된 공개 API 변경)
- T3 — `clamp_menu_pos`를 `panel.rs`에서 `src/ui/menu.rs`로 이동하고 `pub(crate)`로 연다(D6 — 구조 변경, 사용처 2곳 + 시험 3곳)
- T4 — PRD 문면 변경(Out of Scope 한 줄 제거 + FR-56 신설 + 결정 이력) — 사용자가 2026-08-16 승인

## 불가피한 Halt (위임 불가)
- commit 이후의 push·태그·릴리즈 (외부·비가역)
- plan에 없던 구조 결정이 필요해지는 경우(예: 즐겨찾기를 워크스페이스별로 나눠야 한다는 요구 변경)

## Verification Strategy
- 빌드: `cargo build`
- 단위·통합 시험: `cargo test`
- 린트: `cargo clippy --all-targets -- -D warnings`
- 형식: `cargo fmt --check`
- 수동 확인(사용자): 즐겨찾기 목록의 모양·구분선 위치, **항목 툴팁의 전체 경로**, 우클릭 메뉴의 자리와 비활성 표시, **등록·해제가 화면에 곧바로 반영되는지**(`LayoutOutcome` 배선 — 전제 4-b), 재시작 후 목록 유지

## 리뷰 이력 (plan-reviewer 3라운드 — 재호출 상한 소진)
| 라운드 | 판정 | 처리 |
|---|---|---|
| 1 | BLOCKER 1 · MAJOR 2 · MINOR 4 | 전건 반영 — D6(`clamp_menu_pos` 이동처) · D7(세션에 싣는 자리) · `close_menu()` 배선 |
| 2 | BLOCKER 1 · MAJOR 1 · MINOR 3 | 전건 반영 — D7 보강(`with_favorites` 순수 seam) · D8(액션·적용을 `app::favorites`로) · D9(`Vec<String>`) · 4-A에 `LayoutOutcome` |
| 3 | BLOCKER 0 · MAJOR 1 · MINOR 6 | **재호출 상한을 소진해 메인이 직접 대조·처리**. MAJOR(활성 탭이 원격으로 바뀔 때 메뉴 상태 누수)는 근거 코드(`src/ui/panel.rs:1465-1469`의 같은 규칙 선례)를 확인하고 수용 — T3 Design·acceptance·Edge Case에 반영. MINOR 6건도 근거를 확인해 전건 수용(전제 번호 오인용·4-C 누락·사전 승인 목록·경로 변환 자리·`MenuTarget` 정의·`TreeOutcome` 홉) |

- **동일 지적 잔존 0** — 매 라운드 지적이 전부 신규였다(리뷰어도 RECURRING 없음으로 판정). 라운드를 더 열지 않은 이유는 스킬 규약의 상한(재호출 2회)이며, 그 구간의 수정이 새 결함을 만드는 사례가 관측됐기 때문이다
- 3라운드 지적은 **기각 0 · 수용 7**(MAJOR 1 + MINOR 6)이고, 각 근거를 코드에서 직접 확인한 뒤 반영했다

## 문서 역대조 (T4)

PRD FR-56·README에 적은 각 서술이 실제 구현과 1:1로 맞는지 대조했다 — 누락·잔존·변형 0건.

| # | 문서 서술 | 구현 지점 | 판정 |
|---|---|---|---|
| 1 | 로컬 트리 맨 위에 더한 차례대로 보인다 | `tree.rs` `show`의 Local 분기가 `show_favorites`를 드라이브 뿌리보다 먼저 부르고, 순서는 `FavoriteStore`가 지킨다(`add`는 push) | ✅ |
| 2 | 그 아래를 가로 구분선으로 가른다 | `tree.rs` `show_favorites` 끝의 `ui.separator()` | ✅ |
| 3 | 하나도 없으면 목록도 구분선도 그리지 않는다 | `show_favorites` 첫 줄 `if favorites.is_empty() { return; }` (구분선이 그 뒤라 함께 생략) | ✅ |
| 4 | 트리 항목 우클릭 메뉴의 `즐겨찾기`로 담는다 | `tree.rs` `open_node_menu` → `show_menu`의 `MenuTarget::Node` 분기 | ✅ |
| 5 | 이미 담긴 폴더면 그 줄이 비활성 | `show_menu`의 `let enabled = !favorites.iter().any(...)` | ✅ |
| 6 | 즐겨찾기 줄 우클릭 메뉴의 `해제`로 뺀다 | `show_menu`의 `MenuTarget::Favorite` 분기 | ✅ |
| 7 | 이름만 보이고 전체 경로는 툴팁 | `show_favorites`의 `display_name(path)` + `.on_hover_text(path.to_string_lossy())` | ✅ |
| 8 | 누르면 활성 탭이 그 폴더로 간다 | `show_favorites`의 `response.clicked() → select` → `panel.rs`의 `TreeChoice::Local => navigate` | ✅ |
| 9 | 목록은 앱에 하나뿐이라 모든 워크스페이스·패널·탭이 같은 것을 본다 | `ui::app`의 `favorites: FavoriteStore` 한 필드를 `show_layout`이 모든 패널에 내려보낸다 | ✅ |
| 10 | 세션 파일에 담겨 재시작해도 남는다 | `Session.favorites` + `collect_session`의 `with_favorites` + 복원부의 `FavoriteStore::from_paths` | ✅ |
| 11 | 원격 트리는 대상이 아니다(메뉴도 뜨지 않는다) | `show`의 Remote 분기가 `show_favorites`를 부르지 않고 `menu_at`을 비운다 | ✅ |

## Phase Ledger

Phase F 통과 (HEAD 838d31e + F-7 지적 반영분)
Phase G 통과 (커버 대상 FR 100% — 아래 충족표)

### Phase G — PRD 요구 재대조

| FR | 우선순위 | 요건 | 충족 근거 | 판정 |
|---|---|---|---|---|
| FR-56 | Should | 로컬 트리 맨 위에 더한 차례대로 | `tree.rs` `show`의 Local 분기가 `show_favorites`를 뿌리보다 먼저 부른다 · `FavoriteStore::add`가 push | ✅ |
| FR-56 | Should | 아래를 가로 구분선으로 가름 | `show_favorites` 끝의 `ui.separator()` · 시험 `즐겨찾기는_드라이브_뿌리보다_위에_구분선과_함께_선다` | ✅ |
| FR-56 | Should | 하나도 없으면 목록·구분선 미표시 | `if favorites.is_empty() { return; }` · 시험 `즐겨찾기가_없으면_구분선도_그리지_않는다` | ✅ |
| FR-56 | Should | 우클릭 메뉴 `즐겨찾기`로 담기 | `open_node_menu` → `MenuTarget::Node` · 시험 `트리_노드를_우클릭하면_즐겨찾기_메뉴가_뜬다` | ✅ |
| FR-56 | Should | 이미 담긴 폴더면 그 줄 비활성 | `let enabled = !favorites.iter().any(..)` · 시험 `이미_담긴_폴더는_즐겨찾기_줄이_비활성이다`(미등록/등록 대조) | ✅ |
| FR-56 | Should | 즐겨찾기 줄 우클릭 `해제` | `MenuTarget::Favorite` · 시험 `즐겨찾기_줄의_메뉴는_해제다` | ✅ |
| FR-56 | Should | 이름만 표시, 경로는 툴팁 | `display_name` + `on_hover_text` · 툴팁은 수동 확인 항목(hover 필요) | ✅ (툴팁 ⏳ HUMAN-VERIFY) |
| FR-56 | Should | 누르면 활성 탭이 그 폴더로 | `select` → `TreeChoice::Local` → `navigate` · 시험 `즐겨찾기를_누르면_그_폴더로_옮겨간다`(클릭 주입) | ✅ |
| FR-56 | Should | 목록은 앱 하나, 모든 탭 공통 | `ExplorerApp.favorites` 한 필드를 `show_layout`이 모든 패널에 전달 | ✅ |
| FR-56 | Should | 세션에 담겨 재시작해도 남음 | `Session.favorites` + `with_favorites` + `from_paths` · 시험 `즐겨찾기가_세션에_실려_왕복한다` | ✅ (재시작 실물은 ⏳ HUMAN-VERIFY) |
| FR-56 | Should | 원격 트리는 대상 아님(메뉴도 없음) | Remote 분기가 `show_favorites`를 안 부르고 `menu_at`을 비운다 · 시험 2건 | ✅ |
| FR-9 | Should | 폴더 트리 토글·지연 확장(기존) | 상단 구역만 더했고 드라이브 열거·지연 확장·토글 동작 무변경 · 기존 시험 전건 통과 | ✅ 보존 |

- **커버 대상 FR 충족률 100%** (FR-56 11개 요건 + FR-9 보존)
- ⏳ HUMAN-VERIFY 2건(툴팁 표시·재시작 후 목록 유지)은 기계 검증이 불가한 항목이라 사용자 확인 대상이다 — ✅로 둔갑시키지 않는다

## Retry Ledger

## Progress Log
- T1-T2 완료 (커밋 d8e98e9, 474770c): `app::favorites`에 저장소·액션·적용 규칙을 두고 세션에 `favorites`(문자열 목록)를 더했다. 트리 맨 위에 즐겨찾기 줄과 구분선을 그리고 앱→splitter→panel→tree로 목록을 내려보낸다.
  - 결정: `collect_session`이 스프레드라 컴파일러가 못 잡는 자리를 순수 함수 `with_favorites`로 떼어 시험이 직접 부르게 했다(D7 실행).
  - 리뷰: T1 spec/quality 각 MINOR 1(큐 단언·주석 수치 — 반영). T2 quality OK(SUGGEST 1 등록), spec MAJOR 1 — 클릭 경로가 시험에 안 걸린다는 지적을 받아 **클릭 이벤트 주입 시험으로 교체**했고, 그 시험이 죽어 있지 않음을 클릭 처리 임시 제거로 확인했다.
  - 관측: `remote::connection`의 시간 마감(2초) 의존 시험 1건이 전체 실행에서 간헐 실패한다(단독 통과 — 이번 변경과 무관, 대장의 기존 항목과 같은 성질).
- T3-T4 완료 (커밋 55b6166, 838d31e): 트리 우클릭 메뉴로 담기·빼기를 붙이고(`clamp_menu_pos`를 `ui::menu`로 이관), PRD FR-56 신설·README·위키 큐까지 문서를 맞췄다.
  - 결정: 메뉴 상태(`menu_at`)는 **네 갈래로 정리**한다 — 고름·바깥 클릭/Esc·트리 감춤(`close_menu`)·원격 분기 진입. 앞 둘만으로는 트리를 끄거나 원격 탭으로 갔을 때 메뉴가 되살아난다.
  - 리뷰: T3 spec MAJOR 1(바깥클릭/Esc 시험 누락)·MINOR 2 → 전건 반영 후 재리뷰 OK. quality OK(SUGGEST 1 등록).
  - F-7(전체 완료 검증) MAJOR 1 — plan 4-C가 약속한 **splitter 병합 시험**이 실제로는 빠져 있었다. 그 자리가 컴파일러가 못 잡는 홉이라 시험을 채우고(임시 제거로 죽지 않음도 확인), 4-C의 시험 위치 기록도 실제와 맞췄다.

## Next Steps
- 권장 다음 액션: 사용자 화면 확인(툴팁·메뉴 자리·비활성 표시·등록/해제 반영·재시작 유지) 후 `master` 병합·push 승인
- Suggested skills: 공식 `/code-review`
- 위키 갱신: 이번 회차 사실 2건·결정 1건을 vault `pending.md`에 큐로 남겼다(`[PROJECT-FACT]`·`[DECISION]`) — 반영은 위키 세션에서

## Open Questions
- [x] Q1: PRD Out of Scope의 `즐겨찾기` 처리 → **PRD 갱신 후 구현**(FR-56 Should 신설)
- [x] Q2: 즐겨찾기 항목 모양 → **이름만 · 평면 목록**(툴팁에 전체 경로)
- [x] Q3: 나열 순서 → **추가한 순서**
- [x] Q4: 즐겨찾기가 없을 때 → **아무것도 그리지 않음**(구분선도 없음)
