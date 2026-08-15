# Plan: 팝업 공통 디자인 — 둥근 모서리와 전폭 하단 버튼

## 요구 이해
- **원문 요청**: "이미지와 같은 스타일로 팝업 디자인 수정 작업(이미지처럼 흰색 배경은 적용할 필요 없고 색상은 현재 유지함). 이미지처럼 팝업의 모서리 부분은 둥글고 하단에 표시되는 버튼은 화면 사이즈에 맞게 버튼을 표시함. 모든 팝업 동일하게 적용.(앞으로 추가되는 팝업은 공통)"
- **이해한 요구**: 첨부한 iOS alert 이미지에서 **모양 두 가지만** 가져온다 — ① 팝업 네 모서리를 둥글게(12px) ② 하단 버튼을 팝업 **전폭에 균등 분할**하고 구분선으로 나눈다. 색은 지금 다크 팔레트를 그대로 쓴다(흰 배경·파란 글자 아님). 지금 있는 모달 8곳에 **모두** 적용하고, 앞으로 만들 팝업이 같은 규격을 자동으로 쓰도록 **공통 컴포넌트 하나**로 모은 뒤 그것을 거치지 않은 팝업을 시험이 잡게 한다.
- **포함하지 않는 것으로 이해**: 제목·본문의 가운데 정렬, 본문 폭 변경, 버튼 좌우 순서 뒤집기는 하지 않는다(D6·D8). 우클릭 메뉴·알림 토스트도 이번 대상이 아니다(D1).

## Goal
모든 모달 팝업이 하나의 공통 셸을 거쳐 둥근 모서리와 전폭 균등 분할 하단 버튼으로 그려진다.

## Out of Scope
- **본문 폭 변경** — 여덟 대화의 본문 폭(360·420·460·440·1080-좌우여백)은 그대로 둔다. 프레임 바깥 폭은 여백을 18px로 통일하면서 환산되며(D11 표), 그것은 폭 변경이 아니라 여백 통일의 결과다
- 팝업 높이 변경 (앱 설정 560 · 사이트 관리자 680 그대로)
- 제목·본문 문구와 정렬 변경
- 우클릭 팝업 메뉴(`ui/panel.rs:1367`)·알림 토스트(`ui/toast.rs:80`) — 버튼이 없어 하단 버튼 규칙이 성립하지 않고, 모서리만 바꾸는 것은 이번 요구에 없다

## Deferred / Follow-up
- 본문 폭을 규격화(예: 소형 400 / 중형 480)해 대화마다 제각각인 값을 정리 — 이번엔 폭을 건드리지 않아 미룬다
- `ui/site_manager.rs`(1844줄)·`ui/app.rs`(3573줄)의 1500줄 분리 검토선 초과 — 대장에 이미 등록된 항목이며 이번 변경은 두 파일에서 각각 70줄 안쪽이라 분리를 유발하지 않는다

## Investigation Log
- 위키 참조: 관련 위키 자료 없음 — vault(`D:/Personal Project/Obsidian Vault/LLM WIKI`)는 실재하나 MOA는 `20_projects/`에 미등록이고, `30_knowledge`·`40_guides`에 egui 모달·대화 레시피가 없다(`grep -ril "egui|모달|팝업"` → gpui·winui·web 항목만). 코드 1차 출처로 진행
- Deferred 대장 확인: `docs/plans/deferred.md` 53건(이전 plan 7건 이관 후). 이번 주제와 걸리는 항목은 `[2026-07-29] 커스텀 타이틀바의 창 그림자·둥근 모서리` 하나인데, 그것은 **창 자체**(winit 무장식)의 문제라 팝업 프레임과 층이 다르다 — 이번 작업으로 해소되지 않고 방해도 받지 않는다. 잔량 53건 < 100, 최고 경과 23일 < 30일이라 소진 batch는 열지 않는다
- 모달 전수: `grep -rn "Modal::new"` → 8곳. `ui/app.rs:990`(워크스페이스 삭제) · `ui/remote_menu.rs:315`(이름 입력)·`:369`(권한 변경)·`:441`(같은 이름 확인)·`:521`(원격 삭제) · `ui/remote_states.rs:419`(호스트 키) · `ui/settings_dialog.rs:126`(앱 설정) · `ui/site_manager.rs:527`(사이트 관리자)
- **현행 프레임이 셋으로 갈려 있고 "폭"의 뜻도 갈린다**:
  - ① `Modal` 기본값 — `Frame::popup(style)`이며 여백은 `spacing.menu_margin`(6px), 모서리는 `visuals.menu_corner_radius`(egui 기본 6). 호출부는 `ui.set_width(N)`으로 **본문 폭**만 정한다 (`egui-0.35.0/src/containers/modal.rs:100`, `frame.rs:214-221`, `style.rs:1451`)
  - ② `Frame::popup` + `inner_margin(18)` — 같은 이름 확인·원격 삭제. 역시 `set_width`가 **본문 폭**
  - ③ 손수 구성한 `Frame::new()`(여백 0) + `allocate_exact_size` — 앱 설정(480×560, `corner_radius 6`)·사이트 관리자(1080×680, `corner_radius 0`). 여기서 480·1080은 **프레임 폭**이고 본문 여백은 각자 관리한다(`BODY_PAD_X` 20·18)
- 현행 하단 버튼도 셋으로 갈려 있다: `ui.button` 기본 위젯(app.rs·remote_menu 이름/권한·remote_states) / `widgets::design_button` + `right_to_left`(remote_menu 충돌/삭제·site_manager) / 폭 계산 후 단일 버튼(settings_dialog `:430`)
- `widgets::design_button`은 대화 **밖**에서도 쓰인다 — `ui/remote_states.rs:222·261·350·361`(연결 안내·실패 화면), `ui/site_manager.rs:741`(좌측 목록 버튼). 이 함수는 존치한다. `design_button_width`도 `remote_states.rs:346`이 계속 쓴다
- **`widgets::primary_button`의 호출부는 `site_manager.rs:1198` 하나뿐이다**(`grep -rn primary_button` 전수 — 정의부 `widgets.rs:135` 외). T5가 그것을 평면 버튼으로 바꾸면 함수와 `theme::PRIMARY_TEXT`·`PRIMARY_HOT`·`PRIMARY_BORDER`가 고아가 된다(`PRIMARY_FILL`만 `queue_panel.rs:149`가 계속 쓴다)
- **이 crate는 bin+lib 구성**이라(`src/lib.rs`가 `pub mod ui;` 재수출) `pub` 함수·상수는 호출부가 사라져도 `dead_code` 경고가 나지 않는다 — 실제로 실행 경로에서 쓰이지 않는 Win32 구현(`app/window.rs` 등)이 남은 채 `clippy -D warnings`가 통과한다. 즉 죽은 코드는 **린트가 아니라 이 계획이** 처리해야 한다
- **제거 대상 상수를 기존 시험이 단언한다**: `site_manager.rs:1379-1395`의 `대화_치수는_원본과_같다`가 `FOOTER_HEIGHT`(58)·`FOOTER_BUTTON_HEIGHT`(30)·`FOOTER_BUTTON_PAD_X`(24)를 단언하고, `settings_dialog.rs:504-521`의 `바닥_줄_위에_구분선을_긋는다`가 `show_footer(ui, 자리)`를 직접 호출하며(`:512`), `:523-548`의 `본문이_바닥_버튼_자리를_넘지_않는다`가 `FOOTER_HEIGHT`를 참조한다(`:542`)
- 사이트 관리자 본문에는 `ScrollArea`가 없다(`grep -n ScrollArea src/ui/site_manager.rs` → 0건) — 고정 배치라 본문 높이가 줄면 잘릴 수 있다. 일반 탭 폼은 6행이며 행마다 28+11=39px, 탭 28 + 상단 여백 16을 더해도 약 280px로 새 본문 높이 574px에 여유가 크다
- `egui-0.35`/`epaint-0.35`: `CornerRadius { nw, ne, sw, se }`가 모서리별 u8이라 하단 모서리만 둥글게 그릴 수 있다(`epaint-0.35.0/src/corner_radius.rs:13-25`)
- 글꼴 등록은 `ui/app.rs:144-181`에서 `malgun.ttf` **한 벌만** 올린다. `egui::RichText::strong()`은 `visuals.strong_text_color`만 바꿔 굵어지지 않는다
- `C:\Windows\Fonts\malgunbd.ttf`는 실재하나(12.6MB), 글꼴 카탈로그는 `NAME_ID_FAMILY`(name ID 1)만 읽어 색인하므로(`src/app/fonts.rs:215-241`) `malgun.ttf`와 같은 키 `맑은 고딕`에 묻혀 **먼저 찾은 하나만** 남는다(`:117-121`) — 사용자 지정 글꼴의 굵은 face도 같은 이유로 찾을 수 없다
- 이 레포는 UI 규약을 **AGENTS.md Conventions + `src/ui` 소스 훑기 시험**으로 강제하는 선례를 둘 갖는다 — 아이콘(`widgets.rs:993-1037`)·화면 문구(`src/i18n/mod.rs:870-`)
- PRD 경량 확인: `docs/prd.md`의 FR-21(Should·고정 다크)·FR-27(Must·사이트 관리자 바닥 `연결(C)`·`확인(O)`·`취소`)·FR-39(Should·삭제 확인 대화)·FR-47(Must·설정 바닥 `닫기` 하나)은 **버튼의 존재·라벨·개수**를 규정할 뿐 모서리·배치 같은 시각 세부를 규정하지 않는다. 이번 변경은 그 넷을 모두 보존하므로 PRD에 닿지 않는다 — `**PRD**:` 줄을 두지 않는다

