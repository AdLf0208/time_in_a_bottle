# Time in a Bottle — port notes (slice 1, migrated to the SDK format)

Ported from **Time in a Bottle 6.5.4** (`tiab-neoforge-6.5.4.jar`, 88 entries,
SHA-256 `0beb4ff1d81facfe9039e8d657a75836d1e8c13c137c90b491cc07e763f26325`) for
Minecraft Java 1.21.1 / NeoForge. **MIT → licence rank 0**; local-only until
publishing is approved (ARCHITECTURE.md §8). Clean-room Rust rewrite from the
mod's behaviour; what the mod does and its evidence are in
[`FINDINGS.md`](FINDINGS.md).

**Format: the SDK package (2026-11-18), version 0.2.0.** This port was migrated
from the retired `.hnmod` container to one flat `.hmod` (mod API `0.2`, SDK
nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**). `src/runtime.rs` is
**byte-identical** to the pre-migration `mixins/tiab/runtime.rs` — sha256
`3594d9499e82ba0b8efd1ad31e30521d2583c419327692dd4302eede661be9ae`, `cmp`
against the retired copy and against the file inside the delivered package. See
[`## Migration to the SDK format`](#migration-to-the-sdk-format) for the file
map, the method map and every trap.

Status: **ported-partial** — slice 1 (the bottle's stored-tick account, one
accelerator per block, the furnace family as the target, `/tiab time` and the
harness readout) is implemented; the 0.1.0 in-game run is its behaviour
evidence, and the SDK package was **not** re-run in game here (no
`horizon-launcher`, trap 15). The gap list below is the rest of the mod.

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

## What the slice does

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
  runs before the furnace's own `use`, so the menu never opens): a furnace,
  smoker or blast furnace that is not in `#tiab:un_acceleratable` gets an
  accelerator. One use costs **600** stored ticks and runs 600 game ticks at
  rate 1; a second click doubles the rate (`rate / 2 * 600` at the new rate) and
  adds half of the time already used up, up to 128x. Creative mode pays nothing
  (`canUse`).
- **Accelerating.** At `level_tick_post` each accelerator runs `rate` block
  ticks on its furnace, spends one game tick and removes itself at zero. The
  extra `rate - 1` ticks are written into the furnace's saved
  `cooking_time_spent`/`lit_time_remaining`, so it cooks and burns fuel at the
  accelerator's rate (§The API gap).
- **Commands.** `/tiab time add|remove <player> <seconds>` (permission level
  2, upstream's clamp `MAX_STORED_TIME / TICKS_CONST`) and
  `/tiab info <x> <y> <z> <n>` — the harness readout (clock, the bottle's
  account, the furnace's cook/out/lit and the accelerator's rate/remaining in
  one line). Upstream has no commands besides `/tiab time`; the readout is the
  ports' `/powah info` precedent.
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
| `level_tick_post` | `Hooks::level_tick_post` | run each accelerator's extra furnace ticks |
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

### 8. Changelog (the resubmission supersedes #828)

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
2. **Only the furnace family is accelerated** (`minecraft:furnace`, `smoker`,
   `blast_furnace`). Upstream ticks any block entity's own ticker — hoppers,
   campfires, brewing stands, mod machines — and randomly-ticking blocks at a
   1/1365 chance per accelerated tick. Those targets do nothing here (the click
   is claimed and the accelerator ends on its first tick because the target is
   not a furnace).
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
10. **The acceleration is a saved-timer fast-forward, not extra furnace
    ticks** (§5): one cook per game tick at most, the last cook's remainder
    dropped, and the block's `lit` property follows the engine's own tick (so
    it can lag one tick behind the saved timers). A high rate therefore runs
    slightly slower than upstream's `rate` multiplier; 1x and 2x are exact
    except for that last-cook remainder.
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

## Verification

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

Pin **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher 0.2.78. Every command
ran directly in the port (the SDK layout; `build-slot.sh` speaks the old tool)
with `CARGO_INCREMENTAL=0`, no `--target-dir`. Evidence:
`ports/_shared/evidence/tiab/` (`sdk-*.log`, `sdk-uses.json`,
`retired-hnmod-0.1.0/`).

| Check | Command | Result |
|---|---|---|
| Runtime + glue unit tests | `CARGO_INCREMENTAL=0 cargo test --offline` | **21 passed**, 0 failed (`sdk-test.log`) |
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
  `tools/assemble_content.py` (re-creates the carried files from the jar).
- `tiab.hmod` — the delivered package (28,391 B, sha256 `5546d00d…9850b`).
- `FINDINGS.md`, `CONTENT.md`, `PLAN.md`, this file.
