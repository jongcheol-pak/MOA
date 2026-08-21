//! GitHub 릴리즈 조회와 판정 (FR-62).
//!
//! 최신 릴리즈를 물어 **지금 판보다 새 것인지**, 그 릴리즈에 **우리 설치 파일이 있는지**,
//! 본문에 **대조할 체크섬이 적혀 있는지**를 가린다.
//!
//! **릴리즈 제공자를 추상화하지 않는다** — 저장소는 하나이고 앞으로도 하나다.
use super::http::{self, HttpError};
use serde::Deserialize;

/// 최신 릴리즈를 묻는 주소. 저장소가 하나라 상수로 둔다
const LATEST_URL: &str = "https://api.github.com/repos/jongcheol-pak/MOA/releases/latest";

/// GitHub API가 판을 가르는 데 쓰는 값
const ACCEPT: &str = "application/vnd.github+json";

/// 설치 파일 이름의 앞머리 — 생성기가 `MOA-Setup-<버전>.exe`로 만든다
/// (`installer/moa.nsi`의 `OutFile`)
const ASSET_PREFIX: &str = "MOA-Setup-";
const ASSET_SUFFIX: &str = ".exe";

/// SHA256을 hex로 적으면 이 길이다
const SHA256_HEX_LEN: usize = 64;

/// 업데이트 확인이 실패할 수 있는 사유
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateError {
    /// 서버에 닿지 못했거나 응답이 정상이 아니다
    Network,
    /// 응답을 읽었지만 우리가 아는 형태가 아니다
    BadResponse,
    /// 릴리즈에 우리 설치 파일이 없다
    NoAsset,
    /// 릴리즈 본문에 대조할 체크섬이 없다
    NoChecksum,
    /// 받은 파일이 체크섬과 다르다
    ChecksumMismatch,
    /// 내려받기·파일 쓰기에 실패했다
    Download,
    /// 설치 프로그램을 띄우지 못했다
    LaunchFailed,
}

impl From<HttpError> for UpdateError {
    fn from(error: HttpError) -> UpdateError {
        match error {
            // 파일 쓰기 실패만 갈라 둔다 — 사용자가 할 수 있는 일이 다르다(디스크 공간·권한)
            HttpError::Write => UpdateError::Download,
            _ => UpdateError::Network,
        }
    }
}

/// 내려받아 설치할 준비가 된 릴리즈 하나
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    /// `v` 접두사를 뗀 버전 (`0.2.0`)
    pub version: String,
    pub asset_name: String,
    pub asset_url: String,
    /// 릴리즈 본문에서 뽑은 기대 SHA256 (소문자 hex)
    pub sha256: String,
}

