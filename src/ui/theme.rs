//! egui 다크 팔레트 — 탐색기 고정 다크 스타일 (FR-21).
//!
//! 색 값은 현행 Win32 판(`app::theme`)과 **같은 화면색**을 내도록 그대로 옮긴 것이다.
//! 타입만 다르다: Win32는 `COLORREF`(0x00BBGGRR 바이트 순서)라 egui에서 그대로 쓸 수 없어
//! `Color32`로 재정의한다. 전환 UI는 없다(PRD Out of Scope).
use eframe::egui;

/// 창 배경·스플리터 틈
pub const WINDOW_BG: egui::Color32 = egui::Color32::from_rgb(0x1B, 0x1B, 0x1B);
/// 목록·트리·입력 컨트롤 배경
pub const SURFACE_BG: egui::Color32 = egui::Color32::from_rgb(0x1E, 0x1E, 0x1E);
/// 기본 글자색
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE8, 0xE8);
/// 목록 헤더 배경
pub const HEADER_BG: egui::Color32 = egui::Color32::from_rgb(0x25, 0x25, 0x25);
/// 목록 헤더 글자
pub const HEADER_TEXT: egui::Color32 = egui::Color32::from_rgb(0xC8, 0xC8, 0xC8);
/// 트리 연결선
pub const TREE_LINE: egui::Color32 = egui::Color32::from_rgb(0x45, 0x45, 0x45);
/// 버튼·컨트롤 기본 배경
pub const CONTROL_BG: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x2A, 0x2A);
/// 버튼 hover 배경
pub const CONTROL_HOT: egui::Color32 = egui::Color32::from_rgb(0x38, 0x38, 0x38);
/// 버튼 눌림·선택 배경
pub const CONTROL_ACTIVE: egui::Color32 = egui::Color32::from_rgb(0x45, 0x45, 0x45);
/// 비활성 글자색
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(0x6A, 0x6A, 0x6A);
/// 타이틀바 닫기 버튼 hover 배경 — Windows 11 캡션 닫기 버튼과 같은 빨강 (FR-22)
pub const CLOSE_HOT: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
/// 끊긴 네트워크 드라이브의 연결 끊김 배지 — 트리 아이콘 오른쪽 아래에 겹치는 원 (FR-9).
///
/// `CLOSE_HOT`과 값이 같지만 뜻이 다른 자리라 이름을 따로 둔다(팔레트가 `OK_DOT`·`OK_TEXT`
/// 처럼 용도별 상수를 두는 관례와 같다) — `ERROR`(#FF6B6B)는 글자용이라 이 크기의 원에
/// 칠하면 연해서 탐색기 배지와 다르게 읽힌다
pub const OFFLINE_BADGE: egui::Color32 = egui::Color32::from_rgb(0xC4, 0x2B, 0x1C);
/// 분할 패널 경계선 — 패널마다 두르는 기본 테두리 (FR-1·FR-2)
pub const PANE_BORDER: egui::Color32 = egui::Color32::from_rgb(0x33, 0x33, 0x33);
/// 활성 패널 경계선 — 지금 입력을 받는 패널만 한 단계 밝게 두른다.
/// 버튼 상태색(`CONTROL_ACTIVE`)을 빌려 쓰지 않는 이유: 경계선과 버튼은 쓰임이 달라
/// 한쪽을 조정하면 다른 쪽이 함께 바뀐다
pub const PANE_BORDER_ACTIVE: egui::Color32 = egui::Color32::from_rgb(0x5A, 0x5A, 0x5A);
/// 탭의 폴더 아이콘 노랑 — 무채색이 아닌 두 상수 중 하나다(다른 하나는 위 `CLOSE_HOT`).
/// 탐색기 탭은 폴더를 가리키므로 색으로 종류가 드러나는 편이 낫고, 무채색 단계로는 표현할 수 없다
pub const FOLDER_ICON: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xB3, 0x4D);

