# System dialogs & wizards this project is missing — layout-accurate porting spec

Porting reference for the Windows XP **system dialogs / wizards** that `src/` does not implement yet
(see `docs/FEATURE-GAPS.md` §2 "system tools" family and §7 networking). Every claim below carries the
URL it was taken from, and every item carries an explicit **verification status**. Nothing is invented:
where no fetched source contained the strings or geometry, the item is marked `✗ unverified` and a
capture instruction is given instead of guessed values.

Companion documents: `docs/FEATURE-GAPS.md` (what is missing), `docs/PROVENANCE.md` (module → source map).

---

## 0. Method, sources and measurement conventions

### 0.1 Sources actually fetched for this spec

| Source | URL | What it gave | Status of page |
|---|---|---|---|
| ReactOS `devmgr` resources | `https://raw.githubusercontent.com/reactos/reactos/master/dll/win32/devmgr/lang/en-US.rc` | **Device Manager** dialog units, captions, menus, all `IDS_*` strings | 200 |
| ReactOS `devmgr` bootstrap | `https://raw.githubusercontent.com/reactos/reactos/master/base/applications/mscutils/devmgmt/devmgmt.c` and `.../devmgmt/devmgmt.rc` | Devmgr exe = thin bootstrap (`DeviceManager_ExecuteW`), icon, no strings | 200 |
| ReactOS `devmgr` tree listing | `https://github.com/reactos/reactos/tree/master/dll/win32/devmgr` | Presence of `devmgmt/`, `lang/`, `properties/`, `resources/` | 200 |
| ReactOS `cleanmgr` | `https://raw.githubusercontent.com/reactos/reactos/master/base/applications/cleanmgr/cleanmgr/cleanmgr.rc` | **Disk Cleanup** dialog units + strings | 200 |
| ReactOS `mstsc` | `https://raw.githubusercontent.com/reactos/reactos/master/base/applications/mstsc/lang/en-US.rc` | **Remote Desktop Connection** pages, tabs, colour list | 200 |
| ReactOS `syssetup` | `https://raw.githubusercontent.com/reactos/reactos/master/dll/win32/syssetup/lang/en-US.rc` | **Generic XP wizard shell** geometry + per-page titles/subtitles | 200 |
| ReactOS `sysdm` applet | `https://raw.githubusercontent.com/reactos/reactos/master/dll/cpl/sysdm/lang/en-US.rc` | System Properties tabs, "Hardware" tab (Device Manager entry point), Virtual Memory, Startup and Recovery, Environment Variables, Performance Options | 200 |
| ReactOS `netshell` | `https://raw.githubusercontent.com/reactos/reactos/master/dll/shellext/netshell/lang/en-US.rc` | **Network Connections** strings, LAN status pages, ReactOS network-setup wizard pages | 200 |
| ReactOS `base/applications` listing | `https://github.com/reactos/reactos/tree/master/base/applications` | Evidence of *absence*: no `welcome` (tour), `security` (Security Centre), `ntbackup`, `dfrg`, `migwiz`, `netmeeting`, `wmplayer`; `wusa` is present-only-as-stub | 200 |
| ReactOS `dll/cpl` listing | `https://github.com/reactos/reactos/tree/master/dll/cpl` | Evidence of *absence*: no `wuaucpl` (Automatic Updates), no `wscui` (Security Centre) | 200 |
| ReactOS `wusa` | `.../base/applications/wusa/resource.rc`, `.../wusa/res/` | Windows Update stand-alone installer is a **stub**: version block + `IDI_WUSA ICON "res/setup.ico"` only | 200 |
| ReactOS front page | `https://reactos.org` | Reachable; news/landing only — no dialog strings, no usable screenshot text | 200 |
| ReactOS `base/applications/utilman` | `.../utilman/lang/en-US.rc` | Only two strings (`IDS_FAIL_INIT`, `IDS_FAIL_INIT_TITLE`) — Utility Manager, not Remote Assistance | 200 |
| ReactOS `sndrec32` | `.../base/applications/sndrec32/lang/en-US.rc` | Sound Recorder menus/strings (adjacent media tool; used for the WMP comparison) | 200 |
| `wieslawsoltes/WinXp` README | `https://github.com/wieslawsoltes/WinXp` | App inventory (56 apps), `backup`, `diskcleanup`, `diskdefrag`, `hardware`, `systeminfo` ids | 200 |
| `wieslawsoltes/WinXp` COMPATIBILITY | `https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/docs/COMPATIBILITY.md` | Explicit "not pixel-perfect" statement; per-app boundaries | 200 |
| `wieslawsoltes/WinXp` `management.js` | `https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/dist/packages/apps-system/management.js` | **px window sizes + strings** for Task Manager, Backup or Restore Wizard, Disk Cleanup, Disk Defragmenter | 200 |
| `wieslawsoltes/WinXp` `utilities.js` | `.../dist/packages/apps-system/utilities.js` | **px window sizes + strings** for Device Manager, Disk Properties, System Information, Network Connections, Scheduled Task Wizard | 200 |
| `wieslawsoltes/WinXp` `help.js` | `.../dist/packages/apps-system/help.js` | Help and Support Center shell (795×535) + system-tools prose | 200 |
| `wieslawsoltes/WinXp` `media-player.js` | `https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/dist/packages/apps-internet/media-player.js` | WMP-9-style player chrome, palette, px size | 200 |
| Windows Movie Maker product help (XP era, ©2000-2002) | `https://documentation.help/Windows-Movie-Maker/` , `.../idd_task_pane.htm` | **Movie Maker** task-pane strings and help-page titles | 200 |
| How-To Geek, "Using System Restore in XP" | `https://www.howtogeek.com/624/using-system-restore-in-xp/` | System Restore wizard option labels (secondary) | 200 |
| pctechguide, clean-install walkthrough | `https://www.pctechguide.com/how-to-do-a-clean-installation-of-winxp/files-and-settings-transfer-how-to-do-a-clean-installation-of-windows-xp` | **Files and Settings Transfer Wizard** page and option labels (secondary) | 200 |
| Wikipedia, "Windows XP" | `https://en.wikipedia.org/wiki/Windows_XP` | Security Center introduction (SP2), WMP 10/11 upgrade availability, Remote Assistance/Remote Desktop introduction, NetMeeting 3.02 (secondary) | 200 |

### 0.2 Sources that returned **no usable DOM** (stated, not papered over)

| Source | Result | Consequence |
|---|---|---|
| `https://oses.ioblako.com/new.html?autolaunch=windowsxp` | Returns the **V86 OS-launcher page only**: a list of OS cards (including `Windows XP — Dual Format — Boot Installed / Install from CD / Run Preloaded`) plus the keyboard-help panel and the emulator status strip. The XP instance itself boots inside a V86 **canvas**; the DOM exposes no dialog, no window, no control text. | No XP dialog string or metric can be extracted from this URL. Any geometry taken from it would be fabricated. Use it only as a **manual capture target** (§12). |
| `https://wieslawsoltes.github.io/WinXp/` | Blazor/SPA shell; the whole fetched text is `Windows XP Professional`. | Used its **repository sources** instead (same project, `dist/packages/**`). |
| `https://reactos.org/` | Reachable landing page: mission text + news items; no screenshots with extractable text, no dialog strings. | ReactOS strings were taken from its **resource files** on GitHub, which are the authoritative text. |
| `https://documentation.help/Windows-Media-Player/` | 404 — the archive has no WMP manual. | WMP 10/11 remains unverified (§11). |

