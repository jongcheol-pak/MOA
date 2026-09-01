//! 사이트 관리자 대화 — 목록과 `일반` 탭 (FR-27).
//!
//! 원본 `FileExplorer-FTP.dc.html:384-499`. 대화는 1080×680 고정이고 헤더(40px)·본문(574px)·
//! 오류 줄(22px)·바닥 버튼 줄(44px, 공통 셸이 그린다) 네 층으로 나뉜다. 본문은 좌측
//! 400px(사이트 목록 + 버튼 두 줄 — 윗줄 `이름 바꾸기`·`삭제`·`복제`, 아랫줄
//! `새 사이트`·`내보내기`·`가져오기`)과 우측 가변(탭 + 폼)이다.
//!
//! **조작은 값으로 돌려주고 연결·토스트는 여기서 하지 않는다** — 기존 화면 규약과 같다.
//! 다만 사이트 목록 자체의 변경(이름 바꾸기·삭제·복제·추가·차례 바꾸기·등록)은
//! `SiteStore`를 직접 고친다:
//! 평문 비밀번호를 봉인해 담을 수 있는 곳이 `SiteStore::set_password`뿐이라(FR-28),
//! 초안을 값으로 넘기면 봉인 경로가 화면 쪽에 한 벌 더 생긴다.
use crate::remote::charset;
use crate::remote::sites::SiteStore;
use crate::remote::types::{
    CONNECTION_LIMIT_RANGE, Charset, Encryption, LogonType, Protocol, SiteId, TransferMode,
};
use crate::ui::dialog;
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

/// 내보내기·가져오기 흐름 (FR-59) — 이 대화의 자식 모듈.
///
/// **부모의 자식으로 둔 이유**: 그 흐름은 `SiteManager`의 private 필드
/// (`exchange`·`pending_file`·`notice`·`error`)를 직접 만진다. 형제 모듈로 두면 그 필드를
/// `pub(crate)`로 넓혀야 하지만, 자식이면 가시성을 그대로 두고 나눌 수 있다
/// (`ui::app`과 `ui::app::transfer_conflict`가 같은 배치다).
///
/// 나눈 까닭은 변경 이유가 둘이기 때문이다 — 사이트 목록을 고치는 일과 파일로 주고받는
/// 일은 서로 다른 이유로 바뀐다 (AGENTS 「파일」 규약의 네 질문 중 ①·③).
mod exchange;

/// 앱이 이 이름으로 파일 요청을 받는다 — 실체는 자식 모듈에 있다 (FR-59)
pub use exchange::FileRequest;
use exchange::{Exchange, ExchangeAction};

// ── 대화 치수 (원본 `FileExplorer-FTP.dc.html`, plan 시각 속성 표) ──
/// 대화 크기 — 고정이다(`:385`)
const DIALOG_WIDTH: f32 = 1080.0;
/// `찾아보기` 버튼의 좌우 여백 — 폼 안에 들어가는 버튼이라 대화 바닥의 것보다 좁다
const BROWSE_PAD_X: f32 = 12.0;
const DIALOG_HEIGHT: f32 = 680.0;
/// 삭제 확인 대화의 본문 폭 — 워크스페이스 삭제 확인과 같은 값이다.
/// 같은 성격의 물음이 자리마다 다른 크기로 뜨면 판이 흔들려 보인다
const DELETE_CONFIRM_WIDTH: f32 = 360.0;
/// 헤더 — 높이 40px · `padding 0 8px 0 16px` (`:386`)
const HEADER_HEIGHT: f32 = 40.0;
const HEADER_PAD_LEFT: f32 = 16.0;
const HEADER_PAD_RIGHT: f32 = 8.0;
const TITLE_FONT_PX: f32 = 16.0;
/// 닫기 버튼 40×32 · 글리프 15px · hover 빨강 (`:388`)
const CLOSE_WIDTH: f32 = 40.0;
const CLOSE_HEIGHT: f32 = 32.0;
const CLOSE_FONT_PX: f32 = 15.0;

/// 본문 — `gap 22px` · `padding 6px 18px 0` (`:391`)
const BODY_PAD_X: f32 = 18.0;
const BODY_PAD_TOP: f32 = 6.0;
const BODY_GAP: f32 = 22.0;
/// 좌측 열 폭 (`:392`)
const LEFT_WIDTH: f32 = 400.0;
/// 좌측 열 요소 사이 간격 (`:392` `gap:8px`)
const LEFT_GAP: f32 = 8.0;
/// `항목 선택(S):` 한 줄 높이 — 13px 글자 한 줄
const LIST_LABEL_HEIGHT: f32 = 18.0;

/// 목록 웰 — `padding 8px 6px` (`:394`)
const LIST_PAD_X: f32 = 6.0;
const LIST_PAD_Y: f32 = 8.0;
/// 목록 행 — 24px · `padding-left 8px` · gap 6px (`:396`)
const LIST_ROW_HEIGHT: f32 = 24.0;
const LIST_ROW_PAD_LEFT: f32 = 8.0;
const LIST_ROW_GAP: f32 = 6.0;
/// 행 아이콘 자리 (`:397`)
const LIST_ICON: f32 = 16.0;
/// 이름 좌우 여백 — 선택 강조가 글자보다 조금 넓게 칠해진다 (`:403`)
const LIST_NAME_PAD_X: f32 = 5.0;
/// 선택된 행의 이름 강조 (`:689-690`)
const SELECTED_BG: egui::Color32 = egui::Color32::from_rgb(0x2A, 0x5F, 0xA8);
const SELECTED_FG: egui::Color32 = egui::Color32::WHITE;

/// 좌측 버튼 3열 — `grid 1fr 1fr 1fr` gap 8px · 28px (`:407-409`).
///
/// **원본의 좌우 여백 30px(`padding 2px 30px 6px`)은 걷어냈다** — 그만큼 버튼 줄이 위
/// 목록 웰보다 좁아, 웰의 테두리와 버튼 줄의 좌우 선이 어긋나 보였다(2026-09-01 사용자
/// 요청). 지금은 두 줄 다 웰과 같은 `column.left()`~`column.right()`를 쓴다
const GRID_GAP: f32 = 8.0;
/// 버튼 줄이 둘이다 — 윗줄 `이름 바꾸기·삭제·복제`(원본), 아랫줄
/// `새 사이트`(FR-27)·`내보내기`·`가져오기`(FR-59). **두 줄 다 세 칸 균등**이고
/// 좌우 끝은 목록 웰에 맞춘다
const GRID_ROWS: f32 = 2.0;
const GRID_PAD_TOP: f32 = 2.0;
const GRID_PAD_BOTTOM: f32 = 6.0;
const GRID_BUTTON_HEIGHT: f32 = 28.0;

/// 우측 탭 — 28px · `padding 0 14px` (`:415-417`)
const TAB_HEIGHT: f32 = 28.0;
const TAB_PAD_X: f32 = 14.0;
/// 비활성 탭 배경 (`:845`)
const TAB_INACTIVE_BG: egui::Color32 = egui::Color32::from_rgb(0x19, 0x19, 0x19);

/// 폼 — `gap 11px`(행간) · `padding 16px 2px 0` (`:421`)
const FORM_PAD_TOP: f32 = 16.0;
const FORM_PAD_X: f32 = 2.0;
const FORM_ROW_GAP: f32 = 11.0;
/// `포트(P):` 라벨 앞 여백 — 행 기본 간격에 더해진다 (`:431` `margin-left:6px`)
const PORT_LABEL_MARGIN: f32 = 6.0;
/// 포트 필드 폭 (`:434`)
const PORT_WIDTH: f32 = 96.0;

/// 두 번째·세 번째 탭의 안쪽 여백과 세로 간격 (`:441`·`:470`)
const TAB_BODY_PAD: f32 = 2.0;
const TRANSFER_GAP: f32 = 14.0;
const CHARSET_GAP: f32 = 12.0;
/// 라디오 3개 사이 (`:443` `gap:28px`)
const RADIO_GAP: f32 = 28.0;
/// 라디오·체크 행의 들여쓰기 (`:443`·`:452` `padding-left:4px`)
const MARK_INDENT: f32 = 4.0;
/// 체크박스 행 위 여분 (`:452` `margin-top:6px`)
const CHECK_MARGIN_TOP: f32 = 6.0;
/// 스피너 행 들여쓰기 (`:456` `padding-left:34px`)
const SPINNER_INDENT: f32 = 34.0;
/// `인코딩(E):` 행 들여쓰기 (`:480` `padding-left:26px`)
const ENCODING_INDENT: f32 = 26.0;
/// `인코딩(E):` 필드 (`:482`)
const ENCODING_WIDTH: f32 = 210.0;
const ENCODING_HEIGHT: f32 = 26.0;
/// 각주 위 여분 (`:484` `margin-top:14px`)
const FOOTNOTE_MARGIN_TOP: f32 = 14.0;
/// 한 줄 텍스트가 차지하는 높이 — 13px 글자 기준
const TEXT_ROW_HEIGHT: f32 = 18.0;

/// 바닥 버튼 줄 **바로 위**의 오류 문구 줄 높이 — 13px 글자 한 줄이 드는 자리다.
///
/// 버튼이 대화 전폭을 나눠 갖게 되면서(공통 셸) 종전처럼 오류를 버튼 왼쪽에 둘 자리가
/// 없어졌다. 버튼 줄 높이는 `dialog::FOOTER_HEIGHT`(44px)가 정본이고, 이 줄과 더한
/// 66px이 종전 바닥(58px) 자리를 대신한다 — 그만큼 본문이 8px 줄어든다
const ERROR_ROW_HEIGHT: f32 = 22.0;

/// 바닥 버튼의 자리 번호 — 배열에 넣은 순서이며 눌린 칸을 이 이름으로 가려낸다
const CONNECT_BUTTON: usize = 0;
const CONFIRM_BUTTON: usize = 1;

