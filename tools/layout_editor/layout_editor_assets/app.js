// ── Constants ─────────────────────────────────────────────────────

const METRICS = [
  { id: "igt", label: "IGT", showMax: false },
  { id: "deaths", label: "DEATHS", showMax: false },
  { id: "ng_cycle", label: "NG", showMax: false },
  { id: "bosses", label: "BOSS", showMax: true, defaultMax: 165 },
  { id: "great_runes", label: "RUNES", showMax: true, defaultMax: 7 },
  { id: "kindling", label: "KINDLING", showMax: true, icon: "kindling", defaultMax: 8 },
  { id: "scadutree", label: "SHARDS", showMax: true, icon: "scadutree", defaultMax: 20 },
  { id: "scadutree_blessing", label: "BLESSING", showMax: true, icon: "scadutree", defaultMax: 20 },
  { id: "pb", label: "PB", showMax: false },
  { id: "nbtries", label: "TRIES", showMax: false },
];

const DEFAULT_STYLE = {
  border_default: [100, 100, 110, 200],
  border_complete: [60, 200, 90, 255],
  tile_bg: [12, 12, 18, 180],
  window_bg: [12, 12, 18, 166],
  window_border: true,
  label_scale: 0.65,
  value_scale: 1.15,
};

/** Dev: tools/layout_editor/ → ../../assets/icons/; release zip root → assets/icons/ */
const ICON_BASE = (() => {
  const pageDir = new URL(".", location.href);
  const path = decodeURIComponent(pageDir.pathname).replace(/\\/g, "/").toLowerCase();
  const rel = path.includes("/tools/layout_editor/") ? "../../assets/icons/" : "assets/icons/";
  return new URL(rel, pageDir).href;
})();

const PREVIEW_METRICS = {
  igt: "1:23:45",
  deaths: "42",
  ng_cycle: "NG+2",
  bosses: "12/165",
  great_runes: "6/7",
  kindling: "7/8",
  scadutree: "12/20",
  scadutree_blessing: "12/20",
  pb: "42",
  nbtries: "7",
};

const PREVIEW_ITEM_COUNT = "3";

const OVERLAY_CONFIG_URL = (() => {
  const pageDir = new URL(".", location.href);
  const path = decodeURIComponent(pageDir.pathname).replace(/\\/g, "/").toLowerCase();
  const rel = path.includes("/tools/layout_editor/") ? "../../er_overlay.toml" : "er_overlay.toml";
  return new URL(rel, pageDir).href;
})();

const SECTION_NAMES = ["minimalist", "extended"];

const ITEM_CATEGORY_IDS = ["runes", "key_items", "talismans", "consumables"];

function categoryLabel(id) {
  return t(`cat_${id}`);
}

function defaultSectionGrid(name) {
  return {
    gridCols: 8,
    gridRows: name === "minimalist" ? 1 : 3,
  };
}

function inferSectionGrid(tiles) {
  let gridCols = 1;
  let gridRows = 1;
  for (const t of tiles) {
    gridCols = Math.max(gridCols, t.col + t.w);
    gridRows = Math.max(gridRows, t.row + t.h);
  }
  return { gridCols, gridRows };
}

/** Editor grid size on import — never inflate to defaultSectionGrid when tiles are smaller. */
function sectionGridOnImport(name, found, tiles) {
  const inferred = inferSectionGrid(tiles);
  const editorCols = found?.editor_cols;
  const editorRows = found?.editor_rows;
  if (typeof editorCols === "number" || typeof editorRows === "number") {
    return {
      gridCols: Math.max(inferred.gridCols, editorCols ?? inferred.gridCols),
      gridRows: Math.max(inferred.gridRows, editorRows ?? inferred.gridRows),
    };
  }
  if (tiles.length === 0) {
    return defaultSectionGrid(name);
  }
  return inferred;
}

function createDefaultState() {
  return {
    grid: { columns: 8, unit_size: 64, gap: 4, border_radius: 6, window_padding: 8 },
    style: { ...DEFAULT_STYLE },
    overlay: { text_size: 18, scale: 1 },
    default_section: "minimalist",
    sections: SECTION_NAMES.map((name) => ({ name, tiles: [], ...defaultSectionGrid(name) })),
    activeSection: 0,
  };
}

let nextId = 1;
let catalog = [];
let catalogByKey = new Map();
let state = createDefaultState();
let selectedTileIds = new Set();
let clipboardTiles = null;
let dragState = null;
let dragMoveRaf = null;
let dragMovePending = null;
let canvasSelectionBound = false;
let sectionTabsBuilt = false;
const gridTileNodes = new Map();
let gridBgCacheKey = null;
let gridHandlesCacheKey = null;
const textWidthCache = new Map();

function clearSelection() {
  selectedTileIds.clear();
}

function selectOnly(id) {
  selectedTileIds.clear();
  selectedTileIds.add(id);
}

function toggleTileSelection(id) {
  if (selectedTileIds.has(id)) selectedTileIds.delete(id);
  else selectedTileIds.add(id);
}

function primarySelectedId() {
  if (selectedTileIds.size !== 1) return null;
  return selectedTileIds.values().next().value;
}

function tileDataWithoutId(tile) {
  const { _id, ...data } = tile;
  return { ...data };
}

function pointInRect(x, y, rect) {
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}

function clearGridTileNodes() {
  for (const el of gridTileNodes.values()) el.remove();
  gridTileNodes.clear();
  gridBgCacheKey = null;
  gridHandlesCacheKey = null;
}

function resetGridDom() {
  clearGridTileNodes();
  els.gridCanvas.querySelector(".grid-bg")?.remove();
  els.gridCanvas.querySelector(".grid-handles")?.remove();
  els.gridCanvas.querySelector(".grid-tiles")?.remove();
}

function tileContentKey(tile, gm) {
  return [
    tile.kind,
    tile.label,
    tile.metric,
    tile.show_max,
    tile.max_mode,
    tile.max,
    tile.icon,
    tile.key,
    tile.track_equipped,
    tile.w,
    tile.h,
    gm.unit,
    gm.gap,
    gm.preview,
    state.grid.border_radius,
    state.overlay.text_size,
    state.overlay.scale,
    state.style.label_scale,
    state.style.value_scale,
  ].join("\0");
}

function tileGeometryStyle(tile, gm) {
  const left = gm.window_padding + tile.col * (gm.unit + gm.gap);
  const top = gm.window_padding + tile.row * (gm.unit + gm.gap);
  const w = tile.w * gm.unit + (tile.w - 1) * gm.gap;
  const h = tile.h * gm.unit + (tile.h - 1) * gm.gap;
  return { left, top, w, h };
}

function gridContentRectClient() {
  const rect = els.gridCanvas.getBoundingClientRect();
  const gm = gridMetrics();
  return {
    left: rect.left + gm.window_padding,
    top: rect.top + gm.window_padding,
    right: rect.left + gm.w - gm.window_padding,
    bottom: rect.top + gm.h - gm.window_padding,
  };
}

function isDeleteDragIntent(clientX, clientY) {
  const trash = els.trashZone?.getBoundingClientRect();
  if (trash && pointInRect(clientX, clientY, trash)) return true;
  const content = gridContentRectClient();
  return (
    clientX < content.left ||
    clientX > content.right ||
    clientY < content.top ||
    clientY > content.bottom
  );
}

function snapPositionsForDrag() {
  return dragState.snapPositions || dragState.startPositions;
}

function syncFreeDragVisual(clientX, clientY) {
  const gm = gridMetrics();
  const canvasRect = els.gridCanvas.getBoundingClientRect();
  const bases = snapPositionsForDrag();
  const anchorStart = bases.get(dragState.tileId);
  if (!anchorStart) return;

  const anchorLeft = clientX - canvasRect.left - dragState.pointerOffsetX;
  const anchorTop = clientY - canvasRect.top - dragState.pointerOffsetY;
  const anchorGridLeft = gm.window_padding + anchorStart.col * (gm.unit + gm.gap);
  const anchorGridTop = gm.window_padding + anchorStart.row * (gm.unit + gm.gap);
  const dPxX = anchorLeft - anchorGridLeft;
  const dPxY = anchorTop - anchorGridTop;

  for (const id of dragState.tileIds) {
    const start = bases.get(id);
    const t = activeSection().tiles.find((x) => x._id === id);
    const el = gridTileNodes.get(id);
    if (!start || !t || !el) continue;
    const geom = tileGeometryStyle({ ...t, col: start.col, row: start.row }, gm);
    el.style.left = `${geom.left + dPxX}px`;
    el.style.top = `${geom.top + dPxY}px`;
  }
}

function setTilesDeletePendingClass(tileIds, active) {
  for (const id of tileIds) {
    gridTileNodes.get(id)?.classList.toggle("tile--delete-pending", active);
  }
}

