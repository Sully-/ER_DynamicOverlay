import json
from collections import defaultdict

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

EXTRA = {
    "NOKRON, ETERNAL CITY": [1202],
    "LAKE OF ROT": [1204],
}

JSON_PATH = r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data\engus\bosses.json"
OUT_PATH = r"G:\Elden_ring_tools\Overlay\crates\er_game_state\tables\regions_fragment.toml"

with open(JSON_PATH, encoding="utf-8") as f:
    data = json.load(f)

regions = defaultdict(set)
for entry in data:
    toml_name = MAPPING.get(entry["region_name"])
    if not toml_name:
        print(f"WARN unmapped: {entry['region_name']}")
        continue
    for sid in entry["regions"]:
        regions[toml_name].add(sid)

for name, ids in EXTRA.items():
    regions[name].update(ids)

lines = [
    "\n# Region -> subregion IDs (player map id / 1000). Derived from bosses.json.\n"
]
for name in sorted(regions.keys()):
    ids = sorted(regions[name])
    lines.append("[[region]]")
    lines.append(f'name = """{name}"""')
    lines.append(f"subregions = {ids}")
    lines.append("")

with open(OUT_PATH, "w", encoding="utf-8") as f:
    f.write("\n".join(lines))

print(f"Wrote {len(regions)} regions to {OUT_PATH}")