### 전제 검증
| # | 전제 | 확인 근거 (파일:라인·명령) | 결과 |
|---|------|--------------------------|------|
| 1 | 대화 함수의 공개 시그니처를 바꾸지 않고 내부만 교체하면 호출부 영향이 없다 | `grep -rn "show_name_dialog\|show_chmod_dialog\|show_conflict_dialog\|show_delete_confirm"` → 호출부는 `ui/app.rs:1296·1516·1541·1560` 넷뿐이고 모두 반환 타입(`DialogOutcome`)만 소비 | ✅ |
| 2 | `egui`가 모서리별 반경을 지원해 하단 버튼 hover 채움이 둥근 프레임 밖으로 새지 않게 막을 수 있다 | `epaint-0.35.0/src/corner_radius.rs:13-25` | ✅ |
| 3 | 굵은 글꼴 face가 등록돼 있지 않아 `strong()`으로는 굵어지지 않는다 | `ui/app.rs:144-181`(`malgun` 하나만 `font_data.insert`) | ✅ |
| 4 | 굵은 face를 카탈로그에서 이름으로 찾아낼 수 없다 | `src/app/fonts.rs:215-241`(name ID 1만 읽음) + `:117-121`(`or_insert` — 먼저 찾은 것만) | ✅ |
| 5 | `src/ui`에 새 파일을 두면 아이콘·문구 규약 시험이 자동으로 훑는다 | `ui/widgets.rs:1011-1014`·`src/i18n/mod.rs:874-880`(둘 다 `src/ui` 디렉터리 순회) | ✅ |
| 6 | `design_button`·`design_button_width`를 그대로 두어도 대화 밖 화면이 깨지지 않는다 | `grep -rn "design_button"` → 대화 밖 사용처 6곳(`remote_states:222·261·346·350·361`, `site_manager:741`)에는 손대지 않는다 | ✅ |
| 7 | 이번 변경이 PRD의 FR을 건드리지 않는다 | `docs/prd.md` FR-21(Should)·FR-27(Must)·FR-39(Should)·FR-47(Must) 문면 확인(위 Log) | ✅ |
| 8 | 현행 6곳의 360·420·460은 프레임 폭이 아니라 **본문 폭**이다 | `remote_menu.rs:316·370·447·528`·`remote_states.rs:420`·`app.rs:991`의 `ui.set_width` + `egui-0.35.0/src/containers/modal.rs:100`(기본 프레임 = `Frame::popup`, 여백 6) | ✅ |
| 9 | 죽은 `pub` 함수·상수는 `clippy -D warnings`가 잡지 못한다 | `src/lib.rs`(bin+lib 재수출) + 실행 경로에서 쓰이지 않는 `app/window.rs` 등이 남은 채 린트가 통과해 온 사실 | ✅ |
| 10 | 사이트 관리자 하단이 58 → 66px(오류 줄 22 + 버튼 줄 44)로 늘어도 본문이 잘리지 않는다 | 본문에 `ScrollArea` 없음(0건) + 일반 탭 폼 6행 약 280px < 새 본문 높이 574px(680 − 40 − 66) | ✅ |
| 11 | 셸이 푸터를 직접 그리면 5곳이 쓰는 `should_close()` 경로를 보존할 수 있다 | `remote_menu.rs:337·415·495·575`·`settings_dialog.rs:171`·`site_manager.rs:574`가 모두 `Modal` 응답의 그 메서드 하나만 소비 → 셸이 `Shell.should_close`로 되돌려주면 대체 가능 | ✅ |

