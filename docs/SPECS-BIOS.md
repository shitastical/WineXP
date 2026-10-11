# Reborn XP power-on / BIOS (POST → boot menu → XP boot → logon) — porting spec

Implementation-ready transcription of the **Reborn XP** (`xp.quenq.com`) power-on / BIOS sequence, so it
can be reimplemented in Rust and replace this project's current BIOS POST (`src/screens/post.rs` +
`src/screens/bootmenu.rs`, which today transcribe a *different* machine — an Award v6.00PG / ASUS board
from `win32.run`, with a `boot_manager.svelte`-style multi-device menu).

Every string, colour, font and timing below was copied from a **fetched primary source** (the live site's
own HTML / CSS / JS). Nothing is invented. Where a value could not be confirmed it is marked
`✗ not retrieved` rather than guessed.

Companion docs: `docs/SPECS-SYSTEM.md`, `docs/PROVENANCE.md`, `docs/FEATURE-GAPS.md`.

---

## 0. Sources actually fetched

| Source | URL | What it gave | HTTP |
|---|---|---|---|
| Landing page (rendered HTML) | `https://xp.quenq.com/` | The **entire BIOS/boot scene markup** (`#post-screen`, `#bootloader`, `#reinstall-confirm`), the scene list inside `<scene_holder>`, the `#main-boot-screen` element | 200 |
| Landing page (text render) | `https://xp.quenq.com/` | Confirms the rendered order and the string `Quenq Sys BIOS v. ` (version span empty at fetch time) | 200 |
| Boot/POST driver | `https://xp.quenq.com/js/index.js` | The POST line array, the 100 ms line cadence, the 1500 ms hold, the **whole bootloader state machine** (countdown, keys, wipe) and the `n()` transition into boot | 200 |
| Main shell | `https://xp.quenq.com/js/shell.js` | `showBootScreen()` (XP boot GIF), `dismissBoot()` (`bootDelay`), `showBSOD()`, `runOobe()`, scene order | 200 |
| Scenes stylesheet | `https://xp.quenq.com/css/scenes.css` | Black body/`scene_holder`, `scene_bootscreen` rules, the **`vga8x14` font face** | 200 |
| Boot stylesheet | `https://xp.quenq.com/css/index.css` | Every `.boot-screen` / `#post-screen` / `#bootloader` / `#reinstall-confirm` / `.epa-logo` rule | 200 |
| Product page | `https://quenq.com/apps/reborn-xp/` | Context only (feature list); **contains no BIOS markup** | 200 |

> Note: `css/scenes.css` **did not 404** — it served real CSS (it is the scene/shell stylesheet). The BIOS
> screens' own rules live in **`css/index.css`**, not in `scenes.css`. Both are transcribed below.

Scene markup lives **statically in the landing HTML** (not injected by JS). `js/index.js` only animates it
(typewriter + timings + keyboard) and ends by calling the shell's boot routine.

---

## 1. Scene order (exact)

```
POST screen   (#post-screen,   .boot-screen)                     black, typewriter
      │  ~2.5 s (100 ms/line ×10 + 1500 ms hold)
      ▼
Boot menu     (#bootloader,    .boot-screen)                     black, ↑/↓/ENTER/ESC + 10→0 countdown
      ├─ "Reinstall Windows XP" ─▶ Reinstall confirm (#reinstall-confirm, .boot-screen)  navy
      │        ├─ "No, return to main menu" ─▶ back to #bootloader
      │        └─ "Yes, continue Setup"  ─▶ "Formatting virtual disks..." (2 s)
      │                                     ─▶ "Restarting computer..."  (1.5 s) ─▶ location.reload()
      └─ "Start Windows XP" (default / countdown end / ENTER)
      ▼
XP boot       (scene_bootscreen#main-boot-screen)                res/anim/boot.gif, `bootDelay` ms (default 4000)
      ▼
[optional]    (scene_updatescreen_xp)  only if app version changed AND OOBE already passed
      ▼
OOBE          (msoobe app)  if `oobepassed` is unset
   or
Logon         (scene_logon) when `oobepassed` is set            grid; users listed
      ▼
Desktop       (scene_desktop)
```

