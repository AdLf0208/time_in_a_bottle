//! Game-side glue for Time in a Bottle: the bottle's `tiab:stored_time`
//! account, the accelerator table, the accelerated families' timers, the
//! command bodies and the readout.
//!
//! The accelerator is upstream's `TimeAcceleratorEntity` without the entity:
//! the mod API lets a mod declare a block entity type but not an entity type,
//! so `{level, pos, rate, remaining}` lives here and is spent at
//! `level_tick_post`. Upstream ticks its target through the target's own
//! `BlockEntityTicker`; the API exposes no way to run a vanilla block entity's
//! tick (`block_tick` runs the block's *scheduled* tick, not its block
//! entity's), so the extra ticks are written into the timers that block
//! entity's own tick reads — a furnace's [`fast_forward`], a campfire's four
//! `CookingTimes` — and the engine's own tick still finishes the job.
//!
//! Everything here is glue: the rules are in [`crate::runtime`]. Only
//! [`fast_forward`] and the campfire timer step are the API gap's arithmetic,
//! and both are unit-tested.

use std::sync::{Mutex, MutexGuard};

use mod_api::{
    Compound, HostTransaction, ItemKey, ItemPlace, ItemStack, LevelRef, ServerHost, Tag, Text,
    Transaction,
};

use crate::runtime::{Accelerator, Family, Fire, StoredTime};

/// `Registration.TIAB_ITEM`: the bottle's item id.
pub(crate) const BOTTLE: &str = "tiab:time_in_a_bottle";
/// `Registration.STORED_TIME_COMPONENT`: the bottle's account component.
pub(crate) const STORED_TIME: &str = "tiab:stored_time";
/// `TiabMod.TIAB_UN_ACCELERATABLE`: blocks the bottle never accelerates.
pub(crate) const UN_ACCELERATABLE: &str = "tiab:un_acceleratable";
/// `TiabCommand`'s `requires(hasPermission(2))`.
pub(crate) const PERMISSION: u8 = 2;
/// `CampfireBlockEntity`: four cooking slots, `CookingTimes` and `CookingTotalTimes` each.
const CAMPFIRE_SLOTS: usize = 4;

/// One accelerator the port is running, by level and block position.
struct Running {
    level: LevelRef,
    pos: [i32; 3],
    accelerator: Accelerator,
}

/// The accelerators in flight. Upstream keeps each one as an entity in its
/// level; the API cannot declare one, so they are `level_tick_post` state and
/// are not saved (recorded gap).
static RUNNING: Mutex<Vec<Running>> = Mutex::new(Vec::new());

/// The accelerators, for the hooks that add to and spend them.
fn running() -> MutexGuard<'static, Vec<Running>> {
    // A panic while the lock is held cannot corrupt the list: every write below
    // is a push, a field assignment or a `remove` that keeps the vector valid.
    RUNNING.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// The accelerator at `pos` in `level`, if one runs there.
pub(crate) fn accelerator_at(level: LevelRef, pos: [i32; 3]) -> Option<Accelerator> {
    running()
        .iter()
        .find(|running| running.level == level && running.pos == pos)
        .map(|running| running.accelerator)
}

/// Adds an accelerator at `pos`, or replaces the one already there.
pub(crate) fn set_accelerator(level: LevelRef, pos: [i32; 3], accelerator: Accelerator) {
    let mut accelerators = running();
    match accelerators
        .iter_mut()
        .find(|running| running.level == level && running.pos == pos)
    {
        Some(running) => running.accelerator = accelerator,
        None => accelerators.push(Running {
            level,
            pos,
            accelerator,
        }),
    }
}

/// Drops the accelerators of levels that are no longer open: a world reopened
/// gets a new [`LevelRef`], so its old entries can never run again.
pub(crate) fn prune(open: &[LevelRef]) {
    running().retain(|running| open.contains(&running.level));
}

