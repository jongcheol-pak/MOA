//! 내려받은 설치 파일의 SHA256 (FR-62 무결성 대조).
//!
//! **새 패키지를 들이지 않는다** — DPAPI·사이트 봉투 때문에 이미 켜 둔 CNG
//! (`Win32_Security_Cryptography`)에 해시 함수가 함께 들어 있다. `remote::envelope`가
//! 같은 계열로 PBKDF2·AES-GCM을 쓰고 있어 핸들을 감싸는 방식도 그쪽 관례를 따른다.
//!
//! **알고리즘을 고를 수 있게 만들지 않는다** — 대조할 값이 SHA256 하나뿐이라
//! 트레이트나 인자를 두면 쓰이지 않는 갈래만 는다.
use crate::remote::envelope::to_hex;
use std::path::Path;
use windows::Win32::Security::Cryptography::{
    BCRYPT_ALG_HANDLE, BCRYPT_HASH_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
    BCRYPT_SHA256_ALGORITHM, BCryptCloseAlgorithmProvider, BCryptCreateHash, BCryptDestroyHash,
    BCryptFinishHash, BCryptHashData, BCryptOpenAlgorithmProvider,
};
use windows::core::PCWSTR;

/// SHA256 결과 길이 (바이트)
const DIGEST_LEN: usize = 32;

/// 한 번에 읽어 넣는 크기 — 설치 파일이 수십 MB라도 메모리가 이만큼만 든다
const CHUNK: usize = 64 * 1024;

/// 파일의 SHA256을 소문자 hex로 돌려준다. 읽지 못하거나 CNG가 실패하면 `None`.
///
/// 파일을 통째로 메모리에 올리지 않고 조각마다 넣는다
pub fn file_sha256(path: &Path) -> Option<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path).ok()?;
    let algorithm = AlgHandle::open()?;
    let hash = HashHandle::create(&algorithm)?;

    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        // 안전성: 살아 있는 해시 핸들과, 길이를 함께 넘기는 우리 소유 버퍼다
        let status = unsafe { BCryptHashData(hash.0, &buffer[..read], 0) };
        if status.0 != 0 {
            return None;
        }
    }

    let mut digest = [0u8; DIGEST_LEN];
    // 안전성: 같은 해시 핸들과, SHA256 길이에 맞춘 우리 소유 버퍼다.
    // 이 호출 뒤 핸들은 더 쓸 수 없으며 `Drop`이 없앤다
    let status = unsafe { BCryptFinishHash(hash.0, &mut digest, 0) };
    if status.0 != 0 {
        return None;
    }
    Some(to_hex(&digest))
}

/// 기대값과 실제 값이 같은가 — 대소문자와 앞뒤 공백을 무시하고 견준다.
///
/// 릴리즈 노트는 사람이 쓰는 글이라 값이 대문자로 적히거나 줄 끝에 공백이 붙는다
pub fn matches(expected: &str, actual: &str) -> bool {
    expected.trim().eq_ignore_ascii_case(actual.trim())
}

/// 알고리즘 제공자 핸들 — 떨어질 때 반드시 닫는다
struct AlgHandle(BCRYPT_ALG_HANDLE);

impl AlgHandle {
    fn open() -> Option<AlgHandle> {
        let mut handle = BCRYPT_ALG_HANDLE::default();
        // 안전성: 출력 핸들만 받는 호출이다. 성공하면 `Drop`이 닫고, 실패하면 채워지지 않는다
        let status = unsafe {
            BCryptOpenAlgorithmProvider(
                &mut handle,
                BCRYPT_SHA256_ALGORITHM,
                PCWSTR::null(),
                BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
            )
        };
        (status.0 == 0).then_some(AlgHandle(handle))
    }
}

impl Drop for AlgHandle {
    fn drop(&mut self) {
        // 안전성: 열기에 성공한 핸들만 이 타입이 된다. 닫기 실패는 알릴 곳이 없다
        let _ = unsafe { BCryptCloseAlgorithmProvider(self.0, 0) };
    }
}

