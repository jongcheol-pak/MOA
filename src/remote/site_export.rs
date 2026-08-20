//! 사이트 목록 내보내기·가져오기 문서 (FR-59).
//!
//! 등록된 사이트 전부를 파일 하나로 옮기기 위한 것이다. 담기는 것은 사이트의 설정과
//! 비밀번호이며, **비밀번호는 `remote::envelope`가 사용자 암호로 봉한 덩어리 하나**로만 나간다
//! (plan D1) — 사이트마다 흩어 두지 않는 이유는 봉인 한 번으로 끝나고 부분만 새 나갈 자리도
//! 없기 때문이다.
//!
//! **`password_sealed`(DPAPI 봉인 바이트)는 문서에 담지 않는다.** 담으면 같은 PC·같은 계정에서는
//! 그것이 그대로 풀려, 「암호를 비워 비밀번호 없이 내보낸 파일」에서도 비밀번호가 되살아난다
//! (plan D6이 무너진다).
//!
//! 화면을 모른다 — 실패는 열거형으로 올리고 문구는 화면 계층이 정한다 (AGENTS 계층 규약).
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::remote::envelope::{self, Envelope};
use crate::remote::sites::SiteStore;
use crate::remote::types::{
    Charset, Encryption, LogonType, Protocol, SiteId, SiteRecord, TransferMode,
};

/// 파일이 자기를 밝히는 이름 — 다른 JSON을 골랐을 때 그 자리에서 거른다
pub const FORMAT: &str = "moa-sites";
/// 문서 판 번호. 모르는 판은 **읽지 않고 거부한다** — 절반만 해석하면 조용히 어긋난 목록이 된다
pub const VERSION: u32 = 1;

/// 내보낸 사이트 한 벌.
///
/// `SiteRecord`에서 **`id`와 `password_sealed`를 뺀** 나머지에 사이드바 숨김 여부를 더한 것이다.
/// `id`를 빼는 이유는 그 번호가 이 PC 안에서만 뜻이 있어서고(가져오는 쪽이 새로 발급한다),
/// `password_sealed`를 빼는 이유는 모듈 주석에 적은 그대로다
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportedSite {
    pub name: String,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub encryption: Encryption,
    pub logon: LogonType,
    pub user: String,
    pub transfer_mode: TransferMode,
    pub connection_limit: Option<u8>,
    pub charset: Charset,
    /// 사이드바에서 숨긴 사이트인가 — 그것도 그 사이트의 설정이다 (plan D12)
    #[serde(default)]
    pub hidden: bool,
}

/// 파일에 실리는 문서 전체
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteExport {
    pub format: String,
    pub version: u32,
    pub sites: Vec<ExportedSite>,
    /// 비밀번호 묶음 — 암호를 비우고 내보냈으면 `None`이다.
    ///
    /// 푼 내용은 `Vec<String>`을 직렬화한 것이며 **`sites`와 같은 순서·같은 길이**다.
    /// 사이트에 식별자가 없으므로 순서가 둘을 잇는 유일한 끈이다
    #[serde(default)]
    pub secret: Option<Envelope>,
}

/// 내보내기가 실패한 까닭
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// 암호로 봉하지 못했다 — 봉하지 못한 것을 평문으로 대신 담는 길은 두지 않는다
    Seal,
    /// 문서를 만들지 못했다 (직렬화 실패)
    Serialize,
    /// 파일을 쓰지 못했다 — 담긴 문자열은 OS가 준 사유다
    Io(String),
}

/// 가져오기가 실패한 까닭
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportError {
    /// 우리 파일이 아니거나 읽을 수 없는 내용이다
    Broken,
    /// 모르는 판 번호 — 이 앱보다 나중에 만들어진 파일이다
    Unsupported,
    /// 암호가 맞지 않거나 파일이 손상됐다 (둘을 구분하지 않는다 — `envelope` 주석 참조)
    WrongPassphrase,
    /// 파일을 읽지 못했다
    Io(String),
}

/// 내보내기 결과 — 문서와 함께 **비밀번호를 싣지 못한 사이트 수**를 돌려준다 (plan D15).
///
/// 조용히 빼면 사용자는 다른 PC에서 연결할 때야 그 사실을 안다
#[derive(Debug, Clone, PartialEq)]
pub struct ExportOutcome {
    pub document: SiteExport,
    /// 저장된 비밀번호가 있는데 **풀지 못한** 사이트 수.
    /// 애초에 저장된 것이 없는 사이트(익명 등)는 세지 않는다 — 그건 실패가 아니다
    pub password_unreadable: usize,
}