## 리뷰 이력 (plan-reviewer 3라운드 — 재호출 상한 소진으로 종결)
| 라운드 | 판정 | 처리 |
|---|---|---|
| 1 | BLOCKER 4 · MAJOR 3 · MINOR 5 | 전부 수용·반영. 단 B2의 근거 하나(죽은 `pub` 항목을 린트가 막는다)는 **틀려서 정정**했고 2라운드가 정정이 옳다고 확인했다(전제 검증 #9) |
| 2 | BLOCKER 0 · MAJOR 3 · MINOR 3 | 전부 수용·반영 (M1 `ERROR_ROW_HEIGHT` 구성 · M2 시험 이관 · M3 T5 의존) |
| 3 | BLOCKER 0 · MAJOR 1 · MINOR 1 | **메인이 실물 대조 후 수용·반영** — M1'(4-C 한 행·사전 승인 한 줄에 옛 `FOOTER_HEIGHT 66` 지시가 남음)은 `grep -n "66"`으로 잔존을 확인해 두 줄을 고쳤고, m1'(`design_button` 집계)은 `grep -rn design_button`을 다시 세어 **14 hits 중 `remote_states.rs:219`는 주석이라 실호출 13곳(대화 안 7 / 밖 6)**임을 확인해 정정했다 |

> **종결 방식**: 재호출 상한(2회)을 **수렴이 아니라 예산 소진**으로 마쳤다. 동일 지적 잔존은 0이며(라운드마다 지적이 전부 신규였다), 마지막 라운드의 MAJOR 1·MINOR 1은 위 표대로 메인이 근거를 실물에서 확인해 처리했다.

## Risks & Unknowns
| 위험 | 영향 | 완화책 |
|---|---|---|
| 가짜 굵기(겹쳐 그리기)가 작은 글자에서 뭉개져 보일 수 있다 | 주 버튼 라벨 가독성 | 오프셋을 0.6px로 얕게 두고 T1에서 값을 상수 하나(`FAUX_BOLD_OFFSET`)로 뽑아 화면 확인 후 조정 가능하게 |
| 버튼 라벨이 칸보다 길면 잘린다 | 가장 좁은 칸이 같은 이름 확인의 456÷3 = 152px | 라벨을 칸 중앙에 그리고 칸 안으로 클립한다. `덮어쓰기`·`Overwrite` 모두 152px 안에 든다 — T2에서 두 언어 화면 확인 |
| 여백을 6 → 18로 통일하면 확인 대화 6종의 프레임이 넓어 보인다 | 워크스페이스 삭제 372 → 396px 등(D11 표) | 본문 폭은 그대로라 글줄 접힘이 바뀌지 않는다. 여백이 넓어지는 것은 이미지 스타일에 가까워지는 방향 |
| 사이트 관리자 하단이 58 → 66px(22+44)로 늘어 본문이 8px 준다 | 폼 마지막 행 잘림 | 전제 검증 #10에서 여유(574 대 약 280px)를 확인. T5 Edge Case로 화면 확인 |

## Impact Analysis
### 4-A. 심볼/타입 추적 결과
| 심볼 | 영향 받는 파일 | 영향 종류 |
|---|---|---|
| `egui::Modal::new(...)` 8곳 | `ui/app.rs:990` · `ui/remote_menu.rs:315·369·441·521` · `ui/remote_states.rs:419` · `ui/settings_dialog.rs:126` · `ui/site_manager.rs:527` | 전부 `dialog::show`(6곳)·`dialog::show_fixed`(2곳) 호출로 교체. 이후 `Modal::new`은 `ui/dialog.rs` 안에만 남는다 |
| `DIALOG_MARGIN`·`DIALOG_BUTTON_HEIGHT`·`DIALOG_BUTTON_PAD_X`·`DIALOG_BUTTON_GAP` (`ui/remote_menu.rs:21-27`) | `ui/remote_menu.rs` 내부 4곳(`:444·473-480·525·550-567`) | 제거 — 값이 `dialog` 모듈로 옮겨간다 |
| `SCRIM_ALPHA`·`SHADOW_OFFSET_Y`·`SHADOW_BLUR`·`SHADOW_ALPHA`·`CORNER_RADIUS`·`CLOSE_PAD_X`·`CLOSE_MIN_WIDTH` (`ui/settings_dialog.rs:25-53`) | `ui/settings_dialog.rs:126-140`·`:426-445` | 제거 — `dialog` 모듈이 단일 정본으로 든다 |
| `FOOTER_HEIGHT` (`ui/settings_dialog.rs:33`) | 생산 `:148` · 시험 `:509`·`:542` | 제거 → `dialog::FOOTER_HEIGHT`(44) 사용 |
| `show_footer` (`ui/settings_dialog.rs:422-447`) | 호출 `:166` · 시험 `:512` | **제거** — 셸이 가로 구분선과 버튼을 모두 그리므로 남을 일이 없다. 이 함수를 검사하던 시험은 T1의 `dialog.rs`로 **이관**한다(4-C) |
| `HEADER_HEIGHT` (`ui/settings_dialog.rs:32`) | 생산 `:145` · 시험 `:542` | 존치 — 헤더는 현행 유지(D6) |
| `SCRIM_ALPHA`·`SHADOW_OFFSET_Y`·`SHADOW_BLUR`·`SHADOW_ALPHA` (`ui/site_manager.rs:24-28`) | `ui/site_manager.rs:528-540` | 제거 — 같은 이유 |
| `FOOTER_HEIGHT`(58)·`FOOTER_GAP`·`FOOTER_BUTTON_PAD_X`·`FOOTER_BUTTON_HEIGHT` (`ui/site_manager.rs:111-`) | 생산 `:549`·`:1180-1204` · 시험 `:1386-1389` | **전부 제거** — 푸터 높이는 `dialog::FOOTER_HEIGHT`(44)와 아래 `ERROR_ROW_HEIGHT`(22)로 나뉘고, `show_fixed`가 이미 버튼 줄을 뺀 rect를 주므로 `:549`의 `rect.bottom() - FOOTER_HEIGHT` 계산식 자체가 사라진다 |
| `ERROR_ROW_HEIGHT` (신규, `ui/site_manager.rs`) | 생산 — `show_fixed`가 준 rect의 하단 22px | 신설(D12). **생산 코드가 실제로 쓰는 값**이라 죽은 상수가 되지 않는다(비공개 상수는 `pub` 면제가 없다 — 전제 검증 #9) |
| `FOOTER_PAD_X`(18) (`ui/site_manager.rs`) | 오류 문구 좌측 여백 `:1165` | 제거 → 오류 줄은 본문과 같은 `BODY_PAD_X`(18)를 쓴다. 값이 같아 화면상 자리는 그대로다 |
| `widgets::primary_button` (`ui/widgets.rs:135-153`) | 유일 호출부 `ui/site_manager.rs:1198` | **함수 제거** — T5가 호출부를 없애면 죽은 코드가 된다(bin+lib라 린트가 잡지 못하므로 계획이 처리 — D13) |
| `theme::PRIMARY_TEXT`·`PRIMARY_HOT`·`PRIMARY_BORDER` (`ui/theme.rs:70-72`) | `ui/widgets.rs:144-148`(제거될 `primary_button` 안) | 함께 제거. `PRIMARY_FILL`은 `queue_panel.rs:149`가 쓰므로 존치 |
| `widgets::design_button` · `widgets::design_button_width` | 대화 안 사용처(`remote_menu:475·551·562`, `settings_dialog:430·439`, `site_manager:1182·1190`)에서 호출 제거 | 함수 자체는 **존치** — 대화 밖 6곳(`remote_states:222·261·346·350·361`, `site_manager:741`)이 그대로 쓴다 |
| `ui::dialog` (신규 모듈) | `ui/mod.rs` | `pub mod dialog;` 한 줄 추가 |
| `REMOVE_DIALOG_WIDTH` (`ui/app.rs:65`) | `ui/app.rs:991` | 유지 — 본문 폭 360을 셸에 그대로 넘긴다 |

### 4-B. 계약·직렬화 변경
- 없다. 저장 형식(`settings.json` 스키마 v3)·세션·전송 큐 어디도 팝업 모양을 담지 않는다
- 대화 함수의 공개 시그니처(`show_name_dialog` 등 4개, `SettingsDialog::show`, `SiteManager::show`)는 그대로 둔다(전제 검증 #1)
- `widgets::primary_button` 제거는 `pub` 함수 삭제라 공개 API 축소다 — 사용처가 0이 되므로 계획된 변경으로 사전 승인 항목에 등록

### 4-C. 테스트 파일
| 테스트 | 위치 | 이번 처리 |
|---|---|---|
| `대화_치수는_원본과_같다` | `src/ui/site_manager.rs:1379-1395` | **갱신 (T5)** — `FOOTER_HEIGHT`·`FOOTER_BUTTON_HEIGHT`·`FOOTER_BUTTON_PAD_X` 단언을 **제거**하고 `ERROR_ROW_HEIGHT + dialog::FOOTER_HEIGHT == 66.0` 단언으로 대체한다. **66짜리 상수를 새로 두지 않는다** — 생산 코드에 쓸 자리가 없어 시험만 참조하는 죽은 비공개 상수가 된다(D12 상수 구성) |
| `바닥_줄_위에_구분선을_긋는다` | `src/ui/settings_dialog.rs:504-521` | **이관 (T1 → T4)** — 검사 대상인 `show_footer`가 사라지므로, 같은 판정(버튼 1개일 때 가로선 1·세로선 0)을 **T1이 `dialog.rs`에 먼저 만들고** T4가 원본을 제거한다. 판정이 사라지는 구간이 없다 |
| `본문이_바닥_버튼_자리를_넘지_않는다` | `src/ui/settings_dialog.rs:523-548` | **갱신 (T4)** — `FOOTER_HEIGHT` 참조를 `dialog::FOOTER_HEIGHT`로. 푸터가 58 → 44로 줄어 남는 자리가 14px 늘므로 판정은 그대로 통과한다 |
| `본문이_넘치면_스크롤할_수_있다` | `src/ui/settings_dialog.rs:550-573` | 수정 없음 — `FOOTER_HEIGHT`를 참조하지 않는다(`grep -n FOOTER_HEIGHT` 결과 `:148·:509·:542` 셋뿐) |
| `화면_코드에_원본_아이콘_기호가_남아_있지_않다` | `src/ui/widgets.rs:993-1037` | 수정 없음 — `src/ui` 전체를 훑으므로 신규 `dialog.rs`도 자동 대상. 구분선은 그림이라 기호를 쓰지 않는다 |
| `화면_문구가_카탈로그를_거치지_않은_곳이_없다` | `src/i18n/mod.rs:870-` | 수정 없음 — 같은 이유. `dialog.rs`는 문구를 받아 그리기만 한다 |
| 칸 분할·모서리 배분·규약 시험 | `src/ui/dialog.rs` (신규) | **신설 (T1)** |
| `tests/layout_flow.rs`·`remote_concurrency.rs`·`transfer_memory.rs`·`watcher.rs` | `tests/` | 수정 없음 — 회귀 확인용으로만 실행 |

### 4-D. 재사용 확인
| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `dialog::show` (자동 높이 셸) | `Frame::popup`(remote_menu 2곳) · `Modal` 기본값(4곳) | 신규 — 여섯이 제각각이라 하나로 모으는 것이 이번 요구다. 색(채움·테두리·그림자·스크림)은 현행 값을 그대로 가져온다 |
| `dialog::show_fixed` (고정 크기 셸) | 손수 구성한 `Frame`(settings_dialog:129-140, site_manager:529-540) | 신규 — 두 대화는 헤더·본문·푸터를 스스로 배치하므로 자동 높이 셸로 감당되지 않는다(리뷰 B4) |
| `dialog::footer_slots` / `dialog::slot_corners` (순수 계산) | 없음 | 신규 — 칸 분할·모서리 배분을 순수 함수로 노출해야 단위 시험이 그린 결과를 뒤지지 않고 값을 직접 단언할 수 있다 |
| `dialog::ButtonSpec` (라벨 + 강조 여부) | 없음(`grep -rn "struct.*Button"` → `widgets`에 버튼 타입 없음, 전부 함수) | 신규 — 셸이 버튼 목록을 받으려면 최소한의 자리가 필요하다 |
| `dialog::Shell` (셸 응답) | 없음 | 신규 — `Modal` 응답의 `should_close()`와 눌린 칸 번호를 한 자리에 담아 호출부의 기존 취소 경로를 보존한다(전제 검증 #11) |
| `dialog::faux_bold_text` | 없음(`grep -rn "strong()\|Bold"` → 소스에 굵기 처리 없음) | 신규 — 굵은 face 미등록(전제 검증 #3·#4) |

### Verified by
- `grep -rn "Modal::new"` → 8 hits, 모두 위 표에 포함
- `grep -rn "design_button"` → 정의부 제외 14 hits이며 그중 `remote_states.rs:219`는 주석이라 실호출 13곳. 대화 안 7(`remote_menu:475·551·562`, `settings_dialog:430·439`, `site_manager:1182·1190`) / 대화 밖 6(`remote_states:222·261·346·350·361`, `site_manager:741`)으로 나눠 전부 위 표에 분류
- `grep -rn "primary_button"` → 정의부 제외 1 hit(`site_manager.rs:1198`), 위 표에 포함
- `grep -rn "PRIMARY_FILL\|PRIMARY_HOT\|PRIMARY_BORDER\|PRIMARY_TEXT"` → 정의부 제외 6 hits(`widgets.rs:144·146·147·148`, `queue_panel.rs:149·597`), 전부 위 표에 분류
- `grep -rn "show_name_dialog\|show_chmod_dialog\|show_conflict_dialog\|show_delete_confirm\|settings_dialog\.show\|site_manager\.show"` → 호출부 6곳, 전부 `ui/app.rs`이며 반환값만 소비
- `grep -rn "ui\.button("` → 12 hits 중 대화 안 8곳(`app:998·1001`, `remote_menu:326·332·407·410`, `remote_states:438·441`)이 교체 대상, 나머지 4곳(`queue_panel:482·486`, `sidebar:279`, `titlebar:253`)은 팝업이 아니라 제외
- `grep -rn "should_close()"` → 6 hits, 전부 위 전제 검증 #11에 열거

## 동반 변경 판정
| 구분 | 항목 | 근거 | 처리 |
|---|---|---|---|
| 필수 | `AGENTS.md` Conventions에 "모달은 `ui::dialog`를 거친다" 한 줄 | ② 축(같은 규약을 복제한 지점). "앞으로 추가되는 팝업은 공통"이라는 요구를 README 한 줄로만 두면 다음 작업자가 `Modal::new`을 직접 써도 아무 게이트가 걸리지 않아 요구가 한 회차만 유지된다. 이 레포는 같은 성질의 UI 규약(아이콘·문구)을 AGENTS.md + 소스 훑기 시험으로 강제해 왔다 | T6에 편입 (시험은 T1) |
| 필수 | `README.md`에 팝업 공통 규격 한 줄 + 구조 목록에 `dialog.rs` | ① 축(이 변경을 서술하는 문서). README는 설정 화면·사이트 관리자·삭제 확인 대화를 각각 서술하는데, 이제 그 셋이 **공통 규격**을 공유한다는 사실이 어디에도 없다 | T6에 편입 |
| 필수 | 치수 단언 시험 3건 | ③ 축(이 변경이 입력이 되는 검증 자산). 제거·변경되는 상수를 시험이 직접 단언한다(4-C 표) | T4·T5에 편입 |
| 무관 | `docs/prd.md` | FR-21·FR-27·FR-39·FR-47은 버튼의 존재·라벨·개수만 규정하고 이번 변경이 그것을 전부 보존한다(Investigation Log) | 건드리지 않음 |
| 무관 | `Cargo.toml` | ④ 축(버전·매니페스트) — 의존성이 늘지 않는다(egui 기본 기능만 사용) | 건드리지 않음 |

## Decisions
### D1. 적용 범위
- **Options**: A) 모달 8곳 전부 / B) 확인 대화 6종만 / C) 모달 8곳 + 우클릭 메뉴·토스트
- **Chosen**: A
- **Rationale**: "모든 팝업 동일하게 적용"이라는 요구에 가장 가깝다. 우클릭 메뉴·토스트는 버튼이 없어 하단 버튼 규칙이 성립하지 않는다
- **Source**: 사용자 결정(2026-08-15)

### D2. 하단 버튼 모양
- **Options**: A) 평면 + 구분선(이미지 방식) / B) 현재 채움 버튼을 폭만 균등 분할
- **Chosen**: A — 채움·테두리 없이 라벨만 그리고, 푸터 위에 가로 구분선 1px, 버튼 사이에 세로 구분선 1px. hover에서만 `theme::ROW_HOT` 채움
- **Source**: 사용자 결정(2026-08-15)

