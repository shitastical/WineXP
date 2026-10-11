# Porting spec — Windows XP card games & other games

Behavioural specification for re-implementing the Windows XP **games** accurately in Rust:

- **Solitaire** (Klondike)
- **FreeCell**
- **Spider Solitaire**
- **Hearts**
- **3D Pinball for Windows – Space Cadet**
- **Minesweeper** (Game-menu / LED / chording reference)

This document is a *port target*, not a description of what we have already. Where our tree already
contains a counterpart module it is named under **Port notes**; those modules are the current baseline
and may be superseded by the numbers below.

## How to read this document

Every non-obvious fact carries a marker:

| Marker | Meaning |
|---|---|
| ✅ | **Verified** in a primary source (Microsoft/decompilation/source file/help text) named inline. |
| ⚠️ | **Reconstructed / partial** — believed correct and consistent across sources, but not confirmed by an authoritative primary source in this survey. |
| ❌ | **Not found** in this survey. Treat any value as a guess to be measured from a reference build. |

Colours are `#RRGGBB`. Sizes are CSS/DIP pixels at 96 DPI unless stated. Card ranks are `1=A … 13=K`.

> **Asset policy (from `docs/PROVENANCE.md`).** Microsoft's binary assets are **not** committed. Card art
> comes from `cards.dll`, Pinball geometry/art from `PINBALL.DAT`, music from `PINBALL.MID`/`PINBALL2.MID`.
> Our ports must draw cards/table procedurally or load user-supplied assets, and must not vendor leaked
> Windows source (§“Sources” at the end lists the open-source references that *are* usable).

---

## 0. Shared constants

| Thing | Value | Marker |
|---|---|---|
| Playing-card face, classic `cards.dll` "standard" size | `71 × 96` | ✅ (corroborated by `src/apps/solitaire.rs`; cards.dll cards are 71×96) |
| Card corner inset (rank/suit glyph) | ~4 px from the top-left corner, glyph ≈ 12 px | ⚠️ |
| Menu strip height (in-app menu bar) | `20 px` | ✅ (matches `src/apps/{minesweeper,solitaire,pinball}.rs`) |
| Classic dialog face colour | `#D4D0C8`; title strip `#0A246A` with white text | ✅ (`src/apps/solitaire.rs`) |
| Klondike/FreeCell/Spider felt | solid green `#008000` | ⚠️ (uses `0x008000`; the XP table is a flat green with no gradient) |
| Card red | `#C00000`; card black | `#000000` | ⚠️ |
| Font of dialog/card text | Tahoma (`MS Shell Dlg`) ≈ 8 pt / 11–12 px | ⚠️ |
| Ledger scores / stats persistence | `HKCU\Software\Microsoft\…` per game; reset from the game's own dialog | ⚠️ |

The five card games and Minesweeper are **single-window modal-ish apps**: they own a menu bar, disable
window resize (FreeCell/Hearts/Minesweeper are fixed-size), and keep their own score/statistics state
outside the OS.

**Exe names** (Win XP `system32`, and the Start-menu “Games” group): `sol.exe`, `freecell.exe`,
`spider.exe`, `mshearts.exe`, `winmine.exe`, `pinball.exe`. ⚠️ (widely documented, not re-verified here)

---

## 1. Solitaire (Klondike)

### 1.1 Identity

| Item | Value | Marker |
|---|---|---|
| Title bar | `Solitaire` | ✅ (`solitaire.io` how-to; `src/apps/solitaire.rs` sets caption `Solitaire`) |
| Icon | the red-backed deck icon (`solitaire.png` / `cards.dll` deck icon) | ⚠️ |
| Owned by | `Games` folder of the Start menu | ✅ |
| Window | resizable-ish; opens ≈ `800 × 600` in the launchers we surveyed | ✅ (`xp.quenq.com/js/apps/sol.js` sets 800×600) |

### 1.2 Menu structure

```
Game                              Help
 ├ Deal            F2             └ About Solitaire...
 ├ Undo
 ├ ──────────────
 ├ Deck...
 ├ Options...
 ├ ──────────────
 └ Exit
```

- **Deal `F2`** starts a new shuffled deal (this is the hotkey the Microsoft help text itself quotes:
  “On the **Game** menu, click **Deal** (Hot-key is **F2**)”. ✅ from `freecell.io/game/solitaire-windows-xp/`).
- **Undo** — greyed when the history is empty. ✅ (`src/apps/solitaire.rs` greys it on `history.is_empty()`).
- **Deck…** — modal card-back chooser; the back cycles/previews while open. ⚠️
- **Options…** — see §1.4.
- **Exit** closes the app.
- **Help → About Solitaire…** — About box (“Solitaire / Microsoft Windows XP / Version 5.1”). ✅ (`src/apps/solitaire.rs`).
  The canonical XP box also has *Contents (F1) / Search for Help on… / How to Play* rows in the Help menu. ⚠️

### 1.3 Layout & geometry

```
 ┌─────────────── client (green felt #008000) ───────────────┐
 │ [stock] [waste…]        [F1][F2][F3][F4]                  │  ← top row
 │  S      W                ↑ four foundations, one per suit │
 │                                                            │
 │  T1  T2  T3  T4  T5  T6  T7                                │  ← tableau, 7 columns
 └────────────────────────────────────────────────────────────┘
```

- Cards are **71 × 96**. ✅
- **Stock** is the face-down deck, top-left; **waste** sits immediately to its right. Clicking the
  stock turns over 1 (Draw one) or 3 (Draw three) cards; when the stock empties, clicking it recycles
  the waste (Draw-three re-cycles in the same 3-card groups). ✅ (`freecell.io` how-to + Wikipedia).
- **Four foundations** (top-right), one per suit, built **up by suit** `A → K`. ✅
- **Seven tableau piles** (row below), each one card longer than the last: pile *i* (0-based) has
  `i+1` cards, all face-down except the top card. ✅ (Wikipedia: “Seven fanned piles … first pile one
  upturned card, the second two …”).
