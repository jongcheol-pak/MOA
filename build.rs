//! 빌드 스크립트 — 실행 파일에 매니페스트·아이콘·버전 정보를 담는다.
//! 셋 다 msvc 링커 인자로 직접 넘겨 build-dependency 없이 처리한다 (plan D1).

use std::path::{Path, PathBuf};

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    embed_manifest(&root);
    embed_icon(&root);
    embed_version();
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

// ── 버전 리소스 ──
// 탐색기 `속성 → 자세히`와 설치 파일이 읽는 버전·제품 이름은 **실행 파일 리소스**에
// 들어 있다. 아이콘과 같은 방식으로 `.res`를 직접 만들어 링커에 넘긴다.
// 형식은 VS_VERSIONINFO — 고정 정보(VS_FIXEDFILEINFO) 하나에 문자열 표와 번역 표가
// 자식으로 붙는 중첩 구조이며, **모든 블록이 32비트 경계에서 시작해야** 한다.

/// 리소스 타입 RT_VERSION
const RT_VERSION: u16 = 16;
/// 버전 리소스 id — 형식이 1로 못 박는다 (탐색기가 이 번호로 찾는다)
const VERSION_ID: u16 = 1;
/// VS_FIXEDFILEINFO의 표식과 구조 판번호
const VS_FFI_SIGNATURE: u32 = 0xFEEF_04BD;
const VS_FFI_STRUCVERSION: u32 = 0x0001_0000;
/// 이 파일이 도는 OS (VOS_NT_WINDOWS32)와 종류 (VFT_APP — 응용 프로그램)
const VOS_NT_WINDOWS32: u32 = 0x0004_0004;
const VFT_APP: u32 = 1;
/// 문자열 표의 이름이자 번역 표의 값 — 언어 en-US(0x0409) + 문자 집합 Unicode(0x04B0).
/// 아이콘 리소스가 쓰는 `LANG_ID`와 같은 언어이며, 값도 영어로 적는다
const STRING_TABLE_KEY: &str = "040904B0";
const TRANSLATION: u32 = 0x04B0_0409;

fn embed_version() {
    // 버전은 `Cargo.toml`에서 온다 — 그 파일이 바뀌면 다시 굽는다
    println!("cargo:rerun-if-changed=Cargo.toml");
    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("app_version.res");
    std::fs::write(&out, build_version_res()).expect("버전 리소스를 쓰지 못했다");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}

/// 버전 리소스 하나만 담은 RES 파일 내용
fn build_version_res() -> Vec<u8> {
    // RES 파일은 빈 항목 하나로 시작한다 (형식이 정한 표식 — `build_res`와 같다)
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
    push_resource(&mut res, RT_VERSION, VERSION_ID, &version_info());
    res
}

