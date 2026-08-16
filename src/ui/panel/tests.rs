//! 패널 테스트 — 탐색·탭·원격·목록 조작을 한자리에서 검증한다.
//!
//! 본체(`ui::panel`)의 자식 모듈이라 그 파일의 비공개 항목에 그대로 닿는다.

use super::*;
use crate::remote::sites::SiteStore;
use crate::remote::types::{RemotePath, SiteId};

/// 원격 항목 하나 — 여러 테스트가 함께 쓴다
fn remote_entry(name: &str, is_dir: bool) -> RemoteEntry {
    RemoteEntry {
        name: name.to_owned(),
        is_dir,
        is_symlink: false,
        link_target: None,
        size: 0,
        modified: None,
        mode: None,
        owner: None,
    }
}

/// 한 프레임에 그려진 글자를 전부 모은다 — 화면에 실제로 무엇이 보이는지 판정한다
fn drawn_texts(output: &eframe::egui::FullOutput) -> Vec<String> {
    fn collect(shape: &egui::Shape, found: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => found.push(text.galley.text().to_owned()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, found);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut found);
    }
    found
}

/// 한 프레임에 그려진 글과 그 왼쪽 위 자리 — 배치를 견주는 시험이 쓴다
fn drawn_text_positions(output: &eframe::egui::FullOutput) -> Vec<(String, egui::Pos2)> {
    fn collect(shape: &egui::Shape, found: &mut Vec<(String, egui::Pos2)>) {
        match shape {
            egui::Shape::Text(text) => found.push((text.galley.text().to_owned(), text.pos)),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    collect(shape, found);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut found);
    }
    found
}

/// 한 프레임에 그려진 가로 구분선 수 — 구분선은 글이 아니라 얇은 사각형이라 글 목록에 없다.
/// 절대 개수가 아니라 **늘고 주는 것**을 보는 데 쓴다(패널에는 원래 상태 줄 아래 구분선이 있다)
fn separator_count(output: &eframe::egui::FullOutput) -> usize {
    fn count(shape: &egui::Shape, found: &mut usize) {
        match shape {
            // egui `separator()`는 얇은 사각형으로도, 선분으로도 그려진다 — 둘 다 센다
            egui::Shape::Rect(rect) => {
                let size = rect.rect.size();
                if size.y <= 2.0 && size.x > size.y * 4.0 {
                    *found += 1;
                }
            }
            egui::Shape::LineSegment { points, .. } => {
                let (a, b) = (points[0], points[1]);
                if (a.y - b.y).abs() <= 1.0 && (a.x - b.x).abs() > 4.0 {
                    *found += 1;
                }
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    count(shape, found);
                }
            }
            _ => {}
        }
    }
    let mut found = 0;
    for clipped in &output.shapes {
        count(&clipped.shape, &mut found);
    }
    found
}

/// 한 프레임에 그려진 자리표시 막대 수 — 목록 자리에 자리표시가 섰는지 판정한다.
/// 막대는 글자가 아니라 사각형이라 `drawn_texts`로는 보이지 않는다
fn skeleton_bars(output: &eframe::egui::FullOutput) -> usize {
    fn count(shape: &egui::Shape, found: &mut usize) {
        match shape {
            egui::Shape::Rect(rect) if rect.fill == crate::ui::remote_states::SKELETON_FILL => {
                *found += 1;
            }
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    count(shape, found);
                }
            }
            _ => {}
        }
    }
    let mut found = 0;
    for clipped in &output.shapes {
        count(&clipped.shape, &mut found);
    }
    found
}

/// egui는 같은 ID가 한 프레임에 두 번 쓰이면 화면에 경고 텍스트를 그린다.
/// 그 텍스트를 그려진 글자에서 찾아 ID 충돌 여부를 판정한다
fn id_clash_warnings(output: &eframe::egui::FullOutput) -> Vec<String> {
    drawn_texts(output)
        .into_iter()
        .filter(|body| body.contains("use of"))
        .collect()
}

/// 패널을 한 프레임 그린다 — 사이트 목록은 호출부가 준다
fn draw_once(panel: &mut PanelState, sites: &SiteStore) -> eframe::egui::FullOutput {
    draw_once_with_favorites(panel, sites, &[])
}

/// 즐겨찾기를 든 채 한 프레임 그린다 — 트리 위쪽 구역을 보는 시험이 쓴다
fn draw_once_with_favorites(
    panel: &mut PanelState,
    sites: &SiteStore,
    favorites: &[std::path::PathBuf],
) -> eframe::egui::FullOutput {
    let tree = crate::remote::tree_cache::TreeCache::new();
    let remote = RemoteView {
        sites,
        connected: &[],
        tree: &tree,
    };
    let ctx = egui::Context::default();
    let mut icons = crate::fs::icons::IconCache::new();
    let mut textures = crate::ui::icon_tex::IconTextures::new();
    ctx.run_ui(Default::default(), |ui| {
        egui::CentralPanel::default().show(ui, |ui| {
            let ctx = ui.ctx().clone();
            panel.show(
                ui,
                &ctx,
                &mut icons,
                &mut textures,
                remote,
                PanelMenuState::for_panes(1, ViewMode::Details),
                // 전송 대상이 없는 상태 — 이 시험들은 탭 아이콘이 아니라 배치·상태를 본다
                crate::ui::tabs::TransferTargets::default(),
                favorites,
            );
        });
    })
}

/// 패널을 한 프레임 그리고 ID 충돌 경고를 모은다
fn draw_panel(tree_visible: bool) -> Vec<String> {
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
    panel.tree_visible = tree_visible;
    id_clash_warnings(&draw_once(&mut panel, &SiteStore::new()))
}

/// 사이트 하나를 등록하고 그 사이트의 원격 탭을 활성으로 둔 패널.
/// 단계별 화면(README §4·§5)이 실제 렌더 경로를 지나게 하는 준비다
fn remote_panel_in(phase: TabPhase) -> (PanelState, SiteStore) {
    let mut sites = SiteStore::new();
    let site = sites.add("배포 서버");
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        site,
        RemotePath::new("/var/www"),
    ));
    panel.attach_conn(ConnectionId(1));
    panel.set_phase_for(ConnectionId(1), &phase);
    (panel, sites)
}

/// 그 단계의 원격 패널을 한 프레임 그리고 화면 글자를 모은다
fn remote_screen_texts(phase: TabPhase) -> Vec<String> {
    let (mut panel, sites) = remote_panel_in(phase);
    drawn_texts(&draw_once(&mut panel, &sites))
}

/// 열거 결과가 도착한 상황을 만들어 `poll_load`를 실제로 지나게 한다.
/// **헬퍼만 직접 부르면 안 된다** — 호출부가 죽어 있어도 통과하기 때문이다(F-7 B1)
fn commit_dir(panel: &mut PanelState, dir: &str, icons: &mut IconCache) {
    panel.pending_dir = std::path::PathBuf::from(dir);
    panel.pending_nav = PendingNav::None;
    panel.apply_enumerated(EnumOutcome::Ok(Vec::new()), icons);
}

#[test]
fn 폴더를_옮기면_썸네일을_놓는다() {
    // 이 해제는 `ThumbnailCache`의 세대를 올리는 유일한 지점이기도 하다 —
    // 죽으면 떠난 폴더의 썸네일이 계속 남고(NFR-9), 늦게 도착한 결과도 못 거른다.
    // 커밋을 먼저 하고 비교하면 항상 같아져 이 경로가 통째로 죽는다(F-7 B1)
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    commit_dir(&mut panel, r"C:\Users", &mut icons);

    panel.thumbs.accept_for_test(
        std::path::PathBuf::from(r"C:\Users\사진.jpg"),
        Some(sample_thumb()),
    );
    assert_eq!(panel.thumbs.len(), 1, "사전 준비 실패");

    commit_dir(&mut panel, r"C:\Windows", &mut icons);
    assert_eq!(
        panel.thumbs.len(),
        0,
        "폴더를 옮겼는데 이전 폴더의 썸네일이 남았다"
    );
}

