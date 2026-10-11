# Updates

WineXP updates itself from **GitHub releases** of this repository. The whole
engine lives in [`src/update.rs`](../src/update.rs) and is enforced in code, not
by convention: every rule below has a test (`cargo test`).

Installed version: `v0.01.0 pre-alpha` (`DISPLAY_VERSION`), which is Cargo's
`0.1.0`. Comparisons always use the semver, never the label.

## Checking and applying

```sh
winxp --version          # the installed version ("v0.01.0 pre-alpha")
winxp --check-update     # consult GitHub releases; "up to date" or "update available"
winxp --github-latest    # what the newest release offers (tag, asset, size, digest)
winxp --stage-update     # download + verify the next release, apply on next launch
winxp --apply-update     # apply a staged update now (also happens at every launch)
winxp --rollback         # restore the newest backup from updates/backups/
winxp --update-status    # machine-readable state (installed, pending, source…)
winxp --update-log       # the audit trail of every check/stage/apply/refusal
```

An update is applied **before the window opens** on the next launch, so a broken
build can never leave the desktop half-updated.

## How a release is discovered

1. `GET https://api.github.com/repos/<repo>/releases` (newest first).
2. Drafts are ignored — an unpublished release must never install.
3. Each remaining release's tag is parsed as a version; the **highest** one wins.
4. Its asset named exactly `winxp-linux-x86_64` is the artifact. An asset whose
   name starts with `winxp` and is not a `.sha256`/`.json`/`.sig` will do at a
   pinch.
5. The artifact's SHA-256 must be known: GitHub's `digest` field
   (`sha256:<hex>`) or a published `winxp-linux-x86_64.sha256` sibling.
   **A release with no digest is refused, not installed unverified.**
6. Only if that version is **strictly higher** than the installed one is it
   offered. Equal or lower is refused — that is the anti-downgrade rail, and it
   is what stops a re-published older release from eating an upgrade.

The default source is `github:shitastical/WineXP`. Set another with
`WINXP_UPDATE_REPO=owner/repo`, `WINXP_UPDATE_SOURCE=<source>`, or the
`update_source` field in `settings.json`. A source can also be a manifest URL
(`https://…/manifest.json` or `file:///…`) in the bundled
[`manifest.json`](../docs/manifest.example.json) format.

## Safety rails

| # | Rail | Failure mode it prevents |
|---|---|---|
| 1 | Never install a version that is not strictly newer (checked again at apply time) | Downgrades, replays of an older release, equal-version swaps |
| 2 | `min-winexp:` in the release notes gates the minimum base build | New formats landing on old binaries |
| 3 | HTTPS only (`curl --proto '=https'`); insecure schemes refused | MITM'd manifests and artifacts |
| 4 | Size **and** SHA-256 re-verified immediately before the swap | Truncated/corrupted/swapped downloads |
| 5 | The artifact must look executable (ELF header or `#!`) | An HTML error page from a proxy/captive portal being installed as the app |
| 6 | Staged apply: download first, swap on the **next** launch | A half-replaced running binary |
| 7 | The build being replaced is backed up to `updates/backups/` first | Unrecoverable bad updates |
| 8 | Self-check: the staged build must answer `--version` with the promised version — before and after the swap, and it is killed if it hangs | A build that cannot even start |
| 9 | Automatic rollback when the post-swap self-check fails | A desktop that no longer opens |
| 10 | One pending update at a time | Two downloads racing on one file |
| 11 | Writable-directory and free-space probes (artifact + 32 MiB slack) | Updates into read-only or full filesystems |
| 12 | `settings.json` schema guard: a newer schema is never overwritten | A downgrade eating a newer install's state |
| 13 | Every check, stage, apply, refusal and rollback is appended to `update-log.json` | Silent state changes |

Refusals are never fatal: they print a reason and leave the installed build
untouched.

## AppImage

WineXP is a **single-file AppImage**. An AppImage mounts itself read-only under
`/tmp`, so the running executable is not the file a user has — therefore
`current_exe()` prefers `$APPIMAGE`, and the updater replaces the `.AppImage`
itself. Two consequences:

* Keep the AppImage somewhere writable (the installer uses
  `~/.local/share/winxp/WineXP.AppImage`); a system-wide `/opt` install needs
  root to update.
* The pre-swap self-check runs AppImages with `APPIMAGE_EXTRACT_AND_RUN=1`, so a
  machine without working FUSE can still verify an update.

Outside an AppImage (`cargo run`, the plain binary) the updater replaces
`target/…/winxp` instead. Same rails either way.

## Publishing a release

```sh
scripts/package.sh          # builds dist/ artifacts and verifies the AppImage runs
gh release create v0.1.0 \
  dist/winxp-linux-x86_64 \
  dist/winxp-linux-x86_64.sha256 \
  dist/WineXP-0.1.0-linux-x86_64.tar.gz \
  --title "WineXP v0.01.0 pre-alpha" --notes-file docs/RELEASE-NOTES.md
```

* The tag must be a version (`v0.1.0`, `v0.01.0`, `v1.0.0`). Never re-use a tag
  for different bytes: the tag is the version, and a lower one will not install.
* The asset **must** be named `winxp-linux-x86_64`.
* GitHub reports each asset's SHA-256 automatically; `scripts/package.sh` also
  writes a `.sha256` file to publish alongside it.
* Release notes may contain `min-winexp: 0.2.0` to require a minimum base build.

## Environment

| Variable | Effect |
|---|---|
| `WINXP_UPDATE_SOURCE` | Overrides the update source (`github:owner/repo`, `https://…`, `file://…`) |
| `WINXP_UPDATE_REPO` | Overrides just the GitHub repository (`owner/repo`) |
| `WINXP_UPDATE_DIR` | Where staging, backups and the log live (default `~/.local/share/winxp/updates`) |
| `WINXP_GITHUB_TOKEN` (or `GITHUB_TOKEN`) | Lifts the anonymous GitHub rate limit (60 requests/hour) |
| `APPIMAGE` | Set by the AppImage runtime; the file the updater replaces |

## Limitations

* GitHub's 60 requests/hour anonymous limit applies to `--check-update`; set a
  token for frequent checks. Checks are manual (`--check-update`) or
  explicit from the Windows Update applet — nothing polls in the background.
* Releases published without a digest and without a `.sha256` asset are refused.
  That is deliberate, and it means an update cannot be installed from a release
  that was not built by `scripts/package.sh`.
* `--rollback` restores from `updates/backups/`, which only contains builds this
  install has replaced; it cannot reach a version it never ran.
* Only Linux x86-64 artifacts are published today (`winxp-linux-x86_64`).