// ── 선택지 목록 — 문구는 카탈로그가 정하고 여기서는 값과 짝짓기만 한다 ──
/// 프로토콜 선택지 — 첫 항목의 문구는 원본 그대로다 (인벤토리 #69, `:1011`).
/// 나머지 둘은 원본에 없어 같은 말투로 새로 적었다
fn protocol_options() -> [(Protocol, &'static str); 3] {
    [
        (Protocol::Ftp, crate::i18n::site_protocol_ftp()),
        (Protocol::Ftps, crate::i18n::site_protocol_ftps()),
        (Protocol::Sftp, crate::i18n::site_protocol_sftp()),
    ]
}

/// 암호화 선택지 — 기본값 문구는 원본 그대로다 (인벤토리 #72, `:1013`)
fn encryption_options() -> [(Encryption, &'static str); 4] {
    [
        (Encryption::Plain, crate::i18n::site_encryption_plain()),
        (
            Encryption::ExplicitIfAvailable,
            crate::i18n::site_encryption_explicit_optional(),
        ),
        (
            Encryption::ExplicitRequired,
            crate::i18n::site_encryption_explicit(),
        ),
        (
            Encryption::Implicit,
            crate::i18n::site_encryption_implicit(),
        ),
    ]
}

/// 로그온 유형 선택지 (인벤토리 #73, `:1014`).
///
/// **`키 파일`은 SFTP에서만 선다**(D7) — FTP 계열에 없는 개념이라, 세워 두면
/// 고를 수 있는데 반드시 실패하는 값이 된다
fn logon_options(protocol: Protocol) -> Vec<(LogonType, &'static str)> {
    let mut options = vec![
        (LogonType::Normal, crate::i18n::site_logon_normal()),
        (LogonType::Anonymous, crate::i18n::site_logon_anonymous()),
    ];
    if protocol.is_ssh() {
        options.push((LogonType::KeyFile, crate::i18n::site_logon_key_file()));
    }
    options
}

/// 전송 모드 라디오 3종 (인벤토리 #77~79, `:852`) — 라벨과 그 설명을 함께 든다
fn transfer_options() -> [(TransferMode, &'static str, &'static str); 3] {
    [
        (
            TransferMode::Default,
            crate::i18n::site_mode_default(),
            crate::i18n::site_hint_mode_default(),
        ),
        (
            TransferMode::Active,
            crate::i18n::site_mode_active(),
            crate::i18n::site_hint_mode_active(),
        ),
        (
            TransferMode::Passive,
            crate::i18n::site_mode_passive(),
            crate::i18n::site_hint_mode_passive(),
        ),
    ]
}

/// 문자셋 라디오 2종 (인벤토리 #84·#85, `:867`)
fn charset_options() -> [&'static str; 2] {
    ["UTF-8(U)", crate::i18n::site_charset_custom()]
}

/// 포트가 가질 수 있는 범위 — 0은 실제 포트가 아니다
const PORT_RANGE: std::ops::RangeInclusive<u32> = 1..=65535;

/// 대화의 우측 탭 (인벤토리 #66~68). 바깥에서 탭을 지정해 여는 진입점이 없어 모듈 안에 둔다
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ManagerTab {
    #[default]
    General,
    Transfer,
    Charset,
}

/// 사용자가 대화에서 고른 결과
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteManagerOutcome {
    /// 아직 고르지 않았다(대화가 떠 있거나 닫혀 있다)
    None,
    /// 그냥 닫았다 — 초안은 버려진다
    Close,
    /// `확인(O)` — 등록만 한다. 호출부가 토스트를 띄운다 (인벤토리 #89·#91)
    Register(SiteId),
    /// `연결(C)` — 등록하고 곧바로 연결한다 (인벤토리 #88)
    RegisterAndConnect(SiteId),
}

/// 본문을 그리는 동안 모인 조작 — 목록을 빌려 읽는 중이라 여기 담았다가 그린 뒤에 적용한다
#[derive(Debug, Clone, Copy, Default)]
struct BodyOutcome {
    /// 목록에서 새로 고른 사이트
    picked: Option<SiteId>,
    action: Option<ListAction>,
    /// 내보내기·가져오기 버튼 (FR-59)
    exchange: Option<ExchangeAction>,
    /// 이름 바꾸기가 끝났는가(Enter 또는 포커스 잃음)
    rename_done: bool,
}

/// 목록을 끌어 차례를 바꾸는 중인 상태 (FR-27).
///
/// 워크스페이스 카드(`ui::sidebar`)·즐겨찾기(`ui::tree`)와 같은 벌이다 — 임계를 넘어야
/// 재정렬로 보고, 그 전에는 클릭이라 선택이 그대로 일어난다
#[derive(Debug)]
struct SiteDrag {
    /// 끌기 시작한 줄의 자리 — `store.sites()`의 인덱스와 같다
    from: usize,
    start: egui::Pos2,
    /// 임계를 넘어 실제 재정렬로 전환됐는가
    active: bool,
}

/// 아랫줄 한 줄에서 나온 조작 — 성격이 다른 둘이 같은 줄에 선다.
///
/// `새 사이트`는 목록을 고치고 `내보내기`·`가져오기`는 파일을 주고받는다. 한 줄이 둘을
/// 함께 내므로 값 하나에 담아 돌려준다(`BodyOutcome`이 이미 같은 형태다)
#[derive(Debug, Clone, Copy, Default)]
struct BottomOutcome {
    list: Option<ListAction>,
    exchange: Option<ExchangeAction>,
}

/// 좌측 버튼들이 목록에 가하는 변경
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListAction {
    StartRename,
    Delete,
    Duplicate,
    /// `새 사이트` — 기본 이름의 빈 사이트를 목록에 곧바로 더하고 그것을 고른다 (FR-27).
    /// **고른 사이트가 없어도 되는 조작**이라 `apply_list_action`의 선택 가드보다
    /// 앞에서 갈린다
    New,
    /// 끌어 놓아 차례를 바꾼다 — `from` 줄을 목록의 `to` 자리로 옮긴다 (FR-27).
    /// 끄는 줄과 고른 줄이 다를 수 있어 이것도 선택 가드보다 앞에서 갈린다
    Reorder(usize, usize),
}

/// 편집 중인 사이트 설정 한 벌.
///
/// **이름은 여기 없다** — 이름은 목록과 `이름 바꾸기(R)`가 정하고, 새로 등록하는 사이트는
/// 호스트를 이름으로 쓴다(원본의 토스트도 호스트로 사이트를 가리킨다 — 인벤토리 #91).
#[derive(Debug, Clone, PartialEq)]
struct Draft {
    protocol: Protocol,
    host: String,
    /// 사용자가 적은 그대로 둔다 — 숫자로 바꾸는 것은 등록할 때 한 번만 (`parse_port`)
    port: String,
    /// 포트를 사용자가 직접 손댔는가 — 손댄 뒤에는 프로토콜을 바꿔도 값을 지킨다 (Acceptance ③)
    port_edited: bool,
    encryption: Encryption,
    logon: LogonType,
    user: String,
    /// 평문 비밀번호 — 화면에서는 `●`로 가려지고, 등록할 때 `SiteStore::set_password`가 봉인한다.
    /// 대화가 닫히면 초안과 함께 버려진다 (FR-28)
    password: String,
    /// 개인 키 파일 경로 (FR-66) — 다른 필드와 같이 문자열로 쥐고 등록할 때 경로로 바꾼다.
    /// 비어 있으면 「고르지 않음」이다
    pub(super) key_path: String,
    /// 평문 키 암호 — 비밀번호와 같이 가려 보이고 등록할 때 봉인된다 (FR-66·FR-28)
    key_passphrase: String,
    transfer_mode: TransferMode,
    /// `동시 연결 수 제한(L)` 체크 (인벤토리 #80)
    limit_on: bool,
    /// 스피너 값 — 체크를 껐다 켜도 적어 둔 값이 남게 기록과 따로 든다 (인벤토리 #81)
    limit: u8,
    /// `문자셋 직접 설정(C)`을 골랐는가 (인벤토리 #85)
    charset_custom: bool,
    /// `인코딩(E):`에 적은 이름 (인벤토리 #86)
    encoding: String,
}

impl Default for Draft {
    fn default() -> Draft {
        let protocol = Protocol::Ftp;
        Draft {
            protocol,
            host: String::new(),
            port: protocol.default_port().to_string(),
            port_edited: false,
            encryption: Encryption::default(),
            logon: LogonType::default(),
            user: String::new(),
            password: String::new(),
            key_path: String::new(),
            key_passphrase: String::new(),
            transfer_mode: TransferMode::default(),
            limit_on: false,
            limit: *CONNECTION_LIMIT_RANGE.start(),
            charset_custom: false,
            encoding: String::new(),
        }
    }
}

impl Draft {
    /// 등록된 사이트를 편집 상태로 불러온다.
    ///
    /// 비밀번호는 **풀어서** 담는다 — 저장된 것이 있는데 빈칸으로 보이면 사용자는 저장된 적이
    /// 없다고 읽고, 그대로 등록하면 있던 비밀번호가 지워진다. 풀지 못하면(다른 PC에서 가져온
    /// 설정) 빈칸이며 다시 입력받는다
    fn load(store: &SiteStore, id: SiteId) -> Option<Draft> {
        let record = store.get(id)?;
        Some(Draft {
            protocol: record.protocol,
            host: record.host.clone(),
            port: record.port.to_string(),
            // 기본 포트와 다르면 사용자가 정한 값이다 — 프로토콜을 바꿔도 지켜야 한다
            port_edited: record.port != record.protocol.default_port(),
            encryption: record.encryption,
            logon: record.logon,
            user: record.user.clone(),
            password: store.password(id).unwrap_or_default(),
            key_path: record
                .key_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .unwrap_or_default(),
            // **키 암호도 풀어서 담는다** — 비어 보이면 사용자는 저장된 적이 없다고 읽고,
            // 그대로 등록하면 있던 암호가 지워진다(비밀번호와 같은 이유)
            key_passphrase: store.key_passphrase(id).unwrap_or_default(),
            transfer_mode: record.transfer_mode,
            limit_on: record.connection_limit.is_some(),
            // 제한이 꺼져 있으면 스피너는 최솟값에서 시작한다
            limit: record
                .connection_limit
                .unwrap_or(*CONNECTION_LIMIT_RANGE.start()),
            charset_custom: matches!(record.charset, Charset::Named(_)),
            encoding: match &record.charset {
                Charset::Utf8 => String::new(),
                Charset::Named(name) => name.clone(),
            },
        })
    }

    /// 프로토콜을 바꾼다 — 포트를 손대지 않았으면 새 기본값이 따라온다 (Acceptance ③)
    fn set_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
        if !self.port_edited {
            self.port = protocol.default_port().to_string();
        }
        // 키 파일은 SFTP에만 있다 (D7) — FTP로 옮기면 고를 수 없는 값이 남으므로 되돌린다
        if !protocol.is_ssh() && self.logon == LogonType::KeyFile {
            self.logon = LogonType::Normal;
        }
    }

    /// 암호화 설정이 뜻을 갖는가 — SSH는 전송 계층이 이미 암호화돼 고를 것이 없다 (인벤토리 #72)
    fn encryption_enabled(&self) -> bool {
        !self.protocol.is_ssh()
    }

    /// 사용자 이름을 입력할 수 있는가 — 익명만 서버가 정한 계정을 쓴다 (인벤토리 #74).
    ///
    /// **키 파일에서도 사용자 이름은 필요하다** — 갈리는 것은 무엇으로 증명하느냐뿐이다
    fn user_enabled(&self) -> bool {
        self.logon != LogonType::Anonymous
    }

    /// 비밀번호를 입력할 수 있는가 (인벤토리 #75).
    ///
    /// **저장 경로도 이 판정을 쓴다** — 여기가 거짓이면 저장된 비밀번호를 지운다(D9):
    /// 인증 수단을 바꿔 놓고 쓰지도 않는 비밀이 봉인된 채 남지 않게 한다
    fn password_enabled(&self) -> bool {
        self.logon == LogonType::Normal
    }

    /// 키 파일 경로·키 암호를 입력할 수 있는가 (FR-66).
    /// 저장 경로도 이 판정을 쓴다 — 거짓이면 담아 둔 키 암호를 지운다
    fn key_enabled(&self) -> bool {
        self.logon == LogonType::KeyFile
    }

    /// `최대 동시 연결 수(M)`를 조작할 수 있는가 — 제한 체크가 켜졌을 때만이다 (인벤토리 #81)
    fn limit_enabled(&self) -> bool {
        self.limit_on
    }

    /// `인코딩(E):`를 조작할 수 있는가 — 직접 설정을 골랐을 때만이다 (인벤토리 #86)
    fn encoding_enabled(&self) -> bool {
        self.charset_custom
    }

    /// 초안이 정한 문자셋 — 직접 설정이 아니면 UTF-8이다 (D23)
    fn charset(&self) -> Charset {
        if self.charset_custom {
            Charset::Named(self.encoding.trim().to_owned())
        } else {
            Charset::Utf8
        }
    }

    /// 초안이 정한 동시 연결 상한 — 체크가 꺼져 있으면 제한 없음이다 (FR-45·D4)
    fn connection_limit(&self) -> Option<u8> {
        self.limit_on.then(|| {
            self.limit.clamp(
                *CONNECTION_LIMIT_RANGE.start(),
                *CONNECTION_LIMIT_RANGE.end(),
            )
        })
    }
}

/// 적어 넣은 포트 — 비었거나 숫자가 아니면 프로토콜 기본값, 범위를 벗어나면 클램프한다.
///
/// 조용히 버리고 기본 포트로 붙지 않는 이유는 T13의 주소 파서와 같다: 사용자가 적은 것과
/// 다른 곳에 연결되면 안 된다. 다만 여기서는 **적는 도중**의 값(`""`·`2`)도 지나가므로
/// 실패를 오류로 올리지 않고 기본값으로 되돌린다
fn parse_port(text: &str, protocol: Protocol) -> u16 {
    match text.trim().parse::<u32>() {
        Ok(value) => value.clamp(*PORT_RANGE.start(), *PORT_RANGE.end()) as u16,
        Err(_) => protocol.default_port(),
    }
}

/// 사이트 관리자 대화 (FR-27).
#[derive(Debug, Default)]
pub struct SiteManager {
    open: bool,
    tab: ManagerTab,
    /// 편집 중인 사이트 — `None`이면 새로 등록하는 초안이다
    selected: Option<SiteId>,
    draft: Draft,
    /// 이름 바꾸는 중인 글자 — 목록 행이 편집기로 바뀐다
    renaming: Option<String>,
    /// 편집기가 처음 뜨는 프레임인가 — 그때 한 번만 포커스를 준다
    rename_focus: bool,
    /// 방금 실패한 까닭 — 바닥에 그대로 보인다
    error: Option<String>,
    /// 삭제를 묻는 중인 사이트 — 확인 대화가 떠 있는 동안만 값이 있다.
    /// 되돌릴 수 없는 일이라 곧바로 지우지 않는다 (2026-08-16 검토)
    pending_delete: Option<SiteId>,
    /// 내보내기·가져오기가 지나는 단계 (FR-59)
    exchange: Exchange,
    /// 앱에 청해 둔 파일 고르기 — 앱이 한 번 꺼내 가면 비워진다
    pending_file: Option<FileRequest>,
    /// 앱이 알림으로 띄울 결과 문구 — 마찬가지로 한 번만 꺼내 간다
    notice: Option<String>,
    /// 목록을 끌어 차례를 바꾸는 중인 상태 (FR-27) — 대화가 열려 있는 동안만 산다
    drag: Option<SiteDrag>,
}

impl SiteManager {
    pub fn new() -> SiteManager {
        SiteManager::default()
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// 설정을 고치러 대화를 연다.
    ///
    /// `select`가 없으면 목록의 **첫 사이트**를 고른 채 뜬다 (인벤토리 #62 기본값) — 등록된
    /// 사이트가 하나도 없으면 새 초안이다. 실패 화면의 `설정 열기`(인벤토리 #19)는 방금 실패한
    /// 사이트를 지정해 부른다: 고치러 온 사용자가 목록에서 그것을 다시 찾게 하지 않는다
    pub fn open(&mut self, store: &SiteStore, select: Option<SiteId>) {
        self.open_new();
        let target = select
            .filter(|id| store.get(*id).is_some())
            .or_else(|| store.sites().first().map(|record| record.id));
        if let Some(id) = target {
            self.select(store, id);
        }
    }

    /// 연결 메뉴의 `사이트 관리자`(인벤토리 #8) — **빈 초안**으로 연다.
    ///
    /// 이 진입점만 첫 항목을 고르지 않는다. 여기서 기존 사이트를 골라 두면 `확인(O)`이
    /// 그것을 덮어쓰게 되어, 이 경로로는 사이트를 새로 만들 수 없게 된다.
    ///
    /// **추가 경로는 둘이다** — 이 진입점과 좌측 아랫줄의 `새 사이트` 버튼(FR-27).
    /// 둘은 하는 일이 다르다: 여기는 목록을 건드리지 않고 빈 초안만 띄우고(주소를 적어
    /// `확인(O)`을 눌러야 기록이 생긴다), 그 버튼은 **기록을 먼저 만들고** 그것을 고른다
    pub fn open_new(&mut self) {
        self.open = true;
        self.tab = ManagerTab::default();
        self.selected = None;
        self.draft = Draft::default();
        self.renaming = None;
        self.rename_focus = false;
        self.error = None;
    }

    /// 대화를 닫는다 — **고치던 이름을 먼저 확정한다**.
    ///
    /// 이름 편집은 목록 행에서 이뤄지고 `SiteStore`에 곧바로 반영되는데(모듈 주석), Enter를
    /// 누르지 않고 `확인(O)`·`연결(C)`·`X`로 대화를 끝내는 길이 더 흔하다. 여기서 확정하지
    /// 않으면 그 이름은 조용히 사라진다 (사용자 보고)
    fn close(&mut self, store: &mut SiteStore) {
        self.finish_rename(store);
        self.open = false;
        // 묻던 것도 함께 접는다 — 남겨 두면 다음에 열 때 확인 대화부터 뜬다
        self.pending_delete = None;
        // 내보내기·가져오기도 접는다 — **적어 둔 암호가 여기서 함께 사라진다** (FR-59).
        // 아직 띄우지 못한 파일 대화 요청도 함께 버린다: 대화를 닫았다는 것은 그 흐름을
        // 그만두겠다는 뜻이라, 닫은 뒤에 파일 창이 뒤늦게 뜨는 편이 오히려 놀랍다
        self.exchange = Exchange::Idle;
        self.pending_file = None;
        // 끌던 것도 접는다 — 다음에 열 때 끌던 상태로 시작하면 안 된다
        self.drag = None;
        self.renaming = None;
        self.rename_focus = false;
        self.error = None;
        // 초안을 버린다 — 평문 비밀번호도 여기서 함께 사라진다
        self.draft = Draft::default();
    }

    /// 목록에서 사이트를 고른다 — 그 설정이 우측 폼으로 들어온다
    fn select(&mut self, store: &SiteStore, id: SiteId) {
        if let Some(draft) = Draft::load(store, id) {
            self.selected = Some(id);
            self.draft = draft;
            self.renaming = None;
            self.rename_focus = false;
            self.error = None;
        }
    }

    /// 초안을 목록에 반영한다 — 고른 사이트가 있으면 갱신하고, 없으면 새로 만든다.
    /// 등록하지 못하면 까닭을 남기고 `None`이다 (plan Edge Case: 호스트가 빈 값)
    fn commit(&mut self, store: &mut SiteStore) -> Option<SiteId> {
        if self.draft.host.trim().is_empty() {
            self.error = Some(crate::i18n::site_error_no_host().to_owned());
            return None;
        }
        let id = match self.selected.filter(|id| store.get(*id).is_some()) {
            Some(id) => id,
            // 이름은 호스트로 잡는다 — 겹치면 `SiteStore`가 `(2)`를 붙인다
            None => store.add(self.draft.host.trim()),
        };
        let port = parse_port(&self.draft.port, self.draft.protocol);
        if let Some(record) = store.get_mut(id) {
            record.protocol = self.draft.protocol;
            record.host = self.draft.host.trim().to_owned();
            record.port = port;
            record.encryption = self.draft.encryption;
            record.logon = self.draft.logon;
            record.user = self.draft.user.clone();
            // 키 파일이 아니면 담아 둘 경로가 없다 — 비밀번호와 같은 규칙으로 지운다
            record.key_path = self
                .draft
                .key_enabled()
                .then(|| self.draft.key_path.trim())
                .filter(|path| !path.is_empty())
                .map(std::path::PathBuf::from);
            // 전송 설정·문자셋 탭의 값 — `최대 동시 연결 수(M)`는 여기서 기록에 담겨야
            // 연결 관리자의 채널 배정에 그대로 닿는다 (FR-45·D4)
            record.transfer_mode = self.draft.transfer_mode;
            record.connection_limit = self.draft.connection_limit();
            record.charset = self.draft.charset();
        }
        // 그 인증 수단을 쓰지 않으면 담아 둔 비밀을 지운다 (D9) — 익명으로 바꿀 때와 같은
        // 규칙이며, 쓰지도 않는 비밀이 봉인된 채 남지 않게 한다
        let password = if self.draft.password_enabled() {
            self.draft.password.as_str()
        } else {
            ""
        };
        let key_passphrase = if self.draft.key_enabled() {
            self.draft.key_passphrase.as_str()
        } else {
            ""
        };
        // **둘 다 반드시 부른다** — `||`로 이으면 앞이 실패했을 때 뒤가 아예 실행되지 않아,
        // 비밀번호 봉인이 실패한 사이트의 키 암호가 담기지도 지워지지도 않은 채 남는다
        let password_stored = store.set_password(id, password);
        let key_stored = store.set_key_passphrase(id, key_passphrase);
        // 어느 쪽이 실패했는지 가려 알린다 — 키로 접속하는 사용자에게 「비밀번호」라고
        // 알리면 고칠 필요가 없는 칸을 고치게 된다
        if !password_stored {
            self.error = Some(crate::i18n::site_error_password().to_owned());
        } else if !key_stored {
            self.error = Some(crate::i18n::site_error_key_passphrase().to_owned());
        }
        // 등록한 사이트는 사이드바에도 보여야 한다 — 주소창으로 한 번 열어 숨겨 둔 것을
        // 관리자에서 등록하면 그때부터는 사용자가 목록에 두겠다는 뜻이다
        store.unhide(id);
        self.selected = Some(id);
        self.draft.port = port.to_string();
        Some(id)
    }

    /// 좌측 버튼들을 목록에 반영한다 (Acceptance ⑤)
    fn apply_list_action(&mut self, action: ListAction, store: &mut SiteStore) {
        // 아래 둘은 고른 것이 없어도 할 일이 있어 선택 가드보다 앞에서 갈린다
        match action {
            // 호스트가 빈 채로 만들어지므로 사이드바·연결 메뉴·새 탭 메뉴에는 서지 않고
            // (`SiteStore::visible`), 사용자가 주소를 적어 `확인(O)`을 눌러야 등록이 끝난다
            ListAction::New => {
                let id = store.add(crate::i18n::site_default_name());
                self.select(store, id);
                return;
            }
            // 끄는 줄과 고른 줄이 다를 수 있다. **고른 사이트는 그대로다** —
            // `selected`가 `SiteId`라 자리가 바뀌어도 가리키는 곳이 흔들리지 않는다
            ListAction::Reorder(from, to) => {
                store.reorder(from, to);
                return;
            }
            _ => {}
        }
        let Some(id) = self.selected else {
            // 고른 사이트가 없으면 할 것이 없다 (plan Edge Case: 사이트 0개에서 `삭제(D)`)
            return;
        };
        match action {
            ListAction::StartRename => {
                self.renaming = store.get(id).map(|record| record.name.clone());
                self.rename_focus = self.renaming.is_some();
            }
            // 곧바로 지우지 않고 한 번 묻는다 — 주소·로그인 정보가 함께 사라지고 되돌릴 수 없다
            ListAction::Delete => self.pending_delete = Some(id),
            ListAction::Duplicate => {
                if let Some(copy) = store.duplicate(id) {
                    self.select(store, copy);
                }
            }
            // 위에서 이미 갈라져 여기 닿지 않는다
            ListAction::New | ListAction::Reorder(..) => {}
        }
    }

    /// 이름 바꾸기를 마친다 — 빈 이름이면 `SiteStore`가 기본 이름을 붙인다
    fn finish_rename(&mut self, store: &mut SiteStore) {
        self.rename_focus = false;
        let (Some(id), Some(name)) = (self.selected, self.renaming.take()) else {
            return;
        };
        store.rename(id, &name);
    }

    /// 대화를 그린다. 닫혀 있으면 아무것도 그리지 않는다.
    ///
    /// `connected`는 목록 행의 상태 점에 쓴다 — 연결 계층을 통째로 넘기지 않는 것은
    /// 사이드바와 같은 이유다(알아야 하는 것은 "연결이 있는가" 하나뿐이다)
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        store: &mut SiteStore,
        connected: &[SiteId],
    ) -> SiteManagerOutcome {
        if !self.open {
            return SiteManagerOutcome::None;
        }
        let mut outcome = SiteManagerOutcome::None;
        let mut body = BodyOutcome::default();
        // 왼쪽부터 연결·확인·취소 — PRD FR-27이 정한 순서다
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::site_connect()),
            dialog::ButtonSpec::plain(crate::i18n::site_ok()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show_fixed(
            ctx,
            egui::Id::new("사이트 관리자"),
            egui::vec2(DIALOG_WIDTH, DIALOG_HEIGHT),
            &buttons,
            |ui, rect| {
                // `rect`는 바닥 버튼 줄을 뺀 나머지다 — 그 아래쪽 한 줄을 오류 문구가 쓴다
                let header =
                    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), HEADER_HEIGHT));
                let error_row = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - ERROR_ROW_HEIGHT),
                    rect.max,
                );
                let body_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), header.bottom()),
                    egui::pos2(rect.right(), error_row.top()),
                );
                if self.show_header(ui, header) {
                    outcome = SiteManagerOutcome::Close;
                }
                body = self.show_body(ui, body_rect, store, connected);
                self.show_error_row(ui, error_row);
            },
        );
        // 위에 다른 대화가 떠 있는 동안에는 이 대화의 조작을 받지 않는다 — 그것이 답을 기다린다.
        // egui가 아래 모달의 입력을 막아 주지만 그 판정은 **다음 프레임**부터라, 묻기
        // 시작한 그 프레임에 이 대화의 버튼이 함께 눌리는 것을 여기서 막는다.
        // 내보내기·가져오기도 같다 (FR-59) — 파일 대화를 기다리는 동안도 포함한다
        let asking = self.pending_delete.is_some() || self.exchange != Exchange::Idle;
        if let Some(index) = shell.clicked.filter(|_| !asking) {
            let connect = index == CONNECT_BUTTON;
            outcome = match index {
                // 연결·확인은 등록을 거쳐야 한다 — 값이 모자라면 `commit`이 오류를 남기고
                // `None`을 주며, 그때는 대화를 그대로 둔다
                CONNECT_BUTTON | CONFIRM_BUTTON => match self.commit(store) {
                    Some(id) if connect => SiteManagerOutcome::RegisterAndConnect(id),
                    Some(id) => SiteManagerOutcome::Register(id),
                    None => SiteManagerOutcome::None,
                },
                _ => SiteManagerOutcome::Close,
            };
        }
        // 그리는 동안에는 목록을 빌려 읽고 있었다 — 변경은 여기서 적용한다
        if body.rename_done {
            self.finish_rename(store);
        }
        if let Some(id) = body.picked {
            self.select(store, id);
        }
        if let Some(action) = body.action {
            self.apply_list_action(action, store);
        }
        if let Some(action) = body.exchange {
            self.apply_exchange_action(action);
        }
        // 삭제 확인은 **관리자 위에** 뜬다 — 뒤의 대화는 그대로 두고 답만 기다린다
        self.show_delete_confirm(ctx, store);
        // 내보내기·가져오기 대화도 같은 자리다 (FR-59)
        self.show_exchange(ctx, store);
        if shell.should_close && !asking {
            outcome = SiteManagerOutcome::Close;
        }
        if !matches!(outcome, SiteManagerOutcome::None) {
            self.close(store);
        }
        outcome
    }

    /// 사이트 삭제 확인 (2026-08-16 검토) — 워크스페이스 삭제 대화와 같은 구성이다.
    ///
    /// 배경 클릭·`Esc`는 취소로 본다: 지우는 쪽이 기본값이 되면 실수로 지우게 된다
    fn show_delete_confirm(&mut self, ctx: &egui::Context, store: &mut SiteStore) {
        let Some(id) = self.pending_delete else {
            return;
        };
        let Some(name) = store.get(id).map(|record| record.name.clone()) else {
            // 묻는 사이에 대상이 사라졌다 — 물을 것이 없다
            self.pending_delete = None;
            return;
        };
        let buttons = [
            dialog::ButtonSpec::strong(crate::i18n::delete()),
            dialog::ButtonSpec::plain(crate::i18n::cancel()),
        ];
        let shell = dialog::show(
            ctx,
            egui::Id::new("사이트 삭제 확인"),
            DELETE_CONFIRM_WIDTH,
            &buttons,
            |ui| {
                ui.heading(crate::i18n::site_delete_title());
                ui.add_space(8.0);
                ui.label(crate::i18n::dynamic::site_delete_confirm(&name));
                ui.label(crate::i18n::site_delete_detail());
            },
        );
        match shell.clicked {
            Some(0) => {
                store.remove(id);
                self.pending_delete = None;
                self.selected = None;
                self.draft = Draft::default();
                self.renaming = None;
                self.rename_focus = false;
            }
            Some(_) => self.pending_delete = None,
            None => {
                if shell.should_close {
                    self.pending_delete = None;
                }
            }
        }
    }

    /// 헤더 — 제목과 닫기 버튼. 닫기를 눌렀으면 `true` (`:386-388`)
    fn show_header(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        ui.painter().text(
            egui::pos2(rect.left() + HEADER_PAD_LEFT, rect.center().y),
            egui::Align2::LEFT_CENTER,
            crate::i18n::site_title(),
            egui::FontId::proportional(TITLE_FONT_PX),
            theme::TEXT,
        );
        let close = egui::Rect::from_min_size(
            egui::pos2(
                rect.right() - HEADER_PAD_RIGHT - CLOSE_WIDTH,
                rect.center().y - CLOSE_HEIGHT / 2.0,
            ),
            egui::vec2(CLOSE_WIDTH, CLOSE_HEIGHT),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(close));
        widgets::icon_button_styled(
            &mut child,
            // 아이콘 글꼴에서 가져온다 — `✕`(U+2715)는 이 앱의 글꼴에 없어 두부가 된다
            egui_phosphor::regular::X,
            close.size(),
            theme::CLOSE_HOT,
            theme::HEADER_TEXT,
            CLOSE_FONT_PX,
        )
        .clicked()
    }

    /// 본문 — 좌측 목록과 우측 탭·폼 (`:391-492`)
    fn show_body(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        store: &SiteStore,
        connected: &[SiteId],
    ) -> BodyOutcome {
        let content = egui::Rect::from_min_max(
            egui::pos2(rect.left() + BODY_PAD_X, rect.top() + BODY_PAD_TOP),
            egui::pos2(rect.right() - BODY_PAD_X, rect.bottom()),
        );
        let left = egui::Rect::from_min_size(content.min, egui::vec2(LEFT_WIDTH, content.height()));
        let right = egui::Rect::from_min_max(
            egui::pos2(left.right() + BODY_GAP, content.top()),
            content.max,
        );
        let (picked, rename_done, dragged) = self.show_list(ui, left, store, connected);
        // **두 줄을 모두 그린 뒤에 고른다** — 즉시 모드라 그리기를 건너뛰면 그 줄이
        // 화면에서 사라진다. `ListAction`의 생산자가 둘이 되므로 뒤에 그린 아랫줄이
        // 이긴다: 포인터가 하나라 한 프레임에 두 칸이 함께 눌릴 일은 없고, 그래도
        // 겹치면 사용자가 마지막으로 누른 것을 택한다
        let top_action = self.show_list_buttons(ui, left);
        let bottom = self.show_bottom_buttons(ui, left, store);
        // 끌어 놓기가 가장 먼저 그려졌으므로 버튼 줄이 그것을 덮는다
        let action = bottom.list.or(top_action).or(dragged);
        let exchange = bottom.exchange;
        self.show_tabs(ui, right);
        // 이미 연결된 사이트의 전송 모드를 바꿨으면 그 사실을 알린다 (plan Edge Case)
        let transfer_hint = self.selected.is_some_and(|id| {
            connected.contains(&id)
                && store
                    .get(id)
                    .is_some_and(|record| record.transfer_mode != self.draft.transfer_mode)
        });
        self.show_tab_body(ui, right, transfer_hint);
        BodyOutcome {
            picked,
            action,
            exchange,
            rename_done,
        }
    }

    /// 사이트 목록을 두르는 웰 — 라벨 아래부터 윗줄 버튼 바로 위까지.
    ///
    /// **버튼 두 줄이 이 사각형과 좌우를 맞춘다**(`button_grid`) — 그리기와 따로 값으로
    /// 낼 수 있어야 시험이 화면 없이 그 정합을 잰다
    fn well_rect(&self, column: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_max(
            egui::pos2(column.left(), column.top() + LIST_LABEL_HEIGHT + LEFT_GAP),
            egui::pos2(
                column.right(),
                self.buttons_top(column) - GRID_PAD_TOP - LEFT_GAP,
            ),
        )
    }

    /// 좌측 목록 — 라벨 + 웰. 고른 사이트를 돌려준다 (`:393-406`)
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        column: egui::Rect,
        store: &SiteStore,
        connected: &[SiteId],
    ) -> (Option<SiteId>, bool, Option<ListAction>) {
        ui.painter().text(
            egui::pos2(column.left(), column.top() + LIST_LABEL_HEIGHT / 2.0),
            egui::Align2::LEFT_CENTER,
            crate::i18n::site_list_label(),
            egui::FontId::proportional(widgets::FORM_FONT_PX),
            theme::HEADER_TEXT,
        );
        let well = self.well_rect(column);
        ui.painter().rect(
            well,
            0.0,
            theme::WELL_BG,
            egui::Stroke::new(1.0, theme::PANE_BORDER),
            egui::StrokeKind::Inside,
        );

        let rows = egui::Rect::from_min_max(
            egui::pos2(well.left() + LIST_PAD_X, well.top() + LIST_PAD_Y),
            egui::pos2(well.right() - LIST_PAD_X, well.bottom() - LIST_PAD_Y),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rows)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.spacing_mut().item_spacing.y = 0.0;
        // 웰 밖으로 흘러넘치지 않게 자른다 — 사이트가 많으면 아래쪽이 잘린다
        child.set_clip_rect(rows);
        let mut picked = None;
        let mut rename_done = false;
        // **레코드마다 반드시 한 칸을 넣는다** — 이름 바꾸는 중인 줄도 자리를 차지하므로,
        // 한 칸이라도 건너뛰면 그 아래가 전부 밀려 엉뚱한 자리로 옮겨진다
        let mut row_rects = Vec::with_capacity(store.sites().len());
        // 편집기를 그리기 전에 빼 둔다 — 루프 안에서는 `renaming`을 빌리고 있어 함께 못 읽는다
        let focus = std::mem::take(&mut self.rename_focus);
        for (index, record) in store.sites().iter().enumerate() {
            let selected = self.selected == Some(record.id);
            let dot = if connected.contains(&record.id) {
                theme::OK_DOT
            } else {
                theme::TEXT_DIM
            };
            // 이름 바꾸는 중인 줄만 편집기로 바뀐다 — 그 줄은 끌 수 없다(글자를 고르는 중이다)
            if selected && let Some(name) = &mut self.renaming {
                let (rect, done) = show_rename_row(&mut child, name, dot, focus);
                row_rects.push(rect);
                rename_done = done;
                continue;
            }
            let response = show_site_row(&mut child, &record.name, dot, selected);
            row_rects.push(response.rect);
            if response.clicked() {
                picked = Some(record.id);
            }
            // 끌기 시작 — 임계를 넘기 전에는 아직 클릭일 수 있다
            if response.drag_started()
                && let Some(at) = response.interact_pointer_pos()
            {
                self.drag = Some(SiteDrag {
                    from: index,
                    start: at,
                    active: false,
                });
            }
            if response.dragged()
                && let (Some(drag), Some(at)) =
                    (self.drag.as_mut(), response.interact_pointer_pos())
                && (at - drag.start).length() >= widgets::DRAG_THRESHOLD
            {
                drag.active = true;
            }
        }
        let action = self.finish_site_drag(&child, &row_rects);
        (picked, rename_done, action)
    }

    /// 끌던 줄을 놓은 자리를 계산해 조작을 올린다. 끄는 중이면 놓일 자리에 선을 긋는다.
    ///
    /// 워크스페이스·즐겨찾기와 같은 얼개다 — 임계를 못 넘은 제스처는 클릭으로 이미
    /// 처리됐으므로 버튼을 떼는 순간 상태만 비운다
    fn finish_site_drag(&mut self, ui: &egui::Ui, rows: &[egui::Rect]) -> Option<ListAction> {
        let drag = self.drag.as_ref()?;
        if !drag.active {
            if ui.input(|i| !i.pointer.any_down()) {
                self.drag = None;
            }
            return None;
        }
        let from = drag.from;
        // **위치를 못 얻으면 끌기를 접는다** — `?`로 빠져나가면 놓기 판정도 건너뛰어,
        // 그 프레임에 버튼을 뗐어도 `active`가 남아 선이 그대로 보인다 (`ui::tree`도 같다)
        let Some(at) = ui.input(|i| i.pointer.interact_pos()) else {
            self.drag = None;
            return None;
        };
        let insert_at = insert_index_at(at.y, rows);
        if let Some(y) = insert_line_y(insert_at, rows) {
            let line = egui::Rect::from_min_size(
                egui::pos2(ui.max_rect().left(), y - widgets::INSERT_LINE_HEIGHT / 2.0),
                egui::vec2(ui.max_rect().width(), widgets::INSERT_LINE_HEIGHT),
            );
            ui.painter().rect_filled(line, 0.0, theme::ACCENT);
        }
        if ui.input(|i| !i.pointer.any_down()) {
            self.drag = None;
            return widgets::reorder_target(from, insert_at)
                .map(|to| ListAction::Reorder(from, to));
        }
        None
    }

    /// **윗줄** 버튼이 시작하는 y — 목록 웰의 아래끝을 정하는 데도 쓴다.
    ///
    /// 줄이 둘이 되면서 목록 웰이 한 줄 높이(28px)와 줄 간격(8px)만큼 짧아졌다 (plan D10)
    fn buttons_top(&self, column: egui::Rect) -> f32 {
        column.bottom()
            - GRID_PAD_BOTTOM
            - GRID_BUTTON_HEIGHT * GRID_ROWS
            - GRID_GAP * (GRID_ROWS - 1.0)
    }

    /// 버튼 한 줄이 앉을 자리 — 좌우 끝은 **목록 웰과 같다**(`show_list`의 `well`).
    ///
    /// 원본의 좌우 여백 30px을 걷어낸 것이 요구 ④다(`GRID_GAP` 주석) — 이 함수 하나가
    /// 두 줄의 좌우를 정하므로 둘이 어긋날 길이 없다
    fn button_grid(&self, column: egui::Rect, top: f32) -> egui::Rect {
        egui::Rect::from_min_max(
            egui::pos2(column.left(), top),
            egui::pos2(column.right(), top + GRID_BUTTON_HEIGHT),
        )
    }

    /// 한 줄에 세 칸을 균등하게 나눈 폭
    fn button_width(grid: egui::Rect) -> f32 {
        (grid.width() - GRID_GAP * 2.0) / 3.0
    }

    /// 좌측 버튼 **윗줄** — `이름 바꾸기(R)`·`삭제(D)`·`복제(I)` (`:407-409`, 인벤토리 #63~65)
    fn show_list_buttons(&mut self, ui: &mut egui::Ui, column: egui::Rect) -> Option<ListAction> {
        let grid = self.button_grid(column, self.buttons_top(column));
        let width = Self::button_width(grid);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(grid)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = GRID_GAP;
        // 고른 사이트가 없으면 셋 다 할 일이 없다 (인벤토리 #63~65 "사이트 선택 시")
        let enabled = self.selected.is_some();
        let mut action = None;
        for (label, candidate) in [
            (crate::i18n::site_rename(), ListAction::StartRename),
            (crate::i18n::site_delete(), ListAction::Delete),
            (crate::i18n::site_duplicate(), ListAction::Duplicate),
        ] {
            if button_cell(&mut child, label, enabled, width) {
                action = Some(candidate);
            }
        }
        action
    }

    /// 아랫줄 버튼이 시작하는 y
    fn bottom_buttons_top(&self, column: egui::Rect) -> f32 {
        column.bottom() - GRID_PAD_BOTTOM - GRID_BUTTON_HEIGHT
    }

    /// 좌측 버튼 **아랫줄** — `새 사이트`(FR-27)·`내보내기`·`가져오기`(FR-59).
    ///
    /// 이 줄이 종전에는 `exchange` 자식 모듈에 있었다 — `새 사이트`가 들어오면서 더는
    /// 파일을 주고받는 줄이 아니라 여기로 옮겨 왔다. 활성 조건이 칸마다 다르다:
    /// `새 사이트`는 늘 누를 수 있고, **`내보내기`는 등록된 사이트가 없으면 비활성**이며
    /// (내보낼 것이 없다), `가져오기`는 목록이 비어 있어도 할 일이 있어 늘 활성이다
    fn show_bottom_buttons(
        &mut self,
        ui: &mut egui::Ui,
        column: egui::Rect,
        store: &SiteStore,
    ) -> BottomOutcome {
        let grid = self.button_grid(column, self.bottom_buttons_top(column));
        let width = Self::button_width(grid);
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(grid)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = GRID_GAP;

        let mut outcome = BottomOutcome::default();
        if button_cell(&mut child, crate::i18n::site_new(), true, width) {
            outcome.list = Some(ListAction::New);
        }
        for (label, candidate, enabled) in [
            (
                crate::i18n::site_export(),
                ExchangeAction::Export,
                !store.is_empty(),
            ),
            (crate::i18n::site_import(), ExchangeAction::Import, true),
        ] {
            if button_cell(&mut child, label, enabled, width) {
                outcome.exchange = Some(candidate);
            }
        }
        outcome
    }

    /// 우측 탭 줄 (`:415-417`, 인벤토리 #66~68)
    fn show_tabs(&mut self, ui: &mut egui::Ui, column: egui::Rect) {
        let strip = egui::Rect::from_min_size(column.min, egui::vec2(column.width(), TAB_HEIGHT));
        let stroke = egui::Stroke::new(1.0, theme::PANE_BORDER);
        // 스트립 아래 선을 먼저 긋고 활성 탭이 그 위를 덮는다 — 활성 탭만 선이 끊긴다
        ui.painter().line_segment(
            [
                egui::pos2(strip.left(), strip.bottom() - 0.5),
                egui::pos2(strip.right(), strip.bottom() - 0.5),
            ],
            stroke,
        );
        let mut left = strip.left();
        for (tab, label) in [
            (ManagerTab::General, crate::i18n::site_tab_general()),
            (ManagerTab::Transfer, crate::i18n::site_tab_transfer()),
            (ManagerTab::Charset, crate::i18n::site_tab_charset()),
        ] {
            let text = ui.painter().layout_no_wrap(
                label.to_owned(),
                egui::FontId::proportional(widgets::FORM_FONT_PX),
                theme::TEXT,
            );
            let width = text.size().x + TAB_PAD_X * 2.0;
            let rect = egui::Rect::from_min_size(
                egui::pos2(left, strip.top()),
                egui::vec2(width, TAB_HEIGHT),
            );
            left += width;
            let active = self.tab == tab;
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
            let response = child.allocate_rect(rect, egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                0.0,
                if active {
                    theme::SURFACE_BG
                } else {
                    TAB_INACTIVE_BG
                },
            );
            if active {
                // 아래쪽만 두르지 않는다 — 그 자리는 본문과 이어진다 (`:417` `border-bottom:none`)
                for [from, to] in [
                    [rect.left_top(), rect.right_top()],
                    [rect.left_top(), rect.left_bottom()],
                    [rect.right_top(), rect.right_bottom()],
                ] {
                    ui.painter().line_segment([from, to], stroke);
                }
            }
            ui.painter().galley(
                egui::pos2(
                    rect.left() + TAB_PAD_X,
                    rect.center().y - text.size().y / 2.0,
                ),
                text,
                theme::TEXT,
            );
            if response.clicked() {
                self.tab = tab;
            }
        }
    }

    /// 지금 고른 탭의 본문을 그린다 — 셋 중 하나만 그려진다
    fn show_tab_body(&mut self, ui: &mut egui::Ui, column: egui::Rect, transfer_hint: bool) {
        match self.tab {
            ManagerTab::General => self.show_general(ui, column),
            ManagerTab::Transfer => self.show_transfer(ui, self.form_rect(column), transfer_hint),
            ManagerTab::Charset => self.show_charset(ui, self.form_rect(column)),
        }
    }

    /// 탭 본문이 앉는 자리 (`:421` — 탭 줄 아래 `padding 16px 2px 0`)
    fn form_rect(&self, column: egui::Rect) -> egui::Rect {
        egui::Rect::from_min_max(
            egui::pos2(
                column.left() + FORM_PAD_X,
                column.top() + TAB_HEIGHT + FORM_PAD_TOP,
            ),
            egui::pos2(column.right() - FORM_PAD_X, column.bottom()),
        )
    }

    /// `전송 설정` 탭 (`:440-467`, 인벤토리 #76~81).
    ///
    /// 여기서 정한 `최대 동시 연결 수(M)`가 그대로 `SiteRecord`에 담겨 연결 관리자의
    /// 채널 배정이 된다 (FR-45·D4) — 화면만 있고 동작에 닿지 않으면 그 요구가 성립하지 않는다
    fn show_transfer(&mut self, ui: &mut egui::Ui, form: egui::Rect, transfer_hint: bool) {
        let left = form.left() + TAB_BODY_PAD;
        let mut top = form.top() + TAB_BODY_PAD;

        text_row(
            ui,
            left,
            top,
            form.width(),
            crate::i18n::site_label_transfer_mode(),
            theme::HEADER_TEXT,
        );
        top += TEXT_ROW_HEIGHT + TRANSFER_GAP;

        // 전송 모드 라디오 3개 — 가로로 늘어선다 (`:443`)
        let mut row = self.row(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(left + MARK_INDENT, top),
                egui::vec2(form.width() - MARK_INDENT, widgets::FORM_FIELD_HEIGHT),
            ),
        );
        row.spacing_mut().item_spacing.x = RADIO_GAP;
        let mut picked_mode = None;
        for (mode, label, hint) in transfer_options() {
            if widgets::radio_row(
                &mut row,
                label,
                self.draft.transfer_mode == mode,
                Some(hint),
            ) {
                picked_mode = Some(mode);
            }
        }
        if let Some(mode) = picked_mode {
            self.draft.transfer_mode = mode;
        }
        top += widgets::FORM_FIELD_HEIGHT + TRANSFER_GAP + CHECK_MARGIN_TOP;

        // 동시 연결 수 제한 — 이 체크가 아래 스피너의 활성 여부를 가른다 (Acceptance ②)
        let mut row = self.row(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(left + MARK_INDENT, top),
                egui::vec2(form.width() - MARK_INDENT, widgets::FORM_FIELD_HEIGHT),
            ),
        );
        if widgets::check_row(
            &mut row,
            crate::i18n::site_label_limit(),
            self.draft.limit_on,
        ) {
            self.draft.limit_on = !self.draft.limit_on;
        }
        top += widgets::FORM_FIELD_HEIGHT + TRANSFER_GAP;

        let limit_on = self.draft.limit_enabled();
        let mut row = self.row(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(left + SPINNER_INDENT, top),
                egui::vec2(form.width() - SPINNER_INDENT, widgets::SPINNER_HEIGHT),
            ),
        );
        widgets::form_inline_label(&mut row, crate::i18n::site_label_limit_value(), limit_on);
        self.draft.limit =
            widgets::spinner_field(&mut row, self.draft.limit, CONNECTION_LIMIT_RANGE, limit_on);

        if transfer_hint {
            top += widgets::SPINNER_HEIGHT + TRANSFER_GAP;
            text_row(
                ui,
                left,
                top,
                form.width(),
                crate::i18n::site_transfer_mode_notice(),
                theme::WARN,
            );
        }
    }

    /// `문자셋` 탭 (`:469-486`, 인벤토리 #82~87)
    fn show_charset(&mut self, ui: &mut egui::Ui, form: egui::Rect) {
        let left = form.left() + TAB_BODY_PAD;
        let mut top = form.top() + TAB_BODY_PAD;

        for text in [
            crate::i18n::site_charset_heading(),
            crate::i18n::site_charset_label(),
        ] {
            text_row(ui, left, top, form.width(), text, theme::HEADER_TEXT);
            top += TEXT_ROW_HEIGHT + CHARSET_GAP;
        }

        // 라디오 2개 — 세로로 쌓인다 (`:473-479`)
        for (index, label) in charset_options().into_iter().enumerate() {
            let custom = index == 1;
            let mut row = self.row(
                ui,
                egui::Rect::from_min_size(
                    egui::pos2(left + MARK_INDENT, top),
                    egui::vec2(form.width() - MARK_INDENT, widgets::FORM_FIELD_HEIGHT),
                ),
            );
            if widgets::radio_row(&mut row, label, self.draft.charset_custom == custom, None) {
                self.draft.charset_custom = custom;
            }
            top += widgets::FORM_FIELD_HEIGHT + CHARSET_GAP;
        }

        // 인코딩 이름 — 직접 설정일 때만 활성이다 (Acceptance ③)
        let custom = self.draft.encoding_enabled();
        let mut row = self.row(
            ui,
            egui::Rect::from_min_size(
                egui::pos2(left + ENCODING_INDENT, top),
                egui::vec2(form.width() - ENCODING_INDENT, ENCODING_HEIGHT),
            ),
        );
        widgets::form_inline_label(&mut row, crate::i18n::site_label_encoding(), custom);
        widgets::text_field(
            &mut row,
            "encoding",
            &mut self.draft.encoding,
            egui::vec2(ENCODING_WIDTH, ENCODING_HEIGHT),
            custom,
            false,
        );
        top += ENCODING_HEIGHT + FOOTNOTE_MARGIN_TOP;

        // 알아듣지 못하는 이름이면 그 사실을 알린다 (plan Edge Case)
        if custom && !charset::is_known(&self.draft.charset()) {
            text_row(
                ui,
                left,
                top,
                form.width(),
                crate::i18n::site_charset_unknown_hint(),
                theme::WARN,
            );
            top += TEXT_ROW_HEIGHT + CHARSET_GAP;
        }
        text_row(
            ui,
            left,
            top,
            form.width(),
            crate::i18n::site_charset_warning(),
            theme::TEXT_MUTED,
        );
    }

    /// `일반` 탭의 폼 (`:421-436`, 인벤토리 #69~75)
    fn show_general(&mut self, ui: &mut egui::Ui, column: egui::Rect) {
        let form = egui::Rect::from_min_max(
            egui::pos2(
                column.left() + FORM_PAD_X,
                column.top() + TAB_HEIGHT + FORM_PAD_TOP,
            ),
            egui::pos2(column.right() - FORM_PAD_X, column.bottom()),
        );
        let row_height = widgets::FORM_FIELD_HEIGHT;
        let row_rect = |index: usize| {
            egui::Rect::from_min_size(
                egui::pos2(
                    form.left(),
                    form.top() + index as f32 * (row_height + FORM_ROW_GAP),
                ),
                egui::vec2(form.width(), row_height),
            )
        };
        let field_width = form.width() - widgets::FORM_LABEL_WIDTH - widgets::FORM_GAP;

        // 프로토콜 — 바꾸면 포트 기본값이 따라온다 (Acceptance ③)
        let mut row = self.row(ui, row_rect(0));
        widgets::form_label(&mut row, crate::i18n::site_label_protocol(), true);
        let labels: Vec<&str> = protocol_options().iter().map(|(_, label)| *label).collect();
        if let Some(index) = widgets::dropdown_field(
            &mut row,
            "protocol",
            option_label(&protocol_options(), self.draft.protocol),
            field_width,
            true,
            &labels,
        ) {
            self.draft.set_protocol(protocol_options()[index].0);
        }

        // 호스트 + 포트 — 한 행에 둘이 온다 (`:430-435`)
        let mut row = self.row(ui, row_rect(1));
        widgets::form_label(&mut row, crate::i18n::site_label_host(), true);
        let port_label_width = row
            .painter()
            .layout_no_wrap(
                crate::i18n::site_label_port().to_owned(),
                egui::FontId::proportional(widgets::FORM_FONT_PX),
                theme::HEADER_TEXT,
            )
            .size()
            .x;
        let host_width = field_width
            - PORT_LABEL_MARGIN
            - widgets::FORM_GAP
            - port_label_width
            - widgets::FORM_GAP
            - PORT_WIDTH;
        widgets::text_field(
            &mut row,
            "host",
            &mut self.draft.host,
            egui::vec2(host_width, widgets::FORM_FIELD_HEIGHT),
            true,
            false,
        );
        row.add_space(PORT_LABEL_MARGIN);
        widgets::form_inline_label(&mut row, crate::i18n::site_label_port(), true);
        if widgets::text_field(
            &mut row,
            "port",
            &mut self.draft.port,
            egui::vec2(PORT_WIDTH, widgets::FORM_FIELD_HEIGHT),
            true,
            false,
        )
        .changed()
        {
            self.draft.port_edited = true;
        }

        // 암호화 — FTP 계열에만 뜻이 있다 (인벤토리 #72)
        let mut row = self.row(ui, row_rect(2));
        let encryption_enabled = self.draft.encryption_enabled();
        widgets::form_label(
            &mut row,
            crate::i18n::site_label_encryption(),
            encryption_enabled,
        );
        let labels: Vec<&str> = encryption_options()
            .iter()
            .map(|(_, label)| *label)
            .collect();
        if let Some(index) = widgets::dropdown_field(
            &mut row,
            "encryption",
            option_label(&encryption_options(), self.draft.encryption),
            field_width,
            encryption_enabled,
            &labels,
        ) {
            self.draft.encryption = encryption_options()[index].0;
        }

        // 로그온 유형 — `키 파일`은 SFTP에서만 선다 (D7)
        let options = logon_options(self.draft.protocol);
        let mut row = self.row(ui, row_rect(3));
        widgets::form_label(&mut row, crate::i18n::site_label_logon(), true);
        let labels: Vec<&str> = options.iter().map(|(_, label)| *label).collect();
        if let Some(index) = widgets::dropdown_field(
            &mut row,
            "logon",
            option_label(&options, self.draft.logon),
            field_width,
            true,
            &labels,
        ) {
            self.draft.logon = options[index].0;
        }

        // 사용자·비밀번호·키 — 로그온 유형이 셋의 활성을 각자 정한다 (Acceptance ④·D9)
        let user = self.draft.user_enabled();
        let password = self.draft.password_enabled();
        let key = self.draft.key_enabled();

        let mut row = self.row(ui, row_rect(4));
        widgets::form_label(&mut row, crate::i18n::site_label_user(), user);
        widgets::text_field(
            &mut row,
            "user",
            &mut self.draft.user,
            egui::vec2(field_width, widgets::FORM_FIELD_HEIGHT),
            user,
            false,
        );

        let mut row = self.row(ui, row_rect(5));
        widgets::form_label(&mut row, crate::i18n::site_label_password(), password);
        widgets::text_field(
            &mut row,
            "password",
            &mut self.draft.password,
            egui::vec2(field_width, widgets::FORM_FIELD_HEIGHT),
            password,
            true,
        );

        // 키 파일 경로 — 입력칸 옆에 `찾아보기`가 선다 (FR-66)
        let mut row = self.row(ui, row_rect(6));
        widgets::form_label(&mut row, crate::i18n::site_label_key_path(), key);
        let browse_label = crate::i18n::site_key_browse();
        let browse_width = widgets::design_button_width(&row, browse_label, BROWSE_PAD_X);
        widgets::text_field(
            &mut row,
            "key_path",
            &mut self.draft.key_path,
            egui::vec2(
                field_width - browse_width - widgets::FORM_GAP,
                widgets::FORM_FIELD_HEIGHT,
            ),
            key,
            false,
        );
        let clicked = row
            .add_enabled_ui(key, |ui| {
                widgets::design_button(
                    ui,
                    browse_label,
                    if key { theme::TEXT } else { theme::TEXT_DIM },
                    BROWSE_PAD_X,
                    egui::vec2(browse_width, widgets::FORM_FIELD_HEIGHT),
                )
                .clicked()
            })
            .inner;
        if clicked {
            self.request_key_file();
        }

        // 키 암호 — 비밀번호와 같이 가려 보인다
        let mut row = self.row(ui, row_rect(7));
        widgets::form_label(&mut row, crate::i18n::site_label_key_passphrase(), key);
        widgets::text_field(
            &mut row,
            "key_passphrase",
            &mut self.draft.key_passphrase,
            egui::vec2(field_width, widgets::FORM_FIELD_HEIGHT),
            key,
            true,
        );
    }

    /// 폼 한 행을 그릴 자리 — 라벨과 필드가 가로로 늘어선다
    fn row(&self, ui: &mut egui::Ui, rect: egui::Rect) -> egui::Ui {
        let mut row = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        row.spacing_mut().item_spacing.x = widgets::FORM_GAP;
        row
    }

    /// 바닥 버튼 줄 바로 위의 오류 문구 (인벤토리 #88).
    ///
    /// 버튼은 공통 셸이 그리므로 여기서는 사유 한 줄만 남긴다 — 오류가 없으면 빈 줄이다
    fn show_error_row(&self, ui: &egui::Ui, rect: egui::Rect) {
        let Some(error) = &self.error else {
            return;
        };
        // 본문과 같은 선에서 시작한다
        ui.painter().text(
            egui::pos2(rect.left() + BODY_PAD_X, rect.center().y),
            egui::Align2::LEFT_CENTER,
            error,
            egui::FontId::proportional(widgets::FORM_FONT_PX),
            theme::ERROR_TEXT,
        );
    }
}