/// 겹침 판정 열쇠 — `(호스트, 포트, 프로토콜, 실제 사용자)` (plan D3).
///
/// 호스트만 소문자로 맞춘다. DNS는 대소문자를 가리지 않지만 사용자 이름은 서버가 가릴 수 있어
/// 함부로 접으면 서로 다른 계정을 같은 것으로 본다
/// `Hash`를 파생하지 않는다 — 찾는 곳이 목록 훑기 한 곳뿐이라 해시가 필요 없고,
/// 그것 하나 때문에 `Protocol`에 `Hash`를 더하면 이 모듈이 도메인 타입을 바꾸게 된다
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictKey {
    host: String,
    port: u16,
    protocol: Protocol,
    user: String,
}

pub fn conflict_key(host: &str, port: u16, protocol: Protocol, user: &str) -> ConflictKey {
    ConflictKey {
        host: host.trim().to_lowercase(),
        port,
        protocol,
        user: user.trim().to_owned(),
    }
}

/// 문서의 한 항목과 그에 딸린 평문 비밀번호
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedSite {
    pub site: ExportedSite,
    /// 봉투에서 풀어낸 비밀번호 — 봉투가 없거나 빈 값이면 `None`
    pub password: Option<String>,
}

/// 가져오기 전에 세운 계획 — 어느 것이 새것이고 어느 것이 겹치는가
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImportPlan {
    /// 목록에 없던 사이트들
    pub fresh: Vec<PreparedSite>,
    /// 이미 있는 사이트와 겹친 것들 — `(기존 사이트 이름, 기존 식별자, 새 내용)`
    pub conflicts: Vec<(String, SiteId, PreparedSite)>,
}

impl ImportPlan {
    pub fn is_empty(&self) -> bool {
        self.fresh.is_empty() && self.conflicts.is_empty()
    }

    /// 겹치는 사이트의 이름들 — 확인 대화가 그대로 보여 준다
    pub fn conflict_names(&self) -> Vec<String> {
        self.conflicts
            .iter()
            .map(|(name, _, _)| name.clone())
            .collect()
    }
}

/// 가져오기를 마친 뒤의 셈
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportSummary {
    /// 새로 더한 사이트 수
    pub added: usize,
    /// 덮어쓴 사이트 수
    pub replaced: usize,
    /// 겹쳐서 건너뛴 사이트 수
    pub skipped: usize,
    /// 설정은 들어왔지만 **비밀번호를 봉하지 못한** 사이트 수 (plan D15)
    pub password_failed: usize,
}

/// 등록된 사이트 전부를 문서로 만든다.
///
/// `passphrase`가 비어 있으면 비밀번호를 **담지 않는다** (plan D6) — 그 판단은 화면이 이미
/// 내렸고 여기서는 받은 대로 따른다
pub fn build(store: &SiteStore, passphrase: &str) -> Result<ExportOutcome, ExportError> {
    let mut sites = Vec::with_capacity(store.sites().len());
    let mut passwords = Vec::with_capacity(store.sites().len());
    let mut password_unreadable = 0usize;

    for record in store.sites() {
        sites.push(ExportedSite {
            name: record.name.clone(),
            protocol: record.protocol,
            host: record.host.clone(),
            port: record.port,
            encryption: record.encryption,
            logon: record.logon,
            user: record.user.clone(),
            transfer_mode: record.transfer_mode,
            connection_limit: record.connection_limit,
            charset: record.charset.clone(),
            hidden: store.is_hidden(record.id),
        });
        match store.password(record.id) {
            Some(plain) => passwords.push(plain),
            None => {
                // 저장된 것이 없으면 실패가 아니다 — 봉인 바이트가 있는데 못 푼 것만 센다
                if !record.password_sealed.is_empty() {
                    password_unreadable += 1;
                }
                passwords.push(String::new());
            }
        }
    }

    let secret = if passphrase.is_empty() {
        None
    } else {
        let mut plain = serde_json::to_vec(&passwords).map_err(|_| ExportError::Serialize)?;
        let sealed = envelope::seal_with_passphrase(&plain, passphrase);
        // 모아 둔 평문 묶음은 여기서 지운다 — 봉인 결과와 나란히 메모리에 남기지 않는다.
        // `passwords`(개별 `String`)는 지우지 않고 떨어뜨린다: `String`의 안쪽 버퍼를 0으로
        // 채우려면 `as_bytes_mut`이 필요한데, 그 unsafe를 들일 만큼 얻는 것이 없다 —
        // 같은 평문이 이미 `SiteStore::password`가 돌려준 `String`으로 이 함수 밖에도 있다
        crate::remote::secret::zeroize(&mut plain);
        Some(sealed.ok_or(ExportError::Seal)?)
    };

    Ok(ExportOutcome {
        document: SiteExport {
            format: FORMAT.to_owned(),
            version: VERSION,
            sites,
            secret,
        },
        password_unreadable,
    })
}