#[test]
fn 탭을_바꿔_폴더가_달라져도_썸네일을_놓는다() {
    // 탭 전환은 `tabs.switch`로 **활성 탭을 먼저 바꾼 뒤** 그 경로를 읽는다 —
    // 커밋 직전 경로와 비교하는 방식이면 이 경로만 빠져나간다(F-7 m1).
    // 그래서 판정을 캐시(`set_folder`)로 옮겼고, 이 테스트가 그것을 지킨다
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    commit_dir(&mut panel, r"C:\Users", &mut icons);
    panel.thumbs.accept_for_test(
        std::path::PathBuf::from(r"C:\Users\사진.jpg"),
        Some(sample_thumb()),
    );

    // 다른 폴더를 보는 탭을 더한다 — `add`가 그 탭을 곧바로 활성으로 만든다
    panel
        .tabs
        .add(crate::panel::tabs::TabState::new(std::path::PathBuf::from(
            r"C:\Windows",
        )));
    commit_dir(&mut panel, r"C:\Windows", &mut icons);
    assert_eq!(
        panel.thumbs.len(),
        0,
        "탭을 바꿔 폴더가 달라졌는데 이전 폴더의 썸네일이 남았다"
    );

    // 되돌아가는 전환도 같아야 한다 — 새 폴더 썸네일을 담아 두고 원래 탭으로 돌아간다
    panel.thumbs.accept_for_test(
        std::path::PathBuf::from(r"C:\Windows\그림.png"),
        Some(sample_thumb()),
    );
    assert!(panel.tabs.switch(0), "첫 탭으로 되돌아가지 못했다");
    commit_dir(&mut panel, r"C:\Users", &mut icons);
    assert_eq!(panel.thumbs.len(), 0, "되돌아가는 전환에서 남았다");
}

#[test]
fn 같은_폴더를_다시_읽으면_썸네일을_지킨다() {
    // 감시 갱신(FR-10)은 같은 폴더를 다시 읽는다 — 그때마다 버리면
    // 다른 앱이 파일 하나만 만들어도 폴더 전체를 다시 만들게 된다
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    commit_dir(&mut panel, r"C:\Users", &mut icons);
    panel.thumbs.accept_for_test(
        std::path::PathBuf::from(r"C:\Users\사진.jpg"),
        Some(sample_thumb()),
    );

    commit_dir(&mut panel, r"C:\Users", &mut icons); // 감시 갱신
    assert_eq!(panel.thumbs.len(), 1, "같은 폴더인데 썸네일을 버렸다");
}

fn sample_thumb() -> crate::fs::thumbnail::ThumbnailImage {
    crate::fs::thumbnail::ThumbnailImage {
        width: 2,
        height: 2,
        rgba: vec![255; 16],
    }
}

#[test]
fn 썸네일을_올린_프레임은_곧바로_다시_그리라고_알린다() {
    // egui는 입력이 없으면 프레임을 돌리지 않는다 — 이 신호가 빠지면 워커가 늦게 준
    // 썸네일이 사용자가 마우스를 움직일 때까지 형식 아이콘에 머문다 (F-8에서 실제로 그랬다)
    let ctx = egui::Context::default();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    panel.thumbs.accept_for_test(
        std::path::PathBuf::from(r"C:\Users\사진.jpg"),
        Some(sample_thumb()),
    );

    assert_eq!(
        panel.poll_thumbnails(&ctx),
        Some(Duration::ZERO),
        "썸네일을 올린 프레임인데 곧바로 다시 그리라고 알리지 않았다"
    );
    assert_eq!(panel.thumb_textures.len(), 1, "텍스처가 올라가지 않았다");
    // 올릴 것도 기다릴 것도 없으면 알리지 않는다 — 늘 알리면 앱이 쉬지 않고 그린다
    assert_eq!(
        panel.poll_thumbnails(&ctx),
        None,
        "할 일이 없는데도 다시 그리라고 알렸다"
    );
}

#[test]
fn 썸네일을_기다리는_동안은_스스로_깨어난다() {
    // 썸네일 워커는 `fs` 계층이라 egui를 모른다 — 결과가 채널에 들어와도 앱은 알 수 없다.
    // 이 신호가 없으면 사진이 사용자가 마우스를 움직일 때까지 안 나타난다(F-8 실측)
    let ctx = egui::Context::default();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    panel
        .thumbs
        .request(std::path::Path::new(r"C:\Users\아직없음.jpg"));

    assert_eq!(
        panel.poll_thumbnails(&ctx),
        Some(THUMB_POLL_INTERVAL),
        "결과를 기다리는데 다시 깨어날 시점을 알리지 않았다"
    );
}

#[test]
fn 보기_모드는_패널을_거쳐_목록까지_전달된다() {
    // `Command::SetViewMode`가 닿는 지점이다 — 여기서 끊기면 메뉴에서 골라도
    // 목록은 이전 모드로 그려진다 (FR-23)
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
    assert_eq!(panel.view_mode(), ViewMode::Details);
    panel.set_view_mode(ViewMode::SmallIcons);
    assert_eq!(panel.view_mode(), ViewMode::SmallIcons);
}

