//! 화면 문구 카탈로그와 현재 언어 (FR-53).
//!
//! **이 모듈은 화면 계층을 참조하지 않는다** — `ui`·`remote`·`fs` 어디서나 부르므로
//! 최상위에 두고 의존을 한 방향으로 유지한다(AGENTS 계층 규약). 참조하는 것은
//! `app::settings::LanguageSetting` 하나뿐이며, 그것은 저장 값을 담는 순수 데이터라
//! 이쪽을 되참조하지 않는다
//!
//! 문구는 [`strings!`] 매크로에 한 줄씩 적고, 매크로가 그것을 함수로 펼친다.
//! 키가 곧 함수 이름이라 **오타는 컴파일 오류**이고, 한·영 둘 중 하나를 빠뜨려도
//! 마찬가지다 — 화면을 띄워 봐야 아는 실수를 컴파일러가 먼저 잡는다.
//!
//! 문구에 값이 끼어드는 것(`"폴더 3 파일 12"`)은 매크로가 아니라 **손수 쓴 함수**로 둘 것이다
//! — 조사·어순·복수형이 언어마다 달라 자리표시자로는 담기지 않는다. 그 함수들은 문구를
//! 옮기는 task가 들어올 때 이 파일에 함께 들어온다(아직 없다).
use std::sync::atomic::{AtomicU8, Ordering};

use crate::app::settings::LanguageSetting;

/// 화면에 실제로 쓰이는 언어 — `시스템 기본`은 시작할 때 이 둘 중 하나로 풀린다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Korean,
    English,
}

/// 현재 언어. **잠금 없이 스레드 경계를 넘는다** — 화면은 UI 스레드가 그리지만
/// 원격 워커도 오류 문구를 만든다(`remote::connection`)
static CURRENT: AtomicU8 = AtomicU8::new(KOREAN);

const KOREAN: u8 = 0;
const ENGLISH: u8 = 1;

/// 지금 쓰는 언어. 매 프레임 수백 번 불리므로 원자적 읽기 하나로 끝낸다
pub fn current() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        ENGLISH => Language::English,
        // 알 수 없는 값은 한국어로 — 이 앱의 화면이 원래 한국어다
        _ => Language::Korean,
    }
}

/// 설정 값을 받아 현재 언어를 정한다. `시스템 기본`은 **여기서 한 번** 풀어 둔다.
///
/// 문구를 만들 때마다 Win32에 묻지 않는 이유: 시스템 UI 언어는 앱이 도는 동안 바뀌지
/// 않는다(바꾸려면 로그아웃해야 한다) — 매 프레임 수백 번 부를 값이 아니다 (D5)
pub fn set_language(setting: LanguageSetting) {
    let language = match setting {
        LanguageSetting::Korean => Language::Korean,
        LanguageSetting::English => Language::English,
        LanguageSetting::System => system_language(),
    };
    let value = match language {
        Language::Korean => KOREAN,
        Language::English => ENGLISH,
    };
    CURRENT.store(value, Ordering::Relaxed);
}

/// Windows 사용자 UI 언어가 한국어인가.
///
/// 하위 10비트가 언어를 가리키고 상위 6비트가 지역이다 — `ko-KR`뿐 아니라 다른 한국어
/// 변종도 함께 잡으려면 하위 10비트만 본다
pub fn system_language() -> Language {
    use windows::Win32::Globalization::GetUserDefaultUILanguage;
    /// `LANG_KOREAN` — winnt.h의 값
    const LANG_KOREAN: u16 = 0x12;
    // 안전성: 인자도 반환 자원도 없는 순수 조회다
    let id = unsafe { GetUserDefaultUILanguage() };
    // 실패(0)면 한국어로 — 이 앱의 기존 화면이 한국어이므로 그것이 안전한 쪽이다
    if id == 0 || id & 0x3FF == LANG_KOREAN {
        Language::Korean
    } else {
        Language::English
    }
}

