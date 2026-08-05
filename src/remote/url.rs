//! 주소창에 적는 원격 주소 파서 (FR-34).
//!
//! **일반 URL 파서가 아니다** — 이 앱이 아는 세 스킴(`ftp`·`ftps`·`sftp`)만 받는다.
//! 쿼리 문자열·프래그먼트·비밀번호 삽입(`user:pass@host`)은 다루지 않는다:
//! 비밀번호는 사이트 설정에 봉인해 두는 것이라(D14) 주소창으로 받으면 화면과 로그에 남는다.
//!
//! `://`가 없으면 **로컬 경로**로 본다 — `C:\ftp` 같은 폴더 이름이 원격 주소로 오해받지 않게
//! 하는 것이 이 파서의 첫 번째 책임이다.
use crate::remote::types::{Protocol, RemotePath};

/// 주소창에서 뜯어낸 원격 위치
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteUrl {
    pub protocol: Protocol,
    /// `user@` 부분 — 없으면 사이트 설정의 사용자(또는 익명)를 쓴다
    pub user: Option<String>,
    pub host: String,
    /// 적지 않았으면 `None` — 프로토콜 기본 포트를 쓴다
    pub port: Option<u16>,
    /// 경로가 없으면 루트다
    pub path: RemotePath,
}

impl RemoteUrl {
    /// 실제로 접속할 포트 — 적지 않았으면 프로토콜 기본값
    pub fn effective_port(&self) -> u16 {
        self.port.unwrap_or_else(|| self.protocol.default_port())
    }
}

/// 스킴 문자열을 프로토콜로 — 모르는 스킴이면 `None`.
/// 대소문자는 구분하지 않는다(`SFTP://`도 받는다)
fn parse_scheme(scheme: &str) -> Option<Protocol> {
    match scheme.to_ascii_lowercase().as_str() {
        "ftp" => Some(Protocol::Ftp),
        "ftps" => Some(Protocol::Ftps),
        "sftp" => Some(Protocol::Sftp),
        _ => None,
    }
}

/// 주소창 입력을 원격 위치로 나눈다. 원격 주소가 아니면 `None`이며,
/// 그때 호출부는 **여느 때처럼 로컬 경로로** 다룬다 (plan Edge Case).
///
/// 받아들이는 형태: `<스킴>://[사용자@]호스트[:포트][/경로]`
pub fn parse_remote_url(raw: &str) -> Option<RemoteUrl> {
    let raw = raw.trim();
    let (scheme, rest) = raw.split_once("://")?;
    let protocol = parse_scheme(scheme)?;

    // 경로는 **호스트 뒤 첫 `/`**부터다. 그 앞에는 사용자·호스트·포트만 온다
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, ""),
    };

    // 사용자명에 `@`가 들어 있을 수 있어 **마지막 `@`**를 구분자로 본다
    // (T5의 주소 가리기와 같은 판단 — 앞에서 찾으면 사용자명이 잘린다)
    let (user, host_port) = match authority.rsplit_once('@') {
        Some((user, host_port)) => (Some(user), host_port),
        None => (None, authority),
    };
    // 사용자명 자리가 비어 있으면(`@host`) 적지 않은 것으로 본다
    let user = user.filter(|name| !name.is_empty()).map(str::to_owned);

    let (host, port) = split_host_port(host_port)?;
    if host.is_empty() {
        // 스킴만 적은 경우(`sftp://`) — 어디로 갈지 알 수 없다 (plan Edge Case)
        return None;
    }

    Some(RemoteUrl {
        protocol,
        user,
        host,
        port,
        path: RemotePath::new(path),
    })
}