- Move rules: tableau builds **down, alternating colours**; any face-up run may be moved as a unit;
  an empty tableau pile accepts only a **King** (or a King-led run). ✅ (Wikipedia + `freecell.io`).
- Auto-play: **right-click the board** (or the `A` button on a gamepad) sends every playable card to
  its foundation. ✅ (`freecell.io`).
- Double-clicking a card sends it to its foundation. ✅ (`freecell.io`, `src/apps/solitaire.rs`).

Concrete spacing (⚠️ — reproduce from a screenshot; our `src/apps/solitaire.rs` currently uses):
`MARGIN = 8`, `TOP_GAP = 8`, `ROW_GAP = 14`, face-down step `= 4`, draw-three waste fan `= 14`.
The tableau columns are spread evenly across the client width.

### 1.4 Options dialog (`Game ▸ Options…`)

| Control | Values | Effect | Marker |
|---|---|---|---|
| Draw | **Draw one** / **Draw three** | how many cards leave the stock | ✅ |
| Scoring | **Standard** / **Vegas** / **None** | see §1.5 | ✅ (Wikipedia “Scoring in the Microsoft Windows Solitaire version”) |
| Timed game | on/off | 2 points deducted every 10 s | ✅ |
| Status bar | on/off | shows score + time | ⚠️ |
| Keep score / Display status | checkboxes | whether score is shown | ⚠️ |
| Card back grid | the `Deck…` chooser (may be inside Options) | back selection | ⚠️ |

### 1.5 Scoring

**Standard** (✅ Wikipedia, KB101766):

| Action | Points |
|---|---|
| Waste → Tableau | `+5` |
| Waste → Foundation | `+10` |
| Tableau → Foundation | `+10` |
| Turn over a tableau card | `+5` |
| Foundation → Tableau | `−15` |
| Recycle waste (draw-one) | `−100` |
| Minimum score | clamped at `0` |

- A card moved Waste → Tableau → Foundation therefore scores `5 + 10 = 15` (which is why the help text
  advises never to move a Waste card straight to a Foundation). ✅
- **Timed game:** `−2` points per 10 s elapsed. ✅
- **Win bonus (timed):** `700 000 ÷ seconds_to_finish`, awarded only if the game took **≥ 30 s**;
  under 30 s the bonus is 0. ✅
- **Vegas:** starts at `−52`, `+5` per card played to a foundation (net `+208` on a win), and you may
  not recycle the waste. ⚠️

### 1.6 Animation

- Deal animation (cards leave the stock face-down and stack onto the tableau). ⚠️
- Cards **fly** from tableau/waste to a foundation instead of teleporting. ✅ (universal in the XP build)
- **Win animation:** the four foundation stacks “bounce”/cascade off the bottom of the window and fly
  off screen — the classic Solitaire celebration. ✅ (`Cyanoxide/react-xp` README + community reports;
  the same “cards jump across the screen in cascades” behaviour is described for FreeCell at
  `solitairesite.com/freecell/`).
- `Deck…` back preview animates through all backs. ⚠️

### 1.7 Port notes

`src/apps/solitaire.rs` (in-tree) already implements: 71×96 cards, felt `#008000`, Game menu
`Deal / Undo / Options… / Exit`, a `Help ▸ About Solitaire…` box, draw-one/three, timed option and the
full board model. Missing vs this spec: the `Deck…` chooser, the `Help` items other than About, the
Vegas/None scoring modes (if absent), and the win cascade animation.

---

## 2. FreeCell

### 2.1 Identity

| Item | Value | Marker |
|---|---|---|
| Title bar | `FreeCell` | ✅ (`src/startmenu.rs` label `FreeCell`; `freecell.io` page title) |
| Icon | the four-cards-in-a-row icon | ⚠️ |
| Window | fixed size, no resize; not maximisable | ✅ (`wieslawsoltes/WinXp` COMPATIBILITY: FreeCell is board-only) |

### 2.2 Menu structure

```
Game                              Help
 ├ New Game         F2            ├ Contents             F1
 ├ Select Game…     F3            ├ Search for Help on…
 ├ Statistics       F4            ├ How to Play
 ├ Options…                       ├ ──────────────
 ├ ──────────────                └ About FreeCell
 └ Exit
```

- **New Game `F2`** deals the next game number. ✅ (Microsoft help text)
- **Select Game… `F3`** opens the “Select Game” dialog: a number field `1…1 000 000` (XP) and a
  **“Guess”/random** button; entering a number reproduces that exact deal. ✅ (`freecell.io/freecell-windows-xp`:
  “10 million different games and you can choose a game number yourself by clicking on the number”;
  the *exact* count is `1 000 000` — ✅ Wikipedia: “In Windows XP, FreeCell was extended to support a
  total of 1 million card deals”).
- **Statistics `F4`** — games played / won / lost, win %, current & best win/lose streaks, `Reset`. ⚠️
- **Options…** — see §2.5.
- **Help** items mirror the other card games. ⚠️

### 2.3 Layout & geometry

```
 ┌─── 4 free cells ───┐        ┌─── 4 foundations ───┐
 │ ▢  ▢  ▢  ▢         │        │ A♣ A♦ A♥ A♠ …       │
 └────────────────────┘        └─────────────────────┘

 │C1 │C2 │C3 │C4 │C5 │C6 │C7 │C8 │     ← 8 cascades, all cards face up
   7    7    7    7    6    6    6    6   (cards per column)
```

- One 52-card deck, **all face up**. ✅ (Wikipedia)
- **4 free cells** (top-left) hold **one card each**. ✅
- **4 foundations** (top-right), one per suit, built `A → K`. ✅
- **8 cascades**; columns **1–4 get 7 cards, columns 5–8 get 6**. ✅ (Wikipedia + the deal algorithm in §2.4).
- Tableau builds **down, alternating colours**; any face-up run may be moved subject to the supermove
  limit. Empty cascades accept any card/run.
