//! 원격 목록의 우클릭 메뉴와 그 대화들 (FR-39).
//!
//! **로컬 셸 메뉴와 공통 부품으로 묶지 않는다**(plan 비추상화 선언) — 한쪽은 OS가 그리고
//! (`IContextMenu`, 로컬 PIDL 전용 — D21) 한쪽은 우리가 그린다. 겉모습이 비슷하다는 이유로
//! 한 인터페이스에 넣으면 양쪽 제약이 서로를 갉는다.
//!
//! **실행하지 않는다** — 고른 것을 값으로 돌려주고 연결에 명령을 보내는 것은 `ExplorerApp`이다.
use crate::remote::types::RemotePath;
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

/// 메뉴 폭 — 일반 메뉴와 같은 값 (`FileExplorer-FTP.dc.html:355` 계열)
const MENU_WIDTH: f32 = 180.0;
const ROW_HEIGHT: f32 = 28.0;

/// 확인 대화 안쪽 여백 — egui 기본값(6px)은 글이 테두리에 붙어 보인다.
/// 사이트 관리자 본문·푸터의 좌우 여백(18px)에 맞춘다
const DIALOG_MARGIN: i8 = 18;

/// 확인 대화 아래 버튼의 높이·좌우 여백·사이 간격 — 사이트 관리자 푸터와 같은 값이다
/// (`ui::site_manager`의 `FOOTER_BUTTON_*`). 대화마다 버튼 크기가 다르면 눈에 띈다
const DIALOG_BUTTON_HEIGHT: f32 = 30.0;
const DIALOG_BUTTON_PAD_X: f32 = 24.0;
const DIALOG_BUTTON_GAP: f32 = 10.0;

/// 메뉴가 다룰 원격 항목 하나.
///
/// 폴더 여부와 크기까지 함께 든다 — `받기`가 폴더를 만나면 그 아래를 훑어야 하고(FR-38),
/// 큐는 크기를 알아야 진행률을 그린다. 메뉴가 뜨는 **그 순간의 선택**을 담아 두는 값이다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteTarget {
    pub path: RemotePath,
    pub is_dir: bool,
    pub size: u64,
    /// 서버가 알려 준 권한 비트 — 권한 대화의 시작값이다. 주지 않는 서버면 `None`
    pub mode: Option<u32>,
}

/// 사용자가 원격 메뉴에서 고른 것. 실행은 앱이 한다
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteMenuAction {
    /// 고른 항목을 로컬로 받는다 (전송 큐로 들어간다)
    Download,
    /// **다른 패널에서 고른 로컬 항목**을 이 폴더로 올린다.
    ///
    /// 올릴 것을 파일 대화로 고르게 하지 않는다 — 두 칸 탐색기에서 반대편 패널이 이미
    /// "고른 것"을 들고 있고, 끌어다 놓기(FR-38)도 같은 짝을 쓴다
    Upload,
    /// 이름 바꾸기 대화를 연다 — **하나만 고른 때**만 뜬다(새 이름은 하나뿐이다)
    Rename,
    /// 새 폴더 대화를 연다
    NewFolder,
    /// 권한 변경 대화를 연다
    Chmod,
    /// 삭제 확인 대화를 연다
    Delete,
    /// 목록을 다시 읽는다
    Refresh,
}

/// 원격 목록 우클릭 메뉴를 띄운다 (FR-39).
///
/// `selected`는 지금 고른 원격 항목 수다 — 0이면 **항목이 있어야 뜻이 있는 것**이 비활성이고,
/// 둘 이상이면 `이름 바꾸기`가 비활성이다(새 이름은 하나뿐이라 여럿에 줄 수 없다).
/// `connected`가 거짓이면 서버에 닿는 것이 전부 비활성이다 (plan Edge Case)
pub fn show_remote_menu(
    ui: &mut egui::Ui,
    selected: usize,
    connected: bool,
) -> Option<RemoteMenuAction> {
    ui.set_width(MENU_WIDTH);
    let mut chosen = None;
    for row in menu_rows(selected, connected) {
        // 구분선 앞뒤로 "고른 것에 하는 일"과 "이 폴더에 하는 일"이 나뉜다
        if row.separator_before {
            ui.separator();
        }
        if menu_row(ui, row.label, row.enabled) {
            chosen = Some(row.action);
        }
    }
    chosen
}