function syncTileDomPositions(tileIds = null) {
  const gm = gridMetrics();
  const filter = tileIds ? new Set(tileIds) : null;
  for (const tile of activeSection().tiles) {
    if (filter && !filter.has(tile._id)) continue;
    const el = gridTileNodes.get(tile._id);
    if (!el) continue;
    const { left, top, w, h } = tileGeometryStyle(tile, gm);
    el.style.left = `${left}px`;
    el.style.top = `${top}px`;
    el.style.width = `${w}px`;
    el.style.height = `${h}px`;
  }
}

function setTilesMovingClass(tileIds, active) {
  for (const id of tileIds) {
    gridTileNodes.get(id)?.classList.toggle("tile--moving", active);
  }
}

function syncSelectionDom() {
  for (const tile of activeSection().tiles) {
    const el = gridTileNodes.get(tile._id);
    if (!el) continue;
    el.classList.toggle("selected", selectedTileIds.has(tile._id));
    syncResizeHandles(el, tile._id, selectedTileIds.size === 1 && selectedTileIds.has(tile._id));
  }
  renderProperties();
}

/** Full render when selection shape changes (resize handles); otherwise DOM-only sync. */
function updateSelectionAfterChange(prevSize) {
  const nextSize = selectedTileIds.size;
  if (prevSize === 1 || nextSize === 1) {
    render();
    return;
  }
  syncSelectionDom();
}

function activeSection() {
  return state.sections[state.activeSection];
}

function uid() {
  return `t${nextId++}`;
}

// ── DOM refs ────────────────────────────────────────────────────────

const $ = (sel) => document.querySelector(sel);

const els = {
  paletteMetrics: $("#palette-metrics"),
  paletteLabel: $("#palette-label"),
  paletteItemSections: $("#palette-item-sections"),
  catalogCount: $("#catalog-count"),
  gridCanvas: $("#grid-canvas"),
  gridInfo: $("#grid-info"),
  sectionTabs: $("#section-tabs"),
  trashZone: $("#trash-zone"),
  propsEmpty: $("#props-empty"),
  propsTile: $("#props-tile"),
  propsMultiHint: $("#props-multi-hint"),
  propsFields: $("#props-fields"),
  propKind: $("#prop-kind"),
  propLabel: $("#prop-label"),
  propMetric: $("#prop-metric"),
  propKey: $("#prop-key"),
  propCol: $("#prop-col"),
  propRow: $("#prop-row"),
  propW: $("#prop-w"),
  propH: $("#prop-h"),
  propShowMax: $("#prop-show-max"),
  propMaxMode: $("#prop-max-mode"),
  propMaxValue: $("#prop-max-value"),
  propTrackEquipped: $("#prop-track-equipped"),
  propIcon: $("#prop-icon"),
  fieldLabel: $("#field-label"),
  fieldMetric: $("#field-metric"),
  fieldKey: $("#field-key"),
  fieldShowMax: $("#field-show-max"),
  fieldMetricMax: $("#field-metric-max"),
  fieldTrackEquipped: $("#field-track-equipped"),
  fieldIcon: $("#field-icon"),
  cfgColumns: $("#cfg-columns"),
  cfgRows: $("#cfg-rows"),
  cfgUnitSize: $("#cfg-unit-size"),
  cfgGap: $("#cfg-gap"),
  cfgPadding: $("#cfg-padding"),
  cfgDefaultSection: $("#cfg-default-section"),
  cfgTextSize: $("#cfg-text-size"),
  cfgOverlayScale: $("#cfg-overlay-scale"),
  cfgWindowBorder: $("#cfg-window-border"),
};

const STYLE_COLOR_KEYS = ["window_bg", "tile_bg", "border_default", "border_complete"];

const STYLE_PICKER_IDS = {
  window_bg: "picker-window-bg",
  tile_bg: "picker-tile-bg",
  border_default: "picker-border-default",
  border_complete: "picker-border-complete",
};

/** @type {Record<string, { syncFromState: () => void, setOpacityLabel: (s: string) => void }>} */
const stylePickers = {};

// ── TOML parse (local, no CDN) ──────────────────────────────────────

function parseTomlValue(raw) {
  const v = raw.trim();
  if (v.startsWith('"') && v.endsWith('"')) return v.slice(1, -1).replace(/\\"/g, '"');
  if (v === "true") return true;
  if (v === "false") return false;
  if (v.startsWith("[") && v.endsWith("]")) {
    const inner = v.slice(1, -1).trim();
    if (!inner) return [];
    return inner.split(",").map((x) => {
      const t = x.trim();
      const n = Number(t);
      return Number.isFinite(n) ? n : parseTomlValue(t);
    });
  }
  const n = Number(v);
  return Number.isFinite(n) ? n : v;
}

function parseLayoutToml(text) {
  const out = { grid: {}, style: {} };
  let table = null;
  let section = null;
  let tile = null;

  const applyKv = (key, value) => {
    if (tile) return void (tile[key] = value);
    if (section) return void (section[key] = value);
    if (table === "grid") return void (out.grid[key] = value);
    if (table === "style") return void (out.style[key] = value);
    out[key] = value;
  };

  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;

    if (trimmed === "[[section]]") {
      table = null;
      tile = null;
      section = { name: "", tile: [] };
      if (!out.section) out.section = [];
      out.section.push(section);
      continue;
    }
    if (trimmed === "[[section.tile]]") {
      table = null;
      if (!section) {
        section = { name: "", tile: [] };
        if (!out.section) out.section = [];
        out.section.push(section);
      }
      tile = {};
      section.tile.push(tile);
      continue;
    }
    if (trimmed.startsWith("[") && trimmed.endsWith("]") && !trimmed.startsWith("[[")) {
      table = trimmed.slice(1, -1).trim();
      section = null;
      tile = null;
      continue;
    }

    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    applyKv(trimmed.slice(0, eq).trim(), parseTomlValue(trimmed.slice(eq + 1)));
  }

  return out;
}

function parseOverlayToml(text) {
  const out = {};
  for (const line of text.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#") || trimmed.startsWith("[")) continue;
    const eq = trimmed.indexOf("=");
    if (eq === -1) continue;
    const key = trimmed.slice(0, eq).trim();
    if (key === "text_size" || key === "scale") {
      out[key] = parseTomlValue(trimmed.slice(eq + 1));
    }
  }
  return out;
}

async function loadOverlayConfig() {
  try {
    const resp = await fetch(OVERLAY_CONFIG_URL);
    if (!resp.ok) return;
    const parsed = parseOverlayToml(await resp.text());
    if (parsed.text_size != null) state.overlay.text_size = Number(parsed.text_size) || 18;
    if (parsed.scale != null) state.overlay.scale = Number(parsed.scale) || 1;
  } catch {
    /* file:// or missing config — keep defaults */
  }
}

// ── Init ────────────────────────────────────────────────────────────

function rebuildLocalizedPalette() {
  els.paletteLabel.innerHTML = "";
  els.paletteLabel.appendChild(
    makePaletteEl("label", t("labelPalette"), { label: t("defaultTitle") })
  );
  buildItemSections();
}

function init() {
  catalog = Array.isArray(window.LAYOUT_CATALOG) ? window.LAYOUT_CATALOG : [];
  catalogByKey = new Map(catalog.map((e) => [e.key, e]));
  els.catalogCount.textContent = catalog.length;

  window.onLocaleChange = () => {
    rebuildLocalizedPalette();
    syncStylePickers();
    render();
  };

  buildPalette();
  initStylePickers();
  bindEvents();
  syncConfigInputs();
  applyThemeFromState();
  loadOverlayConfig().then(() => {
    syncConfigInputs();
    applyThemeFromState();
    render();
  });
  render();
}

// ── Icons & preview ─────────────────────────────────────────────────

function iconUrl(iconKey) {
  if (!iconKey) return null;
  return `${ICON_BASE}${encodeURIComponent(iconKey)}.png`;
}

function makeIconImg(iconKey, className, alt = "") {
  const img = document.createElement("img");
  img.className = className;
  img.alt = alt;
  img.draggable = false;
  img.loading = "lazy";
  const url = iconUrl(iconKey);
  if (!url) return img;
  img.src = url;
  img.addEventListener("error", () => img.classList.add("tile-icon--missing"));
  return img;
}

function catalogEntry(key) {
  return catalogByKey.get(key);
}

function itemsInCategory(categoryId) {
  return catalog.filter((item) => item.category === categoryId);
}

function filterItems(items, query) {
  const q = query.trim().toLowerCase();
  if (!q) return items;
  return items.filter(
    (item) =>
      item.key.toLowerCase().includes(q) || item.name.toLowerCase().includes(q)
  );
}

function itemIconKey(key) {
  return catalogEntry(key)?.iconKey || key;
}

function itemIsCountable(key) {
  return catalogEntry(key)?.countable ?? false;
}

function rgbaCss(c) {
  return ColorPicker.rgbaCss(c);
}

function clampByte(n) {
  return ColorPicker.clampByte(n);
}

function styleRgba(key) {
  return state.style[key] || DEFAULT_STYLE[key] || [0, 0, 0, 255];
}

