# Handoff: File Explorer with integrated FTP/SFTP (FileZilla-style)

## Overview
A Windows-style dual-pane file explorer where **remote FTP/SFTP servers are a panel type, not a separate app mode**. The user works in local panes, and connecting to a site splits the workspace into paired local + remote panes so files can be dragged across. A transfer queue / server log panel docks at the bottom.

Target: desktop app (Electron / Tauri / WinUI) or a desktop-sized web app. Dark theme, dense, keyboard-driven — closer to a file manager than a web page.

## About the design files
The files in `design-files/` are **design references written as HTML prototypes**. They show intended look, layout, and behavior — they are not production code to copy.

Recreate these designs in the target codebase's existing environment (React, Vue, SwiftUI, WinUI, etc.) using its established component patterns, state library, and styling approach. If no environment exists yet, choose the framework appropriate for a desktop file manager (Electron + React, or Tauri + React/Svelte) and implement the designs there.

The HTML uses a small custom template runtime (`support.js`, `<sc-for>`, `<sc-if>`, `<dc-import>`). Read it as "repeat this list", "render if true", "mount this child component" — do not port the runtime.

## Fidelity
**High fidelity.** All colors, sizes, fonts, paddings, and states are final and intended to be matched precisely. Every hex value and pixel size in this README is authoritative. Copy is Korean and final.

Exception: file/directory listings, server log lines, and queue rows are sample data.

---

## Layout skeleton

```
┌───────────────────────────────────────────────────────────────┐
│ title / menu strip                             28px           │
│ toolbar + address bar                          ~34px          │
├──────────┬────────────────────────────────────────────────────┤
│ sidebar  │  pane grid: rows of [local pane | remote pane]      │
│ 260px    │  each row flex:1, gap 4px                          │
│          │                                                    │
│ ▸ 워크    │  ┌──────────────┬──────────────┐                   │
│   스페이스 │  │ ExplorerPane │ ExplorerPane │  ← pair 1        │
│ ▸ 연결    │  └──────────────┴──────────────┘                   │
│          │  ┌──────────────┬──────────────┐                   │
│          │  │ local        │ remote       │  ← pair 2        │
├──────────┴────────────────────────────────────────────────────┤
│ transfer queue / server log panel   268px (collapsible)        │
├───────────────────────────────────────────────────────────────┤
│ status bar                          30px                      │
└───────────────────────────────────────────────────────────────┘
```

Root: `display:flex; flex-direction:column; position:relative; background:#1E1E1E; overflow:hidden`. Design size 1660 × 990.

---

## Design tokens

### Colors
| Role | Hex |
|---|---|
| App / pane background | `#1E1E1E` |
| Panel strip / tab bar background | `#1B1B1B` |
| Column header, row-alt, control fill | `#252525` |
| Zebra row (odd) | `#252525` |
| Active tab fill | `#2A2A2A` |
| Hover fill (rows, buttons) | `#2E2E2E` |
| Hover fill (menu items, icon buttons) | `#383838` |
| Input / list well background | `#151515` |
| Border, subtle | `#2C2C2C` |
| Border, default | `#333333` |
| Border, control | `#3A3A3A` |
| Text primary | `#E8E8E8` |
| Text secondary | `#C8C8C8` |
| Text tertiary / meta | `#9A9A9A` |
| Text disabled / label | `#6A6A6A` |
| Selection / active accent (blue) | `#4A9EFF` |
| Folder icon | `#E8B34D` |
| Success / connected (dot) | `#4ADE80` |
| Success text | `#7FD6A2` |
| Success fill / border | `#16241C` / `#2F6B4F` |
| Primary button (연결) | fill `#2F6B4F`, border `#3E8A66`, text `#E8FFF2`, hover `#387E5D` |
| Warning / connecting | dot `#E8B34D`, fill `#241F14`, border `#6B562F`, text `#E8B34D` |
| Error | `#FF6B6B`; text on dark `#FF8A8A`; fill `#2A1A1A`; border `#4A2626` |
| Destructive menu hover | `#C42B1C` |
| Progress track | `#2A2A2A` |