/// 메뉴 한 줄의 구성 — 그리기 전에 정해지는 것들
#[derive(Debug, Clone, PartialEq)]
pub struct MenuRow {
    pub label: &'static str,
    pub action: RemoteMenuAction,
    pub enabled: bool,
    pub separator_before: bool,
}

/// 이번에 보일 메뉴 줄들과 각각의 활성 여부 (plan Edge Case).
///
/// 끊긴 연결에서는 **서버에 닿는 것이 전부** 비활성이다 — 눌러도 되지 않는 것을 눌리게 두면
/// 사용자는 눌렀다가 아무 일도 안 일어나는 것을 보게 된다.
/// `올리기`·`새 폴더`·`새로 고침`은 원격 선택이 없어도 뜻이 있다(각각 반대편 패널의 선택,
/// 지금 폴더가 대상이다)
pub fn menu_rows(selected: usize, connected: bool) -> Vec<MenuRow> {
    /// 그 줄이 요구하는 선택 개수
    #[derive(Clone, Copy)]
    enum Needs {
        /// 몇 개든 좋다 — 고른 것이 없어도 뜻이 있다
        Any,
        /// 하나 이상
        Some,
        /// **정확히 하나** — 이름 바꾸기는 새 이름이 하나뿐이라 여럿을 한 번에 다룰 수 없다
        One,
    }
    [
        (
            crate::i18n::remote_download(),
            RemoteMenuAction::Download,
            Needs::Some,
            false,
        ),
        (
            crate::i18n::remote_upload(),
            RemoteMenuAction::Upload,
            Needs::Any,
            false,
        ),
        (
            crate::i18n::remote_rename(),
            RemoteMenuAction::Rename,
            Needs::One,
            false,
        ),
        (
            crate::i18n::remote_chmod(),
            RemoteMenuAction::Chmod,
            Needs::Some,
            false,
        ),
        (
            crate::i18n::remote_delete(),
            RemoteMenuAction::Delete,
            Needs::Some,
            false,
        ),
        (
            crate::i18n::remote_new_folder(),
            RemoteMenuAction::NewFolder,
            Needs::Any,
            true,
        ),
        (
            crate::i18n::menu_refresh(),
            RemoteMenuAction::Refresh,
            Needs::Any,
            false,
        ),
    ]
    .into_iter()
    .map(|(label, action, needs, separator_before)| MenuRow {
        label,
        action,
        enabled: connected
            && match needs {
                Needs::Any => true,
                Needs::Some => selected > 0,
                Needs::One => selected == 1,
            },
        separator_before,
    })
    .collect()
}

/// 메뉴가 차지할 자리 — 화면 밖으로 나가지 않게 위치를 잡는 데 쓴다.
///
/// 테두리·구분선까지 정확히 재지 않는다 — 조금 넉넉하게 잡으면 안쪽으로 더 당겨질 뿐이라
/// 잘리는 것보다 낫다
pub fn menu_size() -> egui::Vec2 {
    let rows = menu_rows(0, false).len() as f32;
    egui::vec2(
        MENU_WIDTH + FRAME_PAD * 2.0,
        rows * ROW_HEIGHT + FRAME_PAD * 4.0,
    )
}

/// 메뉴 테두리와 안쪽 여백을 어림한 값
const FRAME_PAD: f32 = 8.0;

