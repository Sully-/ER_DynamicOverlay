# Layout Editor

Visual layout editor for the Elden Ring overlay. A static web app — no dependencies, no build step.

## Usage

See the "Layout editor" section of the [main README](../../README.md). In short:

1. Open `index.html` in a browser (or serve the folder: `python -m http.server`).
2. Drag palette items onto the grid and set their properties.
3. **Export TOML**, then place the file in `layouts/` and reference it via `layout_file` in `er_overlay.toml`.

## Files

| File | Role |
|------|------|
| `index.html` | Page structure. |
| `app.js` | Logic: grid, drag & drop, properties, TOML import/export. |
| `style.css` | Styles. |
| `catalog.js` | Item palette **generated** from `goods.toml` (`window.LAYOUT_CATALOG`). Do not edit by hand. |
| `gen_catalog.py` | Regenerates `catalog.js`. |

## Regenerating the palette

After any change to `crates/er_game_state/tables/goods.toml` (new item, talismans, etc.):

```powershell
python tools/layout-editor/gen_catalog.py
```

The script reads `goods.toml`, classifies each good (runes / consumables / talismans / key items) and rewrites `catalog.js`.
