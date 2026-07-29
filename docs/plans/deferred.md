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
- [2026-07-28] T6 측정 재현 절차 — 워크스페이스 5개 메모리는 임시 측정 빌드로 얻었고 스크립트를 레포에 남기지 않아 재현에 같은 패치가 필요하다 (출처: 2026-07-26-egui-migration-part2 F-7 m5)

## 종결
- [2026-07-23 → 2026-07-26] shell_menu items_menu의 pidls[0] "items 비지 않음" 암묵 계약 — 반영 (doc 주석 + debug_assert, egui 이식 part1 T3)
- [2026-07-23 → 2026-07-23] part2 실행 — 트리·셸 메뉴·감시·세션·성능 — 반영 (part2 T1~T5 완료, v1 완성)
- [2026-07-23 → 2026-07-23] exe/lnk 아이콘 비동기 프리페치 (T5 성능 미달 시 검토) — 기각 (T5 실측 NFR-1~3 전부 통과 — 프리페치 불필요)
- [2026-07-24 → 2026-07-28] 사이드바 항목 가상 스크롤·커스텀 다크 스크롤바 — 반영 (egui 이식 part2 T2 — `ScrollArea`가 스크롤·히트테스트를 자체 처리해 별도 구현이 필요 없어졌다)
- [2026-07-24 → 2026-07-28] 인라인 이름 편집 EDIT의 다크 스타일링 — 반영 (egui 이식 part2 T2 — Win32 EDIT 대신 `TextEdit`을 쓰면서 밝은 배경 제약이 사라졌다)
- [2026-07-26 → 2026-07-28] 전역 공유 자원 묶기(`SharedResources`) — 기각 (part2 T3에서 재평가: 명령을 `ExplorerApp`이 직접 처리해 `PanelState::show`·`splitter::show_layout`의 인자가 늘지 않았다. 묶어도 호출부 표기만 줄고 내부에서 다시 분해해야 해 실익 없음)