`ESC` on the boot menu (main mode) calls `location.reload()` — i.e. it restarts the whole sequence from the
POST screen. `scene_bsod` and `scene_standby` exist as sibling scenes but are **not** part of the normal
power-on path.

---

## 2. Scene 1 — POST screen (`#post-screen`)

### 2.1 Markup (verbatim from `https://xp.quenq.com/`)

```html
<div id="post-screen" class="boot-screen"><header><pre><span class="epa-logo"></span>
Award Modular BIOS v4.51PG, An Energy Star Ally
Copyright (C) 1984-2026, Project Quenq.

Quenq Sys BIOS v.<span id="current-app-version"></span>
            </pre></header><main><pre id="post-text"></pre></main><footer><pre>Press &lt;DEL&gt; to enter SETUP, &lt;F12&gt; to Boot Menu</pre></footer></div>
```

Structure: `<header>` (BIOS banner) / `<main>` (`#post-text`, where the typewriter lines appear) /
`<footer>` (`<DEL>`/`<F12>` hint pinned to the bottom by `main { flex-grow:1 }`).

### 2.2 The header block — exact lines, in order

The `epa-logo` span is `float:right` (a 136×84 px EPA "Energy Star" logo, §5.4), then:

```
Award Modular BIOS v4.51PG, An Energy Star Ally
Copyright (C) 1984-2026, Project Quenq.

Quenq Sys BIOS v.<version>
```

* Line 1: `Award Modular BIOS v4.51PG, An Energy Star Ally`
* Line 2: `Copyright (C) 1984-2026, Project Quenq.`
* Line 3: *(blank)*
* Line 4: `Quenq Sys BIOS v.` immediately followed by the contents of `#current-app-version`.

`#current-app-version` is filled at runtime by `js/index.js`:

```js
window.CURRENT_APP_VERSION="3.2.2";
const n=document.getElementById("current-app-version");
n&&window.CURRENT_APP_VERSION&&(n.textContent=window.CURRENT_APP_VERSION);
```

At fetch time the live value is **`3.2.2`**, so line 4 renders as `Quenq Sys BIOS v.3.2.2`. (The version is
also the cache-buster on every asset `?v=3.2.2`.) In Rust this must be a runtime value, not a literal.

### 2.3 The typewriter body — exact lines, in order

Appended to `#post-text` one at a time (from `js/index.js`):

```js
const i=[
  "Main Processor : Intel® Pentium® 4 @ 2.80GHz",
  "Memory Testing :  2097152K OK",
  "",
  "Detecting Pri Master ... Virtual Hard Disk C:",
  "Detecting Pri Slave  ... Virtual Hard Disk D:",
  "Detecting Sec Master ... Virtual CD/DVD E:",
  "Detecting Sec Slave  ... None",
  "",
  "Initializing USB Controllers ... Done",
  ""
];
```

Rendered (note the **two spaces** after the colon in line 2, and after `Slave` in line 5):

```
Main Processor : Intel® Pentium® 4 @ 2.80GHz
Memory Testing :  2097152K OK

Detecting Pri Master ... Virtual Hard Disk C:
Detecting Pri Slave  ... Virtual Hard Disk D:
Detecting Sec Master ... Virtual CD/DVD E:
Detecting Sec Slave  ... None

Initializing USB Controllers ... Done

```

(10 array entries; `®` = U+00AE; the last two entries produce the trailing blank line.)

### 2.4 The footer — exact line

```
Press <DEL> to enter SETUP, <F12> to Boot Menu
```

(Source HTML uses `&lt;DEL&gt;` / `&lt;F12&gt;`; the visible glyphs are the literal `<DEL>` and `<F12>`.)

### 2.5 Timing / behaviour

From `js/index.js` (the `e()` function):

```js
let r=0;
e.style.display="flex";                       // show #post-screen
const c=()=>{
  r<i.length
    ? (t.innerHTML += i[r]+"\n", r++, setTimeout(c,100))   // 100 ms between every line
    : setTimeout(()=>{ e.style.display="none"; /* bootloader IIFE */ },1500);  // 1500 ms hold
};
c();
```

* **Line cadence: 100 ms** per line (append `line + "\n"`).
* Lines appear at t = 0, 100, 200 … 900 ms.
* After the last line, the screen is held for a further **1500 ms**, then `#post-screen` is hidden
  (`display:none`) and the boot menu is shown. **Total POST-visible time ≈ 2500 ms.**
