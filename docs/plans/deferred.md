# Deferred 대장

## 대기
- [2026-07-23] FR-13 숨김·시스템 파일 표시 토글 (Could) (출처: 2026-07-23-file-explorer-part1)
- [2026-07-23] FR-14 분할 프리셋 버튼 1/2/4 (Could) (출처: 2026-07-23-file-explorer-part1)
- [2026-07-23] 트리→목록 양방향 동기화 (출처: 2026-07-23-file-explorer-part2 D14, egui 이식 후에도 단방향 유지)
- [2026-07-28] 구 Win32 UI 코드 제거 — 전체 삭제 7개(app/{window,sidebar,menu,layout_host}.rs·panel/{panel,folder_tree,address_bar}.rs)와 부분 삭제 3개(panel/{file_list,tabs}.rs의 Win32 래퍼, app/theme.rs의 COLORREF 상수). enable_dark_mode는 존치. **사용자가 화면 확인 후 판단하기로 보류** (출처: 2026-07-26-egui-migration-part2 T7)
- [2026-07-28] 한글 폰트 서브셋 — egui 메모리의 큰 몫(약 27MB 추정). NFR-2에 여유가 있어 지금은 불필요하나 더 줄여야 하면 첫 후보 (출처: 2026-07-26-egui-migration-part2)
- [2026-07-28] `app/`·`panel/`에 남는 순수 로직의 모듈 재배치(예: `core/`) — Win32 코드 제거 후에 검토 (출처: 2026-07-26-egui-migration-part2)
- [2026-07-28] 사이드바 마지막 항목 뒤 여백 4px — `show_items`가 항목마다 간격을 붙여 마지막 카드 뒤에도 남는다. 화면상 문제는 없음 (출처: 2026-07-26-egui-migration-part2 T2 quality 리뷰 m2)
- [2026-07-28] `debug-2026-07-24-dark-ownerdraw.md` 루트 위치 정리 — `docs/`로 옮길지 결정 (출처: 2026-07-26-egui-migration-part1)
- [2026-07-28] master 미병합 커밋 다수 — 다크 테마·사이드바·PoC·egui 이식이 모두 `task/*` 브랜치에 쌓여 있다. 병합 전략을 사용자와 결정해야 한다 (출처: 2026-07-26-egui-migration-part2)
- [2026-07-28] 워크스페이스 Delete 키 배정 — 사이드바가 키를 전역으로 보는 현재 구조에서는 파일 목록에서 누른 Delete까지 워크스페이스를 지운다. 키 미배정 + 메뉴 표기 제거로 처리했고, 카드에 포커스를 주는(`has_focus`) 방식으로 전환할 때 되살린다. F2도 같은 성질 (출처: 2026-07-26-egui-migration-part2 F-7 M2)
- [2026-07-28] 워크스페이스 키보드 위/아래 전환 — 현행 Win32 판에 있던 것이 이식에서 빠짐. PRD 요구는 아님 (출처: 2026-07-26-egui-migration-part2 F-7)
- [2026-07-29] 설정 팝업 5개 항목의 실제 기능 — 설정·업데이트·릴리즈 노트·오픈소스 라이선스·정보. v1은 항목 표시만 하고 전부 비활성 (출처: 2026-07-29-custom-titlebar T5)
- [2026-07-29] 커스텀 타이틀바의 창 그림자·둥근 모서리 — winit의 무장식 그림자 확장을 eframe이 노출하지 않아 이번엔 포기. `DwmSetWindowAttribute`의 코너 설정으로 모서리만 되살릴 여지는 있다 (출처: 2026-07-29-custom-titlebar D10)
- [2026-07-29] 탭 폭 고정(Windows 11은 탭마다 같은 폭) — 현재는 제목 길이에 맞춘다. 좁은 분할 패널에서 고정폭이 나은지 화면을 보고 판단 (출처: 2026-07-29-tab-strip-and-pane-borders)
- [2026-07-29] `ui/address_bar.rs`의 `nav_button` 활성 경로가 `theme::TEXT`·16px을 수동 재기술 — `widgets::icon_button` 래퍼를 쓰면 짧아지나 "활성·비활성 글꼴 동일" 제약이 코드에서 안 보이게 된다. `DEFAULT_ICON_PX`를 pub으로 올릴지와 함께 판단 (출처: 2026-07-29-tab-strip-and-pane-borders T3 quality S1)
- [2026-07-28] T6 측정 재현 절차 — 워크스페이스 5개 메모리는 임시 측정 빌드로 얻었고 스크립트를 레포에 남기지 않아 재현에 같은 패치가 필요하다 (출처: 2026-07-26-egui-migration-part2 F-7 m5)

