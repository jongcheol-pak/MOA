//! 보기 모드별 렌더 모듈이 함께 쓰는 조각 (FR-4·FR-23).
//!
//! `ui::list_details`(자세히)와 `ui::list_grid`(나머지 보기)가 같은 조작 타입과 텍스트
//! 배치 규칙을 쓴다. 한쪽에 두고 다른 쪽이 참조하면 두 렌더 모듈이 서로를 알게 되므로
//! 공용 조각만 여기로 모은다.
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
    /// 폴더인가 — 로컬·원격 어느 쪽이든 같은 물음이다
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
/// "로컬인가 원격인가"뿐이고(그 조합만으로 전송인지 복사인지가 정해진다), 번호를 실으면
/// 그 사이 패널이 닫혔을 때 가리키는 곳이 사라진다
#[derive(Debug, Clone, PartialEq)]
pub struct FileDrag {
    pub items: Vec<DragItem>,
    /// 원격에서 끌어온 것이면 그 사이트 — 받는 쪽이 어느 서버에서 받을지 알아야 한다
    pub source_site: Option<crate::remote::types::SiteId>,
}

/// 같은 이름이 있을 때 사용자가 고른 것 (FR-55).
///
/// 취소는 이 값이 아니라 대화가 `Cancelled`로 알린다 — 취소는 "무엇을 할지"가 아니라
/// "아무것도 하지 않는다"라서 여기 넣으면 호출부가 그것도 처리해야 할 선택지로 읽는다.
///
/// `DragItem`·`DropOutcome`과 같은 자리에 둔다 — 전송을 여는 쪽(`ui::app`)과 물어보는
/// 쪽(`ui::remote_menu`)이 둘 다 쓰므로, 어느 한쪽에 두면 두 모듈이 서로를 알게 된다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    /// 있던 것을 덮어쓰고 전부 보낸다
    Overwrite,
    /// 겹치는 것만 빼고 나머지를 보낸다
    Skip,
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
/// 원격↔원격은 PRD Out of Scope이고, **로컬↔로컬은 전송이 아니라 셸 복사**라서
/// 아래 `local_copy_target`이 따로 판정한다(FR-60).
///
/// **이 함수에 복사를 섞지 않는다** — 이 값은 전송 큐에 넣을 항목을 거르는 필터로도
/// 쓰이므로(`ui::app::transfer_conflict`), 로컬↔로컬이 여기를 통과하면 복사할 것이
/// 전송 큐에 들어간다
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

/// 이 드롭이 **로컬끼리의 복사**인가 — 맞으면 놓인 폴더를 돌려준다 (FR-60).
///
/// 항목이 전부 로컬이고 놓인 자리도 로컬일 때만 성립한다. 원격이 하나라도 섞이면
/// `None`이며 그 드롭은 종전대로 전송 경로(`drop_direction`)로 간다.
///
/// **같은 폴더에 놓은 것을 걸러내지 않는다** — 사본을 만들지 거부할지는 셸이 정한다(D9).
/// 빈 항목도 `None`이다: 복사를 걸 것이 없으면 아무 일도 일어나지 않아야 한다
pub fn local_copy_target(drop: &DropOutcome) -> Option<&std::path::Path> {
    let DropTarget::Local(dir) = &drop.target else {
        return None;
    };
    if drop.items.is_empty()
        || !drop
            .items
            .iter()
            .all(|item| matches!(item, DragItem::Local { .. }))
    {
        return None;
    }
    Some(dir.as_path())
}

/// 숨김·시스템 항목을 그릴 때 글자·아이콘에 곱하는 불투명도 (FR-13).
///
/// 탐색기가 숨김 항목을 흐리게 보이는 것과 같은 표시다 — 목록에 있지만 보통 항목은
/// 아니라는 것을 색만으로 알린다. **글자와 아이콘에 같은 값을 쓴다**: 한쪽만 흐리면
/// 항목이 반쯤 지워진 것처럼 보인다
pub const HIDDEN_ALPHA: f32 = 0.5;

/// 흐리게 그릴 항목이면 흐린 색으로 바꾼다 — 글자색과 아이콘 tint가 함께 쓰는 한 벌의 규칙.
///
/// **무엇이 그 대상인지는 여기서 정하지 않는다** — 호출부가 `ListRow::is_dimmed()`로
/// 판정해 넘긴다(숨김이거나 시스템). 색 변환과 판정을 한 함수에 두면 목록·트리처럼
/// 판정 기준이 다른 자리가 이 함수를 함께 쓸 수 없다
pub fn dim_if_hidden(color: egui::Color32, hidden: bool) -> egui::Color32 {
    if hidden {
        color.gamma_multiply(HIDDEN_ALPHA)
    } else {
        color
    }
}

