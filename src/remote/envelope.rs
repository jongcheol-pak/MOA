//! 암호 기반 봉투 — 사이트 목록 내보내기 파일 전용 (FR-59).
//!
//! `remote::secret`(DPAPI)과 **쓰임이 다르다**. DPAPI는 지금 로그인한 사용자에게 묶인 키를 쓰므로
//! 다른 PC·다른 계정에서는 풀리지 않는데(그것이 설정 파일 보호에는 맞는 성질이다), 내보내기 파일은
//! 애초에 **다른 PC로 옮기려고** 만드는 것이라 그 성질이 그대로 걸림돌이 된다. 그래서 이쪽은
//! 사용자가 정한 암호에서 키를 파생한다 — 파일과 암호만 있으면 어디서든 풀린다.
//!
//! 알고리즘은 Windows CNG가 그대로 제공하는 것만 쓴다 (plan D2):
//! **PBKDF2-HMAC-SHA256으로 키를 파생하고 AES-256-GCM으로 봉한다.** 새 패키지를 들이지 않는
//! 이유가 그것이며, GCM의 인증 태그가 「틀린 암호」와 「손댄 파일」을 한 검사로 함께 걸러 준다.
//!
//! 키 저장소를 추상화하지 않는 것은 `secret`과 같은 판단이다 — 쓰는 조합이 하나뿐이라 갈래가 없다.
use serde::{Deserialize, Serialize};
use windows::Win32::Security::Cryptography::{
    BCRYPT_AES_ALGORITHM, BCRYPT_ALG_HANDLE, BCRYPT_ALG_HANDLE_HMAC_FLAG,
    BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO, BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO_VERSION,
    BCRYPT_CHAINING_MODE, BCRYPT_FLAGS, BCRYPT_KEY_HANDLE, BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS,
    BCRYPT_SHA256_ALGORITHM, BCRYPT_USE_SYSTEM_PREFERRED_RNG, BCryptCloseAlgorithmProvider,
    BCryptDecrypt, BCryptDeriveKeyPBKDF2, BCryptDestroyKey, BCryptEncrypt, BCryptGenRandom,
    BCryptGenerateSymmetricKey, BCryptOpenAlgorithmProvider, BCryptSetProperty,
};
use windows::core::PCWSTR;

/// 봉투가 쓰는 키 파생 방식 — 파일에 그대로 적히고, 풀 때 이 값이 아니면 거부한다.
///
/// 문자열로 두는 이유: 훗날 다른 방식이 생겨도 **옛 파일이 자기 방식을 스스로 밝힐 수 있다**.
/// 지금은 갈래가 하나뿐이라 열거형을 두지 않는다
pub const KDF_NAME: &str = "PBKDF2-HMAC-SHA256";

/// PBKDF2 반복 횟수 — OWASP가 PBKDF2-SHA256에 권하는 값.
///
/// 이 파생은 **UI 스레드에서 돈다**(plan D13). 그래서 값을 추측으로 고정하지 않고 실측했다 —
/// **2026-08-20 릴리즈 빌드에서 1회 0.126초**로, 상한 1.0초에 한참 못 미쳐 값을 낮추지 않았다.
/// 넘었다면 200,000회로 낮춘다는 것이 plan D13의 규칙이며, `파생_1회_시간을_잰다`가 그 상한을
/// 단언하므로 더 느린 기계에서는 시험이 먼저 알린다
const PBKDF2_ITERATIONS: u64 = 600_000;

/// 소금 길이 — 같은 암호라도 파일마다 다른 키가 나오게 한다
const SALT_LEN: usize = 16;
/// GCM nonce 길이 — 12바이트가 AES-GCM의 표준 길이다(그 밖의 길이는 내부에서 한 번 더 해싱된다)
const NONCE_LEN: usize = 12;
/// GCM 인증 태그 길이 — 최대값을 쓴다
const TAG_LEN: usize = 16;
/// AES-256 키 길이
const KEY_LEN: usize = 32;

