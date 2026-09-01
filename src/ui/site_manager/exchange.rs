//! 사이트 목록 내보내기·가져오기 흐름 (FR-59) — `ui::site_manager`의 자식 모듈.
//!
//! 좌측 아랫줄 버튼 둘과 그 뒤에 이어지는 대화 둘(가져오기 암호 · 겹치는 사이트 확인)이
//! 여기 있다. **내보내기 쪽에는 대화가 없다** — 봉인을 앱 내장 키로 하므로 물을 것이 없고,
//! 버튼을 누르면 곧바로 저장할 자리를 청한다. 부모(`SiteManager`)의 private 필드를 그대로
//! 만지므로 가시성을 넓히지 않는다 — 모듈을 나눈 까닭은 부모 파일의 모듈 주석에 있다.
//!
//! **파일 대화는 여기서 띄우지 않는다** (plan D7) — `IFileDialog::Show`가 자체 메시지 루프를
//! 돌려 이벤트 루프를 재진입시키므로, 「필요하다」만 세워 두고 앱이 프레임을 다 그린 뒤 가져간다.
use std::path::PathBuf;

use eframe::egui;

use super::SiteManager;
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
    /// 개인 키 파일을 고른다 (FR-66) — 확장자가 없는 키가 흔해 모든 파일을 보인다
    OpenKey,
}

/// 내보내기·가져오기가 지나는 단계 (FR-59).
///
/// 한 번에 하나만 진행한다 — 두 흐름이 겹치면 어느 대화가 어느 파일을 기다리는지 알 수 없다
#[derive(Debug, Clone, PartialEq, Default)]
pub(super) enum Exchange {
    #[default]
    Idle,
    /// 파일 저장 자리를 기다리는 중.
    ///
    /// **들고 다닐 암호가 없다** — 봉인은 앱 내장 키로 하므로 사용자에게 받을 것이 없다
    ExportWaitFile,
    /// 열 파일을 기다리는 중
    ImportWaitFile,
    /// 개인 키 파일을 기다리는 중 (FR-66) — 고르면 초안의 키 경로 칸에 들어간다
    KeyWaitFile,
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
            Exchange::ExportWaitFile => self.finish_export(&path, store),
            Exchange::ImportWaitFile => self.begin_import(&path, store),
            // 고른 경로를 초안에 싣기만 한다 — 저장은 사용자가 `확인`을 눌렀을 때다
            Exchange::KeyWaitFile => self.draft.key_path = path.to_string_lossy().into_owned(),
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

    /// 개인 키 파일을 고르는 대화를 청한다 (FR-66).
    ///
    /// 내보내기·가져오기와 **같은 통로**를 쓴다 — 앱이 프레임을 다 그린 뒤 대화를 띄우고
    /// 결과를 [`supply_file`]로 되돌린다. 한 번에 하나만 진행하므로 다른 흐름 중이면
    /// 그것을 접고 이 흐름으로 바꾼다
    ///
    /// [`supply_file`]: SiteManager::supply_file
    pub(super) fn request_key_file(&mut self) {
        self.error = None;
        self.pending_file = Some(FileRequest::OpenKey);
        self.exchange = Exchange::KeyWaitFile;
    }

    /// 좌측 아랫줄 버튼을 흐름으로 잇는다 (FR-59)
    pub(super) fn apply_exchange_action(&mut self, action: ExchangeAction) {
        self.error = None;
        self.exchange = match action {
            // 물을 것이 없다 — 봉인이 앱 내장 키라 곧바로 저장할 자리를 청한다 (plan D2)
            ExchangeAction::Export => {
                self.pending_file = Some(FileRequest::Save {
                    suggested: crate::i18n::file_dialog_export_name().to_owned(),
                });
                Exchange::ExportWaitFile
            }
            ExchangeAction::Import => {
                self.pending_file = Some(FileRequest::Open);
                Exchange::ImportWaitFile
            }
        };
    }

