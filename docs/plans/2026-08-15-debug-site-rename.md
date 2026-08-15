# Debug: 사이트 관리자에서 항목 이름을 바꿔도 저장되지 않는다

## Symptom
사이트 관리자(FR-27)의 `이름 바꾸기(R)`로 목록 항목 이름을 고쳐도 바뀐 이름이 남지 않는다.
대화를 다시 열면 옛 이름 그대로다.

## Reproduction
1. 사이드바 → `사이트 관리자` 열기
2. 목록에서 사이트를 고르고 `이름 바꾸기(R)` 클릭 → 행이 편집기로 바뀐다
3. 이름을 고친 뒤 **Enter를 누르지 않고** `확인(O)`(또는 `연결(C)`·`X`·대화 바깥)을 누른다
4. 대화를 다시 열면 이름이 옛것으로 돌아가 있다

## Phase 1 — Evidence
- 에러·스택 없음(조용한 유실) → 계층별 증거로 실패 지점을 좁혔다
- 저장소 계층 ✅ — `SiteStore::rename`(`remote/sites.rs:75`)은 정상, 단위 테스트도 통과
- 세션 저장 ✅ — `collect_session`(`ui/app.rs:630`)이 `sites`를 싣고 `Close`에서 `persist_session` 호출
- **UI 계층 ✗** — 편집 중인 이름(`SiteManager::renaming`)이 저장소로 넘어가는 길이 끊겨 있었다
- 최근 변경: 이 경로는 egui 이식 이후 그대로(직전 커밋들은 시작 화면·설정 화면 건)

## Phase 2 — Hypotheses
- H1: 대화를 닫을 때 편집 중인 이름을 버린다 — 예측: Enter를 누르면 저장되고 안 누르면 사라진다 — 검증: `close()` 경로 정독 → ✅ 확정
- H2: `lost_focus()`로 편집을 마치는 길이 죽어 있다 — 예측: 다른 곳을 눌러도 확정되지 않는다 — 검증: egui `Memory::lost_focus` 정의 대조 → ✅ 확정
- H3: 세션 저장이 사이트를 담지 않는다 — 예측: 앱을 다시 켜야 원복된다 — 검증: `collect_session` → ❌ 기각
- H4: `TextEdit`이 Enter 이벤트를 삼켜 확정 신호가 사라진다 — 예측: Enter도 듣지 않는다 — 검증: egui는 `ui.input`(불변)으로 이벤트를 걸러 읽어 원본이 남는다 → ❌ 기각

## Phase 3 — Root Cause
이름이 확정되는 길이 **Enter 하나뿐**이었고, 그마저도 둘 중 하나는 처음부터 막혀 있었다.

1. `show_rename_row`가 매 프레임 `response.request_focus()`를 청했다. egui의
   `Memory::lost_focus(id)`는 `had_recent_focus && !has_focus(id)`라, 매 프레임 포커스를 청하면
   `has_focus`가 늘 참이 되어 **`lost_focus()`가 영영 거짓**이다 — 다른 곳을 눌러 편집을 마치는
   길이 통째로 막힌다.
2. `close()`는 `self.renaming = None`으로 편집 중이던 글자를 그냥 버렸다. `확인(O)`·`연결(C)`·
   `X`·대화 바깥 누르기가 모두 이 길로 들어오고, `commit()`은 이름을 건드리지 않는다.

그래서 Enter를 누르지 않은 모든 마무리에서 이름이 조용히 사라졌다.

## Phase 4 — Fix
- Test added:
  - `ui::site_manager::tests::이름을_고치다_대화를_닫아도_저장된다` — 근본 원인 ②(닫으면서 버림)
  - `ui::site_manager::tests::편집기_밖을_누르면_이름이_확정된다` — 근본 원인 ①(포커스 경로).
    헤드리스 `Context::run_ui`로 세 프레임을 그린다: 편집기가 뜬 프레임, 사용자가 밖을 누른
    프레임(포커스는 **프레임 도중** 옮겨간다), 그것을 알아채는 프레임. egui가 `lost_focus`를
    두 프레임 창(`id_two_frames_ago`)으로 판정하기 때문에 세 프레임이 필요하다
- Change:
  - `site_manager.rs:1259` `show_rename_row(.., focus)` — 포커스는 **편집기가 처음 뜬 프레임에만** 청한다
  - `site_manager.rs:361·486` `rename_focus` 상태 — 그 첫 프레임을 표시한다
  - `site_manager.rs:411` `close(&mut self, store)` — 닫기 전에 `finish_rename`으로 이름을 확정한다
- Defense in depth: `select`·`Delete`·`open_new`에서도 `rename_focus`를 함께 정리해 상태가 남지 않게 했다

## Verification
- `cargo test`: 707 passed, 0 failed (통합 테스트 8건 포함 전부 통과)
- 회귀 테스트 RED 확인: `show_rename_row`를 고치기 전 모습(매 프레임 `request_focus`)으로
  임시로 되돌리면 `편집기_밖을_누르면_이름이_확정된다`가 실패한다
- `cargo clippy --all-targets -- -D warnings`: 경고 0
- `cargo fmt --check`: 통과
- 수동 재현: **사용자 확인 필요** (GUI 상호작용이라 자동 검증 대상이 아니다)

## 곁가지 — 빌드가 막혀 있었다
`target/`에 캐시된 빌드 스크립트 출력이 옛 폴더 이름(`FileExplorer`)의 매니페스트 절대 경로를
들고 있어 링커가 `LNK1327`로 죽었다. `build.rs`를 다시 돌려(mtime 갱신) 해소했다 — 소스 변경은
없다. 폴더를 옮기거나 이름을 바꾼 뒤 같은 오류가 나면 `touch build.rs`로 푼다.
