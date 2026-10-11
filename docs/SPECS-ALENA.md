# SPECS-ALENA.md — porting spec from the **Alena0490 / Windows-XP** clone & related demos

**Purpose.** A precise transcription reference (strings, layout, colours, behaviour) for Windows XP
features **this repo does not have yet** (`docs/FEATURE-GAPS.md` is the gap list; `PLAN.md` §1–§2 the
roadmap). Every value below was read out of a fetched source, or is explicitly flagged
**[unverified]**. Nothing here is invented.

**Sources that produced content** (all fetched, format `text`):

| Source | What came out |
|---|---|
| `https://github.com/Alena0490/Windows-XP` (README) | Complete feature list per app — used as a scope check |
| `raw.githubusercontent.com/Alena0490/Windows-XP/main/src/...` | Exact component source: strings, geometry, colours |
| `https://alena0490.github.io/Windows-XP/` | **SPA shell only** — `<div id="root"></div>`, no rendered UI text |
| `https://pranx.com/windows-xp-simulator/` | IE7-labelled window, program list, Run dialog text, Quick Launch/tray labels |
| `https://win32.run/` | **Award BIOS POST text only** — no desktop/app list |
| `https://pranx.com/pipes/` | 3D Pipes screensaver description (joints, easter eggs, controls) |

Alena's repo files read in full for this spec:

```
src/index.css                     (design tokens / Luna colour variables)
src/App.css                       (base window chrome metrics, bevels, buttons)
src/data/appData.tsx              (app inventory + sizes for `dir`)
src/components/taskbarAndStart/…  Footer.tsx, StartMenu.tsx, StartMenu.css, ErrorBubble.tsx, ErrorBubble.css
src/components/mediaPlayer/…      MediaPlayerApp.tsx, MediaPlayerMenu.tsx, VizDropdown.tsx, SkinChooser.tsx, colors.txt
src/components/plus/…             PlusMain.tsx, Plus.css
src/components/display-properties/DisplayProperties.tsx
src/components/ScreensaverOverlay.tsx
src/components/IE/…               IEWindow.tsx, IEMenu.tsx, data/IEData.tsx
src/components/files/data/…       FileManagerData.ts, rootCategories.ts
src/components/CharacterMap…      character-map/CharacterMap.tsx
src/components/Calculator/…       Calculator.tsx, CalculatorMenu.tsx
src/components/voice-recorder/…   VoiceRecorder.tsx, VoiceRecorderMenu.tsx
src/components/picture-viewer/PictureFaxViewer.tsx
src/components/outlook-express/OutlookExpress.tsx
src/components/Terminal/…         Terminal.tsx, TerminalWindow.tsx
src/components/runDialog/Run.tsx
public/WINDOWS/Resources/Themes/Screensavers/  (asset directory listing)
```

> **Licence note for porting.** Alena's *code* is MIT; the XP icons, sounds, wallpapers, fonts, WMP
> skins and the visualization `.mp4`s are Microsoft's. Transcribe **strings, geometry and behaviour**
> into our Rust, never the binaries (matches `PLAN.md` §7 and `docs/PROVENANCE.md`).

---

## 0. Shared Luna design tokens (from `src/index.css` + `src/App.css`)

These are the values every app below inherits. If our `theme.rs` disagrees, these are authoritative
for these screens.

### Typography

| Token | Value |
|---|---|
| `--font-size-xs/sm/md/lg/xl/2xl/3xl` | 9 / 10 / 11 / 12 / 13 / 14 / 22 px |
| Body font | `Tahoma, sans-serif`; Plus! uses `Arial, Helvetica, sans-serif` |
| Title-bar text | `'Trebuchet MS','Lucida Sans Unicode',Arial` 700 **11pt**, `#fff`, `text-shadow: 1px 1px 1px rgba(0,0,0,.45)` |
| Mnemonic | underlined first letter (`.mnemonic`) |

### Spacing / sizes

`--space-1..10` = **2, 3, 4, 6, 8, 10, 12, 14, 18, 24 px** ·
`--app-icon-size: 48px` · `--footer-height: 30px` (taskbar) · `--cell-size: 18px` ·
desktop icon 76×75 px in an `--space-10` (24 px) grid · `.desktop-item` 88 px wide, 11 px label.

### Radii

`--window-radius-sm: 3px`, `--window-radius-md: 6px`, `--window-radius-lg: 10px`,
`--window-radius-top-x: 10px`, `--window-radius-top-y: 5px`, `--start-radius: 8px 8px 0 0`.

### Luna blue (default `data-theme="luna"`)

| Role | Value |
|---|---|
| Title bar start / mid / end | `#0062EA` / `#14A5F4` / `#003398` |
| Title bar light (gradient tail) | `#9BCCFF` (`#7DB9FF` at 100 % in `.app-window`) |
| Taskbar top / bottom | `#3C81F3` / `#285BC5` |
| Start button green | `#4EB74E → #2D7D2D` (root default `#66CC33 → #267C08`) |
| Window surface | `#ECE9D8` (`.app-window` uses `#E6EAD8`) |
| Surface 2 | `#C9C7BA` |
| Lines light / mid / dark / deep | `#FFFFFF` / `#C9C7BA` / `#808080` / `#404040` |
| Menu hover (fill) / text | `#316AC5` / `#FFFFFF` |
| Selected bg | `#0066CC` |
| Disabled text | `#808080` (`#a1a192` in luna) |
| Tooltip | `#FFFFE1` |
| Start-menu right panel | `#D2ECFF → #B7D3FC` |
| Start-menu separator | `#C9C7BA` |
| Power panel | `#316AC5 → #0148B2` |
| Accent (hover glints) | `#FFCC00` / `#FF9933` / `#DE5C2F` |
| Menu glow / vignette | `rgba(255,255,255,.62)` → `rgba(43,63,99,.42)` |

**Silver** (`data-theme="silver"`, labelled **"Silver"** in Appearance): title `#A8A7BF → #BCBDCD → #66667E`,
taskbar `#CCCBDD → #6B758F`, surface `#E6E6ED`, selected `#A6A3B4`, insensitive `#737377`, separator `#ccccd9`.
**Olive Green** (`data-theme="homestead"`): title `#758D5E → #A7B580 → #5E764F`, taskbar `#E9F3C8 → #96A867`, surface `#ECE9D8`.

### Chrome metrics (`src/App.css`)

* `.title-bar` — `min-height: 26px`; Luna gradient `to bottom: #285BC5 50 %, #3C81F3 90 %, #003398 100 %`;
  inset white highlight `inset 0 2px 2px rgba(255,255,255,.55)`; height overridden to 26 px in
  `.calculator-window` with `font-size: 14px`.
* Window buttons: `.xp-title-control` **21.5 × 21.5 px** bitmaps — `Minimize.webp`, `Maximize.webp`,
  `Restore.webp`, `Exit.webp`, `LoginQuestion.webp` (help), plus `Luna/maximize-unfocused.png` when
  maximize is disabled. Classic fallback: 21 × 21 px, gradient `#9CC2FF → #2B74E3`, close `#FFB1A6 → #D94B31`.
* `.app-window` inactive: `--titlebar-gradient-inactive` (luna) `rgb(118,151,231) … rgb(171,186,227)` +
  3 × 3 px inset shadow; title text `color-mix(#D7E7FB 80 %, #fff)`, buttons `opacity: .7`.
* Fullscreen: `position: fixed; inset: 0 0 30px 0` (30 px = taskbar), `border-radius: 0`.
* Minimize: `opacity 0; transform: translateY(18px) scale(.96)` over 180 ms.
* Bevels: `.raised`/`.sunken` = 1 px `#FFF`/`#808080`; `.bevel-raised`/`.bevel-sunken` add
  `inset 2px 2px #F7F4E9` and `inset -2px -2px #716F64`.
* Buttons: `.luna-btn` **20 px tall, min-width 64 px**, 10 px text, border `.5px #003398`, radius 3 px,
  gradient `#FFF 0–35 % → #E6EAD8 100 %`; hover = amber inset glow (`#DE5C2F`/`#FF9933`).
* Checkbox / radio: **14 × 14 px** XP bitmaps (unchecked/hover/active/insensitive/checked, per theme).
* Tooltips: `#FFFFE1`, 1 px `#808080`, `2px 6px` padding, 11 px, `drop-shadow(2px 2px 1px rgba(0,0,0,.38))`.

---

## 1. Start menu — the full structure (`StartMenu.tsx` / `StartMenu.css`)

### Geometry