/// 메뉴 한 줄 — 비활성이면 눌리지 않고 글자가 흐려진다
fn menu_row(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    if enabled && response.hovered() {
        ui.painter().rect_filled(rect, 0.0, theme::MENU_HOT);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        if enabled {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
    );
    enabled && response.clicked()
}

/// 원격 이름으로 쓸 수 있는가 (plan Edge Case).
///
/// `/`를 막는 이유: 원격 경로의 구분자라 이름에 들어가면 **다른 폴더를 가리키게** 된다.
/// 서버가 거부해 주기를 기대하지 않는다 — 어떤 서버는 그대로 만들어 버린다
pub fn validate_name(name: &str) -> Result<&str, &'static str> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(crate::i18n::remote_error_empty());
    }
    if trimmed.contains('/') {
        return Err(crate::i18n::remote_error_slash());
    }
    Ok(trimmed)
}

/// 권한 비트 아홉 개 ↔ 8진 세 자리 (FR-39 chmod).
///
/// 화면은 체크박스 아홉 개로 보이고 서버에는 숫자로 간다 — **한 곳에서 옮겨** 두 표현이
/// 어긋나지 않게 한다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Permissions {
    /// `[소유자, 그룹, 기타] × [읽기, 쓰기, 실행]`
    pub bits: [[bool; 3]; 3],
}

impl Permissions {
    /// POSIX 비트에서 만든다 — 아래 세 자리(0o777)만 본다
    pub fn from_mode(mode: u32) -> Permissions {
        let mut bits = [[false; 3]; 3];
        for (group, row) in bits.iter_mut().enumerate() {
            for (bit, slot) in row.iter_mut().enumerate() {
                let shift = (2 - group) * 3 + (2 - bit);
                *slot = mode & (1 << shift) != 0;
            }
        }
        Permissions { bits }
    }

    /// 서버에 보낼 8진 값
    pub fn to_mode(self) -> u32 {
        let mut mode = 0;
        for (group, row) in self.bits.iter().enumerate() {
            for (bit, on) in row.iter().enumerate() {
                if *on {
                    mode |= 1 << ((2 - group) * 3 + (2 - bit));
                }
            }
        }
        mode
    }

    /// 화면에 보이는 세 자리 — `755`
    pub fn to_octal_text(self) -> String {
        format!("{:03o}", self.to_mode())
    }

    /// 적어 넣은 세 자리를 되읽는다. 숫자가 아니거나 범위를 넘으면 `None`
    pub fn from_octal_text(text: &str) -> Option<Permissions> {
        let mode = u32::from_str_radix(text.trim(), 8).ok()?;
        (mode <= 0o777).then(|| Permissions::from_mode(mode))
    }
}

/// 대화가 이번 프레임에 낸 결론 (FR-39).
///
/// **"아직 고르지 않았다"와 "취소했다"를 구분한다** — 둘을 `None` 하나로 뭉개면 호출부가
/// 취소를 알아채지 못해 대화가 닫히지 않는다(spec 리뷰 M1이 이 결함을 짚었다)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogOutcome<T> {
    /// 아직 고르지 않았다 — 다음 프레임에도 그대로 떠 있어야 한다
    Pending,
    /// 확인했다
    Confirmed(T),
    /// 취소·Esc·바깥 클릭 — 대화를 닫는다
    Cancelled,
}