/// 문서를 파일로 쓴다 — 사람이 열어 볼 수 있게 들여쓴 JSON이다
pub fn write_file(path: &Path, document: &SiteExport) -> Result<(), ExportError> {
    let json = serde_json::to_string_pretty(document).map_err(|_| ExportError::Serialize)?;
    std::fs::write(path, json).map_err(|error| ExportError::Io(error.to_string()))
}

/// 파일에서 문서를 읽는다 — 우리 형식이 아니거나 모르는 판이면 거부한다
pub fn read_file(path: &Path) -> Result<SiteExport, ImportError> {
    let text = std::fs::read_to_string(path).map_err(|error| ImportError::Io(error.to_string()))?;
    parse(&text)
}

/// 문서 문자열을 해석한다 (파일 I/O와 나눠 둔다 — 이쪽만 시험 대상이다)
pub fn parse(text: &str) -> Result<SiteExport, ImportError> {
    let document: SiteExport = serde_json::from_str(text).map_err(|_| ImportError::Broken)?;
    if document.format != FORMAT {
        return Err(ImportError::Broken);
    }
    if document.version != VERSION {
        return Err(ImportError::Unsupported);
    }
    Ok(document)
}

/// 이 문서를 열려면 암호가 필요한가
pub fn needs_passphrase(document: &SiteExport) -> bool {
    document.secret.is_some()
}

/// 문서를 지금 목록과 견줘 계획을 세운다.
///
/// 문서 안에 같은 열쇠가 둘 이상 있으면 **뒤엣것이 이긴다** — 손으로 이어 붙인 파일에서
/// 생길 수 있는 일이고, 그때 앞엣것까지 각각 반영하면 목록에 같은 서버가 두 번 앉는다
pub fn plan_import(
    document: &SiteExport,
    store: &SiteStore,
    passphrase: &str,
) -> Result<ImportPlan, ImportError> {
    let passwords = match &document.secret {
        None => vec![String::new(); document.sites.len()],
        Some(envelope) => {
            let plain = envelope::open_with_passphrase(envelope, passphrase)
                .ok_or(ImportError::WrongPassphrase)?;
            let list: Vec<String> =
                serde_json::from_slice(&plain).map_err(|_| ImportError::Broken)?;
            if list.len() != document.sites.len() {
                return Err(ImportError::Broken);
            }
            list
        }
    };

    // 같은 열쇠가 여럿이면 뒤엣것만 남긴다
    let mut chosen: Vec<(ConflictKey, PreparedSite)> = Vec::new();
    for (index, site) in document.sites.iter().enumerate() {
        let key = conflict_key(&site.host, site.port, site.protocol, effective_user(site));
        let password = passwords
            .get(index)
            .filter(|value| !value.is_empty())
            .cloned();
        let prepared = PreparedSite {
            site: site.clone(),
            password,
        };
        match chosen.iter_mut().find(|(existing, _)| *existing == key) {
            Some((_, slot)) => *slot = prepared,
            None => chosen.push((key, prepared)),
        }
    }

    let mut plan = ImportPlan::default();
    for (key, prepared) in chosen {
        match find_existing(store, &key) {
            Some(record) => plan
                .conflicts
                .push((record.name.clone(), record.id, prepared)),
            None => plan.fresh.push(prepared),
        }
    }
    Ok(plan)
}

