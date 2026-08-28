//! 업데이트 확인·내려받기에 쓰는 최소 HTTP 클라이언트 (FR-62).
//!
//! **새 패키지를 들이지 않는다** — 이미 링크하고 있는 `windows` 크레이트의
//! WinHTTP를 쓴다(`Win32_Networking_WinHttp`). 이 앱이 필요로 하는 것은 GET 둘뿐이라
//! HTTP 크레이트를 더하는 대신 플랫폼 기본 기능을 부른다(AGENTS 최소 의존 원칙).
//!
//! **GET 말고는 만들지 않는다** — 재시도 정책·미들웨어·클라이언트 트레이트를 두지 않는다.
//! 실패하면 사용자가 다시 누르는 것이 이 기능의 재시도다.
use std::path::Path;
use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTP_PORT, INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
    WINHTTP_FLAG_SECURE, WINHTTP_OPEN_REQUEST_FLAGS, WINHTTP_QUERY_CONTENT_LENGTH,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_FLAG_NUMBER64, WINHTTP_QUERY_STATUS_CODE,
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders,
    WinHttpReadData, WinHttpReceiveResponse, WinHttpSendRequest,
};
use windows::core::PCWSTR;

/// 메모리로 받을 응답의 상한 — 이 경로로 받는 것은 릴리즈 목록 JSON 하나뿐이라
/// 수 MB를 넘길 일이 없다. 상한이 없으면 잘못된 주소가 메모리를 끝없이 먹는다
const MAX_IN_MEMORY: usize = 16 * 1024 * 1024;

/// 한 번에 읽어 오는 크기 — 파일로 흘려보낼 때도 이 단위로 끊어 메모리가 일정하다
const CHUNK: usize = 64 * 1024;

/// 서버에 밝히는 이름. **GitHub API는 이 헤더가 없으면 403으로 거절한다**
const USER_AGENT: &str = concat!("MOA/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpError {
    /// 주소를 갈라 읽지 못했다(스킴 없음·호스트 없음·https가 아님)
    BadUrl,
    /// 연결·전송 자체가 실패했다 (Win32 오류 코드)
    Transport(u32),
    /// 응답은 왔지만 200이 아니다 (404·403 등)
    Status(u32),
    /// 받을 것이 상한보다 크다
    TooLarge,
    /// 받은 것을 파일에 쓰지 못했다
    Write,
}

/// 주소를 호스트·포트·경로로 가른 결과
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlParts {
    pub host: String,
    pub port: u16,
    /// 물음표 뒤 질의까지 포함한다 — WinHTTP에 넘기는 「객체 이름」이 그 둘을 함께 받는다
    pub path: String,
    pub secure: bool,
}

/// `https://host[:port]/path?query`를 갈라 읽는다. 읽지 못하면 `None`.
///
/// **`http`도 갈라 읽되 요청은 거절한다**(아래 `open_request`) — 파싱과 정책을 한자리에
/// 섞으면 어느 쪽이 막았는지 알 수 없어, 여기서는 형태만 보고 판정은 부르는 쪽이 한다
pub fn split_url(url: &str) -> Option<UrlParts> {
    let (scheme, rest) = url.split_once("://")?;
    let secure = match scheme.to_ascii_lowercase().as_str() {
        "https" => true,
        "http" => false,
        _ => return None,
    };
    // 경로가 아예 없으면 뿌리(`/`)로 본다 — `https://example.com`은 유효한 주소다
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return None;
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port_text)) => (host, port_text.parse::<u16>().ok()?),
        None => (
            authority,
            if secure {
                INTERNET_DEFAULT_HTTPS_PORT
            } else {
                INTERNET_DEFAULT_HTTP_PORT
            },
        ),
    };
    if host.is_empty() {
        return None;
    }
    Some(UrlParts {
        host: host.to_owned(),
        port,
        path: path.to_owned(),
        secure,
    })
}