// ── 원격 연결 화면의 색 (디자인 README `### Colors` 1:1) ──
// 여기 값들은 **디자인 문서가 정본**이다 — 화면마다 비슷한 색을 새로 고르면 같은 상태가
// 자리마다 다른 색으로 보인다.
//
// 표의 색을 **한 번에 다 옮겨 둔다** — 지금 쓰이지 않는 것(강조 파랑·기본 버튼·웰 배경 등)은
// 사이트 관리자·전송 큐·서버 로그 화면(T12~T21)이 쓸 자리다. 화면마다 그때그때 상수를 더하면
// 같은 색이 파일마다 다른 이름으로 생겨 정본이 갈린다.

/// 강조 파랑 — 선택된 탭 밑줄·진행 막대·라디오 선택
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(0x4A, 0x9E, 0xFF);

/// 연결됨(성공) 계열 — 점 / 글자 / 채움 / 테두리
pub const OK_DOT: egui::Color32 = egui::Color32::from_rgb(0x4A, 0xDE, 0x80);
pub const OK_TEXT: egui::Color32 = egui::Color32::from_rgb(0x7F, 0xD6, 0xA2);
pub const OK_FILL: egui::Color32 = egui::Color32::from_rgb(0x16, 0x24, 0x1C);
pub const OK_BORDER: egui::Color32 = egui::Color32::from_rgb(0x2F, 0x6B, 0x4F);

/// 연결 중(경고) 계열 — 점·글자가 같은 색이다
pub const WARN: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xB3, 0x4D);
pub const WARN_FILL: egui::Color32 = egui::Color32::from_rgb(0x24, 0x1F, 0x14);
pub const WARN_BORDER: egui::Color32 = egui::Color32::from_rgb(0x6B, 0x56, 0x2F);

/// 오류 계열 — 어두운 배경 위 글자는 `ERROR_TEXT`가 따로 있다
pub const ERROR: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x6B, 0x6B);
pub const ERROR_TEXT: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x8A, 0x8A);
pub const ERROR_FILL: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x1A, 0x1A);
pub const ERROR_BORDER: egui::Color32 = egui::Color32::from_rgb(0x4A, 0x26, 0x26);

/// 끝난 전송을 나타내는 초록 — 사이트 관리자의 `연결` 버튼이 채움으로 쓰던 색이며,
/// 그 버튼이 평면이 된 뒤로는 전송 큐의 진행 막대만 쓴다
pub const PRIMARY_FILL: egui::Color32 = egui::Color32::from_rgb(0x2F, 0x6B, 0x4F);

/// 입력·목록 웰 배경 — 사이트 관리자의 입력칸과 목록이 이 색 위에 앉는다
pub const WELL_BG: egui::Color32 = egui::Color32::from_rgb(0x15, 0x15, 0x15);

/// 팝업 메뉴 프레임의 모서리 반경 — 컨텍스트 메뉴·드롭다운이 모두 이 값을 쓴다.
///
/// **정본을 여기 둔 이유**: 종전에는 메뉴마다 `Frame::menu`에 `.corner_radius(0)`을
/// 덧붙이거나 붙이지 않아, 같은 우클릭 메뉴인데 원격 목록은 각지고 설정 메뉴는 둥글었다
/// (2026-08-19 사용자 보고). `apply_dark`가 이 값을 egui 스타일에 세우면
/// `Frame::menu`가 그것을 읽으므로 각 메뉴는 아무것도 적지 않아도 같은 모양이 된다.
/// 대화 팝업(`ui::dialog`)의 12px과는 별개다 — 그쪽은 버튼 줄을 낀 모달이라 부품이 다르다
pub const MENU_CORNER_RADIUS: u8 = 6;

/// 행 hover / 메뉴 hover
pub const ROW_HOT: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x2E, 0x2E);
pub const MENU_HOT: egui::Color32 = egui::Color32::from_rgb(0x38, 0x38, 0x38);

/// 테두리 — 옅은 것 / 컨트롤
pub const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(0x2C, 0x2C, 0x2C);
pub const BORDER_CONTROL: egui::Color32 = egui::Color32::from_rgb(0x3A, 0x3A, 0x3A);

/// 사이드바 카드·사이트 행의 배경과 hover.
///
/// 워크스페이스 카드와 연결 섹션의 사이트 행이 **같은 hover 색**(`#282828`)을 쓴다 —
/// 사이드바 안에 색 정본이 둘로 갈리지 않게 여기에 둔다 (D20)
pub const CARD_BG: egui::Color32 = egui::Color32::from_rgb(0x23, 0x23, 0x23);
pub const CARD_HOT: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);

