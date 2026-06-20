"""Inject or refresh `place` fields in bosses.toml from bosses.json (by flag_id)."""

import json
import re
from pathlib import Path

JSON_PATH = Path(
    r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data\engus\bosses.json"
)
TOML_PATH = Path(
    r"G:\Elden_ring_tools\Overlay\crates\er_game_state\tables\bosses.toml"
)

FIELD_ORDER = ("flag_id", "name", "region", "icon", "place", "dlc")


def load_places() -> dict[int, list[str]]:
    places: dict[int, list[str]] = {}
    with JSON_PATH.open(encoding="utf-8") as f:
        for region in json.load(f):
            for boss in region["bosses"]:
                if place := boss.get("place"):
                    places.setdefault(boss["flag_id"], []).append(place)
    return places


def parse_field(line: str) -> tuple[str, str] | None:
    stripped = line.strip()
    m = re.match(r"^(\w+)\s*=\s*(.+)$", stripped)
    if m:
        return m.group(1), m.group(2)
    return None


def format_boss(fields: dict[str, str]) -> str:
    lines = ["[[boss]]"]
    for key in FIELD_ORDER:
        if key in fields:
            lines.append(f"{key} = {fields[key]}")
    return "\n".join(lines) + "\n"


def main() -> None:
    places = load_places()
    text = TOML_PATH.read_text(encoding="utf-8")

    region_marker = re.search(r"\n# Region -> subregion", text)
    if not region_marker:
        region_marker = re.search(r"\n\[\[region\]\]", text)
    if not region_marker:
        raise SystemExit("Could not find [[region]] section")

    header = text[: region_marker.start()].rstrip() + "\n\n"
    tail = text[region_marker.start() + 1 :].lstrip("\n")
    boss_section = text[: region_marker.start()]

    chunks = re.split(r"(?=\[\[boss\]\])", boss_section)
    bosses: list[str] = []
    updated = 0

    for chunk in chunks:
        if not chunk.strip().startswith("[[boss]]"):
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
            new_place = f'"""{place.replace(chr(34), chr(92) + chr(34))}"""'
            if fields.get("place") != new_place:
                updated += 1
            fields["place"] = new_place
        else:
            fields.pop("place", None)

        bosses.append(format_boss(fields))

    body = "\n".join(bosses)
    TOML_PATH.write_text(header + body + "\n" + tail, encoding="utf-8")
    print(f"Refreshed places on {updated} bosses ({len(bosses)} total)")


if __name__ == "__main__":
    main()
