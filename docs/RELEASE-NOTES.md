# WineXP v0.01.0 pre-alpha

The first public build. Everything in it is a work in progress.

min-winexp: 0.1.0

## What it is

A Windows XP desktop written natively in Rust (`egui`/`eframe`), drawing on the
real Wine prefix on the host for `C:\`, installed programs and drives.

## In this build

- **Boot and setup** — Award BIOS POST, boot-device menu, the XP *starting*
  screen, the Reborn XP reinstall flow, the OOBE wizard (account creation), and
  logon, plus the *Installing Windows Updates…* scene.
- **Shell** — window manager, taskbar, Start menu, tray, balloons, modals,
  menus, desktop icons, Control Panel and its applets.
- **Applications** — Notepad, Paint, WordPad, Minesweeper, Solitaire, Pinball
  (SpaceCadet engine), cmd, Explorer, My Computer, Internet Explorer, Winamp,
  Volume Control, Wine Configuration, Display Properties, Add or Remove
  Programs, User Accounts and error boxes.
- **Self-update** — `winxp --check-update` consults this repository's releases;
  updates are verified by SHA-256, staged, backed up and rolled back
  automatically if the new build fails its self-check. See `docs/UPDATE.md`.
- **Packaging** — a single-file AppImage (`winxp-linux-x86_64`), which is also
  the artifact the updater installs.

## Known limitations

- AppImages need FUSE, or `APPIMAGE_EXTRACT_AND_RUN=1`.
- Audio and MP4 playback are not implemented, so the OOBE intro plays as still
  frames and sounds are silent.
- Some fonts fall back to the bundled Noto faces; the BIOS uses the ported
  `vga8x14` face.
- The reinstall flow resets only WineXP's own account/settings files — it never
  touches the real Wine prefix.

## Install

```sh
chmod +x WineXP-0.1.0-x86_64.AppImage     # or the tarball's install.sh
./install.sh --from-github                # from a checkout
```