/// 버튼 줄의 한 칸 — 비활성이면 글자가 흐려지고 눌리지 않는다. 눌렸으면 `true`.
///
/// 두 줄이 활성 조건만 다르고 그리는 방식이 같아 여기 모았다(윗줄 셋 · 아랫줄의
/// `새 사이트` · 아랫줄 나머지 둘 — 같은 여섯 줄이 세 자리에 되풀이되던 것이다)
fn button_cell(ui: &mut egui::Ui, label: &str, enabled: bool, width: f32) -> bool {
    ui.add_enabled_ui(enabled, |ui| {
        widgets::design_button(
            ui,
            label,
            if enabled {
                theme::TEXT_BUTTON
            } else {
                theme::TEXT_DIM
            },
            0.0,
            egui::vec2(width, GRID_BUTTON_HEIGHT),
        )
    })
    .inner
    .clicked()
}

/// 마우스 y가 줄들 사이 어디에 놓이는가 — `0..=rows.len()`.
/// 줄 가운데를 지났으면 그 줄 아래로 본다.
///
/// **`ui::tree`의 즐겨찾기 정렬과 같은 셈**이다 — 이름 바꾸는 중인 줄이 편집기로 바뀌어
/// 높이가 갈릴 수 있어, 고정 피치로 나누는 사이드바 방식이 아니라 실제 사각형으로 잰다
fn insert_index_at(y: f32, rows: &[egui::Rect]) -> usize {
    rows.iter().filter(|rect| y > rect.center().y).count()
}

