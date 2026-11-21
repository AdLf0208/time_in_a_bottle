# Time in a Bottle — port notes (slices 1–2, the SDK format)

Ported from **Time in a Bottle 6.5.4** (`tiab-neoforge-6.5.4.jar`, 88 entries,
SHA-256 `0beb4ff1d81facfe9039e8d657a75836d1e8c13c137c90b491cc07e763f26325`) for
Minecraft Java 1.21.1 / NeoForge. **MIT → licence rank 0**; local-only until
publishing is approved (ARCHITECTURE.md §8). Clean-room Rust rewrite from the
mod's behaviour; what the mod does and its evidence are in
[`FINDINGS.md`](FINDINGS.md).

**Format: the SDK package (2026-11-18), version 0.2.3.** This port was migrated
from the retired `.hnmod` container to one flat `.hmod` (mod API `0.2`); the
current pin is SDK nightly **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`**
(2026-11-20) with mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**. The
migration was at `0b14f8a`; slice 2 (campfire family, 0.2.3) repinned to
`0a875da` (the API-only delta touches no hook this port implements —
`attack_entity`, `entity_interact`, `hanging_break` gained a leading
`dimension`). `src/runtime.rs` was **byte-identical** to the pre-migration
copy through 0.2.0; the 0.2.1 review added `info_permitted`/`furnace_block` to
it, and 0.2.3 added the `Family`/`Fire` classification and
`campfire_progress`. See
[`## Migration to the SDK format`](#migration-to-the-sdk-format) for the file
map, the method map and every trap, and
[`## Slice 2 (0.2.3)`](#slice-2-023-the-campfire-family) for this slice.

Status: **ported-partial** — slice 1 (the bottle's stored-tick account, one
accelerator per block, the **furnace family**, `/tiab time` and the harness
readout) and slice 2 (**the campfire family**: campfire and soul campfire cook
at the accelerator's rate, proven in game) are implemented. Behaviour evidence:
the 0.1.0 run for slice 1 and the 0.2.3 run (phases A–H, control and
`SELFTEST=1`) in `ports/_shared/evidence/tiab-e2e/`. The gap list below is the
rest of the mod.

**Why this mod:** it is the rank-0 `content`/`logic` core mod
`precheck-mod.py tiab` offers (`DISPATCHABLE`, jar licence MIT, rank 0, no
unported dependency, no worldgen). Former form PR **#828**; this version
supersedes it (§Changelog).

## Licence

MIT, read from the jar's own files: `META-INF/neoforge.mods.toml`
(`license = "MIT"`) and `LICENSE_Time In A Bottle` (MIT, `Copyright (c) <2024>
RealMangoRage`). Rank 0 (permissive), local-only until the maintainer approves
publishing (ARCHITECTURE.md §8); publishable under MIT with this source.
`LICENSE` and `LICENSE-NOTICE.md` travel **inside** the package.

## Class + tier and the approach

- **Class:** `content`; **tier:** `logic` (`ports/ledger.csv`,
  `ports/_shared/source-map/atm10-sources.csv`).
- **Approach** (`horizon/docs/modpacks.md`, "Tiers (the API a port needs)"): a
  `logic` shape — one item plus `use`/tick hooks. The whole mod is a stored
  quantity and the tick semantics that spend it, so the slice proves the
  arithmetic in game (a furnace's cook progress advancing by exactly the ticks
  spent), not just that the item exists.
- **Carry-over vs agent port:** the model, animated texture, eight lang files,
  recipe and block tag are carried from the jar
  ([`tools/assemble_content.py`](tools/assemble_content.py), decisions in
  [`CONTENT.md`](CONTENT.md)); the stored-time account, the accelerator, the
  furnace fast-forward, the commands and the tooltip are the agent port.

## What the port does (slices 1–2)

- **The bottle.** `tiab:time_in_a_bottle`, a stack-size-1 item on the inert
  `minecraft:clock` template, with the component `tiab:stored_time`
  (`{stored, total}`), declared by the package's own `component_types` hook
  before content loads — the same component `Registration.STORED_TIME_COMPONENT`
  declares.
- **Storing.** At the end of every second the first bottle in the 36 main
  inventory slots gains `TICKS_CONST` (20) stored ticks and 20 total ticks.
  `stored` stops at 622,080,000; `total` does not. The stack is written through
  `ItemPlace::PlayerSlot::edit`, which reaches the owner's client itself (the
  retired port needed a `server_tick` inventory resync for a single player's LAN
  guest; it is gone, see §Deviations).
- **Spending.** `TiabItem.useOn`, on the global `right_click_block` hook (which
  runs before the target's own `use`, so a furnace's menu never opens): a block
  entity the port can drive — the **furnace family** (furnace, smoker, blast
  furnace) or the **campfire family** (campfire, soul campfire) — that is not in
  `#tiab:un_acceleratable` gets an accelerator. One use costs **600** stored
  ticks and runs 600 game ticks at rate 1; a second click doubles the rate
  (`rate / 2 * 600` at the new rate) and adds half of the time already used up,
  up to 128x. Creative mode pays nothing (`canUse`).
- **Accelerating.** At `level_tick_post` each accelerator runs `rate` block
  ticks on its target, spends one game tick and removes itself at zero. The
  extra ticks are written into the timers the target's own tick reads — a
  furnace's `cooking_time_spent`/`lit_time_remaining`, a campfire's four
  `CookingTimes` — so it cooks (or roasts) and burns fuel at the accelerator's
  rate (§The API gap).
- **Commands.** `/tiab time add|remove <player> <seconds>` (permission level
  2, upstream's clamp `MAX_STORED_TIME / TICKS_CONST`) and
  `/tiab info <x> <y> <z> <n>` — the harness readout: the clock, the bottle's
  account, and the target's family, first timer and its cooking time, item
  count, fire/fuel state and the accelerator's rate/remaining in one line
  (`kind=furnace|campfire|none cook= ct= out= lit=`). Upstream has no commands
  besides `/tiab time`; the readout is the ports' `/powah info` precedent.
- **Tooltip.** The two carried lang keys, `Stored time HH:MM:SS` and
  `Total accumulated time HH:MM:SS`, built with the same `HH:MM:SS` split
  `CommonHelper` uses.

## Hooks (7 methods + 1 Part + the registration)

| Moment | SDK method | What it does |
|---|---|---|
| `component_types` | `Hooks::component_types` | declares `tiab:stored_time` before content loads |
| `register` | `Hooks::register` | `item_key("tiab:time_in_a_bottle")`, then attaches the `Bottle` Part to it |
| `inventory_tick` (item-scoped) | `ItemPart::inventory_tick` on `Bottle` | the first bottle stores 20 ticks a second |
| `right_click_block` (global, cancellable) | `Hooks::right_click_block` | spend stored ticks on a furnace, claim the click (`InteractionResult::Success` + `Flow::Cancel`) |
| `level_tick_post` | `Hooks::level_tick_post` | run each accelerator's extra block ticks on its target (furnace or campfire timers) |
| `register_commands` | `Hooks::register_commands` | `/tiab time add|remove`, `/tiab info` |
| `player_command` | `Hooks::player_command` | their bodies (raw string, before the tree) |
| `tooltip` (item-scoped) | `ItemPart::tooltip` on `Bottle` | the two time lines |

6 hooks + 1 Part (2 scoped methods) + `register`: **no unstamped hook**, no
invented API, and no `content_root` injection (the content is in the package).

## Migration to the SDK format

*Written 2026-11-18 from this port, the second migration after `imfast`.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70) retired `.hnmod`.
The registry closed every `.hnmod` submission on 2026-11-18, including this
port's **#828**, with one reason: *"republish this as a native `.hmod` with the
SDK from the next Nightly"*. The new unit is **one flat `.hmod`**: content,
native code, or both. Pins: nightly **`0b14f8a`**, mixins **`e59da90`**,
launcher 0.2.78, `cargo-horizon-mod` from `horizon-rust/horizon@main`.

**This port differs from `imfast` in one way that drives everything below: it
carries content.** `imfast` is code only; `tiab` has an item, assets and data,
which used to ride inside the `.hnmod` as `mixins/tiab/data/tiab-content.hmod`
(the AE2 embedded-content pattern). One flat `.hmod` now carries both halves, so
that whole mechanism — the built inner `.hmod`, the `native_data!` blob, the
`.horizon-cache` write and the one-level `content_root` recursion — is deleted
rather than ported.

### 0. Read the surface before writing glue

```sh
cargo horizon-mod new /tmp/opencode/<id>-new --template native   # fetches .horizon/sdk/
cargo horizon-mod hooks <text>            # search the catalog offline
```

The declaration the generator reads is
`.horizon/sdk/api/crates/horizon-mod-vocab/src/hooks/<name>.rs`; the **generated**
`Hooks` trait, every event struct, every `ItemPart`/`BlockEntityPart` method and
every dispatch function are in
`.horizon/sdk/api/crates/horizon-mod-vocab/generated/mod_api_generated.rs`, and
the host methods with their access levels in `.../src/host/<area>.rs`. Read the
generated file, do not guess: `perform_command` does not exist any more, and
half the old bindings became handle methods (`entity.data(host)`,
`place.edit(host, tx, ..)`).

### 1. Files created

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest and the only identity (field map below) |
| `Cargo.toml` | one crate; its only dependency is `horizon-mod-api` from `.horizon/sdk/` |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks`, the `Bottle` `ItemPart` (`#[mod_api::horizon_part]`), the `Mod` state |
| `src/glue.rs` | the game side: the account's component, the accelerator table, the furnace fast-forward, the readout (the old `support.rs`) |
| `src/runtime.rs` | the port's pure logic, **copied byte-for-byte** (`cp`, then `cmp`) |
| `assets/tiab/`, `data/tiab/` | the carried content, moved from `content/assets/` and `content/data/` (16 files) |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` (trap 5) |
| `LICENSE-NOTICE.md` | licence provenance; `LICENSE` was already there |
| `tiab.hmod` | the delivered package |

`assets/<id>/` and `data/<id>/` sit at the **port root**: a package that carries
content has them beside `src/`, and `cargo horizon-mod build` packs them.

### 2. Files deleted (and where the evidence went)

| Deleted | Why | Kept as evidence |
|---|---|---|
| `mixins/tiab/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, one helper per hook, `data/tiab-content.hmod`) | the container is retired; `runtime.rs` moved to `src/`, `support.rs` to `src/glue.rs` | `ports/_shared/evidence/tiab/retired-hnmod-0.1.0/` |
| `tiab.hnmod` | unloadable on launcher ≥ 0.2.70 | same folder |
| `compatibility.json` | `[native]` in the manifest is the pin now | same folder |
| `rust-toolchain.toml` | the pinned Horizon commit decides the toolchain | same folder |
| `tools/horizon.py`, `tools/package.py`, `tools/package.sh` | staging + `.hnmod` packaging are gone | same folder |
| `content/tiab.hmod` | one flat package carries the content | same folder (`content/`) |
| `content/horizon-mod.toml` | the root manifest is the only manifest | same folder |
| `Cargo.lock` | not needed without registry deps; changes the artifact (trap 5) | not kept |

