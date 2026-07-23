# Plan: 멀티 분할 파일 탐색기 v1 — part1 (코어: 셋업~탭)

**PRD**: docs/prd.md
**다음 plan**: docs/plans/2026-07-23-file-explorer-part2.md

## 요구 이해
- **원문 요청**: "파일 탐색기를 구현하려고 하는데 이미지처럼 멀티로 탭을 분할해서 표시하는 기능을 중점으로 개발하려고 하는데 계획을 세워줘. 메모리, 성능에 좋은 개발 언어도 같이. Windows 11 이상에서만 사용할 예정"
- **이해한 요구**: Q-Dir 스크린샷 같은 멀티 패널 파일 탐색기를 새로 만든다. 핵심은 창을 여러 패널로 분할(사용자 선택: 자유 분할 트리형)하고 패널마다 독립 탭을 두는 것. 언어는 메모리·성능 우선으로 사용자가 Rust(windows-rs, Win32 직접)를 확정. v1 범위는 탐색 중심(파일 작업은 셸 컨텍스트 메뉴 위임), Windows 11+ x64 전용.
- **포함하지 않는 것으로 이해**: 복사/이동/삭제 등 파일 작업 UI 자체 구현·드래그앤드롭·검색은 v1에 포함하지 않는다 (PRD Out of Scope 승인됨).

## Goal
창을 자유 분할한 각 패널에서 탭·주소창·파일 목록으로 폴더를 탐색할 수 있는 코어 앱을 완성한다 (이 plan 범위: 셋업~탭).

**전체 목표**: PRD(docs/prd.md)의 Must 8개 + Should 4개를 충족하는 초경량 멀티 분할 파일 탐색기 v1 완성 (part2에서 트리·셸 메뉴·감시·세션·성능 마무리).