### D3. 주 버튼 강조
- **Options**: A) 가짜 굵기(겹쳐 그리기) / B) `malgunbd.ttf`를 별도 글꼴 가족으로 등록 / C) 밝기 차이
- **Chosen**: A — 라벨을 x축으로 `FAUX_BOLD_OFFSET`(0.6px)만큼 밀어 두 번 그린다
- **Rationale**: B는 사용자가 다른 글꼴을 골라도 주 버튼만 맑은 고딕으로 나오고 메모리가 약 12MB 는다(파일 크기 실측). A는 어떤 글꼴이든 그 글꼴로 굵어지고 비용이 없다. 새 푸터를 어차피 `Painter`로 직접 그리므로 자연스럽게 들어간다
- **Source**: 사용자 결정(2026-08-15) + 전제 검증 #3·#4

### D4. 모서리 반경
- **Options**: A) 12px / B) 8px / C) 6px(현행 앱 설정 값)
- **Chosen**: A — 팝업 프레임 네 모서리 12px
- **Source**: 사용자 결정(2026-08-15)

### D5. 사이트 관리자 오류 문구 자리
- **Options**: A) 버튼 줄 바로 위 별도 줄 / B) 본문 맨 아래 / C) 알림 토스트로
- **Chosen**: A — 푸터를 위·아래 두 칸으로 나눠 위칸에 오류 문구, 아래칸에 전폭 버튼
- **Source**: 사용자 결정(2026-08-15)

### D6. 헤더 처리
- **Chosen**: 현행 유지 — 앱 설정·사이트 관리자의 제목 줄과 사이트 관리자의 `✕` 닫기 버튼을 그대로 둔다. 확인 대화 6종의 제목도 왼쪽 정렬 그대로
- **Rationale**: 요구는 모서리와 하단 버튼 둘이다. 헤더까지 바꾸면 변경 범위가 요구를 넘는다
- **Source**: 사용자 결정(2026-08-15)

### D7. 버튼이 3개인 경우
- **Chosen**: 가로 3등분 — 같은 이름 확인(프레임 폭 456 → 칸 152px)·사이트 관리자(1080 → 칸 360px)
- **Rationale**: 세로 스택은 사이트 관리자 하단을 크게 늘린다. 152px면 `덮어쓰기`·`Overwrite` 모두 든다
- **Source**: 사용자 결정(2026-08-15)

### D8. 버튼 좌우 순서
- **Chosen**: 현행 유지 — `[삭제][취소]`(워크스페이스·원격 삭제) · `[확인][취소]`(이름 입력) · `[적용][취소]`(권한 변경) · `[수락][취소]`(호스트 키) · `[덮어쓰기][건너뛰기][취소]`(같은 이름) · `[닫기]`(앱 설정) · `[연결][확인][취소]`(사이트 관리자)
- **Rationale**: 순서를 뒤집으면 지금까지 삭제·덮어쓰기가 있던 자리에 취소가 온다. 사이트 관리자 순서는 PRD FR-27이 `연결(C)`·`확인(O)`·`취소`로 명시한 것과도 맞다
- **Source**: 사용자 결정(2026-08-15)

