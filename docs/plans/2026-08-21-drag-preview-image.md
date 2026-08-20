# Plan: 드래그 중 미리보기 그림

**PRD**: `docs/prd.md`

## 요구 이해

- **원문 요청**: "남은 이슈 중 드래그 미리보기 그림을 계획해 줘"
- **이해한 요구**: 앱에서 탐색기·바탕화면으로 파일을 끌 때(FR-61 ⓑ 내보내기) 지금은 커서 모양만 바뀌고 **끌고 있는 것이 무엇인지 보이지 않는다**. 끄는 동안 그 항목의 반투명 그림이 커서를 따라오게 한다. 대상은 `docs/plans/deferred.md`의 2026-08-20 항목 「드래그 중 미리보기 그림」이며, 그 항목이 적어 둔 「착수 전 확인된 것」 ⓐ~ⓓ를 출발점으로 삼는다.
- **승인 질의로 확정된 것**: ① 그림은 **항목의 실제 셸 썸네일/아이콘**(일반 그림이 아니다) ② 여러 개를 끌면 **첫 항목의 그림 하나만** ③ 드롭 설명 문구(`복사 → 바탕 화면`)를 **켠다** ④ 셸 조회는 **캐시에 있으면 썸네일, 없으면 형식 아이콘**(디스크를 새로 읽어 썸네일을 만들지 않는다).
- **포함하지 않는 것으로 이해**: 앱 **안**에서 탭↔탭으로 끌 때의 그림(그 경로는 egui가 쥐고 있어 셸 헬퍼와 무관하다)·원격 항목 끌어내기·아이콘 여러 장 겹쳐 쌓기·이동(잘라내기) 효과.

## Goal

`fs::drag_source`가 `DoDragDrop`을 부르기 전에 셸의 드래그 이미지 관리자에 **첫 항목의 그림**을 얹어, 앱에서 탐색기로 끄는 동안 그 그림과 드롭 설명 문구가 보이게 한다. 그림을 얻지 못해도 드래그 자체는 종전과 똑같이 된다.

## PRD Coverage

| PRD ID | 우선순위 | 대응 task | 상태 |
|--------|---------|----------|------|
| FR-61 ⓑ (문면 개정 — 내보내기 드래그에 미리보기 그림) | Should | T1·T2·T3 | ✅ 커버 |
| 그 밖의 active Must/Should FR | — | — | 이번 범위 외 (기구현) |

## Out of Scope

- **앱 안 드래그(탭↔탭·로컬↔원격)의 미리보기 그림**. 그 경로는 창을 벗어나기 전까지 egui가 쥐고 있어(`ui::app::pump_export_drag`) 셸 헬퍼가 관여하지 않는다. 화면 표시를 바꾸려면 egui 쪽 그리기를 새로 만들어야 하며 이번 요구와 다른 작업이다.
- **아이콘 여러 장을 겹쳐 쌓고 개수 배지를 그리는 것**(2026-08-21 사용자 결정 — 첫 항목 한 장).
- **이동·링크 효과의 그림**. 이 앱의 드래그는 언제나 복사다(PRD 라인 116).

## Deferred / Follow-up

<!-- 아래 항목의 `docs/plans/deferred.md` 등재와, 이번에 해소하는 대장 항목(2026-08-20 「드래그 중 미리보기 그림」)의 종결 표시는 `pjc:implement-task` Phase F-6.5가 맡는다. -->

- **원격 항목을 탐색기로 끌어내기** — 대장의 2026-08-20 항목 그대로 유지. 이번 변경은 로컬 항목 경로에만 닿는다.
- **[SUGGEST] 프리멀티플라이 알파 되돌리기가 세 곳이 된다** — `fs::thumbnail::bitmap_to_rgba`·`ui::icon_tex::color_bitmap_to_image`에 이어 `fs::drag_image`가 세 번째다. `thumbnail.rs:322`의 주석이 「세 번째 사용처가 생기면 공통 위치를 찾는다」고 이미 예고했다. **이번엔 공통화하지 않는다** — 기존 둘은 RGBA `Vec`을 돌려주는데 이번 것은 BGRA를 DIB 섹션 메모리에 직접 쓰므로, 합치려면 세 곳의 반환 형태를 함께 손봐야 해 이번 범위의 몇 배다(4-D 참조).
- **미리보기 그림의 DPI 재조회** — 그림 크기는 끌기를 시작하는 순간의 배율로 한 번 정한다. 끄는 도중 다른 배율의 모니터로 넘어가도 그림은 그대로다(셸의 드래그 이미지가 원래 그렇게 동작한다 — 다시 얹을 통로가 없다).

## Investigation Log

