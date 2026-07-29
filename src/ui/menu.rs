//! 패널 메뉴와 단축키 (FR-12·FR-26).
//!
//! **상단 메뉴 바는 두지 않는다.** 종전의 보기·이동·탭·워크스페이스 네 메뉴는 항목이 모두
//! 다른 진입점(주소창 버튼·탭 스트립·사이드바 `+`·컨텍스트 메뉴)에 있었고, 유일하게 겹치지
//! 않던 '패널 닫기'는 이 패널 메뉴로 옮겼다.
//!
//! 이 모듈은 상태를 바꾸지 않는다 — 무엇을 하라는 **명령만 값으로 돌려주고**,
//! 실행은 `ui::app`이 한다(패널·워크스페이스 소유자가 거기이기 때문).
use crate::app::layout::{SplitDir, SplitPlace};
use crate::ui::view_mode::ViewMode;
use eframe::egui;

/// 분할 방향 — 새 패널이 놓일 자리를 사용자 관점으로 나타낸다.
///
/// 트리는 축(`SplitDir`)과 앞뒤(`SplitPlace`)만 알므로 여기서 한 번 변환한다 (plan D1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitTo {
    Right,
    Left,
    Up,
    Down,
}

impl SplitTo {
    /// 레이아웃 트리가 쓰는 (축, 배치)로 바꾼다
    pub fn to_layout(self) -> (SplitDir, SplitPlace) {
        match self {
            SplitTo::Right => (SplitDir::Horizontal, SplitPlace::After),
            SplitTo::Left => (SplitDir::Horizontal, SplitPlace::Before),
            SplitTo::Up => (SplitDir::Vertical, SplitPlace::Before),
            SplitTo::Down => (SplitDir::Vertical, SplitPlace::After),
        }
    }
}

/// 메뉴·단축키가 요청하는 동작.
///
/// 워크스페이스 생성·이름 변경·삭제는 여기 없다 — 메뉴 바를 없애면서 생성처가 사라졌고,
/// 그 기능들은 사이드바의 `+`·컨텍스트 메뉴가 `SidebarAction`으로 직접 처리한다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    NewTab,
    CloseTab,
    Back,
    Forward,
    Up,
    Refresh,
    Split(SplitTo),
    ClosePanel,
    /// 표시 중인 폴더에 빈 텍스트 문서를 만든다 (FR-25)
    NewFile,
    /// 표시 중인 폴더에 새 폴더를 만든다 (FR-25)
    NewFolder,
    /// 파일 목록 보기 모드를 바꾼다 (FR-23)
    SetViewMode(ViewMode),
    ToggleSidebar,
}

/// 패널 메뉴 항목의 활성/비활성을 가르는 현재 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelMenuState {
    /// 패널이 2개 이상인가 — 마지막 하나는 닫을 수 없다 (FR-2)
    pub can_close_panel: bool,
    /// 지금 이 패널이 쓰는 보기 모드 — 하위 메뉴에서 점으로 표시한다 (FR-23)
    pub view_mode: ViewMode,
}

impl PanelMenuState {
    /// 화면에 있는 패널 수로 활성 조건을 정한다.
    ///
    /// 패널은 서로를 모르므로 이 판정은 트리를 아는 쪽(`ui::splitter`)이 내려준다 (plan D15).
    /// 그 계산을 여기 두는 이유는 "마지막 하나는 닫을 수 없다"는 규칙(FR-2)이 갈리지 않게
    /// 한 곳에 모으기 위해서다
    pub fn for_panes(pane_count: usize, view_mode: ViewMode) -> PanelMenuState {
        PanelMenuState {
            can_close_panel: pane_count > 1,
            view_mode,
        }
    }
}

