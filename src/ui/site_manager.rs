//! 사이트 관리자 대화 — 목록과 `일반` 탭 (FR-27).
//!
//! 원본 `FileExplorer-FTP.dc.html:384-499`. 대화는 1080×680 고정이고 헤더(40px)·본문·바닥(58px)
//! 세 층으로 나뉜다. 본문은 좌측 400px(사이트 목록 + 버튼 3개)과 우측 가변(탭 + 폼)이다.
//!
//! **조작은 값으로 돌려주고 연결·토스트는 여기서 하지 않는다** — 기존 화면 규약과 같다.
//! 다만 사이트 목록 자체의 변경(이름 바꾸기·삭제·복제·등록)은 `SiteStore`를 직접 고친다:
//! 평문 비밀번호를 봉인해 담을 수 있는 곳이 `SiteStore::set_password`뿐이라(FR-28),
//! 초안을 값으로 넘기면 봉인 경로가 화면 쪽에 한 벌 더 생긴다.
use crate::remote::charset;
use crate::remote::sites::SiteStore;
use crate::remote::types::{
    CONNECTION_LIMIT_RANGE, Charset, Encryption, LogonType, Protocol, SiteId, TransferMode,
};
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui;

// ── 대화 치수 (원본 `FileExplorer-FTP.dc.html`, plan 시각 속성 표) ──
/// 대화 크기 — 고정이다(`:385`)
const DIALOG_WIDTH: f32 = 1080.0;
const DIALOG_HEIGHT: f32 = 680.0;
/// 스크림 — 원본 `rgba(0,0,0,.55)` (`:384`)
const SCRIM_ALPHA: u8 = 140;
/// 그림자 `0 18px 60px rgba(0,0,0,.6)` (`:385`)
const SHADOW_OFFSET_Y: i8 = 18;
const SHADOW_BLUR: u8 = 60;
const SHADOW_ALPHA: u8 = 153;

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

/// 좌측 버튼 3열 — `grid 1fr 1fr 1fr` gap 8px · `padding 2px 30px 6px` · 28px (`:407-409`)
const GRID_GAP: f32 = 8.0;
const GRID_PAD_X: f32 = 30.0;
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

/// 바닥 — 58px · 위 테두리 `#2C2C2C` · 우측 정렬 gap 10px · 버튼 30px `padding 0 24px` (`:495-498`)
const FOOTER_HEIGHT: f32 = 58.0;
const FOOTER_PAD_X: f32 = 18.0;
const FOOTER_GAP: f32 = 10.0;
const FOOTER_BUTTON_HEIGHT: f32 = 30.0;
const FOOTER_BUTTON_PAD_X: f32 = 24.0;

// ── 문구 (인벤토리 #60~75·#88~90 — 원문 그대로다) ──
const TITLE: &str = "사이트 관리자";
const LIST_LABEL: &str = "항목 선택(S):";
const BUTTON_RENAME: &str = "이름 바꾸기(R)";
const BUTTON_DELETE: &str = "삭제(D)";
const BUTTON_DUPLICATE: &str = "복제(I)";
const TAB_GENERAL: &str = "일반";
const TAB_TRANSFER: &str = "전송 설정";
const TAB_CHARSET: &str = "문자셋";
const LABEL_PROTOCOL: &str = "프로토콜(T):";
const LABEL_HOST: &str = "호스트(H):";
const LABEL_PORT: &str = "포트(P):";
const LABEL_ENCRYPTION: &str = "암호화(E):";
const LABEL_LOGON: &str = "로그온 유형(L):";
const LABEL_USER: &str = "사용자(U):";
const LABEL_PASSWORD: &str = "비밀번호(W):";
const FOOTER_CONNECT: &str = "연결(C)";
const FOOTER_OK: &str = "확인(O)";
const FOOTER_CANCEL: &str = "취소";
// 전송 설정 탭 (인벤토리 #76~81)
const LABEL_TRANSFER_MODE: &str = "전송 모드(T):";
const LABEL_LIMIT: &str = "동시 연결 수 제한(L)";
const LABEL_LIMIT_VALUE: &str = "최대 동시 연결 수(M):";
// 문자셋 탭 (인벤토리 #82~87)
const CHARSET_HEADING: &str = "서버에서 파일명에 사용하는 문자셋";
const CHARSET_LABEL: &str = "인코딩:";
const LABEL_ENCODING: &str = "인코딩(E):";
const CHARSET_FOOTNOTE: &str =
    "문자셋을 잘못 지정하면 파일명이 올바르게 보여지지 않을 수 있습니다.";