#[test]
fn 보기_모드는_패널마다_독립이다() {
    // 한 패널에서 바꾼 모드가 다른 패널에 번지면 "패널마다 독립"(FR-23)이 깨진다
    let mut first = PanelState::new(std::path::PathBuf::from(r"C:\"));
    let second = PanelState::new(std::path::PathBuf::from(r"D:\"));
    first.set_view_mode(ViewMode::Tiles);
    assert_eq!(first.view_mode(), ViewMode::Tiles);
    assert_eq!(
        second.view_mode(),
        ViewMode::Details,
        "다른 패널까지 바뀌었다"
    );
}

#[test]
fn 패널_안에서_같은_위젯_id가_두_번_쓰이지_않는다() {
    // 탭 스트립·폴더 트리·파일 목록이 각자 스크롤 영역을 갖는데, 이들이 같은 id를 쓰면
    // 스크롤 위치가 서로 섞인다(화면에는 빨간 경고로 드러난다)
    assert!(
        draw_panel(false).is_empty(),
        "위젯 ID 충돌(트리 숨김): {:?}",
        draw_panel(false)
    );
    assert!(
        draw_panel(true).is_empty(),
        "위젯 ID 충돌(트리 표시): {:?}",
        draw_panel(true)
    );
}

#[test]
fn 원격_목록의_첫_줄은_언제나_상위_이동이다() {
    // 서버가 `..`를 주기도 하고 안 주기도 한다 — 화면은 어느 쪽이든 같아야 한다 (plan T9 ③)
    fn entry(name: &str, is_dir: bool) -> RemoteEntry {
        RemoteEntry {
            name: name.to_owned(),
            is_dir,
            is_symlink: false,
            link_target: None,
            size: 0,
            modified: None,
            mode: None,
            owner: None,
        }
    }

    let 없는_경우 = with_parent_first(vec![entry("public_html", true), entry("a.txt", false)]);
    let names: Vec<&str> = 없는_경우.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["..", "public_html", "a.txt"]);

    let 있는_경우 = with_parent_first(vec![
        entry("..", true),
        entry("public_html", true),
        entry("..", true),
    ]);
    let names: Vec<&str> = 있는_경우.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["..", "public_html"], "`..`가 둘이 되면 안 된다");

    // 빈 폴더에도 상위 이동은 남는다
    assert_eq!(with_parent_first(Vec::new()).len(), 1);
}

/// 로컬 항목 하나 — 이름은 널 종단 UTF-16이라는 불변식을 지켜 만든다
fn local_entry(name: &str, is_dir: bool) -> FileEntry {
    FileEntry {
        name: name.encode_utf16().chain(std::iter::once(0)).collect(),
        is_dir,
        size: 0,
        modified: 0,
        attributes: 0,
    }
}

#[test]
fn 로컬_목록에도_상위_이동_줄이_붙는다() {
    // 사용자 보고(2026-08-13): 원격 목록에는 `..`가 있는데 로컬 목록에는 없었다
    let 보통_폴더 = with_local_parent_first(
        Path::new(r"C:\Program Files"),
        vec![local_entry("Android", true), local_entry("a.txt", false)],
    );
    let names: Vec<String> = 보통_폴더.iter().map(|e| e.name_string()).collect();
    assert_eq!(names, vec!["..", "Android", "a.txt"]);

    // 드라이브 루트에는 올라갈 곳이 없다 — 눌러도 아무 일 없는 줄을 두지 않는다
    let 루트 = with_local_parent_first(Path::new(r"C:\"), vec![local_entry("Windows", true)]);
    let names: Vec<String> = 루트.iter().map(|e| e.name_string()).collect();
    assert_eq!(names, vec!["Windows"]);

    // 열거가 `..`를 함께 주더라도 둘이 되지 않는다
    let 중복 = with_local_parent_first(
        Path::new(r"C:\Users"),
        vec![local_entry("..", true), local_entry("Public", true)],
    );
    let names: Vec<String> = 중복.iter().map(|e| e.name_string()).collect();
    assert_eq!(names, vec!["..", "Public"]);
}

#[test]
fn 로컬_상위_이동을_더블클릭하면_위_폴더로_간다() {
    let ctx = egui::Context::default();
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    // 열거 결과가 도착한 것처럼 커밋시킨다 — 목록을 채우는 실제 경로를 그대로 지난다
    panel.start_load(
        std::path::PathBuf::from(r"C:\Users\Public"),
        PendingNav::Push,
        &ctx,
    );
    panel.apply_enumerated(
        EnumOutcome::Ok(vec![local_entry("Documents", true)]),
        &mut icons,
    );

    let names: Vec<String> = match panel.list.model() {
        crate::ui::file_list::ListModel::Local(rows) => {
            rows.iter().map(|e| e.name_string()).collect()
        }
        crate::ui::file_list::ListModel::Remote(_) => Vec::new(),
    };
    assert_eq!(names, vec!["..", "Documents"], "첫 줄이 상위 이동이 아니다");
    assert_eq!(
        panel.list.counts(),
        (1, 0),
        "상위 이동 줄을 폴더로 세면 개수가 실제와 달라진다"
    );

    // 첫 줄을 더블클릭하면 위 폴더를 읽으러 간다 — `C:\Users\Public\..`이 아니다
    panel.handle_list_action(FileListAction::Open(0), &ctx);
    assert_eq!(panel.pending_dir, std::path::PathBuf::from(r"C:\Users"));
}

#[test]
fn 원격_탭에서는_로컬_전용_작업이_일어나지_않는다() {
    // 열거·감시·썸네일·새 파일은 로컬에만 있는 일이다 (plan T9 ②)
    let ctx = egui::Context::default();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new("/pub"),
    ));
    assert!(panel.is_remote(), "원격 탭이 활성이어야 한다");

    // 새 폴더·새 파일은 아무 일도 하지 않는다
    panel.new_folder(&ctx);
    panel.new_file(&ctx);
    assert!(
        !panel.create.is_running(),
        "원격 탭에서 로컬 생성이 시작됐다"
    );
    // 연결이 없는 원격 탭에서는 목록 요청도 나가지 않는다
    let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
    assert_eq!(panel.request_remote_list(&manager), None);
}

#[test]
fn 한_패널에_로컬_탭과_원격_탭을_섞을_수_있다() {
    // 탭마다 자기 소스로 그려져야 한다 (plan T9 ⑤)
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(3),
        RemotePath::new("/var/www"),
    ));

    let sources = panel.tabs.sources();
    assert_eq!(sources.len(), 2);
    assert!(!sources[0].is_remote(), "첫 탭은 로컬이어야 한다");
    assert!(sources[1].is_remote(), "둘째 탭은 원격이어야 한다");
    assert_eq!(sources[1].site(), Some(SiteId(3)));
    assert_eq!(
        sources[1].remote_path().map(|p| p.as_str()),
        Some("/var/www")
    );

    // 로컬 탭으로 돌아오면 다시 로컬 전용 일이 열린다
    assert!(panel.tabs.switch(0));
    assert!(!panel.is_remote());
    assert_eq!(panel.dir(), std::path::Path::new(r"C:\테스트"));
}

/// 원격 탭 하나를 더해 활성으로 만든 패널
fn panel_with_remote_tab(path: &str) -> PanelState {
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new(path),
    ));
    assert!(panel.is_remote(), "원격 탭이 활성이어야 한다");
    panel
}

#[test]
fn 원격_탭에서_새로_고침은_로컬_열거를_걸지_않는다() {
    let ctx = egui::Context::default();
    let mut panel = panel_with_remote_tab("/var/www");
    panel.refresh(&ctx);
    assert!(
        !panel.load.is_loading(),
        "원격 탭에서 로컬 열거 워커가 떴다"
    );
}

#[test]
fn 원격_탭의_상위_이동은_원격_경로로_가고_루트에서_머문다() {
    // plan T9 Edge Case — 루트를 넘어가지 않는다
    let ctx = egui::Context::default();
    let mut panel = panel_with_remote_tab("/var/www");

    panel.handle_nav(NavAction::Up, &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var")
    );
    panel.handle_nav(NavAction::Up, &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/")
    );
    // 루트에서 한 번 더 눌러도 그대로다
    panel.handle_nav(NavAction::Up, &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/")
    );
    // 로컬 열거 워커도 뜨지 않았다
    assert!(!panel.load.is_loading());
}

#[test]
fn 원격_탭에서는_셸_메뉴를_요청하지_않는다() {
    // 셸은 로컬 PIDL만 다룬다 (D21)
    let ctx = egui::Context::default();
    let mut panel = panel_with_remote_tab("/var/www");
    let request = panel.handle_list_action(
        FileListAction::Context {
            index: None,
            pos: egui::pos2(0.0, 0.0),
        },
        &ctx,
    );
    assert!(request.is_none(), "원격 탭에서 셸 메뉴가 요청됐다");

    // 항목 열기도 로컬 경로를 만들지 않는다
    let opened = panel.handle_list_action(FileListAction::Open(0), &ctx);
    assert!(opened.is_none());
    assert!(!panel.load.is_loading());
}

#[test]
fn 원격_탭을_보는_동안_로컬_감시_통지는_무시된다() {
    // 이전 폴더의 감시가 아직 살아 있어도 원격 화면이 로컬 목록으로 덮이면 안 된다
    let ctx = egui::Context::default();
    let mut panel = panel_with_remote_tab("/var/www");
    let (tx, rx) = std::sync::mpsc::channel();
    panel.watch = Some(DirWatch {
        watcher: crate::fs::watcher::DirWatcher::start(
            std::path::PathBuf::from(r"C:\테스트"),
            tx,
            None,
        ),
        rx,
    });

    panel.poll_watch(&ctx);
    assert!(
        !panel.load.is_loading(),
        "감시 통지로 로컬 열거 워커가 떴다"
    );
}

#[test]
fn 원격_탭에는_사이트_이름과_단계_배지가_함께_보인다() {
    // 인벤토리 #11~13 — 이름은 사이트 설정에서, 배지 문구는 단계에서 온다 (Acceptance ①).
    // 탭이 이름 사본을 들면 `이름 바꾸기(R)` 뒤에 탭만 옛 이름으로 남는다
    let 빈_탭 = remote_screen_texts(TabPhase::New);
    assert!(
        빈_탭.iter().any(|t| t == "배포 서버"),
        "사이트 이름이 탭에 없다: {빈_탭:?}"
    );
    assert!(
        빈_탭.iter().any(|t| t == "연결 없음"),
        "미연결 배지가 없다: {빈_탭:?}"
    );
    assert!(
        remote_screen_texts(TabPhase::Connecting)
            .iter()
            .any(|t| t == "연결 중…"),
        "연결 중 배지가 없다"
    );
    // 연결되면 배지가 프로토콜 이름으로 바뀐다 (새 사이트의 기본값은 FTP다)
    assert!(
        remote_screen_texts(TabPhase::Ok).iter().any(|t| t == "ftp"),
        "연결됨 배지가 프로토콜을 보이지 않는다"
    );
}