/// 선택된 탭의 글자 — 탭 배경 차이가 옅어 기본 글자색(#E8E8E8)으로는 비선택(#9A9A9A)과
/// 잘 갈리지 않는다(2026-08-18 보고). 도크 탭과 전송 큐의 사이트 탭이 함께 쓴다
pub const TEXT_SELECTED: egui::Color32 = egui::Color32::WHITE;

/// 보조 글자 — 밝은 순서대로. `HEADER_TEXT`(#C8C8C8)·`TEXT_DIM`(#6A6A6A)이 그 사이를 메운다
pub const TEXT_BUTTON: egui::Color32 = egui::Color32::from_rgb(0xD8, 0xD8, 0xD8);
pub const TEXT_LOG: egui::Color32 = egui::Color32::from_rgb(0xB4, 0xB4, 0xB4);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0x9A, 0x9A, 0x9A);
pub const TEXT_FAINT: egui::Color32 = egui::Color32::from_rgb(0x8A, 0x8A, 0x8A);

/// 고정 다크 팔레트를 egui 컨텍스트에 적용한다.
/// egui 기본 다크를 토대로, 위 상수로 현행 앱과 같은 색을 덮어쓴다.
pub fn apply_dark(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = WINDOW_BG;
    visuals.window_fill = SURFACE_BG;
    visuals.extreme_bg_color = SURFACE_BG;
    visuals.faint_bg_color = HEADER_BG;
    visuals.override_text_color = Some(TEXT);

    // 위젯 상태별 배경 — 버튼·입력창이 현행 오너드로우와 같은 명도 단계를 갖게 한다
    visuals.widgets.noninteractive.bg_fill = SURFACE_BG;
    visuals.widgets.inactive.bg_fill = CONTROL_BG;
    visuals.widgets.hovered.bg_fill = CONTROL_HOT;
    visuals.widgets.active.bg_fill = CONTROL_ACTIVE;
    visuals.selection.bg_fill = CONTROL_ACTIVE;

    // 팝업 메뉴 모서리 — `Frame::menu`가 여기서 읽어 간다 (`MENU_CORNER_RADIUS` 참조)
    visuals.menu_corner_radius = egui::CornerRadius::same(MENU_CORNER_RADIUS);

    ctx.set_visuals(visuals);

    // egui의 디버그 경고 하나를 끈다 — 디버그 빌드에만 있는 옵션이라 cfg로 가른다.
    //
    // `warn_if_rect_changes_id`는 "같은 자리를 지난 프레임과 다른 위젯이 차지하면 id가
    // 불안정한 것"이라는 휴리스틱이고, 걸리면 그 위젯에 **2px 빨간 테두리**를 두른다.
    // 이 앱은 하단 도크를 여닫을 때마다 패널이 통째로 위아래로 밀리므로 자리를 물려받는
    // 위젯이 수십 개씩 생긴다 — 화면 대부분이 한 프레임 빨갛게 칠해졌다(사용자 보고).
    // 우리 위젯 id는 패널 id·탭 인덱스로 고정돼 있어 이 경고가 겨냥하는 불안정이 아니다
    #[cfg(debug_assertions)]
    ctx.all_styles_mut(|style| style.debug.warn_if_rect_changes_id = false);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `src/ui` 아래의 `.rs`를 하위 폴더까지 모아 온다.
    ///
    /// **비재귀로 훑으면 `ui/panel/` 같은 하위가 통째로 빠진다** — 모달 규약 시험이
    /// 그렇게 짜여 있어 대기 목록에 올라 있다. 새로 쓰는 이 시험은 그 함정을 피한다
    fn ui_sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("ui 디렉터리") {
            let path = entry.expect("항목").path();
            if path.is_dir() {
                ui_sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }

    /// `Frame::menu(...)`로 시작하는 체인 안에서 `.corner_radius(`를 부르는가.
    ///
    /// 같은 파일에 메뉴가 아닌 프레임(대화 셸 등)이 함께 있을 수 있어 파일 전체를
    /// 훑지 않고 **그 호출에서 이어지는 구간만** 본다. 체인은 `.show(`에서 끝난다
    fn menu_frame_sets_corner(source: &str) -> bool {
        let mut rest = source;
        while let Some(at) = rest.find("Frame::menu(") {
            let tail = &rest[at..];
            let end = tail.find(".show(").unwrap_or(tail.len());
            if tail[..end].contains(".corner_radius(") {
                return true;
            }
            rest = &tail[end.min(tail.len())..];
            // 진행이 없으면(찾은 자리가 끝) 무한히 돌지 않도록 끊는다
            if rest.is_empty() {
                break;
            }
        }
        false
    }

    #[test]
    fn 규약_검사는_값을_가리지_않는다() {
        // 검사기 자신을 시험한다 — `0`만 잡던 종전 방식이 놓치던 두 형태를 든다
        assert!(menu_frame_sets_corner(
            "egui::Frame::menu(s).fill(a).corner_radius(0).show(ui, |ui| {})"
        ));
        assert!(menu_frame_sets_corner(
            "egui::Frame::menu(s).corner_radius(2).show(ui, |ui| {})"
        ));
        assert!(menu_frame_sets_corner(
            "egui::Frame::menu(s).corner_radius(egui::CornerRadius::ZERO).show(ui, |ui| {})"
        ));
        // 메뉴 프레임이 모서리를 적지 않으면 통과한다
        assert!(!menu_frame_sets_corner(
            "egui::Frame::menu(s).fill(a).stroke(b).show(ui, |ui| {})"
        ));
        // 메뉴가 아닌 프레임의 모서리는 대상이 아니다 (대화 셸 등)
        assert!(!menu_frame_sets_corner(
            "egui::Frame::new().corner_radius(12).show(ui, |ui| {})"
        ));
    }

    #[test]
    fn 팝업_메뉴는_모서리를_따로_적지_않는다() {
        // 규약: 메뉴 모서리의 정본은 `theme::MENU_CORNER_RADIUS` 하나이고, 각 메뉴는
        // `Frame::menu`가 읽어 가게 둔다. 문서로만 두면 다음 작업자가 다시 `.corner_radius(0)`을
        // 붙여도 아무것도 걸리지 않아, 같은 우클릭 메뉴인데 어떤 것은 각지고 어떤 것은
        // 둥근 상태로 되돌아간다 (2026-08-19 사용자 보고가 그 상태였다)
        let ui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut sources = Vec::new();
        ui_sources(&ui_dir, &mut sources);
        assert!(!sources.is_empty(), "ui 소스를 하나도 읽지 못했다");

        // 이 파일은 규약을 설명하느라 그 문자열을 주석에 담는다. **경로 전체로 견준다** —
        // 이름만 보면 하위 폴더에 같은 이름이 생겼을 때 그 파일까지 조용히 빠진다
        let self_path = ui_dir.join("theme.rs");
        let mut 발견 = Vec::new();
        for path in sources {
            if path == self_path {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("소스를 읽지 못했다");
            // **값을 가리지 않고 잡는다** — `.corner_radius(0)`만 찾으면 `.corner_radius(2)`나
            // `.corner_radius(CornerRadius::ZERO)`로 적은 곳이 규약을 어긴 채 통과한다.
            // 메뉴 프레임을 만드는 자리에서는 모서리를 **아예 적지 않는 것**이 규약이다
            if menu_frame_sets_corner(&source) {
                발견.push(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
        assert!(
            발견.is_empty(),
            "메뉴 모서리를 따로 적은 곳(테마의 공통 값을 쓰도록 지운다): {발견:?}"
        );
    }

    #[test]
    fn 메뉴_모서리는_대화_모서리와_다른_부품이다() {
        // 둘을 한 값으로 묶으면 한쪽을 바꿀 때 다른 쪽이 조용히 따라간다 —
        // 메뉴는 버튼 줄이 없는 얇은 팝업이라 대화(12px)보다 덜 둥글다
        assert_ne!(MENU_CORNER_RADIUS, crate::ui::dialog::CORNER_RADIUS);
    }
}
