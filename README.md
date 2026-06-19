# Elden Ring Overlay (offline, read-only)

A Rust overlay injected into an **already-running** `eldenring.exe`, **read-only**. Shows a customizable dashboard: IGT, a **boss** counter, Great Runes, deaths, NG+, Key items, and **boss/loot checklists** (with item-randomizer support)...

![Elden Ring Overlay](docs/overlay.png)

> **Read-only, offline, no cheating.** No memory writes, no anti-cheat bypass. Single-player, offline use only.

> **Note on development.** This project was developed largely with the assistance of an LLM (code generation, refactoring, documentation). The code is reviewed and tested, but keep this in mind when reviewing or reusing it.

---

## Table of contents

**User guide**
- [Quick start (GitHub release)](#quick-start-github-release)
- [Warnings](#warnings)
- [Installation](#installation)
- [Configuration (`er_overlay.toml`)](#configuration-er_overlaytoml)
- [Checks panel (randomizer-aware)](#checks-panel-randomizer-aware)
- [Challenge mode](#challenge-mode)
- [Customizing the display](#customizing-the-display)
- [Layout editor](#layout-editor)
- [Troubleshooting](#troubleshooting)

**Technical reference**
- [Architecture](#architecture)
- [Layout format (reference)](#layout-format-reference)
- [Available metrics](#available-metrics)
- [Game data (tables)](#game-data-tables)
- [Adding a good](#adding-a-good)
- [Icons](#icons)
- [Development](#development)
- [References](#references)

---

# User guide

## Quick start (GitHub release)

**You don't need to compile anything.** Download a pre-built zip from the [GitHub Releases](https://github.com/Sully-/ER_DynamicOverlay/releases) page (`er-overlay-vX.X.X.zip`), extract it anywhere, and follow the steps below.

### 1. Launch Elden Ring offline

The overlay **does not work with EasyAntiCheat enabled**. Start the game in offline mode, for example:

- Launch `eldenring.exe` directly (not through the EAC launcher), with a `steam_appid.txt` containing `1245620` next to the exe, **or**
- Use your usual offline / no-EAC method.

Keep the game running on the title screen or in a save — the injector attaches to an already-running process.

### 2. Run the overlay

After extracting the zip, you should have a single folder containing at least:

| File / folder | Role |
|---------------|------|
| `er_overlay_injector.exe` | Launcher — **double-click this** |
| `er_overlay.dll` | Overlay (injected into the game) |
| `er_overlay.toml` | Settings (position, scale, hotkeys, layout file…) |
| `layouts/` | Dashboard layout files |
| `tables/` | Boss / checks lists per language |
| `assets/` | Item icons |
| `companion/er_checks_extractor.exe` | Helper that reads a randomizer `regulation.bin` (see [Checks panel](#checks-panel-randomizer-aware)) |
| `layout_editor.html` | Visual layout editor (see step 3) |
| `challenge_state.toml` | *(runtime)* Challenge PB / tries — created when `[challenge] enabled = true` |
| `checks_flags.toml` | *(runtime)* Per-seed randomizer flags — created only when `regulation_path` is set |

**Do not separate these files** — they must stay in the same folder.

1. With Elden Ring already running offline, **double-click `er_overlay_injector.exe`**.
2. The overlay appears in-game (default: top-right HUD only). Open a side panel with `F7` (boss) or `F6` (checks).

That's it. Re-run the injector after each game restart (the overlay is not persistent across launches).

**Useful hotkeys** (defaults in `er_overlay.toml`, hot-reloaded every 2 s):

| Key | Action |
|-----|--------|
| `F8` | Switch layout section (`minimalist` → `extended` → `challenge`, …) |
| `F7` | Toggle boss checklist panel |
| `F6` | Toggle checks panel (boss + loot checklist, randomizer-aware) |
| `F9` | Show / hide the entire overlay (default; see `hide_all_hotkey`) |

The **boss panel**, **checks panel** and the **extended** layout section are mutually exclusive: opening one closes the others.

If something goes wrong, check `logs/er_injector.log` and `logs/er_overlay.log` in the same folder.

### 3. Customize your dashboard (layout editor)

The release zip includes a **visual editor** — no TOML syntax to learn.

![Layout editor](docs/layout-editor.png)

1. Open **`layout_editor.html`** from the extracted folder in your browser (Chrome, Edge, Firefox…).
   - If import/export is blocked, serve the folder instead: open a terminal in the folder and run `python -m http.server`, then go to `http://localhost:8000/layout_editor.html`.
2. **Drag** metrics, labels, and items from the left palette onto the grid.
3. Tune the grid (columns, rows, cell size, gap) and each tile in the right panel.
4. Use **Import layout file** to edit the bundled `layouts/dashboard.toml`, or start from **New**.
5. Click **Export layout file** and save the `.toml` into the `layouts/` folder (e.g. `layouts/my_run.toml`).
6. Edit `er_overlay.toml` and set `layout_file = "layouts/my_run.toml"`. The overlay reloads the file automatically within ~2 seconds (even while the game is running).

**Tips:** create multiple **sections** in one file (e.g. a compact view and a full view) and switch between them with `F8`. See [Customizing the display](#customizing-the-display) for what each tile type does.

### 4. Tweak appearance and behavior

Open `er_overlay.toml` in any text editor. Common options:

- `anchor`, `offset_x`, `offset_y` — position on screen
- `scale`, `text_size`, `icon_size` — size
- `background_opacity`, `gray_tint` — look of unowned items
- `boss_panel_visible`, `boss_panel_hotkey`, `boss_locale` — boss checklist
- `checks_panel_visible`, `checks_panel_hotkey`, `checks_panel_scope` — checks panel
- `regulation_path` — point at a randomizer `regulation.bin` to track moved loot; see [Checks panel](#checks-panel-randomizer-aware)
- `[challenge]` — optional **challenge mode** (PB / failed runs); see [Challenge mode](#challenge-mode)

Full reference: [Configuration](#configuration-er_overlaytoml).

---

## Warnings

- **Offline only** — no multiplayer / online support.
- **Does not bypass EAC** — launch the game without EasyAntiCheat (e.g. run `eldenring.exe` directly with `steam_appid.txt`).
- **Read-only** — no memory writes, this is not a trainer.
- **Transparent, documented injection** (`LoadLibraryW` via `CreateRemoteThread`), no stealth.

## Installation

### From a GitHub release (recommended)

See **[Quick start](#quick-start-github-release)** above. Requirements:

- Windows **x64**
- Elden Ring **offline**, version supported by the release (currently **2.6.2.0 (WW)** and **2.6.2.1 (JP)** — see [Troubleshooting](#troubleshooting) if values show `---`)

### Build from source

For developers who want to compile locally:

- Windows **x64**
- An Elden Ring version supported by [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) (`eldenring` 0.14, e.g. 2.6.x)
- Rust **1.85+**

```powershell
cd Overlay
cargo build --release
```

Artifacts in `target/release/`:

- `er_overlay_injector.exe` — the injector
- `er_overlay.dll` — the overlay itself

The build copies `er_overlay.toml`, `layouts/`, `tables/<lang>/bosses.toml`, `tables/<lang>/checks.toml` and `assets/icons/` next to the binaries. To produce a release-style zip locally: `.\tools\bundle_release.ps1`.

The randomizer helper (`companion/er_checks_extractor`) is a separate .NET project, published self-contained:

```powershell
dotnet publish companion/er_checks_extractor/er_checks_extractor.csproj -c Release
```

Copy the resulting `er_checks_extractor.exe` to `companion/` next to the DLL (or point `checks_extractor_path` at it).

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

## Configuration (`er_overlay.toml`)

Read next to the DLL, **hot-reloaded every 2 seconds** (you can edit it while the game runs). Out-of-range values are clamped to their default with a warning in the log.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `layout_file` | path | `layouts/dashboard.toml` | Layout file to display (see [Customizing the display](#customizing-the-display)). |
| `default_layout_section` | string | — | Section shown at startup (overrides the layout's own `default_section`). |
| `layout_section_hotkey` | string | — | Key to cycle through sections, e.g. `"F8"`, `"Ctrl+Shift+F1"`. |
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
| `boss_panel_hotkey` | string | `F7` | Toggle the boss checklist panel. |
| `boss_panel_scope` | enum | `current-region` | `current-region` or `all-regions`. |
| `boss_panel_visible` | bool | `true` | Show the boss panel at startup (the bundled `er_overlay.toml` ships `false`). At most one of boss / checks shows at startup; boss wins if both are `true`. |
| `boss_panel_layout` | string | — | Panel `x,y,width,height` (pixels or `%`). Omit or `auto` = `"-5, 10, 25%, 92%"` (right-aligned), shifted below the minimalist HUD. Negative x/y = offset from right/bottom edge. |
| `boss_locale` | string | `auto` | Boss table language (`en`, `fr`, …). `auto` reads the game language via Steam; falls back to `en`. |
| `checks_panel_hotkey` | string | `F6` | Toggle the checks panel (boss + loot checklist). |
| `checks_panel_scope` | enum | `current-region` | `current-region` or `all-regions` (the bundled `er_overlay.toml` ships `all-regions`). |
| `checks_panel_visible` | bool | `false` | Show the checks panel at startup. Mutually exclusive with the boss panel (boss wins if both are `true`). |
| `checks_panel_layout` | string | — | Panel `x,y,width,height` (pixels or `%`). Omit or `auto` = `"5, 10, 25%, 92%"` (left-aligned, mirrors the boss panel). |
| `regulation_path` | path | — | Path to the `regulation.bin` the game **loads** (your randomizer / ModEngine mod). Enables per-seed resolution of randomized loot flags. Empty/omitted = vanilla flags. See [Checks panel](#checks-panel-randomizer-aware). |
| `checks_extractor_path` | path | — | Override the helper exe location. Omit to auto-find `companion/er_checks_extractor.exe` (then `er_checks_extractor.exe`) next to the DLL. |

### Challenge mode (`[challenge]`)

Optional ruleset inspired by [EROverlay](https://github.com/soarqin/EROverlay) boss challenge mode. **Disabled by default.**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | bool | `false` | When `false`, challenge metrics show `---` and no progress is tracked. |
| `max_deaths` | u32 | `0` | Deaths allowed **per run** (inclusive). The run fails when run deaths exceed this value. `0` = deathless. |
| `start_flag` | u32 | `101` | Event flag that marks the **start of a run** (flag `101` = left the Cave of Knowledge / Stranded Graveyard, same as EROverlay). |

Example:

```toml
[challenge]
enabled = true
max_deaths = 0      # deathless: one death ends the run
start_flag = 101
```

**Progress file:** `challenge_state.toml` (next to `er_overlay.dll`, created at runtime). Stores personal best (`pb`), failed run count (`nbtries` / `tries`), and internal run state. Delete this file to reset PB and tries.

See [Challenge mode](#challenge-mode) for behaviour and layout tiles.

## Checks panel (randomizer-aware)

The **checks panel** is a single checklist of everything worth completing in a run. A *check* is one thing to do: a **boss to kill** or an **important item to grab**. Think of it as the boss panel, but it also lists key loot — and it can follow the **item randomizer**.

### How to use it (the basics)

1. Start Elden Ring and run the overlay (see [Quick start](#quick-start-github-release)).
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
- If your seed puts an item with **no tracking flag** on a randomized spot, that line is greyed out and labelled **"Untraceable this seed"**. This is normal, not a bug — the game simply gives the overlay nothing to watch for that pickup.
- To go back to vanilla tracking, empty or remove `regulation_path` and save.

## Challenge mode

Track a **personal best** (most bosses killed on a run within your death budget) and how many times the run **failed**, without editing game saves.

### Metrics

Add these to a layout as `kind = "metric"` tiles (the bundled `layouts/dashboard.toml` includes a **`challenge`** section with both):

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

### Notes

- Boss count uses the same 207-boss table as the main `bosses` metric (save-wide kill flags).
- Challenge updates are paused during loading screens / when in-game time is not running (same idea as EROverlay), so respawn fades do not corrupt run state.
- Compatible with `boss_panel_scope` and the rest of the HUD; challenge is independent of the boss checklist panel.

## Customizing the display

What gets shown is driven entirely by the **layout file** (`layout_file`), not by the code. A layout is a **grid** of tiles; each tile occupies one or more cells.

Three tile kinds:

| Kind | Shows |
|------|-------|
| `metric` | A counter or time: IGT, deaths, NG+, bosses killed, challenge **PB** / **TRIES**, group progress, item quantity. |
| `item` | A tracked item (icon **in color** if owned, **greyed out** otherwise; quantity for consumables). Optional `track_equipped = true` adds a **green border** while the item is equipped. |
| `label` | Plain decorative text (heading, separator). |

### Sections

A layout can contain multiple **sections**; only one is visible at a time. Switch between them with `layout_section_hotkey`. Handy for keeping a "minimalist" view and a "full" view on the same key.

Two ways to write a layout:

- **Simple layout**: a flat list of `[[tile]]` entries (forms a single `"default"` section).
- **Multi-section layout**: `[[section]]` blocks (each with a `name`) containing `[[section.tile]]` entries.

The full syntax is in [Layout format](#layout-format-reference). An invalid layout (overlapping tiles, grid overflow, empty section…) is **rejected on load** and reported in the log.

Provided layout: `layouts/dashboard.toml` (three sections: `minimalist`, `extended`, and `challenge` with `pb` / `nbtries`).

## Layout editor

The release zip includes **`layout_editor.html`** at the root (with `layout_editor_assets/`). When building from source, the same files live under `tools/layout_editor/`.

See **[Quick start § 3](#3-customize-your-dashboard-layout-editor)** for the step-by-step workflow. In short: open the HTML file in a browser, drag tiles onto the grid, export a `.toml` into `layouts/`, then set `layout_file` in `er_overlay.toml`.

**Developers:** the item palette is generated from `goods.toml`; after edits run `python tools/goods/gen_catalog.py` (see [Goods toolkit](tools/goods/README.md)).

## Troubleshooting

| Problem | Hint |
|---------|------|
| Injector: "process not found" | Launch Elden Ring first. |
| Injection fails | EAC is active → run the game offline; try running the injector as administrator. |
| "LoadLibraryW returned NULL" | DLL missing / missing dependency / wrong architecture — check the DLL path. |
| All values show `---` | Game version unsupported — check `logs/er_overlay.log` for `Unsupported game executable` or set `show_debug = true`. Supported builds: **2.6.2.0 (WW), 2.6.2.1 (JP)** (`eldenring` 0.14). |
| Game crashes on inject | Check `logs/er_overlay.log`: last line before crash pinpoints the step (`Hudhook::apply`, `build_view_model`, etc.). Update the game if the log says unsupported executable. |
| No icons (only dots) | PNGs missing from `assets/icons` — see [Icons](#icons). |
| Overlay crash | Conflict with another DX12 hook (RTSS, etc.). |
| Challenge metrics always `---` | Set `[challenge] enabled = true` in `er_overlay.toml`. |
| PB / tries look wrong after testing | Delete `challenge_state.toml` next to the DLL and retry on a clean run. |
| Randomized ground loot not tracked | Set `regulation_path` to the `regulation.bin` the game loads; check `logs/er_overlay.log` for the extractor result and that `checks_flags.toml` was written. |
| Checks header has no `[seed]` tag | No seed mapping active — `regulation_path` is unset/wrong, or `er_checks_extractor.exe` is missing next to the DLL. |

### Logs and diagnostics

All runtime output goes to **`logs/`** next to `er_overlay.dll`:

| File | Contents |
|------|----------|
| `er_overlay.log` | DLL init, game version probe, Hudhook, pointer resolution, errors |
| `er_injector.log` | Process lookup, EAC warning, injection result |

Enable **`show_debug = true`** in `er_overlay.toml` for an in-game window (backend, game exe version, resolved pointers).

Verbose logging: set env `RUST_LOG=debug` before launching the injector.

Supported game builds are logged at startup (`Game executable supported` vs `Unsupported game executable`).

---

# Technical reference

## Architecture

A Cargo workspace of 5 crates:

| Crate | Role |
|-------|------|
| `er_overlay_common` | TOML config, layout format, hotkeys, logging, shared types. |
| `er_game_state` | Game reads via **fromsoftware-rs** (`GameDataMan`, `CSEventFlagMan`, `WorldChrMan`) + data tables. `GameStateSource` trait (live impl + testable mock). |
| `er_overlay_ui` | View model + ImGui rendering (tiles, icons, text). |
| `er_overlay_dll` | Injected DLL, DX12 hook via [hudhook](https://github.com/veeenu/hudhook). |
| `er_overlay_injector` | Documented `LoadLibraryW` injector. |

Loop: `er_overlay_dll` polls `er_game_state` (throttled to ~250 ms), builds an `OverlayViewModel`, and `er_overlay_ui` renders it according to the active layout.

## Layout format (reference)

```toml
[grid]
columns = 8          # max placement width (validation)
unit_size = 64       # side of one square cell, in px
gap = 4              # spacing between cells
border_radius = 6
window_padding = 8

[style]
border_default  = [100, 100, 110, 200]  # RGBA
border_complete = [60, 200, 90, 255]     # border when a metric is "complete"
tile_bg         = [12, 12, 18, 180]
label_scale = 0.65   # label size relative to text
value_scale = 1.15   # value size relative to text

default_section = "minimalist"   # optional
```

Then either a flat list of tiles:

```toml
[[tile]]
kind = "metric"
metric = "igt"
col = 0
row = 0
w = 2       # alias of col_span
h = 1       # alias of row_span
label = "IGT"
```

…or sections:

```toml
[[section]]
name = "minimalist"

[[section.tile]]
kind = "label"
col = 0
row = 0
w = 2
h = 1
label = "RUN"
```

**Fields per tile kind** (all: `col`, `row`, `w`/`col_span`, `h`/`row_span`, optional `id`):

- `metric`: `metric` (metric id), `label`, `show_max` (bool, shows `N/total`), `icon` (optional PNG key shown above the text).
- `item`: `key` (a good key from `goods.toml`). Colored icon if owned, greyed out otherwise, quantity for consumables. Optional `track_equipped = true` adds a green border highlight while the item is equipped (talismans, great runes, quick-slot consumables).
- `label`: `label` (text).

**Validation rules**: `columns > 0`, spans `> 0`, no overlapping tiles *within the same section*, `col + col_span ≤ columns`, unique and non-empty section names, non-empty sections. The file is re-validated on every reload (every 2 s).

## Available metrics

The `metric` field of a `metric` tile accepts:

| Metric | Meaning |
|--------|---------|
| `igt` | In-game time (`HH:MM:SS`). |
| `deaths` | Death count. |
| `ng_cycle` | New Game cycle (`NG+N`). |
| `bosses` | Bosses killed out of 207. |
| `pb` | Challenge personal best (requires `[challenge] enabled = true`). |
| `nbtries` | Challenge failed run count (`tries` in EROverlay; same aliases: `tries`, `challenge_pb`, `challenge_tries`). |
| `scadutree_blessing` | Scadutree Blessing level spent at Sites of Grace (`N/20`). Distinct from the `scadutree` good key (fragment inventory count). |
| *group name* | `owned/total` progress of an aggregate group from `goods.toml` (e.g. `great_runes`). |
| *good key* | Quantity (consumable `count = true`) or `0/1` owned state for a unique item. |

Any unknown key renders `---` (unavailable).

## Game data (tables)

### Bosses — `tables/<lang>/bosses.toml`

One complete boss table per language (`tables/en/bosses.toml`, `tables/fr/bosses.toml`, …): 207 entries (165 base + 42 Shadow of the Erdtree), regions, display order, flags, icons. Copied next to the DLL at build time. **Hot-reloaded** when the file changes (same 2 s poll as `er_overlay.toml`); if the locale file is missing, falls back to `tables/en/bosses.toml` (embedded in the DLL). Set `boss_locale = "auto"` to match the in-game language, or override with `fr`. Regenerate a locale with `python tools/gen_boss_locale_toml.py fr` (from `en/bosses.toml` + ER_boss_checklist_R JSON).

### Checks — `tables/<lang>/checks.toml`

The checklist behind the [checks panel](#checks-panel-randomizer-aware). One `[[check]]` per entry; each declares whether it is `dynamic` (randomizer-sensitive ground loot) or not. Embedded in the DLL (`en`) and copied next to it at build time; hot-reloaded like the boss table.

| Field | Required | Description |
|-------|:--------:|-------------|
| `region` | yes | Region the check belongs to (groups the panel). |
| `name` | yes | Display name (boss or item). |
| `place` | — | Location hint (shown as a tooltip). |
| `dlc` | — | `true` to tag the entry `[DLC]`. |
| `dynamic` | yes | `false` = fixed `flag`. `true` = randomizer-sensitive ground loot resolved per seed. |
| `flag` | for static | Event flag checked when `dynamic = false`. |
| `vanilla_flag` | for dynamic | Vanilla acquisition flag; used as fallback when no seed mapping is loaded. |
| `lot_id`, `lot_param` | for dynamic | Stable `ItemLotParam` row id (`map` or `enemy`) used to look up the current flag in a randomizer regulation. |

When `regulation_path` is set, the companion writes a `checks_flags.toml` (`lot_id → current flag` + regulation hash) that the overlay hot-reloads to resolve dynamic checks for the active seed.

### Goods — `crates/er_game_state/tables/goods.toml`

One `[[good]]` row per tracked item. Fields:

| Field | Required | Description |
|-------|:--------:|-------------|
| `key` | yes | Unique id (and default PNG name `{key}.png`). |
| `item_id` | yes | The item's `param_id` (`EquipParamGoods` or `EquipParamAccessory`). |
| `name` | — | Display name. |
| `category` | — | `goods` (default) or `accessory` (talismans). Avoids `param_id` collisions between categories. |
| `count` | — | `true` = stackable consumable → shows the inventory quantity. |
| `max` | — | Display cap for a counter (e.g. scadutree → `N/50`). |
| `pickup_flag` | — | Ownership event flag (fallback when the item is no longer in inventory). |
| `file` | — | Custom PNG name. |
| `icon_id` | — | Used only by the icon-fetching scripts. |

**Aggregate groups**: declared via a `[groups.<name>]` table listing `members` (good keys). The overlay then exposes a `<name>` metric = number of owned members / total. Example:

```toml
[groups.great_runes]
members = ["godrick_rune", "radahn_rune", "morgott_rune", "rykard_rune", "mohg_rune", "malenia_rune"]
```

Talismans (category `accessory`) live in a delimited block (`# --- talismans ---` … `# --- end talismans ---`).

**Adding a new good**: see **[`tools/goods/README.md`](tools/goods/README.md)**.

### Adding a good

Full checklist: **[`tools/goods/README.md`](tools/goods/README.md)**.

```powershell
# after editing goods.toml
python tools/goods/fetch_goods_icons.py --out assets/icons
python tools/goods/gen_catalog.py
cargo test -p er_game_state
```

## Icons

Tiles can display real in-game icons (PNG) instead of colored dots.

Place PNG files in `assets/icons/`, one per good, named after its `key` (e.g. `godrick_rune.png`) or the good's `file` field. Keep `use_item_icons = true` (default) in `er_overlay.toml`. Any missing icon falls back to a colored dot.

PNGs are **gitignored** (`assets/icons/*.png`). When deploying, copy `assets/icons/` next to `er_overlay.dll`.

Generate missing PNGs with `python tools/goods/fetch_goods_icons.py --out assets/icons` (see [`tools/goods/README.md`](tools/goods/README.md)).

## Development

```powershell
cargo test --workspace      # tests
cargo clippy --workspace    # lints
cargo fmt --all             # formatting
```

CI (`.github/workflows/ci.yml`) runs `fmt --check`, `clippy -D warnings` and `test` on every push/PR.

`er_game_state` exposes a `mock` feature (`MockGameState`) for testing the UI without the game.

## References

- [EROverlay](https://github.com/soarqin/EROverlay) — boss overlay; challenge mode semantics reference
- [hudhook](https://github.com/veeenu/hudhook) — DX12 + ImGui hook
- [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) — game structure access
- [SoulSplitter](https://github.com/FrankvdStam/SoulSplitter) — flags / IGT reference
- [SmithBox](https://github.com/vawser/Smithbox) - icons / flags

## License

**GNU Affero General Public License v3.0 (AGPL-3.0-only)** — see [`LICENSE`](LICENSE).

This is a **strong copyleft** license. In short: anyone who distributes this software, a modified version, or a derivative work — **including merely making it available over a network** — must release the complete corresponding source code under the same AGPL-3.0 license. In other words: if you reuse this code, your project must stay open source.
