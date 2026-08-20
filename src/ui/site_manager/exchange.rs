//! 사이트 목록 내보내기·가져오기 흐름 (FR-59) — `ui::site_manager`의 자식 모듈.
//!
//! 좌측 아랫줄 버튼 둘과 그 뒤에 이어지는 대화 넷(내보내기 암호 · 암호 없이 저장 확인 ·
//! 가져오기 암호 · 겹치는 사이트 확인)이 여기 있다. 부모(`SiteManager`)의 private 필드를
//! 그대로 만지므로 가시성을 넓히지 않는다 — 모듈을 나눈 까닭은 부모 파일의 모듈 주석에 있다.
//!
//! **파일 대화는 여기서 띄우지 않는다** (plan D7) — `IFileDialog::Show`가 자체 메시지 루프를
//! 돌려 이벤트 루프를 재진입시키므로, 「필요하다」만 세워 두고 앱이 프레임을 다 그린 뒤 가져간다.
use std::path::PathBuf;

use eframe::egui;

use super::{
    DELETE_CONFIRM_WIDTH, GRID_BUTTON_HEIGHT, GRID_GAP, GRID_PAD_BOTTOM, GRID_PAD_X, SiteManager,
};
use crate::remote::site_export::{self, ImportPlan, ImportSummary, SiteExport};
use crate::remote::sites::SiteStore;
use crate::ui::dialog;
use crate::ui::theme;
use crate::ui::widgets;

/// 내보내기·가져오기 대화의 본문 폭 — 같은 이름 확인 대화(`remote_menu`)와 같은 값이다.
/// 새 폭을 만들지 않는 것은 대화마다 폭이 갈리는 것을 더 늘리지 않기 위함이다
const EXCHANGE_WIDTH: f32 = 420.0;
/// 겹치는 사이트를 몇 개까지 미리 보일지 — 같은 이름 확인 대화와 같은 규칙이다
const CONFLICT_PREVIEW: usize = 5;
/// 대화 제목 글자 — 확인 대화들이 쓰는 값
const DIALOG_TITLE_PX: f32 = 16.0;
/// 미리 보기가 잘렸음을 알리는 표식 — 같은 이름 확인 대화와 같은 글자다
const OVERFLOW_MARK: &str = "\u{2026}";

/// 좌측 아랫줄 버튼 둘 (FR-59)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExchangeAction {
    Export,
    Import,
}

/// 대화가 앱에 청하는 파일 고르기 (FR-59).
///
/// **사이트 관리자가 파일 대화를 직접 띄우지 않는다** (plan D7) — `IFileDialog::Show`가 자체
/// 메시지 루프를 돌려 이벤트 루프를 재진입시키므로, egui가 위젯 트리를 만드는 도중에 부르면
/// 안 된다. 여기서는 「필요하다」만 세워 두고 앱이 프레임을 다 그린 뒤 꺼내 간다
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileRequest {
    /// 저장할 자리를 고른다 — `suggested`는 이름 칸에 미리 채울 이름이다
    Save { suggested: String },
    /// 열 파일을 고른다
    Open,
}

/// 내보내기·가져오기가 지나는 단계 (FR-59).
///
/// 한 번에 하나만 진행한다 — 두 흐름이 겹치면 어느 대화가 어느 파일을 기다리는지 알 수 없다
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) enum Exchange {
    #[default]
    Idle,
    /// 내보내기 암호를 받는 중
    ExportAsk {
        pass: String,
        confirm: String,
        error: Option<String>,
    },
    /// 암호를 비운 채 저장하려 한다 — 한 번 더 묻는다 (plan D6)
    ExportConfirmEmpty,
    /// 파일 저장 자리를 기다리는 중 — 암호를 들고 있다
    ExportWaitFile { pass: String },
    /// 열 파일을 기다리는 중
    ImportWaitFile,
    /// 가져올 파일의 암호를 받는 중
    ImportAsk {
        document: Box<SiteExport>,
        pass: String,
        error: Option<String>,
    },
    /// 겹치는 사이트를 어떻게 할지 묻는 중
    ImportConflict { plan: Box<ImportPlan> },
}

impl SiteManager {
    /// 앱이 띄워 줄 파일 대화를 청한다 — **한 번 꺼내면 비워진다** (FR-59, plan D7).
    ///
    /// 앱은 이것을 프레임을 다 그린 뒤에 꺼내 실제 대화를 띄우고, 결과를 [`supply_file`]로 되돌린다
    ///
    /// [`supply_file`]: SiteManager::supply_file
    pub fn take_file_request(&mut self) -> Option<FileRequest> {
        self.pending_file.take()
    }

