# 작업 내역

## 최근 변경
- 2026-07-23: **v1 part2 완료 — 트리·셸 메뉴·감시·세션·성능 (T1~T5, v1 완성)**
  - **무엇을**: part1 코어 위에 잔여 PRD 요구 전부 구현. ① T1 폴더 트리(`SysTreeView32` 패널별 토글 — 드라이브 루트, TVN_ITEMEXPANDING 지연 확장 시 워커 열거 재사용, 확장은 보류(반환 1) 후 완료 시 `apply_expand`(차용 해제 후), 선택 시 활성 탭 navigate) ② T2 셸 컨텍스트 메뉴(`shell_menu` — SHParseDisplayName→IShellFolder→GetUIObjectOf(IContextMenu)→QueryContextMenu→TrackPopupMenuEx→InvokeCommand, IContextMenu2/3 소유자 메시지 포워딩 thread_local, 0개 선택은 CreateViewObject 배경 메뉴, 배선은 WM_CONTEXTMENU) ③ T3 변경 감시(`DirWatcher` — RDCW overlapped 발행 상시 유지 + 조용 300ms 디바운스, 정지 이벤트, Drop은 회수 스레드에 join 위임, F5=IDM_REFRESH 공용 refresh) ④ T4 세션(`settings.rs` D15 스키마 serde + parse 무결성 검증, TreeShape 스냅숏/재구성, WM_APP_SESSION_COLLECT/RESTORE 같은 스레드 SendMessage 포인터 계약, WM_CLOSE 저장·시작 복원·창 배치 모니터 검사) ⑤ T5 성능 실측(코드 수정 0)
  - **왜**: PRD FR-8(Must)·FR-9~12(Should)·NFR-1~3·NFR-7 충족 → v1 Must 8/8 + Should 4/4 완성. plan: docs/plans/2026-07-23-file-explorer-part2.md (분할 plan 2/2)
  - **핵심 결정**: lib 타깃 도입(bin+lib — tests/ 통합테스트 전제, main.rs mod→use), D18 세션 복원 시 삭제 경로는 is_dir 검사 없이 열거 실패 경로 위임(리뷰 B1 — UI 스레드 블로킹 금지), 디바운스는 "조용 300ms까지 흡수"(RDCW가 발행 사이 변경을 OS 버퍼링하는 특성 반영), Cargo.toml windows feature 4종 추가(Shell_Common·Security·System_IO·System_Threading — 신규 의존성 아님)
  - **검증 결과**: cargo build/clippy(-D warnings)/fmt/test 전부 통과 — 단위 43 + 통합 2(watcher, arming 루프로 시동 레이스 흡수·3회 연속 통과). 성능 실측(release 382KB, APPDATA 리다이렉트로 사용자 설정 미오염): **NFR-1 시작 209ms 중앙값 · NFR-2 2패널 유휴 20.8MB · NFR-3 10만 파일 진입 probe 82회 timeout 0** — 전부 PASS. task마다 spec+quality 이중 리뷰(T3 MAJOR 1·T4 BLOCKER 1 발견·수정 후 재리뷰 OK). **UI 동작(트리·셸 메뉴·자동 갱신·세션 복원 화면 확인)은 HUMAN-VERIFY 잔여**
  - **변경 파일**: src/lib.rs(신규), src/fs/{shell_menu,watcher}.rs(신규), src/panel/folder_tree.rs(신규), src/app/settings.rs(신규), tests/watcher.rs(신규), src/main.rs, src/app/{window,menu,layout,layout_host,mod}.rs, src/panel/{panel,file_list,tabs,mod}.rs, src/fs/{mod,icons}.rs, Cargo.toml
  - **미처리 Deferred**: FR-13 숨김 파일 토글(Could), FR-14 분할 프리셋(Could), 트리→목록 양방향 동기화(D14), shell_menu items_menu의 pidls[0] 암묵 계약 문서화(T2 리뷰 m1)
- 2026-07-23: **v1 part1 완료 — 멀티 분할 파일 탐색기 코어 (T1~T6)**
  - **무엇을**: Rust(windows-rs 0.62)/Win32 직접 호출로 파일 탐색기 코어 구현. ① 프로젝트 셋업(cargo·매니페스트 링커 임베드: PMv2 DPI/공용컨트롤 v6/longPathAware, build-dep 없이 build.rs 링커 플래그) ② 자유 분할 레이아웃 트리(순수 로직 split/close/set_ratio/compute_rects, 최소 120px 클램프) ③ 렌더링·스플리터 드래그·분할 명령(메뉴+Ctrl+\\·Ctrl+Shift+\\·Ctrl+Shift+W, DeferWindowPos 일괄 배치, 부모 히트테스트 스플리터) ④ 파일 목록(SysListView32 가상 모드, 워커 스레드 열거 FindFirstFileExW+`\\?\` 긴경로, 시스템 아이콘 공유+확장자/경로 캐시, StrCmpLogicalW 폴더우선 정렬) ⑤ 네비게이션(주소창 Edit 서브클래스 Enter·버튼 ←→↑, 탭별 히스토리 peek/커밋 분리, pending-커밋 모델 — 실패 시 현 위치 유지, ShellExecuteExW 실행, Alt+←/→/↑) ⑥ 패널별 탭(TabsModel 순수 모델+WC_TABCONTROL, Ctrl+T 복제/Ctrl+W 닫기 — 마지막 탭은 패널 닫기 연결)
  - **왜**: PRD(docs/prd.md) FR-1~7 Must + FR-12 일부 충족. plan: docs/plans/2026-07-23-file-explorer-part1.md (분할 plan 1/2)
  - **어떻게(핵심 결정)**: 창 상태는 `Box<RefCell<...>>` in GWLP_USERDATA(재진입 별칭 안전망 — 1차 방어는 핸들러 필터), LVM_SETITEMCOUNT는 RefCell 차용 해제 후 적용(재진입 표시 누락 방지), unsafe는 소단위 헬퍼 격리+안전성 주석, windows-rs 0.62 개명 주의(`Error::from_win32`→`from_thread`), GetMessageW는 `.0 > 0` 판정
  - **검증 결과**: cargo build/clippy(-D warnings)/fmt/test 전부 통과 — 단위테스트 35/35 (layout 10·enumerate 4·file_list 5·history 6·normalize 5·tabs 5). release exe 252KB. task마다 spec+quality 이중 리뷰(MAJOR 총 5건 발견·전부 수정 후 재리뷰 OK). **UI 동작(창·분할·드래그·탭·탐색)은 빌드 통과 상태이며 화면 확인은 사용자 필요(HUMAN-VERIFY)**
  - **변경 파일**: Cargo.toml, build.rs, app.manifest, src/main.rs, src/app/{window,layout,layout_host,menu}.rs, src/panel/{panel,file_list,address_bar,history,tabs}.rs, src/fs/{enumerate,icons}.rs
  - **미처리 Deferred**: part2 실행(docs/plans/2026-07-23-file-explorer-part2.md — 트리·셸 메뉴·감시·세션·성능), FR-13 숨김 파일 토글(Could), FR-14 분할 프리셋(Could), 아이콘 비동기 프리페치(part2 T5 성능 미달 시)

## 아카이브 인덱스