- **지금은 그림을 얹는 코드가 아예 없다**: `src/fs/drag_source.rs:57`이 `DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect)`를 그대로 부르고, 그 앞뒤에 드래그 이미지 관리자를 만드는 곳이 없다(`grep -rn "IDragSourceHelper\|DragDropHelper" src/` → 0건).
- **호출부는 한 곳뿐이다**: `start_copy_drag`는 `src/ui/app.rs:1758`(`pump_export_drag`)에서만 불린다. 그 밖의 hit 둘은 정의(`drag_source.rs:35`)와 자기 시험(`:147`)이다 — grep 3건 전건 확인.
- **대장이 적어 둔 전제 ⓒ가 코드에서 그대로 확인된다**: `CopyDragSource::GiveFeedback`이 `DRAGDROP_S_USEDEFAULTCURSORS`를 돌려준다(`drag_source.rs:89`). 셸이 그린 드래그 이미지가 보이려면 이 값이어야 한다.
- **필요한 바인딩이 전부 실재하고 feature도 이미 켜져 있다** — 의존성 변경이 없다:
  - `IDragSourceHelper`(`windows-0.62.2/src/Windows/Win32/UI/Shell/mod.rs:19025`)·`IDragSourceHelper2`(`:19088`)·`CLSID_DragDropHelper`(`:6580`)·`DSH_ALLOWDROPDESCRIPTIONTEXT`(`:7828`)·`SHDRAGIMAGE`(`:54165`).
  - `InitializeFromBitmap`은 `Win32_Graphics_Gdi` + `Win32_System_Com`을 요구하고(`:19028`의 `cfg`) `InitializeFromWindow`는 `Win32_System_Com`만 요구한다(`:19035`). **셋 다 `Cargo.toml`에 이미 있다**(`Win32_UI_Shell`·`Win32_System_Com`·`Win32_Graphics_Gdi`).
  - `CreateDIBSection`(`Graphics/Gdi/mod.rs:242`)·`CoCreateInstance`(레포 선례 `src/fs/file_dialog.rs:32`, `CLSCTX_INPROC_SERVER`).
- **그림을 얻는 길이 레포에 이미 있다**: `src/fs/thumbnail.rs:300 make_thumbnail`이 `SHCreateItemFromParsingName` → `IShellItemImageFactory::GetImage(size, SIIGBF_*)`로 `HBITMAP`을 얻고(`:312`), `:325 bitmap_to_rgba`가 `GetObjectW` → `GetDIBits` → **프리멀티플라이 되돌리기**까지 한다(`:383`의 `unmul`). 이번 모듈은 이 절차를 같은 순서로 따르되 산출물이 다르다(아래 4-D).
- **셸 조회 플래그가 전부 실재한다**: `SIIGBF_INCACHEONLY`(`Shell/mod.rs:55215`)·`SIIGBF_ICONONLY`(`:55214`)·`SIIGBF_RESIZETOFIT`(`:55217`).
- **MS 문서 확인 ①** — `IDragSourceHelper` 인터페이스 Remarks: 드래그 이미지 관리자는 `CoCreateInstance(CLSID_DragDropHelper)`로 만들고, **헬퍼가 데이터 객체에 `IDataObject::SetData`로 사설 형식을 싣는다**("To support the drag-and-drop helper object, the data object's SetData and GetData implementations must be able to accept and return arbitrary private formats"). 대장 항목 ⓓ가 가리킨 바로 그 요구다.
- **MS 문서 확인 ②** — `InitializeFromBitmap` Remarks: "you should always pass a bitmap **without premultiplied alpha blending**" — 넘기면 한 번 더 곱해 알파가 두 배가 된다. `GetImage`가 주는 비트맵은 프리멀티플라이일 수 있으므로(`thumbnail.rs:382`의 주석과 같은 판단) **되돌린 뒤** 넘겨야 한다.
- **MS 문서 확인 ③** — `SHDRAGIMAGE` 문서에 `hbmpDragImage`의 **소유권 서술이 없다**(멤버 설명은 "The drag image's bitmap handle."뿐이고 Remarks는 만드는 절차만 적는다). 지우는 쪽이 누구인지 문서로 정해지지 않는다 → D6.
- **MS 문서 확인 ④** — `SHDoDragDrop`은 hwnd가 NULL이면 셸이 **일반(generic) 그림**을 준다("the Shell provides a generic drag image"). 항목별 그림이 아니므로 이번 선택(항목의 실제 썸네일)에는 쓰지 않는다. 바인딩은 `Shell/mod.rs:2974`에 실재한다.
- **DPI를 아는 곳은 `ui`뿐이다**: `fs`는 배율을 모르고(`grep -rn "pixels_per_point" src/fs/` → 0건) `ui::app`의 `ctx`가 안다. 그래서 그림의 물리 픽셀 크기는 호출부가 정해 내려보낸다(D5).
- 위키 참조: `20_projects/personal/moa/feat-shell-integration.md` — 셸 연동 정리 페이지이나 **드래그 내보내기(FR-61)·미리보기 그림 서술이 없다**(2026-08-20 회차가 아직 반영되지 않았다). 각주의 소스 포인터가 `src/fs/thumbnail.rs`를 가리켜 셸 그림 조회가 그 파일에 있다는 것만 확인된다.
- 위키 참조: `20_projects/personal/moa/decisions.md` — 드래그 미리보기 그림을 과거에 기각·보류한 결정이 **없다**(`드래그|drag|썸네일|미리보기` 조회, 관련 결정 0건).
- Deferred 대장 조회(`docs/plans/deferred.md`, `## 대기` 70건(실측 — 항목 72줄 중 해소 표시 2건 제외)): **반증 1건** — 「egui의 끌기 판정을 시험에서 재현하는 방법」(2026-08-18, 두 차례 실측 실패)이라 **드래그의 화면 동작은 자동 시험으로 고정할 수 없다**. 이 plan은 그 경계를 지켜 픽셀 변환·크기 계산 같은 순수 로직만 시험으로 묶고 그림이 실제로 보이는지는 수동 검증에 둔다. 할 일 후보로는 이 계획의 대상 항목(2026-08-20 「드래그 중 미리보기 그림」)이 걸렸다. 잔량 70건은 소진 batch 임계(100건) 미만이고, 가장 오래된 항목의 날짜는 2026-07-23(29일 전)이라 30일 임계에도 닿지 않아 **batch task를 넣지 않는다**.

### 전제 검증

