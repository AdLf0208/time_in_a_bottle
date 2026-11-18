# Time in a Bottle for Horizon

A clean-room Rust port of **Time in a Bottle 6.5.4** (MIT, RealMangoRage) to
Horizon (Minecraft Java 26.3). This is **slice 1**: the bottle, its stored-tick
account, and one target family — a furnace, smoker or blast furnace — advanced
by exactly the ticks the bottle spends.

One file installs it. `tiab.hmod` is a single SDK package (mod API `0.2`): the
item, its model, animated texture, eight lang files, recipe and block tag, and
the Rust code that stores and spends ticks, all in one archive under the pin in
`horizon-mod.toml`.

## What it does

- **Store.** While the bottle is carried, the first one in the inventory gains
  one stored tick per game tick (20 a second), up to 622,080,000 ticks. A
  second line tracks every tick ever stored.
- **Spend.** Right-click a furnace with the bottle: one use costs **600**
  stored ticks and starts an accelerator that gives the furnace one extra
  block tick per game tick for 600 game ticks — the furnace advances by
  exactly the ticks spent. Right-click again to double the rate (600 ticks
  buys 2x, 1,200 buys 4x, and so on), up to **128x**. The click is claimed, so
  the furnace's menu does not open.
- **Creative** pays nothing, as upstream's `canUse` allows; an unfilled bottle
  in survival pays nothing either and starts nothing.
- **Commands.** `/tiab time add|remove <player> <seconds>` (permission level 2)
  and `/tiab info <x> <y> <z> <n>`, a one-line readout of the clock, the
  bottle's account and the furnace at that position.

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

`runtime.rs` is the pure half (the account's arithmetic and one accelerator's
rate), byte-identical to the pre-SDK port; `glue.rs` is the game side. The gaps
(a saved accelerator, targets beyond the furnace family, other players'
inventories, config, particles) are listed in `notes.md`.

Licence: MIT; the carried assets are upstream's own files, re-created from the
audited jar by `tools/assemble_content.py`. `LICENSE` and `LICENSE-NOTICE.md`
travel inside the package.