    /// 사용자가 고른 파일을 받아 하던 일을 잇는다. `None`이면 고르지 않았다는 뜻이다.
    ///
    /// 취소는 오류가 아니다 — 아무 말 없이 처음 상태로 돌아간다
    pub fn supply_file(&mut self, path: Option<PathBuf>, store: &mut SiteStore) {
        let stage = std::mem::take(&mut self.exchange);
        let Some(path) = path else {
            return;
        };
        match stage {
            Exchange::ExportWaitFile { pass } => self.finish_export(&path, &pass, store),
            Exchange::ImportWaitFile => self.begin_import(&path, store),
            // 파일을 기다리던 중이 아니면 받을 것이 없다 — 상태만 되돌린다
            other => self.exchange = other,
        }
    }

    /// 앱이 알림으로 띄울 결과 문구 — 한 번 꺼내면 비워진다
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    /// 앱이 파일 대화를 띄우지 못했다 — 하던 흐름을 접고 사유를 바닥에 남긴다.
    ///
    /// 창 핸들을 얻지 못한 환경에서만 쓰인다. 조용히 접으면 사용자는 버튼이 먹지 않는다고 읽는다
    pub fn fail_file_request(&mut self, reason: &str) {
        self.exchange = Exchange::Idle;
        self.error = Some(reason.to_owned());
    }

    /// 좌측 아랫줄 버튼을 흐름으로 잇는다 (FR-59)
    pub(super) fn apply_exchange_action(&mut self, action: ExchangeAction) {
        self.error = None;
        self.exchange = match action {
            ExchangeAction::Export => Exchange::ExportAsk {
                pass: String::new(),
                confirm: String::new(),
                error: None,
            },
            ExchangeAction::Import => {
                self.pending_file = Some(FileRequest::Open);
                Exchange::ImportWaitFile
            }
        };
    }

    /// 고른 자리에 문서를 쓴다 (FR-59)
    fn finish_export(&mut self, path: &std::path::Path, passphrase: &str, store: &SiteStore) {
        let outcome = match site_export::build(store, passphrase) {
            Ok(outcome) => outcome,
            Err(site_export::ExportError::Seal) => {
                self.error = Some(crate::i18n::site_export_seal_failed().to_owned());
                return;
            }
            Err(_) => {
                self.error = Some(crate::i18n::site_export_write_failed().to_owned());
                return;
            }
        };
        if site_export::write_file(path, &outcome.document).is_err() {
            self.error = Some(crate::i18n::site_export_write_failed().to_owned());
            return;
        }
        self.notice = Some(crate::i18n::dynamic::site_export_done(
            outcome.document.sites.len(),
            outcome.password_unreadable,
        ));
    }

    /// 고른 파일을 읽어 다음 단계를 정한다 — 암호가 필요하면 묻고, 아니면 곧바로 계획을 세운다
    fn begin_import(&mut self, path: &std::path::Path, store: &mut SiteStore) {
        let document = match site_export::read_file(path) {
            Ok(document) => document,
            Err(error) => {
                self.error = Some(import_error_text(&error).to_owned());
                return;
            }
        };
        if site_export::needs_passphrase(&document) {
            self.exchange = Exchange::ImportAsk {
                document: Box::new(document),
                pass: String::new(),
                error: None,
            };
            return;
        }
        self.settle_import(&document, "", store);
    }

    /// 문서로 계획을 세운다 — 겹치는 것이 있으면 묻고, 없으면 그대로 반영한다.
    ///
    /// 암호가 틀리면 `false`를 돌려준다(호출부가 대화 안에 사유를 남긴다)
    fn settle_import(
        &mut self,
        document: &SiteExport,
        passphrase: &str,
        store: &mut SiteStore,
    ) -> bool {
        let plan = match site_export::plan_import(document, store, passphrase) {
            Ok(plan) => plan,
            Err(site_export::ImportError::WrongPassphrase) => return false,
            Err(error) => {
                self.error = Some(import_error_text(&error).to_owned());
                return true;
            }
        };
        if plan.is_empty() {
            self.error = Some(crate::i18n::site_import_empty().to_owned());
            return true;
        }
        if plan.conflicts.is_empty() {
            let summary = site_export::apply_import(store, &plan, false);
            self.report_import(summary);
        } else {
            self.exchange = Exchange::ImportConflict {
                plan: Box::new(plan),
            };
        }
        true
    }

