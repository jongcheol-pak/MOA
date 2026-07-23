# Plan: 멀티 분할 파일 탐색기 v1 — part2 (부가: 트리·셸 메뉴·감시·세션·성능)

**PRD**: docs/prd.md
**이전 plan**: docs/plans/2026-07-23-file-explorer-part1.md

## 이전 part 핸드오프
- 함정: windows-rs 0.62는 `Error::from_win32`→`from_thread` 개명, `SHELLEXECUTEINFOW`는 `Win32_System_Registry` feature 필요(주석 있음), 리스트 카운트는 반드시 RefCell 차용 해제 후 `file_list::apply_item_count`로만 (재진입 표시 누락 — panel.rs 계약 주석 참조).
- 기각된 접근: FileList 내부에서 LVM_SETITEMCOUNT 직접 호출(차용 중 재진입 별칭) — free fn 분리로 확정, 되돌리지 말 것.
- 검증 지름길: `cargo build && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check` 체인. 미소비 심볼은 `#[cfg_attr(not(test), expect(dead_code))]` 자기 정리 패턴 — `tabs.rs::len`이 T4(세션) 소비 대기 중(사용 시 expect 제거 필요).
- 상태 접근: 창 상태는 `state_of()`(try_borrow_mut) 경유만 — 패널·메인 창 RefCell은 별개라 SendMessage 동기 질의 안전 (window.rs send_to 참조).

## 요구 이해
- **원문 요청**: "파일 탐색기를 구현하려고 하는데 이미지처럼 멀티로 탭을 분할해서 표시하는 기능을 중점으로 개발하려고 하는데 계획을 세워줘. 메모리, 성능에 좋은 개발 언어도 같이. Windows 11 이상에서만 사용할 예정"
- **이해한 요구**: part1에서 완성한 코어(분할·탭·목록·네비게이션) 위에, 셸 컨텍스트 메뉴(Must FR-8)·폴더 트리·변경 감시·세션 복원(Should)·성능 실측(NFR-1~3)을 얹어 PRD v1을 완성한다.
- **포함하지 않는 것으로 이해**: 파일 작업 UI 자체 구현 등 PRD Out of Scope 전체. FR-13·14(Could)는 이번 v1에서 Deferred.

## Goal
셸 컨텍스트 메뉴·폴더 트리·자동 새로고침·세션 복원을 추가하고 성능 목표(NFR-1~3)를 실측 통과시켜 v1을 완성한다 (이 plan 범위: part1 이후 잔여 전부).

**전체 목표**: PRD(docs/prd.md)의 Must 8개 + Should 4개를 충족하는 초경량 멀티 분할 파일 탐색기 v1 완성.

## PRD Coverage
| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-1 (자유 분할) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-2 (스플리터·패널 닫기) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-3 (패널별 탭) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-4 (파일 목록·정렬) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-5 (시스템 아이콘) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-6 (주소창·네비게이션) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-7 (더블클릭 진입·실행) | Must | (part1 기구현) | ✅ 이전 part 기구현 |
| FR-8 (셸 컨텍스트 메뉴) | Must | T2 | ✅ 커버 |
| FR-9 (폴더 트리) | Should | T1 | ✅ 커버 |
| FR-10 (변경 감시) | Should | T3 | ✅ 커버 |
| FR-11 (세션 복원) | Should | T4 | ✅ 커버 |
| FR-12 (단축키 — F5 잔여분) | Should | T3 | ✅ 커버 (나머지 키는 part1 기구현) |
| FR-13 (숨김 파일 토글) | Could | (이번 제외 — Deferred) | ⏭️ Deferred |
| FR-14 (분할 프리셋) | Could | (이번 제외 — Deferred) | ⏭️ Deferred |
| NFR-1~3 (성능) | — | T5 (실측·튜닝) | ✅ 커버 |
| NFR-4~6 (DPI·긴 경로·한국어) | — | (part1 기구현 — part1 coverage 표 참조) | ✅ 이전 part 기구현 |
| NFR-7 (%APPDATA% 저장) | — | T4 (D15) | ✅ 커버 |

## Out of Scope
- PRD Out of Scope 전체 (파일 작업 UI·드래그앤드롭·검색·다크 모드·즐겨찾기·가상 폴더·다국어)

