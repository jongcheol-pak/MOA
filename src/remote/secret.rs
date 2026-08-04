//! 비밀번호 봉인 — Windows DPAPI 래퍼 (FR-28·D3).
//!
//! 사이트 비밀번호를 설정 파일에 평문으로 두지 않기 위한 것이다. DPAPI는 **지금 로그인한
//! 사용자**에게 묶인 키로 암·복호하므로, 설정 파일이 통째로 복사돼도 다른 계정·다른 PC에서는
//! 풀리지 않는다.
//!
//! 키 저장소를 추상화하지 않는다 — 쓰는 것이 DPAPI 한 가지뿐이라 갈래가 없다.
//! 자격증명 캐시도 두지 않는다: 연결할 때마다 풀어 쓰고 그 자리에서 버린다.
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

/// 비밀번호를 봉인한다. 빈 값은 봉인할 것이 없어 빈 바이트열이다.
///
/// 실패(정책 차단 등)하면 `None` — 호출부는 **저장을 생략**하고 사용자에게 알린다.
/// 봉인하지 못한 것을 평문으로 대신 저장하는 경로는 두지 않는다.
pub fn seal(plain: &str) -> Option<Vec<u8>> {
    if plain.is_empty() {
        return Some(Vec::new());
    }
    let mut input = plain.as_bytes().to_vec();
    let input_blob = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len()).ok()?,
        pbData: input.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    // 안전성: 입력 블롭은 이 함수가 소유한 버퍼를 가리키고 호출 동안 살아 있다. 출력 블롭은
    // API가 힙에 잡아 주므로 **복사한 뒤 반드시 `LocalFree`로 돌려준다**.
    // `UI_FORBIDDEN`을 주는 이유: 이 함수는 워커 스레드에서도 불릴 수 있어, 시스템이 대화를
    // 띄우면 그 스레드가 사용자 조작을 기다리며 멈춘다
    let sealed = unsafe {
        CryptProtectData(
            &input_blob,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .ok()
        .map(|()| copy_and_free(&mut output))
    };
    // 평문이 담겼던 임시 버퍼를 그 자리에서 덮는다
    input.fill(0);
    sealed
}

/// 봉인을 푼다. 손상됐거나 다른 사용자·다른 PC에서 만든 것이면 `None`이다 —
/// 그때는 비밀번호를 다시 입력받는다.
pub fn unseal(sealed: &[u8]) -> Option<String> {
    if sealed.is_empty() {
        return Some(String::new());
    }
    let mut input = sealed.to_vec();
    let input_blob = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len()).ok()?,
        pbData: input.as_mut_ptr(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();

    // 안전성: `seal`과 같은 규칙이다. 손상된 입력에도 API는 오류를 돌려줄 뿐 패닉하지 않는다
    let mut plain = unsafe {
        CryptUnprotectData(
            &input_blob,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
        .ok()
        .map(|()| copy_and_free(&mut output))?
    };
    let text = String::from_utf8(plain.clone()).ok();
    // 풀어낸 평문 사본을 남기지 않는다
    plain.fill(0);
    text
}

/// API가 잡아 준 출력 블롭을 복사하고 그 메모리를 돌려준다.
///
/// # Safety
/// `output`은 `CryptProtectData`/`CryptUnprotectData`가 **성공했을 때** 채운 블롭이어야 한다.
/// 그 밖의 값을 넘기면 잘못된 포인터를 읽는다.
unsafe fn copy_and_free(output: &mut CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let copied =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    // 돌려주지 않으면 봉인할 때마다 조금씩 샌다
    unsafe { LocalFree(Some(HLOCAL(output.pbData.cast()))) };
    output.pbData = std::ptr::null_mut();
    output.cbData = 0;
    copied
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 봉인과_해제가_원문을_되돌린다() {
        let plain = "아주 비밀스러운 값 42!";
        let sealed = seal(plain).expect("봉인");
        assert_eq!(unseal(&sealed).as_deref(), Some(plain));
    }

    #[test]
    fn 봉인된_바이트에_평문이_남지_않는다() {
        // 설정 파일에 그대로 실리는 값이라 여기 평문이 보이면 봉인의 뜻이 없다
        let plain = "찾을수있는평문";
        let sealed = seal(plain).expect("봉인");
        assert!(
            !sealed
                .windows(plain.len())
                .any(|window| window == plain.as_bytes()),
            "봉인 바이트에 평문이 그대로 들어 있다"
        );
        assert!(sealed.len() > plain.len(), "봉인은 원문보다 길다");
    }

    #[test]
    fn 빈_비밀번호는_봉인하지_않는다() {
        let sealed = seal("").expect("빈 값");
        assert!(sealed.is_empty(), "봉인할 것이 없다");
        assert_eq!(unseal(&sealed).as_deref(), Some(""));
    }

    #[test]
    fn 손상된_입력은_패닉하지_않고_none이다() {
        assert_eq!(unseal(&[0xde, 0xad, 0xbe, 0xef]), None);
        // 제대로 봉인된 값을 한 바이트만 흔들어도 풀리지 않는다
        let mut sealed = seal("원문").expect("봉인");
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        assert_eq!(unseal(&sealed), None);
    }

    #[test]
    fn 아주_긴_비밀번호도_왕복한다() {
        let plain = "가".repeat(1024);
        let sealed = seal(&plain).expect("봉인");
        assert_eq!(unseal(&sealed).as_deref(), Some(plain.as_str()));
    }
}