| # | 전제 | 확인 근거 | 상태 |
|---|------|----------|------|
| 1 | 드래그 이미지 관리자와 관련 형식이 `windows` crate에 있다 | `Shell/mod.rs:19025`·`:19088`·`:6580`·`:7828`·`:54165` 직접 확인 | ✅ |
| 2 | 새 crate·새 feature가 필요 없다 | `InitializeFromBitmap`의 `cfg`가 요구하는 `Win32_Graphics_Gdi`·`Win32_System_Com`이 `Cargo.toml`에 이미 있다 | ✅ |
| 3 | 지금 코드가 헬퍼의 전제를 갖췄다 | `GiveFeedback`이 `DRAGDROP_S_USEDEFAULTCURSORS` 반환(`drag_source.rs:89`) | ✅ |
| 4 | 항목의 그림을 얻는 셸 호출이 레포에서 이미 돈다 | `thumbnail.rs:302~312`(`SHCreateItemFromParsingName` → `GetImage`) | ✅ |
| 5 | 넘기는 비트맵은 프리멀티플라이가 아니어야 한다 | MS 문서 `InitializeFromBitmap` Remarks | ✅ |
| 6 | 프리멀티플라이를 되돌리는 절차가 레포에 있다 | `thumbnail.rs:369~386` | ✅ |
| 7 | 호출부가 한 곳뿐이라 시그니처를 바꿔도 파급이 좁다 | `grep -rn start_copy_drag src/` 3건 전건 확인(정의·시험·`ui/app.rs:1758`) | ✅ |
| 8 | **셸이 만들어 준 `IDataObject`가 임의 사설 형식의 `SetData`를 받는다** | 헬퍼가 그것을 **요구한다**는 것은 MS 문서로 확인했으나, `SHCreateShellItemArrayFromIDLists` → `BindToHandler(BHID_DataObject)`가 준 객체가 실제로 받아 주는지는 **실행해 봐야 안다** | ⚠ 미확인 — **성립을 좌우하지 않는다**: 실패하면 `InitializeFromBitmap`이 오류를 돌려주고 그림 없이 종전대로 끈다(D7). 수동 검증 1로 판정 |
| 9 | `hbmpDragImage`를 누가 지우는가 | MS 문서에 서술이 없다(확인 ③) | ⚠ 미확인 — D6으로 안전한 쪽을 택하고 수동 검증 5(GDI 개체 수 실측)로 판정 |
| 10 | 드래그의 화면 동작은 자동 시험으로 고정할 수 없다 | Deferred 대장 2026-08-18 항목(두 차례 실측 실패) | ✅ |

## Risks & Unknowns

- **R1. 그림이 아예 안 붙을 수 있다** (전제 8). 셸 데이터 객체가 사설 형식을 거부하면 그렇다. 그때도 드래그·복사는 종전대로 되며 그림만 없다. 수동 검증 1에서 바로 드러난다.
- **R2. GDI 핸들 누수** (전제 9). 소유권이 문서에 없어, 지우지 않는 쪽을 택하면 헬퍼도 안 지울 때 끌 때마다 비트맵 한 장이 남는다. 수동 검증 5로 20회 실측해 계단식 증가가 있으면 D6을 뒤집는다.
- **R3. UI 스레드 셸 호출 지연**. 그림 조회는 `DoDragDrop`과 같은 자리(UI 스레드)에서 돈다. `SIIGBF_INCACHEONLY` → 실패 시 `SIIGBF_ICONONLY` 순서라 **디스크에서 썸네일을 새로 만들지 않으므로** 형식 아이콘 조회 수준의 비용이다(`fs::icons`가 같은 성질의 호출을 이미 UI 스레드에서 한다). AGENTS의 「UI 스레드 블로킹 금지」가 겨냥하는 매 프레임 경로가 아니라, 이미 예외로 명시된 `start_copy_drag`의 셸 조회와 같은 자리다.

## Impact Analysis

### 4-A. 심볼/타입 추적 결과

| 심볼 | hit | 확인 결과 |
|------|-----|----------|
| `start_copy_drag` | 3 | 정의 `fs/drag_source.rs:35` · 시험 `:147` · 호출 `ui/app.rs:1758`. **시그니처를 바꾸면 고칠 곳은 그 셋뿐이다**(T3) |
| `IDragSourceHelper`·`DragDropHelper` | 0 | 레포에 선례 없음 — 신규 도입 |
| `bitmap_to_rgba` | 2 | `thumbnail.rs` 안에서만 쓰이는 사적 함수(모듈 밖 노출 없음). 이번 변경이 건드리지 않는다 |
| `SIIGBF_RESIZETOFIT` | 2 | `thumbnail.rs:19`(import)·`:312`(호출). 이번 모듈은 다른 플래그 조합을 쓰므로 그 상수 자체는 그대로다 |
| `start_copy_drag` (레포 전역 — `src/` 밖) | 2 | `AGENTS.md:98`과 `docs/plans/2026-08-20-drag-copy-and-transfer-relist.md:281`. 앞의 것은 **UI 스레드 블로킹 예외 열거가 이 함수의 셸 호출을 `SHParseDisplayName`으로 특정해** 이번 변경으로 낡는다(동반 변경 필수, T3). 뒤의 것은 **지난 회차의 기록 문서라 갱신 대상이 아니다** |

### 4-B. 계약·직렬화 변경

- `fs::drag_source::start_copy_drag`에 인자 하나(`preview_px: i32`)가 는다 — **crate 안 공개 함수이고 호출부가 한 곳**이다. 직렬화·세션 스키마·설정 파일과 무관하다.
- 새 모듈 `fs::drag_image`는 crate 밖으로 나가지 않는다(단일 crate 바이너리).

