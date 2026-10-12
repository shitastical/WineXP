# Feature gaps — XP features this Rust project has not yet implemented

A survey of the Windows XP recreation demos listed below, cross-checked against what this repo has
**already** built (see `docs/PROVENANCE.md`, `PLAN.md` and `src/`), producing a checklist of genuine
**gaps** with the exact source URL/file each can be ported from.

**Priority** is coarse and relative to "a Windows-XP-accurate desktop a user will notice as wrong":
`high` = a first-five-minutes item that users will miss; `medium` = visible/expected but secondary;
`low` = niche, cosmetic, or only reachable through deep menus.

> Nothing here is invented: every row names a source that was actually fetched, or (where marked
> *id not fetched*) an app id that a fetched registry file (`control.js` / `shell.js`) proves exists.
> See **Not retrievable** at the end for sources that returned no usable content.

---

## 0. What is already implemented (so it is NOT listed below)

- **Shell:** Bliss desktop, draggable icons + marquee select, Luna window manager (drag/resize/min/max/close), taskbar + task buttons + tray + clock, Start menu (2 columns, submenus), window menu bar, power modal ("Turn Off Computer"/"Log Off Windows"), "at risk" balloon, desktop icons.
- **Apps:** Internet Explorer (simulated renderer), My Computer, Notepad, Paint, Minesweeper, Winamp, ErrorBox, Control Panel *folder* (classic view; applets listed but inert), User Accounts applet, Command Prompt (`cmd.js` port), Run dialog, Explorer (Wine `C:\`), Wine Configuration, 3D Pinball (recreation).
- **Screens:** Award BIOS POST, boot-device menu, reinstall-confirm / "Installing Windows Updates…", logon layout, XP "starting" screen.
- **Infra:** `vfs.rs` (Wine `C:\` bridge), `accounts.rs` (JSON user store), `settings.rs` (data dir), `wine.rs` (host Wine launch).

Sources surveyed: `xp.quenq.com` (Reborn XP), `wieslawsoltes.github.io/WinXp` + its repo, `pranx.com/windows-xp-simulator`, `win32.run` + its repo, `react-xp.jamiepates.com` + `Cyanoxide/react-xp`, `quenq.com/apps/reborn-xp`.

---

## 1. Built-in apps

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **Solitaire (Klondike)** | Card game; classic draw-one/three, scoring, timed. | `https://xp.quenq.com/js/apps/sol.js` (embeds `https://98.js.org/programs/js-solitaire/`); also `wieslawsoltes/WinXp` `apps-games` `solitaire` | high |
| **WordPad** | Rich-text editor (New/Open/Save/Save As, Undo/Redo, Cut/Copy/Paste, RTF/TXT/HTML save). | `https://xp.quenq.com/js/apps/wordpad.js` | high |
| ~~**Calculator**~~ | *Implemented* (`src/apps/calculator.rs`): XP Standard view — key grid, key colours, display, memory register, real arithmetic. Scientific view is still not faked. | `wieslawsoltes/WinXp` `apps-editors` `calculator` (id not fetched from Reborn XP) | done |
| ~~**Task Manager**~~ | *Implemented* (`src/apps/taskmgr.rs`): all five tabs, menus, status bar; Processes and Performance read the host's real `/proc`. End Task/Switch To report rather than kill. | `https://xp.quenq.com/js/apps/taskmgr.js` | done |
| **Add or Remove Programs** | 4 views: Change or Remove / Add New / Windows Components / Set Program Access; "Remove" flow. | `https://xp.quenq.com/js/apps/appwiz.js` | high |
| **Windows Picture and Fax Viewer** | Image zoom-in/out/fit/actual, rotate CW/CCW, prev/next, delete, slideshow, pan. | `https://xp.quenq.com/js/apps/imgviewer.js` | medium |
| **Help and Support Center** | Chromed help shell (Back/Forward/Home/Favorites) over an HTML help pane. | `https://xp.quenq.com/js/apps/help.js` | medium |
| **Registry Editor (regedit)** | Hive tree (`HKLM`/`HKCU`/`HKUSERS`), value list, inline value editing (REG_SZ/DWORD/bool), status path. | `https://xp.quenq.com/js/apps/regedit.js` | medium |
| ~~**Character Map**~~ | *Implemented* (`src/apps/charmap.rs`): font picker, glyph grid, `U+XXXX: Name` status bar. Copy fills `Characters to copy:` but does not touch a system clipboard. | `wieslawsoltes/WinXp` `apps-editors` `charmap` | done |
| **Outlook Express** | Local mail folders, compose/draft/reply/forward, contacts, MIME `.eml` export. Start menu "E-mail" row is inert. | `wieslawsoltes/WinXp` `apps-internet` `outlook`; `README.md` re mail behaviour | medium |
| **Notepad — missing sub-features** | Word Wrap, Zoom In/Out/Restore, **Font…** dialog (family/style/size + sample), Time/Date insert, Status Bar, dirty `*` title. Our `notepad.rs` is the ShizukuIchi basic port. | `https://xp.quenq.com/js/apps/notepad.js` | medium |
| **Paint — missing sub-features** | **Set as Wallpaper** (center/tile/stretch), Open-with-Paint hand-off, Save-As filter list, dirty-state Save prompt, undo via jspaint. | `https://xp.quenq.com/js/apps/mspaint.js` | medium |
| **Address Book** | Editable contact list with import/export. | `wieslawsoltes/WinXp` `apps-editors` `addressbook` | low |
| **Sound Recorder** | Microphone capture, waveform, trim/effects, WAV export. Start menu lists it (inert). | `wieslawsoltes/WinXp` `apps-internet` `soundrecorder` | low |
| **About Windows (`winver`)** | The "About Windows" version dialog; opened by "About Notepad/Paint/IE" menus upstream. | `https://xp.quenq.com/js/apps/iexplore.js`, `notepad.js`, `mspaint.js` (`apps.load("winver")`) | low |
| **Tour Windows XP (`xptour`)** | First-logon guided tour; also the tray "Take a tour" balloon target. | `https://xp.quenq.com/js/shell.js` (tray + balloon app id `xptour`) | low |
| **App Store / Web App Installer** | In-OS installer that unpacks apps into `C:/Program Files` and creates shortcuts. | `https://quenq.com/apps/reborn-xp/`; shell.js install flow | low |

## 2. Control Panel applets

The Control Panel *folder* exists (`src/apps/controlpanel.rs`) but every applet except **User Accounts** is inert.
The authoritative applet registry is Reborn XP's `control.js` `cplItems[]` (`id`, icon, label) — each id
maps to `https://xp.quenq.com/js/apps/<id>.js`. Where a Reborn XP file was not fetched it is marked
*id not fetched*; the **id and label are proven** by `control.js`.

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **Display Properties** (`desk`) | Themes / Desktop / Screen Saver / Appearance / Settings tabs with live monitor preview. The flagship dialog. | `https://xp.quenq.com/js/apps/control.js` → `apps.load('desk')` (*id not fetched*); `wieslawsoltes/WinXp` `apps-system` `display` | high |
| ~~**Date and Time**~~ (`timedate`) | *Implemented* (`src/apps/timedate.rs`, and Control Panel ▸ Date and Time opens it): painted clock, real calendar, time zones, Internet Time. Not yet opened by the tray clock. | `control.js` → `apps.load('timedate')` (*id not fetched*); `wieslawsoltes/WinXp` `datetime`; tray hook in `shell.js` | done |
| **System Properties** (`systemprops`) | General / Computer Name / Hardware (Device Manager) / Advanced / System Restore tabs. | `control.js` → `apps.load('systemprops')` (*id not fetched*); `wieslawsoltes/WinXp` `systemproperties` | high |
| **Folder Options** (`folderopt`/`folderprops`) | General / View / File Types tabs (hidden files, extensions, single-click, classic folders). | `control.js` → `apps.load('folderopt')` (*id not fetched*); `wieslawsoltes/WinXp` `folderoptions` | high |
| **Taskbar and Start Menu** (`startprops`/`startmenu`) | Taskbar tab (lock, auto-hide, keep on top, Quick Launch, customize notifications) + Start Menu tab (XP vs classic). | `control.js` → `apps.load('startprops')`; taskbar context menu in `https://xp.quenq.com/js/shell.js`; `wieslawsoltes/WinXp` `taskbarproperties` | high |
| **Sounds and Audio Devices** (`soundprops`/`sound`) | Volume / Sounds (sound scheme & per-event sounds) / Audio / Hardware tabs. | `control.js` → `apps.load('soundprops')` (*id not fetched*); sound-event map in `shell.js` (`_systemEventMap`); `wieslawsoltes/WinXp` `sounds` | medium |
| **Mouse** | Buttons / Pointers / Pointer Options (cursor schemes, speed). | `wieslawsoltes/WinXp` `apps-system` `mouse` | medium |
| **Keyboard** | Speed, cursor-blink rate, input locales. | `wieslawsoltes/WinXp` `apps-system` `keyboardsettings` | medium |
| **Internet Options** | General / Security / Privacy / Content / Connections / Programs / Advanced. | `wieslawsoltes/WinXp` `apps-system` `internetoptions` | medium |
| **Fonts** (`fonts`) | Explorer view of `C:\WINDOWS\Fonts` with previews, install/delete. | `control.js` (`explorer.open('C:/WINDOWS/Fonts')`); `wieslawsoltes/WinXp` `fonts` | medium |
| **Network Connections** | LAN/dial-up connection list, status, repair. | `control.js`; `wieslawsoltes/WinXp` `networkconnections` | low |
| **Printers and Faxes** | Printer list, add-printer wizard, queue. | `wieslawsoltes/WinXp` `apps-system` `printers` | low |
| **Administrative Tools** | Shortcut folder to Services/Event Viewer/etc. | `wieslawsoltes/WinXp` `apps-system` `administrative` | low |
| **Scheduled Tasks** (`tasks`) | Task list + scheduled launcher. | `control.js` → `C:/WINDOWS/Tasks`; `wieslawsoltes/WinXp` `scheduledtasks` | low |
| **Power Options / Regional & Language / Accessibility Options** | Remaining standard applets (start menu + `gist` applet lists in `PLAN.md §2.2`). | `wieslawsoltes/WinXp` `apps-system`; applet registry `https://xp.quenq.com/js/apps/control.js` | low |
| **System Restore / Backup / Disk Cleanup / Disk Defragmenter / System Information / Event Viewer / Services / Hardware (Device Manager) / Disk Properties / Recycle Bin properties / Screensaver / About XP** | The "system tools" family, all currently inert Start-menu rows. | `wieslawsoltes/WinXp` `apps-system` (`backup`, `diskcleanup`, `diskdefrag`, `systeminfo`, `hardware`, `eventviewer`, `services`, `diskproperties`, `recycleproperties`, `aboutxp`) + `apps-accessibility` (`screensaver`) | low |

## 3. Shell / desktop behaviour

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **Right-click context menus** | Desktop, icon, taskbar, Start entry, title-bar and tray menus (currently almost none exist). | `https://xp.quenq.com/js/shell.js` (desktop + taskbar + startbtn menus, `contextmenu` markup); `wieslawsoltes/WinXp` (`context menus`, `virtual clipboard`) | high |
| **Quick Launch toolbar + Show Desktop** | Taskbar Quick Launch: Desktop, Explorer, WMP buttons; `minimizeAllWindows()` Show Desktop. | `https://xp.quenq.com/js/shell.js` (`<quicklaunch>`, `wm.minimizeAllWindows()`) | high |
| **Taskbar context menu** | Toolbars submenu (Links/Desktop/Quick Launch/New Toolbar), Cascade/Tile, Task Manager, Lock the Taskbar, Properties. | `https://xp.quenq.com/js/shell.js` (`<taskarea>` menu) | medium |
| **Clickable tray icons + Customize Notifications** | Volume / security / network tray icons with double-click launch; "Adjust Date/Time", "Customize Notifications…". | `https://xp.quenq.com/js/shell.js` (`<traycontain>`, `<trayarea>` menu) | medium |
| **Queued / dismissable notification balloons** | Balloon popups with title/icon/close and a click-through action (today only the one "at risk" balloon). | `https://xp.quenq.com/js/shell.js` (`<balloonpopup>`); base balloon in ShizukuIchi `components/Balloon` | medium |
| **Classic Start menu** | The alternative single-column Start menu, toggled by a setting. | `https://xp.quenq.com/js/shell.js` (`classicStartmenuTemplate`, `classicStart` toggle); `regedit.js` `ClassicStartMenu` | medium |
| **Theme switching (Luna Blue/Silver/Olive, Royale, Zune, Watercolor, Classic)** | Full visual theme/colour-scheme system. | `https://quenq.com/apps/reborn-xp/` ("Every Official Theme…"); `shell.js` `themehandler.changeTheme` | medium |
| **Taskbar toggles** | Auto-hide, keep-on-top, hide clock, show Quick Launch, classic folders, show hidden files, show extensions, full path in title. | `https://xp.quenq.com/js/apps/regedit.js` (`toggleStorage` keys) + `shell.js` (`toggleStorage` map + `handleToggle`) | medium |
| **Multiple logons / Switch User / fast user switching** | Keep several users logged in, switch between desktops, per-user settings, "N programs running" on logon tile. | `https://xp.quenq.com/js/shell.js` (`logonUser`, `switchUser`, `_loggedOnUsers`, `switchLogon`) | medium |
| **Log Off Windows dialog** | "Switch User / Log Off / Cancel" plus "There are running programs. Quit anyway?". | `https://xp.quenq.com/js/shell.js` (`logoffUser`, overlay options) | medium |
| **Desktop drag-and-drop upload + Arrange/New menu** | Drag files from the host onto the desktop; desktop menu Arrange Icons / Refresh / New (Folder, Text, Bitmap) / Paste. | `https://xp.quenq.com/js/shell.js` (`desktopTemplate`, `addDropHandlers`) | low |
| **Wallpaper change from Paint / Desktop** | "Set as Wallpaper" with center/tile/stretch and persistence. | `https://xp.quenq.com/js/apps/mspaint.js` (`setCanvasAsWallpaper`) | low |
| **Window open/close/minimise animations** | Scale/fade window animations. | `https://quenq.com/apps/reborn-xp/` ("window animations") | low |

## 4. Boot / logon

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **BSOD (blue screen of death) screen** | Full-screen Stop error with editable `STOP:` code, offending file and message; click/key to reboot. | `https://xp.quenq.com/js/shell.js` (`showBSOD`, template text) | medium |
| **OOBE / "Welcome" first-run wizard (`msoobe`)** | First-boot setup that creates the initial user accounts, then hands to the logon screen. | `https://xp.quenq.com/js/shell.js` (`runOobe`, `apps.load('msoobe')`) | medium |
| **Password-protected logon** | Logon tile password mode, wrong-password error, per-user password check. | `https://xp.quenq.com/js/shell.js` (`logonUser` password path); landing text in `https://xp.quenq.com/css/` ("Type your password") | medium |
| **Interactive BIOS SETUP / Boot Menu** | "Press DEL to enter SETUP, F12 to Boot Menu", ↑/↓ selection, ENTER to boot, ESC to restart. | `https://xp.quenq.com/css/` landing text (Quenq Sys BIOS); our `screens/bootmenu.rs` already draws the selector | low |
| **Stand By screen** | Dim/standby overlay that wakes on input. | `https://xp.quenq.com/js/shell.js` (`enterStandby`, `exitStandby`, `scene_standby`) | low |
| **F8 Advanced Startup Options / Safe Mode** | F8 boot menu (Safe Mode, Last Known Good, etc.). | `PLAN.md §2.2` → `Sparsh712/WinXP-Portfolio`; referenced in BSOD text in `https://xp.quenq.com/css/` | low |
| ~~**"Installing Windows Updates…" progress**~~ | *(already implemented — `screens/install.rs`)* — listed only so the boot/logon set reads complete. | `https://xp.quenq.com/js/shell.js` (`scene_updatescreen_xp`) | — |
| ~~**Windows Update**~~ | *Implemented* (`src/apps/windowsupdate.rs`): the Windows Update v5 page over the real engine — checks this repository's GitHub releases, installs only a higher version, reads back the audit log. | Windows Update v5 markup + Reborn XP `scene_updatescreen_xp` | done |

## 5. Games

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **Minesweeper upgrades** | Sprite-based board, Game menu Beginner/Intermediate/Expert (+custom), chording, flag→question-mark cycle, smiley face states, 3-digit mine/timer LEDs, safe first click. Our `minesweeper.rs` is the simpler ShizukuIchi version. | `https://xp.quenq.com/js/apps/winmine.js` | medium |
| **FreeCell** | Card game with numbered deals, pause; Start menu row is inert. | `wieslawsoltes/WinXp` `apps-games` `freecell` | medium |
| **Spider Solitaire** | 1/2/4-suit Spider; Start menu row is inert. | `wieslawsoltes/WinXp` `apps-games` `spider` | medium |
| **Hearts** | Four-player Hearts with AI opponents; Start menu row is inert. | `wieslawsoltes/WinXp` `apps-games` `hearts` | low |
| **Internet games (Backgammon, Checkers, Hearts, Reversi, Spades)** | The MSN "Internet Games" set; all inert Start-menu rows. | Start-menu rows in `src/startmenu.rs`; feature reference `https://quenq.com/apps/reborn-xp/` (Games) | low |
| **Tetris** | Bonus game included by the prank simulator. | `https://pranx.com/windows-xp-simulator/` | low |
| **Pinball depth** | Ranks, mission/campaign progress, tilt and full Space Cadet rules — our `pinball.rs` is a recreation, not the full table. | `wieslawsoltes/WinXp` `apps-pinball` (`COMPATIBILITY.md` notes its own table is original too) | low |

## 6. Media

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **Windows Media Player 8** | Full player chrome: WMP8/9/10 skins, Now Playing / Skin Chooser / Media Guide / Copy from CD / Library / Radio / Burn nav, playlist, seek & volume bars, color-cycling, mini-player, visualizations (Bars, Ocean Mist, Fire Storm, Cover Art), fullscreen. | `https://xp.quenq.com/js/apps/wmp.js` | high |
| **Volume Control (`sndvol32`)** | Per-channel volume sliders + Mute, from the tray speaker double-click. | `https://xp.quenq.com/js/apps/sndvol32.js`; tray hook in `shell.js` | high |
| **System sound schemes / event sounds** | Named scheme + per-event WAV (Startup, Logon, Logoff, Error, Ding, Balloon, Hardware Insert/Remove, Recycle Empty…). | `https://xp.quenq.com/js/shell.js` (`_systemEventMap`, `playSystemSound`, `soundSchemes`) | medium |
| **Sound Recorder** | See §1 (Media category duplicate) — microphone capture and WAV export. | `wieslawsoltes/WinXp` `apps-internet` `soundrecorder` | medium |
| **Windows Picture and Fax Viewer** | See §1 — image viewer with zoom/rotate/slideshow. | `https://xp.quenq.com/js/apps/imgviewer.js` | medium |

## 7. Networking

| Feature | What it does | Source URL / JS file | Priority |
|---|---|---|---|
| **Internet Explorer — real browsing & navigation** | Address bar + Go, Back/Forward/Stop/Reload/Home, History/Search/Favorites, status-bar zone ("Internet"), progress bar, plus a **Wayback-Machine** path for period-accurate pages. Our IE is a simulated renderer with no navigation. | `https://xp.quenq.com/js/apps/iexplore.js` (nav/status/zone); `Cyanoxide/react-xp` `README.md` (Wayback Machine implementation) | high |
| **Internet Options applet** | Security/Privacy/Content/Connections/Programs/Advanced pages. | `wieslawsoltes/WinXp` `apps-system` `internetoptions` | medium |
| **Outlook Express** | Local mail client (also §1). | `wieslawsoltes/WinXp` `apps-internet` `outlook` | medium |
| **Set Program Access and Defaults** | Default-program / file-association manager (side view of Add or Remove Programs). | `https://xp.quenq.com/js/apps/appwiz.js` (`set-access` view, file-association list) | low |
| **MSN / Windows Messenger** | Instant-messenger app (local/loopback only — be honest in the UI). Reborn XP has id `msnmsgr`; Start menu row is inert. | `https://xp.quenq.com/js/shell.js` (`msnmsgr` app id); `PLAN.md §2.6` (msn topics) | low |
| **New Connection Wizard / Wireless Network Setup Wizard / Network Setup Wizard** | Connection-setup wizards; all inert Start-menu rows. | Rows in `src/startmenu.rs`; feature reference `https://quenq.com/apps/reborn-xp/` | low |
| **HyperTerminal** | Serial/terminal emulator; inert Start-menu row. | Row in `src/startmenu.rs` (`Communications`) | low |

---

## Not retrievable

These sources returned **no usable content** (an SPA/landing fallback or a non-JS response), so no
feature claims are made from them. Where a repo/README was available for the same project, that is
noted and used instead.

| Requested URL | Result | Substitute used |
|---|---|---|
| `https://xp.quenq.com/css/` | Returned the Reborn XP **landing-page HTML**, not the theme CSS. No CSS files retrievable. | None — theme/metadata claims taken from `js/shell.js` and the `quenq.com/apps/reborn-xp/` page. |
| `https://xp.quenq.com/js/apps/` (directory) and `https://xp.quenq.com/js/apps/index.js` | Same SPA fallback — no app-registry file. | App ids recovered from the fetched `js/shell.js` and `js/apps/control.js`. |
| `https://wieslawsoltes.github.io/WinXp/` | Blazor/WASM SPA; **only** the text "Windows XP Professional" rendered. | Repo `README.md` + `docs/COMPATIBILITY.md` (`wieslawsoltes/WinXp`) — the 56-app inventory. |
| `https://win32.run/` | Landing page renders **only** the BIOS POST text; no desktop/app list. | Repo `ducbao414/win32.run` README (VFS, programs, XP-style File Picker/Saver, 3rd-party programs). |
| `https://react-xp.jamiepates.com/` | SPA shell (`<div id="root">`) only. | Asset chunk map in `/assets/index-DhgSVa4D.js` (FileExplorer, InternetExplorer, Notepad, Paint, Run, Settings, Solitaire, XPScrollbars) + `Cyanoxide/react-xp` README. |

### Fetched but *not* individually retrieved per-app files

`control.js` and `shell.js` prove these Reborn XP app ids exist and are launched via
`apps.load('<id>')`, but the individual `js/apps/<id>.js` files were **not** fetched in this survey
(their contents are therefore not described, only the id/label from the registry):
`desk.js`, `folderopt.js`, `folderprops.js`, `startprops.js`, `systemprops.js`, `timedate.js`,
`soundprops.js`, `install.js`, `tasks.js`, `msoobe.js`, `xptour.js`, `winver.js`, `msnmsgr.js`,
`outlook.js`, `appstore.js`, and the `apps-load('tbd')` "not built" placeholder.

### Files that *were* fetched successfully (content used)

`js/apps/`: `sol.js`, `regedit.js`, `winmine.js`, `taskmgr.js`, `appwiz.js`, `sndvol32.js`,
`mspaint.js`, `notepad.js`, `iexplore.js`, `wmp.js`, `imgviewer.js`, `help.js`, `control.js`,
plus `wordpad.js` and `js/shell.js`. Also `https://quenq.com/apps/reborn-xp/`,
`https://pranx.com/windows-xp-simulator/`, and the GitHub README/COMPATIBILITY sources above.

---

## Notes / assumptions

- "Already implemented" is read from `docs/PROVENANCE.md`, `PLAN.md` and the module list in `src/`
  (`src/apps/*`, `src/screens/*`, `src/{wm,taskbar,startmenu,menu,modal,balloon,icons,vfs,accounts,settings,wine}.rs`).
  `PLAN.md` §1 lists features as *needed*; the actual `src/` tree was used to decide what is present now.
- Reborn XP is a closed-source simulator; its UI text/metrics are used as a transcription reference
  (as already done for `useraccounts.rs`). Verify Quenq's terms before redistribution, per `PROVENANCE.md`.
- `wieslawsoltes/WinXp` explicitly states it is **not pixel-perfect**; treat it as a *feature
  checklist*, not a visual authority (this matches `PLAN.md §2.2`). Many of its "apps" are property
  dialogs / info tools rather than full applications.
- Priority is for "XP-accurate desktop feel", not effort.
