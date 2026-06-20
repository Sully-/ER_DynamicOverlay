#!/usr/bin/env python3
"""Add FieldArea subregion keys missing from bosses.json (interior / boss arenas)."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# map_id / 1000 keys reported by FieldArea but absent from bosses.json `regions` arrays.
EXTRAS: dict[str, list[int]] = {
    "Limgrave": [],
    "Weeping Peninsula": [],
    "Stormveil Castle": [],
    "Liurnia of the Lakes": [1001],  # Chapel of Anticipation (Grafted Scion)
    "Moonlight Altar": [],
    "Academy of Raya Lucaria": [],
    "Caelid": [],
    "Greyoll's Dragonbarrow": [],
    "Altus Plateau": [],
    "Capital Outskirts": [],
    "Mt. Gelmir": [],
    "Volcano Manor": [],
    "Leyndell, Royal Capital": [1110],  # Roundtable Hold
    "Forbidden Lands": [],
    "Mountaintops of the Giants": [],
    "Crumbling Farum Azula": [],
    "Consecrated Snowfield": [],
    "Miquella's Haligtree": [],
    "Siofra River": [1208, 1209],  # boss arenas
    "Mohgwyn Dynasty Mausoleum": [],
    "Ainsel River": [],
    "Deeproot Depths": [],
    "Leyndell, Ashen Capital": [],
    "Gravesite Plain": [2000],  # Belurat
    "Scadu Altus": [2100, 2101, 2102],  # Shadow Keep
    "Ancient Ruins of Rauh": [],
    "Cerulean Coast": [],
    "Charo's Hidden Grave": [],
    "Jagged Peak": [4200, 4202, 4203],  # ruined forges
    "Scaduview": [2500],  # Finger Birthing Grounds
    "Abyssal Woods": [],
    "Enir-Ilim": [],
}

FR_NAMES: dict[str, str] = {
    "Limgrave": "Nécrolimbe",
    "Weeping Peninsula": "Péninsule larmoyante",
    "Stormveil Castle": "Château de Voilorage",
    "Liurnia of the Lakes": "Liurnia, contrée lacustre",
    "Moonlight Altar": "Autel lunaire",
    "Academy of Raya Lucaria": "Académie de Raya Lucaria",
    "Caelid": "Caelid",
    "Greyoll's Dragonbarrow": "Tertre draconique de Greyoll",
    "Altus Plateau": "Plateau Altus",
    "Capital Outskirts": "Faubourgs de la capitale",
    "Mt. Gelmir": "Mont Gelmir",
    "Volcano Manor": "Manoir du volcan",
    "Leyndell, Royal Capital": "Leyndell, capitale royale",
    "Forbidden Lands": "Terres interdites",
    "Mountaintops of the Giants": "Cimes des Géants",
    "Crumbling Farum Azula": "Ruines de Farum Azula",
    "Consecrated Snowfield": "Champs enneigés consacrés",
    "Miquella's Haligtree": "Arbre-Sacré de Miquella",
    "Siofra River": "La Siofra",
    "Mohgwyn Dynasty Mausoleum": "Mausolée de la dynastie Mohgwyn",
    "Ainsel River": "L'Ainsel",
    "Deeproot Depths": "Profondeurs de Fonderacine",
    "Leyndell, Ashen Capital": "Leyndell, capitale des cendres",
    "Gravesite Plain": "Plaine sépulcrale",
    "Scadu Altus": "Altus Occulte",
    "Ancient Ruins of Rauh": "Ruines antiques de Rauh",
    "Cerulean Coast": "Côte céruléenne",
    "Charo's Hidden Grave": "Tombeau secret de Charo",
    "Jagged Peak": "Pic déchiqueté",
    "Scaduview": "Panorama occulte",
    "Abyssal Woods": "Bois abyssaux",
    "Enir-Ilim": "Enir-Ilim",
}


def patch(path: Path, names: dict[str, str]) -> int:
    text = path.read_text(encoding="utf-8")
    updated = 0
    for en_name, extras in EXTRAS.items():
        if not extras:
            continue
        region_name = names.get(en_name, en_name)
        pattern = (
            rf'(\[\[region\]\]\nname = """{re.escape(region_name)}"""\n'
            rf"subregions = \[)([^\]]+)(\])"
        )
        m = re.search(pattern, text)
        if not m:
            print(f"warn: region block not found for {region_name!r} in {path.name}")
            continue
        current = [int(x.strip()) for x in m.group(2).split(",") if x.strip()]
        merged = sorted(set(current) | set(extras))
        if merged == current:
            continue
        new_list = ", ".join(str(x) for x in merged)
        text = text[: m.start(2)] + new_list + text[m.end(2) :]
        updated += 1
        print(f"{path.name}: {region_name} +{sorted(set(extras) - set(current))}")
    if updated:
        path.write_text(text, encoding="utf-8", newline="\n")
    return updated


def main() -> None:
    en = ROOT / "crates/er_game_state/tables/en/bosses.toml"
    fr = ROOT / "crates/er_game_state/tables/fr/bosses.toml"
    n = patch(en, {k: k for k in EXTRAS})
    n += patch(fr, FR_NAMES)
    print(f"done ({n} region blocks updated)")


if __name__ == "__main__":
    main()