- [2026-07-29] 자세히 보기의 고정 헤더 — 가로 스크롤을 위해 헤더를 본문과 같은 `ScrollArea`에 넣어, 세로 스크롤 시 머리글이 함께 올라간다. 세로만 고정하려면 두 영역의 오프셋을 수동 동기화해야 한다 (출처: 2026-07-29-view-modes-and-panel-menu T2)
- [2026-07-29] 보기 모드별 마지막 정렬 기준 기억 — 지금은 모드를 바꿔도 정렬이 유지된다 (출처: 같은 plan)
- [2026-07-29] 자세히 보기 열 추가·제거·순서 변경 — 이번에 열 메타데이터 구조가 생겨 확장 지점이 열렸다 (출처: 같은 plan)
- [2026-07-29] `list_grid::show`의 `visible` 부수 출력 — 즉시 모드에서 렌더 도중 `&mut PanelState`를 잡을 수 없어 택한 절충. 렌더와 수집을 나누는 구조가 있으면 검토 (출처: 같은 plan T14 quality m1)
- [2026-07-29] `PanelState::from_tabs` 인자 증가(4개) — 호출부가 이미 `PanelTabs`를 갖고 있어 그것을 넘기면 필드가 늘어도 시그니처가 안 바뀐다 (출처: 같은 plan T12 quality S1)
- [2026-07-29] 옛 세션 호환 테스트의 JSON 문자열 replace — 필드가 늘 때마다 목록이 길어진다. `serde_json::Value`에서 키를 제거하면 테스트를 안 건드려도 된다 (출처: 같은 plan T12 quality S2)
- [2026-07-29] 빈 영역 클릭 처리가 두 렌더 모듈에서 다른 기법 — 자세히는 콘텐츠 아래 사각형, 격자는 플래그 사후 억제. `list_common`에 헬퍼로 뽑을지 검토 (출처: 같은 plan T10 quality S1)
- [2026-07-29] `file_list::show`의 4-튜플 분해 — `DetailsOutcome`·`GridOutcome`이 `sort_click` 하나만 달라 공통 타입으로 묶을 여지 (출처: 같은 plan T10 quality S2)
- [2026-07-29] `ui/address_bar.rs`의 rustfmt 드리프트 — 이번 작업 전부터 있던 것으로 `cargo fmt --check`가 원래 실패한다. 별도 정리 필요 (출처: 같은 plan)
- [2026-07-30] **NFR-1 위반: 콜드 스타트 1.2~10.3초 (기준 1초 미만)** — 임시 계측으로 구간을 특정했다: `main` 진입~`run_native` 호출 직전이 **14ms**, `run_native` 진입~`ExplorerApp::new` 진입이 **나머지 전부**다. 즉 우리 코드(COM 12ms·세션 0.7ms·맑은 고딕 12.84MB 파싱 20ms·`ShellHost` 0.5ms·`IconCache` 16ms)가 아니라 **eframe의 창 생성 + glutin/glow GL 컨텍스트 초기화** 구간이다. 첫 프레임은 그 직후 0.2초 안에 나와 렌더 자체는 빠르다.
  - **배제된 가설 3개** (측정으로 확인, 재조사 낭비 방지):
    - **창 옵션 무관** — `full`(현행 무장식+위치+크기) / `bare`(`NativeOptions::default()`) / `decor`(장식 켬) / `nodark`(다크 정책 끔) 네 조합을 각 3회 측정했으나, **같은 `full` 설정이 1.2~2.0초와 4.5~9.2초 양쪽으로 나와** 조합 간 차이를 읽을 수 없었다. 조합별 수치를 원인으로 해석하면 틀린 결론이 된다.
    - **실행 간 간섭 무관** — 앞 프로세스의 GPU 자원 정리 지연을 의심해 대기 20초로 4회 측정했으나 4.5~9.2초로 같았다.
    - **끊긴 네트워크 드라이브 무관** — 연결이 끊긴 `Z:` 접근이 8.8초로 콜드 스타트와 수치가 거의 일치해 유력해 보였으나, PATH에 없고 시작 폴더는 홈이며 트리는 `GetLogicalDrives`(66ms)만 쓴다.
  - **남은 성질**: 값이 1.2~10.3초로 8배 흔들린다 → 시스템·드라이버 상태 의존을 시사한다(이 PC GPU는 Intel Arc 140T, 드라이버 32.0.101.8243). **가장 빠를 때조차 1.2초로 기준 미달**이다.
  - **다음 후보**: ① 다른 GPU·PC에서 재측정해 환경 요인인지 판별(가장 값싸다) ② eframe wgpu 백엔드와 비교 — **의존성 변경이라 승인 필요**하며 PRD가 백엔드를 glow로 명시(PoC 실측 wgpu 359MB vs glow 131MB, NFR-2)하므로 PRD 갱신도 함께 검토 ③ `log` 크레이트 로거를 붙여 eframe/glutin/winit 내부 로그의 타임스탬프로 구간을 더 쪼갠다(의존성 추가라 승인 필요).
  - 계측 패치는 커밋하지 않았다 — 재현에는 같은 패치가 필요하다(`ui::app`에 `probe(label)`를 두고 `FE_PROBE_FILE`로 출력 경로를 받아 `main`·`ExplorerApp::new`·`logic`에 호출을 심는 방식). (출처: 2026-07-29-view-modes-and-panel-menu F-8 남은 이슈 조사)
