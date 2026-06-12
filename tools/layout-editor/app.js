// ── Constants ─────────────────────────────────────────────────────

const METRICS = [
  { id: "igt", label: "IGT", showMax: false },
  { id: "deaths", label: "DEATHS", showMax: false },
  { id: "ng_cycle", label: "NG", showMax: false },
  { id: "bosses", label: "BOSS", showMax: true },
  { id: "great_runes", label: "RUNES", showMax: true },
  { id: "kindling", label: "KINDLING", showMax: true, icon: "kindling" },
  { id: "scadutree", label: "SHARDS", showMax: true, icon: "scadutree" },
];

const DEFAULT_STYLE = {
  border_default: [100, 100, 110, 200],
  border_complete: [60, 200, 90, 255],
  tile_bg: [12, 12, 18, 180],
  label_scale: 0.65,
  value_scale: 1.15,
};

/** Dev repo and release zip: tools/layout-editor/ → ../../assets/icons/; legacy release root layout-editor/ → ../assets/icons/ */
const ICON_BASE = (() => {
  const pageDir = new URL(".", location.href);
  const path = decodeURIComponent(pageDir.pathname).replace(/\\/g, "/").toLowerCase();
  const rel = path.includes("/tools/layout-editor/") ? "../../assets/icons/" : "../assets/icons/";
  return new URL(rel, pageDir).href;
})();

const PREVIEW_METRICS = {
  igt: "1:23:45",
  deaths: "42",
  ng_cycle: "NG+2",
  bosses: "12/165",
  great_runes: "6/6",
  kindling: "7/8",
  scadutree: "12/20",
};

const PREVIEW_ITEM_COUNT = "3";

const SECTION_NAMES = ["minimalist", "extended"];

const ITEM_CATEGORIES = [
  { id: "runes", label: "Runes" },
  { id: "key_items", label: "Key items" },
  { id: "talismans", label: "Talismans" },
  { id: "consumables", label: "Consumables" },
];

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

function createDefaultState() {
  return {
    grid: { columns: 8, unit_size: 64, gap: 4, border_radius: 6, window_padding: 8 },
    style: { ...DEFAULT_STYLE },
    default_section: "minimalist",
    sections: SECTION_NAMES.map((name) => ({ name, tiles: [], ...defaultSectionGrid(name) })),
    activeSection: 0,
  };
}

let nextId = 1;
let catalog = [];
let catalogByKey = new Map();
let state = createDefaultState();
let selectedTileId = null;
let dragState = null;

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
  propsEmpty: $("#props-empty"),
  propsTile: $("#props-tile"),
  propKind: $("#prop-kind"),
  propLabel: $("#prop-label"),
  propMetric: $("#prop-metric"),
  propKey: $("#prop-key"),
  propCol: $("#prop-col"),
  propRow: $("#prop-row"),
  propW: $("#prop-w"),
  propH: $("#prop-h"),
  propShowMax: $("#prop-show-max"),
  propIcon: $("#prop-icon"),
  fieldLabel: $("#field-label"),
  fieldMetric: $("#field-metric"),
  fieldKey: $("#field-key"),
  fieldShowMax: $("#field-show-max"),
  fieldIcon: $("#field-icon"),
  cfgColumns: $("#cfg-columns"),
  cfgRows: $("#cfg-rows"),
  cfgUnitSize: $("#cfg-unit-size"),
  cfgGap: $("#cfg-gap"),
  cfgPadding: $("#cfg-padding"),
  cfgDefaultSection: $("#cfg-default-section"),
};

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

// ── Init ────────────────────────────────────────────────────────────

function init() {
  catalog = Array.isArray(window.LAYOUT_CATALOG) ? window.LAYOUT_CATALOG : [];
  catalogByKey = new Map(catalog.map((e) => [e.key, e]));
  els.catalogCount.textContent = catalog.length;

  buildPalette();
  bindEvents();
  syncConfigInputs();
  applyThemeFromState();
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
  return `rgba(${c[0]}, ${c[1]}, ${c[2]}, ${(c[3] ?? 255) / 255})`;
}

function applyThemeFromState() {
  const s = state.style;
  const root = document.documentElement.style;
  root.setProperty("--tile-bg-color", rgbaCss(s.tile_bg));
  root.setProperty("--tile-border-color", rgbaCss(s.border_default));
  root.setProperty("--tile-border-complete", rgbaCss(s.border_complete));
}

function fontSizes(pxW, pxH) {
  const unit = Math.min(pxW, pxH);
  const base = unit / 64;
  return {
    label: Math.max(7, base * state.style.label_scale * 13),
    value: Math.max(9, base * state.style.value_scale * 17),
    solo: Math.max(10, base * state.style.value_scale * 20),
  };
}

