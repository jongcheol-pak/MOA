//! 워크스페이스 사이드바 — 좌측 2줄 카드 목록 (FR-15~FR-19).
//!
//! 아래 시각 상수·색은 현행 Win32 판(`app::sidebar`)에서 **그대로 옮긴 것**이다
//! (part2 D3 — 사용자가 승인한 화면이라 이식에서 임의로 바꾸지 않는다).
//! 폭 토큰만 `app::settings`가 소유한다(세션 저장값 검증이 같은 범위를 쓰기 때문).
//!
//! 이 위젯은 워크스페이스를 **소유하지 않는다** — 목록을 받아 그리고,
//! 사용자 조작은 `SidebarAction` 값으로 돌려준다. 실제 변경은 `ui::app`이 한다.
use crate::app::workspace::WorkspaceList;
use crate::fs::icons::IconCache;
use crate::ui::icon_tex::IconTextures;
use eframe::egui;

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
/// 사이드바 상단 접기 토글 영역
const TOGGLE_STRIP_HEIGHT: f32 = 28.0;
const TOGGLE_SIZE: f32 = 24.0;
const TOGGLE_MARGIN: f32 = 8.0;
/// 접기 토글 아이콘 — 좌측 패널 표시를 뜻하는 반쪽 사각형 기호
const TOGGLE_GLYPH: &str = "◧";
const HEADER_HEIGHT: f32 = 36.0;
const HEADER_LABEL: &str = "워크스페이스";
const HEADER_FONT_PX: f32 = 14.0;
/// 새 워크스페이스 버튼 — 헤더 우측
const PLUS_SIZE: f32 = 24.0;
const PLUS_MARGIN: f32 = 8.0;
const ITEM_HEIGHT: f32 = 60.0;
const ITEM_GAP: f32 = 4.0;
/// 항목 하나가 차지하는 세로 간격(높이 + 아래 여백)
const ITEM_PITCH: f32 = ITEM_HEIGHT + ITEM_GAP;
const ITEM_MARGIN_X: f32 = 8.0;
const ACCENT_BAR_WIDTH: f32 = 3.0;
const ICON_SIZE: f32 = 16.0;
const ICON_X: f32 = 12.0;
const TEXT_X: f32 = 38.0;
const NAME_TOP: f32 = 12.0;
const NAME_FONT_PX: f32 = 15.0;
const SUBTITLE_GAP: f32 = 6.0;
const SUBTITLE_FONT_PX: f32 = 13.0;
/// 텍스트 우측 여백 — 이름·부제가 카드 오른쪽 끝에 닿지 않게 한다(현행과 같은 값)
const TEXT_RIGHT_PAD: f32 = 8.0;
/// 드래그 정렬 시작 임계 — 이만큼 움직여야 재정렬로 본다 (단순 클릭과 구분)
const DRAG_THRESHOLD: f32 = 8.0;
/// 드롭 위치 삽입선 두께
const INSERT_LINE_HEIGHT: f32 = 2.0;

const COLOR_BG: egui::Color32 = egui::Color32::from_rgb(0x1B, 0x1B, 0x1B);
const COLOR_ITEM: egui::Color32 = egui::Color32::from_rgb(0x23, 0x23, 0x23);
const COLOR_ITEM_SELECTED: egui::Color32 = egui::Color32::from_rgb(0x2E, 0x2E, 0x2E);
const COLOR_ITEM_HOVER: egui::Color32 = egui::Color32::from_rgb(0x28, 0x28, 0x28);
const COLOR_ITEM_BORDER: egui::Color32 = egui::Color32::from_rgb(0x2C, 0x2C, 0x2C);
const COLOR_ACCENT: egui::Color32 = egui::Color32::from_rgb(0x4A, 0x9E, 0xFF);
const COLOR_NAME: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE8, 0xE8);
const COLOR_SUBTITLE: egui::Color32 = egui::Color32::from_rgb(0x8A, 0x8A, 0x8A);
const COLOR_HEADER: egui::Color32 = egui::Color32::from_rgb(0x9A, 0x9A, 0x9A);
const COLOR_HEADER_HOT: egui::Color32 = egui::Color32::from_rgb(0xE8, 0xE8, 0xE8);