/// 그 삽입 자리에 그을 가로선의 y — 줄이 하나도 없으면 `None`
fn insert_line_y(insert_at: usize, rows: &[egui::Rect]) -> Option<f32> {
    match insert_at {
        0 => Some(rows.first()?.top()),
        _ => Some(rows.get(insert_at - 1)?.bottom()),
    }
}

/// 목록의 한 줄 — 아이콘·이름. 클릭과 끌기를 함께 보므로 응답을 그대로 돌려준다 (`:396-404`)
fn show_site_row(
    ui: &mut egui::Ui,
    name: &str,
    dot: egui::Color32,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), LIST_ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let text_left = paint_row_icon(ui, rect, dot);
    let text = ui.painter().layout(
        name.to_owned(),
        egui::FontId::proportional(widgets::FORM_FONT_PX),
        theme::TEXT,
        (rect.right() - text_left - LIST_NAME_PAD_X * 2.0).max(0.0),
    );
    let color = if selected {
        // 고른 줄은 이름 뒤에만 강조가 깔린다 — 행 전체가 아니다 (`:403`)
        ui.painter().rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(text_left, rect.center().y - text.size().y / 2.0),
                text.size() + egui::vec2(LIST_NAME_PAD_X * 2.0, 0.0),
            ),
            0.0,
            SELECTED_BG,
        );
        SELECTED_FG
    } else {
        theme::TEXT
    };
    ui.painter().galley(
        egui::pos2(
            text_left + LIST_NAME_PAD_X,
            rect.center().y - text.size().y / 2.0,
        ),
        text,
        color,
    );
    response
}