* There is **no** per-line "detecting … done" pause and **no** memory-count-up animation — it is a plain
  100 ms typewriter.
* The POST screen does **not** read the keyboard (the only handler is on the boot menu). `DEL`/`F12` in the
  hint text are **decorative only** — pressing them does nothing.

### 2.6 CSS (verbatim, `css/index.css`)

```css
.boot-screen{position:fixed;top:0;left:0;width:100%;height:100%;color:#ccc;z-index:99999999;font-family:vga8x14,monospace;font-smooth:never;-webkit-font-smoothing:none;padding:80px 60px;box-sizing:border-box;font-size:20px;line-height:1.6;display:none}
#post-screen{display:none;flex-direction:column;cursor:none}
#post-screen pre{font-family:vga8x14;white-space:pre-wrap;font-size:22px;line-height:1.2}
#post-screen main{flex-grow:1}
```

* Background: `#post-screen` sets **no** background — it is transparent over the **black** body
  (`body{background:#000}` in `scenes.css`) and `scene_holder{background:#000}`. Result: **black screen**.
* Text colour: **`#ccc`** (`rgb(204,204,204)`).
* Font: **`vga8x14`** (monospace fallback). `#post-screen pre` is **22 px / line-height 1.2**; the shared
  `.boot-screen` base is 20 px / 1.6.
* Padding: **80 px top/bottom, 60 px left/right**. Cursor hidden (`cursor:none`). `white-space:pre-wrap`.

---

## 3. Scene 2 — Boot-device menu (`#bootloader`)

### 3.1 Markup (verbatim)

```html
<div id="bootloader" class="boot-screen"><p>Use the ↑ and ↓ keys to move the selection.</p><p>Press ENTER to boot the selected OS, or ESC to restart.</p><p class="title">Please select an option:</p><ul id="boot-options-list"><li data-action="start-xp">Start Windows XP</li><li data-action="install-xp">Reinstall Windows XP</li></ul><p id="countdown-text" class="title">Booting in <span id="timer-seconds">10</span>...</p></div>
```

### 3.2 Exact text, in order

```
Use the ↑ and ↓ keys to move the selection.
Press ENTER to boot the selected OS, or ESC to restart.
PLEASE SELECT AN OPTION:            ← "Please select an option:" (rendered UPPERCASE via CSS)
    Start Windows XP                ← <li data-action="start-xp">  (selected by default)
    Reinstall Windows XP            ← <li data-action="install-xp">
BOOTING IN 10...                    ← "Booting in 10..." (rendered UPPERCASE; 10 counts down)
```

* The `↑`/`↓` are literal U+2191 / U+2193 in the source.
* The two heading lines carry `class="title"` → CSS `text-transform:uppercase` (§5.3), so
  `Please select an option:` **displays** as `PLEASE SELECT AN OPTION:` and `Booting in 10...` as
  `BOOTING IN 10...`. The underlying text is sentence-case.
* `#timer-seconds` starts at the literal `10`.
* List items are **not** numbered/prefixed in the DOM — the indent comes from `ul { margin-left:40px }`.

### 3.3 CSS (verbatim)

```css
#bootloader{background-color:#000}
.boot-screen p{margin:0 0 1em}
.boot-screen .title{text-transform:uppercase;margin-top:1.5em;margin-bottom:.5em}
.boot-screen ul{list-style:none;padding:0;margin:0 0 0 40px}
.boot-screen li{padding:4px 8px;cursor:url('../res/ui/cursors/link.cur'),pointer;display:block;width:fit-content}
.boot-screen li.selected{background-color:#ccc;color:#000}
```

Background **black** (`#000`), text **`#ccc`**, same `vga8x14` font, `.boot-screen` base **20 px / 1.6**,
padding 80/60. The selected row is shown in **inverse video**: background `#ccc`, text `#000`. Items are
clickable (`cursor: link.cur`).

### 3.4 The countdown & keyboard — state machine (verbatim from `js/index.js`)

De-minified, the bootloader IIFE is:

