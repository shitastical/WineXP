# Implementation handoff — XP desktop / Wine workflows

## Goal

A functional XP-style Linux desktop controlling a real Wine prefix, preserving
XP chrome and expected behavior. No distro work is part of this batch. Neither
pixel equivalence nor universal Windows software compatibility is established.
Existing uncommitted changes from the previous session were preserved.

## v0.01.02 — this batch

Six reported defects, each fixed and covered by a test that fails without the fix.

**Start menu submenu travel.** `state.open` was reassigned from "is the pointer
over this row", so the submenu closed the moment the pointer left the row on its
way into the flyout — the user could never reach *All Programs > Accessories >
Accessibility*. Now a row only switches the open submenu when the pointer is
directly over it, and leaving both the panel and the flyout closes it (that
second half fixes a stale flyout surviving leaving the menu).
`src/startmenu.rs::travel_tests` drives real frames through egui and reproduces
the whole path.

**Recycle Bin.** Three separate problems:
- The five "PNG" icons (`bin/recycle/recyclebin/trash/emptybin`) were *HTML error
  pages* saved with a `.png` name — five byte-identical copies, one md5. Replaced
  with icons drawn from scratch by `scripts/make-recycle-icons.py` (the upstream
  shell32 artwork is not redistributable).
- Items were listed by their storage id (`1789423123-report.txt`), not the name
  the user recognises. `Vfs::list_trash` now returns display name + original
  location + id.
- Deleting inside the bin called `recycle()`, which refuses, so it just errored.
  Added `Vfs::purge` (permanent) and `Vfs::empty_trash`, plus a Restore/Empty
  File menu, a trash-aware context menu, and permanent-delete wording in the
  confirmation dialog.
`vfs::Entry` gained an `id` field because two files recycled under the same name
must stay separately selectable; selection now keys on `id`, not the display name.

**Internet Explorer typing.** `GdkEvent.string` was never set, so WebKit received
key presses with no characters and every page looked focused while swallowing
keystrokes. The helper now builds `Gdk.EventKey` directly and carries the text
payload (`src/browser_helper.py`).

**System sounds.** New `src/sound.rs` plus a *Sounds and Audio Devices* applet
(`src/apps/sounds.rs`), 12 XP sounds wired to startup, logon, logoff, shutdown,
recycle, restore/delete/empty and file-operation errors.
Deliberately **zero new dependencies**: every pure-Rust audio crate links ALSA
development headers, which are absent on a default desktop and would break the
"install.sh works on any distro with just the binary" promise. The engine writes
the embedded WAV to a cache file and hands it to whatever player the distro has
(`pw-play`, `paplay`, `aplay`, `ffplay`, `mpv`). It parses the RIFF header to
measure length, so the boot jingles do not stack, and every failure is silent.
Fetch the audio with `scripts/fetch-sounds.sh`.
**Licensing:** the WAVs are Microsoft's, taken from the public Reborn XP
distribution. They are not covered by this project's licence — see the README
before redistributing a build.

