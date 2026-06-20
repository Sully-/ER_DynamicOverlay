#!/usr/bin/env python3
"""Generate en/fr bosses_hardlock.toml from HardLock JSON. Keeps boss names/icons from bosses.toml."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TABLES = ROOT / "crates" / "er_game_state" / "tables"
OUTPUT_NAME = "bosses_hardlock.toml"
SOURCE_NAME = "bosses.toml"
CHECKLIST = Path(r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data")
DEFAULT_JSON = Path(r"c:\Users\sulli\Downloads\HardLock_UWYG_fr.json")

# FR region_name (HardLock) -> EN region_name when subregions are empty or unmapped.
FR_TO_EN_REGION: dict[str, str] = {
    "Nécrolimbe": "Limgrave",
    "Péninsule larmoyante": "Weeping Peninsula",
    "Château de Voilorage": "Stormveil Castle",
    "Liurnia, contrée lacustre": "Liurnia of the Lakes",
    "Académie de Raya Lucaria": "Academy of Raya Lucaria",
    "L'Ainsel": "Ainsel River",
    "La Siofra": "Siofra River",
    "Caelid": "Caelid",
    "Nokron": "Nokron, Eternal City",
    "Plateau Altus": "Altus Plateau",
    "Faubourgs de la capitale": "Capital Outskirts",
    "Mont Gelmir": "Mt. Gelmir",
    "Manoir du volcan": "Volcano Manor",
    "Profondeurs de Fonderacine": "Deeproot Depths",
    "Lac putréfié": "Lake of Rot",
    "Leyndell, capitale royale": "Leyndell, Royal Capital",
    "Terres interdites": "Forbidden Lands",
    "Cimes des Géants": "Mountaintops of the Giants",
    "Ruines de Farum Azula": "Crumbling Farum Azula",
    "Autel lunaire": "Moonlight Altar",
    "Tertre draconique de Greyoll": "Greyoll's Dragonbarrow",
    "Mausolée de la dynastie Mohgwyn": "Mohgwyn Dynasty Mausoleum",
    "Champs enneigés consacrés": "Consecrated Snowfield",
    "Arbre-Sacré de Miquella": "Miquella's Haligtree",
    "Leyndell, capitale des cendres": "Leyndell, Ashen Capital",
    "Plaine sépulcrale": "Gravesite Plain",
    "Côte céruléenne": "Cerulean Coast",
    "Altus Occulte": "Scadu Altus",
    "Base de Rauh": "Rauh Base",
    "Château noir": "Shadow Keep",
    "Fissure des cercueils de pierre": "Stone Coffin Fissure",
    "Panorama occulte": "Scaduview",
    "Cathédrale de Manus Metyr": "Cathedral of Manus Metyr",
    "Passage vers bois abyssaux": "Approach to Abyssal Woods",
    "Bois abyssaux": "Abyssal Woods",
    "Tombeau secret de Charo via Pic déchiqueté": "Charo's Hidden Grave via Jagged Peak",
    "Pic déchiqueté": "Jagged Peak",
    "Ruines antiques de Rauh": "Ancient Ruins of Rauh",
    "Enir-Ilim": "Enir-Ilim",
}


def toml_str(value: str) -> str:
    return f'"""{value.replace(chr(34), chr(92) + chr(34))}"""'


def parse_field(line: str) -> tuple[str, str] | None:
    m = re.match(r"^(\w+)\s*=\s*(.+)$", line.strip())
    return (m.group(1), m.group(2)) if m else None


def load_places_by_flag(path: Path) -> dict[int, list[str | None]]:
    places: dict[int, list[str | None]] = {}
    for region in load_json(path):
        for boss in region["bosses"]:
            place = (boss.get("place") or "").strip() or None
            places.setdefault(boss["flag_id"], []).append(place)
    return places


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
        if "icon" in fields:
            row["icon"] = fields["icon"].strip('"')
        if "place" in fields:
            row["place"] = fields["place"].strip('"').strip('"""')
        if "dlc" in fields:
            row["dlc"] = fields["dlc"] == "true"
        out.setdefault(fid, []).append(row)
    return out


def load_json(path: Path) -> list[dict]:
    return json.loads(path.read_text(encoding="utf-8"))


def build_subregion_en_map(checklist_en: list[dict]) -> dict[frozenset[int], str]:
    mapping: dict[frozenset[int], str] = {}
    for entry in checklist_en:
        key = frozenset(entry.get("regions") or [])
        if key:
            mapping[key] = entry["region_name"]
    return mapping


def resolve_en_region(fr_name: str, subregions: list[int], sub_map: dict[frozenset[int], str]) -> str:
    if fr_name in FR_TO_EN_REGION:
        return FR_TO_EN_REGION[fr_name]
    if subregions:
        key = frozenset(subregions)
        if key in sub_map:
            return sub_map[key]
        best_name = ""
        best_overlap = 0
        sub_set = set(subregions)
        for other_key, en_name in sub_map.items():
            overlap = len(sub_set & set(other_key))
            if overlap > best_overlap:
                best_overlap = overlap
                best_name = en_name
        if best_overlap > 0:
            return best_name
    return fr_name


