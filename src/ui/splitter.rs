//! 자유 분할 레이아웃 — 분할 트리를 화면에 배치하고 스플리터를 드래그한다 (FR-1·FR-2).
//!
//! 배치 계산은 `app::layout::LayoutTree`(순수 로직·단위 테스트 완비)가 그대로 하고,
//! 이 파일은 그 결과를 egui 좌표로 옮겨 그리고 입력을 되돌려주는 일만 한다.
//! egui의 `SidePanel`류 도킹 컨테이너는 쓰지 않는다 — 중첩 자유 분할을 표현하지 못한다.
use crate::app::layout::{LayoutTree, PanelId, Rect as LayoutRect, SplitDir};
use crate::fs::icons::IconCache;
use crate::ui::icon_tex::IconTextures;
use crate::ui::menu::{Command, PanelMenuState};
use crate::ui::panel::{MenuRequest, PanelState};
use crate::ui::theme;
use eframe::egui;
use std::collections::HashMap;

/// 스플리터 히트 영역을 좌우(상하)로 넓히는 여유.
/// 4px 틈만으로는 잡기 어려워 실제 조작 영역만 넓힌다(그리기는 원래 두께 그대로)
const SPLITTER_GRAB_PAD: f32 = 2.0;

/// 패널 테두리 두께 — 일반 경계와 활성 강조가 같은 두께를 쓴다
const PANE_BORDER_WIDTH: f32 = 1.0;

pub fn to_egui_rect(r: LayoutRect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(r.x as f32, r.y as f32),
        egui::vec2(r.w.max(0) as f32, r.h.max(0) as f32),
    )
}

pub fn to_layout_rect(r: egui::Rect) -> LayoutRect {
    LayoutRect {
        x: r.left() as i32,
        y: r.top() as i32,
        w: r.width() as i32,
        h: r.height() as i32,
    }
}

/// 레이아웃이 상위(앱)에 올려보내는 요청 — 어느 패널에서 왔는지까지 담는다
#[derive(Default)]
pub struct LayoutOutcome {
    pub menu: Option<MenuRequest>,
    /// 패널 메뉴에서 고른 명령과 그 패널. **활성 패널이 아니라 메뉴를 연 패널**이다.
    ///
    /// 활성 판정은 포인터가 눌린 위치로만 이뤄지는데, 메뉴 팝업은 자기 패널 밖으로 뻗을 수
    /// 있어 그 위에서 고르면 아래 깔린 패널이 활성이 된다 — 그대로 활성 패널에 적용하면
    /// 닫기·새 파일이 엉뚱한 패널에 간다 (plan D16)
    pub command: Option<(PanelId, Command)>,
}

