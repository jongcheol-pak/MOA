//! 연결별 서버 로그 — 링 버퍼 (FR-40·D13·D14).
//!
//! 워커가 서버와 주고받은 것을 이벤트로 올리면 연결이 여기에 쌓고, 화면(T20)은 읽기만 한다.
//!
//! **상한을 두는 이유**(D13): 장시간 열어 둔 연결의 로그를 끝없이 쌓으면 그것만으로 메모리가
//! 는다. 2000줄이면 평균 100바이트로 잡아 연결당 약 200KB다.
//!
//! **비밀번호는 어느 줄에도 남지 않는다**(D14): 로그는 `⧉` 버튼으로 파일·클립보드에 그대로
//! 나갈 수 있어, 한 번 들어가면 회수할 방법이 없다. 그래서 가리기(`mask_secrets`)는
//! **버퍼에 쌓을 때와 이벤트를 만들 때 양쪽**에서 한다 — 버퍼만 지키면 이벤트를 직접 쓰는 쪽
//! (연결 이벤트를 그대로 소비하는 화면)에 평문이 새어 나간다.
//!
//! 레벨 필터·검색은 두지 않는다 — 디자인에 그 진입점이 없다(plan T5 비추상화 선언).
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

/// 로그 한 줄의 종류 — 디자인의 종류 열(44px)에 그대로 쓰인다
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogKind {
    Status,
    Command,
    Response,
    Error,
}

impl LogKind {
    /// 화면과 복사본에 쓰이는 표기 (디자인 원문)
    pub fn label(self) -> &'static str {
        match self {
            LogKind::Status => crate::i18n::log_kind_status(),
            LogKind::Command => crate::i18n::log_kind_command(),
            LogKind::Response => crate::i18n::log_kind_response(),
            LogKind::Error => crate::i18n::log_kind_error(),
        }
    }
}

/// 로그 한 줄
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// `HH:MM:SS` — 쌓은 시각(로컬)
    pub time: String,
    pub kind: LogKind,
    pub text: String,
}

/// 연결 하나의 서버 로그
#[derive(Debug, Clone, Default)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
}

impl LogBuffer {
    /// 담아 두는 줄 수 상한 (D13)
    pub const CAPACITY: usize = 2000;

    pub fn new() -> LogBuffer {
        LogBuffer {
            lines: VecDeque::new(),
        }
    }

    /// 한 줄 쌓는다. 상한을 넘으면 **가장 오래된 것부터** 밀린다.
    ///
    /// 비밀번호가 섞인 줄은 여기서도 가려진다 — 이벤트를 만들 때 이미 한 번 가리지만,
    /// 버퍼에 직접 쌓는 다른 길이 생겨도 새지 않도록 이 문턱을 함께 둔다
    pub fn push(&mut self, kind: LogKind, text: impl Into<String>) {
        self.push_line(LogLine {
            time: local_hms(SystemTime::now()),
            kind,
            text: mask_secrets(&text.into()),
        });
    }

