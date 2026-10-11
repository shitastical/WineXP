# Windows XP Demo — Plan to a Complete, Installable XP Environment

**Goal.** A demo people can install that gives them a *completely accurate Windows XP UI*: the
full shell (desktop, taskbar, Start menu, right-click menus, Control Panel, all settings), a
working virtual filesystem, a useful app suite, and a **real working Internet Explorer**.

---

## 1. Where the project is today

| | |
|---|---|
| Stack | Native **Rust + egui/eframe** (`glow`/EGL), single binary, assets embedded by `build.rs` |
| Size | ~8 600 lines across 13 shell modules + 8 app modules |
| Shell | Bliss wallpaper, desktop icons + marquee select, Luna window manager (drag/resize/min/max/close), taskbar with task buttons + tray + clock, Start menu (2 columns, submenus), boot windows, "at risk" balloon, power modal |
| Apps | Internet Explorer (built-in renderer), My Computer, Notepad, Paint, Minesweeper, Winamp, ErrorBox |
| Assets | 175 embedded files under `assets/` (icons, sounds? wallpapers, fonts) |
| Runner | `run.sh` — release build, single-instance enforced (kills old copies), PID file |

It is a faithful, line-by-line port of **ShizukuIchi/winXP** (winxp.vercel.app), which is kept in
`.reference/winXP/` and transcribed module-by-module (each Rust file carries a
`//! Transcribed from …` header). **That is the single most valuable asset in this repo**: the
pattern "read the reference module → rewrite it in Rust → keep the source path in a comment" is
what makes this fast and accurate.

### Gap analysis against the goal