/// 사이드바에서 올라온 사용자 조작. 목록을 바꾸는 일은 전부 호출부의 몫이다
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarAction {
    Select(usize),
    Add,
    Rename(usize, String),
    Remove(usize),
    /// `from` 항목을 목록의 `to` 자리로 옮긴다 (`WorkspaceList::reorder`와 같은 계약)
    Reorder(usize, usize),
    ToggleCollapse,
}

/// 드래그 정렬 진행 상태
struct Drag {
    from: usize,
    start: egui::Pos2,
    /// 임계를 넘어 실제 재정렬로 전환됐는가 — 넘기 전에는 단순 클릭일 수 있다
    active: bool,
}

/// 사이드바의 화면 상태 — 편집 중인 이름과 드래그만 갖는다
pub struct WorkspaceSidebar {
    /// 인라인 편집 중인 (인덱스, 입력 중인 이름)
    editing: Option<(usize, String)>,
    /// 편집을 시작한 프레임에 입력칸으로 포커스를 옮기기 위한 표시
    focus_edit: bool,
    /// 새로 추가한 워크스페이스를 다음 프레임에 편집 상태로 만든다.
    /// 추가는 호출부가 처리하므로, 새 항목의 인덱스는 다음 프레임에야 알 수 있다 (FR-16)
    edit_added: bool,
    drag: Option<Drag>,
}

impl Default for WorkspaceSidebar {
    fn default() -> WorkspaceSidebar {
        WorkspaceSidebar::new()
    }
}

impl WorkspaceSidebar {
    pub fn new() -> WorkspaceSidebar {
        WorkspaceSidebar {
            editing: None,
            focus_edit: false,
            edit_added: false,
            drag: None,
        }
    }

    /// 사이드바를 그리고 이번 프레임의 조작 하나를 돌려준다.
    /// 한 프레임에 여러 조작이 겹치면 마지막에 확정된 것만 남는다
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        list: &WorkspaceList,
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> Option<SidebarAction> {
        ui.painter().rect_filled(ui.max_rect(), 0.0, COLOR_BG);
        if self.edit_added {
            // 호출부가 추가를 처리해 활성이 새 항목으로 옮겨진 뒤다
            self.edit_added = false;
            self.begin_edit(list.active_index(), list);
        }
        let mut action = None;
        self.show_toggle_strip(ui, &mut action);
        self.show_header(ui, &mut action);
        self.show_items(ui, list, icons, textures, &mut action);
        if action == Some(SidebarAction::Add) {
            self.edit_added = true;
        }
        action
    }