### 0.3 Dialog units → pixels (used for every ReactOS number below)

ReactOS resource files give **dialog units (DLU)** for font `8, "MS Shell Dlg"` (Tahoma 8 pt). At 96 DPI
that font's base units are **x = 6, y = 13**, therefore:

```
px_x = DLU_x × 6 / 4 = DLU_x × 1.5
px_y = DLU_y × 13 / 8 = DLU_y × 1.625
```

| DLU (w×h) | px (w×h) | Used by |
|---|---|---|
| 317×193 | **476×314** | wizard frame with graphic band (Welcome/Finish) |
| 317×143 | **476×232** | wizard interior page |
| 256×218 | **384×354** | System Properties property page |
| 252×218 | **378×354** | Device Manager property page (General/Driver/Details/Resources/Power) |
| 300×400 | **450×650** | Device Manager hardware pane |
| 246×216 | **369×351** | Network connection properties sheet |
| 235×215 | **353×349** | Disk Cleanup main page |
| 224×230 | **336×374** | Driver File Details |
| 224×121 | **336×197** | Confirm Device Removal |
| 177×74 | **266×120** | Disk Cleanup – Select Drive |
| 242×175 | **363×284** | Remote Desktop Connection tab page |
| 260×262 | **390×426** | Remote Desktop Connection window |
| 227×204 | **341×332** | Visual Effects |
| 275×78 | **413×127** | "Please wait…" status window |

These are **client** sizes; add the Luna window frame when sizing a `src/wm.rs` window.

---

## 1. Generic XP wizard shell (title banner, left illustration panel, Back/Next/Cancel footer)

> **Status: ✅ VERIFIED — geometry and strings from a ReactOS resource file.**
> Source: `https://raw.githubusercontent.com/reactos/reactos/master/dll/win32/syssetup/lang/en-US.rc`
> (the ReactOS re-implementation of the XP Setup/Wizard shell; `dll/win32/syssetup/wizard.c` derives the
> `IDD_*PAGE` dialogs below from `IDD_WELCOMEPAGE` at runtime, which is why every page repeats the same
> 317-DLU width.)

**Purpose:** the shared frame every XP wizard uses — a **left illustration/colour band**, a **header**
(page title + one-line subtitle), a **body** of controls, and a **footer** with Back / Next / Cancel
(plus Finish/Close on the last page). Port this once; the sixteen items below are then mostly page bodies.

**Exact window title:** the shell itself is untitled; each page supplies `CAPTION`. In ReactOS the pages
are captioned `"ReactOS Setup"`; the XP equivalent is `"Windows XP Setup"` (do **not** copy the ReactOS
string literally — `src/` should use the XP product name it already uses elsewhere).

### 1.1 Frame geometry — `IDD_WELCOMEPAGE`, 317×193 DLU = **476×314 px**

| Element | Id | DLU (x, y, w, h) | px (x, y, w, h) |
|---|---|---|---|
| Left illustration panel (implicit: content column starts at x = 115) | — | 0, 0, 115, 193 | **0, 0, 172, 314** |
| Page title | `IDC_WELCOMETITLE` | 115, 8, 195, 24 | 172, 13, 293, 39 |
| Subtitle / body paragraph | `IDC_STATIC` | 115, 40, 195, 50 | 172, 65, 293, 81 |
| "Click Next to continue with Setup." (bottom of content column) | `IDC_STATIC` | 115, 169, 195, 17 | 172, 275, 293, 28 |

### 1.2 Interior page geometry — `IDD_OWNERPAGE`, 317×143 DLU = **476×232 px**

| Element | DLU | px |
|---|---|---|
| Content left edge (all pages agree) | x = 54 | **x = 81** |
| Field/label column | x = 132 … 147 | x = 198 … 221 |
| Standard text field | 147, 35, 148, 14 | 221, 57, 222, 23 |
| Page icon (20×20 DLU) | 21, 7 | 32, 11 (30×33 px) |
| Progress bar (`IDD_PROCESSPAGE`) | 21, 97, 275, 11 | 32, 158, 413, 18 |
| Per-task check icons (10×10 DLU) | 48, 26/38/50/62/74 | 72, 42/62/81/101/121 (15×16 px) |

**Derived rules for the shell (state these in code as constants):**

- Illustration band width = **115 DLU = 172 px**; body column starts at x = 115 DLU (172 px) for
  welcome/finish pages, x = 54 DLU (81 px) for field pages.
- Interior pages are exactly **50 DLU (81 px) shorter** than the welcome/finish frame (143 vs 193 DLU);
  the header banner and the footer occupy that band. *Inference* — the button metrics live in
  `dll/win32/syssetup/wizard.c`, which was **not** fetched.

### 1.3 Page strings (exact ids → exact text; `&` is the accelerator marker)

| Id | String |
|---|---|
| `IDD_WELCOMEPAGE` title | `Welcome to the ReactOS Setup Wizard` |
| body | `This wizard installs ReactOS on your computer.\nThe wizard needs to gather some information about you and your computer to set up ReactOS properly.` |
| footer hint | `Click Next to continue with Setup.` |
| `IDD_FINISHPAGE` title | `Completing the ReactOS Setup Wizard` |
| finish body | `You have successfully installed ReactOS.\n\nIf there is a CD in a drive, remove it. To restart your computer, click Finish.` |
| finish hint | `Your computer will automatically restart in 15 seconds.` |
| `IDS_ACKTITLE` / `IDS_ACKSUBTITLE` | `Acknowledgements` / `The shoulders ReactOS stands on and license information` |
| `IDS_OWNERTITLE` / `IDS_OWNERSUBTITLE` | `Personalize your Software` / `Setup uses this information about yourself to personalize ReactOS.` |
| `IDS_COMPUTERTITLE` / `IDS_COMPUTERSUBTITLE` | `Computer Name and Administrator Password` / `You must provide a name and an Administrator Password for your computer.` |
| `IDS_LOCALETITLE` / `IDS_LOCALESUBTITLE` | `Regional settings` / `You can customize ReactOS for different regions and languages.` |
| `IDS_DATETIMETITLE` / `IDS_DATETIMESUBTITLE` | `Date and Time` / `Set the correct date and time for your computer.` |
| `IDS_PROCESSTITLE` / `IDS_PROCESSSUBTITLE` | `Finishing the Installation` / `Please wait while Setup completes the final steps.` |
| `IDS_THEMESELECTIONTITLE` / `IDS_THEMESELECTIONSUBTITLE` | `Appearance` / `Select the theme you prefer.` |
| Validation strings | `Setup cannot continue until you enter your name.`, `Setup failed to set the computer name.`, `Setup cannot continue until you enter the name of your computer.`, `You must enter a password!`, `The passwords you entered do not match. Please enter the desired password again.`, `Setup was unable to set the local time.` |
| Progress lines (`IDD_PROCESSPAGE`) | `Registering components`, `Installing start menu items`, `Saving settings`, `Installing optional components`, `Removing temporary files` |
| Status window | `Please wait...` (`IDD_STATUSWINDOW_DLG`, 275×78 DLU = 413×127 px, owner-draw logo strip 275×54 + 275×4 bar) |

**Illustration panel:** yes — a full-height bitmap band, 172 px wide. ReactOS ships its own artwork; XP's
per-wizard billboards (Setup, FAST, Network Setup, System Restore) are Microsoft assets and must be
drop-in from `assets-local/`, per `docs/PROVENANCE.md`.

