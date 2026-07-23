//! 분할 레이아웃 트리 — 순수 로직 (HWND 비의존, 단위테스트 대상)
//!
//! 내부 노드 = 분할 방향+비율, 리프 = 패널 (plan D3 — VS Code 방식 트리).
//! 화면 배치는 `compute_rects`가 영역을 리프별 사각형과 스플리터 목록으로 계산한다.
// T3(레이아웃 렌더링)에서 소비 — 사용 시점에 expect가 자동 해제 경고를 낸다
// (테스트 빌드는 테스트가 이미 소비하므로 non-test에만 적용)
#![cfg_attr(not(test), expect(dead_code))]

/// 패널 최소 폭/높이(px, 96DPI 기준). 이보다 작아지는 분할은 거부한다 (plan T2 Edge)
pub const MIN_PANE_SIZE: i32 = 120;
/// 스플리터 두께(px)
pub const SPLITTER_THICKNESS: i32 = 4;

/// 리프(패널) 식별자 — 생성 순서 증가, 재사용 없음
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelId(pub u32);

/// 분할 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDir {
    /// 좌|우 (세로 경계선)
    Horizontal,
    /// 상/하 (가로 경계선)
    Vertical,
}

/// windows crate 비의존 사각형 (plan T2 Design ③)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug)]
enum Node {
    Leaf(PanelId),
    Split {
        dir: SplitDir,
        /// 첫 자식이 차지하는 비율 (0.0~1.0, 클램프됨)
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

/// 스플리터 위치 — 히트테스트·드래그 대상 (T3에서 사용)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitterRect {
    pub rect: Rect,
    pub dir: SplitDir,
    /// 이 스플리터가 조절하는 Split 노드의 식별 경로 (루트부터의 first/second 선택 비트열)
    pub node_path: NodePath,
}

/// 루트→노드 경로 (비트열: 0=first, 1=second). 트리 수정 없이 노드를 안정 지칭한다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodePath {
    bits: u32,
    len: u8,
}

impl NodePath {
    pub const ROOT: NodePath = NodePath { bits: 0, len: 0 };

    fn child(self, second: bool) -> NodePath {
        NodePath {
            bits: self.bits | (u32::from(second) << self.len),
            len: self.len + 1,
        }
    }
}

/// 레이아웃 트리 오류
#[derive(Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// 마지막 1개 패널은 닫을 수 없다 (FR-2)
    LastPanel,
    /// 분할하면 최소 크기 미만이 된다
    TooSmall,
    /// 대상 패널/노드가 없다
    NotFound,
}

pub struct LayoutTree {
    root: Node,
    next_id: u32,
}

impl LayoutTree {
    /// 단일 패널로 시작
    pub fn new() -> (LayoutTree, PanelId) {
        let id = PanelId(0);
        (
            LayoutTree {
                root: Node::Leaf(id),
                next_id: 1,
            },
            id,
        )
    }

    /// 리프 개수
    pub fn panel_count(&self) -> usize {
        fn count(n: &Node) -> usize {
            match n {
                Node::Leaf(_) => 1,
                Node::Split { first, second, .. } => count(first) + count(second),
            }
        }
        count(&self.root)
    }