    /// 접기 토글만 있는 상단 스트립
    fn show_toggle_strip(&mut self, ui: &mut egui::Ui, action: &mut Option<SidebarAction>) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), TOGGLE_STRIP_HEIGHT),
            egui::Sense::hover(),
        );
        let button = egui::Rect::from_min_size(
            egui::pos2(
                rect.left() + TOGGLE_MARGIN,
                rect.top() + (TOGGLE_STRIP_HEIGHT - TOGGLE_SIZE) / 2.0,
            ),
            egui::Vec2::splat(TOGGLE_SIZE),
        );
        let resp = ui.interact(button, ui.id().with("collapse"), egui::Sense::click());
        ui.painter().text(
            button.center(),
            egui::Align2::CENTER_CENTER,
            TOGGLE_GLYPH,
            egui::FontId::proportional(HEADER_FONT_PX),
            if resp.hovered() {
                COLOR_HEADER_HOT
            } else {
                COLOR_HEADER
            },
        );
        if resp.clicked() {
            *action = Some(SidebarAction::ToggleCollapse);
        }
    }

    /// "워크스페이스" 제목과 추가(+) 버튼
    fn show_header(&mut self, ui: &mut egui::Ui, action: &mut Option<SidebarAction>) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), HEADER_HEIGHT),
            egui::Sense::hover(),
        );
        ui.painter().text(
            egui::pos2(rect.left() + ITEM_MARGIN_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            HEADER_LABEL,
            egui::FontId::proportional(HEADER_FONT_PX),
            COLOR_HEADER,
        );
        let plus = egui::Rect::from_min_size(
            egui::pos2(
                rect.right() - PLUS_MARGIN - PLUS_SIZE,
                rect.top() + (HEADER_HEIGHT - PLUS_SIZE) / 2.0,
            ),
            egui::Vec2::splat(PLUS_SIZE),
        );
        let resp = ui
            .interact(plus, ui.id().with("add"), egui::Sense::click())
            .on_hover_text("새 워크스페이스");
        ui.painter().text(
            plus.center(),
            egui::Align2::CENTER_CENTER,
            "+",
            egui::FontId::proportional(HEADER_FONT_PX),
            if resp.hovered() {
                COLOR_HEADER_HOT
            } else {
                COLOR_HEADER
            },
        );
        if resp.clicked() {
            *action = Some(SidebarAction::Add);
        }
    }

    /// 카드 목록 — 스크롤·선택·이름 편집·드래그 정렬
    fn show_items(
        &mut self,
        ui: &mut egui::Ui,
        list: &WorkspaceList,
        icons: &mut IconCache,
        textures: &mut IconTextures,
        action: &mut Option<SidebarAction>,
    ) {
        let can_remove = list.len() > 1;
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let mut first_top = None;
                for index in 0..list.len() {
                    let (row, resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ITEM_HEIGHT),
                        egui::Sense::click_and_drag(),
                    );
                    ui.add_space(ITEM_GAP);
                    if first_top.is_none() {
                        first_top = Some(row.top());
                    }
                    let card = egui::Rect::from_min_max(
                        egui::pos2(row.left() + ITEM_MARGIN_X, row.top()),
                        egui::pos2((row.right() - ITEM_MARGIN_X).max(row.left()), row.bottom()),
                    );
                    if self.editing.as_ref().is_some_and(|(i, _)| *i == index) {
                        self.show_edit(ui, card, index, action);
                    } else {
                        draw_card(ui, card, list, index, resp.hovered(), icons, textures);
                        self.handle_item_input(ui, &resp, index, list, can_remove, action);
                    }
                }
                self.finish_drag(ui, list, first_top, action);
            });
    }

    /// 카드 하나에 대한 클릭·컨텍스트 메뉴·드래그 입력
    fn handle_item_input(
        &mut self,
        ui: &egui::Ui,
        resp: &egui::Response,
        index: usize,
        list: &WorkspaceList,
        can_remove: bool,
        action: &mut Option<SidebarAction>,
    ) {
        if resp.clicked() {
            *action = Some(SidebarAction::Select(index));
        }
        // 선택된 항목에서 F2를 누르면 이름 편집 — 입력 중에는 텍스트 입력이 우선이라 여기서만 본다
        if index == list.active_index()
            && ui.input(|i| i.key_pressed(egui::Key::F2))
            && self.editing.is_none()
        {
            self.begin_edit(index, list);
        }
        resp.context_menu(|ui| {
            if ui.button("이름 바꾸기").clicked() {
                self.begin_edit(index, list);
                ui.close();
            }
            if ui
                .add_enabled(can_remove, egui::Button::new("삭제"))
                .clicked()
            {
                *action = Some(SidebarAction::Remove(index));
                ui.close();
            }
        });

        if resp.drag_started()
            && let Some(pos) = resp.interact_pointer_pos()
        {
            self.drag = Some(Drag {
                from: index,
                start: pos,
                active: false,
            });
        }
        // 임계를 넘어야 재정렬로 본다 — 그 전에는 클릭 제스처와 구분되지 않는다
        if resp.dragged()
            && let (Some(drag), Some(pos)) = (self.drag.as_mut(), resp.interact_pointer_pos())
            && (pos - drag.start).length() >= DRAG_THRESHOLD
        {
            drag.active = true;
        }
    }

    /// 드래그가 끝났으면 놓인 자리를 계산해 재정렬을 요청한다.
    /// 진행 중이면 놓일 자리에 삽입선을 그린다
    fn finish_drag(
        &mut self,
        ui: &egui::Ui,
        list: &WorkspaceList,
        first_top: Option<f32>,
        action: &mut Option<SidebarAction>,
    ) {
        let (Some(drag), Some(first_top)) = (self.drag.as_ref(), first_top) else {
            return;
        };
        if !drag.active {
            // 임계를 못 넘은 제스처 — 클릭으로 이미 처리됐다
            if ui.input(|i| !i.pointer.any_down()) {
                self.drag = None;
            }
            return;
        }
        let Some(pos) = ui.input(|i| i.pointer.interact_pos()) else {
            return;
        };
        let insert_at = insert_index_at(pos.y, first_top, list.len());
        let line_y = first_top + insert_at as f32 * ITEM_PITCH - ITEM_GAP / 2.0;
        let full = ui.max_rect();
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(
                    full.left() + ITEM_MARGIN_X,
                    line_y - INSERT_LINE_HEIGHT / 2.0,
                ),
                egui::vec2(
                    (full.width() - ITEM_MARGIN_X * 2.0).max(0.0),
                    INSERT_LINE_HEIGHT,
                ),
            ),
            0.0,
            COLOR_ACCENT,
        );
        if ui.input(|i| !i.pointer.any_down()) {
            if let Some(to) = reorder_target(drag.from, insert_at) {
                *action = Some(SidebarAction::Reorder(drag.from, to));
            }
            self.drag = None;
        }
    }

    /// 카드 자리에 이름 입력칸을 띄운다. Enter·포커스 이동은 커밋, Esc는 취소
    fn show_edit(
        &mut self,
        ui: &mut egui::Ui,
        card: egui::Rect,
        index: usize,
        action: &mut Option<SidebarAction>,
    ) {
        let Some((_, mut buffer)) = self.editing.take() else {
            return;
        };
        ui.painter().rect_filled(card, 0.0, COLOR_ITEM_SELECTED);
        let edit_rect = egui::Rect::from_min_max(
            egui::pos2(card.left() + TEXT_X, card.top() + NAME_TOP),
            egui::pos2(
                (card.right() - TEXT_RIGHT_PAD).max(card.left() + TEXT_X),
                card.top() + NAME_TOP + NAME_FONT_PX + 8.0,
            ),
        );
        let resp = ui.put(
            edit_rect,
            egui::TextEdit::singleline(&mut buffer).font(egui::FontId::proportional(NAME_FONT_PX)),
        );
        if self.focus_edit {
            resp.request_focus();
            self.focus_edit = false;
        }
        if resp.lost_focus() {
            // Esc로 빠져나온 경우만 취소다 — 그 외(Enter·다른 곳 클릭)는 입력한 이름을 반영한다
            if !ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                *action = Some(SidebarAction::Rename(index, buffer));
            }
            return;
        }
        self.editing = Some((index, buffer));
    }

    fn begin_edit(&mut self, index: usize, list: &WorkspaceList) {
        let Some(item) = list.items().get(index) else {
            return;
        };
        self.editing = Some((index, item.name.clone()));
        self.focus_edit = true;
    }
}