**Kept:** `tools/e2e.sh` (the scenario and its assertions are
format-independent; its launch line was updated, see §15), `tools/assemble_content.py`
(now writes the root `assets/`/`data/` and reproduces them byte-for-byte),
`notes.md`, `FINDINGS.md`, `PLAN.md` (marked historical), `CONTENT.md`,
`README.md` (rewritten: it is packed into the package).

### 3. Manifest field mapping

| Old | New |
|---|---|
| `horizon-mixin.toml` `[mod] id` | `horizon-mod.toml` `id` (also the resource namespace) |
| `[mod] version = "0.1.0"` | `version = "0.2.0"` (a **new** version for a resubmission) |
| `[mod] priority = 100` | gone; ordering is `#[listener(priority = …)]` |
| `[compatibility] horizon_revision = 1d63e0a5…` | `[native] horizon = "0b14f8ac…"` (exact 40 hex) |
| mixins `5a64af93…` (schema 6) | `[native] mixins = "e59da9094f0c…"` |
| — | `[native] game = "26.3"` |
| `[[module]] runtime`, `[[module]] state` | **no plan section**: `src/runtime.rs` is a module, `src/glue.rs` another |
| 9 `[[injection]]` entries (8 hooks + `content_root`) | 7 `Hooks`/`ItemPart` methods + `register`; the `content_root` one is deleted |
| `content/horizon-mod.toml` (the content manifest) | gone: one manifest, `capabilities = ["registries"]`, assets/data at the root |
| `horizon-native.json` | gone |
| — | `api = "0.2"`, `side = "both"`, `license`, `homepage`, `issues`, `authors` |

`side = "both"` is the **installation**, not the hook's logical side: an
integrated server belongs to the client installation (trap 6).
`capabilities = ["registries"]` is the one value the schema allows, and this mod
does declare an item and a component (trap 7).

### 4. Injection → method (the helper map)

| Retired injection (helper) | New method | Mapping notes |
|---|---|---|
| `component-types` (`component_types.rs`) → `types: &mut Vec<String>` | `Hooks::component_types(context, event)` | `event.types.push(STORED_TIME)`, same as before |
| `inventory-tick` (`inventory_tick.rs`) → `game, slot` | `ItemPart::inventory_tick(host, event)` on `Bottle` | item-scoped now (trap 16): `event.slot`, and the acting player from `host.players().acting_player()`; the write is `ItemPlace::PlayerSlot::edit` inside `host.transaction(..)`, not `game.native_carried_stack` |
| `right-click` (`right_click_block.rs`) → `game, pos, face, canceled` | `Hooks::right_click_block(host_side, event) -> Flow` | `event.click.{pos,dimension,hand,stack,player}`; cancel with `*event.cancellation_result = InteractionResult::Success` and `Flow::Cancel` (the old `*canceled = true`) |
| `level-tick-post` (`level_tick_post.rs`) → `game, dimension` | `Hooks::level_tick_post(host_side, event)` | `event.level: LevelRef` replaces the dimension string; `host.block_entity(level, pos)` + `entity.merge_data(host, patch)` replace `game.world.block_entities` + `FurnaceCooking::tick` (§The API gap) |
| `server-tick` (`server_tick.rs`) → `game, phase` | **deleted** | it only re-sent a changed bottle's owner their inventory; an `ItemPlace` write reaches the client itself (verified by the API doc, not in game — §Gaps 10) |
| `register-commands` → `commands: &mut Vec<NativeCommand>` | `Hooks::register_commands(host, event)` | `event.commands.push(Command::op(usage, 2))` / `Command::new(usage)` |
| `perform-command` (`perform_command.rs`) → `game, player, command, ran` | `Hooks::player_command(host, event) -> Flow` | the raw string is `event.command` (no slash), the acting player is `event.player`; **the hook sees commands the player may not run**, so the level-2 check is repeated; answers go to `event.replies` because player helpers reach nobody here; `Flow::Cancel` = ran |
| `tooltip` (`tooltip.rs`) → `game, stack, lines` | `ItemPart::tooltip(host, event)` on `Bottle` | item-scoped; `event.stack`, `event.lines`; the colour is packed **ARGB** now (trap 19) |
| `embedded-content` (`embedded_content.rs`) → `content_root` | **deleted** | the flat package carries `assets/` and `data/`; nothing to write out or append |