| Element | Value |
|---|---|
| Panel | **380 × 478 px**, anchored `bottom: 100%; left: 0` above the Start button, `z-index: 200`, radius `8px 8px 0 0`, `1px solid #003398` |
| User panel | `min-height: 74px`, padding `10px 12px 8px`, inset top highlight `inset 0 3px 2px rgba(255,255,255,.45)`, bottom 2.5 px 3-stop accent border (`#14A5F4` → `#DE5C2F`/`#FF9933`) |
| Avatar | **52 × 52 px**, 2 px `#FFF` border, 6 px radius, double drop shadow; label = user name (`Alena` upstream) — 22 px / 700 / `-0.3px`, white, `1px 1px 0 rgba(0,0,0,.55)` |
| Body grid | `50.5 %` left / `49.5 %` right |
| Left column | padding `6px 6px 0`, bg `#FFFFFF`; rows **min-height 42 px**, padding `4px 8px`, gap 12 px, font 12 px, icon **32 px** wide; two-line rows show 11 px grey sub-label |
| Right column | padding `10px 6px 6px 7px`, bg `#D2ECFF → #B7D3FC`, `border-left: 1px solid #808080` + `inset 1px 0 #FFF`; rows **min-height 21 px**, padding `2px 3px 2px 1px`, gap 6 px, font 11 px, colour `#003398`, icon **22 px**; "top" rows weight 600; submenu caret `▶` (`scaleY(1.6)`, 8 px) |
| `hr` | 1 px gradient `transparent → #C9C7BA → transparent`, margin `4px 6px` (left col) / `6px 8px 6px 22px` (right col) |
| Hover | fill `#0062EA → #14A5F4 → #316AC5`, border `#003398`, text white (sub-label too) |
| Power panel | `min-height 48px`, padding `8px 10px`, gap 14 px, border-top `1px rgba(255,255,255,.4)`, fill `#3C81F3 → #285BC5`; each button **min-height 28 px**, 12 px / 700 white + `1px 1px 1px rgba(0,0,0,.45)` shadow, 22 px icon |
| "All Programs" fly-out | **200 px** wide, `left: 80%; bottom: 0`, bg `#FFF`, border `1px #003398` + 4 px left `border-image` gradient `#316AC5 → #0148B2`, rows **min-height 32 px** (padding `4px 8px`, gap 8 px, 11 px, icon 22 px), `z-index: 400` |
| Nested fly-out | **200 px**, `top: 0; left: 100%`, `z-index: 500` |
| Disabled row | colour `#808080`, `cursor: not-allowed` (`Prohibited.webp`) |
| Empty row | italic, grey, literal text **`(Empty)`**, `pointer-events: none` |

### Rows (exact order, label, tooltip, action)

**Left column**

| # | Label | Tooltip (`data-tooltip`) | Click |
|---|---|---|---|
| 1 | `Internet` / sub `Internet Explorer` | *Finds and displays information and Web sites on the Internet.* | opens IE |
| 2 | `E-mail` / sub `Outlook Express` | *Send and receive electronic mail and newsgroup messages.* | opens Outlook Express |
| — | `<hr>` | | |
| 3 | `Minesweeper` | *Clear the minefield without hitting a mine.* | |
| 4 | `Calculator` | *Performs basic arithmetic, financial, and scientific calculations* | |
| 5 | `Command Prompt` | *Opens a Microsoft Disk Operating System (MS-DOS).* | |
| 6 | `Paint` | *Creates and edits drawings, and displays and edits scanned photos.* | |
| 7 | `Notepad` | *Creates and edits text files using basic text formatting.* | |
| 8 | `All Programs` ▸ | *Opens a list of all programs installed on your computer.* | hover opens fly-out |

**All Programs fly-out**

```
Set Program Access and Defaults        (disabled — two lines)
Windows Catalog                        → wayback microsoft.com catalog page
Windows Update                         (disabled)
──────────────────────────────────────
Accessories ▸
   Accessibility ▸  → On-Screen Keyboard
   Communications ▸ → (Empty)
   Entertainment ▸  → Windows Media Player · Volume Control · Sound Recorder
   System Tools ▸   → Character Map
   Notepad · WordPad · Paint · Calculator · Command Prompt
Games ▸        → Minesweeper · Solitaire
Startup ▸      → (Empty)
──────────────────────────────────────
Outlook Express · Internet Explorer · Display Properties · Windows Plus!
```

**Right column**

| # | Label | Tooltip | Click target |
|---|---|---|---|
| 1 | `My Documents` | *Opens the My Documents folder, where you can store letters, reports, notes, and other kinds of documents.* | FM `documents` |
| 2 | `My Recent Documents` ▸ | *(on the span)* *Opens a list of the documents you used most recently.* | fly-out lists `getRecentDocs()`; empty ⇒ `(Empty)` |
| 3 | `My Pictures` | *Opens the My Pictures folder, where you can store digital photos* | FM `pictures` |
| 4 | `My Music` | *Opens the My Music folder, where you can store music and other audio files* | FM `music` |
| 5 | `My Computer` | *Displays the drives, folders, and files on your computer* | FM root |
| — | `<hr>` | | |
| 6 | `Control Panel` ▸ | *Provides options for you to customize the appearance and functionality of your computer, add or remove programs, and set up network connections and user accounts* | FM `controlpanel`; fly-out: **Display Properties**, **Fonts**, **Printers and Faxes** ▸ `(Empty)`, **Administrative Tools** ▸ `(Empty)`, **Network Connections** ▸ `(Empty)`, **Scanners and Cameras** ▸ `(Empty)`, **Scheduled Tasks** ▸ `(Empty)` |
| 7 | `Set Program Access` / `and Defaults` | *Specifies default programs for web browsing, sending e-mail, playing media, and other activities.* | none (inert) |
| 8 | `Printers and Faxes` | *Displays installed printers and fax printers and helps you add new ones.* | FM `cp-printers` |
| — | `<hr>` | | |
| 9 | `Help and Support` | *Opens the Help and Support Center, where you can find tutorials, troubleshooting information, and support services.* | Help & Support Center |
| 10 | `Search` | *Opens the Search Companion window, where you can look for files, folders, printers, computers, or people.* | File Manager + Search (Rover) |
| 11 | `Run...` | *Opens a program, folder, document, or Web site.* | Run dialog |

**Power panel:** `Log Off` (tooltip *Log off or switch to another user account.*) and
`Turn Off Computer` (tooltip *Turn off, restart, hibernate, or place your computer on standby.*).

