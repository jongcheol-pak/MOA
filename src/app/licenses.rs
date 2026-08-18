//! 오픈소스 라이선스 고지 자산 (FR-57).
//!
//! `assets/licenses.json`은 `cargo run --example gen_licenses`가 만드는 **생성물**이고 이
//! 모듈은 그것을 읽기만 한다. 자산을 exe에 정적으로 담는 이유는 고지가 오프라인에서도
//! 보여야 하고, 빌드가 레지스트리 캐시나 네트워크를 타면 안 되기 때문이다.
//!
//! 의존성이 바뀌었는데 자산을 다시 만들지 않으면 화면의 고지가 실제와 어긋난다. 그것을
//! 잡으려고 자산에 `Cargo.lock`의 지문을 함께 담고, 시험이 현재 lock과 대조한다
//! (`lockfile_fingerprint`).
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 자산 형식 버전 — 필드 구성이 바뀌면 올린다.
pub const SCHEMA_VERSION: u32 = 1;

/// exe에 담기는 자산 원본. 생성기가 이 파일을 덮어쓴다
const ASSET: &str = include_str!("../../assets/licenses.json");

/// 고지 자산 전체.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseData {
    pub schema: u32,
    /// 자산을 만든 시점의 `Cargo.lock` 지문 (`lockfile_fingerprint`)
    pub lock_fingerprint: u64,
    /// 이름순으로 정렬된 구성 요소 — 정렬은 생성기가 해서 담는다
    pub crates: Vec<CrateEntry>,
    /// 중복을 걷어낸 라이선스 전문. 같은 내용은 한 번만 담기고 여럿이 함께 가리킨다
    pub texts: Vec<LicenseText>,
}

/// 구성 요소 하나.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateEntry {
    pub name: String,
    pub version: String,
    /// 선언된 SPDX 식별자 원문 — `MIT OR Apache-2.0`처럼 선택형도 그대로 담는다
    pub spdx: String,
    #[serde(default)]
    pub authors: Vec<String>,
    /// `LicenseData::texts`를 가리키는 자리 번호들
    pub text_indices: Vec<usize>,
    /// 배포 패키지에 원문이 없어 SPDX 표준 전문을 대신 보이는 항목
    #[serde(default)]
    pub standard_text: bool,
    /// 크레이트가 아니라 함께 링크·동봉되는 것(번들 C 소스·글꼴)
    #[serde(default)]
    pub bundled: bool,
}

/// 라이선스 전문 한 벌.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseText {
    /// 이 전문이 무엇인지 알려 주는 이름 — 화면에서 전문 위에 붙는다
    pub spdx: String,
    pub body: String,
}

impl CrateEntry {
    /// 이 항목이 가리키는 전문들을 `text_indices` 차례대로 돌려준다.
    ///
    /// 자리 번호가 범위를 벗어나면 그 자리만 건너뛴다 — 자산이 어긋나도 화면이 비지 않게
    /// 하려는 것이고, 그런 자산은 지문 대조 시험이 따로 잡는다
    pub fn texts<'a>(&self, data: &'a LicenseData) -> Vec<&'a LicenseText> {
        self.text_indices
            .iter()
            .filter_map(|&index| data.texts.get(index))
            .collect()
    }
}

/// 자산 문자열을 읽는다. 형식이 깨져 있으면 **빈 자산**을 준다.
///
/// 실패를 오류로 올리지 않는 이유: 이 자산은 사용자 데이터가 아니라 빌드에 함께 담기는
/// 생성물이라, 깨졌다면 그것은 개발 시점 문제다. 화면은 빈 목록 안내를 보이면 되고
/// 깨짐 자체는 시험이 잡는다
pub fn parse(json: &str) -> LicenseData {
    serde_json::from_str(json).unwrap_or_default()
}

/// exe에 담긴 자산 — 처음 부를 때 한 번만 읽는다.
///
/// 시작할 때가 아니라 **라이선스 대화를 처음 열 때** 파싱되도록 늦춘다(NFR-1 시작 시간)
pub fn load() -> &'static LicenseData {
    static DATA: OnceLock<LicenseData> = OnceLock::new();
    DATA.get_or_init(|| parse(ASSET))
}

