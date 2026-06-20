#!/usr/bin/env python3
"""Set each boss region + region_display_order from checklist JSON (by flag_id)."""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHECKLIST = Path(r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data")

LOCALES = {"en": "engus", "fr": "frafr"}


def toml_str(value: str) -> str:
    return f'"""{value.replace(chr(34), chr(92) + chr(34))}"""'


def load_json(path: Path) -> tuple[dict[int, list[str]], list[str], list[dict]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    by_flag: dict[int, list[str]] = {}
    order: list[str] = []
    seen: set[str] = set()
    for region in data:
        rn = region["region_name"]
        if rn not in seen:
            order.append(rn)
            seen.add(rn)
        for boss in region["bosses"]:
            by_flag.setdefault(boss["flag_id"], []).append(rn)
    return by_flag, order, data


def build_region_tail(json_regions: list[dict]) -> str:
    lines = ["\n# Region -> subregion IDs (player map id / 1000). Derived from bosses.json.\n"]
    for entry in json_regions:
        lines.append("[[region]]")
        lines.append(f"name = {toml_str(entry['region_name'])}")
        lines.append(f"subregions = {entry['regions']}")
        lines.append("")
    return "\n".join(lines)


def patch_file(
    toml_path: Path, by_flag: dict[int, list[str]], order: list[str], json_regions: list[dict]
) -> int:
    text = toml_path.read_text(encoding="utf-8")
    updated = 0

    order_lines = ",\n".join(f"  {toml_str(name)}" for name in order)
    order_block = (
        "# Region display order (bosses.json progression, deduplicated).\n"
        f"region_display_order = [\n{order_lines},\n]\n"
    )
    text, n = re.subn(
        r"# Region display order[^\n]*\nregion_display_order = \[[\s\S]*?\]\n",
        order_block,
        text,
        count=1,
    )
    if n == 0:
        raise SystemExit(f"region_display_order not found in {toml_path}")

    def patch_boss(m: re.Match[str]) -> str:
        nonlocal updated
        block = m.group(0)
        fm = re.search(r"^flag_id\s*=\s*(\d+)", block, re.M)
        if not fm:
            return block
        fid = int(fm.group(1))
        regions = by_flag.get(fid, [])
        if not regions:
            raise SystemExit(f"flag_id {fid} missing from JSON in {toml_path}")
        region = regions.pop(0)
        new_region = f"region = {toml_str(region)}"
        if re.search(r"^region\s*=", block, re.M):
            new_block, c = re.subn(r"^region\s*=.*$", new_region, block, count=1, flags=re.M)
            if c:
                old = re.search(r"^region\s*=.*$", block, re.M)
                if old and old.group(0) != new_region:
                    updated += 1
                return new_block
        return block

    text = re.sub(r"\[\[boss\]\][\s\S]*?(?=\n\[\[boss\]\]|\n# Region|\n\[\[region\]\]|\Z)", patch_boss, text)

    region_marker = re.search(r"\n# Region -> subregion", text)
    if not region_marker:
        raise SystemExit(f"[[region]] section not found in {toml_path}")
    text = text[: region_marker.start()] + build_region_tail(json_regions)

    toml_path.write_text(text, encoding="utf-8", newline="\n")
    return updated


def main() -> None:
    for lang, sub in LOCALES.items():
        json_path = CHECKLIST / sub / "bosses.json"
        by_flag, order, json_regions = load_json(json_path)
        path = ROOT / "crates" / "er_game_state" / "tables" / lang / "bosses.toml"
        n = patch_file(path, by_flag, order, json_regions)
        print(
            f"{lang}: {n} boss regions, {len(order)} display-order entries, "
            f"{len(json_regions)} [[region]] blocks"
        )


if __name__ == "__main__":
    main()