/// 최신 릴리즈를 물어 **지금 판보다 새 것이면** 그 정보를 돌려준다.
///
/// 새 것이 없으면 `Ok(None)` — 오류가 아니다. 릴리즈가 하나도 없는 저장소도 여기로 온다
pub fn fetch_latest() -> Result<Option<ReleaseInfo>, UpdateError> {
    let body = match http::get_bytes(LATEST_URL, ACCEPT) {
        Ok(body) => body,
        // 릴리즈가 하나도 없으면 GitHub이 404를 준다 — 그것은 실패가 아니라 「없음」이다
        Err(HttpError::Status(404)) => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let text = String::from_utf8(body).map_err(|_| UpdateError::BadResponse)?;
    parse_release(&text, env!("CARGO_PKG_VERSION"))
}

/// 응답 본문을 읽어 판정한다 — 네트워크를 타지 않아 그대로 시험할 수 있다.
///
/// `current`는 지금 도는 판이다(호출부는 `CARGO_PKG_VERSION`을 넘긴다)
pub fn parse_release(json: &str, current: &str) -> Result<Option<ReleaseInfo>, UpdateError> {
    let release: Release = serde_json::from_str(json).map_err(|_| UpdateError::BadResponse)?;
    let version = strip_v(&release.tag_name).to_owned();
    if !is_newer(&version, current) {
        return Ok(None);
    }
    let asset = pick_asset(&release.assets).ok_or(UpdateError::NoAsset)?;
    let sha256 = extract_sha256(&release.body).ok_or(UpdateError::NoChecksum)?;
    Ok(Some(ReleaseInfo {
        version,
        asset_name: asset.name.clone(),
        asset_url: asset.browser_download_url.clone(),
        sha256,
    }))
}

/// 응답에서 우리가 쓰는 것만 받는다 — 나머지 필드는 serde가 그냥 지나친다
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    /// 릴리즈 노트 본문. 없는 릴리즈도 있어 기본값을 둔다
    #[serde(default)]
    body: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

/// `v0.2.0` → `0.2.0`. 접두사가 없으면 그대로
fn strip_v(tag: &str) -> &str {
    tag.strip_prefix('v').unwrap_or(tag)
}

/// `latest`가 `current`보다 새 판인가.
///
/// 마디를 점으로 갈라 **수로** 견준다 — 글자로 견주면 `0.1.10`이 `0.1.9`보다 작아진다.
/// 수로 읽지 못하는 마디(`1.0.0-rc1`의 `0-rc1` 등)는 0으로 본다
pub fn is_newer(latest: &str, current: &str) -> bool {
    let latest = version_parts(strip_v(latest));
    let current = version_parts(strip_v(current));
    let len = latest.len().max(current.len());
    for i in 0..len {
        // 적히지 않은 뒷마디는 0이다 — `1.1`과 `1.1.0`은 같은 판이다
        let a = latest.get(i).copied().unwrap_or(0);
        let b = current.get(i).copied().unwrap_or(0);
        if a != b {
            return a > b;
        }
    }
    false
}

fn version_parts(version: &str) -> Vec<u32> {
    version
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0))
        .collect()
}

/// 릴리즈에 딸린 것 중 우리 설치 파일을 고른다.
///
/// **이름으로 가린다** — 체크섬 파일(`....exe.sha256`)이나 GitHub이 자동으로 붙이는
/// 소스 묶음이 함께 있어도 그것을 받지 않는다
fn pick_asset(assets: &[Asset]) -> Option<&Asset> {
    assets
        .iter()
        .find(|asset| asset.name.starts_with(ASSET_PREFIX) && asset.name.ends_with(ASSET_SUFFIX))
}