### D9. 공통 컴포넌트 위치
- **Options**: A) 신규 `src/ui/dialog.rs` / B) `src/ui/widgets.rs`에 추가
- **Chosen**: A
- **Rationale**: `widgets.rs`는 이미 1038줄이고 "작은 조각들"이 책임이다. 대화 셸은 프레임·스크림·본문 여백·푸터가 한 덩어리로 묶인 별개 책임이라, 넣으면 그 파일이 변경 이유를 둘 갖는다(AGENTS.md 분할 판정 ①). 새 파일은 자기 완결적이라 관련 로직이 흩어지지 않는다(④)
- **Source**: AGENTS.md Conventions(파일 1500라인 내외·단일 책임)

### D10. 푸터 높이
- **Chosen**: 44px (`dialog::FOOTER_HEIGHT`) — 버튼 줄 높이의 단일 정본
- **Rationale**: 첨부 이미지에서 버튼 줄 높이가 팝업 폭의 약 16%이며, iOS alert의 실제 규격(폭 270pt · 버튼 44pt)과 같은 비율이다. 현행 값(remote_menu 30 · settings/site 58)이 제각각이라 하나로 모은다
- **Source**: 첨부 이미지 비율 측정

### D11. 셸이 받는 "폭"의 정의와 현행 값 환산
- **Options**: A) 자동 높이 셸은 **본문 폭**을, 고정 크기 셸은 **프레임 크기**를 받는다 / B) 둘 다 프레임 폭으로 통일
- **Chosen**: A
- **Rationale**: 현행 8곳이 이미 그 둘로 갈려 있고(전제 검증 #8), 각자가 이미 가진 값을 그대로 넘길 수 있어야 "본문 폭 변경 없음"이 성립한다. B로 통일하면 여섯 곳이 본문 폭을 프레임 폭으로 손수 환산해야 하고 그 계산이 틀리면 글줄이 접힌다
- **환산표** (셸 여백을 18px로 통일한 결과 — 본문 폭은 전부 보존):

  | 대화 | 셸 함수 | 넘기는 값 | 현행 프레임 폭 | 새 프레임 폭 | 버튼 칸 |
  |---|---|---|---|---|---|
  | 워크스페이스 삭제 | `show` | 본문 360 | 372 (여백 6) | 396 | 2칸 × 198 |
  | 이름 입력 | `show` | 본문 360 | 372 | 396 | 2칸 × 198 |
  | 권한 변경 | `show` | 본문 360 | 372 | 396 | 2칸 × 198 |
  | 같은 이름 확인 | `show` | 본문 420 | 456 (여백 18) | 456 | 3칸 × 152 |
  | 원격 삭제 | `show` | 본문 420 | 456 | 456 | 2칸 × 228 |
  | 호스트 키 | `show` | 본문 460 | 472 (여백 6) | 496 | 2칸 × 248 |
  | 앱 설정 | `show_fixed` | 프레임 480×560 | 480 | 480 | 1칸 × 480 |
  | 사이트 관리자 | `show_fixed` | 프레임 1080×680 | 1080 | 1080 | 3칸 × 360 |
- **Source**: `egui-0.35.0/src/containers/modal.rs:100`·`frame.rs:214-221`·`style.rs:1451` 실측

### D12. 사이트 관리자 푸터 높이
- **Chosen**: 66px = 오류 줄 22 + 버튼 줄 44. 대화 전체 크기(1080×680)와 헤더 40은 그대로이므로 본문이 582 → 574px로 8px 준다
- **상수 구성**: 버튼 줄은 `dialog::FOOTER_HEIGHT`(44)가 정본이고 `show_fixed`가 그만큼 뺀 rect를 준다. 사이트 관리자는 그 rect의 **하단 22px**을 오류 줄로 쓰며 그 값만 자기 상수 `ERROR_ROW_HEIGHT`로 든다 — 66을 상수로 두면 생산 코드가 쓸 자리가 없어 시험만 참조하는 죽은 비공개 상수가 된다(2라운드 리뷰 M1). 시험은 `ERROR_ROW_HEIGHT + dialog::FOOTER_HEIGHT == 66.0`으로 총합을 단언한다
- **Rationale**: 오류 문구는 `FORM_FONT_PX`(13px)라 행간 포함 약 17px이 필요해 22px 칸이면 든다. 본문은 `ScrollArea`가 없어 잘림이 걱정되지만 일반 탭 폼이 약 280px라 여유가 크다(전제 검증 #10)
- **Source**: `widgets.rs:208`(폰트 크기) + `site_manager.rs`의 폼 행 배치 실측

### D13. 죽게 되는 `primary_button` 처리
- **Options**: A) 함수와 고아 색 상수 3개를 제거 / B) 나중에 쓸지 모르니 존치
- **Chosen**: A — `widgets::primary_button`과 `theme::PRIMARY_TEXT`·`PRIMARY_HOT`·`PRIMARY_BORDER`를 제거한다(`PRIMARY_FILL`은 전송 큐가 계속 쓰므로 존치)
- **Rationale**: 이 crate는 bin+lib라 `pub` 항목이 죽어도 린트가 잡지 못한다(전제 검증 #9) — 남기면 아무도 모르는 채 방치된다. AGENTS.md·공통 지침 모두 "나중에 필요할 것 같은 코드"를 금한다. 되살릴 필요가 생기면 git 이력에 있다
- **Source**: `grep -rn primary_button` 전수(호출부 1곳)

### D14. "앞으로 추가되는 팝업도 공통"의 강제 수단
- **Options**: A) AGENTS.md 규약 + `src/ui` 소스 훑기 시험 / B) README 한 줄만 / C) 강제하지 않음
- **Chosen**: A — `ui/dialog.rs` 밖의 `src/ui` 파일에 `Modal::new`이 나타나면 실패하는 시험을 둔다
- **Rationale**: 이 레포는 같은 성질의 규약(아이콘은 phosphor만·문구는 i18n 카탈로그만)을 정확히 그 방식으로 지켜 왔다. 문서 한 줄만으로는 다음 회차에 곧바로 새는 것을 실제로 겪은 규약들이다
- **Source**: `widgets.rs:993-1037` · `src/i18n/mod.rs:870-`의 선례

## 시각 요소 분해

| 요소 | 속성 | 디자인 값 | 확인 방법 |
|------|------|----------|-----------|
| 팝업 프레임 | corner-radius | 12px, 네 모서리 동일 | 첨부 이미지(곡률 약 14px) + D4 |
| 팝업 프레임 | 채움 | `theme::SURFACE_BG` (현행 유지) | 사용자 지시 "색상은 현재 유지" |
| 팝업 프레임 | 테두리 | 1px `theme::BORDER_CONTROL` (현행 유지) | 사용자 지시 |
| 팝업 프레임 | 그림자 | offset [0,18] · blur 60 · 검정 alpha 153 (현행 유지) | `ui/settings_dialog.rs:134-139` |
| 배경 스크림 | 색 | 검정 alpha 140 (현행 유지) | `ui/settings_dialog.rs:127` |
| 본문 | 안쪽 여백 | 18px — 자동 높이 셸이 입힌다. 고정 크기 셸은 호출부가 자체 관리(현행 `BODY_PAD_X` 20·18 유지) | `ui/remote_menu.rs:21` + D11 |
| 본문 | 정렬·글자 크기 | 현행 유지 (왼쪽 정렬, 제목 16px) | D6 |
| 하단 버튼 영역 | 폭 | 팝업 전폭 — 좌우 여백 0, 프레임 테두리에 닿는다 | 첨부 이미지(버튼이 팝업 좌우 끝까지) |
| 하단 버튼 영역 | 높이 | 44px (사이트 관리자는 오류 줄 22px이 위에 더 붙어 푸터 총 66px) | 첨부 이미지 비율 + D10·D12 |
| 하단 버튼 영역 | 상단 경계 | 가로 구분선 1px `theme::BORDER_SUBTLE` | 첨부 이미지 |
| 버튼 사이 | 경계 | 세로 구분선 1px `theme::BORDER_SUBTLE` | 첨부 이미지 |
| 버튼 | 배치 | 버튼 수로 균등 분할 (D11 환산표의 칸 폭) | 첨부 이미지 + D7 |
| 버튼 | 평상시 채움 | 없음 (프레임 배경 그대로) | 첨부 이미지 + D2 |
| 버튼 | hover 채움 | `theme::ROW_HOT` | D2 |
| 버튼 | hover 채움 모서리 | 첫 칸은 좌하단만, 마지막 칸은 우하단만 12px — 버튼 1개면 양쪽 다 | 프레임 모서리 밖으로 새지 않게 |
| 버튼 | 라벨 색 | `theme::TEXT_BUTTON` (현행 유지) | 사용자 지시 |
| 버튼 | 라벨 정렬 | 칸 가운데 | 첨부 이미지 |
| 주 버튼 | 굵기 | 굵게 — x축 0.6px 오프셋 이중 렌더 | 첨부 이미지 + D3 |
| 취소 버튼 | 굵기 | 보통 | 첨부 이미지 |
| 버튼 | 좌우 순서 | 현행 유지 (D8의 7패턴) | D8 |