## Deferred / Follow-up
- FR-13 숨김·시스템 파일 표시 토글 (Could — v1 이후 여유 시)
- FR-14 분할 프리셋 버튼 (Could — v1 이후 여유 시)
- 트리→목록 외 역방향 동기화(목록 이동 시 트리 자동 펼침·선택)는 v1 단방향으로 한정 (D14) — 필요 체감 시 후속
- T2 리뷰 MINOR m1: `shell_menu::items_menu`의 `pidls[0]`이 "items 비지 않음" 암묵 계약 의존 — doc 주석/debug_assert 명시 검토 (현재 도달 불가 경로)

## Investigation Log
- part1과 공통 (동일 세션 작성): 빈 프로젝트 확인·Rust 1.95 stable msvc 실측·vault 미설정·PRD/AGENTS.md 승인 — 상세는 part1 Investigation Log 참조
- 이 plan의 전제: part1 완료 상태(패널·탭·목록·navigate 단일 진입점 존재). part1 미완료 상태로 이 plan을 실행하지 않는다 (**이전 plan** 포인터로 식별)

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| IContextMenu 3단 호출(Query/Track/Invoke)과 서브메뉴(보내기 등) 메시지 포워딩 누락 | 일부 메뉴 항목 미동작 | `HandleMenuMsg`(IContextMenu2/3) 포워딩 구현 + 다중 항목으로 수동 검증 (T2 Edge) |
| ReadDirectoryChangesW 버퍼 오버플로(대량 변경 시 통지 유실) | 목록 불일치 | 오버플로 오류 코드 수신 시 전체 재열거로 폴백 (T3 Edge) |
| 세션 파일 손상·구버전 스키마 | 시작 실패 | version 필드 + 파싱 실패 시 기본 레이아웃 폴백 (T4 Edge, D15) |
| 성능 실측 미달(NFR-1~3) | 목표 미충족 | T5에서 측정→병목 수정→재측정 루프. 아이콘·열거는 part1 설계(캐시·가상화)가 1차 방어 |
| 트리 대량 하위 폴더(수천) 확장 지연 | UI 멈춤 | 지연 확장 + 확장 시에만 1단계 열거, 워커 스레드 사용 (T1 Design) |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `panel::Panel` (part1 산출) | `src/panel/panel.rs` | T1(트리 영역 추가)·T3(감시 연동)이 내부 확장 — navigate 진입점 시그니처는 불변 |
| `app::window::MainWindow` (part1 산출) | `src/app/window.rs` | T4(세션 저장/복원 훅: WM_CLOSE·시작 시), T1(메뉴 토글 배선) |
| `fs::enumerate` (part1 산출) | `src/fs/enumerate.rs` | T1(트리 1단계 열거 재사용)·T3(재열거 호출) — 시그니처 불변, 호출자 추가만 |

> part1 미구현 시점 작성이므로 위 표는 part1 plan의 Design 명세 기준. implement-task 시작 시 실코드와 대조해 어긋나면 Progress Log에 기록 후 조정 (구조 변경이 필요해지면 돌발 결정 → Halt).

### 4-B. 계약·직렬화 변경
- T4가 세션 스키마(settings.json)를 신규 도입 — version 필드 포함(D15). 기존 데이터 없음(신규 파일)이라 마이그레이션 불필요, 이후 버전부터 version으로 호환 처리

### 4-C. 테스트 파일
- part1의 기존 단위테스트(layout·history·정렬·탭) 통과 유지 확인을 각 task 검증에 포함
- 신규: T3 감시 통합테스트(`tests/watcher.rs`), T4 직렬화 왕복 단위테스트

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `FolderTree` | part1에 트리 없음 (part1 plan 전수 확인) | 신규. 1단계 열거는 `fs::enumerate` 재사용 |
| `shell_menu` 모듈 | 없음 | 신규 (COM IContextMenu 래퍼 — 대체 불가) |
| `fs::watcher` | 없음 | 신규. 재열거는 기존 navigate/enumerate 경로 재사용 |
| `app::settings` (세션 직렬화) | 없음 | 신규. serde 사용(part1 D1 승인 의존성 — 신규 의존성 없음) |
| 성능 측정 스크립트 | 없음 | 신규 — 프로젝트 밖 스크래치 폴더에 생성(임시 파일 규칙), 저장 필요 시만 `scripts/`로 |