function setStyleRgba(key, rgba) {
  state.style[key] = rgba.map(clampByte);
}

function initStylePickers() {
  for (const key of STYLE_COLOR_KEYS) {
    const mount = document.getElementById(STYLE_PICKER_IDS[key]);
    if (!mount) continue;
    stylePickers[key] = ColorPicker.mountRgbaColorPicker(mount, {
      getRgba: () => styleRgba(key),
      setRgba: (rgba) => setStyleRgba(key, rgba),
      onChange: () => applyLivePreview(),
      opacityLabel: t("opacity"),
    });
  }
}

function syncStylePickers() {
  for (const picker of Object.values(stylePickers)) {
    picker.syncFromState();
    picker.setOpacityLabel(t("opacity"));
  }
}

function windowBgCss(style) {
  const c = style.window_bg || DEFAULT_STYLE.window_bg;
  if ((c[3] ?? 255) === 0) return "transparent";
  return rgbaCss(c);
}

function applyLivePreview() {
  const s = state.style;
  const winBg = windowBgCss(s);
  const tileBg = rgbaCss(s.tile_bg || DEFAULT_STYLE.tile_bg);
  const borderDef = rgbaCss(s.border_default || DEFAULT_STYLE.border_default);
  const borderComp = rgbaCss(s.border_complete || DEFAULT_STYLE.border_complete);
  const root = document.documentElement.style;

  root.setProperty("--window-bg-color", winBg);
  root.setProperty("--tile-bg-color", tileBg);
  root.setProperty("--tile-border-color", borderDef);
  root.setProperty("--tile-border-complete", borderComp);
  const scales = overlayFontScales();
  root.setProperty("--overlay-label-line", `${overlayFontPx(scales.label)}px`);

  const canvas = els.gridCanvas;
  if (!canvas) return;

  canvas.style.background = winBg;
  canvas.style.border = "none";
  canvas.style.boxShadow =
    s.window_border !== false ? `inset 0 0 0 1.5px ${borderDef}` : "none";
  canvas.classList.toggle("grid-canvas--no-window-chrome", s.window_border === false && (s.window_bg?.[3] ?? 255) === 0);

  for (const tile of canvas.querySelectorAll(".tile")) {
    tile.style.background = tileBg;
    if (tile.classList.contains("tile--complete")) {
      tile.style.borderColor = borderComp;
    } else if (
      !tile.classList.contains("selected") &&
      !tile.classList.contains("overlap")
    ) {
      tile.style.borderColor = borderDef;
    }
  }
}

function applyThemeFromState() {
  applyLivePreview();
}

function overlayTextSize() {
  return Math.max(12, Math.min(48, Number(state.overlay?.text_size) || 18));
}

function overlayPreviewScale() {
  return Math.max(0.25, Math.min(4, Number(state.overlay?.scale) || 1));
}

let _textProbe = null;

function measureTextWidth(text, fontScale) {
  const key = `${text}\0${fontScale}\0${overlayTextSize()}`;
  if (textWidthCache.has(key)) return textWidthCache.get(key);
  if (!_textProbe) {
    _textProbe = document.createElement("span");
    _textProbe.className = "tile-text-probe";
    document.body.appendChild(_textProbe);
  }
  _textProbe.style.fontSize = `${overlayFontPx(fontScale)}px`;
  _textProbe.textContent = text;
  const width = _textProbe.getBoundingClientRect().width;
  textWidthCache.set(key, width);
  return width;
}

/** Window font scale — TTF is already at text_size px; only overlay scale applies. */
function overlayBaseFontScale() {
  return Math.max(0.5, overlayPreviewScale());
}

function overlayFontScales() {
  const base = overlayBaseFontScale();
  return {
    label: base * state.style.label_scale,
    value: base * state.style.value_scale,
  };
}

function overlayFontPx(fontScale) {
  return overlayTextSize() * fontScale;
}

function overlayLineHeight(fontScale) {
  return overlayFontPx(fontScale);
}

/** Same logic as tile_render.rs fit_font_scale (linear shrink to max width). */
function fitFontScale(text, maxWidth, scale) {
  const MIN_SCALE = 0.32;
  if (!text || maxWidth <= 0) return scale;
  const width = measureTextWidth(text, scale);
  if (width <= maxWidth || width <= 0) return scale;
  return Math.max(MIN_SCALE, scale * (maxWidth / width));
}

function maxTextWidth(pxW) {
  const padX = pxW * 0.1;
  return Math.max(8, pxW - padX * 2);
}

function appendTileText(body, text, top, fontScale, className, maxW) {
  const el = document.createElement("div");
  el.className = `tile-layer ${className}`;
  el.style.top = `${top}px`;
  el.style.fontSize = `${overlayFontPx(fontScale)}px`;
  el.style.maxWidth = `${maxW}px`;
  el.textContent = text;
  body.appendChild(el);
  return overlayLineHeight(fontScale);
}

function appendTileIcon(body, iconKey, className, left, top, size, alt = "") {
  const icon = makeIconImg(iconKey, className, alt);
  icon.classList.add("tile-layer", "tile-layer--icon");
  icon.style.left = `${left}px`;
  icon.style.top = `${top}px`;
  icon.style.width = `${size}px`;
  icon.style.height = `${size}px`;
  body.appendChild(icon);
}

function metricHasMax(metricId) {
  const m = METRICS.find((x) => x.id === metricId);
  return m?.showMax ?? false;
}

function defaultMaxForMetric(metricId) {
  const m = METRICS.find((x) => x.id === metricId);
  if (m?.defaultMax) return m.defaultMax;
  const preview = PREVIEW_METRICS[metricId];
  if (preview?.includes("/")) return Number(preview.split("/")[1]) || 1;
  return 1;
}

function tileMaxOverride(tile) {
  if (tile.max_mode === "manual" || typeof tile.max === "number") {
    return tile.max ?? tile.max_value ?? defaultMaxForMetric(tile.metric);
  }
  return null;
}

function metricPreview(metric, showMax, maxOverride = null) {
  let text = PREVIEW_METRICS[metric] ?? "---";
  let complete = false;
  if (text.includes("/") || maxOverride != null) {
    const parts = text.includes("/") ? text.split("/") : [text, String(defaultMaxForMetric(metric))];
    const cur = parts[0];
    const max =
      maxOverride != null ? String(maxOverride) : String(defaultMaxForMetric(metric));
    text = showMax ? `${cur}/${max}` : cur;
    complete = cur === max && max !== "0";
  }
  return { text, complete };
}

function fillPaletteThumb(thumb, kind, data) {
  thumb.innerHTML = "";
  const thumbW = 34;
  const scales = overlayFontScales();
  const maxW = maxTextWidth(thumbW);

  if (kind === "metric") {
    const m = METRICS.find((x) => x.id === (data.id || data.metric)) || data;
    const preview = metricPreview(
      m.id || m.metric,
      m.showMax ?? data.showMax,
      data.max_mode === "manual" || typeof data.max === "number"
        ? data.max ?? data.max_value ?? defaultMaxForMetric(m.id || m.metric)
        : null
    );
    if (m.icon) {
      const img = makeIconImg(m.icon, "tile-icon");
      img.style.width = "82%";
      img.style.maxHeight = "58%";
      thumb.appendChild(img);
    } else {
      const lbl = document.createElement("span");
      lbl.className = "palette-thumb-text";
      lbl.style.fontSize = `${overlayFontPx(scales.label)}px`;
      lbl.textContent = (m.label || m.id).slice(0, 8);
      thumb.appendChild(lbl);
    }
    const val = document.createElement("span");
    val.className = "palette-thumb-value";
    const fitted = fitFontScale(preview.text, maxW, scales.value);
    val.style.fontSize = `${overlayFontPx(fitted)}px`;
    val.textContent = preview.text;
    thumb.appendChild(val);
    return;
  }
  if (kind === "label") {
    const val = document.createElement("span");
    val.className = "palette-thumb-value";
    const fitted = fitFontScale(data.label || t("defaultTitle"), maxW, scales.value);
    val.style.fontSize = `${overlayFontPx(fitted)}px`;
    val.textContent = data.label || t("defaultTitle");
    thumb.appendChild(val);
    return;
  }
  if (kind === "item") {
    const key = data.key;
    const countable = itemIsCountable(key);
    const img = makeIconImg(itemIconKey(key), `tile-icon ${countable ? "tile-icon--item-countable" : "tile-icon--item"}`);
    thumb.appendChild(img);
    if (countable) {
      const val = document.createElement("span");
      val.className = "palette-thumb-value";
      const countScale = fitFontScale(PREVIEW_ITEM_COUNT, thumbW * 0.9, scales.value * 0.85);
      val.style.fontSize = `${overlayFontPx(countScale)}px`;
      val.textContent = PREVIEW_ITEM_COUNT;
      thumb.appendChild(val);
    }
  }
}

