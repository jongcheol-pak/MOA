# 작업 내역

## 최근 변경
- 2026-07-23: **v1 part1 완료 — 멀티 분할 파일 탐색기 코어 (T1~T6)**
  - **무엇을**: Rust(windows-rs 0.62)/Win32 직접 호출로 파일 탐색기 코어 구현. ① 프로젝트 셋업(cargo·매니페스트 링커 임베드: PMv2 DPI/공용컨트롤 v6/longPathAware, build-dep 없이 build.rs 링커 플래그) ② 자유 분할 레이아웃 트리(순수 로직 split/close/set_ratio/compute_rects, 최소 120px 클램프) ③ 렌더링·스플리터 드래그·분할 명령(메뉴+Ctrl+\\·Ctrl+Shift+\\·Ctrl+Shift+W, DeferWindowPos 일괄 배치, 부모 히트테스트 스플리터) ④ 파일 목록(SysListView32 가상 모드, 워커 스레드 열거 FindFirstFileExW+`\\?\` 긴경로, 시스템 아이콘 공유+확장자/경로 캐시, StrCmpLogicalW 폴더우선 정렬) ⑤ 네비게이션(주소창 Edit 서브클래스 Enter·버튼 ←→↑, 탭별 히스토리 peek/커밋 분리, pending-커밋 모델 — 실패 시 현 위치 유지, ShellExecuteExW 실행, Alt+←/→/↑) ⑥ 패널별 탭(TabsModel 순수 모델+WC_TABCONTROL, Ctrl+T 복제/Ctrl+W 닫기 — 마지막 탭은 패널 닫기 연결)
  - **왜**: PRD(docs/prd.md) FR-1~7 Must + FR-12 일부 충족. plan: docs/plans/2026-07-23-file-explorer-part1.md (분할 plan 1/2)
  - **어떻게(핵심 결정)**: 창 상태는 `Box<RefCell<...>>` in GWLP_USERDATA(재진입 별칭 안전망 — 1차 방어는 핸들러 필터), LVM_SETITEMCOUNT는 RefCell 차용 해제 후 적용(재진입 표시 누락 방지), unsafe는 소단위 헬퍼 격리+안전성 주석, windows-rs 0.62 개명 주의(`Error::from_win32`→`from_thread`), GetMessageW는 `.0 > 0` 판정
  - **검증 결과**: cargo build/clippy(-D warnings)/fmt/test 전부 통과 — 단위테스트 35/35 (layout 10·enumerate 4·file_list 5·history 6·normalize 5·tabs 5). release exe 252KB. task마다 spec+quality 이중 리뷰(MAJOR 총 5건 발견·전부 수정 후 재리뷰 OK). **UI 동작(창·분할·드래그·탭·탐색)은 빌드 통과 상태이며 화면 확인은 사용자 필요(HUMAN-VERIFY)**
  - **변경 파일**: Cargo.toml, build.rs, app.manifest, src/main.rs, src/app/{window,layout,layout_host,menu}.rs, src/panel/{panel,file_list,address_bar,history,tabs}.rs, src/fs/{enumerate,icons}.rs
  - **미처리 Deferred**: part2 실행(docs/plans/2026-07-23-file-explorer-part2.md — 트리·셸 메뉴·감시·세션·성능), FR-13 숨김 파일 토글(Could), FR-14 분할 프리셋(Could), 아이콘 비동기 프리페치(part2 T5 성능 미달 시)

## 아카이브 인덱스
