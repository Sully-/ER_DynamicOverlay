# Layout Editor

Visual layout editor for the Elden Ring overlay. A static web app — no dependencies, no build step.

## Usage

See the "Layout editor" section of the [main README](../../../README.md). In short:

1. Open `../layout_editor.html` in a browser (or serve the parent folder: `python -m http.server` from `tools/layout_editor/`).
2. Drag palette items onto the grid and set their properties (including **track_equipped** on item tiles to highlight equipped talismans / runes / quick-slot goods).
3. **Export layout file**, then place it in `layouts/` and reference it via `layout_file` in `er_overlay.toml`.

## Files

| File | Role |
|------|------|
| `../layout_editor.html` | Page entry point. |
| `app.js` | Logic: grid, drag & drop, properties, layout import/export. |
| `i18n.js` | EN/FR UI strings. |
| `style.css` | Styles. |
| `catalog.js` | Item palette **generated** from `goods.toml`. Do not edit by hand. |

## Regenerating the palette

After any change to `goods.toml`, run from the repo root:

```powershell
python tools/goods/gen_catalog.py
```

See [`tools/goods/README.md`](../../goods/README.md) for the full goods workflow.