## Tasks

- [ ] T1. 공통 대화 셸·푸터 모듈 신설 (`src/ui/dialog.rs`)
  - **Type**: D
  - **Design**:
    - ① 배치: 신규 `src/ui/dialog.rs`(D9), `ui/mod.rs`에 `pub mod dialog;` 추가. `ui` 계층 안이며 `app`·`panel`·`fs`를 참조하지 않는다
    - ② 신규 심볼 — **셸이 푸터까지 직접 그린다**(호출부가 전폭 rect를 따로 구할 길이 없으므로):
      - `pub struct ButtonSpec<'a> { label: &'a str, emphasis: bool }`
      - `pub struct Shell { clicked: Option<usize>, should_close: bool }` — `should_close`는 배경 클릭·Esc(현행 `Modal` 응답의 같은 판정)
      - `pub fn show(ctx, id, body_width, buttons, body: impl FnOnce(&mut Ui)) -> Shell` — 자동 높이. 프레임 폭 = `body_width + BODY_MARGIN*2`, 본문 Ui에 여백 18을 셸이 입힌다. 확인 대화 6종용
      - `pub fn show_fixed(ctx, id, frame_size, buttons, content: impl FnOnce(&mut Ui, Rect)) -> Shell` — 고정 크기. `content`에 넘기는 rect는 **푸터를 뺀 나머지**이며 본문 여백은 호출부가 관리한다. 앱 설정·사이트 관리자용
      - `pub fn footer_slots(rect, count) -> Vec<Rect>` · `pub fn slot_corners(index, count, radius) -> CornerRadius` — 순수 계산. 단위 시험이 그린 결과를 뒤지지 않고 값을 직접 단언하도록 노출한다
      - `pub const FOOTER_HEIGHT: f32 = 44.0` · `BODY_MARGIN: i8 = 18`(`egui::Margin::same`이 `i8`만 받는다 — f32가 필요한 자리는 `f32::from`으로 올린다) · `CORNER_RADIUS: u8 = 12` · `FAUX_BOLD_OFFSET: f32 = 0.6`
      - `fn faux_bold_text(painter, pos, text, font, color)` — 굵은 face 없이 굵게 그린다(비공개, 푸터만 쓴다)
      - 사이트 관리자의 오류 줄은 셸이 모르는 그 대화만의 것이므로 `show_fixed`가 넘긴 rect 안에서 호출부가 그린다(D12의 22px을 `content` rect에 포함)
    - ③ 의존 방향: `dialog`는 `theme`만 참조한다. 여덟 대화가 `dialog`를 참조하며 그 역은 없다. `widgets`와는 서로 모른다
    - ④ 비추상화 선언: 대화 종류별 타입(`ConfirmDialog`·`FormDialog`)이나 빌더 패턴을 만들지 않는다 — 여덟 대화의 본문이 제각각이라 공통점은 프레임과 푸터뿐이고, 함수 둘로 두는 것이 실제 동작을 가장 짧은 경로로 보이게 한다
  - **Acceptance**:
    - Given 버튼 2개·프레임 폭 396, When `footer_slots`, Then 두 칸 폭이 같고 합이 정확히 396이며 겹치거나 벌어진 곳이 없다
    - Given 버튼 3개·프레임 폭 456, When `footer_slots`, Then 세 칸이 각각 152이고 합이 456
    - Given 프레임 폭이 버튼 수로 나누어떨어지지 않을 때(예: 폭 397·버튼 2개), When `footer_slots`, Then 칸 합이 정확히 397(나머지 픽셀이 마지막 칸으로)
    - Given 버튼 1개, When `slot_corners(0, 1, 12)`, Then 좌하단·우하단 모두 12, 위 두 모서리 0
    - Given 버튼 3개, When `slot_corners`, Then 0번은 좌하단만 12 · 2번은 우하단만 12 · 1번은 전부 0
    - Given 버튼 1개, When 셸을 그린다, Then 가로 구분선 1개·세로 구분선 0개 (`settings_dialog.rs:504-521`에서 이관해 오는 판정 — 4-C)
    - Given `src/ui`의 모든 파일, When `dialog.rs`를 뺀 나머지에서 `Modal::new`을 찾는다, Then 0건 (D14). **T1 시점에는 T2~T5가 미완이라 이 시험에 `#[ignore]`를 붙여 두고 T5가 뗀다** — 각 task의 `cargo test` 통과와 모순되지 않게 하려는 것이다
    - `cargo test` 통과 · `cargo clippy --all-targets -- -D warnings` 경고 0 · `cargo fmt --check` 무차이
  - **Files**:
    - 주: `src/ui/dialog.rs` (신규)
    - 동반: `src/ui/mod.rs`
    - 테스트: `src/ui/dialog.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 버튼 0개 — 푸터를 그리지 않고 가로 구분선도 두지 않는다
    - 버튼 1개(앱 설정 `닫기`) — 세로 구분선 없이 전폭 한 칸
    - 프레임 폭이 버튼 수로 나누어떨어지지 않을 때 — 나머지 픽셀을 마지막 칸에 몰아 칸 사이 1px 틈이 생기지 않게
    - 라벨이 칸보다 길 때 — 칸 안으로 클립하고 가운데 정렬 유지
    - `body`가 아무것도 그리지 않을 때 — 프레임이 푸터 높이만으로 서고 무너지지 않는다
  - **Halt Forecast**:
    - (i) 자동 높이 대화에서 푸터 rect를 언제 확정하는가 → 셸이 본문을 먼저 그려 높이를 얻은 뒤 푸터를 이어 그린다(Design ②)
    - (ii-a) 신규 파일 추가와 `ui/mod.rs` 모듈 선언 → `## 사전 승인 항목`에 등록
  - **Depends on**: -

- [ ] T2. 원격 대화 4종을 새 셸로 교체 (`ui/remote_menu.rs`)
  - **Type**: D
  - **Design**: 신규 심볼 없음 — 기존 네 함수(`show_name_dialog`·`show_chmod_dialog`·`show_conflict_dialog`·`show_delete_confirm`)의 **내부만** `dialog::show` 호출로 바꾸고 공개 시그니처와 `DialogOutcome`은 그대로 둔다. 파일이 갖던 대화 전용 상수 4개는 `dialog` 모듈로 소유가 옮겨간다(4-A). 새 타입·헬퍼를 이 파일에 두지 않는다 — 넷의 공통 부분은 이미 T1이 들고 있다
  - **Acceptance**:
    - Given 이름 입력·권한 변경·같은 이름 확인·원격 삭제 대화, When 각각을 연다, Then 넷 다 모서리 12px에 전폭 균등 버튼이고 버튼 순서는 D8 그대로다 (HUMAN-VERIFY)
    - `DIALOG_MARGIN`·`DIALOG_BUTTON_HEIGHT`·`DIALOG_BUTTON_PAD_X`·`DIALOG_BUTTON_GAP` 상수가 파일에서 사라지고 `Modal::new` 호출도 남지 않는다 (grep으로 기계 판정)
    - 네 함수의 공개 시그니처와 `DialogOutcome` 반환 규칙(확인/취소/대기 3상태)이 그대로다
    - Esc·바깥 클릭이 종전대로 `Cancelled`를 돌려준다 — `Shell.should_close`가 기존 `response.should_close()` 자리를 대신한다
    - 한국어·영어 두 언어에서 `덮어쓰기`/`Overwrite`가 152px 칸 안에 잘리지 않고 든다 (HUMAN-VERIFY)
    - `cargo test`의 권한 비트 시험 2건이 그대로 통과 (회귀 확인)
  - **Files**:
    - 주: `src/ui/remote_menu.rs`
    - 동반: -
    - 테스트: `src/ui/remote_menu.rs`의 권한 비트 시험 2건 (수정 없음 — 회귀 확인용)
  - **Edge Cases**:
    - 이름 입력 대화의 오류 문구가 본문 안에 남는지 — 푸터로 밀려나지 않게
    - 권한 변경 대화의 체크박스 9개 + 8진 입력이 본문 폭 360 안에서 종전 배치를 유지하는지
    - 같은 이름 확인의 목록 5개 미리보기와 `…` 줄이 본문에 그대로 남는지
  - **Halt Forecast**:
    - (i) 버튼 라벨이 칸을 넘칠 가능성 → T1의 클립 처리로 해결
  - **Depends on**: T1