- Cards are `71 × 96`. ⚠️ (same `cards.dll` deck; the XP FreeCell window is compact and uses the
  standard card size).
- Pointer-only play (one mouse is enough). ✅ (`freecell.io`).

### 2.4 Deal numbering & the Microsoft shuffle ✅

This is the single most porting-critical algorithm, and it is fully public.

```text
state0   = game_number                     # 1 … 32000 (XP: 1 … 1000000)
state(n+1) = (214013 * state(n) + 2531011) mod 2^31      # Microsoft C LCG
rand(n)    = state(n) div 2^16                            # 0 … 32767

deck = [AC, AD, AH, AS, 2C … KS]          # 0 … 51, rank-major then suit CDHS
for i = 0 … 51:
    j = (51 - i) - rand(i) mod (52 - i)   # “swap chosen card with last, emit last”
    swap(deck[i], deck[j])
deal deck[0], deck[1], … across 8 columns row by row
(columns 1-4 receive 7 cards, columns 5-8 receive 6)
```

- The 52-card ordinal mapping is **rank-major, suit order ♣ ♦ ♥ ♠** (`A♣=0 … K♠=51`), so the printed
  card is `rank[i/4] + suit[i%4]`. ✅ (Rosetta Code “Deal cards for FreeCell”).
- The original “Microsoft 32,000” set is exactly this algorithm with `game_number ≤ 32000`; the XP
  build extends the numbering to `1 000 000`. ✅ (Wikipedia; Rosetta Code).
- Known **unsolvable** deals: `#11982` (the only one in 1 … 32 000). ✅ In the first 1 000 000 the eight
  impossible deals are `11982, 146692, 186216, 455889, 495505, 512118, 517776, 781948`. ✅
  (`cards4.net` guide, citing the FreeCell FAQ).
- Easiest/hardest/exact-solution statistics quoted by one launcher: easiest `#25904`, hardest `#31465`,
  longest solution `#29596` (53 moves). ⚠️ (`freecell.io` FAQ — community data, not Microsoft).

### 2.5 Options dialog

FreeCell's `Options…` dialog holds the **auto-play** policy: `Off` / `Quick` (send cards to foundations
as soon as it is safe) and whether to animate auto-play. It does **not** contain a difficulty setting in
the Microsoft build. ⚠️ (The rules site `solitairesite.com/freecell/` describes Easy/Normal/Hard
difficulty tiers — that is *their* web re-implementation, **not** the XP dialog.)

### 2.6 Supermoves ✅

Only one card is moved at a time on the board, but “supermoves” are allowed and animated. The maximum
number of cards movable as one run is

```
C = 2^M × (N + 1)
```

where `M` = number of **empty cascades**, `N` = number of **empty free cells**; the maximum that may be
moved onto an **empty cascade** is `C / 2`. ✅ (Wikipedia, citing the FreeCell FAQ.) The web rules site
states the identical rule (`solitairesite.com/freecell/`: “as many cards as there are empty free cells,
plus one; if an empty column is also available, this number doubles”).

### 2.7 Animation

- Auto-play / manual plays animate the card flying between piles. ⚠️
- **Win:** “when all the cards jump across the screen in cascades at the end” — the four foundations
  cascade off the table. ✅ (`solitairesite.com/freecell/`).

### 2.8 Port notes

No in-tree FreeCell module yet (the Start-menu row `windowsIcons/freecell.png → FreeCell` is inert).
Implement the LCG + Fisher-Yates deal **exactly** as §2.4 so numbered deals are reproducible; keep a
`u32` seed path so a future “Select Game” field is trivial.

---

## 3. Spider Solitaire

### 3.1 Identity

| Item | Value | Marker |
|---|---|---|
| Title bar | `Spider Solitaire` | ✅ (Wikipedia article name; title in the XP build) |
| Icon | the spider / two-deck icon | ⚠️ |
| Origin | shipped from Windows 98 Plus! (1998-06-25), added to Windows Me and **XP** | ✅ (Wikipedia) |
| Window | fixed-size | ⚠️ |

### 3.2 Menu structure

```
Game                              Help
 ├ New Game         F2            ├ Contents             F1
 ├ Undo             Ctrl+Z        ├ Search for Help on…
 ├ Difficulty       F3            ├ How to Play
 ├ Statistics       F4            ├ ──────────────
 ├ Options…         F5            └ About Spider Solitaire
 ├ ──────────────
 └ Exit
```

- Hotkeys reported by the classic FAQ: `F1` Help, `F2` New Game, `F3` Difficulty, `F4` Statistics,
  `F5` Options, `Ctrl+Z` Undo, `D` Deal next row, `M` show an available move. ✅ (GameFAQs FAQ 49848)
  (`Ctrl+O`/`Ctrl+S` open/save-last-game are **Vista+** features and should be omitted from an XP port.) ⚠️
- **Show a move / hint:** in the **XP** build a hint is triggered by **clicking the score panel**
  (the `M`/`H` hotkeys are Vista+). ✅ (Wikipedia, `Microsoft Spider Solitaire` §Features).
- Undo: unlimited, back to the last non-retractable move, each undo costs 1 point. ✅ (Wikipedia)

### 3.3 Difficulty ✅

| XP label | Suits used | Notes |
|---|---|---|
| **Beginner / Easy** | **1 suit** — always **Spades** | four copies of each spade |
| **Intermediate / Medium** | **2 suits** — **Spades + Hearts** | two copies of each |
| **Advanced / Difficult** | **4 suits** | one copy of each |

The XP Help wording is “Easy / Medium / Difficult”; the modern wording is “Beginner / Intermediate /
Advanced” — both refer to the 1/2/4-suit modes. ✅ (Wikipedia + GameFAQs).

### 3.4 Layout & geometry