```js
let busy=false, mode="main", timer=null, seconds=10, sel=0;
const bl   = document.getElementById("bootloader");
const rc   = document.getElementById("reinstall-confirm");
const main = bl.querySelectorAll("#boot-options-list li");
const conf = rc.querySelectorAll("#confirm-options-list li");
const cnt  = bl.querySelector("#countdown-text");
const secs = bl.querySelector("#timer-seconds");

function paint(items){ items.forEach((el,i)=>el.classList.toggle("selected", i===sel)); }

function stopCountdown(){ timer && clearInterval(timer); cnt && (cnt.style.display="none"); }

async function run(){                        // execute the highlighted action
  if (busy) return;
  busy = true;
  stopCountdown();
  window.removeEventListener("keydown", onKey);
  const action = (mode==="main" ? main[sel] : conf[sel]).dataset.action;
  if      (action==="install-xp")   setMode("confirm");
  else if (action==="confirm-wipe") await wipeAll();
  else if (action==="cancel-wipe")  setMode("main");
  else                              proceedToBoot();   // start-xp
}

function setMode(m){                          // "main" | "confirm"
  mode = m; busy = false; sel = 0;
  if (m==="main"){
    bl.style.display = "block";
    rc.style.display = "none";
    paint(main);
    cnt && (cnt.style.display="block");
    seconds = 10;
    timer && clearInterval(timer);
    secs.textContent = seconds;
    timer = setInterval(()=>{ seconds--; secs.textContent = seconds; if (seconds<=0) run(); }, 1000);
  } else {                                    // "confirm"
    bl.style.display = "none";
    rc.style.display = "block";
    paint(conf);
    document.body.focus();
  }
}

function onKey(e){
  if (busy) return;
  if (mode==="main") stopCountdown();          // <-- ANY key hides/stops the countdown
  const items = mode==="main" ? main : conf;
  switch (e.key){
    case "ArrowUp":   e.preventDefault(); sel=(sel-1+items.length)%items.length; paint(items); break;
    case "ArrowDown": e.preventDefault(); sel=(sel+1)%items.length;             paint(items); break;
    case "Enter":     e.preventDefault(); run(); break;
    case "Escape":    e.preventDefault(); mode==="confirm" ? setMode("main") : location.reload(); break;
  }
}

main.forEach((el,i)=>el.addEventListener("click",()=>{ sel=i; paint(main); run(); }));
conf.forEach((el,i)=>el.addEventListener("click",()=>{ sel=i; paint(conf); run(); }));
window.addEventListener("keydown", onKey, {capture:true});
setMode("main");                               // initial state
```

Key facts to reproduce:

* **Default selection = index 0 = `Start Windows XP`** (`setMode("main")` at startup, `sel=0`).
* **Countdown**: starts at **10**, decremented **once per 1000 ms**; `#timer-seconds` updated each tick; when
  it reaches **≤ 0** the **highlighted** option is executed (`run()`). With no interaction that is
  `Start Windows XP`.
* **Any keypress in `main` mode immediately stops and hides the countdown** (`stopCountdown()` runs before
  the switch). So the 10→0 auto-boot only happens if the user does not press a key. Returning to main via
  `No, return to main menu` **restarts** the countdown at 10 and re-shows `#countdown-text`.
* **↑ / ↓** move the highlight, **wrapping around** (modulo list length). **ENTER** executes the highlighted
  option (removes the global keydown listener; one-shot). **ESC** → in `confirm` returns to `main`; in
  `main` calls `location.reload()` (full restart → POST again).
* Mouse clicks on an `<li>` select and immediately execute it.
* The keydown listener is registered with `{capture:true}` on `window`.

---

## 4. Scene 3 — Reinstall confirm (`#reinstall-confirm`)

### 4.1 Markup (verbatim)

```html
<div id="reinstall-confirm" class="boot-screen"><p>Setup is about to erase all contents of your virtual hard drives (C: and E:) and all saved user settings. This action cannot be undone.</p><p>Are you sure you want to continue?</p><p class="title">Select an option:</p><ul id="confirm-options-list"><li data-action="confirm-wipe">Yes, continue Setup</li><li data-action="cancel-wipe">No, return to main menu</li></ul></div>
```

### 4.2 Exact text

