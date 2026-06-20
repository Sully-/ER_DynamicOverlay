#!/usr/bin/env python3
"""Sync bosses.toml from checklist JSON: flag_id, name, place, region, order. Keeps icons."""

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CHECKLIST = Path(r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data")
LOCALES = {"en": "engus", "fr": "frafr"}
FIELD_ORDER = ("flag_id", "name", "region", "icon", "place", "dlc")


def toml_str(value: str) -> str:
    return f'"""{value.replace(chr(34), chr(92) + chr(34))}"""'


def parse_field(line: str) -> tuple[str, str] | None:
    m = re.match(r"^(\w+)\s*=\s*(.+)$", line.strip())
    return (m.group(1), m.group(2)) if m else None


def parse_bosses(text: str) -> dict[int, list[dict]]:
    marker = re.search(r"\n(?:# Region -> subregion|\[\[region\]\])", text)
    boss_text = text[: marker.start()] if marker else text
    out: dict[int, list[dict]] = {}
    for chunk in re.split(r"(?=\[\[boss\]\])", boss_text):
        if "[[boss]]" not in chunk:
            continue
        fields: dict[str, str] = {}
        for line in chunk.splitlines():
            if line.strip() in ("[[boss]]", ""):
                continue
            if parsed := parse_field(line):
                fields[parsed[0]] = parsed[1]
        if "flag_id" not in fields:
            continue
        fid = int(fields["flag_id"])
        row: dict = {"flag_id": fid}
        if "name" in fields:
            row["name"] = fields["name"].strip('"').strip('"""')
        if "region" in fields:
            row["region"] = fields["region"].strip('"').strip('"""')
        if "icon" in fields:
            row["icon"] = fields["icon"].strip('"')
        if "place" in fields:
            row["place"] = fields["place"].strip('"').strip('"""')
        if "dlc" in fields:
            row["dlc"] = fields["dlc"] == "true"
        out.setdefault(fid, []).append(row)
    return out


def load_json_bosses(path: Path) -> tuple[list[dict], list[str], list[dict]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    bosses: list[dict] = []
    order: list[str] = []
    seen: set[str] = set()
    for region in data:
        rn = region["region_name"]
        if rn not in seen:
            order.append(rn)
            seen.add(rn)
        for boss in region["bosses"]:
            bosses.append(
                {
                    "flag_id": boss["flag_id"],
                    "name": boss["boss"],
                    "region": rn,
                    "place": boss.get("place") or None,
                }
            )
    return bosses, order, data


def format_boss(row: dict) -> str:
    lines = ["[[boss]]"]
    lines.append(f"flag_id = {row['flag_id']}")
    lines.append(f"name = {toml_str(row['name'])}")
    lines.append(f"region = {toml_str(row['region'])}")
    lines.append(f'icon = "{row["icon"]}"')
    if row.get("place"):
        lines.append(f"place = {toml_str(row['place'])}")
    lines.append(f"dlc = {'true' if row.get('dlc') else 'false'}")
    return "\n".join(lines)


def build_region_tail(json_regions: list[dict]) -> str:
    lines = ["\n# Region -> subregion IDs (player map id / 1000). Derived from bosses.json.\n"]
    for entry in json_regions:
        lines.append("[[region]]")
        lines.append(f"name = {toml_str(entry['region_name'])}")
        lines.append(f"subregions = {entry['regions']}")
        lines.append("")
    return "\n".join(lines)


def sync_locale(lang: str, json_sub: str) -> None:
    toml_path = ROOT / "crates" / "er_game_state" / "tables" / lang / "bosses.toml"
    text = toml_path.read_text(encoding="utf-8")
    existing = parse_bosses(text)

    json_bosses, order, json_regions = load_json_bosses(CHECKLIST / json_sub / "bosses.json")

    rows: list[dict] = []
    missing_icons: list[int] = []
    for jb in json_bosses:
        fid = jb["flag_id"]
        old_candidates = existing.get(fid, [])
        old = old_candidates.pop(0) if old_candidates else {}
        icon = old.get("icon")
        if not icon:
            slug = re.sub(r"[^a-zA-Z0-9]+", "_", jb["name"].lower()).strip("_")[:48] or "boss"
            icon = slug
            missing_icons.append(fid)
        rows.append(
            {
                "flag_id": fid,
                "name": jb["name"],
                "region": jb["region"],
                "icon": icon,
                "place": jb["place"],
                "dlc": old.get("dlc", False),
            }
        )

    marker = re.search(r"\n(?:# Region -> subregion|\[\[region\]\])", text)
    if not marker:
        raise SystemExit(f"boss section boundary not found in {toml_path}")
    first_boss = re.search(r"\n\[\[boss\]\]", text[: marker.start()])
    header = text[: first_boss.start()].rstrip() + "\n\n" if first_boss else ""

    order_lines = ",\n".join(f"  {toml_str(name)}" for name in order)
    if "region_display_order" in header:
        header = re.sub(
            r"# Region display order[^\n]*\nregion_display_order = \[[\s\S]*?\]\n",
            "# Region display order (bosses.json progression, deduplicated).\n"
            f"region_display_order = [\n{order_lines},\n]\n",
            header,
            count=1,
        )

    body = "\n\n".join(format_boss(r) for r in rows) + "\n"
    toml_path.write_text(header + body + build_region_tail(json_regions), encoding="utf-8", newline="\n")
    print(f"{lang}: {len(rows)} bosses in JSON order, {len(missing_icons)} new icon slugs")


def main() -> None:
    for lang, sub in LOCALES.items():
        sync_locale(lang, sub)


if __name__ == "__main__":
    main()
