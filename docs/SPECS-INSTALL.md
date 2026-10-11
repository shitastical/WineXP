# SPECS-INSTALL.md — the Windows XP installation sequence as implemented by **win32.run**

> **Not the shipped flow.** The demo's "Reinstall Windows XP" follows **Reborn XP** (`xp.quenq.com`)
> instead — confirmation → `Formatting virtual disks...` → `Restarting computer...` → reload, then the
> `msoobe` OOBE wizard (see `docs/SPECS-BIOS.md` §4/§8 and `src/screens/{install,oobe}.rs`). This
> transcription is kept as the reference for the deeper text-mode Setup (partition/format/copy) if
> that flow is added later.

Implementation-ready transcription of the full XP Setup sequence in
[`ducbao414/win32.run`](https://github.com/ducbao414/win32.run) (MIT), for re-implementation as a
purely **visual, non-destructive** animation in this Rust project.

Every string below is copied verbatim from the fetched source (including upstream typos, each marked
`[sic]`). Every duration is either read from the source or **computed from it and marked
`[inferred]`**. Everything the task asked about that win32.run does **not** implement is listed in
§8 rather than invented.

Companion docs: `docs/PROVENANCE.md` (module → source map), `docs/FEATURE-GAPS.md`,
`docs/SPECS-SYSTEM.md`. This file supersedes the win32.run notes in those docs for the install flow.

---

## 0. Sources actually fetched (and their status)

| # | Source | URL | Gave | Status |
|---|---|---|---|---|
| 1 | Repo page | `https://github.com/ducbao414/win32.run` | Tree, licence (MIT), README, "archived Mar 8 2026" | 200 |
| 2 | README (raw) | `.../main/README.md` | Build stack (Svelte/SvelteKit/Tailwind), IndexedDB FS | 200 |
| 3 | Landing HTML | `https://win32.run/` | Static BIOS POST markup (`#pos_loader > #bios`), the 5-circle orbit loader, the asset-preload list, SvelteKit chunk names | 200 (SPA DOM after mount renders nothing without JS) |
| 4 | `src/routes/index.svelte` | raw | **The route allow-list** — the definitive list of install screens | 200 |
| 5 | `src/routes/boot_manager.svelte` | raw | Boot-device menu + `Install Windows` branch | 200 |
| 6 | `src/routes/installation/dos/starting.svelte` | raw | Text-mode loading screen + status-bar message timeline | 200 |
| 7 | `.../dos/welcome.svelte` | raw | "Welcome to Setup." | 200 |
| 8 | `.../dos/eula.svelte` | raw | "Windows XP Licensing Agreement" | 200 |
| 9 | `.../dos/partition.svelte` | raw | File-system / format selection | 200 |
| 10 | `.../dos/format.svelte` | raw | "Please wait while Setup formats the partition" | 200 |
| 11 | `.../dos/copying.svelte` | raw | "Please wait while Setup copies files" | 200 |
| 12 | `.../dos/restart.svelte` | raw | "This portion of Setup has completed successfully." | 200 |
| 13 | `src/routes/installation/95/installing.svelte` | raw | GUI "Installing Windows" phase (steps, phases, intro slides) | 200 |
| 14 | `.../95/product_key_windows.svelte` | raw | "Your Product Key" modal | 200 |
| 15 | `src/routes/xp/starting.svelte` | raw | XP boot-logo loader; routes to GUI install or desktop | 200 |
| 16 | `src/routes/xp/welcome.svelte` | raw | "Welcome" splash | 200 |
| 17 | `src/routes/xp/desktop.svelte` | raw | Mounts `Welcome` + desktop | 200 |
| 18 | `src/lib/utils.js` | raw | `random_files`, `random_int`, `sleep`, `is_installing_windows`, theme setter | 200 |
| 19 | `src/lib/components/dos/status_bar.svelte` | raw | The bottom status bar used by every text-mode screen | 200 |
| 20 | `src/lib/components/xp/RadioBtn.svelte` | raw | The step-list radio indicator | 200 |
| 21 | `src/lib/components/95/{Window,TitleBar,Button}.svelte` | raw | Product-Key dialog chrome | 200 |
| 22 | `tailwind.config.cjs`, `src/app.css` | raw | Fonts, colour tokens | 200 |
| 23 | `https://win32.run/files/EULA.txt` | site | The exact EULA body shown on the licence screen | 200 |

### Fetched but NOT retrievable / not used
- `src/routes/__layout.svelte` — **could not be fetched** (HTTP 206 / tool error). It is not needed:
  the page loader (`index.svelte`) and `app.html` fully describe the routing.
- `https://raw.githubusercontent.com/.../files/EULA.txt` via raw — the raw path returned the file, but
  the **site copy at `https://win32.run/files/EULA.txt` was used** instead because that is exactly
  what `eula.svelte` fetches at runtime.
- **GitHub code search** for strings such as `"Installing Windows Updates"` — **blocked**
  ("Sign in to search code"). Absence of those strings is therefore established from the fetched
  source of the install route + the `index.svelte` route allow-list (§8), not from a repo-wide search.

**Archive note:** the repo is read-only since 2026-03-08; a refactored Svelte-5 version exists at
`ducbao414/win32.run.cf` (not inspected — out of scope for this task).

---

## 1. How the sequence is driven (mechanics you must reproduce)

* **Single-page route loader.** `src/routes/index.svelte` holds one `<svelte:component this={page}>`.
  Every screen dispatches `load_page` with a hard-coded relative URL; `load_page()` is a hand-written
  `if/else` chain of dynamic `import()`s (Vite cannot import by variable). **The install flow is
  exactly the following allow-listed set** — this is why §8 can state what does *not* exist:

  ```
  './boot_manager.svelte'                     './xp/starting.svelte'
  './installation/dos/starting.svelte'        './xp/desktop.svelte'
  './installation/dos/welcome.svelte'         './xp/program_tray.svelte'
  './installation/dos/eula.svelte'            './xp/system_tray.svelte'
  './installation/dos/partition.svelte'       './xp/task_bar.svelte'
  './installation/dos/format.svelte'          './xp/wallpaper.svelte'
  './installation/dos/copying.svelte'         './xp/shutdown.svelte'
  './installation/dos/restart.svelte'         './xp/blackout.svelte'
  './installation/95/installing.svelte'       './xp/programs/display_properties.svelte'
  './installation/95/product_key_windows.svelte'  './xp/programs/internet_explorer.svelte'
                                              './xp/programs/my_computer.svelte'
  ```
  (`xp/welcome.svelte` is **not** in this list — it is imported directly by `xp/desktop.svelte`.)

* **Global install flag.** `utils.set_installing_windows(bool)` writes
  `localStorage['is_installing_windows']`. `boot_manager` sets it `true` when "Install Windows" is
  chosen; `95/installing.svelte` sets it `false` when the GUI phase finishes. `xp/starting.svelte`
  reads it to decide `installation/95/installing.svelte` vs `xp/desktop.svelte`.

* **Key handling.** Svelte `<svelte:window on:keydown|preventDefault={...}>`; the switch is on
  `e.keyCode`: **38 = ↑, 40 = ↓, 13 = Enter, 27 = Esc** (82 = `R` is mentioned in copy but **never
  wired** — see §3.3).

* **Theme.** `boot_manager` calls `utils.set_theme('none')`, setting `<link id="theme" href="">` (no
  `xp.css`). The text-mode Setup therefore uses **plain Tailwind classes and the bundled fonts**, not
  the `xp.css` sheet.

* **Page zoom.** `index.svelte`'s `adjust_zoom()` sets `document.body.style.zoom` to `0.8` when
  `width ≤ 800 || height ≤ 500`, `0.9` when `width ≤ 900 || height ≤ 600`, else `1`. All metrics below
  are **CSS px at zoom = 1**.

* **Document title** throughout: `Microsoft Windows XP Professional`.

* **Two status-bar fonts, two suites.** Text-mode screens use `font-Levi` (the DOS "Levi" face) and a
  `StatusBar` bottom strip in `font-MSSS`; the GUI screen and the 95-style dialog use `font-MSSS`.

---

## 2. Ordered screen list (the deliverable)

Screens 0–14 run in order when "Install Windows" is chosen at the boot menu. Titles are the exact
on-screen strings.

### 0. BIOS POST (static HTML, before Svelte mounts) — **automatic**
* **Owner:** `src/app.html`, `#pos_loader > #bios`. Shown while `/images/*` and `/fonts/*` preload;
  hidden by `document.querySelector('#pos_loader').style.display = 'none'`.
* **Title/layout:** black (`background:#000`), `color:#ccc`, `font-family: monospace`, left-aligned
  block starting ~48 px from the top (`mt-12 ml-8`), `text-lg`.
* **Exact lines** (`<p>` per line, `<br/>` between groups):
  ```
  Award Modular BIOS v6.00PG, An Energy Star Ally
  Copyright (C) 1984-2003, Award Software, Inc.

  ASUS P4S800-MX ACPI BIOS Revision 1008
  Copyright (C) 2003, ASUSTeK COMPUTER INC.

  Main Processor : Intel(R) Pentium(R) 4 CPU 2.40GHz
  Memory Testing : 524288K OK

  IDE Channel 0 Device 0 :  WDC WD400BB-00JRA0
  IDE Channel 1 Device 0 :  ASUS DVD-E616A3

  Verifying DMI Pool Data ...
  ```
  The trailing `...` is `<span class="blink">` — `@keyframes blink-animation { to { visibility:hidden } }`,
  `animation: blink-animation 1s steps(5,start) infinite`.
* **`#loader`** (hidden by default; `display:none`) — the "orbit" loader ported from a CodePen:
  5 `.circle` divs, each 38×38 px, `transform: rotate(225deg)`, 5 px white dot `:after`, `orbit`
  keyframes over **5.5 s** with per-child delays **0 / 240 / 480 / 720 / 960 ms**. `padding-top:100px`,
  fixed width 40 px, centred.
* **Duration:** `[inferred]` width of the asset preload (`xp_loading_logo.jpg`, `xp_loading_mslogo.jpg`,
  `xp_logo_horizontal.png`, `xp_logo.png`, `levi.ttf`, `ms_sans_serif_bold.ttf`, `ms_sans_serif.ttf`).

### 1. Boot-device menu — `src/routes/boot_manager.svelte` — **interactive**
* **Layout:** full-screen black (`w-screen h-screen bg-black`), `font-MSSS`, block at `mt-12 ml-8
  text-lg` (≈48 px top, 32 px left). Two instruction lines in `text-slate-100`:
  ```
  Use the ↑(Up) and ↓(Down) key to move the pointer to desired boot device.
  Press (Enter) to attempt to boot or ESC to cancel.
  ```
* **Non-Chromium warning** (only when `window.chrome == null`), `mt-2 max-w-[500px]`:
  ```
  WIN32.RUN might have unexpected behaviors on browsers that are NOT Chromium-based (Safari, Firefox, Internet Explorer, etc.)
  ```
* **Two headings**, uppercase, `text-slate-100 uppercase mt-4 mb-2`:
  `BOOT OPTIONS:` (entries 0–3) and `OTHER OPTIONS:` (entries 4–7).
* **Options array, verbatim** (each rendered `ml-8 p-2 inline-block`):
  ```
  0  Start Windows Normally
  1  Install Windows
  2  Onboard NIC (IPV4)
  3  Onboard NIC (IPV6          [sic — missing ")"]
  4  BIOS Setup
  5  Device Configuration
  6  BIOS Flash Update
  7  Change Boot Mode Settings
  ```
* **Selection:** index `current_option` (starts **0**). Highlighted row = `text-slate-900 bg-slate-200`;
  others `text-slate-300`. ↑/↓ wrap around the full 8-entry list.
* **Enter:** only entries **0** and **1** do anything.
  * 0 → `set_installing_windows(false)` → `./xp/starting.svelte`
  * 1 → `set_installing_windows(true)`  → `./installation/dos/starting.svelte`  ← **the install path**
  * 2–7 → no-op (a `selected` guard prevents double-firing).
* **Note:** rows are not clickable (the `on:click` handlers are commented out) — keyboard only.

### 2. Text-mode loading — `src/routes/installation/dos/starting.svelte` — **automatic**
* **Title/layout:** full-screen `bg-[rgb(2,7,176)]` (**#0207B0**), `font-Levi`. Header block
  `mt-4 text-xl font-bold text-slate-400 inline-block`:
  ```
  Windows XP Professional Setup
  ─────────────────────────────
  ─────────────────────────────
  ```
  (the wordmark, then **two** 1-px `bg-slate-400` rules — `mb-1` on the first). No body text.
* **Status-bar timeline** (left field of the shared `StatusBar`), emitted by `status_bar.display(messages)`:
  | # | Left message (verbatim) | Hold |
  |---|---|---|
  | 1 | `Press F6 if you need to install a third party SCSI or RAID Driver` | `d: 2000` ms |
  | 2 | `Press F2 to run Automated System Recovery (ASR)...` | `d: 2000` ms |
  | 3–14 | `Setup is loading files <DIRPATH>...` × **12**, where `<DIRPATH>` = `faker.system.directoryPath()` (e.g. `C:\WINDOWS\system32\...`) | `random(2..7)×50` = 100–350 ms each |
  | 15 | `Setup is starting Windows` | `d: 2000` ms |
* **Then** dispatches `./installation/dos/welcome.svelte`. Keys ignored.
* **Duration:** `[inferred]` ≈ 2.0 + 2.0 + 12×≈0.225 + 2.0 ≈ **8.7 s**.

### 3. "Welcome to Setup." — `.../dos/welcome.svelte` — **interactive (needs Enter)**
* **Same blue #0207B0 + double-rule header**, header text `Windows XP Professional Setup`.
* **Body** (`ml-12 mt-8 text-xl`), left-aligned, then a `list-disc` bullet list (`mt-6 ml-12`):
  ```
  Welcome to Setup.
  This portion of the Setup program prepares Microsoft(R) Windows(R) to run on your computer.

    • To setup Windows XP now, press ENTER.
    • To repaire a Windows installation using Recovery Console, press R.      [sic "repaire"]
    • To quite Setup without installing Windows XP, press ESC.               [sic "quite"]
  ```
  (`Welcome to Setup.` is `text-slate-50`; the rest `text-slate-400`.)
* **Status bar (left field):** `ENTER=Continue   R=Repair   ESC=Quit` (rendered with `&nbsp;` separators).
* **Keys:** `Enter (13)` → `./installation/dos/eula.svelte`; `Esc (27)` → `./boot_manager.svelte`.
  **`R` is displayed but not handled** — ↑/↓/`R` do nothing (§8).
* **Duration:** waits indefinitely.

### 4. "Windows XP Licensing Agreement" — `.../dos/eula.svelte` — **interactive (needs Enter)**
* **Header:** `Windows XP Licensing Agreement` + the same double rule.
* **Body:** fetches `GET /files/EULA.txt`, `text()`, replaces `\n\n` with `<br/>`, renders as HTML into
  a scroller `mt-8 p-4 h-[90vh] text-xl text-slate-200 overflow-hidden relative`; the text block is
  positioned `absolute; style:top="{y_pos}%"`.
  * **Exact EULA body:** the standard XP **"END-USER LICENSE AGREEMENT FOR MICROSOFT SOFTWARE /
    WINDOWS XP PROFESSIONAL EDITION"**, footer id **`EULAID:WX.4_PRO_RTL_EN`**, revision stamp
    **`07/27/2001 54834.1`**, sections 1–19 plus the French *GARANTIE LIMITÉE* block. Captured in full
    from `https://win32.run/files/EULA.txt` (see §9 for the leading paragraphs).
* **Status bar (left field):**
  ```
  ENTER=I agree   ESC=I do not agree   ↓(Down)=Next page   ↑(Up)=Previous page
  ```
* **Keys:** `↓ (40)` → `y_pos -= 100`; `↑ (38)` → `y_pos = min(y_pos + 100, 0)`;
  `Enter (13)` → `./installation/dos/partition.svelte`; `Esc (27)` → `./boot_manager.svelte`.
  There is **no checkbox / no F8** — Enter alone accepts. No scroll gate before Enter.
* **Duration:** waits indefinitely.

### 5. File-system / format selection — `.../dos/partition.svelte` — **interactive**
* **Header:** `Windows XP Professional Setup` + double rule.
* **Body** (`ml-12 mt-8 text-xl text-slate-300`):
  ```
  A new partition for Windows XP has been created on 10237 MB Disk 0 at Id 0 on bus 0 on atapi [MBR].
  This partition must be now formatted.
  From the list below, select a file system for the new partition.
  Use the UP and DOWN ARROW key to select the file system you want to use, then press ENTER.
  ```
* **Options** (`<ul class="ml-4 mt-4">`, selected row gets `bg-slate-300 text-[rgb(2,7,176)]`):
  ```
  0  Format the partition using the NTFS file system (Quick)     ← default selection
  1  Format the partition using the FAT file system (Quick)
  2  Format the partition using the NTFS file system
  3  Format the partition using the FAT file system
  ```
* **Status bar:** `ENTER=Continue   ESC=Cancel`.
* **Keys:** `↑/↓` move within 0..3 (clamped); `Enter` → `./installation/dos/format.svelte`
  (**irrespective of which option is highlighted — the choice is not passed on**);
  `Esc` → `./boot_manager.svelte`.
* **Duration:** waits indefinitely.

> Despite the file name `partition.svelte`, this screen is a **"newly created partition → choose a
> file system"** screen. There is **no** XP partition-list screen (see §8).

### 6. "Please wait while Setup formats the partition" — `.../dos/format.svelte` — **automatic**
* **Header:** `Windows XP Professional Setup` + double rule.
* **Body** (`mt-12 text-xl text-slate-400`, each line centred):
  ```
  Please wait while Setup formats the partition
  C: Partition1 [New (Raw)]               10229 MB (10228 MB free)
  On 10237 MB Disk 0 at Id 0 on bus 0 on atapi [MBR]
  ```
  (the `&nbsp;` run between "New (Raw)]" and "10229 MB" is ~10 spaces in the source).
* **Progress box** — absolutely positioned `bottom-[150px]`, centred, `border-double border-4
  border-slate-400`, `max-w-[700px]`:
  * label (left, `ml-4`): `Setup is formatting...`
  * numeric line (centred): `{progress}%`
  * bar: outer `w-[500px] border-2 border-slate-400 h-[40px]` with `p-[4px]`; inner fill
    `h-full bg-yellow-300` (**#FDE047**) width `{progress}%`; `transition-all`.
* **Automation loop:** `while (progress < 100) { await sleep(random(1..3)×300); progress =
  min(100, progress + random(5..20)); }`, then `await sleep(1000)` → `./installation/dos/copying.svelte`.
* **Status bar:** present but empty (`l_message=""`).
* **Duration:** `[inferred]` ≈ 8 steps × ~0.6 s + 1 s ≈ **5.5–6.5 s**.

### 7. "Please wait while Setup copies files" — `.../dos/copying.svelte` — **automatic**
* **Header:** `Windows XP Professional Setup` + double rule.
* **Body** (`mt-12 text-xl text-slate-400`, centred):
  ```
  Please wait while Setup copies files
  to the Windows installation folders.
  This might take several minutes to complete.
  ```
* **Progress box** (same chrome as §6): label `Setup is copying files...`, `{floor(progress)}%`, the
  same `bg-yellow-300` (#FDE047) fill bar.
* **Status bar:** `show_separator={true}` (**adds a 4-px `bg-slate-900` divider** between fields).
  The **right** field is live-driven: `status_bar.r_message = 'Copying: ' + utils.random_files()`,
  refreshed every `random(1..30)×10` = 10–300 ms. `random_files()` = a word from a ~1 500-entry
  Windows-command word list + `.` + one of `dll|icm|sys|inf|bin|exe` (e.g. `Copying: format.sys`).
* **Two concurrent loops:**
  * `update_progress`: `while (progress<100){ await sleep(random(1..10)×10); progress =
    min(100, progress + random(1..10)/12); }` → `./installation/dos/restart.svelte`.
  * `copy_files`: infinite, updates the right status message.
* **Duration:** `[inferred]` ≈ 218 iterations × ~55 ms ≈ **12–15 s**.

### 8. "This portion of Setup has completed successfully." — `.../dos/restart.svelte` — **semi-interactive (15 s countdown)**
* **Header:** `Windows XP Professional Setup` + double rule.
* **Body** (`ml-8 mt-12 text-xl text-slate-400`):
  ```
  This portion of Setup has completed successfully.
  If there is a floppy disk in drive A:, remove it.
  To restart you computer, press ENTER.              [sic "you computer"]
  When your computer restarts, Setup will continue.
  ```
* **Countdown box** (`bottom-[150px]`, `border-double border-4 border-slate-400 pt-8 max-w-[700px]`):
  ```
  Your computer will reboot in {N} seconds...        (singular "second" when N == 1)
  ```
  bar fill here is **`bg-red-500` (#EF4444)**, width `{100*(15-time_left)/15}%`.
* **Automation:** `time_left` starts **15**, decremented every 1000 ms; at 0 → `./xp/starting.svelte`.
* **Status bar:** `ENTER=Restart Computer`; `Enter` also → `./xp/starting.svelte` immediately.
* **Duration:** **15 s** (or Enter).

### 9. XP boot-logo loader (first reboot) — `src/routes/xp/starting.svelte` — **automatic**
* **Layout:** full-screen `bg-black text-slate-100`.
  * Centre (`top-50% left-50%` translated), `animate-fadein` (**fade-in over 5 s**):
    `/images/xp_loading_logo.jpg` at **width 400 px**, then the `.xp-loader`.
  * `.xp-loader`: **150 × 20 px**, `border: 2px solid #b2b2b2`, `border-radius: 7px`, centred,
    `margin-top: 150px`. Three `div`s, each **9 px** wide, `margin-right: 2px`, background
    `linear-gradient(to bottom, #2838c7 0%, #5979ef 17%, #869ef3 32%, #869ef3 45%, #5979ef 59%,
    #2838c7 100%)`, `animation: loader 2s infinite linear` (`translate(-30px)` → `translate(150px)`).
  * Bottom-left (`left-8 bottom-8 text-base font-sans`): `Copyright © Microsoft Corporation`.
  * Bottom-right: `/images/xp_loading_mslogo.jpg`, width 120 px.
* **Automation:** on mount, load `hard_drive` + `wallpaper` (IndexedDB), preload all images/audio/
  fonts, then `do { await sleep(5000); wait_count++; if (wait_count>=2) break; } while(!assets_loaded)`.
  Then preload iframes and context menus, and route on the flag:
  * `is_installing_windows() === true` → `./installation/95/installing.svelte` (**this is the path
    taken during install**, because the flag is still set).
  * `false` → `./xp/desktop.svelte`.
* **Duration:** **5 s minimum, 10 s maximum** (two 5 s waits).

### 10. GUI "Installing Windows" phase — `src/routes/installation/95/installing.svelte` — **automatic** (~7 min)
> Folder is named `95` (it reuses the Win-95-style `Window`/`Button`, hence the Product-Key dialog
> chrome), but the content is explicitly **Windows XP**. This is the single longest screen.

* **Background palette:** page `bg-[#5a7edc]` (**#5A7EDC**). Top band **70 px** `bg-[#00309c]` (**#00309C**)
  containing `/images/xp_logo_horizontal.png` at **height 40 px**, `ml-4`.
* **Separator 1** — 2 px, `linear-gradient(45deg,#466dcd,#c7ddff,#b0c9f7,#5a7edc)`.
* **Main band** — left column background is a radial highlight
  `radial-gradient(circle at 5% 5%, #91b1ef 0, #7698e6 6%, #5a7edc 12%)`; right pane is
  `linear-gradient(#5a7edc, #7698e6, #5a7edc)`.
* **Left column** (`w-1/5 min-w-[300px] pt-12`, `pointer-events-none`) — the checklist, each row
  `mb-4 ml-12 mr-20 text-white font-bold text-base` rendered with `RadioBtn size={20}`:
  | Step label (verbatim) | Initial state |
  |---|---|
  | `Collecting information` | ✔ done (2) |
  | `Dynamic Update` | ✔ done (2) |
  | `Preparing Installation` | ✔ done (2) |
  | `Installing Windows` | ⟳ in progress (1) |
  | `Finalizing installation` | ○ not yet (0) |
  When the phase counter passes index 5, `Installing Windows` becomes done and
  `Finalizing installation` becomes in-progress.
  * `RadioBtn` visual: 20×20 px circle, `linear-gradient(135deg,#dcdcd7,#fff)` with `border:1px
    solid #1d5281`; hover/in-progress shows an **orange** overlay (`from-orange-300 to-orange-200`);
    the tick is the sprite `/images/xp/radio_check.png`. In-progress label turns `text-orange-400`.
  * Below the list: `Setup will complete in approximately:` (`font-bold text-base mt-6`), then
    `{time_left} minutes` (or `1 minute`), then the current phase name
    (`mt-8 ml-8 text-sm text-white`) and its own bar.
  * Phase bar: `ml-8 mr-10 mt-1 border-[#00309c] border h-6 bg-white p-[1px]`, fill `bg-green-600`
    (**#16A34A**) width `{phase.progress}%`.
* **Right pane** (`p-8 pl-20 pr-24`), rotates a marketing slide every **`4 × one_unit` ≈ 45 s**:
  heading `text-3xl text-white font-sans font-bold` with `text-shadow: 2px 2px #000;`; body
  `text-lg text-white mt-8 max-w-[700px]` (paragraphs separated by `<br/><br/>`). The four slides,
  verbatim:

  1. **An exciting new look** — *"Windows® XP Professional sports a visual design that combines a sleek look, clean lines, and appealing colors with a task-oriented design and exceptionally streamlined navigation.* … *The redesigned Start menu makes it easier to find important information and access the programs you use most frequently.* … *By automatically cleaning up the notification area of the taskbar and grouping related taskbar items, Windows XP makes it easier to switch between programs and to open, view, or close multiple items at the same time."*
  2. **Try the easiest Windows® yet** — *"Windows XP makes it easy to manage all of your information. We've enhanced the My Documents, My Pictures, and My Music folders…"* … *"Integrated support for Web publishing means that you can put your documents on the Internet…"* … *"Windows XP Professional also supports integrated CD recording…"*
  3. **Stay up to date** — *"Windows Update provides you with critical software updates and hardware support files for Windows® XP. And Setup gives you the opportunity to ensure that you have the latest software to install Windows quickly and successfully."* … *"For corporate customers, there have been great improvements to the system preparation tool as well as unattended setups."*
  4. **Your computer will be faster and more reliable** — *"Windows® XP Professional not only starts faster than any previous version of Windows, but it also runs your programs more quickly and reliably than ever. If a program becomes unstable, you can close it without having to shut down Windows or lose any of your work."*

* **Separator 2** — 2 px `linear-gradient(45deg,#003399,#f99736,#c2814d,#00309c)`.
* **Bottom band** — 70 px, `linear-gradient(90deg,#3833ac,#00309c)`; at the right
  (`absolute right-10`), the **"5-dot activity indicator"**: five 7×7 px squares (`border-radius:2px`,
  `margin-right:20px`, background **#16A34A**) pulsing via `@keyframes pulse { 20% { box-shadow: 0 0 0
  6px #10b981 } }` with `animation: pulse 2.5s infinite` and delays 0 / 0.5 / 1 / 1.5 / 2 s.
* **Audio:** `<audio src="/audio/xp_installing.mp3" loop autoplay>` (looped install music).

#### 10a. Progress model (exact)
```
total_duration = 7 × 60 × 1000 ms                 // exactly 7 minutes
one_unit       = total_duration / 37             // ≈ 11 351 ms  ("1 simulated minute")
```
`phases`, in order, with `d` in units (and `[inferred]` wall-clock):
| Phase title (verbatim, note the "..." where shown) | units | ≈ wall-clock |
|---|---|---|
| `Installing Devices` | 6 | 68 s |
| `Installing Network...` | 2 | 23 s |
| `Copying files...` | 5 | 57 s |
| `Completing installation...` | 2 | 23 s |
| `Installing Start menu items` | 2 | 23 s |
| `Registering components` | 10 | 114 s |
| `Saving settings` | 7 | 79 s |
| `Removing any temporary files used` | 3 | 34 s |
| **total** | **37** | **420 s = 7 min** |

Loop: every `sleep(phase.d / 100)` it adds **1 %** to the current phase's progress and the same
`phase.d/100` to `time_passed`; `time_left = ceil(max(total - time_passed, 0)/one_unit)` minutes.
**When phase 0 (`Installing Devices`) reaches 100 %**, it mounts the Product-Key dialog (§11) and
sets `paused = true` — the whole simulation halts until "Next >" is clicked. When `time_passed ≥
total_duration`, it calls `set_installing_windows(false)` and routes to `./xp/starting.svelte`.

### 11. "Your Product Key" modal — `.../95/product_key_windows.svelte` — **interactive (blocks §10)**
* **Container:** a Win-95-style `Window` (`src/lib/components/95/Window.svelte`):
  `position:absolute; background:silver; box-shadow: inset -1px -1px #0a0a0a, inset 1px 1px #dfdfdf,
  inset -2px -2px grey, inset 2px 2px #fff; padding:3px`, **600 × 400 px**, min 600 × 400, centred in
  the parent (dynamically positioned). **minimize / maximize / close are all disabled.**
* **Title bar** (`linear-gradient(90deg, navy, #1084d0)`, `font-MSSS`, `text-white font-semibold`):
  ```
  Windows XP Professional Setup
  ```
* **Content, top to bottom** (all `font-MSSS`):
  * White header strip (`p-2`): left — `Your Product Key` (semibold) and
    `Your Product Key uniquely identifies your copy of Windows XP.`; right — the sticker image
    `/images/95/0.png` at width 50 px.
  * Middle row: a black-framed **yellow sticker** (`bg-yellow-500`, rounded) with a key icon and
    ```
    PRODUCT KEY:
    zzzzz-zzzzz-zzzzz-zzzzz-zzzzz
    ```
    and, to its right:
    ```
    The 25-character Product Key appears on the yellow sticker on the back of your Windows CD folder.

    Type the Product Key below:
    ```
  * Input row: label `Product Key:` + **five** `<input>` boxes, each `h-6 w-[85px]`, `maxlength="5"`,
    centred, uppercase, `background:#fff` with a classic inset bevel
    (`inset -1px -1px #fff, inset 1px 1px grey, inset -2px -2px #dfdfdf, inset 2px 2px #0a0a0a`),
    separated by `-` glyphs. Typing 5 chars auto-focuses the next box. **No validation**.
  * Footer (right-aligned): Button **`< Back`** (`disabled`, width 100 px, `margin-right:10px`) and
    Button **`Next >`** (width 100 px).
* **Buttons** (`95/Button.svelte`): `background:silver`, `box-shadow: inset -1px -1px #0a0a0a,
  inset 1px 1px #fff, inset -2px -2px grey, inset 2px 2px #dfdfdf`, `min 50×20 px`, `font-MSSS text-sm
  text-black`, focus = dotted outline at `-4 px` offset, disabled text `text-gray-500`.
* **Effect:** clicking **`Next >`** destroys the dialog and un-pauses §10 (`paused = false`). The modal
  is the only thing the user touches in the entire GUI phase.

### 12. XP boot-logo loader (second reboot) — `xp/starting.svelte` again — **automatic**
Identical to §9, but now `is_installing_windows() === false`, so it routes to `./xp/desktop.svelte`.
Duration: 5–10 s.

### 13. "Welcome" splash — `src/routes/xp/welcome.svelte` — **automatic (~7 s)**
* Mounted by `xp/desktop.svelte` as an overlay (`z-50`), full screen.
* **Layout:** root `bg-[#5a7edc]`; top band 70 px `bg-[#00309c]` (empty); sep-1 gradient as §10;
  centre band `radial-gradient(circle at 5% 5%, #91b1ef 0, #7698e6 6%, #5a7edc 12%)` containing,
  centred, the text `Welcome` — `text-[42px] text-slate-50 italic font-bold`, at `top-40% left-50%`;
  sep-2 gradient `#003399,#f99736,#c2814d,#00309c`; bottom band 70 px.
  (Note: not "Welcome to Windows" — literally just **`Welcome`**.)
* **Audio & dismissal:** plays `/audio/xp_startup.mp3` on `canplaythrough`; destroys itself when the
  audio `ended`, or after a **7000 ms** fallback `setTimeout`.
* **Status bar: none.**

### 14. Desktop — `src/routes/xp/desktop.svelte` — **interactive**
The install is over: full desktop (`WorkSpace` + `TaskBar` + `StartMenu` + `ContextMenu`).
(Out of scope here; see `docs/FEATURE-GAPS.md`.)

---

## 3. Interactive vs automatic (at a glance)

| # | Screen | Driven by | Advance |
|---|---|---|---|
| 0 | BIOS POST | auto | asset preload completes |
| 1 | Boot menu | **interactive** ↑ ↓ Enter | Enter on "Install Windows" |
| 2 | Text-mode loading | auto (+2 s F6, +2 s F2, 12 file lines, +2 s "starting Windows") | ~8.7 s |
| 3 | Welcome to Setup | **interactive** Enter | user |
| 4 | Licence Agreement | **interactive** ↑ ↓ Enter (Esc) | user |
| 5 | File-system choice | **interactive** ↑ ↓ Enter (Esc) | user |
| 6 | Formatting | auto | progress 0→100, ~6 s |
| 7 | Copying files | auto | progress 0→100, ~13 s |
| 8 | Completed / reboot | **semi** — 15 s countdown or Enter | 15 s |
| 9 | Boot loader 1 | auto | 5–10 s |
| 10 | Installing Windows (GUI) | auto, **paused once** | 420 s + pause at phase 0 end |
| 11 | Your Product Key | **interactive** Next ▸ | user (only click) |
| 12 | Boot loader 2 | auto | 5–10 s |
| 13 | Welcome splash | auto | 7 s (or startup-audio end) |
| 14 | Desktop | interactive | — |

**Only four interactions** exist in the whole flow: boot menu, `Enter` on Welcome, `Enter` on the
licence, `↑/↓+Enter` on the file-system list, and the single **`Next >`** click on the Product Key
dialog. Everything else is timed. (For a non-destructive animation this means you can offer "press
Enter / click Next" as the only keys, and everything else plays itself.)

---

## 4. Progress-bar behaviour (exact)

| Screen | Bar geometry | Fill colour | Fill source | Timing |
|---|---|---|---|---|
| §6 Format | 500×40 px, `border-2 #94a3b8`, `p-4px`, inside a `border-double 4px` box | **`bg-yellow-300` #FDE047** | `progress = min(100, progress + random(5..20))` each `random(1..3)×300 ms` | ~5.5–6.5 s `[inferred]` |
| §7 Copying | identical | **`bg-yellow-300` #FDE047** | `progress += random(1..10)/12` each `random(1..10)×10 ms`; label shows `floor(progress)%` | ~12–15 s `[inferred]` |
| §8 Reboot | identical | **`bg-red-500` #EF4444** | `100*(15-time_left)/15 %` — starts at 0 % and **fills as the countdown runs** (the opposite direction to a "draining" timer) | exactly 15 s |
| §10 Phase bar | `h-6`, `border #00309c`, `bg-white p-1px` | **`bg-green-600` #16A34A** | `phase.progress` 0→100, `+1` per `phase.d/100` ms | per-phase (§10a) |
| §10 "remaining" | text only: `{time_left} minute(s)` | — | `ceil(max(total-time_passed,0)/one_unit)` | recomputed each tick |

Notes: the §6/§7 bars use `transition-all`, so the fill animates smoothly between steps; there is no
indeterminate/`marquee` bar anywhere in the install flow (the only marquee-style motion is the §9
boot loader and the §10 five-dot indicator).

---

## 5. Typography

| Tailwind class | Family | Source (`@font-face`) | Used by |
|---|---|---|---|
| `font-Levi` | `Levi`, monospace | `/fonts/levi.ttf` | **all text-mode Setup screens** (§2–§8) — the DOS-style face |
| `font-MSSS` | `MSSS`, monospace | `/fonts/ms_sans_serif.ttf` (400) + `/fonts/ms_sans_serif_bold.ttf` (700) | status-bar strip, GUI install body, 95-style dialog |
| `font-sans` | `Arial, ui-sans-serif, system-ui, …` (Tailwind default stack) | — | §9 copyright line, §10 slide **heading**, §10 "minutes" values |
| `font-Trebuchet` | `Trebuchet`, Arial, sans-serif | `/fonts/trebuchet.ttf` | defined but not used in the install flow |

* Global default (`app.css`): `html { font-family: 'MSSS', Arial, system-ui, "Segoe UI", Roboto,
  Helvetica, sans-serif, … }`.
* Size scale used by the install screens: Tailwind `text-sm` 14 px, `text-base` 16 px, `text-lg`
  18 px, `text-xl` 20 px, `text-3xl` 30 px, plus the bespoke `text-[42px]` on the Welcome splash and
  `text-[20px]`-equivalent `text-xl` headings on the DOS screens.
* Weights: `font-bold` on the DOS wordmark and step labels; `font-semibold` on the 95 title bar and
  Product-Key header; `italic` only on the "Welcome" splash.

## 6. Colours

| Role | Value | Where |
|---|---|---|
| Text-mode Setup background | **#0207B0** (`rgb(2,7,176)`) | §2–§8 full screen |
| Text-mode header text | `#94a3b8` (slate-400) | wordmark + rules |
| Text-mode body — bright | `#f8fafc` (slate-50) | "Welcome to Setup." |
| Text-mode body — dim | `#94a3b8` (slate-400) / `#cbd5e1` (slate-300) | body text / file-system list |
| Status bar background | `#e2e8f0` (slate-200), text `#0f172a` (slate-900), divider `#0f172a` | every DOS screen |
| Selected list row | background `#cbd5e1` on text `#0207B0` | §5 |
| Format/Copy bar fill | **#FDE047** (yellow-300) | §6, §7 |
| Reboot bar fill | **#EF4444** (red-500) | §8 |
| GUI install background | **#5A7EDC** | §10 |
| GUI top band | **#00309C** | §10 |
| GUI instal-loader separator | `linear-gradient(45deg,#466dcd,#c7ddff,#b0c9f7,#5a7edc)` | §10 |
| GUI bottom separator | `linear-gradient(45deg,#003399,#f99736,#c2814d,#00309c)` | §10, §13 |
| GUI bottom band | `linear-gradient(90deg,#3833ac,#00309c)` | §10 |
| GUI radial highlight | `radial-gradient(circle at 5% 5%, #91b1ef 0, #7698e6 6%, #5a7edc 12%)` | §10 left, §13 |
| GUI right pane | `linear-gradient(#5a7edc, #7698e6, #5a7edc)` | §10 |
| Phase bar fill / activity dots | **#16A34A** (green-600) / glow **#10B981** | §10 |
| Boot-loader 3-bar gradient | `#2838c7 → #5979ef → #869ef3 → #5979ef → #2838c7` | §9, §12 |
| Boot-loader frame | `#b2b2b2` | §9, §12 |
| 95 dialog title bar | `linear-gradient(90deg, navy, #1084d0)`, text `#fff` | §11 |
| 95 dialog surface / bevel | `silver`; insets `#0a0a0a / #dfdfdf / grey / #fff` | §11 |
| Product-Key sticker | `bg-yellow-500` on black | §11 |
| Radio button | `linear-gradient(135deg,#dcdcd7,#fff)`, border `#1d5281`, hover orange; tick sprite `radio_check.png` | §10 |

(These are CSS literals, not CSS variables: the install screens do **not** use the `--xp-gradient` /
`--titlebar-gradient` custom properties that `app.css` defines for the desktop.)

---

## 7. Files, fonts and audio the screens depend on

* Images referenced by the install flow: `/images/xp_loading_logo.jpg`,
  `/images/xp_loading_mslogo.jpg`, `/images/xp_logo_horizontal.png`, `/images/xp_logo.png`,
  `/images/xp/radio_check.png`, `/images/95/0.png`, `/images/radio_check.png`, `/images/ms.png`.
* Fonts: `/fonts/levi.ttf`, `/fonts/ms_sans_serif.ttf`, `/fonts/ms_sans_serif_bold.ttf`,
  `/fonts/trebuchet.ttf`.
* Audio: `/audio/xp_installing.mp3` (§10, looped), `/audio/xp_startup.mp3` (§13).
* Data: `GET /files/EULA.txt` (§4); `/json/hard_drive.json` + IndexedDB (`hard_drive`, `wallpaper`) for
  §9/§12 state.
* Upstream libs loaded at `boot_manager` mount: jQuery 3.6.1 + jQuery UI 1.13.2 (+ `xp.css` stylesheet
  only when `set_theme('xp')`; the install uses `set_theme('none')`).

---

## 8. What the task asked about that win32.run does **NOT** implement

These strings/screens are **absent from `ducbao414/win32.run`**. This is proven by the fetched source
of every install screen *and* by the `index.svelte` route allow-list (§1) — there is no other install
component to load. Do **not** attribute them to win32.run.

| Requested element | Present in win32.run? | Nearest evidence |
|---|---|---|
| "Setup is inspecting your computer's hardware configuration…" | **No** | Not in any fetched file; the loader emits `Setup is loading files …` instead |
| "Press F2 to run Automated System Recovery (ASR)" | **Yes** (verbatim `Press F2 to run Automated System Recovery (ASR)...`) | §2 message 2 (2 s) |
| "Press F6 if you need to install a third party SCSI or RAID driver…" | **Yes** (verbatim `Press F6 if you need to install a third party SCSI or RAID Driver`, capital D, no ellipsis) | §2 message 1 (2 s) |
| "Welcome to Setup" screen with the **F3=Quit / R=Repair** trio | **Partial** — the screen exists but the copy is **`press ENTER` / `press R` / `press ESC`**, with `[sic]` spellings, and **`R` is not wired** | §3 |
| "To repair a Windows XP installation …, press R" | **Partial/`[sic]`** — `To repaire a Windows installation using Recovery Console, press R.`; **R key does nothing** | §3 |
| "To quit Setup without installing Windows XP, press F3" | **No F3** — uses `press ESC` (`To quite Setup without installing Windows XP, press ESC.`) | §3 |
| "The following list shows the existing partitions and unpartitioned space on this computer" | **No** — §5 is a "new partition → choose file system" screen | §5 |
| Format to NTFS (Quick) | **Yes** — first option is `Format the partition using the NTFS file system (Quick)`, default-selected | §5 |
| "Please wait while Setup formats the partition…" | **Yes** (exactly `Please wait while Setup formats the partition`, no trailing ellipsis) | §6 |
| Licence agreement (F8) | **Yes** but **Enter**, not F8: `ENTER=I agree`; there is **no F8** anywhere | §4 |
| "Collecting information" / "Dynamic Update" / "Preparing Installation" / "Installing Windows" / "Finalizing installation" | **Yes** — all five, exactly these, in the §10 step list | §10 |
| Product key / "Your Product Key" | **Yes** — the §11 modal | §11 |
| "Installing Windows Updates…" | **No** | Absent from every install screen; the string belongs to **Reborn XP** (`xp.quenq.com/js/shell.js`), as already noted in `docs/FEATURE-GAPS.md` §4 |
| "The date and time settings" | **No** | Absent |
| "Networking Settings" | **No** | Absent (only the em-dash-less phrase `Installing Network...` appears as a §10 phase) |
| "To improve the appearance of the screen during setup…" | **No** | Absent (the §10 right pane rotates marketing slides — see §10, items 1–4 — none is this string) |
| Final reboot into Windows XP | **Yes** — §12 boot loader → §13 `Welcome` splash → §14 desktop | §12–§14 |
| Interactive vs automatic map | **Yes** — §3 | — |
| Progress-bar behaviour | **Yes** — §4 | — |
| Typography / colours | **Yes** — §5, §6 | — |

### Also inferred / caveats
* **`[inferred]` durations** for §0, §2, §6, §7 are computed from the source's `sleep()`/increment
  arithmetic, not measured against a running site.
* **`faker.system.directoryPath()`** (§2) comes from `@faker-js/faker`; the exact path strings are
  random per run and cannot be enumerated.
* **`random_files()`** (§7) draws from a hard-coded ~1 500-word command list in `src/lib/utils.js`
  (`shells`, `command`, `format`, `diskpart`, `fsutil`, `robocopy`, … ) plus an extension from
  `dll|icm|sys|inf|bin|exe`.
* The `installation/95/` folder name is a **historical artefact** (reused Win-95 components); the
  content is XP.
* The site is a client-side SPA: `https://win32.run/` returns only the static BIOS markup before JS
  runs, so **no install screen is visible without a browser** — all of the above was read from source.

---

## 9. EULA body (leading text of `/files/EULA.txt`) — verbatim

Rendered (with `\n\n` → `<br/>`) on §4. Opening lines:

```
END-USER LICENSE AGREEMENT FOR MICROSOFT SOFTWARE

WINDOWS XP PROFESSIONAL EDITION

IMPORTANT—READ CAREFULLY:

This End-User License Agreement ("EULA") is a legal agreement
between you (either an individual or a single entity) and Microsoft Corporation for the Microsoft software
product identified above, which includes computer software and may include associated media, printed
materials, "online" or electronic documentation, and Internet-based services ("Product"). ...
```

Sections `1. GRANT OF LICENSE` … `19.`, then the French `GARANTIE LIMITÉE` block, footer
`EULAID:WX.4_PRO_RTL_EN` on revision stamp `07/27/2001 54834.1`. (Full text fetched from
`https://win32.run/files/EULA.txt`; quote it as-is, do not re-typeset.)

---

## 10. Rust porting notes (mapping to this repo)

* `src/screens/install.rs` currently defines only `InstallStage::{Confirm, TextSetup, Welcome,
  Installing, Updates}` and merges screens. win32.run gives the **exact order and copy** to split it
  into discrete stages matching §2 steps 2–8 + the GUI phase + Product-Key dialog.
* `src/screens/post.rs` already transcribes the Award POST (§0/§1) and `src/screens/bootmenu.rs` the
  boot menu (§1). The boot-menu options in `bootmenu.rs` match win32.run except that the upstream
  `Onboard NIC (IPV6` typo is "corrected" locally — keep it as-is or note the divergence.
* The `Windows 95` dialog chrome for §11 maps onto this repo's `src/wm.rs` (Luna) being **swapped for
  a beveled `95` window**: it is a deliberate 95-style window inside an XP installer, so it needs its
  own renderer, not the Luna one.
* **Non-destructive rule:** in win32.run nothing is actually written — the "hard drive" facts
  (`10237 MB Disk 0`, `10229 MB … 10228 MB free`) are **literals** in the Svelte, and the licence/FS
  choices are ignored downstream (the picked file system is not carried forward). The Rust animation
  should likewise treat every string on these screens as fixed display text and never touch the real
  filesystem or registry.
* **Timings to hard-code** (visual animation constants, `[inferred]`): §2 ≈ 8.7 s ticker, §6 ≈ 6 s,
  §7 ≈ 13 s, §8 15 s, §9/§12 5–10 s, §10 420 s + pause, §13 7 s.
