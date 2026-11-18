# Time in a Bottle — carried content (`assets/tiab/`, `data/tiab/`)

Built by [`tools/assemble_content.py`](assemble_content.py) from
`ports/_shared/licence-cache/tiab-neoforge-6.5.4.jar`
(SHA-256 `0beb4ff1…6325`). Nothing is invented: the model, the animated
texture, the eight lang files, the shaped recipe and the block tag are the
jar's own MIT files, re-spelled only where 26.3 differs. The item declaration
and the creative tab are written here because the jar defines them in Java
(`neoforge/.../core/Registration.java`).

The files live at the **port root** (`assets/tiab/…`, `data/tiab/…`), not in a
separate `content/` folder: the SDK package is one flat `.hmod` that carries the
assets, the data, the manifest and the Rust source together. Re-running the
assembler re-creates exactly these 16 files (verified byte-for-byte: the same
SHA-256 for every one).

## Carried byte-for-byte from the jar

| File | What it is |
|---|---|
| `assets/tiab/models/item/time_in_a_bottle.json` | the item model (`item/generated`) |
| `assets/tiab/textures/item/time_in_a_bottle.png` (+`.mcmeta`) | the texture, animated (`frametime: 2`, `interpolate: true`) |
| `assets/tiab/lang/{en_us,es_ar,es_es,es_mx,pt_br,tr_tr,uk_ua,zh_cn}.json` | the eight lang files, including the two tooltip keys |
| `data/tiab/recipe/time_in_a_bottle.json` | shaped `GGG / DCD / LBL` (gold, diamond, lapis, clock, glass bottle) |
| `data/tiab/tags/block/un_acceleratable.json` | `[minecraft:chest, minecraft:grass_block]` |

The recipe is upgraded the way the carry pipeline does it: `{"item": "x"}` in
`key` becomes the bare id `"x"` (26.3's ingredient spelling), plus `category`
and `group`.

## Written here (from the Java side of the jar)

| File | Why |
|---|---|
| `assets/tiab/items/time_in_a_bottle.json` | 26.3's item model definition (`minecraft:model` → `tiab:item/time_in_a_bottle`) |
| `data/tiab/horizon/item/time_in_a_bottle.json` | the item: `minecraft:clock` template (inert, no use), `max_stack_size: 1`, default component `tiab:stored_time = {stored: 0, total: 0}` — `Registration.TIAB_ITEM` |
| `data/tiab/horizon/creative_tab/tiab.json` | upstream's own creative tab (`Registration.TIAB_CREATIVE_TAB`) |

The component the item names as a default is declared by the package's own
`component_types` hook (`src/lib.rs`), so it exists before content loads.

## Retired with the `.hnmod` format

The pre-migration port built these files into a separate `content/tiab.hmod`
and packed that inside the `.hnmod` as `mixins/tiab/data/tiab-content.hmod`
(the AE2 embedded-content pattern), then wrote it out at startup. One flat
`.hmod` now carries both halves, so the embedded copy and its `content_root`
helper are gone: `ports/_shared/evidence/tiab/retired-hnmod-0.1.0/`.