```
 X X X X X X X X X X      ← rows 1..5 face-down (X)
 X X X X X X X X X X
 X X X X X X X X X X
 X X X X X X X X X X
 O O O O O O O O O O      ← row 6/5 face-up (O)
 [10] [10] [10] [10] [10] ← 5 stock rows of 10, bottom-right, face-down
```

- Two decks = **104 cards**; **54 dealt** to the tableau, **50** left as the stock. ✅
- Ten tableau columns: the **first 4 columns get 6 cards** (5 down + 1 up), the **remaining 6 columns
  get 5 cards** (4 down + 1 up). ✅ (Wikipedia: “first 4 stacks, with the 6th card face up, and 5 cards
  in the remaining 6 stacks”).
- Build **down by rank regardless of suit**; only a run that is **all one suit, descending** can be
  moved as a unit. ✅ (Wikipedia).
- A completed **K→A single-suit run is removed** from the table and scores `+100`. ✅
- Stock: deals **one card to each of the 10 columns**; **not allowed while any column is empty**. ✅
- Cards `71 × 96`. ⚠️

### 3.5 Scoring ✅

```
score = 500
score -= 1 for every move
score -= 1 for every undo
score += 100 for each completed (K→A, one-suit) run     # 8 possible
# max attainable = 800  (500 - moves + 800; a perfect 0-move "win" is not required to reach 800)
```

Exact wording: “the initial score is 500 and every move subtracts one … Every time the player is able
to place an entire sequence of cards of the same suit in order … 100 points are added … There is a
total of eight such sequences … yielding a maximum achievable number of 800.” ✅ (Wikipedia,
`Microsoft Spider Solitaire`, citing *Spider Solitaire Help*). The FAQ example agrees
(131 moves, 8 runs ⇒ `500 − 131 + 800 = 1169`). ✅

### 3.6 Statistics ✅

Per difficulty tab: highest score, number of wins, number of losses, win rate (%), longest win streak,
longest loss streak, current streak, `Reset`. ✅ (GameFAQs FAQ 49848).

### 3.7 Options ✅

`F5` Options checkboxes: “Animate when dealing cards”, “Automatically save game on exit”,
“Automatically open previous game at startup”, “Prompt before saving a game”, “Prompt before opening a
saved game”, “Use sound effects”. Defaults checked: 1, 4, 5, 6. ✅ (GameFAQs FAQ 49848).
> ⚠️ Save/open options are Vista-generation. For a strict XP port keep *Animate cards*, *Use sound*,
> and (if desired) whitespace-free variants; the XP build itself had no explicit save/load game file.

### 3.8 Animation

- Deal **one-row** animation: the ten cards drop face-up onto the ten columns. ✅
- **Win:** a “spectacular final animation” as the completed runs fly off the table. ✅
  (`solitairesite.com/spider-solitaire/win-xp-spider`: “the spectacular final animation that rewards a
  successful resolution”).

### 3.9 Port notes

No in-tree module yet (Start-menu row `windowsIcons/spider.png → Spider Solitaire` is inert). Model the
tableau as `Vec<Vec<Card>>` of length 10, keep a `(u8 suits, u32 score, u32 moves, Vec<Move>)` state, and
implement the “deal only when no column is empty” guard.

---

## 4. Hearts

### 4.1 Identity

| Item | Value | Marker |
|---|---|---|
| Title bar | `Hearts` (XP). Original Win-For-Workgroups title was `The Microsoft Hearts Network`. | ✅ / ⚠️ (Wikipedia: “the original title bar name for the program, ‘The Microsoft Hearts Network’ (although network play was removed in the Windows XP version)”) |
| Also shipped | **Internet Hearts** (Me/XP) — an online variant, separate app id (`src/startmenu.rs` row `Internet Hearts`) | ✅ |
| Window | fixed-size, 4 hands laid out N/E/S/W | ⚠️ |

### 4.2 Menu structure (⚠️ **not verified in this survey**)

Community screenshots show a **`Game` / `Options` / `Help`** bar. Reported items (treat as ⚠️ until
checked against a build):

```
Game                        Options                     Help
 ├ New Game   F2            ├ Animation                 ├ Contents          F1
 ├ ──────────               ├ Sound                     ├ Search for Help on…
 └ Exit                     ├ ──────────                ├ How to Play
                            ├ Statistics…               ├ ──────────
                            ├ Change Player Name…       └ About Hearts
                            └ ──────────
```

> **Change Player Name…** matches the documented first-run behaviour (the game “prompts for the player
> name at startup” before the Vista version switched to using the logged-in account). ✅ (Wikipedia).

### 4.3 Rules (Microsoft Hearts = **Black Lady** variant) ✅

- **4 players**; the human plays against three computer opponents (all three hands are simulated).
- Deal 13 each, then **pass 3**: hand 1 → **left**, hand 2 → **right**, hand 3 → **across**,
  hand 4 → **hold** (“eat”), then the cycle repeats. ✅
- The holder of the **2♣ leads the first trick and must play it**. ✅
- Play = follow the led suit if able; otherwise any card, with restrictions below. **No revoking**:
  the game forbids it. ✅
- **Hearts may not be led until “broken”** — i.e. until a heart has been discarded. A heart may be
  played: (a) by a non-leader who cannot follow suit and this is not the first trick, or (b) by a player
  holding only hearts. ✅
- **Point cards may not be played to the first trick** (except the documented all-hearts edge case). ✅
- Scoring: each **♥ = 1**, **Q♠ = 13**. **Shooting the moon** = taking all 26 point cards ⇒ the shooter
  scores 0 and **every other player scores 26**. ✅
- The **game ends when any player reaches ≥ 100**; **the fewest points wins**. A tie at the low score is
  possible, and if the human is in the tie the game credits the human as winner. The winner's score is
  shown in red; the current leader during play is shown in blue. ✅