/// 릴리즈 노트 본문에서 SHA256을 뽑는다 — 64자 hex 토막을 첫 번째로 만나는 것으로 본다.
///
/// **본문 어디에 적혀 있어도 찾는다** — 마크다운 태그·백틱·파일 이름은 전부 hex가 아닌
/// 글자라 구분자가 된다. 그래서 값을 접어 두어도(`<details>`) 그대로 읽힌다.
/// 정규식을 쓰지 않는 이유는 그 크레이트를 이 하나 때문에 들일 이유가 없기 때문이다
pub fn extract_sha256(body: &str) -> Option<String> {
    body.split(|c: char| !c.is_ascii_hexdigit())
        .find(|token| token.len() == SHA256_HEX_LEN)
        .map(|token| token.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    /// D15가 정한 릴리즈 노트 형식 그대로의 본문 — 사용자용 요약 아래 체크섬이 접혀 있다
    fn 실제_형식_본문() -> String {
        format!(
            "### 새로워진 것\n\
             - 자동 업데이트 — 새 버전이 나오면 제목 줄에 알립니다.\n\
             - 릴리즈 노트 — 설정 메뉴에서 이 페이지를 바로 열 수 있습니다.\n\n\
             <details><summary>파일 무결성 확인용 (SHA256)</summary>\n\n\
             MOA-Setup-0.2.0.exe\n`{HASH}`\n</details>\n"
        )
    }

    fn 응답(tag: &str, body: &str, assets: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","body":{body:?},"assets":[{assets}]}}"#)
    }

    fn 자산(name: &str) -> String {
        format!(
            r#"{{"name":"{name}","browser_download_url":"https://example.com/{name}","size":1}}"#
        )
    }

    #[test]
    fn 새_판이면_정보를_돌려준다() {
        let json = 응답("v0.2.0", &실제_형식_본문(), &자산("MOA-Setup-0.2.0.exe"));
        let info = parse_release(&json, "0.1.0")
            .expect("읽어야 한다")
            .expect("새 판이다");
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.asset_name, "MOA-Setup-0.2.0.exe");
        assert_eq!(info.asset_url, "https://example.com/MOA-Setup-0.2.0.exe");
        assert_eq!(info.sha256, HASH);
    }

    #[test]
    fn 같거나_낮은_판이면_없음이다() {
        let json = 응답("v0.1.0", &실제_형식_본문(), &자산("MOA-Setup-0.1.0.exe"));
        assert_eq!(parse_release(&json, "0.1.0"), Ok(None), "같은 판");
        assert_eq!(parse_release(&json, "0.2.0"), Ok(None), "지금이 더 높다");
    }

    #[test]
    fn 형태가_아니면_읽지_못했다고_한다() {
        assert_eq!(parse_release("{", "0.1.0"), Err(UpdateError::BadResponse));
        assert_eq!(
            parse_release(r#"{"body":"x"}"#, "0.1.0"),
            Err(UpdateError::BadResponse),
            "tag_name이 없다"
        );
    }

    #[test]
    fn 설치_파일이_없으면_그렇게_알린다() {
        let json = 응답(
            "v0.2.0",
            &실제_형식_본문(),
            &자산("MOA-Setup-0.2.0.exe.sha256"),
        );
        assert_eq!(parse_release(&json, "0.1.0"), Err(UpdateError::NoAsset));

        let json = 응답("v0.2.0", &실제_형식_본문(), "");
        assert_eq!(parse_release(&json, "0.1.0"), Err(UpdateError::NoAsset));
    }

    #[test]
    fn 체크섬이_없으면_받지_않는다() {
        // 대조할 값이 없는 릴리즈는 「검증 없이 진행」이 아니라 거절이다 (plan D3)
        let json = 응답(
            "v0.2.0",
            "### 새로워진 것\n- 뭔가 좋아졌습니다.",
            &자산("MOA-Setup-0.2.0.exe"),
        );
        assert_eq!(parse_release(&json, "0.1.0"), Err(UpdateError::NoChecksum));
    }

    #[test]
    fn 버전은_글자가_아니라_수로_견준다() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(
            is_newer("0.1.10", "0.1.9"),
            "글자로 견주면 10 < 9로 뒤집힌다"
        );
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
    }

    #[test]
    fn v_접두사는_있으나_없으나_같다() {
        assert!(is_newer("v0.2.0", "0.1.0"));
        assert!(is_newer("0.2.0", "v0.1.0"));
        assert!(!is_newer("v0.1.0", "v0.1.0"));
    }

    #[test]
    fn 적히지_않은_뒷마디는_0으로_본다() {
        assert!(!is_newer("1.1", "1.1.0"), "같은 판이다");
        assert!(is_newer("1.1.1", "1.1"));
    }

    #[test]
    fn 수로_읽지_못하는_마디는_0으로_본다() {
        // 형식이 깨진 값에 패닉하지 않는 것이 목적이다 — 그런 릴리즈는 새 판으로 보지 않는다
        assert!(!is_newer("어쩌고", "0.1.0"));
        assert!(!is_newer("1.0.0-rc1", "1.0.0"));
    }

    #[test]
    fn 체크섬은_본문_어디에_있든_찾는다() {
        assert_eq!(extract_sha256(&실제_형식_본문()).as_deref(), Some(HASH));
        assert_eq!(
            extract_sha256(&format!("맨 앞에 있어도 {HASH} 찾는다")).as_deref(),
            Some(HASH)
        );
        assert_eq!(
            extract_sha256(&HASH.to_ascii_uppercase()).as_deref(),
            Some(HASH),
            "대문자로 적혀도 소문자로 돌려준다"
        );
    }

    #[test]
    fn 체크섬이_아닌_것은_뽑지_않는다() {
        assert_eq!(extract_sha256(""), None);
        assert_eq!(extract_sha256("체크섬이 없는 본문"), None);
        assert_eq!(extract_sha256(&HASH[..63]), None, "63자는 아니다");
        assert_eq!(
            extract_sha256(&format!("{HASH}0")),
            None,
            "65자짜리는 잘라 쓰지 않는다"
        );
        assert_eq!(
            extract_sha256("MOA-Setup-0.2.0.exe <details><summary>x</summary>"),
            None,
            "파일 이름·태그에는 64자 hex가 없다"
        );
    }
}