### 4-C. 테스트 파일

- `src/fs/drag_source.rs`의 `mod tests` 2건(`끌_것이_없으면_시작하지_않는다`·`읽지_못한_경로는_목록에서_빠진다`) — 첫 번째가 `start_copy_drag(&[])`를 부르므로 **인자 추가에 맞춰 고쳐야 한다**(T3의 Files에 포함).
- 새 시험은 `src/fs/drag_image.rs`의 `mod tests`에 둔다(레포 관례 — 단위는 `#[cfg(test)] mod tests`).
- 통합 테스트(`tests/`)에는 드래그 관련 파일이 없다(`grep -rn "drag" tests/` → 0건).

### 4-D. 재사용 확인

| 신규 심볼 | 유사 기존 구현 검색 결과 | 재사용/신규 사유 |
|---|---|---|
| `fs::drag_image::build` | `fs::thumbnail::make_thumbnail`(`thumbnail.rs:300`) — 같은 `SHCreateItemFromParsingName` → `GetImage` 순서 | **신규.** 호출 순서는 그대로 따르되 산출물이 다르다 — 그쪽은 egui 텍스처용 RGBA `Vec`을 돌려주고 이번 것은 셸에 넘길 GDI `HBITMAP`을 돌려준다. 게다가 그쪽은 워커 스레드 전용 캐시 경로(`ThumbnailCache`)의 안쪽 사적 함수라 UI 스레드에서 부를 통로가 없다 |
| `fs::drag_image`의 프리멀티플라이 되돌리기 | `thumbnail.rs:369~386` · `ui::icon_tex.rs:256`(`color_bitmap_to_image`) | **신규(부분 중복 인정).** 세 번째 사용처이며 `thumbnail.rs:322`의 주석이 이 시점을 예고했다. 다만 기존 둘은 RGBA `Vec`을 만들고 이번 것은 BGRA를 DIB 섹션 메모리에 직접 쓴다 — 합치려면 세 곳의 반환 형태를 통일해야 해 이번 범위의 몇 배다. `## Deferred / Follow-up`에 `[SUGGEST]`로 남긴다 |
| `DragImage`(반환 타입) | 레포에 드래그 이미지를 다루는 타입 없음(hit 0) | **신규.** `HBITMAP` 하나와 크기를 함께 들어야 `SHDRAGIMAGE`를 채울 수 있다 |
| 드래그 이미지 관리자 생성 | `fs::file_dialog.rs:32`·`fs::file_op.rs:123`의 `CoCreateInstance` | **패턴 재사용.** 같은 형태(`CoCreateInstance(&CLSID, None, CLSCTX_INPROC_SERVER)`)를 따른다 |

### Verified by

- `grep -rn "start_copy_drag\|IDragSourceHelper\|DragDropHelper\|bitmap_to_rgba" src/` (전건 Read)
- `windows-0.62.2` 소스 직접 확인(위 라인 번호들)
- MS Learn 문서 4건(`IDragSourceHelper`·`InitializeFromWindow`·`InitializeFromBitmap`·`SHDRAGIMAGE`·`SHDoDragDrop`)

## 동반 변경 판정

| 축 | 발견 | 구분 | 처리 |
|---|---|---|---|
| ① 문서 | PRD FR-61 ⓑ가 내보내기 동작을 서술하는데 미리보기 그림이 없다 | **필수** | T1 |
| ① 문서 | `README.md:26`의 「끌어다 놓아 복사」가 내보내기를 서술한다 | **필수** | T4 |
| ① 문서 | `AGENTS.md:98`의 UI 스레드 블로킹 예외 열거가 이 함수의 셸 호출을 `SHParseDisplayName` 하나로 특정한다 — 같은 자리에서 `SHCreateItemFromParsingName`+`GetImage`가 더 돌게 되므로 열거가 실제와 어긋난다. R3이 **바로 그 문면에 기대어** 새 호출을 정당화하므로 고치지 않으면 근거가 자기모순이 된다 | **필수** | T3 |
| ⑤ 무효화 | `drag_source.rs`의 모듈 주석 「직접 만드는 것은 `IDropSource` 하나」와 `GiveFeedback`의 「우리가 그릴 것이 없으므로 기본을 쓴다」가 **거짓이 된다** | **필수** | T3 |
| ③ 검증 자산 | `drag_source.rs`의 시험 `끌_것이_없으면_시작하지_않는다`가 바뀐 시그니처로 깨진다 | **필수** | T3 |
| ② 규약 복제 | 아이콘·문구·팝업 규약의 소스 훑기 시험 — 새 아이콘 기호도 새 화면 문구도 만들지 않는다(드롭 설명 문구는 윈도우가 그린다) | 무관 | 건드리지 않는다 |
| ④ 매니페스트 | `Cargo.toml` — 필요한 feature가 전부 켜져 있음을 `cfg` 대조로 확인했다(전제 2). 이 프로젝트에는 릴리즈·태그 규약이 없다(AGENTS: CI/CD 없음, 단일 exe) | 무관 | 건드리지 않는다 |
| ⑤ 무효화 | `fs::thumbnail`·`ui::icon_tex` — 이번 모듈은 그 둘을 부르지 않고 그 둘도 이번 변경을 모른다 | 무관 | 건드리지 않는다 |

## Decisions

