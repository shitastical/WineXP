# Specs from code-lead XP repositories

Mined, port-ready detail (window geometry, menu/tab lists, literal button strings, colour
values, layout and algorithm notes) from nine XP-recreation projects, for apps this Rust
project does not yet have. Companion to `docs/FEATURE-GAPS.md`; the "Fills gap" lines point
at the gap-table rows that each source can satisfy.

All values below were read from the repositories as fetched on the date of writing. Where a
repo returned no usable source (docs/art only), that is stated explicitly.

**Fetch status at a glance**

| # | Repo | Usable source? |
|---|---|---|
| 1 | qqkookie/XPAccApps | Yes — Win32 `.rc` resources + C sources |
| 2 | wieslawsoltes/WinXp | Yes — ES-module JS + CSS |
| 3 | Phault/winOS | **No — 404, not retrievable** |
| 4 | codedit/InternetExploder | Thin — C# shell + bundled WebKit binaries |
| 5 | rmellis/Windows-Media-Player-9-WebApp-Clone | Yes — `style.css` + README |
| 6 | Ingan121/ModernActiveDesktop | Yes — HTML/JS/CSS applets |
| 7 | AdamWhiteHat/TotalWinUICustomization | Yes — C# WinForms designer sources |
| 8 | Yosi2377/WinXpDesktop | Yes — single-file `index.html` (strings) |
| 9 | aduncandev/web-xp | Yes — React sources + README |

---

## 1. qqkookie/XPAccApps — XP Accessories (ReactOS/Wine port)

- **URL:** https://github.com/qqkookie/XPAccApps
- **XP apps implemented:** `xcalc` (Calculator), `xtaskmgr` (Task Manager), `xnotepad`
  (Notepad, multi-tab), `xwordpad` (WordPad/RTF), `xmspaint` (Paint), `xmine` /
  `MISC/winemine` (Minesweeper), `xclock` (Clock), `xclipbrd` (Clipboard Viewer),
  `xchcp`, `charmap`, `fontview`, `winhlp32`.
- **Tech stack:** C / C++ / Win32 resources, CMake + Ninja (Visual Studio 2022), Windows-only.
  High-DPI aware (144 dpi). String tables translated in `lang/<locale>.rc`; dialogs use
  8 pt "MS Shell Dlg". Settings stored in `~/AppData/Local/XPAccApp.ini` or `HKCU`.
- **Licence:** repo is **GPL-2.0** (`LICENSE`); many files are Wine/ReactOS and carry
  LGPL-2.1-or-later / GPL-2.0+ headers (e.g. `notepad.rc`, `winemine.rc`).
- **Exact source files used:** `calc/lang/en-US.rc`, `MISC/notepad/notepad.rc`,
  `MISC/wordpad_ROS/lang/en-US.rc`, `MISC/winemine/{winemine.rc,main.h}`,
  `clock/clock.rc`, `clipbrd/{clipbrd.rc,lang/en-US.rc}`.

### Calculator (`xcalc`) — dialog units, layout, labels
Dialogs are measured in **dialog units (DLU)**; standard calculator is `169 x 128`,
scientific `316 x 163`, conversion `320 x 130`. Each key is **24 x 18 DLU**; columns are on a
26-DLU pitch.

- **Standard layout:** output `RTEXT` at (5,1,159,14); row `Back | CE | C` at y=23,
  w=41/42; memory column x=5 (`MC`,`MR`,`MS`,`M+`); digit columns x=36/62/88; operator column
  x=114; function column x=140 (`Sqrt`,`%`,`1/x`,`=`). Memory indicator cell `CTEXT` 18x16.
- **Key label strings (exact):** `Back`, `CE`, `C`, `MC`, `MR`, `MS`, `M+`, `Sqrt`, `%`,
  `1/x`, `=`, `+/-`, `,`, digits `0`–`9`, ops `/ * - +`.
- **Scientific extras:** `Sta`, `Ave`, `Sum`, `s`, `Dat`, `F-E`, `dms`, `sin`, `cos`, `tan`,
  `(`, `)`, `Exp`, `x^y`, `x^2`, `x^3`, `ln`, `log`, `n!`, `pi`, `Mod`, `Or`, `Lsh`, `And`,
  `Xor`, `Not`, `Int`, hex `A`–`F`; radio groups
  `Hex/Dec/Oct/Bin`, `Degrees/Radians/Gradians`, `Qword/Dword/Word/Byte`; checkboxes
  `Inv`, `Hyp`. `Sqrt` is left as a hidden duplicate button in the scientific dialog.
- **Menus:** `View` → `Standard`, `Scientific`, `Conversion`, sep, `Hex F5`, `Decimal F6`,
  `Octal F7`, `Binary F8`, sep, `Degrees F2`, `Radians F3`, `Gradians F4`, sep,
  `Group digits`; `Misc` → `Copy Ctrl+C`, `Paste Ctrl+V`, sep, `Help Topics`,
  `About ReactOS Calc`.
- **Statistics box:** `163 x 85`; list `156 x 40`; buttons `RET`, `LOAD`, `CD`, `CAD`;
  counter `n=0`.