def load_json_bosses(data: list[dict], lang: str, sub_map: dict[frozenset[int], str]) -> tuple[list[dict], list[str]]:
    bosses: list[dict] = []
    order: list[str] = []
    seen: set[str] = set()

    for region in data:
        fr_region = region["region_name"]
        subregions = region.get("regions") or []
        if lang == "fr":
            region_name = fr_region
        else:
            region_name = resolve_en_region(fr_region, subregions, sub_map)

        if region_name not in seen:
            order.append(region_name)
            seen.add(region_name)

        region_dlc = bool(region.get("dlc"))
        for boss in region["bosses"]:
            place = (boss.get("place") or "").strip() or None
            bosses.append(
                {
                    "flag_id": boss["flag_id"],
                    "region": region_name,
                    "place": place,
                    "dlc": region_dlc,
                }
            )
    return bosses, order


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


def build_region_tail(data: list[dict], lang: str, sub_map: dict[frozenset[int], str]) -> str:
    lines = ["\n# Region -> subregion IDs (player map id / 1000). Derived from HardLock JSON.\n"]
    for entry in data:
        fr_name = entry["region_name"]
        subregions = entry.get("regions") or []
        if lang == "fr":
            name = fr_name
        else:
            name = resolve_en_region(fr_name, subregions, sub_map)
        lines.append("[[region]]")
        lines.append(f"name = {toml_str(name)}")
        lines.append(f"subregions = {subregions}")
        lines.append("")
    return "\n".join(lines)


def header_for(lang: str, count: int) -> str:
    if lang == "en":
        return (
            f"# Elden Ring — {count} boss encounters (HardLock UWYG layout).\n"
            "# Names/icons: preserved from bosses.toml.\n"
            "# Regions/order: HardLock_UWYG_fr.json.\n\n"
        )
    return (
        f"# Elden Ring boss checklist — language 'fr' (HardLock UWYG layout).\n"
        f"# {count} boss encounters. Names/icons preserved; regions from HardLock JSON.\n\n"
    )


def generate(json_path: Path) -> None:
    data = load_json(json_path)
    checklist_en = load_json(CHECKLIST / "engus" / "bosses.json")
    sub_map = build_subregion_en_map(checklist_en)
    places_en = load_places_by_flag(CHECKLIST / "engus" / "bosses.json")

    for lang in ("en", "fr"):
        source_path = TABLES / lang / SOURCE_NAME
        toml_path = TABLES / lang / OUTPUT_NAME
        existing = parse_bosses(source_path.read_text(encoding="utf-8"))
        json_bosses, order = load_json_bosses(data, lang, sub_map)

        rows: list[dict] = []
        missing: list[int] = []
        for jb in json_bosses:
            fid = jb["flag_id"]
            old_candidates = existing.get(fid, [])
            old = old_candidates.pop(0) if old_candidates else None
            if not old or "name" not in old:
                missing.append(fid)
                slug = f"boss_{fid}"
                name = f"[?] Unknown boss {fid}"
                icon = slug
                dlc = jb.get("dlc", False)
            else:
                name = old["name"]
                icon = old.get("icon") or f"boss_{fid}"
                dlc = old.get("dlc", jb.get("dlc", False))

            if lang == "en":
                place_candidates = places_en.get(fid, [])
                place_en = place_candidates.pop(0) if place_candidates else None
                place = place_en or (old.get("place") if old else jb["place"])
            else:
                place = jb["place"] or (old.get("place") if old else None)

            rows.append(
                {
                    "flag_id": fid,
                    "name": name,
                    "region": jb["region"],
                    "icon": icon,
                    "place": place,
                    "dlc": dlc,
                }
            )

        order_lines = ",\n".join(f"  {toml_str(name)}" for name in order)
        header = header_for(lang, len(rows))
        header += (
            "# Region display order (HardLock JSON progression, deduplicated).\n"
            f"region_display_order = [\n{order_lines},\n]\n\n"
        )
        body = "\n\n".join(format_boss(r) for r in rows) + "\n"
        toml_path.write_text(
            header + body + build_region_tail(data, lang, sub_map),
            encoding="utf-8",
            newline="\n",
        )
        print(
            f"{lang}: wrote {toml_path.name} — {len(rows)} bosses, {len(order)} regions"
            + (f", {len(missing)} missing in {SOURCE_NAME}: {missing[:5]}" if missing else "")
        )


def main() -> None:
    json_path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_JSON
    if not json_path.is_file():
        raise SystemExit(f"JSON not found: {json_path}")
    generate(json_path)


if __name__ == "__main__":
    main()