### Verified by
- part1 plan Files 목록 전수 대조 → 트리·감시·세션·셸 메뉴 모듈 없음 확인 (part1 산출 예정 파일: window/layout/layout_host/menu/panel/file_list/address_bar/history/tabs/enumerate/icons)

## Decisions
> part1 Decisions(D1~D12)는 이 plan에도 그대로 적용된다 (의존성·UI 방식·스레딩·에러 표시·한국어 문구 등). 아래는 part2 고유 결정.

### D13. 셸 컨텍스트 메뉴 구현 (FR-8)
- **Options**: A) `IShellFolder::GetUIObjectOf`→`IContextMenu` 표준 절차(Query/TrackPopupMenu/Invoke + IContextMenu2/3 메시지 포워딩) / B) 자체 메뉴로 주요 동작만 흉내
- **Chosen**: A
- **Rationale**: v1의 파일 작업 위임 전략 자체가 이 요구의 근거 — 복사/삭제/속성 등 전부를 셸이 제공. B는 근본 해결이 아님(기능 재구현 필요).
- **Source**: PRD FR-8 + Out of Scope(파일 작업 위임)

### D14. 트리 동작 (FR-9)
- **Options**: A) 패널별 트리, 지연 확장(TVN_ITEMEXPANDING 시 1단계만 열거), 트리 선택→활성 탭 이동(단방향) / B) 목록 이동 시 트리 자동 펼침(양방향 동기화)까지
- **Chosen**: A
- **Rationale**: 양방향 동기화는 깊은 경로에서 연쇄 확장(대량 열거)을 유발해 NFR 위협 + 구현 복잡도 대비 v1 가치 낮음. 단방향이 Q-Dir 기본 체감과 동일. B는 Deferred에 기록.
- **Source**: PRD FR-9 + NFR-3

### D15. 세션 스키마·저장 시점 (FR-11)
- **Options**: A) `{version:1, window:{x,y,w,h,maximized}, layout:<트리 재귀>, panels:[{tabs:[{path}], active_tab}]}` — WM_CLOSE 시 1회 저장, 시작 시 로드 / B) 변경 시마다 즉시 저장
- **Chosen**: A
- **Rationale**: v1은 단일 인스턴스 가정이 없어도 종료 시 저장이 단순·충분. 히스토리는 저장하지 않음(경로만 — 파일 크기·복잡도 절약, 재시작 후 히스토리 초기화는 관례적 체감). B는 I/O 잦음.
- **Source**: PRD FR-11 + NFR-7 (%APPDATA% 위치)
- **주의**: 저장 경로의 실제 값·사용자명은 문서에 기록하지 않음 (환경변수 `%APPDATA%` 표기만)

### D16. 감시 전략 (FR-10)
- **Options**: A) 표시 중인 폴더(활성 탭)당 워커 1개, `ReadDirectoryChangesW` 동기 호출 스레드 + 300ms 디바운스 후 재열거 통지 / B) 변경 항목 단위로 목록 부분 갱신
- **Chosen**: A
- **Rationale**: 부분 갱신(B)은 정렬 위치 재계산·가상 리스트 인덱스 관리가 복잡하고 이득이 작음. 재열거는 백그라운드라 UI 무정지(D5)이고 디바운스로 폭주 억제. 폴더 이탈·탭 전환 시 워커 취소.
- **Source**: PRD FR-10 + D5(스레딩)

### D17. 성능 측정 방법 (NFR-1~3)
- **Options**: A) PowerShell 측정 스크립트(시작 시간: 프로세스 생성→창 표시, 메모리: WorkingSet64) + 10만 파일 테스트 폴더 생성 스크립트(스크래치 폴더) / B) 전용 프로파일러 도입
- **Chosen**: A
- **Rationale**: NFR 판정에는 재현 가능한 수치 측정이면 충분. 스크립트는 시스템 임시 폴더에 생성(프로젝트 오염 방지, CLAUDE.md 임시 파일 규칙). 미달 시 병목 분석은 그때 도구 선택.
- **Source**: PRD NFR-1~3 + 성공 기준

## Tasks