/// 이름 입력 대화 — 이름 바꾸기와 새 폴더가 같은 모양이라 함께 쓴다.
///
/// 확인을 누르면 `Confirmed(이름)`, 취소·Esc·바깥 클릭이면 `Cancelled`다
pub fn show_name_dialog(
    ctx: &egui::Context,
    title: &str,
    name: &mut String,
    error: &mut Option<String>,
) -> DialogOutcome<String> {
    let mut confirmed = None;
    let mut closed = false;
    let response = egui::Modal::new(egui::Id::new(("원격 이름 대화", title))).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(egui::RichText::new(title).size(16.0).color(theme::TEXT));
        ui.add_space(10.0);
        ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
        if let Some(message) = error.as_ref() {
            ui.add_space(6.0);
            ui.label(egui::RichText::new(message).color(theme::ERROR_TEXT));
        }
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(crate::i18n::remote_ok()).clicked() {
                match validate_name(name) {
                    Ok(valid) => confirmed = Some(valid.to_owned()),
                    Err(message) => *error = Some(message.to_owned()),
                }
            }
            if ui.button(crate::i18n::cancel()).clicked() {
                closed = true;
            }
        });
    });
    if response.should_close() {
        closed = true;
    }
    if closed {
        *error = None;
        return DialogOutcome::Cancelled;
    }
    match confirmed {
        Some(name) => DialogOutcome::Confirmed(name),
        None => DialogOutcome::Pending,
    }
}

/// 권한 변경 대화 — 8진 세 자리와 체크박스 아홉 개가 서로 따라간다 (Acceptance ③)
pub fn show_chmod_dialog(
    ctx: &egui::Context,
    permissions: &mut Permissions,
    octal: &mut String,
) -> DialogOutcome<u32> {
    // 문구가 언어를 따르므로 상수가 아니라 그때그때 만든다
    let groups = [
        crate::i18n::remote_owner(),
        crate::i18n::remote_group(),
        crate::i18n::remote_others(),
    ];
    let bits = [
        crate::i18n::remote_read(),
        crate::i18n::remote_write(),
        crate::i18n::remote_execute(),
    ];
    let mut confirmed = None;
    let mut closed = false;
    let response = egui::Modal::new(egui::Id::new("원격 권한 변경")).show(ctx, |ui| {
        ui.set_width(360.0);
        ui.label(
            egui::RichText::new(crate::i18n::remote_chmod_title())
                .size(16.0)
                .color(theme::TEXT),
        );
        ui.add_space(10.0);
        for (group, label) in groups.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(*label).color(theme::HEADER_TEXT));
                for (bit, name) in bits.iter().enumerate() {
                    if ui
                        .checkbox(&mut permissions.bits[group][bit], *name)
                        .changed()
                    {
                        // 체크를 바꾸면 숫자가 따라간다
                        *octal = permissions.to_octal_text();
                    }
                }
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(crate::i18n::remote_chmod_octal()).color(theme::HEADER_TEXT),
            );
            if ui
                .add(egui::TextEdit::singleline(octal).desired_width(80.0))
                .changed()
                && let Some(parsed) = Permissions::from_octal_text(octal)
            {
                // 숫자를 고치면 체크가 따라간다 — 잘못 적은 동안에는 그대로 둔다
                *permissions = parsed;
            }
        });
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(crate::i18n::remote_apply()).clicked() {
                confirmed = Some(permissions.to_mode());
            }
            if ui.button(crate::i18n::cancel()).clicked() {
                closed = true;
            }
        });
    });
    if response.should_close() {
        closed = true;
    }
    if closed {
        return DialogOutcome::Cancelled;
    }
    match confirmed {
        Some(mode) => DialogOutcome::Confirmed(mode),
        None => DialogOutcome::Pending,
    }
}

