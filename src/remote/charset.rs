//! 원격 파일명 인코딩 (FR-46·D23).
//!
//! 서버가 파일명을 어떤 문자셋으로 주고받는지는 표준이 정해 주지 않는다 — 옛 FTP 서버는
//! 그 지역 코드 페이지(한국이면 CP949)를 그대로 쓴다. 사이트 관리자의 `문자셋` 탭이 그것을
//! 지정하면 이 모듈이 바이트 ↔ 문자열을 옮긴다.
//!
//! **새 패키지를 들이지 않는다** (D23·4-D ⑥):
//! - UTF-8·Latin-1(ISO-8859-1)은 규칙이 단순해 순수 Rust로 옮긴다
//! - CP949는 표가 1만 7천 자를 넘어 손으로 담을 것이 못 되므로 **Windows 코드 페이지 변환**
//!   (`MultiByteToWideChar`/`WideCharToMultiByte`)에 맡긴다. 어차피 Windows 전용 앱이다
//!
//! 모르는 이름을 적었으면 **UTF-8로 폴백**하고 그 사실을 `is_known`으로 알린다 — 조용히
//! 다른 인코딩으로 처리하면 파일명이 깨진 채로 굳는다.
use crate::remote::types::Charset;

/// 알아듣는 코드 페이지 — 이름은 대소문자·구분자를 무시하고 견준다.
///
/// FileZilla가 쓰는 표기(`CP949`)와 서버가 알리는 표기(`EUC-KR`·`KS_C_5601-1987`)가 달라
/// 한 줄로 묶는다. 목록에 없으면 UTF-8 폴백이다 (D23)
const CP949_NAMES: [&str; 5] = ["CP949", "EUCKR", "MS949", "KSC56011987", "UHC"];
const LATIN1_NAMES: [&str; 4] = ["LATIN1", "ISO88591", "CP28591", "L1"];
const UTF8_NAMES: [&str; 3] = ["UTF8", "UTF", "65001"];

/// CP949(통합 완성형) Windows 코드 페이지 번호
const CP949: u32 = 949;

/// 인코딩 이름이 가리키는 처리 방식
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Latin1,
    /// Windows 코드 페이지에 맡긴다
    CodePage(u32),
}

