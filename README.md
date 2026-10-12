# WineXP

**Repo:** https://github.com/shitastical/WineXP

> ## ⚠️ THIS IS VIBE-CODED SLOP. DO NOT EXPECT IT TO WORK. ⚠️
>
> This repository is **100% vibe coded**. I typed prompts, an AI typed code, and I
> pasted the results in. That is the entire development methodology. There is no
> design document worth the name, no test suite to speak of, and no adult
> supervision.
>
> **This README was also AI-generated**, because I couldn't be bothered to write
> one. I asked DeepSeek to write it for me. So you're reading a robot's
> description of a robot's code. It's robots all the way down.
>
> **This project is completely broken and probably always will be.** It is a
> personal toy that I am making public for fun. It is not a product. It is not
> stable. It is not ready. It is a mess, and that mess is the point.

---

## What is this supposed to be?

A real, native, **Windows XP desktop recreated in Rust** (using `egui`/`eframe`),
with the long-term dream of deep **Wine** integration — hence the name.

The extremely ambitious, mostly-unrealised idea:

- A visually/mechanically faithful Windows XP desktop: boot sequence, logon,
  accounts, Control Panel, Explorer, `cmd.exe`, and the stock apps.
- It talks to **your actual Wine prefix** — `C:\` is your real
  `~/.wine/drive_c`, with full read/write access.
- Eventually: run actual Windows apps installed in Wine, alongside the ones
  that have been reimplemented in Rust.
- Eventually eventually: edit Wine configs, manage Wine directly, be an actual
  usable desktop replacement. Windows 7/11 themes someday, maybe. Probably not.

None of that is finished. Most of it doesn't exist. The parts that do exist are
mostly held together with comments that say "ported from X" and a prayer.

## Is it accurate?

Parts of it were ported from real sources (other XP recreations, live XP
simulator sites) rather than invented, and there's a `docs/PROVENANCE.md` that
tries to keep track. But "ported accurately" and "works" are two different
things, and this project is mostly the first one while failing at the second.

## Build

It's a Rust project. If you somehow want to run it:

```sh
cargo run
```

Godspeed. There's a `run.sh` too. It probably errors. That's normal here.

You'll want **Wine** installed if you want the `C:\` integration to point at
anything. Otherwise it will point at `~/.wine/drive_c` and shrug.

## Requirements

- Rust (recent-ish)
- Linux (this was developed on Linux; other platforms are untested fantasy)
- Wine, optionally, if you want the filesystem bits to do something
- A sound player — `pw-play`, `paplay`, `aplay`, `ffplay` or `mpv` — if you
  want the XP system sounds. Without one WineXP is simply silent.
- A high tolerance for things being broken

## System sounds

`src/sound.rs` plays the Windows XP sound scheme. It has **no audio
dependencies** on purpose: every pure-Rust audio crate links ALSA's development
headers, which are missing on a default desktop install, and that would break
the "just copy one binary" promise that `install.sh` makes. So the WAV is handed
to whatever player you already have.

The audio files live in `assets/sounds/` and are not in a fresh clone:

```sh
./scripts/fetch-sounds.sh
```

Without them WineXP builds and runs, just without sound. Turn the scheme off (or
audition individual sounds) from **Control Panel > Sounds and Audio Devices**.

**Licensing:** `assets/sounds/*.wav` are Microsoft's Windows XP sounds, obtained
from the public [Reborn XP](https://xp.quenq.com) distribution. They are **not**
covered by this project's licence below and are included for personal,
non-commercial use. Don't redistribute a build containing them without sorting
that out first — delete `assets/sounds/` and run `scripts/fetch-sounds.sh` on a
machine you're allowed to use them on.

## Memory safety

The one thing that wasn't vibe coded into a corner: the crate is
`#![forbid(unsafe_code)]`. So it may be slop, but at least it's *memory-safe*
slop. Small wins.

## Contributing

Please don't. Or do, whatever. I'm not your dad. But understand that this is a
personal project I'm dumping in public, not a maintained library. Issues saying
"this doesn't work" will be met with "yes, I know".

## License

I haven't decided. Assume "look at it, learn from it, don't sue me." If you
need something concrete before the repo goes further, that's a problem for
Future Me.

---

*Again: vibe coded, AI-written, work in progress, completely broken, probably
forever. You have been warned. Enjoy the slop.*