/// 암호로 봉한 한 덩어리. 파일에 이 모습 그대로 실린다.
///
/// 이진 값은 전부 소문자 hex 문자열이다 (plan D9) — 담기는 것이 소금 16 + nonce 12 + 태그 16 +
/// 본문 수백 바이트뿐이라 base64의 크기 이점이 뜻이 없고, hex는 변환표도 패딩도 없어 짧게 끝난다
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    /// 키 파생 방식 — 지금은 [`KDF_NAME`] 하나뿐이다
    pub kdf: String,
    pub iterations: u64,
    pub salt: String,
    pub nonce: String,
    pub tag: String,
    pub ciphertext: String,
}

/// 암호로 봉한다. 봉하지 못하면(난수·파생·암호화 실패) `None`이다.
///
/// **빈 암호를 여기서 막지 않는다** — 「암호를 비우면 비밀번호를 담지 않는다」는 판단은 호출부의
/// 몫이고(plan D6), 이 함수는 받은 것을 그대로 봉한다
pub fn seal_with_passphrase(plain: &[u8], passphrase: &str) -> Option<Envelope> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce = [0u8; NONCE_LEN];
    random_bytes(&mut salt)?;
    random_bytes(&mut nonce)?;

    let mut key = derive_key(passphrase, &salt, PBKDF2_ITERATIONS)?;
    let sealed = encrypt(&key, &nonce, plain);
    // 파생한 키는 여기서 쓸 일이 끝났다 — 봉인 결과와 함께 메모리에 남기지 않는다
    crate::remote::secret::zeroize(&mut key);
    let (ciphertext, tag) = sealed?;

    Some(Envelope {
        kdf: KDF_NAME.to_owned(),
        iterations: PBKDF2_ITERATIONS,
        salt: to_hex(&salt),
        nonce: to_hex(&nonce),
        tag: to_hex(&tag),
        ciphertext: to_hex(&ciphertext),
    })
}

/// 봉투를 푼다. **틀린 암호와 손댄 파일을 구분하지 않고 둘 다 `None`**이다 —
/// 어느 쪽인지 알려 주면 공격자에게 단서가 되고, 사용자가 할 일(암호를 다시 넣는다)도 같다.
///
/// 모르는 파생 방식·길이가 어긋난 값도 `None`이다
pub fn open_with_passphrase(envelope: &Envelope, passphrase: &str) -> Option<Vec<u8>> {
    if envelope.kdf != KDF_NAME {
        return None;
    }
    let salt = from_hex(&envelope.salt)?;
    let nonce = from_hex(&envelope.nonce)?;
    let tag = from_hex(&envelope.tag)?;
    let ciphertext = from_hex(&envelope.ciphertext)?;
    if salt.len() != SALT_LEN || nonce.len() != NONCE_LEN || tag.len() != TAG_LEN {
        return None;
    }

    let mut key = derive_key(passphrase, &salt, envelope.iterations)?;
    let opened = decrypt(&key, &nonce, &tag, &ciphertext);
    crate::remote::secret::zeroize(&mut key);
    opened
}

/// 바이트를 소문자 hex로
pub fn to_hex(bytes: &[u8]) -> String {
    const DIGITS: [char; 16] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f',
    ];
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[usize::from(byte >> 4)]);
        out.push(DIGITS[usize::from(byte & 0x0f)]);
    }
    out
}

/// hex를 바이트로 — 길이가 홀수이거나 hex가 아닌 글자가 있으면 `None`.
///
/// **대문자도 받는다** — 다른 도구가 만든 파일을 사람이 손으로 옮겨 적는 경우가 있다
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let digits: Vec<u8> = text.bytes().collect();
    let mut out = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        out.push((high << 4) | low);
    }
    Some(out)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// 알고리즘 제공자 핸들 — 떨어질 때 반드시 닫는다.