- [ ] T3. 워크스페이스 삭제·호스트 키 대화 교체 (`ui/app.rs` · `ui/remote_states.rs`)
  - **Type**: C
  - **Acceptance**:
    - Given 워크스페이스 삭제 확인, When 연다, Then 모서리 12px·전폭 `[삭제][취소]` (HUMAN-VERIFY) 이고 Esc가 종전대로 취소로 동작한다
    - Given SFTP 호스트 키 확인, When 연다, Then 모서리 12px·전폭 `[수락][취소]` (HUMAN-VERIFY) 이고 지문이 고정폭 글꼴로 남는다
    - 두 파일에 `Modal::new` 호출이 남지 않는다 (grep으로 기계 판정)
    - 두 대화의 결정 전달 경로(`pending_remove`, `HostKeyDecision`)가 바뀌지 않는다 — 취소는 종전대로 `Reject`로 매핑된다
  - **Files**:
    - 주: `src/ui/app.rs` (`:990-1006`), `src/ui/remote_states.rs` (`:419-445`)
    - 동반: -
    - 테스트: -
  - **Edge Cases**:
    - 워크스페이스 삭제 대화가 떠 있는 동안 목록이 바뀌어 대상이 사라지는 기존 처리(`ui/app.rs:985-988`)를 건드리지 않는다
    - 호스트 키 대화에서 `Changed` 경고 문구가 길어 본문이 늘어나도 푸터가 본문 아래에 그대로 붙는다
    - `remote_states.rs`의 대화 밖 `design_button` 4곳(`:222·261·350·361`)은 손대지 않아 연결 안내·실패 화면이 종전 모양 그대로다
  - **Halt Forecast**:
    - (i) 호스트 키 대화는 `Option<HostKeyDecision>`을 돌려주고 배경 클릭에 대한 기존 처리가 없다 → 셸의 `should_close`를 `Reject`로 매핑해 종전 동작(대화 유지)이 아니라 거절이 되지 않도록, 현행과 같이 **버튼을 눌러야만** 결정이 나가게 유지한다
  - **Depends on**: T1

- [ ] T4. 앱 설정 대화 푸터 교체 (`ui/settings_dialog.rs`)
  - **Type**: C
  - **Acceptance**:
    - Given 앱 설정 대화, When 연다, Then 모서리 12px이고 하단 `닫기`가 전폭 한 칸이다 (HUMAN-VERIFY)
    - 본문 스크롤 영역이 종전대로 제목·`닫기` 사이에서만 스크롤한다(둘은 제자리)
    - `CORNER_RADIUS`·`SCRIM_ALPHA`·`SHADOW_*`·`CLOSE_PAD_X`·`CLOSE_MIN_WIDTH`·`FOOTER_HEIGHT` 상수와 `show_footer` 함수가 파일에서 사라지고 `Modal::new` 호출도 남지 않는다 (grep으로 기계 판정)
    - `바닥_줄_위에_구분선을_긋는다`는 **T1이 `dialog.rs`에 만든 동등 판정으로 이미 대체돼 있으므로** 여기서 제거한다 — 판정이 비는 구간이 없다(4-C)
    - `본문이_바닥_버튼_자리를_넘지_않는다`가 `dialog::FOOTER_HEIGHT`(44) 참조로 갱신돼 통과하고, `본문이_넘치면_스크롤할_수_있다`는 손대지 않아도 그대로 통과한다
    - PRD FR-47의 "바닥 버튼은 `닫기` 하나"가 그대로다
  - **Files**:
    - 주: `src/ui/settings_dialog.rs` (`:126-172`, `:426-445`, 상수 `:25-53`)
    - 동반: -
    - 테스트: `src/ui/settings_dialog.rs:504-521` 제거(T1로 이관 완료) · `:523-548` 갱신
  - **Edge Cases**:
    - 본문이 창보다 길 때 스크롤바가 푸터를 침범하지 않는지 — 본문 rect를 `show_fixed`가 넘긴 rect로 한정한다
    - 푸터가 58 → 44로 줄어 본문 자리가 14px 늘어난다 — `본문이_바닥_버튼_자리를_넘지_않는다`가 더 여유롭게 통과할 뿐 판정이 뒤집히지 않는다
    - 즉시 저장 동작(확인 버튼 없음)이 그대로인지
  - **Halt Forecast**:
    - (i) 이 대화는 `allocate_exact_size`로 크기를 스스로 잡는다 → `show_fixed`가 프레임 크기(480×560)를 받아 푸터를 뺀 rect를 넘기므로 헤더·본문 계산만 그 rect 기준으로 다시 잡는다
  - **Depends on**: T1

- [ ] T5. 사이트 관리자 푸터 교체와 오류 문구 재배치 (`ui/site_manager.rs` · `ui/widgets.rs` · `ui/theme.rs`)
  - **Type**: D
  - **Design**:
    - ① 배치: 오류 줄 높이 상수 `ERROR_ROW_HEIGHT: f32 = 22.0`을 `site_manager.rs`의 기존 상수 구역(`:111` 부근)에 둔다 — 이 대화만의 값이라 `dialog`로 올리지 않는다(다른 일곱 대화에는 오류 줄이 없다)
    - ② 신규 심볼과 책임: `ERROR_ROW_HEIGHT` 하나뿐. `show_fixed`가 준 content rect의 하단 22px이 오류 줄이고 그 위가 본문이라는 사실을 이름으로 남긴다
    - ③ 의존 방향: `site_manager`가 `dialog`를 참조한다. 반대는 없다. `widgets::primary_button` 참조가 끊기며 그 함수와 색 상수 3개가 제거된다(D13)
    - ④ 비추상화 선언: 오류 줄을 `dialog` 셸의 기능(예: `show_fixed`에 `error: Option<&str>` 인자 추가)으로 올리지 않는다 — 여덟 대화 중 하나만 쓰는 것을 공통 API에 넣으면 나머지 일곱이 매번 `None`을 적게 된다
  - **Acceptance**:
    - Given 사이트 관리자, When 연다, Then 모서리 12px(종전 0)이고 하단이 `[연결][확인][취소]` 3등분(칸 360px)이다 (HUMAN-VERIFY)
    - Given 저장 실패로 오류가 있는 상태, When 화면을 본다, Then 오류 문구가 버튼 줄 **바로 위 22px 줄**에 남고 버튼과 겹치지 않는다 (HUMAN-VERIFY — D5·D12)
    - `대화_치수는_원본과_같다`가 `ERROR_ROW_HEIGHT + dialog::FOOTER_HEIGHT == 66.0` 단언으로 갱신돼 통과하고, `FOOTER_HEIGHT`·`FOOTER_BUTTON_HEIGHT`·`FOOTER_BUTTON_PAD_X` 단언이 사라진다 (D12 상수 구성)
    - `widgets::primary_button`과 `theme::PRIMARY_TEXT`·`PRIMARY_HOT`·`PRIMARY_BORDER`가 제거되고, `theme::PRIMARY_FILL`은 남아 `queue_panel` 시험이 그대로 통과한다 (D13)
    - `src/ui`에서 `dialog.rs` 밖의 `Modal::new`이 0건이 되어 T1의 규약 시험에서 `#[ignore]`를 떼도 통과한다 (D14)
    - PRD FR-27의 버튼 3종과 라벨이 그대로다
    - 좌측 목록의 `이름 바꾸기(R)`·`삭제(D)`·`복제(I)` 버튼(`:741`)은 손대지 않아 종전 모양 그대로다
  - **Files**:
    - 주: `src/ui/site_manager.rs` (`:527-553`, `:1150-1215`, 상수 `:24-28`·`:111`·푸터 상수)
    - 동반: `src/ui/widgets.rs`(`primary_button` 제거 `:134-153`), `src/ui/theme.rs`(`PRIMARY_TEXT`·`PRIMARY_HOT`·`PRIMARY_BORDER` 제거 `:70-72`), `src/ui/dialog.rs`(규약 시험 `#[ignore]` 해제)
    - 테스트: `src/ui/site_manager.rs:1379-1395` 갱신
  - **Edge Cases**:
    - 오류 문구가 길어 22px 줄을 넘칠 때 — 넘치는 부분은 잘라 두 줄이 되지 않게(줄 높이 고정). 좌측 여백은 본문과 같은 `BODY_PAD_X`(18)를 쓴다 — 제거되는 `FOOTER_PAD_X`와 값이 같아 자리가 그대로다
    - 오류가 없을 때 그 줄이 빈 채로 자리만 차지하는 것이 어색하지 않은지 (HUMAN-VERIFY)
    - 본문이 582 → 574px로 8px 주는데 일반 탭 폼 6행(약 280px)·고급 탭이 잘리지 않는지 (HUMAN-VERIFY — `ScrollArea`가 없다)
    - `연결` 버튼이 종전 초록 채움이었다 — 평면으로 바뀌므로 `emphasis: true`(굵게)로 표시해 주 동작임을 남긴다
  - **Halt Forecast**:
    - (i) 푸터가 두 줄이 되어 높이가 는다 → D12에서 66px로 확정하고 대화 전체 크기(1080×680)는 그대로 둔다
    - (ii-a) `widgets::primary_button`(`pub` 함수) 제거 → `## 사전 승인 항목`에 등록
  - **Depends on**: T1, T2, T3, T4 — 규약 시험(`Modal::new` 0건)이 `src/ui` **전체**를 훑으므로 네 대화가 모두 교체된 뒤에야 `#[ignore]`를 뗄 수 있다

