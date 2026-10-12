# WineXP v0.01.01 pre-alpha

Second build. Same toy, more of it, and now it can update itself from here.

min-winexp: 0.1.0

## What is new since v0.01.0

- **Self-update, from this repository's releases.** `winxp --check-update` asks
  GitHub for releases, and only a release whose tag is a **higher** version than
  the installed one is ever offered. `winxp --stage-update` downloads it, checks
  its SHA-256 and that it is a real executable, stages it, and swaps it in on the
  next launch — after backing up the build being replaced and self-checking the
  new one. A build that cannot answer `--version` is rolled back automatically.
  `winxp --rollback` restores the previous build by hand.
- **Single-file AppImage.** `WineXP-0.1.1-x86_64.AppImage` is the whole
  application, which is also why updates are one file: the updater replaces the
  `.AppImage` the runtime points at with `$APPIMAGE`. `install.sh` puts it in
  `~/.local/share/winxp/` with a launcher and a desktop entry
  (`./install.sh --from-github` fetches and verifies the newest release).
- **Windows Update applet** (Start ▸ Windows Update): the XP-era Windows Update v5
  layout, wired to the real engine — it checks GitHub, lists a high-priority
  update with its size and notes, installs it after verification, shows the
  Reborn XP *Installing Windows Updates…* scene while it works, and reads back the
  audit log under *View installation history*.
- **Four more applets** — Calculator (Standard view), Character Map, Windows Task
  Manager (live `/proc` process list and performance graphs), and Date and Time
  Properties (real clock, calendar, time zones). Control Panel ▸ Date and Time
  now opens the applet; `taskmgr`, `calc` and `charmap` resolve from Run.

## Known limitations

- The updater refuses a release that publishes no SHA-256 digest. That is
  deliberate: nothing installs unverified.
- `Restart Now` in the applet applies the staged update, but the desktop in front
  of you keeps running the old build until you start it again.
- AppImages need FUSE, or `APPIMAGE_EXTRACT_AND_RUN=1`.
- Still no audio and no MP4 playback, so sounds are silent and the OOBE intro is a
  frame sequence.
- The reinstall flow resets only WineXP's own account/settings files — the real
  Wine prefix is never erased.

## Install

```sh
chmod +x WineXP-0.1.1-x86_64.AppImage
./WineXP-0.1.1-x86_64.AppImage            # just run it
./install.sh --from-github                # install for this user (from a checkout)
```