// ── Cargo.lock 지문 ──

/// FNV-1a 64비트 초깃값
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a 64비트 소수
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// `Cargo.lock`의 지문 — 이름·버전 줄만 보고 FNV-1a 64로 접는다.
///
/// **줄 전체가 아니라 `name`·`version`만 보는 이유**: lock 파일은 cargo 버전에 따라 주석
/// 머리말·`checksum` 표기가 달라지는데, 그것이 바뀌었다고 라이선스 고지가 달라지지는
/// 않는다. 반대로 **버전이 하나라도 바뀌면 원문이 달라질 수 있어** 지문도 달라져야 한다.
///
/// 해시를 직접 쓰는 것은 std에 해시 함수가 없고 이 하나를 위해 의존성을 더하지 않기
/// 위해서다(충돌을 노린 공격을 막는 용도가 아니라 갱신 누락을 알아채는 용도다)
pub fn lockfile_fingerprint(lock: &str) -> u64 {
    let mut hash = FNV_OFFSET;
    for line in lock.lines() {
        let line = line.trim();
        if !line.starts_with("name = ") && !line.starts_with("version = ") {
            continue;
        }
        for byte in line.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        // 줄 경계를 섞는다 — 없으면 `name = "ab"` + `version = "c"`와
        // `name = "a"` + `version = "bc"`가 같은 값이 된다
        hash ^= u64::from(b'\n');
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 자산이_깨져_있으면_빈_자료를_준다() {
        assert_eq!(parse("{ 이건 JSON이 아니다"), LicenseData::default());
        assert_eq!(parse(""), LicenseData::default());
    }

    #[test]
    fn 담긴_자산을_읽을_수_있다() {
        let data = load();
        assert_eq!(data.schema, SCHEMA_VERSION);
    }

    #[test]
    fn 전문_자리_번호가_범위를_벗어나면_그_자리만_건너뛴다() {
        let data = LicenseData {
            schema: SCHEMA_VERSION,
            lock_fingerprint: 0,
            crates: vec![CrateEntry {
                name: "foo".into(),
                version: "1.0.0".into(),
                spdx: "MIT".into(),
                authors: Vec::new(),
                text_indices: vec![0, 9, 1],
                standard_text: false,
                bundled: false,
            }],
            texts: vec![
                LicenseText {
                    spdx: "MIT".into(),
                    body: "첫째".into(),
                },
                LicenseText {
                    spdx: "Apache-2.0".into(),
                    body: "둘째".into(),
                },
            ],
        };
        let bodies: Vec<&str> = data.crates[0]
            .texts(&data)
            .iter()
            .map(|text| text.body.as_str())
            .collect();
        assert_eq!(bodies, ["첫째", "둘째"]);
    }

    #[test]
    fn 지문은_공백과_주석_차이를_무시한다() {
        let plain = "\
[[package]]
name = \"alpha\"
version = \"1.0.0\"
";
        let noisy = "\
# 이 파일은 cargo가 만든다
[[package]]
  name = \"alpha\"
\tversion = \"1.0.0\"
source = \"registry+https://example.invalid\"
checksum = \"deadbeef\"
";
        assert_eq!(lockfile_fingerprint(plain), lockfile_fingerprint(noisy));
    }

    #[test]
    fn 지문은_버전이_하나만_달라도_바뀐다() {
        let before = "name = \"alpha\"\nversion = \"1.0.0\"\n";
        let after = "name = \"alpha\"\nversion = \"1.0.1\"\n";
        assert_ne!(lockfile_fingerprint(before), lockfile_fingerprint(after));
    }

    #[test]
    fn 버전이_없는_항목은_이름만_지문에_든다() {
        let with_version = "name = \"alpha\"\nversion = \"1.0.0\"\n";
        let name_only = "name = \"alpha\"\n";
        assert_ne!(
            lockfile_fingerprint(with_version),
            lockfile_fingerprint(name_only)
        );
        // 경로 의존처럼 버전 줄이 없어도 이름은 그대로 반영된다
        assert_ne!(lockfile_fingerprint(name_only), FNV_OFFSET);
    }
}
