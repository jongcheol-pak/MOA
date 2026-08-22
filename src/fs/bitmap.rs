//! GDI 비트맵의 픽셀을 32bpp BGRA로 읽는다 — 그 절차를 담는 한 곳.
//!
//! 같은 `GetObjectW` → 헤더 세우기 → `CreateCompatibleDC` → `GetDIBits` 절차가 세 곳에
//! 흩어져 있었다(`fs::thumbnail`의 썸네일·`fs::drag_image`의 끌기 그림·`ui::icon_tex`의
//! 아이콘 둘). 2026-08-22에 컨텍스트 메뉴 아이콘이 **네 번째**가 될 참이라 여기로 모았다.
//!
//! **후처리는 모으지 않는다** — 셋이 읽은 픽셀로 하는 일이 서로 다르다(썸네일은 RGBA로
//! 뒤집어 자기 타입에 담고, 끌기 그림은 BGRA인 채로 알파만 되돌리며, 아이콘은 egui 이미지로
//! 바꾼다). 그것까지 한 함수에 넣으면 분기 인자가 늘어 읽기 어려워진다.
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    GetDIBits, GetObjectW, HBITMAP, HDC,
};

/// 32bpp top-down DIB 헤더를 만든다.
///
/// **음수 높이가 top-down**이다 — 첫 행이 이미지 위쪽이라 뒤집지 않고 그대로 담고 그대로
/// 읽는다. 읽을 때(`bgra_from_hbitmap`)와 새 DIB를 만들 때(`fs::drag_image`) 같은 헤더를
/// 써야 두 방향이 어긋나지 않는다
pub(crate) fn dib_header(width: i32, height: i32) -> BITMAPINFO {
    let mut header = BITMAPINFO::default();
    header.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    header.bmiHeader.biWidth = width;
    header.bmiHeader.biHeight = -height;
    header.bmiHeader.biPlanes = 1;
    header.bmiHeader.biBitCount = 32;
    header.bmiHeader.biCompression = BI_RGB.0;
    header
}

/// 비트맵의 픽셀을 32bpp BGRA(top-down)로 읽는다 — `(폭, 높이, 픽셀)`.
///
/// **원본이 몇 비트든 네 바이트로 온다** — `GetDIBits`에 32bpp를 **청하기** 때문이며,
/// 셸은 8·24bpp 비트맵도 준다. 그래서 원본의 `bmBitsPixel`을 조건으로 쓰면 안 된다
/// (거르면 아이콘·썸네일이 조용히 사라진다). 읽지 못한 것은 `GetDIBits`가 0줄로 알린다.
///
/// 알파는 **프리멀티플라이일 수 있다** — 되돌리는 것은 부르는 쪽의 몫이다(후처리가
/// 서로 달라 여기서 하지 않는다).
pub(crate) fn bgra_from_hbitmap(bitmap: HBITMAP) -> Option<(i32, i32, Vec<u8>)> {
    if bitmap.is_invalid() {
        // 1bpp 마스크만 있는 흑백 아이콘 등 — 드물어 그리지 않는다
        return None;
    }
    // 안전성: 위에서 유효성을 걸러 낸 핸들에만 GDI를 부르고, 만든 DC는 이 함수가 해제한다.
    // 픽셀 버퍼는 `width * height * 4`로 잡아 `GetDIBits`가 요구하는 크기를 채운다
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 무효한_핸들은_읽지_않는다() {
        // 셸이 아이콘을 주지 못하면 널 핸들이 온다 — GDI를 부르기 전에 걸러야 한다
        assert!(bgra_from_hbitmap(HBITMAP::default()).is_none());
    }

    #[test]
    fn 헤더는_top_down이고_32bpp다() {
        // 이 둘이 어긋나면 그림이 뒤집히거나 바이트 수가 맞지 않는다
        let header = dib_header(16, 24);
        assert_eq!(header.bmiHeader.biWidth, 16);
        assert_eq!(
            header.bmiHeader.biHeight, -24,
            "음수 높이가 top-down — 양수면 첫 행이 아래쪽이 된다"
        );
        assert_eq!(header.bmiHeader.biBitCount, 32);
        assert_eq!(header.bmiHeader.biPlanes, 1);
        assert_eq!(header.bmiHeader.biCompression, BI_RGB.0);
    }
}