/// 삭제 확인 대화 (Acceptance ①·Halt Forecast).
///
/// **자동으로 지우는 경로는 없다** — 메뉴에서 곧바로 삭제로 가는 길이 없고, 이 대화가
/// `Confirmed`를 돌려준 자리에서만 명령이 나간다.
///
/// **재귀 여부는 묻지 않는다** — 파일이냐 폴더냐는 목록이 이미 알고(`RemoteTarget::is_dir`)
/// 그에 맞는 명령은 앱이 고른다. 예전에는 `안에 든 것까지 지웁니다` 체크로 그것을
/// 사용자에게 물었는데, 켜도 나가는 것은 `RMD`/`rmdir`(빈 폴더 전용)이라 안에 든 것은
/// 어차피 지워지지 않았다 — 문구가 하는 일과 달랐다.
///
/// 돌려주는 값: `Confirmed(())`면 지운다. 다른 대화들과 같은 결론 타입을 쓴다
pub fn show_delete_confirm(ctx: &egui::Context, targets: &[RemoteTarget]) -> DialogOutcome<()> {
    let mut confirmed = None;
    let mut closed = false;
    let response = egui::Modal::new(egui::Id::new("원격 삭제 확인"))
        // 기본 팝업 모양(채움·테두리·그림자)은 그대로 두고 안쪽 여백만 넓힌다
        .frame(
            egui::Frame::popup(&ctx.style_of(ctx.theme()))
                .inner_margin(egui::Margin::same(DIALOG_MARGIN)),
        )
        .show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(
                egui::RichText::new(crate::i18n::remote_delete_title())
                    .size(16.0)
                    .color(theme::TEXT),
            );
            ui.add_space(8.0);
            ui.label(crate::i18n::dynamic::remote_delete_count(targets.len()));
            for item in targets.iter().take(5) {
                ui.label(egui::RichText::new(item.path.as_str()).color(theme::TEXT_MUTED));
            }
            if targets.len() > 5 {
                ui.label(egui::RichText::new("…").color(theme::TEXT_MUTED));
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(crate::i18n::remote_delete_irreversible())
                    .color(theme::ERROR_TEXT),
            );
            ui.add_space(12.0);
            // 오른쪽부터 그린다 — 사이트 관리자 푸터와 같은 순서(확인·취소)를 뒤집어 넣는다
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = DIALOG_BUTTON_GAP;
                if widgets::design_button(
                    ui,
                    crate::i18n::cancel(),
                    theme::TEXT_BUTTON,
                    DIALOG_BUTTON_PAD_X,
                    egui::vec2(0.0, DIALOG_BUTTON_HEIGHT),
                )
                .clicked()
                {
                    closed = true;
                }
                if widgets::design_button(
                    ui,
                    crate::i18n::delete(),
                    theme::TEXT_BUTTON,
                    DIALOG_BUTTON_PAD_X,
                    egui::vec2(0.0, DIALOG_BUTTON_HEIGHT),
                )
                .clicked()
                {
                    confirmed = Some(());
                }
            });
        });
    if response.should_close() {
        closed = true;
    }
    if closed {
        return DialogOutcome::Cancelled;
    }
    match confirmed {
        Some(()) => DialogOutcome::Confirmed(()),
        None => DialogOutcome::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 권한_비트와_팔진_값이_서로_따라간다() {
        // Acceptance ③ — 화면은 체크박스, 서버는 숫자다. 한 곳에서 옮겨야 어긋나지 않는다
        let permissions = Permissions::from_mode(0o755);
        assert_eq!(permissions.to_mode(), 0o755);
        assert_eq!(permissions.to_octal_text(), "755");
        // 소유자 rwx / 그룹 r-x / 기타 r-x
        assert_eq!(permissions.bits[0], [true, true, true]);
        assert_eq!(permissions.bits[1], [true, false, true]);
        assert_eq!(permissions.bits[2], [true, false, true]);

        // 숫자를 고치면 체크가 따라온다
        let parsed = Permissions::from_octal_text("640").expect("8진");
        assert_eq!(parsed.bits[0], [true, true, false]);
        assert_eq!(parsed.bits[1], [true, false, false]);
        assert_eq!(parsed.bits[2], [false, false, false]);
        assert_eq!(parsed.to_mode(), 0o640);

        // 왕복해도 같다
        for mode in [0o000, 0o600, 0o644, 0o700, 0o777] {
            assert_eq!(Permissions::from_mode(mode).to_mode(), mode);
        }
    }

    #[test]
    fn 잘못_적은_팔진_값은_받아들이지_않는다() {
        // 적는 도중의 값(빈 문자열·`8`)에 체크가 흔들리면 안 된다
        assert!(Permissions::from_octal_text("").is_none());
        assert!(Permissions::from_octal_text("8").is_none());
        assert!(
            Permissions::from_octal_text("1000").is_none(),
            "0o777을 넘는다"
        );
        assert!(Permissions::from_octal_text(" 644 ").is_some());
    }

    #[test]
    fn 이름에_구분자를_넣으면_거부한다() {
        // plan Edge Case — `/`가 들어가면 다른 폴더를 가리키게 된다.
        // 사유 문구는 카탈로그가 정하므로 언어를 고정하고 원문과 견준다
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        assert_eq!(validate_name("보고서.txt"), Ok("보고서.txt"));
        assert_eq!(validate_name("  여백  "), Ok("여백"));
        assert_eq!(validate_name(""), Err("이름을 입력해 주세요."));
        assert_eq!(validate_name("   "), Err("이름을 입력해 주세요."));
        assert_eq!(validate_name("위/아래"), Err("이름에 / 는 쓸 수 없습니다."));
    }

    #[test]
    fn 메뉴가_한_프레임을_그린다() {
        // 선택 개수·연결 유무 조합이 모두 패닉 없이 도는지 본다
        let ctx = egui::Context::default();
        for selected in [0, 1, 3] {
            for connected in [false, true] {
                let _ = ctx.run_ui(Default::default(), |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        show_remote_menu(ui, selected, connected);
                    });
                });
            }
        }
    }

    #[test]
    fn 끊긴_연결에서는_서버에_닿는_줄이_모두_비활성이다() {
        // plan Edge Case — 연결이 끊긴 상태 → 항목 비활성
        for selected in [0, 1, 3] {
            assert!(
                menu_rows(selected, false).iter().all(|row| !row.enabled),
                "끊긴 연결에서 눌리는 줄이 남았다"
            );
        }
    }

    #[test]
    fn 여럿을_고르면_이름_바꾸기가_비활성이다() {
        // quality 리뷰 M1 — 여럿을 고른 채 이름을 바꾸면 첫 항목만 바뀌고 나머지는
        // 아무 말 없이 버려졌다. 새 이름은 하나뿐이므로 애초에 누를 수 없게 한다
        let enabled = |rows: &[MenuRow], action: RemoteMenuAction| {
            rows.iter()
                .find(|row| row.action == action)
                .expect("메뉴 줄")
                .enabled
        };
        let one = menu_rows(1, true);
        assert!(enabled(&one, RemoteMenuAction::Rename));
        let many = menu_rows(3, true);
        assert!(!enabled(&many, RemoteMenuAction::Rename));
        // 여럿에도 뜻이 있는 것은 그대로 열려 있다
        assert!(enabled(&many, RemoteMenuAction::Delete));
        assert!(enabled(&many, RemoteMenuAction::Chmod));
        assert!(enabled(&many, RemoteMenuAction::Download));
    }

    #[test]
    fn 고른_것이_없어도_할_수_있는_일은_남는다() {
        let rows = menu_rows(0, true);
        let enabled: Vec<_> = rows
            .iter()
            .filter(|row| row.enabled)
            .map(|row| row.action.clone())
            .collect();
        assert_eq!(
            enabled,
            vec![
                // 올리기는 **반대편 패널의 선택**이 대상이라 원격 선택과 무관하다
                RemoteMenuAction::Upload,
                RemoteMenuAction::NewFolder,
                RemoteMenuAction::Refresh,
            ]
        );
        // 하나를 고르면 나머지도 열린다
        assert!(menu_rows(1, true).iter().all(|row| row.enabled));
        // 문구는 구현이 정한 신규 문구 그대로다 (디자인 미제공)
        let labels: Vec<_> = rows.iter().map(|row| row.label).collect();
        assert_eq!(
            labels,
            vec![
                "받기",
                "올리기",
                "이름 바꾸기…",
                "권한 변경…",
                "삭제…",
                "새 폴더…",
                "새로 고침"
            ]
        );
    }
}
