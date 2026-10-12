# WineXP — Detailed Plan to a Completish XP Desktop

## 1. Product definition

WineXP should become a practical, native Linux application that presents an
accurate Windows XP desktop, manages its own installation and updates, opens
real files, browses the real web, and runs/manages Windows applications through
Wine. The user should use ordinary desktop actions rather than terminal Wine
commands. Developer diagnostics must remain available without confusing normal
workflows.

The target for this roadmap is **completish**, not a claim that WineXP is Windows
or that every Windows executable is compatible. A completish release has a
coherent daily-use feature set, no fake success in its core workflows, tested
installation/update safety, credible XP fidelity, clear compatibility limits,
and a documented way to diagnose unsupported operations.

### Non-goals for this roadmap

- Building a Linux distribution, installer partitioner or bootloader.
- Formatting disks, replacing a user's desktop session, changing global Wine
  configuration, or installing privileged system packages without consent.
- Reimplementing the Windows kernel, device drivers, DirectX or Wine itself.
- Guaranteeing arbitrary modern applications or software requiring XP drivers.
- Themes for Windows 7/10/11 before the XP base works.
- Shipping leaked Windows source or pretending proprietary assets are freely
  redistributable.

### Meaning of completion

A feature is implemented only when the user-facing interface performs its
advertised action. It is verified only when the relevant observable behavior
has been checked. Building does not prove a GUI works. A static checkbox is not
an implemented setting. Launching an unrelated window is not implementation of
an XP property dialog. Compatibility must name tested distributions, sessions,
architectures and applications rather than say “ANY Linux system.”

## 2. Current baseline and debt

The repository contains a Rust egui desktop/window manager, XP boot/logon/OOBE,
Control Panel, editors, games, command shell, a Wine C: filesystem bridge,
accounts/settings, and a verified-release updater.

Recent additions include real per-user setup, a WineXP launcher TUI,
WebKit-backed browsing behind the existing IE interface, download actions,
Folder/Internet Options, host System Information and Device Manager, real Wine
registry access, and Explorer file operations.

These additions are not proof of completeness:

- WebKit currently renders snapshots through a system Python/GI helper. Native
  dialogs, pure Wayland, resource use and high-frame-rate content need work.
- External Wine applications are separate native windows, not yet coherently
  represented in the simulated desktop/taskbar.
- Several property pages and deep tools remain partial or unavailable.
- Settings/error/modal controls are inconsistent between applications.
- Explorer is only a subset of XP Explorer; common file dialogs are not unified.
- Existing README/gap/provenance documents lag the implementation.
- The current AppImage does not bundle every browser/Wine dependency.
- No visual comparison against a real XP SP3 reference is established.

## 3. Definition of a completish release

All of the following are release gates:

1. A downloaded artifact opens setup when appropriate, performs a real
   non-destructive per-user installation, creates a desktop launcher, preserves
   documents/settings/prefixes on repair and uninstall, and reports failures.
2. Installed startup reaches a working desktop with accounts and persisted
   settings. Logoff/restart/standby have intentional, documented behavior.
3. Shell input, focus, task switching, context menus, minimization/restoration,
   and keyboard conventions work reliably at supported display sizes.
4. Explorer can navigate, create, rename, copy/move, recycle/restore, inspect and
   open files. Editors use a shared file picker/save workflow.
5. IE performs real navigation/search/history/favorites/downloads. Downloads
   can be opened through the desktop and installers launch without Wine CLI use.
6. Installed programs can be discovered, launched and uninstalled from WineXP.
   Failures expose actionable diagnostics rather than a fictitious success.
7. Common XP settings are functional or explicitly unavailable. No routinely
   used control silently does nothing.
8. Core app workflows and games are useful, not merely visual placeholders.
9. Updates are checked/staged/applied with integrity checks, backup, rollback
   and understandable user notification. Absence of a release is not failure.
10. A measured visual review covers the desktop, Start menu, Explorer, dialogs,
    setup and major applications. Deviations are recorded and bounded.
11. A support matrix covers at least representative Debian/Ubuntu-family,
    Fedora-family and Arch-family environments, X11 and a defined Wayland path.