## PRD Coverage
| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-1 (자유 분할) | Must | T2, T3 | ✅ 커버 |
| FR-2 (스플리터·패널 닫기) | Must | T2, T3 | ✅ 커버 |
| FR-3 (패널별 탭) | Must | T6 | ✅ 커버 |
| FR-4 (파일 목록·정렬) | Must | T4 | ✅ 커버 |
| FR-5 (시스템 아이콘) | Must | T4 | ✅ 커버 |
| FR-6 (주소창·네비게이션) | Must | T5 | ✅ 커버 |
| FR-7 (더블클릭 진입·실행) | Must | T5 | ✅ 커버 |
| FR-8 (셸 컨텍스트 메뉴) | Must | (part2 담당) | ⏭️ 다음 part |
| FR-9 (폴더 트리) | Should | (part2 담당) | ⏭️ 다음 part |
| FR-10 (변경 감시) | Should | (part2 담당) | ⏭️ 다음 part |
| FR-11 (세션 복원) | Should | (part2 담당) | ⏭️ 다음 part |
| FR-12 (단축키) | Should | T3, T5, T6 (부분 — F5는 part2) | ✅ 커버(부분) |
| FR-13 (숨김 파일 토글) | Could | (이번 제외 — Deferred) | ⏭️ Deferred |
| FR-14 (분할 프리셋) | Could | (이번 제외 — Deferred) | ⏭️ Deferred |
| NFR-1~3 (성능) | — | 설계 반영(T4 가상화·백그라운드 열거), 실측은 part2 T5 | ⏭️ 다음 part |
| NFR-4 (PMv2 DPI) | — | T1 (매니페스트), T3 (WM_DPICHANGED 재배치) | ✅ 커버 |
| NFR-5 (긴 경로·유니코드) | — | T1 (longPathAware 매니페스트), T4 (Edge: `\\?\` 열거·UTF-16) | ✅ 커버 |
| NFR-6 (한국어 UI) | — | T1(창 제목)·T3(메뉴)·T4~T6(문구) — D10 확정 | ✅ 커버 |
| NFR-7 (%APPDATA% 저장) | — | (part2 T4 담당 — D15) | ⏭️ 다음 part |

## Out of Scope
- PRD Out of Scope 전체 (파일 작업 UI·드래그앤드롭·검색·다크 모드·즐겨찾기·가상 폴더·다국어)

## Deferred / Follow-up
- **다음 분할 plan**: docs/plans/2026-07-23-file-explorer-part2.md — T1~T5 (트리·셸 메뉴·감시·세션·성능, 전체의 후반부, 미실행)
- FR-13 숨김·시스템 파일 표시 토글 (Could — v1 이후 여유 시)
- FR-14 분할 프리셋 버튼 (Could — v1 이후 여유 시)
- exe/lnk 아이콘 비동기 프리페치 — part2 T5 성능 실측 미달 시 검토 (T4 quality 리뷰 M1 후속)
- HWND 재사용 이론적 경합(워커 PostMessage 직전 패널 파괴+재사용) — 실용 위험 낮음, 수용 (T4 quality 리뷰 m1)

## Investigation Log
- 위키 참조: 없음(vault 미설정·경로 부재) — 코드 1차 출처로 진행 (Test-Path 4개 후보 경로 모두 False, 2026-07-23)
- 프로젝트 폴더 빈 상태 확인: Glob `**/*` 0건 (2026-07-23) → 기존 코드·유사 구현·테스트 없음, Impact Analysis 4-A~4-C 해당 없음의 근거
- Rust 툴체인 실측: cargo 1.95.0 / rustc 1.95.0, stable-x86_64-pc-windows-msvc active (PowerShell 실행 확인, 2026-07-23) → T1 성립 확인 완료 (msvc 링커 전제 포함)
- deferred.md 대장: 없음 (신규 프로젝트 — 확인만 기록)
- AGENTS.md: bootstrap-agents-md로 신규 생성, 사용자 [Y] 승인 (2026-07-23) — 아키텍처 계층형(단일 crate) 확정
- PRD: docs/prd.md 사용자 승인 완료 (2026-07-23) — FR-1 자유 분할(트리형)·한국어 UI·%APPDATA% 저장 확정

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| windows-rs 바인딩 시그니처가 기억과 다를 수 있음 | 빌드 실패·재작업 | 구현 시 crate 문서(docs.rs/context7) 대조 후 사용 (추측 사용 금지 — CLAUDE.md 원칙) |
| `LVS_OWNERDATA` 가상 모드에서 정렬·아이콘 처리 실수 시 대량 폴더 성능 저하 | NFR-3 미달 | 정렬은 데이터 모델에서 1회 수행, 아이콘은 확장자 캐시 + 표시 시점 지연 조회 (D7·D8) |
| 트리형 레이아웃 리사이즈 시 자식 HWND 다건 이동으로 깜빡임 | UX 저하 | `BeginDeferWindowPos`/`DeferWindowPos` 일괄 배치 (T3 Design) |
| unsafe FFI 실수 (핸들 수명·포인터) | 크래시 | AGENTS.md 규칙 — unsafe는 함수 단위 격리 + 안전 래퍼, 상위 로직은 safe만 |
| 긴 경로(260+)·유니코드 처리 누락 | NFR-5 미달 | 전 구간 W API + 열거 시 `\\?\` 접두 처리 (T4 Edge Cases) |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
- 해당 없음 — 빈 프로젝트 (기존 심볼 0, Investigation Log의 Glob 0건 근거)

### 4-B. 계약·직렬화 변경
- 해당 없음 — 기존 계약 없음. 신규 직렬화(세션 스키마)는 part2 T4에서 도입 (version 필드 포함 설계, part2 Decisions 참조)

### 4-C. 테스트 파일
- 기존 테스트 없음. 신규: 각 task의 순수 로직 모듈에 `#[cfg(test)]` 단위테스트 동반 (Files에 명시)

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| 전체 (LayoutTree, FileList, Panel, TabStrip, History 등) | 기존 코드 0건 (빈 프로젝트 — Glob 전체 0건) | 재사용 대상 자체가 없어 전부 신규. 외부 GUI 프레임워크 재사용은 D2에서 기각(메모리 목표) |

### Verified by
- Glob `**/*` → 0건 (docs/prd.md·AGENTS.md는 본 계획 세션에서 생성) — 기존 구현·호출자·테스트 부재 확정

## Decisions
### D1. 의존성 (최소 의존 원칙)
- **Options**: A) windows + serde + serde_json만 / B) A + 매니페스트용 build-dep(embed-manifest 등) / C) GUI crate(egui/Slint) 추가
- **Chosen**: A
- **Rationale**: windows crate는 Win32 FFI 바인딩으로 직접 구현 불가(대체 불가). 설정 직렬화의 수제 JSON 파서는 오류 유발적이라 serde가 합리적. 매니페스트 임베드는 msvc 링커 플래그(`/MANIFEST:EMBED`, `/MANIFESTINPUT:`)를 build.rs의 `cargo:rustc-link-arg-bins`로 전달하면 build-dep 없이 가능 → B 기각. C는 프레임워크 자체 메모리로 NFR-2(50MB) 위협 → 기각.
- **Source**: 사용자 언어 선택(Rust/windows-rs) + AGENTS.md Stack + msvc 툴체인 실측(Investigation Log)

### D2. UI 구성 방식
- **Options**: A) Win32 공용 컨트롤 직접 (SysListView32/WC_TABCONTROL/Edit) / B) 전부 자체 그리기(Direct2D) / C) GUI 프레임워크
- **Chosen**: A
- **Rationale**: Q-Dir 수준 경량성의 핵심. 공용 컨트롤은 OS 제공이라 메모리 추가 비용이 거의 없고 가상 모드·시스템 아이콘 연동이 내장. B는 개발량 과다, C는 D1에서 기각.
- **Source**: PRD NFR-1~3 + 사용자 스택 확정

### D3. 레이아웃 표현 (FR-1 자유 분할)
- **Options**: A) 트리 (내부 노드 = 분할 방향+비율, 리프 = 패널) / B) 격자 좌표 배열
- **Chosen**: A
- **Rationale**: VS Code 방식. 자유 중첩 분할을 자연스럽게 표현하고 close 시 형제 승격이 단순. B는 중첩 분할 표현 불가.
- **Source**: 사용자 선택(자유 분할 트리형, 2026-07-23 질문 라운드)

### D4. 분할·새 탭 시 초기 경로
- **Options**: A) 현재 활성 탭 경로 복제 / B) 홈 폴더 고정
- **Chosen**: A
- **Rationale**: VS Code 에디터 분할·브라우저 탭 복제 관례. 분할 직후 같은 폴더에서 이어 작업하는 시나리오가 지배적.
- **Source**: 참조 모델 관례 (VS Code split / Q-Dir)

### D5. 스레딩 모델
- **Options**: A) UI 스레드 + std::thread 워커, 결과는 채널 + `WM_APP` 통지 (열거 세대 토큰으로 낡은 결과 폐기) / B) tokio async
- **Chosen**: A
- **Rationale**: GUI 메시지 루프와 자연 결합, async 런타임 메모리·복잡도 불필요. AGENTS.md Conventions 확정 사항.
- **Source**: AGENTS.md Conventions(동시성)

### D6. 에러 처리·표시
- **Options**: A) `windows::core::Result` 전파 + 폴더 열기 실패는 패널 목록 영역에 한국어 문구 표시(목록 대신) / B) 메시지박스 팝업
- **Chosen**: A
- **Rationale**: 권한 없는 폴더 진입 같은 일상 오류에 모달 팝업은 과함. 문구 톤은 일반 사용자 언어("이 폴더에 접근할 수 없습니다"). 치명 초기화 실패(main)만 메시지박스.
- **Source**: AGENTS.md 에러 처리 + decision-points UI 동작(문구 톤)

### D7. 정렬 규칙 (FR-4)
- **Options**: A) 이름 = `StrCmpLogicalW`(탐색기와 동일한 숫자 인지 정렬), 크기·날짜 = 수치 비교, 폴더 항상 우선 / B) 단순 유니코드 코드포인트 비교
- **Chosen**: A
- **Rationale**: Windows 탐색기와 체감 일치("파일2" < "파일10"). API 1개 호출로 구현 부담 없음.
- **Source**: Windows 탐색기 표준 동작

### D8. 아이콘 조회 (FR-5)
- **Options**: A) 시스템 이미지 리스트 공유(`SHGetFileInfoW` + `LVSIL_SMALL` 연결) + 확장자→인덱스 캐시, exe/lnk 등 개별 아이콘은 표시 시점 지연 조회 / B) 파일마다 즉시 개별 조회
- **Chosen**: A
- **Rationale**: 아이콘 복사본을 만들지 않아 메모리 절약(NFR-2), 10만 파일에서 조회 폭주 방지(NFR-3). B는 대량 폴더에서 열거 지연 유발.
- **Source**: PRD NFR-2·3

### D9. 히스토리 모델 (FR-6)
- **Options**: A) 탭당 `Vec<PathBuf>` + 커서 인덱스 (새 이동 시 커서 뒤 절단) / B) 전역 히스토리
- **Chosen**: A
- **Rationale**: 브라우저 표준 모델. 탭별 독립 히스토리는 FR-3 요구사항.
- **Source**: PRD FR-3·FR-6

### D10. 명령 배치·단축키 (FR-12 일부)
- **Options**: A) 메인 메뉴 바(파일/보기) + 액셀러레이터 테이블, 패널 헤더에는 [←][→][↑]+주소창만 / B) 패널마다 전체 툴바
- **Chosen**: A
- **Rationale**: 패널이 늘수록 패널별 툴바는 공간·메모리 낭비. 분할 명령은 활성 패널 대상이므로 전역 메뉴가 자연스러움. 단축키: Ctrl+\\ 좌우 분할, Ctrl+Shift+\\ 상하 분할, Ctrl+Shift+W 패널 닫기, Ctrl+T 새 탭, Ctrl+W 탭 닫기, Alt+←/→ 히스토리, Alt+↑ 상위. UI 문구 전부 한국어(NFR-6).
- **Source**: PRD FR-12 + Q-Dir 참조 (패널 헤더 최소화)

### D11. 드라이브 루트에서 상위 이동
- **Options**: A) 상위 없음 → 버튼 비활성 / B) '내 PC' 가상 뷰 제공
- **Chosen**: A
- **Rationale**: 셸 네임스페이스 가상 폴더는 PRD Out of Scope. 드라이브 전환은 주소창 입력으로 가능(v1).
- **Source**: PRD Out of Scope

### D12. 테스트 전략
- **Options**: A) 순수 로직(레이아웃 트리·정렬 비교·히스토리)을 UI 비의존 모듈로 분리해 단위테스트, HWND 필요 코드는 테스트 비대상 / B) UI 자동화 테스트 도입
- **Chosen**: A
- **Rationale**: AGENTS.md 테스트 규칙 그대로. UI 자동화는 v1 투자 대비 효과 낮음. UI 동작은 HUMAN-VERIFY로 구분 보고.
- **Source**: AGENTS.md Conventions(테스트)

## Tasks

- [x] T1. 프로젝트 셋업 — cargo init, 의존성, 매니페스트, 빈 메인 창
  - **Type**: C
  - **Design**: ① 배치: 루트 `Cargo.toml`·`build.rs`·`app.manifest`, `src/main.rs`, `src/app/window.rs` ② 신규 심볼: `main`(진입·COM 초기화 `CoInitializeEx`·메시지 루프), `app::window::MainWindow`(창 클래스 등록·생성·`WndProc` 디스패치 소유) ③ 의존: window → windows crate만, main → window ④ 비추상화: 범용 Win32 래퍼 라이브러리를 만들지 않음 — 이 앱에 필요한 함수만 감싼다. `#![windows_subsystem = "windows"]`로 콘솔 억제.
  - **Acceptance**: Given 빈 프로젝트, When `cargo build`·`cargo clippy --all-targets -- -D warnings` 실행, Then 경고·에러 0. When `cargo run`, Then 크기 조절 가능한 빈 창 표시(제목 "파일 탐색기") — 창 표시·DPI 선명도는 HUMAN-VERIFY. 매니페스트(PMv2 DPI·공용 컨트롤 v6·longPathAware)가 build.rs 링커 플래그로 임베드됨.
  - **Files**:
    - 주: `Cargo.toml`, `build.rs`, `app.manifest`, `src/main.rs`, `src/app/window.rs`
    - 동반: `.gitignore` (target/ 제외), `src/app/mod.rs`
  - **Edge Cases**:
    - COM 초기화 실패 → 메시지박스 후 종료 (치명 오류 — D6)
    - 창 클래스 중복 등록(다중 실행) → 다중 인스턴스 허용, 클래스 등록 실패 시에만 종료
  - **Halt Forecast**:
    - (ii-a) cargo init(프로젝트 구조 생성)·의존성 추가(windows, serde, serde_json)·git init → `## 사전 승인 항목`에 등록
  - **Depends on**: -

- [x] T2. 레이아웃 트리 모델 (순수 로직) — FR-1·FR-2의 두뇌
  - **Type**: C
  - **Design**: ① 배치: `src/app/layout.rs` (HWND 비의존 순수 모듈 — D12) ② 신규 심볼: `LayoutTree` — split(리프를 지정 방향으로 분할해 새 리프 반환)/close(리프 제거·형제 승격)/set_ratio/compute_rects(영역 → 리프별 Rect + 스플리터 Rect 목록 계산) 책임. `PanelId`(리프 식별자) ③ 의존: windows crate 비의존(RECT 대신 자체 `Rect` 구조체) — T3가 참조 ④ 비추상화: 도킹/플로팅/탭화 등 범용 도킹 프레임워크로 일반화하지 않음. 패널 수 상한 없음(스플리터 최소 크기로 자연 제한).
  - **Acceptance**: Given 단일 리프, When 좌우 분할 → 상하 분할 → 닫기 시퀀스, Then 트리 구조·비율·compute_rects 결과가 기대값과 일치하는 단위테스트 전부 통과 (`cargo test`). 마지막 1개 리프 close는 Err 반환.
  - **Files**:
    - 주: `src/app/layout.rs`
    - 테스트: 동일 파일 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 마지막 1개 패널 close → 거부(Err) (FR-2)
    - 비율 극단값 → 최소 패널 폭/높이 120px 미만이면 분할 거부 또는 비율 클램프
    - 영역 0×0(창 최소화) → compute_rects가 빈 Rect 허용, 패닉 금지
  - **Halt Forecast**:
    - (i) "비율 클램프 기준?" → Edge Cases에 명시(최소 120px)
  - **Depends on**: -

- [x] T3. 레이아웃 렌더링·스플리터·분할 명령 — FR-1·FR-2 완성
  - **Type**: D
  - **Design**: ① 배치: `src/app/layout_host.rs`(트리↔HWND 배치), `src/app/menu.rs`(메뉴 바·액셀러레이터), window.rs에 배선 ② 신규 심볼: `LayoutHost` — LayoutTree의 compute_rects 결과로 패널 HWND들을 `DeferWindowPos` 일괄 배치, 스플리터 히트테스트·드래그(SetCapture, 커서 IDC_SIZEWE/NS), WM_SIZE 재배치 책임. `menu::build_menu`·`menu::accel_table` — 한국어 메뉴(보기: 좌우 분할/상하 분할/패널 닫기)와 단축키(D10) 정의 ③ 의존: LayoutHost → layout.rs(T2)·panel(T4 이후 실 패널, 그 전엔 자리표시 자식 창) — window.rs가 호출 ④ 비추상화: 스플리터를 별도 HWND로 만들지 않음(부모 히트테스트 방식 — 창 수 절약).
  - **Acceptance**: Given 실행된 앱, When 메뉴/단축키로 좌우·상하 분할 반복 후 스플리터 드래그·패널 닫기, Then 패널들이 겹침·틈 없이 배치되고 드래그로 비율 변경, 마지막 패널 닫기 메뉴는 비활성 — 화면 동작은 HUMAN-VERIFY. 빌드·clippy 0은 기계 검증.
  - **Files**:
    - 주: `src/app/layout_host.rs`, `src/app/menu.rs`
    - 동반: `src/app/window.rs`(WM_SIZE·WM_COMMAND·마우스 메시지 배선), `src/app/mod.rs`, `src/app/layout.rs`(SplitterRect.node_area 필드 확장 — 드래그 비율 계산용, 구현 중 자율 추가), `src/main.rs`(TranslateAcceleratorW 루프)
  - **Edge Cases**:
    - 창 최소화 → 0 크기 재배치 무해 통과 (T2 Edge와 연동)
    - 드래그 중 캡처 상실(Alt+Tab 등) → WM_CAPTURECHANGED에서 드래그 상태 정리
    - DPI 변경(모니터 이동) → WM_DPICHANGED에서 재배치 (NFR-4)
    - 분할 최소 크기 미달 → 분할 명령 무시 + 상태 유지
  - **Halt Forecast**:
    - (i) "스플리터를 HWND로 만들까?" → Design ④에서 결정(부모 히트테스트)
    - (i) "분할 명령 UI 위치?" → D10에서 결정(메뉴+단축키)
  - **Depends on**: T1, T2

- [x] T4. 파일 목록 패널 — 가상 리스트뷰·백그라운드 열거·정렬·아이콘 (FR-4·FR-5)
  - **Type**: D
  - **Design**: ① 배치: `src/panel/file_list.rs`(리스트뷰 래퍼), `src/fs/enumerate.rs`(워커 열거), `src/fs/icons.rs`(아이콘 캐시), `src/panel/panel.rs`(패널 컨테이너 골격) ② 신규 심볼: `FileList` — `SysListView32`(`LVS_OWNERDATA`) 생성, `LVN_GETDISPINFO`로 항목 공급, 열(이름/크기/종류/수정일)·헤더 클릭 정렬 상태 책임. `fs::enumerate::spawn_enumerate(path, generation, tx)` — 워커 스레드에서 `FindFirstFileExW`(대용량 힌트 플래그) 열거 후 채널+`WM_APP_ENUM_DONE` 통지. `fs::icons::IconCache` — 확장자→시스템 이미지 리스트 인덱스 캐시(D8). `panel::Panel` — 자식(주소창 영역은 T5에서 채움)+목록 배치 컨테이너 ③ 의존: Panel → FileList → fs::* ; LayoutHost(T3)가 Panel HWND를 배치 ④ 비추상화: 파일 시스템 추상 트레이트(백엔드 교체용) 안 만듦 — Win32 직접 호출만.
  - **Acceptance**: Given 폴더 경로가 지정된 패널, When 열거 완료, Then 이름/크기/종류/수정일이 표시되고 헤더 클릭으로 정렬 토글(이름은 StrCmpLogicalW·폴더 우선 — 정렬 비교 함수는 단위테스트) — 목록 표시는 HUMAN-VERIFY. Given 10만 파일 테스트 폴더, When 진입, Then UI 입력이 멈추지 않음(열거 중 "읽는 중…" 표시, NFR-3 체감 확인은 part2 T5에서 실측).
  - **Files**:
    - 주: `src/panel/file_list.rs`, `src/fs/enumerate.rs`, `src/fs/icons.rs`, `src/panel/panel.rs`
    - 동반: `src/panel/mod.rs`, `src/fs/mod.rs`, `src/main.rs`(모듈 등록), `src/app/layout_host.rs`(자리표시 창 → panel::create 교체), `Cargo.toml`(windows feature 확장 — 구현 중 자율 추가 기록. WM_APP 배선은 window.rs가 아니라 패널 자체 프로시저가 처리하는 것으로 조정)
    - 테스트: `src/panel/file_list.rs`·`src/fs/enumerate.rs`의 `#[cfg(test)]`(정렬 비교·열거 결과 모델)
  - **Edge Cases**:
    - 접근 거부 폴더 → 목록 영역에 "이 폴더에 접근할 수 없습니다" 문구 (D6)
    - 빈 폴더 → "빈 폴더" 문구
    - 열거 중 다른 폴더로 이동 → 세대 토큰 불일치 결과 폐기 (D5)
    - 긴 경로(260+) → `\\?\` 접두로 열거, 표시는 원 경로
    - 유니코드·이모지 파일명 → UTF-16 그대로 표시 (변환 없음)
    - 심볼릭 링크/junction → 대상 따라가지 않고 항목 자체 표시
  - **Halt Forecast**:
    - (i) "정렬 규칙?" → D7 / "아이콘 전략?" → D8 / "스레딩?" → D5 — 모두 사전 결정됨
  - **Depends on**: T1, T3

- [x] T5. 네비게이션 — 주소창·히스토리·더블클릭 (FR-6·FR-7)
  - **Type**: D
  - **Design**: ① 배치: `src/panel/address_bar.rs`(Edit 컨트롤+[←][→][↑] 버튼), `src/panel/history.rs`(순수 로직), panel.rs에 통합 ② 신규 심볼: `AddressBar` — 경로 표시·Enter 입력 처리·버튼 상태(활성/비활성) 책임. `History` — Vec<PathBuf>+커서, push/back/forward/can_* (D9) ③ 의존: Panel이 AddressBar·History·FileList를 소유하고 "경로 이동" 흐름을 조율(navigate(path) 단일 진입점) ④ 비추상화: URL·셸 네임스페이스 파싱 안 함 — 파일시스템 경로 문자열만.
  - **Acceptance**: Given 히스토리 로직, When push/back/forward 시퀀스(분기 이동 포함), Then 커서·목록 상태가 기대값과 일치하는 단위테스트 통과. Given 실행 앱, When 주소창에 존재 경로 입력(Enter)·폴더 더블클릭·파일 더블클릭·Alt+←/→/↑, Then 각각 이동/진입/연결 프로그램 실행(`ShellExecuteExW`)/히스토리 이동 — HUMAN-VERIFY.
  - **Files**:
    - 주: `src/panel/address_bar.rs`, `src/panel/history.rs`
    - 동반: `src/panel/panel.rs`(navigate 조율), `src/app/menu.rs`(Alt 단축키), `src/panel/mod.rs`, `src/app/window.rs`(Alt 액셀 → 활성 패널 라우팅), `src/app/layout_host.rs`(active_hwnd 신설), `src/panel/file_list.rs`(entry_at 소비), `Cargo.toml`(Win32_System_Registry — SHELLEXECUTEINFOW 구조체 요구) — 구현 중 자율 추가 기록 (spec 리뷰 MINOR 반영)
    - 테스트: `src/panel/history.rs` `#[cfg(test)]`
  - **Edge Cases**:
    - 존재하지 않는 경로 입력 → "경로를 찾을 수 없습니다" 문구 표시, 현 위치 유지
    - back 대상 폴더가 그 사이 삭제됨 → 동일 문구 + 현 위치 유지 (히스토리 항목은 보존)
    - 드라이브 루트에서 상위 → 버튼·단축키 비활성/무시 (D11)
    - 파일 실행 실패(연결 프로그램 없음) → 셸 기본 오류 UI에 위임 (ShellExecuteEx가 표시)
    - 상대 경로·따옴표 포함 입력 → 절대 경로로 정규화 후 판정
  - **Halt Forecast**:
    - (i) "잘못된 경로 처리?" → Edge Cases 명시 / "히스토리 모델?" → D9
  - **Depends on**: T4

- [x] T6. 패널별 탭 (FR-3)
  - **Type**: D
  - **Design**: ① 배치: `src/panel/tabs.rs`, panel.rs 통합 ② 신규 심볼: `TabStrip` — `WC_TABCONTROL` 래퍼(추가/닫기/전환 UI, 탭 제목=폴더명). `TabState` — path+History 묶음(활성 탭 전환 시 FileList 내용 교체) ③ 의존: Panel이 Vec<TabState>+TabStrip 소유. 히스토리는 TabState로 이관(T5의 History 재사용 — 신규 로직 아님) ④ 비추상화: 탭 드래그 재배열·패널 간 탭 이동은 만들지 않음(v2).
  - **Acceptance**: Given 실행 앱, When Ctrl+T(현재 경로 복제 새 탭)·탭 클릭 전환·Ctrl+W(탭 닫기), Then 탭별 경로·히스토리가 독립 유지되고 마지막 탭 닫기는 패널 닫기로 연결(마지막 패널의 마지막 탭이면 무시) — HUMAN-VERIFY. 탭 상태 전환 로직(활성 인덱스·닫기 규칙)은 단위테스트 통과.
  - **Files**:
    - 주: `src/panel/tabs.rs`
    - 동반: `src/panel/panel.rs`, `src/app/menu.rs`(Ctrl+T/W), `src/panel/mod.rs`
    - 테스트: `src/panel/tabs.rs` `#[cfg(test)]`(탭 목록 모델)
  - **Edge Cases**:
    - 마지막 패널의 마지막 탭 닫기 → 무시 (앱 종료는 창 닫기로만 — FR-2와 일관)
    - 탭 다수(20+) → 탭 컨트롤 기본 스크롤 화살표에 위임 (커스텀 처리 없음)
    - 탭 전환 중 열거 진행 중 → 세대 토큰으로 이전 탭 결과 폐기 (D5)
  - **Halt Forecast**:
    - (i) "새 탭 초기 경로?" → D4 / "마지막 탭 닫기 정책?" → Edge Cases 명시
  - **Depends on**: T5

## 사전 승인 항목 (일괄 승인 대상)
- T1 — `git init` + `.gitignore` 생성 (비git 폴더 → 버전 관리 시작. 되돌리기 안전장치 확보 목적, 비파괴)
- T1 — `cargo init` 프로젝트 구조 생성 (Cargo.toml·src/ 신규 — 구조 변경)
- T1 — 의존성 추가: `windows`(windows-rs), `serde`, `serde_json` (D1 — 추가 의존성 없음, 이후 필요 시 별도 승인)
- 전체 — 로컬 작업 브랜치 checkpoint/task 완료 commit (CLAUDE.md Git 예외 — implement-task 규약 위임. push·병합·태그·릴리즈는 불포함)

## 불가피한 Halt (위임 불가)
- (없음 — push·병합·릴리즈·PR·파괴적 작업·외부 서비스가 이 plan에 없음. 계획에 없던 돌발 결정 발생 시 그 지점에서 Halt)

## Verification Strategy
- 빌드: `cargo build` (경고 0)
- Lint: `cargo clippy --all-targets -- -D warnings`
- 포맷: `cargo fmt --check`
- 단위 테스트: `cargo test` (layout·history·정렬·탭 모델)
- 수동 검증: task별 Acceptance의 HUMAN-VERIFY 항목 — 빌드 통과와 구분해 "사용자 확인 필요"로 보고 (CLAUDE.md 검증 원칙)

## Phase Ledger

## Retry Ledger

## Progress Log
- T1-T2 완료: 프로젝트 셋업(windows-rs 0.62.2·매니페스트 링커 임베드·빈 창) + 레이아웃 트리 모델(split/close/set_ratio/compute_rects, 테스트 11개). 빌드/clippy/fmt/test 전부 통과.
  - 결정: Error::from_win32는 0.62에서 from_thread로 개명됨(적용). GetMessageW는 `.0 > 0` 판정(-1 오류 함정). 미소비 모듈은 `#![cfg_attr(not(test), expect(dead_code))]` 자기 정리형 패턴 사용 — T3에서 소비 시 제거 필요.
  - NodePath = 루트→노드 비트열(0=first,1=second), 스플리터가 이 경로로 set_ratio 호출.
- T3-T4 완료: 레이아웃 렌더링(스플리터 드래그·메뉴·단축키) + 파일 목록(가상 리스트뷰·워커 열거·정렬·아이콘). 테스트 20/20.
  - 결정: 창 상태는 Box<RefCell<...>> in GWLP_USERDATA (재진입 별칭 안전망 — 1차 방어는 핸들러 필터). LVM_SETITEMCOUNT는 반드시 RefCell 차용 해제 후 apply_item_count로 (재진입 표시 누락 방지). exe/lnk/ico는 경로별 아이콘 캐시. Panel은 pub struct가 아니라 panel::create(parent)->HWND + 창 귀속 상태(관례 일치 — Design 표기와 다름, spec 리뷰 수용).
  - T5 참고: navigate로 실폴더 이동 시 clear() 직후 apply_item_count(hwnd, 0) 필요 (quality 리뷰 follow-up).

## Next Steps
- part1 완료 후 → 남은 분할 plan: docs/plans/2026-07-23-file-explorer-part2.md — pjc:implement-task로 별도 실행

## Open Questions
- [x] Q1: 개발 언어/스택? → Rust(windows-rs) 확정 (2026-07-23)
- [x] Q2: v1 범위? → 탐색+멀티 분할 중심 확정 (2026-07-23)
- [x] Q3: 분할 방식? → 자유 분할(트리형) 확정 — 프리셋은 Could(FR-14) (2026-07-23)
- [x] Q4: UI 언어/설정 저장/우선순위? → 한국어 고정 / %APPDATA% / PRD 배정 그대로 확정 (2026-07-23)
- [x] Q5: plan 분할? → B(2개 분할) 확정 (2026-07-23)