**Footer:** Back / Next / Cancel. Order and accelertors follow the MS wizard: `&Back`, `&Next >`,
`Cancel`. ReactOS's own wizard controller (not fetched) creates them — treat the footer band as
y = 177…193 DLU (**288…314 px**, ≈ 26 px tall) and place the three buttons right-aligned inside it.

---

## 2. Windows Update / Automatic Updates

> **Status: ✗ UNVERIFIED — no fetched source contains these dialogs.** Do not implement from guesswork.

**Purpose:** download and install OS updates (Windows Update web/ActiveX control, `wuauclt.exe`) and the
Automatic Updates control-panel applet that schedules them.

| Field | Value |
|---|---|
| Exact window title | **not verified** |
| Tabs / buttons / labels | **not verified** |
| Approx. size (px) | **not verified** |
| Left-panel illustration | **not verified** |
| Source URL | none (see below) |

Evidence of *absence* rather than fabrication:

- ReactOS ships `base/applications/wusa` (Windows Update Standalone Installer) as a **stub**:
  `resource.rc` contains only a version block and `IDI_WUSA ICON "res/setup.ico"` —
  `https://raw.githubusercontent.com/reactos/reactos/master/base/applications/wusa/resource.rc`.
- No `wuaucpl` applet exists in `dll/cpl` (`https://github.com/reactos/reactos/tree/master/dll/cpl`) and
  no `wuauclt`/Windows Update UI exists in `base/applications`
  (`https://github.com/reactos/reactos/tree/master/base/applications`).
- The daily-fetched secondary subset (Wikipedia) only confirms the *feature names*: the OS "Update method"
  row lists `Windows Update`, `Windows Server Update Services (WSUS)`, `System Center Configuration
  Manager (SCCM)`, and SP3 "began being automatically pushed out to **Automatic Updates** users on
  July 10, 2008" — `https://en.wikipedia.org/wiki/Windows_XP`.
- `https://oses.ioblako.com/new.html?autolaunch=windowsxp` renders only the V86 launcher; the XP guest
  is a canvas, so nothing can be scraped from it (§0.2).

**To close this gap:** capture the two windows manually in a real XP SP3 VM and record
(title, page order, every label, DLU-equivalent pixel boxes). Until then this row must stay unimplemented
rather than approximated.

---

## 3. System Restore

> **Status: ◐ PARTIAL — wizard option labels verified from a fetched secondary page; geometry, title
> string and page count NOT verified.**

**Purpose:** roll the system (registry, drivers, system files) back to a restore point; also create
restore points manually. Reachable at `Start → All Programs → Accessories → System Tools → System Restore`.

| Field | Value | Source |
|---|---|---|
| Exact window title | not verified (referenced only as "System Restore") | `https://www.howtogeek.com/624/using-system-restore-in-xp/` |
| Page 1 options | `Create a restore point`, `Restore my computer to an earlier time` | same |
| Navigation | `Next` button; final `confirmation screen`; "Your system will reboot and then restore your computer back to the previous configuration." | same |
| Create-restore-point page | free-text **description** field | same |
| Restore page | a **calendar** with bolded dates where restore points exist + a **restore point list** on the right-hand side, then a confirmation screen | same |
| Approx. size (px) | not verified | — |
| Left-panel illustration | not verified — the wizard belongs to the §1 shell family, so a billboard is expected but no source confirms it | — |

ReactOS implements no System Restore client: there is no `sr*` application under
`https://github.com/reactos/reactos/tree/master/base/applications`, and the fetched System Properties
resource (`.../dll/cpl/sysdm/lang/en-US.rc`) provides only **General, Hardware, Advanced** pages plus the
sub-dialogs (Performance Options, User Profiles, Startup and Recovery, Environment Variables, Virtual
Memory, Hardware Profiles) — **no System Restore tab**. Its `IDD_HARDWAREPROFILES` string
`IDS_HINT_SYS_RESTORE " Restores this window to normal size."` in `devmgr` is a *window* hint, not the feature.

**Note for porters:** the two option labels above are the strongest evidence gathered; treat every other
System Restore string as still to be captured.

---

## 4. Device Manager

> **Status: ✅ VERIFIED — ReactOS `devmgr` resource file (dialog units, captions, menus, all strings) +
> a live-demo implementation for px sizing.**

Sources:
`https://raw.githubusercontent.com/reactos/reactos/master/dll/win32/devmgr/lang/en-US.rc`,
`https://github.com/reactos/reactos/tree/master/dll/win32/devmgr` (the exe `devmgmt` in
`base/applications/mscutils/devmgmt` is only a 30-line bootstrap calling `DeviceManager_ExecuteW` —
`.../mscutils/devmgmt/devmgmt.c`),
`https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/dist/packages/apps-system/utilities.js`.

**Purpose:** tree of every installed device, grouped by type or connection, with enable/disable/uninstall,
driver update and rollback, per-device property pages and resource listings. Launched standalone
(`devmgmt.msc`, tree-only view) or from **System Properties → Hardware → Device Manager**.

**Exact window title:** ReactOS uses `IDS_APPNAME "ReactOS Device Manager"`; the XP shell title is
`Device Manager` (MMC console form: `Device Manager` with the standalone snap-in scope). The fetched
demo uses `title: 'Device Manager'` (`utilities.js`, `hardwareApp`).

**Approximate size (px):**

| Surface | DLU | px |
|---|---|---|
| Hardware/property pane (`IDD_HARDWARE`, tree + "Device Properties" group) | 300×400 | **450×650** |
| Per-device property page (General/Driver/Details/Resources/Power) | 252×218 | **378×354** |
| Driver File Details | 224×230 | **336×374** |
| Confirm Device Removal | 224×121 | **336×197** |
| Live-demo window (`hardwareApp`) | — | **530×404** (px, from `width`/`height`) |

**Left-panel illustration:** none (classic MMC console, two panes: tree left, device/properties right).

**Menu bar (exact, `IDM_MAINMENU`)**

| Menu | Items (exact strings, accelerators marked with `&`) |
|---|---|
| `&File` | `E&xit` |
| `&Action` | `Scan for hardware changes`, `Add hardware` |
| `&View` | `D&evices by type`, `De&vices by connection`, `Resources by t&ype`, `Resources by co&nnection` *(grayed)*, ― , `Sho&w hidden devices` |
| `&Help` | `&About` |

**Context-menu / toolbar strings (`IDS_MENU_*`)**

| Id | String |
|---|---|
| `IDS_MENU_UPDATE` | `Update driver software...` |
| `IDS_MENU_ENABLE` / `IDS_MENU_DISABLE` / `IDS_MENU_UNINSTALL` | `Enable` / `Disable` / `Uninstall` |
| `IDS_MENU_SCAN` / `IDS_MENU_ADD` / `IDS_MENU_PROPERTIES` | `Scan for hardware changes` / `Add hardware` / `Properties` |
| Tooltips | `Properties`, `Scan for hardware changes`, `Enable`, `Disable`, `Update Driver Software`, `Uninstall` |
| Hints (status bar) | ` Open property dialog for the current selection.`, ` Scan for changed or new plug and play devices.`, ` Enables the selected device.`, ` Disables the selected device.`, ` Launches the Update Driver Software wizard for the selected device.`, ` Uninstalls the driver for the selected device.`, ` Adds a legacy (non-Plug and Play) device to the computer.`, ` Displays devices by hardware type.`, ` Displays devices by connection.`, ` Displays resources by type.`, ` Displays resources by connection type.`, ` Displays legacy devices and devices that are no longer installed.`, ` Closes this window.` |

