//! 끌기 중 커서를 따라오는 미리보기 그림 만들기 (FR-61 내보내기).
//!
//! 셸의 드래그 이미지 관리자(`IDragSourceHelper`)는 **프리멀티플라이가 아닌** 32bpp 비트맵을
//! 요구한다 — 넘긴 값에 알파를 한 번 더 곱하기 때문이다. 그런데 셸이 주는 그림
//! (`IShellItemImageFactory::GetImage`)은 프리멀티플라이일 수 있어, 픽셀을 읽어 되돌린 뒤
//! 새 DIB 섹션에 담아 넘긴다.
//!
//! **디스크에서 썸네일을 새로 만들지 않는다** — 이 조회는 `DoDragDrop`과 같은 UI 스레드에서
//! 돌기 때문이다. 이미 만들어 둔 것이 있으면 그것을(`SIIGBF_INCACHEONLY`) 쓰고, 없으면
//! 형식 아이콘으로 되돌린다(`SIIGBF_ICONONLY`).
//!
//! 이 모듈은 `SHDRAGIMAGE`를 채우지 않는다 — 그것은 끌기를 여는 `fs::drag_source`의 몫이다.
use std::path::Path;

use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDIBits, GetObjectW, HBITMAP, HDC,
};
use windows::Win32::UI::Shell::{
    IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY, SIIGBF_INCACHEONLY,
};
use windows::core::HSTRING;

/// 셸에 넘길 드래그 그림 — 비트맵 하나와 그 실제 크기.
///
/// 크기를 함께 드는 이유는 셸이 **청한 것보다 작은 그림**을 줄 수 있어서다
/// (`SHDRAGIMAGE`에는 실제 크기를 적어야 한다).
pub struct DragImage {
    pub bitmap: HBITMAP,
    pub width: i32,
    pub height: i32,
}

impl DragImage {
    /// 셸에 **얹지 못한** 비트맵을 되돌린다.
    ///
    /// 얹는 데 성공했으면 부르지 않는다 — 소유권이 넘어갔는지 문서에 서술이 없어
    /// 지우는 쪽이 위험하다(`docs/plans/2026-08-21-drag-preview-image.md` D6).
    pub fn delete(self) {
        // 안전성: `build`가 `CreateDIBSection`으로 만든 유효한 핸들이며, 이 값을 소비하는
        // 메서드라 같은 비트맵을 두 번 지울 수 없다
        unsafe {
            let _ = DeleteObject(self.bitmap.into());
        }
    }
}

/// 경로 하나의 그림을 만든다. 만들지 못하면 `None`(그림 없이 끌면 된다).
///
/// `px`는 **물리 픽셀**로 청하는 한 변의 길이다 — 화면 배율을 아는 쪽이 정해 내려보낸다
/// (`fs`는 배율을 모른다).
pub fn build(path: &Path, px: i32) -> Option<DragImage> {
    if px <= 0 {
        // 셸에 0 크기를 청하지 않는다
        return None;
    }
    // 안전성: 아래 호출은 모두 COM이 초기화된 스레드에서 돌고(끌기를 여는 UI 스레드),
    // 얻은 인터페이스는 이 함수 안에서만 살다 `Drop`으로 해제된다. 셸이 준 비트맵은
    // 픽셀을 읽은 뒤 이 함수가 지우고, 새로 만든 비트맵만 밖으로 나간다
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).ok()?;
        let size = SIZE { cx: px, cy: px };
        // 이미 만들어 둔 미리보기가 있으면 그것을 쓰고, 없으면 형식 아이콘으로 되돌린다 —
        // 새로 만들게 하면 큰 동영상 하나에 몇 초가 걸려 끌기가 그동안 멎는다
        let shell_bitmap = factory
            .GetImage(size, SIIGBF_INCACHEONLY)
            .or_else(|_| factory.GetImage(size, SIIGBF_ICONONLY))
            .ok()?;

        let read = read_bgra(shell_bitmap);
        let _ = DeleteObject(shell_bitmap.into());
        let (width, height, mut pixels) = read?;

        unpremultiply(&mut pixels);
        let bitmap = new_dib(width, height, &pixels)?;
        Some(DragImage {
            bitmap,
            width,
            height,
        })
    }
}

/// 비트맵의 픽셀을 32bpp BGRA(top-down)로 읽는다 — `(폭, 높이, 픽셀)`.
///
/// 셸이 8·24bpp를 주더라도 `GetDIBits`에 32bpp를 청하므로 언제나 네 바이트로 온다.
///
/// 안전성: 유효한 HBITMAP에만 호출한다. 내부에서 만든 DC는 이 함수가 해제한다
unsafe fn read_bgra(bitmap: HBITMAP) -> Option<(i32, i32, Vec<u8>)> {
    unsafe {
        let mut info = BITMAP::default();
        let written = GetObjectW(
            bitmap.into(),
            size_of::<BITMAP>() as i32,
            Some(&mut info as *mut BITMAP as *mut core::ffi::c_void),
        );
        if written == 0 || info.bmWidth <= 0 || info.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (info.bmWidth, info.bmHeight);

        let mut header = dib_header(width, height);
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        let hdc: HDC = CreateCompatibleDC(None);
        if hdc.is_invalid() {
            return None;
        }
        let lines = GetDIBits(
            hdc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut core::ffi::c_void),
            &mut header,
            DIB_RGB_COLORS,
        );
        let _ = DeleteDC(hdc);
        if lines == 0 {
            return None;
        }
        Some((width, height, pixels))
    }
}

