//! 앱 상태 ↔ 세션 파일 변환 (FR-11·FR-20, NFR-7).
//!
//! 저장 스키마(`app::settings::Session`)는 그대로 쓴다 — 현행 판이 쓰던 v2 그대로이며
//! 이 이식에서 형식을 바꾸지 않는다. 이 모듈은 그 스키마와 앱 상태 사이를 옮기는 **순수 변환**만
//! 담당해 UI 없이 왕복을 검증할 수 있게 한다. eframe의 `persistence`는 쓰지 않는다(D8) —
//! `settings.json`이 정본이며 저장 경로가 둘이 되면 서로 어긋난다.
use crate::app::layout::TreeShape;
use crate::app::settings::{
    LayoutNode, PanelSession, SESSION_VERSION, Session, SidebarSession, WindowState,
    WorkspaceSession,
};
use std::path::PathBuf;

/// 저장·복원 사이를 오가는 워크스페이스 한 벌.
/// UI 타입(`WorkspaceView`·`PanelState`)에 기대지 않아 단위 테스트에서 그대로 만들 수 있다
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceState {
    pub name: String,
    /// 분할 구조 (패널 id는 담지 않는다 — 복원 시 새로 부여된다)
    pub shape: TreeShape,
    /// 분할 트리 리프의 walk 순서와 1:1 (`settings` 스키마 계약)
    pub panels: Vec<PanelTabs>,
    pub active_panel: usize,
}

/// 패널 하나의 탭 구성과 목록 표시 상태
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelTabs {
    pub tabs: Vec<PathBuf>,
    pub active_tab: usize,
    /// 자세히 보기 열 폭 — 비면 기본 폭으로 시작한다
    pub columns: Vec<f32>,
    /// 보기 모드 키 — 비면 기본값(자세히)으로 시작한다
    pub view_mode: String,
}

/// 현재 상태를 저장 스키마로 옮긴다
pub fn to_session(
    window: WindowState,
    sidebar: SidebarSession,
    active_workspace: usize,
    workspaces: &[WorkspaceState],
) -> Session {
    Session {
        version: SESSION_VERSION,
        window,
        sidebar,
        active_workspace,
        workspaces: workspaces
            .iter()
            .map(|workspace| WorkspaceSession {
                name: workspace.name.clone(),
                layout: LayoutNode::from_shape(&workspace.shape),
                panels: workspace
                    .panels
                    .iter()
                    .map(|panel| PanelSession {
                        tabs: panel
                            .tabs
                            .iter()
                            .map(|path| path.to_string_lossy().into_owned())
                            .collect(),
                        active_tab: panel.active_tab,
                        columns: panel.columns.clone(),
                        view_mode: panel.view_mode.clone(),
                    })
                    .collect(),
                active_panel: workspace.active_panel,
            })
            .collect(),
    }
}

/// 저장된 세션에서 워크스페이스 목록을 되살린다.
///
/// 무결성(패널 수 = 리프 수, 활성 인덱스 범위, 빈 탭 없음)은 `settings::parse_session`이
/// 이미 확인한 뒤이므로 여기서는 형 변환만 한다
pub fn restore(session: &Session) -> Vec<WorkspaceState> {
    session
        .workspaces
        .iter()
        .map(|workspace| WorkspaceState {
            name: workspace.name.clone(),
            shape: workspace.layout.to_shape(),
            panels: workspace
                .panels
                .iter()
                .map(|panel| PanelTabs {
                    tabs: panel.tabs.iter().map(PathBuf::from).collect(),
                    active_tab: panel.active_tab,
                    columns: panel.columns.clone(),
                    view_mode: panel.view_mode.clone(),
                })
                .collect(),
            active_panel: workspace.active_panel,
        })
        .collect()
}