#[test]
fn 사이트를_아는_미연결_탭은_안내_대신_다시_연결을_보인다() {
    // 사용자 보고(2026-08-13): 재시작하면 원격 탭이 사이트·경로를 되찾고도 "주소창에
    // sftp://호스트 를 입력해 연결하세요"를 보였다 — 이미 아는 것을 다시 묻는 화면이다
    let 화면 = remote_screen_texts(TabPhase::New);
    assert!(
        화면.iter().any(|t| t == "다시 연결"),
        "다시 연결 버튼이 없다: {화면:?}"
    );
    for 사라져야_할_문구 in ["sftp://호스트", "끌어다 놓아도 됩니다"] {
        assert!(
            !화면.iter().any(|t| t.contains(사라져야_할_문구)),
            "'{사라져야_할_문구}'가 남아 있다: {화면:?}"
        );
    }
    // 붙을 사이트를 화면 밖(앱)이 알아낼 수 있어야 버튼이 일을 한다
    let (panel, _) = remote_panel_in(TabPhase::New);
    assert!(panel.active_site().is_some(), "탭이 사이트를 잃었다");
}

#[test]
fn 사이트를_찾을_수_없는_탭에는_주소_안내가_남는다() {
    // 사이트를 지운 뒤 남은 탭은 붙을 곳을 모른다 — 그 탭에는 다시 알려 주어야 한다
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(999),
        RemotePath::new("/var/www"),
    ));
    let 화면 = drawn_texts(&draw_once(&mut panel, &SiteStore::new()));
    assert!(
        화면.iter().any(|t| t.contains("sftp://호스트")),
        "주소 안내가 사라졌다: {화면:?}"
    );
    assert!(
        !화면.iter().any(|t| t == "다시 연결"),
        "붙을 곳을 모르는데 다시 연결을 보인다: {화면:?}"
    );
}

#[test]
fn 단계마다_본문이_통째로_달라진다() {
    // README §4·§5 — 연결 전·중·실패에 목록 대신 그 단계의 화면이 보인다 (Acceptance ①③④)
    // 사이트를 아는 미연결 탭에는 `다시 연결` 버튼이 선다 (아래 두 테스트가 자세히 본다)
    let 미연결 = remote_screen_texts(TabPhase::New);
    assert!(
        미연결.iter().any(|t| t == "다시 연결"),
        "미연결 화면에 다시 연결이 없다: {미연결:?}"
    );

    let 연결_중 = remote_screen_texts(TabPhase::Connecting);
    assert!(
        연결_중.iter().any(|t| t == "취소"),
        "연결 중 취소 버튼이 없다: {연결_중:?}"
    );

    let 실패 = remote_screen_texts(TabPhase::Error {
        message: "530 Login incorrect".to_owned(),
        kind: crate::remote::types::FailureKind::Auth,
    });
    for 문구 in [
        "연결하지 못했습니다",
        "재시도",
        "설정 열기",
        "서버 로그 보기",
    ] {
        assert!(
            실패.iter().any(|t| t.contains(문구)),
            "실패 화면에 '{문구}'가 없다: {실패:?}"
        );
    }
    assert!(
        실패.iter().any(|t| t.contains("530 Login incorrect")),
        "서버가 준 사유가 보이지 않는다: {실패:?}"
    );
}

#[test]
fn 연결되지_않은_원격_패널은_항목_수를_모른다고_보인다() {
    // 인벤토리 #95 — `폴더 0 파일 0`으로 보이면 "빈 폴더"라는 없는 말을 하게 된다
    for phase in [
        TabPhase::New,
        TabPhase::Connecting,
        TabPhase::Error {
            message: "530".to_owned(),
            kind: crate::remote::types::FailureKind::Auth,
        },
    ] {
        let texts = remote_screen_texts(phase.clone());
        assert!(
            texts
                .iter()
                .any(|t| t == crate::ui::remote_states::UNKNOWN_COUNT),
            "{phase:?}에서 `—`가 보이지 않는다: {texts:?}"
        );
        assert!(
            !texts.iter().any(|t| is_item_count(t)),
            "{phase:?}인데 항목 수를 세어 보였다: {texts:?}"
        );
    }
    // 연결되면 보통의 항목 수로 돌아온다
    let 연결됨 = remote_screen_texts(TabPhase::Ok);
    assert!(
        연결됨.iter().any(|t| is_item_count(t)),
        "연결됐는데 항목 수가 없다: {연결됨:?}"
    );
}

/// 상태 줄의 항목 수 표시인가 — 트리 토글(`폴더 트리`)과 구분한다
fn is_item_count(text: &str) -> bool {
    text.starts_with("폴더 ") && text.contains("파일 ")
}

#[test]
fn 원격_탭_화면에서도_위젯_id가_겹치지_않는다() {
    // Acceptance ⑧ — 단계별 화면이 목록 자리에 들어와도 id 공간이 섞이면 안 된다
    for phase in [
        TabPhase::New,
        TabPhase::Connecting,
        TabPhase::Error {
            message: "530".to_owned(),
            kind: crate::remote::types::FailureKind::Auth,
        },
        TabPhase::Ok,
    ] {
        let (mut panel, sites) = remote_panel_in(phase.clone());
        let clashes = id_clash_warnings(&draw_once(&mut panel, &sites));
        assert!(
            clashes.is_empty(),
            "{phase:?}에서 위젯 ID 충돌: {clashes:?}"
        );
    }
}

#[test]
fn 패널의_마지막_원격_탭을_닫으면_연결을_접는다() {
    // FR-32 — 같은 연결을 쓰는 탭이 남아 있으면 접지 않는다 (Acceptance ⑥)
    let ctx = egui::Context::default();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new("/a"),
    ));
    panel.attach_conn(ConnectionId(5));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new("/b"),
    ));
    panel.attach_conn(ConnectionId(5));

    assert_eq!(
        panel.handle_tab(TabAction::Close(2), &ctx),
        None,
        "같은 연결을 쓰는 탭이 남았는데 연결을 접으려 했다"
    );
    assert_eq!(
        panel.handle_tab(TabAction::Close(1), &ctx),
        Some(ConnectionId(5)),
        "마지막 원격 탭을 닫았는데 연결이 남았다"
    );
    // 로컬 탭만 남았으니 더 접을 것이 없다
    assert!(!panel.is_remote());
}

#[test]
fn 연결_단계는_그_연결을_쓰는_모든_탭에_퍼진다() {
    // 배경 탭이 옛 단계로 남으면 그 탭으로 돌아갔을 때 화면이 실제와 어긋난다
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\테스트"));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new("/a"),
    ));
    panel.attach_conn(ConnectionId(5));
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new("/b"),
    ));
    panel.attach_conn(ConnectionId(7)); // 다른 연결을 쓰는 탭

    assert!(panel.set_phase_for(ConnectionId(5), &TabPhase::Ok));
    let phases: Vec<TabPhase> = panel
        .tabs
        .sources()
        .iter()
        .filter_map(|source| match source {
            TabSource::Remote { phase, .. } => Some(phase.clone()),
            TabSource::Local(_) => None,
        })
        .collect();
    assert_eq!(
        phases,
        vec![TabPhase::Ok, TabPhase::Connecting],
        "다른 연결의 탭까지 바뀌었거나, 대상 탭이 바뀌지 않았다"
    );
    // 없는 연결에는 아무 일도 일어나지 않는다
    assert!(!panel.set_phase_for(ConnectionId(99), &TabPhase::Ok));
}

#[test]
fn 남의_답이나_지난_위치의_목록은_받지_않는다() {
    // 세대만 보면 한 연결을 두 패널이 나눠 쓸 때 남의 답을 제 목록으로 삼는다
    let mut icons = IconCache::new();
    let mut panel = panel_with_remote_tab("/var/www");
    panel.attach_conn(ConnectionId(1));
    let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
    // 연결이 죽어 있어도 세대는 올라간다 — 여기서는 세대·위치 판정만 본다
    panel.request_remote_list(&manager);
    let generation = panel.remote_generation;

    assert!(!panel.awaits_remote_list(generation + 1, &RemotePath::new("/var/www")));
    assert!(!panel.awaits_remote_list(generation, &RemotePath::new("/etc")));
    assert!(panel.awaits_remote_list(generation, &RemotePath::new("/var/www")));
    assert!(!panel.apply_remote_listed(
        generation,
        &RemotePath::new("/etc"),
        Vec::new(),
        &mut icons
    ));
    assert!(panel.apply_remote_listed(
        generation,
        &RemotePath::new("/var/www"),
        Vec::new(),
        &mut icons
    ));
}

