//! 정보 대화 (FR-58).
//!
//! 타이틀바 설정 메뉴의 `정보`가 연다. 가운데에 앱 아이콘, 그 아래 이름과 버전 한 줄뿐이라
//! 높이를 본문이 정한다 — 그래서 `dialog::show_fixed`가 아니라 `dialog::show`를 쓴다.
//!
//! **아이콘은 표시할 물리 픽셀 크기로 CPU에서 줄여 올린다.** 256px 자산을 그대로 텍스처로
//! 올리고 96px 자리에 그리면 축소를 GPU가 하는데, 선형 필터는 인접 네 텍셀만 보므로 크게
//! 줄일 때 가장자리가 자글거린다(2026-08-19 사용자 지적). 화면 배율이 바뀌면 필요한 물리
//! 크기도 달라지므로 만든 크기를 함께 들고 다르면 다시 만든다.
use crate::i18n;
use crate::ui::dialog;
use crate::ui::theme;
use eframe::egui;

/// 아이콘 원본 — 생성기(`examples/gen_app_icon.rs`)가 만든 256×256 PNG
const ICON_ASSET: &[u8] = include_bytes!("../../assets/app_icon_256.png");

/// 아이콘을 그릴 크기(논리 px) — 2026-08-19 사용자 결정
const ICON_PX: f32 = 96.0;
/// 아이콘과 이름·버전 줄 사이 (D12)
const ICON_TEXT_GAP: f32 = 16.0;
/// 이름·버전 줄의 글자 크기 (D12) — 설정·라이선스 대화의 제목과 같은 값이라
/// 이 앱에 새 치수를 만들지 않는다
const NAME_FONT_PX: f32 = 16.0;
/// 본문 폭 (D5) — 96px 아이콘 좌우로 여백이 남는다. 프레임 폭은 여기에 셸의 여백이 더해진다
const BODY_WIDTH: f32 = 248.0;

/// 정보 대화 — 열림 상태와 아이콘 텍스처만 든다.
///
/// 텍스처를 `close()`에서 버리지 않는 이유: 다시 열 때 같은 배율이면 그대로 쓴다
#[derive(Default)]
pub struct AboutDialog {
    open: bool,
    /// 올려 둔 텍스처와 **그것을 만든 물리 픽셀 크기**
    icon: Option<(egui::TextureHandle, u32)>,
}

impl AboutDialog {
    pub fn new() -> AboutDialog {
        AboutDialog::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    /// 대화를 그린다. 닫혀 있으면 아무것도 그리지 않는다
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }
        let pixels_per_point = ctx.pixels_per_point();
        let icon = self.icon_texture(ctx, physical_icon_px(pixels_per_point));

        let buttons = [dialog::ButtonSpec::strong(i18n::close())];
        let shell = dialog::show(
            ctx,
            egui::Id::new("정보 대화"),
            BODY_WIDTH,
            &buttons,
            |ui| {
                show_body(ui, icon, pixels_per_point);
            },
        );
        if shell.clicked.is_some() || shell.should_close {
            self.close();
        }
    }

    /// 지금 배율에 맞는 텍스처를 돌려준다. 들고 있는 것이 다른 크기면 다시 만든다.
    ///
    /// 자산을 읽지 못하면 `None` — 아이콘 없이 이름·버전만 그린다(타이틀바가 아이콘 없이도
    /// 자리를 잡는 것과 같은 처리)
    fn icon_texture(&mut self, ctx: &egui::Context, physical: u32) -> Option<egui::TextureId> {
        if !cache_hit(self.icon.as_ref().map(|(_, size)| *size), physical) {
            let image = decode_icon(physical)?;
            let handle = ctx.load_texture("about_icon", image, egui::TextureOptions::LINEAR);
            self.icon = Some((handle, physical));
        }
        self.icon.as_ref().map(|(handle, _)| handle.id())
    }
}

/// 본문 — 아이콘과 그 아래 한 줄을 가운데에 세운다
fn show_body(ui: &mut egui::Ui, icon: Option<egui::TextureId>, pixels_per_point: f32) {
    let line = ui.painter().layout_no_wrap(
        i18n::dynamic::about_version_line(),
        egui::FontId::proportional(NAME_FONT_PX),
        theme::TEXT,
    );
    // 아이콘이 없으면 그 자리도 간격도 두지 않는다 — 빈 96px이 남으면 대화가 비어 보인다
    let icon_block = if icon.is_some() {
        ICON_PX + ICON_TEXT_GAP
    } else {
        0.0
    };
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), icon_block + line.size().y),
        egui::Sense::hover(),
    );
    if let Some(icon) = icon {
        let placed = egui::Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + ICON_PX / 2.0),
            egui::Vec2::splat(ICON_PX),
        );
        ui.painter().image(
            icon,
            snap_to_pixels(placed, pixels_per_point),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    let line_left = rect.center().x - line.size().x / 2.0;
    ui.painter().galley(
        egui::pos2(line_left, rect.top() + icon_block),
        line,
        theme::TEXT,
    );
}

/// 들고 있는 텍스처를 그대로 쓸 수 있는가 — 만든 물리 크기가 요청과 같을 때만이다
fn cache_hit(cached: Option<u32>, requested: u32) -> bool {
    cached == Some(requested)
}

