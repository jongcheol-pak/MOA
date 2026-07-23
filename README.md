# FileExplorer — 멀티 분할 경량 파일 탐색기

Windows 11 이상(x64)에서 하나의 창을 **자유 분할(트리형)**하고, 각 패널이 **독립 탭**을 가지는 초경량 파일 탐색기입니다. Rust(windows-rs)로 Win32 API를 직접 호출해 GUI 프레임워크 없이 구현했습니다 (Q-Dir의 멀티 패널 + VS Code의 자유 분할 UX 참고).

## 핵심 기능 (v1 part1 — 현재 구현 상태)

- **자유 분할**: 활성 패널을 좌우/상하로 중첩 분할 (보기 메뉴, `Ctrl+\` 좌우, `Ctrl+Shift+\` 상하), 스플리터 드래그로 비율 조절, `Ctrl+Shift+W` 패널 닫기 (마지막 1개는 닫기 불가)
- **패널별 탭**: `Ctrl+T` 새 탭(현재 경로 복제), `Ctrl+W` 탭 닫기(마지막 탭은 패널 닫기로 연결), 탭 클릭 전환, 탭별 독립 히스토리
- **파일 목록**: 상세 보기(이름·크기·종류·수정한 날짜), 열 클릭 정렬(탐색기와 동일한 숫자 인지 정렬·폴더 우선), 시스템 아이콘, 가상 리스트뷰 + 백그라운드 열거(대량 폴더 UI 무정지)
- **탐색**: 주소창 경로 입력(Enter), 뒤로/앞으로/상위(`Alt+←/→/↑`), 더블클릭 폴더 진입·파일 실행, 잘못된 경로 입력 시 현 위치 유지
- 긴 경로(260자+)·유니코드 파일명, Per-Monitor v2 DPI, 한국어 UI

> part2(예정): 폴더 트리 · 우클릭 셸 컨텍스트 메뉴 · 변경 자동 새로고침 · 세션 저장/복원 · 성능 실측 (docs/plans/2026-07-23-file-explorer-part2.md)

## 실행 방법

```
cargo run              # 디버그 실행
cargo build --release  # 배포 빌드 (단일 exe, 약 250KB)
```

요구 사항: Rust stable(1.80+, msvc 툴체인), Windows 11 이상.

검증: `cargo test`(단위테스트), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`

## 아키텍처

계층형 단일 crate — 모듈 분리 (상세 규약은 `AGENTS.md`, 요구사항 정본은 `docs/prd.md`):

```
src/
├── main.rs            # 진입점 — COM 초기화, 액셀러레이터, 메시지 루프
├── app/
│   ├── window.rs      # 메인 창 — 명령 라우팅, RefCell 상태(GWLP_USERDATA)
│   ├── layout.rs      # 분할 레이아웃 트리 (순수 로직 — split/close/비율/배치 계산)
│   ├── layout_host.rs # 트리 → 자식 HWND 배치, 스플리터 드래그
│   └── menu.rs        # 한국어 메뉴 바·단축키 테이블
├── panel/
│   ├── panel.rs       # 패널 컨테이너 — pending-커밋 탐색 조율
│   ├── tabs.rs        # 탭 모델(순수)+WC_TABCONTROL 래퍼
│   ├── file_list.rs   # SysListView32 가상 모드 — 표시·정렬
│   ├── address_bar.rs # 주소창(버튼+Edit)·경로 정규화
│   └── history.rs     # 탭별 탐색 히스토리 (순수 로직)
└── fs/
    ├── enumerate.rs   # 워커 스레드 디렉터리 열거 (FindFirstFileExW, \\?\ 긴경로)
    └── icons.rs       # 셸 아이콘·종류 캐시 (시스템 이미지 리스트 공유)
```

핵심 플로우: 탐색 요청 → `Panel::navigate`(pending 등록, 세대 토큰 증가) → 워커 스레드 열거 → 채널+`WM_APP` 통지 → 성공 시에만 경로·히스토리 커밋 (실패 시 현 위치 유지). 파일 작업(복사·삭제 등)은 v1에서 셸 컨텍스트 메뉴에 위임 예정(part2).