/// The levels whose accelerators moved this tick, and each one's entries.
///
/// The `level_tick_post` pass takes its own level's entries out, ticks them and
/// keeps the survivors, so a level's list can never be ticked twice in a tick.
pub(crate) fn take_for(level: LevelRef) -> Vec<([i32; 3], Accelerator)> {
    let mut accelerators = running();
    let mut taken = Vec::new();
    let mut index = 0;
    while index < accelerators.len() {
        if accelerators[index].level == level {
            let gone = accelerators.remove(index);
            taken.push((gone.pos, gone.accelerator));
        } else {
            index += 1;
        }
    }
    taken
}

/// The bottle's account; [`StoredTime::EMPTY`] when the component is absent
/// (the item's default is the same `{stored: 0, total: 0}`).
pub(crate) fn stored(stack: &ItemStack) -> StoredTime {
    let Some(patch) = stack.components() else {
        return StoredTime::EMPTY;
    };
    let Some(Tag::Compound(data)) = patch.as_nbt().get(&STORED_TIME.into()) else {
        return StoredTime::EMPTY;
    };
    let int = |key: &str| match data.get(&key.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    match (int("stored"), int("total")) {
        (Some(stored), Some(total)) => StoredTime { stored, total },
        _ => StoredTime::EMPTY,
    }
}

/// Writes the account into the stack's component, as upstream's
/// `StoredTimeComponent` (one `{stored, total}` compound).
pub(crate) fn set_stored(stack: &mut ItemStack, time: StoredTime) -> bool {
    let mut data = Compound::new();
    data.insert("stored".into(), Tag::Int(time.stored));
    data.insert("total".into(), Tag::Int(time.total));
    let mut patch = Compound::new();
    patch.insert(STORED_TIME.into(), Tag::Compound(data));
    stack.apply_components(&patch).is_ok()
}

/// The first bottle in the main inventory; upstream's `findTiabItem` reads
/// `getInventory().items` (36 slots) in order.
pub(crate) fn first_bottle_slot(
    host: &mut dyn ServerHost,
    player: u128,
    bottle: ItemKey,
) -> Option<i32> {
    (0..36).find(|slot| {
        ItemPlace::PlayerSlot {
            player,
            slot: *slot,
        }
        .resource(host)
        .is_some_and(|resource| resource.item() == bottle)
    })
}

/// The account of the bottle in `slot`.
pub(crate) fn account_in_slot(
    host: &mut dyn ServerHost,
    player: u128,
    slot: i32,
) -> Option<StoredTime> {
    let stack = ItemPlace::PlayerSlot { player, slot }.stack(host)?;
    Some(stored(&stack))
}

/// Replaces the bottle in `slot`'s account, keeping the item and count; true
/// when the write went through.
pub(crate) fn store_in_slot(
    host: &mut dyn ServerHost,
    player: u128,
    slot: i32,
    time: StoredTime,
) -> bool {
    let place = ItemPlace::PlayerSlot { player, slot };
    host.transaction(|host, tx: &Transaction| {
        let wrote = place.edit(host, tx, |stack| {
            set_stored(stack, time);
        });
        Ok::<bool, ()>(wrote)
    })
    .unwrap_or(false)
}

/// The refusal `/tiab info` answers a source without reach or permission. One line, so the
/// harness readout and a player's probe read the same way.
pub(crate) const INFO_DENIED: &str =
    "tiab info: too far away; stand within 6 blocks of the furnace or ask an operator";

/// The extra block ticks one accelerator owes on top of the furnace's own single tick for this
/// game tick: upstream's `TimeAcceleratorEntity.tick` runs the target's ticker `rate` times per
/// game tick in addition to the engine's own tick, so a rate-1 use doubles a furnace's speed.
pub(crate) const fn extra_block_ticks(rate: i32) -> i32 {
    rate
}

/// A furnace's saved timers after `extra` accelerated block ticks.
///
/// Returns `None` when there is nothing to advance: the accelerator runs at
/// rate 1 (vanilla already ticks), the furnace is unlit (the engine lights it,
/// and burns its fuel, on its own tick), or its cook time is not known yet.
///
/// The timer is written **at most at `total`**, never past it: vanilla's own tick completes the
/// cook on `cooking_time_spent >= cooking_total_time`, so a step that would overshoot still
/// counts every tick it can (no part of a rate-1 step is dropped) and the engine stays the one
/// that produces the output. A rate above 1 can overshoot by up to `extra - 1` on the step that
/// completes a cook; upstream runs the furnace's own routine and carries that over, and the
/// difference is recorded in `notes.md`.
pub(crate) fn fast_forward(timer: i32, total: i32, lit: i32, extra: i32) -> Option<(i32, i32)> {
    if extra <= 0 || lit <= 0 || total <= 1 {
        return None;
    }
    let timer = timer.saturating_add(extra).min(total);
    let lit = lit.saturating_sub(extra).max(0);
    Some((timer, lit))
}

/// The family of the block entity at `pos`, when the accelerator can drive it.
pub(crate) fn family_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<Family> {
    let block = level.block(host, pos)?;
    crate::runtime::accelerated_family(&block.id(&*host))
}

/// Whether the block at `pos` is in `#tiab:un_acceleratable`.
pub(crate) fn un_acceleratable(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(block) = level.block(host, pos) else {
        return false;
    };
    host.block_in_tag(block, UN_ACCELERATABLE).unwrap_or(false)
}

/// Runs `extra` accelerated block ticks of the block entity at `pos`: the saved timers its own
/// routine reads move forward together, so it cooks and burns fuel at the accelerator's rate.
///
/// One family per entry of [`Family`]: a furnace's `cooking_time_spent`/`lit_time_remaining`, a
/// campfire's four `CookingTimes`. False when there is nothing there to advance (a stale handle, a
/// block outside the families, an unlit furnace, a block without the timers).
pub(crate) fn accelerate(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    extra: i32,
) -> bool {
    match family_at(host, level, pos) {
        Some(Family::Furnace) => fast_forward_furnace(host, level, pos, extra),
        Some(Family::Campfire) => fast_forward_campfire(host, level, pos, extra),
        None => false,
    }
}

/// [`fast_forward`] on the furnace at `pos`, read from and written back to its saved timers.
fn fast_forward_furnace(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    extra: i32,
) -> bool {
    let Some(entity) = host.block_entity(level, pos) else {
        return false;
    };
    let Some(data) = entity.data(&*host) else {
        return false;
    };
    let int = |key: &str| match data.get(&key.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    let (Some(timer), Some(total), Some(lit)) = (
        int("cooking_time_spent"),
        int("cooking_total_time"),
        int("lit_time_remaining"),
    ) else {
        return false;
    };
    let Some((timer, lit)) = fast_forward(timer, total, lit, extra) else {
        return false;
    };
    let mut patch = Compound::new();
    patch.insert("cooking_time_spent".into(), Tag::Int(timer));
    patch.insert("lit_time_remaining".into(), Tag::Int(lit));
    entity.merge_data(host, patch).unwrap_or(false)
}

/// The campfire at `pos` after `extra` accelerated block ticks: every occupied slot's
/// `CookingTimes` entry moves as [`crate::runtime::campfire_progress`] says, and the campfire's
/// own tick drops the finished item at its cooking time.
fn fast_forward_campfire(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    extra: i32,
) -> bool {
    let Some(entity) = host.block_entity(level, pos) else {
        return false;
    };
    let Some(data) = entity.data(&*host) else {
        return false;
    };
    let (Some(progress), Some(total)) = (
        int_array(&data, "CookingTimes"),
        int_array(&data, "CookingTotalTimes"),
    ) else {
        return false;
    };
    let occupied = campfire_slots(&data);
    let fire = match level.block_property(host, pos, "lit").as_deref() {
        Some("true") => Fire::Lit,
        _ => Fire::Unlit,
    };
    let mut next = progress;
    let mut changed = false;
    for slot in 0..CAMPFIRE_SLOTS {
        if !occupied[slot] {
            continue;
        }
        let after = crate::runtime::campfire_progress(progress[slot], total[slot], extra, fire);
        if after != progress[slot] {
            next[slot] = after;
            changed = true;
        }
    }
    if !changed {
        return false;
    }
    let mut patch = Compound::new();
    patch.insert("CookingTimes".into(), Tag::IntArray(next.to_vec()));
    entity.merge_data(host, patch).unwrap_or(false)
}

/// A four-entry integer array from a block entity's saved data (`CookingTimes` and its totals).
fn int_array(data: &Compound, key: &str) -> Option<[i32; CAMPFIRE_SLOTS]> {
    match data.get(&key.into()) {
        Some(Tag::IntArray(values)) => Some(std::array::from_fn(|index| {
            values.get(index).copied().unwrap_or(0)
        })),
        _ => None,
    }
}

/// Which campfire slots hold an item, from its saved `Items` list (`CampfireBlockEntity`'s four
/// `Container` slots, each stack carrying its `Slot` byte). The saved list is the source: a
/// campfire is not a menu container, so the item-handler view of it need not answer.
fn campfire_slots(data: &Compound) -> [bool; CAMPFIRE_SLOTS] {
    let mut occupied = [false; CAMPFIRE_SLOTS];
    let Some(Tag::List(items)) = data.get(&"Items".into()) else {
        return occupied;
    };
    for item in items {
        let Tag::Compound(entry) = item else {
            continue;
        };
        let slot = match entry.get(&"Slot".into()) {
            Some(Tag::Byte(slot)) => usize::from(*slot as u8),
            Some(Tag::Int(slot)) => usize::try_from(*slot).unwrap_or(usize::MAX),
            _ => continue,
        };
        if let Some(cell) = occupied.get_mut(slot) {
            *cell = true;
        }
    }
    occupied
}

/// `/tiab info <x> <y> <z> <n>`: the tick clock, the first bottle's account and
/// the accelerated block entity at `pos`, in one line (the harness readout;
/// upstream has no commands of its own besides `/tiab time`).
///
/// `kind` names the family the line reports; `cook`/`ct` are the first occupied timer and its
/// cooking time, `out` the items the block holds (a furnace's output slot, a campfire's slots) and
/// `lit` the fuel or fire state.
pub(crate) fn info_line(
    host: &mut dyn ServerHost,
    player: u128,
    bottle: ItemKey,
    level: LevelRef,
    pos: [i32; 3],
    sequence: i32,
) -> String {
    let time = first_bottle_slot(host, player, bottle)
        .and_then(|slot| account_in_slot(host, player, slot))
        .unwrap_or(StoredTime::EMPTY);
    let target = target_state(host, level, pos);
    let accelerator = accelerator_at(level, pos).map_or_else(
        || "none".to_owned(),
        |accelerator| format!("{}x{}", accelerator.rate, accelerator.remaining),
    );
    let clock = level.day_time(&*host).unwrap_or(-1);
    format!(
        "tiab {},{},{} n={sequence} t={clock} stored={} total={} kind={} cook={} ct={} out={} lit={} acc={accelerator}",
        pos[0],
        pos[1],
        pos[2],
        time.stored,
        time.total,
        target.kind,
        target.cook,
        target.total,
        target.out,
        target.lit
    )
}

/// What `/tiab info` reports of the block at `pos`: the family and its timers.
struct TargetState {
    /// `furnace`, `campfire` or `none`.
    kind: &'static str,
    /// The cook timer (a furnace's `cooking_time_spent`, a campfire's first occupied slot).
    cook: i32,
    /// The cooking time that timer runs to.
    total: i32,
    /// The items the block holds: a furnace's output slot, a campfire's occupied slots.
    out: u32,
    /// A furnace's `lit_time_remaining`, or a campfire's `lit` state as 1/0.
    lit: i32,
}

/// The state of the block entity at `pos`; `kind = none` when it is not a family this mod drives.
fn target_state(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> TargetState {
    let none = TargetState {
        kind: "none",
        cook: -1,
        total: 0,
        out: 0,
        lit: 0,
    };
    let Some(family) = family_at(host, level, pos) else {
        return none;
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return none;
    };
    let Some(data) = entity.data(&*host) else {
        return none;
    };
    let int = |key: &str| match data.get(&key.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    match family {
        Family::Furnace => TargetState {
            kind: "furnace",
            cook: int("cooking_time_spent").unwrap_or(-1),
            total: int("cooking_total_time").unwrap_or(0),
            out: ItemPlace::BlockEntity {
                level,
                pos,
                side: None,
                slot: 2,
            }
            .amount(host),
            lit: int("lit_time_remaining").unwrap_or(0),
        },
        Family::Campfire => {
            let progress = int_array(&data, "CookingTimes").unwrap_or([0; CAMPFIRE_SLOTS]);
            let total = int_array(&data, "CookingTotalTimes").unwrap_or([0; CAMPFIRE_SLOTS]);
            let occupied = campfire_slots(&data);
            let first = occupied.iter().position(|slot| *slot);
            let out = occupied.iter().filter(|slot| **slot).count() as u32;
            let lit = i32::from(matches!(
                level.block_property(host, pos, "lit").as_deref(),
                Some("true")
            ));
            TargetState {
                kind: "campfire",
                cook: first.map_or(-1, |slot| progress[slot]),
                total: first.map_or(0, |slot| total[slot]),
                out,
                lit,
            }
        }
    }
}

/// The two `%s` halves of a `HH:MM:SS` clock, as the carried lang keys take
/// them.
pub(crate) fn clock_parts(ticks: i32) -> [Text; 3] {
    let clock = StoredTime::clock_text(ticks);
    let mut parts = clock.split(':');
    let mut next = || Text::literal(parts.next().unwrap_or("00").to_owned());
    [next(), next(), next()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upstream's `TimeAcceleratorEntity.tick` runs the target's ticker `rate` times on top
    /// of the engine's own tick, so a rate-1 accelerator owes exactly one extra block tick a
    /// game tick (the first use is the 2x speed the item promises). The in-game leg's E phase
    /// measured 600 block ticks for a 600-tick rate-1 use, which is what this pins.
    #[test]
    fn a_rate_one_accelerator_owes_one_extra_tick() {
        assert_eq!(extra_block_ticks(1), 1);
        assert_eq!(extra_block_ticks(2), 2);
        assert_eq!(fast_forward(10, 200, 100, 0), None);
    }

    #[test]
    fn the_timer_advances_by_the_extra_ticks() {
        assert_eq!(fast_forward(10, 200, 100, 1), Some((11, 99)));
        assert_eq!(fast_forward(10, 200, 100, 63), Some((73, 37)));
    }

    #[test]
    fn the_cook_timer_is_capped_at_its_total() {
        // The timer is written at most at the total, never past it: vanilla's own tick is what
        // completes the cook (`cooking_time_spent >= cooking_total_time`), and a timer written
        // at the total still counts the whole accelerated step. A timer short of it is
        // advanced normally, so at rate 1 no accelerated tick is ever lost.
        assert_eq!(fast_forward(199, 200, 100, 1), Some((200, 99)));
        assert_eq!(fast_forward(199, 200, 100, 63), Some((200, 37)));
        assert_eq!(fast_forward(150, 200, 100, 63), Some((200, 37)));
        assert_eq!(fast_forward(10, 200, 100, 1), Some((11, 99)));
    }

    #[test]
    fn the_fuel_runs_out_at_the_accelerated_rate_and_stops_at_zero() {
        assert_eq!(fast_forward(0, 200, 100, 63), Some((63, 37)));
        assert_eq!(fast_forward(0, 200, 10, 63), Some((63, 0)));
    }

    #[test]
    fn an_unlit_or_unmeasured_furnace_is_left_alone() {
        // The engine lights the furnace and burns its fuel on its own tick.
        assert_eq!(fast_forward(10, 200, 0, 63), None);
        assert_eq!(fast_forward(10, 0, 100, 63), None);
        assert_eq!(fast_forward(10, 1, 100, 63), None);
    }
}
