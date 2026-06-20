#!/usr/bin/env python3
"""Apply scaling-prefixed boss names from reference toml (names only)."""

import re
from pathlib import Path

REF = Path(r"c:\Users\sulli\Downloads\bosses.toml")
EN = Path(r"G:\Elden_ring_tools\Overlay\crates\er_game_state\tables\en\bosses.toml")
FR = Path(r"G:\Elden_ring_tools\Overlay\crates\er_game_state\tables\fr\bosses.toml")

SCALING_RE = re.compile(r"^\[(\d+)\]\s*(.*)$", re.DOTALL)


def parse_names(path: Path) -> dict[int, list[str]]:
    text = path.read_text(encoding="utf-8")
    out: dict[int, list[str]] = {}
    for chunk in re.split(r"(?=\[\[boss\]\])", text):
        if "[[boss]]" not in chunk:
            continue
        fm = re.search(r"^flag_id\s*=\s*(\d+)", chunk, re.M)
        nm = re.search(r'^name\s*=\s*"""(.*?)"""', chunk, re.M)
        if fm and nm:
            out.setdefault(int(fm.group(1)), []).append(nm.group(1))
    return out


def strip_scaling(name: str) -> str:
    m = SCALING_RE.match(name)
    return m.group(2) if m else name


def extract_scaling(name: str) -> tuple[str | None, str]:
    m = SCALING_RE.match(name)
    if m:
        return m.group(1), m.group(2)
    return None, name


def toml_str(value: str) -> str:
    return f'"""{value.replace(chr(34), chr(92) + chr(34))}"""'


def patch_names(path: Path, names_by_flag: dict[int, list[str]], fr_mode: bool = False) -> int:
    text = path.read_text(encoding="utf-8")
    marker = re.search(r"\n(?:# Region -> subregion|\[\[region\]\])", text)
    if not marker:
        raise SystemExit(f"region section not found in {path}")
    head = text[: marker.start()]
    tail = text[marker.start() :]
    updated = 0

    def patch_boss(m: re.Match[str]) -> str:
        nonlocal updated
        block = m.group(0)
        fm = re.search(r"^flag_id\s*=\s*(\d+)", block, re.M)
        if not fm:
            return block
        fid = int(fm.group(1))
        ref_names = names_by_flag.get(fid, [])
        if not ref_names:
            print(f"warn: flag_id {fid} missing in reference, skipping {path.name}")
            return block
        ref_name = ref_names.pop(0)

        if fr_mode:
            scale, _ = extract_scaling(ref_name)
            nm = re.search(r'^name\s*=\s*"""(.*)"""', block, re.M)
            if not nm:
                return block
            fr_base = strip_scaling(nm.group(1))
            new_name = f"[{scale}] {fr_base}" if scale else fr_base
        else:
            new_name = ref_name

        new_line = f"name = {toml_str(new_name)}"
        old = re.search(r"^name\s*=.*$", block, re.M)
        if old and old.group(0) != new_line:
            updated += 1
        return re.sub(r"^name\s*=.*$", new_line, block, count=1, flags=re.M)

    new_head = re.sub(
        r"\[\[boss\]\][\s\S]*?(?=\n\[\[boss\]\]|\Z)",
        patch_boss,
        head,
    )
    path.write_text(new_head + tail, encoding="utf-8", newline="\n")
    return updated


def main() -> None:
    ref_names = parse_names(REF)
    print(f"reference: {sum(len(names) for names in ref_names.values())} names")
    en_n = patch_names(EN, ref_names, fr_mode=False)
    fr_n = patch_names(FR, parse_names(REF), fr_mode=True)
    print(f"en: updated {en_n} names")
    print(f"fr: updated {fr_n} names")


if __name__ == "__main__":
    main()