- **Conversion tab:** `Category:`, `Convert from:`, `Convert to:` combo boxes (140 wide) +
  `Convert` button (76 x 17). 14 categories with long unit tables — `Angle`, `Area`,
  `Consumption (engines)`, `Currencies`, `Energy`, `Force`, `Lengths`, `Power`, `Pressure`,
  `Temperature`, `Time`, `Velocity`, `Volume`, `Weights` (e.g. area includes `Acres (Scots)`,
  `Tsubo (坪 Japan)`, `Pyeong (坪 Korea)`; length includes `Barleycorns`, `Chi/Ja (尺)`,
  `Kabiet`, `Zhang (丈)`). Currency list includes `Crypto Bitcoin`, `Crypto Ethereum`.

### Minesweeper (`MISC/winemine`) — board metrics & dialogs
From `main.h` / `winemine.rc`:

```
BEGINNER   : 9 x 9  , 10 mines
ADVANCED   : 16 x 16, 40 mines      (menu label "Intermediate")
EXPERT     : 30 x 16, 99 mines
MAX        : 30 x 24
MINE cell  : 16 x 16 px   LED : 12 x 23 px   FACE : 24 x 24 px
margins    : board 5 px (H+V), bottom 20 px
```

- **Menus:** `Game` → `New F2`, sep, `Question Marks`, sep, `Beginner`, `Intermediate`,
  `Expert`, `Custom...`, sep, `Fastest Times`, sep, `Exit`; `Help` → `About WineMine`.
- **Flag cycle:** `NORMAL → QUESTION → FLAG → COMPLETE`. Face states:
  `SPRESS/COOL/DEAD/OOH/SMILE`; game status `WAITING/PLAYING/GAMEOVER/WON`.
- **Dialogs:** Fastest Times `170 x 80` (rows Beginner/Intermediate/Expert + `Reset Results`);
  Congratulations `160 x 60` ("Please enter your name"); Custom Game `140 x 100`
  (`Rows` / `Columns` / `Mines`). Confirm text `All results will be lost. Are you sure?`
  with caption `Reset Results`.

### Notepad (`MISC/notepad/notepad.rc`)
- **Menus:** `File` (`New Ctrl+N`, `Open... Ctrl+O`, `Save Ctrl+S`, `Save as...`, `Print...
  Ctrl+P`, `Page Setup...`, `Printer Setup...`, `Exit`); `Edit` (`Undo Ctrl+Z`, `Cut/Copy/
  Paste`, `Delete Del`, `Select all Ctrl+A`, `Time/Date F5`, `Wrap long lines`, `Font...`);
  `Search` (`Search... Ctrl+F`, `Search next F3`, `Replace... Ctrl+H`, `Go To... Ctrl+G`);
  `View` (`Status Bar`); `Help`.
- **Strings:** status `Ln %ld, Col %ld`; untitled `Untitled`; encodings `Unicode (UTF-16)`,
  `Unicode (UTF-16 big-endian)`, `Unicode (UTF-8)`; filters `All files (*.*)`,
  `Text files (*.txt)`.
- **Dialogs:** Page Setup `240 x 95` with group `Margins (millimeters)` and Left/Top/Right/
  Bottom fields + OK/Cancel/`&Help`; Go To Line `130 x 55` (`Go To` / `Cancel`).
- **Accelerators:** Ctrl A/C/F/H/N/O/P/S/V/X/Z/G, `VK_BACK`+Alt = Undo, F1/F3/F5,
  Ctrl+Insert = Copy, Shift+Insert = Paste.

### WordPad (`MISC/wordpad_ROS/lang/en-US.rc`)
- **Menus:** `File` (`New... Ctrl+N`, `Open... Ctrl+O`, `Save Ctrl+S`, `Save As...`,
  `Print... Ctrl+P`, `Print preview`, `Page Setup...`, `Exit`); `Edit` (`Undo Ctrl+Z`,
  `Redo Ctrl+Y`, `Cut/Copy/Paste/Clear Del`, `Select All Ctrl+A`, `Find... Ctrl+F`,
  `Find Next F3`, `Replace... Ctrl+H`, `Read-only`, `Modified`, `Extras` submenu →
  `Selection info`, `Character format`, `Def. char format`, `Paragraph format`, `Get text`);
  `View` (`Toolbar`, `Format Bar`, `Ruler`, `Status Bar`, `Options...`); `Insert`
  (`Date and time...`); `Format` (`Font...`, `Bullet points`, `Paragraph...`, `Tabs...`,
  `Background` → `System Ctrl+1`, `Pale yellow Ctrl+2`); `Help` (`About Wine Wordpad`).
- **Colour popup (exact list):** `Black, Maroon, Green, Olive, Navy, Purple, Teal, Gray,
  Silver, Red, Lime, Yellow, Blue, Fuchsia, Aqua, White, Automatic`.
- **File-type labels:** `All documents (*.*)`, `Text documents (*.txt)`,
  `Unicode text document (*.txt)`, `Rich text format (*.rtf)`, `Printer files (*.prn)`.
  New-document types: `Rich text document`, `Text document`, `Unicode text document`.
- **Options dialog (`IDD_FORMATOPTS`, `280 x 110`):** group `Line wrapping` → radio
  `&No line wrapping` / `Wrap text by the &window border` / `Wrap text by the &margin`;
  group `Toolbars` → `Toolbar` / `Format Bar` / `Ruler` / `Status Bar`.