/// 정적 문구를 함수로 펼친다.
///
/// 한 줄이 `키 => "한국어" / "English"` 형태이고, 매크로는 그 줄마다
/// `pub fn 키() -> &'static str`을 만든다. **하는 일은 이 펼침 하나**이며 조건·분기·
/// 타입 조작이 없다 — 반복을 줄이는 것이지 간접화를 더하는 것이 아니다.
macro_rules! strings {
    ($($(#[$doc:meta])* $name:ident => $ko:literal / $en:literal;)*) => {
        $(
            $(#[$doc])*
            pub fn $name() -> &'static str {
                match current() {
                    Language::Korean => $ko,
                    Language::English => $en,
                }
            }
        )*
    };
}

strings! {
    /// 설정 대화 제목
    settings_title => "설정" / "Settings";
    /// 설정 대화의 그룹 제목 — 모양
    settings_group_appearance => "모양" / "Appearance";
    settings_group_startup => "시작" / "Startup";
    settings_group_exit => "종료" / "Exit";
    settings_group_files => "파일 보기" / "Files";
    settings_group_language => "언어" / "Language";
    settings_font => "글꼴" / "Font";
    /// 글꼴을 고르지 않은 상태 — 목록 맨 앞 항목이기도 하다
    settings_font_default => "기본값 (맑은 고딕)" / "Default (Malgun Gothic)";
    /// 목록을 읽는 동안 보이는 안내 — 워커가 1.5초쯤 걸린다
    settings_font_scanning => "글꼴 목록을 읽는 중…" / "Reading font list…";
    settings_auto_start => "윈도우 시작 시 실행" / "Run at Windows startup";
    /// 레지스트리 쓰기가 막힌 환경에서 보이는 안내 — 조용히 되돌리면 왜 안 켜지는지 알 수 없다
    settings_auto_start_failed
        => "시작 프로그램 설정을 바꾸지 못했습니다"
        / "Could not change the startup setting";
    settings_tray_on_close => "닫으면 트레이로 보내기" / "Send to tray on close";
    settings_show_extensions => "파일 확장명" / "File name extensions";
    settings_show_hidden => "숨김 항목" / "Hidden items";
    /// 언어 선택지 — `English`는 두 언어에서 같다
    settings_language_system => "시스템 기본" / "System default";
    settings_language_korean => "한국어" / "Korean";
    settings_language_english => "English" / "English";
    close => "닫기" / "Close";
    delete => "삭제" / "Delete";
    rename => "이름 바꾸기" / "Rename";
    connect => "연결" / "Connect";

    // ── 타이틀바 (FR-22) ──
    titlebar_show_workspaces => "워크스페이스 목록 보이기" / "Show workspace list";
    titlebar_hide_workspaces => "워크스페이스 목록 숨기기" / "Hide workspace list";
    titlebar_restore => "이전 크기로" / "Restore";
    titlebar_maximize => "최대화" / "Maximize";
    titlebar_minimize => "최소화" / "Minimize";
    /// 설정 메뉴의 나머지 넷 — 아직 비활성이다
    titlebar_updates => "업데이트" / "Updates";
    titlebar_release_notes => "릴리즈 노트" / "Release notes";
    titlebar_licenses => "오픈소스 라이선스" / "Open source licenses";
    titlebar_about => "정보" / "About";

    // ── 패널 메뉴 (FR-23) ──
    /// 열 메뉴 캡션 (인벤토리 #22)
    menu_columns => "표시할 컬럼" / "Columns";
    menu_view => "보기" / "View";
    menu_refresh => "새로 고침" / "Refresh";
    menu_new_file => "새 파일" / "New file";
    menu_new_folder => "새 폴더" / "New folder";
    menu_split_right => "오른쪽 분할" / "Split right";
    menu_split_left => "왼쪽 분할" / "Split left";
    menu_split_up => "위쪽 분할" / "Split up";
    menu_split_down => "아래쪽 분할" / "Split down";

    // ── 보기 모드 8종 (FR-23) ──
    view_extra_large_icons => "아주 큰 아이콘" / "Extra large icons";
    view_large_icons => "큰 아이콘" / "Large icons";
    view_medium_icons => "보통 아이콘" / "Medium icons";
    view_small_icons => "작은 아이콘" / "Small icons";
    view_list => "목록" / "List";
    view_details => "자세히" / "Details";
    view_tiles => "타일" / "Tiles";
    view_content => "내용" / "Content";

    // ── 워크스페이스 사이드바 (FR-20) ──
    sidebar_workspaces => "워크스페이스" / "Workspaces";
    sidebar_saved_sites => "등록된 사이트" / "Saved sites";
    sidebar_add_site => "새 사이트 추가…" / "Add site…";
    sidebar_new_workspace => "새 워크스페이스" / "New workspace";
    sidebar_refresh_sites => "사이트 목록 새로 고침" / "Refresh site list";

    // ── 탭 스트립·주소창 ──
    tabs_new => "새 탭" / "New tab";
    tabs_close => "탭 닫기" / "Close tab";
    tabs_menu => "메뉴" / "Menu";
    address_back => "뒤로" / "Back";
    address_forward => "앞으로" / "Forward";
    address_up => "상위 폴더" / "Up";

    // ── 사이트 드롭다운 (FR-27) ──
    /// 사이트 드롭다운 캡션 (인벤토리 #92)
    site_dropdown_open => "연결 사이트를 새 탭으로" / "Open site in a new tab";
    site_dropdown_other => "다른 사이트로 새 탭 열기" / "Open another site in a new tab";

    // ── 폴더 트리 (FR-9) ──
    /// 아직 읽는 중인 노드의 자리 표시 — 로컬·원격 트리가 같은 문구를 쓴다
    tree_loading => "읽는 중…" / "Loading…";

    // ── 패널·목록·상태 줄 ──
    panel_folder_tree => "폴더 트리" / "Folder tree";
    panel_remote_tree => "원격 트리" / "Remote tree";
    /// 새로 만들 대상 — 실패 문구에 끼워 넣는다
    panel_kind_folder => "폴더" / "folder";
    panel_kind_file => "파일" / "file";
    column_name => "이름" / "Name";
    column_size => "크기" / "Size";
    column_type => "종류" / "Type";
    column_modified => "수정한 날짜" / "Date modified";
    column_permissions => "권한" / "Permissions";
    column_owner => "소유자" / "Owner";
    /// 상태 표시줄의 큐 토글 (인벤토리 #53)
    status_queue => "전송 큐" / "Transfer queue";
    /// 실패 알약 (인벤토리 #57)
    status_failed => "실패" / "Failed";
    /// 폴더를 펼치는 중임을 알리는 문구
    status_expanding => "펼치는 중…" / "Expanding…";
    /// 새로 만드는 폴더·파일의 기본 이름 — 화면 언어를 따라 실제 이름이 정해진다.
    /// 파일 쪽은 Windows 탐색기의 `새로 만들기 > 텍스트 문서`와 같은 이름이다 (사용자 확정)
    create_folder_base => "새 폴더" / "New folder";
    create_file_base => "새 텍스트 문서" / "New Text Document";
    create_no_name => "쓸 수 있는 이름을 찾지 못했습니다" / "Could not find a usable name";
    /// 자동 워크스페이스 이름의 앞부분 — `워크스페이스 3`처럼 뒤에 번호가 붙는다 (D7)
    workspace_auto_prefix => "워크스페이스 " / "Workspace ";
    /// 사이트를 찾을 수 없을 때 탭에 보일 이름 (사이트가 지워진 뒤 남은 탭)
    tabs_missing_site => "알 수 없는 사이트" / "Unknown site";

    // ── 사이트 관리자 (FR-27) ──
    /// 접근 키 알파벳은 **영어에서도 그대로 둔다** — 키 배정이 바뀌면 익힌 조작이 깨진다
    site_title => "사이트 관리자" / "Site Manager";
    site_list_label => "항목 선택(S):" / "Select entry(S):";
    site_rename => "이름 바꾸기(R)" / "Rename(R)";
    site_delete => "삭제(D)" / "Delete(D)";
    site_duplicate => "복제(I)" / "Duplicate(I)";
    site_tab_general => "일반" / "General";
    site_tab_transfer => "전송 설정" / "Transfer";
    site_tab_charset => "문자셋" / "Charset";
    site_label_protocol => "프로토콜(T):" / "Protocol(T):";
    site_label_host => "호스트(H):" / "Host(H):";
    site_label_port => "포트(P):" / "Port(P):";
    site_label_encryption => "암호화(E):" / "Encryption(E):";
    site_label_logon => "로그온 유형(L):" / "Logon type(L):";
    site_label_user => "사용자(U):" / "User(U):";
    site_label_password => "비밀번호(W):" / "Password(W):";
    site_connect => "연결(C)" / "Connect(C)";
    site_ok => "확인(O)" / "OK(O)";
    site_label_transfer_mode => "전송 모드(T):" / "Transfer mode(T):";
    site_label_limit => "동시 연결 수 제한(L)" / "Limit simultaneous connections(L)";
    site_label_limit_value => "최대 동시 연결 수(M):" / "Maximum connections(M):";
    site_charset_heading
        => "서버에서 파일명에 사용하는 문자셋"
        / "Character set the server uses for file names";
    site_charset_label => "인코딩:" / "Encoding:";
    site_label_encoding => "인코딩(E):" / "Encoding(E):";
    /// 알아듣지 못하는 인코딩 이름을 적었을 때 — 조용히 UTF-8로 처리하면 파일명이 깨진 채로 굳는다
    site_charset_unknown_hint
        => "이 이름은 알지 못해 UTF-8로 처리합니다."
        / "This name is not recognized, so UTF-8 is used.";
    site_charset_warning
        => "문자셋을 잘못 지정하면 파일명이 올바르게 보여지지 않을 수 있습니다."
        / "A wrong character set can make file names display incorrectly.";
    /// 이미 연결된 서버의 전송 모드를 바꿨을 때 — 지금 연결에 바로 듣지 않는다는 것을
    /// 알리지 않으면 사용자는 같은 실패를 다시 겪는다
    site_transfer_mode_notice
        => "이미 연결된 서버입니다. 바꾼 전송 모드는 다음 연결부터 적용됩니다."
        / "Already connected. The new transfer mode applies from the next connection.";
    /// 호스트를 비운 채 등록하려 할 때 — 무엇을 해야 하는지까지 알린다
    site_error_no_host
        => "호스트 주소를 입력해야 등록할 수 있습니다."
        / "Enter a host address to save this site.";
    /// 비밀번호 봉인이 실패했을 때 — 평문으로 대신 담지 않는다 (FR-28)
    site_error_password
        => "비밀번호를 저장하지 못했습니다. 연결할 때 다시 입력해 주세요."
        / "Could not save the password. Enter it again when connecting.";
    site_protocol_ftp => "FTP - 파일 전송 프로토콜" / "FTP - File Transfer Protocol";
    site_protocol_ftps
        => "FTPS - TLS로 보호되는 파일 전송 프로토콜"
        / "FTPS - File Transfer Protocol over TLS";
    site_protocol_sftp => "SFTP - SSH 파일 전송 프로토콜" / "SFTP - SSH File Transfer Protocol";
    site_encryption_plain => "일반 FTP 사용 (안전하지 않음)" / "Use plain FTP (insecure)";
    site_encryption_explicit_optional
        => "TLS를 통한 명시적 FTP가 가능한 경우 사용"
        / "Use explicit FTP over TLS if available";
    site_encryption_explicit => "TLS를 통한 명시적 FTP 필요" / "Require explicit FTP over TLS";
    site_encryption_implicit => "TLS를 통한 묵시적 FTP 필요" / "Require implicit FTP over TLS";
    site_logon_normal => "일반" / "Normal";
    site_logon_anonymous => "익명" / "Anonymous";
    site_mode_default => "기본(E)" / "Default(E)";
    site_mode_active => "능동형(A)" / "Active(A)";
    site_mode_passive => "수동형(P)" / "Passive(P)";
    site_charset_custom => "문자셋 직접 설정(C)" / "Set character set manually(C)";

    // ── 원격 메뉴 (FR-31) ──
    remote_download => "받기" / "Download";
    remote_upload => "올리기" / "Upload";
    remote_rename => "이름 바꾸기…" / "Rename…";
    remote_new_folder => "새 폴더…" / "New folder…";
    remote_chmod => "권한 변경…" / "Change permissions…";
    remote_delete => "삭제…" / "Delete…";
    /// 이름에 쓸 수 없는 글자를 적었을 때
    remote_error_slash => "이름에 / 는 쓸 수 없습니다." / "A name cannot contain /.";
    remote_error_empty => "이름을 입력해 주세요." / "Enter a name.";
    remote_ok => "확인" / "OK";
    cancel => "취소" / "Cancel";
    remote_chmod_title => "권한 변경" / "Change permissions";
    remote_chmod_octal => "숫자(8진):" / "Octal:";
    remote_apply => "적용" / "Apply";
    remote_owner => "소유자" / "Owner";
    remote_group => "그룹" / "Group";
    remote_others => "기타" / "Others";
    remote_read => "읽기" / "Read";
    remote_write => "쓰기" / "Write";
    remote_execute => "실행" / "Execute";
    remote_delete_title => "원격 항목 삭제" / "Delete remote items";
    remote_delete_irreversible => "되돌릴 수 없습니다." / "This cannot be undone.";
    remote_delete_recursive => "폴더 안에 든 것까지 지웁니다" / "Also delete folder contents";

    // ── 원격 탭 상태 (FR-31) ──
    remote_hint_head => "주소창에 " / "Type ";
    remote_hint_tail => " 를 입력해 연결하세요" / " in the address bar to connect";
    /// 미연결 탭 안내 둘째 줄 (인벤토리 #15)
    remote_hint_drag
        => "사이드바의 사이트를 이 탭으로 끌어다 놓아도 됩니다"
        / "You can also drag a site from the sidebar onto this tab";
    /// 사이트를 아는 미연결 탭의 버튼 — 재시작 뒤 복원된 탭이 이것을 보인다
    remote_reconnect => "다시 연결" / "Reconnect";
    /// 실패 화면 제목 (인벤토리 #16)
    remote_fail_title => "연결하지 못했습니다" / "Could not connect";
    /// 실패 화면 버튼·링크 (인벤토리 #18~20)
    remote_fail_retry => "재시도" / "Retry";
    remote_fail_settings => "설정 열기" / "Open settings";
    remote_fail_view_log => "서버 로그 보기" / "View server log";
    /// 서버가 사유를 주지 않았을 때 보일 문구
    remote_fail_reason_fallback => "서버가 응답하지 않았습니다." / "The server did not respond.";
    /// 실패 사유 뒤에 늘 붙는 안내 (인벤토리 #17)
    remote_fail_reason_hint
        => "암호화 설정이 서버와 다를 수도 있습니다."
        / "The encryption setting may not match the server.";
    remote_connecting => "연결 중…" / "Connecting…";
    remote_not_connected => "연결 없음" / "Not connected";
    remote_hostkey_first => "이 서버를 처음 연결합니다" / "Connecting to this server for the first time";
    remote_hostkey_changed => "서버 지문이 전과 다릅니다" / "The server fingerprint has changed";
    remote_hostkey_accept => "수락하고 연결" / "Accept and connect";

    // ── 트레이 메뉴 (FR-50) ──
    /// 우클릭 메뉴 항목 — 요청 문구 그대로다
    tray_show => "실행" / "Open";
    tray_quit => "종료" / "Quit";

    /// 설정 대화의 언어 항목 라벨
    settings_language_label => "앱 언어" / "App language";
}

/// 값이 끼어드는 문구 — 조사·어순이 언어마다 달라 자리표시자로 담기지 않는다 (D2).
///
/// 정적 문구와 달리 매크로로 펼치지 않는다: 인자 개수·순서를 컴파일러가 검사해야 하고,
/// 언어마다 문장을 통째로 다시 쓸 수 있어야 한다
pub mod dynamic {
    use super::{Language, current};

    /// 폴더를 열지 못한 사유 — 경로 이름이 문장 안에 들어간다
    pub fn open_denied(name: &str) -> String {
        match current() {
            Language::Korean => format!("'{name}' 폴더를 열 권한이 없습니다"),
            Language::English => format!("You do not have permission to open '{name}'"),
        }
    }

    pub fn open_not_found(name: &str) -> String {
        match current() {
            Language::Korean => format!("'{name}' 폴더를 찾을 수 없습니다"),
            Language::English => format!("Could not find '{name}'"),
        }
    }

    pub fn open_failed(name: &str) -> String {
        match current() {
            Language::Korean => format!("'{name}' 폴더를 여는 중 문제가 발생했습니다"),
            Language::English => format!("Something went wrong while opening '{name}'"),
        }
    }

    /// 새로 만들기 실패 — 한국어는 조사가 붙고 영어는 관사가 붙는다 (D2)
    pub fn create_failed(kind: &str, error: &str) -> String {
        match current() {
            Language::Korean => format!("새 {kind}을(를) 만들지 못했습니다 — {error}"),
            Language::English => format!("Could not create the new {kind} — {error}"),
        }
    }

    /// 상태 줄의 폴더·파일 개수 — 어순이 언어마다 다르다
    pub fn item_counts(dirs: usize, files: usize) -> String {
        match current() {
            Language::Korean => format!("폴더 {dirs} 파일 {files}"),
            Language::English => format!("{dirs} folders, {files} files"),
        }
    }

    /// 전송 큐 요약 — `3건 대기 · 12.4 MB/s · 00:41 남음` / `3 pending · 12.4 MB/s · 00:41 left`.
    ///
    /// **개수와 남은 시간은 언어마다 붙는 자리가 다르다**(한국어는 뒤에 `남음`, 영어는 `left`)
    /// — 그 두 조각만 언어별로 만들고, 사이에 끼는 속도·구분점은 순서가 같아 이어 붙인다.
    ///
    /// 남은 시간을 모를 때 쓸 값(`unknown`)은 **호출부가 준다** — 그 값의 정본은 전송 큐 쪽에
    /// 있고, 여기서 그것을 참조하면 문구 카탈로그가 원격 계층을 거꾸로 참조하게 된다
    pub fn queue_summary(
        pending: usize,
        speed: Option<&str>,
        eta: Option<&str>,
        unknown: &str,
    ) -> String {
        let language = current();
        let mut out = match language {
            Language::Korean => format!("{pending}건 대기"),
            Language::English => format!("{pending} pending"),
        };
        if let Some(speed) = speed {
            out.push_str(" · ");
            out.push_str(speed);
        }
        out.push_str(" · ");
        match eta {
            Some(eta) => match language {
                Language::Korean => out.push_str(&format!("{eta} 남음")),
                Language::English => out.push_str(&format!("{eta} left")),
            },
            None => out.push_str(unknown),
        }
        out
    }

    /// 원격 삭제 확인 — 영어는 하나일 때 단수형이다
    pub fn remote_delete_count(count: usize) -> String {
        match current() {
            Language::Korean => format!("{count}개 항목을 서버에서 지웁니다."),
            Language::English if count == 1 => "1 item will be deleted from the server.".to_owned(),
            Language::English => format!("{count} items will be deleted from the server."),
        }
    }

    /// 사이트를 등록한 뒤 뜨는 알림 (FR-27)
    pub fn site_registered(host: &str) -> String {
        match current() {
            Language::Korean => format!("{host} 등록됨 · 더블클릭하여 연결"),
            Language::English => format!("{host} added · double-click to connect"),
        }
    }
}

/// 전역 언어를 건드리는 시험끼리 겹치지 않게 한다.
///
/// `cargo test`는 시험을 **여러 스레드에서 동시에** 돌린다 — 값을 끝에 되돌리는
/// 것만으로는 부족하다: 한 시험이 영어로 두고 단언하는 그 찰나에 다른 시험이
/// 값을 바꾸면 엉뚱한 언어를 읽는다. 그래서 **본문이 도는 동안 잠근다**
#[cfg(test)]
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// 언어를 바꾸는 시험이 드는 가드 — 들고 있는 동안 다른 시험이 끼어들지 못하고,
/// 떨어질 때 시험 전의 언어로 되돌아간다.
///
/// **문구를 쓰는 모든 모듈의 시험이 이것을 쓴다** — `ui`·`remote`의 시험도 화면 문구를
/// 단언하려면 언어를 정해야 하고, 그때 잠그지 않으면 서로를 흔든다.
///
/// 둘째 자리를 읽는 코드는 없다 — **들고 있는 것 자체가 하는 일**이다
#[cfg(test)]
pub struct LanguageGuard(
    Language,
    #[allow(dead_code)] std::sync::MutexGuard<'static, ()>,
);

#[cfg(test)]
impl LanguageGuard {
    /// 언어를 고정하고 가드를 든다. 가드가 떨어지면 원래 언어로 돌아간다
    pub fn lock(setting: LanguageSetting) -> LanguageGuard {
        // 앞선 시험이 단언에 실패해 잠금이 오염됐어도 이어서 돈다 —
        // 우리가 지키는 것은 `AtomicU8` 하나이고 그 값은 곧 덮인다
        let guard = TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = current();
        set_language(setting);
        LanguageGuard(before, guard)
    }
}

#[cfg(test)]
impl Drop for LanguageGuard {
    fn drop(&mut self) {
        let setting = match self.0 {
            Language::Korean => LanguageSetting::Korean,
            Language::English => LanguageSetting::English,
        };
        set_language(setting);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 언어에_따라_다른_문구를_돌려준다() {
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        assert_eq!(settings_title(), "설정");
        set_language(LanguageSetting::English);
        assert_eq!(settings_title(), "Settings");
    }

    #[test]
    fn 시스템_기본은_윈도우_ui_언어를_따른다() {
        let _guard = LanguageGuard::lock(LanguageSetting::System);
        assert_eq!(
            current(),
            system_language(),
            "시스템 기본인데 시스템 언어와 다르다"
        );
    }

    #[test]
    fn 값이_끼어드는_문구의_한국어는_이관_전_그대로다() {
        // 이관하며 조사·어순이 바뀌면 화면이 조용히 달라진다 — 문장 전체를 고정한다
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        assert_eq!(
            dynamic::open_denied("문서"),
            "'문서' 폴더를 열 권한이 없습니다"
        );
        assert_eq!(
            dynamic::open_not_found("문서"),
            "'문서' 폴더를 찾을 수 없습니다"
        );
        assert_eq!(
            dynamic::open_failed("문서"),
            "'문서' 폴더를 여는 중 문제가 발생했습니다"
        );
        assert_eq!(
            dynamic::create_failed("폴더", "접근 거부"),
            "새 폴더을(를) 만들지 못했습니다 — 접근 거부"
        );
        assert_eq!(dynamic::item_counts(3, 12), "폴더 3 파일 12");
        assert_eq!(
            dynamic::site_registered("example.test"),
            "example.test 등록됨 · 더블클릭하여 연결"
        );
    }

    #[test]
    fn 큐_요약은_언어마다_조각_순서가_다르다() {
        // 한국어는 `남음`이 뒤에, 영어는 `left`가 뒤에 온다 — 조각을 이어 붙이면 어순이 깨진다
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        assert_eq!(
            dynamic::queue_summary(3, Some("12.4 MB/s"), Some("00:41"), "—"),
            "3건 대기 · 12.4 MB/s · 00:41 남음"
        );
        // 남은 시간을 모르면 호출부가 준 값이 그 자리에 들어간다
        assert_eq!(dynamic::queue_summary(3, None, None, "—"), "3건 대기 · —");
        set_language(LanguageSetting::English);
        assert_eq!(
            dynamic::queue_summary(3, Some("12.4 MB/s"), Some("00:41"), "—"),
            "3 pending · 12.4 MB/s · 00:41 left"
        );
    }

    #[test]
    fn 알_수_없는_값은_한국어로_읽는다() {
        // 이 앱의 화면이 원래 한국어다 — 모르는 값에 영어를 주면 갑자기 화면이 바뀐다
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        CURRENT.store(99, Ordering::Relaxed);
        assert_eq!(current(), Language::Korean);
    }
}