```
Setup is about to erase all contents of your virtual hard drives (C: and E:) and all saved user settings. This action cannot be undone.
Are you sure you want to continue?
SELECT AN OPTION:                   ← "Select an option:" (UPPERCASE via CSS)
    Yes, continue Setup             ← <li data-action="confirm-wipe">
    No, return to main menu         ← <li data-action="cancel-wipe">
```

Shown only when `install-xp` is chosen on the boot menu. Uses the same `<li class="selected">` inverse-video
highlight as the main menu (`Yes, continue Setup` is selected first). No countdown here.

### 4.3 CSS (verbatim)

```css
#reinstall-confirm{background-color:navy}
```

Background **navy** = CSS `navy` = **`#000080`**. Same `#ccc` text, `vga8x14` font, 20 px / 1.6, 80/60
padding. (This is the one boot screen with a non-black background.)

---

## 5. Colours, fonts, geometry (summary)

### 5.1 Backgrounds / foregrounds

| Scene | Background | Text |
|---|---|---|
| `#post-screen` | black (transparent over `#000` body) | `#ccc` |
| `#bootloader` | **`#000`** | `#ccc` |
| `#reinstall-confirm` | **`navy` (`#000080`)** | `#ccc` |
| selected `<li>` | `#ccc` | `#000` |
| wipe `div.boot-message` | `#000` (inline) | `#fff`, **24 px** (`.boot-message`) |

### 5.2 Fonts

* Boot screens use the custom pixel font **`vga8x14`**, defined in `css/scenes.css`:
  ```css
  @font-face{font-family:vga8x14;src:url('../res/fonts/Web437_IBM_VGA_8x14.woff')}
  ```
  So the file is **`res/fonts/Web437_IBM_VGA_8x14.woff`** (fallback `monospace`). Antialiasing is disabled
  (`font-smooth:never; -webkit-font-smoothing:none`).
* Sizes: `.boot-screen` base **20 px / line-height 1.6**; `#post-screen pre` overrides to **22 px / 1.2**;
  wipe message **24 px**.

### 5.3 Shared `.boot-screen` base (verbatim)

```css
.boot-screen{position:fixed;top:0;left:0;width:100%;height:100%;color:#ccc;z-index:99999999;font-family:vga8x14,monospace;font-smooth:never;-webkit-font-smoothing:none;padding:80px 60px;box-sizing:border-box;font-size:20px;line-height:1.6;display:none}
.boot-screen p{margin:0 0 1em}
.boot-screen .title{text-transform:uppercase;margin-top:1.5em;margin-bottom:.5em}
.boot-screen ul{list-style:none;padding:0;margin:0 0 0 40px}
.boot-screen li{padding:4px 8px;cursor:url('../res/ui/cursors/link.cur'),pointer;display:block;width:fit-content}
.boot-screen li.selected{background-color:#ccc;color:#000}
.boot-screen .boot-message{display:flex;justify-content:center;align-items:center;width:100%;height:100%;font-size:24px;color:#fff}
```

### 5.4 The EPA logo (`.epa-logo`, verbatim)