12. Runtime profiling and reliability checks support stated resource targets.
13. Source safety, packaging policy, asset provenance and limitations are clear.

## 4. Execution rules

- Preserve unrelated and pre-existing work; do not reset the working tree.
- Use existing Rust/egui conventions and assets first; avoid unnecessary new
  dependencies. Rust retains `#![forbid(unsafe_code)]`.
- Implement useful vertical slices before broad visual-only scaffolding.
- For every slice record: UI entry point, data source, operation, persistence,
  failure behavior, recovery, checks, and remaining limitations.
- Test state-changing operations in isolated HOME/XDG/WINEPREFIX directories.
- Never run downloaded installers or alter the real Wine registry/prefix as a
  test without specific authorization for the chosen operation.
- Verify final edited files after repairs. Preserve command exit status through
  output pipelines. Do not suppress warnings/assertions to pass a check.
- Prefer cancellation/backups to irreversible removal; never format anything.
- Ask about genuinely consequential choices, not routine implementation details.
- Do not deploy/publish/push merely because a development plan mentions releases.

## 5. Ordered workstreams

### Phase A — Stabilize the shared foundation

**Purpose:** consistent controls, reliable state and user-facing errors unblock
nearly every other feature. This is the first execution phase.

A1. Inventory every reachable desktop/Start/Run/Control Panel entry. Classify it
as working, partial, disabled or missing. Separate dead controls from explicitly
disabled functionality. Link classification to source and a user workflow.

A2. Establish shared XP property-page controls: button states, disabled text,
keyboard-focus rectangle, checkbox/radio geometry, tabs and multi-row tab
navigation, group-box borders, input fields, scrollbars and standard margins.
Use the existing palette and trace visual values to reference screenshots or
existing source measurements. Avoid globally changing browser content styling.

A3. Add keyboard conventions: Tab/Shift+Tab focus, Space activation, Escape
Cancel, default-button Enter where appropriate, Ctrl+Tab property pages,
arrow-key navigation, Alt accelerators and focus restoration after modals.
Ensure events affect only the focused window, not every drawn app.

A4. Introduce consistent XP modal/message infrastructure. Error dialogs need
icon, title, wrapped message, default/cancel behavior and details/copy action.
Log technical detail without exposing secrets. Normal user wording should say
what failed and what can be done; debugging detail can identify Wine/WebKit.

A5. Make settings writes atomic, validated and observable. Report permission,
full-disk/newer-schema errors. Prevent stale settings snapshots overwriting
unrelated changes. Add schema migration strategy before incompatible fields.

A6. Verify shared controls through headless egui interaction tests and real
rendered windows. Keep test assertions for disabled controls and modal focus.

**Acceptance:** focused dialogs can be used with keyboard and pointer; disabled
controls cannot activate; Cancel does not persist changes; Apply failure is
visible; no duplicate widget-ID panic; unit/integration checks pass.

### Phase B — Real installation and maintenance

B1. Separate application installation state from first-run account/OOBE state.
A missing receipt/binary triggers Setup; an existing install can be repaired
without deleting accounts. Developer launch remains available deliberately.

B2. Implement per-user installation with staging, executable self-check,
atomic replacement, backup, launcher, desktop entry and logo. Respect absolute
XDG directories and unusual HOME characters. Reject unsafe/invalid destinations.
Keep source-copy/install errors recoverable and preserve the installed build.

B3. Replace fake formatting/reboot progress with XP-styled truthful setup pages:
Welcome, information, destination/capabilities, copy/register, completion/error.
Progress reflects actual operations. Installation cancellation cannot mark the
application installed. Never claim installing Windows drivers or formatting.

B4. Audit install.sh/run.sh parity, quoting, signals, failure exit codes and
single-instance management. Stop only a verified tracked process. Document
headless commands and useful logs. TUI must work without ANSI color and recover
from a failed operation without discarding its diagnostic output.

B5. Implement safe uninstall: remove owned application/integration/update timer,
preserve user documents/accounts/settings/prefix by default; any optional data
purge requires explicit separate authorization and clear scope.

