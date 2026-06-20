"""Generate region_display_order for bosses.toml from bosses.json progression."""

import json
import re
from pathlib import Path

MAPPING = {
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

# Overlay-only regions (not separate JSON entries); inserted near related areas.
EXTRA_AFTER = {
    "SIOFRA RIVER": ["NOKRON, ETERNAL CITY"],
    "AINSEL RIVER": ["LAKE OF ROT"],
}

JSON_PATH = Path(
    r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data\engus\bosses.json"
)
TOML_PATH = Path(
    r"G:\Elden_ring_tools\Overlay\crates\er_game_state\tables\bosses.toml"
)


def build_display_order() -> list[str]:
    with JSON_PATH.open(encoding="utf-8") as f:
        data = json.load(f)

    order: list[str] = []
    seen: set[str] = set()

    def append(name: str) -> None:
        if name not in seen:
            order.append(name)
            seen.add(name)

    for entry in data:
        toml_name = MAPPING.get(entry["region_name"])
        if not toml_name:
            print(f"WARN unmapped JSON region: {entry['region_name']}")
            continue
        append(toml_name)
        for extra in EXTRA_AFTER.get(toml_name, []):
            append(extra)

    return order


def patch_toml(order: list[str]) -> None:
    text = TOML_PATH.read_text(encoding="utf-8")
    formatted = ",\n".join(f'  """{name}"""' for name in order)
    block = (
        "# Region display order (bosses.json progression, deduplicated to bosses.toml labels).\n"
        f"region_display_order = [\n{formatted},\n]\n\n"
    )

    if "region_display_order" in text:
        text = re.sub(
            r"# Region display order.*?\nregion_display_order = \[[\s\S]*?\]\n\n",
            block,
            text,
            count=1,
        )
    else:
        # Insert after the header comments, before the first [[boss]].
        text = re.sub(r"(\n)(?=\[\[boss\]\])", r"\n" + block, text, count=1)

    TOML_PATH.write_text(text, encoding="utf-8")
    print(f"Wrote region_display_order ({len(order)} regions) to {TOML_PATH}")


if __name__ == "__main__":
    patch_toml(build_display_order())