- Usually 6–10 hands per game; the theoretical maximum before anyone hits 100 is 16. ✅
- **Computer opponent names:** XP uses **`Pauline`, `Michele`, `Ben`** (the WFW 3.1 build used
  `Anna`, `Lynda`, `Terri`; the Vista+ build uses `West`, `North`, `East`). ✅
- **Help → “How to Play” quotation** (XP): Shakespeare, *Julius Caesar* III.ii —
  “I come not, friends, to steal away your hearts…”. ✅ (Wikipedia; removed from Vista onward).

### 4.4 Layout & geometry (⚠️)

Standard cross layout: the human's hand at the **bottom** (fan of 13, 71×96 cards), opponents at the
left/top/right as small face-down fans, played-card pile in the centre, per-player score line above each
seat, and a scoreboard shown after each hand. Exact pixel metrics ❌ — measure from a reference build.

### 4.5 Animation (⚠️)

Cards fly from the hand to the centre; each trick's point cards stay in front of the player who took
them, then sweep to that player's score. Pass animation moves three cards to the neighbour.

### 4.6 Port notes

No in-tree module yet (Start-menu row `windowsIcons/heart.png → Hearts` is inert). The rules in §4.3 are
complete enough to implement directly; the menu/geometry must be pinned down from a reference build
before claiming accuracy.

---

## 5. 3D Pinball for Windows – Space Cadet

### 5.1 Identity

| Item | Value | Marker |
|---|---|---|
| Title bar | `3D Pinball for Windows - Space Cadet` | ✅ (`k4zmu2a/SpaceCadetPinball` `translations.cpp` → `Msg::STRING` English `"3D Pinball for Windows - Space Cadet"`) |
| About box title | `About 3D Pinball` | ✅ (same file) |
| Splash | a “3D Pinball” splash with a small faded-edge pinball graphic | ✅ (Wikipedia) |
| Data files | `PINBALL.DAT` (table geometry + art), `PINBALL.MID` / `PINBALL2.MID` (music) | ✅ (Wikipedia + `.reference/SpaceCadetPinball/Doc/.dat file format.txt`) |
| Music default | **off** by default in the XP build | ✅ (Wikipedia) |
| Origin | Cinematronics/Full Tilt! “Space Cadet” table; MS builds for 95/NT4/2000/Me/XP | ✅ |

### 5.2 Menu structure (original Windows menus)

Three top-level menus: **Game**, **Options**, **Help** ✅ (`translations.cpp` `Msg::Menu1_Game`,
`Menu1_Options`, `Menu1_Help`). Item order and labels use the original `Menu1_*` strings:

```
Game                                 Options                          Help
 ├ New Game            F2           ├ Full Screen            F4      ├ Help Topics   F1
 ├ Launch Ball         Space         ├ Select Players ▸                ├ ──────────────
 ├ Pause/Resume Game   F3           │   ├ 1 Player                    ├ About 3D Pinball
 ├ ──────────────                  │   ├ 2 Players
 ├ High Scores...                  │   ├ 3 Players
 ├ Demo                            │   └ 4 Players
 ├ ──────────────                  ├ Player Controls...     F8
 └ Exit                Esc         ├ Sounds                 F5
                                   └ Music                  F6
```

- **“Sounds” and “Music” appear as check items** in Options (toggled state shown with a check). ⚠️
- **`Select Table`** (`Menu1_Select_Table`) and **`Table Resolution`**
  (`Menu1_Table_Resolution`, with `Use Maximum Resolution (640×480 / 800×600 / 1024×768)`) exist in the
  strings but belong to the **Full Tilt! three-table** build; the XP build ships only the Space Cadet
  table at 640×480. ✅ (`translations.cpp` + Wikipedia: “only supports 640×480-pixel resolution, while
  [Full Tilt!] supports three different resolutions up to 1024×768”).
- The `k4zmu2a` ImGui port adds modern menus (`Audio`, `Graphics`, `Game Data`, `Reset All Options`,
  language, debug/cheats). Those are **not** original and must not be presented as XP behaviour. ✅
  (`winmain.cpp` `RenderUi()`).

### 5.3 Key bindings

Default controls, verified from the decompilation's default key map:

| Action | Key | Mouse | Marker |
|---|---|---|---|
| Left flipper | `Z` | left button | ✅ (`options.cpp` defaults) |
| Right flipper | `/` | right button | ✅ |
| Plunger (launch) | `Space` | middle button | ✅ |
| Nudge left | `X` | `X1` | ✅ |
| Nudge right | `.` | `X2` | ✅ |
| Nudge bottom | `↑` | `X2+1` | ✅ |
| New Game | `F2` | — | ✅ |
| Pause/Resume | `F3` | — | ✅ |
| Full Screen | `F4` | — | ✅ |
| Sounds | `F5` | — | ✅ |
| Music | `F6` | — | ✅ |
| Player Controls… | `F8` | — | ✅ |
| Toggle menu display | `F9` *(modern port)* | — | ⚠️ |
| Exit | `Esc` | — | ✅ |

`Player Controls…` is the key-mapper dialog: group “Control Options” (the six bindings above) plus
“Instructions” text and a `Default` button; the instructions remind the user that **NUMLOCK must be
off** to use the numeric-keypad arrow keys. ✅ (`translations.cpp` `KEYMAPPER_*`).

### 5.4 Window & table geometry

| Resolution | Window | Table render surface | Marker |
|---|---|---|---|
| Base | `640 × 480` | `600 × 416` (≈40 px right sidebar) | ✅ (`fullscrn.cpp`: `resolution_array[0] = {640, 480, 600, 416, 501}`) |
| Full Tilt! only | `800 × 600` | `752 × 520` | ✅ (`resolution_array[1]`) |
| Full Tilt! only | `1024 × 768` | `960 × 666` | ✅ (`resolution_array[2]`) |

- The extra right strip is the **side display**: score, ball number, player number and a table-specific
  image. ✅ (Wikipedia).
