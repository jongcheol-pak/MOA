//! 세션 저장/복원 — %APPDATA%\FileExplorer\settings.json (FR-11·FR-20, plan D9·D18, NFR-7)
//!
//! 스키마 v2: {version, window{x,y,w,h,maximized}, sidebar{width,collapsed}, active_workspace,
//! workspaces[{name, layout<트리 재귀>, panels[{tabs,active_tab}], active_panel}]}.
//! 각 워크스페이스의 panels 배열은 그 layout 리프의 walk 순서(좌→우, 상→하)와 1:1 대응한다.
//! 히스토리는 저장하지 않는다 — 경로만 (D15: 재시작 후 히스토리 초기화는 관례적 체감).
//! 손상·구버전(v1)·미래 version 파일은 전부 "세션 없음"으로 폴백한다 (사용자 결정: 마이그레이션 없음).
use crate::app::layout::{SplitDir, TreeShape};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 현재 스키마 버전 — 필드가 바뀌면 올리고 하위 호환 처리를 추가한다 (D15).
/// v1(워크스페이스 개념 이전) 파일은 폴백되어 기본 워크스페이스 1개로 시작한다
pub const SESSION_VERSION: u32 = 2;

/// 사이드바 기본·최소·최대 폭(px, 96DPI 기준 — plan `## 시각 요소 분해`).
/// 저장값 검증이 이 범위를 쓰므로 세션 모듈이 소유하고, 사이드바 창(T4·T7)이 같은 상수를 참조한다
pub const SIDEBAR_DEFAULT_WIDTH: i32 = 260;
pub const SIDEBAR_MIN_WIDTH: i32 = 160;
pub const SIDEBAR_MAX_WIDTH: i32 = 480;

/// 저장 파일명을 품는 앱 폴더 (%APPDATA% 하위)
const APP_DIR: &str = "FileExplorer";
const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub window: WindowState,
    pub sidebar: SidebarSession,
    /// 재시작 시 화면에 띄울 워크스페이스 (workspaces 인덱스)
    pub active_workspace: usize,
    pub workspaces: Vec<WorkspaceSession>,
}

/// 사이드바 표시 상태 (FR-19·FR-20) — 창 내부 상태 타입 `sidebar::SidebarState`와 구분해 `Session` 접미사
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SidebarSession {
    pub width: i32,
    pub collapsed: bool,
}

impl Default for SidebarSession {
    fn default() -> SidebarSession {
        SidebarSession {
            width: SIDEBAR_DEFAULT_WIDTH,
            collapsed: false,
        }
    }
}

/// 워크스페이스 한 벌 — 이름 + 분할 구조 + 패널별 탭 (FR-20)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub name: String,
    pub layout: LayoutNode,
    pub panels: Vec<PanelSession>,
    /// 사이드바 부제(경로) 산출에 쓰는 활성 패널 인덱스 (D18 — 승격 시 활성 패널 복원까지는 하지 않는다)
    pub active_panel: usize,
}

/// 창 위치·크기 (일반 상태 기준 사각형) + 최대화 여부
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub maximized: bool,
}