### D1. 그림의 출처 — 첫 항목의 셸 그림 한 장
- **Options**: ⓐ 항목의 실제 썸네일/아이콘 ⓑ `SHDoDragDrop`의 일반 그림 ⓒ 만들지 않음
- **Chosen**: ⓐ (2026-08-21 사용자 결정). 여러 개를 끌면 **목록의 첫 항목** 하나로 그린다.
- **Source**: 대장 항목의 문면(「끌고 있는 항목의 반투명 썸네일」) + 사용자 승인 질의.

### D2. 셸 조회 플래그 — 캐시에 있으면 썸네일, 없으면 아이콘
- **Chosen**: `GetImage(size, SIIGBF_INCACHEONLY)`를 먼저 부르고, 실패하면 `GetImage(size, SIIGBF_ICONONLY)`로 되돌린다(2026-08-21 사용자 결정).
- **근거**: 두 갈래 모두 디스크에서 썸네일을 새로 만들지 않아 UI 스레드가 멎지 않는다(R3). 이미 본 사진·동영상은 진짜 미리보기가 뜬다.

### D3. 드롭 설명 문구를 켠다
- **Chosen**: 헬퍼를 `IDragSourceHelper2`로 얻어 `SetFlags(DSH_ALLOWDROPDESCRIPTIONTEXT)`를 부른다. `IDragSourceHelper2`를 얻지 못하면 `IDragSourceHelper`로 진행한다(문구만 없다).
- **Source**: 사용자 결정 + `Shell/mod.rs:7828`(상수 실재). 문구는 윈도우가 자기 언어로 그리므로 **i18n 카탈로그에 더할 것이 없다**.
- **주의**: 바인딩의 `SetFlags`가 `u32`를 받으므로 `DSH_ALLOWDROPDESCRIPTIONTEXT.0 as u32`로 넘긴다(`Shell/mod.rs:19097` — `dwflags: u32`).

### D4. 배치 — 신규 모듈 `src/fs/drag_image.rs`
- **Options**: ⓐ `drag_source.rs`에 함께 둔다 ⓑ 새 모듈로 가른다
- **Chosen**: ⓑ. AGENTS의 분할 판정 ①(변경 이유가 둘 이상)이 「예」다 — 드래그 프로토콜(언제 그만둘지·어떤 효과인지)과 그림 만들기(셸 조회·GDI 비트맵)는 바뀌는 이유가 다르다. ④(분리하면 관련 로직이 흩어지는가)는 「아니오」 — 그림 만들기는 자족적이다.

### D5. 그림 크기는 호출부가 정해 내려보낸다
- **Chosen**: `start_copy_drag(paths: &[PathBuf], preview_px: i32)`로 인자를 하나 늘리고, `ui::app::pump_export_drag`이 `(96.0 * ctx.pixels_per_point()).round() as i32`를 넘긴다.
- **근거**: `fs`는 화면 배율을 모른다(Investigation Log). 96 논리 픽셀은 탐색기의 드래그 이미지와 비슷한 크기이며, 배율 200%면 192 물리 픽셀을 청한다. 셸이 그보다 작은 그림밖에 없으면 있는 것을 준다(`SIIGBF_RESIZETOFIT`은 요청 크기 **안으로** 맞춘다 — `thumbnail.rs:310`의 주석과 같은 성질).

### D6. 비트맵 소유권 — 성공하면 지우지 않는다
- **Options**: ⓐ 넘긴 뒤 항상 지운다 ⓑ 성공하면 지우지 않고 실패했을 때만 지운다
- **Chosen**: ⓑ. 문서에 소유권 서술이 없다(전제 9). 잘못 지우면 재활용된 GDI 핸들을 남의 것과 함께 지우게 되어 원인 추적이 어려운 고장이 나고, 지우지 않으면 최악이라도 **끌 때마다 비트맵 한 장**이라 실측으로 잡힌다. 수동 검증 5에서 계단식 증가가 관측되면 ⓐ로 뒤집는다.
- **실패 경로의 해제 근거**: **얹지 못한 모든 경로**에서 지운다 — `InitializeFromBitmap`이 오류를 돌려준 경우뿐 아니라 **그 호출에 닿기 전에 그만두는 경우**(헬퍼 생성 실패 등)도 포함한다. 얹지 못했다는 것은 소유권이 넘어가지 않았다는 뜻이며, 오류 반환 뒤에도 소유권을 가져가는 COM 메서드는 관행에 어긋나므로 이 가정을 채택한다(문서에 서술이 없다는 점은 전제 9와 같다). 애초에 그 갈래가 생기지 않도록 **헬퍼를 먼저 만들고 그 다음에 그림을 만든다**(T3 Design ①).

### D7. 실패는 조용한 저하
- **Chosen**: 그림을 못 얻었거나 헬퍼 생성·`InitializeFromBitmap`이 실패하면 **아무것도 알리지 않고** 종전대로 `DoDragDrop`을 부른다. 상태 줄 알림도 로그도 남기지 않는다.
- **근거**: 미리보기 그림은 장식이고, 못 붙었다고 복사를 막을 이유가 없다. 이 앱은 GUI라 `println!`도 금지다(AGENTS DO NOT).

### D8. `SHDRAGIMAGE`의 남은 두 칸 — 커서는 그림 한가운데, 색 키는 쓰지 않는다
- **Chosen**: `ptOffset = (width/2, height/2)`(커서가 그림 한가운데에 놓인다) · `crColorKey = COLORREF(0xFFFF_FFFF)`(`CLR_NONE` — `Win32/UI/Controls/mod.rs:1838`에 `-1i32`로 실재한다).
- **근거**: 32bpp 알파 비트맵을 넘기므로 투명은 알파가 정하고 색 키로 뚫을 것이 없다. 커서 자리는 끌기를 **시작한 지점의 항목 안 좌표를 알 수 없어**(`start_copy_drag`은 경로 목록만 받는다) 한가운데가 치우침 없는 기본값이다. 구현자가 임의로 정하면 그림이 커서에서 어긋나 보인다.

