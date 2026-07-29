//! 파일 목록 보기 모드와 배치 계산 (FR-23).
//!
//! Windows 탐색기의 보기 8종을 담고, 각 모드가 **항목을 어디에 그릴지**를 순수 함수로 계산한다.
//! 그리기·입력은 `ui::list_details`(자세히)와 `ui::list_grid`(나머지)가 맡는다 —
//! 격자 열 수·셀 위치는 오프바이원이 잦아 UI 없이 검증할 수 있게 여기로 분리했다
//! (AGENTS: 순수 로직을 UI에서 분리해 테스트).
use eframe::egui;

/// 파일 목록을 그리는 방식 (FR-23) — Windows 탐색기의 보기 메뉴와 같은 8종
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewMode {
    ExtraLargeIcons,
    LargeIcons,
    MediumIcons,
    SmallIcons,
    List,
    /// 이름·크기·종류·수정한 날짜 4열 (FR-4) — 기본값
    #[default]
    Details,
    Tiles,
    Content,
}

/// 항목이 채워지는 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// 왼→오른쪽으로 채우고 넘치면 다음 줄 (아이콘 4종·타일)
    Horizontal,
    /// 위→아래로 채우고 넘치면 오른쪽 열 (목록)
    Vertical,
    /// 한 줄에 하나, 폭 전체 (자세히·내용)
    Rows,
}

/// 격자 간격 — 가로세로 모두 이만큼 띄운다 (plan 시각 속성 표)
pub const GRID_SPACING: f32 = 8.0;
/// 격자에서 이름이 차지할 최대 줄 수 (plan 시각 속성 표)
pub const GRID_NAME_ROWS: usize = 2;

impl ViewMode {
    /// 메뉴에 보이는 순서 — plan `### 참조 정합 인벤토리 — '보기' 하위 메뉴` 8행 그대로
    pub const ALL: [ViewMode; 8] = [
        ViewMode::ExtraLargeIcons,
        ViewMode::LargeIcons,
        ViewMode::MediumIcons,
        ViewMode::SmallIcons,
        ViewMode::List,
        ViewMode::Details,
        ViewMode::Tiles,
        ViewMode::Content,
    ];

