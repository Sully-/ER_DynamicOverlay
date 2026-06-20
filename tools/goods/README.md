# Adding a tracked good

Checklist to add a new item to the overlay (tracking + icon + layout editor palette).
No Rust changes are required for a standard good.

## Checklist

- [ ] Look up `item_id`, `icon_id`, and (if needed) `pickup_flag`
- [ ] Add a `[[good]]` row in `crates/er_game_state/tables/goods.toml`
- [ ] Fetch or export `{key}.png` into `assets/icons/`
- [ ] Regenerate the layout-editor palette (`catalog.js`)
- [ ] (Optional) Add a tile in a layout file or via the layout editor
- [ ] Run `cargo test -p er_game_state` (duplicate-key guard)
- [ ] Rebuild if you run from `target/` (`cargo build` copies icons next to the DLL)

---

## 1. Look up IDs

| Field | Source |
|-------|--------|
| `item_id` | Row ID in `EquipParamGoods` (consumables, keys, runes…) or `EquipParamAccessory` (talismans). Smithbox param tables or `Documentation/ER/Icon List - Goods.txt`. |
| `icon_id` | Same tables (`iconId` field). Base-game goods are also in Smithbox `Documentation/ER/Icon List - Goods.txt` (column 3). |
| `pickup_flag` | Event flag for permanent ownership when the item leaves the inventory (keys, quest items). Often found in [ER_boss_checklist_R](https://github.com/…) `bosses_dlc.json` / `item_ids.json`, or Smithbox flag lists. Omit for stackable consumables (`count = true`). |

**Verify DLC / Shadow of the Erdtree IDs** against your installed `regulation.bin`:

```powershell
cd tools\GetGoodsIconIds
dotnet build
dotnet run -- lookup "G:\Steam\steamapps\common\ELDEN RING\Game" <item_id>
# → prints: <item_id>,<icon_id>
```

---

## 2. Edit `goods.toml`

File: `crates/er_game_state/tables/goods.toml`

Minimal unique item (owned / not owned):

```toml
[[good]]
key = "drawing_room_key"
item_id = 8134
name = "Drawing-Room Key"
pickup_flag = 400072
icon_id = 3062
```

Stackable consumable (shows inventory quantity):

```toml
[[good]]
key = "stonesword_key"
count = true
item_id = 8000
name = "Stonesword Key"
icon_id = 228
```

Talisman (use `category = "accessory"`, place inside the `# --- talismans ---` block):

```toml
[[good]]
category = "accessory"
key = "crimson_amber_medallion"
item_id = 1000
name = "Crimson Amber Medallion"
icon_id = 18000
```

### Where to insert the row

| Item type | Section in `goods.toml` |
|-----------|-------------------------|
| Talisman | Between `# --- talismans ---` and `# --- end talismans ---` |
| Story / dungeon key | After `# Story / dungeon keys` (required for layout-editor category `key_items`) |
| Other consumable | With similar items (e.g. smithing stones, kindling) |

### Field reference

| Field | Required | Notes |
|-------|:--------:|-------|
| `key` | yes | Unique snake_case id; default PNG name is `{key}.png`. |
| `item_id` | yes | Param row id (`EquipParamGoods` or `EquipParamAccessory`). |
| `name` | recommended | Display name (layout editor palette). |
| `icon_id` | recommended | Used by icon scripts only; not read at runtime. |
| `category` | talismans | `"accessory"` for talismans; default `"goods"`. Avoids param id collisions. |
| `count` | stackables | `true` → metric shows inventory quantity. |
| `max` | optional | Display cap, e.g. scadutree `max = 50`. |
| `pickup_flag` | unique items | Event flag when item is consumed / lost from inventory. |
| `file` | optional | Override PNG filename. |

**Aggregate groups** (e.g. `great_runes`): add the key to a `[groups.<name>]` `members` list — no Rust change.

---

## 3. Icon PNG (`assets/icons/{key}.png`)

PNGs are gitignored; generate them locally.

### Automatic fetch (try this first)

```powershell
python tools/goods/fetch_goods_icons.py --out assets/icons
```

Sources tried in order:

1. Loose PNGs in the local game folder (`ELDEN_RING_GAME` or common Steam paths)
2. [elden-ring-media](https://github.com/elden-ring-playground/elden-ring-media) (base game)
3. [EldenRing-SaveForge](https://github.com/oisis/EldenRing-SaveForge) (some DLC talismans / goods)
4. Local fallbacks `_sf_kindling.png`, `_sf_scadu.png`

Expected output: `Done: N unique file(s), 0 missing`.

### Manual export from game files (DLC / missing icons)

When the fetch script reports `miss`, extract from menu textures with `tools/goods/GetGoodsIconIds/`:

```powershell
$game = "G:\Steam\steamapps\common\ELDEN RING\Game"
$bin  = "tools\GetGoodsIconIds\bin\Debug\net9.0"

cd tools\GetGoodsIconIds
dotnet build

Copy-Item "$game\oo2core_6_win64.dll" $bin -Force   # required once per build output
cd $bin

# optional: find subtexture name
.\GetGoodsIconIds.exe probe $game <icon_id>

# export PNG
.\GetGoodsIconIds.exe export $game <icon_id> "..\..\..\..\assets\icons\<key>.png"
```

> **Note:** Oodle must be loaded from the tool's output directory (`GetCurrentDirectory`), so run the `.exe` from `bin\Debug\net9.0\`, not via `dotnet run` from another cwd.

---

## 4. Layout editor palette

Regenerate `catalog.js` after every `goods.toml` change:

```powershell
python tools/goods/gen_catalog.py
```

This updates `tools/layout_editor/layout_editor_assets/catalog.js` (200+ items). Keys under `# Story / dungeon keys` appear in the **Key items** palette category.

Open the editor: `tools/layout_editor/layout_editor.html` (see [layout editor README](layout_editor/layout_editor_assets/README.md)).

---

## 5. Show it on the HUD (optional)

Tracking alone does not add a dashboard tile. Either:

- Open the **layout editor**, drag the item onto the grid, export TOML, set `layout_file` in `er_overlay.toml`, or
- Edit a layout file directly, e.g. `layouts/dashboard.toml`:

```toml
[[sections.main.tiles]]
kind = "item"
key = "drawing_room_key"
col = 0
row = 4
```

Metrics: use the good `key` for quantity / owned state, or a `[groups.*]` name for aggregate progress (`great_runes`, etc.).

---

## 6. Verify

```powershell
cargo test -p er_game_state          # duplicate key test in tables.rs
python tools/goods/fetch_goods_icons.py  # 0 missing
cargo build                          # copies assets/icons/ next to the DLL
```

---

## Tool map

| Path | Role |
|------|------|
| `crates/er_game_state/tables/goods.toml` | **Source of truth** — tracked items, embedded in the DLL at compile time. |
| `tools/goods/fetch_goods_icons.py` | Download / copy icon PNGs into `assets/icons/`. |
| `tools/goods/GetGoodsIconIds/` | C# helper: `lookup` (icon_id from regulation), `export` (PNG from game menu atlases), `probe` (debug). |
| `tools/goods/gen_catalog.py` | Regenerates layout-editor `catalog.js` from `goods.toml`. |
| `assets/icons/` | Runtime PNG folder (gitignored). Copied to `target/*/assets/icons/` on build. |
| `layouts/*.toml` | HUD tile placement (optional). |

---

## Example: story key (base game)

```powershell
# 1. IDs from Smithbox Icon List - Goods.txt: item 8134 → icon 3062, flag from checklist
# 2. Edit goods.toml under "# Story / dungeon keys"
# 3.
python tools/goods/fetch_goods_icons.py --out assets/icons
python tools/goods/gen_catalog.py
cargo test -p er_game_state
```

## Example: DLC key item (Shadow of the Erdtree)

Same as above; if fetch misses:

```powershell
cd tools\GetGoodsIconIds && dotnet build
$game = "G:\Steam\steamapps\common\ELDEN RING\Game"
Copy-Item "$game\oo2core_6_win64.dll" bin\Debug\net9.0\ -Force
cd bin\Debug\net9.0
.\GetGoodsIconIds.exe export $game 3806 "..\..\..\..\assets\icons\hole_laden_necklace.png"
```