```css
.epa-logo{image-rendering:pixelated;display:block;float:right;width:136px;height:84px;background-image:url("data:image/gif;base64,R0lGODlhiABUALMAAAEBAQH/Af//AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAACH5BAEAAAAALAAAAACIAFQAAwT/EMhJq704ayC6/6C3jWRpnmgZrmybvnAsS62Q1nOuz+tO9b6g8AcaYkLGZKyo1DCb0OMnqppSo88rKav1cbtbKzgnHqPK5rMovVyzq7Y3ryPfoOuvO96NJ9P7HHyAc39ygoM6h2CKiH5xZoyNjpCFkkORSZiWk1eam5xQnp+gmZWjiaaiMKqncKmmO6ytYYeyVbNBdLCBQru4bYGPRF6+vzePu7YZxcYnborMJtHNtxOM03bY1NkVvtrLwtt5r+HW5dLn4s7lkd/d6erV7+Dj8W3nytz29VK58PtOiuXrt6+GQXAGWRC0l9BFwIZILLgbBTEiworJJgK8dO3fRjUe/98JDPlxhJ6L9KiRNGfS1kkaK6VolPFlocR8L2Gu+jOTXy2RFwYGCzmzUs+bOoclNSes1tGhLflZ8wKzKTKr5IjElElS4yupOsUUGpvl2dOqKyfC2ooWaxyeLKvyYmpDqNaY7tZa1ef2blizLJkMzIlUXty4epfeQSLoqlymO7d+S6z051xea4F2S9qY7Tx00gpDRqPLcUa6N3lmvQc6zGfIHC6jRZwOiMTZmiO3NqlV5JrVl4nWDUgjNc3JXR37Jos62OF+I5c+37nbyePKbhU6m7qQHbOcdlOfdu6X+zrpSmVfX96RLb6awT2HLn4kPWz2elweDGpUvmv1z3knYP9/Trln0Xu3GTGWTXqFcxJpZ9kXnhromUdfbvC1dR8qhClooYQI4tYSc734N0eFF6bYW4bi1RVhH6Ux+B6LD3U4y0sOcvWURfFgsleLsdhY0Fu1vZhbSfwZieSSTDbpZBcBRCnllFRWaeWVWGap5ZZcdukll0+GKeaYAFEJwJQUBHABmhJI2WYFaqY5gZpmunlmlG/mOaedbNY5551uxnkmoHjWaaeciA6qZ6J0/inooopG+micgtJJKZyOZspopG1equejkmK6KaAWVBoqqKj+uaing+LJ6amaqtoqnKyaOuqerrI5K665xgqprZzWOiuqfBbr6q597iqpsaC+SuxEob/66qysk36q6LHRAqtpqndSi2mz094a7ZvceruqtZSWmyer4l4LLLfN+tluvLmiaSap1NYrbKLo6lqpveYS2mieEQAAOw==");background-size:contain;background-position:center;background-repeat:no-repeat}
```

The logo is an inline **base64 GIF, 136×84 px**, floated to the top-right of the POST header. The full data
URI is above; the raw asset is **not** a separate file (it is embedded in `css/index.css`). To port it,
decode the base64 to `assets/` (or redraw) — it is the green/black EPA "Energy Star" mark.

### 5.5 Screen size

* The boot screens are **`position:fixed; width:100%; height:100%`** — they fill the viewport; there is **no
  fixed pixel canvas or resolution**. The whole simulation is scaled by `body { zoom }`:
  `js/index.js` sets `zoom` to **100 %** (`innerWidth ≥ 950`), **85 %** (`750–949`), or **70 %** (`< 750`),
  remembered in `localStorage.userPreferredZoom`.
* `<scene_holder screenmode="0">` selects the colour-depth filter (`setColorDepth`, applied *after* boot);
  default `0` = no filter.
* In **portrait** orientation `#portrait-overlay` (background **navy**, white `vga8x14` text) covers the
  screen with "Please rotate your device to landscape mode for the best experience." and hides
  `scene_holder` — i.e. the whole BIOS sequence is suppressed on a portrait phone.

> **Not retrieved:** no design-resolution constant, no DPI assumption, and no `1:1` pixel grid was present
> in the fetched CSS. The screens are simply full-bleed and font-sized in CSS `px`.

---

## 6. Transition into the XP boot scene

When the boot menu resolves to `start-xp` (or the countdown expires), `js/index.js` runs `proceedToBoot()`:

```js
document.getElementById("post-screen").style.display="none";
document.getElementById("bootloader").style.display="none";
document.getElementById("reinstall-confirm").style.display="none";
document.getElementById("seo-content").style.display="none";
await window.dm.ready();
window.shell.showBootScreen();
window.APP_VERSION = window.CURRENT_APP_VERSION;
window.shell.setup().then(()=>{ window.wm.setup(); window.apps.setup(); })
  .then(()=>{ window.initializeScreensaver && window.initializeScreensaver(); shell.dismissBoot(); });
```

`Shell.showBootScreen()` (from `js/shell.js`) fills the empty `<scene_bootscreen id="main-boot-screen">`
with the animated XP boot logo:

* Default boot skin: `{ id:"default-xp-pro", path:"res/anim/boot.gif", style:"contain", bgColor:"#000000", type:"gif" }`
  → an **`<img src="res/anim/boot.gif">`**, background `#000`, `object-fit:contain`, centred; fades in with
  `opacity transition 0.5s ease-in` (set to `0` then `1` after 50 ms). If `type:"video"` it is a looping
  muted `<video>` instead. `localStorage.activeBootSkinConfig` can override path/style/bg/type.

