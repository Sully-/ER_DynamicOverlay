// ── RGBA color picker (native color input + opacity slider) ─────────

function clampByte(n) {
  return Math.max(0, Math.min(255, Math.round(Number(n) || 0)));
}

function rgbaCss(c) {
  return `rgba(${c[0]}, ${c[1]}, ${c[2]}, ${(c[3] ?? 255) / 255})`;
}

function rgbToHex(r, g, b) {
  return (
    "#" +
    [r, g, b]
      .map((x) => clampByte(x).toString(16).padStart(2, "0"))
      .join("")
  );
}

function hexToRgb(hex) {
  const raw = String(hex || "").replace("#", "");
  if (raw.length === 3) {
    return [
      parseInt(raw[0] + raw[0], 16),
      parseInt(raw[1] + raw[1], 16),
      parseInt(raw[2] + raw[2], 16),
    ];
  }
  if (raw.length >= 6) {
    return [
      parseInt(raw.slice(0, 2), 16),
      parseInt(raw.slice(2, 4), 16),
      parseInt(raw.slice(4, 6), 16),
    ];
  }
  return [0, 0, 0];
}

/**
 * Mount a color + opacity control bound to a `[r, g, b, a]` array (0–255).
 * @returns {{ syncFromState: () => void }}
 */
function mountRgbaColorPicker(container, { getRgba, setRgba, onChange, opacityLabel = "Opacity" }) {
  container.innerHTML = "";
  container.classList.add("color-picker");

  const row = document.createElement("div");
  row.className = "color-picker-row";

  const swatch = document.createElement("div");
  swatch.className = "color-picker-swatch";
  swatch.title = "Preview";

  const colorInput = document.createElement("input");
  colorInput.type = "color";
  colorInput.className = "color-picker-color";

  const alphaWrap = document.createElement("div");
  alphaWrap.className = "color-picker-alpha";

  const alphaText = document.createElement("span");
  alphaText.className = "color-picker-alpha-label";
  alphaText.textContent = opacityLabel;

  const alphaRange = document.createElement("input");
  alphaRange.type = "range";
  alphaRange.min = "0";
  alphaRange.max = "100";
  alphaRange.className = "color-picker-alpha-range";

  const alphaVal = document.createElement("span");
  alphaVal.className = "color-picker-alpha-val";

  alphaWrap.append(alphaText, alphaRange, alphaVal);

  const valueText = document.createElement("code");
  valueText.className = "color-picker-value muted";

  row.append(swatch, colorInput, alphaWrap);
  container.append(row, valueText);

  function readRgba() {
    const c = getRgba();
    return [
      clampByte(c?.[0]),
      clampByte(c?.[1]),
      clampByte(c?.[2]),
      clampByte(c?.[3] ?? 255),
    ];
  }

  function writeRgba(rgba) {
    setRgba(rgba.map(clampByte));
  }

  function updateDisplay(rgba) {
    swatch.style.background = rgbaCss(rgba);
    colorInput.value = rgbToHex(rgba[0], rgba[1], rgba[2]);
    const pct = Math.round((rgba[3] / 255) * 100);
    alphaRange.value = String(pct);
    alphaVal.textContent = `${pct}%`;
    valueText.textContent = `[${rgba.join(", ")}]`;
  }

  function notifyChange() {
    updateDisplay(readRgba());
    onChange?.();
  }

  colorInput.addEventListener("input", () => {
    const [r, g, b] = hexToRgb(colorInput.value);
    const cur = readRgba();
    writeRgba([r, g, b, cur[3]]);
    notifyChange();
  });

  alphaRange.addEventListener("input", () => {
    const cur = readRgba();
    const a = clampByte((Number(alphaRange.value) / 100) * 255);
    writeRgba([cur[0], cur[1], cur[2], a]);
    notifyChange();
  });

  return {
    syncFromState() {
      updateDisplay(readRgba());
    },
    setOpacityLabel(label) {
      alphaText.textContent = label;
    },
  };
}

window.ColorPicker = {
  mountRgbaColorPicker,
  rgbaCss,
  clampByte,
};