- **Other dialogs:** Paragraph format `220 x 110` (Indentation Left/Right/First line,
  Alignment; alignment strings `Left/Right/Center`); Tabs `200 x 110` (`&Add`, `&Remove`,
  `Remove al&l`); Date and time `130 x 80`; New `140 x 80`. Strings:
  `Save changes to '%s'?`, `You have chosen to save in plain text format, which will cause
  all formatting to be lost. Are you sure that you wish to do this?`.

### Clock (`clock/clock.rc`) — `xclock`
Pop-up menu (title `XClock`): `Analog`, `Digital`, `Seconds`, `24 Hours`, sep,
`No Title Bar`, `Always on Top`, `Dark Color`, sep, `About XClock`, `Exit Alt+X`.
(Commented-out but present in source: `Date on Titlebar`, `Font...`.)

### Clipboard Viewer (`clipbrd`) — `xclipbrd`
- **Menus:** `Action` → `Open...`, `Save As...`, `Delete Del`,
  `Select Font` submenu (`Tahoma 11`, `Lucida Console 11`, `Malgun Gothic 12`),
  `Help Topics`, `About`, `Exit Alt+X`; `Display` → `Automatic`.
- **Strings:** `Clipboard Viewer`, `Clipboard Element`,
  `Clear contents of the Clipboard?` / `Clear Clipboard`; filters
  `Clipboard files (*.clp)`.
- **Format-name table to copy verbatim:** `Unknown Format, Text, Bitmap, OEM Text,
  Unicode Text, DIB Bitmap, Locale Data, Enhanced Metafile, Metafile, Color Palette,
  DIB Bitmap (Version 5), Symbolic Link Format, Data Interchange Format, Drop Data`.

**Fills gaps:** Calculator, WordPad, Clipboard Viewer, Notepad sub-features, Minesweeper
upgrades, Clock.

---

## 2. wieslawsoltes/WinXp — 56-app browser desktop

- **URL:** https://github.com/wieslawsoltes/WinXp (live: wieslawsoltes.github.io/WinXp)
- **XP apps implemented:** 56 registered definitions in 11 packages — Explorer/shell;
  `apps-editors` (notepad, wordpad, calculator, charmap, addressbook); `apps-games`
  (minesweeper, solitaire, freecell, spider, hearts); `apps-graphics` (paint,
  pictureviewer); `apps-internet` (ie, outlook, mediaplayer, soundrecorder, cmd);
  `apps-pinball`; `apps-accessibility` (osk, magnifier, accessibility, screensaver);
  `apps-system` (33 dialogs — controlpanel, addremove, administrative, fonts, mouse,
  keyboardsettings, internetoptions, display, systemproperties, datetime, sounds,
  taskbarproperties, folderoptions, taskmanager, regedit, backup, diskcleanup, diskdefrag,
  run, search, networkconnections, users, systeminfo, hardware, eventviewer, services,
  scheduledtasks, recent, printers, diskproperties, recycleproperties, help, aboutxp).
- **Tech stack:** native ES-module HTML/CSS/JS, no build step; virtual filesystem over
  IndexedDB (localStorage fallback); WebGPU wallpaper renderer with WebGL2 + Canvas 2D
  fallbacks; Node built-in test runner.
- **Licence:** **no `LICENSE` file in the repo**; ships `THIRD_PARTY_NOTICES.md`,
  `wine-lgpl-2.1.txt` (Tahoma from Wine), `xp-css-license.txt`. Treat as
  third-party-licensed assets; code licence unstated.
- **Exact source files used:** `dist/packages/shell/theme.css`,
  `dist/packages/apps-editors/calculator.js`, `dist/packages/apps-system/settings.js`,
  `dist/packages/apps-system/index.js`, plus `docs/COMPATIBILITY.md`.

### Theme tokens (`shell/theme.css`) — copy these
```css
--xp-face:#ece9d8;  --xp-blue:#0054e3;  --xp-selection:#316ac5;  --xp-border:#7f9db9;
--taskbar-height:30px;  --xp-accent:#003c74;
--xp-title-top:#0997ff;  --xp-title-bottom:#0053ee;
font: Tahoma 11px;   button text 11px;
```
Scheme overrides: **Silver** `blue #9196a7, title #e7e8f4→#afb3c8, accent #5b6073`;
**Olive** `blue #7b882f, title #d1d5a4→#8c9c4a, accent #606e2a, selection #8a9864`;
**Classic** `blue #0a246a, title #0a246a→#a6caf0, face #d4d0c8`.

Reusable metric recipe: button `min 75x23`, 1px `#003c74`, radius 3, gradient
`#fff → #f8f7f3 20% → #f0efea 83% → #d8d6c3`, hover ring `inset 0 0 0 2px #f6cb70`,
primary ring `#a4c8f0`, disabled `#c9c7ba` on `#f5f4ed`. Menubar/status 22 px; menu item
22 px; popup `min-width 180px`; menu separator `margin:3px 2px 3px 25px`; toolbar 32 px;
tabs `radius 3 3 0 0`, selected `border-top:3px #ffc73c`; scrollbar 17 px.