- [ ] T6. README·AGENTS.md에 팝업 공통 규격 기술
  - **Type**: A
  - **Acceptance**:
    - `AGENTS.md` Conventions에 "모달 대화는 `ui::dialog`의 셸을 거친다 — `Modal`을 직접 쓰지 않으며 규약은 `dialog.rs`의 시험이 지킨다"는 취지의 한 줄이 있다 (아이콘·화면 문구 항목과 같은 형식)
    - `README.md`에 모든 모달 팝업이 같은 규격(둥근 모서리 12px·전폭 균등 하단 버튼)을 쓰고 그 규격이 `src/ui/dialog.rs` 한 곳에 있다는 서술이 있다
    - `README.md` 디렉터리 구조 목록에 `dialog.rs` 한 줄이 추가된다
    - 존재하지 않는 기능을 적지 않는다 (커스텀 타이틀바 절의 "창 그림자·둥근 모서리는 없습니다"는 **창**에 대한 서술이라 그대로 둔다)
  - **Files**:
    - 주: `README.md`, `AGENTS.md`
    - 동반: -
    - 테스트: -
  - **Edge Cases**: 해당 없음 (문서만 — 동작·빌드에 닿지 않는다)
  - **Halt Forecast**:
    - 없음 — 문서 두 파일의 추가 서술뿐이라 파괴적·외부·의존성 요소가 없다
  - **Depends on**: T1, T2, T3, T4, T5

## 사전 승인 항목 (일괄 승인 대상)
- T1 — 신규 파일 `src/ui/dialog.rs` 추가와 `src/ui/mod.rs`의 `pub mod dialog;` 선언 (계획된 구조 변경)
- T2·T4·T5 — 대화 전용 상수 제거·변경(`remote_menu`의 `DIALOG_*` 4개 · `settings_dialog`의 `CORNER_RADIUS`·`SCRIM_ALPHA`·`SHADOW_*`·`CLOSE_*`·`FOOTER_HEIGHT` · `site_manager`의 `SCRIM_ALPHA`·`SHADOW_*`와 푸터 상수 5개(`FOOTER_HEIGHT`·`FOOTER_PAD_X`·`FOOTER_GAP`·`FOOTER_BUTTON_HEIGHT`·`FOOTER_BUTTON_PAD_X`) 제거 및 `ERROR_ROW_HEIGHT`(22) 신설). 모두 모듈 내부(`const`, 비공개)라 외부 계약이 아니다
- T5 — `widgets::primary_button`(`pub` 함수)과 `theme::PRIMARY_TEXT`·`PRIMARY_HOT`·`PRIMARY_BORDER`(`pub` 상수) 제거 (D13 — 호출부가 0이 되며 bin+lib라 린트가 잡지 못한다)
- T4·T5 — 기존 시험 처리: `대화_치수는_원본과_같다`·`본문이_바닥_버튼_자리를_넘지_않는다`는 **새 규격 값으로 갱신**, `바닥_줄_위에_구분선을_긋는다`는 **T1이 `dialog.rs`에 동등 판정을 먼저 만든 뒤 원본을 제거**하는 이관이다. 셋 어느 것도 판정이 사라지지 않는다
- T4 — `settings_dialog::show_footer` 함수 제거 (셸이 그 일을 대신하므로 호출부가 0이 된다)
- T6 — `AGENTS.md` Conventions에 모달 규약 한 줄 추가 (D14)

## 불가피한 Halt (위임 불가)
- master 브랜치 push·태그·릴리즈 — 구현·검증이 끝난 뒤 별도 승인
- PRD `docs/prd.md` 문면 변경 — 이번 계획은 PRD에 닿지 않지만(동반 변경 판정), 구현 중 닿는 것이 드러나면 그 자리에서 멈추고 승인받는다
- 시험을 **대체 없이 삭제**하는 선택 — 위 사전 승인은 갱신과 이관(동등 판정을 먼저 만든 뒤 원본 제거)까지다. 판정 자체를 없애야 한다는 판단이 서면 멈추고 승인받는다

## Verification Strategy
- 빌드: `cargo build`
- 린트: `cargo clippy --all-targets -- -D warnings` (경고 0)
- 서식: `cargo fmt --check`
- 테스트: `cargo test`
- 규약: `src/ui`에서 `dialog.rs` 밖의 `Modal::new`이 0건 (T1이 넣는 시험이 자동 판정 — T5부터 활성)
- 수동 검증 (HUMAN-VERIFY — 빌드로 판정 불가):
  1. `cargo run --release`로 앱을 띄운다
  2. 워크스페이스 삭제 · 원격 이름 바꾸기 · 권한 변경 · 원격 삭제 · 같은 이름 확인 · 호스트 키 확인 · 앱 설정 · 사이트 관리자 8개 팝업을 차례로 연다
  3. 각각에서 확인한다 — 네 모서리가 둥근가 · 하단 버튼이 좌우 끝까지 닿고 균등한가 · 구분선이 보이는가 · 주 버튼이 굵은가 · hover 채움이 둥근 모서리 밖으로 새지 않는가
  4. 사이트 관리자에서 이름을 비운 채 `확인`을 눌러 오류 문구가 버튼 줄 위에 뜨는지, 폼 마지막 행이 잘리지 않는지 본다
  5. 설정에서 언어를 영어로 바꾸고 같은 이름 확인 대화를 다시 열어 `Overwrite`가 칸에 드는지 본다

## Phase Ledger

## Retry Ledger

## Progress Log

## Next Steps

## Open Questions
- [x] Q1: 적용 범위 → 모달 8곳 전부 (D1)
- [x] Q2: 하단 버튼 모양 → 평면 + 구분선 (D2)
- [x] Q3: 주 버튼 강조 → 가짜 굵기 (D3)
- [x] Q4: 모서리 반경 → 12px (D4)
- [x] Q5: 사이트 관리자 오류 문구 자리 → 버튼 줄 위 별도 줄 (D5)
- [x] Q6: 헤더 처리 → 현행 유지 (D6)
- [x] Q7: 버튼 3개 배치 → 가로 3등분 (D7)
- [x] Q8: 버튼 좌우 순서 → 현행 유지 (D8)