/// 이미 연결된 서버의 전송 모드를 바꿨을 때 (plan Edge Case) — 지금 연결에 바로 듣지 않는다는
/// 것을 알리지 않으면, 사용자는 바꾼 설정이 곧바로 듣는 줄 알고 같은 실패를 다시 겪는다
const TRANSFER_APPLY_HINT: &str =
    "이미 연결된 서버입니다. 바꾼 전송 모드는 다음 연결부터 적용됩니다.";
/// 알아듣지 못하는 인코딩 이름을 적었을 때 (plan Edge Case) — 조용히 UTF-8로 처리하면
/// 파일명이 깨진 채로 굳는다. 상태 표시줄은 아직 없어(T21) 이 자리에서 알린다
const CHARSET_UNKNOWN_HINT: &str = "이 이름은 알지 못해 UTF-8로 처리합니다.";
/// 호스트를 비운 채 등록하려 할 때 (plan Edge Case) — 무엇을 해야 하는지까지 알린다
const ERROR_NO_HOST: &str = "호스트 주소를 입력해야 등록할 수 있습니다.";
/// 비밀번호 봉인이 실패했을 때 — 평문으로 대신 담지 않는다 (FR-28)
const ERROR_PASSWORD: &str = "비밀번호를 저장하지 못했습니다. 연결할 때 다시 입력해 주세요.";

/// 프로토콜 선택지 — 첫 항목의 문구는 원본 그대로다 (인벤토리 #69, `:1011`).
/// 나머지 둘은 원본에 없어 같은 말투로 새로 적었다
const PROTOCOL_OPTIONS: [(Protocol, &str); 3] = [
    (Protocol::Ftp, "FTP - 파일 전송 프로토콜"),
    (Protocol::Ftps, "FTPS - TLS로 보호되는 파일 전송 프로토콜"),
    (Protocol::Sftp, "SFTP - SSH 파일 전송 프로토콜"),
];

/// 암호화 선택지 — 기본값 문구는 원본 그대로다 (인벤토리 #72, `:1013`)
const ENCRYPTION_OPTIONS: [(Encryption, &str); 4] = [
    (Encryption::Plain, "일반 FTP 사용 (안전하지 않음)"),
    (
        Encryption::ExplicitIfAvailable,
        "TLS를 통한 명시적 FTP가 가능한 경우 사용",
    ),
    (Encryption::ExplicitRequired, "TLS를 통한 명시적 FTP 필요"),
    (Encryption::Implicit, "TLS를 통한 묵시적 FTP 필요"),
];

/// 로그온 유형 선택지 (인벤토리 #73, `:1014`) — 키 파일은 이번 범위 밖이라 둘뿐이다
const LOGON_OPTIONS: [(LogonType, &str); 2] =
    [(LogonType::Normal, "일반"), (LogonType::Anonymous, "익명")];

/// 전송 모드 라디오 3종 (인벤토리 #77~79, `:852`)
const TRANSFER_OPTIONS: [(TransferMode, &str); 3] = [
    (TransferMode::Default, "기본(E)"),
    (TransferMode::Active, "능동형(A)"),
    (TransferMode::Passive, "수동형(P)"),
];

/// 문자셋 라디오 2종 (인벤토리 #84·#85, `:867`)
const CHARSET_OPTIONS: [&str; 2] = ["UTF-8(U)", "문자셋 직접 설정(C)"];

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
    /// 이름 바꾸기가 끝났는가(Enter 또는 포커스 잃음)
    rename_done: bool,
}