### Calculator (window 285x337, scientific 545x405)
Keys: `MC MR MS M+ | Backspace CE C`, digit pad, `÷ × − + =`, `sqrt % 1/x`, `+/− .`; scientific
adds `Sta, ln/eˣ, log/10ˣ, ( ), sin/cos/tan (Inv/Hyp), x²/√x, xʸ, n!, π, Int/|x|, Mod, Ave,
Sum, s/σ, x³/∛x, Exp` and hex `A–F`. Menus: `Edit` (Copy Ctrl+C, Paste Ctrl+V, Evaluate
Expression... Ctrl+E), `View` (Standard, Scientific, Statistics...), `Operations`
(And, Or, Xor, Not, Left shift, Right shift), `Help`. Dialogs: Evaluate Expression width 430,
Statistics Box width 410. Status text `n statistical values · Degrees`.

### Control Panel (750x503, min 480x318)
Category list (exact): `Appearance and Themes`, `Network and Internet Connections`,
`Add or Remove Programs`, `Sounds, Speech, and Audio Devices`,
`Performance and Maintenance`, `Printers and Other Hardware`, `User Accounts`,
`Date, Time, Language, and Regional Options`, `Accessibility Options`.
Applet ids/labels: `display→Display`, `sounds→Sounds and Audio Devices`,
`networkconnections→Network Connections`, `systemproperties→System`, `users→User Accounts`,
`addremove→Add or Remove Programs`, `datetime→Date and Time`, `folderoptions→Folder Options`,
`taskbarproperties→Taskbar and Start Menu`, `printers→Printers and Faxes`,
`accessibility→Accessibility Options`, `scheduledtasks→Scheduled Tasks`, `fonts→Fonts`,
`mouse→Mouse`, `keyboardsettings→Keyboard`, `internetoptions→Internet Options`,
`administrative→Administrative Tools`. Sidebar buttons: `Switch to Classic View` /
`Switch to Category View`, `Windows Update / About`, `Help and Support`.

### Property dialogs — window sizes + tab lists (high value)
| Dialog | Size | Tabs / key labels |
|---|---|---|
| Display Properties | 435x494 (min 390x400) | Themes, Desktop, Screen Saver, Appearance, Settings |
| System Properties | 455x450 | General, Computer Name, Hardware, Advanced |
| Date and Time Properties | 452x378 | Date & Time, Time Zone, Internet Time |
| Sounds and Audio Devices Properties | 430x416 | Volume, Sounds, Audio |
| Taskbar and Start Menu Properties | 407x344 | Taskbar, Start Menu |
| Folder Options | 397x330 | General, File Types |
| Internet Properties | 420x329 | — (Home page, History) |
| Mouse Properties | 390x300 (fixed) | — |
| Keyboard Properties | 390x281 (fixed) | — |
| Add or Remove Programs | 648x431 | search box + Enable/Disable/Open |
| Administrative Tools | 564x344 | Task Manager, Event Viewer, Services, Registry Editor, System Information, Backup and Restore |
| Fonts | 590x399 | Font + Size selectors |

Display Properties specifics: wallpaper list `(None) / Bliss / Bliss · High Resolution`,
Position `Stretch / Center / Tile / Fill`, `Browse…` opens `My Pictures`; Appearance
`Windows and buttons:` = `Windows XP style / Silver / Olive Green / Windows Classic style`,
`Font:` = `Tahoma / Arial / Verdana`; Screen Saver = `Starfield / Mystify / 3D Text`,
`Enable screen saver`, `Wait:` 1–120 minutes; Settings shows `Color quality: Highest (32 bit)`.
Preview renders a mini window titled `My Computer` with the text `Welcome to Windows XP`.

Folder Options file-type table: `TXT→Notepad`, `HTML→Internet Explorer`,
`PNG / JPG / SVG→Picture and Fax Viewer`, `MP3 / WAV / MP4→Windows Media Player`.

Date & Time: clock face has 12 ticks (30° apart); hand transforms
`hour = (h%12*30 + m/2)°`, `minute = m*6°`, `second = s*6°`; calendar header `S M T W T F S`.
Sounds: scheme `Windows Default / No Sounds`; program-event list `Default Beep, Exclamation,
Critical Stop, New Mail Notification, Start Windows`; test sound arpeggio
`523.25, 659.25, 783.99, 1046.5 Hz`.

**Fills gaps:** Display Properties, Date and Time, System Properties, Folder Options,
Taskbar/Start Menu properties, Sounds, Mouse, Keyboard, Internet Options, Fonts, Add or
Remove Programs, Calculator, Control Panel category view.

---

## 3. Phault/winOS — **not retrievable**

- **URL:** https://github.com/Phault/winOS
- **Status:** every fetch returned **HTTP 404** (`github.com/Phault/winOS`,
  `/tree/master/scripts`, and the `api.github.com/repos/Phault/winOS` endpoint). The repo
  appears to have been deleted, renamed or made private since a web index snapshot that
  described it as *"Windows XP as a web page"* and showed a `scripts/` tree and a Releases
  page. **No source, README, geometry or colour values could be obtained**, so no porting
  detail is claimed from it here. Substitute: the Explorer / My Computer needs this repo
  would have covered are already covered by §2 (`shell/explorer.js`, `explorer.css`) and §9
  (web-xp Explorer with task pane, folders tree, drive letters, XP view modes).

---

## 4. codedit/InternetExploder — IE6 chrome (C#/WebKit shell)

- **URL:** https://github.com/codedit/InternetExploder
- **XP app implemented:** Internet Explorer 6 ("a complete rewrite powered by WebKit").
- **Tech stack:** C# / .NET **WinForms**, hosting bundled **WebKit** (`WebKit.dll`,
  `WebKit.Interop.dll`, `JavaScriptCore.dll`, `CFLite.dll`, `SQLite3.dll`) — native binaries
  shipped in `Resources/`, copied to `bin/Debug|Release` before build. Windows-only.
