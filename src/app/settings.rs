//! 세션 저장/복원 — %APPDATA%\FileExplorer\settings.json (FR-11, plan D15, NFR-7)
//!
//! 스키마(D15): {version, window{x,y,w,h,maximized}, layout<트리 재귀>, panels[{tabs,active_tab}]}.
//! panels 배열은 layout 리프의 walk 순서(좌→우, 상→하)와 1:1 대응한다.
//! 히스토리는 저장하지 않는다 — 경로만 (D15: 재시작 후 히스토리 초기화는 관례적 체감).
//! 손상·구버전·미래 version 파일은 전부 "세션 없음"으로 폴백한다 (T4 Edge).
use crate::app::layout::{SplitDir, TreeShape};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 현재 스키마 버전 — 필드가 바뀌면 올리고 하위 호환 처리를 추가한다 (D15)
pub const SESSION_VERSION: u32 = 1;

/// 저장 파일명을 품는 앱 폴더 (%APPDATA% 하위)
const APP_DIR: &str = "FileExplorer";
const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub window: WindowState,
    pub layout: LayoutNode,
    pub panels: Vec<PanelSession>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelSession {
    /// 탭별 폴더 경로 (탭 순서)
    pub tabs: Vec<String>,
    pub active_tab: usize,
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

/// 파싱 + 무결성 검증 (파일 I/O와 분리 — 단위테스트 대상)
pub fn parse_session(text: &str) -> Option<Session> {
    let session: Session = serde_json::from_str(text).ok()?;
    if session.version != SESSION_VERSION {
        return None; // 미래/과거 버전 — 기본 레이아웃 폴백 (D15)
    }
    // panels는 layout 리프와 1:1 — 어긋나면 파일 오염으로 보고 전체 폴백
    if session.panels.len() != session.layout.leaf_count() {
        return None;
    }
    if session
        .panels
        .iter()
        .any(|p| p.tabs.is_empty() || p.active_tab >= p.tabs.len())
    {
        return None;
    }
    if !layout_ratios_valid(&session.layout) {
        return None;
    }
    if session.window.w <= 0 || session.window.h <= 0 {
        return None;
    }
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
                },
                PanelSession {
                    tabs: vec!["C:\\".into()],
                    active_tab: 0,
                },
                PanelSession {
                    tabs: vec!["C:\\Windows".into()],
                    active_tab: 0,
                },
            ],
        }
    }

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
    fn 패널_리프_수_불일치는_폴백이다() {
        let mut s = sample();
        s.panels.pop();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 빈_탭이나_범위_밖_활성은_폴백이다() {
        let mut s = sample();
        s.panels[1].tabs.clear();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);

        let mut s = sample();
        s.panels[0].active_tab = 9;
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(parse_session(&json), None);
    }

    #[test]
    fn 레이아웃_스냅숏_상호_변환_왕복() {
        let s = sample();
        let shape = s.layout.to_shape();
        assert_eq!(LayoutNode::from_shape(&shape), s.layout);
    }
}
