#!/usr/bin/env python3
"""Verify bosses.toml coherence: regions, scaling, JSON alignment."""

import json
import re
import sys
from pathlib import Path

BossKey = tuple[int, int]

ROOT = Path(__file__).resolve().parents[1]
EN = ROOT / "crates/er_game_state/tables/en/bosses.toml"
FR = ROOT / "crates/er_game_state/tables/fr/bosses.toml"
JSON_EN = Path(r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data\engus\bosses.json")
JSON_FR = Path(r"G:\Elden_ring_tools\ER_boss_checklist_R\Overlay\assets\data\frafr\bosses.json")
RRL = Path(r"G:\Elden_ring_tools\RRL\Overlay\data\engus\bosses.json")
REF_SCALING = Path(r"c:\Users\sulli\Downloads\bosses.toml")

SCALING_RE = re.compile(r"^\[(\d+)\]\s*(.*)$", re.DOTALL)


def parse_toml(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    marker = re.search(r"\n(?:# Region -> subregion|\[\[region\]\])", text)
    boss_part = text[: marker.start()] if marker else text
    region_part = text[marker.start() :] if marker else ""

    bosses = {}
    occurrences: dict[int, int] = {}
    for chunk in re.split(r"(?=\[\[boss\]\])", boss_part):
        if "[[boss]]" not in chunk:
            continue
        block = {}
        for key, pat in [
            ("flag_id", r"^flag_id\s*=\s*(\d+)"),
            ("name", r'^name\s*=\s*"""(.*?)"""'),
            ("region", r'^region\s*=\s*"""(.*?)"""'),
            ("place", r'^place\s*=\s*"""(.*?)"""'),
            ("dlc", r"^dlc\s*=\s*(true|false)"),
        ]:
            m = re.search(pat, chunk, re.M)
            if m:
                val = m.group(1)
                if key == "flag_id":
                    block[key] = int(val)
                elif key == "dlc":
                    block[key] = val == "true"
                else:
                    block[key] = val
        if "flag_id" in block:
            fid = block["flag_id"]
            occurrence = occurrences.get(fid, 0)
            occurrences[fid] = occurrence + 1
            bosses[(fid, occurrence)] = block

    regions = {}
    region_order = []
    for chunk in re.split(r"(?=\[\[region\]\])", region_part):
        if "[[region]]" not in chunk:
            continue
        nm = re.search(r'^name\s*=\s*"""(.*?)"""', chunk, re.M)
        sr = re.search(r"^subregions\s*=\s*\[(.*?)\]", chunk, re.M | re.S)
        if nm and sr:
            name = nm.group(1)
            subs = [int(x.strip()) for x in sr.group(1).split(",") if x.strip()]
            regions[name] = subs
            region_order.append(name)

    display_order = []
    for line in text.splitlines():
        m = re.match(r'^region_display_order\s*=\s*\[(.*)\]', line)
        if m:
            display_order = [
                x.strip().strip('"').strip("'")
                for x in m.group(1).split(",")
                if x.strip()
            ]

    return {
        "bosses": bosses,
        "regions": regions,
        "region_order": display_order or region_order,
    }


def load_json(path: Path) -> dict:
    data = json.loads(path.read_text(encoding="utf-8"))
    bosses = {}
    occurrences: dict[int, int] = {}
    region_map = {}
    region_subs = {}
    order = []
    for region in data:
        rname = region["region_name"]
        order.append(rname)
        subs = region["regions"]
        region_subs[rname] = sorted(subs)
        for sid in subs:
            region_map[sid] = rname
        for b in region["bosses"]:
            fid = b["flag_id"]
            occurrence = occurrences.get(fid, 0)
            occurrences[fid] = occurrence + 1
            bosses[(fid, occurrence)] = {
                "boss": b["boss"],
                "place": b.get("place"),
                "region": rname,
            }
    return {
        "bosses": bosses,
        "region_subs": region_subs,
        "order": order,
    }


def load_rrl_scaling(path: Path) -> dict[BossKey, dict]:
    data = json.loads(path.read_text(encoding="utf-8"))
    out = {}
    occurrences: dict[int, int] = {}
    for region in data:
        for b in region["bosses"]:
            fid = b["flag_id"]
            occurrence = occurrences.get(fid, 0)
            occurrences[fid] = occurrence + 1
            out[(fid, occurrence)] = {
                "scaling": b.get("scaling"),
                "boss": b["boss"],
                "region": region["region_name"],
            }
    return out


def extract_scaling(name: str) -> tuple[int | None, str]:
    m = SCALING_RE.match(name)
    if m:
        return int(m.group(1)), m.group(2)
    return None, name


def expected_name_from_scaling(base: str, scaling: int | None) -> str:
    if scaling and scaling > 0:
        # RRL already includes prefix in boss field when scaling > 0
        return base
    return base


def main() -> int:
    errors = []
    warnings = []

    en = parse_toml(EN)
    fr = parse_toml(FR)
    json_en = load_json(JSON_EN)
    json_fr = load_json(JSON_FR)

    # --- Parse / structure ---
    print("=== Structure ===")
    print(f"EN bosses: {len(en['bosses'])}  FR bosses: {len(fr['bosses'])}")
    print(f"EN [[region]] blocks: {len(en['regions'])}  JSON regions: {len(json_en['region_subs'])}")

    if len(en["bosses"]) != 207:
        errors.append(f"EN boss count {len(en['bosses'])} != 207")
    if len(fr["bosses"]) != len(en["bosses"]):
        errors.append(f"FR count {len(fr['bosses'])} != EN count {len(en['bosses'])}")

    en_flags = set(en["bosses"])
    fr_flags = set(fr["bosses"])
    if en_flags != fr_flags:
        only_en = en_flags - fr_flags
        only_fr = fr_flags - en_flags
        if only_en:
            errors.append(f"flag_ids only in EN: {sorted(only_en)[:5]}...")
        if only_fr:
            errors.append(f"flag_ids only in FR: {sorted(only_fr)[:5]}...")

    # --- Region names in boss entries ---
    print("\n=== Zones (boss.region) ===")
    region_names = set(en["regions"])
    bad_region_refs = []
    for fid, b in en["bosses"].items():
        if b.get("region") not in region_names:
            bad_region_refs.append((fid, b.get("region")))
    if bad_region_refs:
        errors.append(f"{len(bad_region_refs)} bosses reference unknown region label")
        for fid, r in bad_region_refs[:5]:
            errors.append(f"  flag_id={fid} region={r!r}")

    # Boss region vs JSON
    region_mismatch_json = []
    for fid, jb in json_en["bosses"].items():
        tb = en["bosses"].get(fid)
        if not tb:
            region_mismatch_json.append((fid, "missing in TOML", jb["region"]))
        elif tb["region"] != jb["region"]:
            region_mismatch_json.append((fid, tb["region"], jb["region"]))
    missing_json = set(json_en["bosses"]) - set(en["bosses"])
    extra_toml = set(en["bosses"]) - set(json_en["bosses"])
    if missing_json:
        errors.append(f"{len(missing_json)} bosses in JSON missing from TOML")
    if extra_toml:
        errors.append(f"{len(extra_toml)} bosses in TOML not in JSON")
    print(f"Boss region vs JSON EN: {len(region_mismatch_json)} mismatches")
    for fid, toml_r, json_r in region_mismatch_json[:8]:
        print(f"  flag_id={fid}: TOML={toml_r!r} JSON={json_r!r}")
    if len(region_mismatch_json) > 8:
        print(f"  ... +{len(region_mismatch_json) - 8} more")

    # [[region]] subregions vs JSON
    subregion_mismatch = []
    for rname, json_subs in json_en["region_subs"].items():
        toml_subs = sorted(en["regions"].get(rname, []))
        if toml_subs != json_subs:
            subregion_mismatch.append((rname, toml_subs, json_subs))
    missing_region_blocks = set(json_en["region_subs"]) - set(en["regions"])
    extra_region_blocks = set(en["regions"]) - set(json_en["region_subs"])
    print(f"[[region]] subregions vs JSON: {len(subregion_mismatch)} mismatches")
    for rname, toml_s, json_s in subregion_mismatch[:5]:
        print(f"  {rname}: TOML={toml_s} JSON={json_s}")
    if missing_region_blocks:
        errors.append(f"JSON regions missing [[region]] block: {sorted(missing_region_blocks)}")
    if extra_region_blocks:
        warnings.append(f"Extra [[region]] blocks not in JSON: {sorted(extra_region_blocks)}")

    # Duplicate subregion IDs across regions
    seen_sub = {}
    dup_subs = []
    for rname, subs in en["regions"].items():
        for sid in subs:
            if sid in seen_sub:
                dup_subs.append((sid, seen_sub[sid], rname))
            seen_sub[sid] = rname
    if dup_subs:
        errors.append(f"{len(dup_subs)} duplicate subregion IDs in [[region]]")
        for sid, a, b in dup_subs[:5]:
            errors.append(f"  subregion {sid}: {a} and {b}")

    # Region display order
    if en["region_order"] != json_en["order"]:
        warnings.append("region order differs from JSON (may use region_display_order)")

    # FR regions vs JSON FR
    fr_region_mismatch = []
    for fid, jb in json_fr["bosses"].items():
        tb = fr["bosses"].get(fid)
        if tb and tb["region"] != jb["region"]:
            fr_region_mismatch.append((fid, tb["region"], jb["region"]))
    print(f"Boss region vs JSON FR: {len(fr_region_mismatch)} mismatches")

    # EN vs FR scaling prefix (region labels differ by locale — not compared here)
    en_fr_scale = []
    for fid in sorted(en_flags & fr_flags):
        es, _ = extract_scaling(en["bosses"][fid]["name"])
        fs, _ = extract_scaling(fr["bosses"][fid]["name"])
        if es != fs:
            en_fr_scale.append((fid, es, fs))
    print(f"EN vs FR scaling prefix mismatch: {len(en_fr_scale)}")

    # --- Scaling ---
    print("\n=== Scaling ===")
    rrl = load_rrl_scaling(RRL) if RRL.is_file() else {}
    ref_names = parse_toml(REF_SCALING)["bosses"] if REF_SCALING.is_file() else {}

    # Internal: name prefix vs RRL scaling field
    rrl_scaling_issues = []
    for fid, rb in rrl.items():
        tb = en["bosses"].get(fid)
        if not tb:
            continue
        scaling = rb.get("scaling")
        prefix, base = extract_scaling(tb["name"])
        rrl_prefix, rrl_base = extract_scaling(rb["boss"])
        if scaling == 0 or scaling is None:
            if prefix is not None:
                rrl_scaling_issues.append((fid, "RRL scaling=0 but TOML has prefix", tb["name"], rb["boss"]))
        else:
            if prefix != scaling:
                rrl_scaling_issues.append((fid, f"prefix {prefix} != scaling {scaling}", tb["name"], rb["boss"]))
            if tb["name"] != rb["boss"]:
                # name text differs (Tree Sentinel 1/2 etc.)
                if strip_base(tb["name"]) != strip_base(rb["boss"]) or prefix != rrl_prefix:
                    pass  # handled below as name mismatch
    rrl_name_only = []
    for fid, rb in rrl.items():
        tb = en["bosses"].get(fid)
        if not tb or tb["name"] == rb["boss"]:
            continue
        if not any(x[0] == fid for x in rrl_scaling_issues):
            rrl_name_only.append((fid, tb["name"], rb["boss"]))

    print(f"TOML vs RRL scaling rules: {len(rrl_scaling_issues)} issues")
    for row in rrl_scaling_issues[:10]:
        print(f"  flag_id={row[0]}: {row[1]}")
        print(f"    TOML: {row[2]}")
        print(f"    RRL:  {row[3]}")
    if len(rrl_scaling_issues) > 10:
        print(f"  ... +{len(rrl_scaling_issues) - 10} more")
    if rrl_name_only:
        print(f"RRL name diffs (hors règle scaling): {len(rrl_name_only)}")
        for fid, tname, rname in rrl_name_only:
            print(f"  flag_id={fid}: TOML={tname!r} RRL={rname!r}")

    # vs Downloads reference
    ref_mismatch = []
    for fid, tb in en["bosses"].items():
        ref = ref_names.get(fid)
        if ref and ref["name"] != tb["name"]:
            ref_mismatch.append((fid, tb["name"], ref["name"]))
    missing_ref = set(en["bosses"]) - set(ref_names)
    print(f"TOML EN vs Downloads/bosses.toml names: {len(ref_mismatch)} mismatches, {len(missing_ref)} missing in ref")

    # Stats on scaling prefixes
    with_prefix = sum(1 for b in en["bosses"].values() if extract_scaling(b["name"])[0] is not None)
    without_prefix = len(en["bosses"]) - with_prefix
    print(f"Bosses with [N] prefix: {with_prefix}, without: {without_prefix}")

    # --- Summary ---
    print("\n=== Résumé ===")
    if errors:
        print(f"ERREURS ({len(errors)}):")
        for e in errors:
            print(f"  - {e}")
    else:
        print("Aucune erreur structurelle.")

    if warnings:
        print(f"Avertissements ({len(warnings)}):")
        for w in warnings:
            print(f"  - {w}")

    ok_regions = (
        not bad_region_refs
        and not region_mismatch_json
        and not fr_region_mismatch
        and not subregion_mismatch
        and not dup_subs
        and not missing_json
        and not extra_toml
    )
    print(f"\nZones cohérentes avec JSON (EN+FR): {'OUI' if ok_regions else 'NON'}")
    print(f"Scaling cohérent avec RRL: {'OUI' if not rrl_scaling_issues else f'NON ({len(rrl_scaling_issues)} écarts)'}")
    print(f"Scaling identique à Downloads/bosses.toml: {'OUI' if not ref_mismatch else f'NON ({len(ref_mismatch)} écarts)'}")

    return 1 if errors else 0


def strip_base(name: str) -> str:
    _, base = extract_scaling(name)
    return base


if __name__ == "__main__":
    sys.exit(main())