- **Licence:** **GPL-3.0** (`LICENSE`). README states all artwork is © Microsoft.
- **Usable source?** Thin. The managed code is `Internet Explorer 6/main.Designer.cs`
  (~20 KB), `main.cs`, `CrashView.*`, `Program.cs`. There is **no managed IE6 menu/chrome
  definition** — the README's TODO confirms the top menu bar, downloads, favorites and
  history are **not implemented**. So the only concretely portable artefacts are assets and
  a feature checklist:
  - **Toolbar bitmap set** (normal + `_hover` pairs): `Back`, `Forward`, `Stop`, `Refresh`,
    `Home`, `Search`, `Favorites`, `Channels`, `Fullscreen`, `Print`, `Open`, `Go`; plus
    `download.avi` (download progress animation), `error.htm`, `splash.bmp`, `WinLogo.png`,
    `pagerror.gif`, `throbber`.
  - **Feature TODO (a gap checklist for any IE6 recreation):** downloads, favorites, history,
    every item that belongs in the top menu bar, pop-up windows, real error pages, crash
    reporter, stored settings/homepage.

---

## 5. rmellis/Windows-Media-Player-9-WebApp-Clone — WMP 9

- **URL:** https://github.com/rmellis/Windows-Media-Player-9-WebApp-Clone (live: wmp9.ywa.app)
- **XP app implemented:** Windows Media Player 9 Series.
- **Tech stack:** vanilla JS/HTML/CSS PWA; Web Audio API (`BiquadFilterNode`, `PannerNode`
  HRTF, `GainNode`, `AnalyserNode`), hls.js, FFmpeg.wasm (MKV → MP4 remux), jsmediatags,
  MediaRecorder, SpeechRecognition.
- **Licence:** **GPL-2.0**.
- **Exact source used:** `style.css` (all colours/geometry), `README.md` (nav, shortcuts, skins).

### Geometry
- Grid: **145px left taskbar | 1fr centre | 270px right playlist**, rows **1fr / 75px**.
  Shell: `border 1px #142a5c; border-radius 8px; padding 4px`.
- Top bar height **38px**; bottom transport height **75px**; status bar **24px**.
- Transport: play button **34px** round, secondary round buttons **26px**, seek bar **12px**
  with a 28x14 thumb; volume is a `clip-path` triangle (60x14).

### Colour values (`:root`)
```css
--wmp9-frame-top:#7b9ccf;  --wmp9-frame-bot:#4667a4;
--wmp9-light-blue-top:#eff4fb; --wmp9-light-blue-mid:#dbe7f5; --wmp9-light-blue-bot:#a4c0e4;
--wmp9-taskbar-text:#1a326c; --wmp9-playlist-bg:#485a97;
--wmp9-silver-top:#f2f6fb; --wmp9-silver-mid:#cfdcf0; --wmp9-silver-line:#aabfe0; --wmp9-silver-bot:#c3d2e8;
--wmp9-border-light:#e8edf4; --wmp9-green-glow:#00ee00;
--wmp9-border-blue:#142a5c; --wmp9-border-soft:#8ba1c6; --wmp9-hover-blue:#3399ff; --wmp9-button-icon:#2b4263;
/* status bar bg #31363a ; top-bar swoosh gradient #6184ba → #4a6cb0 ; seek fill #d6efc2 → #8cc060 */
```

### Menus / navigation (exact)
- Top bar menus: **File, View, Play, Tools, Help** (app icon at left).
- Left taskbar nav: **Now Playing, Media Guide, Copy from CD, Media Library, Radio Tuner,
  Copy to CD/Device, Premium Services, Skin Chooser**.
- **10-band graphic EQ, bands 60 Hz–16 kHz**, via `BiquadFilterNode`s; SRS-WOW spatial audio
  via `PannerNode` HRTF; digital gain via `GainNode` up to **200 %**.
- Console bar: track title/KHz, visualizer effect buttons, seek bar flanked by time-skip
  buttons. Right Info Center: playlist + total running time + colour-customisable mini
  sound-wave.

### 13 skins (name → window size / shape)
`Default (Royal)` (classic blue) · `Miniplayer` 320x400 · `Compact` 450x130 ·
`Christmas Bauble` 400x420 (circle) · `Xmas Wreath` 450x450 (circle, red border) ·
`Diamond Prism` 520x520 (`polygon(50% 0,100% 50%,50% 100%,0 50%)`) · `Vertical Pillar`
240x450 · `BMO Console` 320x480 (D-pad + animated face) · `Floating Capsule` 350x480 ·
`Hex Grid` 480x530 (hexagon) · `Cyber Shield` 420x520 (`polygon(0 0,100% 0,80% 100%,20% 100%)`) ·
`Organic Blob` 420x420 (`border-radius:40% 60% 70% 30% / 40% 50% 60% 50%`) ·
`Rattleballs` 420x520.