    /// 메뉴에 표시할 문구 (인벤토리 표 원문)
    pub fn label(self) -> &'static str {
        match self {
            ViewMode::ExtraLargeIcons => "아주 큰 아이콘",
            ViewMode::LargeIcons => "큰 아이콘",
            ViewMode::MediumIcons => "보통 아이콘",
            ViewMode::SmallIcons => "작은 아이콘",
            ViewMode::List => "목록",
            ViewMode::Details => "자세히",
            ViewMode::Tiles => "타일",
            ViewMode::Content => "내용",
        }
    }

    /// 세션에 저장할 키 — **variant 이름과 따로 둔다**.
    /// serde derive를 쓰면 나중에 variant 이름만 바꿔도 저장 파일이 못 읽히게 된다
    pub fn as_key(self) -> &'static str {
        match self {
            ViewMode::ExtraLargeIcons => "extra_large_icons",
            ViewMode::LargeIcons => "large_icons",
            ViewMode::MediumIcons => "medium_icons",
            ViewMode::SmallIcons => "small_icons",
            ViewMode::List => "list",
            ViewMode::Details => "details",
            ViewMode::Tiles => "tiles",
            ViewMode::Content => "content",
        }
    }

    /// 저장된 키로 되살린다. 모르는 키는 기본값(자세히)으로 — 설정 파일이 손상되거나
    /// 옛 버전이 쓰던 키가 남아 있어도 목록이 안 그려지는 상태로 시작하지 않게 한다
    pub fn from_key(key: &str) -> ViewMode {
        ViewMode::ALL
            .into_iter()
            .find(|mode| mode.as_key() == key)
            .unwrap_or_default()
    }

    /// 항목 아이콘의 한 변 (px) — 인벤토리 표의 아이콘 크기
    pub fn icon_px(self) -> f32 {
        match self {
            ViewMode::ExtraLargeIcons => 256.0,
            ViewMode::LargeIcons => 96.0,
            ViewMode::MediumIcons => 48.0,
            ViewMode::Tiles => 48.0,
            ViewMode::Content => 32.0,
            ViewMode::SmallIcons | ViewMode::List | ViewMode::Details => 16.0,
        }
    }

    /// 항목 하나가 차지하는 칸 크기 (plan 시각 속성 표).
    /// `Rows` 흐름(자세히·내용)의 가로값은 뜻이 없다 — 폭 전체를 쓰므로 높이만 유효하다
    pub fn cell_size(self) -> egui::Vec2 {
        match self {
            ViewMode::ExtraLargeIcons => egui::vec2(280.0, 320.0),
            ViewMode::LargeIcons => egui::vec2(120.0, 150.0),
            ViewMode::MediumIcons => egui::vec2(76.0, 100.0),
            ViewMode::Tiles => egui::vec2(220.0, 64.0),
            ViewMode::SmallIcons | ViewMode::List => egui::vec2(200.0, 20.0),
            ViewMode::Content => egui::vec2(0.0, 48.0),
            ViewMode::Details => egui::vec2(0.0, 20.0),
        }
    }

    /// 항목이 채워지는 방향
    pub fn flow(self) -> Flow {
        match self {
            ViewMode::ExtraLargeIcons
            | ViewMode::LargeIcons
            | ViewMode::MediumIcons
            | ViewMode::SmallIcons
            | ViewMode::Tiles => Flow::Horizontal,
            ViewMode::List => Flow::Vertical,
            ViewMode::Details | ViewMode::Content => Flow::Rows,
        }
    }

    /// 이 모드가 자세히 보기인가 — 열·머리글이 있는 유일한 모드다
    pub fn is_details(self) -> bool {
        self == ViewMode::Details
    }
}

/// 격자 배치 결과 — 몇 열 몇 줄인지와 콘텐츠 전체 크기
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridMetrics {
    pub mode: ViewMode,
    /// 가로로 놓이는 칸 수 (`Vertical` 흐름에서는 열 수)
    pub columns: usize,
    /// 세로로 놓이는 칸 수
    pub rows: usize,
    pub cell: egui::Vec2,
}

/// 뷰포트 크기에 맞춰 항목을 어떻게 배치할지 계산한다.
///
/// `Horizontal`은 폭에 맞춰 열 수를 정하고 줄이 늘어나며(세로 스크롤),
/// `Vertical`은 높이에 맞춰 줄 수를 정하고 열이 늘어난다(가로 스크롤).
/// `Rows`는 한 줄에 하나씩 폭 전체를 쓴다
pub fn grid_metrics(mode: ViewMode, viewport: egui::Vec2, item_count: usize) -> GridMetrics {
    let cell = mode.cell_size();
    if item_count == 0 {
        // 빈 폴더는 두 축 모두 0이어야 한다 — 한쪽만 0으로 두면 그릴 것이 없는데도
        // 콘텐츠 크기가 남아 스크롤 막대가 허공을 가리킨다
        return GridMetrics {
            mode,
            columns: 0,
            rows: 0,
            cell,
        };
    }
    let (columns, rows) = match mode.flow() {
        Flow::Rows => (1, item_count),
        Flow::Horizontal => {
            let columns = fit_count(viewport.x, cell.x);
            (columns, item_count.div_ceil(columns))
        }
        Flow::Vertical => {
            let rows = fit_count(viewport.y, cell.y);
            (item_count.div_ceil(rows), rows)
        }
    };
    GridMetrics {
        mode,
        columns,
        rows,
        cell,
    }
}