function metricPreview(metric, showMax) {
  const text = PREVIEW_METRICS[metric] ?? "---";
  let complete = false;
  if (showMax && text.includes("/")) {
    const [cur, max] = text.split("/");
    complete = cur === max && max !== "0";
  }
  return { text, complete };
}

function fillPaletteThumb(thumb, kind, data) {
  thumb.innerHTML = "";
  if (kind === "metric") {
    const m = METRICS.find((x) => x.id === (data.id || data.metric)) || data;
    const preview = metricPreview(m.id || m.metric, m.showMax ?? data.showMax);
    if (m.icon) {
      const img = makeIconImg(m.icon, "tile-icon");
      img.style.width = "82%";
      img.style.maxHeight = "58%";
      thumb.appendChild(img);
    } else {
      const lbl = document.createElement("span");
      lbl.className = "palette-thumb-text";
      lbl.textContent = (m.label || m.id).slice(0, 8);
      thumb.appendChild(lbl);
    }
    const val = document.createElement("span");
    val.className = "palette-thumb-value";
    val.textContent = preview.text;
    thumb.appendChild(val);
    return;
  }
  if (kind === "label") {
    const val = document.createElement("span");
    val.className = "palette-thumb-value";
    val.style.fontSize = "0.62rem";
    val.textContent = data.label || "TITRE";
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
      val.textContent = PREVIEW_ITEM_COUNT;
      thumb.appendChild(val);
    }
  }
}