**Installer icon.** The menu entry used `assets/login/logo__windows_xp.png`
(272x164) written to every size. It now uses `assets/winxp-icon.png` (the
project's own icon) rendered to 16/32/48/64/128/256/512 at install time and
written to `hicolor/<size>/apps`, `pixmaps/`, `share/pixmaps/` and `icons/`.

**Test flake (not a reported bug).** `tests/update.rs` failed a *different* test
on nearly every run. Cause: each test copies a ~35 MB binary and execs it while
a concurrent copy is still open for writing → `ETXTBSY`. `Sandbox::run` now
retries that specific error. `temp_root()` also used a single shared probe file
that parallel tests deleted out from under each other.

66 tests pass (56 binary + 3 + 7 integration); `cargo build --release` is
warning-free.

## Added in this batch

- `src/sysinfo.rs`: asynchronous detection in system app windows; bounded command
  execution; fastfetch / neofetch / screenfetch, procfs/sysfs, lscpu/lspci/uname/df
  sources. Real CPU family/model/stepping, architecture, memory/swap and BIOS.
  Missing hardware values are not invented.
- System Properties: active account registration name, wrapped CPU text,
  Device Manager / Windows Update actions, host environment viewer.
- System Information: correct category collapse ownership; independent scrolling;
  real CPU/memory summary, environment and running host tasks; Copy/Refresh/Tools
  menus. Unimplemented hardware resource queries are explicitly unavailable.
- Device Manager: sysfs display/disk/network/sound/input/USB/PCI inventory,
  scan and read-only properties. Reachable via Hardware and `devmgmt.msc`.
- Folder Options: General/View/File Types panels and saved hidden-file,
  known-extension, single/double-click, task-pane and title-path preferences.
  Explorer reloads preferences periodically. Reachable from Control Panel,
  My Computer Tools, Explorer Tools and `control folders`.
- Explorer: address field + Enter/Go, Back/Forward keyboard shortcuts, real Wine
  file associations, file-operation context menus, New Folder, Rename,
  recoverable recycling/restore, properties, shared Cut/Copy/Paste clipboard.
  Filesystem changes operate on the actual Wine C: drive. Existing items are
  not intentionally overwritten; recursive symlink copying is rejected.
- Run: quoted executable paths and arguments forwarded to Wine without host
  shell evaluation, plus additional native app aliases and errors.
- Wine Configuration: direct native winecfg, regedit, uninstaller and cmd actions.
- Registry Editor: Wine hive parsing (escaped paths and timestamp headers), key
  navigation, real values, REG_SZ/DWORD edits through Wine reg.exe on a worker.
- Add or Remove Programs: access to Wine's registered software manager,
  installation via Run, and refresh.
- Internet Options: persisted homepage, blank/default home choices, history
  clearing, JavaScript and zoom; real Wine Internet Settings shortcut.
  IE favorites/history are saved, with history capped at 200 entries.

## Real installation

- `src/setup.rs` installs the running AppImage (or developer binary) per user,
  self-checks the copied executable with `--version`, backs up an existing build,
  creates `~/.local/bin/winxp`, desktop entry and embedded XP icon, then writes an
  installation receipt. No formatting, account reset or Wine-prefix erasure.
- Fresh AppImage launches are gated into the XP-styled Setup wizard before the
  desktop/start option. Developer raw-binary launches retain `--skip-boot`.
- Setup displays actual worker progress, capability diagnostics, retryable errors
  and copyable error details. Existing installation may be repaired from boot.
- `--install` and `--installation-status` expose real setup to scripts; install.sh
  invokes the same installation path. No distribution package manager is run.
- A per-user systemd timer is created/enabled when a functioning user service
  manager is available. It stages verified updates daily; the existing updater
  applies with backup/self-check/rollback on launch. Without systemd user services,
  Setup explicitly reports manual Windows Update fallback.
- Isolated CLI installation was exercised with temporary HOME/XDG directories,
  no package manager/systemd PATH, and no real user configuration changes: copied
  app, launcher `--version`, desktop entry, XP icon and receipt all passed.
- Final debug build, 28 binary unit tests and install.sh shell syntax passed.
  No packaged release AppImage or actual automatic-update timer was tested.

## Real browser

- `src/browser.rs` is a Rust IPC/texture/input bridge to embedded
  `src/browser_helper.py`, which uses the installed WebKitGTK engine.
- Browser creation is lazy to avoid spawning duplicate engines during the
  window manager's temporary natural-size probes.
- Existing IE toolbar artwork/layout is retained. Back/Forward, Stop, Refresh,
  Home, editable Address/Go, Search, session Favorites/History, and Print are
  wired. Mail/Messenger attempt Wine programs, which may not be installed.
- Pages support real HTTPS/JavaScript, pointer/link clicks, text/special keys,
  scrolling. Popup navigations are redirected into the current page.
- Download progress and completion are shown in File Download. Files go into
  `<Wine drive_c>/users/<host USER>/Downloads`; Run/Open Folder use Wine.
  Downloads are never automatically executed.
- Worker starts lazily (not for temporary geometry probes) and shuts down when
  the browser is dropped. Bounded IPC queue; one page texture; 4 fps active /
  1 fps idle snapshot baseline. This is **not** smooth video rendering.
- Missing dependencies retain an explicitly labelled offline demo fallback.

### Linux dependencies

System Python with `gi`, Cairo integration, GTK 3, WebKit2 4.1 typelib/runtime.
On Debian/Ubuntu the relevant packages are typically `python3-gi`,
`python3-gi-cairo`, `gir1.2-gtk-3.0`, `gir1.2-webkit2-4.1`.
`WINXP_BROWSER_PYTHON` overrides `/usr/bin/python3` for different installations.
No Chromium bundle or new Cargo dependency was added.

The renderer currently needs a native hidden GTK window; on X11 it is placed
outside the visible screen. Wayland sessions use XWayland when DISPLAY exists.
**Pure Wayland without XWayland is not verified and may expose the helper
window.** A direct native surface or better portable offscreen backend remains
needed. AppImage packaging does not bundle these system dependencies.

## Checks actually run

- Rust compile and binary unit tests passed during implementation (28 tests).
  The final check result is reported in the session summary.
- Debug build and Xvfb desktop launch exercised. An IE address widget ID collision
  exposed at launch was repaired; later launch stayed alive with no application
  panic. Software EGL/DRI warnings are expected under Xvfb.
- WebKit worker loaded live `https://example.com/` with nonblank snapshot pixels.
- Local HTTP workflow: page load, pointer link click, Back, Forward and .exe
  fixture download passed; downloaded bytes matched. Text entry updated the page
  title successfully in a separate fixture.
- VFS unit workflow: create, rename, copy, recycle, restore and overwrite/self-copy
  refusal passed against an isolated temporary drive.
- Wine registry parser / quoted command tests passed.

No real installer was executed; no live Wine registry edit/uninstall was tested.
No pixel comparison to real XP, release packaging, cross-distro matrix or full
integration test suite was run. Do not call this pixel-perfect or complete.

## Next priorities / known gaps

0. Installation does not bundle Wine/WebKit/Python or guarantee arbitrary distro
   compatibility. Add optional dependency bundling and a distribution matrix;
   do not call this universally self-contained. External installer dialogs still
   need a Wine XP theme and testing against actual applications.
1. Review browser integration through the actual IE UI; keyboard focus, clipboard,
   IME, native file chooser/security dialogs and context menus need broader work.
2. Measure memory/CPU on representative workloads. WebKit is a multiprocess
   modern engine and is not negligible; no speedup/lightweight guarantee is made.
3. Make XP-shaped modal controls instead of default egui Window/Grid controls in
   the new dialog subsets; match tabs/spacing/fonts against real screenshots.
4. Implement true Performance/User Profiles/Startup Recovery dialogs rather than
   unrelated app launches (unsupported controls currently disabled).
5. System Restore/Remote/Automatic Updates options remain shell UI subsets, not
   host services. Hardware disabling/driver installation is not implemented.
6. Registry writes require Wine; persisted hive files can lag the Wine server.
   HKU is only the default hive view; HKCR merged-user semantics are incomplete.
7. Handle cancellation/partial recursive copies and case-insensitive Windows
   filename collision semantics. Selected indices can shift after directory edits.
8. Complete native common Open/Save dialogs and route all app file workflows
   through them. My Computer remains largely a source-demo view.
9. Complete Internet Options privacy/cache/security-zone controls; basic
   homepage/history/JavaScript/zoom preferences are implemented.
10. XP-style rendering of external Wine installers requires an actual Wine theme;
    this batch does not force global Wine configuration or promise modern app
    compatibility.

Rust retains `#![forbid(unsafe_code)]`; native WebKit/Wine dependencies do not
inherit Rust's memory-safety guarantee. No commits, releases or deployments made.
