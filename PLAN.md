# Time in a Bottle — slice 1 plan

> **Historical (0.1.0).** This is the plan the retired `.hnmod` slice was built
> from: `.hnmod`, `mixins/<id>/`, per-hook helper files, `support.rs` and the
> embedded-content helper no longer exist. The flat SDK package that replaced it,
> and how each injection became a `Hooks`/`ItemPart` method, are in
> [`notes.md`](notes.md) → `## Migration to the SDK format`. The behaviour, the
> account's arithmetic and the test list below still hold.

**Goal.** `tiab:time_in_a_bottle` stores ticks while it is carried and spends
them accelerating one block entity: a furnace advances by exactly the ticks
spent. One `.hnmod` carries the item and the behaviour (AE2 embedded-content
pattern).

**Approach** (from `horizon/docs/modpacks.md`, `logic` tier): one item plus
`use`/tick hooks — the whole mod is a stored quantity and the tick semantics
that spend it. No block, no menu, no screen.

**Shape of the slice**

1. Content (`content/tiab.hmod`): the jar's model, animated texture, eight lang
   files, the shaped recipe, `#tiab:un_acceleratable`; a 26.3 item definition
   on the `minecraft:clock` template (`max_stack_size: 1`, default component
   `tiab:stored_time = {stored: 0, total: 0}`), and a creative tab.
2. Pure runtime (`runtime.rs`, TDD): `StoredTime` (accumulate/spend/remove,
   cap), the accelerator's `{rate, remaining}` arithmetic (600-tick use,
   doubling, `rate/2 * 600` cost, 128 ceiling), `HH:MM:SS` clock text, seconds
   → ticks with upstream's clamp.
3. Glue (`support.rs` + one helper per injection):
   - `component_types` — register `tiab:stored_time`;
   - `inventory_tick` — each second, the first bottle in the inventory gains
     `TICKS_CONST` stored (and total), capped; the player's screen is synced at
     `server_tick` End;
   - `right_click_block` — with the bottle in the acting hand, on a furnace that
     is not in `#tiab:un_acceleratable`: create or upgrade the accelerator,
     charge `stored`, claim the click (the furnace menu does not open);
   - `level_tick_post` — the target furnace gets `rate` of its own ticks this
     game tick; the accelerator loses one tick and ends at 0;
   - `register_commands` / `perform_command` — `/tiab time add|remove <player>
     <seconds>` at permission 2;
   - `tooltip` — the two stored/total lines;
   - `content_root` — write the embedded content out and load it.
4. Ladder: `cargo test` (red → green), fmt, clippy, `horizon.py stage → check
   → test → build`, package, `cargo horizon-mod check` both kinds.
5. E2E (`tools/e2e.sh`, The End arena): store by carrying, give by command,
   spend on a furnace, measure the furnace's `cooking_time_spent` through
   `/data get block` and the output count against a control furnace; a
   `SELFTEST=1` leg that fails; an `EMBEDDED=1` leg with no `--dev-mod` plus an
   unusable-`.horizon-cache` control.

**Write ownership:** `ports/tiab/**`, `ports/_shared/evidence/tiab/**`.

**Stop conditions:** a stamped hook missing from the pin, an artefact that
contradicts the brief, or a build that cannot reach the pin's source hash →
report, do not invent an API.
