#!/usr/bin/env python3
"""Assemble the carried `assets/` and `data/` of the Time in a Bottle package.

The jar is the ATM10 file (`ports/_shared/licence-cache/tiab-neoforge-6.5.4.jar`),
kept as the licence and provenance evidence. Every asset is taken from it:
the item model, the animated texture and its `.mcmeta`, all eight lang files,
the shaped recipe (re-spelled for 26.3's plain-string ingredients) and the
`#tiab:un_acceleratable` block tag. The item declaration (its vanilla template,
`max_stack_size` and default `tiab:stored_time` component) and the creative tab
are written here because the jar defines them in Java (`Registration.java`).

The package is one flat SDK layout, so the files land at the port root
(`assets/tiab/…`, `data/tiab/…`) beside `src/` and the manifest, not in a
separate content folder. Rerun it to rebuild them (it deletes the two generated
folders first):

    python3 tools/assemble_content.py
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import zipfile

ROOT = Path(__file__).resolve().parents[1]
JAR = ROOT.parent / "_shared/licence-cache/tiab-neoforge-6.5.4.jar"
CONTENT = ROOT
MOD = "tiab"
# SHA-256 of the pack's jar, the licence evidence (`audit_licences.py --mod tiab`).
JAR_SHA256 = "0beb4ff1d81facfe9039e8d657a75836d1e8c13c137c90b491cc07e763f26325"

CARRIED = [
    "assets/tiab/models/item/time_in_a_bottle.json",
    "assets/tiab/textures/item/time_in_a_bottle.png",
    "assets/tiab/textures/item/time_in_a_bottle.png.mcmeta",
    "assets/tiab/lang/en_us.json",
    "assets/tiab/lang/es_ar.json",
    "assets/tiab/lang/es_es.json",
    "assets/tiab/lang/es_mx.json",
    "assets/tiab/lang/pt_br.json",
    "assets/tiab/lang/tr_tr.json",
    "assets/tiab/lang/uk_ua.json",
    "assets/tiab/lang/zh_cn.json",
    "data/tiab/recipe/time_in_a_bottle.json",
    "data/tiab/tags/block/un_acceleratable.json",
]

# 26.3's item model definition; the model itself is the jar's own file.
ITEM_DEFINITION = {"model": {"type": "minecraft:model", "model": f"{MOD}:item/time_in_a_bottle"}}

# `Registration.TIAB_ITEM`: a plain item (no vanilla template behaviour is
# wanted, so the inert `minecraft:clock` is the template), stack size 1, and
# the default `tiab:stored_time` component `{stored: 0, total: 0}`.
ITEM = {
    "template": "minecraft:clock",
    "max_stack_size": 1,
    "components": {f"{MOD}:stored_time": {"stored": 0, "total": 0}},
}

# Upstream's own creative tab (`Registration.TIAB_CREATIVE_TAB`, titled with
# the item's translation key).
TAB = {
    "title": "item.tiab.time_in_a_bottle",
    "icon": f"{MOD}:time_in_a_bottle",
    "items": [f"{MOD}:time_in_a_bottle"],
}

def upgrade_recipe(recipe: dict) -> dict:
    """26.3 ingredient spelling: a bare item id, or `#tag`, not `{"item": …}`."""
    for key, value in list(recipe.get("key", {}).items()):
        if isinstance(value, dict) and "item" in value:
            recipe["key"][key] = value["item"]
        elif isinstance(value, dict) and "tag" in value:
            recipe["key"][key] = f"#{value['tag']}"
    recipe.setdefault("category", "misc")
    recipe.setdefault("group", "")
    return recipe


def write(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(data, bytes):
        path.write_bytes(data)
    else:
        path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def main() -> None:
    raw = JAR.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != JAR_SHA256:
        raise SystemExit(f"jar SHA-256 {digest} is not the audited record {JAR_SHA256}")
    archive = zipfile.ZipFile(JAR)
    for folder in ("assets", "data"):
        shutil.rmtree(CONTENT / folder, ignore_errors=True)

    for name in CARRIED:
        data = archive.read(name)
        if name.endswith("recipe/time_in_a_bottle.json"):
            data = (json.dumps(upgrade_recipe(json.loads(data)), indent=2) + "\n").encode()
        write(CONTENT / name, data)

    write(CONTENT / f"assets/{MOD}/items/time_in_a_bottle.json", ITEM_DEFINITION)
    write(CONTENT / f"data/{MOD}/horizon/item/time_in_a_bottle.json", ITEM)
    write(CONTENT / f"data/{MOD}/horizon/creative_tab/tiab.json", TAB)

    files = sorted(
        path
        for folder in ("assets", "data")
        for path in (CONTENT / folder).rglob("*")
        if path.is_file()
    )
    print(f"jar sha256 {digest}")
    print(f"{len(files)} files under {CONTENT}")
    for path in files:
        print(" ", path.relative_to(CONTENT))


if __name__ == "__main__":
    main()