- [2026-07-30] NFR-3의 프레임 시간 미측정 — 10만 파일 폴더에서 "응답 유지·모드 전환 2.5~2.8초"까지는 외부 스크립트로 확인했으나, 프레임당 소요 시간은 앱 내부 계측(`ctx.input(|i| i.stable_dt)`) 없이는 잴 수 없다. 상시 표시할 성질이 아니므로 디버그 오버레이나 임시 계측으로 다룰지 결정 필요 (출처: 같은 조사)

## 종결
- [2026-07-23 → 2026-07-26] shell_menu items_menu의 pidls[0] "items 비지 않음" 암묵 계약 — 반영 (doc 주석 + debug_assert, egui 이식 part1 T3)
- [2026-07-23 → 2026-07-23] part2 실행 — 트리·셸 메뉴·감시·세션·성능 — 반영 (part2 T1~T5 완료, v1 완성)
- [2026-07-23 → 2026-07-23] exe/lnk 아이콘 비동기 프리페치 (T5 성능 미달 시 검토) — 기각 (T5 실측 NFR-1~3 전부 통과 — 프리페치 불필요)
- [2026-07-24 → 2026-07-28] 사이드바 항목 가상 스크롤·커스텀 다크 스크롤바 — 반영 (egui 이식 part2 T2 — `ScrollArea`가 스크롤·히트테스트를 자체 처리해 별도 구현이 필요 없어졌다)
- [2026-07-24 → 2026-07-28] 인라인 이름 편집 EDIT의 다크 스타일링 — 반영 (egui 이식 part2 T2 — Win32 EDIT 대신 `TextEdit`을 쓰면서 밝은 배경 제약이 사라졌다)
- [2026-07-26 → 2026-07-28] 전역 공유 자원 묶기(`SharedResources`) — 기각 (part2 T3에서 재평가: 명령을 `ExplorerApp`이 직접 처리해 `PanelState::show`·`splitter::show_layout`의 인자가 늘지 않았다. 묶어도 호출부 표기만 줄고 내부에서 다시 분해해야 해 실익 없음)