/// 저장된 창 위치를 화면 안으로 끌어온다.
///
/// 모니터를 떼거나 배치를 바꾸면 지난번 좌표가 화면 밖일 수 있고, 그대로 띄우면
/// 창이 보이지 않는다. 크기가 화면보다 크면 화면에 맞춰 줄인다
pub fn clamp_window(window: WindowState, monitor_w: i32, monitor_h: i32) -> WindowState {
    if monitor_w <= 0 || monitor_h <= 0 {
        return window; // 모니터 크기를 모르면(조회 실패) 저장값을 그대로 믿는다
    }
    let w = window.w.min(monitor_w);
    let h = window.h.min(monitor_h);
    WindowState {
        x: window.x.clamp(0, (monitor_w - w).max(0)),
        y: window.y.clamp(0, (monitor_h - h).max(0)),
        w,
        h,
        maximized: window.maximized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::layout::SplitDir;
    use crate::app::settings::parse_session;

    fn sample() -> Vec<WorkspaceState> {
        vec![
            WorkspaceState {
                name: "워크스페이스 1".into(),
                shape: TreeShape::Split {
                    dir: SplitDir::Horizontal,
                    ratio: 0.4,
                    first: Box::new(TreeShape::Leaf),
                    second: Box::new(TreeShape::Leaf),
                },
                panels: vec![
                    PanelTabs {
                        tabs: vec![PathBuf::from(r"C:\Users"), PathBuf::from(r"D:\")],
                        active_tab: 1,
                        columns: vec![200.0, 60.0, 120.0, 90.0],
                        view_mode: "tiles".into(),
                    },
                    PanelTabs {
                        tabs: vec![PathBuf::from(r"C:\Windows")],
                        active_tab: 0,
                        columns: Vec::new(),
                        view_mode: String::new(),
                    },
                ],
                active_panel: 1,
            },
            WorkspaceState {
                name: "자료 정리".into(),
                shape: TreeShape::Leaf,
                panels: vec![PanelTabs {
                    tabs: vec![PathBuf::from(r"D:\작업")],
                    active_tab: 0,
                    columns: Vec::new(),
                    view_mode: "large_icons".into(),
                }],
                active_panel: 0,
            },
        ]
    }

    fn window() -> WindowState {
        WindowState {
            x: 100,
            y: 50,
            w: 1200,
            h: 800,
            maximized: false,
        }
    }

    #[test]
    fn 워크스페이스는_왕복해도_같다() {
        let states = sample();
        let session = to_session(window(), SidebarSession::default(), 1, &states);
        assert_eq!(restore(&session), states);
    }

    #[test]
    fn 저장한_세션은_무결성_검사를_통과한다() {
        // 앱이 만든 세션이 자기 파서에 거부되면 재시작마다 조용히 초기화된다
        let session = to_session(window(), SidebarSession::default(), 0, &sample());
        let json = serde_json::to_string(&session).unwrap();
        assert_eq!(parse_session(&json), Some(session));
    }

    #[test]
    fn 열_폭_필드가_없는_옛_세션도_그대로_복원된다() {
        // 열 폭은 나중에 더한 필드다. 이것 때문에 스키마 버전을 올리면 `parse_session`이
        // 통째로 폴백해 **기존 사용자의 워크스페이스·분할·탭이 전부 초기화된다** (plan D5).
        // 그래서 버전을 2로 둔 채 `#[serde(default)]`로 더했고, 이 테스트가 그 계약을 지킨다
        let session = to_session(window(), SidebarSession::default(), 0, &sample());
        let json = serde_json::to_string(&session).unwrap();
        let without_columns = json
            .replace(",\"columns\":[200.0,60.0,120.0,90.0]", "")
            .replace(",\"columns\":[]", "")
            .replace(",\"view_mode\":\"tiles\"", "")
            .replace(",\"view_mode\":\"large_icons\"", "")
            .replace(",\"view_mode\":\"\"", "");
        assert!(
            !without_columns.contains("columns") && !without_columns.contains("view_mode"),
            "테스트가 새 필드를 실제로 걷어내지 못했다"
        );

        let parsed = parse_session(&without_columns).expect("옛 세션이 거부됐다");
        let restored = restore(&parsed);
        // 열 폭만 비고 나머지 구성은 그대로여야 한다
        assert_eq!(restored.len(), sample().len());
        assert_eq!(restored[0].shape, sample()[0].shape);
        assert_eq!(restored[0].panels[0].tabs, sample()[0].panels[0].tabs);
        assert_eq!(
            restored[0].panels[0].active_tab,
            sample()[0].panels[0].active_tab
        );
        assert!(restored[0].panels[0].columns.is_empty(), "없던 폭이 생겼다");
    }

    #[test]
    fn 열_폭은_패널마다_따로_왕복한다() {
        // 한 패널에서 조절한 폭이 다른 패널에 번지면 "패널마다 독립"이 깨진다
        let states = sample();
        let session = to_session(window(), SidebarSession::default(), 0, &states);
        let restored = restore(&session);
        assert_eq!(
            restored[0].panels[0].columns,
            vec![200.0, 60.0, 120.0, 90.0]
        );
        assert!(restored[0].panels[1].columns.is_empty());
    }

    #[test]
    fn 보기_모드는_패널마다_따로_왕복한다() {
        // 한 패널에서 고른 모드가 다른 패널에 번지면 "패널마다 독립"(FR-23)이 깨진다
        let session = to_session(window(), SidebarSession::default(), 0, &sample());
        let restored = restore(&session);
        assert_eq!(restored[0].panels[0].view_mode, "tiles");
        assert!(restored[0].panels[1].view_mode.is_empty());
        assert_eq!(restored[1].panels[0].view_mode, "large_icons");
    }

    #[test]
    fn 저장된_보기_모드_키는_실제_모드로_되살아난다() {
        // 세션에 담기는 것은 문자열이라, 그것이 모드로 되돌아오는지까지 확인해야
        // "재시작하면 모드가 유지된다"가 성립한다
        use crate::ui::view_mode::ViewMode;
        let session = to_session(window(), SidebarSession::default(), 0, &sample());
        let restored = restore(&session);
        assert_eq!(
            ViewMode::from_key(&restored[0].panels[0].view_mode),
            ViewMode::Tiles
        );
        assert_eq!(
            ViewMode::from_key(&restored[1].panels[0].view_mode),
            ViewMode::LargeIcons
        );
        // 빈 문자열은 기본값으로 — 저장된 적 없는 패널이다
        assert_eq!(
            ViewMode::from_key(&restored[0].panels[1].view_mode),
            ViewMode::Details
        );
    }

    #[test]
    fn 화면_밖_창은_안으로_들어온다() {
        let far = WindowState {
            x: 5000,
            y: -300,
            ..window()
        };
        let fixed = clamp_window(far, 1920, 1080);
        assert_eq!(fixed.x, 1920 - 1200);
        assert_eq!(fixed.y, 0);
        assert_eq!((fixed.w, fixed.h), (1200, 800));
    }

    #[test]
    fn 화면보다_큰_창은_화면_크기로_줄인다() {
        let huge = WindowState {
            x: 0,
            y: 0,
            w: 4000,
            h: 3000,
            maximized: false,
        };
        let fixed = clamp_window(huge, 1920, 1080);
        assert_eq!((fixed.x, fixed.y, fixed.w, fixed.h), (0, 0, 1920, 1080));
    }

    #[test]
    fn 모니터_크기를_모르면_저장값을_그대로_쓴다() {
        let w = window();
        assert_eq!(clamp_window(w.clone(), 0, 0), w);
    }
}
