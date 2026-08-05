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

/// 끌어 옮기는 항목 하나 (FR-38).
///
/// **로컬과 원격을 한 타입에 섞지 않는다** — 로컬은 파일시스템 경로, 원격은 서버 경로이고
/// 크기도 원격만 미리 안다. 하나로 뭉치면 받는 쪽이 매번 "이게 어느 쪽이지"를 되묻게 된다
#[derive(Debug, Clone, PartialEq)]
pub enum DragItem {
    Local {
        path: std::path::PathBuf,
        is_dir: bool,
    },
    Remote {
        path: crate::remote::types::RemotePath,
        is_dir: bool,
        size: u64,
    },
}

impl DragItem {
    pub fn is_dir(&self) -> bool {
        match self {
            DragItem::Local { is_dir, .. } | DragItem::Remote { is_dir, .. } => *is_dir,
        }
    }

    /// 옮겨 놓을 때 쓸 이름 — 경로의 마지막 조각
    pub fn name(&self) -> String {
        match self {
            DragItem::Local { path, .. } => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default(),
            DragItem::Remote { path, .. } => path.file_name().unwrap_or_default().to_owned(),
        }
    }
}

/// 목록에서 끌기 시작할 때 싣는 값 (FR-38).
///
/// **패널 번호를 싣지 않는다** — 받는 쪽이 알아야 하는 것은 "어디서 왔나"가 아니라
/// "로컬인가 원격인가"뿐이고(로컬↔로컬·원격↔원격은 이번 범위 밖이다), 번호를 실으면
/// 그 사이 패널이 닫혔을 때 가리키는 곳이 사라진다
#[derive(Debug, Clone, PartialEq)]
pub struct FileDrag {
    pub items: Vec<DragItem>,
    /// 원격에서 끌어온 것이면 그 사이트 — 받는 쪽이 어느 서버에서 받을지 알아야 한다
    pub source_site: Option<crate::remote::types::SiteId>,
}

/// 끌어다 놓은 자리 — 그 패널이 지금 보고 있는 폴더다
#[derive(Debug, Clone, PartialEq)]
pub enum DropTarget {
    Local(std::path::PathBuf),
    Remote {
        site: crate::remote::types::SiteId,
        dir: crate::remote::types::RemotePath,
    },
}

/// 드롭 한 번의 결과 — 실제로 큐에 넣는 것은 앱이 한다 (plan T22 의존 방향)
#[derive(Debug, Clone, PartialEq)]
pub struct DropOutcome {
    pub items: Vec<DragItem>,
    /// 원격에서 끌어왔으면 그 사이트
    pub source_site: Option<crate::remote::types::SiteId>,
    pub target: DropTarget,
}

/// 끌어다 놓은 항목 하나가 실제로 옮겨지는가, 옮겨진다면 어느 방향인가 (FR-38).
///
/// **로컬 → 원격은 올리기, 원격 → 로컬은 받기**뿐이다. 로컬끼리·원격끼리는 `None`이다 —
/// 로컬↔로컬 이동·복사와 원격↔원격 전송은 PRD Out of Scope다(같은 자리에 놓은 경우도 여기 든다)
pub fn drop_direction(
    item: &DragItem,
    target: &DropTarget,
) -> Option<crate::remote::connection::TransferDirection> {
    use crate::remote::connection::TransferDirection;
    match (item, target) {
        (DragItem::Local { .. }, DropTarget::Remote { .. }) => Some(TransferDirection::Upload),
        (DragItem::Remote { .. }, DropTarget::Local(_)) => Some(TransferDirection::Download),
        _ => None,
    }
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
    use crate::remote::connection::TransferDirection;
    use crate::remote::types::{RemotePath, SiteId};

    fn local_item() -> DragItem {
        DragItem::Local {
            path: std::path::PathBuf::from(r"C:\work\app.js"),
            is_dir: false,
        }
    }

    fn remote_item() -> DragItem {
        DragItem::Remote {
            path: RemotePath::new("/var/www/app.js"),
            is_dir: false,
            size: 10,
        }
    }

    fn local_target() -> DropTarget {
        DropTarget::Local(std::path::PathBuf::from(r"C:\down"))
    }

    fn remote_target() -> DropTarget {
        DropTarget::Remote {
            site: SiteId(1),
            dir: RemotePath::new("/var/www"),
        }
    }

    #[test]
    fn 로컬에서_원격으로_끌면_올리기다() {
        // Acceptance ①
        assert_eq!(
            drop_direction(&local_item(), &remote_target()),
            Some(TransferDirection::Upload)
        );
        assert_eq!(
            drop_direction(&remote_item(), &local_target()),
            Some(TransferDirection::Download)
        );
    }

    #[test]
    fn 같은_쪽끼리_끌면_아무_일도_없다() {
        // Acceptance ② — 로컬↔로컬·원격↔원격은 PRD Out of Scope다(자기 자신에게 놓은 것도 포함)
        assert_eq!(drop_direction(&local_item(), &local_target()), None);
        assert_eq!(drop_direction(&remote_item(), &remote_target()), None);
    }

    #[test]
    fn 끌_항목의_이름은_경로의_마지막_조각이다() {
        assert_eq!(local_item().name(), "app.js");
        assert_eq!(remote_item().name(), "app.js");
        assert!(!local_item().is_dir());
    }

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
