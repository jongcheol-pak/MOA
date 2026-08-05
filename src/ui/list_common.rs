//! 보기 모드별 렌더 모듈이 함께 쓰는 조각 (FR-4·FR-23).
//!
//! `ui::list_details`(자세히)와 `ui::list_grid`(나머지 보기)가 같은 조작 타입과 텍스트
//! 배치 규칙을 쓴다. 한쪽에 두고 다른 쪽이 참조하면 두 렌더 모듈이 서로를 알게 되므로
//! 공용 조각만 여기로 모은다.
use crate::ui::theme;
use eframe::egui;
use std::sync::Arc;

/// 목록이 상위(패널)에 돌려주는 사용자 조작.
/// 즉시 모드라 콜백을 등록하지 않고 이번 프레임의 조작을 값으로 반환한다
#[derive(Clone, PartialEq, Debug, Default)]
pub enum FileListAction {
    #[default]
    None,
    /// 항목 실행 — 폴더면 진입, 파일이면 연결 프로그램 (호출부가 판정)
    Open(usize),
    /// 컨텍스트 메뉴 요청 — `index`가 `None`이면 빈 영역(폴더 배경 메뉴)
    Context {
        index: Option<usize>,
        pos: egui::Pos2,
    },
}

/// 텍스트를 **한 줄로만** 배치하고, 폭을 넘으면 끝을 `…`로 줄인 갤리를 만든다.
///
/// `Painter::layout`을 쓰면 안 된다 — 그 함수의 폭 인자는 자르는 폭이 아니라 **줄바꿈 폭**이라
/// 긴 이름이 여러 줄이 된다. 행 높이가 고정인 자세히 보기에서는 2줄이 되는 순간 아래 행과
/// 겹쳐 글자가 포개져 보인다(사용자 보고 4번)
pub fn elided_galley(
    painter: &egui::Painter,
    text: String,
    font: egui::FontId,
    max_width: f32,
) -> Arc<egui::Galley> {
    elided_galley_rows(painter, text, font, max_width, 1)
}

/// 색을 지정하는 변형.
///
/// **갤리는 색을 구워 넣는다** — `Painter::galley`에 넘기는 색은 갤리 안이 `PLACEHOLDER`일
/// 때만 쓰인다. 그래서 기본색으로 만든 갤리를 그리면서 다른 색을 넘겨도 **아무 일도 일어나지
/// 않는다**(로그·큐 표에서 실제로 그랬다 — T20 리뷰). 색이 다른 자리는 이 함수로 만든다
pub fn elided_galley_colored(
    painter: &egui::Painter,
    text: String,
    font: egui::FontId,
    max_width: f32,
    color: egui::Color32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, max_width);
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

/// 줄 수를 지정하는 변형 — 격자 보기의 이름은 두 줄까지 쓴다 (plan 시각 속성 표).
///
/// `max_rows`를 넘으면 마지막 줄 끝에 `…`가 붙는다. 파일 이름은 공백 없는 긴 토큰이 흔해
/// 단어 단위로만 끊으면 폭을 넘은 채 잘리므로 아무 곳에서나 끊게 한다
pub fn elided_galley_rows(
    painter: &egui::Painter,
    text: String,
    font: egui::FontId,
    max_width: f32,
    max_rows: usize,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, theme::TEXT, max_width);
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    painter.layout_job(job)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 갤리에 실제로 구워진 색 — `Painter::galley`에 넘기는 색은 이 값이 `PLACEHOLDER`일 때만 쓰인다
    fn baked_color(galley: &egui::Galley) -> egui::Color32 {
        galley.job.sections[0].format.color
    }

    #[test]
    fn 색을_지정한_갤리는_그_색을_구워_넣는다() {
        // T20 리뷰 — 기본색으로 만든 갤리를 그리면서 다른 색을 넘겨도 아무 일도 일어나지 않는다.
        // 로그 본문·큐 표의 색이 실제로는 전부 기본색으로 나오던 결함이 여기서 비롯됐다
        let ctx = egui::Context::default();
        let mut plain = None;
        let mut colored = None;
        let _ = ctx.run_ui(Default::default(), |ui| {
            let painter = ui.painter();
            plain = Some(elided_galley(
                painter,
                "본문".to_owned(),
                egui::FontId::proportional(13.0),
                100.0,
            ));
            colored = Some(elided_galley_colored(
                painter,
                "본문".to_owned(),
                egui::FontId::proportional(13.0),
                100.0,
                theme::TEXT_LOG,
            ));
        });
        assert_eq!(baked_color(&plain.expect("기본")), theme::TEXT);
        assert_eq!(baked_color(&colored.expect("지정")), theme::TEXT_LOG);
    }

    /// 앱과 같은 글꼴을 설치한 뒤 배치한다 — 이 crate는 egui 기본 글꼴 기능을 끄고
    /// 맑은 고딕을 직접 등록하므로, 글꼴 없이 배치하면 모든 글자 폭이 0이 되어
    /// 폭 기준 검증이 무의미해진다
    fn layout(text: &str, width: f32, rows: usize) -> (usize, bool) {
        let ctx = egui::Context::default();
        let has_font = crate::ui::app::install_fonts(&ctx);
        let mut result = (0, false);
        let _ = ctx.run_ui(Default::default(), |ui| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let galley = elided_galley_rows(ui.painter(), text.to_owned(), font, width, rows);
            result = (galley.rows.len(), galley.elided);
        });
        assert!(has_font, "맑은 고딕을 읽지 못해 폭 기준 검증을 할 수 없다");
        result
    }

    #[test]
    fn 한_줄_배치는_폭을_넘으면_줄인다() {
        let long = "NTUSER.DAT{71e7eeb8-8e0f-11f0-80fa-000d3aa7ca88}.TM.blf";
        let (rows, elided) = layout(long, 100.0, 1);
        assert_eq!(rows, 1);
        assert!(elided);
    }

    #[test]
    fn 격자_이름은_두_줄까지_쓴다() {
        // 한 줄로 자르면 격자에서 이름이 지나치게 짧아진다 — 탐색기도 두 줄을 쓴다
        let long = "아주긴한글파일이름입니다그리고더깁니다.txt";
        let (one, _) = layout(long, 100.0, 1);
        let (two, _) = layout(long, 100.0, 2);
        assert_eq!(one, 1);
        assert_eq!(two, 2, "두 줄을 허용했는데 한 줄로 잘렸다");
    }

    #[test]
    fn 두_줄에_들어가면_줄이지_않는다() {
        let (rows, elided) = layout("짧은이름.txt", 300.0, 2);
        assert_eq!(rows, 1);
        assert!(!elided);
    }

    #[test]
    fn 아주_좁은_폭에서도_패닉하지_않는다() {
        for width in [0.0, 1.0, 3.0, -10.0] {
            for rows in [1, 2] {
                let (drawn, _) = layout("아주긴한글파일이름.txt", width, rows);
                assert!(drawn <= rows, "폭 {width}·{rows}줄: 허용 줄 수를 넘겼다");
            }
        }
    }
}