/// 패널 메뉴를 그리고 고른 항목을 돌려준다 (FR-26).
///
/// 항목 순서·구분선 위치는 plan `## 시각 요소 분해`의 인벤토리 표 13행 그대로다.
/// 진입점이 이 메뉴 하나뿐이므로, 여기서 빠진 기능은 마우스로 닿을 수 없게 된다
pub fn panel_menu_items(ui: &mut egui::Ui, state: PanelMenuState, out: &mut Option<Command>) {
    // 마우스를 올리기만 해도 펼쳐진다 — 메뉴 안에서 부른 `menu_button`은 egui가
    // `SubMenuButton`으로 바꾸고, 그것이 hover로 열린다 (사용자 요청 8번)
    ui.menu_button("보기", |ui| view_items(ui, state.view_mode, out));
    ui.separator();
    split_items(ui, out);
    ui.separator();
    item(ui, "새로 고침", "F5", true, Command::Refresh, out);
    ui.separator();
    item(ui, "새 파일", "", true, Command::NewFile, out);
    item(ui, "새 폴더", "", true, Command::NewFolder, out);
    ui.separator();
    item(
        ui,
        "닫기",
        "Ctrl+Shift+W",
        state.can_close_panel,
        Command::ClosePanel,
        out,
    );
}

/// 보기 모드 8종 (FR-23) — 지금 쓰는 모드 왼쪽에 점을 찍는다.
///
/// 문구·순서는 plan `### 참조 정합 인벤토리 — '보기' 하위 메뉴` 8행 그대로다.
/// 모드를 나타내는 아이콘은 넣지 않는다 — phosphor에 대응 글리프가 없어 두부가 될 위험이
/// 있고(사이드바 `◧` 사례), 점만으로도 지금 모드가 드러난다
fn view_items(ui: &mut egui::Ui, current: ViewMode, out: &mut Option<Command>) {
    for mode in ViewMode::ALL {
        let mark = if mode == current { "•" } else { " " };
        let button = egui::Button::new(format!("{mark} {}", mode.label()));
        if ui.add(button).clicked() {
            *out = Some(Command::SetViewMode(mode));
            ui.close();
        }
    }
}

/// 네 방향 분할 항목 (FR-1) — 패널 메뉴 안에 놓인다
fn split_items(ui: &mut egui::Ui, out: &mut Option<Command>) {
    for (label, shortcut, to) in [
        ("오른쪽 분할", "Ctrl+Alt+→", SplitTo::Right),
        ("왼쪽 분할", "Ctrl+Alt+←", SplitTo::Left),
        ("위쪽 분할", "Ctrl+Alt+↑", SplitTo::Up),
        ("아래쪽 분할", "Ctrl+Alt+↓", SplitTo::Down),
    ] {
        item(ui, label, shortcut, true, Command::Split(to), out);
    }
}

/// 메뉴 항목 하나 — 오른쪽에 단축키를 함께 보인다(`shortcut`이 비면 표기하지 않는다)
fn item(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    command: Command,
    out: &mut Option<Command>,
) {
    let button = egui::Button::new(label).right_text(shortcut);
    if ui.add_enabled(enabled, button).clicked() {
        *out = Some(command);
        ui.close();
    }
}

/// 이번 프레임에 눌린 단축키를 명령으로 바꾼다.
///
/// 무수식 키는 F5 말고는 단축키로 두지 않는다 — 이름 편집 중 텍스트 입력을 가로챈다(현행 판의 같은 결정).
/// 메뉴에 적힌 F2는 사이드바가 직접 처리한다. 삭제는 키를 배정하지 않았다 —
/// 지금 구조에서는 사이드바가 키를 **전역으로** 보기 때문에, Delete를 받으면 파일 목록에서 누른
/// Delete까지 워크스페이스를 지운다. 카드에 포커스를 주고 `has_focus()`일 때만 받으면 해결되지만
/// 그 전환은 사이드바 입력 전반에 걸쳐 있어 별도 작업으로 미뤘다(Deferred)
pub fn poll_shortcuts(ctx: &egui::Context) -> Option<Command> {
    // 포커스를 가진 위젯이 있으면 단축키를 보지 않는다 — 주소창·이름 편집이 키를 먼저 가져간다.
    // 이 검사는 텍스트 입력뿐 아니라 포커스를 받은 위젯 전부를 덮는다(가로채기를 막는 쪽으로 넉넉하게)
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    ctx.input_mut(|input| {
        shortcut_table()
            .into_iter()
            .find_map(|(modifiers, key, command)| {
                let shortcut = egui::KeyboardShortcut::new(modifiers, key);
                input.consume_shortcut(&shortcut).then_some(command)
            })
    })
}

