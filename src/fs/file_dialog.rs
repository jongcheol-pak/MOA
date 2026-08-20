//! 파일 저장·열기 대화 (FR-59) — Win32 공용 대화 래퍼.
//!
//! 사이트 목록 내보내기·가져오기가 파일을 고르는 유일한 통로다. 셸이 그리는 대화를 그대로 쓰므로
//! 최근 폴더·즐겨찾기·검색이 사용자가 아는 모습으로 나온다.
//!
//! **부르는 자리가 정해져 있다** — `IFileDialog::Show`는 자체 메시지 루프를 돌려 이벤트 루프를
//! 재진입시키므로, egui가 위젯 트리를 절반쯤 만든 상태에서 부르면 안 된다. 셸 컨텍스트
//! 메뉴(`fs::shell_menu`)와 같은 제약이며 `ui::app`이 프레임을 다 그린 뒤에 부른다.
//!
//! 이 모듈에는 시험이 없다 — 실제 대화를 띄워 사람이 고르는 것이 동작의 전부라
//! 자동으로 확인할 수 있는 것이 없다 (AGENTS: HWND가 필요한 UI 로직은 시험 비대상).
use std::path::PathBuf;

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance, CoTaskMemFree};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileOpenDialog, FileSaveDialog, IFileDialog, IFileOpenDialog, IFileSaveDialog,
    SIGDN_FILESYSPATH,
};
use windows::core::{HSTRING, PCWSTR};

/// 내보내기 파일의 확장자 — `remote::site_export`가 정한 형식의 겉이름이다 (plan D5)
pub const EXTENSION: &str = "moasites";

/// 저장할 파일을 고르게 한다. 취소하면 `None`.
///
/// `suggested`는 파일 이름 칸에 미리 채워 넣을 이름이다
pub fn pick_save(owner: HWND, suggested: &str) -> Option<PathBuf> {
    // 안전성: COM 개체를 만드는 표준 호출이다. UI 스레드는 `ui::app`이 STA로 초기화해 두었다
    let dialog: IFileSaveDialog =
        unsafe { CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER) }.ok()?;
    let name = HSTRING::from(suggested);
    // 안전성: 방금 만든 개체이며 문자열은 이 함수가 호출 동안 소유한다
    unsafe { dialog.SetFileName(&name) }.ok()?;
    show(&dialog, owner)
}

/// 열 파일을 고르게 한다. 취소하면 `None`
pub fn pick_open(owner: HWND) -> Option<PathBuf> {
    // 안전성: `pick_save`와 같다
    let dialog: IFileOpenDialog =
        unsafe { CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER) }.ok()?;
    show(&dialog, owner)
}

/// 필터를 걸고 대화를 띄운 뒤 고른 경로를 돌려준다.
///
/// 두 대화가 `IFileDialog`를 함께 상속하므로 여기서 한 번만 적는다 — 갈리는 것은 만드는 개체와
/// 저장 쪽의 기본 파일 이름뿐이다
fn show(dialog: &IFileDialog, owner: HWND) -> Option<PathBuf> {
    // 필터 문자열은 대화가 사는 동안 유효해야 한다 — 지역 변수로 붙잡아 둔다
    let label = HSTRING::from(crate::i18n::file_dialog_filter());
    let pattern = HSTRING::from(format!("*.{EXTENSION}"));
    let filters = [COMDLG_FILTERSPEC {
        pszName: PCWSTR(label.as_ptr()),
        pszSpec: PCWSTR(pattern.as_ptr()),
    }];
    let extension = HSTRING::from(EXTENSION);

    // 안전성: 위 문자열들은 이 함수가 소유하며 `Show`가 돌아올 때까지 살아 있다
    unsafe {
        dialog.SetFileTypes(&filters).ok()?;
        // 사용자가 확장자를 지우고 저장해도 이것이 붙는다
        dialog.SetDefaultExtension(&extension).ok()?;
        // 취소·`Esc`·창 닫기는 오류가 아니라 「고르지 않았다」다 — 사유를 가리지 않고 `None`으로 접는다
        dialog.Show(Some(owner)).ok()?;
        let item = dialog.GetResult().ok()?;
        let raw = item.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
        let path = raw.to_string().ok().map(PathBuf::from);
        // 셸이 잡아 준 문자열은 우리가 돌려줘야 한다
        CoTaskMemFree(Some(raw.as_ptr().cast()));
        path
    }
}
