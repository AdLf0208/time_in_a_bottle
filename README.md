# Time in a Bottle — a Horizon port

**Time in a Bottle** is a Minecraft mod. A mod for Minecraft - This item allows you to accelerate the rate at which blocks tick, ported to 1.16 - 1.21+. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [RealMangorage/time-in-a-bottle](https://github.com/RealMangorage/time-in-a-bottle) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Time in a Bottle ([source](https://github.com/RealMangorage/time-in-a-bottle)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `tiab.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

- **The bottle.** `tiab:time_in_a_bottle`, a stack-size-1 item on the inert
- **Storing.** At the end of every second the first bottle in the 36 main
- **Spending.** `TiabItem.useOn`, on the global `right_click_block` hook (which
- **Accelerating.** At `level_tick_post` each accelerator runs `rate` block
- **Commands.** `/tiab time add|remove <player> <seconds>` (permission level
- **Tooltip.** The two carried lang keys, `Stored time HH:MM:SS` and

## Not yet ported

- `LevelRef::block_tick` runs the block's **scheduled** tick
- `block_entity_tick`/`block_entity_ticks` are `BlockEntityPart` methods, and
- there is no `furnace`/`smelt`/`tick a block entity` host call.
- **Kept:** a furnace cooks and burns fuel at the accelerator's rate; the fuel
- **Lost:** one cook finishes per game tick **at most** and the remainder of the
- **The accelerator is not an entity and is not saved.** The API declares no
- **Only the furnace family is accelerated** (`minecraft:furnace`, `smoker`,
- **The command reaches only the acting player.** `/tiab time add <player>`

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `tiab.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Time in a Bottle was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/RealMangorage/time-in-a-bottle

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run tiab` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/tiab/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
