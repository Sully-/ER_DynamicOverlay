# Elden Ring Overlay (offline, read-only)

A read-only overlay for an **already-running** `eldenring.exe`. It shows a customizable dashboard — IGT, a **boss** counter, Great Runes, deaths, NG+, key items — plus **boss/loot checklists** with item-randomizer support.

> **Read-only, offline, no cheating.** No memory writes, no anti-cheat bypass. Single-player, offline use only.

> **This file is the user guide** shipped with the release. Full documentation, source code and developer notes are on GitHub: **<https://github.com/Sully-/ER_DynamicOverlay>**.

> 🇫🇷 Version française : [README.fr.md](README.fr.md).

---

## Table of contents

- [Quick start](#quick-start)
- [Warnings](#warnings)
- [Configuration (`er_overlay.toml`)](#configuration-er_overlaytoml)
- [The dashboard: tiles, tracking modes and metrics](#the-dashboard-tiles-tracking-modes-and-metrics)
- [Checks panel (randomizer-aware)](#checks-panel-randomizer-aware)
- [Challenge mode](#challenge-mode)
- [Layout editor](#layout-editor)
- [Troubleshooting](#troubleshooting)
- [License](#license)

---

## Quick start

### 1. Launch Elden Ring offline

The overlay **does not work with EasyAntiCheat enabled**. Start the game in offline mode, for example:

- Launch `eldenring.exe` directly (not through the EAC launcher), with a `steam_appid.txt` containing `1245620` next to the exe, **or**
- Use your usual offline / no-EAC method.

Requirements: Windows **x64**, and an Elden Ring build supported by this release (currently **2.6.2.0 (WW)** and **2.6.2.1 (JP)** — see [Troubleshooting](#troubleshooting) if values show `---`).

Keep the game running on the title screen or in a save — the injector attaches to an already-running process.

### 2. Run the overlay

Keep every file from the extracted folder together — **do not separate them:**

| File / folder | Role |
|---------------|------|
| `er_overlay_injector.exe` | Launcher — **double-click this** |
| `er_overlay.dll` | Overlay (injected into the game) |
| `er_overlay.toml` | Settings (position, scale, hotkeys, layout file…) |
| `layouts/` | Dashboard layout files |
| `tables/` | Boss / checks lists per language |
| `assets/` | Item icons |
| `companion/er_checks_extractor.exe` | Helper that reads a randomizer `regulation.bin` (see [Checks panel](#checks-panel-randomizer-aware)) |
| `layout_editor.html` | Visual layout editor (see [Layout editor](#layout-editor)) |
| `challenge_state.toml` | *(runtime)* Challenge PB / tries — created when `[challenge] enabled = true` |
| `checks_flags.toml` | *(runtime)* Per-seed randomizer flags — created only when `regulation_path` is set |

With Elden Ring already running offline, **double-click `er_overlay_injector.exe`**. The overlay appears in-game (default: top-right HUD only). Re-run the injector after each game restart — it is not persistent across launches.

**Default hotkeys** (defined in `er_overlay.toml`, hot-reloaded every 2 s):

| Key | Action |
|-----|--------|
| `F8` | Switch layout section (`minimalist` → `extended` → `challenge`, …) |
| `F7` | Toggle the boss checklist panel |
| `F6` | Toggle the checks panel (boss + loot checklist, randomizer-aware) |
| `F9` | Show / hide the entire overlay |

The **boss panel**, the **checks panel** and the **extended** layout section are mutually exclusive: opening one closes the others.

If something goes wrong, check `logs/er_injector.log` and `logs/er_overlay.log` in the same folder.

### 3. Customize your dashboard

Everything shown is driven by a **layout file** — a grid of tiles. Edit it visually with the bundled **`layout_editor.html`** (no TOML to learn), then point `layout_file` at your file in `er_overlay.toml`. See [Layout editor](#layout-editor) for the workflow and [The dashboard](#the-dashboard-tiles-tracking-modes-and-metrics) for what each tile does.

### 4. Tweak appearance and behavior

Open `er_overlay.toml` in any text editor (hot-reloaded ~every 2 s). The most common options are `anchor` / `offset_x` / `offset_y` (position), `scale` / `text_size` / `icon_size` (size), and the panel toggles. See [Configuration](#configuration-er_overlaytoml) for the full reference.

### Advanced injector (command line)

For specific cases you can run the injector from a terminal with flags:

```powershell
# target a specific process id
.\er_overlay_injector.exe --pid 12345
# explicit DLL path
.\er_overlay_injector.exe --dll ".\er_overlay.dll"
# validate everything without injecting
.\er_overlay_injector.exe --dry-run
```

## Warnings

- **Offline only** — no multiplayer / online support.
- **Does not bypass EAC** — launch the game without EasyAntiCheat (e.g. run `eldenring.exe` directly with `steam_appid.txt`).
- **Read-only** — no memory writes, this is not a trainer.
- **Transparent, documented injection** (`LoadLibraryW` via `CreateRemoteThread`), no stealth.

## Configuration (`er_overlay.toml`)

Read next to the DLL and **hot-reloaded every 2 seconds** — you can edit it while the game runs. Out-of-range values are clamped to their default with a warning in the log.

### Appearance & position

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `layout_file` | path | `layouts/dashboard.toml` | Layout file to display (see [The dashboard](#the-dashboard-tiles-tracking-modes-and-metrics)). |
| `default_layout_section` | string | — | Section shown at startup (overrides the layout's own `default_section`). |
| `anchor` | enum | `top-right` | Anchor corner: `top-left`, `top-right`, `bottom-left`, `bottom-right`. |
| `offset_x`, `offset_y` | px | `16`, `16` | Offset from the anchor corner. |
| `scale` | 0–4 | `1.0` | Global overlay scale. |
| `text_size` | px (≤72) | `18` | Base font size. |
| `icon_size` | px (≤128) | `24` | Reference icon size. |
| `background_opacity` | 0–1 | `0.65` | Window background opacity. |
| `gray_tint` | 0–1 | `0.40` | Tint of **unowned** items (lower = darker). |
| `use_item_icons` | bool | `true` | `true` = real PNG icons when present, otherwise colored dots. |
| `icons_dir` | path | `assets/icons` | PNG folder (relative to the DLL). |
| `show_debug` | bool | `false` | Shows a diagnostics window (backend, resolved pointers, loaded flags). |

### Hotkeys

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `layout_section_hotkey` | string | `F8` | Cycle through layout sections, e.g. `"F8"`, `"Ctrl+Shift+F1"`. |
| `boss_panel_hotkey` | string | `F7` | Toggle the boss checklist panel. |
| `checks_panel_hotkey` | string | `F6` | Toggle the checks panel (boss + loot checklist). |
| `hide_all_hotkey` | string | `F9` | Show / hide the entire overlay. |

### Boss & checks panels

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `boss_panel_visible` | bool | `true`* | Show the boss panel at startup. At most one of boss / checks shows at startup; boss wins if both are `true`. |
| `boss_panel_scope` | enum | `current-region` | `current-region` or `all-regions`. |
| `boss_panel_layout` | string | — | Panel `x,y,width,height` (pixels or `%`). Omit or `auto` = `"-5, 10, 25%, 92%"` (right-aligned), shifted below the minimalist HUD. Negative x/y = offset from right/bottom edge. |
| `boss_locale` | string | `auto` | Boss table language (`en`, `fr`, …). `auto` reads the game language via Steam; falls back to `en`. |
| `checks_panel_visible` | bool | `false` | Show the checks panel at startup. Mutually exclusive with the boss panel (boss wins if both are `true`). |
| `checks_panel_scope` | enum | `current-region`* | `current-region` or `all-regions`. |
| `checks_panel_layout` | string | — | Panel `x,y,width,height` (pixels or `%`). Omit or `auto` = `"5, 10, 25%, 92%"` (left-aligned, mirrors the boss panel). |
| `regulation_path` | path | — | Path to the `regulation.bin` the game **loads** (your randomizer / ModEngine mod). Enables per-seed resolution of randomized loot flags. Empty/omitted = vanilla flags. See [Checks panel](#checks-panel-randomizer-aware). |
| `checks_extractor_path` | path | — | Override the helper exe location. Omit to auto-find `companion/er_checks_extractor.exe` (then `er_checks_extractor.exe`) next to the DLL. |

\* The shipped `er_overlay.toml` sets `boss_panel_visible = false` and `checks_panel_scope = all-regions`.

The optional challenge ruleset is configured under `[challenge]` — see [Challenge mode](#challenge-mode).

## The dashboard: tiles, tracking modes and metrics

Everything on screen is driven by the **layout file** (`layout_file`). A layout is a **grid** of tiles; each tile occupies one or more cells. The easiest way to edit it is the visual [Layout editor](#layout-editor).

### Tile kinds

| Kind | Shows |
|------|-------|
| `metric` | A counter or time: IGT, deaths, NG+, bosses killed, challenge **PB** / **TRIES**, group progress, item quantity. See [Available metrics](#available-metrics). |
| `item` | A single tracked item, with one or more **tracking modes** (below). |
| `label` | Plain decorative text (heading, separator). |

### Item tracking modes

An `item` tile can track up to three **independent** aspects of an item. You can combine them on the same tile (e.g. a talisman that highlights when equipped *and* stays lit once acquired).

| Mode | Enable with | What it does |
|------|-------------|--------------|
| **Owned** (default) | *(always on)* | Icon **in color** when the item is currently in your inventory (or its pickup flag is set), **greyed out** otherwise. Consumables show their quantity instead. |
| **Equipped** | `track_equipped = true` | Adds a **green border** while the item is **currently equipped** — talismans, Great Runes, quick-slot consumables, pouch. Ideal for seeing your active loadout at a glance. |
| **Historic** | `historic = true` | Keeps the item marked as owned **even after you no longer hold it** (consumed, sold, discarded). Instead of only reading your current inventory, it also checks the item's **acquisition flag**, so "did I ever pick this up?" stays true. **Randomizer-aware:** it resolves the seed-specific flag when `regulation_path` is set. |

**Why they matter**

- **Equipped** answers *"is this talisman/rune slotted right now?"* — great for build/loadout HUDs.
- **Historic** answers *"have I obtained this at least once this run?"* — essential for one-time or consumable items (e.g. scarseals/soreseals, scorpion charms) that you might swap out, so the tile doesn't go dark the moment the item leaves your inventory.

In the layout editor, both are simple checkboxes on an item tile. In TOML they look like this:

```toml
[[section.tile]]
kind = "item"
key = "fire_scorpion_charm"
track_equipped = true   # green border while worn
historic = true         # stays lit once obtained
col = 0
row = 0
```

### Available metrics

The `metric` field of a `metric` tile accepts:

| Metric | Meaning |
|--------|---------|
| `igt` | In-game time (`HH:MM:SS`). |
| `deaths` | Death count. |
| `ng_cycle` | New Game cycle (`NG+N`). |
| `bosses` | Bosses killed out of 207. |
| `pb` | Challenge personal best (requires `[challenge] enabled = true`). |
| `nbtries` | Challenge failed run count (aliases: `tries`, `challenge_pb`, `challenge_tries`). |
| `scadutree_blessing` | Scadutree Blessing level spent at Sites of Grace (`N/20`). |
| *group name* | `owned/total` progress of an aggregate group (e.g. `great_runes`). |
| *item key* | Quantity (for a consumable) or `0/1` owned state for a unique item. |

Any unknown key renders `---` (unavailable).

### Sections

A layout can contain multiple **sections**; only one is visible at a time. Switch between them with `layout_section_hotkey` (`F8` by default) — handy for keeping a "minimalist" and a "full" view on the same key. The bundled `layouts/dashboard.toml` ships three sections: `minimalist`, `extended`, and `challenge`.

## Checks panel (randomizer-aware)

The **checks panel** is a single checklist of everything worth completing in a run. A *check* is one thing to do: a **boss to kill** or an **important item to grab**. Think of it as the boss panel, but it also lists key loot — and it can follow the **item randomizer**.

### How to use it (the basics)

1. Start Elden Ring and run the overlay (see [Quick start](#quick-start)).
2. Press **`F6`** to open or close the panel.
3. Play normally. Each line ticks itself off the moment you kill the boss or pick up the item — you don't click anything.

What you see:

- Checks are **grouped by region** (Limgrave, Liurnia, …), so you can see what's left where you are.
- A check you've completed is **ticked / highlighted**; one you haven't is dim.
- Hover a line to see a **location hint** (where to find it).
- By default the panel shows the **region you're currently in**. To list every region at once, set `checks_panel_scope = "all-regions"` in `er_overlay.toml`.

That's all most people need. The rest of this section is **only for randomizer players**.

### Vanilla (no mods): nothing to do

If you play normal Elden Ring, you're done — the checklist is built in and works out of the box. Leave `regulation_path` empty in `er_overlay.toml` and just press `F6`.

### With the item randomizer (thefifthmatt, [Nexus #428](https://www.nexusmods.com/eldenring/mods/428))

The randomizer **shuffles where items are**, so a given spot on the ground holds a different item every seed. To tick those off correctly, the overlay has to read the **same `regulation.bin` your game is actually loading** (the modded one, not the vanilla game file).

Do this once per setup:

1. **Find your modded `regulation.bin`.** It's the file the randomizer generated for your seed — usually inside the mod folder you launch the game with, for example:
   - ModEngine 2: `…\ModEngine2\mod\regulation.bin`
   - Randomizer output folder: wherever you told the randomizer to write, next to its other files.

   If you're not sure, it's the `regulation.bin` your launch profile / ModEngine config points at — **not** the one in your Steam `Game\` install.

2. **Tell the overlay where it is.** Open `er_overlay.toml` and set `regulation_path` to that full path. Use **single quotes** so you don't have to double your backslashes:

```toml
regulation_path = 'C:\Games\ModEngine2\mod\regulation.bin'
```

3. **Save the file.** Within ~2 seconds the overlay reads the modded regulation on its own (via the bundled `companion/er_checks_extractor.exe`) and starts tracking the right items for your seed.

4. **Check it worked:** the panel header shows a **`[seed]`** tag when the seed mapping is active. No tag = it didn't load (see below).

You only do this once. When you **change seed**, just point `regulation_path` at the new `regulation.bin` (or replace the file at the same path) and save — the overlay re-reads it automatically. You never run the helper by hand.

**If the `[seed]` tag doesn't appear:**

- Double-check the path is the **modded** `regulation.bin` and that the file exists (typos, wrong folder).
- Make sure `companion/er_checks_extractor.exe` is present next to `er_overlay.dll` (don't move files out of the extracted folder).
- Look at `logs/er_overlay.log` — it logs whether the extractor ran and wrote `checks_flags.toml`.

**Good to know**

- Bosses and chest loot use fixed flags, so they tick off the same with or without the randomizer. Only **ground loot** needs the seed step above.
- If your seed puts an item with **no tracking flag** on a randomized spot, that line is greyed out and labelled **"Untraceable this seed"**. This is normal, not a bug.
- To go back to vanilla tracking, empty or remove `regulation_path` and save.

## Challenge mode

Track a **personal best** (most bosses killed on a run within your death budget) and how many times the run **failed**, without editing game saves. **Disabled by default.**

### Configuration (`[challenge]`)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | When `false`, challenge metrics show `---` and no progress is tracked. |
| `max_deaths` | u32 | `0` | Deaths allowed **per run** (inclusive). The run fails when run deaths exceed this value. `0` = deathless. |
| `start_flag` | u32 | `101` | Event flag that marks the **start of a run** (flag `101` = left the Cave of Knowledge / Stranded Graveyard). |

```toml
[challenge]
enabled = true
max_deaths = 0      # deathless: one death ends the run
start_flag = 101
```

### Metrics

Add these as `kind = "metric"` tiles (the bundled `layouts/dashboard.toml` includes a **`challenge`** section with both):

| Metric | Label idea | Meaning |
|--------|------------|---------|
| `pb` | PB | Highest boss kill count recorded while the current run is still within the death budget. |
| `nbtries` | TRIES | Number of failed runs (increments once when deaths exceed `max_deaths`, not once per extra death). |

When `[challenge].enabled = false`, both show `---`.

### Typical deathless run (`max_deaths = 0`)

| Event | PB | TRIES |
|-------|-----|-------|
| Kill 1 boss, no deaths | 1 | 0 |
| First death (run failed) | 1 (frozen) | 1 |
| Kill another boss on the same save | 1 | 1 |

After a failed run, PB stays frozen until you start a **new game** (flag `101` clears with zero deaths on the character). You can keep playing on the same save; the overlay just stops counting a new PB for that failed run.

### Enabling in-game

1. Set `enabled = true` under `[challenge]` in `er_overlay.toml` (hot-reloaded ~every 2 s).
2. Press **`F8`** until the **`challenge`** layout section is visible, or add `pb` / `nbtries` tiles to your own layout.
3. Leave the tutorial cave — run tracking starts when flag `101` becomes active.

**Notes**

- **Progress file:** `challenge_state.toml` (next to `er_overlay.dll`, created at runtime) stores personal best, failed run count, and internal run state. Delete it to reset PB and tries.
- Challenge updates are paused during loading screens / when in-game time is not running, so respawn fades do not corrupt run state.

## Layout editor

The zip includes a **visual editor** — no TOML syntax to learn — as **`layout_editor.html`** at the root (with `layout_editor_assets/`).

1. Open **`layout_editor.html`** from the extracted folder in your browser (Chrome, Edge, Firefox…).
   - If import/export is blocked, serve the folder instead: open a terminal in the folder and run `python -m http.server`, then go to `http://localhost:8000/layout_editor.html`.
2. **Drag** metrics, labels, and items from the left palette onto the grid.
3. Tune the grid (columns, rows, cell size, gap) and each tile in the right panel — including the `track_equipped` and `historic` toggles for item tiles (see [Item tracking modes](#item-tracking-modes)).
4. Use **Import layout file** to edit the bundled `layouts/dashboard.toml`, or start from **New**.
5. Click **Export layout file** and save the `.toml` into the `layouts/` folder (e.g. `layouts/my_run.toml`).
6. Edit `er_overlay.toml` and set `layout_file = "layouts/my_run.toml"`. The overlay reloads the file automatically within ~2 seconds (even while the game is running).

**Tip:** create multiple **sections** in one file (e.g. a compact view and a full view) and switch between them with `F8`.

## Troubleshooting

| Problem | Hint |
|---------|------|
| Injector: "process not found" | Launch Elden Ring first. |
| Injection fails | EAC is active → run the game offline; try running the injector as administrator. |
| "LoadLibraryW returned NULL" | DLL missing / missing dependency / wrong architecture — check the DLL path. |
| All values show `---` | Game version unsupported — check `logs/er_overlay.log` for `Unsupported game executable` or set `show_debug = true`. Supported builds: **2.6.2.0 (WW), 2.6.2.1 (JP)**. |
| Game crashes on inject | Check `logs/er_overlay.log`: the last line before the crash pinpoints the step. Update the game if the log says unsupported executable. |
| No icons (only dots) | PNGs missing from `assets/icons` — keep the folder next to `er_overlay.dll`. |
| Overlay crash | Conflict with another DX12 hook (RTSS, etc.). |
| Item tile never lights up | Wrong `key`, or the item leaves your inventory — add `historic = true` to keep it lit after acquisition (see [Item tracking modes](#item-tracking-modes)). |
| Equipped highlight never shows | `track_equipped = true` only lights up while the item is actually equipped (talismans, runes, quick slots, pouch). |
| Challenge metrics always `---` | Set `[challenge] enabled = true` in `er_overlay.toml`. |
| PB / tries look wrong after testing | Delete `challenge_state.toml` next to the DLL and retry on a clean run. |
| Randomized ground loot not tracked | Set `regulation_path` to the `regulation.bin` the game loads; check `logs/er_overlay.log` and that `checks_flags.toml` was written. |
| Checks header has no `[seed]` tag | No seed mapping active — `regulation_path` is unset/wrong, or `er_checks_extractor.exe` is missing next to the DLL. |

### Logs and diagnostics

All runtime output goes to **`logs/`** next to `er_overlay.dll`:

| File | Contents |
|------|----------|
| `er_overlay.log` | DLL init, game version probe, hook, pointer resolution, errors |
| `er_injector.log` | Process lookup, EAC warning, injection result |

Enable **`show_debug = true`** in `er_overlay.toml` for an in-game diagnostics window. For verbose logging, set env `RUST_LOG=debug` before launching the injector.

## License

**GNU Affero General Public License v3.0 (AGPL-3.0-only)** — see [`LICENSE`](LICENSE).

This is a **strong copyleft** license: anyone who distributes this software, a modified version, or a derivative work — **including merely making it available over a network** — must release the complete corresponding source code under the same AGPL-3.0 license.