Only two background tones dominate: `#1E1E1E` surfaces and `#252525` controls. Do not introduce gradients.

### Typography
- UI font: system UI sans (Segoe UI / Malgun Gothic on Windows). No web font needed.
- Monospace (server log only): `Consolas, "D2Coding", monospace`, 12px / 17px line-height.
- Sizes: 11px badges, 12px meta and disabled labels, **13px is the base UI size** (rows, tabs, buttons, labels), 14px dialog/error headings, 15px sidebar item names, 19px error glyph.
- Weight 400 throughout; hierarchy comes from color, not weight.

### Spacing / metrics
- Row heights: list row `24px`, tab `28px`, menu item `26–28px`, column header `22px`, address bar field `22px`, button `28–30px`, status bar `30px`.
- Sidebar width `260px`; sidebar workspace item height `60px`.
- Pane gap `4px`. Panel height `268px`.
- Cell padding `0 6px`; menu item padding `0 12px`; button padding `0 16px` (small) / `0 24px` (dialog).
- **Border radius: 0 everywhere** except status dots (`border-radius:3–4px`, i.e. circles) and radio circles (`8px`). This is a deliberately square, native-Windows look.
- Shadows only on floating layers: menus `0 10px 28px rgba(0,0,0,0.6)`, dropdowns `0 8px 24px rgba(0,0,0,0.5)`, toast `0 8px 24px rgba(0,0,0,0.5)`.

### Icons
Folder icons are drawn with two absolutely-positioned `div`s (tab + body) in `#E8B34D` inside a 16×16 box — replace with the codebase's real icon set. Glyphs used as text: `✕ ▾ ▼ ⏸ ⧉ ⚙ 🗑 −`. Arrows in the queue: `↑` upload, `↓` download.

---

## Screens / views

### 1. Sidebar — workspaces + connections
Width 260px, background `#1E1E1E`, right border `#2C2C2C`.

**워크스페이스 section**: each item is 60px tall, `background:#232323` (inactive) / `#2E2E2E` (active), 1px `#2C2C2C` border, `padding-left:38px`, folder icon absolutely at `left:12px; top:22px`. Active item has a 3px `#4A9EFF` left edge bar. Two stacked lines: name (15px `#E8E8E8`, ellipsized) with a `패널 N` count badge (`#242424` fill, `#333333` border, `#8A8A8A` text, 11px, height 16px), and path (13px `#8A8A8A`). Hover on inactive → `#282828`.

Clicking a workspace switches the pane grid to **that workspace's own set of pane pairs**. Each workspace owns its pairs and its connections independently.

**연결 section**: lists only remote sites (sftp / ftps / ftp) — local paths live under workspaces. Each row shows a status dot, site name, and protocol. Interactions:
- single click → select
- **double click → connect** (adds a local+remote pane pair for that site)
- right click → context menu: site name header, divider, `🗑 삭제 / Del` (hover `#C42B1C`). Deleting removes only the sidebar shortcut, **not** the saved site in the site manager.
- `+` button → connect menu (below)

### 2. Connect menu (`+` in the 연결 header)
Floating panel, `#1E1E1E`, 1px `#333333`, shadow, `padding:4px 0`, 13px text. Contents:
- caption `등록된 사이트` (12px `#6A6A6A`, padding `4px 12px 6px`)
- one row per registered site (28px, hover `#383838`), showing protocol dot + name
- `새 사이트 추가…` at the bottom, which opens the site manager

There is **no 설정… item** — it was removed.

### 3. ExplorerPane (`design-files/ExplorerPane.dc.html`)
The same component renders local and remote panes; only a badge differs. Structure top→bottom:

1. **Tab bar** — 28px, tabs with folder icon + title + `✕`. Active tab `#2A2A2A` / `#E8E8E8`; inactive transparent / `#9A9A9A`, hover `#252525`. Then a `+` button (24px, hover `#383838`).
2. **Site dropdown caret `▾`** — 18px wide, immediately right of `+`, shown **on every pane (local and remote) whenever at least one site is registered**. `#9A9A9A`, hover `#E8E8E8` on `#383838`; when open, background `#383838`. Opens a 250px dropdown at `left:8px; top:28px`: caption `연결 사이트를 새 탭으로`, then one 28px row per site with a status dot (green `#4ADE80` if connected, else `#6A6A6A`), name, and protocol in `#6A6A6A` 12px. Picking a site opens it as a new tab in that pane. A full-bleed transparent overlay closes the menu on outside click.
3. **Path/address row** — 22px field, `#1E1E1E` background, 13px text, ellipsized. For a not-yet-connected remote tab it shows the placeholder `sftp://` in `#6A6A6A`.
4. **Column header** — 22px, `#252525`, `#C8C8C8` 13px. Right-click opens the column menu (186px, caption `표시할 컬럼`, rows with a `#7FD6A2` check glyph). Remote panes add **권한** and **소유자** columns; disabled entries use `cursor:default` and dim text.
5. **Row list** — 24px rows, zebra `#252525` on odd rows, hover `#2E2E2E`. Folder-first sorting, `..` always first. Locked/inaccessible entries render at `opacity:0.4` icon and `#6A6A6A` text. Symlinks show `→ target` after the name.
6. **Status line** — item counts, e.g. `폴더 4 파일 7`.

### 4. Remote pane tab states
A remote pane's tabs are the unit of connection. Tab label order is: folder icon → **protocol badge** → site name → `✕`.

Badge: 15px tall, `padding:0 5px`, 11px text, 1px border, 4px gap, with a 5px dot.

| Tab phase | Dot | Fill | Border | Text | Badge label |
|---|---|---|---|---|---|
| connected | `#4ADE80` | `#16241C` | `#2F6B4F` | `#7FD6A2` | protocol (`sftp` / `ftps` / `ftp`) |
| connecting | `#E8B34D` | `#241F14` | `#6B562F` | `#E8B34D` | `연결 중…` |
| new / empty | `#6A6A6A` | `#242424` | `#3A3A3A` | `#8A8A8A` | `연결 없음` |

Folder icon opacity is `1` when connected, `0.45` otherwise. Pane body per phase:
- **connected** → normal remote listing
- **connecting** → skeleton: 8 bars, height 12px, `#262626`, 6px gaps, varying widths, `padding-top:14px`
- **new** → centered empty state, 13px, `#6A6A6A`: `주소창에 sftp://호스트 를 입력해 연결하세요` (with `sftp://` in `#7FD6A2`) and `사이드바의 사이트를 이 탭으로 끌어다 놓아도 됩니다`
- **failed** → see below

Tab actions: `+` duplicates the current tab's connection; `▾` opens the site dropdown; `✕` closes the tab, and closing the last tab disconnects the pane. Item count shows `—` while not connected.

### 5. Failed connection state
Centered column, gap 14px, `padding:0 28px`, `text-align:center`, on `#1E1E1E`:
- 34px circle, 2px `#FF6B6B` border, `border-radius:18px`, `!` glyph 19px `#FF6B6B`
- 14px `#E8E8E8`: `연결하지 못했습니다`
- 13px / 1.6 `#9A9A9A`, `text-wrap:pretty`: `530 Login incorrect — 사용자 또는 비밀번호가 올바르지 않습니다.` / `암호화 설정이 서버와 다를 수도 있습니다.`
- two buttons (`재시도`, `설정 열기`): 28px, `padding:0 16px`, `#252525` fill, `#3A3A3A` border, `#D8D8D8` text, hover `#2E2E2E`
- text link `서버 로그 보기`, 12px `#6A6A6A`, hover `#C8C8C8`

### 6. Transfer queue panel
268px tall, `#1E1E1E`, top border `#333333`.

**Panel tab strip** (28px, `#1B1B1B`), tabs in this order:
`전송 큐 (N)` · `서버 로그` · **`성공 (N)`** · `실패 (N)`