/// 단축키 → 명령 대응표 (현행 `app::menu::create_accels`와 같은 구성).
/// 수식 키가 많은 조합을 앞에 두어 `Ctrl+Shift+\`가 `Ctrl+\`로 오인되지 않게 한다
fn shortcut_table() -> [(egui::Modifiers, egui::Key, Command); 14] {
    let ctrl_shift = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
    let ctrl_alt = egui::Modifiers::CTRL | egui::Modifiers::ALT;
    [
        // 기존 두 단축키는 뜻을 그대로 잇는다 — 좌우 분할이 곧 오른쪽, 상하 분할이 곧 아래쪽이었다 (D4)
        (
            ctrl_shift,
            egui::Key::Backslash,
            Command::Split(SplitTo::Down),
        ),
        (ctrl_shift, egui::Key::W, Command::ClosePanel),
        (
            egui::Modifiers::CTRL,
            egui::Key::Backslash,
            Command::Split(SplitTo::Right),
        ),
        (
            ctrl_alt,
            egui::Key::ArrowRight,
            Command::Split(SplitTo::Right),
        ),
        (
            ctrl_alt,
            egui::Key::ArrowLeft,
            Command::Split(SplitTo::Left),
        ),
        (ctrl_alt, egui::Key::ArrowUp, Command::Split(SplitTo::Up)),
        (
            ctrl_alt,
            egui::Key::ArrowDown,
            Command::Split(SplitTo::Down),
        ),
        (egui::Modifiers::CTRL, egui::Key::T, Command::NewTab),
        (egui::Modifiers::CTRL, egui::Key::W, Command::CloseTab),
        (egui::Modifiers::CTRL, egui::Key::B, Command::ToggleSidebar),
        (egui::Modifiers::ALT, egui::Key::ArrowLeft, Command::Back),
        (
            egui::Modifiers::ALT,
            egui::Key::ArrowRight,
            Command::Forward,
        ),
        (egui::Modifiers::ALT, egui::Key::ArrowUp, Command::Up),
        (egui::Modifiers::NONE, egui::Key::F5, Command::Refresh),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 네_방향은_축과_배치로_정확히_갈린다() {
        // 이 매핑이 틀리면 "왼쪽 분할"이 오른쪽에 패널을 만드는 식으로 조용히 어긋난다
        assert_eq!(
            SplitTo::Right.to_layout(),
            (SplitDir::Horizontal, SplitPlace::After)
        );
        assert_eq!(
            SplitTo::Left.to_layout(),
            (SplitDir::Horizontal, SplitPlace::Before)
        );
        assert_eq!(
            SplitTo::Up.to_layout(),
            (SplitDir::Vertical, SplitPlace::Before)
        );
        assert_eq!(
            SplitTo::Down.to_layout(),
            (SplitDir::Vertical, SplitPlace::After)
        );
    }

    #[test]
    fn 기존_분할_단축키는_뜻을_그대로_잇는다() {
        // Ctrl+\(좌우 분할) = 오른쪽, Ctrl+Shift+\(상하 분할) = 아래쪽 — 익힌 동작이 바뀌면 안 된다 (D4)
        let table = shortcut_table();
        let find = |modifiers, key| {
            table
                .iter()
                .find(|(m, k, _)| *m == modifiers && *k == key)
                .map(|(_, _, command)| *command)
        };
        assert_eq!(
            find(egui::Modifiers::CTRL, egui::Key::Backslash),
            Some(Command::Split(SplitTo::Right))
        );
        assert_eq!(
            find(
                egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
                egui::Key::Backslash
            ),
            Some(Command::Split(SplitTo::Down))
        );
    }

    #[test]
    fn 네_방향_단축키가_모두_배정돼_있다() {
        let table = shortcut_table();
        let ctrl_alt = egui::Modifiers::CTRL | egui::Modifiers::ALT;
        let find = |key| {
            table
                .iter()
                .find(|(m, k, _)| *m == ctrl_alt && *k == key)
                .map(|(_, _, command)| *command)
        };
        assert_eq!(
            find(egui::Key::ArrowRight),
            Some(Command::Split(SplitTo::Right))
        );
        assert_eq!(
            find(egui::Key::ArrowLeft),
            Some(Command::Split(SplitTo::Left))
        );
        assert_eq!(find(egui::Key::ArrowUp), Some(Command::Split(SplitTo::Up)));
        assert_eq!(
            find(egui::Key::ArrowDown),
            Some(Command::Split(SplitTo::Down))
        );
    }

    #[test]
    fn 같은_키_조합이_두_명령에_겹치지_않는다() {
        // 겹치면 표에서 앞선 것만 동작하고 뒤엣것은 조용히 죽는다
        let table = shortcut_table();
        for (index, (modifiers, key, _)) in table.iter().enumerate() {
            for (other_modifiers, other_key, command) in &table[index + 1..] {
                assert!(
                    !(modifiers == other_modifiers && key == other_key),
                    "{command:?}의 단축키가 앞선 항목과 겹친다"
                );
            }
        }
    }

    #[test]
    fn fr12_기본_단축키가_모두_들어_있다() {
        // 네 방향 Ctrl+Alt 조합은 `네_방향_단축키가_모두_배정돼_있다`가 따로 덮는다
        let table = shortcut_table();
        let has = |modifiers, key| table.iter().any(|(m, k, _)| *m == modifiers && *k == key);
        assert!(has(egui::Modifiers::CTRL, egui::Key::T)); // 새 탭
        assert!(has(egui::Modifiers::CTRL, egui::Key::W)); // 탭 닫기
        assert!(has(egui::Modifiers::ALT, egui::Key::ArrowLeft)); // 뒤로
        assert!(has(egui::Modifiers::ALT, egui::Key::ArrowRight)); // 앞으로
        assert!(has(egui::Modifiers::NONE, egui::Key::F5)); // 새로 고침
        assert!(has(egui::Modifiers::CTRL, egui::Key::Backslash)); // 오른쪽 분할
        assert!(has(
            egui::Modifiers::CTRL | egui::Modifiers::SHIFT,
            egui::Key::Backslash
        )); // 아래쪽 분할
    }

    /// 주어진 그리기를 한 번 실행하고 **그려진 텍스트를 순서대로** 모은다.
    /// 구분선은 글자가 없어 잡히지 않으므로 항목 문구만 남는다
    fn drawn_labels(draw: impl FnMut(&mut egui::Ui)) -> Vec<String> {
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
        crate::ui::app::install_fonts(&ctx);
        let output = ctx.run_ui(Default::default(), draw);
        let mut labels = Vec::new();
        for clipped in &output.shapes {
            collect(&clipped.shape, &mut labels);
        }
        labels
    }

    /// 패널 메뉴 본문을 그려 라벨을 모은다
    fn menu_labels(state: PanelMenuState) -> Vec<String> {
        drawn_labels(|ui| {
            let mut command = None;
            panel_menu_items(ui, state, &mut command);
        })
    }

    /// '보기' 하위 메뉴를 그려 라벨을 모은다 — 호버로 열리는 팝업이라
    /// 패널 메뉴를 그리는 것만으로는 잡히지 않아 직접 부른다
    fn view_labels(current: ViewMode) -> Vec<String> {
        drawn_labels(|ui| {
            let mut command = None;
            view_items(ui, current, &mut command);
        })
    }

    #[test]
    fn 패널_메뉴는_요청한_순서와_문구를_그린다() {
        // plan `## 시각 요소 분해`의 인벤토리 표 13행 중 글자가 있는 항목들.
        // 메뉴 바를 없앤 뒤 이 메뉴가 유일한 마우스 진입점이라, 항목이 빠지면 그 기능에
        // 마우스로 닿을 수 없게 된다
        let labels = menu_labels(PanelMenuState::for_panes(2, ViewMode::Details));
        let expected = [
            "보기",
            "오른쪽 분할",
            "왼쪽 분할",
            "위쪽 분할",
            "아래쪽 분할",
            "새로 고침",
            "새 파일",
            "새 폴더",
            "닫기",
        ];
        let found: Vec<&String> = labels
            .iter()
            .filter(|label| expected.contains(&label.as_str()))
            .collect();
        assert_eq!(
            found,
            expected.iter().collect::<Vec<_>>(),
            "메뉴 항목의 문구나 순서가 인벤토리와 다르다: {labels:?}"
        );
    }

    #[test]
    fn 보기_하위_메뉴는_여덟_모드를_순서대로_그린다() {
        // plan `### 참조 정합 인벤토리 — '보기' 하위 메뉴` 8행 그대로여야 한다
        let labels = view_labels(ViewMode::Details);
        let expected: Vec<String> = ViewMode::ALL
            .iter()
            .map(|mode| mode.label().to_owned())
            .collect();
        let found: Vec<String> = labels
            .iter()
            .map(|label| label.trim_start_matches(['•', ' ']).to_owned())
            .filter(|label| expected.contains(label))
            .collect();
        assert_eq!(
            found, expected,
            "보기 항목의 문구나 순서가 다르다: {labels:?}"
        );
    }

    #[test]
    fn 지금_쓰는_모드에만_점이_붙는다() {
        // 점이 없거나 여러 개면 어느 모드로 보고 있는지 알 수 없다 (4번 이미지의 표시 방식)
        for current in [ViewMode::Details, ViewMode::Tiles, ViewMode::List] {
            let marked: Vec<String> = view_labels(current)
                .into_iter()
                .filter(|label| label.starts_with('•'))
                .collect();
            assert_eq!(
                marked.len(),
                1,
                "{current:?}: 점이 하나가 아니다 — {marked:?}"
            );
            assert!(
                marked[0].contains(current.label()),
                "{current:?}: 점이 엉뚱한 항목에 붙었다 — {marked:?}"
            );
        }
    }

    #[test]
    fn 마지막_패널_하나는_닫을_수_없다() {
        // FR-2 — 이 조건이 뒤집히면 마지막 패널을 닫아 빈 화면이 된다
        let mode = ViewMode::Details;
        assert!(!PanelMenuState::for_panes(1, mode).can_close_panel);
        assert!(PanelMenuState::for_panes(2, mode).can_close_panel);
        assert!(PanelMenuState::for_panes(4, mode).can_close_panel);
        // 패널이 0개인 상태는 정상 흐름에 없지만, 그때도 닫기를 열어주면 안 된다
        assert!(!PanelMenuState::for_panes(0, mode).can_close_panel);
    }

    #[test]
    fn 메뉴_바가_없어도_단축키는_모두_살아_있다() {
        // 메뉴 바를 지우면서 잃은 것이 없어야 한다 — 이동·탭 명령은 이제 단축키와
        // 주소창·탭 스트립 버튼으로만 닿는다
        let table = shortcut_table();
        for command in [
            Command::NewTab,
            Command::CloseTab,
            Command::Back,
            Command::Forward,
            Command::Up,
            Command::Refresh,
            Command::ClosePanel,
            Command::ToggleSidebar,
        ] {
            assert!(
                table.iter().any(|(_, _, c)| *c == command),
                "{command:?}의 단축키가 사라졌다"
            );
        }
    }

    #[test]
    fn 무수식_키는_f5_외에_두지_않는다() {
        // F2·Delete를 단축키로 두면 이름 편집 중 텍스트 입력을 가로챈다(현행과 같은 결정)
        for (modifiers, key, _) in shortcut_table() {
            if modifiers == egui::Modifiers::NONE {
                assert_eq!(key, egui::Key::F5);
            }
        }
    }
}