/// VS_VERSION_INFO 블록 전체
fn version_info() -> Vec<u8> {
    let parts = version_parts();
    let short = format!("{}.{}.{}", parts[0], parts[1], parts[2]);
    let full = format!("{}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);

    // 문자열 표 — 탐색기 `속성 → 자세히`가 이 이름들로 값을 찾는다
    let strings: [(&str, &str); 8] = [
        ("CompanyName", "jongcheol-pak"),
        ("FileDescription", "MOA - Multi-pane file explorer"),
        ("FileVersion", &full),
        ("InternalName", "moa"),
        ("LegalCopyright", "Copyright (c) 2026 jongcheol-pak"),
        ("OriginalFilename", "moa.exe"),
        ("ProductName", "MOA"),
        ("ProductVersion", &short),
    ];
    let mut table_body = Vec::new();
    for (key, value) in strings {
        append_child(&mut table_body, &version_string(key, value));
    }
    let mut string_info_body = Vec::new();
    append_child(
        &mut string_info_body,
        &version_block(STRING_TABLE_KEY, 0, true, &table_body),
    );
    let string_info = version_block("StringFileInfo", 0, true, &string_info_body);

    // 번역 표 — 위 문자열 표가 어느 언어·문자 집합인지 알린다
    let mut var_info_body = Vec::new();
    append_child(
        &mut var_info_body,
        &version_block("Translation", 4, false, &TRANSLATION.to_le_bytes()),
    );
    let var_info = version_block("VarFileInfo", 0, true, &var_info_body);

    let fixed = fixed_file_info(parts);
    let mut body = Vec::new();
    body.extend_from_slice(&fixed);
    pad4(&mut body);
    append_child(&mut body, &string_info);
    append_child(&mut body, &var_info);
    version_block("VS_VERSION_INFO", fixed.len() as u16, false, &body)
}

/// `Cargo.toml`의 버전을 네 자리로 — 넷째 자리(빌드 번호)는 쓰지 않아 늘 0이다
fn version_parts() -> [u16; 4] {
    let read = |key: &str| {
        std::env::var(key)
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(0)
    };
    [
        read("CARGO_PKG_VERSION_MAJOR"),
        read("CARGO_PKG_VERSION_MINOR"),
        read("CARGO_PKG_VERSION_PATCH"),
        0,
    ]
}

/// VS_FIXEDFILEINFO — 판번호를 사람이 읽는 문자열이 아니라 숫자로 담는 52바이트 고정 블록.
/// 설치 관리자·비교 도구는 문자열이 아니라 이쪽을 본다
fn fixed_file_info(parts: [u16; 4]) -> Vec<u8> {
    let high = (u32::from(parts[0]) << 16) | u32::from(parts[1]);
    let low = (u32::from(parts[2]) << 16) | u32::from(parts[3]);
    let mut info = Vec::with_capacity(52);
    for field in [
        VS_FFI_SIGNATURE,
        VS_FFI_STRUCVERSION,
        high, // FileVersionMS
        low,  // FileVersionLS
        high, // ProductVersionMS
        low,  // ProductVersionLS
        0x3F, // FileFlagsMask — 아래 플래그 중 뜻이 있는 비트
        0,    // FileFlags — 디버그·시험판 표시 없음
        VOS_NT_WINDOWS32,
        VFT_APP,
        0, // FileSubtype — 응용 프로그램에는 뜻이 없다
        0, // FileDateMS
        0, // FileDateLS
    ] {
        info.extend_from_slice(&field.to_le_bytes());
    }
    info
}

/// 문자열 항목 하나 — 값 길이를 **문자 수**로 적는 것이 형식의 규정이다(바이트 수가 아니다)
fn version_string(key: &str, value: &str) -> Vec<u8> {
    let text = wide(value);
    version_block(key, (text.len() / 2) as u16, true, &text)
}

/// 버전 리소스 블록 하나 — 헤더(전체 길이·값 길이·값 종류) + 키 + 정렬 + 본문.
/// `is_text`가 참이면 본문이 UTF-16 문자열, 거짓이면 이진 값이다
fn version_block(key: &str, value_len: u16, is_text: bool, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes()); // wLength — 길이를 알고 나서 채운다
    out.extend_from_slice(&value_len.to_le_bytes());
    out.extend_from_slice(&u16::from(is_text).to_le_bytes());
    out.extend_from_slice(&wide(key));
    pad4(&mut out);
    out.extend_from_slice(body);
    let len = out.len() as u16;
    out[..2].copy_from_slice(&len.to_le_bytes());
    out
}

/// 자식 블록을 부모 본문에 잇는다 — 다음 자식이 32비트 경계에서 시작하도록 맞춘다
fn append_child(body: &mut Vec<u8>, child: &[u8]) {
    body.extend_from_slice(child);
    pad4(body);
}

/// UTF-16LE 바이트 + 종단 널
fn wide(text: &str) -> Vec<u8> {
    let mut out: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// 4바이트 경계까지 0을 채운다
fn pad4(buf: &mut Vec<u8>) {
    while !buf.len().is_multiple_of(4) {
        buf.push(0);
    }
}