#[test]
fn 만드는_중_원격_탭으로_옮겨_가면_로컬_열거를_걸지_않는다() {
    // 새 폴더를 만드는 워커가 도는 사이 원격 탭으로 옮겨 가면, 완료 시점의 활성 탭은
    // 원격이다 — 그때 활성 탭 기준으로 다시 읽으면 빈 경로를 열거하게 된다
    use std::time::{Duration, Instant};

    let ctx = egui::Context::default();
    let dir = std::env::temp_dir().join(format!("fe_t9_생성_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);

    let mut panel = PanelState::new(dir.clone());
    panel.new_folder(&ctx);
    assert!(panel.create.is_running(), "생성이 시작되지 않았다");

    // 만드는 사이 원격 탭으로 옮겨 간다
    panel.tabs.add(crate::panel::tabs::TabState::remote(
        SiteId(1),
        RemotePath::new("/var/www"),
    ));
    assert!(panel.is_remote());

    let deadline = Instant::now() + Duration::from_secs(3);
    while panel.create.is_running() && Instant::now() < deadline {
        panel.poll_create(&ctx);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!panel.create.is_running(), "생성이 끝나지 않았다");
    // **`load.pending`으로 본다** — `pending_dir`은 원격 탭에서 어차피 빈 경로라
    // 가드 유무를 가리지 못한다. 열거 워커가 떴는지가 유일하게 둘을 가르는 신호다
    assert!(
        !panel.load.is_loading(),
        "원격 탭인데 로컬 열거 워커가 떴다"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn 원격_목록의_우클릭은_셸_메뉴를_띄우지_않는다() {
    // Acceptance ⑤ — 셸 메뉴는 로컬 경로가 있어야 뜬다(D21). 원격 탭에서는 자체 메뉴다
    let mut panel = PanelState::new(PathBuf::from(r"C:\"));
    let pos = egui::pos2(120.0, 80.0);

    let ctx = egui::Context::default();
    let request = panel.handle_list_action(
        FileListAction::Context {
            index: Some(0),
            pos,
        },
        &ctx,
    );
    assert!(request.is_some(), "로컬 탭에서는 셸 메뉴를 청해야 한다");
    assert!(panel.remote_menu_at.is_none(), "로컬 탭에 원격 메뉴가 떴다");

    panel.open_remote_tab(SiteId(1), RemotePath::new("/var/www"));
    let request = panel.handle_list_action(
        FileListAction::Context {
            index: Some(0),
            pos,
        },
        &ctx,
    );
    assert!(request.is_none(), "원격 탭에서 셸 메뉴를 청했다");
    assert_eq!(panel.remote_menu_at, Some(pos), "원격 메뉴가 뜨지 않았다");
}

#[test]
fn 갓_나뉜_패널은_원격_탭_하나만_갖는다() {
    // 사용자 보고 — 연결을 열면 시작 폴더 탭이 함께 남아 탭이 둘이었다
    let mut panel = PanelState::new(PathBuf::from(r"C:\"));
    assert_eq!(panel.tabs.len(), 1, "새 패널은 탭 하나로 시작한다");

    panel.open_remote_tab_only(SiteId(1), RemotePath::new("/var/www"));
    assert_eq!(panel.tabs.len(), 1, "원격 탭만 남아야 한다");
    assert!(
        matches!(panel.tabs.active().source, TabSource::Remote { .. }),
        "남은 탭이 원격이 아니다"
    );

    // 쓰던 패널에 여는 길(`open_remote_tab`)은 그대로 더한다 — 그 탭들은 사용자가 열었다
    panel.open_remote_tab(SiteId(2), RemotePath::root());
    assert_eq!(panel.tabs.len(), 2, "기존 탭을 지우면 안 된다");
}

#[test]
fn 마지막_탭을_닫으면_패널_닫기를_청한다() {
    // 사용자 보고 — 원격 탭이 홀로 있는 패널에서 ✕가 아무 반응도 하지 않았다
    let ctx = egui::Context::default();
    let mut alone = PanelState::new(PathBuf::from(r"C:\"));
    alone.open_remote_tab_only(SiteId(1), RemotePath::root());
    alone.handle_tab(TabAction::Close(0), &ctx);
    assert_eq!(alone.tabs.len(), 1, "마지막 탭 자체는 남는다");
    assert!(alone.close_requested, "패널 닫기를 청하지 않았다");

    // 탭이 둘이면 탭만 닫힌다 — 패널은 그대로 둔다
    let mut pair = PanelState::new(PathBuf::from(r"C:\"));
    pair.open_remote_tab(SiteId(1), RemotePath::root());
    pair.handle_tab(TabAction::Close(1), &ctx);
    assert_eq!(pair.tabs.len(), 1);
    assert!(!pair.close_requested, "탭이 남았는데 패널을 닫으려 했다");
}

#[test]
fn 패널이_쓰는_연결은_중복_없이_모인다() {
    // 패널을 닫을 때 회수 대상을 고르는 근거다 (FR-32)
    let mut panel = PanelState::new(PathBuf::from(r"C:\"));
    assert!(panel.conns().is_empty(), "로컬 탭만 있는데 연결이 잡혔다");

    panel.open_remote_tab(SiteId(1), RemotePath::new("/a"));
    panel.attach_conn(ConnectionId(7));
    panel.open_remote_tab(SiteId(1), RemotePath::new("/b"));
    panel.attach_conn(ConnectionId(7));
    assert_eq!(
        panel.conns(),
        vec![ConnectionId(7)],
        "같은 연결이 두 번 담겼다"
    );
}

#[test]
fn 가장자리에서_연_메뉴는_화면_안으로_당겨진다() {
    // quality 리뷰 m1 — 셸 메뉴는 OS가 보정해 주지만(D21) 우리가 그리는 메뉴는 아니다
    let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1200.0, 800.0));
    let size = egui::vec2(200.0, 240.0);
    // 안쪽에서 열면 그 자리 그대로다
    assert_eq!(
        clamp_menu_pos(screen, egui::pos2(100.0, 100.0), size),
        egui::pos2(100.0, 100.0)
    );
    // 오른쪽·아래 가장자리에서 열면 안으로 당긴다
    assert_eq!(
        clamp_menu_pos(screen, egui::pos2(1150.0, 780.0), size),
        egui::pos2(1000.0, 560.0)
    );
    // 화면보다 큰 메뉴는 왼쪽 위를 맞춘다 — 아래가 잘려도 첫 줄은 보인다
    let huge = egui::vec2(2000.0, 2000.0);
    assert_eq!(
        clamp_menu_pos(screen, egui::pos2(600.0, 400.0), huge),
        egui::pos2(0.0, 0.0)
    );
}

#[test]
fn 권한이_없으면_그_폴더로_옮기고_목록_자리에_사유를_적는다() {
    // 2026-08-16 사용자 요청 — 종전에는 이전 목록을 그대로 둔 채 상태 줄에만 사유를 적어,
    // 주소창·트리가 가리키는 곳과 목록이 갈렸다
    let _guard = crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    // 첫 프레임의 시작 열거를 걸지 않는다 — 그 결과가 오면 이 시험이 만든 상태를 덮는다
    panel.deferred_start = None;
    commit_dir(&mut panel, r"C:\Users", &mut icons);

    let denied = std::path::PathBuf::from(r"C:\Documents and Settings");
    panel.pending_dir = denied.clone();
    panel.pending_nav = PendingNav::Push;
    panel.apply_enumerated(EnumOutcome::AccessDenied, &mut icons);

    assert_eq!(
        panel.tabs.active().source.local_path(),
        Some(denied.as_path()),
        "권한이 막힌 폴더로 옮기지 않았다"
    );
    assert!(
        panel.status.is_empty(),
        "상태 줄에 사유가 남았다: {}",
        panel.status
    );
    assert_eq!(panel.list.counts(), (0, 0), "이전 목록이 남았다");
    assert!(panel.shows_denied(), "사유를 적을 상태가 아니다");
    assert!(panel.watch.is_none(), "읽지 못한 폴더를 감시하고 있다");

    // 그린 화면에도 그 말이 있다 — 판정 헬퍼만 보면 그리기가 죽어도 통과한다(F-7 B1)
    let texts = drawn_texts(&draw_once(&mut panel, &SiteStore::new()));
    assert!(
        texts
            .iter()
            .any(|text| text == "이 폴더를 열 권한이 없어 내용을 표시할 수 없습니다"),
        "목록 자리에 사유가 없다: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text.contains("권한이 없습니다")),
        "상태 줄 문구가 남아 있다: {texts:?}"
    );

    // 안내는 `..` 줄과 겹치지 않는다 — 겹치면 두 글이 포개져 둘 다 읽히지 않는다
    // (2026-08-16 사용자 보고)
    let placed = drawn_text_positions(&draw_once(&mut panel, &SiteStore::new()));
    let 안내 = placed
        .iter()
        .find(|(text, _)| text.starts_with("이 폴더를 열 권한이"))
        .expect("권한 안내")
        .1;
    let 첫줄 = placed
        .iter()
        .find(|(text, _)| text == "..")
        .expect("`..` 줄")
        .1;
    assert!(
        안내.y > 첫줄.y + crate::ui::list_details::ROW_HEIGHT,
        "안내가 `..` 줄과 겹친다 (안내 {}, 첫 줄 {})",
        안내.y,
        첫줄.y
    );

    // 읽어 낸 폴더로 옮기면 안내는 사라진다
    commit_dir(&mut panel, r"C:\Users", &mut icons);
    assert!(!panel.shows_denied(), "안내가 그대로 남았다");
}

#[test]
fn 즐겨찾기는_드라이브_뿌리보다_위에_구분선과_함께_선다() {
    // FR-56 — 트리 맨 위가 바로가기 자리다. 구분선이 그 아래를 가른다
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
    panel.tree_visible = true;
    panel.deferred_start = None;
    let favorites = [
        std::path::PathBuf::from(r"D:\작업"),
        std::path::PathBuf::from(r"C:\Users"),
    ];

    let output = draw_once_with_favorites(&mut panel, &SiteStore::new(), &favorites);
    let texts = drawn_text_positions(&output);

    let 작업 = texts
        .iter()
        .find(|(text, pos)| text == "작업" && pos.x < TREE_WIDTH)
        .expect("즐겨찾기 `작업` 줄")
        .1;
    let users = texts
        .iter()
        .find(|(text, pos)| text == "Users" && pos.x < TREE_WIDTH)
        .expect("즐겨찾기 `Users` 줄")
        .1;
    // 더한 차례 그대로다 (사용자 결정: 이름순이 아니다)
    assert!(작업.y < users.y, "추가한 차례가 뒤바뀌었다");

    // 드라이브 뿌리는 그 아래에 선다 — 주소창에도 `C:\` 같은 경로가 있어 **트리 구역만** 본다
    // (트리는 상태 줄 아래에서 시작한다)
    let 토글 = texts
        .iter()
        .find(|(text, _)| text == TREE_TOGGLE_ICON)
        .expect("트리 토글 아이콘")
        .1;
    let 드라이브 = texts
        .iter()
        .filter(|(text, pos)| pos.x < TREE_WIDTH && pos.y > 토글.y && text.ends_with(":\\"))
        .map(|(_, pos)| pos.y)
        .fold(f32::INFINITY, f32::min);
    assert!(
        드라이브.is_finite(),
        "드라이브 뿌리가 그려지지 않았다: {texts:?}"
    );
    assert!(
        users.y < 드라이브,
        "즐겨찾기가 드라이브 아래로 내려갔다 (즐겨찾기 {}, 드라이브 {드라이브})",
        users.y
    );
}

#[test]
fn 즐겨찾기가_없으면_구분선도_그리지_않는다() {
    // 사용자 결정 — 쓰지 않는 사람의 화면은 지금과 똑같아야 한다
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
    panel.tree_visible = true;
    panel.deferred_start = None;

    let 빈_즐겨찾기 = separator_count(&draw_once_with_favorites(
        &mut panel,
        &SiteStore::new(),
        &[],
    ));
    let 한_건 = separator_count(&draw_once_with_favorites(
        &mut panel,
        &SiteStore::new(),
        &[std::path::PathBuf::from(r"D:\작업")],
    ));

    assert_eq!(
        한_건,
        빈_즐겨찾기 + 1,
        "즐겨찾기 구분선이 하나 늘지 않았다 (빈 {빈_즐겨찾기}, 한 건 {한_건})"
    );
}

#[test]
fn 원격_트리에는_즐겨찾기가_서지_않는다() {
    // 사용자 명시 제외 — 원격 패널에서는 바로가기 자체가 뜻이 없다(로컬 경로다)
    let (mut panel, sites) = remote_panel_in(TabPhase::Ok);
    panel.tree_visible = true;
    let favorites = [std::path::PathBuf::from(r"D:\작업")];

    let texts = drawn_texts(&draw_once_with_favorites(&mut panel, &sites, &favorites));

    assert!(
        !texts.iter().any(|text| text == "작업"),
        "원격 트리에 즐겨찾기가 그려졌다: {texts:?}"
    );
}

#[test]
fn 즐겨찾기를_누르면_그_폴더로_옮겨간다() {
    // 바로가기의 본래 목적 — 누르면 활성 탭이 그리로 간다
    let ctx = egui::Context::default();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\"));
    panel.deferred_start = None;

    // 트리가 올린 선택을 패널이 어떻게 다루는지 본다 — 그리기는 위 시험들이 덮는다
    panel.navigate(std::path::PathBuf::from(r"D:\작업"), &ctx);

    assert_eq!(
        panel.pending_dir,
        std::path::PathBuf::from(r"D:\작업"),
        "고른 폴더로 열거를 걸지 않았다"
    );
    assert!(
        matches!(panel.pending_nav, PendingNav::Push),
        "히스토리에 쌓지 않았다"
    );
}

#[test]
fn 트리_토글은_트리_위쪽_왼쪽_끝에_선다() {
    // 2026-08-16 사용자 결정 — 토글이 목록 쪽(트리 오른쪽)에 있으면 무엇을 여는 버튼인지
    // 읽히지 않는다. 상태 줄이 패널 전폭을 쓰고 토글은 트리 폭 안에 선다
    let (mut panel, sites) = remote_panel_in(TabPhase::Ok);
    panel.tree_visible = true;
    let texts = drawn_text_positions(&draw_once(&mut panel, &sites));
    let 토글 = texts
        .iter()
        .find(|(text, _)| text == TREE_TOGGLE_ICON)
        .expect("트리 토글 아이콘")
        .1;
    assert!(
        토글.x < TREE_WIDTH,
        "토글이 트리 폭({TREE_WIDTH}) 밖에 있다: {}",
        토글.x
    );
}

#[test]
fn 트리는_파일_목록과_같은_높이에서_시작한다() {
    // 2026-08-16 사용자 보고 — 트리 첫 줄이 상태 줄 옆까지 올라가 목록보다 위에 떠 있었다.
    // 트리 폭 안(왼쪽)에 그려진 글과 목록 열 머리글의 높이를 견준다
    let _guard = crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
    let (mut panel, sites) = remote_panel_in(TabPhase::Ok);
    panel.tree_visible = true;
    let output = draw_once(&mut panel, &sites);
    let texts = drawn_text_positions(&output);

    let 토글 = texts
        .iter()
        .find(|(text, _)| text == TREE_TOGGLE_ICON)
        .expect("트리 토글 아이콘")
        .1;
    let 머리글 = texts
        .iter()
        .find(|(text, pos)| text.starts_with("이름") && pos.x > TREE_WIDTH)
        .expect("목록의 `이름` 열 머리글")
        .1;
    // 원격 트리의 뿌리 줄 — 이름이 없는 루트는 경로 그대로(`/`) 그려진다
    let 첫줄 = texts
        .iter()
        .find(|(text, pos)| text == "/" && pos.x < TREE_WIDTH)
        .expect("트리 뿌리 줄")
        .1
        .y;

    assert!(
        첫줄 > 토글.y,
        "트리가 상태 줄까지 올라와 있다 (트리 {첫줄}, 토글 {})",
        토글.y
    );
    // 열 머리글과 나란하다 — 위젯 안쪽 여백만큼의 차이는 남는다
    assert!(
        (첫줄 - 머리글.y).abs() < 12.0,
        "트리 첫 줄과 목록 머리글의 높이가 어긋난다 (트리 {첫줄}, 머리글 {})",
        머리글.y
    );
}

#[test]
fn 원격_패널의_트리_토글은_원격_트리다() {
    // Acceptance ① (인벤토리 #94) — 같은 자리의 **툴팁**이 소스에 따라 갈린다.
    // 문구는 카탈로그가 정하므로 한국어로 고정하고 원문과 견준다
    let _guard = crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
    let 로컬 = PanelState::new(std::path::PathBuf::from(r"C:\"));
    assert_eq!(로컬.tree_toggle_tooltip(), "폴더 트리");
    let (원격, _) = remote_panel_in(TabPhase::Ok);
    assert_eq!(원격.tree_toggle_tooltip(), "원격 트리");
}

#[test]
fn 트리_토글은_문구가_아니라_아이콘으로_그린다() {
    // 사용자 요청 — 상태 줄의 `폴더 트리`·`원격 트리` 문구를 아이콘 하나로 줄였다.
    // 툴팁은 hover해야 뜨므로 그려진 글에는 문구가 남지 않아야 한다
    let _guard = crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
    for 화면 in [
        drawn_texts(&draw_once(
            &mut PanelState::new(std::path::PathBuf::from(r"C:\")),
            &SiteStore::new(),
        )),
        remote_screen_texts(TabPhase::Ok),
    ] {
        assert!(
            화면.iter().any(|text| text == TREE_TOGGLE_ICON),
            "트리 토글 아이콘이 없다: {화면:?}"
        );
        assert!(
            !화면
                .iter()
                .any(|text| text == "폴더 트리" || text == "원격 트리"),
            "트리 토글 문구가 남아 있다: {화면:?}"
        );
    }
}

#[test]
fn 트리에서_고른_원격_폴더로_목록이_옮겨간다() {
    // Acceptance ⑤ — 옮기는 것으로 끝나면 화면은 옛 목록 그대로다(spec 리뷰 B1).
    // 옮긴 뒤 **깃발이 서고**, 그 깃발을 거둔 쪽이 실제로 조회를 보내야 한다
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);
    panel.take_remote_dirty();
    panel.navigate_remote(RemotePath::new("/var/www/html"));
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www/html")
    );
    assert!(panel.take_remote_dirty(), "다시 읽어 달라는 표시가 없다");
    assert!(!panel.take_remote_dirty(), "깃발이 한 번에 거둬지지 않았다");

    // 거둔 쪽이 그 위치로 조회를 보낸다 — 세대와 위치가 함께 맞아야 답을 받는다
    let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
    panel.request_remote_list(&manager);
    assert!(panel.awaits_remote_list(panel.remote_generation, &RemotePath::new("/var/www/html")));

    // 상위 이동도 같은 길을 쓴다 — 옮기고 나서 아무도 다시 읽지 않던 자리였다
    let ctx = egui::Context::default();
    panel.handle_nav(NavAction::Up, &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www")
    );
    assert!(panel.take_remote_dirty(), "상위 이동 뒤에 표시가 없다");
}

#[test]
fn 원격_트리의_뿌리는_최상단까지_거슬러_올라간다() {
    // plan Edge Case — 루트가 `/`가 아닌 서버도 있어 `/`로 못 박지 않는다
    let (panel, _) = remote_panel_in(TabPhase::Ok);
    let (conn, root) = panel.remote_tree_root().expect("연결된 원격 탭");
    assert_eq!(conn, ConnectionId(1));
    assert_eq!(root.as_str(), "/");
    // 로컬 탭에는 원격 트리가 없다
    let 로컬 = PanelState::new(std::path::PathBuf::from(r"C:\"));
    assert!(로컬.remote_tree_root().is_none());
}

#[test]
fn 성공한_이동은_되돌릴_자리를_남기지_않는다() {
    // F-7 2라운드 B1 — 자리가 남으면 **나중의 무관한 실패**(새로 고침·작업 후 재조회)가
    // 옛 폴더로 경로만 되돌린다
    let mut icons = IconCache::new();
    let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);
    panel.set_remote_path(RemotePath::new("/var/www/html"));
    panel.request_remote_list(&manager);
    let moved = panel.remote_generation;
    // 답이 도착해 이동이 섰다
    assert!(panel.apply_remote_listed(
        moved,
        &RemotePath::new("/var/www/html"),
        Vec::new(),
        &mut icons
    ));

    // 그 뒤의 새로 고침이 실패해도 경로는 그대로여야 한다
    panel.request_remote_list(&manager);
    let refreshed = panel.remote_generation;
    assert!(
        !panel.revert_remote_path(refreshed),
        "성공한 이동이 되돌려졌다"
    );
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www/html")
    );
}

#[test]
fn 다른_요청의_실패는_경로를_건드리지_않는다() {
    // F-7 2라운드 B2 — 같은 연결을 두 패널이 나눠 쓰면 세대가 겹친다.
    // 되돌리기는 **그 요청의 세대**이면서 **아직 그 자리에 있을 때**만 일어나야 한다
    let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);
    panel.set_remote_path(RemotePath::new("/root"));
    panel.request_remote_list(&manager);
    let generation = panel.remote_generation;

    // 남의 세대로는 되돌지 않는다
    assert!(!panel.revert_remote_path(generation + 1));
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/root")
    );

    // 그 사이 다른 곳으로 또 옮겼으면 지난 되돌리기는 무효다
    panel.set_remote_path(RemotePath::new("/etc"));
    assert!(
        !panel.revert_remote_path(generation),
        "지난 요청이 지금 위치를 되돌렸다"
    );
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/etc")
    );
}

#[test]
fn 조회가_실패하면_옮기기를_무른다() {
    // F-7 리뷰 B2 — 주소창은 새 폴더를, 목록은 이전 폴더를 가리킨 채 갈라지면
    // 그 위에서 연 메뉴가 보이는 것과 다른 경로에 삭제·권한 변경을 건다
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www")
    );
    let manager = ConnectionManager::new(std::sync::Arc::new(|| {}));
    panel.set_remote_path(RemotePath::new("/root"));
    panel.request_remote_list(&manager);
    let generation = panel.remote_generation;

    assert!(
        panel.revert_remote_path(generation),
        "되돌릴 자리가 없다고 했다"
    );
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www"),
        "이전 폴더로 돌아오지 않았다"
    );
    // 되돌린 뒤에는 다시 청하지 않는다 — 실패·성공이 번갈아 도는 고리를 만들지 않는다
    panel.take_remote_dirty();
    assert!(!panel.take_remote_dirty());
    // 돌아갈 자리는 한 번만 쓴다
    assert!(!panel.revert_remote_path(generation));
}

#[test]
fn 원격_탭에서_연_새_탭은_로컬_시작_폴더다() {
    // 사용자 보고 — 원격 탭에서 `+`를 누르면 연결이 없는 원격 탭이 복제돼 목록이 빈 채로 섰다.
    // 새 탭은 로컬 시작 폴더를 가리켜야 한다
    let ctx = egui::Context::default();
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);

    panel.handle_tab(TabAction::New, &ctx);

    let source = &panel.tabs.active().source;
    assert!(!source.is_remote(), "원격 탭이 그대로 복제됐다");
    let path = source.local_path().expect("로컬 탭이어야 한다");
    assert!(
        !path.as_os_str().is_empty(),
        "새 탭이 열거할 수 없는 빈 경로를 가리킨다"
    );
}

#[test]
fn 원격_탭을_바꾸면_그_탭의_목록을_다시_읽는다() {
    // F-7 3라운드 B1 — 목록은 탭이 아니라 패널 하나가 든다. 탭만 바꾸고 목록을 그대로 두면
    // 주소창은 이 탭을, 목록은 저 탭의 폴더를 보인다 — 그 위에서 연 원격 메뉴가
    // **화면에 없는 경로**에 삭제·권한 변경을 건다
    let ctx = egui::Context::default();
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);
    // 같은 사이트의 다른 폴더를 원격 탭으로 하나 더 연다
    // (`Ctrl+T`는 원격 위치를 복제하지 않는다 — 로컬 시작 폴더를 연다)
    let site = panel.active_site().expect("원격 탭");
    panel.open_remote_tab(site, RemotePath::new("/var/log"));
    panel.take_remote_dirty();

    // 첫 원격 탭으로 돌아간다 — 그 탭이 보는 곳을 다시 읽어야 한다
    let first = panel
        .tabs
        .sources()
        .iter()
        .position(|source| matches!(source, TabSource::Remote { .. }))
        .expect("원격 탭");
    panel.handle_tab(TabAction::Switch(first), &ctx);
    assert!(
        panel.take_remote_dirty(),
        "원격 탭으로 바꿨는데 목록을 다시 읽지 않는다"
    );
    // 답이 오기 전까지 목록은 비어 있어야 한다 — 옛 탭의 항목이 남으면 그 사이에
    // 연 메뉴가 화면에 없는 경로를 겨눈다 (F-7 4라운드 M1)
    assert_eq!(
        panel
            .list
            .selected_remote(&RemotePath::new("/var/www"))
            .len(),
        0,
        "전환 직후 옛 항목이 남아 있다"
    );

    // 로컬 탭으로 바꾸면 로컬 열거가 도므로 이 깃발은 서지 않는다
    let local = panel
        .tabs
        .sources()
        .iter()
        .position(|source| matches!(source, TabSource::Local(_)))
        .expect("로컬 탭");
    panel.handle_tab(TabAction::Switch(local), &ctx);
    assert!(!panel.take_remote_dirty(), "로컬 탭에 원격 조회를 청했다");
}

#[test]
fn 원격_탭을_여는_사이_도착한_로컬_열거는_버린다() {
    // 실사용 결함(2026-08-05): 사이트를 더블클릭하면 앱이 죽었다. 분할로 만들어진 패널이
    // 시작 폴더를 읽는 중에 활성 탭이 원격이 되고, 뒤늦게 온 로컬 결과를 그 탭에 커밋하려
    // 했기 때문이다(개발 빌드는 단언으로 종료, 배포 빌드는 원격 탭이 로컬 탭으로 둔갑)
    let ctx = egui::Context::default();
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    panel.start_load(
        std::path::PathBuf::from(r"C:\Windows"),
        PendingNav::Push,
        &ctx,
    );
    assert!(panel.load.is_loading(), "열거가 시작되지 않았다");

    // 그 사이 사이트를 연다 — 활성 탭이 원격이 된다
    panel.open_remote_tab(SiteId(1), RemotePath::new("/var/www"));
    assert!(panel.is_remote());
    assert!(!panel.load.is_loading(), "원격 탭인데 `읽는 중…`이 남았다");

    // 이미 채널에 실려 있던 결과가 뒤늦게 도착해도 원격 탭은 그대로여야 한다
    panel.apply_enumerated(EnumOutcome::Ok(Vec::new()), &mut icons);
    assert!(panel.is_remote(), "원격 탭이 로컬 탭으로 둔갑했다");
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www")
    );
    assert!(panel.status.is_empty(), "원격 탭에 로컬 상태 문구가 남았다");
}

#[test]
fn 원격_폴더를_더블클릭하면_그_안으로_들어간다() {
    // 사용자 보고(2026-08-05): 원격 목록에서 폴더를 더블클릭해도 아무 일도 없었다 —
    // 여는 경로가 로컬 경로를 요구해 원격 탭에서는 통째로 빠져나갔기 때문이다
    let ctx = egui::Context::default();
    let mut icons = IconCache::new();
    let (mut panel, _) = remote_panel_in(TabPhase::Ok);
    let generation = panel.request_remote_list(&ConnectionManager::new(std::sync::Arc::new(|| {})));
    let _ = generation;
    panel.list.set_remote_entries(
        with_parent_first(vec![
            remote_entry("public_html", true),
            remote_entry("a.txt", false),
        ]),
        &mut icons,
    );
    panel.take_remote_dirty();

    // 폴더(인덱스 1 — 0은 `..`)로 들어간다
    panel.handle_list_action(FileListAction::Open(1), &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www/public_html")
    );
    assert!(
        panel.take_remote_dirty(),
        "들어간 폴더의 목록을 청하지 않는다"
    );

    // `..`로 위로 올라간다
    panel
        .list
        .set_remote_entries(with_parent_first(Vec::new()), &mut icons);
    panel.handle_list_action(FileListAction::Open(0), &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www")
    );

    // 파일은 열지 않는다 — 원격 파일 열기는 범위 밖이다
    panel.list.set_remote_entries(
        with_parent_first(vec![remote_entry("a.txt", false)]),
        &mut icons,
    );
    panel.take_remote_dirty();
    panel.handle_list_action(FileListAction::Open(1), &ctx);
    assert_eq!(
        panel.tabs.active().source.remote_path().map(|p| p.as_str()),
        Some("/var/www"),
        "파일을 눌렀는데 위치가 바뀌었다"
    );
    assert!(!panel.take_remote_dirty());
}

#[test]
fn 처음_읽는_중에는_목록_자리에_자리표시를_세운다() {
    let _언어 = crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
    let ctx = egui::Context::default();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    panel.start_load(
        std::path::PathBuf::from(r"C:\Users"),
        PendingNav::None,
        &ctx,
    );
    assert!(
        panel.shows_loading_placeholder(),
        "아직 아무것도 못 읽었는데 자리표시를 세우지 않는다"
    );

    let 프레임 = draw_once(&mut panel, &SiteStore::new());
    let 화면 = drawn_texts(&프레임);
    assert!(
        화면.iter().any(|t| t == "읽는 중…"),
        "읽는 중이라는 것이 화면에 없다: {화면:?}"
    );
    assert_eq!(
        skeleton_bars(&프레임),
        crate::ui::remote_states::SKELETON_BARS,
        "목록 자리가 빈칸이다 — 자리표시가 서지 않았다"
    );
}

#[test]
fn 목록이_있는_폴더에서_옮기는_중에는_이전_목록을_둔다() {
    let _언어 = crate::i18n::LanguageGuard::lock(crate::app::settings::LanguageSetting::Korean);
    let ctx = egui::Context::default();
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    panel.apply_enumerated(
        EnumOutcome::Ok(vec![local_entry("Documents", true)]),
        &mut icons,
    );
    panel.start_load(
        std::path::PathBuf::from(r"C:\Users\Public"),
        PendingNav::Push,
        &ctx,
    );
    assert!(
        !panel.shows_loading_placeholder(),
        "보여줄 목록이 있는데 자리표시로 덮었다 — 옮길 때마다 화면이 한 번 더 깜빡인다"
    );

    let 프레임 = draw_once(&mut panel, &SiteStore::new());
    let 화면 = drawn_texts(&프레임);
    assert!(
        화면.iter().any(|t| t == "Documents"),
        "이전 폴더의 목록이 사라졌다: {화면:?}"
    );
    assert_eq!(
        skeleton_bars(&프레임),
        0,
        "보여줄 목록이 있는데 자리표시로 덮었다"
    );
}

#[test]
fn 다_읽고_나면_자리표시를_거둔다() {
    let mut icons = IconCache::new();
    let mut panel = PanelState::new(std::path::PathBuf::from(r"C:\Users"));
    panel.apply_enumerated(EnumOutcome::Ok(Vec::new()), &mut icons);
    assert!(
        !panel.shows_loading_placeholder(),
        "다 읽었는데 자리표시가 남았다"
    );
}