## Tasks

- [x] T1. PRD FR-61 ⓑ에 미리보기 그림 한 구 더하기
  - **Type**: A
  - **Acceptance**: Given `docs/prd.md:90`의 FR-61, When ⓑ 내보내기 서술을 읽으면, Then **끄는 동안 첫 항목의 그림과 드롭 설명 문구가 보인다**는 것과 **그림을 얻지 못해도 복사는 그대로 된다**는 것이 적혀 있다. 검증 방법 칸에는 그림이 HUMAN-VERIFY임을 명시한다. FR-61의 다른 문면(ⓐ 받기·로컬 항목 한정)은 한 글자도 바뀌지 않는다.
  - **Files**:
    - 주: `docs/prd.md`
  - **Edge Cases**: 해당 없음 (문서)
  - **Halt Forecast**:
    - (ii-b) 수동 검증 1에서 그림이 전혀 붙지 않으면(전제 8 부정) 이 문면은 **없는 기능을 정본에 적은 것이 된다** → `## 불가피한 Halt` (T4와 같은 위험을 공유한다). 그 밖에는 없음 — 문서 문면 수정이며 파괴적·외부 요소가 없다(PRD 개정 승인은 `## 사전 승인 항목`에 있다)
  - **Depends on**: -

- [ ] T2. `fs::drag_image` — 셸에서 그림을 얻어 GDI 비트맵으로 만든다
  - **Type**: C
  - **Design**: ① **배치** — 신규 `src/fs/drag_image.rs`, `src/fs/mod.rs`에 `pub mod drag_image;` 등록(D4). ② **신규 심볼** — `DragImage { bitmap: HBITMAP, width: i32, height: i32 }`(셸에 넘길 비트맵과 그 크기) · `pub fn build(path: &Path, px: i32) -> Option<DragImage>`(경로 하나의 그림을 만든다) · `impl DragImage { pub fn delete(self) }`(실패 경로에서 되돌린다 — `SHDRAGIMAGE`를 채우는 것은 이 모듈이 아니라 `drag_source`이며 그 값은 D8이 정한다) · `fn unpremultiply(pixels: &mut [u8])`(BGRA 버퍼를 제자리에서 스트레이트 알파로 되돌리는 **순수 함수** — 시험 대상). ③ **의존 방향** — `windows`와 `std`만 안다. `fs::thumbnail`·`ui`를 참조하지 않고 아무도 이 모듈을 참조하지 않는다(T3에서 `drag_source`가 부른다). ④ **비추상화 선언** — 캐시를 두지 않는다(끌 때마다 한 번 조회) · 여러 항목 합성·개수 배지를 만들지 않는다 · 워커 스레드로 미루지 않는다(`DoDragDrop`이 UI 스레드에서 마우스를 쥐므로 그 자리에서 끝나야 한다) · 기존 두 곳의 픽셀 변환과 공통화하지 않는다(4-D).
  - **Acceptance**:
    - Given 알파를 쓰는 프리멀티플라이 BGRA 버퍼, When `unpremultiply`를 부르면, Then 알파가 0인 픽셀은 색까지 0이 되고 그 밖의 픽셀은 색이 `c*255/a`로 되돌아가며 알파 값 자체는 바뀌지 않는다.
    - Given 알파 채널이 전부 0인 버퍼(알파를 쓰지 않는 비트맵), When `unpremultiply`를 부르면, Then 모든 픽셀이 불투명(알파 255)이 되고 색은 그대로다 — `thumbnail.rs:369`와 같은 규칙이다.
    - Given COM을 초기화한 시험 스레드와 실재하는 파일 하나, When `build(그 경로, 96)`을 부르면, Then `Some`이 오고 `width`·`height`가 둘 다 1 이상 96 이하이며, 돌려받은 비트맵을 `delete`로 지울 수 있다.
    - Given 실재하지 않는 경로, When `build`를 부르면, Then `None`이고 패닉하지 않는다.
    - `cargo clippy --all-targets -- -D warnings` 경고 0.
  - **Files**:
    - 주: `src/fs/drag_image.rs` (신규)
    - 동반: `src/fs/mod.rs` (모듈 등록 1줄)
    - 테스트: `src/fs/drag_image.rs`의 `#[cfg(test)] mod tests`
  - **Edge Cases**:
    - 빈 경로·없는 파일·권한 없는 경로 → `None`(D7이 받는다).
    - `px`가 0 이하 → 조회하지 않고 `None`(셸에 0 크기를 청하지 않는다).
    - 셸이 요청보다 작은 그림을 준다 → 그대로 쓴다(`SHDRAGIMAGE`에 실제 크기를 적는다).
    - `GetImage`가 8/24bpp 비트맵을 준다 → `GetDIBits`에 32bpp를 청하므로 항상 32bpp로 받는다(`thumbnail.rs:341`과 같은 방식).
    - 시험 스레드의 COM 초기화 — `CoInitializeEx(COINIT_APARTMENTTHREADED)`를 시험 안에서 부르고 끝에 `CoUninitialize`한다(`fs::file_op`의 워커가 같은 형태를 쓴다).
  - **Halt Forecast**:
    - (i) `GetImage`가 `SIIGBF_INCACHEONLY`로 늘 실패해 아이콘만 뜬다 → 결함이 아니다. D2가 그 폴백을 설계로 정했고 수동 검증 2가 그 경로를 본다.
    - (ii-a) 신규 모듈 추가(구조 변경) → `## 사전 승인 항목`
  - **Depends on**: -