**Behaviour:** opening the menu plays the Start sound (or the Plus! theme's menu sound); clicking any
row also plays it. The menu stays open (annotated `key={isMenuOpen ? 'open' : 'closed'}` in Footer) and
closes on the first outside `mousedown`. There is **no** classic single-column Start menu in this clone.

> **Gap vs this repo:** we already have a 2-column Start menu, but not the `All Programs` fly-out
> contents above, the per-row tooltips, `My Recent Documents`, or the `Control Panel` fly-out.

---

## 2. Taskbar, Quick Launch & notification area (`Footer.tsx`)

* **Start button** — logo bitmap + literal text **`start`** (lower-case), tooltip **`Click here to begin.`**;
  **double-click = fullscreen**; single click toggles the Start menu.
* **Quick Launch** (on by default) — three `quick-launch__item` buttons, all activated on **double-click**:
  * IE — tooltip `Launch Internet Explorer Browser`
  * Show Desktop — tooltip `Show Desktop` (opens `…/desktop`)
  * Windows Media Player — two-line tip: title `Windows Media Player`, desc `Plays your digital media including music, videos, CDs, DVDs, and Internet Radio.`
* **Task buttons** — icon + label; File Manager and On-Screen Keyboard substitute a live title/icon.
  Click = bring to front; click on the active button = minimise.
* **Taskbar context menu** (exact items, in order): `Toolbars` ▸ (`Address` ☐, `Links` ☐, `Desktop` ☐,
  `Quick Launch` ☑ — all checkable), separator, `Cascade Windows` (disabled),
  `Tile Windows Horizontally` (disabled), `Tile Windows Vertically` (disabled), `Show the Desktop`,
  separator, `Task Manager` (disabled), separator, `Lock the Taskbar` (☑ disabled), `Properties` (disabled).
* **Toolbars** when enabled render inline: `Links` ▸ *Customize Links · Free Hotmail · Windows ·
  Windows Media*; `Desktop` ▸ *My Computer · My Documents · Recycle Bin*.
* **Tray** (right→left: clock, volume, security):
  * security icon — tooltip **`Your computer might be at risk`**
  * volume icon — tooltip **`Volume: NN%`**, or **`Muted`** when muted/0 %; **single click** opens the
    small volume pop-up after a 250 ms delay, **double-click** opens the full Volume Control
  * clock — `h:mm` (12-hour, locale), tooltip **`<Weekday>, <Month D>, 2003, <h:mm AM/PM>`** — note the
    **hard-coded year 2003** while weekday/month/day/time are real.

---

## 3. Security Center / firewall tray balloon (`ErrorBubble.tsx` + `.css`)

This is the **Security Center "at risk" balloon**. Behaviour: scheduled **5 000 ms** after the shell
mounts, plays the balloon sound, auto-hides at **15 000 ms**, and can be closed by its button.

Geometry (rem assumed 16 px):

| Property | Value |
|---|---|
| Position | `right: 3.1rem; bottom: 2.35rem` (≈ 50 px / 38 px, i.e. just above the tray), `z-index: 100` |
| Box | `width: 17.2rem` (≈ 275 px), `min-height: 4.2rem`, padding `.55rem 1.45rem .55rem .65rem`, bg `#FFFFE1`, `1px solid #808080`, radius 6 px, shadow `2px 2px 4px rgba(0,0,0,.25)` |
| Tail | two triangles (border trick): outer `.8rem` `#808080`, inner `.68rem` `#FFFFE1`, anchored `right: 1.45rem` |
| Header | 1 rem icon + **bold** title, `gap: .35rem`, `padding-right: .85rem` |
| Close | **1 × 1 rem**, `top: .25rem; right: .35rem`, `.5px solid #A1A192`, radius 2 px, bg `color-mix(#ECE9D8 50 %, white)`, 10 px bold `#808080`; hover → bg `#FFCC00`, text `#000` |
| Body | 11 px, `line-height: 1.35`, paragraphs indented `1.35rem`, 0.12 rem gaps |

**Exact strings**

```
Your computer might be at risk            ← header, bold, with shield/security icon
Antivirus software might not be installed
Click this balloon to fix this problem.
```

> **[unverified]** No **firewall**-specific balloon text exists in the fetched sources — the only
> Security Center string set found is the antivirus one above. The Windows Firewall balloon
> ("Your computer might be at risk — Windows Firewall is turned off. Click this balloon to fix this
> problem.") is **not** sourced here and must not be presented as transcribed.

---

## 4. Windows Media Player 8 chrome (`src/components/mediaPlayer/**`)

### Shell

* Window title: **`Windows Media Player`**; menu-bar `About` dialog title **`About Windows Media Player`**.
* Layout regions: left navigation (`Now Playing`, `Media Guide`, `Copy from CD`, `Media Library`,
  `Radio Tuner`, `Copy to CD or Device`, `Skin Chooser` — each rendered on **two lines**, e.g. `Now<br/>Playing`),
  centre visualization + song info, bottom transport, right playlist drawer, bottom equalizer drawer,
  and a `video-viewer` overlay.
* Panel exclusivity: `data-xp-panel` = `equalizer` | `playlist` | `viz` — opening one closes the others
  (XP / Professional / Rusty skins).
* Colours (`colors.txt`): *Play / Stop / Sound* gradient **`#04053a → #5679bb`**, *Skip* gradient
  **`#5679bb → #849cce`**, green **`#00ff00`**.
* `skin-mode-toggle`, `fullscreen-toggle` (disabled), `shuffle-mode-toggle`, `info`, `equlizer-toggle`,
  `show-song-cover`, `playlist-toggle`, `player-minimize`, `player-close`, `switch-skin` buttons.
* Tooltips (exact): `Small Screen`, `Size 320x240`, `Switch to Full Mode` / `Switch to Skin Mode`,
  `Full Screen`, `Turn Shuffle On` / `Turn Shuffle Off`, `Show Song Info` / `Hide Song Info`,
  `Show Equalizer and Settings` / `Hide Equalizer and Settings` (Aquarium inverts the wording),
  `Display album art`, `Show Playlist` / `Hide Playlist`, `Minimize`, `Close`,
  `Next Visualization`, `Show Controls` / `Hide Controls` (Headspace),
  `Close Equalizer`, `Reset graphic equalizer controls`, `Turn Equalizer On` / `Turn Equalizer Off`
  (Compact), `Current Preset`, `Next preset`, `Previous settings` / `Next settings`,
  `Show Equalizer` / `Hide Equalizer`, `Show playlist` / `Close playlist`, `Balance`, `Volume`,
  `play song`, `Play`, `Pause`, `Stop`, `Previous`, `Rewind Back`, `Rewind Forward`.

### Song info / playlist strings

```
Unknown Artist            No track selected            --:--
Artist:  Album:  Track:  Elapsed time:  Total time:      ← Heart info panel
Current Playlist          Total Time: m:ss              ← playlist header + total
Show: <track>   Clip: <track>   Author: <artist>
Copyright: © 2001 Microsoft Corporation                 ← Classic skin only
Playing <track> m:ss / m:ss   |   Paused   |   Stopped
```

Formatting: `m:ss` with `padStart(2,'0')`. Durations start as `--:--` until metadata loads.
Playlist dropdown caret `⯆`; per-row active state on the current track.

### Menu bar (`MediaPlayerMenu.tsx`) — exact item strings, mnemonics, shortcuts, state

**`File`** — `Open...` (`O`) · `Open URL...` (`U`, disabled) · `Close` (`C`) · — · `New Playlist...` (`N`, dis) ·
`Edit Current Playlist...` (`E`, dis) · `Add to Media Library` (`A`, dis) · `Save Media As...` (`M`, dis) ·
`Save Playlist` (`S`, dis) · `Save Playlist As...` (`A`, dis) · — · `Copy` (`y`, dis) · — ·
`Properties` (`P`, dis) · `Work Offline` (`l`, dis) · — · `Exit` (`x`).

**`View`** — `Full Mode` (`F`, Ctrl+1, ☑ when full) · `Skin Mode` (`S`, Ctrl+2, ☑ when skinned) · — ·
`Full Mode Options ▸` (`Show Menu Bar`, —, `Now Playing`, `Media Guide`, `Copy from CD`, `Media Library`,
`Radio Tuner`, `Copy to CD or Device`, `Skin Chooser` — all ☑) ·
`Now Playing Tools ▸` (`Show Equalizer and Settings`, `Show Video and Visualization`, `Show Media
Information`, `Show Playlist`, `Show Captions` — all ☑) · `Taskbar ▸` (`Windows Taskbar` dis,
`Windows Media Player in Taskbar` dis) · — · `Visualizations ▸` (tree below) · — ·
`File Markers ▸` (`Previous Marker` Ctrl+Shift+B dis, `Next Marker` Ctrl+Shift+F dis) ·
`Statistics` (dis) · — · `Full Screen` Alt+Enter · `Refresh` (dis) ·
`Zoom ▸` (`50%` Alt+1 dis, `100%` Alt+2 dis, `200%` Alt+3 dis, —, `Fit Video to Player on Resize` dis).

**`Play`** — `Play/Pause` (Ctrl+P) · `Stop` (Ctrl+S) · — · `Previous` (Ctrl+B) · `Next` (Ctrl+F) · — ·
`Shuffle/Repeat ▸` (`Shuffle` Ctrl+H ☐, `Repeat` Ctrl+T ☐) · — · `Volume Up` (F10) · `Volume Down` (F9) ·
`Mute`/`Unmute` (F8) · — · `Play Speed ▸` (`Slow` Ctrl+Shift+G, `Normal` Ctrl+Shift+N, `Fast` Ctrl+Shift+F —
one checked; rates 0.5 / 1 / 2).

**`Tools`** — `Download Visualizations` (`D`, dis) · `Search for Media Files...` (`S`, F3, dis) · — ·
`License Management` (`L`, dis) · — · `Options...` (`O`, dis).

**`Help`** — `Help Tools` (`H`, F1, dis) · — · `Check for Player Updates...` (`C`, dis) ·
`Privacy Statement` (`P`, dis) · `About Windows Media Player` (`A`).

### Visualizations (menu tree + skin-dropdown labels + asset filenames)

`VizDropdown` (the `✱` button) exposes 8 top-level entries: **Album Art, Ambience, Bars and Waves,
Battery, Particle, Plenoptic, Spikes, Musical Colors** (the last three* are the README's names).

| Group | Item | `.mp4` file | Internal label |
|---|---|---|---|
| — | Album Art | *(cover image; falls back to `music/visualizations/fallback.webp`)* | `Album Art` |
| Ambience | Random / Water / Falloff / Swirl | `Ambience Water.mp4`, `Ambience Falloff.mp4`, `Ambience Swirl.mp4` | `Ambience` / `Ambience:Water` / `:Falloff` / `:Swirl` / `:Random` |
| Bars and Waves | Bars / Ocean Mist / Fire Storm / Scope | `Bars and Waves - Bars.mp4`, `Bars and Waves Oceam Mist.mp4` *(sic — "Oceam")*, `Bars and Waves Firestorm.mp4`, `Bars and Waves Osciloscop.mp4` | `Bars and Waves:Bars` etc. |
| Battery | Randomization / Lotus / Event Horizon / Smoke or Water? | `Battery Randomization.mp4`, `Battery - Lotos.mp4`, `Battery Event Horizon.mp4`, `Battery - Smoke or Water.mp4` | `Battery:Lotus` … |
| Particle | Particle / Rotating Particle | `Particle.mp4`, `RotatingParticle.mp4` | `Particle:Particle`, `Particle:Rotating Particle` |
| Plenoptics | Plenoptic (random) / Smokey Circles / Smokey Lines / Vox | `Penoptic Smokey CirclesSM.mp4`, `PlenopticsSmokeyLines.mp4`, `PlenopticVox.mp4` | `Plenoptic`, `Plenoptics:Smokey Circles`, `:Smokey Lines`, `:Vox` |
| Spikes | Spike / Amoeba *(disabled)* | `Spikes.mp4` | `Spikes:Spike` |
| Musical Colors | Colors in Motion / Ice Crystals *(disabled)* | `MusicalColors.mp4` | `Musical Colors:Colors in Motion` |

`◀` / `▶` buttons (`move-back` / `move-next`) cycle presets inside the current group; the current
label is shown in `.visualization`. Visualizations are **video files**, not procedurally drawn —
our Rust port needs either the same `mp4` playback or a native renderer for the same names.

### Skins (Skin Chooser)

Chooser chrome: buttons **`Apply Skin`** (`A`, icon) · **`More Skins`** (`S`, disabled, split by
a divider) · close `×`; caption **`Click Apply Skin to open the skin`**. Right pane prints
`Title: <title>` / `Author: Microsoft Corporation` /
`Copyright: (C)2001 Microsoft Corporation. All rights reserved.`
(`MS_COPYRIGHT` in the source is `Copyright: (C)2001 Microsoft Corporation. All rights reserved.`).
All 14 skins share `author: 'Microsoft Corporation'`.

Left list order, exactly: **`(Default Media Player)`**, `Classic`, `Compact`, `Headspace`, `Heart`,
`Miniplayer`, `Plus! Aquarium`, `Plus! da Vinci`, `Plus! Nature`, `Plus! Space`, `Professional`,
`Rusty`, `Toothy`, `Windows XP`. Note `(Default Media Player)` maps to the **Corporate** skin
(ids: `classic`, `compact`, `corporate`, `headspace`, `heart`, `miniplayer`, `professional`, `rusty`,
`toothy`, `windowsxp`, `aquarium`, `space`, `davinci`, `nature`).

Per-skin specifics worth porting (all in `MediaPlayerApp.tsx`):

* **Classic** — three-row status block (`Show:` / `Clip:`+`Author:`+`Copyright:` / `Playing …`), two
  separators around the transport, logo fallback `classic/icon_wmlogo.bmp`, default skin title `Classic`.
* **Compact** — decorative `.logo`, `Graphic Equalizer` drawer title with disabled `previous/next
  settings` buttons, an EQ on/off row (`Turn Equalizer On/Off` + label) and a one-button preset switcher.
* **Corporate** — fallback logo `corporate/CorporateLogo.webp`; the *de facto* default player.
* **Heart** — info panel with `Artist: / Album: / Track: / Elapsed time: / Total time:` and an EQ
  preset switcher (`Previous preset` / `Next preset` / preset name).
* **Headspace** — 10-band EQ, extra `song-buttons-close` control, `Show/Hide Controls` on `✱`,
  history-scribble styling; playlist visible by default.
* **Miniplayer** — taskbar-compact layout with inline progress track.
* **Professional** — dedicated `Graphic Equalizer` titled drawer.
* **Toothy** — animated decorative `.eye` over the visualization; equalizer drawer closed by default.
* **Rusty / Windows XP** — share the XP drawer system.
* **Aquarium / Space / Da Vinci / Nature** — Plus! skins; Aquarium has decorative jewels
  (`.ruby-left`, `.ruby-right`, `.diamond-left`, `.diamond-right`, `.pendant`), theme-specific menu
  sounds, and an equalizer/volume drawer with an open/close animation (drawer includes `bass`, `treble`,
  `balance`).
* Video overlay tooltips: `Size 320x240` / `Small Screen` (the `video-viewer` div).

Equalizer defaults: every band starts at **50**; bands are `bass`, `treble`, `balance` plus ten extra
`eq-slider-wrap--headspace-only` inputs; `reset` button restores them.

---

## 5. Microsoft Plus! for Windows XP (`src/components/plus/**`)

### Window

* Title-bar text: **`Microsoft Plus!`** (with the Plus! icon that opens the system menu).
* Size **795 × 563 px**; fullscreen `100vw × (100vh − 30px)`; `font: 9pt Arial, Helvetica, sans-serif`,
  `letter-spacing: .5px`. Selection: white on `#2d5ea8`.
* Palette (CSS custom properties on `.plus-window`): header **`#99e823`**, intro text **`#ccf6b4`**,
  body text **`#c3f6a6`**, background **`#164218`**, menu item text **`#021308`**,
  scrollbar face/highlight/shadow `#5b834d`, track/dark-shadow `#313526`, arrow black,
  `--right-frame-width: 445px`, `--right-frame-height: 375px`, local button colour `white`.

### Layout

| Element | Position / size |
|---|---|
| `.main-page` | `height: calc(100% − 28px)`, background `img/main-bg.webp` (100 % auto, top-left); digital-media/games/screen-savers use `img/empty-bg.webp`; themes use `aquarium/davinci/nature/space.webp`; more-plus uses `more-plus.webp` |
| Sidebar `aside` | `left: 48px; top: 85px; width: 210px` |
| `Plus!` label | 12 pt bold `#99e823`, `margin: 0 0 3px 25px` |
| Menu rows | `186 × 17 px`, `padding-left: 25px`, 9 pt / 700, `letter-spacing: .8px`, colour `#021308`; hover → `#99e823`; active → `MENU_SELECTED.GIF` 186 × 16 |
| Separator row | 190 × 7 px `MENU_LINE.GIF` |
| `More Plus!` row | white, 11 pt / 700, `letter-spacing: 1.35px` |
| Content pane | `left: 290px; top: 45px; 445 × 210px`; on Themes the height becomes `75 %` |
| Home hotspot | invisible `150 × 85 px`, `bottom: 30px; left: 30px` |
| Footer | `right: 28px; bottom: 29px; width: 480px`, 6.5 pt right-aligned |

### Pages and exact copy

**Navigation rows:** `Digital Media`, `Games`, `Themes`, `Screen Savers`, separator, `More Plus!`.
Every row plays the menu-command sound and the themes can supply their own sound.

**Welcome** (default page) — heading `Welcome`, sub-heading `to Microsoft Plus! for Windows XP`, body:
> `The ultimate companion to Windows XP delivers exciting new features for digital music, gaming, new desktop graphics, photos, and much more, letting you personalize your computer experience like never before.`
> `Click any of the links on the left and discover a new and exciting multimedia experience on your computer.`

Right-hand links: **`Release Notes`** (→ GitHub release `v1.8.0` in IE), **`Registration`**
(→ Plus! *registration* error dialog), **`About Plus!`** (→ About Plus! dialog). Balloon/exclamation
sound plays on the first two.

**Digital Media** — `Plus! Digital Media` / *Enhance your audio and video exprerience with advanced
digital media tools.* *(sic: "exprerience")* — seven products, each with a 48 px icon,
click-to-expand description and a `More Information` link:

| Product heading | Description (verbatim) |
|---|---|
| `Plus! Voice Command for Windows Media Player` | *Enables you to control Windows Media Player by simply speaking commands. Just sit back, relax, and tell the Player what to do.* |
| `Plus! MP3 Audio Conventor` *(sic)* | *Converts MP3 files quickly and easily to Windows Media audio files, which take up less than half the space on your computer and deliver the same audio quality.* |
| `Plus! CD Label Maker` | *Creates and prints colorful, customized CD labels, inserts, and booklets. Automatically adds your track listings and artwork to enhance your creations.* |
| `Plus! Speaker Enhancement` | *Instantly boosts the clarity and richness of many computer speakers to make your music and videos more vibrant than ever.* |
| `Plus! Personal DJ` | *Makes it easier to create custom playlists with your favorite songs in Windows Media Player. Hear the music you want when you want it.* |
| `Microsoft Windows Media Player Skins` | *Personalizes Windows Media Player with unique graphical skins.* |
| `Plus! 3-D Visualizations` | *Adds stunning new 3-D visual effects that change and move in response to your music in Windows Media Player.* |

**Games** — `Plus! Games` / *Experience the fun and excitement of three new games that incorporate the
latest in computer technology, 3-D graphics, and big screen sights and sounds.* —

| Game | Description (verbatim) |
|---|---|
| `Russian Square Plus! Edition` | *Tests your thinking and strategy skills in an addictive new game for fans of Tetris. The action will keep you coming back for more!* |
| `The Labyrinth Plus! Edition` | *Rolls the classic tabletop labyrinth game into an "a-maze-ing" 3-D world filled with vibrant visuals and sounds. Test your skills with more than 40 unique and challenging levels. You'll never get bored!* |
| `HyperBowl Plus! Edition` | *Takes the fun and excitement of classic bowling out of the alley and onto the streets of Ancient Rome. HyperBowl delivers arcade-style action using the latest in computer technology, 3-D graphics, and sound.* |

Each has a `More Info` link under the artwork.

**Themes** — `Plus! Themes` / *Persinalize your computer with enhanced themes that include, screen
savers, wallpapers, icons, sounds, pointers, skins for Microsoft Windows Media Player, and much more.*
*(sic: "Persinalize", "include,")* — list rows with 27 px icons: `Plus! Aquarium`, `Plus! Space`,
`Plus! Nature`, `Plus! Da Vinci` (selected row turns `#99e823` + underline). A centred
**`Select the theme`** link jumps to Display Properties → **Themes** with that theme pre-selected.
Applying a theme sets wallpaper (`PlusAquarium.webp`, `PlusSpace.webp`, `PlusNature.webp`,
`PlusdaVinci.webp`), sets the screensaver (`aquarium|space|nature|daVinci`), switches cursors, sounds
and the Recycle Bin icon, and writes `localStorage['wmp-skin-mode']` to force WMP into the matching skin.

**Screen Savers** — `Plus! Screen Savers` / *Enjoy astounding, lifelike, 3-D screen savers. Experience
a virtual aquarium, personalize your computer with your own photos and music, explore a space station,
and more!* — eight rows with 33 px icons, each labelled on two lines (`Plus!<br />Aquarium` …):
`Aquarium`, `My Pictures`, `Space`, `Nature`, `da Vinci`, `Robot Circus`, `Sand Pendulum`,
`Mercury Pool`. Preview pane has a screenshot image plus two links: **`Preview`** and
**`Select this screen saver`** (disabled while the mapping is missing — `My Pictures` has no mapping
into Display Properties).

**More Plus!** — `More Plus!` / `Thank you for purchasing Microsoft Plus! for Windows XP.` /
*Now that you've seen and experienced Microsoft Plus!, our **Web site** makes it easy for you to keep up
with the latest technology and make the most of your experience on Microsoft Windows XP. **Visit us
online** for the latest Plus! developments and special offers exclusively for Plus! owners.* / `Enjoy!`
plus the checkbox **`Tell me when new versions of Microsoft Plus! are available.`**
(links → `alena-pumprova.cz` and the GitHub repo).

**Footer** (all pages):
> `Copyright (C) Microsoft Corporation. All Rights Reserverd.` *(sic: "Reserverd")*
> `This program is protected by U.S. and international copyright laws as described in About Plus!`
> `Privacy Information` (right-aligned, underlined)

**Dialogs:** `About Plus!` (period about box: product info, version, product ID, image credits) and a
Plus! **registration** error dialog (error type `registration`).

> **Gap vs this repo:** we have no Plus! window at all; this section is a complete first-draft spec.
> The four Plus! wallpaper files and the eight screensaver assets are Microsoft-derived — implement
> with the `assets-local/` drop-in policy.

---

## 6. 3D Pipes & screensavers

### Alena's Screen Saver tab (`DisplayProperties.tsx`, Screen Saver tab, `450,50` default position)

Drop-down order and internal values (14 entries):

```
(None)                        ''
Plus! Aquarium                aquarium
Plus! da Vinci                daVinci
Plus! Nature                  nature
Plus! Space                   space
Curves and Colors             curvesAndColors
Flying Windows                flyingWindows
Haunted House                 hauntedHouse
Maze                          maze
Mercury Pool                  mercuryPool
Mystify Your Mind             mystifyYourMind
Sand Pendulum                 theSandPendulum
Robot Circus                  theRobotCircus
Windows 98                    windows98
```

Fieldset **`Screen saver`** contains the combo plus `Settings` (`S`, inert), `Preview` (`P`, disabled
when none) and a wait row: **`Wait:`** (`W`) numeric input **min 1 / max 999, default 10**, the unit
label `minutes`, and the checkbox **`On resume, display Welcome screen`**. A second fieldset
**`Monitor power`** shows the energy icon, the text `To adjust monitor power settings and save energy,
click Power.` and the button **`Power...`** (`w`). Buttons at bottom: `OK`, `Cancel`, `Apply`.

Runtime overlay (`ScreensaverOverlay.tsx`): fullscreen `position: fixed; inset: 0;
z-index: 2147483647; background: #000; cursor: none`, one muted looping `<video>` at
`object-fit: cover`; dismissal listens for `mousemove`, `mousedown`, `keydown`, `touchstart` after a
**500 ms** guard. The Aquarium saver additionally plays random bubble sounds every
**1.5 s + rand(0–4 s)**, stopped on dismiss and on `visibilitychange`.

Plus! savers map to saver names as: `aquarium→aquarium`, `space→space`, `nature→nature`,
`davinci→daVinci`, `robot-circus→theRobotCircus`, `sand-pendulum→theSandPendulum`,
`mercury-pool→mercuryPool`, and **`my-pictures` has no mapping** (so *Select this screen saver* is
disabled for it).

### The 3D Pipes asset discrepancy (high-value porting target)

`public/WINDOWS/Resources/Themes/Screensavers/` contains:

```
aquarium.mp4   curvesAndColors.mp4   daVinci.mp4   flyingwindows.mp4   hauntedHouse.mp4
maze.mp4   mercuryPool.mp4   MystifyYourMind.mp4   nature.mp4   space.mp4
theRobotCircus.mp4   theSandPendulum.mp4   windows98.mp4
Windows 3D Pipes Screensaver.mp4          ← present, but NOT in the Screen Saver list
compressed/                               ← second copy of the set
```

So Alena **ships a 3D Pipes video but never exposes it in Display Properties**. Two porting routes:
wire the video (fast, matches their asset), or implement the screensaver natively.

### 3D Pipes spec — from `pranx.com/pipes/` *(external source; copy is marketing text, not product strings)*

* Page title / label: **`3D Pipes Screensaver`**; nicknamed *"The plumber's nightmare"*; shipped in most
  Windows versions **up to and including XP**.
* Visual description (verbatim from the page): *"tangled colored pipes being drawn like snakes on a dark
  background. The screen was cleared after a while fading out the pipes or pixelating them."*
* Easter eggs (verbatim): *"a nice Windows easter egg when the joints were replaced by teapots or when
  pipes were painted as red and white candy canes"* — i.e. pipe joints can become **teapots**, and pipe
  segments **candy-cane** red/white.
* Settings surfaced on that page: label **`Joints:`** with the options **`Elbow`**, **`Ball`**,
  **`Mixed`**, and a separate **`Cycle`** option (animate through joint styles).
* Controls on the page: **`Mouse Controls`** toggle (`OFF`/`ON`), **`Full Screen`** (F11),
  **`Hide Intro`** (Esc); "Click the 3D pipes to change the view. Enable mouse controls … and drag the
  screen to change the view smoothly and use the scroll to zoom."

**[unverified]** The native XP dialog strings for 3D Pipes (**`3D Pipes Setup`**, `Joints`, `Elbow`,
`Ball`, `Mixed`, `Cycle`, `Surface Style` — `Solid`/`Tubes`/`Candy Cane`/`Teapot`, `Pipe Style`
`Traditional`/`Cycling`, `Joint Style`, `Resolution` `Low/Medium/High`, `Size`, `Number of pipes`,
`Speed`, `Random`, `Custom Colors…`) are **not** present in any fetched source and must be verified
against a real XP SP3 install (or `xfce-winxp-tc`) before being written into code.

**Our repo status:** 3D Pipes is *not* in `docs/FEATURE-GAPS.md`, and our `screens/display_properties.rs`
is planned. This section is the first source for it.

---

## 7. Internet Explorer chrome

### 7.1 Alena's IE6+SP2 window (`IE/IEWindow.tsx`, `IEMenu.tsx`, `IE/data/IEData.tsx`)

* **Window title format:** `<page title> – Microsoft Internet Explorer` (EN dash, U+2013), page titles
  **truncated to 45 characters + `...`**; fallback title `Alena Pumprová – Microsoft Internet Explorer`.
  Title and favicon are pushed to the shell (`onTitleChange` / `onFaviconChange`), so **each IE window
  becomes its own taskbar button** with the live page title and favicon.
* Home = Wayback-archived Google (`web.archive.org/web/20031024040025if_/http://www.google.com/`),
  default page = the author's portfolio.
* **Toolbar (left→right):** `Back ▾` · `Forward ▾` · `Refresh` · `Stop` · `Home` · `Search` ·
  `Favourites` · `Media` · `History` (icon only) · `Mail` · `Print` · `Edit` · `Discuss`.
  Back/Forward have split dropdown arrows (inert) and disable at the ends of history.
* **Address row:** label `Address` (mnemonic **d**), text input with the current favicon, a **20 × 20 px**
  Luna stepper button on the right (`Luna/bottom-stepper.png`, hover/active variants), a `Go` button and
  a **`Links »`** dropdown containing `MSN.com`, `Radio Station Guide`, `Windows Media`.
* **Status bar:** current favicon + the string **`Internet`** (with the internet-shortcut icon) — the
  security-zone field a real XP status bar shows.
* **Explorer bars** (toggle, one at a time): `IEFavourites`, `IEHistory`, `IESearchCompanion`,
  `IEMedia`, plus `TipOfTheDay`, `OpenDialog`, `AddFavourite`.

**Menu bar — full tree** (mnemonics underlined here, `⇥` = shortcut, *(dis)* = disabled in source):

```
File: New ▸ (Window · Message dis · Post dis · Contact dis · Internet Call dis)
      Open... Ctrl+O · Edit with Windows Notepad (dis) · —
      Save Ctrl+S (dis) · Save As... · —
      Page Setup... (dis) · Print... Ctrl+P · Print Preview... (dis) · —
      Send ▸ (Page by E-mail... dis · Link by E-mail... dis · Shortcut to Desktop dis)
      Import and Export... (dis) · —
      Properties (dis) · Work Offline (dis) · Close
Edit: Cut Ctrl+X · Copy Ctrl+C · Paste Ctrl+V · — · Select All Ctrl+A (dis) · Find (on This Page)... Ctrl+F (dis)
View: Toolbars ▸ (Standard Buttons · Address Bar · Links dis) · Status Bar ·
      Explorer Bar ▸ (Search · Add to Favorites... · History · Media dis) · —
      Go To ▸ (Back · Forward · Home Page) · Stop Esc · Refresh F5 · —
      Text Size ▸ (Largest … Smallest, all dis) ·
      Encoding ▸ (Auto-Select, Unicode (UTF-8), Western European (Windows),
                  Central European (Windows) — all dis) · —
      Source · Privacy Report... · Full Screen F11
Favourites: Add to Favorites... · Organize Favorites... (dis) · — · 5 folders with all bookmarks:
      Search & Mail ▸ Google · Seznam.cz · Seznam E-mail · Centrum.cz · ICQ · xChat
      Social ▸ Lidé.cz · Spolužáci · Libimseti.cz · LinkedIn · Zpovědnice
      Games ▸ PAC-MAN · Nu Pogodi · The Morning After · Harlequin's Fortune ·
               Detective Game · Superhry.cz · Českéhry.cz · Happy Tree Friends · Miniclip
      Entertainment ▸ Alena Pumprová · Nova.cz · Očko TV · Kinobox.cz · Lamer.cz · iDnes.cz
      Tech ▸ Microsoft.com · Mobilmania · MSN.com · Windows-XP/GitHub.com
Tools: Mail and News ▸ (Read Mail · New Message... · Send a Link... · Send Page... · — · Read News — all dis)
       Pop-up Blocker ▸ (Turn Off Pop-up Blocker dis · Pop-up Blocker Settings... dis) · —
       Manage Add-ons... (dis) · — · Windows Update (dis) · — · Internet Options... (dis)
Help: Contents and Index · Tip of the Day · For Netscape Users (dis) · Online Support (dis) ·
      Send Feedback (dis) · — · About Internet Explorer
```

`About` dialog title: **`Internet Explorer`**. `Print...` routes to the `printerConnect` error;
`Contents and Index` and `Privacy Report...` route to the `webPageNotFound` error.

**Network-error page strings** (shown for blocked/unavailable domains):

```
Internet Explorer cannot display the webpage
What you can try:
  [ Diagnose Connection Problems ]
  More information ▸
    This problem can be caused by a variety of issues, including:
      · Internet connectivity has been lost.
      · The website is temporarily unavailable.
      · The Domain Name Server (DNS) is not reachable.
      · The Domain Name Server (DNS) does not have a listing for the website's domain.
      · There might be a typing error in the address.
    For offline users
    You can still view subscribed feeds and some recently viewed webpages.
    To view subscribed feeds:
      1. Click the Favorites button ⭐, click Feeds, and then click the feed you want to view.
    To view recently visited webpages (might not work on all pages):
      1. Press Alt, click File, and then click Work Offline.
      2. Click the Favorites button ⭐, click History, and then click the page you want to view.
```

`Save As...` writes into `c-alena-favorites` and names the file
`<sanitised page title>.html`; `View Source` opens the page source in Notepad
(fallback body: `The source for this page cannot be displayed.\r\n\r\n<url>`).

### 7.2 IE7-style chrome (`pranx.com/windows-xp-simulator`) — **not IE7 metrics, only labels**

The pranx simulator is the only fetched source that labels an IE window **IE7**. What it exposes:

* Window label **`Internet Explorer - IE7`** (space-hyphen-space).
* Menu bar: `File  Edit  View  Favorites  Tools  Help` (US spelling **Favorites**).
* Address bar with an **`Address`** label and a **`Go`** button; a **`Links`** bar is present.
* Taskbar/page inventory seen in its markup: `Start` button, `My Computer`, `Internet`,
  `Mozilla Firefox`, `E-mail` (`Mozilla Thunderbird`), `Windows Media Player`, `Minesweeper`,
  `Tetris`, `Command Prompt`, `Norton Commander`, `Winamp`, `My Music`, `My Network Places`,
  `Control Panel`, `Printers and Faxes`, `Help and Support`, `Search`, `Run`, `All Programs`;
  `All Programs` ▸ `Windows Update · Accessories · Games · Startup · Internet Explorer · MSN Explorer ·
  Windows Messenger · Outlook Express · Remote Assistance · Windows Media Player`; power rows
  `Log Off`, `Shut Down` (note: **`Shut Down`**, two words, vs XP's `Turn Off Computer`).
* Quick Launch: `Show Desktop`, `IE 7`, `Winamp`, `CMD`, `Welcome`.
* System tray: `Winamp`, `MSN`, `LAN`.
* Browser chrome content: `File Edit View Favorites Tools Help`, `Address`, `Go`, `Links`.
* Its Run dialog text is identical to XP's standard sentence (see §8.9) with the title
  **`Run Dialog`***(their label, not the real XP window title `Run`)*.

**[unverified]** No pixel metrics, toolbar icon order, or tab-strip strings for IE7 are available from
this source — it is a prank simulator's marketing page, and the strings above come from its markup, not
from a rendered IE7. Treat 7.2 as a **label-level reference only**; for real IE7 chrome take
measurements from a VM or `xfce-winxp-tc`.

> **Gap vs this repo:** our IE (`src/apps/internetexplorer.rs`) is a simulated renderer with no
> navigation. The gap list (`FEATURE-GAPS.md` §7) already asks for the address bar, Back/Forward/
> Stop/Reload/Home, History/Search/Favorites, status-bar zone and a Wayback path — the tree in §7.1 is
> the exact chrome to build, including the `Links »` dropdown and the per-window taskbar button.

---

## 8. Control Panel and the app-by-app specs

### 8.1 Control Panel (`files/data/FileManagerData.ts`)

The Control Panel is a File-Manager node `{ id: 'controlpanel', name: 'Control Panel',
folderType: 'controlpanel' }` and is **excluded from the My Computer category grouping** (along with
`recyclebin`) — see `rootCategories.ts`. Its 11 children, in this exact order:

| id | Label | Type | Wired to |
|---|---|---|---|
| `cp-appearance` | `Appearance and Themes` | folder | empty |
| `cp-network` | `Network and Internet Connections` | folder | empty |
| `cp-programs` | `Add or Remove Programs` | **file** | nothing (inert) |
| `cp-audio` | `Sounds, Speech, and Audio Devices` | folder | empty |
| `cp-performance` | `Performance and Maintenance` | folder | empty |
| `cp-printers` | `Printers and Other Hardware` | folder | empty (reached from the Start menu too) |
| `cp-users` | `User Accounts` | folder | empty |
| `cp-date` | `Date, Time, Language, and Regional Options` | folder | empty |
| `cp-accessibility` | `Accessibility Options` | folder | empty |
| `cp-fonts` | `Fonts` | folder | `C:\WINDOWS\Fonts` |
| `cp-display` | `Display` | folder | **Display Properties** |

Every entry carries `modified: '10/05/2003'`. The Start-menu Control Panel fly-out additionally offers
`Administrative Tools`, `Network Connections`, `Scanners and Cameras` and `Scheduled Tasks` (all empty).

Related My Computer grouping strings (`ROOT_CATEGORY_ORDER`, printed as group headers in Thumbnails /
Tiles / Icons / Details but **not** List): `Files Stored on This Computer`, `Hard Disk Drives`,
`Devices with Removable Storage`.

**[unverified]** The Control Panel **category view** chrome — the page heading `Pick a category`, the
left task pane (`See Also`, `Troubleshooters`, `Windows Update`), and the category-view icon sizes —
is not present in any fetched source. Only the 11 labels above are proven. If it matters, take the
category-view strings from a VM screenshot per `PLAN.md` §2.1.

### 8.2 Display Properties (`DisplayProperties.tsx`)

* Window title **`Display Properties`**; title bar has only **Help** and **Close** buttons (no minimise
  or maximise), default position `(450, 50)`.
* Tabs, in order: **`Themes` · `Desktop` · `Screen Saver` · `Appearance` · `Settings`**; footer buttons
  **`OK` · `Cancel` · `Apply`**.
* **Themes** — text: *A theme is a background plus a set of sounds, icons, and other elements to help you
  personalize your computer with one click.*; label `Theme:`; combo `Windows XP`, `Plus! Aquarium`,
  `Plus! Da Vinci`, `Plus! Nature`, `Plus! Space`; buttons `Save As...`, `Delete` (disabled);
  `Sample:` preview image.
* **Desktop** — CRT monitor preview + `Bac`**`k`**`ground:` list; buttons `Browse...` (`B`) and
  `Position:` (`P`) = `Stretch | Center | Tile`; `Color:` (`C`) colour input.
* **Screen Saver** — see §6.
* **Appearance** — preview swatch; `Windows and buttons:` = `Windows XP style`
  (`Windows Classic style` present but disabled); `Color scheme:` = `Default (blue)`, `Olive Green`,
  `Silver`; `Font size:` = `Normal`, `Large`, `Extra Large`; buttons `Effect...`, `Advanced`.
* **Settings** — `Display:` line: `Default Monitor on Intel(R) 82865G Graphic Controller`;
  fieldset `Screen resolution` with a 5-stop slider (`Less` … `More`) and resolution labels
  `640 by 480 pixels`, `800 by 600 pixels`, `1024 by 768 pixels`, `1280 by 1024 pixels`,
  `1600 by 1200 pixels`; fieldset `Color quality` = `Medium (16 bit)`, `Highest (32 bit)`;
  buttons `Troubleshoot...`, `Advanced`. The CRT preview filters are
  `blur(1.5px) saturate(.4)` … `blur(0) saturate(1.2)`, plus `grayscale(.3) contrast(.9)` in 16-bit.
* Applying a theme or appearance change shows a **`Please wait…`** overlay (1.5 s).
* **Wallpaper list** (28 presets + 28 bitmap/`.bmp` entries). Preset labels, in order:
  `(None)`, `Ascent`, `Autumn`, `Azul`, `Bliss`, `Crystal`, `Follow`, `Friend`, `Home`, `Moon Flower`,
  `Peace`, `Power`, `Purple Flower`, `Radiance`, `Red Moon Desert`, `Ripples`, `Stonehenge`, `Tulips`,
  `Vortex Space`, `Wind`, `Windows XP`, `Plus! Aquarium`, `Plus! Aquarium 2`, `Plus! Da Vinci`,
  `Plus! Nature`, `Plus! Nature 2`, `Plus! Space`, `Plus! Space 2`.
  Bitmap labels: `Upstream (16 color)`, `Solstice`, `Snakeskin`, `Seaside`, `Petroglyph (16 color)`,
  `Leather (16 color)`, `Leaf Fossils (16 color)`, `Geometrix`, `Hazy Autumn (16 color)`,
  `Maple Trails`, `Swimming Pool`, `River Sumida`, `Hiking Boot`, `Blue Monday`, `Fiddle Head`,
  `Furry Dog`, `Prairie Wind`, `Rhododendron`, `Zapotec`, `Coffee Bean`, `Santa Fe Stucco`,
  `Greenstone`, `Blue Lace (16 color)`, `Blue Rivets`, `Gone Fishing`, `Feather Texture`, `Soap Bubbles`
  (files: `Upstream16.bmp`, `Solstice.bmp`, `Snakeskin.bmp`, `Seaside.bmp`, `Petroglyph16.bmp`,
  `Leather16.bmp`, `Leaf-Fossils16.bmp`, `Geometrix.bmp`, `Hazy-Autumn16.bmp`, `maple-trails.bmp`,
  `swimming-pool.bmp`, `river-sumida.bmp`, `Hiking-Boot.bmp`, `blue-monday.bmp`, `fiddle-head.bmp`,
  `furry-dog.bmp`, `Prairie-Wind.bmp`, `rhododendron.bmp`, `zapotec.bmp`, `Coffee-Bean.bmp`,
  `Santa-Fe-Stucco.bmp`, `Greenstone.bmp`, `Blue-Lace16.bmp`, `Blue-Rivets.bmp`, `Gone-Fishing.bmp`,
  `Feather-Texture.bmp`, `Soap-Bubbles.bmp`). Picking a `.bmp` forces `Position = Tile`.
  Presets live at `WINDOWS/Web/Wallpaper/<Name>.webp`; the picker is the shared **Open** dialog with
  filter `Background Files (*.bmp, *.gif, *.jpg, *.jpeg, *.png, *.webp)` / `All Files`.

### 8.3 Calculator (`Calculator.tsx`, `CalculatorMenu.tsx`)

* Title bar text: **`Calculator`** (with the Calculator icon). Title bar height overridden to **26 px**
  with **14 px** text (`.calculator-window .title-bar`); default position `(400, 150)`.
* Buttons: minimise, **maximise disabled**, close.
* Menus (exact): **Edit** → `Copy` Ctrl+C · `Paste` Ctrl+V. **View** → `Scientific` ☐ · `Standard` ☑ ·
  separator · `Digit grouping` ☐. **Help** → `Help Topics` (disabled) · separator ·
  `About Calculator` → About dialog titled **`Calculator`**.
* Two modes: **Standard** (`CalculatorApp`) and **Scientific** (`CalculatorScientific`). Scientific adds
  `sin cos tan` with `Inv`/`Hyp`, angle modes `Degrees/Radians/Grads`, `log ln x² x³ xʸ n! π 1/x Int F-E
  Exp dms`, bitwise `And Or Xor Not Lsh Mod`, and bases `Hex Dec Oct Bin` with unavailable digits
  disabled. Digit grouping uses a **cs-CZ** locale.
* Our gap list wants exactly this; no px size for the window body was captured (it lives in
  `Calculator/Calculator.css`, **not fetched**) — **[unverified: window pixel size]**.

### 8.4 Character Map (`CharacterMap.tsx`)

* Window title: **`Character Map`**; default position `(200, 100)`; buttons minimise / maximise
  (toggle) / close.
* Body, top to bottom:
  * header: literal `Font:` + custom font drop-down (type icon + display name + stepper arrow) +
    **`Help`** button (opens Help and Support).
  * character grid: buttons per glyph with `U+XXXX` tooltip; click selects, double-click appends;
    `data-tt-pos` flips tooltip side based on remaining space.
  * footer: `Characters to copy:` + `<output>` (rendered in the selected font) + **`Select`** (`luna-btn`)
    and **`Copy`** (`luna-btn secondary`, label flips to **`Copied!`** for 1.2 s) ·
    checkbox **`Advanced view`**.
  * status bar: single field **`U+XXXX: <character name>`**.
* Character set: real per-font glyph table via `opentype.js`; fallback = code points **0x20–0xFF
  excluding 0x7F**; **Windows-1252 mapping for 0x80–0x9F** so `€`, en/em dash and curly quotes show
  properly. Default selection is code point `0x21`; the hex is upper-case, `padStart(4,'0')`.

### 8.5 Sound Recorder (`VoiceRecorder.tsx`, `VoiceRecorderMenu.tsx`)

* Window title: **`Sound - Sound Recorder`** (the document name followed by the program name); About
  dialog title **`Sound Recorder`**; default position `(400, 150)`; maximise enabled.
* Menu (exact, with mnemonics):
  * **File** — `New` · `Open...` · `Save` · `Save As...` · `Revert...` (dis) · `Properties` · — · `Exit`
  * **Edit** — `Copy` Ctrl+C (dis) · `Paste Insert` Ctrl+V (dis) · `Paste Mix` (dis) · — ·
    `Insert File...` (dis) · `Mix With File...` (dis) · — · `Delete Before Current Position` (dis) ·
    `Delete After Current Position` (dis) · — · `Audio Properties` (dis)
  * **Effects** — `Increase Volume (by 25%)` · `Decrease Volume` · — · `Increase Speed (by 100%)` ·
    `Decrease Speed` · — · `Add Echo` · `Reverse`
  * **Help** — `Help Topics` (dis) · — · `About Sound Recorder`
* Body: two readouts `Position:` and `Length:` each followed by `n.nn sec.` (2 decimals) with a
  **140 × 40 px** waveform canvas between them; a scrubber slider (`step .01`, disabled while recording
  or when empty); five transport buttons in order **Prev · Next · Play · Stop · Record**
  (`aria-label`s, 0.5 s skip). Record pauses and resumes into the same take.
* Effects: volume ±25 %, speed ×2 / ×0.5 (true resampling — pitch *and* duration), echo (0.3 s delay /
  0.5 feedback), reverse; all re-encoded as WAV.
* Save As defaults to **`Sound.webm`**; Open filter `Sounds (*.wav, *.mp3)` / `All Files`.
* Unsaved-changes dialog: title bar override **`Sound Recorder`**, message lines
  **`The current sound has changed.`** / **`Do you want to save the changes?`** with **Yes / No / Cancel**;
  opening it plays the exclamation sound.
* Nothing here needs a microphone to be faithful if the shell is honest about it (our assets/behaviour
  policy in `PLAN.md` §8.2 — "working subset" labelling).

### 8.6 Windows Picture and Fax Viewer (`PictureFaxViewer.tsx`)

* **Window title: `<file name> - Windows Picture and Fax Viewer`** (hyphen-as-dash as written).
  Default position `(200, 100)`; has minimise / maximise(→fullscreen) / close and a system menu opened
  from the Picture-and-Fax-Viewer icon.
* Bottom **viewer button strip**, in order (with each button's exact tooltip — including the upstream
  typos, kept here because we're transcribing, not proof-reading):

| Order | aria-label | Tooltip |
|---|---|---|
| 1 | Previous image | `Preious Image (Left Arrow)` *(sic)* |
| 2 | Next image | `Next Image (Right Arrow)` |
| — | separator | |
| 3 | Show at a full size | `Actual Size (Ctrl+A)` |
| 4 | Fit the window | `Best Fit (Ctrl+B)` |
| 5 | Slideshow | `Start Slideshow (F11)` / `Stop Slideshow (F11)` |
| — | separator | |
| 6 | Zoom in | `Zoom In (+)` (disabled at 3×) |
| 7 | Zoom out | `Zoom Out (-)` (disabled at 0.25×) |
| — | separator | |
| 8 | Rotate right | `Rotate Clockwise (Ctrl+K)` |
| 9 | Rotate left | `Rotate Conter Clockwise (Ctrl+L)` *(sic)* |
| — | separator | |
| 10 | Delete | `Delete (Delete)` → `accessDenied` error |
| 11 | Print | `Print (Ctrl+P)` → `printerConnect` error |
| 12 | Save | `Copy To (Ctrl+S)` → `accessDenied` error |
| 13 | Open for editing | `Closes this program and opens the image for editing (Ctrl+E)` |
| — | separator | |
| 14 | Help | `Help (F1)` → `accessDenied` error |

* **Slideshow overlay** buttons, pulled to the image's top-right corner: `Start Slideshow`,
  `Pause Slideshow`, separator, `Preious Picture` *(sic)*, `Next Picture`, separator,
  `Close the Window`. Auto-advance every **3 000 ms**, looping; controls auto-hide after **3 000 ms**
  of inactivity and reappear on mouse move; slideshow state adds `picturev-window--slideshow` and goes
  fullscreen-ish (`position/inset` reset, no title-bar offset).
* Zoom: `0.25 → 3` in `0.25` steps; rotation stored per image id (±90°); `fitMode` is `fit` (default) or
  `actual`.
* Keyboard (only while this window is active, and never while an input/textarea/contentEditable has
  focus): `←`/`→` navigate · `F11` toggle slideshow · `Esc` exit (closes if it was started in slideshow
  mode) · `+`/`=` zoom in · `-` zoom out · `Ctrl+A` actual size · `Ctrl+B` best fit · `Ctrl+K` rotate CW ·
  `Ctrl+L` rotate CCW · `Ctrl+P` print error · `Ctrl+S` save error · `Ctrl+E` open in Paint (then close).
* Called from File Manager's **Picture Tasks** ("View as a slide show") and from image thumbnails.

### 8.7 Outlook Express (`OutlookExpress.tsx` + README)

* Window title: **`Outlook Express`**; About title **`Outlook Express`**; **1 200 ms** loading screen
  (`OutlookLoading`) first; default position `(400, 150)`.
* Window title/duty behaviour (from the README, not re-verified in the fetched file):
  unread counts in the folder tree **and the window title**; read state persisted in `localStorage`.
* Menu bar (File, Edit, View, Tools, Message, Help — mnemonics and nested submenus), a context-aware
  toolbar (**Reply / Reply All / Forward / Print / Delete** appear only once a folder is open), a
  **Create Mail** split button with a **Stationery** dropdown (six designs + `Blank`), `Select
  Stationery` dialog and `Send Web Page` dialog, a **Window Layout** dialog toggling the **Contacts** and
  **Folder List** panes, an **Identities** dropdown (Switch/Add/Manage Identities), a Find bar, and a
  **Tip of the Day** panel with animated Previous/Next transitions.
* Local folders: **Local Folders** overview table (folder / unread / total) over
  **Inbox, Outbox, Sent Items, Deleted Items, Drafts**; the message list shows **From**
  (Inbox/Deleted) or **To** (Sent/Drafts/Outbox) and has a **Flag** column on Inbox/Deleted Items;
  envelope read/unread icons; a preview pane whose links open in Internet Explorer.
* "Go directly to my Inbox" is an optional, persisted startup setting.
* Compose window is **independent** of the main window (draggable, minimise, maximise/fullscreen):
  `Apply Stationery` (Leaves, Citrus Punch, Sunflower, Maize, Clear Day, Nature, or Blank),
  **Bcc** toggle, `Insert Signature`, `Set Priority` (High/Normal/Low), a *visually disabled* formatting
  toolbar (font, size, bold/italic/underline, alignment, lists), and a full
  File / Edit / View / Insert / Format / Tools / Message / Help menu bar.
* Sending is real, via EmailJS, to `webmaster@alenanet.com`; the sent item is appended to Sent Items with
  `To: webmaster@alenanet.com`.

### 8.8 Command Prompt (`Terminal.tsx`, `TerminalWindow.tsx`)

* **Window title: `C:\WINDOWS\system32\cmd.exe`** with the MS-DOS icon; classic (non-Luna) chrome
  (`xp-classic-window`); default position `(400, 150)`.
* Font **Lucida Console**, width **80 ch**, **25 visible lines**; prompt literal `C:\>`; auto-scroll to
  bottom; a `is-scrollable` class is toggled when output overflows; default colours
  background **`#000000`**, foreground **`#c0c0c0`**.
* `color` palette map (exact hexes): `0 #000000` · `1 #000080` · `2 #008000` · `3 #FF69B4` ·
  `4 #800000` · `5 #962EFF` · `6 #FF8000` · `7 #c0c0c0` · `8 #808080` · `9 #FF0000` · `a #00ff00` ·
  `b #00ffff` · `c #FF00FF` · `d #8B4513` · `e #ffff00` · `f #ffffff`.
* Commands and their verbatim output:

| Command | Output |
|---|---|
| `help` | `Available commands:` then the list `help - Show this help`, `cls - Clear screen`, `dir - List files`, `echo - Print text`, `ver - Show version`, `date - Show date`, `time - Show time`, `exit - Close window`, `systeminfo - Show system information`, `color - Change terminal color`, then a blank line |
| `ver` | `Microsoft Windows XP [Version 5.1.2600]` |
| `date` | `The current date is: <Www> <dd/mm/yyyy>` / `Enter the new date: (mm-dd-yy)` |
| `time` | `The current time is: HH:MM:SS.00` / `Enter the new time:` |
| `dir` | ` Volume in drive C has no label.` / ` Directory of C:\` / one row per app `28/04/2003 09:00 <name padded to 25> <size padded to 10> bytes` / ` 0 File(s)               0 bytes` / `N Dir(s)           X,XXX bytes free` |
| `echo` | `ECHO is on.` |
| `echo <text>` | the text |
| `color` | `Sets the default console foreground and background colors.` + `COLOR [attr]` + the `0 = Black 8 = Dark Gray` … `7 = Gray F = White` table + `Example: color 0A (black background, green text)` |
| `color XY` | sets both colours; bad code ⇒ `Invalid color code.` |
| `systeminfo` | `Host Name: ALENA-PC` / `OS Name: Microsoft Windows XP Professional` / `OS Version: 5.1.2600 Service Pack 2` / `OS Manufacturer: Microsoft Corporation` / `System Type: X86-based PC` / `Total Physical Memory: 512 MB` / blank / `--- Developer Info ---` / `Name: Alena Pumprová` / `Role: Frontend Developer` / `Skills: React, TypeScript, CSS` / `GitHub: github.com/Alena0490` / `Portfolio: alena-pumprova.cz` |
| `exit` | closes the window |
| anything else | `'<cmd>' is not recognized as an internal or external command,` / `operable program or batch file.` |

The `dir` file list is derived from `TERMINAL_APPS` in `src/data/appData.tsx` — 27 rows, each with the
size string used above: `Login Screen 7,190` · `Minesweeper 171,420` · `Internet Explorer 1,520,050` ·
`Paint 229,210` · `Calculator 26,530` · `Command Prompt 10,740` · `Loading Screen 3,020` ·
`Start Menu 24,280` · `Taskbar 49,950` · `Error Bubble 1,940` · `Critical Error 7,900` ·
`Notepad 45,120` · `Shutdown Screen 22,810` · `Shutdown Display 3,730` · `File Manager 281,910` ·
`Windows Media Player 1,028,380` · `Solitaire 178,000` · `Display Properties 54,920` ·
`On-Screen Keyboard 272,500` · `Volume Control 25,420` · `WordPad 147,960` · `Windows Plus! 48,700` ·
`Character Map 97,380` · `Outlook Express 251,630` · `Windows Picture and Fax Viewer 22,760` ·
`Help and Support Center 282,940` · `Sound Recorder 30,790`.

> These numbers are useful twice: as `dir` output **and** as the File Manager "size" column.

### 8.9 Run dialog (`runDialog/Run.tsx`)

* **Window title: `Run`** (the pranx simulator labels its own copy `Run Dialog` — that is *their*
  caption, not XP's). Title bar has only **Help** and **Close**. Default position `(450, 50)`.
* Body: Run icon; text **`Type the name of a program, folder, document, or Internet resource, and Windows
  will open it for you.`**; label **`Open:`** with a text input (`autoFocus`) and a dropdown arrow whose
  history is pre-seeded with **`notepad`, `calc`, `mspaint`**; buttons **`OK` · `Cancel` · `Browse...`**.
* Command table (exact, lower-cased before comparison): `notepad`, `calc`, `mspaint`, `iexplore`, `cmd`,
  `explorer`, `minesweeper`, `solitaire`, `wmplayer`, `desk.cpl`, `osk`. Enter also triggers OK; the
  dialog closes after any command.

### 8.10 Volume Control (`VolumeControl.tsx`)

* Window title **`Volume Control`** with the speaker icon; About dialog titled **`Volume Control`**;
  default position `(400, 150)`; **maximise disabled**.
* Master row labelled **`Volume Control`** with the mute label **`Mute all`**, then four mixer channels in
  order: **`Wave`**, **`SW Synth`**, **`Line In`**, **`CD Audio`** — each with volume slider, balance and
  Mute. Footer device name: **`Creative Sound Blaster PCI`**.
* Behaviour: `Wave` drives all system sounds, `CD Audio` drives Windows Media Player playback.
* Also reachable as a small pop-up from a single click on the tray speaker (250 ms delay), while
  double-click opens this full mixer. Menu: Options / Help with mnemonics + an About dialog.

### 8.11 On-Screen Keyboard, Solitaire, WordPad, Help & Support, Paint, Notepad, Minesweeper

Present in the upstream repo and described in its README, but **out of scope for this spec** (they are
either already in our repo or already itemised in `docs/FEATURE-GAPS.md`). Per-app CSS/geometry for them
was not fetched. See `src/components/{keyboard,solitaire,wordpad,help,paint,notepad,minesweeper}/` in the
upstream repo if a card is reopened.

### 8.12 Apps the task asked about that **no fetched source implements**

| App | Status in this survey | Nearest verified source |
|---|---|---|
| **HyperTerminal** | **Not present.** Alena's Start menu `Communications` submenu is literally `(Empty)`; no terminal-emulator app exists in any fetched source. Our `FEATURE-GAPS.md` §7 already calls it an inert row. | None — **[unverified]**; do not invent strings, or take them from a VM screenshot. |
| **Clock** | **No standalone Clock app.** Time is only the taskbar clock (§2) plus the tray tooltip with the hard-coded year 2003. Alena's Control Panel has no *Date and Time* applet either (the closest is the inert `Date, Time, Language, and Regional Options` folder). | Taskbar clock spec in §2; for a Date/Time applet use `wieslawsoltes/WinXp` `datetime` (`FEATURE-GAPS.md` §2). |
| **Clipboard Viewer** | **Not present** in any fetched source. Clipboard is only driven implicitly (IE address-bar Cut/Copy/Paste, Character Map `Copy`, Calculator `Copy`/`Paste`). | None — **[unverified]**. |
| **Add or Remove Programs** | Present only as an inert Control Panel entry (§8.1). | Reborn XP `appwiz.js` in `FEATURE-GAPS.md` §1. |

---

## 9. What this changes for our roadmap (short)

1. **New rows for `docs/FEATURE-GAPS.md`** (they are absent today): the **Plus! control panel**
   (§5), **3D Pipes** (§6 — including the asset-but-not-wired discrepancy), and the **Security Center
   balloon's exact 5 s / 15 s timing and geometry** (§3).
2. **Upgrades existing rows with exact strings**: Start menu fly-outs and tooltips (§1), taskbar menus,
   Quick Launch and tray tooltips (§2), Display Properties tab copy + the 28-wallpaper list (§8.2),
   IE menu/toolbar/status/error copy (§7.1), Sound Recorder and Picture & Fax Viewer menus and tooltips
   (§8.5–8.6), Command Prompt output (§8.8), Run dialog (§8.9).
3. **Everything numeric in §0 should land in `theme.rs`** as named constants with the source path in a
   comment, per the workflow in `PLAN.md` §5.
4. **Do not vendor** Alena's icons, sounds, WMP skins, visualization `mp4`s or screensaver `mp4`s —
   they are Microsoft assets, excluded from her MIT licence. Keep the `assets-local/` drop-in policy.

---

## 10. Explicitly unverified / not retrieved

| Item | Why |
|---|---|
| `https://alena0490.github.io/Windows-XP/` rendered UI | SPA: `fetch` returns `<div id="root"></div>` plus the preload manifest (30 per-app chunks). No text, no geometry. Recorded here rather than guessed. |
| `https://win32.run/` desktop & app list | Returns only the Award BIOS POST text (`Award Modular BIOS v6.00PG… ASUS P4S800-MX… WDC WD400BB-00JRA0…`). Recorded, not guessed. |
| Window pixel sizes for WMP / IE / Calculator / Character Map / Sound Recorder / Outlook / Picture Viewer / Volume Control / Run / Command Prompt | They live in the per-app CSS files (`MediaPlayer.css`, `IEWindow.css`, `Calculator.css`, …) which were **not** fetched in this pass. Only the shared chrome (`App.css`), the Start menu (380 × 478) and Plus! (795 × 563) sizes were captured. **Read the per-app CSS before coding each window.** |
| Control Panel category-view chrome | Headings such as `Pick a category`, task-pane strings and icon sizes are not in any fetched source (see §8.1). |
| Native **3D Pipes** setup dialog strings | Only the pranx marketing page (§6) and Alena's unused `Windows 3D Pipes Screensaver.mp4` exist; the real `3D Pipes Setup` labels are **not** sourced. |
| IE7 chrome metrics | Alena's IE is `IE6`-era (its menu even lists the SP2 `Pop-up Blocker` and `Manage Add-ons...`); pranx only supplies the label `Internet Explorer - IE7`, the six menu names, `Address`/`Go`/`Links` and its own taskbar inventory (§7.2). No measurements. |
| Firewall balloon text | Only the antivirus Security Center balloon exists in the sources (§3). |
| Windows Firewall / Security Center Control Panel applet | Not implemented in any fetched source. |
| Exact `Initial $Windows`/OOBE, MSN Explorer and Windows Messenger chrome | Present only as Start-menu labels (per the README/pranx list), no app source fetched. |
