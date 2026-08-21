//! 좌측 사이드바 — 워크스페이스 2줄 카드 목록(FR-15~FR-19)과 그 아래 **연결 섹션**(FR-28·FR-33)을
//! 한 스크롤에 잇는다. 원본 디자인도 사이드바 전체가 한 컬럼이라, 워크스페이스가 많으면
//! 연결 섹션이 아래로 밀리며 함께 스크롤된다.
//!
//! 아래 시각 상수·색은 현행 Win32 판(`app::sidebar`)에서 **그대로 옮긴 것**이다
//! (part2 D3 — 사용자가 승인한 화면이라 이식에서 임의로 바꾸지 않는다).
//! 폭 토큰만 `app::settings`가 소유한다(세션 저장값 검증이 같은 범위를 쓰기 때문).
//!
//! 이 위젯은 워크스페이스를 **소유하지 않는다** — 목록을 받아 그리고,
//! 사용자 조작은 `SidebarAction` 값으로 돌려준다. 실제 변경은 `ui::app`이 한다.
use crate::app::workspace::WorkspaceList;
use crate::fs::icons::IconCache;
use crate::remote::sites::SiteStore;
use crate::remote::types::{SiteId, SiteRecord};
use crate::ui::icon_tex::IconTextures;
use crate::ui::theme;
use eframe::egui;

// ── 시각 토큰 (plan `## 시각 요소 분해` 1:1, 96DPI 기준 고정 px) ──
const HEADER_HEIGHT: f32 = 36.0;
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

// ── 연결 섹션 시각 토큰 (원본 `FileExplorer-FTP.dc.html:57-69`) ──
/// 연결 헤더가 워크스페이스 목록과 떨어지는 간격 (`margin-top:14px`)
const CONNECT_HEADER_TOP: f32 = 14.0;
/// 새로 고침 글리프 크기 — `+`(15px)보다 한 단계 작다 (인벤토리 #2)
const REFRESH_FONT_PX: f32 = 14.0;
/// 사이드바의 `+` 글리프 크기 — **탭 스트립의 새 탭 버튼과 같은 값**이다 (사용자 결정).
///
/// 원본은 연결 `+`만 15px였지만(인벤토리 #3, `:61`), 같은 뜻의 버튼이 자리마다 다른 크기·
/// 다른 글리프로 그려지면 하나의 앱으로 보이지 않는다. 글리프도 `egui_phosphor`의 `PLUS`로
/// 맞춘다 — ASCII `+`는 본문 글꼴에서 와 획 두께·여백이 아이콘과 다르다
const PLUS_ICON_PX: f32 = 12.0;
/// 헤더 우측 두 버튼 사이 간격
const HEADER_BUTTON_GAP: f32 = 2.0;
/// 사이트 행 높이·행간 (인벤토리 #4)
const SITE_ROW_HEIGHT: f32 = 36.0;
const SITE_ROW_GAP: f32 = 2.0;
/// 사이트 행 좌우 여백과 요소 사이 간격
const SITE_PAD_X: f32 = 8.0;
const SITE_GAP: f32 = 8.0;
/// 선택된 사이트의 왼쪽 강조 테두리 두께
const SITE_EDGE_WIDTH: f32 = 2.0;
/// 상태 점 지름
const SITE_DOT: f32 = 7.0;
/// 사이트 이름·프로토콜 글자 크기 (인벤토리 #4·#5)
const SITE_NAME_PX: f32 = 13.0;
const SITE_PROTO_PX: f32 = 12.0;
/// 연결 메뉴 폭·캡션 (인벤토리 #6, 원본 `:367-368`)
const CONNECT_MENU_WIDTH: f32 = 246.0;
/// 사이트 우클릭 메뉴 폭 (인벤토리 #9, 원본 `:355`)
const SITE_MENU_WIDTH: f32 = 180.0;
/// 두 메뉴가 함께 쓰는 캡션 글자 크기 (원본 `:368`·`:371`).
/// 행 높이는 여기서 정하지 않는다 — `theme::menu_style`이 세운 공통 값(`MENU_ITEM_HEIGHT`)을 따른다
const MENU_CAPTION_PX: f32 = 12.0;
/// 사이트 컨텍스트 메뉴의 삭제 옆에 붙는 단축키 표기 (인벤토리 #10)
const HIDE_SITE_SHORTCUT: &str = "Del";