/// 이름 바꾸는 중인 줄 — 편집이 끝났으면 `true` (Enter 또는 포커스를 잃었을 때).
/// `focus`는 편집기가 처음 뜬 프레임에만 참이다
fn show_rename_row(
    ui: &mut egui::Ui,
    name: &mut String,
    dot: egui::Color32,
    focus: bool,
) -> (egui::Rect, bool) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), LIST_ROW_HEIGHT),
        egui::Sense::hover(),
    );
    let text_left = paint_row_icon(ui, rect, dot);
    let edit_rect = egui::Rect::from_min_max(egui::pos2(text_left, rect.top()), rect.max);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(edit_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let response = child.add(
        egui::TextEdit::singleline(name)
            .id_salt("사이트 이름 바꾸기")
            .font(egui::FontId::proportional(widgets::FORM_FONT_PX))
            .desired_width(edit_rect.width()),
    );
    // 포커스는 처음 한 번만 청한다 — 매 프레임 청하면 `has_focus`가 늘 참이라
    // `lost_focus()`가 영영 거짓이 되고(egui `Memory::lost_focus`), 다른 곳을 눌러
    // 편집을 마치는 길이 통째로 막힌다
    if focus {
        response.request_focus();
        return (rect, false);
    }
    let done = response.lost_focus() || child.input(|input| input.key_pressed(egui::Key::Enter));
    (rect, done)
}

