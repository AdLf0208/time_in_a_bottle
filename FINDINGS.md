# Time in a Bottle — findings (jar inventory + upstream behaviour)

Ported artefact: `ports/_shared/licence-cache/tiab-neoforge-6.5.4.jar`
(88 entries, kept as licence/provenance evidence). Licence **MIT**, read from
`META-INF/neoforge.mods.toml` (`license = "MIT"`) and the jar's
`LICENSE_Time In A Bottle` (MIT, Copyright (c) 2024 RealMangoRage). Rank 0.

Upstream analysis: `https://github.com/RealMangorage/time-in-a-bottle`,
commit **`b8bb6d62f379d2d132193c10b6651e029057895e`** (2025-12-20), the last
1.21.1 state of the repo (`gradle.properties` says `version=6.5.3`,
`minecraft_version=1.21.1`; the next commits move to 26.x and version 8.x).
The jar's `neoforge.mods.toml` says 6.5.4, so the pack file is one publish ahead
of the last 1.21.1 commit; no source difference was observed in the classes the
slice uses. The clone was deleted after this file was written.

## Jar inventory (measured with `unzip -l`, not the mod's name)

| Family | Entries | Notes |
|---|---|---|
| Java classes | 25 top-level/nested | `TiabItem`, `TimeAcceleratorEntity`, `StoredTimeComponent`, `TiabCommand`, `TiabMod`, NeoForge registration/config, JEI + EMI plugins, a client renderer |
| Item model | 1 | `assets/tiab/models/item/time_in_a_bottle.json` (parent `item/generated`) |
| Item texture | 1 (+`.mcmeta`) | `assets/tiab/textures/item/time_in_a_bottle.png`, animated: `frametime: 2`, `interpolate: true` |
| Lang files | 8 | `en_us`, `es_ar`, `es_es`, `es_mx`, `pt_br`, `tr_tr`, `uk_ua`, `zh_cn` |
| Recipes | 1 | `data/tiab/recipe/time_in_a_bottle.json`, shaped `GGG / DCD / LBL` (gold, diamond, lapis, clock, glass bottle) |
| Block tags | 1 | `data/tiab/tags/block/un_acceleratable.json` = `[minecraft:chest, minecraft:grass_block]` |
| Blocks / blockstates / block models / loot / advancements | **0** | the mod adds no block |

The dispatch brief said "3 item tags"; the artefact has **no item tags** — one
block tag. The artefact wins (AGENTS.md, "A brief may not invent a mod's
contents"). The brief's "one item" is correct, but the mod also carries an
**entity class** and a **command**, which the slice has to account for.

`assets/tiab/lang/en_us.json` keys (carried):
`item.tiab.time_in_a_bottle`, `.tooltip.stored_time` ("Stored time %s:%s:%s"),
`.tooltip.total_accumulated_time` ("Total accumulated time %s:%s:%s"),
`emi.category.tiab.test`.

## Behaviour extracted from the upstream source (files read)

`neoforge/.../core/Registration.java`
: the item is `max_stack_size 1`, its default component is
  `tiab:stored_time` = `{stored: 0, total: 0}`; the accelerator entity is
  `tiab:accelerator`, 1x1, `MobCategory.MISC`; a creative tab holds the item.

`common/.../items/TiabItem.java`
: `tickPlayer` finds the first bottle in the 36 main inventory slots and
  `tickBottle(stack, 20)`; `tickBottle` adds `ticks` to `stored` (capped at
  `MAX_STORED_TIME`) and to `total` (uncapped), then rewrites the two lore
  lines. `useOn`:

  - refuses (FAIL) when the clicked block has no block entity **and** does not
    randomly tick, or when the block is in `#tiab:un_acceleratable`;
  - if an accelerator already exists on that block: `nextRate = rate * 2`,
    `timeAdded = (600 - remaining) / 2`, cost `rate/2 * 600` at the new rate;
    at `rate >= 2^(MAX_RATE_MULTI-1)` it does nothing;
  - else it creates an accelerator with `remaining = 600` and `rate = 1`, cost
    600;
  - creative mode pays nothing; otherwise the cost comes out of `stored`;
  - success plays a sound.

`common/.../entities/TimeAcceleratorEntity.java`
: every server tick, for `i in 0..rate`: tick the target block entity through
  its own `BlockEntityTicker`; a block with no ticker but `isRandomlyTicking`
  gets one `randomTick` at a 1/1365 chance; anything else removes the
  accelerator. Then `remaining -= 1`; at `<= 0` it removes itself. Position,
  rate and remaining are saved in the entity's NBT.

`common-api/.../ITiabConfig.java` (the NeoForge config's defaults)
: `max_stored_time = 622_080_000` ticks, `ticks_const = 20`,
  `each_use_duration = 30` s, `max_rate_multi = 8`.

`common/.../commands/TiabCommand.java`
: `/tiab time add|remove <player> <seconds>`, permission level 2; seconds are
  clamped to `max_stored_time / ticks_const`; the amount moved is
  `seconds * 20`; refuses when the target has no bottle ("No Time in a bottle
  item in inventory"); answers with an action-bar line.

`neoforge/.../NeoForgeTiabMod.java`, `common/.../TiabMod.java`
: `PlayerTickEvent.Post` → `tickPlayer`, and the accumulator only runs when
  `server.getTickCount() % 20 == 0` (20 ticks a second, one stored tick per
  game tick).

`common/.../misc/CommonHelper.java`
: the tooltip format is `HH:MM:SS` of `stored / 20` seconds (and `total / 20`).

## Numbers the port must keep (all from the files above)

| Value | Source |
|---|---|
| storage cap 622,080,000 ticks | `ITiabConfig.MAX_STORED_TIME()` |
| 20 ticks per accumulation step, one step a second | `TICKS_CONST()`, `TiabMod.tickPlayer` |
| 30 s per use ⇒ 600 ticks of acceleration | `EACH_USE_DURATION()`, `TiabItem.getEachUseDuration` |
| first use costs 600 stored ticks (rate 1) | `TiabItem.getEnergyCost(1)` |
| an upgrade doubles the rate, costs `rate/2 * 600` at the new rate, adds half the used-up time | `TiabItem.useOn` |
| rate ceiling 128 (`2^(8-1)`) | `MAX_RATE_MULTI()` |
| a random-ticking target's chance is 1/1365 per accelerated tick | `TimeAcceleratorEntity.tick` |

## Where the rewrite differs (pin limits, recorded in `notes.md`)

- The accelerator is **not an entity**: the pin has no mod entity
  registration. The same `{dimension, pos, rate, remaining}` state lives in the
  port (in memory), ticked at `level_tick_post`.
- Only a **furnace** target is accelerated in this slice (the pin exposes a
  vanilla furnace's own tick routine to an injected module); other block
  entities and random-ticking blocks are the gap list.
- The bottle's component is `tiab:stored_time` (API 65), with the pin's
  component helpers instead of NeoForge's `DataComponents` write.