function metricIconSize(pxW, pxH, hasLabel, valueH) {
  const minDim = Math.min(pxW, pxH);
  if (!hasLabel) {
    const verticalPad = minDim * 0.06;
    const valueGap = valueH * 0.25;
    const maxFromHeight = pxH - valueH - valueGap - verticalPad * 2;
    return Math.min(minDim, maxFromHeight, minDim * 0.72);
  }
  return minDim * 0.38;
}

function fillTileBody(body, tile, pxW, pxH) {
  body.innerHTML = "";
  const scales = overlayFontScales();
  const maxW = maxTextWidth(pxW);

  if (tile.kind === "label") {
    if (!tile.label) return;
    const labelText = tile.label;
    const fitted = fitFontScale(labelText, maxW, scales.value);
    const textH = overlayLineHeight(fitted);
    appendTileText(body, labelText, (pxH - textH) * 0.5, fitted, "tile-value tile-value--solo", maxW);
    return;
  }

  if (tile.kind === "metric") {
    const preview = metricPreview(tile.metric, tile.show_max, tileMaxOverride(tile));
    const hasLabel = !!tile.label;
    const labelText = tile.label || "";
    const labelScale = scales.label;
    const valueScale = fitFontScale(preview.text, maxW, scales.value);
    const labelH = overlayLineHeight(labelScale);
    const valueH = overlayLineHeight(valueScale);
    const hasIcon = !!(tile.icon && iconUrl(tile.icon));
    const iconSize = hasIcon ? metricIconSize(pxW, pxH, hasLabel, valueH) : 0;
    const blockH = hasIcon
      ? hasLabel
        ? iconSize + labelH * 0.35 + labelH + valueH
        : iconSize + valueH * 0.25 + valueH
      : hasLabel
        ? labelH + valueH * 1.1
        : valueH;
    let y = (pxH - blockH) * 0.5;

    if (hasIcon) {
      appendTileIcon(
        body,
        tile.icon,
        "tile-icon tile-icon--metric",
        (pxW - iconSize) * 0.5,
        y,
        iconSize,
        tile.label
      );
      y += iconSize + (hasLabel ? labelH * 0.25 : valueH * 0.25);
    }

    if (hasLabel) {
      y += appendTileText(body, labelText, y, labelScale, "tile-label", maxW);
    }
    appendTileText(body, preview.text, y, valueScale, "tile-value", maxW);
    return { complete: preview.complete };
  }

  if (tile.kind === "item") {
    const countable = itemIsCountable(tile.key);
    const iconSize = Math.min(pxW, pxH) * (countable ? 0.58 : 0.78);
    const iy = countable ? pxH * 0.12 : (pxH - iconSize) * 0.5;
    appendTileIcon(
      body,
      itemIconKey(tile.key),
      `tile-icon ${countable ? "tile-icon--item-countable" : "tile-icon--item"}`,
      (pxW - iconSize) * 0.5,
      iy,
      iconSize,
      tile.key
    );
    if (countable) {
      const countScale = fitFontScale(PREVIEW_ITEM_COUNT, pxW * 0.9, scales.value * 0.85);
      appendTileText(
        body,
        PREVIEW_ITEM_COUNT,
        pxH - countScale * 16 - 2,
        countScale,
        "tile-value tile-count",
        pxW * 0.9
      );
    }
    return tile.track_equipped ? { complete: true } : {};
  }
  return {};
}

function buildPalette() {
  els.paletteMetrics.innerHTML = "";
  for (const m of METRICS) {
    els.paletteMetrics.appendChild(makePaletteEl("metric", m.label, m));
  }

  els.paletteLabel.innerHTML = "";
  els.paletteLabel.appendChild(
    makePaletteEl("label", t("labelPalette"), { label: t("defaultTitle") })
  );

  buildItemSections();

  for (const m of METRICS) {
    const opt = document.createElement("option");
    opt.value = m.id;
    opt.textContent = m.id;
    els.propMetric.appendChild(opt);
  }
}

function makePaletteEl(kind, title, data, subtitle = "") {
  const el = document.createElement("div");
  el.className = "palette-item";
  el.draggable = true;
  el.dataset.kind = kind;

  const thumb = document.createElement("div");
  thumb.className = "palette-thumb";
  fillPaletteThumb(thumb, kind, data);
  el.appendChild(thumb);

  const meta = document.createElement("div");
  meta.className = "palette-meta";
  const titleEl = document.createElement("div");
  titleEl.className = "palette-title";
  titleEl.textContent = title;
  meta.appendChild(titleEl);
  if (subtitle) {
    const sub = document.createElement("div");
    sub.className = "pi-key";
    sub.textContent = subtitle;
    meta.appendChild(sub);
  }
  el.appendChild(meta);

  el.addEventListener("dragstart", (e) => {
    e.dataTransfer.setData("application/x-tile", JSON.stringify({ kind, ...data }));
    e.dataTransfer.effectAllowed = "copy";
  });
  return el;
}

function buildItemSections() {
  els.paletteItemSections.innerHTML = "";
  for (const catId of ITEM_CATEGORY_IDS) {
    const cat = { id: catId, label: categoryLabel(catId) };
    const section = document.createElement("div");
    const all = itemsInCategory(cat.id);
    section.className = "item-section collapsed";
    section.dataset.category = cat.id;

    const head = document.createElement("button");
    head.type = "button";
    head.className = "item-section-head";
    head.innerHTML = `<span>${cat.label}</span><span class="item-section-count">${all.length}</span>`;
    head.addEventListener("click", () => {
      const opening = section.classList.contains("collapsed");
      section.classList.toggle("collapsed");
      if (opening) renderItemSection(cat.id);
    });

    const content = document.createElement("div");
    content.className = "item-section-content";

    const search = document.createElement("input");
    search.type = "search";
    search.className = "input item-section-search";
    search.placeholder = t("searchCategory", { category: cat.label.toLowerCase() });
    search.addEventListener("input", () => renderItemSection(cat.id));
    search.addEventListener("click", (e) => e.stopPropagation());

    const body = document.createElement("div");
    body.className = "palette palette-scroll item-section-body";

    content.append(search, body);
    section.append(head, content);
    els.paletteItemSections.appendChild(section);
  }
}

function fillPaletteContainer(container, items) {
  container.innerHTML = "";
  for (const item of items) {
    container.appendChild(
      makePaletteEl("item", item.name, { key: item.key, name: item.name }, item.key)
    );
  }
}

function renderItemSection(categoryId) {
  const section = els.paletteItemSections.querySelector(
    `[data-category="${categoryId}"]`
  );
  if (!section) return;

  const all = itemsInCategory(categoryId);
  const query = section.querySelector(".item-section-search")?.value ?? "";
  const items = filterItems(all, query);
  const countEl = section.querySelector(".item-section-count");
  if (countEl) {
    countEl.textContent = query.trim()
      ? `${items.length}/${all.length}`
      : String(all.length);
  }
  fillPaletteContainer(section.querySelector(".item-section-body"), items);
}

function renderItemPalette() {
  for (const catId of ITEM_CATEGORY_IDS) renderItemSection(catId);
}

// ── Render ──────────────────────────────────────────────────────────

function render(options = {}) {
  const opts = {
    preview: true,
    tabs: true,
    grid: true,
    props: true,
    config: true,
    info: true,
    ...options,
  };
  if (opts.preview) applyLivePreview();
  if (opts.tabs) syncSectionTabs();
  if (opts.grid) renderGrid();
  if (opts.props) renderProperties();
  if (opts.config) syncConfigInputs();
  if (opts.info) updateGridInfo();
}

function ensureSectionTabs() {
  if (sectionTabsBuilt) return;
  sectionTabsBuilt = true;
  els.sectionTabs.innerHTML = "";
  state.sections.forEach((sec, i) => {
    const tab = document.createElement("button");
    tab.className = "section-tab";
    tab.type = "button";
    tab.textContent = sec.name;
    tab.addEventListener("click", () => {
      state.activeSection = i;
      clearSelection();
      clearGridTileNodes();
      render();
    });
    els.sectionTabs.appendChild(tab);
  });
}

function syncSectionTabs() {
  ensureSectionTabs();
  els.sectionTabs.querySelectorAll(".section-tab").forEach((tab, i) => {
    tab.classList.toggle("active", i === state.activeSection);
  });
}

function minGridBounds() {
  let minCol = 1;
  let minRow = 1;
  for (const t of activeSection().tiles) {
    minCol = Math.max(minCol, t.col + t.w);
    minRow = Math.max(minRow, t.row + t.h);
  }
  return { minCol, minRow };
}

function gridMetrics() {
  const sec = activeSection();
  const preview = overlayPreviewScale();
  const { unit_size, gap, window_padding } = state.grid;
  const unit = unit_size * preview;
  const gapPx = gap * preview;
  const pad = window_padding * preview;
  const { minCol, minRow } = minGridBounds();
  const cols = Math.max(sec.gridCols ?? 8, minCol);
  const rows = Math.max(sec.gridRows ?? 1, minRow);
  const w = pad * 2 + cols * unit + (cols - 1) * gapPx;
  const h = pad * 2 + rows * unit + (rows - 1) * gapPx;
  return { columns: cols, unit, gap: gapPx, window_padding: pad, rows, w, h, preview };
}