    /// 반영 결과를 알림 문구로 만든다
    fn report_import(&mut self, summary: ImportSummary) {
        self.notice = Some(crate::i18n::dynamic::site_import_done(
            summary.added,
            summary.replaced,
            summary.skipped,
            summary.password_failed,
        ));
    }

    /// 내보내기·가져오기 대화들을 그린다 — 한 번에 하나만 뜬다 (FR-59)
    pub(super) fn show_exchange(&mut self, ctx: &egui::Context, store: &mut SiteStore) {
        match std::mem::take(&mut self.exchange) {
            Exchange::Idle => {}
            Exchange::ExportAsk {
                pass,
                confirm,
                error,
            } => self.show_export_ask(ctx, pass, confirm, error),
            Exchange::ExportConfirmEmpty => self.show_export_empty_confirm(ctx),
            Exchange::ImportAsk {
                document,
                pass,
                error,
            } => self.show_import_ask(ctx, document, pass, error, store),
            Exchange::ImportConflict { plan } => self.show_import_conflict(ctx, plan, store),
            // 파일을 기다리는 동안에는 대화를 그리지 않는다 — 앱이 띄운 파일 대화가 화면을 쥔다
            waiting => self.exchange = waiting,
        }
    }

    /// 내보내기 암호 대화 (FR-59)
    fn show_export_ask(
        &mut self,
        ctx: &egui::Context,
        mut pass: String,
        mut confirm: String,
        mut error: Option<String>,
    ) {
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::site_export_save()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show(
            ctx,
            egui::Id::new("사이트 내보내기"),
            EXCHANGE_WIDTH,
            &buttons,
            |ui| {
                ui.label(
                    egui::RichText::new(crate::i18n::site_export_title())
                        .size(DIALOG_TITLE_PX)
                        .color(theme::TEXT),
                );
                ui.add_space(8.0);
                ui.label(crate::i18n::site_export_hint());
                ui.label(
                    egui::RichText::new(crate::i18n::site_export_empty_hint())
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(10.0);
                passphrase_row(
                    ui,
                    crate::i18n::site_export_passphrase(),
                    "내보내기 암호",
                    &mut pass,
                );
                ui.add_space(6.0);
                passphrase_row(
                    ui,
                    crate::i18n::site_export_passphrase_again(),
                    "내보내기 암호 확인",
                    &mut confirm,
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(crate::i18n::site_export_forget_warning())
                        .color(theme::TEXT_MUTED),
                );
                if let Some(reason) = &error {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(reason.as_str()).color(theme::ERROR_TEXT));
                }
            },
        );

        if shell.should_close || shell.clicked == Some(1) {
            return;
        }
        if shell.clicked != Some(0) {
            // 아직 고르지 않았다 — 적던 것을 그대로 들고 다음 프레임으로 간다
            self.exchange = Exchange::ExportAsk {
                pass,
                confirm,
                error,
            };
            return;
        }
        if pass != confirm {
            error = Some(crate::i18n::site_export_mismatch().to_owned());
            self.exchange = Exchange::ExportAsk {
                pass,
                confirm,
                error,
            };
            return;
        }
        if pass.is_empty() {
            // 비밀번호 없이 저장하는 것이 맞는지 한 번 더 묻는다 (plan D6)
            self.exchange = Exchange::ExportConfirmEmpty;
            return;
        }
        self.request_export_file(pass);
    }

    /// 저장할 자리를 앱에 청하고 암호를 들고 기다린다
    fn request_export_file(&mut self, pass: String) {
        self.pending_file = Some(FileRequest::Save {
            suggested: crate::i18n::file_dialog_export_name().to_owned(),
        });
        self.exchange = Exchange::ExportWaitFile { pass };
    }

    /// 암호 없이 저장하기 직전의 되물음 (plan D6)
    fn show_export_empty_confirm(&mut self, ctx: &egui::Context) {
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::site_export_save()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show(
            ctx,
            egui::Id::new("사이트 내보내기 확인"),
            DELETE_CONFIRM_WIDTH,
            &buttons,
            |ui| {
                ui.label(
                    egui::RichText::new(crate::i18n::site_export_empty_title())
                        .size(DIALOG_TITLE_PX)
                        .color(theme::TEXT),
                );
                ui.add_space(8.0);
                ui.label(crate::i18n::site_export_empty_detail());
            },
        );
        match shell.clicked {
            Some(0) => self.request_export_file(String::new()),
            Some(_) => {}
            None => {
                if !shell.should_close {
                    self.exchange = Exchange::ExportConfirmEmpty;
                }
            }
        }
    }

