//! Game-side glue for Time in a Bottle: the bottle's `tiab:stored_time`
//! account, the accelerator table, the furnace fast-forward, the command
//! bodies and the readout.
//!
//! The accelerator is upstream's `TimeAcceleratorEntity` without the entity:
//! the mod API lets a mod declare a block entity type but not an entity type,
//! so `{level, pos, rate, remaining}` lives here and is spent at
//! `level_tick_post`. Upstream ticks its target through the target's own
//! `BlockEntityTicker`; the API exposes no way to run a vanilla block entity's
//! tick (`block_tick` runs the block's *scheduled* tick, not its block
//! entity's), so the extra ticks are written into the furnace's saved timers
//! instead — see [`fast_forward`] and `notes.md`.
//!
//! Everything here is glue: the rules are in [`crate::runtime`], which stays
//! game-free and byte-identical to the pre-migration file. Only
//! [`fast_forward`] is new arithmetic (the API gap's shape), and it is
//! unit-tested.

use std::sync::{Mutex, MutexGuard};

use mod_api::{
    Compound, HostTransaction, ItemKey, ItemPlace, ItemStack, LevelRef, ServerHost, Tag, Text,
    Transaction,
};

use crate::runtime::{Accelerator, StoredTime};

/// `Registration.TIAB_ITEM`: the bottle's item id.
pub(crate) const BOTTLE: &str = "tiab:time_in_a_bottle";
/// `Registration.STORED_TIME_COMPONENT`: the bottle's account component.
pub(crate) const STORED_TIME: &str = "tiab:stored_time";
/// `TiabMod.TIAB_UN_ACCELERATABLE`: blocks the bottle never accelerates.
pub(crate) const UN_ACCELERATABLE: &str = "tiab:un_acceleratable";
/// `TiabCommand`'s `requires(hasPermission(2))`.
pub(crate) const PERMISSION: u8 = 2;

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

/// The extra block ticks one accelerator owes on top of the furnace's own
/// single tick for this game tick.
pub(crate) const fn extra_block_ticks(rate: i32) -> i32 {
    rate - 1
}

/// A furnace's saved timers after `extra` accelerated block ticks.
///
/// Returns `None` when there is nothing to advance: the accelerator runs at
/// rate 1 (vanilla already ticks), the furnace is unlit (the engine lights it,
/// and burns its fuel, on its own tick), or its cook time is not known yet.
///
/// `timer` is held **one tick short** of `total`, so the furnace's own next
/// tick completes the cook: one cook still finishes per game tick at most, and
/// the part of the last cook left over is dropped. Upstream runs every extra
/// tick through the furnace's own routine, so it carries that remainder over —
/// the difference is recorded in `notes.md`.
pub(crate) fn fast_forward(timer: i32, total: i32, lit: i32, extra: i32) -> Option<(i32, i32)> {
    if extra <= 0 || lit <= 0 || total <= 1 {
        return None;
    }
    let timer = timer.saturating_add(extra).min(total - 1);
    let lit = lit.saturating_sub(extra).max(0);
    Some((timer, lit))
}

/// Whether the block at `pos` is a furnace, smoker or blast furnace: the one
/// family whose saved timers this port knows how to advance.
pub(crate) fn is_furnace(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(block) = level.block(host, pos) else {
        return false;
    };
    matches!(
        block.id(&*host).as_str(),
        "minecraft:furnace" | "minecraft:smoker" | "minecraft:blast_furnace"
    )
}

/// Whether the block at `pos` is in `#tiab:un_acceleratable`.
pub(crate) fn un_acceleratable(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(block) = level.block(host, pos) else {
        return false;
    };
    host.block_in_tag(block, UN_ACCELERATABLE).unwrap_or(false)
}

/// Runs `extra` accelerated block ticks of the furnace at `pos`: the saved
/// `cooking_time_spent` and `lit_time_remaining` move forward together, so the
/// furnace cooks and burns fuel at the accelerator's rate. False when there is
/// nothing there to advance (a stale handle, an unlit furnace, a block without
/// the timers).
pub(crate) fn accelerate(
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

/// `/tiab info <x> <y> <z> <n>`: the tick clock, the first bottle's account and
/// the furnace at `pos`, in one line (the harness readout; upstream has no
/// commands of its own besides `/tiab time`).
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
    let (cook, out, lit) = furnace_state(host, level, pos);
    let accelerator = accelerator_at(level, pos).map_or_else(
        || "none".to_owned(),
        |accelerator| format!("{}x{}", accelerator.rate, accelerator.remaining),
    );
    let clock = level.day_time(&*host).unwrap_or(-1);
    format!(
        "tiab {},{},{} n={sequence} t={clock} stored={} total={} cook={cook} out={out} lit={lit} acc={accelerator}",
        pos[0], pos[1], pos[2], time.stored, time.total
    )
}

/// The furnace at `pos`: its cook timer, its output count and its remaining
/// fuel; `-1, 0, 0` when there is no furnace there.
fn furnace_state(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> (i32, u32, i32) {
    let Some(entity) = host.block_entity(level, pos) else {
        return (-1, 0, 0);
    };
    let Some(data) = entity.data(&*host) else {
        return (-1, 0, 0);
    };
    let int = |key: &str| match data.get(&key.into()) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    };
    let cook = int("cooking_time_spent").unwrap_or(-1);
    let lit = int("lit_time_remaining").unwrap_or(0);
    let out = ItemPlace::BlockEntity {
        level,
        pos,
        side: None,
        slot: 2,
    }
    .amount(host);
    (cook, out, lit)
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

    #[test]
    fn a_rate_one_accelerator_owes_no_extra_tick() {
        assert_eq!(extra_block_ticks(1), 0);
        assert_eq!(fast_forward(10, 200, 100, 0), None);
    }

    #[test]
    fn the_timer_advances_by_the_extra_ticks() {
        assert_eq!(fast_forward(10, 200, 100, 1), Some((11, 99)));
        assert_eq!(fast_forward(10, 200, 100, 63), Some((73, 37)));
    }

    #[test]
    fn the_last_tick_of_a_cook_is_left_to_the_furnace() {
        // One tick short of the total, never at or past it: the furnace's own
        // tick is what completes the cook.
        assert_eq!(fast_forward(199, 200, 100, 63), Some((199, 37)));
        assert_eq!(fast_forward(150, 200, 100, 63), Some((199, 37)));
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