function gridLayoutCacheKey(gm) {
  return `${gm.columns}x${gm.rows}@${gm.unit},${gm.gap},${gm.window_padding},${gm.preview},${gm.w},${gm.h}`;
}

function ensureGridTilesLayer(canvas) {
  let layer = canvas.querySelector(".grid-tiles");
  if (!layer) {
    layer = document.createElement("div");
    layer.className = "grid-tiles";
    canvas.appendChild(layer);
  }
  return layer;
}

function rebuildGridBackground(canvas, gm) {
  const key = gridLayoutCacheKey(gm);
  if (gridBgCacheKey === key) return;
  gridBgCacheKey = key;
  canvas.querySelector(".grid-bg")?.remove();
  const bg = document.createElement("div");
  bg.className = "grid-bg";
  for (let r = 0; r < gm.rows; r++) {
    for (let c = 0; c < gm.columns; c++) {
      const cell = document.createElement("div");
      cell.className = "grid-cell";
      cell.style.left = `${gm.window_padding + c * (gm.unit + gm.gap)}px`;
      cell.style.top = `${gm.window_padding + r * (gm.unit + gm.gap)}px`;
      cell.style.width = `${gm.unit}px`;
      cell.style.height = `${gm.unit}px`;
      bg.appendChild(cell);
    }
  }
  canvas.insertBefore(bg, canvas.firstChild);
}

function rebuildGridHandles(canvas, gm) {
  const key = gridLayoutCacheKey(gm);
  let handles = canvas.querySelector(".grid-handles");
  if (!handles) {
    handles = document.createElement("div");
    handles.className = "grid-handles";
    canvas.appendChild(handles);
  }
  if (gridHandlesCacheKey === key) return;
  gridHandlesCacheKey = key;
  handles.innerHTML = "";
  const mk = (cls, axis, title) => {
    const el = document.createElement("div");
    el.className = `grid-resize-handle ${cls}`;
    el.title = title;
    el.addEventListener("mousedown", (e) => startGridResize(e, axis));
    handles.appendChild(el);
    return el;
  };
  mk("grid-resize-e", "e", t("resizeWiderCols"));
  mk("grid-resize-s", "s", t("resizeWiderRows"));
  mk("grid-resize-se", "se", t("resizeGrid"));
  if (dragState?.mode === "grid") canvas.classList.add("grid-canvas--resizing");
}

function syncResizeHandles(el, tileId, show) {
  el.querySelectorAll(".resize-zone, .resize-handle").forEach((node) => node.remove());
  if (!show) return;
  const zone = document.createElement("div");
  zone.className = "resize-zone";
  zone.title = t("resize");
  zone.addEventListener("mousedown", (e) => startResize(e, tileId));
  el.appendChild(zone);
  const handle = document.createElement("div");
  handle.className = "resize-handle";
  handle.title = t("resize");
  handle.addEventListener("mousedown", (e) => startResize(e, tileId));
  el.appendChild(handle);
}

function updateTileEl(el, tile, gm, overlap) {
  const { left, top, w, h } = tileGeometryStyle(tile, gm);
  el.style.left = `${left}px`;
  el.style.top = `${top}px`;
  el.style.width = `${w}px`;
  el.style.height = `${h}px`;
  el.style.borderRadius = `${state.grid.border_radius * gm.preview}px`;
  el.classList.toggle("selected", selectedTileIds.has(tile._id));
  el.classList.toggle("overlap", overlap);
  el.classList.toggle(
    "tile--resizing",
    dragState?.mode === "resize" && dragState.tileId === tile._id
  );
  el.classList.toggle(
    "tile--moving",
    dragState?.mode === "move" && dragState.tileIdSet?.has(tile._id)
  );
  syncResizeHandles(el, tile._id, selectedTileIds.size === 1 && selectedTileIds.has(tile._id));
}

function updateOverlapClasses(overlaps) {
  for (const tile of activeSection().tiles) {
    const el = gridTileNodes.get(tile._id);
    el?.classList.toggle("overlap", overlaps.has(tile._id));
  }
}

function renderGrid() {
  const gm = gridMetrics();
  const canvas = els.gridCanvas;
  canvas.style.width = `${gm.w}px`;
  canvas.style.height = `${gm.h}px`;

  rebuildGridBackground(canvas, gm);
  rebuildGridHandles(canvas, gm);
  const tilesLayer = ensureGridTilesLayer(canvas);

  const overlaps = findOverlaps(activeSection().tiles);
  const liveIds = new Set();

  for (const tile of activeSection().tiles) {
    liveIds.add(tile._id);
    const contentKey = tileContentKey(tile, gm);
    let el = gridTileNodes.get(tile._id);
    if (!el || el.dataset.contentKey !== contentKey) {
      el?.remove();
      el = renderTileEl(tile, gm, overlaps.has(tile._id));
      el.dataset.contentKey = contentKey;
      tilesLayer.appendChild(el);
      gridTileNodes.set(tile._id, el);
    } else {
      updateTileEl(el, tile, gm, overlaps.has(tile._id));
    }
  }

  for (const [id, el] of gridTileNodes) {
    if (!liveIds.has(id)) {
      el.remove();
      gridTileNodes.delete(id);
    }
  }
}

function bindCanvasSelection() {
  if (canvasSelectionBound) return;
  canvasSelectionBound = true;
  els.gridCanvas.addEventListener("mousedown", (e) => {
    if (dragState) return;
    if (
      e.target === els.gridCanvas ||
      e.target.classList.contains("grid-bg") ||
      e.target.classList.contains("grid-cell") ||
      e.target.classList.contains("grid-tiles")
    ) {
      if (!e.ctrlKey && !e.metaKey) {
        clearSelection();
        render({ tabs: false, config: false });
      }
    }
  });
}

function renderTileEl(tile, gm, overlap) {
  const el = document.createElement("div");
  el.className = "tile";
  el.dataset.id = tile._id;
  if (selectedTileIds.has(tile._id)) el.classList.add("selected");
  if (overlap) el.classList.add("overlap");

  const left = gm.window_padding + tile.col * (gm.unit + gm.gap);
  const top = gm.window_padding + tile.row * (gm.unit + gm.gap);
  const w = tile.w * gm.unit + (tile.w - 1) * gm.gap;
  const h = tile.h * gm.unit + (tile.h - 1) * gm.gap;

  el.style.left = `${left}px`;
  el.style.top = `${top}px`;
  el.style.width = `${w}px`;
  el.style.height = `${h}px`;
  el.style.borderRadius = `${state.grid.border_radius * gm.preview}px`;
  el.style.background = rgbaCss(state.style.tile_bg);

  const body = document.createElement("div");
  body.className = "tile-body";
  const meta = fillTileBody(body, tile, w, h);
  if (meta?.complete) el.classList.add("tile--complete");
  el.style.borderColor = rgbaCss(
    meta?.complete ? state.style.border_complete : state.style.border_default
  );
  el.appendChild(body);

  const singleSelected = selectedTileIds.size === 1 && selectedTileIds.has(tile._id);
  if (singleSelected) {
    const zone = document.createElement("div");
    zone.className = "resize-zone";
    zone.title = t("resize");
    zone.addEventListener("mousedown", (e) => startResize(e, tile._id));
    el.appendChild(zone);

    const handle = document.createElement("div");
    handle.className = "resize-handle";
    handle.title = t("resize");
    handle.addEventListener("mousedown", (e) => startResize(e, tile._id));
    el.appendChild(handle);
  }

  if (dragState?.mode === "resize" && dragState.tileId === tile._id) {
    el.classList.add("tile--resizing");
  }
  if (dragState?.mode === "move" && dragState.tileIdSet?.has(tile._id)) {
    el.classList.add("tile--moving");
  }

  el.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("resize-handle") || e.target.classList.contains("resize-zone")) {
      return;
    }
    e.preventDefault();
    const prevSize = selectedTileIds.size;
    const mod = e.ctrlKey || e.metaKey;
    if (mod) {
      toggleTileSelection(tile._id);
      if (!selectedTileIds.has(tile._id)) {
        updateSelectionAfterChange(prevSize);
        return;
      }
    } else if (!selectedTileIds.has(tile._id)) {
      selectOnly(tile._id);
      updateSelectionAfterChange(prevSize);
    }
    startMove(e, tile._id);
  });

  return el;
}

function findOverlaps(tiles) {
  const ids = new Set();
  for (let i = 0; i < tiles.length; i++) {
    for (let j = i + 1; j < tiles.length; j++) {
      if (tilesOverlap(tiles[i], tiles[j])) {
        ids.add(tiles[i]._id);
        ids.add(tiles[j]._id);
      }
    }
  }
  return ids;
}

function tilesOverlap(a, b) {
  return (
    a.col < b.col + b.w &&
    a.col + a.w > b.col &&
    a.row < b.row + b.h &&
    a.row + a.h > b.row
  );
}

