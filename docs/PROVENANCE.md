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
| `src/screens/{post,bootmenu,install}.rs` | `index.html` scenes + `css/scenes.css` | BIOS POST (`Award Modular BIOS v4.51PG`), boot selector, reinstall-confirm, "Installing Windows Updates…" |
| `src/screens/login.rs` | `css/scenes.css` (`scene_logon`) | Logon layout/metrics (band `#5a7edc`, 50/50 split, usericon 48px, `#fe0` hover border, `#7f9db9` password box) |
| `src/apps/useraccounts.rs` | `js/apps/userprops.js` + `.userprops-cpl` in `css/applayouts.css` | Windows XP "User Accounts" applet: pages, exact strings, colours, validation, 5-account limit |
| `assets/cpl/*`, `assets/users/*`, `assets/login/*` | `res/…` | Icons, user pictures, logos (Microsoft assets) |

Licence: Reborn XP is a simulator by Project Quenq; UI text/metrics are transcribed for a faithful clone. Verify Quenq's terms before redistribution.