- The playfield is rendered through a one-point perspective (the table is drawn as a trapezoid), with
  sprite/number animations over a background bitmap. ✅ (Wikipedia + `pinball.rs` doc-comment).
- `PINBALL.DAT` container format: signature `PARTOUT(4.0)RESOURCE`; group/entry table with entry types
  `1`=8 bpp bitmap, `3`=group name, `5`=palette, `9`=string, `10`=i16 array, `11`=f32 array (collision
  boxes), `12`=16 bpp zMap; 8-bpp bitmap header carries `resolution (0=640×480,1=800×600,2=1024×768,
  −1=all)`, width/height, x/y position, size, flags (raw/DIB/spliced). ✅
  (`.reference/SpaceCadetPinball/Doc/.dat file format.txt`).
- **Physics loop:** fixed-step, data-driven; `Options.UpdatesPerSecond` default **120**, min 60,
  max 360; frames per second default **60** (also 60…360). ✅ (`options.h`: `MinUps=60, MaxUps=360,
  DefUps=120, DefFps=60`). The XP build itself capped the frame rate (a famous Raymond Chen change). ✅
- Sound: `MinSoundChannels=1, Max=32, Def=8`; volume `0…128` (`MIX_MAX_VOLUME`). ✅ (`options.h`).

### 5.5 Scoring

| Item | Value | Marker |
|---|---|---|
| Score multiplier ladder | `{1, 2, 3, 5, 10}` | ✅ (`TPinballTable.cpp`: `score_multipliers[5]`) |
| Applied score | `Score += score × score_multipliers[ScoreMultiplier]` | ✅ |
| Jackpot start | `20 000` | ✅ (`JackpotScore = 20000`) |
| Jackpot cap | `5 000 000` (Space Cadet); `10 000 000` (Full Tilt) | ✅ |
| Bonus score start | `10 000` | ✅ (`BonusScore = 10000`) |
| Bonus score cap | `5 000 000`; “Bonus up 1 Million” award | ✅ |
| Score rollover | if `CurScore > 1 000 000 000`, subtract `1 000 000 000` (score is kept as a 9-digit display) | ✅ |
| Balls per player | 3 (+ replay/extra ball) | ⚠️ |
| Multiplayer | up to **4 players**, alternating turns | ✅ (`Select Players` 1–4) |

Scoring events with their in-game text (all ✅ from `translations.cpp`): `Replay Awarded`, `Ball Locked`,
`Center Post`, `Bonus Awarded`, `Bonus Activated`, `Weapons Upgraded`, `Engine Upgraded`,
`Extra Ball`, `Reflex Shot Award`, `Final Battle Won`, `Hyperspace Bonus`, `Jackpot Awarded`,
`Multiball`, `Ramp Bonus Awarded`, `Skill Shot`, `Crash Bonus`, `Replay Ball`, `Re-Deploy`,
`Mission Accepted/Completed`, `Promotion to %s`, `Demotion to %s`, `TILT!`, `Game Over`.

### 5.6 Ranks & missions ✅

- **Nine ranks**, low → high: **Cadet, Ensign, Lieutenant, Captain, LT Commander (a.k.a. “Lt Commander”),
  Commander, Commodore, Admiral, Fleet Admiral**. ✅ (Wikipedia + `translations.cpp` rank strings;
  note the string table spells the fifth tier `Lt Commander`).
  Filling the blue mission-circle lights promotes the player; a light in the orange circle turns on. ✅
- **Missions** (from the string table), including: `Target Practice`, `Launch Training`,
  `Re-Entry Training`, `Science`, `Stray Comet`, `Black Hole`, `Space Radiation`, `Bug Hunt`,
  `Alien Menace`, `Rescue`, `Satellite Retrieval`, `Recon`, `Doomsday Machine`, `Cosmic Plague`,
  `Secret`, `Time Warp`, `Maelstrom`. ✅
- A mission is **accepted** by hitting the mission targets to select it, then sending the ball up the
  **launch ramp** (`Launch Ramp To Accept %s Mission`). ✅
- **Fuel:** the lights in the passage under the launch ramp go out one by one; re-light them by rolling
  over them or re-launching up the ramp. Out of fuel ⇒ `Mission Aborted`. ✅ (Wikipedia + string table).
- Mission progress messages: `Attack Bumpers Hits / Left: %d`, `Launches Left: %d`,
  `Re-Entries Left: %d`, `Drop Targets / Left: %d`, `Targets / Left: %d`, `Lane Passes / Left: %d`,
  `Remote Attack Bumper Hits / Left: %d`, plus `Hit Mission Targets To Select Mission`,
  `Re-Fuel Ship`, `Warning -- Low Fuel`, `Ship Re-Fueled`, `Enter Wormhole`, `Enter Black Hole`,
  `Hyperspace Launch`, `Comet Destroyed`, `Radiation Eliminated`, `Xenomorphs Destroyed`,
  `Survivors Rescued`, `Aliens Repelled`, `Satellite Repaired`, `Science Mission Completed`. ✅
- **Tilt / nudge:** nudging moves the ball but repeated nudges trigger `TILT!` and lock the flippers
  until the ball drains. ✅ (decompilation `nudge.cpp`, `TiltLockFlag` in `TPinballTable.cpp`).

### 5.7 Dialogs

- **High Scores…** — table with `Rank`, `Name`, `Score` columns and a `Clear` button
  (`Clear High Scores?` confirmation with `Confirm`/`Cancel`). ✅ (`translations.cpp` `HIGHSCORES_*`).
- **About 3D Pinball** — the About box (version/serial text). ✅
- **Exit** — a confirm popup (“Exit the game?” with `OK`/`Cancel`) in the modern port. ⚠️ (the XP build
  closed immediately; the popup is added by `winmain.cpp`).

### 5.8 Demo / attract mode

- `Game ▸ Demo` toggles a computer-played **attract/demo mode** (`DemoActive`); `Demo\nPlayer 1..4`
  strings show multi-player demo frames. ✅ (`winmain.cpp` + `translations.cpp`).
