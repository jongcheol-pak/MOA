//! 정보 화면용 앱 아이콘 자산 생성기 (FR-58) — `cargo run --example gen_app_icon`.
//!
//! `docs/AppIcon.png`(1083×1105)를 256×256으로 줄여 `assets/app_icon_256.png`를 만든다.
//! 이 파일은 **커밋되는 생성물**이며 앱은 그것을 읽기만 한다.
//!
//! 원본을 그대로 exe에 담지 않는 이유: 1.3MB가 실행 파일에 얹힌다. 반대로 표시 크기(96px)
//! 그대로 담으면 화면 배율이 100%를 넘는 순간 확대라 흐려진다. 256px은 배율 266%까지
//! **축소만** 일어나는 크기이면서 60KB 안팎에 그친다.
//!
//! 예제 타깃이라 화면 출력과 `main -> Result`를 쓴다(GUI 프로덕션의 `println!` 금지는 콘솔
//! 창이 없는 exe를 겨냥한 것이고, 개발용 CLI에는 오류를 알릴 수단이 필요하다).
use image::imageops::FilterType;
use std::path::PathBuf;

/// 자산 한 변(px) — 위 모듈 주석의 근거로 정한 값
const SIZE: u32 = 256;

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = root.join("docs").join("AppIcon.png");
    let out_dir = root.join("assets");
    let out = out_dir.join("app_icon_256.png");

    let bytes = std::fs::read(&source)
        .map_err(|error| format!("{}: 읽지 못했다 — {error}", source.display()))?;
    let original = image::load_from_memory(&bytes)
        .map_err(|error| format!("{}: PNG가 아니다 — {error}", source.display()))?;
    println!("원본 {}×{}", original.width(), original.height());

    // **정사각으로 맞춘다** — 원본은 1083×1105라 가로가 2% 늘어난다. 여백을 덧대 비율을
    // 지키면 같은 자리에서 아이콘이 그만큼 작아 보이고, 배포 중인 `AppIcon.ico`도 이미
    // 정사각(96×96 등)이라 그것과 다르게 보이게 된다
    let resized = original.resize_exact(SIZE, SIZE, FilterType::Lanczos3);

    std::fs::create_dir_all(&out_dir)
        .map_err(|error| format!("{}: 만들지 못했다 — {error}", out_dir.display()))?;
    resized
        .save(&out)
        .map_err(|error| format!("{}: 쓰지 못했다 — {error}", out.display()))?;

    let written = std::fs::metadata(&out)
        .map_err(|error| format!("{}: 크기를 읽지 못했다 — {error}", out.display()))?
        .len();
    println!("{} {SIZE}×{SIZE} {written}B", out.display());
    Ok(())
}