- [ ] T3. `fs::drag_source`가 끌기 전에 그림을 얹는다
  - **Type**: C
  - **Design**: ① **배치·순서** — `src/fs/drag_source.rs`의 `start_copy_drag` 안, `DoDragDrop` 바로 앞. 순서는 **헬퍼 생성 → 문구 플래그 → 그림 만들기 → `InitializeFromBitmap`**이다(헬퍼를 먼저 얻어야, 그림을 만들어 놓고 헬퍼가 없어 버리는 갈래가 생기지 않는다 — D6). ② **신규 심볼** — 없다(기존 함수에 인자 하나와 얹는 절차가 는다). ③ **의존 방향** — `fs::drag_source` → `fs::drag_image`(같은 계층, 단방향). `ui::app`은 종전대로 `drag_source`만 안다. ④ **비추상화 선언** — 헬퍼를 감싸는 래퍼 타입·트레이트를 만들지 않는다(쓰는 곳이 한 곳이다).
  - **Acceptance**:
    - Given 로컬 항목 여럿을 끄는 상황, When `start_copy_drag(paths, px)`가 돌면, Then `CoCreateInstance(&CLSID_DragDropHelper)`로 얻은 헬퍼에 **첫 항목의 그림**을 `InitializeFromBitmap`으로 얹은 뒤 `DoDragDrop`을 부른다(D1).
    - **코드 확인으로 판정**(실패를 실행 중에 강제할 수단이 없다 — 전제 10): 헬퍼 생성·`build`·`InitializeFromBitmap` 세 갈래의 실패가 **모두 같은 지점으로 합류해** 종전과 같은 인자의 `DoDragDrop` 한 줄에 닿는다(조기 `return`이 없다). 그 경로에 알림·로그·`println!`이 없다(D7). **얹지 못한 모든 경로**에서 `delete`를 부른다(D6).
    - **코드 확인으로 판정**: `IDragSourceHelper2`를 얻는 곳이 `cast`/질의 실패를 받아 `IDragSourceHelper`로 잇는 분기를 갖는다 — 그 갈래에서는 `SetFlags`를 건너뛰고 `InitializeFromBitmap`은 그대로 부른다(D3).
    - Given `start_copy_drag(&[], px)`, When 부르면, Then 종전처럼 `false`이고 COM에 닿지 않는다 — 기존 시험이 인자 추가에 맞춰 고쳐진 채 통과한다.
    - 모듈 주석의 「직접 만드는 것은 `IDropSource` 하나」와 `GiveFeedback`의 「우리가 그릴 것이 없다」가 **새 동작에 맞게 고쳐져 있다**(옛 문면이 남아 있지 않다 — `grep`으로 확인).
    - `ui::app::pump_export_drag`이 `(96.0 * ctx.pixels_per_point()).round() as i32`를 넘긴다(D5).
    - **코드 확인으로 판정**: `SHDRAGIMAGE`의 네 칸이 D8대로 채워져 있다 — `sizeDragImage`는 `build`가 돌려준 실제 크기, `ptOffset`은 `(width/2, height/2)`, `crColorKey`는 `COLORREF(0xFFFF_FFFF)`.
    - `AGENTS.md:98`의 UI 스레드 블로킹 예외 열거에 **셸 그림 조회가 더해져 있다** — 「`SHParseDisplayName`」만 적힌 옛 문면이 남아 있지 않다(`grep`으로 확인).
    - `cargo test` 전건 통과 · `cargo clippy --all-targets -- -D warnings` 경고 0.
  - **Files**:
    - 주: `src/fs/drag_source.rs`
    - 동반: `src/ui/app.rs` (`pump_export_drag`의 호출 1곳) · `AGENTS.md` (98번 줄의 예외 열거 한 구)
    - 테스트: `src/fs/drag_source.rs`의 `mod tests` 2건(첫 번째가 인자 추가 대상)
  - **Edge Cases**:
    - 첫 항목의 그림만 실패하고 나머지는 성공 → **다른 항목으로 갈아타지 않는다**(D1이 「첫 항목」으로 못박았다). 그림 없이 끈다.
    - `DoDragDrop`이 취소로 끝남 → 반환 판정은 종전 그대로(`DRAGDROP_S_DROP`만 참).
    - 헬퍼가 데이터 객체에 `SetData`를 걸지 못함(전제 8) → `InitializeFromBitmap`이 오류를 돌려주고 D7 경로로 간다.
    - 같은 드래그를 연달아 여러 번 시작 → 매번 헬퍼를 새로 만들고 함수를 벗어날 때 `Drop`으로 놓는다(전역 상태를 두지 않는다).
  - **Halt Forecast**:
    - (i) 그림을 얹은 뒤 `DoDragDrop`의 중첩 메시지 루프가 달라진다 → 달라지지 않는다. 얹는 것은 데이터 객체에 형식을 싣는 일이고 루프를 시작하는 것은 종전과 같은 `DoDragDrop` 한 줄이다.
    - (ii-a) `start_copy_drag`의 시그니처 변경(호출부 1곳) → `## 사전 승인 항목`
    - (ii-b) 수동 검증에서 창이 굳거나 앱이 패닉하면 plan에 없던 방향 전환이다 → `## 불가피한 Halt`
  - **Depends on**: T2

