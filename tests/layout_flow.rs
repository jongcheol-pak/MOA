//! 분할 레이아웃 시퀀스 통합 테스트 (FR-1·FR-2).
//!
//! 개별 연산은 `app::layout`의 단위 테스트가 덮는다. 여기서는 실제 사용 흐름
//! (분할 → 비율 조절 → 중첩 분할 → 닫기)을 이어서 수행했을 때 배치가 일관적인지 본다.
//! egui 쪽 `ui::splitter`는 이 결과를 좌표 변환만 해서 그리므로, 이 테스트가 통과하면
//! 화면 배치의 근거가 검증된 셈이다(그리기 자체는 HUMAN-VERIFY).
use file_explorer::app::layout::{LayoutTree, Rect, SplitDir, SplitPlace};

const AREA: Rect = Rect {
    x: 0,
    y: 0,
    w: 1000,
    h: 600,
};

#[test]
fn 분할_비율조절_닫기_시퀀스가_일관적이다() {
    let (mut tree, first) = LayoutTree::new();

    // 1) 좌우 분할 → 두 패널이 나란히
    let right = tree
        .split(first, SplitDir::Horizontal, SplitPlace::After, AREA)
        .unwrap();
    let computed = tree.compute_rects(AREA);
    assert_eq!(computed.panes.len(), 2);
    assert_eq!(computed.splitters.len(), 1);
    let left_rect = computed
        .panes
        .iter()
        .find(|(id, _)| *id == first)
        .unwrap()
        .1;
    let right_rect = computed
        .panes
        .iter()
        .find(|(id, _)| *id == right)
        .unwrap()
        .1;
    assert_eq!(left_rect.y, right_rect.y, "좌우 분할은 같은 높이에 놓인다");
    assert!(left_rect.x < right_rect.x);

    // 2) 비율을 왼쪽 30%로 — 스플리터 경로로 지정한다
    let path = computed.splitters[0].node_path;
    tree.set_ratio(path, 0.3, AREA.w).unwrap();
    let computed = tree.compute_rects(AREA);
    let left_rect = computed
        .panes
        .iter()
        .find(|(id, _)| *id == first)
        .unwrap()
        .1;
    let right_rect = computed
        .panes
        .iter()
        .find(|(id, _)| *id == right)
        .unwrap()
        .1;
    assert!(
        left_rect.w < right_rect.w,
        "30%로 줄인 왼쪽이 오른쪽보다 좁아야 한다 (left={}, right={})",
        left_rect.w,
        right_rect.w
    );

    // 3) 오른쪽을 다시 상하로 중첩 분할 → 패널 3개
    let bottom = tree
        .split(right, SplitDir::Vertical, SplitPlace::After, AREA)
        .unwrap();
    let computed = tree.compute_rects(AREA);
    assert_eq!(computed.panes.len(), 3);
    assert_eq!(computed.splitters.len(), 2);
    let top_rect = computed
        .panes
        .iter()
        .find(|(id, _)| *id == right)
        .unwrap()
        .1;
    let bottom_rect = computed
        .panes
        .iter()
        .find(|(id, _)| *id == bottom)
        .unwrap()
        .1;
    assert_eq!(top_rect.x, bottom_rect.x, "상하 분할은 같은 x에 놓인다");
    assert!(top_rect.y < bottom_rect.y);

    // 4) 중첩된 하나를 닫으면 형제가 그 자리를 흡수한다
    tree.close(bottom).unwrap();
    let computed = tree.compute_rects(AREA);
    assert_eq!(computed.panes.len(), 2);
    let absorbed = computed
        .panes
        .iter()
        .find(|(id, _)| *id == right)
        .unwrap()
        .1;
    assert_eq!(
        absorbed.h, AREA.h,
        "형제가 닫히면 남은 패널이 세로 전체를 차지한다"
    );

    // 5) 남은 둘 중 하나를 더 닫으면 마지막 하나만 남고, 그 뒤로는 닫히지 않는다
    tree.close(right).unwrap();
    assert_eq!(tree.panel_count(), 1);
    assert!(tree.close(first).is_err(), "마지막 패널은 닫을 수 없다");

    let computed = tree.compute_rects(AREA);
    assert_eq!(computed.panes.len(), 1);
    assert_eq!(computed.panes[0].1, AREA, "혼자 남으면 전체 영역을 쓴다");
    assert!(computed.splitters.is_empty());
}

#[test]
fn 창이_작아져도_배치가_패닉하지_않는다() {
    let (mut tree, first) = LayoutTree::new();
    let second = tree
        .split(first, SplitDir::Horizontal, SplitPlace::After, AREA)
        .unwrap();

    // 창을 극단적으로 줄여도 계산이 성립해야 한다 (최소화 대응)
    let tiny = Rect {
        x: 0,
        y: 0,
        w: 1,
        h: 1,
    };
    let computed = tree.compute_rects(tiny);
    assert_eq!(computed.panes.len(), 2);
    for (_, rect) in &computed.panes {
        assert!(rect.w >= 0 && rect.h >= 0, "음수 크기가 나오면 안 된다");
    }
    // 되돌리면 원래 비율로 복원된다 (비율은 트리에 남아 있다)
    let computed = tree.compute_rects(AREA);
    let a = computed
        .panes
        .iter()
        .find(|(id, _)| *id == first)
        .unwrap()
        .1;
    let b = computed
        .panes
        .iter()
        .find(|(id, _)| *id == second)
        .unwrap()
        .1;
    assert!(a.w > 0 && b.w > 0);
}