function renderProperties() {
  const count = selectedTileIds.size;
  if (count === 0) {
    els.propsEmpty.classList.remove("hidden");
    els.propsTile.classList.add("hidden");
    return;
  }
  els.propsEmpty.classList.add("hidden");
  els.propsTile.classList.remove("hidden");

  const multi = count > 1;
  els.propsMultiHint.classList.toggle("hidden", !multi);
  els.propsFields.classList.toggle("hidden", multi);
  if (multi) {
    els.propsMultiHint.textContent = t("propertiesMulti", { count });
    return;
  }

  const tile = activeSection().tiles.find((t) => t._id === primarySelectedId());
  if (!tile) {
    els.propsEmpty.classList.remove("hidden");
    els.propsTile.classList.add("hidden");
    clearSelection();
    return;
  }

  els.propKind.value = tile.kind;
  els.fieldLabel.classList.toggle("hidden", tile.kind === "item");
  els.fieldMetric.classList.toggle("hidden", tile.kind !== "metric");
  els.fieldKey.classList.toggle("hidden", tile.kind !== "item");
  els.fieldShowMax.classList.toggle("hidden", tile.kind !== "metric");
  els.fieldMetricMax.classList.toggle(
    "hidden",
    tile.kind !== "metric" || (!metricHasMax(tile.metric) && !tile.show_max)
  );
  els.fieldTrackEquipped.classList.toggle("hidden", tile.kind !== "item");
  els.fieldIcon.classList.toggle("hidden", tile.kind !== "metric");

  els.propLabel.value = tile.label || "";
  els.propMetric.value = tile.metric || "igt";
  els.propKey.value = tile.key || "";
  els.propCol.value = tile.col;
  els.propRow.value = tile.row;
  els.propW.value = tile.w;
  els.propH.value = tile.h;
  els.propShowMax.checked = !!tile.show_max;
  const maxManual = tile.max_mode === "manual" || typeof tile.max === "number";
  els.propMaxMode.value = maxManual ? "manual" : "auto";
  els.propMaxValue.value = maxManual
    ? tile.max ?? tile.max_value ?? defaultMaxForMetric(tile.metric)
    : defaultMaxForMetric(tile.metric);
  els.propMaxValue.disabled = !maxManual;
  els.propTrackEquipped.checked = !!tile.track_equipped;
  els.propIcon.value = tile.icon || "";
}

function syncConfigInputs() {
  const sec = activeSection();
  els.cfgColumns.value = sec.gridCols ?? 8;
  if (els.cfgRows) els.cfgRows.value = sec.gridRows ?? 1;
  els.cfgUnitSize.value = state.grid.unit_size;
  els.cfgGap.value = state.grid.gap;
  els.cfgPadding.value = state.grid.window_padding;
  els.cfgDefaultSection.value = state.default_section;
  if (els.cfgTextSize) els.cfgTextSize.value = state.overlay.text_size;
  if (els.cfgOverlayScale) els.cfgOverlayScale.value = state.overlay.scale;
  syncStylePickers();
  if (els.cfgWindowBorder) {
    els.cfgWindowBorder.checked = state.style.window_border !== false;
  }
}

function updateGridInfo() {
  const gm = gridMetrics();
  const n = activeSection().tiles.length;
  els.gridInfo.textContent = t("gridInfo", {
    cols: gm.columns,
    rows: gm.rows,
    count: tileCountLabel(n),
  });
}

// ── Tile CRUD ───────────────────────────────────────────────────────

function createTile(kind, data, col, row) {
  const base = { _id: uid(), kind, col, row, w: 1, h: 1 };
  if (kind === "label") {
    return { ...base, label: data.label || t("defaultTitle") };
  }
  if (kind === "metric") {
    const m = METRICS.find((x) => x.id === data.id) || data;
    return {
      ...base,
      w: 2,
      metric: m.id || data.metric || "igt",
      label: m.label || data.label || "METRIC",
      show_max: m.showMax ?? data.showMax ?? false,
      max_mode: data.max_mode ?? (typeof data.max === "number" ? "manual" : "auto"),
      max: typeof data.max === "number" ? data.max : undefined,
      icon: m.icon || data.icon || undefined,
    };
  }
  if (kind === "item") {
    return { ...base, key: data.key || "godrick_rune", track_equipped: !!data.track_equipped };
  }
  return base;
}

function deleteSelectedTiles() {
  if (selectedTileIds.size === 0) return;
  const sec = activeSection();
  sec.tiles = sec.tiles.filter((t) => !selectedTileIds.has(t._id));
  clearSelection();
  render({ config: false });
}

function copySelectedTiles() {
  if (selectedTileIds.size === 0) return;
  clipboardTiles = activeSection()
    .tiles.filter((t) => selectedTileIds.has(t._id))
    .map(tileDataWithoutId);
}

function pasteTiles() {
  if (!clipboardTiles?.length) return;
  const sec = activeSection();
  clearSelection();
  for (const data of clipboardTiles) {
    const tile = { ...data, _id: uid(), col: data.col + 1, row: data.row + 1 };
    sec.tiles.push(tile);
    selectedTileIds.add(tile._id);
  }
  render();
}

function applyPropChanges() {
  const tile = activeSection().tiles.find((t) => t._id === primarySelectedId());
  if (!tile) return;
  const gm = gridMetrics();
  const beforeKey = tileContentKey(tile, gm);
  if (tile.kind !== "item") tile.label = els.propLabel.value;
  if (tile.kind === "metric") {
    tile.metric = els.propMetric.value;
    tile.show_max = els.propShowMax.checked;
    if (els.propMaxMode.value === "manual") {
      tile.max_mode = "manual";
      tile.max = Math.max(1, Number(els.propMaxValue.value) || defaultMaxForMetric(tile.metric));
    } else {
      tile.max_mode = "auto";
      delete tile.max;
    }
    els.propMaxValue.disabled = els.propMaxMode.value !== "manual";
    const icon = els.propIcon.value.trim();
    tile.icon = icon || undefined;
    els.fieldMetricMax.classList.toggle(
      "hidden",
      !metricHasMax(tile.metric) && !tile.show_max
    );
  }
  if (tile.kind === "item") {
    tile.key = els.propKey.value.trim() || tile.key;
    tile.track_equipped = els.propTrackEquipped.checked;
  }
  tile.col = Math.max(0, Number(els.propCol.value) || 0);
  tile.row = Math.max(0, Number(els.propRow.value) || 0);
  tile.w = Math.max(1, Number(els.propW.value) || 1);
  tile.h = Math.max(1, Number(els.propH.value) || 1);

  const afterKey = tileContentKey(tile, gm);
  const overlaps = findOverlaps(activeSection().tiles);
  renderProperties();
  updateGridInfo();

  const el = gridTileNodes.get(tile._id);
  if (!el) {
    render({ config: false });
    return;
  }
  if (beforeKey !== afterKey) {
    const next = renderTileEl(tile, gm, overlaps.has(tile._id));
    next.dataset.contentKey = afterKey;
    el.replaceWith(next);
    gridTileNodes.set(tile._id, next);
  } else {
    updateTileEl(el, tile, gm, overlaps.has(tile._id));
  }
  updateOverlapClasses(overlaps);
}

// ── Drag & drop ───────────────────────────────────────────────────

function cellFromPoint(clientX, clientY) {
  const rect = els.gridCanvas.getBoundingClientRect();
  const gm = gridMetrics();
  const x = clientX - rect.left - gm.window_padding;
  const y = clientY - rect.top - gm.window_padding;
  const col = Math.floor(x / (gm.unit + gm.gap));
  const row = Math.floor(y / (gm.unit + gm.gap));
  return {
    col: Math.max(0, Math.min(gm.columns - 1, col)),
    row: Math.max(0, row),
  };
}

function startMove(e, tileId) {
  const sec = activeSection();
  const anchor = sec.tiles.find((t) => t._id === tileId);
  if (!anchor) return;
  const tileIds = selectedTileIds.has(tileId) ? [...selectedTileIds] : [tileId];
  const tileIdSet = new Set(tileIds);
  const startPositions = new Map();
  for (const id of tileIds) {
    const t = sec.tiles.find((x) => x._id === id);
    if (t) startPositions.set(id, { col: t.col, row: t.row });
  }
  const origin = cellFromPoint(e.clientX, e.clientY);
  const gm = gridMetrics();
  const anchorGeom = tileGeometryStyle(anchor, gm);
  const canvasRect = els.gridCanvas.getBoundingClientRect();
  dragState = {
    mode: "move",
    tileId,
    tileIds,
    tileIdSet,
    startPositions,
    snapPositions: null,
    freeDrag: false,
    offsetCol: origin.col - anchor.col,
    offsetRow: origin.row - anchor.row,
    pointerOffsetX: e.clientX - canvasRect.left - anchorGeom.left,
    pointerOffsetY: e.clientY - canvasRect.top - anchorGeom.top,
  };
  setTilesMovingClass(tileIds, true);
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
  updateTrashHighlight(e.clientX, e.clientY);
}