function fillTileBody(body, tile, pxW, pxH) {
  body.innerHTML = "";
  const sizes = fontSizes(pxW, pxH);

  if (tile.kind === "label") {
    const val = document.createElement("div");
    val.className = "tile-value tile-value--solo";
    val.style.fontSize = `${sizes.solo}px`;
    val.textContent = tile.label || "TITRE";
    body.appendChild(val);
    return;
  }

  if (tile.kind === "metric") {
    const preview = metricPreview(tile.metric, tile.show_max);
    if (tile.icon) {
      const icon = makeIconImg(tile.icon, "tile-icon tile-icon--metric", tile.label);
      const iconPx = Math.min(pxW, pxH) * 0.38;
      icon.style.width = `${iconPx}px`;
      icon.style.maxHeight = `${iconPx}px`;
      body.appendChild(icon);
    }
    const lbl = document.createElement("div");
    lbl.className = "tile-label";
    lbl.style.fontSize = `${sizes.label}px`;
    lbl.textContent = tile.label || tile.metric;
    body.appendChild(lbl);
    const val = document.createElement("div");
    val.className = "tile-value";
    val.style.fontSize = `${sizes.value}px`;
    val.textContent = preview.text;
    body.appendChild(val);
    return { complete: preview.complete };
  }

  if (tile.kind === "item") {
    const countable = itemIsCountable(tile.key);
    const iconPx = Math.min(pxW, pxH) * (countable ? 0.58 : 0.78);
    const icon = makeIconImg(itemIconKey(tile.key), `tile-icon ${countable ? "tile-icon--item-countable" : "tile-icon--item"}`, tile.key);
    icon.style.width = `${iconPx}px`;
    icon.style.maxHeight = `${iconPx}px`;
    body.appendChild(icon);
    if (countable) {
      const val = document.createElement("div");
      val.className = "tile-value tile-count";
      val.style.fontSize = `${sizes.value * 0.85}px`;
      val.textContent = PREVIEW_ITEM_COUNT;
      body.appendChild(val);
    }
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
    makePaletteEl("label", "Label (texte)", { label: "TITRE" })
  );

  buildItemSections();
  renderItemPalette();

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
  const t = document.createElement("div");
  t.className = "palette-title";
  t.textContent = title;
  meta.appendChild(t);
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
  for (const cat of ITEM_CATEGORIES) {
    const section = document.createElement("div");
    section.className = "item-section";
    section.dataset.category = cat.id;

    const head = document.createElement("button");
    head.type = "button";
    head.className = "item-section-head";
    head.innerHTML = `<span>${cat.label}</span><span class="item-section-count">0</span>`;
    head.addEventListener("click", () => section.classList.toggle("collapsed"));

    const content = document.createElement("div");
    content.className = "item-section-content";

    const search = document.createElement("input");
    search.type = "search";
    search.className = "input item-section-search";
    search.placeholder = `Search ${cat.label.toLowerCase()}…`;
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
  for (const cat of ITEM_CATEGORIES) renderItemSection(cat.id);
}

// ── Render ──────────────────────────────────────────────────────────

function render() {
  applyThemeFromState();
  renderSectionTabs();
  renderGrid();
  renderProperties();
  syncConfigInputs();
  updateGridInfo();
}

function renderSectionTabs() {
  els.sectionTabs.innerHTML = "";
  state.sections.forEach((sec, i) => {
    const tab = document.createElement("button");
    tab.className = "section-tab" + (i === state.activeSection ? " active" : "");
    tab.type = "button";
    tab.textContent = sec.name;
    tab.addEventListener("click", () => {
      state.activeSection = i;
      selectedTileId = null;
      render();
    });
    els.sectionTabs.appendChild(tab);
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
  const { unit_size, gap, window_padding } = state.grid;
  const unit = unit_size;
  const { minCol, minRow } = minGridBounds();
  const cols = Math.max(sec.gridCols ?? 8, minCol);
  const rows = Math.max(sec.gridRows ?? 1, minRow);
  const w = window_padding * 2 + cols * unit + (cols - 1) * gap;
  const h = window_padding * 2 + rows * unit + (rows - 1) * gap;
  return { columns: cols, unit, gap, window_padding, rows, w, h };
}

function renderGrid() {
  const gm = gridMetrics();
  const canvas = els.gridCanvas;
  canvas.style.width = `${gm.w}px`;
  canvas.style.height = `${gm.h}px`;
  canvas.innerHTML = "";

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
  canvas.appendChild(bg);

  const overlaps = findOverlaps(activeSection().tiles);
  for (const tile of activeSection().tiles) {
    canvas.appendChild(renderTileEl(tile, gm, overlaps.has(tile._id)));
  }

  attachGridResizeHandles(canvas, gm);
}

function attachGridResizeHandles(canvas, gm) {
  const mk = (cls, axis, title) => {
    const el = document.createElement("div");
    el.className = `grid-resize-handle ${cls}`;
    el.title = title;
    el.addEventListener("mousedown", (e) => startGridResize(e, axis));
    return el;
  };
  canvas.appendChild(mk("grid-resize-e", "e", "Élargir colonnes"));
  canvas.appendChild(mk("grid-resize-s", "s", "Élargir lignes"));
  canvas.appendChild(mk("grid-resize-se", "se", "Élargir la grille"));
  if (dragState?.mode === "grid") canvas.classList.add("grid-canvas--resizing");
}

function renderTileEl(tile, gm, overlap) {
  const el = document.createElement("div");
  el.className = "tile";
  el.dataset.id = tile._id;
  if (tile._id === selectedTileId) el.classList.add("selected");
  if (overlap) el.classList.add("overlap");

  const left = gm.window_padding + tile.col * (gm.unit + gm.gap);
  const top = gm.window_padding + tile.row * (gm.unit + gm.gap);
  const w = tile.w * gm.unit + (tile.w - 1) * gm.gap;
  const h = tile.h * gm.unit + (tile.h - 1) * gm.gap;

  el.style.left = `${left}px`;
  el.style.top = `${top}px`;
  el.style.width = `${w}px`;
  el.style.height = `${h}px`;
  el.style.borderRadius = `${state.grid.border_radius}px`;

  const body = document.createElement("div");
  body.className = "tile-body";
  const meta = fillTileBody(body, tile, w, h);
  if (meta?.complete) el.classList.add("tile--complete");
  el.appendChild(body);

  if (tile._id === selectedTileId) {
    const zone = document.createElement("div");
    zone.className = "resize-zone";
    zone.title = "Redimensionner";
    zone.addEventListener("mousedown", (e) => startResize(e, tile._id));
    el.appendChild(zone);

    const handle = document.createElement("div");
    handle.className = "resize-handle";
    handle.title = "Redimensionner";
    handle.addEventListener("mousedown", (e) => startResize(e, tile._id));
    el.appendChild(handle);
  }

  if (dragState?.mode === "resize" && dragState.tileId === tile._id) {
    el.classList.add("tile--resizing");
  }

  el.addEventListener("mousedown", (e) => {
    if (e.target.classList.contains("resize-handle") || e.target.classList.contains("resize-zone")) {
      return;
    }
    e.preventDefault();
    const wasSelected = selectedTileId === tile._id;
    selectedTileId = tile._id;
    startMove(e, tile._id);
    if (!wasSelected) render();
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
  const tile = activeSection().tiles.find((t) => t._id === selectedTileId);
  if (!tile) {
    els.propsEmpty.classList.remove("hidden");
    els.propsTile.classList.add("hidden");
    return;
  }
  els.propsEmpty.classList.add("hidden");
  els.propsTile.classList.remove("hidden");

  els.propKind.value = tile.kind;
  els.fieldLabel.classList.toggle("hidden", tile.kind === "item");
  els.fieldMetric.classList.toggle("hidden", tile.kind !== "metric");
  els.fieldKey.classList.toggle("hidden", tile.kind !== "item");
  els.fieldShowMax.classList.toggle("hidden", tile.kind !== "metric");
  els.fieldIcon.classList.toggle("hidden", tile.kind !== "metric");

  els.propLabel.value = tile.label || "";
  els.propMetric.value = tile.metric || "igt";
  els.propKey.value = tile.key || "";
  els.propCol.value = tile.col;
  els.propRow.value = tile.row;
  els.propW.value = tile.w;
  els.propH.value = tile.h;
  els.propShowMax.checked = !!tile.show_max;
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
}

function updateGridInfo() {
  const gm = gridMetrics();
  const n = activeSection().tiles.length;
  els.gridInfo.textContent = `${gm.columns}×${gm.rows} · ${n} tuile${n !== 1 ? "s" : ""}`;
}

// ── Tile CRUD ───────────────────────────────────────────────────────

function createTile(kind, data, col, row) {
  const base = { _id: uid(), kind, col, row, w: 1, h: 1 };
  if (kind === "label") {
    return { ...base, label: data.label || "TITRE" };
  }
  if (kind === "metric") {
    const m = METRICS.find((x) => x.id === data.id) || data;
    return {
      ...base,
      w: 2,
      metric: m.id || data.metric || "igt",
      label: m.label || data.label || "METRIC",
      show_max: m.showMax ?? data.showMax ?? false,
      icon: m.icon || data.icon || undefined,
    };
  }
  if (kind === "item") {
    return { ...base, key: data.key || "godrick_rune" };
  }
  return base;
}

function deleteSelectedTile() {
  if (!selectedTileId) return;
  const sec = activeSection();
  sec.tiles = sec.tiles.filter((t) => t._id !== selectedTileId);
  selectedTileId = null;
  render();
}

function applyPropChanges() {
  const tile = activeSection().tiles.find((t) => t._id === selectedTileId);
  if (!tile) return;
  if (tile.kind !== "item") tile.label = els.propLabel.value;
  if (tile.kind === "metric") {
    tile.metric = els.propMetric.value;
    tile.show_max = els.propShowMax.checked;
    const icon = els.propIcon.value.trim();
    tile.icon = icon || undefined;
  }
  if (tile.kind === "item") tile.key = els.propKey.value.trim() || tile.key;
  tile.col = Math.max(0, Number(els.propCol.value) || 0);
  tile.row = Math.max(0, Number(els.propRow.value) || 0);
  tile.w = Math.max(1, Number(els.propW.value) || 1);
  tile.h = Math.max(1, Number(els.propH.value) || 1);
  render();
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
  const tile = activeSection().tiles.find((t) => t._id === tileId);
  if (!tile) return;
  const origin = cellFromPoint(e.clientX, e.clientY);
  dragState = {
    mode: "move",
    tileId,
    offsetCol: origin.col - tile.col,
    offsetRow: origin.row - tile.row,
  };
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
}

function startResize(e, tileId) {
  e.preventDefault();
  e.stopPropagation();
  selectedTileId = tileId;
  dragState = { mode: "resize", tileId };
  document.addEventListener("mousemove", onDragMove);
  document.addEventListener("mouseup", onDragEnd);
}

function startGridResize(e, axis) {
  e.preventDefault();
  e.stopPropagation();
  selectedTileId = null;
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

  if (dragState.mode === "grid") {
    const gm = gridMetrics();
    const step = gm.unit + gm.gap;
    const dCol = Math.round((e.clientX - dragState.startX) / step);
    const dRow = Math.round((e.clientY - dragState.startY) / step);
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
  const cell = cellFromPoint(e.clientX, e.clientY);
  const prev = { col: tile.col, row: tile.row, w: tile.w, h: tile.h };

  if (dragState.mode === "move") {
    const { columns } = gridMetrics();
    tile.col = Math.max(0, Math.min(columns - tile.w, cell.col - dragState.offsetCol));
    tile.row = Math.max(0, cell.row - dragState.offsetRow);
  } else {
    applyResize(tile, cell);
  }

  if (
    prev.col !== tile.col ||
    prev.row !== tile.row ||
    prev.w !== tile.w ||
    prev.h !== tile.h
  ) {
    render();
  }
}

function onDragEnd() {
  dragState = null;
  document.removeEventListener("mousemove", onDragMove);
  document.removeEventListener("mouseup", onDragEnd);
  render();
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
    selectedTileId = tile._id;
    render();
  });
}

// ── TOML export ─────────────────────────────────────────────────────

function rgba(arr) {
  return `[${arr.join(", ")}]`;
}

/** Global column count for Rust — max across sections, editor grid size, and tile bounds. */
function globalGridColumns() {
  let cols = state.grid.columns ?? 8;
  for (const sec of state.sections) {
    cols = Math.max(cols, sec.gridCols ?? 8);
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
          `${sec.name} : « ${name} » dépasse la grille (${t.col} + ${t.w} > ${columns} colonnes)`
        );
      }
    }
    const overlaps = findOverlaps(sec.tiles);
    if (overlaps.size > 0) {
      errors.push(`${sec.name} : tuiles qui se chevauchent`);
    }
  }
  const nonEmpty = state.sections.filter((s) => s.tiles.length > 0);
  if (nonEmpty.length === 0) {
    errors.push("Aucune tuile à exporter");
  }
  if (
    state.default_section &&
    !nonEmpty.some((s) => s.name === state.default_section)
  ) {
    errors.push(
      `Section par défaut « ${state.default_section} » vide — ajoutez une tuile ou changez la section par défaut`
    );
  }
  return errors;
}

function exportToml() {
  const columns = globalGridColumns();
  const lines = [];
  lines.push("# Généré par ER Overlay Layout Editor");
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
  lines.push(`label_scale = ${state.style.label_scale}`);
  lines.push(`value_scale = ${state.style.value_scale}`);
  lines.push("");
  lines.push(`default_section = "${state.default_section}"`);
  lines.push("");

  for (const section of state.sections) {
    if (section.tiles.length === 0) continue;
    lines.push("[[section]]");
    lines.push(`name = "${section.name}"`);
    lines.push("");
    for (const tile of section.tiles) {
      lines.push("[[section.tile]]");
      lines.push(`kind = "${tile.kind}"`);
      if (tile.kind === "metric") {
        lines.push(`metric = "${tile.metric}"`);
        lines.push(`label = "${tile.label}"`);
        if (tile.show_max) lines.push("show_max = true");
        if (tile.icon) lines.push(`icon = "${tile.icon}"`);
      } else if (tile.kind === "label") {
        lines.push(`label = "${tile.label}"`);
      } else if (tile.kind === "item") {
        lines.push(`key = "${tile.key}"`);
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
    alert(`Export impossible :\n\n${errors.join("\n")}`);
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
  }
  if (raw.default_section) {
    newState.default_section = raw.default_section;
  }

  const sections = raw.section || [];
  if (sections.length > 0) {
    newState.sections = SECTION_NAMES.map((name) => {
      const found = sections.find((s) => s.name === name);
      const tiles = (found?.tile || []).map(parseTileDef);
      const inferred = inferSectionGrid(tiles);
      const fileCols = raw.grid?.columns ?? defaultSectionGrid(name).gridCols;
      return {
        name,
        tiles,
        ...defaultSectionGrid(name),
        gridCols: Math.max(fileCols, inferred.gridCols),
        gridRows: Math.max(
          defaultSectionGrid(name).gridRows,
          inferred.gridRows
        ),
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
  selectedTileId = null;
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
    return {
      ...base,
      metric: t.metric,
      label: t.label || t.metric,
      show_max: !!t.show_max,
      icon: t.icon || undefined,
    };
  }
  if (t.kind === "label") {
    return { ...base, label: t.label || "" };
  }
  if (t.kind === "item") {
    return { ...base, key: t.key || "" };
  }
  return base;
}

// ── Events ──────────────────────────────────────────────────────────

function bindEvents() {
  setupCanvasDrop();

  $("#btn-export").addEventListener("click", downloadToml);
  $("#btn-new").addEventListener("click", () => {
    if (state.sections.some((s) => s.tiles.length) && !confirm("Effacer le layout actuel ?")) return;
    state = createDefaultState();
    selectedTileId = null;
    render();
  });
  $("#btn-delete-tile").addEventListener("click", deleteSelectedTile);

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

  document.addEventListener("keydown", (e) => {
    if (e.key === "Delete" || e.key === "Backspace") {
      if (document.activeElement.tagName === "INPUT" || document.activeElement.tagName === "SELECT") return;
      deleteSelectedTile();
    }
    if (e.key === "Escape") {
      selectedTileId = null;
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
    banner.textContent = `Erreur au démarrage : ${err.message}`;
    document.body.prepend(banner);
  }
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