/// `available` 안에 `size`짜리 칸이 몇 개 들어가는가 — 칸 사이 간격을 감안한다.
///
/// **최소 1을 보장한다** — 0을 돌려주면 호출부가 그것으로 나누다 죽고, 칸이 뷰포트보다
/// 커도 한 칸은 그려서 스크롤로 볼 수 있어야 한다
fn fit_count(available: f32, size: f32) -> usize {
    // NaN·무한대·0 이하는 전부 여기서 걸러진다 — 아래 나눗셈이 뜻 없는 값을 내지 않게 한다
    if !size.is_finite() || size <= 0.0 || !available.is_finite() {
        return 1;
    }
    // n개가 들어가려면 n*size + (n-1)*spacing <= available
    let usable = available + GRID_SPACING;
    let each = size + GRID_SPACING;
    let count = (usable / each).floor();
    if count.is_finite() && count >= 1.0 {
        count as usize
    } else {
        1
    }
}

impl GridMetrics {
    /// 콘텐츠 전체 크기 — 스크롤 막대의 범위가 이 값에서 나온다
    pub fn content_size(&self) -> egui::Vec2 {
        egui::vec2(
            span(self.columns, self.cell.x),
            span(self.rows, self.cell.y),
        )
    }

    /// `index`번째 항목이 놓일 자리 (원점 기준 상대 좌표).
    /// 범위를 넘는 인덱스도 계산은 되지만 호출부가 항목 수로 걸러야 한다
    pub fn item_rect(&self, index: usize) -> egui::Rect {
        let (column, row) = match self.mode.flow() {
            // 위→아래로 채운 뒤 오른쪽 열로 넘어간다
            Flow::Vertical => (index / self.rows.max(1), index % self.rows.max(1)),
            // 왼→오른쪽으로 채운 뒤 다음 줄로 넘어간다
            Flow::Horizontal | Flow::Rows => {
                (index % self.columns.max(1), index / self.columns.max(1))
            }
        };
        let min = egui::pos2(
            column as f32 * (self.cell.x + GRID_SPACING),
            row as f32 * (self.cell.y + GRID_SPACING),
        );
        egui::Rect::from_min_size(min, self.cell)
    }

    /// 세로 스크롤 위치로부터 그려야 할 인덱스 범위 (가상 스크롤).
    /// `Vertical` 흐름은 가로로 늘어나므로 전부 그린다 — 열 수가 화면 밖으로 커지는 경우는
    /// 항목이 아주 많을 때뿐이고, 그때는 호출부가 가로 범위로 다시 거른다
    pub fn visible_range(
        &self,
        viewport_top: f32,
        viewport_bottom: f32,
        item_count: usize,
    ) -> std::ops::Range<usize> {
        if item_count == 0 || self.rows == 0 {
            return 0..0;
        }
        match self.mode.flow() {
            Flow::Vertical => 0..item_count,
            Flow::Horizontal | Flow::Rows => {
                let pitch = self.cell.y + GRID_SPACING;
                let first_row = (viewport_top / pitch).floor().max(0.0) as usize;
                let last_row = (viewport_bottom / pitch).ceil() as usize + 1;
                let first = first_row.saturating_mul(self.columns.max(1));
                let last = last_row.saturating_mul(self.columns.max(1)).min(item_count);
                first.min(item_count)..last
            }
        }
    }
}