**Page captions and labels**

| Page | Caption | Key labels (exact) |
|---|---|---|
| `IDD_HARDWARE` | (pane of the console) | `&Devices:`, group `Device Properties`, buttons `&Troubleshoot...`, `P&roperties` |
| `IDD_DEVICEGENERAL` | `General` | `Device type:`, `Manufacturer:`, `Location:`, group `Device status`, `&Troubleshoot...`, `&Device usage:` |
| `IDD_DEVICEDRIVER` | `Driver` | `Driver provider:`, `Driver date:`, `Driver version:`, `Digital Signer:`, `&Driver Details...`, `To view details about the driver files.`, `&Update Driver...`, `Update the device driver.`, `&Roll Back Driver`, `If the device fails after updating the driver, roll back to the previously installed driver.`, `&Uninstall`, `To uninstall the driver (Advanced).` |
| `IDD_DRIVERDETAILS` | `Driver File Details` | `&Driver files:`, `Provider:`, `File version:`, `Copyright:`, `Digital Signer:`, `OK` |
| `IDD_DEVICEDETAILS` | `Details` | property-name combo + property-value list (no static labels) |
| `IDD_DEVICERESOURCES` | `Resources` | `Resource settings:` |
| `IDD_DEVICEPOWER` | `Power` | (icon + device name only) |
| `IDD_UNINSTALLDRIVER` | `Confirm Device Removal` | `Warning: You are about to uninstall this device from your system.`, `OK`, `Cancel` |

**Status / property strings (`IDS_*`, selection)**

`Name`, `Type`, `Manufacturer: %1`, `Location: %1`, `Device Status: %1`, `Unknown`, `Location %1!u! (%2)`,
` (Code %1!u!)`, `Use this device (enable)`, `Do not use this device (disable)`, `Unknown device`,
`No drivers are installed for this device.`, `on %1`, `Not available`, `Not digitally signed`,
`No driver files are required or have been loaded for this device.`, `&Troubleshoot...`, `E&nable Device`,
`Re&install Driver`, `P&roperties`, `U&pdate Driver...`, `Restart &Computer...`, `&Copy`,
`Resource type`, `Setting`, `Memory range`, `IRQ`, `DMA`, `I/O range`;
resource types `Memory`, `Input/output (IO)`, `Direct Memory Access (DMA)`, `Interrupt request (IRQ)`.

**Device-status sentences (selection: the ones a port needs first)**

`This device is working properly.` · `This device is not configured correctly.` ·
`This device is either not present, not working properly, or does not have all the drivers installed.` ·
`This device is disabled.` · `This device is not started.` · `The drivers for this device need to be reinstalled.` ·
`This device cannot work properly until you restart your computer.` · `This device is causing a resource conflict.` ·
`ReactOS has stopped this device because it has reported problems.` · `Your registry may be bad.`
(The full 40-entry code table is in the source file; ReactOS renames `Windows` → `ReactOS` throughout —
port the XP wording, keep the ReactOS ids.)

**Entry point from System Properties** (verified in `.../dll/cpl/sysdm/lang/en-US.rc`, `IDD_PROPPAGEHARDWARE`,
256×218 DLU = 384×354 px): group `Device Manager`, text `The Device Manager lists all the hardware devices
installed on your computer. Use the Device Manager to change the properties of any device.`, button
`&Device Manager...` (90×15 DLU = 135×24 px). Sibling groups: `Hardware Wizard` (+ `&Hardware Wizard...`),
`Hardware Profiles` (+ `Hard&ware Profiles...`).

---

## 5. Disk Defragmenter

> **Status: ◐ PARTIAL — px size, window title, action strings and status lines come from a fetched live
> demo; ReactOS does not implement the tool, so no DLU geometry or XP-verbatim string table exists.**

Sources: `https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/dist/packages/apps-system/management.js`
(`diskDefragApp`); absence verified against `https://github.com/reactos/reactos/tree/master/base/applications`.

| Field | Value |
|---|---|
| Purpose | analyse a volume's layout and defragment it (MMC snap-in `dfrg.msc`); per-volume Analyze/Defragment, an allocation map and a legend |
| Exact window title | `Disk Defragmenter` |
| Approx. size (px) | **650×440** (`width: 650, height: 440`), resizable |
| Left-panel illustration | none (MMC snap-in: volume table on top, allocation map + legend + report below) |
| Menus | `Action` → `Analyze`, `Optimize`, `Exit`; `Help` → `About Disk Defragmenter` |
| Volume table columns | `Volume`, `File System`, `Capacity`, `Storage`; row `Local Disk (C:)`, `Virtual FS` |
| Map legend | `Files`, `Recycle Bin`, `Available` (swatches `#244bcb`, `#d62321`, `#fff` with `1px solid #aaa`) |
| Buttons | `Analyze`, `Optimize`, `Close` |
| Status bar | `Ready` → `Analysis complete` → `Optimizing…` → `Optimization complete` |
| About text | `This tool analyzes and compacts the virtual filesystem. Browser storage does not expose physical disk fragmentation.` |

**Caveat for porters:** the demo's button is `Optimize`, its map legend is 3 swatches, and the report text
is that project's own wording. The XP original uses a different report block and legend
("Estimated disk usage before/after defragmentation"). **Do not treat any of the above as XP-verbatim
except the window title and the volume-table column names.** No fetched source contained the XP strings.

---

## 6. Disk Cleanup

> **Status: ✅ VERIFIED — ReactOS `cleanmgr.rc` (dialog units + exact strings), cross-checked with the
> live demo's px size.**

Sources: `https://raw.githubusercontent.com/reactos/reactos/master/base/applications/cleanmgr/cleanmgr/cleanmgr.rc`
(project path: `base/applications/cleanmgr/cleanmgr/`), plus
`https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/dist/packages/apps-system/management.js`.

**Purpose:** free disk space by deleting selected categories of junk files; reports how much each category
holds and how much is selected, with a description box and a `Details...` drill-down.

**Exact window titles**

