# Layout Editor

Visual layout editor for the Elden Ring overlay. A static web app — no dependencies, no build step.

## Usage

See the "Layout editor" section of the [main README](../../../README.md). In short:

1. Open `../layout_editor.html` in a browser (or serve the parent folder: `python -m http.server` from `tools/layout_editor/`).
2. Drag palette items onto the grid and set their properties.
3. **Export layout file**, then place it in `layouts/` and reference it via `layout_file` in `er_overlay.toml`.

## Files

| File | Role |
|------|------|
| `../layout_editor.html` | Page entry point. |
| `app.js` | Logic: grid, drag & drop, properties, layout import/export. |
| `i18n.js` | EN/FR UI strings. |
| `style.css` | Styles. |
| `catalog.js` | Item palette **generated** from `goods.toml` (`window.LAYOUT_CATALOG`). Do not edit by hand. |
| `gen_catalog.py` | Regenerates `catalog.js`. |

## Regenerating the palette

After any change to `crates/er_game_state/tables/goods.toml` (new item, talismans, etc.):

```powershell
python tools/layout_editor/layout_editor_assets/gen_catalog.py
```

The script reads `goods.toml`, classifies each good (runes / consumables / talismans / key items) and rewrites `catalog.js`.