/// 분할 트리 (직렬화 전용 미러 — layout::TreeShape와 상호 변환)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutNode {
    Leaf,
    Split {
        horizontal: bool,
        ratio: f32,
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PanelSession {
    /// 탭별 폴더 경로 (탭 순서)
    pub tabs: Vec<String>,
    pub active_tab: usize,
    /// 자세히 보기 열 폭 4개 (이름·크기·종류·수정한 날짜). 패널마다 독립이다.
    ///
    /// **필드가 없는 옛 파일도 그대로 읽혀야 하므로 `default`를 쓴다** — 스키마 버전을 올리면
    /// `parse_session`이 통째로 폴백해 워크스페이스·분할·탭까지 초기화된다 (plan D5).
    /// 빈 벡터는 "저장된 폭 없음"이며 복원 시 기본 폭이 된다
    #[serde(default)]
    pub columns: Vec<f32>,
    /// 보기 모드 키 (FR-23). 빈 문자열은 "저장 안 됨"이며 복원 시 기본값(자세히)이 된다.
    /// 열 폭과 같은 이유로 `default`를 쓴다 — 스키마 버전을 올리면 옛 세션이 통째로 버려진다
    #[serde(default)]
    pub view_mode: String,
}

impl LayoutNode {
    fn leaf_count(&self) -> usize {
        match self {
            LayoutNode::Leaf => 1,
            LayoutNode::Split { first, second, .. } => first.leaf_count() + second.leaf_count(),
        }
    }

    /// 직렬화 노드 → 레이아웃 스냅숏
    pub fn to_shape(&self) -> TreeShape {
        match self {
            LayoutNode::Leaf => TreeShape::Leaf,
            LayoutNode::Split {
                horizontal,
                ratio,
                first,
                second,
            } => TreeShape::Split {
                dir: if *horizontal {
                    SplitDir::Horizontal
                } else {
                    SplitDir::Vertical
                },
                ratio: *ratio,
                first: Box::new(first.to_shape()),
                second: Box::new(second.to_shape()),
            },
        }
    }

    /// 레이아웃 스냅숏 → 직렬화 노드
    pub fn from_shape(shape: &TreeShape) -> LayoutNode {
        match shape {
            TreeShape::Leaf => LayoutNode::Leaf,
            TreeShape::Split {
                dir,
                ratio,
                first,
                second,
            } => LayoutNode::Split {
                horizontal: matches!(dir, SplitDir::Horizontal),
                ratio: *ratio,
                first: Box::new(LayoutNode::from_shape(first)),
                second: Box::new(LayoutNode::from_shape(second)),
            },
        }
    }
}

/// 세션 파일 경로 — %APPDATA% 미설정(비정상 환경)이면 None
fn settings_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|base| PathBuf::from(base).join(APP_DIR).join(FILE_NAME))
}

/// 종료 시 저장 — 디렉터리가 없으면 생성. 실패는 조용히 생략 (T4 Edge: 디스크 풀 등 —
/// 다음 실행은 이전/기본값으로 뜬다)
pub fn save_session(session: &Session) {
    let Some(path) = settings_path() else {
        return;
    };
    let Ok(json) = serde_json::to_string_pretty(session) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, json);
}

/// 시작 시 로드 — 없음/손상/버전 불일치/무결성 위반은 전부 None (기본 레이아웃 폴백)
pub fn load_session() -> Option<Session> {
    let text = std::fs::read_to_string(settings_path()?).ok()?;
    parse_session(&text)
}

/// 파싱 + 무결성 검증 (파일 I/O와 분리 — 단위테스트 대상).
/// 사이드바 폭만 클램프로 교정하고(정상 사용 중에도 범위를 벗어날 수 있음 — D9),
/// 나머지 위반은 파일 오염으로 보고 전체 폴백한다
pub fn parse_session(text: &str) -> Option<Session> {
    let mut session: Session = serde_json::from_str(text).ok()?;
    if session.version != SESSION_VERSION {
        return None; // 미래/과거 버전 — 기본 레이아웃 폴백 (D15)
    }
    if session.workspaces.is_empty() || session.active_workspace >= session.workspaces.len() {
        return None;
    }
    for ws in &session.workspaces {
        // panels는 layout 리프와 1:1 — 어긋나면 파일 오염으로 보고 전체 폴백
        if ws.panels.len() != ws.layout.leaf_count() {
            return None;
        }
        if ws.active_panel >= ws.panels.len() {
            return None;
        }
        if ws
            .panels
            .iter()
            .any(|p| p.tabs.is_empty() || p.active_tab >= p.tabs.len())
        {
            return None;
        }
        if !layout_ratios_valid(&ws.layout) {
            return None;
        }
    }
    if session.window.w <= 0 || session.window.h <= 0 {
        return None;
    }
    session.sidebar.width = session
        .sidebar
        .width
        .clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH);
    Some(session)
}