| Window | Exact caption |
|---|---|
| Progress/calculating dialog | `Disk Cleanup` |
| Main page (`IDD_PROPERTIES_MAIN`) | `Disk Cleanup` |
| Drive chooser (`IDD_SELECTDRIVE`) | `Disk Cleanup - Select Drive` |
| Live-demo title | `Disk Cleanup for (C:)` (i.e. XP's `IDS_PROPERTIES_MAIN_TITLE` format string) |

**Sizes (px):** main page 235×215 DLU = **353×349 px**; drive chooser 177×74 DLU = **266×120 px**;
live-demo window **430×390 px** (non-resizable).

**Layout (`IDD_PROPERTIES_MAIN`, 235×215 DLU)**

| Element | Id | DLU (x, y, w, h) | px |
|---|---|---|---|
| Drive icon (20×20) | `IDC_DISKICON` | 6, 6, 20, 20 | 9, 10, 30, 33 |
| "You can use Disk Cleanup to free up to 0.0MB of disk space on drive C." | `IDC_TOTAL_CLEANABLE` | 36, 6, 192, 18 | 54, 10, 288, 29 |
| `Files to delete:` | `IDC_STATIC` | 6, 30, 222, 8 | 9, 49, 333, 13 |
| Category list (report view, no column header, bordered) | `IDC_HANDLERLIST` | 6, 42, 222, 66 | 9, 68, 333, 107 |
| `Total amount of disk space gained:` / right-aligned amount | `IDC_STATIC` / `IDC_SELECTED_GAINED` | 6, 114, 144, 8 / 156, 114, 73, 8 | 9, 185, 216, 13 / 234, 185, 110, 13 |
| Group box `Description` | `IDC_STATIC` | 6, 126, 222, 84 | 9, 205, 333, 137 |
| Description text | `IDC_DESCRIPTION` | 12, 138, 210, 54 | 18, 224, 315, 88 |
| Button `Details...` | `IDC_DETAILS` | 150, 192, 74, 14 | 225, 312, 111, 23 |

**Drive chooser (`IDD_SELECTDRIVE`, 177×74 DLU):** text `Select the drive to clean up.` (12, 6, 150, 8);
combo 12, 24, 150, 90 (dropdown list, no integral height); `OK` (default, 30, 48, 50, 14); `E&xit`
(96, 48, 50, 14 — note: not "Cancel").

**Exact strings (`STRINGTABLE`)**

| Id | String |
|---|---|
| `IDS_PROPERTIES_MAIN_TITLE` | `Disk Cleanup for (%c:)` |
| `IDS_TOTAL_CLEANABLE_CAPTION` | `You can use Disk Cleanup to free up to %s of disk space on (%c:).` |
| `IDS_DISK_CLEANUP` | `Disk Cleanup` |
| `IDS_CONFIRM_DELETE` | `Are you sure you want to delete these files permanently?` |
| `IDS_CALCULATING` | `Disk Cleanup is calculating how much space can be gained on (%s).` |
| `IDS_SCANNING` | `Scanning: %s` |
| `IDS_CLEANING_CAPTION` | `Disk Cleanup is cleaning up files on %s.` |
| `IDS_CLEANING` | `Cleaning: %s` |

**Left-panel illustration:** none (single-pane dialog with a 30-px drive icon top-left).
**Live-demo variants for reference:** checkbox rows `Recycle Bin (…)`, `Temporary files (…)`,
`Internet Explorer history`, result line `Total amount of disk space you gain: …`,
note `Selected files are permanently deleted from the virtual disk.`, confirm dialog title `Disk Cleanup`.

---

## 7. Backup

> **Status: ◐ PARTIAL — title, px size, page strings and buttons from the fetched live demo; ReactOS ships
> no backup tool, so no DLU geometry or XP-verbatim string table exists.**

Sources: `.../apps-system/management.js` (`backupApp`); absence verified against
`https://github.com/reactos/reactos/tree/master/base/applications` (no `ntbackup`).

| Field | Value |
|---|---|
| Purpose | back up files/settings to a file or device, or restore them from an earlier backup; also Automated System Recovery |
| Exact window title | `Backup or Restore Wizard` |
| Approx. size (px) | **520×390**, `resizable:false` |
| Left-panel illustration | the demo has an intro block with a 64-px backup icon — **the XP wizard uses the standard §1 wizard shell with a billboard**; no fetched source confirms the XP bitmap |
| Strings | `Welcome to the Backup or Restore Wizard`, `Back up your virtual files and desktop settings, or restore them from a previous backup.`, `A backup includes every user folder and the contents of the Recycle Bin.`, `Save the backup on your computer so you can restore it in another browser profile.` |
| Buttons | `Back Up Files…`, `Restore Files…`, `Cancel` |
| Confirm dialog | title `Restore Backup`, text `Restore this backup? It will replace the current virtual disk and settings. Export a backup first if you want to keep the current state.` |
| Notifications | `Backup completed` / `Your virtual disk and settings have been exported.`; `Restore completed` / `Your files and settings have been restored.` |

**Caveat:** the XP original (`ntbackup.exe`, wizard + Advanced mode with `Backup`/`Restore and Manage Media`
tabs) is much larger than this; only the wizard title is safe to carry over verbatim.

---

## 8. Files and Settings Transfer Wizard

> **Status: ◐ PARTIAL — page and option labels verified from a fetched walkthrough page; window title,
> size and illustration NOT verified. ReactOS has no `migwiz`.**

Sources: `https://www.pctechguide.com/how-to-do-a-clean-installation-of-winxp/files-and-settings-transfer-how-to-do-a-clean-installation-of-windows-xp`,
absence via `https://github.com/reactos/reactos/tree/master/base/applications`.

| Field | Value |
|---|---|
| Purpose | collect a user's files + personal settings from an old PC (or this PC) and apply them on a new XP install |
| Launch path (verified) | `Start → All Programs → Accessories → System Tools → Files and Settings Transfer Wizard` |
| Exact window title | not verified |
| Verified page / labels | page title `Which computer is this?`; option `New computer`; option `I don't need the Wizard Disk. I have already collected my files and settings from my old computer`; `Next`; a progress phase that "organises your preserved files and settings for the transfer… then transfers the files and applies the settings"; a finished page inviting a restart |
| Approx. size (px) | not verified |
| Left-panel illustration | not verified (expected: §1 wizard shell billboard) |

**To close this gap:** capture `FASTWIZ`/`migwiz` manually. The `Old computer` branch, the Wizard-Disk
branch and the file-selection checkbox tree are all unverified here.

---

## 9. Network Setup Wizard

> **Status: ◐ PARTIAL — ReactOS *does* implement wizard pages with equivalent content (inside ReactOS
> Setup) and the Network Connections string table; the XP `netsetup.exe` "Network Setup Wizard" window
> itself is NOT verified.**

Source: `https://raw.githubusercontent.com/reactos/reactos/master/dll/shellext/netshell/lang/en-US.rc`
(note: **`dll/shellext/netshell/`**, not `dll/win32/netshell/` — the latter is a 404).

**Purpose:** configure a home/small-office network: internet connection type, connection sharing, computer
name and workgroup; also reachable from Network Connections' task pane.

**Verified wizard-page content (ReactOS Setup network pages, 317×143 DLU = 476×232 px each, i.e. exactly the
§1 interior-page size)**

| Page (id) | Caption | Title / subtitle strings | Controls |
|---|---|---|---|
| `IDD_NETWORKINITPAGE` | `ReactOS Setup` | `Network Install` / `Installs required networking components.` | `Please wait while ReactOS installs networking components.`, smooth progress bar 21, 58, 275, 11 DLU |
| `IDD_NETWORKSETTINGSPAGE` | `ReactOS Setup` | `Network Settings` / `Installs network software used to connect to other networks, computers, and the Internet.` | `Choose whether to use typical or custom settings:`; radios `Typical Settings`, `Custom Settings`; descriptions `Creates network connections using the Client for ReactOS Networks, File and Print Sharing for ReactOS Networks, and the TCP/IP transport protocol with automatic addressing.` and `Allows you to manually configure network components.` |
| `IDD_NETWORKCOMPONENTPAGE` | `ReactOS Setup` | `Network Components` / `Determine which networking components are used on your computer. To add new components, click on Install.` | `For Device:`, `Components checked are used by this connection:`, component list (report view, no header, bordered), `&Install...`, `&Uninstall`, `&Properties`, group `Description` |
| `IDD_NETWORKDOMAINPAGE` | `ReactOS Setup` | `Workgroup or Network Domain` / `Specify how the computer will be used - as part of a workgroup or as part of a network domain.` | `Do you want this computer to be a member of a domain?\n(You can obtain this information from your network administrator.)`; radios `No, this computer will be part of a workgroup instead.`, `Yes, this computer will be part of a domain.`; `Workgroup or Domain Name:`; long `NOTE: If this computer isn't going to be part of a network…` paragraph; validation `Setup cannot continue until you\nenter the name of your domain\nor workgroup.` |

**Verified Network Connections strings (`STRINGTABLE`)** — relevant because the XP Network Setup Wizard is
entered from this folder's task pane:

`Network Connections` · column headers `Name`, `Type`, `Status`, `Device Name`, `Phone # or Host Address`,
`Owner` · type `LAN or High-Speed Internet` · status `Disabled`, `Not Connected`, `Network cable unplugged`,
`Acquiring network address`, `Connected` · context actions `Enable`, `Disable`, `Status`, `Repair`,
`Create Shortcut`, `Delete`, `Rename`, `Properties`, `Open Network Connections` · details keys
`Physical Address`, `IP Address`, `Subnet Mask`, `Default Gateway`, `DHCP Server`, `Lease Obtained`,
`Lease Expires`, `DNS Servers`, `WINS Servers`, `Property`, `Value`, `Assigned by DHCP`,
`Manually configured` · speed formats `%u Bit/s`, `%u KBit/s`, `%u MBit/s`, `%u GBit/s`.

**Verified dialogs from the same file:** `IDD_NETPROPERTIES` (General; 246×216 DLU = 369×351 px:
`Connect Using:`, `&Configure`, `Components checked are used by this connection:`, `&Install`, `&Uninstall`,
`&Properties`, group `Description`, `&Show Icon in taskbar when connected`,
`&Notify me when this connection has limited or no connectivity`);
`IDD_LAN_NETSTATUS` (General; 200×180 DLU = 300×293 px: groups `Connection` and `Activity`,
`Status:`/`Duration:`/`Speed:`, `Sent`/`Received`, `Bytes:`, `&Properties`, `&Disable`);
`IDD_LAN_NETSTATUSADVANCED` (Support: `Address type:`, `IP Address:`, `Subnet Mask:`, `Default Gateway:`, `&Details...`);
`IDD_LAN_NETSTATUSDETAILS` (`Network Connection Details`, 190×175 DLU = 285×284 px, `Network Connection &Details:`, `&Close`).

**Live-demo reference:** `networkApp` = `Network Connections`, **590×380 px**, menus `File → Close`,
`View → Refresh`.

---

## 10. Remote Assistance

> **Status: ✗ UNVERIFIED — no fetched source contains this dialog set.**

Sources checked without a hit: ReactOS `base/applications` listing (no Remote Assistance component),
`.../utilman/lang/en-US.rc` (Utility Manager only: `Failed to initialize the Utility Manager library! The error code is %lu.` /
`Initialization failure`), `.../dll/cpl` listing (no RA applet).
Wikipedia confirms the feature exists — "Remote Assistance and Remote Desktop were also added, which allow
users to connect to a computer running Windows XP from across a network or the Internet and access their
applications, files, printers, and devices or request help"
(`https://en.wikipedia.org/wiki/Windows_XP`) — but gives no dialog text or geometry.

| Field | Value |
|---|---|
| Purpose | invite a helper to view/take control of the session; offer help when invited; exchange the invitation file / e-mail invitation (`rcimlby.exe`, msrc.dll, "RA" DCOM component) |
| Exact window title, tabs, buttons, labels, px size, left-panel illustration | **all unverified** |

**To close this gap:** capture the three surfaces separately — (a) *Invitation* wizard
(who to invite, expiry, password, connection method), (b) the offered-invitation/approval dialog,
(c) the Chat/File-Transfer toolbar that appears during a session.

---

## 11. Windows Tour

> **Status: ✗ UNVERIFIED — no fetched source contains the tour content.**

Sources checked without a hit: ReactOS `base/applications` listing (no `welcome`/tour app);
`https://oses.ioblako.com/new.html?autolaunch=windowsxp` (V86 canvas only, §0.2);
`https://wieslawsoltes.github.io/WinXp/` + its README/`COMPATIBILITY.md` (no tour app in the 56-app
inventory — its closest items are `aboutxp` and `help`).

What **is** already known inside this repo: `docs/FEATURE-GAPS.md` §1 records that Reborn XP exposes an
`xptour` app id, used by `shell.js` for the tray "Take a tour" balloon target. That is the *launch hook*,
not the tour's content.

| Field | Value |
|---|---|
| Purpose | first-logon guided tour: what's new in XP, music/video/photos, sharing the computer, going mobile, taking a tour of Windows XP |
| Exact window title, page list, strings, px size, left-panel illustration | **all unverified** |

---

## 12. Security Center

> **Status: ✗ UNVERIFIED for strings/geometry — only the existence and purpose are sourced.**

Source: `https://en.wikipedia.org/wiki/Windows_XP` (SP2 section): "Service Pack 2 also added Security
Center, an interface that provides a general overview of the system's security status, including the state
of the firewall and automatic updates. Third-party firewall and antivirus software can also be monitored
from Security Center." The article's image caption also names the window: "**Windows Security Center**
window running Windows XP SP2, showing no virus protection installed".

| Field | Value |
|---|---|
| Purpose | one pane showing the state of `Firewall`, `Automatic Updates` and `Virus Protection`, with alerts toggles and links to the Windows Firewall applet / Automatic Updates applet |
| Exact window title | `Windows Security Center` (name sourced from an image caption; treat as **likely, not verified**) |
| Tabs / buttons / labels | **unverified** |
| Approx. size (px) | **unverified** |
| Left-panel illustration | none expected (single-pane applet with a status banner) — **unverified** |
| ReactOS evidence | none: no `wscui` applet in `dll/cpl`, no Security Center app in `base/applications` |

---

## 13. Movie Maker

> **Status: ✅ VERIFIED for pane/task strings and help-page titles (primary XP-era product help);
> window geometry and px size are NOT verified.**

Sources: `https://documentation.help/Windows-Movie-Maker/` (full table of contents) and
`https://documentation.help/Windows-Movie-Maker/idd_task_pane.htm` (©2000-2002 Microsoft; the XP-era
version of the product help).

**Purpose:** capture from a video device or import media, arrange clips on storyboard/timeline, apply
transitions/effects/titles/audio, then save to computer, CD, e-mail, Web or DV tape.

**Verified interface inventory (panes and dialogs, exact strings)**

| Surface | Exact strings |
|---|---|
| Movie Tasks pane | `Capture Video`, `Edit Movie`, `Finish Movie`, `Movie Making Tips` |
| `Capture Video` group (verbatim description) | `Provides options to begin making a movie, such as capturing video and importing existing video, pictures, or audio.` |
| `Edit Movie` group | `Provides options for making a movie, such as viewing existing video, pictures, audio, video effects, or adding movie titles and credits. You can also start AutoMovie to begin making a movie automatically.` |
| `Finish Movie` group | `Provides options for saving your final movie, such as to your computer, to a recordable CD, to send as an attachment in an e-mail message, to the Web, or to a tape in a DV camera.` |
| `Movie Making Tips` group | `Provides help for completing common tasks in Windows Movie Maker.` |
| Other panes | `Collections pane`, `Contents pane`, `Movie Tasks pane`, `monitor`, `storyboard`, `timeline` |
| Dialogs named in help | `Configure Video Capture Device` dialog box; `Save Movie Wizard` (with per-destination completions: "saving to My computer", "saving a movie to a recordable CD", "sending a movie in an e-mail message", "sending a movie to the Web", "recording to tape") |
| Menus/toolbar | (`About the menu bar and toolbar`) — items not enumerated in the fetched pages |
| Options dialogs | `Setting general options` (default author, temporary storage location, automatically open last project, save interval for automatic file recovery, automatically download uninstalled codecs, reset warning dialog boxes, clear passwords/user names, restore defaults) and `Setting advanced options` (default durations, video format, aspect ratio, maximum movie file size for e-mail) |
| View switching | storyboard ↔ timeline; `Zooming in and out`; `To fit the timeline on the screen` |

**Not verified:** the exact window title string, the px size of the three-pane window, and the
left/right pane widths. Capture from a real XP install to fill these in.

---

## 14. Windows Media Player 10 / 11 (look only)

> **Status: ◐ PARTIAL — a fetched live demo gives the WMP-9-era chrome, px size and palette; the
> WMP 10/11 tab set (`Now Playing / Library / Rip / Burn / Sync`) is NOT verified by any fetched source.**

Sources: `https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/dist/packages/apps-internet/media-player.js`;
`https://raw.githubusercontent.com/wieslawsoltes/WinXp/main/docs/COMPATIBILITY.md`;
`https://en.wikipedia.org/wiki/Windows_XP` (WMP 10 ships in XP MCE 2005; "Windows Media Format Runtime and
Windows Media Player 11" are listed as upgrades for XP; SP3 "omits security updates for Windows Media
Player 10, although the player is included in Windows XP MCE 2005"; the N/K/KN editions omit WMP).
`https://documentation.help/Windows-Media-Player/` → **404** (no WMP manual in that archive).

**Verified from the demo (WMP 9 style, self-described as such)**

| Field | Value |
|---|---|
| Exact window title | `Windows Media Player` (becomes `"<track> - Windows Media Player"` while playing) |
| Approx. size (px) | **755×525**, min 425×320 |
| Left navigation | `Now Playing`, `Media Library`, `Media Guide`, then `Open...`, `Import...` |
| Playlist pane | heading `Now Playing` / `Media Library — All Media`; tools `+`, `−`, `↑`, `↓` |
| Controls | `▶ Play`, `Ⅱ Pause`, `■ Stop`, `▐◀ Previous`, `▶▌ Next`, `⇄ Shuffle`, `↻ Repeat` (`Repeat off|all|one`), `◖)) Mute`, seek slider, time `00:00 / 00:00`, status `Ready` / `Playing` / `Paused` / `Stopped` / `Opening media...` / `Playback finished` |
| Menus | `File` (`Open...` Ctrl+O, `Import Media...`, `Save Playlist As...`, `Open Playlist...`, `Export Current File...`, `Properties`, `Exit`), `View` (`Now Playing`, `Media Library`, `Show Playlist`, `Enhancements → Graphic Equalizer and Play Speed`), `Play` (`Play / Pause` Space, `Stop`, `Previous`, `Next`, `Shuffle`, `Repeat`, `Mute`), `Tools` (`Add Demo Music`, `Remove Unavailable Files`, `Clear Playlist`), `Help` (`Windows Media Player Help`, `About Windows Media Player`) |
| Palette (used by the demo's CSS) | panel `#183453`, rule `#49607e`, text `#c4d9f0`, visual centre `#0d2448` → edge `#020712`, spectrum bars `#7eeaff` → `#2b9bec` → `#2843ab`, secondary text `#6e7f92`, active row `#287847` |
| About dialog text | `Windows Media Player 9 style player` … `Plays audio and video formats supported by your current browser.` … `The demonstration music is an original synthesized composition.` |

**Unverified for WMP 10/11:** the five top-level nav labels, the WMP 10 dark-blue and WMP 11 black chrome,
the `Library` tree columns, `Rip`/`Burn`/`Sync` layouts, and the "Now Playing" mini-player/visualizer
choices. Also note ReactOS ships **no** WMP: `base/applications` contains `mplay32` (classic Media Player)
and `sndrec32` (Sound Recorder, menus `&File`/`&Edit`/`Effect&s`/`&?`, title `Sound Recorder`) — neither is
Windows Media Player.

---

## 15. MSN Explorer / NetMeeting / Remote Desktop Connection

> **Remote Desktop Connection: ✅ VERIFIED (ReactOS `mstsc` resource).**
> **NetMeeting and MSN Explorer: ✗ UNVERIFIED (absent from ReactOS; no dialog source fetched).**

### 15.1 Remote Desktop Connection — ✅ VERIFIED

Source: `https://raw.githubusercontent.com/reactos/reactos/master/base/applications/mstsc/lang/en-US.rc`

**Purpose:** connect the client to a remote Windows terminal server / XP Pro host.

**Exact window title:** `Remote Desktop Connection` (`IDD_CONNECTDIALOG`, 260×262 DLU = **390×426 px**,
`DS_CENTER`, modal frame; contains a SysTabControl at 7, 50, 246, 190 DLU, `Co&nnect` (default, 147, 245,
50, 14 DLU) and `Cancel` (203, 245, 50, 14 DLU)).

**Tabs:** `General` (`IDS_TAB_GENERAL`), `Display` (`IDS_TAB_DISPLAY`). Each tab page is 242×175 DLU =
**363×284 px**, `WS_CHILD`.

| Tab page | Exact strings and controls |
|---|---|
| `General` (`IDD_GENERAL`) | group `Logon settings`; `Enter the server address.`; `&Server:` + combo (89, 39, 131, 15); `&User name:` + edit (89, 55, 131, 14); group `Connection settings`; `Save the current connection settings or open an existing configuration.`; buttons `S&ave`, `Sa&ve As...`, `Op&en...` (each 50×14 DLU); two 20×20-DLU icons |
| `Display` (`IDD_DISPLAY`) | group `&Remote desktop size`; `Set the screen size of your remote desktop. Drag the slider to the far right to go fullscreen.`; trackbar `IDC_GEOSLIDER` (56, 42, 124, 17); `Less` / `More`; centred resolution text `IDS_PIXEL "%lux%lu Pixels"` / `IDS_FULLSCREEN "Full Screen"`; group `&Colors`; colour combo `IDC_BPPCOMBO` (56, 102, 128, 80); owner-draw colour strip (56, 121, 127, 10, `SS_SUNKEN`); `Note: Settings on the remote computer might override this setting.` |
| Colour list | `256 Colors`, `High Color (15 bit)`, `High Color (16 bit)`, `True Color (24 bit)`, `Highest Quality (32 bit)` |
| Other | `IDS_BROWSESERVER "<Browse for more...>"`, `IDS_HEADERTEXT1 "Remote Desktop"`, `IDS_HEADERTEXT2 "Connection"` |

**Left-panel illustration:** none. **Font note:** the connect dialog is declared with
`FONT 8, "MS Shell Dlg", 400, 0, 0x1` (charset 1 = DEFAULT_CHARSET), unlike the tab pages
(`0x0` = ANSI_CHARSET) — worth reproducing if text metrics matter.

### 15.2 NetMeeting — ✗ UNVERIFIED

ReactOS has no NetMeeting (`https://github.com/reactos/reactos/tree/master/base/applications`).
The archive `https://documentation.help/` lists only **`Microsoft NetMeeting Chat`** and
**`Microsoft NetMeeting Whiteboard`** manuals (per-component help, not the main window). Wikipedia confirms
"Microsoft NetMeeting 3.02" as an XP upgrade (`https://en.wikipedia.org/wiki/Windows_XP`).
No fetched source gives the main window title, the `Place a Call` dialog, the `Audio Tuning Wizard`, or any
geometry → **all unverified**.

### 15.3 MSN Explorer — ✗ UNVERIFIED

Only an existence claim is sourced: XP "was originally bundled with Internet Explorer 6, Outlook Express 6,
Windows Messenger, and **MSN Explorer**" (`https://en.wikipedia.org/wiki/Windows_XP`).
No ReactOS component, no fetched page with dialog strings or geometry → **all unverified**.
(Windows Messenger, the other bundled IM client, has a help manual in the `documentation.help` A–Z index as
`Windows Messenger .NET`, but that page was not fetched for this spec.)

---

## 16. Summary

| # | Item | ReactOS source? | Verified level | Title | Size (px) | Left panel | Source of record |
|---|---|---|---|---|---|---|---|
| 1 | Generic XP wizard shell | yes (`syssetup`) | ✅ | per-page `CAPTION` | 476×314 frame / 476×232 page | **yes, 172 px** | `dll/win32/syssetup/lang/en-US.rc` |
| 2 | Windows Update / Automatic Updates | stub only (`wusa`) | ✗ | — | — | — | none (capture needed) |
| 3 | System Restore | no | ◐ (labels only) | — | — | unknown | howtogeek.com walkthrough |
| 4 | Device Manager | yes (`devmgr`) | ✅ | `Device Manager` | 450×650 pane / 530×404 demo | none | `dll/win32/devmgr/lang/en-US.rc` |
| 5 | Disk Defragmenter | no | ◐ | `Disk Defragmenter` | 650×440 | none | `apps-system/management.js` |
| 6 | Disk Cleanup | yes (`cleanmgr`) | ✅ | `Disk Cleanup` / `Disk Cleanup for (C:)` | 353×349 / 430×390 | none | `cleanmgr/cleanmgr.rc` |
| 7 | Backup | no | ◐ | `Backup or Restore Wizard` | 520×390 | unknown (expect §1) | `apps-system/management.js` |
| 8 | Files and Settings Transfer Wizard | no | ◐ (labels only) | — | — | unknown | pctechguide walkthrough |
| 9 | Network Setup Wizard | wizard pages only | ◐ | `ReactOS Setup` (XP: not verified) | 476×232 per page | unknown | `dll/shellext/netshell/lang/en-US.rc` |
| 10 | Remote Assistance | no | ✗ | — | — | — | none (capture needed) |
| 11 | Windows Tour | no | ✗ | — | — | — | none (`xptour` id only, via `FEATURE-GAPS.md`) |
| 12 | Security Center | no | ✗ (name only) | `Windows Security Center` (likely) | — | unknown | en.wikipedia.org/wiki/Windows_XP |
| 13 | Movie Maker | no | ✅ (strings) / ✗ (size) | — | — | panes documented | documentation.help/Windows-Movie-Maker |
| 14 | WMP 10 / 11 | no (`mplay32` ≠ WMP) | ◐ (WMP 9 chrome only) | `Windows Media Player` | 755×525 | none | `apps-internet/media-player.js` |
| 15 | Remote Desktop Connection | yes (`mstsc`) | ✅ | `Remote Desktop Connection` | 390×426 / 363×284 page | none | `base/applications/mstsc/lang/en-US.rc` |
| 16 | NetMeeting | no | ✗ | — | — | — | none (capture needed) |
| 17 | MSN Explorer | no | ✗ | — | — | — | none (capture needed) |

**Legend:** ✅ verified from a fetched primary source (ReactOS resource units, or a product help file);
◐ partial (some strings from a fetched source, geometry/title missing); ✗ unverified — do **not** implement
from this document.

### 16.1 Porting order suggested by the evidence above

1. §1 wizard shell — it is the container for §3, §7, §8, §9 and much of §13.
2. §6 Disk Cleanup, §4 Device Manager, §15.1 Remote Desktop Connection — fully specified, straight ports.
3. §5, §7, §13, §14 — port the verified strings now, style the chrome from the §1 shell, and mark the
   unverified parts with `// TODO(capture)`.
4. §2, §10, §11, §12, §15.2, §15.3 — **blocked**: capture first (below).

### 16.2 Capture checklist for the unverified items

Run a real XP SP3 instance (a VM, **not** the V86 canvas at `oses.ioblako.com`, which exposes no DOM —
§0.2) and, for each, record window title, client size in px, every tab/button/label string, and a note on
whether the left billboard of §1 is present:

- [ ] `Automatic Updates` control-panel applet + the `wuauclt`/Windows Update progress dialogs (§2)
- [ ] `System Restore` wizard: title, page sequence, the calendar pane, confirmation (§3)
- [ ] `ntbackup` wizard page list + Advanced-mode tabs (§7)
- [ ] `Files and Settings Transfer Wizard`: `Old computer` branch, Wizard-Disk branch, file-selection tree (§8)
- [ ] `Network Setup Wizard` (`netsetup.exe`): title, page list, illustration (§9)
- [ ] Remote Assistance: invitation wizard, approval dialog, in-session Chat/File Transfer toolbar (§10)
- [ ] `Tour Windows XP`: page/chapter list, illustrations, close behaviour (§11)
- [ ] Security Center: banner text, the three status rows, `Change the way Security Center alerts me` (§12)
- [ ] Movie Maker: window title, three-pane px widths, menu bar items (§13)
- [ ] WMP 10 **and** 11: five nav labels, colour values, library columns (§14)
- [ ] NetMeeting 3.02 main window + `Place a Call` + `Audio Tuning Wizard`; MSN Explorer sign-in chrome (§15.2/§15.3)

---

## 17. Notes, assumptions and licensing

- **DLU conversion** assumes `MS Shell Dlg` = Tahoma 8 pt with base units 6×13 at 96 DPI — the standard for
  XP dialogs. At other DPI or with a different shell font the divisors change; keep the conversion in one
  helper if it is ever needed at runtime.
- All ReactOS strings above are **GPL-2.0-or-later** (resource files carry that header; `cleanmgr` is MIT,
  per `cleanmgr.rc`). They are transcribed here as *reference text* to be matched against XP, consistent
  with the transcription policy in `docs/PROVENANCE.md`. ReactOS renames `Windows` → `ReactOS` in several
  status strings; port the XP wording, keep the ids.
- `wieslawsoltes/WinXp` states in `docs/COMPATIBILITY.md` that it "has **not been verified as
  pixel-perfect Windows XP**", so its px sizes are treated as *reference geometry*, not ground truth, and
  are marked ◐.
- `documentation.help` is a static mirror of Microsoft's own XP-era product help (©2000-2002 Microsoft);
  its strings are the product's wording, but the archive is a third-party mirror.
- Wikipedia is used only for *existence and naming* claims (Security Center, WMP 10/11, Remote
  Assistance/Remote Desktop, NetMeeting 3.02), never for strings.
- Nothing in this file was taken from the Windows XP source leak or any leaked asset; see the policy in
  `docs/PROVENANCE.md`.