### Keyboard shortcuts (port verbatim)
`Space` Play/Pause · `Ctrl+S` Stop · `Ctrl+P` Play · `Ctrl+F/B` Next/Prev · `Ctrl+H` Shuffle ·
`Ctrl+T` Repeat · `Ctrl+O` Open · `Ctrl+U` Open URL · `Ctrl+E` Equalizer ·
`Ctrl+L` Info Center/Playlist · `F1` Help · `F8` Mute · `F9/F10` Volume −/+ · `Shift+S`
Capture frame · `←/→` Seek ∓10 s · `Enter` Apply skin · `Esc` return to Default.

**Fills gaps:** Windows Media Player (nav tabs, seek/volume bars, visualizations, skins,
equalizer), volume control styling, media keyboard shortcuts.

---

## 6. Ingan121/ModernActiveDesktop — Display Properties & Internet Options

- **URL:** https://github.com/Ingan121/ModernActiveDesktop (live: madesktop.ingan121.com)
- **XP apps implemented:** Active Desktop / Win98 desktop. Bundled apps: Channel Bar, JSPaint,
  Solitaire, Clock, Visualizer (+ Lyrics), ChannelViewer (IE4 with IE6 elements), Calculator,
  **Configurator** (a remake of the *Display Properties* and *Internet Options* control-panel
  applets — "some of them are pretty pixel-perfect"), On-Screen Keyboard, Colour Picker.
- **Tech stack:** pure HTML/CSS/JS, Chromium-124 optimised; optional Electron "system plugin";
  Wallpaper Engine / Lively hosts; IndexedDB/localStorage for configs, schemes and images;
  consumes 98.css / XP.css / 7.css.
- **Licence:** **MIT** (Microsoft-provided assets excluded from the MIT grant).
- **Exact source used:** `README.md`, `apps/inetcpl/general.html`, `apps/madconf/appearance.html`,
  tree listings for `apps/madconf`, `js/` (DeskMover.js, DeskSettings.js, MadMenu.js, confmgr.js),
  `apps/channelviewer/stylesheets/xpcss4mad.css`.

### Display Properties — `ModernActiveDesktop Properties` (`config.html` / `apps/madconf`)
- **Appearance tab** (`appearance.html`): `Scheme:` selector with the full scheme preset list:
  `Current Scheme, Windows Classic (98), Windows Standard (2000 / Me), Windows XP style,
  Windows Aero, Windows Basic`, then system schemes
  (`Brick, Desert, Eggplant, Lilac, Maple, Marine, Plum, Pumpkin, Rainy Day,
  Red White and Blue, Rose, Slate, Spruce, Storm, Teal, Wheat, Windose, Catppuccin Mocha,
  Arc-Dark Blur (Beta), VGUI`) and all XP variants
  (`Windows XP Olive Green, Windows XP Silver, Windows XP Royale, Windows XP Royale Noir,
  Windows XP Zune, Windows XP Embedded, Watercolor, Windows XP Fallback, Windows Aero Fallback,
  Windows Classic (95 / NT4), Windows Classic (98SE), Windows Classic (16 colors)`),
  Plus! 95/98, Windows 1/2/3, high-contrast, etc.
- **Item list (exact strings to reuse):** `3D Object, 3D Light, 3D Shadow, Active Title Bar,
  Active Window Border, Application Background, Caption Buttons, Desktop, Disabled Item,
  Hyperlink, Inactive Title Bar, Inactive Window Border, Menu, Menu Bar (Flat), Message Box,
  Palette Title, Scrollbar, Selected Item, ToolTip, Window`.
- **Fonts offered:** `Pixelated MS Sans Serif`, `Fixedsys Excelsior` (+ Custom Size + Colour).
- **Controls:** `Save As...`, `Delete`, `Item:`, `Size:`, `Color:`, `Color 2:`, `Font:`,
  `Effects` tab, `OK / Cancel / Apply`.
- **Hidden export hooks:** typing `!copycss`, `!copyjson`, `!copyreg`, `!copytheme` in the
  scheme-save dialog exports CSS / JSON / `HKCU\Control Panel\Colors` registry text /
  Windows `*.theme` file respectively.

### Internet Options — `apps/inetcpl/general.html` (verbatim labels)
```
Home page  — "You can change which page to use for your home page."
             Address:  [ field ]   Use current   Use default   Use blank
Miscellaneous "Open pages in..."
             ( ) New browser tab / window
             ( ) New ChannelViewer window (System plugin)
             ( ) ChannelViewer (deprecated)
             ( ) System default browser
Advanced     [ ] Show favicon in the address bar
             [ ] Play system sounds
             [ ] Enable JavaScript
             [ ] Launch Channels in full screen window
             [ ] Enable classic styling
Buttons      OK   Cancel   Apply
```

### ChannelViewer (IE4/IE6 chrome) — toolbar asset contract
Each toolbar glyph ships in three states — `X.png`, `X_hover.png`, `X_disabled.png`
(`Back`, `Forward`, `Stop`, `Refresh`, `Home`, `Favorites`, `Channels`, `Fullscreen`,
`Print`, `Open`, `Go`) plus `throbber.{png,webp}` and a `zone_internet.png` /
`zone_local.png` status-zone icon pair. XP-variant glyphs live under `images/xp/`.

**Fills gaps:** Display Properties (Themes/Appearance + Effects), Internet Options
(General/Security/Privacy/Content/Connections/Programs/Advanced), theme/scheme switching,
Hyperlink/Menu colours, colour-scheme import/export.

---

## 7. AdamWhiteHat/TotalWinUICustomization — "Control Freak" (Display Properties)

