# Time in a Bottle for Horizon

A clean-room Rust port of **Time in a Bottle 6.5.4** (MIT, RealMangoRage) to
Horizon (Minecraft Java 26.3). The bottle, its stored-tick account, and the
block entities it can drive: the **furnace family** (furnace, smoker, blast
furnace) and the **campfire family** (campfire, soul campfire), advanced by
exactly the ticks the bottle spends.

One file installs it. `tiab.hmod` is a single SDK package (mod API `0.2`): the
item, its model, animated texture, eight lang files, recipe and block tag, and
the Rust code that stores and spends ticks, all in one archive under the pin in
`horizon-mod.toml`.

## What it does

- **Store.** While the bottle is carried, the first one in the inventory gains
  one stored tick per game tick (20 a second), up to 622,080,000 ticks. A
  second line tracks every tick ever stored.
- **Spend.** Right-click a furnace or a campfire with the bottle: one use costs
  **600** stored ticks and starts an accelerator that gives the target one
  extra block tick per game tick for 600 game ticks — a furnace cooks, and a
  campfire roasts, at twice the speed for that long. Right-click again to double
  the rate (600 ticks buys 2x, 1,200 buys 4x, and so on), up to **128x**. The
  click is claimed, so a furnace's menu does not open and a campfire's food slot
  is not used.
- **Creative** pays nothing, as upstream's `canUse` allows; an unfilled bottle
  in survival pays nothing either and starts nothing.
- **Commands.** `/tiab time add|remove <player> <seconds>` (permission level 2)
  and `/tiab info <x> <y> <z> <n>`, a one-line readout of the clock, the
  bottle's account and the target's family, timer, items and fuel.

## Build and check

```sh
cargo test                                              # the pure half
cargo clippy --all-targets -- -D warnings
cargo fmt --check
rm -f Cargo.lock                                        # it would be packed
cargo horizon-mod build                                 # target/horizon-mod/tiab.hmod
cargo horizon-mod check tiab.hmod
cargo horizon-mod fits tiab.hmod --build nightly
```

`runtime.rs` is the pure half (the account's arithmetic, one accelerator's rate
and the two families' timer rules); `glue.rs` is the game side. The gaps (a
saved accelerator, hoppers and mod machines as targets, other players'
inventories, config, sounds and particles) are listed in `notes.md`.

Licence: MIT; the carried assets are upstream's own files, re-created from the
audited jar by `tools/assemble_content.py`. `LICENSE` and `LICENSE-NOTICE.md`
travel inside the package.
