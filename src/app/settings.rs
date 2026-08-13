//! 세션 저장/복원 — %APPDATA%\MOA\settings.json (FR-11·FR-20, plan D9·D18, NFR-7)
//!
//! 스키마 v3: v2 + {sites, queue[], dock} 이며 탭이 문자열에서 `TabSession{kind,path,site}`로
//! 넓어졌다 (FR-44). v2 파일은 **승격**되어 그대로 살아난다(`promote_v2`) — 원격 쪽 필드는
//! 비어 있는 기본값이 된다. v1과 손상·미래 버전은 종전대로 "세션 없음"으로 폴백한다.
//!
//! 스키마 v2: {version, window{x,y,w,h,maximized}, sidebar{width,collapsed}, active_workspace,
//! workspaces[{name, layout<트리 재귀>, panels[{tabs,active_tab}], active_panel}]}.
//! 각 워크스페이스의 panels 배열은 그 layout 리프의 walk 순서(좌→우, 상→하)와 1:1 대응한다.
//! 히스토리는 저장하지 않는다 — 경로만 (D15: 재시작 후 히스토리 초기화는 관례적 체감).
//! 손상·구버전(v1)·미래 version 파일은 전부 "세션 없음"으로 폴백한다 (사용자 결정: 마이그레이션 없음).
use crate::app::layout::{SplitDir, TreeShape};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// 현재 스키마 버전 — 필드가 바뀌면 올리고 하위 호환 처리를 추가한다 (D15).
/// v1(워크스페이스 개념 이전) 파일은 폴백되어 기본 워크스페이스 1개로 시작한다
pub const SESSION_VERSION: u32 = 3;

/// 승격해 읽을 수 있는 가장 낮은 버전 (D17) — v1은 워크스페이스 개념 이전이라 폴백이다
const PROMOTABLE_VERSION: u32 = 2;

/// 세션에 담는 전송 큐 항목 수의 상한 (plan Halt Forecast ii-a).
///
/// 큐가 1만 건이어도 저장은 앞의 1000건까지만 한다 — 세션 파일은 시작할 때 통째로 읽히므로
/// 무한정 커지면 창이 뜨는 시간이 그만큼 늘어난다. 넘친 것은 조용히 버린다(전송은 다시
/// 끌어다 놓으면 된다)
pub const QUEUE_SESSION_LIMIT: usize = 1000;

/// 사이드바 기본·최소·최대 폭(px, 96DPI 기준 — plan `## 시각 요소 분해`).
/// 저장값 검증이 이 범위를 쓰므로 세션 모듈이 소유하고, 사이드바 창(T4·T7)이 같은 상수를 참조한다
pub const SIDEBAR_DEFAULT_WIDTH: i32 = 260;
pub const SIDEBAR_MIN_WIDTH: i32 = 160;
pub const SIDEBAR_MAX_WIDTH: i32 = 480;

/// 저장 파일명을 품는 앱 폴더 (%APPDATA% 하위)
const APP_DIR: &str = "MOA";
/// 앱 이름이 `FileExplorer`이던 시절의 폴더 — 처음 한 번 파일을 옮겨 오는 데만 쓴다
const LEGACY_APP_DIR: &str = "FileExplorer";
const FILE_NAME: &str = "settings.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub version: u32,
    pub window: WindowState,
    pub sidebar: SidebarSession,
    /// 재시작 시 화면에 띄울 워크스페이스 (workspaces 인덱스)
    pub active_workspace: usize,
    pub workspaces: Vec<WorkspaceSession>,
    /// 등록된 사이트 (FR-44). v2 파일에는 없어 승격 시 빈 목록이 된다
    #[serde(default)]
    pub sites: SiteSession,
    /// 아직 끝나지 않은 전송들 — **다시 시작하지는 않는다**(FR-44)
    #[serde(default)]
    pub queue: Vec<QueueSession>,
    /// 하단 도크의 열림 상태
    #[serde(default)]
    pub dock: DockSession,
}