/// 텍스트를 **한 줄로만** 배치하고, 폭을 넘으면 끝을 `…`로 줄인 갤리를 만든다.
///
/// `Painter::layout`을 쓰면 안 된다 — 그 함수의 폭 인자는 자르는 폭이 아니라 **줄바꿈 폭**이라
/// 긴 이름이 여러 줄이 된다. 행 높이가 고정인 자세히 보기에서는 2줄이 되는 순간 아래 행과
/// 겹쳐 글자가 포개져 보인다(사용자 보고 4번)
///
/// **갤리는 색을 구워 넣는다** — `Painter::galley`에 넘기는 색은 갤리 안이 `PLACEHOLDER`일
/// 때만 쓰인다. 그래서 기본색으로 만든 갤리를 그리면서 다른 색을 넘겨도 **아무 일도 일어나지
/// 않는다**(로그·큐 표에서 실제로 그랬다 — T20 리뷰). 그래서 색을 인자로 받는다
pub fn elided_galley_colored(
    painter: &egui::Painter,
    text: String,
    font: egui::FontId,
    max_width: f32,
    color: egui::Color32,
) -> Arc<egui::Galley> {
    elided_galley_rows(painter, text, font, max_width, 1, color)
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
    color: egui::Color32,
) -> Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(text, font, color, max_width);
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
    use crate::ui::theme;

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
        // Acceptance ② — 이 함수는 **전송 방향만** 답한다. 원격↔원격은 PRD Out of Scope이고,
        // 로컬↔로컬은 전송이 아니라 셸 복사라 `local_copy_target`이 판정한다(FR-60)
        assert_eq!(drop_direction(&local_item(), &local_target()), None);
        assert_eq!(drop_direction(&remote_item(), &remote_target()), None);
    }

    #[test]
    fn 로컬끼리_놓으면_그_폴더로_복사한다() {
        // Acceptance ⓐ (FR-60)
        let drop = DropOutcome {
            items: vec![local_item()],
            source_site: None,
            target: local_target(),
        };
        assert_eq!(
            local_copy_target(&drop),
            Some(std::path::Path::new(r"C:\down"))
        );
    }

    #[test]
    fn 원격이_섞이거나_대상이_원격이면_복사가_아니다() {
        // Acceptance ⓑⓒ — 그 드롭은 종전대로 전송 경로로 간다
        let 섞임 = DropOutcome {
            items: vec![local_item(), remote_item()],
            source_site: None,
            target: local_target(),
        };
        assert_eq!(
            local_copy_target(&섞임),
            None,
            "원격이 하나라도 섞이면 복사가 아니다"
        );

        let 원격_대상 = DropOutcome {
            items: vec![local_item()],
            source_site: None,
            target: remote_target(),
        };
        assert_eq!(local_copy_target(&원격_대상), None, "올리기는 전송이다");
    }

    #[test]
    fn 놓은_것이_없으면_복사할_것도_없다() {
        // Acceptance ⓓ
        let 빈_드롭 = DropOutcome {
            items: Vec::new(),
            source_site: None,
            target: local_target(),
        };
        assert_eq!(local_copy_target(&빈_드롭), None);
    }

    #[test]
    fn 끌_항목의_이름은_경로의_마지막_조각이다() {
        assert_eq!(local_item().name(), "app.js");
        assert_eq!(remote_item().name(), "app.js");
    }

    #[test]
    fn 숨김_항목만_흐려진다() {
        // 숨김이 아닌 항목의 색은 손대지 않는다 — 목록 전체가 흐려지면 표시가 뜻을 잃는다
        assert_eq!(dim_if_hidden(theme::TEXT, false), theme::TEXT);
        let dimmed = dim_if_hidden(theme::TEXT, true);
        assert!(
            dimmed.a() < theme::TEXT.a(),
            "숨김 항목인데 불투명도가 그대로다"
        );
        // 아이콘 tint도 같은 규칙을 쓴다 (한쪽만 흐리면 항목이 반쯤 지워진 것처럼 보인다)
        assert_eq!(
            dim_if_hidden(egui::Color32::WHITE, true).a(),
            dimmed.a(),
            "글자와 아이콘의 흐림 정도가 다르다"
        );
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
            plain = Some(elided_galley_colored(
                painter,
                "본문".to_owned(),
                egui::FontId::proportional(13.0),
                100.0,
                theme::TEXT,
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
        let has_font = crate::ui::app::install_fonts(&ctx, None);
        let mut result = (0, false);
        let _ = ctx.run_ui(Default::default(), |ui| {
            let font = egui::TextStyle::Body.resolve(ui.style());
            let galley = elided_galley_rows(
                ui.painter(),
                text.to_owned(),
                font,
                width,
                rows,
                theme::TEXT,
            );
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
