# Time in a Bottle — validation record (0.2.3)

This file describes **the build that produced the delivered `tiab.hmod`**: the
pins, the toolchain, the ladder, the in-game legs and what is not covered. It
travels with the source tag so a reviewer can match the archive to a run.

## Identity

| Field | Value |
|---|---|
| Mod | `tiab` — Time in a Bottle (port of RealMangoRage's 6.5.4, MIT) |
| Version | **0.2.3** (`horizon-mod.toml`, `Cargo.toml`) |
| Format | one flat `.hmod`, `api = "0.2"`, package format `native-1`, `side = "both"`, `capabilities = ["registries"]` |
| Horizon pin | `0a875da91dbb0fdcb7ba8c42c11341aa229af51a` (Nightly 2026-11-20) |
| Mixins pin | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| Game | Minecraft Java 26.3 |
| Toolchain | the pin's (Rust 1.99.1; built here with the launcher's toolchain, edition 2024) |
| Launcher | 0.2.92 (`launcher-game.sh` verified its SHA256SUMS signature) |

## Artifact

| | |
|---|---|
| File | `ports/tiab/tiab.hmod` |
| Size | **33,762 B** |
| SHA-256 | `c5fcfe581c7bc0b0dc1e6bbefa4b2470d3588dcd6c7284d76a033ec50d5f1ab6` |
| Entries | **25** (`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `uses.json`, `src/{lib,glue,runtime}.rs`, the 16 `assets/`+`data/` files) |
| Packed `Cargo.lock` | none (25 entries; `.gitignore` keeps it out, `rm -f Cargo.lock` before each build) |
| Reproducible | two builds with `rm -f Cargo.lock` before each are `cmp`-identical |

## Ladder (all through `ports/tools/build-slot.sh`, `CARGO_INCREMENTAL=0`)

| Step | Command | Result |
|---|---|---|
| Unit tests | `cargo test --offline` | 28 passed, 0 failed |
| Lints | `cargo clippy --offline --all-targets -- -D warnings` | exit 0, 0 warnings |
| Formatting | `cargo fmt --check` (no `--all`: the fetched SDK is a path dependency) | clean |
| Build ×2 | `cargo horizon-mod build` | `cmp`-identical |
| Package check | `cargo horizon-mod check tiab.hmod` | exit 0, `tiab 0.2.3 (native-mod, format native-1/0a875da/e59da90, for 26.3)` |
| Fits the pin | `cargo horizon-mod fits tiab.hmod --build 0a875da` | `0a875da  made for`, exit 0 |
| Fits another build | `cargo horizon-mod fits tiab.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 |
| Package gate | `python3 ports/tools/verify-sdk-package.py tiab` | 1/1 (runtime byte-identical tree↔package) |

Logs: `ports/_shared/evidence/tiab/sdk-{test,clippy,fmt,build-twice,check,fits,fits-other}.log`;
each opens with the package sha256 and size it ran against. The red run of the new
family's tests is `sdk-test-red-campfire.log`.

## In game

Game: `ports/tools/launcher-game.sh build tiab` → `Compiling tiab (1 of 1)` from
this package's `src/` and `horizon-mod.toml` (byte-identical to the delivered
file's), launched `--offscreen --creative --headless`-less (offscreen) in a
scratch home, The End arena at x=200.

| Leg | Result | Log |
|---|---|---|
| main (phases A–H) | `PASS` + `EXIT:0` | `tiab-e2e/e2e-launcher.log` |
| control: the same Nightly without the package | `FAIL` at phase A (the item id does not exist) + `EXIT:1` | `tiab-e2e/e2e-launcher-control.log` |
| counterfactual: `SELFTEST=1` | `FAIL` (campfire timer one tick off) + `EXIT:1` | `tiab-e2e/e2e-launcher-selftest.log` |

`python3 ports/tools/check-e2e-logs.py tiab-e2e --strict` → `logs=3 ok=3 missing-exit=0 bad-exit=0`.

## What the in-game legs assert

- A: the bottle registers and is read back; an unknown `tiab:` id is refused; three furnaces cook.
- B: carrying the bottle stores exactly one tick per game tick (20 a second).
- C: creative mode pays nothing; the use runs its 600-tick budget over the control's baseline.
- D: an unfilled bottle pays nothing and starts nothing in survival; no menu opens.
- E: a paid use costs exactly 600 stored ticks; a second click cannot upgrade it.
- F: 1,200 stored ticks buy two uses (rate 2), a third cannot reach rate 4; the accelerated ticks
  are within `rate × remaining − rate × cooks` (the fast-forward's measured loss bound).
- G: a campfire is a target; its first slot advances by exactly the game ticks it lived through
  plus the ticks the accelerator owed, while the control campfire advances by its clock alone; the
  accelerated potato finishes and drops as `minecraft:baked_potato` while the control is still
  cooking.
- H: the carried shaped recipe crafts exactly one bottle in a real crafting table and consumes
  every ingredient; a grid without the clock and the clock alone craft nothing.

## Not covered (recorded, not implied)

- Hoppers, brewing stands and mod machines as targets: no API runs a vanilla block entity's tick.
- Randomly-ticking blocks at the jar's 1/1365 chance: no API answers "does this state tick
  randomly".
- A saved accelerator (the table is in memory), a config file, sounds/particles, the client
  overlay, JEI/EMI, and `/tiab time` targeting another player.
- `#tiab:un_acceleratable` is carried and read, but both of its blocks are outside the two
  families the port drives, so no in-game observation distinguishes it at this pin.
- The soul campfire shares the campfire branch in `accelerated_family` and is unit-tested; only
  `minecraft:campfire` is placed in game.