    /// 모든 패널 id (좌→우, 상→하 순)
    pub fn panel_ids(&self) -> Vec<PanelId> {
        fn walk(n: &Node, out: &mut Vec<PanelId>) {
            match n {
                Node::Leaf(id) => out.push(*id),
                Node::Split { first, second, .. } => {
                    walk(first, out);
                    walk(second, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    /// `target` 리프를 지정 방향으로 분할하고 새 리프 id를 반환한다.
    /// `area`는 현재 전체 영역 — 분할 결과가 최소 크기 미만이면 거부한다.
    /// 새 리프는 second(오른쪽/아래)에 배치된다.
    pub fn split(
        &mut self,
        target: PanelId,
        dir: SplitDir,
        area: Rect,
    ) -> Result<PanelId, LayoutError> {
        // 분할 가능 검사: 현재 target 리프의 실제 사각형 기준
        let rects = self.compute_rects(area);
        let target_rect = rects
            .panes
            .iter()
            .find(|(id, _)| *id == target)
            .map(|(_, r)| *r)
            .ok_or(LayoutError::NotFound)?;
        let available = match dir {
            SplitDir::Horizontal => target_rect.w,
            SplitDir::Vertical => target_rect.h,
        };
        if available < MIN_PANE_SIZE * 2 + SPLITTER_THICKNESS {
            return Err(LayoutError::TooSmall);
        }

        let new_id = PanelId(self.next_id);
        fn replace(n: &mut Node, target: PanelId, dir: SplitDir, new_id: PanelId) -> bool {
            match n {
                Node::Leaf(id) if *id == target => {
                    let old = *id;
                    *n = Node::Split {
                        dir,
                        ratio: 0.5,
                        first: Box::new(Node::Leaf(old)),
                        second: Box::new(Node::Leaf(new_id)),
                    };
                    true
                }
                Node::Leaf(_) => false,
                Node::Split { first, second, .. } => {
                    replace(first, target, dir, new_id) || replace(second, target, dir, new_id)
                }
            }
        }
        if !replace(&mut self.root, target, dir, new_id) {
            return Err(LayoutError::NotFound);
        }
        self.next_id += 1;
        Ok(new_id)
    }

    /// `target` 리프를 닫는다 — 형제 노드가 부모 자리로 승격된다.
    pub fn close(&mut self, target: PanelId) -> Result<(), LayoutError> {
        if matches!(&self.root, Node::Leaf(id) if *id == target) {
            return Err(LayoutError::LastPanel);
        }
        // target 리프를 직접 자식으로 가진 Split을 찾아 형제로 치환
        fn prune(n: &mut Node, target: PanelId) -> bool {
            if let Node::Split { first, second, .. } = n {
                let hit_first = matches!(first.as_ref(), Node::Leaf(id) if *id == target);
                let hit_second = matches!(second.as_ref(), Node::Leaf(id) if *id == target);
                if hit_first || hit_second {
                    let survivor = if hit_first {
                        std::mem::replace(second.as_mut(), Node::Leaf(PanelId(u32::MAX)))
                    } else {
                        std::mem::replace(first.as_mut(), Node::Leaf(PanelId(u32::MAX)))
                    };
                    *n = survivor;
                    return true;
                }
                return prune(first, target) || prune(second, target);
            }
            false
        }
        if prune(&mut self.root, target) {
            Ok(())
        } else {
            Err(LayoutError::NotFound)
        }
    }

    /// 경로가 가리키는 Split 노드의 비율을 설정한다 (스플리터 드래그).
    /// 최소 크기 유지를 위해 호출부가 준 axis 길이 기준으로 클램프한다.
    pub fn set_ratio(
        &mut self,
        path: NodePath,
        ratio: f32,
        axis_len: i32,
    ) -> Result<(), LayoutError> {
        let node = Self::node_at_mut(&mut self.root, path).ok_or(LayoutError::NotFound)?;
        if let Node::Split { ratio: r, .. } = node {
            *r = clamp_ratio(ratio, axis_len);
            Ok(())
        } else {
            Err(LayoutError::NotFound)
        }
    }

    fn node_at_mut(root: &mut Node, path: NodePath) -> Option<&mut Node> {
        let mut cur = root;
        for i in 0..path.len {
            match cur {
                Node::Split { first, second, .. } => {
                    cur = if (path.bits >> i) & 1 == 0 {
                        first.as_mut()
                    } else {
                        second.as_mut()
                    };
                }
                Node::Leaf(_) => return None,
            }
        }
        Some(cur)
    }

    /// 영역을 리프별 사각형 + 스플리터 목록으로 계산한다.
    /// 0 이하 크기 영역도 패닉 없이 빈 사각형으로 처리한다 (창 최소화 대응).
    pub fn compute_rects(&self, area: Rect) -> ComputedLayout {
        let mut out = ComputedLayout {
            panes: Vec::new(),
            splitters: Vec::new(),
        };
        fn walk(n: &Node, area: Rect, path: NodePath, out: &mut ComputedLayout) {
            match n {
                Node::Leaf(id) => out.panes.push((*id, area)),
                Node::Split {
                    dir,
                    ratio,
                    first,
                    second,
                } => {
                    let (a, sp, b) = split_rect(area, *dir, *ratio);
                    out.splitters.push(SplitterRect {
                        rect: sp,
                        dir: *dir,
                        node_path: path,
                    });
                    walk(first, a, path.child(false), out);
                    walk(second, b, path.child(true), out);
                }
            }
        }
        walk(&self.root, area, NodePath::ROOT, &mut out);
        out
    }
}

/// compute_rects 결과 — 패널 사각형과 스플리터 목록
pub struct ComputedLayout {
    pub panes: Vec<(PanelId, Rect)>,
    pub splitters: Vec<SplitterRect>,
}

/// 영역을 방향·비율로 (first, 스플리터, second)로 나눈다
fn split_rect(area: Rect, dir: SplitDir, ratio: f32) -> (Rect, Rect, Rect) {
    let clamp0 = |v: i32| v.max(0);
    match dir {
        SplitDir::Horizontal => {
            let usable = clamp0(area.w - SPLITTER_THICKNESS);
            let first_w = (usable as f32 * ratio) as i32;
            let second_w = usable - first_w;
            (
                Rect { w: first_w, ..area },
                Rect {
                    x: area.x + first_w,
                    w: SPLITTER_THICKNESS.min(clamp0(area.w)),
                    ..area
                },
                Rect {
                    x: area.x + first_w + SPLITTER_THICKNESS,
                    w: clamp0(second_w),
                    ..area
                },
            )
        }
        SplitDir::Vertical => {
            let usable = clamp0(area.h - SPLITTER_THICKNESS);
            let first_h = (usable as f32 * ratio) as i32;
            let second_h = usable - first_h;
            (
                Rect { h: first_h, ..area },
                Rect {
                    y: area.y + first_h,
                    h: SPLITTER_THICKNESS.min(clamp0(area.h)),
                    ..area
                },
                Rect {
                    y: area.y + first_h + SPLITTER_THICKNESS,
                    h: clamp0(second_h),
                    ..area
                },
            )
        }
    }
}

/// 비율을 최소 패널 크기가 유지되도록 클램프
fn clamp_ratio(ratio: f32, axis_len: i32) -> f32 {
    let usable = (axis_len - SPLITTER_THICKNESS).max(1);
    let min_frac = (MIN_PANE_SIZE as f32 / usable as f32).min(0.5);
    ratio.clamp(min_frac, 1.0 - min_frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 0,
        y: 0,
        w: 1200,
        h: 800,
    };

    #[test]
    fn 단일_패널로_시작한다() {
        let (tree, first) = LayoutTree::new();
        assert_eq!(tree.panel_count(), 1);
        assert_eq!(tree.panel_ids(), vec![first]);
        let layout = tree.compute_rects(AREA);
        assert_eq!(layout.panes, vec![(first, AREA)]);
        assert!(layout.splitters.is_empty());
    }

    #[test]
    fn 좌우_분할하면_두_패널이_나란히_배치된다() {
        let (mut tree, first) = LayoutTree::new();
        let second = tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        assert_eq!(tree.panel_count(), 2);

        let layout = tree.compute_rects(AREA);
        let r1 = layout.panes.iter().find(|(id, _)| *id == first).unwrap().1;
        let r2 = layout.panes.iter().find(|(id, _)| *id == second).unwrap().1;
        // 같은 높이, 좌우 인접 (스플리터 두께만큼 간격)
        assert_eq!(r1.h, AREA.h);
        assert_eq!(r2.h, AREA.h);
        assert_eq!(r1.x + r1.w + SPLITTER_THICKNESS, r2.x);
        assert_eq!(r1.w + SPLITTER_THICKNESS + r2.w, AREA.w);
        assert_eq!(layout.splitters.len(), 1);
    }

    #[test]
    fn 중첩_분할_좌우_후_상하() {
        let (mut tree, first) = LayoutTree::new();
        let right = tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        let right_bottom = tree.split(right, SplitDir::Vertical, AREA).unwrap();
        assert_eq!(tree.panel_count(), 3);
        assert_eq!(tree.panel_ids(), vec![first, right, right_bottom]);

        let layout = tree.compute_rects(AREA);
        let rb = layout
            .panes
            .iter()
            .find(|(id, _)| *id == right_bottom)
            .unwrap()
            .1;
        // 오른쪽 아래 사분면에 위치해야 한다
        assert!(rb.x > 0 && rb.y > 0);
        assert_eq!(layout.splitters.len(), 2);
    }

    #[test]
    fn 닫으면_형제가_승격된다() {
        let (mut tree, first) = LayoutTree::new();
        let second = tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        tree.close(first).unwrap();
        assert_eq!(tree.panel_count(), 1);
        assert_eq!(tree.panel_ids(), vec![second]);
        // 승격 후 전체 영역을 차지
        assert_eq!(tree.compute_rects(AREA).panes, vec![(second, AREA)]);
    }

    #[test]
    fn 리프를_닫으면_형제_분할_서브트리가_통째로_승격된다() {
        // A | (B / C) 구조에서 A를 닫으면 (B / C) Split이 루트로 승격되어야 한다
        let (mut tree, a) = LayoutTree::new();
        let b = tree.split(a, SplitDir::Horizontal, AREA).unwrap();
        let c = tree.split(b, SplitDir::Vertical, AREA).unwrap();
        tree.close(a).unwrap();

        assert_eq!(tree.panel_count(), 2);
        assert_eq!(tree.panel_ids(), vec![b, c]);
        let layout = tree.compute_rects(AREA);
        let rb = layout.panes.iter().find(|(id, _)| *id == b).unwrap().1;
        let rc = layout.panes.iter().find(|(id, _)| *id == c).unwrap().1;
        // 승격된 상하 분할이 전체 영역을 나눠 가진다
        assert_eq!(rb.w, AREA.w);
        assert_eq!(rc.w, AREA.w);
        assert_eq!(rb.h + SPLITTER_THICKNESS + rc.h, AREA.h);
    }

    #[test]
    fn 마지막_패널은_닫을_수_없다() {
        let (mut tree, first) = LayoutTree::new();
        assert_eq!(tree.close(first), Err(LayoutError::LastPanel));
    }

    #[test]
    fn 없는_패널은_분할_닫기_모두_실패한다() {
        let (mut tree, _first) = LayoutTree::new();
        let ghost = PanelId(99);
        assert_eq!(
            tree.split(ghost, SplitDir::Horizontal, AREA),
            Err(LayoutError::NotFound)
        );
        assert_eq!(tree.close(ghost), Err(LayoutError::NotFound));
    }

    #[test]
    fn 최소_크기_미만이면_분할을_거부한다() {
        let (mut tree, first) = LayoutTree::new();
        let narrow = Rect {
            x: 0,
            y: 0,
            w: MIN_PANE_SIZE * 2 + SPLITTER_THICKNESS - 1,
            h: 800,
        };
        assert_eq!(
            tree.split(first, SplitDir::Horizontal, narrow),
            Err(LayoutError::TooSmall)
        );
        // 세로 분할은 높이가 충분하므로 성공
        assert!(tree.split(first, SplitDir::Vertical, narrow).is_ok());
    }

    #[test]
    fn 비율_조절은_클램프된다() {
        let (mut tree, first) = LayoutTree::new();
        tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        let path = tree.compute_rects(AREA).splitters[0].node_path;

        // 극단값 → 최소 크기 유지 비율로 클램프
        tree.set_ratio(path, 0.0, AREA.w).unwrap();
        let layout = tree.compute_rects(AREA);
        let min_w = layout.panes.iter().map(|(_, r)| r.w).min().unwrap();
        assert!(min_w >= MIN_PANE_SIZE - 1); // 반올림 오차 1px 허용

        tree.set_ratio(path, 1.0, AREA.w).unwrap();
        let layout = tree.compute_rects(AREA);
        let min_w = layout.panes.iter().map(|(_, r)| r.w).min().unwrap();
        assert!(min_w >= MIN_PANE_SIZE - 1);
    }

    #[test]
    fn 영_크기_영역도_패닉하지_않는다() {
        let (mut tree, first) = LayoutTree::new();
        tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        let zero = Rect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        };
        let layout = tree.compute_rects(zero);
        assert_eq!(layout.panes.len(), 2);
        for (_, r) in layout.panes {
            assert!(r.w >= 0 && r.h >= 0);
        }
    }

    #[test]
    fn 분할_후_id는_재사용되지_않는다() {
        let (mut tree, first) = LayoutTree::new();
        let a = tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        tree.close(a).unwrap();
        let b = tree.split(first, SplitDir::Horizontal, AREA).unwrap();
        assert_ne!(a, b);
    }
}