- Idle behaviour shows `Continue Play` / `F2 Starts New Game` prompts and a `Careful...` nudge warning. ✅

### 5.9 Port notes

`src/apps/pinball.rs` (in-tree) is an **original** recreation, not a line port: it defines its own 3-D
playfield (`TABLE_W = 200`, `TABLE_L = 430`, camera distance `CAM = 480`, `Z_LIFT = 0.62`, ball
`r = 6`), its own physics constants and a **plausible but invented** rank ladder
(`Cadet/Ensign/Lieutenant/Commander/Captain/Wing Commander/Group Captain/Admiral/Fleet Admiral/Space
Cadet` with score thresholds). The *authentic* rank list is the nine-name ladder in §5.6 and the
authentic thresholds live in `PINBALL.DAT`; the in-tree ladder must be replaced when the real data is
loaded. `wieslawsoltes/WinXp`'s Pinball app is likewise explicitly an original table — do **not** copy
its mechanics as XP truth. ✅ (`docs/COMPATIBILITY.md`.)

The only faithful path is the `k4zmu2a/SpaceCadetPinball` decompilation (MIT): load the user-supplied
`PINBALL.DAT`, re-implement `TPinballTable`/`T*` components, the `TEdgeManager` collision grid and the
`timer`/`gdrv` layers. Our port must not link or vendor the C++ or the Microsoft data blob.

---

## 6. Minesweeper (XP reference)

Included because the task asks for its Game-menu difficulties, chording, flag/question cycle and LED
counters. (The XP build is `winmine.exe`; the in-tree `src/apps/minesweeper.rs` already mirrors the
ShizukuIchi port of it.)

### 6.1 Difficulty presets ✅

| Menu item | Rows × Cols | Mines |
|---|---|---|
| `Beginner` | `9 × 9` | `10` |
| `Intermediate` | `16 × 16` | `40` |
| `Expert` | `16 × 30` (30 wide) | `99` |
| `Custom...` | Height `9…24`, Width `9…30` | `10…668` (bounded by `cells − 1`) |

✅ (matches `wieslawsoltes/WinXp` `.reference/…/Minesweeper/config.js` and the in-tree
`Difficulty::config()`).

### 6.2 Game menu ✅

```
Game                              Help
 ├ New                 F2         ├ Contents            F1
 ├ ──────────────               ├ Search for Help on…
 ├ Beginner       ✔              ├ Using Help
 ├ Intermediate                  ├ ──────────────
 ├ Expert                        └ About Minesweeper
 ├ Custom...
 ├ ──────────────
 ├ Marks (?)      ✔
 ├ Color          ✔
 ├ Sound
 ├ ──────────────
 ├ Best Times...
 ├ ──────────────
 └ Exit
```

✅ (`.reference/winXP/src/WinXP/apps/Minesweeper/dropDownData.js`, mirrored by the in-tree `menus()`).
`Beginner/Intermediate/Expert` show a **radio check** on the active difficulty; `Marks (?)` and `Color`
are check items; `Best Times...` opens the per-difficulty fastest-time table with a `Reset Scores`
button. ✅

### 6.3 Board mechanics

| Behaviour | Detail | Marker |
|---|---|---|
| First click | **always safe** — mines are placed *after* the first click, excluding the clicked cell (and, where possible, its neighbours) | ✅ (`wieslawsoltes/WinXp` COMPATIBILITY: “safe first-click generation”; in-tree `start_game(exclude)`) |
| Chording | on a **revealed number** whose adjacent flags equal the number, middle-click (or left+right) reveals all remaining neighbours; if a flag is wrong, the player loses | ✅ (in-tree `chord()`; Wikipedia: flags/question marks) |
| Flag cycle | right-click cycles `blank → flag → question mark (?) → blank`; the `?` step is only present while **Marks (?)** is enabled | ✅ (ShizukuIchi `CeilState::{Cover,Flag,Unknown}` + `Marks (?)` menu item) |
| Left-click a flag | ignored (does not reveal) | ✅ (in-tree `open_ceil`) |
| Win | when every non-mine cell is revealed, all remaining mines are **auto-flagged** | ✅ (Wikipedia) |
| Loss | the clicked mine shows as an exploded `Die` cell; other mines are revealed; **mis-flagged safe cells show a red X** over the mine | ✅ (Wikipedia; in-tree `MisFlagged`) |
| Face button | the smiley has four states — neutral, `O`-face (button held), **cool sunglasses** on win, **X-eyes** on loss | ✅ (in-tree `Status::{New,Started,Died,Won}`) |

### 6.4 LED counters

- Two **3-digit, seven-segment style LED** read-outs in the recessed top panel: **mines remaining** on
  the **left** (`mines − flags`) and **elapsed seconds** on the **right**. ✅
- Digits are red on black, zero-padded to 3 digits; the timer counts whole seconds from the first click
  and caps at `999`. ✅
- Cell size is `16 px`; the grid panel is `cols×16 + 6` by `rows×16 + 6` with the recessed 3 px
  highlight edges. ✅ (in-tree `grid_size()` — matches the sprite-based XP board).

### 6.5 Port notes

`src/apps/minesweeper.rs` implements the presets, safe first click, chording, the flag→?→blank cycle,
the smiley states, the LED counters and the `Game`/`Help` menus. Gaps vs the XP build: `Best Times...`,
`Custom...` dialog, `Color` (monochrome mode), `Sound`, and the red X on mis-flagged cells (rendered but
worth re-checking).

---

## 7. Cross-game integration

- **Start menu → Games** lists (in our tree, `src/startmenu.rs`): `FreeCell`, `Hearts`, `Internet Hearts`,
  `Minesweeper`, `Pinball`, `Solitaire`, `Spider Solitaire`. ✅ (rows exist) — all except Minesweeper and
  Pinball are currently **inert** rows.