/// 좌측 버튼 3개가 목록에 가하는 변경
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListAction {
    StartRename,
    Delete,
    Duplicate,
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
    }

    /// 암호화 설정이 뜻을 갖는가 — SSH는 전송 계층이 이미 암호화돼 고를 것이 없다 (인벤토리 #72)
    fn encryption_enabled(&self) -> bool {
        !self.protocol.is_ssh()
    }

    /// 사용자·비밀번호를 입력할 수 있는가 — 익명은 서버가 정한 계정을 쓴다 (인벤토리 #74·#75)
    fn credentials_enabled(&self) -> bool {
        self.logon == LogonType::Normal
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
    /// 방금 실패한 까닭 — 바닥에 그대로 보인다
    error: Option<String>,
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

    /// `새 사이트 추가…`(인벤토리 #8) — **빈 초안**으로 연다.
    ///
    /// 이 진입점만 첫 항목을 고르지 않는다. 여기서 기존 사이트를 골라 두면 `확인(O)`이 그것을
    /// 덮어쓰게 되어, 디자인이 `새 사이트` 버튼을 없앤 뒤(README §9) 남은 유일한 추가 경로가 사라진다
    pub fn open_new(&mut self) {
        self.open = true;
        self.tab = ManagerTab::default();
        self.selected = None;
        self.draft = Draft::default();
        self.renaming = None;
        self.error = None;
    }

    fn close(&mut self) {
        self.open = false;
        self.renaming = None;
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
            self.error = None;
        }
    }

    /// 초안을 목록에 반영한다 — 고른 사이트가 있으면 갱신하고, 없으면 새로 만든다.
    /// 등록하지 못하면 까닭을 남기고 `None`이다 (plan Edge Case: 호스트가 빈 값)
    fn commit(&mut self, store: &mut SiteStore) -> Option<SiteId> {
        if self.draft.host.trim().is_empty() {
            self.error = Some(ERROR_NO_HOST.to_owned());
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
            // 전송 설정·문자셋 탭의 값 — `최대 동시 연결 수(M)`는 여기서 기록에 담겨야
            // 연결 관리자의 채널 배정에 그대로 닿는다 (FR-45·D4)
            record.transfer_mode = self.draft.transfer_mode;
            record.connection_limit = self.draft.connection_limit();
            record.charset = self.draft.charset();
        }
        // 익명은 비밀번호를 쓰지 않는다 — 남아 있던 것을 함께 지운다
        let password = if self.draft.credentials_enabled() {
            self.draft.password.as_str()
        } else {
            ""
        };
        if !store.set_password(id, password) {
            self.error = Some(ERROR_PASSWORD.to_owned());
        }
        // 등록한 사이트는 사이드바에도 보여야 한다 — 주소창으로 한 번 열어 숨겨 둔 것을
        // 관리자에서 등록하면 그때부터는 사용자가 목록에 두겠다는 뜻이다
        store.unhide(id);
        self.selected = Some(id);
        self.draft.port = port.to_string();
        Some(id)
    }

    /// 좌측 버튼 3개를 목록에 반영한다 (Acceptance ⑤)
    fn apply_list_action(&mut self, action: ListAction, store: &mut SiteStore) {
        let Some(id) = self.selected else {
            // 고른 사이트가 없으면 할 것이 없다 (plan Edge Case: 사이트 0개에서 `삭제(D)`)
            return;
        };
        match action {
            ListAction::StartRename => {
                self.renaming = store.get(id).map(|record| record.name.clone());
            }
            ListAction::Delete => {
                store.remove(id);
                self.selected = None;
                self.draft = Draft::default();
                self.renaming = None;
            }
            ListAction::Duplicate => {
                if let Some(copy) = store.duplicate(id) {
                    self.select(store, copy);
                }
            }
        }
    }

    /// 이름 바꾸기를 마친다 — 빈 이름이면 `SiteStore`가 기본 이름을 붙인다
    fn finish_rename(&mut self, store: &mut SiteStore) {
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
        let response = egui::Modal::new(egui::Id::new("사이트 관리자"))
            .backdrop_color(egui::Color32::from_black_alpha(SCRIM_ALPHA))
            .frame(
                egui::Frame::new()
                    .fill(theme::SURFACE_BG)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER_CONTROL))
                    .corner_radius(0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, SHADOW_OFFSET_Y],
                        blur: SHADOW_BLUR,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(SHADOW_ALPHA),
                    }),
            )
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(DIALOG_WIDTH, DIALOG_HEIGHT),
                    egui::Sense::hover(),
                );
                let header =
                    egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), HEADER_HEIGHT));
                let footer = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - FOOTER_HEIGHT),
                    rect.max,
                );
                let body_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), header.bottom()),
                    egui::pos2(rect.right(), footer.top()),
                );
                if self.show_header(ui, header) {
                    outcome = SiteManagerOutcome::Close;
                }
                body = self.show_body(ui, body_rect, store, connected);
                if let Some(footer_outcome) = self.show_footer(ui, footer, store) {
                    outcome = footer_outcome;
                }
            });
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
        if response.should_close() {
            outcome = SiteManagerOutcome::Close;
        }
        if !matches!(outcome, SiteManagerOutcome::None) {
            self.close();
        }
        outcome
    }

    /// 헤더 — 제목과 닫기 버튼. 닫기를 눌렀으면 `true` (`:386-388`)
    fn show_header(&mut self, ui: &mut egui::Ui, rect: egui::Rect) -> bool {
        ui.painter().text(
            egui::pos2(rect.left() + HEADER_PAD_LEFT, rect.center().y),
            egui::Align2::LEFT_CENTER,
            TITLE,
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
            "✕",
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
        let (picked, rename_done) = self.show_list(ui, left, store, connected);
        let action = self.show_list_buttons(ui, left);
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
            rename_done,
        }
    }

    /// 좌측 목록 — 라벨 + 웰. 고른 사이트를 돌려준다 (`:393-406`)
    fn show_list(
        &mut self,
        ui: &mut egui::Ui,
        column: egui::Rect,
        store: &SiteStore,
        connected: &[SiteId],
    ) -> (Option<SiteId>, bool) {
        ui.painter().text(
            egui::pos2(column.left(), column.top() + LIST_LABEL_HEIGHT / 2.0),
            egui::Align2::LEFT_CENTER,
            LIST_LABEL,
            egui::FontId::proportional(widgets::FORM_FONT_PX),
            theme::HEADER_TEXT,
        );
        let well = egui::Rect::from_min_max(
            egui::pos2(column.left(), column.top() + LIST_LABEL_HEIGHT + LEFT_GAP),
            egui::pos2(
                column.right(),
                self.buttons_top(column) - GRID_PAD_TOP - LEFT_GAP,
            ),
        );
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
        for record in store.sites() {
            let selected = self.selected == Some(record.id);
            let dot = if connected.contains(&record.id) {
                theme::OK_DOT
            } else {
                theme::TEXT_DIM
            };
            // 이름 바꾸는 중인 줄만 편집기로 바뀐다
            if selected && let Some(name) = &mut self.renaming {
                rename_done = show_rename_row(&mut child, name, dot);
                continue;
            }
            if show_site_row(&mut child, &record.name, dot, selected) {
                picked = Some(record.id);
            }
        }
        (picked, rename_done)
    }

    /// 버튼 3열이 시작하는 y — 목록 웰의 아래끝을 정하는 데도 쓴다
    fn buttons_top(&self, column: egui::Rect) -> f32 {
        column.bottom() - GRID_PAD_BOTTOM - GRID_BUTTON_HEIGHT
    }

    /// 좌측 버튼 3열 (`:407-409`, 인벤토리 #63~65)
    fn show_list_buttons(&mut self, ui: &mut egui::Ui, column: egui::Rect) -> Option<ListAction> {
        let top = self.buttons_top(column);
        let grid = egui::Rect::from_min_max(
            egui::pos2(column.left() + GRID_PAD_X, top),
            egui::pos2(column.right() - GRID_PAD_X, top + GRID_BUTTON_HEIGHT),
        );
        let button_width = (grid.width() - GRID_GAP * 2.0) / 3.0;
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
            (BUTTON_RENAME, ListAction::StartRename),
            (BUTTON_DELETE, ListAction::Delete),
            (BUTTON_DUPLICATE, ListAction::Duplicate),
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
            (ManagerTab::General, TAB_GENERAL),
            (ManagerTab::Transfer, TAB_TRANSFER),
            (ManagerTab::Charset, TAB_CHARSET),
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
            LABEL_TRANSFER_MODE,
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
        for (mode, label) in TRANSFER_OPTIONS {
            if widgets::radio_row(&mut row, label, self.draft.transfer_mode == mode) {
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
        if widgets::check_row(&mut row, LABEL_LIMIT, self.draft.limit_on) {
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
        widgets::form_inline_label(&mut row, LABEL_LIMIT_VALUE, limit_on);
        self.draft.limit =
            widgets::spinner_field(&mut row, self.draft.limit, CONNECTION_LIMIT_RANGE, limit_on);

        if transfer_hint {
            top += widgets::SPINNER_HEIGHT + TRANSFER_GAP;
            text_row(
                ui,
                left,
                top,
                form.width(),
                TRANSFER_APPLY_HINT,
                theme::WARN,
            );
        }
    }

    /// `문자셋` 탭 (`:469-486`, 인벤토리 #82~87)
    fn show_charset(&mut self, ui: &mut egui::Ui, form: egui::Rect) {
        let left = form.left() + TAB_BODY_PAD;
        let mut top = form.top() + TAB_BODY_PAD;

        for text in [CHARSET_HEADING, CHARSET_LABEL] {
            text_row(ui, left, top, form.width(), text, theme::HEADER_TEXT);
            top += TEXT_ROW_HEIGHT + CHARSET_GAP;
        }

        // 라디오 2개 — 세로로 쌓인다 (`:473-479`)
        for (index, label) in CHARSET_OPTIONS.into_iter().enumerate() {
            let custom = index == 1;
            let mut row = self.row(
                ui,
                egui::Rect::from_min_size(
                    egui::pos2(left + MARK_INDENT, top),
                    egui::vec2(form.width() - MARK_INDENT, widgets::FORM_FIELD_HEIGHT),
                ),
            );
            if widgets::radio_row(&mut row, label, self.draft.charset_custom == custom) {
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
        widgets::form_inline_label(&mut row, LABEL_ENCODING, custom);
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
                CHARSET_UNKNOWN_HINT,
                theme::WARN,
            );
            top += TEXT_ROW_HEIGHT + CHARSET_GAP;
        }
        text_row(
            ui,
            left,
            top,
            form.width(),
            CHARSET_FOOTNOTE,
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
        widgets::form_label(&mut row, LABEL_PROTOCOL, true);
        let labels: Vec<&str> = PROTOCOL_OPTIONS.iter().map(|(_, label)| *label).collect();
        if let Some(index) = widgets::dropdown_field(
            &mut row,
            "protocol",
            option_label(&PROTOCOL_OPTIONS, self.draft.protocol),
            field_width,
            true,
            &labels,
        ) {
            self.draft.set_protocol(PROTOCOL_OPTIONS[index].0);
        }

        // 호스트 + 포트 — 한 행에 둘이 온다 (`:430-435`)
        let mut row = self.row(ui, row_rect(1));
        widgets::form_label(&mut row, LABEL_HOST, true);
        let port_label_width = row
            .painter()
            .layout_no_wrap(
                LABEL_PORT.to_owned(),
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
        widgets::form_inline_label(&mut row, LABEL_PORT, true);
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
        widgets::form_label(&mut row, LABEL_ENCRYPTION, encryption_enabled);
        let labels: Vec<&str> = ENCRYPTION_OPTIONS.iter().map(|(_, label)| *label).collect();
        if let Some(index) = widgets::dropdown_field(
            &mut row,
            "encryption",
            option_label(&ENCRYPTION_OPTIONS, self.draft.encryption),
            field_width,
            encryption_enabled,
            &labels,
        ) {
            self.draft.encryption = ENCRYPTION_OPTIONS[index].0;
        }

        // 로그온 유형
        let mut row = self.row(ui, row_rect(3));
        widgets::form_label(&mut row, LABEL_LOGON, true);
        let labels: Vec<&str> = LOGON_OPTIONS.iter().map(|(_, label)| *label).collect();
        if let Some(index) = widgets::dropdown_field(
            &mut row,
            "logon",
            option_label(&LOGON_OPTIONS, self.draft.logon),
            field_width,
            true,
            &labels,
        ) {
            self.draft.logon = LOGON_OPTIONS[index].0;
        }

        // 사용자·비밀번호 — 익명이면 둘 다 비활성이다 (Acceptance ④)
        let credentials = self.draft.credentials_enabled();
        let mut row = self.row(ui, row_rect(4));
        widgets::form_label(&mut row, LABEL_USER, credentials);
        widgets::text_field(
            &mut row,
            "user",
            &mut self.draft.user,
            egui::vec2(field_width, widgets::FORM_FIELD_HEIGHT),
            credentials,
            false,
        );

        let mut row = self.row(ui, row_rect(5));
        widgets::form_label(&mut row, LABEL_PASSWORD, credentials);
        widgets::text_field(
            &mut row,
            "password",
            &mut self.draft.password,
            egui::vec2(field_width, widgets::FORM_FIELD_HEIGHT),
            credentials,
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

    /// 바닥 — 오류 문구와 버튼 3개 (`:495-498`, 인벤토리 #88~90)
    fn show_footer(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        store: &mut SiteStore,
    ) -> Option<SiteManagerOutcome> {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left(), rect.top() + 0.5),
                egui::pos2(rect.right(), rect.top() + 0.5),
            ],
            egui::Stroke::new(1.0, theme::BORDER_SUBTLE),
        );
        if let Some(error) = &self.error {
            ui.painter().text(
                egui::pos2(rect.left() + FOOTER_PAD_X, rect.center().y),
                egui::Align2::LEFT_CENTER,
                error,
                egui::FontId::proportional(widgets::FORM_FONT_PX),
                theme::ERROR_TEXT,
            );
        }
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(egui::Rect::from_min_max(
                    egui::pos2(rect.left() + FOOTER_PAD_X, rect.top()),
                    egui::pos2(rect.right() - FOOTER_PAD_X, rect.bottom()),
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = FOOTER_GAP;
        // 오른쪽부터 그린다 — 원본의 배치 순서(연결·확인·취소)를 뒤집어 넣는다
        let cancel = widgets::design_button(
            &mut child,
            FOOTER_CANCEL,
            theme::TEXT_BUTTON,
            FOOTER_BUTTON_PAD_X,
            egui::vec2(0.0, FOOTER_BUTTON_HEIGHT),
        )
        .clicked();
        let confirm = widgets::design_button(
            &mut child,
            FOOTER_OK,
            theme::TEXT_BUTTON,
            FOOTER_BUTTON_PAD_X,
            egui::vec2(0.0, FOOTER_BUTTON_HEIGHT),
        )
        .clicked();
        let connect = widgets::primary_button(
            &mut child,
            FOOTER_CONNECT,
            FOOTER_BUTTON_PAD_X,
            egui::vec2(0.0, FOOTER_BUTTON_HEIGHT),
        )
        .clicked();

        if cancel {
            return Some(SiteManagerOutcome::Close);
        }
        if confirm || connect {
            let id = self.commit(store)?;
            return Some(if connect {
                SiteManagerOutcome::RegisterAndConnect(id)
            } else {
                SiteManagerOutcome::Register(id)
            });
        }
        None
    }
}

/// 목록의 한 줄 — 아이콘·이름. 눌렸으면 `true` (`:396-404`)
fn show_site_row(ui: &mut egui::Ui, name: &str, dot: egui::Color32, selected: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), LIST_ROW_HEIGHT),
        egui::Sense::click(),
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
    response.clicked()
}

/// 이름 바꾸는 중인 줄 — 편집이 끝났으면 `true` (Enter 또는 포커스를 잃었을 때)
fn show_rename_row(ui: &mut egui::Ui, name: &mut String, dot: egui::Color32) -> bool {
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
    response.request_focus();
    response.lost_focus() || child.input(|input| input.key_pressed(egui::Key::Enter))
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
        // 인벤토리 #60~75·#88~90 — 여기서 한 글자라도 다듬으면 화면과 명세가 갈린다
        assert_eq!(TITLE, "사이트 관리자");
        assert_eq!(LIST_LABEL, "항목 선택(S):");
        assert_eq!(BUTTON_RENAME, "이름 바꾸기(R)");
        assert_eq!(BUTTON_DELETE, "삭제(D)");
        assert_eq!(BUTTON_DUPLICATE, "복제(I)");
        assert_eq!(TAB_GENERAL, "일반");
        assert_eq!(TAB_TRANSFER, "전송 설정");
        assert_eq!(TAB_CHARSET, "문자셋");
        assert_eq!(LABEL_PROTOCOL, "프로토콜(T):");
        assert_eq!(LABEL_HOST, "호스트(H):");
        assert_eq!(LABEL_PORT, "포트(P):");
        assert_eq!(LABEL_ENCRYPTION, "암호화(E):");
        assert_eq!(LABEL_LOGON, "로그온 유형(L):");
        assert_eq!(LABEL_USER, "사용자(U):");
        assert_eq!(LABEL_PASSWORD, "비밀번호(W):");
        assert_eq!(FOOTER_CONNECT, "연결(C)");
        assert_eq!(FOOTER_OK, "확인(O)");
        assert_eq!(FOOTER_CANCEL, "취소");
        // 드롭다운의 기본값 문구도 원본에 적힌 그대로다 (`:1011`·`:1013`·`:1014`)
        assert_eq!(PROTOCOL_OPTIONS[0].1, "FTP - 파일 전송 프로토콜");
        assert_eq!(
            ENCRYPTION_OPTIONS[1].1,
            "TLS를 통한 명시적 FTP가 가능한 경우 사용"
        );
        assert_eq!(LOGON_OPTIONS[0].1, "일반");
        assert_eq!(LOGON_OPTIONS[1].1, "익명");
    }

    #[test]
    fn 대화_치수는_원본과_같다() {
        // Acceptance ② — 1080×680 · 좌측 400px · 라벨 96px · 필드 28px · 바닥 58px · 버튼 30px
        assert_eq!(DIALOG_WIDTH, 1080.0);
        assert_eq!(DIALOG_HEIGHT, 680.0);
        assert_eq!(LEFT_WIDTH, 400.0);
        assert_eq!(widgets::FORM_LABEL_WIDTH, 96.0);
        assert_eq!(widgets::FORM_FIELD_HEIGHT, 28.0);
        assert_eq!(FOOTER_HEIGHT, 58.0);
        assert_eq!(FOOTER_BUTTON_HEIGHT, 30.0);
        assert_eq!(FOOTER_BUTTON_PAD_X, 24.0);
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
    fn 익명이면_사용자와_비밀번호가_비활성이다() {
        // Acceptance ④ (인벤토리 #74·#75)
        let mut draft = Draft::default();
        assert!(draft.credentials_enabled(), "기본값은 `일반`이다");
        draft.logon = LogonType::Anonymous;
        assert!(!draft.credentials_enabled());
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

        manager.apply_list_action(ListAction::Delete, &mut store);
        assert!(store.get(copy).is_none());
        assert_eq!(manager.selected, None, "지운 사이트가 선택으로 남지 않는다");
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
    fn 호스트가_비면_등록을_거부하고_까닭을_남긴다() {
        // plan Edge Case — 조용히 실패하면 사용자는 등록된 줄 안다
        let mut store = SiteStore::new();
        let mut manager = SiteManager::new();
        manager.open_new();
        manager.draft.host = "   ".to_owned();
        assert_eq!(manager.commit(&mut store), None);
        assert_eq!(manager.error.as_deref(), Some(ERROR_NO_HOST));
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
        manager.close();
        assert!(!manager.is_open());
        assert_eq!(manager.draft, Draft::default());
    }

    #[test]
    fn 두_번째_세_번째_탭_문구도_인벤토리_원문_그대로다() {
        // 인벤토리 #76~87 (원본 `:442`·`:852`·`:454`·`:457`·`:471-472`·`:867`·`:481`·`:484`)
        assert_eq!(LABEL_TRANSFER_MODE, "전송 모드(T):");
        assert_eq!(TRANSFER_OPTIONS[0].1, "기본(E)");
        assert_eq!(TRANSFER_OPTIONS[1].1, "능동형(A)");
        assert_eq!(TRANSFER_OPTIONS[2].1, "수동형(P)");
        assert_eq!(LABEL_LIMIT, "동시 연결 수 제한(L)");
        assert_eq!(LABEL_LIMIT_VALUE, "최대 동시 연결 수(M):");
        assert_eq!(CHARSET_HEADING, "서버에서 파일명에 사용하는 문자셋");
        assert_eq!(CHARSET_LABEL, "인코딩:");
        assert_eq!(CHARSET_OPTIONS[0], "UTF-8(U)");
        assert_eq!(CHARSET_OPTIONS[1], "문자셋 직접 설정(C)");
        assert_eq!(LABEL_ENCODING, "인코딩(E):");
        assert_eq!(
            CHARSET_FOOTNOTE,
            "문자셋을 잘못 지정하면 파일명이 올바르게 보여지지 않을 수 있습니다."
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
        // plan Edge Case — 지금 연결에 바로 듣지 않는다는 것을 알리지 않으면
        // 사용자는 바꾼 설정이 곧바로 듣는 줄 알고 같은 실패를 다시 겪는다
        assert_eq!(
            TRANSFER_APPLY_HINT,
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