/// 주소의 내용을 통째로 메모리에 받는다 (릴리즈 목록 JSON 용도).
///
/// `accept`는 `Accept` 헤더 값이다 — GitHub API가 판을 가르는 데 쓴다
pub fn get_bytes(url: &str, accept: &str) -> Result<Vec<u8>, HttpError> {
    let mut body = Vec::new();
    // 릴리즈 목록 JSON은 몇 KB라 진행을 보일 것이 없다
    read_response(
        url,
        Some(accept),
        |_, _| {},
        |chunk| {
            if body.len() + chunk.len() > MAX_IN_MEMORY {
                return Err(HttpError::TooLarge);
            }
            body.extend_from_slice(chunk);
            Ok(())
        },
    )?;
    Ok(body)
}

/// 주소의 내용을 파일로 흘려 받는다 (설치 파일 용도).
///
/// **실패하면 받다 만 파일을 지운다** — 남겨 두면 다음 실행이 그것을 온전한 파일로 오인한다
/// `progress`는 조각을 하나 받을 때마다 `(받은 누적 바이트, 전체 크기)`로 불린다 —
/// 전체 크기는 서버가 `Content-Length`를 주지 않으면 `None`이다(chunked 전송)
pub fn download_to_file(
    url: &str,
    dest: &Path,
    progress: impl FnMut(u64, Option<u64>),
) -> Result<(), HttpError> {
    use std::io::Write;

    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|_| HttpError::Write)?;
    }
    let mut file = std::fs::File::create(dest).map_err(|_| HttpError::Write)?;
    let outcome = read_response(url, None, progress, |chunk| {
        file.write_all(chunk).map_err(|_| HttpError::Write)
    });
    // 핸들을 먼저 닫아야 아래 삭제가 「사용 중」으로 실패하지 않는다
    drop(file);
    if let Err(error) = outcome {
        let _ = std::fs::remove_file(dest);
        return Err(error);
    }
    Ok(())
}

/// GET 한 번을 돌려 응답 본문을 조각마다 `sink`에 넘긴다.
///
/// 받는 쪽이 메모리에 쌓든 파일에 쓰든 이 함수는 모른다 — 그래서 위 둘이 같은 길을 쓴다.
/// 리다이렉트는 WinHTTP가 알아서 따라간다(릴리즈 자산 주소가 저장소로 넘겨진다)
fn read_response(
    url: &str,
    accept: Option<&str>,
    mut progress: impl FnMut(u64, Option<u64>),
    mut sink: impl FnMut(&[u8]) -> Result<(), HttpError>,
) -> Result<(), HttpError> {
    let parts = split_url(url).ok_or(HttpError::BadUrl)?;
    // **https만 받는다** — 업데이트 설치 파일을 평문 경로로 받지 않는다(가로채 바꿔치기 방지).
    // 체크섬 대조가 뒤에 있지만 그 값도 같은 서버에서 오므로 전송 보호를 대신하지 못한다
    if !parts.secure {
        return Err(HttpError::BadUrl);
    }

    let session = Session::open()?;
    let connect = session.connect(&parts.host, parts.port)?;
    let request = connect.open_request(&parts.path, parts.secure)?;

    let mut headers = format!("User-Agent: {USER_AGENT}\r\n");
    if let Some(accept) = accept {
        headers.push_str(&format!("Accept: {accept}\r\n"));
    }
    let headers: Vec<u16> = headers.encode_utf16().collect();

    // 안전성: 위에서 만든 유효한 요청 핸들과, 이 함수가 살아 있는 동안 유지되는 헤더
    // 버퍼를 넘긴다. 보낼 본문이 없어 optional 인자는 비운다
    unsafe { WinHttpSendRequest(request.0, Some(&headers), None, 0, 0, 0) }
        .map_err(|error| HttpError::Transport(error.code().0 as u32))?;
    // 안전성: 같은 요청 핸들. 두 번째 인자는 예약분이라 반드시 null이다
    unsafe { WinHttpReceiveResponse(request.0, std::ptr::null_mut()) }
        .map_err(|error| HttpError::Transport(error.code().0 as u32))?;

    let status = request.status_code()?;
    if status != 200 {
        return Err(HttpError::Status(status));
    }

    // **상태 코드를 본 뒤에 묻는다** — `WinHttpReceiveResponse`가 리다이렉트를 이미 따라갔으므로
    // 여기서 읽는 값은 최종 응답의 것이다
    let total = request.content_length();

    let mut buffer = vec![0u8; CHUNK];
    let mut received = 0u64;
    loop {
        let mut read = 0u32;
        // 안전성: 유효한 요청 핸들과, 길이를 함께 넘기는 우리 소유 버퍼. 읽은 바이트 수는
        // 스택 변수에 받는다
        unsafe {
            WinHttpReadData(
                request.0,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                &mut read,
            )
        }
        .map_err(|error| HttpError::Transport(error.code().0 as u32))?;
        if read == 0 {
            break;
        }
        sink(&buffer[..read as usize])?;
        received += u64::from(read);
        progress(received, total);
    }
    Ok(())
}