/// 카드 하나를 그린다 (배경·강조 바·아이콘·이름·부제)
fn draw_card(
    ui: &egui::Ui,
    card: egui::Rect,
    list: &WorkspaceList,
    index: usize,
    hovered: bool,
    icons: &mut IconCache,
    textures: &mut IconTextures,
) {
    let item = &list.items()[index];
    let is_active = index == list.active_index();
    let fill = if is_active {
        COLOR_ITEM_SELECTED
    } else if hovered {
        COLOR_ITEM_HOVER
    } else {
        COLOR_ITEM
    };
    let painter = ui.painter();
    painter.rect_filled(card, 0.0, fill);
    painter.rect_stroke(
        card,
        0.0,
        egui::Stroke::new(1.0, COLOR_ITEM_BORDER),
        egui::StrokeKind::Inside,
    );
    if is_active {
        painter.rect_filled(
            egui::Rect::from_min_size(card.min, egui::vec2(ACCENT_BAR_WIDTH, card.height())),
            0.0,
            COLOR_ACCENT,
        );
    }

    if let Some(texture) = textures.get(ui.ctx(), icons.himl(), icons.dir_icon()) {
        let icon = egui::Rect::from_min_size(
            egui::pos2(
                card.left() + ICON_X,
                card.top() + (ITEM_HEIGHT - ICON_SIZE) / 2.0,
            ),
            egui::Vec2::splat(ICON_SIZE),
        );
        ui.painter().image(
            texture.id(),
            icon,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    let text_left = card.left() + TEXT_X;
    let text_width = (card.right() - TEXT_RIGHT_PAD - text_left).max(0.0);
    let name = elide(ui, &item.name, NAME_FONT_PX, COLOR_NAME, text_width);
    let name_top = card.top() + NAME_TOP;
    ui.painter()
        .galley(egui::pos2(text_left, name_top), name, COLOR_NAME);
    let subtitle = elide(
        ui,
        &item.subtitle,
        SUBTITLE_FONT_PX,
        COLOR_SUBTITLE,
        text_width,
    );
    ui.painter().galley(
        egui::pos2(text_left, name_top + NAME_FONT_PX + 4.0 + SUBTITLE_GAP),
        subtitle,
        COLOR_SUBTITLE,
    );
}

/// 한 줄로 말줄임한 텍스트 배치 — 이름·경로가 카드 폭을 넘으면 끝을 '…'로 줄인다
fn elide(
    ui: &egui::Ui,
    text: &str,
    font_px: f32,
    color: egui::Color32,
    max_width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        egui::FontId::proportional(font_px),
        color,
    );
    job.wrap = egui::text::TextWrapping {
        max_width,
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    ui.painter().layout_job(job)
}

/// 포인터 y가 가리키는 **삽입 위치**(0..=count) — 카드 중앙보다 아래면 그 뒤에 넣는다
fn insert_index_at(y: f32, first_top: f32, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let raw = ((y - first_top) / ITEM_PITCH).floor();
    let index = raw.clamp(0.0, count as f32 - 1.0) as usize;
    let center = first_top + index as f32 * ITEM_PITCH + ITEM_HEIGHT / 2.0;
    if y > center { index + 1 } else { index }
}

/// 삽입 위치를 `WorkspaceList::reorder`의 목적지 인덱스로 바꾼다.
/// reorder는 항목을 **꺼낸 뒤** 넣으므로, 자기보다 뒤에 놓을 때는 한 칸 당겨야 한다.
/// 제자리로 놓는 경우(자기 앞·자기 뒤)는 바꿀 것이 없어 None
fn reorder_target(from: usize, insert_at: usize) -> Option<usize> {
    if insert_at == from || insert_at == from + 1 {
        return None;
    }
    Some(if insert_at > from {
        insert_at - 1
    } else {
        insert_at
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 삽입_위치는_카드_중앙을_기준으로_갈린다() {
        let top = 100.0;
        // 첫 카드 위쪽 절반 → 0번 앞
        assert_eq!(insert_index_at(top + 5.0, top, 3), 0);
        // 첫 카드 아래쪽 절반 → 0번 뒤
        assert_eq!(insert_index_at(top + ITEM_HEIGHT - 5.0, top, 3), 1);
        // 둘째 카드 아래쪽 절반 → 1번 뒤
        assert_eq!(
            insert_index_at(top + ITEM_PITCH + ITEM_HEIGHT - 5.0, top, 3),
            2
        );
    }

    #[test]
    fn 목록_밖_좌표는_양_끝으로_클램프된다() {
        let top = 100.0;
        assert_eq!(insert_index_at(top - 500.0, top, 3), 0);
        assert_eq!(insert_index_at(top + 5000.0, top, 3), 3);
        // 빈 목록은 항상 0
        assert_eq!(insert_index_at(top, top, 0), 0);
    }

    #[test]
    fn 제자리에_놓으면_재정렬하지_않는다() {
        assert_eq!(reorder_target(1, 1), None);
        assert_eq!(reorder_target(1, 2), None);
    }

    #[test]
    fn 뒤로_옮길_때는_한_칸_당겨진다() {
        // 0번을 3번 자리(2번 뒤)에 놓으면 결과 목록에서는 인덱스 2다
        assert_eq!(reorder_target(0, 3), Some(2));
        // 앞으로 옮길 때는 그대로
        assert_eq!(reorder_target(2, 0), Some(0));
    }
}