function startResize(e, tileId) {
  e.preventDefault();
  e.stopPropagation();
  selectOnly(tileId);
  dragState = { mode: "resize", tileId };
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
}

function startGridResize(e, axis) {
  e.preventDefault();
  e.stopPropagation();
  clearSelection();
  const sec = activeSection();
  dragState = {
    mode: "grid",
    axis,
    startX: e.clientX,
    startY: e.clientY,
    startColumns: sec.gridCols ?? 8,
    startRows: sec.gridRows ?? 1,
  };
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
  render();
}

function applyResize(tile, cell) {
  const gm = gridMetrics();
  const maxW = Math.max(1, gm.columns - tile.col);
  tile.w = Math.max(1, Math.min(maxW, cell.col - tile.col + 1));
  tile.h = Math.max(1, cell.row - tile.row + 1);
}

function onDragMove(e) {
  if (!dragState) return;
  dragMovePending = { clientX: e.clientX, clientY: e.clientY };
  if (dragMoveRaf) return;
  dragMoveRaf = requestAnimationFrame(() => {
    dragMoveRaf = null;
    const pending = dragMovePending;
    dragMovePending = null;
    if (pending) onDragMoveFrame(pending.clientX, pending.clientY);
  });
}

function onDragMoveFrame(clientX, clientY) {
  if (!dragState) return;

  if (dragState.mode === "grid") {
    const gm = gridMetrics();
    const step = gm.unit + gm.gap;
    const dCol = Math.round((clientX - dragState.startX) / step);
    const dRow = Math.round((clientY - dragState.startY) / step);
    const { minCol, minRow } = minGridBounds();
    const sec = activeSection();
    const prevCol = sec.gridCols ?? 8;
    const prevRow = sec.gridRows ?? 1;

    if (dragState.axis === "e" || dragState.axis === "se") {
      sec.gridCols = Math.max(minCol, Math.min(32, dragState.startColumns + dCol));
    }
    if (dragState.axis === "s" || dragState.axis === "se") {
      sec.gridRows = Math.max(minRow, Math.min(32, dragState.startRows + dRow));
    }
    if (sec.gridCols !== prevCol || sec.gridRows !== prevRow) render();
    return;
  }

  const tile = activeSection().tiles.find((t) => t._id === dragState.tileId);
  if (!tile) return;
  const cell = cellFromPoint(clientX, clientY);
  const prev = { col: tile.col, row: tile.row, w: tile.w, h: tile.h };

  if (dragState.mode === "move") {
    const deleteIntent = isDeleteDragIntent(clientX, clientY);
    updateTrashHighlight(clientX, clientY);

    if (deleteIntent) {
      dragState.freeDrag = true;
      setTilesDeletePendingClass(dragState.tileIds, true);
      syncFreeDragVisual(clientX, clientY);
      return;
    }

    dragState.freeDrag = false;
    setTilesDeletePendingClass(dragState.tileIds, false);

    const anchorStart = dragState.startPositions.get(dragState.tileId);
    if (!anchorStart) return;
    const anchor = activeSection().tiles.find((t) => t._id === dragState.tileId);
    if (!anchor) return;
    const { columns } = gridMetrics();
    const targetCol = Math.max(
      0,
      Math.min(columns - anchor.w, cell.col - dragState.offsetCol)
    );
    const targetRow = Math.max(0, cell.row - dragState.offsetRow);
    const dCol = targetCol - anchorStart.col;
    const dRow = targetRow - anchorStart.row;
    let changed = false;
    if (!dragState.snapPositions) dragState.snapPositions = new Map();
    for (const id of dragState.tileIds) {
      const t = activeSection().tiles.find((x) => x._id === id);
      const start = dragState.startPositions.get(id);
      if (!t || !start) continue;
      const nextCol = Math.max(0, Math.min(columns - t.w, start.col + dCol));
      const nextRow = Math.max(0, start.row + dRow);
      if (t.col !== nextCol || t.row !== nextRow) {
        t.col = nextCol;
        t.row = nextRow;
        changed = true;
      }
      dragState.snapPositions.set(id, { col: t.col, row: t.row });
    }
    if (changed) syncTileDomPositions(dragState.tileIds);
    return;
  }

  applyResize(tile, cell);

  if (
    prev.col !== tile.col ||
    prev.row !== tile.row ||
    prev.w !== tile.w ||
    prev.h !== tile.h
  ) {
    syncTileDomPositions([dragState.tileId]);
  }
}

function shouldDeleteMovedTiles(clientX, clientY) {
  return isDeleteDragIntent(clientX, clientY);
}

function updateTrashHighlight(clientX, clientY) {
  const trash = els.trashZone;
  if (!trash) return;
  const overTrash = dragState?.mode === "move" && pointInRect(clientX, clientY, trash.getBoundingClientRect());
  const deleteIntent = dragState?.mode === "move" && isDeleteDragIntent(clientX, clientY);
  trash.classList.toggle("trash-zone--active", overTrash || deleteIntent);
}

function onDragEnd(e) {
  if (dragMoveRaf) {
    cancelAnimationFrame(dragMoveRaf);
    dragMoveRaf = null;
  }
  if (dragMovePending && dragState) {
    onDragMoveFrame(dragMovePending.clientX, dragMovePending.clientY);
    dragMovePending = null;
  }

  if (dragState?.mode === "move") {
    setTilesMovingClass(dragState.tileIds, false);
    setTilesDeletePendingClass(dragState.tileIds, false);
  }
  if (dragState?.mode === "move" && e && shouldDeleteMovedTiles(e.clientX, e.clientY)) {
    const ids = dragState.tileIdSet ?? new Set(dragState.tileIds);
    const sec = activeSection();
    sec.tiles = sec.tiles.filter((t) => !ids.has(t._id));
    clearSelection();
  }
  dragState = null;
  els.trashZone?.classList.remove("trash-zone--active");
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
  render({ config: false });
}

function setupCanvasDrop() {
  const canvas = els.gridCanvas;

  canvas.addEventListener("dragover", (e) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    canvas.classList.add("drag-over");
  });

  canvas.addEventListener("dragleave", () => {
    canvas.classList.remove("drag-over");
  });

  canvas.addEventListener("drop", (e) => {
    e.preventDefault();
    canvas.classList.remove("drag-over");
    const raw = e.dataTransfer.getData("application/x-tile");
    if (!raw) return;
    const data = JSON.parse(raw);
    const cell = cellFromPoint(e.clientX, e.clientY);
    const tile = createTile(data.kind, data, cell.col, cell.row);
    activeSection().tiles.push(tile);
    selectOnly(tile._id);
    render();
  });
}

// ── TOML export ─────────────────────────────────────────────────────

function rgba(arr) {
  return `[${arr.join(", ")}]`;
}

/** Global column count for Rust — max across sections, editor grid size, and tile bounds. */
function globalGridColumns() {
  let cols = 1;
  for (const sec of state.sections) {
    cols = Math.max(cols, sec.gridCols ?? 1);
    for (const t of sec.tiles) {
      cols = Math.max(cols, t.col + t.w);
    }
  }
  return cols;
}

function collectLayoutExportErrors() {
  const columns = globalGridColumns();
  const errors = [];
  for (const sec of state.sections) {
    for (const t of sec.tiles) {
      const name = t.label || t.key || t.metric || t.kind;
      if (t.col + t.w > columns) {
        errors.push(
          t("exportOverflow", {
            section: sec.name,
            name,
            col: t.col,
            w: t.w,
            columns,
          })
        );
      }
    }
    const overlaps = findOverlaps(sec.tiles);
    if (overlaps.size > 0) {
      errors.push(t("exportOverlap", { section: sec.name }));
    }
  }
  const nonEmpty = state.sections.filter((s) => s.tiles.length > 0);
  if (nonEmpty.length === 0) {
    errors.push(t("exportNoTiles"));
  }
  if (
    state.default_section &&
    !nonEmpty.some((s) => s.name === state.default_section)
  ) {
    errors.push(t("exportDefaultSectionEmpty", { name: state.default_section }));
  }
  return errors;
}