`Shell.dismissBoot()` then times the hand-off:

```js
const delay = parseInt(localStorage.getItem("bootDelay") || "4000");   // default 4000 ms
setTimeout(() => {
  image.style.opacity = "0";                 // fade the boot logo out
  setTimeout(() => {
    /* show progress cursor overlay */
    setTimeout(() => {
      this._boot.style.display = "none";
      this.setColorDepth(localStorage.getItem("colorDepth"));
      await this.runOobe();                  // → OOBE wizard or logon screen
      if (localStorage.getItem("oobepassed")) this._logon.style.display = "grid";
    }, 1000);
  }, 500);
}, delay);
```

* **XP boot logo visible for `bootDelay` ms — default `4000`.** `localStorage.bootDelay` overrides; **dev
  mode** (`localStorage.devMode === "true"`) forces it to `0` and enables fast logon.
* Then: logo fades (0.5 s transition) → 500 ms → progress cursor → 1000 ms → boot scene hidden and the
  logon/OOBE scene is chosen.
* `runOobe()`: if `oobepassed` is unset → launch the **`msoobe`** OOBE wizard; if set and there is exactly
  one password-less user → auto-logon; otherwise → `scene_logon` (`display:grid`).
* `scene_logon` (from `scenes.css`) is the blue `#5a7edc` Luna logon: top/bottom `#00309c`/gradient bands,
  `leftcontain` logo + `To begin, click your user name` / italic `welcome` (`#welcome`, 48 px), user tiles on
  the right, footer `After you log on, you can add and change accounts.` + red `Turn off computer` button.

---

## 7. Keyboard / mouse reference (all scenes)

| Key / action | `#bootloader` (main) | `#reinstall-confirm` |
|---|---|---|
| — (idle) | Countdown 10→0, auto-runs highlighted item | — |
| ↑ / ↓ | Move highlight (wraps) | Move highlight (wraps) |
| ENTER | Execute highlighted item | Execute highlighted item |
| ESC | `location.reload()` (restart entire boot) | Return to `#bootloader` (countdown restarts at 10) |
| Any key | Stops & hides the countdown | — |
| Click `<li>` | Select + execute | Select + execute |
| DEL / F12 | (hint text only — **no handler**) | (n/a) |

---

## 8. The wipe sub-sequence (`Yes, continue Setup`)

Verbatim from `js/index.js`:

```js
bl.style.display="none"; rc.style.display="none";
const n=document.createElement("div");
n.className="boot-message";
n.style.cssText="font-family: vga8x14; font-size:22px; position:fixed; top:0; left:0; width:100%; height:100%; z-index:999999999; background-color:#000; color:#fff; display:flex; align-items:center; justify-content:center;";
document.body.appendChild(n);
n.textContent="Formatting virtual disks...";
localStorage.clear();
await Promise.all([
  new Promise(r=>window.indexedDB.deleteDatabase("rebornxp_fs_C").onsuccess=r),
  new Promise(r=>window.indexedDB.deleteDatabase("rebornxp_fs_E").onsuccess=r)
]);
await new Promise(r=>setTimeout(r,2000));
n.textContent="Restarting computer...";
await new Promise(r=>setTimeout(r,1500));
window.location.reload();
```

* Clears **`localStorage`** and deletes the IndexedDB virtual drives **`rebornxp_fs_C`** and
  **`rebornxp_fs_E`** (the `D:` drive is not touched — it is the persistent-storage drive).
* Centred full-screen black message: `Formatting virtual disks...` (**2000 ms**), then
  `Restarting computer...` (**1500 ms**), then `location.reload()`.
* Note the inline style sets `font-size:22px` while the `.boot-message` class would set 24 px — inline wins
  here, so the wipe text is **22 px** white on black.

---

## 9. BSOD (for completeness — `showBSOD`)

Not on the normal power-on path, but part of `js/shell.js` and referenced by the task. `showBSOD({file, code, message})`
hides all desktop/logon/boot/overlay scenes, fills `scene_bsod > pre` from this template, then shows the
scene; a single `pointerdown` or `keydown` calls `reboot()`.