- **URL:** https://github.com/AdamWhiteHat/TotalWinUICustomization
- **XP app implemented:** the Windows XP era **Display Properties** dialog (colours, fonts,
  theme), recreating the classic dialog removed after Windows 7.
- **Tech stack:** C# / .NET **WinForms** (Windows-only). Writes `HKCU\Control Panel\Colors`
  **and** generates/installs a `.theme` file as a workaround for Win10/11 ignoring the raw
  registry values.
- **Licence:** **none declared** — the repo tree contains no `LICENSE` file; README makes no
  licence statement. Treat as all-rights-reserved unless clarified.
- **Exact source used:** `Source/TotalWinUICustomization/MainForm.Designer.cs` (and file tree:
  `WindowsUIMockupControl.*`, `ColorHelper.cs`, `ThemeHelper.cs`, `RegistryHelper.cs`,
  `Types.cs`, `UIElementAssociation.cs`).

### Geometry & controls
- Form `ClientSize 570 x 518`, `FormBorderStyle Fixed3D`, title
  **"Control Freak - Total Windows Customization"**.
- Colour rows: `Item:` combo (269x21), then `Color 1:` and `Color 2:` buttons — each a
  54x23 button whose nested swatch panel is 29x19, drawn with a `│▾` drop glyph in Consolas 9.
- Font rows: `Font:` combo (271x21, autocomplete), `Size:` combo (47x21) with items in
  descending order **`72,48,36,28,26,24,22,20,18,16,14,12,11,10,9,8`** (text `9`), a bold
  toggle `B` and italic toggle `/` (22x19 each), and a `Color:` button like the swatch buttons.
- Footer: link `"{0} pending changes..."` (DodgerBlue/Blue), buttons `Discard changes` and
  `Apply changes...`.

### XP-era system-colour palette (mockup control defaults — port directly)
```
ActiveTitle       rgb(153,180,209)  #99b4d1   gradient  rgb(185,209,234) #b9d1ea
InactiveTitle     rgb(191,205,219)  #bfcddb   gradient  rgb(215,228,242) #d7e4f2
Hilight / MenuHighlight rgb(0,120,212) #0078d4
HotTracking       rgb(0,102,204)
ButtonFace        #f0f0f0     MenuBar #f0f0f0     Window #ffffff
InfoWindow        rgb(255,255,225) #ffffe1     GrayText rgb(109,109,109)
Scrollbar         rgb(200,200,200)             Desktop #000000
ActiveBorder Gray   InactiveBorder Silver
```

**Fills gaps:** Display Properties → Appearance (item/colour/font editors, "pending changes"
apply model), window-metric concepts used by the Rust theme system.

---

## 8. Yosi2377/WinXpDesktop — single-file XP simulator

- **URL:** https://github.com/Yosi2377/WinXpDesktop
- **XP apps implemented (one `index.html`):** Notepad, Calculator (with `M+ M− MR MC`),
  Paint (Canvas: brush/rect/circle/eraser, colour picker, brush size), Minesweeper, Solitaire
  (drag-and-drop, stock/waste/foundation/tableau), Snake, Tic-Tac-Toe, Windows Media Player,
  Google Chrome, Internet Explorer (iframe + URL bar), PSP Emulator, My Computer,
  My Documents, Recycle Bin, Control Panel (appearance/themes, display, sound, system info,
  storage, date/time), Outlook Express, Task Manager, Sticky Notes, Calendar, Photo Viewer,
  Command Prompt, File Explorer, System Volume, Pong, Breakout, 2048, Monitor, Clipboard,
  Pomodoro, Quick Links, Code Snippets, Daily Planner, Todo List.
- **Tech stack:** pure HTML/CSS/JS single file, no dependencies; Canvas 2D, Web Audio API,
  `localStorage`. Israel timezone.
- **Licence:** **none declared** (no LICENSE in the repo tree).
- **Exact source used:** `README.md` + `index.html` (desktop icon labels, Start menu and
  context-menu strings).

### Verbatim strings to reuse
- **Desktop / Start menu entries (exact):** `My Computer, Recycle Bin, My Documents, Notepad,
  Calculator, Paint, Minesweeper, Solitaire, Media Player, PSP EMU, Google Chrome, Snake,
  Tic-Tac-Toe, Task Manager, Sticky Notes, Calendar, Photo Viewer, Command Prompt,
  File Explorer, System Volume, Pong, Breakout, 2048, Monitor, Clipboard, Pomodoro,
  Quick Links, Code Snippets, Daily Planner`; Start menu header `Start`, `Add App`, clock
  `12:00`, `User`.
- **Context menu (right-click / Menu button):** `Refresh, Toggle On Top, Set Window Size...,
  Change Theme..., Move Menu Button, Change Wallpaper..., About, Minimize Window,
  Reset Menu Position, Toggle Clock Format, Close Window, Menu`.
- **Power:** `Log Off`, `Turn Off Computer`.
- **Themes (6):** `Classic Blue` (default), `Olive Green`, `Silver`, `Luna`, `Royale`, `Zune`.
- **Behaviour constants:** screensaver activates after **5 minutes** idle; clock 12/24 h
  toggle with full-date tooltip; windows fade-in/minimise animations; double-click title
  maximises.

**Fills gaps:** Start-menu/context-menu string set, theme list, screensaver timer, Control
Panel applet grouping — a cheap transcription reference for shell text.