| Area | Today | Needed for the goal |
|---|---|---|
| Shell | Desktop, taskbar, Start menu | Context menus, Show Desktop, Alt+Tab, tray behaviour, balloon queue, Run/Search/Help, Welcome/login screen, snapping, animations |
| Settings | none persisted | Control Panel folder + ~14 applets, Display Properties (5 tabs), a persisted settings store |
| Filesystem | My Computer is a static view | A real sandboxed VFS (`C:\` tree), file ops, Recycle Bin, common Open/Save dialogs |
| Apps | 7 | ~25 more (Calculator, Task Manager, Command Prompt, WordPad, Solitaire, Pinball, WMP 8, Outlook Express, system tools …) |
| Browser | Simulated page renderer | **Real browsing** via an embedded OS webview, behind faithful IE6 chrome |
| Accuracy | Best-effort by eye | A measured diff harness with reference screenshots and a pass threshold |
| Distribution | `./run.sh` | AppImage/.deb/.msi/.dmg, asset policy, provenance/licence file |

### Known defects to fix first

1. ~~Start button never opened the Start menu.~~ **Fixed and verified** this session: the taskbar's
   full-width background click handler was registered *after* the Start button, and egui's hit test
   prefers the most recently registered widget, so it silently ate every Start click. The background
   handler is now registered first (`src/taskbar.rs`).
2. Start-menu row hit boxes are offset from the drawn rows (the row rect height and the `y` advance
   disagree) — clicking the middle of a row misses. Verify and align in `src/startmenu.rs`.
3. Paint's dialogs are hand-rolled rather than the XP common dialogs; it should use the shared
   Open/Save dialog once Phase 2 lands.
4. 19 dead-code warnings (`theme::text`, `icons::layout_top_left`, `menu::ASSET_KEYS`, …). Wire them up
   or delete them — they are the map of things that were started and not finished.

---

## 2. Reference inventory (what to harvest, and from where)

Clone everything into `.reference/` (git-ignored) and read the source, **not** the live site.

### 2.1 Ground truth for pixel accuracy
- **A real XP SP3 install in a VM at 1024×768, 100 % DPI** — the only authoritative source. Capture a
  library of ~120 screens into `reference-screens/`.
- `https://github.com/rozniak/xfce-winxp-tc` — C/GTK project explicitly aiming at pixel-perfect XP.
  Best source for **measurements and layout** (it had to solve the same problems natively).
- `https://github.com/botoxparty/xp.css` — Luna CSS: exact colours, border radii, gradients. Convert
  straight into `src/theme.rs` constants.
- `https://github.com/Simandara/Windows-XP-UI-Kit` — Figma kit. **CC-BY-NC-ND: visual reference only,
  never vendored.**
- `https://archive.org/details/msoffice97-excel-b3515-en-full` — Office-era chrome reference (the
  Vintage Software Collection may also have 2000/2003).

### 2.2 Shell, Control Panel and settings
- `https://github.com/wieslawsoltes/WinXp` — **56 app definitions**, plain HTML/CSS/JS, no deps.
  Use as the **feature checklist**; its own README says pixel-perfect equivalence is *not*
  established, so never treat it as the visual authority. Read `docs/COMPATIBILITY.md` and
  `THIRD_PARTY_NOTICES.md`.
- `https://github.com/Ingan121/ModernActiveDesktop` — Display Properties and Internet Options
  applets, some near pixel-perfect (98-era: check tab layout against XP).
- `https://github.com/AdamWhiteHat/TotalWinUICustomization` — C# Appearance/Display Properties.
- `https://github.com/Sparsh712/WinXP-Portfolio` — boot sequence, F8 advanced options, F2 BIOS,
  Turn Off Computer dialog with its amber/red/green buttons.
- `https://gist.github.com/theamith/5559224`, `https://gist.github.com/begoon/5089091` —
  the complete list of control-panel applets and `*.cpl` / run commands. **Use as the completeness
  checklist.**
- `https://en.wikipedia.org/wiki/Control_Panel_(Windows)` — applet list.

### 2.3 Explorer, filesystem, file dialogs
- `https://github.com/Phault/winOS` — near-pixel-perfect XP look **with a working filesystem**.
- `https://github.com/ducbao414/win32.run` — XP file system, **XP-style file picker/saver dialogs**,
  Word. Best source for common dialogs.
- `https://github.com/Alena0490/Windows-XP` — Explorer, Command Prompt, Paint, Notepad, IE6,
  Display Properties, Help and Support Center, Picture and Fax Viewer, Solitaire, Outlook Express and
  a skinned WMP 8. Pure CSS with Luna variables → the best styling source.

### 2.4 Other full desktops worth mining
`https://github.com/ShizukuIchi/winXP` (current base) ·
`https://github.com/Cyanoxide/react-xp` (IE + Wayback Machine) ·
`https://github.com/proxy-m/OnlineWinXP` ·
`https://github.com/vimaltiwari2612/WindowsXP` ·
`https://github.com/Aldemylla/windows-xp-web` ·
`https://github.com/faisalAkhtar/windows-xp` ·
`https://github.com/leekli/windows-desktop-fun` ·
`https://github.com/Yosi2377/WinXpDesktop` (Calculator, Paint, Task Manager) ·
`https://github.com/ganoninc/winerror-simulator` (the XP error-window effect) ·
`https://github.com/topics/windows-xp`

### 2.5 The browser
- `https://github.com/codedit/InternetExploder` — IE6 skin over WebKit: the closest "real browser in
  an IE shell" example.
- In Rust the route is a native webview — **`wry`** (WebView2 on Windows, WKWebView on macOS,
  WebKitGTK on Linux) or **Tauri** — with IE6 chrome drawn by us. This is why the current iframe
  approach can't work: most sites refuse to be framed, so a real webview removes the need for a proxy.
- Period-accurate pages: the **Wayback Machine** approach from `Cyanoxide/react-xp`.

### 2.6 Apps
- Games / media / messaging: `https://github.com/NamanOG/WindowsXP-Pinball`
  (+ `https://alula.github.io/SpaceCadetPinball/`), `https://quenq.com/apps/reborn-xp/`
  (claims 1:1 Paint, Notepad, WordPad, Calculator, Minesweeper, Solitaire, Pinball, WMP, cmd — not
  confirmed open source, use as a **visual comparison tool only**),
  `https://github.com/topics/msn` (WhatsMSN, msn-js),
  `https://github.com/qqkookie/XPAccApps` (C++ ReactOS ports: Calculator, Task Manager, WordPad,
  Paint, Minesweeper, clock — behaviour/layout reference).
- `https://98.js.org` (and its `os-gui` library on jsdelivr) — Paint, Sound Recorder, Solitaire;
  shares the jspaint base. `https://jspaint.app` remains the Paint reference the user asked for.
- Office (only if wanted): `https://github.com/bittuhere/msoffice` (HTML/CSS/JS Excel, Word,
  PowerPoint, OneNote + IndexedDB — the only one with all three, needs an Office 2003 restyle),
  `https://github.com/storytold/wordcraft` (clean-room Rust `.docx` read/write, MIT/Apache),
  `https://github.com/vinay3254/Msword` (React+Node, needs a backend).

### 2.7 Theming and assets
- `https://github.com/botoxparty/xp.css`, `https://github.com/luiz-chagas/react-windows-xp`
- `https://github.com/Luka12-dev/Windows-XP-Theme` — 500+ XP icons plus startup/shutdown/error
  sounds. **These are Microsoft's assets** — see §7.
- `https://github.com/mshernandez5/WelcomeXP` — XP login screen as a theme.

---

## 3. Architecture additions

Five new pieces unlock almost everything else. Build them before the app suite.

```
src/
  settings.rs      # typed, persisted app+system settings (JSON in the app data dir)
  vfs.rs           # sandboxed virtual filesystem: C:\ tree, drives, recycle bin, file ops
  dialogs.rs       # XP common dialogs: Open, Save As, Message box, Browse for Folder, Properties
  screens/         # Control Panel applets + Display Properties, one module each
  contextmenu.rs   # shared right-click menu infrastructure
tools/
  shot.sh          # Xvfb + xdotool + scrot: render a named screen at 1024x768
  diff.py          # per-screen pixel/SSIM diff against reference-screens/
reference-screens/ # captured ground truth
docs/PROVENANCE.md # Rust module -> source repo -> file -> licence
```

Three structural changes to the existing code:

1. **`AppKind` becomes a registry, not an enum.** Today it is a closed enum with a `key() -> u8` and a
   match in `apps::create` / `apps::from_name` / `tasks`. Moving to
   `&'static AppDef { id, title, icon, default_size, resizable, factory }` lets new apps be added in
   one place and makes single-instance de-duplication data-driven.
2. **Screens render from a table.** Right now the Start menu, tray balloons and the WM each hard-code
   their own geometry. Control Panel needs ~25 dialogs; define them as data
   (`(id, size, tabs, controls)`) and render generically. This is also what makes `--screenshot`
   diffing cheap.
3. **A `--screenshot <name>` CLI flag.** `winxp --screenshot display-properties-tab4 --out shot.png`
   renders one screen, writes it, exits. Without this, automated visual comparison is impossible; with
   it, accuracy becomes a number you can regress-test.

---

## 4. Phased roadmap

Effort is in focused days for one person. Phases 0–2 are the ones that make it *feel* like XP.

### Phase 0 — Accuracy harness and repo hygiene (3–4 days)
- `git init`, CI (fmt, clippy, `cargo test`, build matrix), `docs/PROVENANCE.md` skeleton.
- Build `reference-screens/`: capture ~120 XP screens in a VM (desktop, Start menu, every Control
  Panel applet, Display tabs, Explorer views, each app's menus, the power modal, error boxes).
- Build `tools/shot.sh` + `tools/diff.py` and the `--screenshot` flag.
- **Acceptance:** `tools/diff.py` prints a similarity score for the desktop, Start menu, My Computer
  and Paint, and the score is stable across runs.

### Phase 1 — Shell completeness and settings (2–3 weeks)
- Context menus everywhere (desktop, icons, taskbar, Start entries, title bar, tray).
- **Display Properties**, all five tabs (Themes, Desktop, Screen Saver, Appearance, Settings) with the
  live preview monitor. This is the flagship settings dialog — get it exact.
- Control Panel folder + applets: Display, Add or Remove Programs, Date and Time, Regional and
  Language Options, Sounds and Audio Devices, Mouse, Keyboard, System, Network Connections, User
  Accounts, Folder Options, Power Options, Accessibility Options, Fonts.
- `settings.rs` persisted to the platform data dir; theme/wallpaper/clock changes survive restart.
- Notification area: clickable tray icons, queued balloons, clock → Date and Time, **Show Desktop**.
- **Alt+Tab** switcher, window snapping to screen edges, minimize/restore animation.
- **Run…** dialog, **Search**, Help and Support Center shell, Welcome/login screen, and the full
  Turn Off / Log Off / Restart / Stand By modal.
- **Acceptance:** every applet opens; Display Settings persists across a restart; each of the ~25
  screens scores above threshold in `tools/diff.py`.

### Phase 2 — Explorer and the virtual filesystem (2 weeks)
- `vfs.rs`: sandboxed `C:\` tree (`WINDOWS`, `Program Files`, `Documents and Settings\User\My
  Documents\My Pictures / My Music`, `Recycle Bin`), backed by a real directory in the app data dir.
- Explorer: address bar with breadcrumbs, Back/Forward/Up, View menu (Thumbnails/Tiles/Icons/List/
  Details), status bar, sorting, Rename / New / Delete / Copy / Cut / Paste / Properties.
- **Common Open/Save dialogs** (`dialogs.rs`) over the VFS, and port Notepad/Paint/WordPad to them.
- Recycle Bin with restore/empty; Search Companion; Folder Options; Picture and Fax Viewer.
- **Acceptance:** create a folder, save from Notepad and Paint into it, copy it elsewhere, restart, and
  the file is still there and opens.

### Phase 3 — The app suite (3–4 weeks)
Ordered by "what people will actually reach for":
1. Calculator · Command Prompt (small shell over the VFS: `dir`, `cd`, `type`, `mkdir`, `del`, `cls`,
   `ver`) · Task Manager (Applications/Processes/Performance, with the tray CPU meter) · WordPad ·
   Character Map · On-Screen Keyboard.
2. Games: Solitaire, FreeCell, Spider Solitaire, Pinball, Minesweeper upgrades (Best Times).
3. Media: Windows Media Player 8 (with skins), Sound Recorder, Volume Control.
4. Outlook Express, MSN Messenger (local/loopback only — be honest about this).
5. System tools: Disk Defragmenter, System Restore, System Information, Regedit, Event Viewer,
   Services, Disk Cleanup, the Accessibility wizard, Narrator, Magnifier.
   Label each as **working** or **accurate shell with a working subset**, both in the UI and in the README.

### Phase 4 — The browser (3–5 weeks, the highest-risk item)
- **Recommended:** `wry` in a child window whose geometry tracks the XP window, with our own IE6 chrome
  (menu bar, toolbar, address bar + Go, Links bar, status bar with zone, Favorites, History).
- Fallbacks: if no webview is available at runtime (headless Linux without WebKitGTK), fall back to the
  current built-in renderer plus a Wayback-Machine page list, and say so in the status bar.
- Internet Options: General, **Security**, Privacy, Content (certificates), Connections, Programs,
  Advanced — wired to real settings where the webview allows it.
- **Acceptance:** navigate to a live site, Back/Forward/Refresh/Home/Stop work, and the chrome scores
  against the reference screenshot.

### Phase 5 — Accuracy pass, sound, and packaging (2 weeks)
- Run `tools/diff.py` across all reference screens; fix the largest deviations first; keep a scoreboard
  in `docs/ACCURACY.md`.
- Sounds (startup, shutdown, error, navigation click), cursor sets, wallpapers, screensavers, the
  logon/logoff sequence.
- Packaging: AppImage + `.deb` (Linux), MSI + portable `.zip` (Windows), `.dmg` (macOS). Keep `run.sh`
  for development; add `--resolution 1024x768`, `--fullscreen`, `--scale`.
- **Acceptance:** the packaged artifact opens the desktop on a clean machine with no toolchain
  installed, and `./run.sh` still works exactly as today (release build, single instance).

### Phase 6 — Optional: Office 2003
Only if the user wants it: an Excel/Word/PowerPoint 2003 chrome over the `msoffice` / `wordcraft`
engines, restyled from VM screenshots. Medium effort, large scope — treat as a separate project.

---

## 5. The harvesting workflow (how the agent should work)

For every screen or app:

1. `git clone --depth 1 <repo> .reference/<name>` (files are git-ignored).
2. Read the module that implements it (`src/WinXP/apps/<App>/index.js`, `config.js`,
   `dropDownData.js`, and the `styled-components` CSS in the same file).
3. Rewrite it in Rust. Keep the header comment `//! <App> — faithful port of apps/<App>/…` and copy
   the exact pixel values out of the CSS into named constants.
4. Add the screen to `tools/shot.sh`, capture it, and diff it against the VM reference.
5. Add one row to `docs/PROVENANCE.md`: Rust module → repo → file → licence.

**Never guess a colour or a dimension**: every value should be traceable to a CSS rule, a screenshot
measurement, or a `theme.rs` constant that names its source.

---

## 6. Verification strategy

- **Per change:** `cargo build` (and `cargo clippy`) must be clean; the app must actually start.
- **Per screen:** `tools/shot.sh <screen>` then `tools/diff.py <screen>` — a number, not an opinion.
- **Per app:** its primary workflow driven end-to-end (type, draw, save, reopen, play, navigate).
- **Headless caveat:** this machine's Xvfb + llvmpipe harness is flaky (window occasionally dies
  after heavy interaction). Prefer a real X session or Xvfb per test with a fresh process, and always
  assert on the *process still running* plus the captured pixels, not on the exit code alone.

---

## 7. Legal and asset policy (decide before distributing)

- **Do not use the leaked Windows XP source.** Harvest only from the open-source repos and live demos
  above.
- **The XP icons, sounds, wallpaper and fonts are Microsoft's.** Shipping them in a public installer is
  a real risk. Two acceptable strategies:
  1. Ship a small set of Microsoft-derived assets with an explicit, prominent notice (what most demos
     do), or
  2. Ship only open-source recreations and support an `assets-local/` drop-in directory so users
     supply their own files (the XPForge approach).
  Pick one deliberately and document it in `README.md` + `docs/PROVENANCE.md`.
- **Licences are mixed.** `xp.css` (MIT), `wordcraft` (MIT/Apache), ReactOS/Wine (GPL/LGPL — read for
  behaviour, don't vendor), the Figma UI kit (CC-BY-NC-ND — reference only). The provenance table is
  what keeps this honest.

---

## 8. Decisions needed from you

1. **Browser:** embed a real OS webview (`wry`, real browsing, new dependency, per-platform WebView
   requirements) or keep a simulated renderer for period authenticity? *Recommendation: `wry`, with
   the simulated renderer as the fallback.*
2. **How much must be *real*?** e.g. should Command Prompt/Regedit/Disk Defragmenter actually work, or
   is an accurate shell with a working subset acceptable? *Recommendation: real for the tools people
   will try (cmd, Task Manager, Calculator, WordPad), shell for the deep system tools.*
3. **Office 2003 in or out?** It roughly doubles the project.
4. **Asset policy:** strategy 1 or 2 from §7.

---

## 9. Immediate next steps (start here)

1. Fix the Start-menu row hit boxes, then re-verify against the reference screenshot.
2. Start the repo: `git init`, `.gitignore` (`.reference/`, `reference-screens/`, `target/`),
   `docs/PROVENANCE.md`, README with build/run/asset instructions.
3. Add `--screenshot <name>` and build `tools/shot.sh` + `tools/diff.py`.
4. Capture `reference-screens/` for the desktop, Start menu, My Computer, Paint, Notepad and the
   power modal, and record baseline scores.
5. Turn `AppKind` into the `AppDef` registry and add the empty `settings.rs` / `vfs.rs` / `dialogs.rs`
   modules, so Phases 1–3 can proceed in parallel without churn.