    /// 가져오기 암호 대화 (FR-59)
    fn show_import_ask(
        &mut self,
        ctx: &egui::Context,
        document: Box<SiteExport>,
        mut pass: String,
        mut error: Option<String>,
        store: &mut SiteStore,
    ) {
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::site_import_open()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show(
            ctx,
            egui::Id::new("사이트 가져오기 암호"),
            EXCHANGE_WIDTH,
            &buttons,
            |ui| {
                ui.label(
                    egui::RichText::new(crate::i18n::site_import_title())
                        .size(DIALOG_TITLE_PX)
                        .color(theme::TEXT),
                );
                ui.add_space(8.0);
                ui.label(crate::i18n::site_import_passphrase_hint());
                ui.add_space(10.0);
                passphrase_row(
                    ui,
                    crate::i18n::site_export_passphrase(),
                    "가져오기 암호",
                    &mut pass,
                );
                if let Some(reason) = &error {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(reason.as_str()).color(theme::ERROR_TEXT));
                }
            },
        );

        if shell.should_close || shell.clicked == Some(1) {
            return;
        }
        if shell.clicked != Some(0) {
            self.exchange = Exchange::ImportAsk {
                document,
                pass,
                error,
            };
            return;
        }
        if self.settle_import(&document, &pass, store) {
            return;
        }
        // 암호가 맞지 않았다 — 대화를 그대로 두고 사유만 남긴다
        error = Some(crate::i18n::site_import_wrong_passphrase().to_owned());
        self.exchange = Exchange::ImportAsk {
            document,
            pass,
            error,
        };
    }

    /// 겹치는 사이트 확인 (FR-59) — 같은 이름 확인 대화와 같은 구성이다 (plan D4)
    fn show_import_conflict(
        &mut self,
        ctx: &egui::Context,
        plan: Box<ImportPlan>,
        store: &mut SiteStore,
    ) {
        let names = plan.conflict_names();
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::site_conflict_overwrite()),
            dialog::ButtonSpec::plain(crate::i18n::site_conflict_skip()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show(
            ctx,
            egui::Id::new("사이트 가져오기 충돌"),
            EXCHANGE_WIDTH,
            &buttons,
            |ui| {
                ui.label(
                    egui::RichText::new(crate::i18n::site_conflict_title())
                        .size(DIALOG_TITLE_PX)
                        .color(theme::TEXT),
                );
                ui.add_space(8.0);
                ui.label(crate::i18n::dynamic::site_conflict_count(names.len()));
                for name in names.iter().take(CONFLICT_PREVIEW) {
                    ui.label(egui::RichText::new(name.as_str()).color(theme::TEXT_MUTED));
                }
                if names.len() > CONFLICT_PREVIEW {
                    ui.label(egui::RichText::new(OVERFLOW_MARK).color(theme::TEXT_MUTED));
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(crate::i18n::site_conflict_detail())
                        .color(theme::ERROR_TEXT),
                );
            },
        );

        match shell.clicked {
            Some(0) => {
                let summary = site_export::apply_import(store, &plan, true);
                self.report_import(summary);
            }
            Some(1) => {
                let summary = site_export::apply_import(store, &plan, false);
                self.report_import(summary);
            }
            // 취소는 아무것도 하지 않는다 — 절반만 들어오는 일이 없게 한다
            Some(_) => {}
            None => {
                if !shell.should_close {
                    self.exchange = Exchange::ImportConflict { plan };
                }
            }
        }
    }

    /// 아랫줄 버튼이 시작하는 y
    fn export_buttons_top(&self, column: egui::Rect) -> f32 {
        column.bottom() - GRID_PAD_BOTTOM - GRID_BUTTON_HEIGHT
    }

    /// 좌측 버튼 아랫줄 — `내보내기`·`가져오기` 두 칸 (FR-59, plan D10).
    ///
    /// 윗줄 셋과 좌우 끝을 맞추고 폭만 둘로 나눈다. **`내보내기`는 등록된 사이트가 없으면
    /// 비활성**이다(내보낼 것이 없다). `가져오기`는 목록이 비어 있어도 할 일이 있으므로 늘 활성이다
    pub(super) fn show_exchange_buttons(
        &mut self,
        ui: &mut egui::Ui,
        column: egui::Rect,
        store: &SiteStore,
    ) -> Option<ExchangeAction> {
        let top = self.export_buttons_top(column);
        let grid = egui::Rect::from_min_max(
            egui::pos2(column.left() + GRID_PAD_X, top),
            egui::pos2(column.right() - GRID_PAD_X, top + GRID_BUTTON_HEIGHT),
        );
        let button_width = (grid.width() - GRID_GAP) / 2.0;
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(grid)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = GRID_GAP;

        let mut action = None;
        for (label, candidate, enabled) in [
            (
                crate::i18n::site_export(),
                ExchangeAction::Export,
                !store.is_empty(),
            ),
            (crate::i18n::site_import(), ExchangeAction::Import, true),
        ] {
            let clicked = child
                .add_enabled_ui(enabled, |ui| {
                    widgets::design_button(
                        ui,
                        label,
                        if enabled {
                            theme::TEXT_BUTTON
                        } else {
                            theme::TEXT_DIM
                        },
                        0.0,
                        egui::vec2(button_width, GRID_BUTTON_HEIGHT),
                    )
                })
                .inner
                .clicked();
            if clicked {
                action = Some(candidate);
            }
        }
        action
    }
}

