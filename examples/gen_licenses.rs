//! 오픈소스 라이선스 고지 자산 생성기 (FR-57) — `cargo run --example gen_licenses`.
//!
//! `assets/licenses.json`을 만든다. 이 파일은 **커밋되는 생성물**이라 앱 빌드는 레지스트리
//! 캐시도 네트워크도 보지 않는다. 예제 타깃에 둔 것은 `cargo build`가 예제를 빌드하지 않아
//! 배포 산출물이 늘지 않으면서도 `cargo test`·`clippy --all-targets`가 컴파일 검사를 해
//! 주기 때문이다.
//!
//! 대상 집합은 **`cargo tree`가 정한다** — `cargo metadata`의 의존 그래프에는 feature가
//! 꺼진 optional까지 남아 실제로 링크되지 않는 크레이트가 섞인다(2026-08-18 실측 155 대 164).
//! 경로·저작자만 metadata에서 얻는다.
//!
//! 예제 타깃이라 화면 출력과 `main -> Result`를 쓴다(GUI 프로덕션의 `println!` 금지는 콘솔
//! 창이 없는 exe를 겨냥한 것이고, 개발용 CLI에는 오류를 알릴 수단이 필요하다).
use moa::app::licenses::{CrateEntry, LicenseData, LicenseText, SCHEMA_VERSION};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// 고지 대상 플랫폼 — 이 앱은 x64 Windows 전용이다 (AGENTS Stack)
const TARGET: &str = "x86_64-pc-windows-msvc";

/// 라이선스 원문으로 볼 파일 이름의 머리 — 대소문자를 가리지 않는다
const LICENSE_PREFIXES: [&str; 5] = ["license", "licence", "copying", "notice", "unlicense"];

fn main() -> Result<(), String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let targets = collect_targets()?;
    println!("대상 크레이트 {}개", targets.len());

    let packages = read_package_info()?;
    let mut builder = TextPool::default();
    let mut crates: Vec<CrateEntry> = Vec::new();

    for (name, version, spdx) in &targets {
        let info = packages
            .get(&(name.clone(), version.clone()))
            .ok_or_else(|| format!("{name} {version}: cargo metadata에 없다"))?;
        let files = read_license_files(&info.dir)?;
        let entry = if files.is_empty() {
            // 배포 패키지에 원문이 없는 크레이트 — SPDX 표준 전문으로 채우고 그 사실을 표시한다
            let indices = standard_indices(&root, spdx, &mut builder)
                .map_err(|err| format!("{name} {version}: {err}"))?;
            CrateEntry {
                name: name.clone(),
                version: version.clone(),
                spdx: spdx.clone(),
                authors: info.authors.clone(),
                text_indices: indices,
                standard_text: true,
                bundled: false,
            }
        } else {
            let indices = files
                .into_iter()
                .map(|(label, body)| builder.intern(label, body))
                .collect();
            CrateEntry {
                name: name.clone(),
                version: version.clone(),
                spdx: spdx.clone(),
                authors: info.authors.clone(),
                text_indices: indices,
                standard_text: false,
                bundled: false,
            }
        };
        crates.push(entry);
    }

    crates.extend(bundled_entries(&root, &packages, &mut builder)?);
    // 대소문자를 섞어 세우면 `Phosphor`가 소문자 크레이트 사이에서 엉뚱한 자리에 선다
    crates.sort_by_key(|entry| entry.name.to_lowercase());

    let lock = std::fs::read_to_string(root.join("Cargo.lock"))
        .map_err(|err| format!("Cargo.lock을 읽지 못했다: {err}"))?;
    let data = LicenseData {
        schema: SCHEMA_VERSION,
        lock_fingerprint: moa::app::licenses::lockfile_fingerprint(&lock),
        crates,
        texts: builder.texts,
    };

    let json = serde_json::to_string_pretty(&data)
        .map_err(|err| format!("자산을 직렬화하지 못했다: {err}"))?;
    let out = root.join("assets").join("licenses.json");
    std::fs::write(&out, json).map_err(|err| format!("{}: {err}", out.display()))?;
    println!(
        "{} — 구성 요소 {}개 · 전문 {}개",
        out.display(),
        data.crates.len(),
        data.texts.len()
    );
    Ok(())
}

// ── 대상 집합 ──