B6. Align updater targets with installed artifacts. Handle AppImage/raw binary,
FUSE/extraction mode, downloaded copies, staged pending updates and concurrent
checks. Exercise verify/stage/apply/rollback through actual installed launchers.

B7. Automatic updates: persisted preference, visible status, manual check,
verified staging, deferral while busy and restart notification. User service
manager integration is optional; non-systemd systems need a portable in-app
scheduler. Never silently report a timer installed if activation failed.

B8. Dependency strategy: capability detection before setup completion, optional
bundled runtimes, dependency hints and explicit unsupported conditions. Avoid
privileged installation or arbitrary package-manager commands. Keep packaging
architecture-specific and test the stated ABI floor.

**Acceptance:** fresh install/repair/uninstall/error cases pass in isolated
homes; launcher survives update/rollback; an uninstalled AppImage cannot bypass
Setup by selecting Start; no real user files are erased; diagnostics are useful.

### Phase C — Complete shell behavior

C1. Desktop/Start menu hitboxes must match painted geometry. Ensure nested menus
stay open across pointer travel and close appropriately; keyboard navigation,
Escape/outside click and long-list scrolling must be correct.

C2. Window focus/z-order: no click-through, modal background input, active-title
mismatch or keyboard broadcast. Drag/resize/min/max restoration must honor
screen bounds and small resolutions. Dynamic app titles update task buttons.

C3. Taskbar/Quick Launch/Show Desktop, tray click/double-click, clock properties,
window task-button right-click, taskbar context menu and lock/auto-hide options.

C4. Alt+Tab and keyboard shortcuts: Win menu, Win+R, Win+D, Alt+F4, Ctrl+Esc,
Alt+Space and restore paths. Specify how Linux desktop shortcuts interact with
WineXP when it is not a full session.

C5. Desktop file icons backed by a real user desktop folder: create/rename,
selection/marquee, drag/drop, copy/paste/recycle and persistent arrangement.
Special icons have the correct Properties actions.

C6. Logon/account behavior: name/picture/password/disabled-account handling,
lock/logoff/switch-user semantics. Replace weak password storage before claiming
security. Shell accounts are not Linux users or Wine security boundaries.

C7. Standby and screensaver are shell features unless OS integration is explicitly
implemented. Do not claim the Linux machine suspended when only the view dims.

**Acceptance:** first-five-minutes shell workflows work by pointer and keyboard;
no background app reacts to focused input; settings survive restart; taskbar and
menus remain usable at 800×600 and 1024×768.

### Phase D — Explorer, documents and common dialogs

D1. Define Windows path semantics for C:, Wine mappings and supported drives.
Normalize separators/root parents, handle Unicode/spaces, case-insensitive lookup
where appropriate, and avoid accidental host-root exposure through invalid paths.

D2. Explorer: editable Address/Go, Back/Forward/Up history, Refresh, folder tree,
common task pane, details/status, sort and view modes (Icons/List/Details first;
Tiles/Thumbnails later). Selection must follow stable paths, not shifting indices.

D3. File operations: New Folder/file, Rename, Copy/Cut/Paste, conflicts, progress,
Cancel, readonly errors, properties and permissions. Long recursive operations
run off the UI thread. Define symlink policy and partial-copy cleanup explicitly.

D4. Recycle Bin: original name/path/deletion date, restore conflicts, restore all,
empty with confirmation, metadata consistency and interrupted operation recovery.
Never confuse recycle with permanent delete.

D5. Shared XP Open/Save As/Browse for Folder dialogs: places bar, location, list,
filename/type filters, default extension, overwrite confirmation and failure
messages. Route Notepad/Paint/WordPad/image viewer through the same interface.

D6. File associations: enumerate real Wine classes/handlers; native app overrides
for files WineXP handles; Open With and association changes with confirmation.
Shortcuts require actual target/arguments/working-directory handling.

D7. Host drag/drop import: explicit copy into the Wine drive, collision and size
handling, no silent movement of original host documents. Downloads are visible
from Explorer and can be launched using the same association path.