///
/// 감싸는 이유: 봉인 한 번에 제공자를 둘(HMAC-SHA256·AES) 열고 그 사이에 실패로 빠져나가는
/// 길이 여럿이라, 손으로 닫으면 **어느 한 갈래에서 반드시 빠뜨린다**
struct AlgHandle(BCRYPT_ALG_HANDLE);

impl AlgHandle {
    fn open(algorithm: PCWSTR, flags: BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS) -> Option<AlgHandle> {
        let mut handle = BCRYPT_ALG_HANDLE::default();
        // 안전성: 출력 핸들만 받는 호출이다. 성공하면 `Drop`이 닫고, 실패하면 핸들이 채워지지 않는다
        let status =
            unsafe { BCryptOpenAlgorithmProvider(&mut handle, algorithm, PCWSTR::null(), flags) };
        (status.0 == 0).then_some(AlgHandle(handle))
    }
}

impl Drop for AlgHandle {
    fn drop(&mut self) {
        // 안전성: 열기에 성공한 핸들만 이 타입이 되므로 유효하다
        // 닫기 실패는 되돌릴 방법이 없고 알릴 곳도 없다 — 상태만 확인하고 넘긴다
        let _ = unsafe { BCryptCloseAlgorithmProvider(self.0, 0) };
    }
}

/// 대칭 키 핸들 — 떨어질 때 반드시 없앤다.
///
/// 키 오브젝트 버퍼를 우리가 잡지 않는다 — Windows 8부터 `BCryptGenerateSymmetricKey`에
/// `pbKeyObject`를 주지 않으면 CNG가 스스로 잡고 `BCryptDestroyKey`가 함께 돌려준다.
/// 이 앱은 Windows 11 전용이라 그 경로만 쓴다
struct KeyHandle(BCRYPT_KEY_HANDLE);

impl Drop for KeyHandle {
    fn drop(&mut self) {
        // 안전성: 생성에 성공한 핸들만 이 타입이 된다
        // 위와 같다 — `Drop`에서는 실패를 올릴 곳이 없다
        let _ = unsafe { BCryptDestroyKey(self.0) };
    }
}