/// 사이드바에서 올라온 사용자 조작. 목록을 바꾸는 일은 전부 호출부의 몫이다.
///
/// 폭 변경은 조작으로 올리지 않는다 — egui `Panel`이 폭을 스스로 관리하고 범위까지 클램프하므로,
/// 호출부가 그린 뒤 `response.rect.width()`로 읽으면 된다(왕복시키면 같은 값이 두 곳에 생긴다)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarAction {
    Select(usize),
    Add,
    Rename(usize, String),
    Remove(usize),
    /// `from` 항목을 목록의 `to` 자리로 옮긴다 (`WorkspaceList::reorder`와 같은 계약)
    Reorder(usize, usize),

    // ── 연결 섹션 (FR-28·FR-33, 인벤토리 #1~10) ──
    /// 사이트 행 클릭 — 고르기만 한다
    SelectSite(SiteId),
    /// 사이트 행 더블클릭·연결 메뉴 선택 — 이 사이트로 연결한다 (인벤토리 #4·#7)
    ConnectSite(SiteId),
    /// **목록에서 지운다** — 그 사이트의 연결·원격 탭·전송 큐 항목을 함께 걷어낸다 (FR-29).
    /// 사이트 기록만은 사이트 관리자에 남아 되돌릴 수 있다 (인벤토리 #10)
    RemoveSite(SiteId),
    /// 헤더 `⟳` (인벤토리 #2)
    RefreshSites,
    /// 헤더 `+` (인벤토리 #3) — 연결 메뉴는 사이드바가 직접 띄우므로 이 조작은
    /// 메뉴가 열렸다는 사실만 알린다(호출부가 상태 줄 등에 쓸 수 있다)
    OpenConnectMenu,
    /// `사이트 관리자` (인벤토리 #8 — 문구는 2026-08-20에 원본과 갈렸다) — 관리자를 연다
    OpenSiteManager,
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
    /// 연결 섹션에서 고른 사이트 — 왼쪽 강조 테두리가 붙는다 (인벤토리 #4)
    selected_site: Option<SiteId>,
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
            selected_site: None,
            editing: None,
            focus_edit: false,
            edit_added: false,
            drag: None,
        }
    }

    /// 사이드바를 그리고 이번 프레임의 조작을 **발생 순서대로** 돌려준다.
    ///
    /// 한 프레임에 조작이 둘 이상 겹칠 수 있어 목록으로 돌려준다 —
    /// 이름을 고치다가 다른 항목을 클릭하면 커밋(`Rename`)과 전환(`Select`)이 함께 일어나고,
    /// 하나만 남기면 둘 중 하나가 조용히 사라진다. 각 조작이 대상 인덱스를 품고 있어
    /// 처리 순서가 뒤바뀌어도 결과는 같다
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        list: &WorkspaceList,
        sites: &SiteStore,
        connected: &[SiteId],
        icons: &mut IconCache,
        textures: &mut IconTextures,
    ) -> Vec<SidebarAction> {
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, theme::WINDOW_BG);
        if self.edit_added {
            // 호출부가 추가를 처리해 활성이 새 항목으로 옮겨진 뒤다
            self.edit_added = false;
            self.begin_edit(list.active_index(), list);
        }
        let mut actions = Vec::new();
        // 두 섹션을 **한 스크롤 안에** 세로로 잇는다 — 원본도 사이드바 전체가 한 컬럼이라
        // 워크스페이스가 많으면 연결 섹션이 아래로 밀리며 함께 스크롤된다 (원본 `:36-69`)
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.show_header(ui, &mut actions);
                self.show_items(ui, list, icons, textures, &mut actions);
                self.show_connections(ui, sites, connected, &mut actions);
            });
        if actions.contains(&SidebarAction::Add) {
            self.edit_added = true;
        }
        actions
    }

    /// "워크스페이스" 제목과 추가(+) 버튼
    fn show_header(&mut self, ui: &mut egui::Ui, actions: &mut Vec<SidebarAction>) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), HEADER_HEIGHT),
            egui::Sense::hover(),
        );
        ui.painter().text(
            egui::pos2(rect.left() + ITEM_MARGIN_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::i18n::sidebar_workspaces(),
            egui::FontId::proportional(HEADER_FONT_PX),
            theme::TEXT_MUTED,
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
            .on_hover_text(crate::i18n::sidebar_new_workspace());
        // 사이트 헤더의 두 버튼과 같은 함수를 쓴다 — hover 표현이 갈리지 않게
        header_glyph(
            ui,
            plus,
            egui_phosphor::regular::PLUS,
            PLUS_ICON_PX,
            resp.hovered(),
        );
        if resp.clicked() {
            actions.push(SidebarAction::Add);
        }
    }

    /// 카드 목록 — 스크롤·선택·이름 편집·드래그 정렬
    fn show_items(
        &mut self,
        ui: &mut egui::Ui,
        list: &WorkspaceList,
        icons: &mut IconCache,
        textures: &mut IconTextures,
        actions: &mut Vec<SidebarAction>,
    ) {
        let can_remove = list.len() > 1;
        // 스크롤은 `show`가 두 섹션을 함께 감싼다 — 여기서 또 열면 스크롤이 중첩된다
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
                self.show_edit(ui, card, index, actions);
            } else {
                draw_card(ui, card, list, index, resp.hovered(), icons, textures);
                self.handle_item_input(ui, &resp, index, list, can_remove, actions);
            }
        }
        self.finish_drag(ui, list, first_top, actions);
    }

    /// 카드 하나에 대한 클릭·컨텍스트 메뉴·드래그 입력
    fn handle_item_input(
        &mut self,
        ui: &egui::Ui,
        resp: &egui::Response,
        index: usize,
        list: &WorkspaceList,
        can_remove: bool,
        actions: &mut Vec<SidebarAction>,
    ) {
        if resp.clicked() {
            actions.push(SidebarAction::Select(index));
        }
        // 선택된 항목에서 F2를 누르면 이름 편집 — 입력 중에는 텍스트 입력이 우선이라 여기서만 본다
        if index == list.active_index()
            && ui.input(|i| i.key_pressed(egui::Key::F2))
            && self.editing.is_none()
        {
            self.begin_edit(index, list);
        }
        resp.context_menu(|ui| {
            theme::menu_style(ui);
            if ui.button(crate::i18n::rename()).clicked() {
                self.begin_edit(index, list);
                ui.close();
            }
            if ui
                .add_enabled(can_remove, egui::Button::new(crate::i18n::delete()))
                .clicked()
            {
                actions.push(SidebarAction::Remove(index));
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
        actions: &mut Vec<SidebarAction>,
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
            theme::ACCENT,
        );
        if ui.input(|i| !i.pointer.any_down()) {
            if let Some(to) = reorder_target(drag.from, insert_at) {
                actions.push(SidebarAction::Reorder(drag.from, to));
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
        actions: &mut Vec<SidebarAction>,
    ) {
        let Some((_, mut buffer)) = self.editing.take() else {
            return;
        };
        ui.painter().rect_filled(card, 0.0, theme::ROW_HOT);
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
                actions.push(SidebarAction::Rename(index, buffer));
            }
            return;
        }
        self.editing = Some((index, buffer));
    }

    /// 메뉴에서 워크스페이스를 추가했을 때도 `+` 버튼과 같이 곧바로 이름을 고치게 한다 (FR-16).
    /// 새 항목의 인덱스는 호출부가 추가를 끝낸 다음 프레임에야 정해진다
    /// 연결 섹션 — 헤더(`⟳`·`+`)와 사이트 목록 (인벤토리 #1~10).
    ///
    /// **숨긴 사이트는 여기 나오지 않는다** — 지운 것이 아니라 사이드바에서만 감춘 것이라
    /// 사이트 관리자에는 그대로 남는다 (README §1)
    fn show_connections(
        &mut self,
        ui: &mut egui::Ui,
        sites: &SiteStore,
        connected: &[SiteId],
        actions: &mut Vec<SidebarAction>,
    ) {
        ui.add_space(CONNECT_HEADER_TOP);
        self.show_connect_header(ui, sites, actions);
        // 사이트가 하나도 없으면 **다음에 할 일을 적는다** — 종전에는 헤더만 남아, 앱을 처음 연
        // 사람이 서버를 어디서 등록하는지 화면에서 알 수 없었다 (2026-08-16 검토)
        if sites.visible().next().is_none() {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), SITE_ROW_HEIGHT),
                egui::Sense::hover(),
            );
            ui.painter().text(
                egui::pos2(rect.left() + ITEM_MARGIN_X, rect.center().y),
                egui::Align2::LEFT_CENTER,
                crate::i18n::sidebar_no_sites(),
                egui::FontId::proportional(HEADER_FONT_PX),
                theme::TEXT_MUTED,
            );
            return;
        }
        for record in sites.visible() {
            let live = connected.contains(&record.id);
            self.show_site_row(ui, record, live, actions);
        }
    }

    /// `연결` 제목과 우측 두 버튼 — 새로 고침(`⟳`)·연결 메뉴(`+`) (인벤토리 #1~3)
    fn show_connect_header(
        &mut self,
        ui: &mut egui::Ui,
        sites: &SiteStore,
        actions: &mut Vec<SidebarAction>,
    ) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), HEADER_HEIGHT),
            egui::Sense::hover(),
        );
        ui.painter().text(
            egui::pos2(rect.left() + ITEM_MARGIN_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::i18n::connect(),
            egui::FontId::proportional(HEADER_FONT_PX),
            theme::TEXT_MUTED,
        );

        let top = rect.top() + (HEADER_HEIGHT - PLUS_SIZE) / 2.0;
        let plus_rect = egui::Rect::from_min_size(
            egui::pos2(rect.right() - PLUS_MARGIN - PLUS_SIZE, top),
            egui::Vec2::splat(PLUS_SIZE),
        );
        let refresh_rect = egui::Rect::from_min_size(
            egui::pos2(plus_rect.left() - HEADER_BUTTON_GAP - PLUS_SIZE, top),
            egui::Vec2::splat(PLUS_SIZE),
        );

        let refresh = ui
            .interact(
                refresh_rect,
                ui.id().with("sites_refresh"),
                egui::Sense::click(),
            )
            .on_hover_text(crate::i18n::sidebar_refresh_sites());
        header_glyph(
            ui,
            refresh_rect,
            egui_phosphor::regular::ARROW_CLOCKWISE,
            REFRESH_FONT_PX,
            refresh.hovered(),
        );
        if refresh.clicked() {
            actions.push(SidebarAction::RefreshSites);
        }

        let plus = ui
            .interact(plus_rect, ui.id().with("sites_add"), egui::Sense::click())
            .on_hover_text(crate::i18n::sidebar_connect_menu());
        header_glyph(
            ui,
            plus_rect,
            egui_phosphor::regular::PLUS,
            PLUS_ICON_PX,
            plus.hovered(),
        );
        if plus.clicked() {
            actions.push(SidebarAction::OpenConnectMenu);
        }
        show_connect_menu(&plus, sites, actions);
    }

    /// 사이트 한 줄 — 클릭은 고르기, 더블클릭은 연결, 우클릭은 메뉴 (인벤토리 #4·#5·#9·#10)
    fn show_site_row(
        &mut self,
        ui: &mut egui::Ui,
        record: &SiteRecord,
        connected: bool,
        actions: &mut Vec<SidebarAction>,
    ) {
        // 끌 수도 있다 — 패널 탭 스트립에 놓으면 그 패널의 새 탭으로 열린다 (FR-38·인벤토리 #15)
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), SITE_ROW_HEIGHT),
            egui::Sense::click_and_drag(),
        );
        ui.add_space(SITE_ROW_GAP);
        let selected = self.selected_site == Some(record.id);
        if selected {
            ui.painter().rect_filled(rect, 0.0, theme::ROW_HOT);
        } else if resp.hovered() {
            ui.painter().rect_filled(rect, 0.0, theme::CARD_HOT);
        }
        // 고른 사이트만 왼쪽에 강조 띠가 붙는다 (원본 `border-left`)
        if selected {
            let edge =
                egui::Rect::from_min_size(rect.min, egui::vec2(SITE_EDGE_WIDTH, rect.height()));
            ui.painter().rect_filled(edge, 0.0, theme::ACCENT);
        }

        let dot_center = egui::pos2(
            rect.left() + SITE_EDGE_WIDTH + SITE_PAD_X + SITE_DOT / 2.0,
            rect.center().y,
        );
        // 연결돼 있으면 초록 점 — 아니면 흐린 점 (README `### Colors`)
        let dot_color = if connected {
            theme::OK_DOT
        } else {
            theme::TEXT_DIM
        };
        ui.painter()
            .circle_filled(dot_center, SITE_DOT / 2.0, dot_color);

        // 프로토콜을 먼저 오른쪽에 붙인다 — 이름이 길어도 밀려나지 않는다 (plan Edge Case)
        let proto = record.protocol.label();
        let proto_galley = ui.painter().layout_no_wrap(
            proto.to_owned(),
            egui::FontId::proportional(SITE_PROTO_PX),
            theme::TEXT_MUTED,
        );
        let proto_left = rect.right() - SITE_PAD_X - proto_galley.size().x;
        ui.painter().galley(
            egui::pos2(proto_left, rect.center().y - proto_galley.size().y / 2.0),
            proto_galley,
            theme::TEXT_MUTED,
        );

        let name_left = dot_center.x + SITE_DOT / 2.0 + SITE_GAP;
        let name_width = (proto_left - SITE_GAP - name_left).max(0.0);
        let name = elide(ui, &record.name, SITE_NAME_PX, theme::TEXT, name_width);
        ui.painter().galley(
            egui::pos2(name_left, rect.center().y - name.size().y / 2.0),
            name,
            theme::TEXT,
        );

        // 더블클릭이 연결이다 — 클릭은 고르기만 한다(잘못 눌러 연결이 열리지 않게)
        if resp.double_clicked() {
            self.selected_site = Some(record.id);
            actions.push(SidebarAction::ConnectSite(record.id));
        } else if resp.clicked() {
            self.selected_site = Some(record.id);
            actions.push(SidebarAction::SelectSite(record.id));
        }
        // 끌기 시작을 알린다 — 받는 쪽(탭 스트립)이 놓는 순간 이 값을 가져간다.
        // 좌표를 주고받지 않는 이유는 스트립이 스크롤 안에 있어 화면 좌표가 흔들리기 때문이다
        if resp.drag_started() {
            egui::DragAndDrop::set_payload(ui.ctx(), record.id);
        }
        if resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        }
        show_site_context_menu(&resp, record, actions);
    }

    pub fn edit_after_add(&mut self) {
        self.edit_added = true;
    }

    /// 메뉴 "이름 바꾸기" 진입점 — 사이드바 안의 F2·컨텍스트 메뉴와 같은 편집을 연다
    pub fn start_rename(&mut self, index: usize, list: &WorkspaceList) {
        self.begin_edit(index, list);
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
/// 헤더 우측 아이콘 글리프 — 평소 흐리고 마우스를 올리면 밝아진다 (인벤토리 #2·#3)
fn header_glyph(ui: &egui::Ui, rect: egui::Rect, glyph: &str, size: f32, hovered: bool) {
    // 마우스가 올라가면 **배경을 깐다** — 탭의 닫기·새 탭 버튼과 같은 표현이다(같은 함수를 쓴다).
    // 글자색만 바꾸면 같은 아이콘 버튼인데 자리마다 반응이 달라 보인다 (사용자 결정)
    if hovered {
        crate::ui::widgets::hover_backdrop(ui.painter(), rect, theme::CONTROL_HOT);
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        glyph,
        egui::FontId::proportional(size),
        if hovered {
            theme::TEXT
        } else {
            theme::TEXT_MUTED
        },
    );
}

/// 연결 메뉴 — `+` 버튼에서 열린다 (인벤토리 #6~8).
///
/// 목록에 **숨긴 사이트는 넣지 않는다** — 사이드바에서 감춘 것을 여기서 다시 보이면
/// 감춘 의미가 없다
fn show_connect_menu(plus: &egui::Response, sites: &SiteStore, actions: &mut Vec<SidebarAction>) {
    egui::Popup::menu(plus).show(|ui| {
        theme::menu_style(ui);
        ui.set_width(CONNECT_MENU_WIDTH);
        ui.label(
            egui::RichText::new(crate::i18n::sidebar_saved_sites())
                .size(MENU_CAPTION_PX)
                .color(theme::TEXT_MUTED),
        );
        for record in sites.visible() {
            let button = egui::Button::new(egui::RichText::new(&record.name).color(theme::TEXT))
                .right_text(
                    egui::RichText::new(record.protocol.label())
                        .size(SITE_PROTO_PX)
                        .color(theme::TEXT_MUTED),
                );
            if ui.add(button).clicked() {
                actions.push(SidebarAction::ConnectSite(record.id));
                ui.close();
            }
        }
        ui.separator();
        let add = egui::Button::new(
            egui::RichText::new(crate::i18n::sidebar_site_manager()).color(theme::TEXT),
        );
        if ui.add(add).clicked() {
            actions.push(SidebarAction::OpenSiteManager);
            ui.close();
        }
    });
}

/// 사이트 우클릭 메뉴 — 사이트 이름 머리와 삭제 하나뿐이다 (인벤토리 #9·#10).
///
/// 여기서 지우는 것은 **사이드바 바로가기**다 — 사이트 자체는 사이트 관리자에 남는다
fn show_site_context_menu(
    row: &egui::Response,
    record: &SiteRecord,
    actions: &mut Vec<SidebarAction>,
) {
    egui::Popup::context_menu(row).show(|ui| {
        theme::menu_style(ui);
        ui.set_width(SITE_MENU_WIDTH);
        ui.label(
            egui::RichText::new(&record.name)
                .size(MENU_CAPTION_PX)
                .color(theme::TEXT_MUTED),
        );
        ui.separator();
        let hide = egui::Button::new(
            egui::RichText::new(crate::i18n::sidebar_hide_site()).color(theme::TEXT),
        )
        .right_text(
            egui::RichText::new(HIDE_SITE_SHORTCUT)
                .size(SITE_PROTO_PX)
                .color(theme::TEXT_MUTED),
        );
        // **파괴색을 쓰지 않는다** — 원본은 이 자리를 삭제로 보고 빨갛게 칠했지만(`:358`)
        // 실제로는 사이드바에서 감출 뿐 사이트는 남는다. 되돌릴 수 있는 일에 되돌릴 수 없는
        // 일의 색을 쓰면 그 색의 뜻이 닳는다 (2026-08-16 검토)
        let clicked = ui.add(hide).clicked();
        if clicked {
            actions.push(SidebarAction::RemoveSite(record.id));
            ui.close();
        }
    });
}

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
        theme::ROW_HOT
    } else if hovered {
        theme::CARD_HOT
    } else {
        theme::CARD_BG
    };
    let painter = ui.painter();
    painter.rect_filled(card, 0.0, fill);
    painter.rect_stroke(
        card,
        0.0,
        egui::Stroke::new(1.0, theme::BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );
    if is_active {
        painter.rect_filled(
            egui::Rect::from_min_size(card.min, egui::vec2(ACCENT_BAR_WIDTH, card.height())),
            0.0,
            theme::ACCENT,
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
    let name = elide(ui, &item.name, NAME_FONT_PX, theme::TEXT, text_width);
    let name_top = card.top() + NAME_TOP;
    ui.painter()
        .galley(egui::pos2(text_left, name_top), name, theme::TEXT);
    let subtitle = elide(
        ui,
        &item.subtitle,
        SUBTITLE_FONT_PX,
        theme::TEXT_FAINT,
        text_width,
    );
    ui.painter().galley(
        egui::pos2(text_left, name_top + NAME_FONT_PX + 4.0 + SUBTITLE_GAP),
        subtitle,
        theme::TEXT_FAINT,
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

    /// 사이드바를 한 프레임 그리고 화면에 나온 글자를 모은다
    fn draw_sidebar(sites: &SiteStore, connected: &[SiteId]) -> Vec<String> {
        fn collect(shape: &egui::Shape, found: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => found.push(text.galley.text().to_owned()),
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, found);
                    }
                }
                _ => {}
            }
        }
        let ctx = egui::Context::default();
        let mut sidebar = WorkspaceSidebar::new();
        let list = WorkspaceList::new();
        let mut icons = IconCache::new();
        let mut textures = IconTextures::new();
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                sidebar.show(ui, &list, sites, connected, &mut icons, &mut textures);
            });
        });
        let mut found = Vec::new();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut found);
        }
        found
    }

    #[test]
    fn 연결_섹션_문구는_인벤토리_원문_그대로다() {
        // 인벤토리 #1·#6·#8·#10 — 다듬으면 화면과 명세가 갈린다.
        // **카탈로그를 거쳐도 한국어 값은 원문이어야 한다** — 그것을 여기서 잡는다
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        assert_eq!(crate::i18n::connect(), "연결");
        assert_eq!(crate::i18n::sidebar_saved_sites(), "등록된 사이트");
        // 이 항목만 **원본과 갈린다** — 사용자 요청(2026-08-20)으로 문구를 바꿨다 (FR-59)
        assert_eq!(crate::i18n::sidebar_site_manager(), "사이트 관리자");
        assert_eq!(crate::i18n::delete(), "삭제");
        assert_eq!(HIDE_SITE_SHORTCUT, "Del");
    }

    #[test]
    fn 연결_섹션_치수는_원본과_같다() {
        // 인벤토리 #4·#5·#6·#9 · 시각 속성 표
        assert_eq!(SITE_ROW_HEIGHT, 36.0);
        assert_eq!(SITE_DOT, 7.0);
        assert_eq!(SITE_EDGE_WIDTH, 2.0);
        assert_eq!(SITE_PROTO_PX, 12.0);
        assert_eq!(CONNECT_HEADER_TOP, 14.0);
        assert_eq!(REFRESH_FONT_PX, 14.0);
        // `+`는 원본(15px)이 아니라 **탭 스트립의 새 탭 버튼과 같은 크기**를 쓴다 (사용자 결정)
        assert_eq!(PLUS_ICON_PX, 12.0);
        assert_eq!(CONNECT_MENU_WIDTH, 246.0);
        assert_eq!(SITE_MENU_WIDTH, 180.0);
        assert_eq!(theme::MENU_ITEM_HEIGHT, 28.0);
    }

    #[test]
    fn 연결_섹션은_사이트를_이름과_프로토콜로_보인다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // Acceptance ① — 등록된 사이트가 헤더 아래에 줄로 선다
        let mut sites = SiteStore::new();
        sites.add("배포 서버");
        let texts = draw_sidebar(&sites, &[]);
        assert!(texts.iter().any(|t| t == "연결"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "배포 서버"), "{texts:?}");
        // 새 사이트의 기본 프로토콜은 FTP다
        assert!(texts.iter().any(|t| t == "ftp"), "{texts:?}");
    }

    #[test]
    fn 사이트가_없으면_헤더만_남는다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // plan Edge Case — 빈 목록에서도 섹션이 사라지지 않는다
        let texts = draw_sidebar(&SiteStore::new(), &[]);
        assert!(texts.iter().any(|t| t == "연결"), "{texts:?}");
    }

    #[test]
    fn 숨긴_사이트는_사이드바에서만_사라진다() {
        // Acceptance ③ · README §1 — 사이트 관리자 목록에는 그대로 남아야 한다
        let mut sites = SiteStore::new();
        let id = sites.add("배포 서버");
        sites.hide(id);

        assert!(sites.get(id).is_some(), "사이트가 저장소에서 지워졌다");
        assert_eq!(sites.visible().count(), 0);
        let texts = draw_sidebar(&sites, &[]);
        assert!(
            !texts.iter().any(|t| t == "배포 서버"),
            "숨긴 사이트가 사이드바에 남았다: {texts:?}"
        );
    }

    #[test]
    fn 사이드바_기본_폭은_260이다() {
        // Acceptance ⑤ · D24 — 저장된 폭이 있으면 그것이 이긴다(세션 복원이 덮어쓴다)
        assert_eq!(crate::app::settings::SIDEBAR_DEFAULT_WIDTH, 260);
    }

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