**Acceptance:** create folder → edit/save → copy/move → recycle/restore → restart
→ reopen works on isolated and representative real prefixes. No silent overwrite,
UI blocking on long copies, stale selection, or fake file-operation success.

### Phase E — Wine runtime and installed-program experience

E1. Centralize executable selection (`WINE`/PATH), prefix, architecture, working
directory, environment, arguments and logging. Reap children and report spawn
versus application-exit failures distinctly. No host shell interpolation.

E2. Downloaded EXE/MSI/batch/document opening: normal Open/Run actions, safety
prompt where XP would show one, progress/completion state and clear errors.
MSI invokes the correct Wine association/msiexec flow. Never auto-run downloads.

E3. Installed-program inventory from both Program Files and actual uninstall
registry. Support per-user/32/64-bit registrations, display name/version/publisher,
uninstaller arguments and shortcuts. Refresh after installer/uninstaller exits.

E4. Add or Remove Programs should use real registrations, not guess removal by
folder deletion. Launch the proper uninstaller, show actual result and preserve
logs. Include repair/change actions only when the application provides them.

E5. Integrate external Wine windows into the desktop/session: identify ownership,
track task buttons/focus/minimize/close, handle dialogs and multi-window apps.
Investigate Wine virtual-desktop/session integration without forcing global
prefix configuration. Wayland/X11 limitations must be explicit.

E6. XP appearance for external applications: evaluate Wine theme support, window
manager decoration, fonts and DPI in a project-owned prefix. Obtain user consent
before changing an existing prefix. Keep per-application overrides reversible.
Application-owned custom installers cannot be guaranteed identical to XP.

E7. Registry editor: live Wine queries/writes, hive semantics, default/string/DWORD/
binary types, create/rename/delete keys/values, import/export, confirmation,
backup and permission errors. Avoid direct edits to live registry hive files.

E8. Configuration UI: prefix information, drives, version overrides, graphics,
audio, libraries and application-specific settings backed by real operations.
Do not claim kernel driver management or fake network/driver state.

**Acceptance:** a chosen authorized installer can be downloaded, launched,
registered, opened from WineXP and uninstalled. External windows behave coherently;
errors are diagnosable; no Wine CLI is required for routine user actions.

### Phase F — Internet Explorer as a usable browser

F1. Preserve IE chrome and exact toolbar geometry while replacing simulated
content with a maintained real engine. Keep fallback visibly labeled, not a
fake live page. Test HTTPS, redirects, JavaScript, cookies and popup navigation.

F2. Navigation: Address/Go, Back/Forward history, Stop, Refresh, Home, Search,
status/progress/title, open-link actions, keyboard shortcuts and focus handoff.
Page input must support selection, copy/cut/paste, scrolling, Tab and IME.

F3. Favorites/history: persistence, organize/remove, folders, title display,
history grouping/clear, address suggestions and privacy deletion semantics.
Changes in one browser window must not overwrite another window's state.

F4. Downloads: authentic File Download/security/save-progress/completion dialogs,
Save/Run choices, destination selection, collision handling, cancellation,
network errors, resume if supported and Open Folder. Validate completed files
before marking them complete; do not report interrupted downloads as success.

F5. Internet Options: homepage, browsing data/cookies/cache, privacy preferences,
JavaScript, zoom/accessibility, supported proxy settings and certificate
information. Unsupported XP zones/ActiveX should be disabled/explained.

F6. Native browser dialogs: file upload, permissions, print, authentication,
certificate errors and JavaScript dialogs should appear inside a coherent XP UI
where supported; no invisible helper popup that stalls browsing.

F7. Replace/upgrade the snapshot bridge where needed: frame throttling while
hidden/minimized, bounded input/output, worker restart and cleanup, no GUI-thread
blocking, pure Wayland strategy and documented rendering limitations.

F8. Profile representative pages: static article, form, downloads and dynamic
site. Measure memory/CPU/startup. Do not claim modern WebKit is negligible or
that native libraries inherit Rust's memory-safety guarantee.

**Acceptance:** navigation, form input/paste, file upload/download, Home/options,
favorites/history and recovery pass through actual IE controls. No runaway
workers or hidden blocking dialogs; missing runtime yields useful diagnostics.

