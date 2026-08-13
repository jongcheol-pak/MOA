# Debug: 앱 실행 시 창이 깜빡이면서 화면이 표시된다

## Symptom

앱을 시작하면 화면이 한 번 번쩍인 뒤 앱 화면이 나타난다. 사용자 보고(2026-08-13).

## Reproduction

1. 창을 **최대화한 상태로** 앱을 종료한다(세션에 `window.maximized = true`가 저장된다).
2. 앱을 다시 실행한다.
3. 앱 화면이 뜨기 직전 **흰 사각형**이 잠깐 보였다가 사라진다.

계측은 화면을 60~100ms 간격으로 연속 캡처해(PowerShell + `CopyFromScreen`) 프레임 간
픽셀 차이를 재는 방식으로 했다. 창 생성·첫 프레임 시각은 임시 프로브로 파일에 기록했다
(조사 후 제거).

## Phase 1 — Evidence

- 에러·스택트레이스 없음(시각 결함).
- 타이밍 계측(`ExplorerApp::new` 진입 = 0 기준):
  - `run_native` 호출 → `new` 진입까지 약 1.08초(창 생성·GL 초기화 구간)
  - 첫 프레임 `logic`은 `new` 완료 +11ms
- 화면 캡처(같은 기준):
  - **t = -223ms에 흰 사각형 등장**(크기·위치가 세션에 저장된 일반 창 사각형과 일치)
  - t = -143ms, -72ms 페이드 아웃 → t = -7ms 사라짐
  - t = +294ms 앱 화면(다크) 표시
- 실패 레이어: **창 생성 단계**(우리 코드가 아니라 eframe/winit 경로)

## Phase 2 — Hypotheses

- H1: 창이 보인 뒤 첫 프레임까지 비어 있어 흰 화면이 노출된다 — 예측: eframe이 창을 미리
  보이게 한다 — 검증: eframe 소스 → **기각**. `glow_integration.rs:165`가 `with_visible(false)`로
  숨겨 만들고 `epi_integration.rs:326`이 첫 프레임을 그린 뒤 `set_visible(true)`를 한다.
- H2: 복원 위치 보정(`restore_window` → `clamp_window`)이 첫 프레임에 창을 옮긴다 —
  예측: 저장 위치가 화면 밖일 때만 — 검증: 관측된 위치가 화면 안이고 첫 프레임부터
  사각형이 변하지 않음 → **기각**
- H3: 최대화 복원이 창 생성 단계에서 창을 드러낸다 — 예측: `with_maximized(true)`를 빼면
  흰 사각형이 사라진다 — 검증: 실험 빌드 → **확정**(창 생성 전후 프레임 차이 전부 0.00)

## Phase 3 — Root Cause

`ViewportBuilder::with_maximized(true)`를 주면 winit가 창을 만든 **직후**
`set_maximized(true)`를 부르고(`winit-0.30.13/src/platform_impl/windows/window.rs:1402`),
그것이 `ShowWindow(hwnd, SW_MAXIMIZE)`로 이어진다(`window_state.rs:363`).
**`SW_MAXIMIZE`는 창을 표시한다** — eframe이 흰 화면을 막으려고 숨겨 둔 창이 이 순간
강제로 드러나고, 아직 GL이 아무것도 내보내지 않아 창 클래스의 기본(흰색) 배경 브러시가
그대로 보인다.

같은 결함에 **두 번째 경로**가 있었다(1차 수정 뒤 재계측에서 드러남): 최대화를 첫 프레임의
`logic`에서 걸면, eframe이 그 프레임을 그려 창을 보이게 하기 *전에* `SW_MAXIMIZE`가 먼저
창을 드러낸다. 그리고 창이 보인 뒤 최대화로 크기가 바뀌는 순간에도 아직 그리지 않은 자리가
흰색으로 칠해진다.

## Phase 4 — Fix

- Change:
  - `src/main.rs` — 창 생성 시 `with_maximized`를 주지 않는다. 대신 세션이 최대화였으면
    처음부터 **그 자리 모니터의 작업 영역**만 하게 띄운다(크기가 튀지 않게).
  - `src/ui/window_start.rs` (신규) — 시작 사각형 결정(순수)과 작업 영역 조회(Win32).
  - `src/ui/app.rs` — `restoring_maximized` 카운터. **첫 프레임이 아닌 다음 프레임부터**
    `ViewportCommand::Maximized(true)`를 걸고, 최대화가 실제로 걸릴 때까지 관측 사각형을
    저장하지 않는다(되돌릴 일반 크기가 작업 영역 크기로 덮이지 않게).
  - `src/app/theme.rs` — 창 클래스 배경 브러시를 앱 배경색으로(`paint_unpainted_as_window_bg`).
    아직 그리지 않은 자리가 흰색 대신 창 배경색이 되어 표시·리사이즈 순간에도 색이 이어진다.
- Test added: `src/ui/window_start.rs`의 단위 테스트 3건(최대화 여부별 시작 사각형,
  작업 영역을 모를 때의 폴백). 창 표시 타이밍 자체는 자동 테스트 대상이 아니라
  위 재현 절차로 수동 확인한다.

## Verification

- Build: OK (경고 0)
- Tests: 621/621 통과
- Lint: `cargo clippy --all-targets -- -D warnings` 통과, `cargo fmt --check` 통과
- 수동 재현: 최대화 상태로 재시작해 연속 캡처 — **흰 화면 프레임 없음**. 화면이 한 번에
  다크 앱 화면으로 나타난다(수정 전에는 흰 사각형이 약 0.2~0.3초 보였다).
