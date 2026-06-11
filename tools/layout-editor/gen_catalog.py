#!/usr/bin/env python3
"""Generate catalog.js from goods.toml for the layout editor palette."""
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TABLES = ROOT / "crates" / "er_game_state" / "tables"
OUT_JS = Path(__file__).resolve().parent / "catalog.js"

# Layout-editor palette only — not an overlay aggregate group.
KEY_ITEMS_SECTION = "# Story / dungeon keys"


def parse_key_item_keys(text: str) -> set[str]:
    """Good keys listed after the story-keys section header in goods.toml."""
    idx = text.find(KEY_ITEMS_SECTION)
    if idx == -1:
        return set()
    return set(re.findall(r'^key\s*=\s*"([^"]+)"', text[idx:], re.M))


def parse_rune_keys(text: str) -> set[str]:
    """Great rune members — the only group used for palette categorization."""
    m = re.search(
        r'^\[groups\.great_runes\]\s*\nmembers\s*=\s*\[([^\]]*)\]',
        text,
        re.M,
    )
    if not m:
        return set()
    return set(re.findall(r'"([^"]+)"', m.group(1)))


def palette_category(
    key: str,
    pickup_flag: bool,
    countable: bool,
    rune_keys: set[str],
    key_item_keys: set[str],
) -> str:
    if key in key_item_keys:
        return "key_items"
    if key in rune_keys or key == "great_rune_generic" or pickup_flag:
        return "runes"
    if countable:
        return "consumables"
    return "talismans"


def parse_goods(path: Path) -> list[dict]:
    text = path.read_text(encoding="utf-8")
    rune_keys = parse_rune_keys(text)
    key_item_keys = parse_key_item_keys(text)
    blocks = re.split(r"\n\[\[good\]\]\n", text)
    items: list[dict] = []
    seen: set[str] = set()
    for block in blocks[1:]:
        key_m = re.search(r'^key\s*=\s*"([^"]+)"', block, re.M)
        name_m = re.search(r'^name\s*=\s*"([^"]+)"', block, re.M)
        if not key_m:
            continue
        key = key_m.group(1)
        if key in seen:
            continue
        seen.add(key)
        pickup_flag = re.search(r"^pickup_flag\s*=", block, re.M) is not None
        countable = re.search(r"^count\s*=\s*true", block, re.M) is not None
        items.append(
            {
                "key": key,
                "name": name_m.group(1) if name_m else key,
                "iconKey": key,
                "category": palette_category(
                    key, pickup_flag, countable, rune_keys, key_item_keys
                ),
                "countable": countable,
            }
        )
    return items


def main() -> None:
    items = parse_goods(TABLES / "goods.toml")
    payload = json.dumps(items, ensure_ascii=False, indent=2)
    OUT_JS.write_text(f"window.LAYOUT_CATALOG = {payload};\n", encoding="utf-8")
    print(f"{len(items)} items -> {OUT_JS}")


if __name__ == "__main__":
    main()