/// 이름에서 구분자를 떼고 대문자로 맞춘다 — `euc-kr`·`EUC_KR`·`euckr`가 모두 같은 것이다
fn normalize(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// 이 문자셋을 실제로 알아듣는가 — 모르면 UTF-8로 처리하되 화면이 그 사실을 알린다
pub fn is_known(charset: &Charset) -> bool {
    match charset {
        Charset::Utf8 => true,
        Charset::Named(name) => resolve(name).is_some(),
    }
}

/// 이름 → 처리 방식. 모르는 이름·빈 이름은 `None`이다 (호출부가 UTF-8로 폴백한다)
fn resolve(name: &str) -> Option<Encoding> {
    let key = normalize(name);
    if key.is_empty() {
        return None;
    }
    if UTF8_NAMES.contains(&key.as_str()) {
        return Some(Encoding::Utf8);
    }
    if LATIN1_NAMES.contains(&key.as_str()) {
        return Some(Encoding::Latin1);
    }
    if CP949_NAMES.contains(&key.as_str()) {
        return Some(Encoding::CodePage(CP949));
    }
    None
}

fn encoding_of(charset: &Charset) -> Encoding {
    match charset {
        Charset::Utf8 => Encoding::Utf8,
        // 모르는 이름·빈 값은 UTF-8이다 (plan Edge Case)
        Charset::Named(name) => resolve(name).unwrap_or(Encoding::Utf8),
    }
}

/// 서버가 준 바이트를 화면에 보일 문자열로.
///
/// 옮길 수 없는 바이트는 **치환 문자(`U+FFFD`)로 보인다** — 오류로 만들어 목록을 통째로
/// 버리면 나머지 파일까지 보이지 않는다
pub fn decode_name(bytes: &[u8], charset: &Charset) -> String {
    match encoding_of(charset) {
        Encoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        // Latin-1은 바이트 값이 곧 유니코드 부호점이다 (U+0000~U+00FF)
        Encoding::Latin1 => bytes.iter().map(|b| char::from(*b)).collect(),
        Encoding::CodePage(page) => decode_code_page(bytes, page),
    }
}

/// 화면·명령에 쓸 문자열을 서버가 알아들을 바이트로.
///
/// 그 인코딩에 없는 글자는 대체 문자(`?`)가 된다 — Windows 변환이 그렇게 정하며,
/// 우리가 임의로 비슷한 글자를 골라 넣지 않는다(서버가 다른 파일을 가리키게 된다)
pub fn encode_name(name: &str, charset: &Charset) -> Vec<u8> {
    match encoding_of(charset) {
        Encoding::Utf8 => name.as_bytes().to_vec(),
        Encoding::Latin1 => name
            .chars()
            .map(|c| if (c as u32) <= 0xFF { c as u8 } else { b'?' })
            .collect(),
        Encoding::CodePage(page) => encode_code_page(name, page),
    }
}

/// Windows 코드 페이지 → UTF-16 → `String`.
///
/// # 안전성
/// `MultiByteToWideChar`는 길이를 바이트/문자 수로 받고 버퍼 밖을 건드리지 않는다.
/// 필요한 길이를 먼저 물어(출력 버퍼 없이 호출) 그만큼만 잡아 두 번째 호출에 넘긴다
fn decode_code_page(bytes: &[u8], page: u32) -> String {
    use windows::Win32::Globalization::MultiByteToWideChar;
    if bytes.is_empty() {
        return String::new();
    }
    // 첫 호출은 필요한 UTF-16 길이만 알아본다
    let needed = unsafe { MultiByteToWideChar(page, Default::default(), bytes, None) };
    if needed <= 0 {
        // 코드 페이지가 없는 시스템이거나 옮길 수 없는 바이트열이다 — UTF-8로 물러선다
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let mut wide = vec![0u16; needed as usize];
    let written =
        unsafe { MultiByteToWideChar(page, Default::default(), bytes, Some(wide.as_mut_slice())) };
    if written <= 0 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    String::from_utf16_lossy(&wide[..written as usize])
}

/// `String` → UTF-16 → Windows 코드 페이지 바이트.
///
/// # 안전성
/// `decode_code_page`와 같다 — 길이를 먼저 물어 그만큼만 잡는다.
/// 대체 문자 지정(`lpdefaultchar`)은 기본값에 맡긴다(`?`)
fn encode_code_page(name: &str, page: u32) -> Vec<u8> {
    use windows::Win32::Globalization::WideCharToMultiByte;
    if name.is_empty() {
        return Vec::new();
    }
    let wide: Vec<u16> = name.encode_utf16().collect();
    let needed = unsafe { WideCharToMultiByte(page, 0, &wide, None, None, None) };
    if needed <= 0 {
        return name.as_bytes().to_vec();
    }
    let mut out = vec![0u8; needed as usize];
    let written =
        unsafe { WideCharToMultiByte(page, 0, &wide, Some(out.as_mut_slice()), None, None) };
    if written <= 0 {
        return name.as_bytes().to_vec();
    }
    out.truncate(written as usize);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `한글.txt`의 CP949 바이트 — `한`=0xC7 0xD1, `글`=0xB1 0xDB
    const HANGUL_CP949: [u8; 8] = [0xC7, 0xD1, 0xB1, 0xDB, b'.', b't', b'x', b't'];

    #[test]
    fn cp949_한글_파일명이_옳게_디코딩되고_왕복한다() {
        // Acceptance ⑦ — 이 변환이 틀리면 CP949 서버의 목록이 통째로 깨져 보인다
        let charset = Charset::Named("CP949".to_owned());
        let name = decode_name(&HANGUL_CP949, &charset);
        assert_eq!(name, "한글.txt");
        assert_eq!(encode_name(&name, &charset), HANGUL_CP949.to_vec());
    }

    #[test]
    fn 인코딩_이름은_표기_차이를_가리지_않는다() {
        // 서버·사용자가 적는 표기가 제각각이다 — `euc-kr`과 `CP949`는 같은 것이다
        for name in ["cp949", "CP-949", "euc-kr", "EUC_KR", "ms949", "UHC"] {
            let charset = Charset::Named(name.to_owned());
            assert!(is_known(&charset), "{name}을 알아듣지 못했다");
            assert_eq!(decode_name(&HANGUL_CP949, &charset), "한글.txt", "{name}");
        }
    }

    #[test]
    fn 모르는_이름은_utf8로_물러서고_그_사실을_알린다() {
        // plan Edge Case — 조용히 다른 인코딩으로 처리하면 파일명이 깨진 채로 굳는다
        let unknown = Charset::Named("셔프트-9".to_owned());
        assert!(!is_known(&unknown));
        assert_eq!(decode_name("한글.txt".as_bytes(), &unknown), "한글.txt");

        // 빈 값도 마찬가지다 (직접 설정을 골라 놓고 아무것도 적지 않은 경우)
        let empty = Charset::Named(String::new());
        assert!(!is_known(&empty));
        assert_eq!(encode_name("한글", &empty), "한글".as_bytes().to_vec());
    }

    #[test]
    fn utf8는_바이트를_그대로_주고받는다() {
        let charset = Charset::Utf8;
        assert!(is_known(&charset));
        assert_eq!(encode_name("한글.txt", &charset), "한글.txt".as_bytes());
        assert_eq!(decode_name("한글.txt".as_bytes(), &charset), "한글.txt");
    }

    #[test]
    fn latin1은_바이트_값이_곧_부호점이다() {
        let charset = Charset::Named("ISO-8859-1".to_owned());
        assert!(is_known(&charset));
        // 0xE9 = é
        assert_eq!(
            decode_name(&[0xE9, b'.', b't', b'x', b't'], &charset),
            "é.txt"
        );
        assert_eq!(
            encode_name("é.txt", &charset),
            vec![0xE9, b'.', b't', b'x', b't']
        );
        // 그 인코딩에 없는 글자는 대체 문자가 된다 — 비슷한 글자를 임의로 고르지 않는다
        assert_eq!(encode_name("한", &charset), vec![b'?']);
    }

    #[test]
    fn 옮길_수_없는_바이트는_치환_문자로_보인다() {
        // 목록을 통째로 버리면 나머지 파일까지 보이지 않는다 (plan Edge Case)
        let broken = [0xFF, 0xFE, b'a'];
        let decoded = decode_name(&broken, &Charset::Utf8);
        assert!(
            decoded.contains('\u{FFFD}'),
            "치환 문자가 없다: {decoded:?}"
        );
        assert!(decoded.ends_with('a'), "옮길 수 있는 부분은 남아야 한다");
    }

    #[test]
    fn 빈_이름은_빈_결과다() {
        let charset = Charset::Named("CP949".to_owned());
        assert_eq!(decode_name(&[], &charset), "");
        assert_eq!(encode_name("", &charset), Vec::<u8>::new());
    }
}