/// 계획을 목록에 반영한다. `overwrite`가 거짓이면 겹치는 것은 건드리지 않는다
pub fn apply_import(store: &mut SiteStore, plan: &ImportPlan, overwrite: bool) -> ImportSummary {
    let mut summary = ImportSummary::default();

    for prepared in &plan.fresh {
        let id = store.add(&prepared.site.name);
        write_record(store, id, &prepared.site);
        apply_password(store, id, prepared, &mut summary);
        apply_hidden(store, id, prepared.site.hidden);
        summary.added += 1;
    }

    for (_, id, prepared) in &plan.conflicts {
        if !overwrite {
            summary.skipped += 1;
            continue;
        }
        write_record(store, *id, &prepared.site);
        apply_password(store, *id, prepared, &mut summary);
        apply_hidden(store, *id, prepared.site.hidden);
        summary.replaced += 1;
    }
    summary
}

/// 기록을 갈아 끼운다 — **식별자는 그대로 두고** 이름 겹침만 `(2)`로 가른다 (plan D14).
///
/// `SiteStore::get_mut`이 아니라 `insert`를 쓰는 이유가 그것이다: `get_mut`은 이름을 손대지 않아
/// 덮어쓴 이름이 다른 사이트와 그대로 겹친다
fn write_record(store: &mut SiteStore, id: SiteId, site: &ExportedSite) {
    // 봉인된 비밀번호는 여기서 옮기지 않는다 — 있던 것을 지키고(D16), 새 것은 `apply_password`가 담는다
    let kept = store
        .get(id)
        .map(|record| record.password_sealed.clone())
        .unwrap_or_default();
    store.insert(SiteRecord {
        id,
        name: site.name.clone(),
        protocol: site.protocol,
        host: site.host.clone(),
        port: site.port,
        encryption: site.encryption,
        logon: site.logon,
        user: site.user.clone(),
        password_sealed: kept,
        transfer_mode: site.transfer_mode,
        connection_limit: site.connection_limit,
        charset: site.charset.clone(),
    });
}

/// 비밀번호를 담는다 — **문서에 없으면 있던 것을 지우지 않는다** (plan D16).
///
/// 「비밀번호를 빼고 내보냈다」는 것은 「비밀번호를 지우겠다」는 뜻이 아니다. 없는 것으로 있는 것을
/// 덮으면 사용자가 기대하지 않은 자리에서 로그인 정보가 사라진다.
///
/// 담지 못하면 그 사실을 셈에 남긴다 (plan D15) — 나머지 사이트는 그대로 반영하고, 화면이
/// 「N개는 비밀번호를 저장하지 못했습니다」로 알린다. **DPAPI 봉인 실패 자체는 시험에서
/// 만들어 낼 수 없으므로**(`CryptProtectData`를 실패시킬 길이 없다) 이 분기는 `set_password`가
/// 거짓을 주는 **다른 갈래**(대상 사이트가 없을 때 — `sites.rs`의 `get_mut`이 `None`)로 덮는다
fn apply_password(
    store: &mut SiteStore,
    id: SiteId,
    prepared: &PreparedSite,
    summary: &mut ImportSummary,
) {
    let Some(password) = &prepared.password else {
        return;
    };
    if !store.set_password(id, password) {
        summary.password_failed += 1;
    }
}

fn apply_hidden(store: &mut SiteStore, id: SiteId, hidden: bool) {
    if hidden {
        store.hide(id);
    } else {
        store.unhide(id);
    }
}

/// 로그인에 실제로 쓰이는 사용자 이름 — 익명이면 `anonymous`다 (`SiteRecord::effective_user`와 같은 규칙)
fn effective_user(site: &ExportedSite) -> &str {
    match site.logon {
        LogonType::Anonymous => "anonymous",
        LogonType::Normal => &site.user,
    }
}