/// 시스템 난수로 채운다
fn random_bytes(buffer: &mut [u8]) -> Option<()> {
    // 안전성: 우리가 가진 가변 버퍼만 넘긴다. 제공자를 주지 않고 시스템 기본 RNG를 쓴다
    let status = unsafe { BCryptGenRandom(None, buffer, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    (status.0 == 0).then_some(())
}

/// 암호에서 AES-256 키를 파생한다
fn derive_key(passphrase: &str, salt: &[u8], iterations: u64) -> Option<[u8; KEY_LEN]> {
    // PBKDF2의 의사난수 함수로 쓰려면 HMAC 플래그를 켠 해시 제공자여야 한다
    let algorithm = AlgHandle::open(BCRYPT_SHA256_ALGORITHM, BCRYPT_ALG_HANDLE_HMAC_FLAG)?;
    let mut key = [0u8; KEY_LEN];
    // 안전성: 핸들은 유효하고, 입력·출력 슬라이스는 이 함수가 소유해 호출 동안 살아 있다
    let status = unsafe {
        BCryptDeriveKeyPBKDF2(
            algorithm.0,
            Some(passphrase.as_bytes()),
            Some(salt),
            iterations,
            &mut key,
            0,
        )
    };
    (status.0 == 0).then_some(key)
}

/// GCM 모드로 세운 AES 키를 만든다 — 제공자와 키 핸들을 함께 돌려준다(제공자가 먼저 닫히면 안 된다)
fn aes_gcm_key(key: &[u8]) -> Option<(AlgHandle, KeyHandle)> {
    let algorithm = AlgHandle::open(
        BCRYPT_AES_ALGORITHM,
        BCRYPT_OPEN_ALGORITHM_PROVIDER_FLAGS(0),
    )?;
    // 연쇄 모드 이름은 **널로 끝나는 UTF-16 문자열의 바이트열**로 넘긴다 (CNG 계약)
    let mode: Vec<u16> = "ChainingModeGCM\0".encode_utf16().collect();
    // 안전성: `mode`가 소유한 메모리를 그 길이만큼만 바이트로 다시 본다(정렬 요구가 없는 u8이다)
    let mode_bytes =
        unsafe { std::slice::from_raw_parts(mode.as_ptr().cast::<u8>(), mode.len() * 2) };
    // 안전성: 방금 연 제공자 핸들과 이 함수가 소유한 버퍼만 넘긴다
    let status =
        unsafe { BCryptSetProperty(algorithm.0.into(), BCRYPT_CHAINING_MODE, mode_bytes, 0) };
    if status.0 != 0 {
        return None;
    }

    let mut handle = BCRYPT_KEY_HANDLE::default();
    // 안전성: 키 오브젝트를 주지 않으면 CNG가 스스로 잡는다(`KeyHandle` 주석 참조)
    let status = unsafe { BCryptGenerateSymmetricKey(algorithm.0, &mut handle, None, key, 0) };
    (status.0 == 0).then(|| (algorithm, KeyHandle(handle)))
}

/// GCM 모드 정보를 채운다.
///
/// # Safety
/// 돌려받은 구조체는 `nonce`·`tag`가 살아 있는 동안만 유효하다 — 그 둘보다 오래 쓰면 해제된
/// 메모리를 가리킨다
unsafe fn cipher_mode_info(
    nonce: &mut [u8],
    tag: &mut [u8],
) -> Option<BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO> {
    Some(BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO {
        cbSize: u32::try_from(size_of::<BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO>()).ok()?,
        dwInfoVersion: BCRYPT_AUTHENTICATED_CIPHER_MODE_INFO_VERSION,
        pbNonce: nonce.as_mut_ptr(),
        cbNonce: u32::try_from(nonce.len()).ok()?,
        pbTag: tag.as_mut_ptr(),
        cbTag: u32::try_from(tag.len()).ok()?,
        ..Default::default()
    })
}

/// 봉한다 — 암호문과 인증 태그를 돌려준다
fn encrypt(key: &[u8], nonce: &[u8], plain: &[u8]) -> Option<(Vec<u8>, [u8; TAG_LEN])> {
    let (_algorithm, key_handle) = aes_gcm_key(key)?;
    let mut nonce = nonce.to_vec();
    let mut tag = [0u8; TAG_LEN];
    // 안전성: `nonce`·`tag`는 이 함수가 소유하며 아래 호출이 끝날 때까지 살아 있다
    let info = unsafe { cipher_mode_info(&mut nonce, &mut tag)? };

    // GCM은 스트림 모드라 암호문 길이가 평문과 같다 — 자리를 미리 잡아 한 번에 받는다
    let mut out = vec![0u8; plain.len()];
    let mut written = 0u32;
    // 안전성: 모든 버퍼가 이 함수 소유이고 호출 동안 살아 있다. `info`는 GCM이 요구하는
    // 인증 모드 정보이며 IV 인자는 GCM에서 쓰지 않는다(nonce가 `info` 안에 있다)
    let status = unsafe {
        BCryptEncrypt(
            key_handle.0,
            Some(plain),
            Some(std::ptr::from_ref(&info).cast()),
            None,
            Some(&mut out),
            &mut written,
            BCRYPT_FLAGS(0),
        )
    };
    if status.0 != 0 || written as usize != out.len() {
        return None;
    }
    Some((out, tag))
}

/// 푼다 — 태그가 맞지 않으면 `None`
fn decrypt(key: &[u8], nonce: &[u8], tag: &[u8], ciphertext: &[u8]) -> Option<Vec<u8>> {
    let (_algorithm, key_handle) = aes_gcm_key(key)?;
    let mut nonce = nonce.to_vec();
    let mut tag = tag.to_vec();
    // 안전성: `encrypt`와 같다 — 두 버퍼가 호출 동안 살아 있다
    let info = unsafe { cipher_mode_info(&mut nonce, &mut tag)? };

    let mut out = vec![0u8; ciphertext.len()];
    let mut written = 0u32;
    // 안전성: `encrypt`와 같은 계약이다. 태그가 맞지 않으면 API가 오류를 돌려줄 뿐 패닉하지 않는다
    let status = unsafe {
        BCryptDecrypt(
            key_handle.0,
            Some(ciphertext),
            Some(std::ptr::from_ref(&info).cast()),
            None,
            Some(&mut out),
            &mut written,
            BCRYPT_FLAGS(0),
        )
    };
    if status.0 != 0 || written as usize != out.len() {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 봉인과_해제가_원문을_되돌린다() {
        let plain = "사이트 목록과 비밀번호가 담긴 본문".as_bytes();
        let sealed = seal_with_passphrase(plain, "내 암호 1234").expect("봉인");
        let opened = open_with_passphrase(&sealed, "내 암호 1234").expect("해제");
        assert_eq!(opened, plain);
    }

    #[test]
    fn 빈_평문과_긴_평문도_왕복한다() {
        let empty = seal_with_passphrase(&[], "암호").expect("빈 평문 봉인");
        assert_eq!(open_with_passphrase(&empty, "암호"), Some(Vec::new()));

        let long = "가".repeat(1024);
        let sealed = seal_with_passphrase(long.as_bytes(), "암호").expect("긴 평문 봉인");
        assert_eq!(
            open_with_passphrase(&sealed, "암호").as_deref(),
            Some(long.as_bytes())
        );
    }

    #[test]
    fn 비ascii_암호도_왕복한다() {
        // 암호는 UTF-8 바이트 그대로 파생에 들어간다 — 한글·이모지도 그대로 쓸 수 있어야 한다
        let plain = b"secret";
        let sealed = seal_with_passphrase(plain, "비밀번호🔒한글").expect("봉인");
        assert_eq!(
            open_with_passphrase(&sealed, "비밀번호🔒한글").as_deref(),
            Some(&plain[..])
        );
    }

    #[test]
    fn 틀린_암호로는_풀리지_않는다() {
        let sealed = seal_with_passphrase(b"body", "correct horse").expect("봉인");
        assert_eq!(open_with_passphrase(&sealed, "wrong horse"), None);
        assert_eq!(open_with_passphrase(&sealed, ""), None);
    }

    #[test]
    fn 손댄_봉투는_풀리지_않는다() {
        // 인증 태그가 하는 일이다 — 한 바이트만 흔들려도 거부해야 한다
        let sealed = seal_with_passphrase(b"body body body", "암호").expect("봉인");

        let mut broken = sealed.clone();
        let mut bytes = from_hex(&broken.ciphertext).expect("hex");
        bytes[0] ^= 0xff;
        broken.ciphertext = to_hex(&bytes);
        assert_eq!(open_with_passphrase(&broken, "암호"), None, "암호문 변조");

        let mut broken = sealed.clone();
        let mut bytes = from_hex(&broken.tag).expect("hex");
        bytes[0] ^= 0xff;
        broken.tag = to_hex(&bytes);
        assert_eq!(open_with_passphrase(&broken, "암호"), None, "태그 변조");

        let mut broken = sealed.clone();
        broken.kdf = "PBKDF2-HMAC-SHA1".to_owned();
        assert_eq!(
            open_with_passphrase(&broken, "암호"),
            None,
            "모르는 파생 방식"
        );
    }

    #[test]
    fn 길이가_어긋난_봉투는_풀리지_않는다() {
        let mut sealed = seal_with_passphrase(b"body", "암호").expect("봉인");
        sealed.nonce = to_hex(&[0u8; 8]);
        assert_eq!(open_with_passphrase(&sealed, "암호"), None);

        let mut sealed = seal_with_passphrase(b"body", "암호").expect("봉인");
        sealed.salt = to_hex(&[0u8; 4]);
        assert_eq!(open_with_passphrase(&sealed, "암호"), None);

        let mut sealed = seal_with_passphrase(b"body", "암호").expect("봉인");
        sealed.tag = "not hex".to_owned();
        assert_eq!(open_with_passphrase(&sealed, "암호"), None);
    }

    #[test]
    fn 같은_평문도_봉할_때마다_달라진다() {
        // 소금과 nonce를 매번 새로 뽑기 때문이다 — 같으면 두 파일을 견줘 내용을 짐작할 수 있다
        let first = seal_with_passphrase(b"same body", "암호").expect("첫째");
        let second = seal_with_passphrase(b"same body", "암호").expect("둘째");
        assert_ne!(first.salt, second.salt);
        assert_ne!(first.nonce, second.nonce);
        assert_ne!(first.ciphertext, second.ciphertext);
    }

    #[test]
    fn 봉투를_직렬화해도_평문이_남지_않는다() {
        let plain = "찾을수있는평문";
        let sealed = seal_with_passphrase(plain.as_bytes(), "암호").expect("봉인");
        let json = serde_json::to_string(&sealed).expect("직렬화");
        assert!(!json.contains(plain), "평문이 남았다: {json}");

        let back: Envelope = serde_json::from_str(&json).expect("역직렬화");
        assert_eq!(back, sealed);
        assert_eq!(
            open_with_passphrase(&back, "암호").as_deref(),
            Some(plain.as_bytes())
        );
    }

    #[test]
    fn hex가_왕복하고_잘못된_글자를_거른다() {
        let bytes = [0x00, 0x0f, 0xa5, 0xff];
        assert_eq!(to_hex(&bytes), "000fa5ff");
        assert_eq!(from_hex("000fa5ff").as_deref(), Some(&bytes[..]));
        // 대문자도 받는다
        assert_eq!(from_hex("000FA5FF").as_deref(), Some(&bytes[..]));
        assert_eq!(to_hex(&[]), "");
        assert_eq!(from_hex(""), Some(Vec::new()));
        // 홀수 길이·비hex 글자
        assert_eq!(from_hex("abc"), None);
        assert_eq!(from_hex("zz"), None);
        assert_eq!(from_hex("0 1"), None);
    }

    #[test]
    fn 핸들을_여러_번_써도_끝까지_돈다() {
        // 제공자·키 핸들이 새면 반복하는 사이에 바닥난다 — 그때 이 시험이 실패로 알린다.
        // **파생 반복은 1회로 낮춰 돈다** — 누수는 호출 횟수로 드러나지 파생 강도로 드러나지
        // 않으므로, 600,000회를 1,000번 돌려 시험을 몇 분씩 붙잡을 이유가 없다
        let salt = [7u8; SALT_LEN];
        for index in 0..1000 {
            let key = derive_key("암호", &salt, 1).expect("파생");
            let (ciphertext, tag) = encrypt(&key, &[9u8; NONCE_LEN], b"body").expect("봉인");
            let opened = decrypt(&key, &[9u8; NONCE_LEN], &tag, &ciphertext);
            assert_eq!(
                opened.as_deref(),
                Some(&b"body"[..]),
                "{index}번째 왕복 실패"
            );
        }
    }

    #[test]
    #[ignore = "실측 전용 — `cargo test --release -- --ignored --nocapture`로 돌린다"]
    fn 파생_1회_시간을_잰다() {
        // plan D13의 상한 대조. 이 시험이 찍은 값을 `PBKDF2_ITERATIONS`의 주석에 적는다
        let salt = [0u8; SALT_LEN];
        let started = std::time::Instant::now();
        let key = derive_key("측정용 암호", &salt, PBKDF2_ITERATIONS).expect("파생");
        let elapsed = started.elapsed().as_secs_f64();
        assert_eq!(key.len(), KEY_LEN);
        println!("PBKDF2 {PBKDF2_ITERATIONS}회 파생: {elapsed:.3}초");
        assert!(
            elapsed < 1.0,
            "파생이 {elapsed:.3}초로 plan D13 상한(1.0초)을 넘었다 — 반복을 200,000회로 낮춘다"
        );
    }
}