```css
/* scenes.css */
scene_bsod{background:#000082;display:none;color:#fff;font-size:2.5vh;position:absolute;width:100%;height:100%;overflow:hidden;cursor:none}
scene_bsod pre{margin:30px;font-family:vga8x14;cursor:none}
```

Template (the `FILE_PLACEHOLDER` / `MESSAGE_PLACEHOLDER` / `CODE_PLACEHOLDER` tokens are substituted; the
static default shown in the HTML is the `UXTHEME.DLL` / `ILLEGAL_UXSTYLE_INPUT_VALUE` / `0x00000069` case):

```
A problem has been detected and Windows XP has been shut down to prevent damage
to your computer.

The problem seems to be caused by the following file: FILE_PLACEHOLDER

MESSAGE_PLACEHOLDER

If this is the first time you've seen this Stop error screen,
refresh your browser tab. If this screen appears again, follow
these steps:

Check to make sure any new hardware or software is properly installed.
If this is a new installation, ask Quenq or your browser vendor
for any Windows XP modifications you might need.

If problems continue, disable or remove any newly installed hardware
or software. Disable BIOS memory options such as chaching or shadowing.
If you need to use Safe Mode to remove or disable components, refresh
your browser tab, press F8 to select Advanced Startup Options, and then
select Safe Mode.

Technical information:

*** STOP: CODE_PLACEHOLDER (0xFD3094C2,0x00000001,0xFBFE7617,0x00000000)


*** FILE_PLACEHOLDER - Address FDF23422 base at FDF24000, DateStamp 3d6dd67c
```

(The misspelling `chaching` is verbatim in the source. Default `code="0x0000007B"`,
`message="INACCESSIBLE_BOOT_DEVICE"`.)

---

## 10. Implementation notes for Rust

* The existing `src/screens/post.rs` / `src/screens/bootmenu.rs` hold a **different** BIOS (Award v6.00PG /
  ASUS, `win32.run`/`boot_manager.svelte`). To match Reborn XP they must be **replaced wholesale** by the
  strings and layout in §2–§5 here — the header block, the 10-line typewriter, and the 2-item ↑/↓/ENTER/ESC
  menu with a 10→0 countdown.
* Suggested constants (all verified above):
  `POST_LINES` (§2.3, 10 entries incl. two `""`), `HEADER_LINES` (§2.2), `FOOTER = "Press <DEL> to enter SETUP, <F12> to Boot Menu"`,
  `MENU = ["Start Windows XP", "Reinstall Windows XP"]`,
  `CONFIRM = ["Yes, continue Setup", "No, return to main menu"]`,
  `POST_LINE_MS = 100`, `POST_HOLD_MS = 1500`, `COUNTDOWN_START = 10`, `COUNTDOWN_TICK_MS = 1000`,
  `BOOT_DELAY_MS = 4000`.
* Font/colour: `vga8x14` at 20–22 px, `#ccc` on black; confirm screen on navy `#000080`; selected row =
  `#ccc` bg / `#000` text; headings uppercased; 80/60 px padding; list indent 40 px; item padding 4/8 px.
* Version string on the `Quenq Sys BIOS v.` line must come from the app version (currently `3.2.2`).

---

## 11. Not retrieved / caveats

* **No design resolution** is fixed anywhere — screens are viewport-fixed and scaled by `body { zoom }`
  (§5.5). Any hard-coded "screen size" for Rust would be an assumption, not a source value.
* The **EPA logo** exists only as an inline base64 GIF inside `css/index.css` (captured verbatim in §5.4);
  no separate image URL was found.
* `res/anim/boot.gif`, `res/fonts/Web437_IBM_VGA_8x14.woff`, `res/ui/cursors/*.cur` are referenced by the
  fetched files but were **not themselves fetched** (binary assets).
* The **`msoobe` OOBE wizard** and the **logon scene** are downstream of the BIOS sequence and are only
  outlined here (§6); their internal screens were not enumerated (out of scope for this spec).
* Everything in §2–§8 was read directly from the fetched `index.js`, `shell.js` and `index.css`; the only
  transformation applied is **de-minification** of the JavaScript for readability. All quoted strings,
  colours, timings and CSS declarations are verbatim.