/// 같은 접속 대상이 목록에 이미 있는가
fn find_existing<'a>(store: &'a SiteStore, key: &ConflictKey) -> Option<&'a SiteRecord> {
    store.sites().iter().find(|record| {
        conflict_key(
            &record.host,
            record.port,
            record.protocol,
            record.effective_user(),
        ) == *key
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::types::CONNECTION_LIMIT_RANGE;

    /// 시험용 사이트 하나를 채워 넣는다
    fn add_site(store: &mut SiteStore, name: &str, host: &str, user: &str) -> SiteId {
        let id = store.add(name);
        if let Some(record) = store.get_mut(id) {
            record.protocol = Protocol::Sftp;
            record.host = host.to_owned();
            record.port = 2222;
            record.user = user.to_owned();
            record.transfer_mode = TransferMode::Passive;
            record.connection_limit = Some(*CONNECTION_LIMIT_RANGE.end());
            record.charset = Charset::Named("CP949".to_owned());
        }
        id
    }

    #[test]
    fn 설정과_비밀번호가_그대로_왕복한다() {
        let mut source = SiteStore::new();
        let first = add_site(&mut source, "배포 서버", "deploy.test", "deploy");
        let second = add_site(&mut source, "숨긴 서버", "hidden.test", "hide");
        let third = source.add("익명 서버");
        if let Some(record) = source.get_mut(third) {
            record.host = "anon.test".to_owned();
            record.logon = LogonType::Anonymous;
        }
        assert!(source.set_password(first, "비밀!1234"));
        assert!(source.set_password(second, "another"));
        source.hide(second);

        let outcome = build(&source, "내보내기 암호").expect("내보내기");
        assert_eq!(outcome.password_unreadable, 0);
        let text = serde_json::to_string_pretty(&outcome.document).expect("직렬화");

        let document = parse(&text).expect("해석");
        let mut target = SiteStore::new();
        let plan = plan_import(&document, &target, "내보내기 암호").expect("계획");
        assert_eq!(plan.fresh.len(), 3);
        assert!(plan.conflicts.is_empty());
        let summary = apply_import(&mut target, &plan, true);
        assert_eq!(summary.added, 3);
        assert_eq!(summary.password_failed, 0);

        // `id`를 뺀 열 필드와 숨김 여부가 원본과 같다
        for original in source.sites() {
            let copied = target
                .sites()
                .iter()
                .find(|record| record.host == original.host)
                .expect("옮겨온 사이트");
            assert_eq!(copied.name, original.name);
            assert_eq!(copied.protocol, original.protocol);
            assert_eq!(copied.port, original.port);
            assert_eq!(copied.encryption, original.encryption);
            assert_eq!(copied.logon, original.logon);
            assert_eq!(copied.user, original.user);
            assert_eq!(copied.transfer_mode, original.transfer_mode);
            assert_eq!(copied.connection_limit, original.connection_limit);
            assert_eq!(copied.charset, original.charset);
            assert_eq!(
                target.is_hidden(copied.id),
                source.is_hidden(original.id),
                "{} 숨김 여부",
                original.name
            );
            // 비밀번호는 푼 평문끼리 견준다
            assert_eq!(target.password(copied.id), source.password(original.id));
        }
    }

    #[test]
    fn 암호를_비우면_비밀번호가_담기지_않는다() {
        let mut source = SiteStore::new();
        let id = add_site(&mut source, "배포 서버", "deploy.test", "deploy");
        assert!(source.set_password(id, "찾을수있는평문"));

        let outcome = build(&source, "").expect("내보내기");
        assert!(outcome.document.secret.is_none(), "봉투가 없어야 한다");
        let text = serde_json::to_string(&outcome.document).expect("직렬화");
        assert!(!text.contains("찾을수있는평문"), "평문이 남았다");
        assert!(
            !text.contains("password_sealed"),
            "DPAPI 봉인 바이트가 문서에 실렸다: {text}"
        );

        // 같은 PC에서 가져와도 비밀번호는 비어 있다
        let document = parse(&text).expect("해석");
        assert!(!needs_passphrase(&document));
        let mut target = SiteStore::new();
        let plan = plan_import(&document, &target, "").expect("계획");
        assert_eq!(plan.fresh.len(), 1);
        assert_eq!(plan.fresh[0].password, None);
        apply_import(&mut target, &plan, true);
        let copied = target.sites().first().expect("사이트");
        assert_eq!(target.password(copied.id), None);
    }

    #[test]
    fn 암호를_넣어도_문서에_봉인_바이트가_없다() {
        let mut source = SiteStore::new();
        let id = add_site(&mut source, "배포 서버", "deploy.test", "deploy");
        assert!(source.set_password(id, "찾을수있는평문"));

        let outcome = build(&source, "암호").expect("내보내기");
        let text = serde_json::to_string(&outcome.document).expect("직렬화");
        assert!(!text.contains("찾을수있는평문"));
        assert!(!text.contains("password_sealed"), "봉인 바이트가 실렸다");
    }

    #[test]
    fn 틀린_암호로는_계획을_세우지_못한다() {
        let mut source = SiteStore::new();
        let id = add_site(&mut source, "배포 서버", "deploy.test", "deploy");
        assert!(source.set_password(id, "비밀"));
        let document = build(&source, "맞는 암호").expect("내보내기").document;

        let mut target = SiteStore::new();
        assert_eq!(
            plan_import(&document, &target, "틀린 암호"),
            Err(ImportError::WrongPassphrase)
        );
        // 저장소는 그대로다
        assert!(target.is_empty());
        assert_eq!(
            apply_import(&mut target, &ImportPlan::default(), true),
            ImportSummary::default()
        );
    }

    #[test]
    fn 같은_접속_대상만_겹침으로_본다() {
        let mut source = SiteStore::new();
        add_site(&mut source, "배포 서버", "Deploy.Test", "deploy");
        let document = build(&source, "").expect("내보내기").document;

        // 호스트 대소문자만 다른 같은 대상 → 겹침
        let mut target = SiteStore::new();
        add_site(&mut target, "이미 있던 것", "deploy.test", "deploy");
        let plan = plan_import(&document, &target, "").expect("계획");
        assert_eq!(plan.conflicts.len(), 1, "대소문자는 접어서 본다");
        assert!(plan.fresh.is_empty());
        assert_eq!(plan.conflict_names(), vec!["이미 있던 것".to_owned()]);

        // 사용자만 달라도 다른 대상
        let mut target = SiteStore::new();
        add_site(&mut target, "다른 계정", "deploy.test", "other");
        assert_eq!(
            plan_import(&document, &target, "")
                .expect("계획")
                .fresh
                .len(),
            1
        );

        // 포트만 달라도 다른 대상
        let mut target = SiteStore::new();
        let id = add_site(&mut target, "다른 포트", "deploy.test", "deploy");
        if let Some(record) = target.get_mut(id) {
            record.port = 22;
        }
        assert_eq!(
            plan_import(&document, &target, "")
                .expect("계획")
                .fresh
                .len(),
            1
        );

        // 프로토콜만 달라도 다른 대상
        let mut target = SiteStore::new();
        let id = add_site(&mut target, "다른 프로토콜", "deploy.test", "deploy");
        if let Some(record) = target.get_mut(id) {
            record.protocol = Protocol::Ftp;
        }
        assert_eq!(
            plan_import(&document, &target, "")
                .expect("계획")
                .fresh
                .len(),
            1
        );
    }

    #[test]
    fn 덮어쓰기와_건너뛰기가_셈에_그대로_나온다() {
        let mut source = SiteStore::new();
        add_site(&mut source, "겹치는 서버", "same.test", "deploy");
        add_site(&mut source, "새 서버", "fresh.test", "deploy");
        let document = build(&source, "").expect("내보내기").document;

        let mut target = SiteStore::new();
        let existing = add_site(&mut target, "옛 이름", "same.test", "deploy");
        if let Some(record) = target.get_mut(existing) {
            record.transfer_mode = TransferMode::Active;
        }

        // 건너뛰기 — 겹친 것은 그대로 둔다
        let mut skipped_store = target.clone();
        let plan = plan_import(&document, &skipped_store, "").expect("계획");
        let summary = apply_import(&mut skipped_store, &plan, false);
        assert_eq!(
            summary,
            ImportSummary {
                added: 1,
                replaced: 0,
                skipped: 1,
                password_failed: 0
            }
        );
        let kept = skipped_store.get(existing).expect("그대로 있는 사이트");
        assert_eq!(kept.name, "옛 이름");
        assert_eq!(kept.transfer_mode, TransferMode::Active);

        // 덮어쓰기 — 식별자는 지키고 값만 바뀐다
        let plan = plan_import(&document, &target, "").expect("계획");
        let summary = apply_import(&mut target, &plan, true);
        assert_eq!(
            summary,
            ImportSummary {
                added: 1,
                replaced: 1,
                skipped: 0,
                password_failed: 0
            }
        );
        let replaced = target.get(existing).expect("덮어쓴 사이트");
        assert_eq!(replaced.name, "겹치는 서버");
        assert_eq!(replaced.transfer_mode, TransferMode::Passive);
        assert_eq!(target.sites().len(), 2);
    }

    #[test]
    fn 덮어쓸_이름이_다른_사이트와_겹치면_번호가_붙는다() {
        // `insert` 경로를 쓴다는 것이 이 단언으로 판정된다 (plan D14)
        let mut source = SiteStore::new();
        add_site(&mut source, "배포 서버", "same.test", "deploy");
        let document = build(&source, "").expect("내보내기").document;

        let mut target = SiteStore::new();
        // 같은 이름을 이미 다른 사이트가 쓰고 있다
        add_site(&mut target, "배포 서버", "other.test", "other");
        let existing = add_site(&mut target, "덮일 사이트", "same.test", "deploy");

        let plan = plan_import(&document, &target, "").expect("계획");
        apply_import(&mut target, &plan, true);
        let replaced = target.get(existing).expect("덮어쓴 사이트");
        assert_eq!(replaced.name, "배포 서버 (2)");
        assert_eq!(target.sites().len(), 2, "새로 만들지 않는다");
    }

    #[test]
    fn 문서에_없는_비밀번호는_있던_것을_지우지_않는다() {
        // plan D16 — 「비밀번호를 빼고 내보냈다」가 「비밀번호를 지우겠다」는 뜻은 아니다
        let mut source = SiteStore::new();
        add_site(&mut source, "배포 서버", "same.test", "deploy");
        let document = build(&source, "").expect("내보내기").document;

        let mut target = SiteStore::new();
        let existing = add_site(&mut target, "옛 이름", "same.test", "deploy");
        assert!(target.set_password(existing, "지키고 싶은 비밀번호"));

        let plan = plan_import(&document, &target, "").expect("계획");
        apply_import(&mut target, &plan, true);
        assert_eq!(
            target.password(existing).as_deref(),
            Some("지키고 싶은 비밀번호")
        );
    }

    #[test]
    fn 문서_안에_같은_대상이_둘이면_뒤엣것이_이긴다() {
        let mut source = SiteStore::new();
        add_site(&mut source, "먼저", "same.test", "deploy");
        add_site(&mut source, "나중", "same.test", "deploy");
        // 저장소는 이름만 갈라 둘을 다 받는다 — 문서에도 둘이 실린다
        let document = build(&source, "").expect("내보내기").document;
        assert_eq!(document.sites.len(), 2);

        let target = SiteStore::new();
        let plan = plan_import(&document, &target, "").expect("계획");
        assert_eq!(plan.fresh.len(), 1, "하나로 접힌다");
        assert_eq!(plan.fresh[0].site.name, "나중");
    }

    #[test]
    fn 사이트가_없으면_빈_문서가_되고_가져와도_아무_일도_없다() {
        let source = SiteStore::new();
        let outcome = build(&source, "암호").expect("내보내기");
        assert!(outcome.document.sites.is_empty());

        let mut target = SiteStore::new();
        let plan = plan_import(&outcome.document, &target, "암호").expect("계획");
        assert!(plan.is_empty());
        assert_eq!(
            apply_import(&mut target, &plan, true),
            ImportSummary::default()
        );
        assert!(target.is_empty());
    }

    #[test]
    fn 우리_파일이_아니거나_모르는_판은_거부한다() {
        assert_eq!(parse(""), Err(ImportError::Broken));
        assert_eq!(parse("{}"), Err(ImportError::Broken));
        assert_eq!(parse("깨진 내용"), Err(ImportError::Broken));
        assert_eq!(
            parse(r#"{"format":"other-tool","version":1,"sites":[]}"#),
            Err(ImportError::Broken)
        );
        assert_eq!(
            parse(r#"{"format":"moa-sites","version":99,"sites":[]}"#),
            Err(ImportError::Unsupported)
        );
        // 판이 맞으면 봉투가 없어도 읽힌다
        let document = parse(r#"{"format":"moa-sites","version":1,"sites":[]}"#).expect("해석");
        assert!(document.secret.is_none());
    }

    #[test]
    fn 비밀번호_묶음의_길이가_어긋나면_깨진_파일이다() {
        // 손으로 사이트를 지운 파일이 여기 걸린다
        let mut source = SiteStore::new();
        let first = add_site(&mut source, "첫째", "one.test", "a");
        add_site(&mut source, "둘째", "two.test", "b");
        assert!(source.set_password(first, "비밀"));
        let mut document = build(&source, "암호").expect("내보내기").document;
        document.sites.pop();

        let target = SiteStore::new();
        assert_eq!(
            plan_import(&document, &target, "암호"),
            Err(ImportError::Broken)
        );
    }

    #[test]
    fn 풀지_못한_비밀번호는_세어서_알린다() {
        // 다른 계정에서 온 봉인 바이트 — 설정은 담되 그 사실을 셈에 남긴다 (plan D15)
        let mut source = SiteStore::new();
        let id = add_site(&mut source, "남의 설정", "other.test", "deploy");
        if let Some(record) = source.get_mut(id) {
            record.password_sealed = vec![0xde, 0xad, 0xbe, 0xef];
        }
        let outcome = build(&source, "암호").expect("내보내기");
        assert_eq!(outcome.password_unreadable, 1);
        assert_eq!(outcome.document.sites.len(), 1, "설정 자체는 담긴다");

        // 저장된 것이 애초에 없는 사이트는 세지 않는다
        let mut source = SiteStore::new();
        add_site(&mut source, "비밀번호 없음", "none.test", "deploy");
        assert_eq!(
            build(&source, "암호")
                .expect("내보내기")
                .password_unreadable,
            0
        );
    }

    #[test]
    fn 비밀번호를_담지_못하면_셈에_남고_나머지는_그대로_반영된다() {
        // plan D15의 가져오기 쪽 절반. `set_password`가 거짓을 주는 갈래로 분기를 덮는다 —
        // 대상 사이트가 없으면 봉인에 성공해도 담을 곳이 없어 거짓이다(`sites.rs`의 `get_mut`)
        let mut store = SiteStore::new();
        let mut summary = ImportSummary::default();
        let prepared = PreparedSite {
            site: ExportedSite {
                name: "없는 사이트".to_owned(),
                protocol: Protocol::Ftp,
                host: "gone.test".to_owned(),
                port: 21,
                encryption: Encryption::default(),
                logon: LogonType::Normal,
                user: "user".to_owned(),
                transfer_mode: TransferMode::default(),
                connection_limit: None,
                charset: Charset::Utf8,
                hidden: false,
            },
            password: Some("담기지 못할 비밀번호".to_owned()),
        };
        apply_password(&mut store, SiteId(999), &prepared, &mut summary);
        assert_eq!(summary.password_failed, 1, "담지 못한 것을 세어야 한다");

        // 담을 곳이 있으면 세지 않는다
        let id = add_site(&mut store, "정상 사이트", "ok.test", "user");
        let mut summary = ImportSummary::default();
        apply_password(&mut store, id, &prepared, &mut summary);
        assert_eq!(summary.password_failed, 0);
        assert_eq!(store.password(id).as_deref(), Some("담기지 못할 비밀번호"));

        // 문서에 비밀번호가 없으면 셈에도 손대지 않는다 (D16 — 지우지 않으므로 실패도 아니다)
        let mut summary = ImportSummary::default();
        let without = PreparedSite {
            password: None,
            ..prepared.clone()
        };
        apply_password(&mut store, SiteId(999), &without, &mut summary);
        assert_eq!(summary.password_failed, 0);
    }

    #[test]
    fn 아주_긴_이름과_비ascii_이름도_왕복한다() {
        let mut source = SiteStore::new();
        let long = "가".repeat(300);
        add_site(&mut source, &long, "long.test", "deploy");
        add_site(&mut source, "서버 🚀 emoji", "emoji.test", "deploy");
        let document = build(&source, "").expect("내보내기").document;

        let mut target = SiteStore::new();
        let plan = plan_import(&document, &target, "").expect("계획");
        apply_import(&mut target, &plan, true);
        let names: Vec<&str> = target.sites().iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&long.as_str()));
        assert!(names.contains(&"서버 🚀 emoji"));
    }

    #[test]
    fn 파일로_쓰고_읽어도_같다() {
        let mut source = SiteStore::new();
        let id = add_site(&mut source, "배포 서버", "deploy.test", "deploy");
        assert!(source.set_password(id, "비밀!1234"));
        let document = build(&source, "암호").expect("내보내기").document;

        let dir = std::env::temp_dir().join("moa-site-export-test");
        std::fs::create_dir_all(&dir).expect("임시 폴더");
        let path = dir.join("사이트.moasites");
        write_file(&path, &document).expect("쓰기");
        let read = read_file(&path).expect("읽기");
        assert_eq!(read, document);
        let _ = std::fs::remove_file(&path);

        // 없는 파일은 읽기 실패다
        assert!(matches!(
            read_file(&dir.join("없는파일.moasites")),
            Err(ImportError::Io(_))
        ));
    }
}