- [x] T1. 폴더 트리 — 패널별 토글 (FR-9)
  - **Type**: C
  - **Design**: ① 배치: `src/panel/folder_tree.rs` ② 신규 심볼: `FolderTree` — `SysTreeView32` 생성, 드라이브 루트 나열, `TVN_ITEMEXPANDING` 지연 확장(하위 폴더 1단계만, 워커 열거 재사용), 선택 시 `Panel::navigate` 호출 책임 ③ 의존: Panel이 소유·배치(트리 표시 시 좌측 고정폭 200px), fs::enumerate 재사용 ④ 비추상화: 목록→트리 역방향 동기화 없음(D14), 트리 전용 아이콘 처리 없음(폴더 아이콘 IconCache 재사용).
  - **Acceptance**: Given 실행 앱, When 메뉴 "보기 > 폴더 트리"(패널별 토글), Then 트리 표시/숨김이 활성 패널에만 적용되고, 노드 확장은 하위 1단계만 열거, 노드 클릭 시 활성 탭이 해당 경로로 이동 — HUMAN-VERIFY. 빌드·clippy·기존 테스트 0 실패는 기계 검증.
  - **Files**:
    - 주: `src/panel/folder_tree.rs`
    - 동반: `src/panel/panel.rs`(배치·토글 상태), `src/app/menu.rs`(토글 메뉴), `src/app/window.rs`(트리 토글 WM_COMMAND 배선 — 4-A 표와 정합), `src/panel/mod.rs`
  - **Edge Cases**:
    - 접근 거부 폴더 확장 → 확장 실패 무해 처리(하위 없음 표시)
    - 하위 폴더 수천 개 → 지연 확장 + 워커 열거로 UI 무정지
    - 준비 안 된 드라이브(빈 광학 드라이브 등) → 확장 시도 실패 무해 처리
  - **Halt Forecast**:
    - (i) "동기화 범위?" → D14 / "트리 폭?" → Design(고정 200px, 조절은 v2)
  - **Depends on**: - (part1 완료 전제)

- [x] T2. 셸 컨텍스트 메뉴 (FR-8)
  - **Type**: C
  - **Design**: ① 배치: `src/fs/shell_menu.rs` ② 신규 심볼: `show_context_menu(hwnd, paths: &[PathBuf], pt)` — `SHParseDisplayName`→부모 `IShellFolder`→`GetUIObjectOf(IContextMenu)`→`QueryContextMenu`→`TrackPopupMenuEx`→`InvokeCommand` 절차 + `IContextMenu2/3` 소유자 메시지(WM_INITMENUPOPUP 등) 포워딩 책임. unsafe는 이 모듈에 격리 ③ 의존: FileList의 `NM_RCLICK`/`WM_CONTEXTMENU`에서 선택 항목 경로들로 호출 ④ 비추상화: 메뉴 항목 필터링·커스텀 항목 추가 없음(셸 그대로).
  - **Acceptance**: Given 파일 다중 선택, When 우클릭, Then Windows 표준 컨텍스트 메뉴가 표시되고 복사·삭제·속성·보내기(서브메뉴)가 실제 동작 — HUMAN-VERIFY. 항목 0개 선택 우클릭은 폴더 배경 메뉴(새로 만들기 포함) 표시.
  - **Files**:
    - 주: `src/fs/shell_menu.rs`
    - 동반: `src/panel/file_list.rs`(선택 항목 조회), `src/fs/mod.rs`, `src/panel/panel.rs`(WM_CONTEXTMENU·메뉴 메시지 포워딩 배선 — 리스트뷰 알림이 부모 패널 창에 도착하므로 구현 중 추가), `Cargo.toml`(windows crate `Win32_UI_Shell_Common` feature — ITEMIDLIST 정의, 신규 의존성 아님)
  - **Edge Cases**:
    - 메뉴 표시 중 대상 파일 삭제됨 → InvokeCommand 실패 무해 처리(셸 오류 UI 위임)
    - 서브메뉴(보내기 등) → IContextMenu2/3 메시지 포워딩으로 동작 보장
    - 다중 선택에 폴더+파일 혼합 → 그대로 셸에 전달(셸이 공통 메뉴 계산)
  - **Halt Forecast**:
    - (i) "메뉴 구현 방식?" → D13 사전 결정
  - **Depends on**: -

