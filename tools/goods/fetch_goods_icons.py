#!/usr/bin/env python3
"""Fetch goods icon PNGs listed in crates/er_game_state/tables/goods.toml.

Sources (in order):
  1. Local Elden Ring install (ELDEN_RING_GAME or common Steam paths)
  2. GitHub elden-ring-media (base-game icons)
  3. EldenRing-SaveForge GitHub (DLC talismans etc.)
  4. SaveForge PNGs already in assets/icons (_sf_kindling.png, _sf_scadu.png)

Usage:
  python tools/goods/fetch_goods_icons.py
  python tools/goods/fetch_goods_icons.py --out assets/icons
  python tools/goods/fetch_goods_icons.py --verify-ids
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import tomllib
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GOODS_TOML = ROOT / "crates" / "er_game_state" / "tables" / "goods.toml"
OUT_DIR = ROOT / "assets" / "icons"
PROBE_PROJECT = ROOT / "scripts" / "_param_probe" / "Probe.csproj"

SMITHBOX_REG = Path(
    r"G:\Elden_ring_tools\Modding\Smithbox\code\Smithbox\src\Smithbox.Data"
    r"\Assets\PARAM\ER\Regulations\1.16.1 (11611000)\regulation.bin"
)

GITHUB_MEDIA = (
    "https://raw.githubusercontent.com/elden-ring-playground/elden-ring-media/main"
)

SAVEFORGE_RAW = (
    "https://raw.githubusercontent.com/oisis/EldenRing-SaveForge/main"
    "/frontend/public/items"
)

GAME_SEARCH_PATHS = [
    Path(r"G:\Steam\steamapps\common\ELDEN RING\Game"),
    Path(r"C:\Program Files (x86)\Steam\steamapps\common\ELDEN RING\Game"),
    Path(r"D:\Steam\steamapps\common\ELDEN RING\Game"),
    Path(r"G:\SteamLibrary\steamapps\common\ELDEN RING\Game"),
    Path(r"E:\SteamLibrary\steamapps\common\ELDEN RING\Game"),
]

MENU_SUBPATHS = (
    "menu/low/00_solo",
    "menu/hi/00_solo",
)

TALISMAN_GROUPS = frozenset({"talisman", "negative_talismans"})

# SaveForge-style exports kept in-repo as manual fallbacks for DLC items.
SAVEFORGE_LOCAL: dict[str, str] = {
    "kindling.png": "_sf_kindling.png",
    "scadutree.png": "_sf_scadu.png",
}


def resolve_file(entry: dict) -> str:
    return entry.get("file") or f"{entry['key']}.png"


def load_goods() -> tuple[str, list[dict]]:
    data = tomllib.loads(GOODS_TOML.read_text(encoding="utf-8"))
    prefix = data.get("texture_prefix", "MENU_Knowledge")
    goods = [g for g in data["good"] if "icon_id" in g]
    for g in goods:
        g["file"] = resolve_file(g)
    return prefix, goods


def game_root() -> Path | None:
    env = Path(os.environ["ELDEN_RING_GAME"]) if "ELDEN_RING_GAME" in os.environ else None
    candidates = ([env] if env else []) + GAME_SEARCH_PATHS
    for path in candidates:
        if path and (path / "menu").is_dir():
            return path
    return None


def local_png(game: Path, prefix: str, icon_id: int) -> Path | None:
    name = f"{prefix}_{icon_id:05d}.png"
    for sub in MENU_SUBPATHS:
        candidate = game / sub / name
        if candidate.is_file():
            return candidate
    return None


def download_url(url: str) -> bytes | None:
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "fetch_goods_icons.py"})
        with urllib.request.urlopen(req, timeout=20) as resp:
            return resp.read()
    except urllib.error.HTTPError:
        return None
    except urllib.error.URLError as err:
        print(f"  network error for {url}: {err}", file=sys.stderr)
        return None


def download_saveforge(entry: dict) -> bytes | None:
    group = entry.get("group", "")
    key = entry["key"]
    if group in TALISMAN_GROUPS:
        subpath = f"talismans/{key}.png"
    elif group == "smithing_stones":
        subpath = f"smithing_stones/{key}.png"
    else:
        subpath = f"goods/{key}.png"
    return download_url(f"{SAVEFORGE_RAW}/{subpath}")


def download_github(prefix: str, icon_id: int) -> bytes | None:
    name = f"{prefix}_{icon_id:05d}.png"
    for sub in MENU_SUBPATHS:
        url = f"{GITHUB_MEDIA}/{sub}/{name}"
        data = download_url(url)
        if data:
            return data
    return None


def verify_icon_ids() -> int:
    if not PROBE_PROJECT.is_file():
        print("Probe project missing; skip --verify-ids", file=sys.stderr)
        return 1
    if not SMITHBOX_REG.is_file():
        print(f"Smithbox regulation not found: {SMITHBOX_REG}", file=sys.stderr)
        return 1
    print("Verifying icon_id via Smithbox regulation (DecryptERRegulation)...")
    result = subprocess.run(
        ["dotnet", "run", "--project", str(PROBE_PROJECT), "--", "--verify-ids"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(result.stderr or result.stdout, file=sys.stderr)
        return result.returncode
    print(result.stdout)
    return 0


def fetch_one(entry: dict, prefix: str, game: Path | None, out_dir: Path) -> bool:
    icon_id = int(entry["icon_id"])
    dest = out_dir / entry["file"]
    key = entry["key"]

    if dest.is_file():
        print(f"  skip {key}: {dest.name} already exists")
        return True

    if game is not None:
        src = local_png(game, prefix, icon_id)
        if src is not None:
            shutil.copy2(src, dest)
            print(f"  ok   {key}: copied {src.relative_to(game)}")
            return True

    data = download_github(prefix, icon_id)
    if data:
        dest.write_bytes(data)
        print(f"ok   {key}: downloaded from elden-ring-media ({icon_id:05d})")
        return True

    data = download_saveforge(entry)
    if data:
        dest.write_bytes(data)
        print(f"  ok   {key}: downloaded from EldenRing-SaveForge")
        return True

    fallback = out_dir / SAVEFORGE_LOCAL.get(dest.name, "")
    if fallback.is_file():
        shutil.copy2(fallback, dest)
        print(f"  ok   {key}: copied SaveForge fallback {fallback.name}")
        return True

    print(
        f"  miss {key}: icon_id={icon_id:05d} — export via Smithbox File Browser "
        f"({prefix}_{icon_id:05d}.png) or set ELDEN_RING_GAME",
        file=sys.stderr,
    )
    return False


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--verify-ids",
        action="store_true",
        help="Run dotnet probe against Smithbox regulation (no download)",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=OUT_DIR,
        help=f"Output directory (default: {OUT_DIR})",
    )
    args = parser.parse_args()

    if args.verify_ids:
        return verify_icon_ids()

    prefix, goods = load_goods()
    out_dir = args.out
    out_dir.mkdir(parents=True, exist_ok=True)

    game = game_root()
    if game:
        print(f"Game install: {game}")
    else:
        print("No local Elden Ring install found (set ELDEN_RING_GAME for DLC icons)")

    by_file: dict[str, list[dict]] = {}
    for entry in goods:
        by_file.setdefault(entry["file"], []).append(entry)

    ok = 0
    fail = 0
    for file_name, entries in by_file.items():
        representative = entries[0]
        if fetch_one(representative, prefix, game, out_dir):
            ok += 1
            dest = out_dir / file_name
            for extra in entries[1:]:
                extra_dest = out_dir / extra["file"]
                if not extra_dest.is_file() and dest.is_file():
                    shutil.copy2(dest, extra_dest)
                    print(f"  copy {extra['key']} -> {extra_dest.name}")
        else:
            fail += 1

    print(f"Done: {ok} unique file(s), {fail} missing")
    if fail:
        print(
            "Missing icons: place SaveForge exports as _sf_kindling.png / _sf_scadu.png, "
            "or set ELDEN_RING_GAME for in-game MENU_Knowledge PNGs.",
            file=sys.stderr,
        )
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
