"""Rebuild bosses.toml: places, field order, region_display_order, [[region]] data."""

import importlib.util
import json
import re
from collections import defaultdict
from pathlib import Path

JSON_PATH = Path(
    r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data\engus\bosses.json"
)
TOML_PATH = Path(
    r"G:\Elden_ring_tools\Overlay\crates\er_game_state\tables\bosses.toml"
)

FIELD_ORDER = ("flag_id", "name", "region", "icon", "place", "dlc")

REGION_MAPPING = {
    "Limgrave": "LIMGRAVE",
    "Weeping Peninsula": "WEEPING PENINSULA",
    "Stormveil Castle": "LIMGRAVE",
    "Liurnia of the Lakes": "LIURNIA OF THE LAKES",
    "Moonlight Altar": "POST LIURNIA",
    "Academy of Raya Lucaria": "LIURNIA OF THE LAKES",
    "Caelid": "CAELID",
    "Greyoll's Dragonbarrow": "DRAGONBARROW",
    "Altus Plateau": "ALTUS PLATEAU",
    "Capital Outskirts": "LEYNDELL",
    "Mt. Gelmir": "MT. GELMIR",
    "Volcano Manor": "MT. GELMIR",
    "Leyndell, Royal Capital": "LEYNDELL",
    "Forbidden Lands": "FORBIDDEN LANDS",
    "Mountaintops of the Giants": "MOUNTAINTOPS OF THE GIANTS",
    "Crumbling Farum Azula": "CRUMBLING FARUM AZULA + LEYNDELL, ASHEN CAPITAL",
    "Consecrated Snowfield": "CONSECRATED SNOWFIELD",
    "Miquella's Haligtree": "HALIGTREE",
    "Siofra River": "SIOFRA RIVER",
    "Mohgwyn Dynasty Mausoleum": "MOHGWYN PALACE",
    "Ainsel River": "AINSEL RIVER",
    "Deeproot Depths": "DEEPROOT DEPTHS",
    "Leyndell, Ashen Capital": "CRUMBLING FARUM AZULA + LEYNDELL, ASHEN CAPITAL",
    "Gravesite Plain": "The Land of Shadow",
    "Scadu Altus": "The Land of Shadow",
    "Ancient Ruins of Rauh": "The Land of Shadow",
    "Cerulean Coast": "The Land of Shadow",
    "Charo's Hidden Grave": "The Land of Shadow",
    "Jagged Peak": "The Land of Shadow",
    "Scaduview": "The Land of Shadow",
    "Abyssal Woods": "The Land of Shadow",
    "Enir-Ilim": "The Land of Shadow",
}

EXTRA_SUBREGIONS = {
    "NOKRON, ETERNAL CITY": [1202],
    "LAKE OF ROT": [1204],
}


def load_places() -> dict[int, list[str]]:
    places: dict[int, list[str]] = {}
    with JSON_PATH.open(encoding="utf-8") as f:
        for region in json.load(f):
            for boss in region["bosses"]:
                if place := boss.get("place"):
                    places.setdefault(boss["flag_id"], []).append(place)
    return places


def toml_str(value: str) -> str:
    return f'"""{value.replace(chr(34), chr(92) + chr(34))}"""'


def parse_field(line: str) -> tuple[str, str] | None:
    m = re.match(r"^(\w+)\s*=\s*(.+)$", line.strip())
    return (m.group(1), m.group(2)) if m else None


def format_boss(fields: dict[str, str]) -> str:
    lines = ["[[boss]]"]
    for key in FIELD_ORDER:
        if key in fields:
            lines.append(f"{key} = {fields[key]}")
    return "\n".join(lines)


def load_region_display_order() -> list[str]:
    script = Path(__file__).with_name("gen_region_order.py")
    spec = importlib.util.spec_from_file_location("gen_region_order", script)
    if spec is None or spec.loader is None:
        raise RuntimeError("gen_region_order.py not found")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.build_display_order()  # type: ignore[attr-defined]


def build_region_tail() -> str:
    with JSON_PATH.open(encoding="utf-8") as f:
        data = json.load(f)

    subregions: dict[str, set[int]] = defaultdict(set)
    for entry in data:
        name = REGION_MAPPING.get(entry["region_name"])
        if not name:
            continue
        subregions[name].update(entry["regions"])
    for name, ids in EXTRA_SUBREGIONS.items():
        subregions[name].update(ids)

    lines = ["# Region -> subregion IDs (player map id / 1000). Derived from bosses.json.", ""]
    for name in sorted(subregions.keys()):
        lines.append("[[region]]")
        lines.append(f"name = {toml_str(name)}")
        lines.append(f"subregions = {sorted(subregions[name])}")
        lines.append("")
    return "\n".join(lines)


def main() -> None:
    places = load_places()
    text = TOML_PATH.read_text(encoding="utf-8")

    region_marker = re.search(r"\n(?:# Region -> subregion|\[\[region\]\])", text)
    boss_text = text[: region_marker.start()] if region_marker else text

    chunks = re.split(r"(?=\[\[boss\]\])", boss_text)
    bosses: list[str] = []

    for chunk in chunks:
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

        flag_id = int(fields["flag_id"])
        place_candidates = places.get(flag_id, [])
        if place := (place_candidates.pop(0) if place_candidates else None):
            fields["place"] = toml_str(place)
        else:
            fields.pop("place", None)
        bosses.append(format_boss(fields))

    order = load_region_display_order()
    order_lines = ",\n".join(f"  {toml_str(name)}" for name in order)
    header = (
        "# Elden Ring — 207 boss encounters (full health bar).\n"
        "# Names: fromsoft-boss-checker/eldenRingRegions.json\n"
        "# Flags: SoulSplitter (SoulMemory/EldenRing/Boss.cs) + optional overrides.\n"
        "# Mapped: 207 / 207\n\n"
        "# Region display order (bosses.json progression, deduplicated to bosses.toml labels).\n"
        f"region_display_order = [\n{order_lines},\n]\n\n"
    )

    body = "\n\n".join(bosses) + "\n\n"
    tail = build_region_tail()
    TOML_PATH.write_text(header + body + tail, encoding="utf-8")
    with_place = sum(1 for b in bosses if "place =" in b)
    print(f"Wrote {len(bosses)} bosses ({with_place} with place)")


if __name__ == "__main__":
    main()