/// 칸 `count`개가 차지하는 길이 (사이 간격 포함)
fn span(count: usize, size: f32) -> f32 {
    if count == 0 {
        return 0.0;
    }
    count as f32 * size + (count - 1) as f32 * GRID_SPACING
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 여덟_모드가_모두_들어_있다() {
        // 인벤토리 표 8행과 1:1 — 하나라도 빠지면 메뉴에서 고를 수 없다
        assert_eq!(ViewMode::ALL.len(), 8);
        let labels: Vec<&str> = ViewMode::ALL.iter().map(|m| m.label()).collect();
        assert_eq!(
            labels,
            [
                "아주 큰 아이콘",
                "큰 아이콘",
                "보통 아이콘",
                "작은 아이콘",
                "목록",
                "자세히",
                "타일",
                "내용",
            ]
        );
    }

    #[test]
    fn 기본값은_자세히다() {
        assert_eq!(ViewMode::default(), ViewMode::Details);
        assert!(ViewMode::default().is_details());
    }

    #[test]
    fn 저장_키는_왕복한다() {
        for mode in ViewMode::ALL {
            assert_eq!(
                ViewMode::from_key(mode.as_key()),
                mode,
                "{mode:?} 왕복 실패"
            );
        }
    }

    #[test]
    fn 저장_키는_서로_겹치지_않는다() {
        // 겹치면 복원 시 다른 모드로 되살아난다
        let mut keys: Vec<&str> = ViewMode::ALL.iter().map(|m| m.as_key()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(keys.len(), before);
    }

    #[test]
    fn 모르는_키는_자세히로_돌아간다() {
        assert_eq!(ViewMode::from_key(""), ViewMode::Details);
        assert_eq!(ViewMode::from_key("없는_모드"), ViewMode::Details);
        assert_eq!(ViewMode::from_key("Details"), ViewMode::Details);
    }

    #[test]
    fn 작은_아이콘과_목록은_흐름이_다르다() {
        // 사용자 요청 8번의 핵심 구분 — 둘 다 16px이지만 채우는 방향이 반대다
        assert_eq!(ViewMode::SmallIcons.flow(), Flow::Horizontal);
        assert_eq!(ViewMode::List.flow(), Flow::Vertical);
        assert_eq!(ViewMode::SmallIcons.icon_px(), ViewMode::List.icon_px());
    }

    #[test]
    fn 열_수는_뷰포트_폭에_맞춰_정해진다() {
        // 큰 아이콘 셀 폭 120 + 간격 8 → 128마다 한 칸, 마지막 칸은 간격이 없다
        let metrics = grid_metrics(ViewMode::LargeIcons, egui::vec2(400.0, 600.0), 10);
        assert_eq!(metrics.columns, 3, "120*3 + 8*2 = 376 <= 400");
        assert_eq!(metrics.rows, 4, "10개를 3열로 놓으면 4줄");
    }

    #[test]
    fn 폭이_한_칸보다_좁아도_한_열은_그린다() {
        // 0열이 되면 나눗셈에서 죽고, 아주 큰 아이콘은 좁은 패널에서 늘 이 경우다
        let metrics = grid_metrics(ViewMode::ExtraLargeIcons, egui::vec2(50.0, 600.0), 5);
        assert_eq!(metrics.columns, 1);
        assert_eq!(metrics.rows, 5);
    }

    #[test]
    fn 항목이_없으면_어느_흐름이든_빈_격자다() {
        // 세로 흐름(목록)은 줄 수를 뷰포트 높이로 정하므로, 0개 처리를 따로 하지 않으면
        // 그릴 것이 없는데도 콘텐츠 높이가 남아 스크롤 막대가 허공을 가리킨다
        for mode in [ViewMode::MediumIcons, ViewMode::List, ViewMode::Details] {
            let metrics = grid_metrics(mode, egui::vec2(800.0, 600.0), 0);
            assert_eq!(metrics.rows, 0, "{mode:?}: 줄이 남았다");
            assert_eq!(metrics.columns, 0, "{mode:?}: 열이 남았다");
            assert_eq!(
                metrics.content_size(),
                egui::vec2(0.0, 0.0),
                "{mode:?}: 콘텐츠 크기가 0이 아니다"
            );
            assert_eq!(metrics.visible_range(0.0, 600.0, 0), 0..0, "{mode:?}");
        }
    }

    #[test]
    fn 목록은_세로로_채운_뒤_오른쪽_열로_넘어간다() {
        // 높이 68 → 20+8=28 간격으로 2줄(20*2+8=48 <= 68, 3줄이면 76 > 68)
        let metrics = grid_metrics(ViewMode::List, egui::vec2(600.0, 68.0), 5);
        assert_eq!(metrics.rows, 2);
        assert_eq!(metrics.columns, 3, "5개를 2줄로 놓으면 3열");
        // 0,1은 첫 열 위아래 / 2는 둘째 열 맨 위
        assert_eq!(metrics.item_rect(0).min.x, 0.0);
        assert_eq!(metrics.item_rect(1).min.x, 0.0);
        assert!(metrics.item_rect(1).min.y > metrics.item_rect(0).min.y);
        assert!(metrics.item_rect(2).min.x > metrics.item_rect(0).min.x);
        assert_eq!(metrics.item_rect(2).min.y, 0.0);
    }

    #[test]
    fn 아이콘_격자는_가로로_채운_뒤_다음_줄로_넘어간다() {
        let metrics = grid_metrics(ViewMode::LargeIcons, egui::vec2(400.0, 600.0), 7);
        assert_eq!(metrics.columns, 3);
        // 0,1,2는 첫 줄 / 3은 둘째 줄 맨 왼쪽
        assert_eq!(metrics.item_rect(0).min.y, metrics.item_rect(2).min.y);
        assert!(metrics.item_rect(1).min.x > metrics.item_rect(0).min.x);
        assert_eq!(metrics.item_rect(3).min.x, 0.0);
        assert!(metrics.item_rect(3).min.y > metrics.item_rect(0).min.y);
    }

    #[test]
    fn 칸은_서로_겹치지_않는다() {
        // 간격 계산이 틀리면 아이콘·이름이 옆 칸을 침범한다
        let metrics = grid_metrics(ViewMode::MediumIcons, egui::vec2(500.0, 400.0), 12);
        for index in 0..11 {
            let a = metrics.item_rect(index);
            for other in (index + 1)..12 {
                let b = metrics.item_rect(other);
                assert!(!a.intersects(b), "{index}번과 {other}번 칸이 겹친다");
            }
        }
    }

    #[test]
    fn 콘텐츠_크기는_칸과_간격의_합이다() {
        let metrics = grid_metrics(ViewMode::LargeIcons, egui::vec2(400.0, 600.0), 7);
        // 3열 3줄 → 120*3 + 8*2 = 376 / 150*3 + 8*2 = 466
        assert_eq!(metrics.content_size(), egui::vec2(376.0, 466.0));
    }

    #[test]
    fn 보이는_범위만_돌려준다() {
        // 1만 개를 다 그리면 스크롤이 멈춘다 (NFR-3)
        let metrics = grid_metrics(ViewMode::MediumIcons, egui::vec2(500.0, 400.0), 10_000);
        let range = metrics.visible_range(0.0, 400.0, 10_000);
        assert!(range.start == 0);
        assert!(
            range.len() < 100,
            "화면에 보이는 것보다 훨씬 많이 그리려 한다: {}",
            range.len()
        );
    }

    #[test]
    fn 스크롤을_내리면_범위도_내려간다() {
        let metrics = grid_metrics(ViewMode::MediumIcons, egui::vec2(500.0, 400.0), 10_000);
        let top = metrics.visible_range(0.0, 400.0, 10_000);
        let down = metrics.visible_range(4000.0, 4400.0, 10_000);
        assert!(down.start > top.end, "스크롤해도 같은 범위를 그린다");
        assert!(down.end <= 10_000);
    }

    #[test]
    fn 범위는_항목_수를_넘지_않는다() {
        // 마지막 줄이 덜 찬 경우 — 넘으면 인덱싱에서 죽는다
        let metrics = grid_metrics(ViewMode::LargeIcons, egui::vec2(400.0, 600.0), 7);
        let range = metrics.visible_range(0.0, 10_000.0, 7);
        assert!(range.end <= 7);
    }

    #[test]
    fn 자세히와_내용은_한_줄에_하나씩이다() {
        for mode in [ViewMode::Details, ViewMode::Content] {
            let metrics = grid_metrics(mode, egui::vec2(800.0, 600.0), 5);
            assert_eq!(metrics.columns, 1, "{mode:?}");
            assert_eq!(metrics.rows, 5, "{mode:?}");
        }
    }
}