Active tab: background `#1E1E1E`; text `#E8E8E8`, or `#7FD6A2` for 성공, `#FF8A8A` for 실패. Inactive `#9A9A9A`, hover `#E8E8E8`. 성공 and 실패 are **filters over the same queue rows** (`kind: done | err`), not separate lists; 전송 큐 shows all. Right side: `⏸`, `✕`, and `▼` (collapse) icon buttons, 26×26, hover `#383838`.

**Per-connection tab row** (28px, bottom border `#2C2C2C`, `padding:0 4px`, gap 2px): `전체` plus one tab per active connection, each with a 6px status dot and a 2px bottom border accent when active.

**Queue table** — grid `34px 1fr 300px 120px 84px 118px 150px`: 방향 / 로컬 파일 / 원격 파일 / 서버 / 크기 / 진행률 / 상태. Header 22px `#252525`; rows 24px, zebra, hover `#2E2E2E`. Progress cell: 110×6px track `#2A2A2A` with a fill bar colored by state (green complete, blue active, red error). Direction arrow and state text are colored per state.

### 7. Server log panel
Same 268px shell and tab strip; right side has `⧉` and `▼`. Body `padding:6px 10px`, 2px gaps, monospace 12px/17px rows of three columns: time (`#6A6A6A`, 62px), kind (44px, colored by severity — command / response / error), message (`#B4B4B4`, ellipsized).

### 8. Status bar (always visible, 30px)
`padding:0 10px`, gap 14px, 13px, `#1E1E1E`, top border `#2C2C2C`:
`▸/▾ 전송 큐` (toggles the queue panel) · queue summary `#9A9A9A` · 240×6px overall progress bar · current transfer filename `#C8C8C8` · spacer · failure pill (only when failures exist: `● 실패 N`, `#FF6B6B` on `#2A1A1A` with `#4A2626` border, height 20px, `padding:0 8px`) · connection state `● label` · `▸/▾ 로그` toggle.

### 9. Site manager dialog
Modal over a scrim; header, two-column body, footer.

**Left column** — sites list in a `#151515` well with `#333333` border. The list shows **registered sites only, flat, with no `내 사이트` root node**; each row is 24px, `padding-left:8px`, gap 6px, folder icon + name, selected row highlighted. Below it a 3-column button grid (gap 8px, `padding:2px 30px 6px`): `이름 바꾸기(R)` · `삭제(D)` · `복제(I)`. The 새 사이트 / 새 폴더 / 새 북마크 buttons were removed.

**Right column** — tabs: `일반` · `전송 설정` · `문자셋`. (The `고급` tab is to be removed; the transfer-settings and charset tabs are pending redesign from reference images the user will supply — build them from the current HTML as-is and expect changes.)

*일반 tab*: labeled rows, label column `96px` `#C8C8C8` 13px, controls 26–28px on `#151515` with `#3A3A3A` border — host, port, protocol, encryption, logon type, user, password. The 배경색 and 코맨트 fields were removed; the remaining vertical space is left empty.

*문자셋 tab*: heading text, two radio options (14px circle, `border-radius:8px`, 1px border, 6px inner dot), then an `인코딩(E):` row (210px field) that is disabled — dimmed fill and text — unless the custom-charset radio is selected. Footnote in `#9A9A9A` 13px / 1.7: `문자셋을 잘못 지정하면 파일명이 올바르게 보여지지 않을 수 있습니다.`

**Footer** (58px, top border `#2C2C2C`, right-aligned, gap 10px, buttons 30px `padding:0 24px`): `연결(C)` primary green, then `확인(O)` and `취소` secondary. `연결(C)` registers the site and connects immediately; `확인(O)` registers only and shows a toast `<host> 등록됨 · 더블클릭하여 연결`.

### 10. Toast
Bottom-right, `right:16px; bottom:44px`, 34px tall, `padding:0 14px`, `#252525` fill, `#3A3A3A` border, shadow, 13px `#E8E8E8`, with a 7px `#6A6A6A` dot. Auto-dismiss after **3200 ms**.

---

## Interactions & behavior