/// 해시 오브젝트 핸들 — 떨어질 때 반드시 없앤다.
///
/// **오브젝트 버퍼를 우리가 잡지 않는다** — Windows 8부터 `pbHashObject`를 주지 않으면
/// CNG가 스스로 잡고 `BCryptDestroyHash`가 함께 돌려준다. 이 앱은 Windows 11 전용이라
/// 그 경로만 쓴다(`remote::envelope`의 키 핸들과 같은 판단)
struct HashHandle(BCRYPT_HASH_HANDLE);

impl HashHandle {
    fn create(algorithm: &AlgHandle) -> Option<HashHandle> {
        let mut handle = BCRYPT_HASH_HANDLE::default();
        // 안전성: 살아 있는 제공자 핸들을 넘기고 출력 핸들만 받는다. 오브젝트 버퍼와
        // 비밀 키는 주지 않는다(각각 CNG 자체 할당·해시에 불필요)
        let status = unsafe { BCryptCreateHash(algorithm.0, &mut handle, None, None, 0) };
        (status.0 == 0).then_some(HashHandle(handle))
    }
}

impl Drop for HashHandle {
    fn drop(&mut self) {
        // 안전성: 만들기에 성공한 핸들만 이 타입이 된다
        let _ = unsafe { BCryptDestroyHash(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 시험용 임시 파일 — 이름에 프로세스 번호를 넣어 병렬 실행이 서로를 밟지 않게 한다
    fn temp_file(label: &str, contents: &[u8]) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("moa_sha256_{label}_{}.bin", std::process::id()));
        std::fs::write(&path, contents).expect("임시 파일 쓰기");
        path
    }

    #[test]
    fn 빈_파일은_알려진_값을_낸다() {
        let path = temp_file("empty", b"");
        let digest = file_sha256(&path).expect("계산해야 한다");
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn abc는_알려진_값을_낸다() {
        let path = temp_file("abc", b"abc");
        let digest = file_sha256(&path).expect("계산해야 한다");
        let _ = std::fs::remove_file(&path);
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn 조각_경계를_넘는_파일도_한_번에_읽은_것과_같다() {
        // 64KB 조각을 넘겨야 여러 번의 `BCryptHashData`가 이어 붙는지 확인된다 —
        // 한 조각에 담기는 파일만 시험하면 그 이어 붙임이 검증되지 않는다
        let contents: Vec<u8> = (0..(CHUNK * 2 + 123)).map(|i| (i % 251) as u8).collect();
        let path = temp_file("large", &contents);
        let digest = file_sha256(&path).expect("계산해야 한다");
        let _ = std::fs::remove_file(&path);

        // 같은 내용을 한 조각으로 넣어 견준다
        let algorithm = AlgHandle::open().expect("제공자");
        let hash = HashHandle::create(&algorithm).expect("해시");
        let status = unsafe { BCryptHashData(hash.0, &contents, 0) };
        assert_eq!(status.0, 0);
        let mut once = [0u8; DIGEST_LEN];
        let status = unsafe { BCryptFinishHash(hash.0, &mut once, 0) };
        assert_eq!(status.0, 0);

        assert_eq!(digest, to_hex(&once));
        assert_eq!(digest.len(), 64, "hex는 언제나 64자다");
    }

    #[test]
    fn 없는_파일은_없음을_돌려준다() {
        let path = std::env::temp_dir().join(format!("moa_sha256_없음_{}.bin", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(file_sha256(&path), None);
    }

    #[test]
    fn 대소문자와_공백을_무시하고_견준다() {
        let value = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(matches(&value.to_ascii_uppercase(), value));
        assert!(matches(&format!("  {value}\n"), value));
        assert!(!matches(value, &value.replace('a', "b")));
    }
}