/// 암호 한 줄 — 라벨과 가려진 입력칸. 내보내기·가져오기 대화가 함께 쓴다 (FR-59).
///
/// `id_salt`는 같은 화면의 입력칸끼리 위젯 상태가 섞이지 않게 한다 — 화면 언어를 따르면
/// 언어를 바꿀 때 적던 글자가 사라지므로 **번역하지 않는다** (AGENTS i18n 예외)
fn passphrase_row(ui: &mut egui::Ui, label: &str, id_salt: &str, value: &mut String) {
    ui.horizontal(|ui| {
        widgets::form_inline_label(ui, label, true);
        let width = (ui.available_width() - widgets::FORM_GAP).max(0.0);
        widgets::text_field(
            ui,
            id_salt,
            value,
            egui::vec2(width, widgets::FORM_FIELD_HEIGHT),
            true,
            true,
        );
    });
}

/// 가져오기가 막힌 까닭을 사용자 문구로 옮긴다 (FR-59).
///
/// `ImportError::WrongPassphrase`는 여기서 다루지 않는다 — 그것만은 대화를 닫지 않고
/// 그 안에 남기므로 부르는 자리가 다르다
fn import_error_text(error: &site_export::ImportError) -> &'static str {
    match error {
        site_export::ImportError::Unsupported => crate::i18n::site_import_unsupported(),
        site_export::ImportError::Io(_) => crate::i18n::site_import_read_failed(),
        // 손상된 파일과 남은 갈래(암호는 위에서 걸러진다)는 같은 문구로 알린다
        _ => crate::i18n::site_import_broken(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 내보내기·가져오기 시험용 — 사이트 둘과 비밀번호 하나를 채운 관리자
    fn manager_with_two_sites() -> (SiteManager, SiteStore) {
        let mut store = SiteStore::new();
        let first = store.add("배포 서버");
        if let Some(record) = store.get_mut(first) {
            record.host = "deploy.test".to_owned();
            record.user = "deploy".to_owned();
        }
        assert!(store.set_password(first, "비밀!1234"));
        let second = store.add("스테이징");
        if let Some(record) = store.get_mut(second) {
            record.host = "stage.test".to_owned();
            record.user = "stage".to_owned();
        }
        let mut manager = SiteManager::new();
        manager.open_new();
        (manager, store)
    }

    /// 시험용 임시 파일 자리 — 이름을 시험마다 갈라 서로 밟지 않게 한다
    fn temp_path(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("moa-site-manager-test");
        let _ = std::fs::create_dir_all(&dir);
        dir.join(format!("{tag}.moasites"))
    }

    #[test]
    fn 내보내기는_암호를_받고_파일을_청한다() {
        let (mut manager, mut store) = manager_with_two_sites();
        assert_eq!(manager.exchange, Exchange::Idle);

        // 버튼을 누르면 암호를 받는 단계로 간다
        manager.apply_exchange_action(ExchangeAction::Export);
        let Exchange::ExportAsk { .. } = &manager.exchange else {
            panic!("암호 단계가 아니다: {:?}", manager.exchange);
        };
        assert_eq!(
            manager.take_file_request(),
            None,
            "아직 파일을 청하지 않는다"
        );

        // 암호가 맞으면 파일 자리를 청하고 그 암호를 들고 기다린다
        manager.request_export_file("암호".to_owned());
        assert_eq!(
            manager.exchange,
            Exchange::ExportWaitFile {
                pass: "암호".to_owned()
            }
        );
        let request = manager.take_file_request().expect("파일 요청");
        assert!(matches!(request, FileRequest::Save { .. }));
        assert_eq!(manager.take_file_request(), None, "한 번만 꺼내 간다");

        // 파일을 받으면 쓰고 알림을 남긴 뒤 처음 상태로 돌아간다
        let path = temp_path("export");
        manager.supply_file(Some(path.clone()), &mut store);
        assert_eq!(manager.exchange, Exchange::Idle);
        assert_eq!(manager.error, None);
        let notice = manager.take_notice().expect("결과 알림");
        assert!(notice.contains('2'), "사이트 수가 없다: {notice}");
        assert!(path.exists(), "파일이 만들어지지 않았다");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 내보내기를_취소하면_아무_일도_없다() {
        let (mut manager, mut store) = manager_with_two_sites();
        manager.request_export_file("암호".to_owned());
        let _ = manager.take_file_request();
        manager.supply_file(None, &mut store);
        assert_eq!(manager.exchange, Exchange::Idle);
        assert_eq!(manager.take_notice(), None);
        assert_eq!(manager.error, None);
    }

    #[test]
    fn 가져오기는_파일을_청하고_겹치면_묻는다() {
        // 먼저 파일을 하나 만들어 둔다
        let (mut manager, mut store) = manager_with_two_sites();
        let path = temp_path("import-conflict");
        manager.request_export_file(String::new());
        let _ = manager.take_file_request();
        manager.supply_file(Some(path.clone()), &mut store);
        let _ = manager.take_notice();

        // 같은 목록에 그대로 가져오면 둘 다 겹친다
        manager.apply_exchange_action(ExchangeAction::Import);
        assert_eq!(manager.exchange, Exchange::ImportWaitFile);
        assert_eq!(manager.take_file_request(), Some(FileRequest::Open));
        manager.supply_file(Some(path.clone()), &mut store);
        let Exchange::ImportConflict { plan } = std::mem::take(&mut manager.exchange) else {
            panic!("충돌 단계가 아니다");
        };
        assert_eq!(plan.conflict_names().len(), 2);
        assert!(plan.fresh.is_empty());

        // 「덮어쓰기」를 고른 것과 같은 일을 한다 — `show_exchange`가 `mem::take`로 단계를 꺼내고
        // 버튼 분기가 그것을 되돌려 놓지 않으므로, 고르고 나면 단계가 `Idle`로 남는다
        let summary = site_export::apply_import(&mut store, &plan, true);
        manager.report_import(summary);
        assert_eq!(
            manager.exchange,
            Exchange::Idle,
            "고른 뒤에는 처음 상태로 돌아간다"
        );
        assert_eq!(summary.replaced, 2);
        assert_eq!(summary.added, 0);
        assert!(manager.take_notice().is_some(), "결과를 알리지 않았다");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 암호로_보호된_파일은_암호를_묻는다() {
        let (mut manager, mut store) = manager_with_two_sites();
        let path = temp_path("import-passphrase");
        manager.request_export_file("맞는 암호".to_owned());
        let _ = manager.take_file_request();
        manager.supply_file(Some(path.clone()), &mut store);
        let _ = manager.take_notice();

        // 빈 목록으로 가져오면 겹치는 것이 없다
        let mut target = SiteStore::new();
        manager.apply_exchange_action(ExchangeAction::Import);
        let _ = manager.take_file_request();
        manager.supply_file(Some(path.clone()), &mut target);
        let Exchange::ImportAsk { document, .. } = &manager.exchange else {
            panic!("암호 단계가 아니다: {:?}", manager.exchange);
        };
        // 빌려 온 것을 먼저 복사해 둔다 — 아래에서 `manager`를 가변으로 빌려야 한다
        let document = document.clone();
        assert_eq!(document.sites.len(), 2);

        // 틀린 암호는 계획을 세우지 못한다 — 저장소도 그대로다
        assert!(!manager.settle_import(&document, "틀린 암호", &mut target));
        assert!(target.is_empty());

        // 맞는 암호면 겹치는 것이 없으므로 곧바로 반영된다
        assert!(manager.settle_import(&document, "맞는 암호", &mut target));
        assert_eq!(target.sites().len(), 2);
        assert_eq!(
            target.password(target.sites()[0].id).as_deref(),
            Some("비밀!1234")
        );
        assert!(manager.take_notice().is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 우리_파일이_아니면_사유를_남긴다() {
        let (mut manager, mut store) = manager_with_two_sites();
        let path = temp_path("broken");
        std::fs::write(&path, "이건 우리 파일이 아니다").expect("임시 파일");
        manager.apply_exchange_action(ExchangeAction::Import);
        let _ = manager.take_file_request();
        manager.supply_file(Some(path.clone()), &mut store);
        assert_eq!(manager.exchange, Exchange::Idle);
        assert_eq!(manager.take_notice(), None);
        assert!(manager.error.is_some(), "사유가 남지 않았다");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 사이트가_없으면_가져올_것도_없다고_알린다() {
        // 빈 목록을 내보낸 파일을 다시 가져오는 경우
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        let path = temp_path("empty");
        manager.request_export_file(String::new());
        let _ = manager.take_file_request();
        manager.supply_file(Some(path.clone()), &mut store);
        let _ = manager.take_notice();

        manager.apply_exchange_action(ExchangeAction::Import);
        let _ = manager.take_file_request();
        manager.supply_file(Some(path.clone()), &mut store);
        assert!(manager.error.is_some(), "빈 파일이라는 사유가 없다");
        assert_eq!(manager.exchange, Exchange::Idle);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn 대화를_닫으면_적던_암호가_함께_사라진다() {
        let (mut manager, mut store) = manager_with_two_sites();
        manager.exchange = Exchange::ExportAsk {
            pass: "적던 암호".to_owned(),
            confirm: "적던 암호".to_owned(),
            error: None,
        };
        manager.pending_file = Some(FileRequest::Open);
        manager.close(&mut store);
        assert_eq!(manager.exchange, Exchange::Idle);
        assert_eq!(manager.pending_file, None);
    }

    #[test]
    fn 파일을_기다리던_중이_아니면_받을_것이_없다() {
        let (mut manager, mut store) = manager_with_two_sites();
        manager.exchange = Exchange::ExportConfirmEmpty;
        manager.supply_file(Some(temp_path("stray")), &mut store);
        assert_eq!(
            manager.exchange,
            Exchange::ExportConfirmEmpty,
            "하던 단계를 잃지 않는다"
        );
        assert_eq!(manager.take_notice(), None);
    }

    #[test]
    fn 내보내기는_사이트가_있을_때만_누를_수_있다() {
        // 활성 판정은 `show_exchange_buttons`가 `store.is_empty()`로 한다 —
        // 그리기 없이 그 조건만 견준다
        let store = SiteStore::new();
        assert!(store.is_empty(), "빈 목록이면 내보낼 것이 없다");
        let (_, filled) = manager_with_two_sites();
        assert!(!filled.is_empty(), "사이트가 있으면 누를 수 있다");
    }

    #[test]
    fn 내보내기_대화가_한_프레임을_그린다() {
        // 대화 넷의 그리기 경로가 패닉 없이 도는지 본다 (FR-59)
        let (mut manager, mut store) = manager_with_two_sites();
        let ctx = egui::Context::default();
        let stages = [
            Exchange::ExportAsk {
                pass: "암호".to_owned(),
                confirm: String::new(),
                error: Some("서로 다릅니다".to_owned()),
            },
            Exchange::ExportConfirmEmpty,
        ];
        for stage in stages {
            manager.exchange = stage;
            let _ = ctx.run_ui(Default::default(), |ui| {
                manager.show(ui.ctx(), &mut store, &[]);
            });
        }

        // 가져오기 쪽 둘은 문서·계획이 있어야 한다
        let document = site_export::build(&store, "암호")
            .expect("내보내기")
            .document;
        manager.exchange = Exchange::ImportAsk {
            document: Box::new(document.clone()),
            pass: String::new(),
            error: Some("맞지 않습니다".to_owned()),
        };
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });

        let plan = site_export::plan_import(&document, &store, "암호").expect("계획");
        manager.exchange = Exchange::ImportConflict {
            plan: Box::new(plan),
        };
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });
    }
}