`Hooks` method signatures are not free: a cancellable hook returns `Flow`, a
replacement returns `Option`, a collection adds to the event's list. `runtime.rs`
kept its public surface, so the formulas ported without a change.

### 5. The API gap: no way to run a vanilla block entity's tick

The retired port called `horizon_client::furnace::FurnaceCooking::tick` — the
engine's own `AbstractFurnaceBlockEntity.serverTick` — from an injected module,
which a mod cannot do. Checked at `0b14f8a`, not assumed:

- `LevelRef::block_tick` runs the block's **scheduled** tick
  (`ServerLevel.tickBlock`), not its block entity's;
- `block_entity_tick`/`block_entity_ticks` are `BlockEntityPart` methods, and
  `Registrar::attach_block_entity_part` documents *"the mod block entity type
  `key`"* — a vanilla furnace type cannot be attached;
- there is no `furnace`/`smelt`/`tick a block entity` host call.

`ARCHITECTURE.md` §3 already carries this as a named gap — *"No helper to tick
**another** block entity | Time in a Bottle, accelerators | `block_entity_tick`
ticks the owning entity only"* — and the SDK pin is its second measurement: the
issue shape stands, *"`tick_block_entity(pos)` / a rate-modifier hook"*.

So the accelerator's extra ticks are written into the furnace's **saved
timers** with `BlockEntityRef::merge_data` (the same path `/data merge block`
takes; `merge_data`, not `replace_data`, so `Items` survives):

