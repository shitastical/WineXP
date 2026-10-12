# WineXP v0.01.02 pre-alpha

Six things you reported, actually fixed — and one test flake that had been
quietly failing a different test on nearly every run.

min-winexp: 0.1.0

## Fixed

- **The Recycle Bin actually works now.** Three separate problems were stacked:
  its icons were *HTML error pages saved with a `.png` name* (five byte-identical
  copies of a 404 page, which is why the icon was missing); it listed items by
  their internal storage id (`1789423123-report.txt`) instead of the name you
  recognise; and deleting something while you were already inside the bin called
  the *recycle* function, which refuses by design, so it could only ever report
  an error. It now shows real icons, real names, your original location in
  Properties, works **Restore**, permanent **Delete**, and **Empty Recycle Bin**.
  Recycle the same filename twice and you keep both, each independently
  restorable — restoring never overwrites a file that has since reappeared.
- **You can type into Google in Internet Explorer.** The WebKit helper was
  synthesising key events without ever setting the event's character payload, so
  WebKit received the right key with no text attached. Pages looked focused and
  silently swallowed everything you typed.
- **Start menu submenus stay open while you travel through them.** Hovering
  *All Programs* opened the list, but the list closed the instant your pointer
  left the row on its way into it — so *Accessories ▸ Accessibility* was
  unreachable. Rows now only switch the open submenu when the pointer is directly
  over them. While fixing this I found a second bug: a submenu that was open
  stayed open after you left the menu, so it reappeared the next time you opened
  it. Fixed too.
- **WineXP shows up in your Linux application menu.** If you had never run an
  install, there was no `.desktop` file, no launcher and no icon anywhere. There
  is a real per-user installer now (`winxp --install`, or `./install.sh`), it
  passes `desktop-file-validate`, and it works on any Linux distro with no root
  and no package manager. `winxp --uninstall` removes only what it created and
  leaves your settings, accounts and Wine prefix alone.
- **The menu entry uses the WineXP icon.** It was writing a 272×164 login logo
  into every icon size, so desktops had to guess. It now installs the project's
  own icon at 16/32/48/64/128/256/512.
- **Nothing auto-starts.** Booting used to open Internet Explorer, Minesweeper,
  Winamp and My Computer every time. You now land on an empty desktop.
- **No more stray "Google" window at the top left.** That was the WebKit helper's
  hidden GTK window: it was placed off-screen, and Mutter clamped it back to
  `(0,0)` as a separate managed window. It is now an unmanaged popup that stays
  hidden, and it does not even start until you open Internet Explorer.

## New

- **Windows XP system sounds.** Startup, logon, logoff and shutdown jingles,
  the recycle-bin sound, chimes on restore and delete, and an error sound when a
  file operation fails. **Control Panel ▸ Sounds and Audio Devices** is a new
  applet that switches the scheme off and on and plays any of the twelve sounds
  on demand.
  There are **no new dependencies** for this. Every pure-Rust audio library
  needs ALSA's development headers, which are missing on a default desktop and
  would have broken the "copy one binary, works anywhere" promise, so WineXP
  hands the sound to whatever player you already have installed (`pw-play`,
  `paplay`, `aplay`, `ffplay`, `mpv`). If you have none, WineXP is simply silent
  and says so in the applet.
- A dependency-free Recycle Bin icon generator
  (`scripts/make-recycle-icons.py`) and an audio fetcher
  (`scripts/fetch-sounds.sh`).

## Verification

66 tests pass (56 binary unit tests, 3 and 7 integration), the release build is
warning-free, and each fix above has a test that fails without it — including
three that drive real egui frames through the exact pointer path that used to
collapse the Start menu.

`tests/update.rs` had been flaky for a while: each test copies a ~35 MB binary
and runs it, and on nearly every run a *different* test failed, because a
concurrent copy was still open for writing when another test tried to execute its
copy (`ETXTBSY`). Now retried deliberately, plus a race on a shared temp-file
probe removed. Eight consecutive runs, all green.

Installed and verified on this machine: `winxp --install` exits 0, the desktop
entry validates, the launcher runs, the icon cache is refreshed, and the daily
verified-update timer is enabled.

## Not fixed / still rough

- **The Recycle Bin icons are drawn from scratch**, not the original shell32
  artwork, because Microsoft's icons are not redistributable. They read as a
  translucent XP-style bin at every size, but they are not pixel-identical.
- **The sound files are Microsoft's Windows XP sounds**, fetched from the public
  [Reborn XP](https://xp.quenq.com) distribution and included here for personal,
  non-commercial use. They are **not** covered by this project's licence. If you
  intend to redistribute a build containing them, delete `assets/sounds/`, run
  `scripts/fetch-sounds.sh` locally, and settle that first — WineXP builds and
  runs perfectly well without them, just silently.
- WebKitGTK 4.1 is still not installed on this machine, so Internet Explorer
  cannot actually be launched here to watch the typing fix in the real browser.
  The fix is in the event the helper sends; it needs a machine with
  `gir1.2-webkit2-4.1` to confirm end-to-end.
- The Start menu still draws some default egui controls rather than XP-shaped
  ones.
- Still `#![forbid(unsafe_code)]`, still vibe coded, still genuinely a toy.

## Upgrade

```sh
winxp --check-update     # finds v0.01.02
winxp --stage-update     # downloads, verifies, stages
                         # the swap happens on next launch
```

Or from scratch:

```sh
./install.sh --from-github
```

If a release will not update, tell me — that path has only ever been exercised
by tests, never by a real person on a real machine.