/// 읽은 픽셀을 담은 새 DIB 섹션을 만든다 — 이것이 셸에 넘어갈 비트맵이다.
///
/// 안전성: `pixels`가 `width * height * 4`바이트임을 부르는 쪽이 보장한다. 성공하면
/// 만들어진 비트맵의 소유권이 밖으로 나간다
unsafe fn new_dib(width: i32, height: i32, pixels: &[u8]) -> Option<HBITMAP> {
    unsafe {
        let header = dib_header(width, height);
        let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(None, &header, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        if bits.is_null() {
            let _ = DeleteObject(bitmap.into());
            return None;
        }
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
        Some(bitmap)
    }
}

/// 32bpp·무압축·**top-down**(첫 행이 그림 위쪽) 헤더. 읽을 때와 만들 때 같은 것을 쓴다
fn dib_header(width: i32, height: i32) -> BITMAPINFO {
    let mut header = BITMAPINFO::default();
    header.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    header.bmiHeader.biWidth = width;
    // 음수 높이가 top-down — 뒤집기 없이 그대로 담고 그대로 읽는다
    header.bmiHeader.biHeight = -height;
    header.bmiHeader.biPlanes = 1;
    header.bmiHeader.biBitCount = 32;
    header.bmiHeader.biCompression = BI_RGB.0;
    header
}

/// BGRA 버퍼를 제자리에서 스트레이트 알파로 되돌린다.
///
/// `InitializeFromBitmap`이 넘긴 값에 알파를 **한 번 더** 곱하므로, 프리멀티플라이인 채로
/// 주면 반투명한 가장자리가 두 번 곱해져 사라진다.
///
/// 알파를 쓰지 않는 비트맵(알파가 전부 0)은 그대로 두면 그림 전체가 투명해지므로 불투명으로
/// 채운다 — 픽셀마다 판정하면 진짜 투명한 자리까지 메워 검은 테두리가 생기므로 **버퍼 전체**로
/// 판정한다(`fs::thumbnail`·`ui::icon_tex`의 같은 규칙)
fn unpremultiply(pixels: &mut [u8]) {
    let opaque_bitmap = pixels.chunks_exact(4).all(|px| px[3] == 0);
    for px in pixels.chunks_exact_mut(4) {
        let (b, g, r, a) = (px[0], px[1], px[2], px[3]);
        if opaque_bitmap {
            // 알파를 안 쓰는 비트맵 — 색은 그대로 두고 불투명으로 만든다
            px[3] = 255;
            continue;
        }
        if a == 0 {
            // 실제로 투명한 픽셀 — 색까지 지워야 가장자리에 잔상이 남지 않는다
            px.copy_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let unmul = |c: u8| ((c as u32 * 255 + a as u32 / 2) / a as u32).min(255) as u8;
        px[0] = unmul(b);
        px[1] = unmul(g);
        px[2] = unmul(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};

    #[test]
    fn 프리멀티플라이를_되돌린다() {
        // 알파 128에 절반으로 곱해져 있던 색이 원래 값으로 돌아온다
        let mut pixels = vec![64u8, 64, 64, 128, 0, 0, 0, 0];
        unpremultiply(&mut pixels);
        assert_eq!(pixels[3], 128, "알파 값 자체는 바뀌지 않는다");
        assert_eq!(&pixels[0..3], &[128, 128, 128], "색이 c*255/a로 되돌아온다");
        assert_eq!(&pixels[4..8], &[0, 0, 0, 0], "투명한 픽셀은 색까지 지운다");
    }

    #[test]
    fn 알파를_쓰지_않는_비트맵은_불투명으로_채운다() {
        // 알파가 전부 0이면 알파 채널을 쓰지 않는 비트맵이다 — 그대로 두면 전체가 투명해진다
        let mut pixels = vec![10u8, 20, 30, 0, 40, 50, 60, 0];
        unpremultiply(&mut pixels);
        assert_eq!(&pixels[0..4], &[10, 20, 30, 255]);
        assert_eq!(&pixels[4..8], &[40, 50, 60, 255]);
    }

    #[test]
    fn 없는_경로는_그림을_만들지_않는다() {
        let 없는_경로 = PathBuf::from(r"C:\이런 폴더는 없다\없는 파일.txt");
        assert!(build(&없는_경로, 96).is_none());
    }

    #[test]
    fn 크기를_0으로_청하면_셸에_묻지_않는다() {
        // COM에 닿기 전에 돌아간다 — 초기화하지 않은 스레드에서도 안전하다
        assert!(build(Path::new("."), 0).is_none());
    }

    #[test]
    fn 실제_파일의_그림을_만든다() {
        // 셸 호출 경로가 살아 있는지 본다 — 픽셀 변환 시험만으로는 GetImage·DIB 생성이
        // 실제로 도는지 알 수 없다
        let 초기화됨 = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        let 경로 = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let 그림 = build(&경로, 96);
        // `None`을 허용하지 않는다 — 실재하는 평범한 파일이면 썸네일이 없어도 형식 아이콘은
        // 나와야 하므로, 여기서 물러서면 두 갈래가 모두 끊긴 회귀를 잡지 못한다
        let 그림 = 그림.expect("실재하는 파일이면 적어도 형식 아이콘은 얻어야 한다");
        assert!(
            그림.width >= 1 && 그림.width <= 96,
            "청한 크기 안에 들어온다"
        );
        assert!(그림.height >= 1 && 그림.height <= 96);
        그림.delete();
        if 초기화됨 {
            // 안전성: 위에서 성공한 초기화와 같은 스레드에서 1회 호출
            unsafe {
                CoUninitialize();
            }
        }
    }
}