/// 행 왼쪽의 문서 아이콘 — 원본은 작은 사각형 조각으로 그린다 (`:397-402`).
/// 글꼴 글리프에 기대지 않으려고 여기서도 직접 그리며, 아래쪽 점만 연결 상태색이다.
/// 이름이 시작할 x를 돌려준다
fn paint_row_icon(ui: &egui::Ui, rect: egui::Rect, dot: egui::Color32) -> f32 {
    let icon = egui::Rect::from_min_size(
        egui::pos2(
            rect.left() + LIST_ROW_PAD_LEFT,
            rect.center().y - LIST_ICON / 2.0,
        ),
        egui::vec2(LIST_ICON, LIST_ICON),
    );
    let painter = ui.painter();
    painter.rect_filled(
        egui::Rect::from_min_size(icon.min + egui::vec2(3.0, 2.0), egui::vec2(10.0, 12.0)),
        1.0,
        egui::Color32::from_rgb(0x9A, 0xA3, 0xAA),
    );
    for offset in [4.0, 8.0] {
        painter.rect_filled(
            egui::Rect::from_min_size(icon.min + egui::vec2(5.0, offset), egui::vec2(6.0, 2.0)),
            0.0,
            theme::CONTROL_BG,
        );
    }
    painter.rect_filled(
        egui::Rect::from_min_size(icon.min + egui::vec2(5.0, 11.0), egui::vec2(2.0, 2.0)),
        0.0,
        dot,
    );
    icon.right() + LIST_ROW_GAP
}

/// 탭 본문의 한 줄 텍스트 — 폭을 넘으면 말줄임한다
fn text_row(ui: &egui::Ui, left: f32, top: f32, width: f32, text: &str, color: egui::Color32) {
    let galley = ui.painter().layout(
        text.to_owned(),
        egui::FontId::proportional(widgets::FORM_FONT_PX),
        color,
        width,
    );
    ui.painter().galley(egui::pos2(left, top), galley, color);
}