/// 저장되는 사이트 목록.
///
/// `remote::sites::SiteStore`를 **그대로** 담는다 — 필드를 하나하나 옮겨 적는 사본을 두면
/// 사이트에 항목이 늘 때마다(문자셋·동시 연결 수가 그랬다) 두 곳을 함께 고쳐야 하고,
/// 한쪽만 고쳐지면 저장은 되는데 복원이 안 되는 조용한 손실이 생긴다.
/// 비밀번호는 이 타입 안에서 이미 DPAPI로 봉인돼 있다(평문은 어디에도 없다 — FR-28)
pub type SiteSession = crate::remote::sites::SiteStore;

/// 저장되는 전송 큐 항목 하나 (FR-44).
///
/// **끝난 것은 담지 않는다** — 되살릴 이유가 없다. 담는 것은 대기·실패뿐이고, 복원 뒤에도
/// 스스로 시작하지 않는다(연결부터 사용자가 연다)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueSession {
    /// 어느 사이트의 전송인가 — 그 사이트가 사라졌으면 복원 때 이 항목을 버린다
    pub site: u32,
    /// `upload`/`download`
    pub direction: String,
    pub local: String,
    pub remote: String,
    #[serde(default)]
    pub size: u64,
    /// 실패로 저장된 것이면 그 사유 — 비어 있으면 대기였다
    #[serde(default)]
    pub error: String,
}

/// 하단 도크의 상태 (FR-36·FR-40)
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DockSession {
    /// `queue`/`log`, 빈 문자열이면 닫혀 있었다
    #[serde(default)]
    pub panel: String,
    /// 큐 화면의 거르개 키
    #[serde(default)]
    pub filter: String,
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

/// 탭 하나가 가리키는 곳 (FR-44).
///
/// v2까지는 문자열 하나(로컬 경로)였다 — 그 형태도 그대로 읽는다(`Deserialize` 참조)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TabSession {
    /// `local`/`remote`
    pub kind: String,
    /// 로컬 경로 또는 원격 경로
    pub path: String,
    /// 원격 탭이 가리키는 사이트 — 로컬이면 `None`
    pub site: Option<u32>,
}

/// 원격 탭이 담는 것 — 사이트와 그 안의 경로 (plan 신규 심볼 `RemoteTabSession`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteTabSession {
    pub site: u32,
    pub path: String,
}

impl TabSession {
    pub const LOCAL: &'static str = "local";
    pub const REMOTE: &'static str = "remote";

    pub fn local(path: String) -> TabSession {
        TabSession {
            kind: TabSession::LOCAL.to_owned(),
            path,
            site: None,
        }
    }

    pub fn remote(remote: RemoteTabSession) -> TabSession {
        TabSession {
            kind: TabSession::REMOTE.to_owned(),
            path: remote.path,
            site: Some(remote.site),
        }
    }

    /// 원격 탭이면 그 사이트와 경로 — 로컬이면 `None`
    pub fn as_remote(&self) -> Option<RemoteTabSession> {
        if self.kind != TabSession::REMOTE {
            return None;
        }
        Some(RemoteTabSession {
            site: self.site?,
            path: self.path.clone(),
        })
    }
}

impl From<&str> for TabSession {
    /// 시험과 레거시 어댑트가 로컬 탭을 짧게 적기 위한 길
    fn from(path: &str) -> TabSession {
        TabSession::local(path.to_owned())
    }
}