/// `호스트[:포트]`를 나눈다. IPv6는 대괄호 표기(`[::1]:2121`)를 쓴다.
///
/// **포트가 숫자가 아니면 통째로 실패**시킨다 — 숫자가 아닌 값을 조용히 버리고 기본 포트로
/// 붙으면, 사용자가 적은 것과 다른 곳에 연결된다
fn split_host_port(raw: &str) -> Option<(String, Option<u16>)> {
    // IPv6 리터럴은 콜론이 주소 자체에 들어 있어 대괄호로 감싼다
    if let Some(rest) = raw.strip_prefix('[') {
        let (inside, after) = rest.split_once(']')?;
        let port = match after {
            "" => None,
            with_port => Some(with_port.strip_prefix(':')?.parse().ok()?),
        };
        return Some((inside.to_owned(), port));
    }
    match raw.rsplit_once(':') {
        Some((host, port)) => Some((host.to_owned(), Some(port.parse().ok()?))),
        None => Some((raw.to_owned(), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> RemoteUrl {
        parse_remote_url(raw).unwrap_or_else(|| panic!("파싱 실패: {raw}"))
    }

    #[test]
    fn 세_스킴을_알아본다() {
        assert_eq!(parse("sftp://example.test").protocol, Protocol::Sftp);
        assert_eq!(parse("ftp://example.test").protocol, Protocol::Ftp);
        assert_eq!(parse("ftps://example.test").protocol, Protocol::Ftps);
        // 대소문자는 구분하지 않는다
        assert_eq!(parse("SFTP://example.test").protocol, Protocol::Sftp);
    }

    #[test]
    fn 호스트만_적으면_루트를_가리킨다() {
        let url = parse("sftp://example.test");
        assert_eq!(url.host, "example.test");
        assert_eq!(url.user, None);
        assert_eq!(url.port, None);
        assert_eq!(url.path.as_str(), "/");
        // 포트를 적지 않으면 프로토콜 기본값이다
        assert_eq!(url.effective_port(), 22);
    }

    #[test]
    fn 사용자와_포트와_경로를_나눈다() {
        // Acceptance ③의 본보기
        let url = parse("ftp://user@example.test:2121/pub");
        assert_eq!(url.protocol, Protocol::Ftp);
        assert_eq!(url.user.as_deref(), Some("user"));
        assert_eq!(url.host, "example.test");
        assert_eq!(url.port, Some(2121));
        assert_eq!(url.path.as_str(), "/pub");
        assert_eq!(url.effective_port(), 2121);
    }

    #[test]
    fn 끝의_빗금은_루트다() {
        assert_eq!(parse("ftps://example.test/").path.as_str(), "/");
        assert_eq!(parse("ftps://example.test/").effective_port(), 21);
    }

    #[test]
    fn 원격_주소가_아니면_로컬로_넘긴다() {
        // `://`가 없으면 전부 로컬 경로다 — 이것이 이 파서의 첫 번째 책임이다
        assert_eq!(parse_remote_url(r"C:\ftp"), None);
        assert_eq!(parse_remote_url(r"C:\Users\me\문서"), None);
        assert_eq!(parse_remote_url(r"\\server\share"), None);
        assert_eq!(parse_remote_url("example.test"), None);
        // 모르는 스킴도 마찬가지다
        assert_eq!(parse_remote_url("http://example.test"), None);
        assert_eq!(parse_remote_url("ssh://example.test"), None);
    }

    #[test]
    fn 호스트가_없으면_무시한다() {
        // 스킴만 적었다 — 어디로 갈지 알 수 없다 (plan Edge Case)
        assert_eq!(parse_remote_url("sftp://"), None);
        assert_eq!(parse_remote_url("sftp:///pub"), None);
        assert_eq!(parse_remote_url("sftp://user@"), None);
    }

    #[test]
    fn 포트가_숫자가_아니면_실패한다() {
        // 조용히 버리고 기본 포트로 붙으면 사용자가 적은 것과 다른 곳에 연결된다
        assert_eq!(parse_remote_url("sftp://example.test:포트"), None);
        assert_eq!(parse_remote_url("sftp://example.test:99999"), None);
        assert_eq!(parse_remote_url("sftp://example.test:"), None);
    }

    #[test]
    fn ipv6는_대괄호_표기를_쓴다() {
        // 주소 자체에 콜론이 들어 있어 대괄호가 없으면 포트와 구분되지 않는다.
        // 값은 RFC 3849 문서용 대역이다
        let url = parse("sftp://[2001:db8::1]");
        assert_eq!(url.host, "2001:db8::1");
        assert_eq!(url.port, None);

        let with_port = parse("sftp://[2001:db8::1]:2222/pub");
        assert_eq!(with_port.host, "2001:db8::1");
        assert_eq!(with_port.port, Some(2222));
        assert_eq!(with_port.path.as_str(), "/pub");
    }

    #[test]
    fn 사용자명에_골뱅이가_있어도_호스트를_잃지_않는다() {
        // 계정이 메일 주소인 서버가 있다 — 앞에서 자르면 호스트가 사용자명으로 딸려 간다
        let url = parse("ftp://me@corp.test@example.test/pub");
        assert_eq!(url.user.as_deref(), Some("me@corp.test"));
        assert_eq!(url.host, "example.test");
        assert_eq!(url.path.as_str(), "/pub");
    }

    #[test]
    fn 경로의_한글과_공백은_그대로_남는다() {
        // 서버가 UTF-8을 쓰면 그대로 보내야 한다 — 여기서 인코딩하지 않는다
        let url = parse("sftp://example.test/자료 모음/보고서");
        assert_eq!(url.path.as_str(), "/자료 모음/보고서");
    }

    #[test]
    fn 앞뒤_공백은_다듬는다() {
        // 주소를 붙여 넣으면 공백이 딸려 오는 일이 흔하다
        assert_eq!(parse("  sftp://example.test/pub  ").host, "example.test");
    }
}