/// `cargo tree`로 실제 링크되는 크레이트의 (이름, 버전, SPDX)를 모은다.
///
/// 출력 한 줄은 `name vX.Y.Z|<spdx>`이고 이미 나온 항목에는 ` (*)`가 붙는다.
/// 자기 자신(`moa`)은 경로가 딸려 오며 고지 대상이 아니다
fn collect_targets() -> Result<Vec<(String, String, String)>, String> {
    let output = run(
        "cargo",
        &[
            "tree", "--target", TARGET, "-e", "normal", "--prefix", "none", "--format", "{p}|{l}",
        ],
    )?;
    let mut seen: Vec<(String, String, String)> = Vec::new();
    for line in output.lines() {
        let line = line.trim().trim_end_matches(" (*)");
        if line.is_empty() {
            continue;
        }
        let (package, spdx) = line
            .split_once('|')
            .ok_or_else(|| format!("cargo tree 출력을 알아볼 수 없다: {line}"))?;
        // 로컬 패키지는 `name vX.Y.Z (경로)` 꼴이라 경로를 떼어낸다
        let package = package.split(" (").next().unwrap_or(package);
        let (name, version) = package
            .rsplit_once(" v")
            .ok_or_else(|| format!("cargo tree 출력을 알아볼 수 없다: {line}"))?;
        if name == env!("CARGO_PKG_NAME") {
            continue;
        }
        let row = (
            name.to_string(),
            version.to_string(),
            spdx.trim().to_string(),
        );
        if !seen.contains(&row) {
            seen.push(row);
        }
    }
    if seen.is_empty() {
        return Err("cargo tree가 대상을 하나도 주지 않았다".into());
    }
    Ok(seen)
}

/// 패키지 하나에서 얻는 것 — 소스 디렉터리와 저작자.
struct PackageInfo {
    dir: PathBuf,
    authors: Vec<String>,
}

/// `cargo metadata`로 이름+버전별 소스 디렉터리와 저작자를 모은다.
fn read_package_info() -> Result<HashMap<(String, String), PackageInfo>, String> {
    let output = run(
        "cargo",
        &[
            "metadata",
            "--filter-platform",
            TARGET,
            "--format-version",
            "1",
        ],
    )?;
    let value: serde_json::Value =
        serde_json::from_str(&output).map_err(|err| format!("cargo metadata 파싱 실패: {err}"))?;
    let packages = value["packages"]
        .as_array()
        .ok_or("cargo metadata에 packages 배열이 없다")?;
    let mut map = HashMap::new();
    for package in packages {
        let (Some(name), Some(version), Some(manifest)) = (
            package["name"].as_str(),
            package["version"].as_str(),
            package["manifest_path"].as_str(),
        ) else {
            continue;
        };
        let dir = Path::new(manifest)
            .parent()
            .ok_or_else(|| format!("{name}: manifest 경로에 부모가 없다"))?
            .to_path_buf();
        let authors = package["authors"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|a| a.as_str())
                    .map(strip_email)
                    .collect()
            })
            .unwrap_or_default();
        map.insert(
            (name.to_string(), version.to_string()),
            PackageInfo { dir, authors },
        );
    }
    Ok(map)
}

/// 저작자 표기에서 메일 주소·계정을 뗀다 — 개인정보를 커밋되는 자산에 담지 않는다.
///
/// crates.io 메타데이터에 공개돼 있더라도 이 레포의 파일로 옮기지 않는 것이 규약이다.
/// `<>`로 감싼 것뿐 아니라 **맨몸으로 붙어 있는 주소·핸들**(`Rich Geldreich rich@…`,
/// `Amod Malviya @amodm` — 둘 다 실측)도 있어 `@`가 든 낱말은 통째로 뺀다
fn strip_email(author: &str) -> String {
    let head = author.split_once('<').map_or(author, |(name, _)| name);
    head.split_whitespace()
        .filter(|word| !word.contains('@'))
        .collect::<Vec<_>>()
        .join(" ")
}

// ── 원문 수집 ──