// ── 핸들 ──
//
// WinHTTP 핸들 셋은 만든 역순으로 닫아야 한다. 각자를 Drop 타입으로 감싸 두면
// 중간에 오류로 빠져나가도 닫히는 것이 보장되고, 닫는 순서는 지역 변수의 소멸 순서가 지킨다
// (`remote::envelope`의 CNG 핸들 래퍼와 같은 방식이다)

struct Session(*mut core::ffi::c_void);

impl Session {
    fn open() -> Result<Session, HttpError> {
        let agent: Vec<u16> = USER_AGENT
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        // 안전성: 널로 끝나는 이름 버퍼를 넘기고, 프록시 인자는 자동 검색이라 비운다.
        // 실패는 널 핸들로 돌아온다
        let handle = unsafe {
            WinHttpOpen(
                PCWSTR(agent.as_ptr()),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            )
        };
        if handle.is_null() {
            return Err(HttpError::Transport(last_error()));
        }
        Ok(Session(handle))
    }

    fn connect(&self, host: &str, port: u16) -> Result<Connect, HttpError> {
        let host: Vec<u16> = host.encode_utf16().chain(std::iter::once(0)).collect();
        // 안전성: 살아 있는 세션 핸들과 널로 끝나는 호스트 버퍼. 마지막 인자는 예약분(0)이다
        let handle = unsafe { WinHttpConnect(self.0, PCWSTR(host.as_ptr()), port, 0) };
        if handle.is_null() {
            return Err(HttpError::Transport(last_error()));
        }
        Ok(Connect(handle))
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // 안전성: 우리가 열어 아직 닫지 않은 핸들. 실패해도 할 수 있는 일이 없다
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

struct Connect(*mut core::ffi::c_void);

impl Connect {
    fn open_request(&self, path: &str, secure: bool) -> Result<Request, HttpError> {
        let path: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        let flags = if secure {
            WINHTTP_FLAG_SECURE
        } else {
            WINHTTP_OPEN_REQUEST_FLAGS(0)
        };
        // 안전성: 살아 있는 연결 핸들과 널로 끝나는 경로 버퍼. 동사·판·참조 주소는 기본값을
        // 쓰고(널), 받을 형식 목록도 제한하지 않는다(널)
        let handle = unsafe {
            WinHttpOpenRequest(
                self.0,
                PCWSTR::null(),
                PCWSTR(path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                flags,
            )
        };
        if handle.is_null() {
            return Err(HttpError::Transport(last_error()));
        }
        Ok(Request(handle))
    }
}

impl Drop for Connect {
    fn drop(&mut self) {
        // 안전성: 위 Session::drop과 같다
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

struct Request(*mut core::ffi::c_void);

impl Request {
    /// 응답의 상태 코드를 숫자로 읽는다
    fn status_code(&self) -> Result<u32, HttpError> {
        let mut status = 0u32;
        let mut size = std::mem::size_of::<u32>() as u32;
        // 안전성: 응답을 이미 받은 요청 핸들. 숫자 플래그를 켜 두어 WinHTTP가 문자열이 아니라
        // u32를 채우며, 그 크기를 함께 넘긴다. 헤더 이름과 색인은 상태 코드 조회에 쓰이지 않는다
        unsafe {
            WinHttpQueryHeaders(
                self.0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some((&raw mut status).cast()),
                &mut size,
                std::ptr::null_mut(),
            )
        }
        .map_err(|error| HttpError::Transport(error.code().0 as u32))?;
        Ok(status)
    }

    /// 응답 본문의 전체 크기 — 서버가 `Content-Length`를 주지 않으면(chunked 전송) `None`.
    ///
    /// 없는 헤더를 묻는 것은 오류가 아니라 알려 주지 않았다는 뜻이라 `Option`으로 돌려준다
    fn content_length(&self) -> Option<u64> {
        let mut length = 0u64;
        let mut size = std::mem::size_of::<u64>() as u32;
        // 안전성: 응답을 이미 받은 요청 핸들. 64비트 숫자 플래그를 켜 두어 WinHTTP가 문자열이
        // 아니라 u64를 채우며, 그 크기를 함께 넘긴다. 헤더 이름과 색인은 이 조회에 쓰이지 않는다
        unsafe {
            WinHttpQueryHeaders(
                self.0,
                WINHTTP_QUERY_CONTENT_LENGTH | WINHTTP_QUERY_FLAG_NUMBER64,
                PCWSTR::null(),
                Some((&raw mut length).cast()),
                &mut size,
                std::ptr::null_mut(),
            )
        }
        .ok()?;
        Some(length)
    }
}

impl Drop for Request {
    fn drop(&mut self) {
        // 안전성: 위 Session::drop과 같다
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

/// 널 핸들로 돌아온 호출의 사유. 핸들을 돌려주는 WinHTTP 함수는 `Result`가 아니라
/// 널을 주므로 오류 코드를 따로 물어야 한다
fn last_error() -> u32 {
    // 안전성: 인자 없는 조회 함수다
    unsafe { windows::Win32::Foundation::GetLastError().0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 주소를_호스트와_경로로_가른다() {
        let parts = split_url("https://api.github.com/repos/a/b/releases/latest")
            .expect("갈라 읽어야 한다");
        assert_eq!(parts.host, "api.github.com");
        assert_eq!(parts.path, "/repos/a/b/releases/latest");
        assert_eq!(parts.port, 443);
        assert!(parts.secure);
    }

    #[test]
    fn 포트를_적으면_그것을_쓴다() {
        let parts = split_url("https://example.com:8443/x").expect("갈라 읽어야 한다");
        assert_eq!(parts.host, "example.com");
        assert_eq!(parts.port, 8443);
        assert_eq!(parts.path, "/x");
    }

    #[test]
    fn 질의는_경로에_함께_담긴다() {
        // WinHTTP의 「객체 이름」이 물음표 뒤까지 함께 받는다 — 여기서 잘라내면 질의가 사라진다
        let parts = split_url("https://example.com/search?q=1&r=2").expect("갈라 읽어야 한다");
        assert_eq!(parts.path, "/search?q=1&r=2");
    }

    #[test]
    fn 경로가_없으면_뿌리로_본다() {
        let parts = split_url("https://example.com").expect("갈라 읽어야 한다");
        assert_eq!(parts.path, "/");
    }

    #[test]
    fn http는_갈라_읽되_안전하지_않다고_표시한다() {
        // 파싱은 하고 요청을 막는 것은 `read_response`다 — 어느 쪽이 막았는지 구분하기 위함
        let parts = split_url("http://example.com/x").expect("갈라 읽어야 한다");
        assert!(!parts.secure);
        assert_eq!(parts.port, 80);
    }

    #[test]
    fn 읽지_못하는_주소는_없음을_돌려준다() {
        assert_eq!(split_url(""), None);
        assert_eq!(split_url("example.com/x"), None, "스킴이 없다");
        assert_eq!(
            split_url("ftp://example.com/x"),
            None,
            "다루지 않는 스킴이다"
        );
        assert_eq!(split_url("https://"), None, "호스트가 없다");
        assert_eq!(split_url("https:///x"), None, "호스트가 비었다");
        assert_eq!(
            split_url("https://example.com:99999/x"),
            None,
            "포트가 범위를 넘는다"
        );
    }
}