impl<'de> Deserialize<'de> for TabSession {
    /// v2의 문자열 탭과 v3의 객체 탭을 **둘 다** 읽는다.
    ///
    /// 승격(`promote_v2`)이 형태까지 바꾸려면 세션 전체를 두 번 파싱해야 한다 —
    /// 여기서 받아들이면 한 번으로 끝난다
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<TabSession, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Legacy(String),
            Tab {
                kind: String,
                path: String,
                #[serde(default)]
                site: Option<u32>,
            },
        }
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Legacy(path) => TabSession::local(path),
            Repr::Tab { kind, path, site } => TabSession { kind, path, site },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PanelSession {
    /// 탭이 가리키는 곳들 (탭 순서)
    pub tabs: Vec<TabSession>,
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

/// 앱 이름을 바꾸기 전 폴더에 있던 설정을 새 폴더로 **복사**해 온다.
///
/// 새 폴더에 이미 파일이 있으면 아무것도 하지 않는다 — 새 이름으로 한 번이라도 저장했으면
/// 그쪽이 최신이다. 옛 파일은 지우지 않는다: 복사가 어긋나도 되돌릴 자리를 남긴다.
/// 실패는 조용히 넘긴다(기본 레이아웃으로 뜰 뿐이다 — 저장 실패와 같은 규약)
fn migrate_from_legacy_dir(path: &Path) {
    if path.exists() {
        return;
    }
    let Some(base) = std::env::var_os("APPDATA") else {
        return;
    };
    let legacy = PathBuf::from(base).join(LEGACY_APP_DIR).join(FILE_NAME);
    if !legacy.exists() {
        return;
    }
    if let Some(dir) = path.parent()
        && std::fs::create_dir_all(dir).is_err()
    {
        return;
    }
    let _ = std::fs::copy(&legacy, path);
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
    let path = settings_path()?;
    migrate_from_legacy_dir(&path);
    let text = std::fs::read_to_string(path).ok()?;
    parse_session(&text)
}

/// 파싱 + 무결성 검증 (파일 I/O와 분리 — 단위테스트 대상).
/// 사이드바 폭만 클램프로 교정하고(정상 사용 중에도 범위를 벗어날 수 있음 — D9),
/// 나머지 위반은 파일 오염으로 보고 전체 폴백한다
pub fn parse_session(text: &str) -> Option<Session> {
    let mut session: Session = serde_json::from_str(text).ok()?;
    match session.version {
        SESSION_VERSION => {}
        // v2는 승격해 살린다 — 워크스페이스·분할·탭·열 폭·보기 모드가 그대로 남는다 (D17)
        PROMOTABLE_VERSION => session = promote_v2(session),
        // v1과 미래 버전 — 기본 레이아웃 폴백 (D15)
        _ => return None,
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
        // 알 수 없는 탭 종류는 파일 오염으로 본다 — 조용히 로컬로 취급하면 원격 경로가
        // 로컬 경로인 척 되살아난다(quality 리뷰 m1). 종류가 늘어나면 버전을 올린다
        if ws.panels.iter().any(|p| {
            p.tabs.iter().any(|tab| {
                !matches!(tab.kind.as_str(), TabSession::LOCAL | TabSession::REMOTE)
                    || (tab.kind == TabSession::REMOTE && tab.site.is_none())
            })
        }) {
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

/// v2 세션을 v3로 올린다 (D17).
///
/// 원격 쪽은 v2에 **아무것도 없으므로** 전부 기본값이다(사이트 없음·큐 없음·도크 닫힘).
/// 탭은 읽는 자리(`TabSession`의 `Deserialize`)에서 이미 로컬 탭으로 받아들여져 있다.
///
/// **마이그레이션 프레임워크를 만들지 않는다**(plan 비추상화 선언) — 한 단계뿐이고,
/// 다음 단계가 생기면 그때 v3→v4 함수를 하나 더 둔다
pub fn promote_v2(session: Session) -> Session {
    Session {
        version: SESSION_VERSION,
        ..session
    }
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
            sites: SiteSession::default(),
            queue: Vec::new(),
            dock: DockSession::default(),
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

    /// v2 파일 원문 — 원격 관련 필드가 하나도 없고 탭이 문자열 배열이다
    fn v2_text() -> String {
        r#"{
            "version": 2,
            "window": {"x": 10, "y": 20, "w": 1280, "h": 800, "maximized": false},
            "sidebar": {"width": 300, "collapsed": true},
            "active_workspace": 0,
            "workspaces": [{
                "name": "작업",
                "layout": {"Split": {"horizontal": true, "ratio": 0.5,
                    "first": "Leaf", "second": "Leaf"}},
                "panels": [
                    {"tabs": ["C:\\Users", "D:\\"], "active_tab": 1,
                     "columns": [200.0, 60.0, 120.0, 90.0], "view_mode": "tiles"},
                    {"tabs": ["C:\\Windows"], "active_tab": 0}
                ],
                "active_panel": 1
            }]
        }"#
        .to_owned()
    }

    #[test]
    fn v2_파일은_승격되어_그대로_살아난다() {
        // Acceptance ① — 워크스페이스·분할·탭·열 폭·보기 모드가 전부 보존된다 (D17)
        let session = parse_session(&v2_text()).expect("v2는 승격돼야 한다");
        assert_eq!(session.version, SESSION_VERSION);
        assert_eq!(session.sidebar.width, 300);
        assert!(session.sidebar.collapsed);
        let workspace = &session.workspaces[0];
        assert_eq!(workspace.name, "작업");
        assert_eq!(workspace.active_panel, 1);
        assert_eq!(workspace.panels[0].active_tab, 1);
        assert_eq!(workspace.panels[0].columns, vec![200.0, 60.0, 120.0, 90.0]);
        assert_eq!(workspace.panels[0].view_mode, "tiles");
        // 문자열 탭은 로컬 탭으로 받아들여진다
        let tabs: Vec<&str> = workspace.panels[0]
            .tabs
            .iter()
            .map(|tab| tab.path.as_str())
            .collect();
        assert_eq!(tabs, vec![r"C:\Users", r"D:\"]);
        assert!(
            workspace.panels[0]
                .tabs
                .iter()
                .all(|tab| tab.site.is_none())
        );
        // 원격 쪽은 전부 기본값이다 (plan Edge Case)
        assert!(session.sites.is_empty());
        assert!(session.queue.is_empty());
        assert_eq!(session.dock, DockSession::default());
    }

    #[test]
    fn v1과_미래_버전은_종전대로_폴백이다() {
        // Acceptance ⑥ — 승격 경로는 v2 하나뿐이다
        let v1 = v2_text().replace(r#""version": 2"#, r#""version": 1"#);
        assert!(parse_session(&v1).is_none());
        let v4 = v2_text().replace(r#""version": 2"#, r#""version": 4"#);
        assert!(parse_session(&v4).is_none());
        assert!(parse_session("{망가진").is_none());
    }

    #[test]
    fn 원격_탭과_큐가_왕복해도_같다() {
        // Acceptance ② — v3 왕복이 동일하다
        let mut session = sample();
        session.workspaces[0].panels[0].tabs = vec![
            TabSession::local(r"C:\Users".to_owned()),
            TabSession::remote(RemoteTabSession {
                site: 7,
                path: "/var/www".to_owned(),
            }),
        ];
        session.workspaces[0].panels[0].active_tab = 1;
        session.queue = vec![QueueSession {
            site: 7,
            direction: "upload".to_owned(),
            local: r"C:\work\app.js".to_owned(),
            remote: "/var/www/app.js".to_owned(),
            size: 1234,
            error: String::new(),
        }];
        session.dock = DockSession {
            panel: "log".to_owned(),
            filter: "error".to_owned(),
        };

        let text = serde_json::to_string(&session).expect("직렬화");
        let back = parse_session(&text).expect("왕복");
        assert_eq!(back, session);
        let remote_tab = back.workspaces[0].panels[0].tabs[1]
            .as_remote()
            .expect("원격 탭");
        assert_eq!(remote_tab.site, 7);
        assert_eq!(remote_tab.path, "/var/www");
    }

    #[test]
    fn 알_수_없는_탭_종류는_폴백이다() {
        // quality 리뷰 m1 — 조용히 로컬로 취급하면 원격 경로가 로컬 경로인 척 되살아난다
        let mut session = sample();
        session.workspaces[0].panels[0].tabs = vec![TabSession {
            kind: "무엇".to_owned(),
            path: "/var/www".to_owned(),
            site: Some(1),
        }];
        session.workspaces[0].panels[0].active_tab = 0;
        let text = serde_json::to_string(&session).expect("직렬화");
        assert!(parse_session(&text).is_none(), "모르는 종류를 받아들였다");

        // 사이트를 잃은 원격 탭도 파일 오염이다 — 어느 사이트인지 알 수 없다
        session.workspaces[0].panels[0].tabs = vec![TabSession {
            kind: TabSession::REMOTE.to_owned(),
            path: "/var/www".to_owned(),
            site: None,
        }];
        let text = serde_json::to_string(&session).expect("직렬화");
        assert!(
            parse_session(&text).is_none(),
            "사이트 없는 원격 탭을 받아들였다"
        );
    }
}