### Phase G — Settings and everyday application completeness

G1. Audit Control Panel entries and expected tabs. Prioritize Display, Folder,
Internet, Taskbar/Start Menu, System, Date/Time and User Accounts. Functional
settings must persist and affect the actual appropriate subsystem.

G2. Display: real wallpaper choices/layout, desktop preview, screen saver subset,
appearance preview and supported scaling. Do not change host resolution without
explicit confirmation/revert timeout. Theme work remains XP-focused.

G3. System Information: truthful detected CPU/RAM/BIOS/GPU/storage/network,
asynchronous bounded sources, refresh/export/search, unavailable instead of
invented fallback hardware. Separate recreated OS identity from actual host OS.

G4. System Properties: correct General layout, name/description state, real
Device Manager subset, Environment Variables scope, and meaningful Performance,
Profiles/Recovery/Remote tabs only where supported. No misleading service toggles.

G5. Notepad: New/Open/Save/Save As, dirty-save confirmation, editing/clipboard,
Find/Replace/Go To, word wrap, font, status, Time/Date, encoding/line endings.

G6. WordPad: consistent file workflows and honest format support, selection,
formatting/undo/redo/printing subset. Do not label a plain-text export as RTF.

G7. Paint: shared dialogs, dirty-save confirmation, undo/redo, canvas resize,
selection/cut/copy/paste, text, supported image formats, wallpaper action.

G8. Calculator: arithmetic and memory correctness, divide-by-zero/error states,
keyboard input. Implement Scientific only when the operations are correct.

G9. Task Manager: actual WineXP app list with Switch/End actions; host process
list permissions and clear scope; performance history; no invented task rows.
Require confirmation for ending external processes, never kill arbitrary hosts.

G10. Help: searchable local topics that accurately describe implemented workflows,
troubleshooting, paths/logs and limitations. About/version identity consistent.

G11. Games/media: finish expected game menus/rules/save records for existing games;
playback/volume must control a real backend before claiming audio support.
Defer nonessential new games until core workflows are complete.

**Acceptance:** each primary app completes its advertised workflow; save errors
and Cancel are correct; common settings affect behavior and survive restart.

### Phase H — Visual fidelity and accessibility

H1. Assemble licensed/usable reference material, preferably XP SP3 screenshots at
1024×768 and 100% DPI. Other recreations are feature/measurement aids, not proof
of equivalence to Windows. Record source and uncertainty.

H2. Build deterministic screenshot fixtures for desktop, Start/submenus,
Explorer, setup/logon, common dialogs and Control Panel tabs. Isolate settings,
clock/randomness/network and provide fresh process per screenshot where needed.

H3. Compare geometry, fonts, line heights, control spacing, borders, gradients,
icons, selected/disabled/focused states and wrapping. Prioritize biggest visible
errors. Store repeatable scores plus human-reviewed differences; no invented
pass threshold without reference baselines.

H4. Font policy: XP commonly uses Tahoma; current bundled Noto is not identical.
Support legally supplied fonts or appropriate redistributable replacements.
Do not silently distribute host Microsoft fonts. Verify metrics per fallback.

H5. Keyboard and accessibility: labels/focus order, high contrast/large text
where possible, accessible descriptions, tooltip timing and pointer feedback.
Visual fidelity should not make controls unusable at supported scaling.

**Acceptance:** reference coverage and deviations documented; no claim of
indistinguishability without actual comparison; screenshots stable across runs.

### Phase I — Performance, portability, security and packaging

I1. Establish measured budgets after baseline profiling: idle desktop RSS/CPU,
startup latency, browser overhead, input-to-frame behavior, large directory/copy
performance and memory after repeated window open/close. Specify hardware used.

I2. Eliminate unnecessary per-frame filesystem scans/process probes/allocations;
cache with invalidation and do blocking work off the UI thread. Limit logs,
history, frame queues, process polling and image decode sizes.

I3. Audit process lifecycle: reap spawned children, bounded timeouts, cancellation,
helper crash/restart, shutdown and prefix process ownership. Avoid broad pkill.