/// 선택지 표에서 지금 값의 표기를 찾는다 — 표에 없으면 첫 항목(있을 수 없는 경우의 안전값)
fn option_label<T: PartialEq>(options: &[(T, &'static str)], current: T) -> &'static str {
    options
        .iter()
        .find(|(value, _)| *value == current)
        .map(|(_, label)| *label)
        .unwrap_or(options[0].1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::manager::ConnectionManager;

    #[test]
    fn 문구는_인벤토리_원문_그대로다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 인벤토리 #60~75·#88~90 — 여기서 한 글자라도 다듬으면 화면과 명세가 갈린다
        assert_eq!(crate::i18n::site_title(), "사이트 관리자");
        assert_eq!(crate::i18n::site_list_label(), "사이트(S):");
        assert_eq!(crate::i18n::site_rename(), "이름 바꾸기(R)");
        assert_eq!(crate::i18n::site_delete(), "삭제(D)");
        assert_eq!(crate::i18n::site_duplicate(), "복제(I)");
        // 아래 셋은 **원본 인벤토리에 없는 항목**이다 — 앞 둘은 사용자 요청(2026-08-20)으로
        // 더한 내보내기·가져오기 버튼이고(FR-59), `새 사이트`는 원본이 **뺐던** 버튼을
        // 사용자 요청(2026-09-01)으로 되살린 것이다(FR-27). 원본과 갈린 사실을 여기 적어 둔다
        assert_eq!(crate::i18n::site_export(), "내보내기");
        assert_eq!(crate::i18n::site_import(), "가져오기");
        assert_eq!(crate::i18n::site_new(), "새 사이트");
        assert_eq!(crate::i18n::site_tab_general(), "일반");
        assert_eq!(crate::i18n::site_tab_transfer(), "전송 설정");
        assert_eq!(crate::i18n::site_tab_charset(), "문자셋");
        assert_eq!(crate::i18n::site_label_protocol(), "프로토콜(T):");
        assert_eq!(crate::i18n::site_label_host(), "호스트(H):");
        assert_eq!(crate::i18n::site_label_port(), "포트(P):");
        assert_eq!(crate::i18n::site_label_encryption(), "암호화(E):");
        assert_eq!(crate::i18n::site_label_logon(), "로그온 유형(L):");
        assert_eq!(crate::i18n::site_label_user(), "사용자(U):");
        assert_eq!(crate::i18n::site_label_password(), "비밀번호(W):");
        assert_eq!(crate::i18n::site_connect(), "연결(C)");
        assert_eq!(crate::i18n::site_ok(), "확인(O)");
        assert_eq!(crate::i18n::cancel(), "취소");
        // 드롭다운의 기본값 문구도 원본에 적힌 그대로다 (`:1011`·`:1013`·`:1014`)
        assert_eq!(protocol_options()[0].1, "FTP - 파일 전송 프로토콜");
        assert_eq!(
            encryption_options()[1].1,
            "TLS를 통한 명시적 FTP가 가능한 경우 사용"
        );
        assert_eq!(logon_options(Protocol::Ftp)[0].1, "일반");
        assert_eq!(logon_options(Protocol::Ftp)[1].1, "익명");
        assert_eq!(logon_options(Protocol::Sftp)[2].1, "키 파일");
    }

    #[test]
    fn 대화_치수는_원본과_같다() {
        // Acceptance ② — 1080×680 · 좌측 400px · 라벨 96px · 필드 28px.
        // 바닥은 원본의 58px 자리를 오류 줄 22px + 공통 셸의 버튼 줄 44px이 대신한다
        assert_eq!(DIALOG_WIDTH, 1080.0);
        assert_eq!(DIALOG_HEIGHT, 680.0);
        assert_eq!(LEFT_WIDTH, 400.0);
        assert_eq!(widgets::FORM_LABEL_WIDTH, 96.0);
        assert_eq!(widgets::FORM_FIELD_HEIGHT, 28.0);
        // 바닥은 오류 문구 줄과 공통 셸의 버튼 줄로 나뉜다 — 종전 58px 자리를 대신하며
        // 버튼이 커진 만큼(30 → 44) 8px 늘었다
        assert_eq!(ERROR_ROW_HEIGHT + dialog::FOOTER_HEIGHT, 66.0);
        // 헤더·목록 행·탭·버튼 3열도 원본 값이다
        assert_eq!(HEADER_HEIGHT, 40.0);
        assert_eq!(LIST_ROW_HEIGHT, 24.0);
        assert_eq!(TAB_HEIGHT, 28.0);
        assert_eq!(GRID_BUTTON_HEIGHT, 28.0);
    }

    #[test]
    fn 프로토콜을_바꾸면_포트가_따라간다() {
        // Acceptance ③ — FTP/FTPS 21, SFTP 22
        let mut draft = Draft::default();
        assert_eq!(draft.port, "21");
        draft.set_protocol(Protocol::Sftp);
        assert_eq!(draft.port, "22");
        draft.set_protocol(Protocol::Ftps);
        assert_eq!(draft.port, "21");
    }

    #[test]
    fn 사용자가_고친_포트는_프로토콜을_바꿔도_지켜진다() {
        // Acceptance ③ 뒷문장 — 적어 둔 값이 사라지면 엉뚱한 곳에 연결된다
        let mut draft = Draft {
            port: "2121".to_owned(),
            port_edited: true,
            ..Draft::default()
        };
        draft.set_protocol(Protocol::Sftp);
        assert_eq!(draft.port, "2121");
    }

    #[test]
    fn 포트는_범위로_클램프된다() {
        // plan Edge Case — 0·65536 초과는 실제 포트가 아니다
        assert_eq!(parse_port("0", Protocol::Ftp), 1);
        assert_eq!(parse_port("70000", Protocol::Ftp), 65535);
        assert_eq!(parse_port("2222", Protocol::Sftp), 2222);
        // 적는 도중의 빈 값·문자는 프로토콜 기본값으로 되돌린다
        assert_eq!(parse_port("", Protocol::Sftp), 22);
        assert_eq!(parse_port("포트", Protocol::Ftp), 21);
        assert_eq!(parse_port(" 21 ", Protocol::Ftp), 21);
    }

    #[test]
    fn 로그온_유형이_세_필드의_활성을_각자_정한다() {
        // Acceptance 진리표 (인벤토리 #74·#75 + FR-66) — 종전의 `credentials_enabled` 하나로는
        // 「키 파일 → 사용자 활성 · 비밀번호 잠김」을 만들 수 없어 셋으로 갈랐다 (D9)
        let mut draft = Draft {
            protocol: Protocol::Sftp,
            ..Draft::default()
        };

        // 일반 — 사용자·비밀번호만
        assert!(draft.user_enabled(), "기본값은 `일반`이다");
        assert!(draft.password_enabled());
        assert!(!draft.key_enabled());

        // 키 파일 — 사용자는 그대로 열리고 비밀번호가 잠기며 키 두 칸이 열린다
        draft.logon = LogonType::KeyFile;
        assert!(draft.user_enabled(), "키 파일도 사용자 이름은 직접 준다");
        assert!(!draft.password_enabled());
        assert!(draft.key_enabled());

        // 익명 — 셋 다 잠긴다
        draft.logon = LogonType::Anonymous;
        assert!(!draft.user_enabled());
        assert!(!draft.password_enabled());
        assert!(!draft.key_enabled());
    }

    #[test]
    fn 키_파일은_sftp에서만_고를_수_있다() {
        // D7 — FTP 계열에 없는 개념이라 세워 두면 「고를 수 있는데 반드시 실패하는 값」이 된다
        assert_eq!(logon_options(Protocol::Ftp).len(), 2);
        assert_eq!(logon_options(Protocol::Ftps).len(), 2);
        let ssh = logon_options(Protocol::Sftp);
        assert_eq!(ssh.len(), 3);
        assert_eq!(ssh[2].0, LogonType::KeyFile);
    }

    #[test]
    fn 프로토콜을_ftp로_바꾸면_키_파일_유형이_되돌려진다() {
        // D7 — 고를 수 없는 값이 초안에 남으면 목록에 없는 것이 골라진 상태가 된다
        let mut draft = Draft::default();
        draft.set_protocol(Protocol::Sftp);
        draft.logon = LogonType::KeyFile;
        draft.set_protocol(Protocol::Ftp);
        assert_eq!(draft.logon, LogonType::Normal);
    }

    #[test]
    fn ssh는_암호화를_고를_것이_없다() {
        // 인벤토리 #72 — 암호화 항목은 FTP 계열일 때만 뜻이 있다
        let mut draft = Draft::default();
        assert!(draft.encryption_enabled());
        draft.protocol = Protocol::Sftp;
        assert!(!draft.encryption_enabled());
    }

    /// 사이트 하나가 든 저장소와 그것을 고른 관리자
    fn manager_with_site() -> (SiteManager, SiteStore, SiteId) {
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.select(&store, id);
        (manager, store, id)
    }

    #[test]
    fn 설정을_고치러_열면_첫_사이트가_골라져_있다() {
        // 인벤토리 #62 기본값 — 원본도 `selectedSite: 0`으로 뜬다
        let mut store = SiteStore::new();
        let first = store.add("배포 서버");
        let second = store.add("스테이징");
        if let Some(record) = store.get_mut(second) {
            record.host = "staging.test".to_owned();
        }
        let mut manager = SiteManager::new();

        manager.open(&store, None);
        assert_eq!(manager.selected, Some(first));

        // 지정한 사이트가 있으면 그것을 고른다 (실패 화면의 `설정 열기` — 인벤토리 #19)
        manager.open(&store, Some(second));
        assert_eq!(manager.selected, Some(second));
        assert_eq!(manager.draft.host, "staging.test");

        // 그 사이 지워진 사이트를 지정하면 첫 항목으로 물러선다
        manager.open(&store, Some(SiteId(99)));
        assert_eq!(manager.selected, Some(first));

        // 등록된 사이트가 없으면 새 초안이다
        manager.open(&SiteStore::new(), None);
        assert_eq!(manager.selected, None);
        assert_eq!(manager.draft, Draft::default());
    }

    #[test]
    fn 새_사이트_추가는_아무것도_고르지_않고_연다() {
        // 인벤토리 #8 — 여기서 기존 사이트를 골라 두면 `확인(O)`이 그것을 덮어쓴다
        let mut store = SiteStore::new();
        store.add("배포 서버");
        let mut manager = SiteManager::new();
        manager.open_new();
        assert!(manager.is_open());
        assert_eq!(manager.selected, None);
    }

    #[test]
    fn 이름_바꾸기_삭제_복제가_저장소에_반영된다() {
        // Acceptance ⑤
        let (mut manager, mut store, id) = manager_with_site();

        manager.apply_list_action(ListAction::StartRename, &mut store);
        assert_eq!(manager.renaming.as_deref(), Some("배포 서버"));
        manager.renaming = Some("스테이징".to_owned());
        manager.finish_rename(&mut store);
        assert_eq!(store.get(id).expect("사이트").name, "스테이징");

        manager.apply_list_action(ListAction::Duplicate, &mut store);
        assert_eq!(store.sites().len(), 2);
        let copy = manager.selected.expect("복제본이 선택된다");
        assert_ne!(copy, id);
        assert_eq!(store.get(copy).expect("사본").name, "스테이징 (2)");

        // 삭제는 곧바로 지우지 않고 **묻는 상태로만** 들어간다 (2026-08-16 검토)
        manager.apply_list_action(ListAction::Delete, &mut store);
        assert_eq!(manager.pending_delete, Some(copy));
        assert!(store.get(copy).is_some(), "묻기도 전에 지워졌다");
    }

    #[test]
    fn 이름을_고치다_대화를_닫아도_저장된다() {
        // Enter를 누르지 않고 `확인(O)`·`연결(C)`·`X`로 끝내는 길 — 편집 중이던 이름이
        // 버려지면 사용자가 보기에는 "이름을 바꿔도 저장되지 않는" 것이 된다
        let (mut manager, mut store, id) = manager_with_site();

        manager.apply_list_action(ListAction::StartRename, &mut store);
        manager.renaming = Some("스테이징".to_owned());
        manager.close(&mut store);

        assert_eq!(store.get(id).expect("사이트").name, "스테이징");
        assert_eq!(manager.renaming, None, "닫은 뒤에는 편집이 남지 않는다");
    }

    #[test]
    fn 편집기_밖을_누르면_이름이_확정된다() {
        // 포커스를 매 프레임 청하면 `lost_focus()`가 영영 거짓이 되어(egui `Memory::lost_focus`)
        // 다른 곳을 눌러 편집을 마치는 길이 막힌다. 두 프레임을 그려 그 길을 지킨다
        let (mut manager, mut store, id) = manager_with_site();
        manager.apply_list_action(ListAction::StartRename, &mut store);
        manager.renaming = Some("스테이징".to_owned());

        // 첫 프레임 — 편집기가 뜨며 포커스를 청한다. 아직 확정하지 않는다
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });
        assert_eq!(
            store.get(id).expect("사이트").name,
            "배포 서버",
            "편집기가 막 떴을 뿐인데 확정됐다"
        );
        assert!(manager.renaming.is_some(), "편집이 이어져야 한다");

        // 둘째 프레임 — 사용자가 편집기 밖을 누른다. 포커스는 **프레임 도중**에 옮겨가고,
        // egui는 그 이전을 두 프레임 창(`id_two_frames_ago`)으로 알아챈다
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
            ui.ctx()
                .memory_mut(|memory| memory.request_focus(egui::Id::new("편집기 밖")));
        });
        // 셋째 프레임 — 편집기가 포커스를 잃은 것을 알아채고 이름을 확정한다
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });

        assert_eq!(
            store.get(id).expect("사이트").name,
            "스테이징",
            "포커스를 잃으면 이름이 확정된다"
        );
        assert_eq!(manager.renaming, None, "확정한 뒤에는 편집이 남지 않는다");
    }

    #[test]
    fn 고른_사이트가_없으면_세_버튼이_아무_일도_하지_않는다() {
        // plan Edge Case — 사이트 0개에서 `삭제(D)`
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        for action in [
            ListAction::StartRename,
            ListAction::Delete,
            ListAction::Duplicate,
        ] {
            manager.apply_list_action(action, &mut store);
        }
        assert!(store.is_empty());
        assert_eq!(manager.renaming, None);
    }

    #[test]
    fn 새_사이트는_고른_것이_없어도_목록에_항목을_더한다() {
        // FR-27 — 세 버튼과 달리 `새 사이트`만은 선택 가드보다 앞에서 갈린다.
        // 이름이 겹치면 `SiteStore`가 `(2)`를 붙인다
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        assert_eq!(manager.selected, None, "빈 초안으로 열린다");

        manager.apply_list_action(ListAction::New, &mut store);
        assert_eq!(store.sites().len(), 1);
        assert_eq!(store.sites()[0].name, "새 사이트");
        assert_eq!(
            manager.selected,
            Some(store.sites()[0].id),
            "그것이 골라진다"
        );

        manager.apply_list_action(ListAction::New, &mut store);
        assert_eq!(store.sites().len(), 2);
        assert_eq!(store.sites()[1].name, "새 사이트 (2)");
        assert_eq!(
            manager.selected,
            Some(store.sites()[1].id),
            "나중 것이 골라진다"
        );

        // **그 경로가 실제로 빈 호스트를 낳는다** — 주소를 적어야 등록이 끝난다.
        // 고르는 자리(사이드바·연결 메뉴·새 탭 메뉴)에도 서지 않는다
        assert!(manager.draft.host.is_empty());
        assert_eq!(manager.commit(&mut store), None, "호스트가 비어 거부된다");
        assert_eq!(
            manager.error.as_deref(),
            Some(crate::i18n::site_error_no_host())
        );
        assert_eq!(store.visible().count(), 0);
    }

    #[test]
    fn 삽입_자리는_줄_가운데를_기준으로_갈린다() {
        // 줄 셋을 20px 높이로 세운다 (`ui::tree`의 즐겨찾기 정렬과 같은 셈)
        let rows: Vec<egui::Rect> = (0..3)
            .map(|i| {
                egui::Rect::from_min_size(
                    egui::pos2(0.0, 100.0 + i as f32 * 20.0),
                    egui::vec2(200.0, 20.0),
                )
            })
            .collect();

        assert_eq!(insert_index_at(105.0, &rows), 0, "첫 줄 위쪽 절반 → 맨 앞");
        assert_eq!(
            insert_index_at(115.0, &rows),
            1,
            "첫 줄 가운데를 지나면 그 아래"
        );
        assert_eq!(insert_index_at(135.0, &rows), 2, "가운데 줄 아래쪽 절반");
        assert_eq!(insert_index_at(9999.0, &rows), 3, "마지막 줄 아래 → 맨 끝");
        assert_eq!(
            insert_index_at(0.0, &rows),
            0,
            "웰 위로 벗어나도 맨 앞으로 잘린다"
        );
        // 줄이 없으면 그을 선도 없다
        assert_eq!(insert_index_at(100.0, &[]), 0);
        assert_eq!(insert_line_y(0, &[]), None);
        // 선은 그 자리 앞줄의 아래끝(또는 첫 줄의 위끝)에 그어진다
        assert_eq!(insert_line_y(0, &rows), Some(100.0));
        assert_eq!(insert_line_y(3, &rows), Some(160.0));
    }

    #[test]
    fn 끌어_놓으면_차례가_바뀌고_고른_사이트는_그대로다() {
        // FR-27 — 자리가 바뀌어도 `selected`는 `SiteId`라 가리키는 곳이 흔들리지 않는다
        let mut store = SiteStore::new();
        let ids: Vec<SiteId> = ["첫째", "둘째", "셋째"]
            .into_iter()
            .map(|name| {
                let id = store.add(name);
                if let Some(site) = store.get_mut(id) {
                    site.host = "example.test".to_owned();
                }
                id
            })
            .collect();
        let mut manager = SiteManager::new();
        manager.open(&store, Some(ids[0]));
        assert_eq!(manager.selected, Some(ids[0]));

        // 첫째를 맨 끝으로 끈다 — 삽입 자리 3 → 목적지 2
        let to = widgets::reorder_target(0, 3).expect("자리가 바뀐다");
        manager.apply_list_action(ListAction::Reorder(0, to), &mut store);
        let names: Vec<&str> = store.sites().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["둘째", "셋째", "첫째"]);
        assert_eq!(manager.selected, Some(ids[0]), "고른 사이트는 그대로다");
        // 그 차례가 곧 고르는 자리에 서는 차례다
        let visible: Vec<&str> = store.visible().map(|s| s.name.as_str()).collect();
        assert_eq!(visible, vec!["둘째", "셋째", "첫째"]);
    }

    #[test]
    fn 제자리에_놓거나_사이트가_하나뿐이면_차례가_그대로다() {
        // 자기 앞·자기 뒤로 놓는 것은 바꿀 것이 없다
        assert_eq!(widgets::reorder_target(1, 1), None, "자기 앞");
        assert_eq!(widgets::reorder_target(1, 2), None, "자기 뒤");
        // 사이트가 하나면 어디에 놓아도 제자리다
        assert_eq!(widgets::reorder_target(0, 0), None);
        assert_eq!(widgets::reorder_target(0, 1), None);
    }

    #[test]
    fn 임계를_못_넘은_끌기는_차례를_바꾸지_않는다() {
        // 그 전에는 클릭이라 선택만 일어난다. 임계 값은 `ui::widgets`가 하나로 갖는다
        let mut manager = SiteManager::new();
        manager.drag = Some(SiteDrag {
            from: 0,
            start: egui::pos2(10.0, 10.0),
            active: false,
        });
        let ctx = egui::Context::default();
        let rows = vec![egui::Rect::from_min_size(
            egui::pos2(0.0, 100.0),
            egui::vec2(200.0, 20.0),
        )];
        let mut action = None;
        let _ = ctx.run_ui(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                action = manager.finish_site_drag(ui, &rows);
            });
        });
        assert_eq!(action, None, "임계를 못 넘었으면 조작이 없다");
    }

    #[test]
    fn 버튼_두_줄은_목록_웰과_좌우가_같다() {
        // 요구 ④ — 원본의 좌우 여백 30px을 걷어냈다. 화면 없이 사각형 값으로 잰다
        let manager = SiteManager::new();
        let column = egui::Rect::from_min_size(
            egui::pos2(18.0, 46.0),
            egui::vec2(LEFT_WIDTH, 574.0 - BODY_PAD_TOP),
        );
        let well = manager.well_rect(column);
        for (이름, grid) in [
            (
                "윗줄",
                manager.button_grid(column, manager.buttons_top(column)),
            ),
            (
                "아랫줄",
                manager.button_grid(column, manager.bottom_buttons_top(column)),
            ),
        ] {
            assert_eq!(grid.left(), well.left(), "{이름} 왼쪽이 웰과 어긋난다");
            assert_eq!(grid.right(), well.right(), "{이름} 오른쪽이 웰과 어긋난다");
        }
        // 세 칸이 간격 둘을 빼고 균등하게 나눠 갖는다
        let grid = manager.button_grid(column, manager.buttons_top(column));
        let width = SiteManager::button_width(grid);
        assert!((width * 3.0 + GRID_GAP * 2.0 - grid.width()).abs() < 0.01);
    }

    #[test]
    fn 호스트가_비면_등록을_거부하고_까닭을_남긴다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // plan Edge Case — 조용히 실패하면 사용자는 등록된 줄 안다
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.host = "   ".to_owned();
        assert_eq!(manager.commit(&mut store), None);
        assert_eq!(
            manager.error.as_deref(),
            Some("호스트 주소를 입력해야 등록할 수 있습니다.")
        );
        assert!(store.is_empty(), "거부했는데 사이트가 생겼다");
    }

    #[test]
    fn 등록하면_초안이_사이트가_되고_비밀번호는_봉인된다() {
        // Acceptance ⑤·⑥의 저장 쪽 — 이름은 호스트로 잡고 평문은 남지 않는다 (FR-28)
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.host = "example.test".to_owned();
        manager.draft.port = "2222".to_owned();
        manager.draft.protocol = Protocol::Sftp;
        manager.draft.user = "deploy".to_owned();
        manager.draft.password = "초안비밀번호".to_owned();

        let id = manager.commit(&mut store).expect("등록");
        let record = store.get(id).expect("사이트");
        assert_eq!(record.name, "example.test");
        assert_eq!(record.host, "example.test");
        assert_eq!(record.port, 2222);
        assert_eq!(record.protocol, Protocol::Sftp);
        assert_eq!(record.user, "deploy");
        let json = serde_json::to_string(&store).expect("직렬화");
        assert!(!json.contains("초안비밀번호"), "평문이 남았다");
        assert_eq!(store.password(id).as_deref(), Some("초안비밀번호"));
        // 등록한 사이트는 사이드바에도 보인다
        assert!(!store.is_hidden(id));
    }

    #[test]
    fn 키_파일로_등록하면_경로와_암호가_담긴다() {
        // Acceptance ⓓ (FR-66)
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.set_protocol(Protocol::Sftp);
        manager.draft.host = "key.test".to_owned();
        manager.draft.user = "deploy".to_owned();
        manager.draft.logon = LogonType::KeyFile;
        manager.draft.key_path = r"C:\keys\id_ed25519".to_owned();
        manager.draft.key_passphrase = "키암호".to_owned();

        let id = manager.commit(&mut store).expect("등록");
        assert_eq!(
            store.get(id).expect("사이트").key_path,
            Some(std::path::PathBuf::from(r"C:\keys\id_ed25519"))
        );
        assert_eq!(store.key_passphrase(id).as_deref(), Some("키암호"));
    }

    #[test]
    fn 저장된_키_암호와_경로는_편집_상태로_따라온다() {
        // Acceptance ⓔ — 되싣지 않으면 편집만 하고 저장해도 담아 둔 암호가 지워진다
        let mut store = SiteStore::new();
        let id = store.add("키 서버");
        if let Some(record) = store.get_mut(id) {
            record.protocol = Protocol::Sftp;
            record.logon = LogonType::KeyFile;
            record.key_path = Some(std::path::PathBuf::from(r"C:\keys\id_ed25519"));
        }
        assert!(store.set_key_passphrase(id, "담아둔키암호"));

        let mut manager = SiteManager::new();
        manager.open_new();
        manager.select(&store, id);
        assert_eq!(manager.draft.key_path, r"C:\keys\id_ed25519");
        assert_eq!(manager.draft.key_passphrase, "담아둔키암호");

        // 키 값은 그대로 두고 등록에 필요한 주소만 채운다 — 그래도 값이 살아 있어야 한다
        manager.draft.host = "key.test".to_owned();
        let same = manager.commit(&mut store).expect("등록");
        assert_eq!(store.key_passphrase(same).as_deref(), Some("담아둔키암호"));
    }

    #[test]
    fn 키_파일로_바꿔_저장하면_담아_둔_비밀번호가_지워진다() {
        // Acceptance ⓕ (D9) — 익명으로 바꿀 때와 같은 규칙. 쓰지도 않는 비밀을 남기지 않는다
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        if let Some(record) = store.get_mut(id) {
            record.protocol = Protocol::Sftp;
            record.logon = LogonType::Normal;
        }
        assert!(store.set_password(id, "쓰지않게될비밀번호"));

        let mut manager = SiteManager::new();
        manager.open_new();
        manager.select(&store, id);
        manager.draft.host = "key.test".to_owned();
        manager.draft.logon = LogonType::KeyFile;
        manager.draft.key_path = r"C:\keys\id_ed25519".to_owned();

        let same = manager.commit(&mut store).expect("등록");
        assert_eq!(store.password(same), None, "쓰지 않는 비밀번호가 남았다");
    }

    #[test]
    fn 키_파일이_아니면_담아_둔_키_값이_지워진다() {
        // 위 규칙의 거울상 — 비밀번호 인증으로 되돌리면 키 경로·암호가 남지 않는다
        let mut store = SiteStore::new();
        let id = store.add("키 서버");
        if let Some(record) = store.get_mut(id) {
            record.protocol = Protocol::Sftp;
            record.logon = LogonType::KeyFile;
            record.key_path = Some(std::path::PathBuf::from(r"C:\keys\id_ed25519"));
        }
        assert!(store.set_key_passphrase(id, "지워질키암호"));

        let mut manager = SiteManager::new();
        manager.open_new();
        manager.select(&store, id);
        manager.draft.host = "key.test".to_owned();
        manager.draft.logon = LogonType::Normal;

        let same = manager.commit(&mut store).expect("등록");
        assert_eq!(store.get(same).expect("사이트").key_path, None);
        assert_eq!(store.key_passphrase(same), None);
    }

    #[test]
    fn 익명으로_등록하면_비밀번호를_담지_않는다() {
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.host = "example.test".to_owned();
        manager.draft.password = "쓰지않을값".to_owned();
        manager.draft.logon = LogonType::Anonymous;

        let id = manager.commit(&mut store).expect("등록");
        assert_eq!(store.password(id), None);
    }

    #[test]
    fn 고른_사이트를_다시_등록하면_새로_만들지_않는다() {
        let (mut manager, mut store, id) = manager_with_site();
        manager.draft.host = "example.test".to_owned();
        assert_eq!(manager.commit(&mut store), Some(id));
        assert_eq!(store.sites().len(), 1, "같은 사이트가 둘이 됐다");
        assert_eq!(
            store.get(id).expect("사이트").name,
            "배포 서버",
            "이름은 목록이 정한다 — 등록이 호스트로 덮어쓰지 않는다"
        );
    }

    #[test]
    fn 저장된_비밀번호는_편집_상태로_따라온다() {
        // 빈칸으로 보이면 사용자는 저장된 적이 없다고 읽고, 그대로 등록하면 지워진다
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        assert!(store.set_password(id, "저장된비밀번호"));
        let draft = Draft::load(&store, id).expect("초안");
        assert_eq!(draft.password, "저장된비밀번호");
    }

    #[test]
    fn 대화를_닫으면_초안이_사라진다() {
        // plan Edge Case — `Esc`로 닫으면 초안 폐기. 평문 비밀번호도 여기서 함께 버려진다
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.host = "example.test".to_owned();
        manager.draft.password = "버려질값".to_owned();
        manager.close(&mut SiteStore::new());
        assert!(!manager.is_open());
        assert_eq!(manager.draft, Draft::default());
    }

    #[test]
    fn 두_번째_세_번째_탭_문구도_인벤토리_원문_그대로다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // 인벤토리 #76~87 (원본 `:442`·`:852`·`:454`·`:457`·`:471-472`·`:867`·`:481`·`:484`)
        assert_eq!(crate::i18n::site_label_transfer_mode(), "전송 모드(T):");
        assert_eq!(transfer_options()[0].1, "기본(E)");
        assert_eq!(transfer_options()[1].1, "능동형(A)");
        assert_eq!(transfer_options()[2].1, "수동형(P)");
        assert_eq!(crate::i18n::site_label_limit(), "동시 연결 수 제한(L)");
        assert_eq!(
            crate::i18n::site_label_limit_value(),
            "최대 동시 연결 수(M):"
        );
        assert_eq!(
            crate::i18n::site_charset_heading(),
            "서버에서 파일명에 사용하는 문자셋"
        );
        assert_eq!(crate::i18n::site_charset_label(), "인코딩:");
        assert_eq!(charset_options()[0], "UTF-8(U)");
        assert_eq!(charset_options()[1], "문자셋 직접 설정(C)");
        assert_eq!(crate::i18n::site_label_encoding(), "인코딩(E):");
        assert_eq!(
            crate::i18n::site_charset_warning(),
            "문자셋을 잘못 지정하면 파일 이름이 깨져 보일 수 있습니다."
        );
        // 인코딩 필드 폭·높이도 원본 값이다 (`:482`)
        assert_eq!(ENCODING_WIDTH, 210.0);
        assert_eq!(ENCODING_HEIGHT, 26.0);
    }

    #[test]
    fn 제한을_켜야_최대_연결_수를_고칠_수_있다() {
        // Acceptance ② (인벤토리 #81) — 꺼져 있으면 흐림 + 조작 불가
        let mut draft = Draft::default();
        assert!(!draft.limit_enabled());
        assert_eq!(draft.connection_limit(), None, "꺼져 있으면 제한 없음이다");
        draft.limit_on = true;
        draft.limit = 4;
        assert!(draft.limit_enabled());
        assert_eq!(draft.connection_limit(), Some(4));
    }

    #[test]
    fn 직접_설정을_골라야_인코딩을_적을_수_있다() {
        // Acceptance ③ (인벤토리 #86)
        let mut draft = Draft::default();
        assert!(!draft.encoding_enabled());
        assert_eq!(draft.charset(), Charset::Utf8);
        draft.charset_custom = true;
        draft.encoding = "  CP949  ".to_owned();
        assert!(draft.encoding_enabled());
        // 앞뒤 공백은 떼고 담는다 — 이름 대조에 걸리면 알아듣지 못한다
        assert_eq!(draft.charset(), Charset::Named("CP949".to_owned()));
    }

    #[test]
    fn 스피너_값은_범위를_벗어나지_않는다() {
        // Acceptance ④ — 1~10
        let mut draft = Draft {
            limit_on: true,
            limit: 200,
            ..Draft::default()
        };
        assert_eq!(draft.connection_limit(), Some(10));
        draft.limit = 0;
        assert_eq!(draft.connection_limit(), Some(1));
    }

    #[test]
    fn 설정한_최대_연결_수가_채널_배정에_그대로_닿는다() {
        // Acceptance ⑤ (FR-45) — 화면 값이 `SiteRecord`를 거쳐 연결 관리자에 닿는 경로를 고정한다.
        // 이 사슬이 끊기면 UI만 있고 동작은 예전 그대로가 된다
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.host = "example.test".to_owned();
        manager.draft.limit_on = true;
        manager.draft.limit = 2;
        manager.draft.transfer_mode = TransferMode::Passive;
        manager.draft.charset_custom = true;
        manager.draft.encoding = "CP949".to_owned();

        let id = manager.commit(&mut store).expect("등록");
        let record = store.get(id).expect("사이트").clone();
        assert_eq!(record.connection_limit, Some(2));
        assert_eq!(record.transfer_mode, TransferMode::Passive);
        assert_eq!(record.charset, Charset::Named("CP949".to_owned()));

        // 상한 2 = 탐색 1 + 전송 1 (D4)
        let connections = ConnectionManager::new(std::sync::Arc::new(|| {}));
        assert_eq!(connections.transfer_slots(&record), 1);
    }

    #[test]
    fn 연결된_서버의_전송_모드를_바꾸면_안내가_뜬다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        // plan Edge Case — 지금 연결에 바로 듣지 않는다는 것을 알리지 않으면
        // 사용자는 바꾼 설정이 곧바로 듣는 줄 알고 같은 실패를 다시 겪는다
        assert_eq!(
            crate::i18n::site_transfer_mode_notice(),
            "이미 연결된 서버입니다. 바꾼 전송 모드는 다음 연결부터 적용됩니다."
        );
        let (mut manager, mut store, id) = manager_with_site();
        manager.tab = ManagerTab::Transfer;
        manager.draft.transfer_mode = TransferMode::Passive;

        // 연결이 없으면 알릴 것이 없다 — 다음 연결이 곧 첫 연결이다
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[]);
        });
        // 연결이 있으면 안내가 그려진다(그리기 경로가 패닉 없이 도는지까지 함께 본다)
        let _ = ctx.run_ui(Default::default(), |ui| {
            manager.show(ui.ctx(), &mut store, &[id]);
        });
        assert!(manager.is_open());
    }

    #[test]
    fn 편집_상태로_불러오면_두_탭_값도_따라온다() {
        let mut store = SiteStore::new();
        let id = store.add("배포 서버");
        if let Some(record) = store.get_mut(id) {
            record.host = "example.test".to_owned();
            record.connection_limit = Some(5);
            record.transfer_mode = TransferMode::Active;
            record.charset = Charset::Named("EUC-KR".to_owned());
        }
        let draft = Draft::load(&store, id).expect("초안");
        assert!(draft.limit_enabled());
        assert_eq!(draft.limit, 5);
        assert_eq!(draft.transfer_mode, TransferMode::Active);
        assert!(draft.encoding_enabled());
        assert_eq!(draft.encoding, "EUC-KR");
    }

    #[test]
    fn 대화가_한_프레임을_그린다() {
        // 그리기 경로 전체(헤더·목록·버튼·탭·폼·바닥)가 패닉 없이 도는지 본다 —
        // 치수 단언만으로는 자리 계산의 뒤집힌 사각형을 잡지 못한다
        let (mut manager, mut store, id) = manager_with_site();
        manager.renaming = None;
        let ctx = egui::Context::default();
        let mut outcome = SiteManagerOutcome::Register(id);
        let _ = ctx.run_ui(Default::default(), |ui| {
            outcome = manager.show(ui.ctx(), &mut store, &[id]);
        });
        assert_eq!(outcome, SiteManagerOutcome::None, "아무도 누르지 않았다");
        assert!(manager.is_open(), "저절로 닫혔다");

        // 다른 두 탭(T16이 채운다)으로 옮겨도 그리기가 선다
        for tab in [ManagerTab::Transfer, ManagerTab::Charset] {
            manager.tab = tab;
            let _ = ctx.run_ui(Default::default(), |ui| {
                manager.show(ui.ctx(), &mut store, &[]);
            });
        }
    }
}