- [x] T3. 변경 감시 자동 새로고침 + F5 (FR-10, FR-12 잔여)
  - **Type**: C
  - **Design**: ① 배치: `src/fs/watcher.rs` ② 신규 심볼: `DirWatcher` — 감시 스레드 시작/중지(활성 탭 경로 변경 시 재시작), `ReadDirectoryChangesW` 수신→300ms 디바운스→`WM_APP_DIR_CHANGED` 통지 책임 ③ 의존: Panel이 활성 탭 경로로 소유·재시작, 통지 수신 시 기존 재열거 경로 호출. F5는 menu.rs 액셀러레이터→동일 재열거 ④ 비추상화: 변경 항목 단위 부분 갱신 없음(D16 — 전체 재열거).
  - **Acceptance**: 통합테스트(`tests/watcher.rs`): Given 임시 폴더 감시 시작, When 파일 생성/삭제, Then 디바운스 후 변경 통지 1회 수신 (HWND 비의존 — 채널 수신으로 검증). Given 실행 앱, When 표시 중 폴더에 외부에서 파일 추가·F5 입력, Then 목록 자동/수동 갱신 — HUMAN-VERIFY.
  - **Files**:
    - 주: `src/fs/watcher.rs`
    - 동반: `src/panel/panel.rs`(수명 관리·통지 배선), `src/app/menu.rs`(F5), `src/fs/mod.rs`, `src/app/window.rs`(F5 WM_COMMAND 배선 — 액셀러레이터는 메인 창 도착), `Cargo.toml`(windows feature: Security·System_IO·System_Threading — RDCW/이벤트, 신규 의존성 아님), `src/lib.rs`(신규)·`src/main.rs`(mod→use — tests/가 내부 모듈을 import하려면 lib 타깃 필요: plan 명시 통합테스트의 기계적 전제)
    - 테스트: `tests/watcher.rs`
  - **Edge Cases**:
    - 버퍼 오버플로 통지(대량 변경) → 전체 재열거 폴백
    - 감시 중 폴더 자체가 삭제됨 → 감시 종료 + 목록에 접근 불가 문구(D6)
    - 네트워크 드라이브 경로 → 감시 시작 실패 시 감시 없이 동작(F5 수동만) — 무해 저하
    - 탭 고속 전환 → 이전 감시 스레드 취소 후 재시작(누수 금지)
  - **Halt Forecast**:
    - (i) "부분 갱신 vs 재열거?" → D16 / "디바운스 값?" → D16(300ms)
  - **Depends on**: -

- [ ] T4. 세션 저장/복원 (FR-11)
  - **Type**: C
  - **Design**: ① 배치: `src/app/settings.rs` ② 신규 심볼: `Session`(serde 구조체 — D15 스키마)·`save_session`/`load_session`(%APPDATA%\FileExplorer\settings.json, 디렉터리 없으면 생성) ③ 의존: MainWindow가 WM_CLOSE에서 수집·저장, 시작 시 로드해 LayoutTree·패널·탭 재구성(없거나 손상 시 기본 1패널 1탭 홈 폴더) ④ 비추상화: 설정 일반화(옵션 시스템) 없음 — 세션 필드만.
  - **Acceptance**: 단위테스트: Session 직렬화→역직렬화 왕복 동일성 + 손상 JSON 입력 시 기본값 반환. Given 분할·탭 구성 후 종료, When 재시작, Then 레이아웃·탭 경로·창 위치 복원 — HUMAN-VERIFY.
  - **Files**:
    - 주: `src/app/settings.rs`
    - 동반: `src/app/window.rs`(저장/복원 훅), `src/app/mod.rs`
    - 테스트: `src/app/settings.rs` `#[cfg(test)]`
  - **Edge Cases**:
    - settings.json 없음(최초 실행)/손상/미래 version → 기본 레이아웃 폴백, 오류 무해 처리
    - 저장된 탭 경로가 삭제됨 → 해당 탭은 홈 폴더로 대체(탭 수 유지)
    - 창 위치가 분리된 모니터 밖 → 주 모니터로 클램프
    - %APPDATA% 쓰기 실패(디스크 풀 등) → 저장 생략하고 정상 종료(다음 실행은 이전/기본값)
  - **Halt Forecast**:
    - (i) "스키마·저장 시점?" → D15 사전 결정
  - **Depends on**: - (레이아웃 직렬화는 part1 T2 산출 LayoutTree 사용)

