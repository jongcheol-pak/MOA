//! 빌드 스크립트 — 실행 파일에 매니페스트와 아이콘을 담는다.
//! 둘 다 msvc 링커 인자로 직접 넘겨 build-dependency 없이 처리한다 (plan D1).

use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    embed_manifest(&root);
    embed_icon(&root);
}

fn embed_manifest(root: &Path) {
    let manifest = root.join("app.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}",
        manifest.display()
    );
    println!("cargo:rerun-if-changed=app.manifest");
}

// ── 아이콘 리소스 ──
// 탐색기·작업 표시줄이 exe에서 읽는 아이콘은 **실행 파일 리소스**에만 들어 있다.
// 링커가 받아들이는 `.res`(RES 파일 형식)를 여기서 직접 만들어 넘긴다 —
// `winres`·`embed-resource` 같은 build-dependency를 들이지 않기 위함이다.

/// 리소스 타입 RT_ICON — 크기별 그림 한 장씩
const RT_ICON: u16 = 3;
/// 리소스 타입 RT_GROUP_ICON — 그림들을 묶는 목차. 탐색기는 이것을 먼저 찾는다
const RT_GROUP_ICON: u16 = 14;
/// 리소스 헤더 크기 — 타입·이름을 둘 다 정수 id로 쓸 때 32바이트 고정
const RES_HEADER_SIZE: u32 = 32;
/// 아이콘 그룹 id. 탐색기는 **가장 작은 id의 그룹**을 그 exe의 아이콘으로 쓴다
const GROUP_ID: u16 = 1;
/// ICO 디렉터리 항목 하나의 크기
const DIR_ENTRY_SIZE: usize = 16;
/// 리소스 언어 (en-US) — 아이콘 조회는 언어를 가리지 않으므로 관례값을 쓴다
const LANG_ID: u16 = 0x0409;
/// 메모리 속성 (MOVEABLE | DISCARDABLE) — PE 리소스에서는 쓰이지 않는 옛 필드지만
/// 형식상 자리가 있어 rc.exe가 아이콘에 넣는 값을 그대로 둔다
const MEMORY_FLAGS: u16 = 0x1010;

fn embed_icon(root: &Path) {
    println!("cargo:rerun-if-changed=docs/AppIcon.ico");
    let ico_path = root.join("docs").join("AppIcon.ico");
    let ico = std::fs::read(&ico_path).expect("docs/AppIcon.ico를 읽지 못했다");
    let res = build_res(&ico).expect("docs/AppIcon.ico가 ICO 형식이 아니다");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("app_icon.res");
    std::fs::write(&out, res).expect("아이콘 리소스를 쓰지 못했다");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}

/// ICO 파일을 RES 파일 내용으로 옮긴다.
/// 그림 데이터는 손대지 않고 그대로 담고, 목차만 리소스용(GRPICONDIR)으로 다시 쓴다
fn build_res(ico: &[u8]) -> Option<Vec<u8>> {
    let count = read_u16(ico, 4)? as usize;
    if read_u16(ico, 0)? != 0 || read_u16(ico, 2)? != 1 || count == 0 {
        return None;
    }
    // RES 파일은 빈 항목 하나로 시작한다 (형식이 정한 표식)
    let mut res: Vec<u8> = vec![
        0, 0, 0, 0, // DataSize
        32, 0, 0, 0, // HeaderSize
        0xFF, 0xFF, 0, 0, // 타입: 정수 id 0
        0xFF, 0xFF, 0, 0, // 이름: 정수 id 0
        0, 0, 0, 0, // DataVersion
        0, 0, // MemoryFlags
        0, 0, // LanguageId
        0, 0, 0, 0, // Version
        0, 0, 0, 0, // Characteristics
    ];
    // 목차 — ICO 항목의 앞 12바이트는 그대로 쓰고, 파일 오프셋 자리만 리소스 id로 바꾼다
    let mut group: Vec<u8> = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes());
    group.extend_from_slice(&(count as u16).to_le_bytes());
    for index in 0..count {
        let entry = 6 + index * DIR_ENTRY_SIZE;
        let offset = read_u32(ico, entry + 12)? as usize;
        let size = read_u32(ico, entry + 8)? as usize;
        let image = ico.get(offset..offset.checked_add(size)?)?;
        // 그림 id는 1부터 — 그룹 id와 번호 공간이 달라 겹쳐도 무방하다
        let icon_id = (index + 1) as u16;
        push_resource(&mut res, RT_ICON, icon_id, image);
        group.extend_from_slice(ico.get(entry..entry + 12)?);
        group.extend_from_slice(&icon_id.to_le_bytes());
    }
    push_resource(&mut res, RT_GROUP_ICON, GROUP_ID, &group);
    Some(res)
}

/// 리소스 하나(헤더 + 데이터)를 RES 파일에 이어 붙인다. 데이터는 4바이트 경계로 맞춘다
fn push_resource(res: &mut Vec<u8>, type_id: u16, name_id: u16, data: &[u8]) {
    res.extend_from_slice(&(data.len() as u32).to_le_bytes());
    res.extend_from_slice(&RES_HEADER_SIZE.to_le_bytes());
    // 0xFFFF는 "뒤따르는 2바이트가 문자열이 아니라 정수 id"라는 표식이다
    res.extend_from_slice(&0xFFFFu16.to_le_bytes());
    res.extend_from_slice(&type_id.to_le_bytes());
    res.extend_from_slice(&0xFFFFu16.to_le_bytes());
    res.extend_from_slice(&name_id.to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes()); // DataVersion
    res.extend_from_slice(&MEMORY_FLAGS.to_le_bytes());
    res.extend_from_slice(&LANG_ID.to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes()); // Version
    res.extend_from_slice(&0u32.to_le_bytes()); // Characteristics
    res.extend_from_slice(data);
    while !res.len().is_multiple_of(4) {
        res.push(0);
    }
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    let raw = bytes.get(at..at + 2)?;
    Some(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let raw = bytes.get(at..at + 4)?;
    Some(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}