| Saved key (the engine's own, `crates/horizon-client/src/save.rs`) | Write |
|---|---|
| `cooking_time_spent` | `+ extra`, held one tick short of `cooking_total_time` |
| `lit_time_remaining` | `- extra`, floored at 0 |

`extra = rate - 1`, because the furnace's own tick already runs once per game
tick. The shape is `glue::fast_forward`, pure and unit-tested (5 tests).

### The campfire family (slice 2)

The same gap blocks the second family. A lit `CampfireBlockEntity`'s own tick (`cookTick`) raises
each occupied slot's `CookingTimes` entry by one and completes the cook at
`progress >= total`, dropping the result at the block's corner; an unlit one runs `cooldownTick`
(two off a tick, never below zero). The port writes the four `CookingTimes` entries the same way it
writes a furnace's timer, and the campfire's own tick still finishes and drops the item:

| Saved key (engine's own, `crates/horizon-client-render/src/containers.rs`) | Write |
|---|---|
| `CookingTimes` (int array, 4) | `+ extra` while lit, held at `CookingTotalTimes[i]` |
| `CookingTimes` (int array, 4) | `- 2 * extra` while unlit, floored at 0 (vanilla's cooldown) |

The occupancy comes from the block entity's saved `Items` list (`Slot` byte per stack), not from
the item-handler view: a campfire is not a menu container, so `ItemPlace::BlockEntity` does not
answer for it, and `items[slot].is_some()` is only true after `place_food` has run.

**A corrected loss bound (measured, 2026-11-21).** The furnace text above used to say no rate-1
accelerated tick is ever lost. It is not true: the fast-forward writes the timer **at `total`**,
and a furnace tick that starts at `cooking_total` completes the cook and discards the increment it
just added — up to one accelerated tick per completed cook, at any rate. Measured on the 0.2.3
package: a rate-1 probe sampled every 25 ticks for 1,500 ticks broke
`cooked = game ticks + delivered` in 1 of 60 intervals, by exactly one tick
(`ports/_shared/evidence/tiab/probe-rate1-intervals.log`), and the e2e's C/E phases each lost one
tick over 8–9 cooks. The e2e therefore bounds the accelerated ticks by the number of cooks in the
window instead of asserting exactness.

**Semantics this keeps and loses** (also §Gaps 10):

- **Kept:** a furnace cooks and burns fuel at the accelerator's rate; the fuel
  lasts `rate` times less; a rate-1 accelerator writes nothing; an unlit
  furnace, an unknown cook time and a block that is not a furnace are left to
  the engine; the accelerator still spends one game tick per game tick.
- **Lost:** one cook finishes per game tick **at most** and the remainder of the
  last cook is dropped (upstream runs each extra tick through the furnace's own
  routine and carries it over). At 2x that costs about 1 %, at 128x up to one
  cook per completion. Fuel remainders (a lava bucket's bucket) and the
  `RecipesUsed` experience bookkeeping stay the engine's own, one per vanilla
  tick.

### 6. Build and check (the ladder for an SDK package)

`build-slot.sh` speaks the old staged tool and does not apply to the SDK
layout, so every command runs **directly in the port** with
`CARGO_INCREMENTAL=0` — no staging, no staged tree, no `--target-dir`:

```sh
cd ports/tiab
CARGO_INCREMENTAL=0 cargo test                       # 21 passed
CARGO_INCREMENTAL=0 cargo clippy --offline --all-targets -- -D warnings
cargo fmt --check                                    # NOT `--all` (trap 4)
rm -f Cargo.lock                                     # trap 5
CARGO_INCREMENTAL=0 cargo horizon-mod build          # -> target/horizon-mod/tiab.hmod
cp target/horizon-mod/tiab.hmod ./tiab.hmod
cargo horizon-mod check tiab.hmod                    # exit 0
cargo horizon-mod check tiab.hmod --json
cargo horizon-mod fits tiab.hmod --build nightly      # exit 0
sha256sum tiab.hmod
```

Measured for **this** port (all logs in `ports/_shared/evidence/tiab/`):

- `21 passed` unit tests (16 `runtime.rs` + 5 `glue::fast_forward`), 0 failed;
- clippy 0 warnings, `fmt --check` clean;
- `SDK 0b14f8a … Built ./target/horizon-mod/tiab.hmod`, EXIT 0;
- package **28,391 B**, sha256
  `5546d00d88d90e1383d6245dfd1ca3a6f217c824245244a6b382f00ebe09850b`,
  **25 entries** (`horizon-mod.toml`, `Cargo.toml`, `LICENSE`,
  `LICENSE-NOTICE.md`, `README.md`, the 16 `assets/`+`data/` files,
  `src/glue.rs`, `src/lib.rs`, `src/runtime.rs`, `uses.json`) — one `.hmod`
  carries the item *and* the code;
- `check` EXIT 0
  (`tiab 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`),
  `--json` EXIT 0, `fits --build nightly` EXIT 0 (`0b14f8a  made for`);
- `uses.json` has 252 surface entries (`hook:component_types`,
  `hook:inventory_tick`, `hook:right_click_block`, `hook:level_tick_post`,
  `hook:register_commands`, `hook:player_command`, `hook:tooltip`,
  `method:ItemPart::…`, `handle:BlockEntityRef::merge_data`,
  `method:HostTransaction::transaction`, `capability:registries`, …);
- the packaged `src/runtime.rs` is byte-identical (`cmp`, same sha256);
- a **fresh-clone repeat** (`.horizon/`, `target/`, `Cargo.lock` removed, then
  `cargo horizon-mod build`) fetched the SDK by itself and produced the **same
  28,391 B and the same sha256**.

### 7. Traps

Inherited from `imfast` and re-confirmed here (1–15), then the ones this port
added (16–26):

1. **A leftover old path refuses the package** — `error: unexpected package
   path: mixins/tiab/runtime.rs`: move `mixins/` and the `.hnmod` out *before*
   the first build.
2. **A 38-character hash reads as missing pins** — `native requires game and
   exact revisions`. Copy the two 40-hex commits; the mixins commit is
   `e59da9094f0c…`, **not** the old `5a64af93…`.
3. **`cargo horizon-mod check` with no file checks wasm** — use
   `check <file.hmod>` for the package, `cargo check`/`test` for the crate.
4. **`cargo fmt --all --check` reports the SDK's own files** (`.horizon/sdk` is
   a path dependency). Use `cargo fmt --check`; never "fix" the fetched SDK.
5. **A packed `Cargo.lock` changes the artifact** (and resolves newer crates
   than the SDK's own lock). `rm -f Cargo.lock` before the build;
   `/Cargo.lock` in `.gitignore`.
6. **`side` is the installation side** — `both` for anything but a
   dedicated-only mod.
7. **`capabilities = ["registries"]` is not optional**, even for a mod that
   declares nothing: it is the schema's one value.
8. **`runtime.rs` as a module** — declare `pub mod runtime;`, never
   `#[allow(dead_code)]` in the file itself.
9. **`Hooks: Any + Send + Sync`** — session state is a `Mutex` (or a `OnceLock`
   of immutable keys), never `RefCell`.
10. **`ServerTickPhase` is `#[non_exhaustive]`** — no longer relevant here (the
    `server_tick` hook retired), but the same wildcard rule applies to
    `HostSide` and every event enum.
11. **The context types changed shape** — `LifecycleContext { authority,
    epoch: WorldEpoch, tick: TickId }`, public `get()`.
12. **`commands()` is `#[access(WorldOnly)]`** — fine at `server_tick` and
    `player_command` (world phase), refused in `register`.
13. **No `rust-toolchain.toml`** — the pinned commit and the launcher decide the
    toolchain (this crate: edition 2024, built here with rustc 1.97.0; the SDK
    declares 1.99.1).
14. **`cargo horizon-mod build` does pack a native project** — the pinned
    `docs/mods.md` text that says it refuses a `Cargo.toml` predates the merge.
15. **The e2e needs a launcher-built game** (`horizon-launcher native-build`)
    and `horizon-launcher` is not installed here. `tools/e2e.sh` now refuses to
    run without `GAME=` (no default path exists in this layout), and its
    `--dev-mod content` launch, `EMBEDDED`/`CONTROL` legs and
    `tools/horizon.py ctl` fallback are gone with the retired format. Its
    assertions are unchanged and still the behaviour's proof.
16. **A `Part` is not a `Hooks` method.** `inventory_tick` and `tooltip` are
    `scope = "item"`: the game dispatches them through the `ItemPart` attached
    to the item with `Registrar::item_part`, and the impl needs
    `#[mod_api::horizon_part]` (it generates the hidden `item_subscribed`). A
    `Hooks::inventory_tick` compiles and is never called.
17. **Handle types are not constructible.** `ItemKey(u32)` has no public
    constructor and its field is private: get one from `r.item_key(id)` in
    `register` (or from `stack.item()`). A global hook that must recognise the
    mod's item stores the key (`OnceLock<ItemKey>`) at registration.
18. **`perform_command` is gone; `player_command` is a different moment.** It
    receives the raw string *before* the command tree runs, **also for commands
    the player may not run** — so the permission check must be repeated — and
    `PlayerRef::tell`/`action_bar` reach nobody there: every answer goes to
    `event.replies`. A registered command that is always answered here never
    reaches `run_mod_command`.
19. **The tooltip colour is packed ARGB.** The retired hook took RGB
    (`0x55FF55`); the SDK packs `rgba` and `u32::MAX` means "the text's own", so
    `0x55FF55` is alpha 0 — invisible text. Use `0xFF55_FF55` / `0xFFAA_AAAA`.
20. **`Compound::get` wants `&JavaString`, which `mod_api` does not export.**
    `data.get(&key.into())` infers it; there is no `get(&str)`, and
    `horizon_core` is not a dependency of a mod.
21. **"Run a block entity's tick" has no API** — see §5. `block_tick` is the
    *scheduled* block tick; `attach_block_entity_part` is documented for a mod
    block entity type. Do not attach a Part to `minecraft:furnace` and hope.
22. **`merge_data`, not `replace_data`.** `merge_data` merges into the saved
    compound and loads it (so `Items` survives); they are handle methods —
    `entity.merge_data(host, patch)` — not host methods.
23. **`host.players().player(uuid)` is wrong.** `player` is on the host;
    `game_mode`, `permission_level`, `level`, `uuid`, `tell`, `action_bar` are
    `PlayerRef` methods (`player.game_mode(host)`).
24. **`host.transaction(|h, tx| …)` needs `HostTransaction` in scope and an
    *unannotated* closure parameter.** Writing `|h: &mut dyn ServerHost, tx|`
    pins the object lifetime and the blanket impl then demands `'static`
    (`lifetime may not live long enough`).
25. **`i32::saturating_sub` floors at `i32::MIN`, not 0.** A fuel clamp written
    as `lit.saturating_sub(extra)` went negative; the fast-forward unit test
    caught it (`Some((63, -53))`). `.max(0)` is the clamp.
26. **`README.md` at the port root is packed.** It is part of the delivered file,
    so it must describe the current format (the old one documented the
    `.hnmod`). `notes.md`, `FINDINGS.md`, `PLAN.md`, `CONTENT.md` and `tools/`
    are not packed; `assets/`, `data/`, `src/`, `Cargo.toml`,
    `horizon-mod.toml`, `LICENSE`, `LICENSE-NOTICE.md` and `README.md` are.
27. **Something outside a port job rewrote every manifest at 13:04:24 on
    2026-11-18.** All 28 `ports/*/horizon-mod.toml` changed in that second and
    lost `license`, `homepage` and `issues` — this port's and `imfast`'s
    included. The fields are valid (`check` exits 0 with them, and they are
    packed into the package), and `cargo horizon-mod build` itself does **not**
    rewrite the manifest: a build with the three fields restores the 28,391 B
    artifact, without them 28,328 B. So re-read the manifest after a wave if a
    registration depends on its licence fields, and keep the licence in
    `LICENSE-NOTICE.md` (packed) as well, which is what this package does.
28. **`ctl screen` hangs at `0a875da`.** One call blocked a probe until the guard's
    idle deadline (`horizon-ctl`'s read timeout is off by default, which is why
    the `e2e-guard` shim exists). `tools/e2e.sh` never calls it: ingredients are
    found with `ctl inventory` (its `hotbar[]` carries the slot number) and a
    container's state comes from `/tiab info`. `ctl menu` answers but lists only
    some slots, so it is not a slot table either.
29. **`click-slot` needs a renderer.** On `--headless` every call answers
    `{"error":"no renderer yet"}` because it clicks the drawn screen; the recipe
    phase needs `--offscreen` (a window that draws but is not shown). The e2e
    launches offscreen for that reason.
30. **`look-at-block X Y Z west` fails on a campfire.** The block is 7/16 high, so
    a standing player's eye is above its side faces and the harness answers
    `no face of block … is visible from here`; aim at `up`. A furnace (a full
    block) still takes `west`.

### 8. Changelog for 0.2.3 (this slice)

For the registry entry's `changelog` field:

> **0.2.3** — the campfire family. A lit campfire or soul campfire is now a
> target too: one use writes its four `CookingTimes` timers at the accelerator's
> rate, so a potato roasts in half the game ticks while the campfire's own tick
> still finishes and drops it, and an unlit campfire cools at the same rate.
> Repinned to Horizon `0a875da` (Nightly 2026-11-20; mixins unchanged, no hook
> this port uses changed). `/tiab info` now reports the target's family, its
> cooking time and its items (`kind= cook= ct= out= lit=`), so the campfire
> timer is readable in one atomic line. Still open: hoppers and mod machines as
> targets (no API runs a vanilla block entity's tick), a saved accelerator, a
> config, sounds/particles, and the off-hand for `/tiab time`.

### 8a. Changelog of 0.2.0 (the resubmission that superseded #828)

For the registry entry's `changelog` field:

> **0.2.0** — republished as one flat SDK `.hmod` (mod API 0.2, pinned to
> Horizon `0b14f8a` / mixins `e59da90`) after the 2026-11-18 format change
> retired the `.hnmod` container; this version **supersedes #828**, which the
> registry closed with *"republish this as a native `.hmod` with the SDK from
> the next Nightly"*. Same slice as 0.1.0: the bottle and its stored-tick
> account, the furnace-family accelerator, `/tiab time` and the `/tiab info`
> readout — the item, its assets and its data now travel inside the same
> package instead of an embedded inner `.hmod`. Known gaps unchanged: the
> accelerator is a saved-timer fast-forward (not an entity, not saved), only the
> furnace family is accelerated, and another player's inventory cannot be
> written.

## Pin facts that shaped this slice (re-checked at `0b14f8a`)

1. **The API declares no entity type**, so upstream's
   `TimeAcceleratorEntity` cannot exist. The port keeps `{level, pos, rate,
   remaining}` in a `level_tick_post` table instead. It is not saved (gap 1).
   Entries of a level that is no longer open are pruned each tick
   (`host.levels().list_levels()`), so a reopened world cannot accumulate them.
2. **A furnace's own tick routine is not reachable from a mod** — the retired
   port's `FurnaceCooking::tick` call has no SDK equivalent (§5). Measured on
   the 0.1.0 port: it had to be fed the **world's** tables
   (`world_recipes!`/`world_items!`), not `game.res.recipes()`, or
   `recipes.cook` found no recipe and the timer never moved.
3. **A block entity's write inside its own tick is reverted**, which is why the
   accelerated ticks run at `level_tick_post`, after the level's own pass.
4. **`right_click_block` is the hook that fires on a furnace**, not
   `use_item_on`: the block's own `use` consumes the click first, so the item's
   `useOn` is never reached. Claiming the cancelling hook also keeps the
   furnace menu shut, as upstream's `SUCCESS` does.
5. **The account must be read in one atomic line.** `/tiab info` prints the
   game clock beside the account, so the e2e computes the spend exactly:
   `20 * (floor((t1-1)/20) - floor((t0-1)/20)) - (stored1 - stored0)` — a
   command is handled before the tick's level pass, so a read at `t` sees the
   state before that tick's end-of-second step. The clock is
   `LevelRef::day_time` now (the engine's `game.world.game_time` had no direct
   equivalent; `day_time` is the shared Overworld tick counter).

## Content (one flat package)

`assets/tiab/` and `data/tiab/` are generated from the jar by
[`tools/assemble_content.py`](tools/assemble_content.py); [`CONTENT.md`](CONTENT.md)
lists every carried and every written file and why. 16 files: the item model,
the animated texture and its `.mcmeta`, eight lang files, 26.3's item
definition, the item declaration (clock template, stack 1, `tiab:stored_time`
default), the creative tab, the recipe and the block tag. Re-running the
assembler reproduces all 16 byte-for-byte (verified: same SHA-256 for each).

## Deviations and gaps

1. **The accelerator is not an entity and is not saved.** The API declares no
   entity type, so the state lives in a `level_tick_post` table: a restart or a
   chunk unload ends a running acceleration, and the client draws no floating
   text. Upstream saves `{position, timeRate, remainingTime}` in the entity's
   NBT.
2. **Two families are accelerated: the furnace's and the campfire's**
   (`minecraft:furnace`, `smoker`, `blast_furnace`; `minecraft:campfire`,
   `soul_campfire`). Upstream ticks any block entity's own ticker — hoppers,
   brewing stands, mod machines — and randomly-ticking blocks at a 1/1365 chance
   per accelerated tick. Those targets do nothing here: the click is not claimed
   (their own `use` runs) because no API runs a vanilla block entity's tick, and
   a block without a family this port can drive is upstream's `FAIL` too. A
   randomly-ticking block would need to know that its state ticks randomly, and
   the API has no such predicate (`block_state_*` exposes replacement, speed,
   light and the like, not the random-tick flag), so it stays a gap.
3. **The command reaches only the acting player.** `/tiab time add <player>`
   refuses another player: the API has no inventory write for another player.
4. **No config file.** The four `ITiabConfig` defaults are the constants in
   `runtime.rs` (`max_stored_time`, `ticks_const`, `each_use_duration`,
   `max_rate_multi`); upstream's server config is not read.
5. **No sounds, particles or client renderer** (upstream plays a pitch-shifted
   sound per rate and draws the remaining time over the block).
6. **No JEI/EMI category.** The lang key `emi.category.tiab.test` is carried
   with the rest of `en_us`; the integration is absent.
7. **The bottle's component is read from the first bottle in the 36 main
   inventory slots**, as `findTiabItem` does; a bottle in the off hand is not
   found for the command (it is found by the held stack for a use).
8. **`/tiab info` is a port extension** (the harness readout); upstream has no
   such command.
9. **A creative use still starts an accelerator on a block with nothing
   stored**, exactly as `canUse` allows; the account is untouched.
10. **The acceleration is a saved-timer fast-forward, not extra block-entity
    ticks** (§5): one cook per game tick at most, the cook's remainder dropped,
    and the block's `lit` property follows the engine's own tick (so it can lag
    one tick behind the saved timers). A high rate therefore runs slightly
    slower than upstream's `rate` multiplier, and **up to one accelerated tick
    per completed cook** is lost at any rate: a furnace or campfire tick that
    starts at its cooking time completes the cook and discards the increment it
    added (measured, 1 in 60 sampled intervals; §The campfire family).
11. **Command answers are chat replies.** `player_command` runs with no player's
    state swapped in, so `PlayerRef::action_bar` reaches nobody: "Added N
    seconds" and the upstream tell both go to `event.replies`. A console
    `/tiab …` is not handled (the hook is player-only), as with the retired
    `perform_command`.
12. **No client prediction of the click.** The client side of
    `right_click_block` returns `Flow::Continue`; the server claims the click
    with `Success`, which corrects the client's predicted furnace menu.
13. **The inventory resync is gone.** The retired `server_tick` leg existed
    because a single player's client is a LAN guest and a direct stack write did
    not reach it; `ItemPlace::PlayerSlot::edit` documents that the player's
    client gets the change, so there is no `server_tick` hook and no queue.
    *Not verified in game* — no launcher here (§15).
14. **The e2e was not re-run** (trap 15). Its scenario, assertions and
    `SELFTEST=1` fail-capability are unchanged; `GAME=` must point at a
    launcher-built game carrying `tiab.hmod`.

## Upstream defects fixed

None found in the 0.1.0 slice, and the migration found none either. The jar is
internally consistent: the `#tiab:un_acceleratable` tag names blocks its `useOn`
tests, the recipe's ingredients exist in 26.3, and the four config defaults
agree between `ITiabConfig` and the command's clamp. The deviations above are
pin/API limits, not defects. The migration **fixed one port defect of its own**:
the tooltip colours were carried over as RGB and would have rendered as
alpha-0 text under the SDK's packed `rgba` (trap 19).

## Slice 2 (0.2.3): the campfire family

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

**The gap it comes from** (§Deviations 2, before this slice):

> **Only the furnace family is accelerated** (`minecraft:furnace`, `smoker`,
> `blast_furnace`). Upstream ticks any block entity's own ticker — hoppers,
> campfires, brewing stands, mod machines — and randomly-ticking blocks at a
> 1/1365 chance per accelerated tick.

**Why this family, and why the tier fits.** A campfire is the second block entity whose own tick
this port can reproduce without running it: its state is four `CookingTimes` entries that its own
tick advances, exactly the shape the furnace family already uses at `level_tick_post`. It is a
`logic`-tier family (a tick hook and two saved keys; no entity, no menu, no renderer), it is in the
jar's behaviour (`TimeAcceleratorEntity.tick` runs the clicked block entity's ticker, whatever it
is), and it is provable in game with an exact timer against an untouched control campfire. The
other half of the gap — hoppers, brewing stands, mod machines — has no API (a vanilla block
entity's tick cannot be run), and randomly-ticking blocks still need a predicate the API does not
expose (§Deviations 2).

### Red, then green

The rules went into `runtime.rs` first, with stubs that could only fail
(`accelerated_family` knowing the furnace family only, `campfire_progress` returning its input).
`cargo test` then printed, verbatim (`ports/_shared/evidence/tiab/sdk-test-red-campfire.log`):

```
---- runtime::tests::the_campfire_family_is_the_second_accelerated_target stdout ----
thread 'runtime::tests::the_campfire_family_is_the_second_accelerated_target' panicked at src/runtime.rs:536:9:
assertion `left == right` failed
  left: None
 right: Some(Campfire)
---- runtime::tests::a_campfire_timer_stops_at_its_cooking_time stdout ----
thread 'runtime::tests::a_campfire_timer_stops_at_its_cooking_time' panicked at src/runtime.rs:571:9:
assertion `left == right` failed
  left: 599
 right: 600
---- runtime::tests::a_lit_campfire_advances_by_the_extra_ticks stdout ----
thread 'runtime::tests::a_lit_campfire_advances_by_the_extra_ticks' panicked at src/runtime.rs:562:9:
assertion `left == right` failed
  left: 10
 right: 11
---- runtime::tests::an_unlit_campfire_cools_two_ticks_a_tick stdout ----
thread 'runtime::tests::an_unlit_campfire_cools_two_ticks_a_tick' panicked at src/runtime.rs:580:9:
assertion `left == right` failed
  left: 100
 right: 98

failures:
    runtime::tests::a_campfire_timer_stops_at_its_cooking_time
    runtime::tests::a_lit_campfire_advances_by_the_extra_ticks
    runtime::tests::an_unlit_campfire_cools_two_ticks_a_tick
    runtime::tests::the_campfire_family_is_the_second_accelerated_target

test result: FAILED. 24 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Green: `28 passed; 0 failed` (`sdk-test.log`). The five rules the tests pin:

| Rule | Test |
|---|---|
| `minecraft:campfire`/`soul_campfire` are the second family; `chest`, `hopper`, `grass_block` are none | `the_campfire_family_is_the_second_accelerated_target` |
| a lit slot's timer rises by `extra` | `a_lit_campfire_advances_by_the_extra_ticks` |
| it stops at its cooking time (the campfire's own tick completes on `>=`) | `a_campfire_timer_stops_at_its_cooking_time` |
| an unlit campfire cools two a tick per accelerated tick, never below zero | `an_unlit_campfire_cools_two_ticks_a_tick` |
| an empty slot holds no timer and no step invents one | `an_empty_campfire_slot_stays_empty` |

### In game

`launcher-game.sh run tiab` (built `Compiling tiab (1 of 1)`; `--offscreen`, The End arena). Phase G
is the new family: two lit campfires with one potato each, one accelerated, one the control.

```
== G: a campfire is a target too, at exactly the accelerator's rate ==
  tiab 202,101,10 n=28 t=5299 stored=40 total=40 kind=campfire cook=26 ct=600 out=1 lit=1 acc=none
  tiab 202,101,12 n=29 t=5305 stored=60 total=60 kind=campfire cook=13 ct=600 out=1 lit=1 acc=none
  over 105 game ticks the campfire's timer advanced 210 (105 of them accelerated); the control's advanced 134
  the accelerated potato finished and dropped (1 baked potato); the control is still cooking (cook 406)
PASS: … a paid use makes a campfire's potato cook at exactly the accelerator's rate and finish while the control is still raw, and the jar's shaped recipe crafts exactly one bottle from its five ingredients
EXIT:0
```

The numbers are exact relations, not readings: the accelerated campfire's first slot advanced by
`(g2_T - g1_T) + (g1_REM - g2_REM)` — the game ticks it lived through plus the ticks the
accelerator still owed, read from the same one-line snapshot — while the control advanced by
exactly its own clock ticks. The finished potato left the fire (`out=0`) and appeared as a
`minecraft:baked_potato` item entity, while the control's was still half-cooked (`cook 406` of
600). `SELFTEST=1` asserts one tick more than that and fails (`EXIT:1`); the game without the
package fails at phase A because the item id does not exist (`EXIT:1`).

Phase H is the parity rule's sharpest case: the carried shaped recipe is crafted in a real
crafting table (grid 100-108, result 200) and gives exactly one bottle with every ingredient
consumed; a grid without the clock and the clock alone both craft nothing.

### Coverage (every family in the jar)

| Family (jar inventory, `FINDINGS.md`) | Status |
|---|---|
| item `tiab:time_in_a_bottle` (model, animated texture, 8 lang files) | **tested** — A: given, read back from the inventory, an unknown `tiab:` id refused; the tooltip's two lines are unit-tested formatting (`clock_parts`) and not asserted in game (client text, no renderer assertion needed) |
| recipe `GGG / DCD / LBL` | **tested** — H: crafted, 1 bottle, ingredients consumed; no-clock and clock-alone controls stay empty |
| block tag `#tiab:un_acceleratable` (chest, grass_block) | **carried and read by the click path; not independently assertable** — both of its blocks are outside the two families the port drives (upstream's tag has the same role: a target the mod cannot tick), so no in-game observation distinguishes it at this pin |
| the furnace family (furnace, smoker, blast furnace) | **tested** — C, D, E, F: creative/paid/refused uses, exact spend, rate doubling and the 128x ceiling |
| the campfire family (campfire, soul campfire) | **tested** — G: timer at exactly the accelerator's rate, the cook completes and drops while the control cooks on; soul campfire shares the id branch (`accelerated_family`), not placed in game |
| hoppers, brewing stands, mod machines as targets | **blocked(no vanilla-block-entity-tick API)** — the click is not claimed; §Deviations 2 |
| randomly-ticking blocks at 1/1365 | **blocked(no random-tick predicate API)** — `block_random_tick` runs one, nothing answers "does this state tick randomly" |
| the accelerator entity (persistence, floating time text) | **blocked(no mod entity registration)** — the table is in memory; §Deviations 1 |
| sounds, particles, client renderer | **not ported** — client-tier; §Deviations 5 |
| JEI/EMI category | **not ported** — integration; the lang key is carried; §Deviations 6 |
| config file (`ITiabConfig`) | **not ported** — the four defaults are `runtime.rs` constants; §Deviations 4 |
| `/tiab time` | **tested** — B, F: add/remove drive the account; the reply "Added 60 seconds" is asserted (G) |
| `/tiab info` | **tested** — every phase parses it; it refuses a non-family block (`kind=none`) and a source without reach/permission |

## Verification

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

Pin **`0a875da91dbb0fdcb7ba8c42c11341aa229af51a`** (Nightly 2026-11-20), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.92. (The 0.2.0/0.2.1
records below were taken at `0b14f8a` / launcher 0.2.78.) Every build, test and
lint step ran through `ports/tools/build-slot.sh run --job W1-tiab --port
ports/tiab --step test|lint|build|check -- …` with `CARGO_INCREMENTAL=0`; the
package steps run locally in the slot (`BUILD_LOCAL=1`) because the remote box
has no `cargo-horizon-mod`. No `--target-dir`, no private `BUILD_SLOTS_DIR`.
Evidence: `ports/_shared/evidence/tiab/` (`sdk-*.log`, each opening with the
package sha, `probe-rate1-intervals.log`, `retired-0.2.1/`,
`retired-hnmod-0.1.0/`) and `ports/_shared/evidence/tiab-e2e/`.

#### Slice 2 / 0.2.3 at `0a875da` (the delivered package)

Everything below ran on the delivered bytes (`tiab.hmod` 33,762 B, sha256
`c5fcfe58…1ab6`); every log opens with that sha and its size.

| Check | Command | Result |
|---|---|---|
| Unit tests | `CARGO_INCREMENTAL=0 cargo test --offline` | **28 passed**, 0 failed (`sdk-test.log`; red first, `sdk-test-red-campfire.log`) |
| Lints | `cargo clippy --offline --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package | `cargo horizon-mod build`, twice, `rm -f Cargo.lock` before each | `cmp`-identical, 33,762 B, sha256 `c5fcfe58…1ab6` (`sdk-build-twice.log`) |
| Registry check | `cargo horizon-mod check tiab.hmod` | `tiab 0.2.3 (native-mod, format native-1/0a875da/e59da90, for 26.3)`, **EXIT 0** (`sdk-check.log`) |
| Fits the pin | `cargo horizon-mod fits tiab.hmod --build 0a875da` | `0a875da  made for`, EXIT 0 (`sdk-fits.log`) |
| Fits another build | `cargo horizon-mod fits tiab.hmod --build 0b14f8a` | `0b14f8a  fits`, EXIT 0 (`sdk-fits-other.log`) |
| Package gate | `python3 ports/tools/verify-sdk-package.py tiab` | **1/1 verified** (runtime byte-identical tree↔package, no packed `Cargo.lock`, 25 entries) |
| In game, main | `launcher-game.sh run tiab` (phases A–H) | **PASS + `EXIT:0`** (`tiab-e2e/e2e-launcher.log`) |
| In game, control | the same Nightly **without** the package | `FAIL`, `EXIT:1` — the item id does not exist (`tiab-e2e/e2e-launcher-control.log`) |
| In game, counterfactual | `SELFTEST=1` (G's campfire assertion off by one) | `FAIL`, `EXIT:1` (`tiab-e2e/e2e-launcher-selftest.log`) |
| e2e log gate | `python3 ports/tools/check-e2e-logs.py tiab-e2e --strict` | `logs=3 ok=3 missing-exit=0 bad-exit=0` |
| Rate-1 probe | 60 intervals × 25 ticks, `cooked` vs `ticks + delivered` | 59 exact, 1 short by one tick (`probe-rate1-intervals.log`) |

#### Slice 1 / 0.2.0 (historical)

| Check | Command | Result |
| Lints | `cargo clippy --offline --all-targets -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` | clean (`sdk-fmt.log`) |
| Package | `cargo horizon-mod build` | `target/horizon-mod/tiab.hmod`, EXIT 0 (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check tiab.hmod` | `tiab 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)`, **28,391 B**, sha256 `5546d00d…9850b`, **EXIT 0** (`sdk-check.log`, `sdk-check-json.log`) |
| What it uses | `unzip -p tiab.hmod uses.json` | 252 surface entries (`sdk-uses.json`) |
| Fits the pinned build | `cargo horizon-mod fits tiab.hmod --build nightly` | `0b14f8a  made for`, EXIT 0 (`sdk-fits.log`) |
| `runtime.rs` untouched | `unzip -p tiab.hmod src/runtime.rs \| cmp -` the retired file | identical, sha256 `3594d949…e9ae` (`sdk-runtime-identical.log`) |
| Reproducible | two consecutive builds | same 28,391 B, same sha256 (`sdk-built-twice.log`) |
| Fresh-clone repeat | `.horizon/`, `target/`, `Cargo.lock` removed, then `cargo horizon-mod build` | SDK fetched by itself; same 28,391 B, same sha256 (`sdk-fresh-clone.md`) |
| Carried content | `python3 tools/assemble_content.py` | 16 files, every SHA-256 unchanged |

### Not re-run in this migration

- **The in-game e2e** (the End arena, three furnaces, scenario A–F and the
  negative run): no `horizon-launcher` here to build a game
  carrying the mod (trap 15). The 0.1.0 run below stands as the behaviour's
  evidence; what changed is the *transport* (host methods instead of
  `Game`/`FurnaceCooking` calls) and, for the accelerator, the mechanism (§5),
  which is a recorded gap rather than a silent one.
- **The staged ladder** (`tools/horizon.py stage/check/test/build`) — those
  tools are deleted with the retired format.
- `python3 ports/tools/check-e2e-logs.py tiab --strict` still reads the three
  0.1.0 logs (`logs=3 ok=3`, `e2e.log` PASS+EXIT:0, `e2e-embedded.log`
  PASS+EXIT:0, `e2e-selftest.log` FAIL+EXIT:1 — the fail-capable run); no new
  game log exists to add.

### What the 0.1.0 e2e shows (a real game, `--headless`, The End arena)

```
== A: the bottle registers, and three furnaces cook ==
  bottle in the inventory; A cook 60, B cook 54, C cook 43
== B: carrying the bottle stores exactly one tick a game tick ==
  207 game ticks: stored 0 -> 200 (exactly 200)
== C: creative mode pays nothing (upstream's canUse) ==
  creative use: spent 0, ran exactly 594 block ticks (its remaining budget)
== D: an unfilled bottle pays for nothing in survival ==
  survival, nothing stored: spent 0, ran 0 block ticks, no menu
== E: a paid use costs exactly 600 and runs exactly 600 block ticks ==
  one use: cost exactly 600 stored ticks, ran exactly 575 block ticks (its remaining budget)
== F: 1,200 stored ticks buy two uses; a third cannot reach rate 4 ==
  two uses: cost 1200, remaining 2x567, ran 1134 block ticks
PASS: … the bottle registers and stores exactly one tick a game tick while carried, creative mode pays nothing and an unfilled bottle pays nothing in survival, 620 carried ticks pay exactly 600 for a use that runs exactly 600 block ticks, and 1,200 stored ticks buy two uses whose third cannot reach rate 4
EXIT:0
```

The `info` line prints the accelerator's remaining budget (`acc=1x594`), so
"ran exactly its remaining budget" is an exact statement: the furnace's total
cooked ticks (outputs × 200 + `cooking_time_spent`) advanced by the number of
block ticks the accelerator still owed, and the control furnace (same chunk,
never accelerated) advanced by the clock alone. The same run is fail-capable
three ways: `SELFTEST=1` flips E's assertion by one tick and exits 1; B, C and
D are controls that must not change a number; and the retired embedded-content
control leg showed the ids unknown without the content.

### Packaged artifacts

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `tiab.hmod` (0.2.3, SDK, delivered) | 33,762 B | `c5fcfe581c7bc0b0dc1e6bbefa4b2470d3588dcd6c7284d76a033ec50d5f1ab6` | 25 |
| `tiab.hmod` (0.2.1, SDK) | 30,049 B | `af9399a5e6d02184bb1bd15646de114d253ffc33006ab365ddb2fe119f49f8d1` | 25 |
| `tiab.hmod` (0.2.0, SDK) | 28,391 B | `5546d00d88d90e1383d6245dfd1ca3a6f217c824245244a6b382f00ebe09850b` | 25 |
| `content/tiab.hmod` (0.1.0, retired) | 5,973 B | `753112b914754202c680584f9b1c094f5d6955755df33735ff1a8b3b275e9f36` | 17 |
| `tiab.hnmod` (0.1.0, retired) | 18,685 B | `d6c974f348a339cdf8976f31835d24f39071845f62cfa3b244f3bb58e89257ec` | 14 |

The 0.1.0 pair is kept in `ports/_shared/evidence/tiab/retired-hnmod-0.1.0/`
(the `.hnmod` also in `ports/_shared/horizon-mods/publish/tiab/`, which still
holds the 0.1.0 tree and needs regenerating by the orchestrator — outside this
job's write ownership).

## Provenance (upstream analysis)

Repository `https://github.com/RealMangorage/time-in-a-bottle`, commit
**`b8bb6d62f379d2d132193c10b6651e029057895e`** (2025-12-20, the last 1.21.1
state: `gradle.properties` `version=6.5.3`, `minecraft_version=1.21.1`; the
jar's `neoforge.mods.toml` says 6.5.4). Files read and behaviour extracted:
[`FINDINGS.md`](FINDINGS.md); the shallow clone was deleted after the findings
were written. The jar stays as licence evidence in
`ports/_shared/licence-cache/tiab-neoforge-6.5.4.jar`. A public repo does not
relax the publishing bar: MIT, maintainer approval still required.

## Files

- `horizon-mod.toml` — the only manifest: `id = "tiab"`, `version = "0.2.0"`,
  `api = "0.2"`, `side = "both"`, `[native]` pinned to `0b14f8a` / `e59da90`.
- `Cargo.toml` — one crate; the only dependency is `horizon-mod-api`.
- `src/lib.rs` — the `Hooks` impl (component types, commands, the click, the
  level tick) and the `Bottle` `ItemPart` (carried account, tooltip).
- `src/glue.rs` — the account's component, the accelerator table, the furnace
  fast-forward and its readout.
- `src/runtime.rs` — the pure rules, byte-identical to the retired file.
- `assets/tiab/`, `data/tiab/` — the carried content (16 files).
- `LICENSE`, `LICENSE-NOTICE.md`, `README.md` — inside the package.
- `tools/e2e.sh` (the scenario; needs `GAME=` from a launcher build),
  `tools/recipe_check.py` (the carried recipe, crafted in a real table from the
  e2e), `tools/assemble_content.py` (re-creates the carried files from the jar).
- `tiab.hmod` — the delivered package (33,762 B, sha256 `c5fcfe58…1ab6`).
- `FINDINGS.md`, `CONTENT.md`, `PLAN.md`, this file.

## Review fixes (2026-11-18 review, rebuilt 2026-11-19 at 0.2.1)

Skills: superpowers/using-superpowers, systematic-debugging, test-driven-development, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

**Finding (`src/glue.rs:291`, `src/lib.rs:153`).** `/tiab info` accepted arbitrary coordinates in the
player's level, read a block entity there and reported slot 2's count for any ITEMS container, with
no distance or access check.

**Fix.** `runtime::info_permitted(permission, distance)` (an operator — level 2 — or the console may
read anywhere; everyone else only within `INFO_REACH` = 6 blocks, an unreadable distance refused) is
checked before the readout, and `glue::furnace_state` now answers `(-1, 0, 0)` unless the block is a
furnace, smoker or blast furnace (`runtime::furnace_block`, which `glue::is_furnace` also uses), so
another container's slot 2 is never read. A refusal prints `INFO_DENIED`.

**Tests.** `runtime::tests::a_diagnostic_read_needs_reach_or_permission` and
`only_the_furnace_family_is_read` (both red against stubs that allowed everything).

**A port defect the in-game leg found (fixed here).** The migrated `glue::extra_block_ticks` returned
`rate - 1`, so a rate-1 use was a no-op; upstream's `TimeAcceleratorEntity.tick` runs the target's
ticker `getTimeRate()` times *in addition* to the engine's tick (read at
`common/…/TimeAcceleratorEntity.java:64-77`), and C's `ran exactly 594 block ticks` assertion failed
at 0. The function now returns `rate`, pinned by `glue::tests::a_rate_one_accelerator_owes_one_extra_tick`.
The cook timer is also written **at most at `total`** (vanilla completes on `>=`), so a rate-1 step is
never dropped; a rate above 1 can lose up to `rate - 1` ticks on the step that completes a cook, which
is F's recorded bound (`glue::tests::the_cook_timer_is_capped_at_its_total`).

**Ladder (0.2.1).** `cargo test` 23 passed · clippy `-D warnings` exit 0 · `fmt --check` exit 0 · two
builds byte-identical · `check` exit 0 (`tiab 0.2.1 …`) · `fits --build 0b14f8a` exit 0.

**In game.** `launcher-game.sh run tiab` → `PASS` + `E2E EXIT:0` (C 594, D 0, E 600/576, F 2×567 with
8 lost finishing 12 cooks inside the bound); control + `EXIT:1`. Evidence:
`ports/_shared/evidence/tiab-e2e/`.

**Package.** `tiab.hmod` **30,049 B**, sha256
`af9399a5e6d02184bb1bd15646de114d253ffc33006ab365ddb2fe119f49f8d1`.