/// 비율 유한성 검사 (NaN/무한대 오염 방어 — 재구성 clamp의 1차 관문)
fn layout_ratios_valid(node: &LayoutNode) -> bool {
    match node {
        LayoutNode::Leaf => true,
        LayoutNode::Split {
            ratio,
            first,
            second,
            ..
        } => ratio.is_finite() && layout_ratios_valid(first) && layout_ratios_valid(second),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Session {
        Session {
            version: SESSION_VERSION,
            window: WindowState {
                x: 100,
                y: 50,
                w: 1200,
                h: 800,
                maximized: false,
            },
            sidebar: SidebarSession {
                width: 300,
                collapsed: false,
            },
            active_workspace: 1,
            workspaces: vec![
                WorkspaceSession {
                    name: "워크스페이스 1".into(),
                    layout: LayoutNode::Split {
                        horizontal: true,
                        ratio: 0.4,
                        first: Box::new(LayoutNode::Leaf),
                        second: Box::new(LayoutNode::Split {
                            horizontal: false,
                            ratio: 0.5,
                            first: Box::new(LayoutNode::Leaf),
                            second: Box::new(LayoutNode::Leaf),
                        }),
                    },
                    panels: vec![
                        PanelSession {
                            tabs: vec!["C:\\Users".into(), "D:\\".into()],
                            active_tab: 1,
                            ..Default::default()
                        },
                        PanelSession {
                            tabs: vec!["C:\\".into()],
                            active_tab: 0,
                            ..Default::default()
                        },
                        PanelSession {
                            tabs: vec!["C:\\Windows".into()],
                            active_tab: 0,
                            ..Default::default()
                        },
                    ],
                    active_panel: 2,
                },
                WorkspaceSession {
                    name: "자료 정리".into(),
                    layout: LayoutNode::Leaf,
                    panels: vec![PanelSession {
                        tabs: vec!["D:\\작업".into()],
                        active_tab: 0,
                        ..Default::default()
                    }],
                    active_panel: 0,
                },
            ],
        }
    }

    /// v1(워크스페이스 이전) 스키마 원문 — 폴백 검증용
    const V1_JSON: &str = r#"{
        "version": 1,
        "window": {"x": 0, "y": 0, "w": 1200, "h": 800, "maximized": false},
        "layout": "Leaf",
        "panels": [{"tabs": ["C:\\"], "active_tab": 0}]
    }"#;

    #[test]
    fn 직렬화_역직렬화_왕복_동일성() {
        let s = sample();
        let json = serde_json::to_string_pretty(&s).unwrap();
        let back = parse_session(&json).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn 손상_json은_기본값_폴백이다() {
        assert_eq!(parse_session("{invalid json"), None);
        assert_eq!(parse_session(""), None);
        assert_eq!(parse_session("{}"), None);
    }

    #[test]
    fn 미래_버전은_폴백이다() {
        let mut s = sample();
        s.version = SESSION_VERSION + 1;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 구버전_v1_파일은_폴백이다() {
        // 사용자 결정: 마이그레이션 없이 초기화 (기본 워크스페이스 1개로 시작)
        assert_eq!(parse_session(V1_JSON), None);
    }

    #[test]
    fn 패널_리프_수_불일치는_폴백이다() {
        let mut s = sample();
        s.workspaces[0].panels.pop();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 빈_탭이나_범위_밖_활성은_폴백이다() {
        let mut s = sample();
        s.workspaces[0].panels[1].tabs.clear();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);

        let mut s = sample();
        s.workspaces[0].panels[0].active_tab = 9;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 빈_워크스페이스_목록이나_범위_밖_활성_워크스페이스는_폴백이다() {
        let mut s = sample();
        s.workspaces.clear();
        s.active_workspace = 0;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);

        let mut s = sample();
        s.active_workspace = 9;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 범위_밖_활성_패널은_폴백이다() {
        let mut s = sample();
        s.workspaces[0].active_panel = 3; // 패널은 3개(0~2)
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 사이드바_폭은_범위로_클램프된다() {
        let mut s = sample();
        s.sidebar.width = 100;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(
            parse_session(&json).unwrap().sidebar.width,
            SIDEBAR_MIN_WIDTH
        );

        let mut s = sample();
        s.sidebar.width = 9999;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(
            parse_session(&json).unwrap().sidebar.width,
            SIDEBAR_MAX_WIDTH
        );
    }

    #[test]
    fn 레이아웃_스냅숏_상호_변환_왕복() {
        let s = sample();
        let shape = s.workspaces[0].layout.to_shape();
        assert_eq!(LayoutNode::from_shape(&shape), s.workspaces[0].layout);
    }
}