I4. Keep Rust unsafe forbidden; inspect dependency risks; treat Wine and browser
as separate native attack surfaces. Wine is not a sandbox. Protect logs/paths,
argument boundaries, downloaded execution prompts and filesystem operations.

I5. Compatibility matrix: distro/version, libc floor, desktop/session, GPU/software
rendering, Wine version/prefix architecture, Python/GI/WebKit availability,
AppImage FUSE/extract mode. Test before documenting support.

I6. Package the smallest practical runtime set. Provide optional browser or
capability fallbacks rather than bundling Chromium. Dependencies bundled must
have licenses/security-update policy. Update artifacts cannot change architecture.

I7. Release docs, provenance/license notices, install/repair/uninstall instructions,
known limitations and recovery. Update user-facing branding to WineXP consistently.

I8. CI: compile, scoped formatting/lint, unit/integration tests, isolated install/
update checks, browser tests when dependencies exist, screenshot checks when
reference data exists. Unsupported runners are reported, not silently “passed.”

**Acceptance:** stated compatibility and resource claims have measurements;
portable artifacts run on clean supported setups; safety/recovery checks pass;
release is not published without user authorization.

## 6. Test scenarios and release checklist

### Installation/update safety

- Fresh download → Setup → install → launcher → OOBE/desktop.
- Cancel/failure before receipt → no Start option/no false installed state.
- Repair installed artifact preserves settings/accounts/prefix/documents.
- HOME/XDG with spaces, apostrophes and non-ASCII characters.
- Readonly/full destination, invalid copied executable, missing dependency.
- Stage higher verified version; reject equal/older/insecure/wrong digest.
- Apply through installed launcher; self-check failure leaves/rolls back build.
- Uninstall removes owned launch integration, not documents/settings/Wine.

### Desktop/files/apps

- Start and nested flyout pointer travel; keyboard launch and Escape.
- Focus changes and modal background cannot activate another app.
- Save unsaved document; Cancel/Don't Save/Save; reopen after restart.
- Unicode file name; collision; readonly file; long copy cancellation.
- Recycle/restore conflict; corrupted metadata; interrupted operation.
- Download installer → explicit Run → installed app appears → uninstall.

### Browser

- Live HTTPS redirect; local deterministic form/link fixtures.
- Back/Forward/Stop/Refresh/Home through actual toolbar controls.
- Paste/non-ASCII input/scroll; page versus address-bar focus.
- Download success/collision/failure/cancel; byte integrity and destination.
- WebKit missing/crashed/stalled; offline fallback and worker restart.
- Multiple/minimized/closed browser windows; memory/worker cleanup.

### Visual/portability

- Desktop/Start/property pages/common dialogs/setup reference comparison.
- 800×600, 1024×768 and higher resolutions; supported scaling.
- X11, defined XWayland path, documented pure-Wayland limitation until resolved.
- Representative supported distributions and AppImage FUSE/extract modes.

## 7. Immediate execution queue

1. Finish Phase A shared-control/input/save reliability already in progress.
2. Add regression checks for keyboard focus, disabled actions and Cancel.
3. Exercise settings windows through the actual desktop; repair widget/focus
   bugs exposed by the checks, not merely compile errors.
4. Audit Explorer file-operation selection and long-operation safety.
5. Finish browser worker lifecycle, privacy/download dialogs and multi-window
   settings semantics.
6. Align setup/update behavior with installed artifact launch and add portable
   automatic-update scheduling where user systemd is unavailable.
7. Build the feature-status inventory, then complete missing high-priority
   Control Panel/editor workflows.
8. Establish reference screenshot comparison before promising pixel equivalence.
9. Integrate external Wine app windows and XP theming using a reversible,
   project-owned prefix strategy before broad compatibility claims.
10. Profile, package and run the release checklist on supported environments.

## 8. Progress accounting

The live task list tracks the current slice; this roadmap is intentionally much
larger than a single implementation batch. Mark a phase complete only when its
acceptance conditions hold. Preserve usable checkpoints and report actual
verification, blockers and remaining limits. “Keep working” does not authorize
publishing, destructive user-data changes or claiming unverified perfection.