- `wieslawsoltes/WinXp` groups them as package **`apps-games`** with ids
  `minesweeper, solitaire, freecell, spider, hearts`, and a separate **`apps-pinball`** package with id
  `pinball`. ✅ (`README.md`).
- `wieslawsoltes/WinXp` explicitly warns it is **not** pixel-perfect and that its FreeCell numbered seeds
  are **its own**, *not* Microsoft deal numbers; its Hearts follows suit/first-trick/hearts-broken/
  penalty/shoot-the-moon rules; its Spider supports 1/2/4 suits; its Klondike supports draw one/three,
  waste recycling, legal sequences, foundations, scoring and undo. ✅ (`docs/COMPATIBILITY.md`).
- Consistency rule for our ports: **card art must be drawn procedurally** (no `cards.dll`), the
  FreeCell deal generator must follow §2.4 exactly, and Pinball must load a user-supplied `PINBALL.DAT`.

---

## 8. Rust module mapping (target)

| Game | Module | Status |
|---|---|---|
| Solitaire (Klondike) | `src/apps/solitaire.rs` | partially present (see §1.7) |
| FreeCell | `src/apps/freecell.rs` (new) | not started — implement §2 |
| Spider Solitaire | `src/apps/spider.rs` (new) | not started — implement §3 |
| Hearts | `src/apps/hearts.rs` (new) | not started — implement §4.3 rules; pin menu/geometry first |
| 3D Pinball | `src/apps/pinball.rs` | recreation present; replace invented ranks with §5.6 and load real data |
| Minesweeper | `src/apps/minesweeper.rs` | present; close the §6.5 gaps |

Register each new `AppKind` in `src/apps/mod.rs` (`create`, `from_name`, `desktop_icons`) and wire the
inert `src/startmenu.rs` rows.

---

## 9. Unverified / not retrievable in this survey

**Facts left unverified (⚠️ / ❌) and where to pin them down:** exact pixel spacing and overlap of the
Klondike/FreeCell/Spider tables; the Hearts menu items and board metrics; the `Deck…` card-back chooser
contents; FreeCell/Spider `Help` submenu wording; per-difficulty `Best Times` default records; Pinball
per-mission target counts and rank thresholds (these live in `PINBALL.DAT`, not in the decompiled code).

**Requested sources that returned no usable game detail:**

| URL | Result |
|---|---|
| `https://www.xpgameplus.com/` | Returned only the page title and “Total: 60 Games” — a JS-rendered index with **no game content**. ⚠️ |
| `https://pinball.alula.me/` | Returned only the heading and “Downloading…” — the WebAssembly build renders client-side; no text content. The underlying engine is the `alula/SpaceCadetPinball` Emscripten port, so the decompilation source (used above) is the real reference. ⚠️ |
| `https://xp-cards.com/spider` | Returned only the page title; no rules/menu text. |
| `https://kbhgames.com/game/freecell-windows-xp` | Game listing only; confirms the game is “developed by gameboss” with XP graphics, but no menu/rule detail. |
| `https://www.freecell.io/game/freecell-windows-xp/` | Descriptive prose + FAQ (used for the deal-number/easiest-hardest claims); the site credits “GameBoss”. |
| `https://wieslawsoltes.github.io/WinXp/` live app | Not fetched as an app; the repo `README.md` + `docs/COMPATIBILITY.md` were used instead, and they state the project is **not** pixel-perfect. |

---

## 10. Sources

- **3D Pinball (primary, MIT):** `k4zmu2a/SpaceCadetPinball` — `README.md`,
  `SpaceCadetPinball/translations.cpp` + `translations.h` (window title, all menu/mission/rank strings),
  `winmain.cpp` (menu tree, key handling), `options.cpp` + `options.h` (default key map, UPS/FPS
  bounds), `fullscrn.cpp` (resolutions `{640,480,600,416}`, `{800,600,752,520}`, `{1024,768,960,666}`),
  `TPinballTable.cpp` (score multipliers, jackpot/bonus caps), `Doc/.dat file format.txt`. Local copy in
  `.reference/SpaceCadetPinball/`.
- **FreeCell deal algorithm:** Rosetta Code, *Deal cards for FreeCell* (LCG + Fisher-Yates, card order,
  sample deals #1 and #617); `cards4.net` *FreeCell Microsoft Deal Numbers* (impossible deals; 32 000 vs
  1 000 000 range).
- **FreeCell / Spider / Hearts background:** Wikipedia — *FreeCell*, *Microsoft FreeCell*,
  *Spider (solitaire)*, *Microsoft Spider Solitaire*, *Microsoft Hearts*, *Klondike (solitaire)*,
  *Microsoft Minesweeper*, *Full Tilt! Pinball* / *3D Pinball for Windows – Space Cadet*.
- **Klondike scoring:** Wikipedia *Klondike (solitaire)* §Scoring, citing Microsoft KB101766.
- **Spider scoring / options / hotkeys:** GameFAQs FAQ 49848 (*Spider Solitaire*) and Wikipedia
  *Microsoft Spider Solitaire* §Features/§Scoring.
- **Rules/help prose, XP look-and-feel:** `freecell.io/game/solitaire-windows-xp/`,
  `freecell.io/game/freecell-windows-xp/`, `solitairesite.com/freecell/`,
  `solitairesite.com/spider-solitaire/win-xp-spider`, `kbhgames.com/game/freecell-windows-xp`.
- **Browser-recreation feature checklist (not an authority):** `wieslawsoltes/WinXp` `README.md` and
  `docs/COMPATIBILITY.md`.
- **In-tree references:** `src/apps/{solitaire,pinball,minesweeper}.rs`, `src/apps/mod.rs`,
  `src/startmenu.rs`, `docs/PROVENANCE.md`, `docs/FEATURE-GAPS.md`,
  `.reference/winXP/src/WinXP/apps/Minesweeper/{config,dropDownData}.js`.