function exportToml() {
  const columns = globalGridColumns();
  const lines = [];
  lines.push(t("exportComment"));
  lines.push("# Square grid units: 1 unit = 1 icon square.");
  lines.push("");
  lines.push("[grid]");
  lines.push(`columns = ${columns}`);
  lines.push(`unit_size = ${state.grid.unit_size}`);
  lines.push(`gap = ${state.grid.gap}`);
  lines.push(`border_radius = ${state.grid.border_radius}`);
  lines.push(`window_padding = ${state.grid.window_padding}`);
  lines.push("");
  lines.push("[style]");
  lines.push(`border_default = ${rgba(state.style.border_default)}`);
  lines.push(`border_complete = ${rgba(state.style.border_complete)}`);
  lines.push(`tile_bg = ${rgba(state.style.tile_bg)}`);
  lines.push(`window_bg = ${rgba(state.style.window_bg || DEFAULT_STYLE.window_bg)}`);
  lines.push(`window_border = ${state.style.window_border !== false}`);
  lines.push(`label_scale = ${state.style.label_scale}`);
  lines.push(`value_scale = ${state.style.value_scale}`);
  lines.push("");
  lines.push(`default_section = "${state.default_section}"`);
  lines.push("");

  for (const section of state.sections) {
    if (section.tiles.length === 0) continue;
    lines.push("[[section]]");
    lines.push(`name = "${section.name}"`);
    if (section.gridCols != null) lines.push(`editor_cols = ${section.gridCols}`);
    if (section.gridRows != null) lines.push(`editor_rows = ${section.gridRows}`);
    lines.push("");
    for (const tile of section.tiles) {
      lines.push("[[section.tile]]");
      lines.push(`kind = "${tile.kind}"`);
      if (tile.kind === "metric") {
        lines.push(`metric = "${tile.metric}"`);
        if (tile.label) lines.push(`label = "${tile.label}"`);
        if (tile.show_max) lines.push("show_max = true");
        if (tile.max_mode === "manual" || typeof tile.max === "number") {
          lines.push(`max = ${tile.max ?? tile.max_value ?? defaultMaxForMetric(tile.metric)}`);
        }
        if (tile.icon) lines.push(`icon = "${tile.icon}"`);
      } else if (tile.kind === "label") {
        if (tile.label) lines.push(`label = "${tile.label}"`);
      } else if (tile.kind === "item") {
        lines.push(`key = "${tile.key}"`);
        if (tile.track_equipped) lines.push("track_equipped = true");
      }
      lines.push(`col = ${tile.col}`);
      lines.push(`row = ${tile.row}`);
      if (tile.w !== 1) lines.push(`w = ${tile.w}`);
      if (tile.h !== 1) lines.push(`h = ${tile.h}`);
      lines.push("");
    }
  }
  return lines.join("\n");
}

function downloadToml() {
  const errors = collectLayoutExportErrors();
  if (errors.length > 0) {
    alert(`${t("exportImpossible")}\n\n${errors.join("\n")}`);
    return;
  }
  const blob = new Blob([exportToml()], { type: "text/plain;charset=utf-8" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = "layout.toml";
  a.click();
  URL.revokeObjectURL(a.href);
}

// ── TOML import ─────────────────────────────────────────────────────

function importToml(text) {
  const raw = parseLayoutToml(text);
  const newState = createDefaultState();

  if (raw.grid) {
    Object.assign(newState.grid, raw.grid);
  }
  if (raw.style) {
    Object.assign(newState.style, raw.style);
    for (const key of STYLE_COLOR_KEYS) {
      if (!newState.style[key]) newState.style[key] = [...DEFAULT_STYLE[key]];
    }
    if (newState.style.window_border == null) newState.style.window_border = DEFAULT_STYLE.window_border;
  }
  if (raw.default_section) {
    newState.default_section = raw.default_section;
  }

  const sections = raw.section || [];
  if (sections.length > 0) {
    newState.sections = SECTION_NAMES.map((name) => {
      const found = sections.find((s) => s.name === name);
      const tiles = (found?.tile || []).map(parseTileDef);
      return {
        name,
        tiles,
        ...sectionGridOnImport(name, found, tiles),
      };
    });
  } else if (raw.tile?.length) {
    newState.sections = [
      { name: "minimalist", tiles: raw.tile.map(parseTileDef) },
      { name: "extended", tiles: [] },
    ];
    newState.default_section = "minimalist";
  }

  state = newState;
  state.grid.columns = globalGridColumns();
  clearSelection();
  resetGridDom();
  applyThemeFromState();
  render();
}

function parseTileDef(t) {
  const base = {
    _id: uid(),
    kind: t.kind,
    col: t.col ?? 0,
    row: t.row ?? 0,
    w: t.w ?? t.col_span ?? 1,
    h: t.h ?? t.row_span ?? 1,
  };
  if (t.kind === "metric") {
    const maxManual = typeof t.max === "number";
    return {
      ...base,
      metric: t.metric,
      label: t.label ?? "",
      show_max: !!t.show_max,
      max_mode: maxManual ? "manual" : t.max === "auto" ? "auto" : "auto",
      max: maxManual ? t.max : undefined,
      icon: t.icon || undefined,
    };
  }
  if (t.kind === "label") {
    return { ...base, label: t.label || "" };
  }
  if (t.kind === "item") {
    return { ...base, key: t.key || "", track_equipped: !!t.track_equipped };
  }
  return base;
}

// ── Events ──────────────────────────────────────────────────────────

function bindEvents() {
  setupCanvasDrop();
  bindCanvasSelection();

  $("#btn-export").addEventListener("click", downloadToml);
  $("#btn-new").addEventListener("click", () => {
    if (state.sections.some((s) => s.tiles.length) && !confirm(t("confirmClear"))) return;
    state = createDefaultState();
    clearSelection();
    resetGridDom();
    render();
  });
  $("#btn-delete-tile").addEventListener("click", deleteSelectedTiles);

  $("#import-file").addEventListener("change", async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    importToml(await file.text());
    e.target.value = "";
  });

  for (const input of [
    els.propLabel,
    els.propMetric,
    els.propKey,
    els.propCol,
    els.propRow,
    els.propW,
    els.propH,
    els.propShowMax,
    els.propMaxMode,
    els.propMaxValue,
    els.propTrackEquipped,
    els.propIcon,
  ]) {
    input.addEventListener("input", applyPropChanges);
    input.addEventListener("change", applyPropChanges);
  }

  els.cfgColumns.addEventListener("change", () => {
    const { minCol } = minGridBounds();
    activeSection().gridCols = Math.max(minCol, Math.min(32, Number(els.cfgColumns.value) || 8));
    render();
  });
  if (els.cfgRows) {
    els.cfgRows.addEventListener("change", () => {
      const { minRow } = minGridBounds();
      activeSection().gridRows = Math.max(minRow, Math.min(32, Number(els.cfgRows.value) || 1));
      render();
    });
  }
  els.cfgUnitSize.addEventListener("change", () => {
    state.grid.unit_size = Math.max(16, Number(els.cfgUnitSize.value) || 64);
    render();
  });
  els.cfgGap.addEventListener("change", () => {
    state.grid.gap = Math.max(0, Number(els.cfgGap.value) || 0);
    render();
  });
  els.cfgPadding.addEventListener("change", () => {
    state.grid.window_padding = Math.max(0, Number(els.cfgPadding.value) || 8);
    render();
  });
  els.cfgDefaultSection.addEventListener("change", () => {
    state.default_section = els.cfgDefaultSection.value;
  });
  if (els.cfgTextSize) {
    els.cfgTextSize.addEventListener("change", () => {
      state.overlay.text_size = Math.max(12, Math.min(48, Number(els.cfgTextSize.value) || 18));
      applyThemeFromState();
      render();
    });
  }
  if (els.cfgOverlayScale) {
    els.cfgOverlayScale.addEventListener("change", () => {
      state.overlay.scale = Math.max(0.25, Math.min(4, Number(els.cfgOverlayScale.value) || 1));
      applyThemeFromState();
      render();
    });
  }
  if (els.cfgWindowBorder) {
    els.cfgWindowBorder.addEventListener("change", () => {
      state.style.window_border = els.cfgWindowBorder.checked;
      applyLivePreview();
    });
  }

  document.addEventListener("keydown", (e) => {
    if (document.activeElement.tagName === "INPUT" || document.activeElement.tagName === "SELECT") {
      return;
    }
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === "c") {
      e.preventDefault();
      copySelectedTiles();
      return;
    }
    if (mod && e.key.toLowerCase() === "v") {
      e.preventDefault();
      pasteTiles();
      return;
    }
    if (mod && e.key.toLowerCase() === "a") {
      e.preventDefault();
      clearSelection();
      for (const tile of activeSection().tiles) selectedTileIds.add(tile._id);
      render();
      return;
    }
    if (e.key === "Delete" || e.key === "Backspace") {
      deleteSelectedTiles();
    }
    if (e.key === "Escape") {
      clearSelection();
      render();
    }
  });

  document.body.addEventListener("dragover", (e) => {
    if (e.dataTransfer.types.includes("Files")) e.preventDefault();
  });
  document.body.addEventListener("drop", async (e) => {
    const file = e.dataTransfer.files[0];
    if (!file) return;
    if (file.name.endsWith(".toml")) {
      e.preventDefault();
      importToml(await file.text());
    }
  });
}

function boot() {
  try {
    init();
  } catch (err) {
    console.error(err);
    const banner = document.createElement("div");
    banner.style.cssText =
      "position:fixed;inset:0 auto auto 0;right:0;background:#e05050;color:#fff;padding:12px 16px;z-index:9999;font:14px sans-serif";
    banner.textContent = t("bootError", { message: err.message });
    document.body.prepend(banner);
  }
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