---

## 9. aduncandev/web-xp — React XP desktop (fork of ShizukuIchi/winXP)

- **URL:** https://github.com/aduncandev/web-xp (live: webxp.net)
- **XP apps implemented:** Calculator (Standard + Scientific), Command Prompt (`dir, copy,
  tree, attrib, ipconfig, ping, tasklist, taskkill, shutdown, start` …), Guest Book,
  Internet Explorer (address bar/history/back-forward, iframe), Minesweeper (3 difficulties,
  flags, chording, timer, mine counter), Notepad (File/Edit/Format menus, word wrap, cursor
  tracking), Paint (jspaint-derived, opens/saves real image files), Klondike Solitaire
  (draw-1/draw-3, Standard/Vegas scoring), Windows Explorer (task pane, folders tree, address
  bar, view modes), Windows Picture and Fax Viewer, WordPad (real `.rtf`), Winamp 2.x via
  **webamp**, Windows Media Player (music/video/picture + playlists + library), 3D Pinball
  (98.js.org WASM port), Voltorb Flip, PictoChat, webXP Tour, The XP Shop.
- **Tech stack:** React 18 (hooks + `useReducer`), styled-components, Vite; IndexedDB-backed
  VFS; optional Node 22 guest-book backend using built-in `node:sqlite`.
- **Licence:** **MIT** for the code (© 2019 Shizuku Yang winXP + © 2025-26 Aaron Duncan);
  Microsoft/Nintendo art & personal content explicitly excluded from the grant.
- **Exact source used:** `README.md`; source tree `src/` (`App.jsx`, `WinXP/`, `components/`,
  `context/`, `hooks/`, `index.css`, `xpArt.js`, `privacyNotice.js`).

### Port-worthy feature/geometry notes
- **Shell:** Start menu with pinned programs + most-used tracking + nested submenus mirroring
  real XP; taskbar running-app buttons, **Quick Launch**, tray clock + volume; desktop icons
  with click-to-select and rubber-band drag-select; window focus / z-order; global volume+mute.
- **Screensavers:** `3D Pipes`, `3D Text`, `Flying Objects` (three.js).
- **BSOD:** triggered by any unhandled JS error, with the real error text placed in the
  `STOP:` code.
- **Control Panel working applets:** `Display Properties`, `System Properties`,
  `User Accounts`, `Folder Options`.
- **Task Manager:** Applications tab reflects the real open windows; classic animated
  Performance graphs.
- **Recycle Bin:** really holds deleted files, restores to origin, empties.
- **Compressed (zipped) folders:** browse an archive in place, extraction wizard,
  password-protected zips; backup/restore wizard exports the whole machine to a zip.
- **Boot/login:** BIOS-style boot-device menu (click the CD to run setup) → XP setup wizard →
  animated boot → login; `?guest` and "Skip the startup screen" paths; fast user switching;
  Guest account toggle.
- **VFS:** IndexedDB with a schema version that triggers a recovery screen on format change;
  drive letters, file associations, Open With, per-user icon layouts.
- **Start menu power rows:** Log Off / Switch User, per-user files & settings.

**Fills gaps:** Explorer (task pane, folders tree, drive letters, XP view modes), Display/System
Properties + User Accounts + Folder Options applets working, Task Manager Applications tab,
compressed folders, screensavers, BSOD, Solitaire draw-1/3 & Vegas scoring, WordPad `.rtf`.

---

## Cross-repo quick-reference (what to grab from where)

| Need | Best concrete source |
|---|---|
| Calculator keypad grid + every key label | §1 `calc/lang/en-US.rc` (DLU geometry) + §2 `calculator.js` (px sizes) |
| Notepad / WordPad / Clipboard Viewer menu strings | §1 `.rc` files (verbatim) |
| Minesweeper board sizes & sprite metrics | §1 `MISC/winemine/main.h` |
| Control-Panel applet ids + category grouping | §2 `apps-system/index.js` |
| Display/System/Date-Time/Sounds/Folder dialog sizes & tabs | §2 `apps-system/settings.js` |
| Theme colour tokens (Blue/Silver/Olive/Classic) | §2 `shell/theme.css`; §7 system palette |
| Display Properties item/colour/font editor + schemes | §6 `appearance.html`; §7 `MainForm.Designer.cs` |
| Internet Options tab labels | §6 `inetcpl/general.html` |
| WMP9 layout grid, colours, skins, EQ, shortcuts | §5 `style.css` + README |
| IE6 toolbar glyph/state contract | §6 ChannelViewer assets; §4 `Resources/Static.Resources` |
| Shell Start/context-menu wording, screensaver timer | §8 `index.html` strings |
| Explorer + working Control Panel applets architecture | §9 README feature map |
| **Unavailable** | §3 Phault/winOS (404) |

## Notes / assumptions
- Only `docs/SPECS-CODELEADS.md` was written; no other file was modified, and no cargo/git
  command was run.
- Values are transcribed from the fetched revisions of each repo and are unverified against
  a real Windows XP installation; treat §2 self-declared "not pixel-perfect" caveat as
  applying to its metrics.
- Licences are as reported by each repository; §3, §7 and §8 have **no licence file**. Reuse
  of Microsoft-authored artwork (all repos) is the upstream projects' own disclaimer, not a
  grant that transfers.