/// 분할된 패널들을 그리고 스플리터 드래그·활성 패널 전환을 처리한다.
///
/// `panels`에 없는 `PanelId`가 트리에 있으면 그 자리는 비워 둔다 —
/// 분할 직후 새 패널을 만들어 넣는 것은 호출부(`ExplorerApp`)의 몫이다
#[allow(clippy::too_many_arguments)]
pub fn show_layout(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    tree: &mut LayoutTree,
    panels: &mut HashMap<PanelId, PanelState>,
    active: &mut PanelId,
    icons: &mut IconCache,
    textures: &mut IconTextures,
) -> LayoutOutcome {
    let mut outcome = LayoutOutcome::default();
    let area = ui.available_rect_before_wrap();
    let computed = tree.compute_rects(to_layout_rect(area));
    // 패널은 서로를 모르므로(모듈 주석) 닫기 가능 여부는 트리를 아는 이곳에서 정해 내려준다 (plan D15).
    // 보기 모드는 패널마다 다르므로 아래 루프에서 각자의 것을 싣는다
    let pane_count = computed.panes.len();

    // 클릭이 일어난 패널을 활성으로 삼는다.
    // 패널 rect에 `interact`를 걸면 그 위젯이 목록·버튼 클릭을 가로채므로
    // (겹치는 위젯은 나중에 등록된 쪽이 이긴다) 포인터 위치로만 판정한다
    let pressed_at = ctx
        .input(|i| i.pointer.any_pressed().then(|| i.pointer.interact_pos()))
        .flatten();

    for (id, rect) in &computed.panes {
        let pane = to_egui_rect(*rect);
        if pane.width() <= 0.0 || pane.height() <= 0.0 {
            continue;
        }
        if let Some(pos) = pressed_at
            && pane.contains(pos)
        {
            *active = *id;
        }
        let Some(panel) = panels.get_mut(id) else {
            continue;
        };
        // 패널마다 독립된 id 공간을 준다 — 같은 위젯이 여러 패널에 있어도 상태가 섞이지 않는다
        let builder = egui::UiBuilder::new()
            .max_rect(pane)
            .id_salt(("pane", id.0));
        let menu_state = PanelMenuState::for_panes(pane_count, panel.view_mode());
        let requested = ui
            .scope_builder(builder, |ui| {
                ui.set_clip_rect(pane);
                panel.show(ui, ctx, icons, textures, menu_state)
            })
            .inner;
        // 한 프레임에 메뉴는 하나만 뜬다 — 먼저 요청한 패널 것을 쓴다
        if outcome.menu.is_none() {
            outcome.menu = requested.menu;
        }
        // 명령도 한 프레임에 하나만 — 어느 패널이 요청했는지 함께 담는다
        if outcome.command.is_none()
            && let Some(command) = requested.command
        {
            outcome.command = Some((*id, command));
        }
    }

    // 패널 경계 — 여러 패널이 있을 때만 의미가 있다(하나뿐이면 창 가장자리와 겹친다).
    //
    // 패널 내용을 **모두 그린 뒤** 여기서 두른다 — egui는 나중에 그린 도형이 위에 오므로
    // 먼저 그으면 목록·트리에 덮여 보이지 않는다.
    // 스플리터 틈(4px)이 배경색이라 패널끼리 경계가 눈에 띄지 않던 것을 이 테두리가 대신한다
    if computed.panes.len() > 1 {
        for (id, rect) in &computed.panes {
            let pane = to_egui_rect(*rect);
            // 그리기 루프의 0크기 가드는 이 패스에 이어지지 않는다 — 여기서 다시 거른다
            if pane.width() <= 0.0 || pane.height() <= 0.0 {
                continue;
            }
            // 활성 패널만 한 단계 밝게 — 어디에 입력이 가는지 눈으로 알 수 있어야 한다
            let color = if id == active {
                theme::PANE_BORDER_ACTIVE
            } else {
                theme::PANE_BORDER
            };
            ui.painter().rect_stroke(
                pane,
                0.0,
                egui::Stroke::new(PANE_BORDER_WIDTH, color),
                egui::StrokeKind::Inside,
            );
        }
    }

    // id는 목록 인덱스로 만든다 — `NodePath`는 Hash를 구현하지 않기 때문이다.
    // 드래그 도중에는 인덱스가 흔들리지 않는다: 트리 구조를 바꾸는 분할·닫기는 버튼 클릭,
    // 즉 **별도의 포인터 누름**으로만 일어나므로 같은 드래그 제스처 안에서 함께 발생할 수 없다
    for (index, splitter) in computed.splitters.iter().enumerate() {
        let rect = to_egui_rect(splitter.rect);
        // 그리기는 원래 두께로, 잡기는 조금 넓게
        ui.painter().rect_filled(rect, 0.0, theme::WINDOW_BG);
        let grab = match splitter.dir {
            SplitDir::Horizontal => rect.expand2(egui::vec2(SPLITTER_GRAB_PAD, 0.0)),
            SplitDir::Vertical => rect.expand2(egui::vec2(0.0, SPLITTER_GRAB_PAD)),
        };
        let id = ui.id().with(("splitter", index));
        let resp = ui.interact(grab, id, egui::Sense::drag());
        let cursor = match splitter.dir {
            SplitDir::Horizontal => egui::CursorIcon::ResizeHorizontal,
            SplitDir::Vertical => egui::CursorIcon::ResizeVertical,
        };
        if resp.hovered() || resp.dragged() {
            ctx.set_cursor_icon(cursor);
        }
        if resp.dragged()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            let node = splitter.node_area;
            let (start, len, at) = match splitter.dir {
                SplitDir::Horizontal => (node.x as f32, node.w, pos.x),
                SplitDir::Vertical => (node.y as f32, node.h, pos.y),
            };
            if len > 0 {
                // 최소 패널 크기 클램프는 set_ratio가 축 길이를 받아 처리한다
                let ratio = (at - start) / len as f32;
                let _ = tree.set_ratio(splitter.node_path, ratio, len);
            }
        }
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 사각형_변환은_왕복해도_같다() {
        let r = LayoutRect {
            x: 10,
            y: 20,
            w: 300,
            h: 400,
        };
        assert_eq!(to_layout_rect(to_egui_rect(r)), r);
    }

    #[test]
    fn 음수_크기는_0으로_잘린다() {
        let r = LayoutRect {
            x: 0,
            y: 0,
            w: -5,
            h: -5,
        };
        let e = to_egui_rect(r);
        assert_eq!(e.width(), 0.0);
        assert_eq!(e.height(), 0.0);
    }
}