    /// 고른 자리에 문서를 쓴다 (FR-59)
    fn finish_export(&mut self, path: &std::path::Path, store: &SiteStore) {
        let outcome = match site_export::build(store) {
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
        if !self.settle_import(&document, "", store) {
            // 여기서의 `false`는 「암호가 틀렸다」가 아니다 — 이 경로는 암호를 물은 적이 없다.
            // 앱 내장 키 봉투가 변조됐거나 둘 중 어느 열쇠도 아닌 `kdf`라는 뜻이라 손상으로 알린다.
            // 알리지 않으면 사용자에게는 파일을 골랐는데 아무 일도 없는 것으로 보인다
            self.error = Some(crate::i18n::site_import_broken().to_owned());
        }
    }

    /// 문서로 계획을 세운다 — 겹치는 것이 있으면 묻고, 없으면 그대로 반영한다.
    ///
    /// **봉투를 열지 못하면 `false`를 돌려주고 사유는 남기지 않는다** — 그 뜻이 호출부마다
    /// 다르기 때문이다. 암호 대화 안에서는 「암호가 맞지 않는다」이고, 대화를 거치지 않은
    /// 경로에서는 「파일이 손상됐다」이다
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
                    crate::i18n::site_import_passphrase(),
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
    fn 내보내기는_곧바로_파일을_청한다() {
        let (mut manager, mut store) = manager_with_two_sites();
        assert_eq!(manager.exchange, Exchange::Idle);

        // 버튼 한 번으로 저장할 자리를 청한다 — 중간에 묻는 대화가 없다 (plan D2)
        manager.apply_exchange_action(ExchangeAction::Export);
        assert_eq!(manager.exchange, Exchange::ExportWaitFile);
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
        manager.apply_exchange_action(ExchangeAction::Export);
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
        manager.apply_exchange_action(ExchangeAction::Export);
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
    fn 구버전_암호_보호_파일은_암호를_묻는다() {
        // 직전 버전이 사용자 암호로 만든 파일. 지금의 내보내기는 이 형태를 만들지 않으므로
        // 픽스처로 세워 파일에 쓴다 — 그 파일을 아직 열 수 있는지가 이 시험의 명제다
        let (mut manager, store) = manager_with_two_sites();
        let path = temp_path("import-passphrase");
        let sites = site_export::build(&store).expect("내보내기").document.sites;
        let legacy = site_export::legacy_document(
            &sites,
            // `sites`와 같은 순서 — 둘째 사이트는 비밀번호가 없다
            &["비밀!1234".to_owned(), String::new()],
            "맞는 암호",
        );
        site_export::write_file(&path, &legacy).expect("파일 쓰기");

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
        // 이 경로의 사유는 대화 안에 남는다 — 바닥에 「손상됐다」가 뜨면 안 된다 (Phase F M1)
        assert_eq!(manager.error, None, "암호 대화 경로가 바닥에 사유를 남겼다");

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
    fn 봉투를_열지_못하면_대화_없이_사유를_남긴다() {
        // 앱 내장 키 봉투는 암호를 묻지 않고 곧바로 열려 하므로, 열지 못하면 그 자리에서
        // 알려야 한다 — 알리지 않으면 파일을 골랐는데 아무 일도 없는 것으로 보인다 (Phase F M1)
        let (mut manager, mut store) = manager_with_two_sites();

        // ⓐ 봉인 바이트가 손상된 파일
        let tampered = temp_path("import-tampered");
        let mut document = site_export::build(&store).expect("내보내기").document;
        if let Some(sealed) = document.secret.as_mut() {
            sealed.ciphertext = "00".repeat(sealed.ciphertext.len() / 2);
        }
        site_export::write_file(&tampered, &document).expect("파일 쓰기");
        manager.apply_exchange_action(ExchangeAction::Import);
        let _ = manager.take_file_request();
        manager.supply_file(Some(tampered.clone()), &mut store);
        assert_eq!(manager.exchange, Exchange::Idle);
        assert_eq!(manager.take_notice(), None, "가져온 것이 없다");
        assert!(manager.error.is_some(), "사유가 남지 않았다");
        let _ = std::fs::remove_file(&tampered);

        // ⓑ 둘 중 어느 열쇠도 아닌 `kdf` — 더 새로운 앱이 만든 파일이 여기 걸린다
        let unknown = temp_path("import-unknown-kdf");
        let mut document = site_export::build(&store).expect("내보내기").document;
        if let Some(sealed) = document.secret.as_mut() {
            sealed.kdf = "PBKDF2-HMAC-SHA512-미래".to_owned();
        }
        site_export::write_file(&unknown, &document).expect("파일 쓰기");
        manager.error = None;
        manager.apply_exchange_action(ExchangeAction::Import);
        let _ = manager.take_file_request();
        manager.supply_file(Some(unknown.clone()), &mut store);
        assert!(manager.error.is_some(), "사유가 남지 않았다");
        let _ = std::fs::remove_file(&unknown);
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
        manager.apply_exchange_action(ExchangeAction::Export);
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
        // 암호를 들고 있는 단계는 이제 가져오기 쪽 하나뿐이다
        let (mut manager, mut store) = manager_with_two_sites();
        let document = site_export::build(&store).expect("내보내기").document;
        manager.exchange = Exchange::ImportAsk {
            document: Box::new(document),
            pass: "적던 암호".to_owned(),
            error: None,
        };
        manager.pending_file = Some(FileRequest::Open);
        manager.close(&mut store);
        assert_eq!(manager.exchange, Exchange::Idle);
        assert_eq!(manager.pending_file, None);
    }

    #[test]
    fn 파일_창을_열지_못하면_사유를_남기고_흐름을_접는다() {
        // 창 핸들이 없는 환경 — 앱이 `fail_file_request`로 알린다 (FR-59).
        // 조용히 접으면 사용자는 버튼이 먹지 않는다고 읽는다
        let (mut manager, _) = manager_with_two_sites();
        manager.apply_exchange_action(ExchangeAction::Import);
        assert_eq!(manager.exchange, Exchange::ImportWaitFile);

        manager.fail_file_request("파일 창을 열지 못했습니다");
        assert_eq!(manager.exchange, Exchange::Idle, "하던 흐름을 접는다");
        assert_eq!(
            manager.error.as_deref(),
            Some("파일 창을 열지 못했습니다"),
            "사유가 바닥에 남지 않았다"
        );
    }

    #[test]
    fn 파일을_기다리던_중이_아니면_받을_것이_없다() {
        let (mut manager, mut store) = manager_with_two_sites();
        let document = site_export::build(&store).expect("내보내기").document;
        let plan = site_export::plan_import(&document, &store, "").expect("계획");
        let stage = Exchange::ImportConflict {
            plan: Box::new(plan),
        };
        manager.exchange = stage.clone();
        manager.supply_file(Some(temp_path("stray")), &mut store);
        assert_eq!(manager.exchange, stage, "하던 단계를 잃지 않는다");
        assert_eq!(manager.take_notice(), None);
    }

    #[test]
    fn 내보내기는_사이트가_있을_때만_누를_수_있다() {
        // 활성 판정은 `SiteManager::show_bottom_buttons`가 `store.is_empty()`로 한다 —
        // 그리기 없이 그 조건만 견준다
        let store = SiteStore::new();
        assert!(store.is_empty(), "빈 목록이면 내보낼 것이 없다");
        let (_, filled) = manager_with_two_sites();
        assert!(!filled.is_empty(), "사이트가 있으면 누를 수 있다");
    }

    #[test]
    fn 가져오기_대화가_한_프레임을_그린다() {
        // 이 모듈에 남은 대화 둘의 그리기 경로가 패닉 없이 도는지 본다 (FR-59).
        // 내보내기 쪽은 대화가 없어져 그릴 것이 없다
        let (mut manager, mut store) = manager_with_two_sites();
        let ctx = egui::Context::default();
        let document = site_export::build(&store).expect("내보내기").document;
        manager.exchange = Exchange::ImportAsk {
            document: Box::new(document.clone()),
            pass: String::new(),
            error: Some("맞지 않습니다".to_owned()),
        };
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });

        let plan = site_export::plan_import(&document, &store, "").expect("계획");
        manager.exchange = Exchange::ImportConflict {
            plan: Box::new(plan),
        };
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });
    }
}