/// 크레이트 디렉터리 최상위의 라이선스 원문을 (라벨, 본문)으로 모은다.
///
/// 라벨은 `LICENSE-MIT` 같은 이름에서 뒷부분을, 그렇지 않으면 파일 이름을 쓴다 —
/// 화면에서 전문 위에 붙어 무엇을 보고 있는지 알린다
fn read_license_files(dir: &Path) -> Result<Vec<(String, String)>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            return Err(format!(
                "{}를 읽지 못했다({err}) — 레지스트리 캐시가 없으면 `cargo fetch` 후 다시 실행한다",
                dir.display()
            ));
        }
    };
    let mut files: Vec<(String, String)> = Vec::new();
    for entry in entries.flatten() {
        if !entry
            .file_type()
            .map(|kind| kind.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy().to_string();
        let lower = file_name.to_lowercase();
        if !LICENSE_PREFIXES.iter().any(|head| lower.starts_with(head)) {
            continue;
        }
        // UTF-8이 아닌 원문은 손실 없이 담을 수 없어 건너뛴다 — 어느 것을 건너뛰었는지 알린다
        match std::fs::read_to_string(entry.path()) {
            Ok(body) => files.push((label_of(&file_name), body)),
            Err(err) => println!("  건너뜀: {}\\{file_name} ({err})", dir.display()),
        }
    }
    // 디렉터리 열거 순서는 보장되지 않는다 — 자산이 실행마다 달라지지 않게 이름으로 세운다
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// `LICENSE-MIT` → `MIT`, `COPYING` → `COPYING`
fn label_of(file_name: &str) -> String {
    let stem = file_name.split('.').next().unwrap_or(file_name);
    match stem.split_once('-') {
        Some((_, tail)) if !tail.is_empty() => tail.to_string(),
        _ => stem.to_string(),
    }
}

/// SPDX 식별자에 해당하는 표준 전문의 자리 번호들.
///
/// `MIT OR Apache-2.0`처럼 여럿을 선언하면 그 전부를 담는다(선언 그대로 보이고 전문도
/// 둘 다 보이게 한다 — 어느 쪽을 골랐는지 자산이 임의로 정하지 않는다)
fn standard_indices(root: &Path, spdx: &str, pool: &mut TextPool) -> Result<Vec<usize>, String> {
    let mut indices = Vec::new();
    for id in split_spdx(spdx) {
        let path = root.join("assets").join("spdx").join(format!("{id}.txt"));
        let body = std::fs::read_to_string(&path).map_err(|_| {
            format!(
                "표준 전문이 없다 — {}를 두어야 한다({spdx})",
                path.display()
            )
        })?;
        indices.push(pool.intern(id, body));
    }
    if indices.is_empty() {
        return Err(format!("SPDX 식별자를 읽어낼 수 없다: {spdx}"));
    }
    Ok(indices)
}

/// `(MIT OR Apache-2.0) AND Unicode-3.0` 같은 식을 식별자들로 가른다.
fn split_spdx(spdx: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for token in spdx.split([' ', '(', ')', '/']) {
        let token = token.trim();
        if token.is_empty() || matches!(token, "OR" | "AND" | "WITH") {
            continue;
        }
        let id = token.to_string();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

// ── 함께 링크·동봉되는 것 ──

/// 크레이트의 SPDX 필드로는 드러나지 않는 고지 3건.
///
/// `-sys` 크레이트가 담아 정적 링크하는 C 소스와 exe에 담기는 글꼴은 그 크레이트와
/// 라이선스가 다르고, 원문도 크레이트 디렉터리 **하위 경로**에 있어 최상위 훑기에 걸리지 않는다
fn bundled_entries(
    root: &Path,
    packages: &HashMap<(String, String), PackageInfo>,
    pool: &mut TextPool,
) -> Result<Vec<CrateEntry>, String> {
    let mut entries = Vec::new();

    // libssh2 — ssh2 → libssh2-sys가 담은 C 소스 (SFTP)
    let (version, dir) = find_package(packages, "libssh2-sys")?;
    let body = read_bundled(&dir.join("libssh2").join("COPYING"))?;
    entries.push(CrateEntry {
        name: "libssh2 (libssh2-sys 번들)".into(),
        version,
        spdx: "BSD-3-Clause".into(),
        authors: Vec::new(),
        text_indices: vec![pool.intern("BSD-3-Clause", body)],
        standard_text: false,
        bundled: true,
    });

    // zlib — libssh2-sys → libz-sys가 담은 C 소스
    let (version, dir) = find_package(packages, "libz-sys")?;
    let body = read_bundled(&dir.join("src").join("zlib").join("LICENSE"))?;
    entries.push(CrateEntry {
        name: "zlib (libz-sys 번들)".into(),
        version,
        spdx: "Zlib".into(),
        authors: Vec::new(),
        text_indices: vec![pool.intern("Zlib", body)],
        standard_text: false,
        bundled: true,
    });

    // Phosphor Icons — egui-phosphor의 res/*.ttf가 exe에 담긴다.
    // 그 폴더에 라이선스 파일이 없어(크레이트 README가 MIT임을 밝힌다) 표준 전문을 쓴다
    let (version, _) = find_package(packages, "egui-phosphor")?;
    let indices = standard_indices(root, "MIT", pool)?;
    entries.push(CrateEntry {
        name: "Phosphor Icons (글꼴)".into(),
        version,
        spdx: "MIT".into(),
        authors: Vec::new(),
        text_indices: indices,
        standard_text: true,
        bundled: true,
    });

    Ok(entries)
}

fn find_package(
    packages: &HashMap<(String, String), PackageInfo>,
    name: &str,
) -> Result<(String, PathBuf), String> {
    packages
        .iter()
        .find(|((key, _), _)| key == name)
        .map(|((_, version), info)| (version.clone(), info.dir.clone()))
        .ok_or_else(|| format!("{name}을(를) cargo metadata에서 찾지 못했다"))
}

fn read_bundled(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))
}

// ── 전문 중복 제거 ──

/// 같은 내용의 전문을 한 번만 담고 자리 번호를 돌려준다.
#[derive(Default)]
struct TextPool {
    texts: Vec<LicenseText>,
}

impl TextPool {
    fn intern(&mut self, spdx: impl Into<String>, body: String) -> usize {
        // 줄 끝 공백·개행 차이만으로 같은 전문이 두 벌 담기지 않게 다듬는다
        let body = body.replace("\r\n", "\n").trim_end().to_string();
        if let Some(index) = self.texts.iter().position(|text| text.body == body) {
            return index;
        }
        self.texts.push(LicenseText {
            spdx: spdx.into(),
            body,
        });
        self.texts.len() - 1
    }
}

// ── 도구 실행 ──

fn run(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|err| format!("{program} 실행 실패: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} {}: 종료 코드 {:?}\n{}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|err| format!("{program} 출력이 UTF-8이 아니다: {err}"))
}