Connection lifecycle:
1. Double-click a sidebar site (or pick one from a connect/site menu) → phase `connecting`; the pane shows the skeleton and the tab badge reads `연결 중…`.
2. After the server responds → `ok`: listing renders, badge becomes the protocol, dot green.
3. On failure → `error`: the failed-connection view replaces the listing. `재시도` restarts the attempt; `설정 열기` opens the site manager.

In the prototype, connect resolves after **1600 ms** and a new-tab connect after **1200 ms**, and `legacy-backup` always fails — these are simulation stand-ins for real async FTP calls, not intended timings.

Auto-split: connecting from an idle workspace collapses the workspace's own pairs to the first one and appends a local+remote pair per connection, so the local source sits immediately left of its remote target.

Other behavior to implement:
- Drag files between the paired local and remote panes to enqueue a transfer (target behavior; not simulated in the prototype).
- Drag a sidebar site onto a pane's tab bar to open it as a new tab there.
- Typing `sftp://host` in a remote pane's address bar connects that tab.
- Menus and dropdowns close on outside click (full-bleed transparent overlay, `z-index` above content) and on `Esc`.
- `✕` on a tab stops propagation so it doesn't also select the tab.
- Queue panel and log panel are mutually exclusive in the same dock slot; the status bar carets toggle them.
- Shortcuts referenced in the UI: `Del` (delete site), `Ctrl+S` (site manager) — wire the real accelerators in the host app.

## State model

Root:
- `workspaceIndex` — selected workspace
- `connections: [{ site, phase: 'connecting'|'ok'|'error', workspaceIndex }]`
- `paneTabs: { [connection]: { list: [{ id, site, phase: 'ok'|'connecting'|'new' }], activeIndex } }` — per-pane tab sets; a tab with `site: null` is the `new` empty tab
- `registeredSites` (site manager records) and `hiddenSites` (removed from the sidebar but still registered) — kept separate on purpose
- `queue: [{ direction, localPath, remotePath, server, size, percent, state, kind: 'wait'|'active'|'done'|'err' }]`
- `queueFilter: 'all'|'done'|'err'`, `queueServer` (per-connection tab), `dockPanel: 'queue'|'log'|null`
- dialog/menu flags: `siteManagerOpen`, `dialogTab`, `selectedSite`, `connectMenuOpen`, `contextMenu`, `columnMenu`, `toast`
- column visibility: `showPermissions`, `showOwner`

Per-pane local state: `siteDropdownOpen`.

Transfers, listings, and connections should be real async operations against an FTP/SFTP client in the host process; the prototype's `setTimeout`s mark where those awaits belong.

## Assets
None. All iconography is CSS shapes or text glyphs — substitute the codebase's icon set. No images or fonts to ship.

## Screenshots
`screenshots/` — captured from the prototype, scaled to fit; treat the README values as authoritative over pixel measurement of these images.

| File | Shows |
|---|---|
| `01-overview.png` | Full layout: sidebar, three local+remote pane pairs, a failed connection in the bottom-right pane |
| `02-transfer-queue.png` | Queue panel, `전체` tab, mixed active / waiting / complete / error rows |
| `03-queue-filter-success.png` | `성공` filter — completed rows only |
| `04-queue-filter-failed.png` | `실패` filter — error rows only |
| `05-server-log.png` | Server log panel, monospace rows |
| `06-site-manager-general.png` | Site manager, `일반` tab, flat site list with no root node |
| `07-site-manager-transfer.png` | Site manager, `전송 설정` tab (pending redesign) |
| `08-site-manager-charset.png` | Site manager, `문자셋` tab, disabled 인코딩 field |

## Files
- `design-files/FileExplorer-FTP.dc.html` — the full screen: sidebar, pane grid, connection states, queue/log panels, status bar, site manager, all menus. Start here.
- `design-files/ExplorerPane.dc.html` — the reusable pane (tab bar, site dropdown, address row, columns, rows, status line), used for both local and remote.
- `design-files/support.js` — the prototype's template runtime. **Reference only; do not port.**

Open `FileExplorer-FTP.dc.html` in a browser to interact with the prototype: click workspaces, double-click sidebar sites, use a pane's `+` and `▾`, and toggle the queue/log from the status bar.