- [ ] T5. 성능 실측·튜닝 (NFR-1~3)
  - **Type**: C
  - **Design**: ① 배치: 측정·데이터 생성 스크립트는 시스템 스크래치 폴더(임시 파일 규칙 — 프로젝트 미오염). 코드 수정은 병목 발견 시 해당 모듈 국소 수정 ② 신규 심볼: 없음 예정(측정 결과에 따른 국소 수정만 — 새 구조 필요 시 돌발 결정으로 Halt) ③ 의존: release 빌드 대상 측정 ④ 비추상화: 벤치마크 프레임워크(criterion) 도입 없음.
  - **Acceptance**: `cargo build --release` 산출물로 — NFR-1: 프로세스 시작→창 표시 1초 미만, NFR-2: 패널 2개 유휴 WorkingSet 50MB 미만, NFR-3: 10만 파일 폴더 진입 시 UI 응답 유지(열거 중 입력 가능) — 각각 측정 수치를 완료 보고에 기록. 미달 시 수정→재측정 (동일 이슈 3회 실패 시 중단·보고).
  - **Files**:
    - 주: (측정 스크립트 — 스크래치 폴더, 산출물 아님)
    - 동반: 병목 수정 시 해당 모듈 (범위: 기존 파일 국소 수정만)
  - **Edge Cases**:
    - 콜드/웜 스타트 편차 → 3회 측정 중앙값 사용
    - 백신 실시간 검사 간섭 → 측정 편차로 기록(결과 해석에 명시)
    - 10만 파일 생성 스크립트 → 스크래치 폴더에 생성, 측정 후 삭제
  - **Halt Forecast**:
    - (i) "측정 방법?" → D17 사전 결정
    - (ii-b) 측정 미달로 구조 변경(모듈 재설계·의존성 추가)이 필요해지는 경우 → plan에 없는 돌발 결정, Halt 후 사용자 확인
  - **Depends on**: T1~T4 (전 기능 탑재 상태에서 측정)

## 사전 승인 항목 (일괄 승인 대상)
- 전체 — 로컬 작업 브랜치 checkpoint/task 완료 commit (part1과 동일 위임. push·병합·태그·릴리즈는 불포함)
- (신규 의존성 없음 — serde 등은 part1 T1에서 승인·추가 완료 전제)

## 불가피한 Halt (위임 불가)
- T5 — 성능 미달로 구조 변경·의존성 추가가 필요해지는 경우 (plan에 없는 돌발 결정)
- (그 외 push·병합·릴리즈·PR·파괴적 작업 없음)

## Verification Strategy
- 빌드: `cargo build` / release 측정: `cargo build --release`
- Lint: `cargo clippy --all-targets -- -D warnings`
- 포맷: `cargo fmt --check`
- 단위 테스트: `cargo test` (part1 기존 테스트 포함 전체 통과 유지)
- 통합 테스트: `cargo test --test watcher`
- 수동 검증: task별 HUMAN-VERIFY 항목 구분 보고 + T5 성능 수치 보고

## Phase Ledger

## Retry Ledger

## Progress Log
- T1 완료 (커밋 436026d): FolderTree(SysTreeView32) 지연 확장·패널별 토글·선택 시 활성 탭 이동. 확장은 TVN_ITEMEXPANDING 보류(반환 1) 후 워커 열거 완료 시 apply_expand(차용 해제 후) — apply_item_count와 동일 계약. 리뷰 spec/quality 모두 OK, 테스트 35/35.
- T2 완료 (커밋 c1f3f58): shell_menu(IContextMenu 3단 + IContextMenu2/3 포워딩 thread_local). 배선은 NM_RCLICK 대신 WM_CONTEXTMENU 채택(키보드 메뉴 키 포함, 화면 좌표 직접 — Design 병기 문구 중 후자). Cargo.toml에 Win32_UI_Shell_Common feature 추가. 차용은 collect_context_menu_request 안에서 종료 후 모달 표시. 리뷰 OK(MINOR m1 → Deferred).

## Next Steps
- part1(docs/plans/2026-07-23-file-explorer-part1.md) 완료 후 이 plan을 pjc:implement-task로 실행
- 전체 완료 시 Phase G(PRD 재검증)는 이 part에서 수행 (active Must FR 100% — 두 part 합산 기준)

## Open Questions
- [x] Q1~Q5: part1과 공통 — 전부 해소됨 (part1 Open Questions 참조)