    fn push_line(&mut self, line: LogLine) {
        if self.lines.len() >= LogBuffer::CAPACITY {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    pub fn iter(&self) -> impl Iterator<Item = &LogLine> {
        self.lines.iter()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// 복사(`⧉`)용 본문 — 화면과 같은 순서로 시각·종류·본문을 잇는다.
    ///
    /// 긴 줄은 자르지 않는다(말줄임은 화면이 하는 일이고, 복사본은 원문이어야 쓸모가 있다)
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for line in &self.lines {
            out.push_str(&line.time);
            out.push('\t');
            out.push_str(line.kind.label());
            out.push('\t');
            out.push_str(&line.text);
            out.push('\n');
        }
        out
    }
}

/// 비밀번호가 드러나는 자리를 가린다 (D14).
///
/// 두 가지를 본다 — FTP의 `PASS <비밀번호>` 명령과, 주소에 자격증명이 붙은 형태
/// (`sftp://사용자:비밀번호@호스트`). 둘 다 그대로 남으면 로그를 내보내는 순간 새어 나간다.
///
/// **여러 번 걸어도 결과가 같다** — 이벤트를 만들 때와 버퍼에 쌓을 때 두 번 지나기 때문이다.
pub(crate) fn mask_secrets(text: &str) -> String {
    const MASK: &str = "******";
    let trimmed = text.trim_start();
    // 바이트로 견준다 — 한글이 섞인 줄에서 4바이트 자리가 글자 경계가 아닐 수 있다
    let starts_with_pass = trimmed
        .as_bytes()
        .get(..4)
        .is_some_and(|head| head.eq_ignore_ascii_case(b"PASS"));
    if starts_with_pass {
        // `PASS`만 있고 인자가 없으면 가릴 것이 없다
        let rest = &trimmed[4..];
        if rest.is_empty() || rest.starts_with(char::is_whitespace) {
            return format!("PASS {MASK}");
        }
    }
    mask_url_credentials(text, MASK)
}

/// `스킴://사용자:비밀번호@호스트`에서 비밀번호 자리만 가린다.
///
/// **`@`는 뒤에서부터 찾는다.** 비밀번호에 `@`가 들어 있는 일이 흔한데(`p@ss`), 앞에서부터
/// 찾으면 그 `@`를 호스트 구분자로 오인해 비밀번호 뒷부분이 평문으로 남는다.
/// 찾는 범위는 호스트 부분(첫 `/` 또는 공백 앞)까지다 — 뒤쪽 경로의 `@`에 끌려가지 않는다.
///
/// 한 줄에 주소가 여럿이면 **전부** 가린다 — 하나만 가리고 나머지를 흘려보내면 그것으로 샌다.
fn mask_url_credentials(text: &str, mask: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(scheme_end) = rest.find("://") {
        let after_scheme = scheme_end + 3;
        out.push_str(&rest[..after_scheme]);
        rest = &rest[after_scheme..];

        let authority_end = rest
            .find(|c: char| c == '/' || c.is_whitespace())
            .unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        let Some(at) = authority.rfind('@') else {
            continue;
        };
        let credentials = &authority[..at];
        let Some(colon) = credentials.find(':') else {
            continue;
        };
        out.push_str(&credentials[..colon]);
        out.push(':');
        out.push_str(mask);
        // 가린 자리 다음(`@`)부터 이어 본다
        rest = &rest[at..];
    }
    out.push_str(rest);
    out
}

/// 지금 시각을 로컬 `HH:MM:SS`로. 변환에 실패하면 빈 문자열이다
fn local_hms(now: SystemTime) -> String {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

    let Ok(since_epoch) = now.duration_since(UNIX_EPOCH) else {
        return String::new();
    };
    // FILETIME은 1601-01-01부터의 100나노초 단위다 — 유닉스 기준점과 11644473600초 차이
    const EPOCH_DIFFERENCE_SECONDS: u64 = 11_644_473_600;
    let intervals = (since_epoch.as_secs() + EPOCH_DIFFERENCE_SECONDS) * 10_000_000
        + u64::from(since_epoch.subsec_nanos() / 100);
    let file_time = FILETIME {
        dwLowDateTime: intervals as u32,
        dwHighDateTime: (intervals >> 32) as u32,
    };

    let mut utc = Default::default();
    let mut local = Default::default();
    // 안전성: 인자가 모두 스택 소유이고 널이 될 수 없다. 실패하면 빈 시각으로 둔다
    // (`panel::file_list::format_filetime`이 쓰는 것과 같은 변환 쌍)
    unsafe {
        if FileTimeToSystemTime(&file_time, &mut utc).is_err()
            || SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).is_err()
        {
            return String::new();
        }
    }
    format!(
        "{:02}:{:02}:{:02}",
        local.wHour, local.wMinute, local.wSecond
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 종류_표기는_디자인_원문과_같다() {
        let _guard =
            crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
        assert_eq!(LogKind::Status.label(), "상태:");
        assert_eq!(LogKind::Command.label(), "명령:");
        assert_eq!(LogKind::Response.label(), "응답:");
        assert_eq!(LogKind::Error.label(), "오류:");
    }

    #[test]
    fn 상한을_넘으면_가장_오래된_줄부터_밀린다() {
        let mut buffer = LogBuffer::new();
        for index in 0..LogBuffer::CAPACITY + 100 {
            buffer.push(LogKind::Status, format!("{index}번째"));
        }
        assert_eq!(buffer.len(), LogBuffer::CAPACITY);
        let first = buffer.iter().next().expect("첫 줄");
        assert_eq!(first.text, "100번째", "가장 오래된 100줄이 밀려야 한다");
        let last = buffer.iter().last().expect("마지막 줄");
        assert_eq!(last.text, format!("{}번째", LogBuffer::CAPACITY + 99));
    }

    #[test]
    fn 비밀번호_명령은_가려져_기록된다() {
        // D14 — 로그는 파일·클립보드로 나갈 수 있어 한 번 들어가면 회수할 수 없다
        let mut buffer = LogBuffer::new();
        buffer.push(LogKind::Command, "PASS 진짜비밀번호");
        buffer.push(LogKind::Command, "pass 소문자도");
        buffer.push(
            LogKind::Status,
            "sftp://deploy:진짜비밀번호@example.test:22 에 연결 중...",
        );

        let text = buffer.to_text();
        assert!(
            !text.contains("진짜비밀번호"),
            "평문 비밀번호가 남았다: {text}"
        );
        assert!(!text.contains("소문자도"));
        assert!(text.contains("PASS ******"));
        assert!(text.contains("sftp://deploy:******@example.test:22"));
        // 버퍼 안의 줄 자체에도 남지 않는다
        assert!(
            buffer
                .iter()
                .all(|line| !line.text.contains("진짜비밀번호"))
        );
    }

    #[test]
    fn 비밀번호처럼_보이는_다른_줄은_건드리지_않는다() {
        let mut buffer = LogBuffer::new();
        // `PASS`로 시작하지만 다른 낱말인 경우
        buffer.push(LogKind::Response, "PASSIVE 모드로 전환했습니다");
        buffer.push(
            LogKind::Status,
            "sftp://deploy@example.test:22 에 연결 중...",
        );
        let text = buffer.to_text();
        assert!(text.contains("PASSIVE 모드로 전환했습니다"));
        // 비밀번호 자리가 없는 주소는 그대로다
        assert!(text.contains("sftp://deploy@example.test:22"));
        assert!(!text.contains("******"));
    }

    #[test]
    fn 비밀번호에_골뱅이가_들어_있어도_전부_가려진다() {
        // `@`를 앞에서부터 찾으면 `p@ss`의 뒷부분이 평문으로 남는다
        let mut buffer = LogBuffer::new();
        buffer.push(
            LogKind::Status,
            "sftp://deploy:p@ss@example.test:22 에 연결",
        );
        buffer.push(LogKind::Status, "ftp://user:a@b@c@host/경로/파일@이름.txt");

        let text = buffer.to_text();
        assert!(!text.contains("p@ss"), "비밀번호가 남았다: {text}");
        assert!(text.contains("sftp://deploy:******@example.test:22"));
        // 경로에 있는 `@`는 자격증명이 아니다 — 호스트 부분에서만 찾는다
        assert!(
            text.contains("ftp://user:******@host/경로/파일@이름.txt"),
            "{text}"
        );
    }

    #[test]
    fn 한_줄에_주소가_여럿이면_전부_가린다() {
        // 하나만 가리고 나머지를 흘려보내면 그것으로 샌다
        let masked = mask_secrets("sftp://a:비밀1@host1 에서 sftp://b:비밀2@host2 로 옮깁니다");
        assert!(!masked.contains("비밀1"), "{masked}");
        assert!(!masked.contains("비밀2"), "{masked}");
        assert_eq!(
            masked,
            "sftp://a:******@host1 에서 sftp://b:******@host2 로 옮깁니다"
        );
    }

    #[test]
    fn 가리기는_여러_번_걸어도_결과가_같다() {
        // 이벤트를 만들 때와 버퍼에 쌓을 때 두 번 지난다
        let once = mask_secrets("PASS 비밀");
        assert_eq!(mask_secrets(&once), once);
        let url = mask_secrets("sftp://deploy:p@ss@example.test:22");
        assert_eq!(mask_secrets(&url), url);
    }

    #[test]
    fn 한글로_시작하는_줄도_안전하게_다룬다() {
        // 가리기 판정을 글자가 아니라 바이트로 자르면 여기서 패닉한다 — 로그 대부분이 한글이다
        let mut buffer = LogBuffer::new();
        buffer.push(LogKind::Status, "연결에 실패해 1초 뒤 다시 시도합니다");
        buffer.push(LogKind::Error, "가");
        buffer.push(LogKind::Response, "");
        assert_eq!(buffer.len(), 3);
        assert!(buffer.to_text().contains("연결에 실패해"));
    }

    #[test]
    fn 복사본은_시각_종류_본문_순서로_이어진다() {
        let mut buffer = LogBuffer::new();
        buffer.push_line(LogLine {
            time: "15:02:12".to_owned(),
            kind: LogKind::Command,
            text: "USER deploy".to_owned(),
        });
        buffer.push_line(LogLine {
            time: "15:02:13".to_owned(),
            kind: LogKind::Response,
            text: "230 User deploy logged in".to_owned(),
        });
        assert_eq!(
            buffer.to_text(),
            "15:02:12\t명령:\tUSER deploy\n15:02:13\t응답:\t230 User deploy logged in\n"
        );
    }

    #[test]
    fn 빈_버퍼의_복사본은_빈_문자열이다() {
        let buffer = LogBuffer::new();
        assert!(buffer.is_empty());
        assert_eq!(buffer.to_text(), "");
    }

    #[test]
    fn 아주_긴_줄과_제어문자는_그대로_남는다() {
        // 말줄임은 화면이 하는 일이다 — 복사본이 잘리면 쓸모가 없다
        let mut buffer = LogBuffer::new();
        let long = "가".repeat(5000);
        buffer.push(LogKind::Response, long.clone());
        buffer.push(LogKind::Response, "제어\t문자\u{7f}가 섞인 응답");

        let first = buffer.iter().next().expect("첫 줄");
        assert_eq!(first.text.chars().count(), 5000);
        assert!(buffer.to_text().contains('\u{7f}'));
    }

    #[test]
    fn 같은_초에_여러_줄이_들어가도_모두_남는다() {
        let mut buffer = LogBuffer::new();
        for index in 0..50 {
            buffer.push(LogKind::Status, format!("{index}"));
        }
        assert_eq!(buffer.len(), 50);
        // 시각이 같아도 순서는 넣은 대로다
        let texts: Vec<&str> = buffer.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(texts[0], "0");
        assert_eq!(texts[49], "49");
    }

    #[test]
    fn 쌓은_줄에는_시각이_붙는다() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogKind::Status, "연결 중");
        let line = buffer.iter().next().expect("첫 줄");
        // `HH:MM:SS` 형태 — 시간대 변환이 실패하는 환경에서는 빈 문자열이다
        assert!(
            line.time.is_empty() || (line.time.len() == 8 && line.time.matches(':').count() == 2),
            "시각 표기가 이상하다: {}",
            line.time
        );
    }

    #[test]
    fn 비우면_길이가_0이_된다() {
        let mut buffer = LogBuffer::new();
        buffer.push(LogKind::Status, "한 줄");
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.to_text(), "");
    }
}