/// 아이콘을 올릴 텍스처의 물리 픽셀 크기 — 표시 크기 × 화면 배율.
///
/// 1을 하한으로 두는 것은 비정상 배율(0 이하·NaN)에서 빈 텍스처를 만들지 않기 위해서다
fn physical_icon_px(pixels_per_point: f32) -> u32 {
    let physical = ICON_PX * pixels_per_point;
    if physical >= 1.0 {
        // 원본(256)보다 큰 값도 그대로 쓴다 — 배율 267% 이상에서는 확대가 되지만
        // 자글거림은 축소에서 생기므로 화질이 완만하게 무뎌질 뿐이다
        physical.round() as u32
    } else {
        1
    }
}

/// 자산을 `size`×`size` RGBA로 줄인다. 읽지 못하면 `None`
fn decode_icon(size: u32) -> Option<egui::ColorImage> {
    let original = image::load_from_memory(ICON_ASSET).ok()?;
    let resized = original.resize_exact(size, size, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [size as usize, size as usize],
        rgba.as_raw(),
    ))
}

/// 사각형의 좌상단을 물리 픽셀 격자에 맞춘다 (D13).
///
/// 텍스처를 표시 크기와 같은 물리 크기로 만들어도 좌상단이 반픽셀 어긋난 자리에 놓이면
/// GPU가 이웃 픽셀에 걸쳐 섞어 그려, 애써 CPU에서 곱게 줄인 것이 다시 흐려진다
fn snap_to_pixels(rect: egui::Rect, pixels_per_point: f32) -> egui::Rect {
    if pixels_per_point <= 0.0 {
        return rect;
    }
    let snap = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
    egui::Rect::from_min_size(egui::pos2(snap(rect.left()), snap(rect.top())), rect.size())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::LanguageSetting;
    use crate::i18n::LanguageGuard;

    #[test]
    fn 닫힌_대화는_열림_상태가_아니다() {
        let dialog = AboutDialog::new();
        assert!(!dialog.is_open());
    }

    #[test]
    fn 열고_닫으면_상태가_따라온다() {
        let mut dialog = AboutDialog::new();
        dialog.open();
        assert!(dialog.is_open());
        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn 이름과_버전을_한_줄로_잇는다() {
        let _guard = LanguageGuard::lock(LanguageSetting::Korean);
        let line = i18n::dynamic::about_version_line();
        assert!(line.starts_with("MOA "), "이름이 앞에 선다: {line}");
        assert!(
            line.ends_with(env!("CARGO_PKG_VERSION")),
            "버전이 뒤에 붙는다: {line}"
        );
    }

    #[test]
    fn 앱_이름은_두_언어에서_같다() {
        let korean = {
            let _guard = LanguageGuard::lock(LanguageSetting::Korean);
            i18n::about_app_name()
        };
        let english = {
            let _guard = LanguageGuard::lock(LanguageSetting::English);
            i18n::about_app_name()
        };
        assert_eq!(korean, "MOA");
        assert_eq!(english, "MOA");
    }

    #[test]
    fn 자산은_256px_정사각이다() {
        let image = image::load_from_memory(ICON_ASSET).expect("자산을 읽지 못했다");
        assert_eq!((image.width(), image.height()), (256, 256));
    }

    #[test]
    fn 요청한_크기로_줄여_준다() {
        for size in [96, 192] {
            let image = decode_icon(size).expect("자산을 줄이지 못했다");
            assert_eq!(image.size, [size as usize, size as usize]);
            // RGBA 네 채널이 모두 담겼는가 — 크기만 맞고 내용이 비면 화면이 투명해진다
            assert_eq!(image.pixels.len(), (size * size) as usize);
        }
    }

    #[test]
    fn 같은_크기는_다시_만들지_않는다() {
        assert!(cache_hit(Some(96), 96));
        assert!(!cache_hit(Some(96), 192), "배율이 바뀌면 다시 만든다");
        assert!(!cache_hit(None, 96), "아직 만든 적이 없으면 만든다");
    }

    #[test]
    fn 물리_크기는_배율을_따른다() {
        assert_eq!(physical_icon_px(1.0), 96);
        assert_eq!(physical_icon_px(1.5), 144);
        assert_eq!(physical_icon_px(2.0), 192);
    }

    #[test]
    fn 비정상_배율에도_크기가_1_이상이다() {
        assert_eq!(physical_icon_px(0.0), 1);
        assert_eq!(physical_icon_px(-1.0), 1);
        assert_eq!(physical_icon_px(f32::NAN), 1);
    }

    #[test]
    fn 격자에_맞추면_좌상단이_정수_물리_픽셀에_선다() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.3, 20.6), egui::Vec2::splat(96.0));
        let snapped = snap_to_pixels(rect, 2.0);
        assert_eq!(snapped.left() * 2.0, (snapped.left() * 2.0).round());
        assert_eq!(snapped.top() * 2.0, (snapped.top() * 2.0).round());
        // 크기는 건드리지 않는다 — 텍스처와 같은 크기를 유지해야 확대·축소가 없다
        assert_eq!(snapped.size(), rect.size());
    }

    #[test]
    fn 배율이_0이면_그대로_둔다() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.3, 20.6), egui::Vec2::splat(96.0));
        assert_eq!(snap_to_pixels(rect, 0.0), rect);
    }
}