- [ ] T4. README 갱신
  - **Type**: A
  - **Acceptance**: Given `README.md:26`의 「끌어다 놓아 복사」 항목, When 창 밖으로 끌어내는 서술을 읽으면, Then **끄는 동안 첫 항목의 그림과 `복사 → 대상` 설명 문구가 커서를 따라온다**는 것이 적혀 있고, 그 절의 다른 문장은 바뀌지 않았다. 존재하지 않는 기능(개수 배지·앱 안 드래그의 그림)을 적지 않는다.
  - **Files**:
    - 주: `README.md`
  - **Edge Cases**: 해당 없음 (문서)
  - **Halt Forecast**:
    - (ii-b) 수동 검증 1에서 그림이 전혀 붙지 않으면(전제 8 부정) 이 문면은 **없는 기능을 적은 것이 된다** → `## 불가피한 Halt`
  - **Depends on**: T3

## 사전 승인 항목 (일괄 승인 대상)

- T2 — 신규 모듈 `src/fs/drag_image.rs` 추가와 `src/fs/mod.rs` 등록(구조 변경).
- T3 — `fs::drag_source::start_copy_drag`의 시그니처에 `preview_px: i32` 추가(호출부 1곳 · 4-A로 전수 확인).
- T1 — `docs/prd.md` FR-61 ⓑ 문면 개정(요구사항 정본 수정).
- T3 — `AGENTS.md:98`의 UI 스레드 블로킹 예외 열거에 셸 그림 조회를 더한다(프로젝트 가이드 정본 수정 — 이 plan 승인이 `pjc:record-project-fact`의 개별 승인을 갈음하므로, **더할 문구를 여기 그대로 적어 승인 시점에 보이게 한다**).
  - 지금: `` **앱→탐색기 드래그 내보내기**(`fs::drag_source::start_copy_drag`의 `SHParseDisplayName` — 끄는 항목 수만큼 셸 네임스페이스를 조회한다. …) ``
  - 개정: 같은 괄호 안에 한 구를 더한다 — `` …의 `SHParseDisplayName`**과 첫 항목의 미리보기 그림 조회**(`fs::drag_image` — `SIIGBF_INCACHEONLY` → `ICONONLY` 순으로 물어 **디스크에서 썸네일을 새로 만들지 않는다**) — 끄는 항목 수만큼 셸 네임스페이스를 조회한다. … ``
  - 그 줄의 나머지(세션 저장·사이트 목록 내보내기·OS 드롭 받기 서술)는 바뀌지 않는다.

## 불가피한 Halt (위임 불가)

- 수동 검증에서 `DoDragDrop` 경로가 창을 굳히거나 앱을 패닉시키는 경우 — plan에 없던 방향 전환이므로 사용자에게 보고하고 지시를 받는다(그림 붙이기를 되돌릴지, 다른 초기화 방식으로 갈지).
- **수동 검증 1에서 그림이 전혀 붙지 않는 경우**(전제 8이 부정된 것 — 셸 데이터 객체가 사설 형식의 `SetData`를 받지 않는다). 그러면 T2·T3은 그대로 성립하지만 **T1(PRD)·T4(README)가 없는 기능을 적은 것이 되므로** 그 두 문면을 되돌리고 사용자에게 보고해 지시를 받는다(데이터 객체를 직접 구현하는 길로 갈지, Deferred로 되돌릴지 — plan에 없던 방향 전환이다).
- commit·push·병합·태그·릴리즈·PR.

## Verification Strategy

- 빌드: `cargo build`
- 서식·린트: `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings`
- 단위 테스트: `cargo test`
- **수동 검증** (드래그의 화면 동작은 자동 시험으로 재현할 수 없다 — 전제 10):
  1. 사진이 든 폴더를 큰 아이콘 보기로 열어 썸네일이 다 뜬 뒤, 사진 하나를 바탕화면으로 끈다 → **그 사진의 반투명 그림**이 커서를 따라오고 `복사 → 바탕 화면` 문구가 뜬다. (그림이 아예 없으면 전제 8이 부정된 것이다 — R1.)
  2. 자세히 보기에서 텍스트 파일 하나를 끈다(썸네일 캐시에 없다) → **형식 아이콘**이 뜬다.
  3. 여러 개를 골라 끈다 → **첫 항목의 그림 하나**가 뜨고, 놓으면 고른 것 전부가 복사된다.
  4. 끄는 도중 `Esc`를 누른다 → 그림이 사라지고 아무것도 복사되지 않는다.
  5. 작업 관리자 `세부 정보` 탭에 `GDI 개체` 열을 켜고 **20회 끌었다 놓기** → 개체 수가 계단식으로 늘지 않는다(D6·R2 판정. 늘면 D6을 ⓐ로 뒤집는다).
  6. 원격 탭의 항목을 골라 창 밖으로 끈다 → 종전대로 아무 일도 일어나지 않는다(내보내기 대상이 아니다).

## Phase Ledger

## Retry Ledger

## Progress Log

## Next Steps

- 승인되면 `pjc:implement-task`로 T1부터 순서대로 실행한다.

## Open Questions

- [x] 그림의 종류 → **항목의 실제 썸네일/아이콘**(D1, 2026-08-21)
- [x] 여러 개를 끌 때 → **첫 항목 한 장**(D1, 2026-08-21)
- [x] 드롭 설명 문구 → **켠다**(D3, 2026-08-21)
- [x] 셸 조회 비용 → **캐시에 있으면 썸네일, 없으면 아이콘**(D2, 2026-08-21)
