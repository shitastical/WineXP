# Provenance

Every module's look, layout and behaviour is traceable to a source. This file records the mapping
from our Rust modules to the upstream project/file they were transcribed or ported from, with the
upstream licence.

**Policy:** only open-source projects are used as sources. Microsoft's assets (icons, wallpaper,
sounds, fonts, `PINBALL.DAT`/`PINBALL.MID`) are **not** committed; they are fetched to
`assets-local/` at runtime or supplied by the user. No leaked Windows source is used.

## Shell and applications (base: ShizukuIchi/winXP)

| Our module | Source repo | Source path | Licence |
|---|---|---|---|
| `src/wm.rs` | ShizukuIchi/winXP | `src/WinXP/Windows/index.js`, `HeaderButtons.js` | MIT |
| `src/taskbar.rs` | ShizukuIchi/winXP | `src/WinXP/Footer/index.js` | MIT |
| `src/startmenu.rs` | ShizukuIchi/winXP | `src/WinXP/Footer/FooterMenu(.Data).js` | MIT |
| `src/menu.rs` | ShizukuIchi/winXP | `src/components/{WindowDropDowns,SubMenu}` | MIT |
| `src/modal.rs` | ShizukuIchi/winXP | `src/WinXP/Modal/index.js` | MIT |
| `src/balloon.rs` | ShizukuIchi/winXP | `src/components/Balloon/index.js` | MIT |
| `src/icons.rs` | ShizukuIchi/winXP | `src/WinXP/Icons/index.js` | MIT |
| `src/apps/mycomputer.rs` | ShizukuIchi/winXP | `src/WinXP/apps/MyComputer/*` | MIT |
| `src/apps/minesweeper.rs` | ShizukuIchi/winXP | `src/WinXP/apps/Minesweeper/*` | MIT |
| `src/apps/notepad.rs` | ShizukuIchi/winXP | `src/WinXP/apps/Notepad/*` | MIT |
| `src/apps/internetexplorer.rs` | ShizukuIchi/winXP | `src/WinXP/apps/InternetExplorer/*`, `src/components/Google/*` | MIT |
| `src/apps/{paint,winamp,errorbox}.rs` | ShizukuIchi/winXP | `src/WinXP/apps/*` | MIT |

## Planned / in-progress (see the plan)

| Our module | Source repo | Licence | Notes |
|---|---|---|---|
| `src/pinball/**` | k4zmu2a/SpaceCadetPinball | MIT | Pure-Rust re-implementation of the engine; loads user-supplied `PINBALL.DAT`. No C++ compiled or linked. |
| `src/screens/{post,bootmenu,starting,shutdown}.rs` | ducbao414/win32.run | MIT | Boot/POST/boot-menu/starting sequence (`boot_manager.svelte`, `xp/starting.svelte`). Svelte read only; no JS vendored. |
| `src/screens/welcome.rs` | aduncandev/web-xp, Fy644/Windows-XP-login, Alena0490/Windows-XP | see repos | Welcome/logon screen (win32.run has none). Verify each repo's licence before copying values. |
| `src/vfs.rs`, `src/dialogs.rs`, `src/controls/**` | ducbao414/win32.run | MIT | VFS, common Open/Save dialogs, shared controls. |
| `src/screens/display_properties.rs` | ducbao414/win32.run, wieslawsoltes/WinXp | MIT / see repo | Display Properties; verify `wieslawsoltes/WinXp` licence. |

## Assets

| Asset | Owner | Handling |
|---|---|---|
| XP icons, Bliss wallpaper, boot logos, sounds, Tahoma/Noto fonts | Microsoft | Not committed. `assets-local/` drop-in or fetched at runtime. |
| `PINBALL.DAT`, `PINBALL.MID` | Microsoft | Not committed. Carved from the user-fetched resource blob at runtime. |
| `gm.sf2` soundfont | see upstream | Provenance unverified; treat as user-supplied until confirmed. |

## Ported from Reborn XP (`xp.quenq.com`, Quenq)

| Our module | Source | Notes |
|---|---|---|
| `src/screens/post.rs` | `index.html` `#post-screen`, `css/index.css`, `js/index.js` | BIOS POST: header (`Award Modular BIOS v4.51PG`, `Quenq Sys BIOS v.3.2.2`), the 10-line typewriter at 100 ms/line + 1500 ms hold, the `.epa-logo` Energy Star mark (136×84), 80/60 px padding, `#ccc` on black, `vga8x14` at 22 px/1.2 |
| `src/screens/bootmenu.rs` | `index.html` `#bootloader`, `css/index.css`, `js/index.js` | Two options, ↑/↓ wrap, ENTER/ESC, the 10→0 countdown that stops on any key, inverse-video selection, uppercase `.title` headings, `ul` indent 40 px / `li` padding 4×8 px |
| `src/screens/install.rs` | `index.html` `#reinstall-confirm`, `js/index.js` | "Reinstall Windows XP": navy confirmation (title/paragraph copy, list, click-or-keyboard), then `Formatting virtual disks...` (2000 ms) and `Restarting computer...` (1500 ms) before the reload |
| `src/screens/oobe.rs` | `js/apps/msoobe.js`, `app.msoobe` rules in `css/applayouts.css`, `css/scenes.css` | The OOBE wizard: intro splash, `Welcome to Microsoft Windows` → `Who will use this computer?` → `Thank you!`, the five username inputs with their labels/hints/defaults, `a()` name validation, Back/Next/Finish footer, and the final `Installing system files…` / `Copying additional files…` pass |
| `src/screens/updates.rs` | `scene_updatescreen_xp` in `index.html` + `css/scenes.css` | The 80/auto/96 grid, `#5a7edc` page, `#00309c` bands with 2 px gradient rules, radial `#94b4ef` glow, `#update-xp-text` heading |
| `src/screens/login.rs` | `css/scenes.css` (`scene_logon`) | Logon layout/metrics (band `#5a7edc`, 50/50 split, usericon 48px, `#fe0` hover border, `#7f9db9` password box) |
| `src/apps/useraccounts.rs` | `js/apps/userprops.js` + `.userprops-cpl` in `css/applayouts.css` | Windows XP "User Accounts" applet: pages, exact strings, colours, validation, 5-account limit |
| `assets/cpl/*`, `assets/users/*`, `assets/login/*`, `assets/oobe/*` | `res/…` | Icons, user pictures (the 23 OOBE `.bmp`s + `guest.bmp`), logos, and the OOBE Back/Next sprite sheets |
| `assets/fonts/Web437_IBM_VGA_8x14.ttf` | `res/fonts/Web437_IBM_VGA_8x14.woff` | The BIOS/boot screens' `vga8x14` pixel face, converted woff→ttf with fontTools |
| `assets/oobe/intro/*.jpg` | `res/anim/intro.mp4` | The OOBE intro video's frames (ffmpeg, 2 fps), played back at the source cadence since egui cannot decode MP4 |

Licence: Reborn XP is a simulator by Project Quenq; UI text/metrics are transcribed for a faithful clone. Verify Quenq's terms before redistribution.
