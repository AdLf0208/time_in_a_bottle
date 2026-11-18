//! Time in a Bottle for Horizon: a bottle that stores ticks and spends them.
//!
//! Time in a Bottle (see `FINDINGS.md`) gives one item, a stored-tick account
//! on it, and an accelerator that makes a block tick faster. This package is
//! the item, its carried assets and data, and the code that spends the
//! account: while the first bottle in an inventory is carried it gains one
//! stored tick per game tick, and right-clicking a furnace, smoker or blast
//! furnace with it spends [`runtime::EACH_USE_TICKS`] of them for one use —
//! 600 game ticks at one extra block tick each. A second click on the same
//! block doubles the rate, up to [`runtime::MAX_RATE`].
//!
//! The account and one accelerator's arithmetic are [`runtime`]'s, pure and
//! unit-tested, and byte-identical to the pre-SDK port. [`glue`] holds the
//! game side: the item's `tiab:stored_time` component, the accelerator table
//! (the API cannot declare an entity) and the furnace's saved timers.

mod glue;
pub mod runtime;

use std::sync::OnceLock;

use mod_api::{
    self, ClientHost, Command, ComponentTypesEvent, Flow, Hooks, HostSide, HostTransaction,
    InteractionResult, InventoryTickEvent, ItemKey, ItemPart, ItemPlace, LevelTickPostEvent,
    PlayerCommandEvent, RegisterCommandsEvent, Registrar, RegistrationContext,
    RightClickBlockEvent, ServerHost, Text, TooltipEvent,
};

use crate::glue::{
    BOTTLE, STORED_TIME, accelerator_at, account_in_slot, clock_parts, extra_block_ticks,
    first_bottle_slot, info_line, is_furnace, set_accelerator, store_in_slot, stored, take_for,
    un_acceleratable,
};
use crate::runtime::{Accelerator, TICKS_CONST, seconds_to_ticks};

/// The mod's session state: the bottle's registry key, needed to recognise the
/// carried stack from a global hook.
#[derive(Default)]
struct Mod {
    bottle: OnceLock<ItemKey>,
}

impl Mod {
    /// The bottle's item key; the registration below always sets it first.
    fn bottle(&self) -> Option<ItemKey> {
        self.bottle.get().copied()
    }
}

/// The bottle's own moments: `inventory_tick` and `tooltip` belong to one item,
/// so the API calls them through a Part attached to that item.
struct Bottle {
    key: ItemKey,
}

#[mod_api::horizon_part]
impl ItemPart for Bottle {
    /// `TiabItem.tickPlayer`: at the end of every second, the first bottle in
    /// the player's inventory stores one tick per game tick.
    fn inventory_tick(&self, host: &mut dyn ServerHost, event: &mut InventoryTickEvent<'_>) {
        let Some(player) = host.players().acting_player() else {
            return;
        };
        let uuid = player.uuid();
        let Some(level) = player.level(&*host) else {
            return;
        };
        let Some(clock) = level.day_time(&*host) else {
            return;
        };
        if clock % i64::from(TICKS_CONST) != 0 {
            return;
        }
        if first_bottle_slot(host, uuid, self.key) != Some(event.slot) {
            return;
        }
        let Some(before) = account_in_slot(host, uuid, event.slot) else {
            return;
        };
        let after = before.accumulate(TICKS_CONST);
        if after == before {
            return;
        }
        store_in_slot(host, uuid, event.slot, after);
    }

    /// The two lore lines, as upstream's `CommonHelper` builds them.
    fn tooltip(&self, _host: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
        if event.stack.item() != self.key {
            return;
        }
        let time = stored(event.stack);
        event.lines.push((
            Text::translate(
                "item.tiab.time_in_a_bottle.tooltip.stored_time",
                clock_parts(time.stored).to_vec(),
            ),
            0xFF55_FF55,
        ));
        event.lines.push((
            Text::translate(
                "item.tiab.time_in_a_bottle.tooltip.total_accumulated_time",
                clock_parts(time.total).to_vec(),
            ),
            0xFFAA_AAAA,
        ));
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Before content loads: declare the bottle's `tiab:stored_time` component,
    /// which the carried item names as a default.
    fn component_types(&self, _context: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        event.types.push(STORED_TIME.to_owned());
    }

    /// The bottle's registry key and the Part that hears its own moments.
    fn register(&self, r: &mut dyn Registrar, _context: &RegistrationContext) {
        let key = r.item_key(BOTTLE);
        let _ = self.bottle.set(key);
        r.item_part(key, Bottle { key });
    }

    /// `/tiab time add|remove <player> <seconds>` (upstream's permission level
    /// 2) and `/tiab info <x> <y> <z> <n>`, the harness readout.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::op(
            "tiab time add <player:player> <seconds:integer>",
            glue::PERMISSION,
        ));
        event.commands.push(Command::op(
            "tiab time remove <player:player> <seconds:integer>",
            glue::PERMISSION,
        ));
        event
            .commands
            .push(Command::new("tiab info <pos:block_pos> <n:integer>"));
    }

    /// The command bodies. The API has no inventory write for another player,
    /// so the target has to be the acting player (recorded gap); the player's
    /// state is not swapped in here, so every answer goes through `replies`.
    fn player_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut PlayerCommandEvent<'_>,
    ) -> Flow {
        let Some(bottle) = self.bottle() else {
            return Flow::Continue;
        };
        let words: Vec<&str> = event.command.split_whitespace().collect();
        if let ["tiab", "info", x, y, z, n] = words.as_slice() {
            let (Ok(x), Ok(y), Ok(z), Ok(n)) = (
                x.parse::<i32>(),
                y.parse::<i32>(),
                z.parse::<i32>(),
                n.parse::<i32>(),
            ) else {
                event.replies.push(Text::literal("tiab info needs x y z n"));
                return Flow::Cancel;
            };
            let Some(player) = host.player(event.player) else {
                return Flow::Cancel;
            };
            let Some(level) = player.level(&*host) else {
                return Flow::Cancel;
            };
            let line = info_line(host, event.player, bottle, level, [x, y, z], n);
            event.replies.push(Text::literal(line));
            return Flow::Cancel;
        }
        let ["tiab", "time", action, target, seconds] = words.as_slice() else {
            return Flow::Continue;
        };
        let add = match *action {
            "add" => true,
            "remove" => false,
            _ => return Flow::Continue,
        };
        let Some(player) = host.player(event.player) else {
            return Flow::Cancel;
        };
        if player.permission_level(&*host).unwrap_or(0) < glue::PERMISSION {
            event.replies.push(Text::literal(
                "You do not have permission to use this command",
            ));
            return Flow::Cancel;
        }
        if !matches!(*target, "@s" | "@p") && *target != event.name {
            event.replies.push(Text::literal(
                "Only the acting player can be targeted at this pin",
            ));
            return Flow::Cancel;
        }
        let Ok(seconds) = seconds.parse::<i32>() else {
            event
                .replies
                .push(Text::literal("Seconds must be a whole number"));
            return Flow::Cancel;
        };
        let Some(slot) = first_bottle_slot(host, event.player, bottle) else {
            event
                .replies
                .push(Text::literal("No Time in a bottle item in inventory"));
            return Flow::Cancel;
        };
        let Some(time) = account_in_slot(host, event.player, slot) else {
            return Flow::Cancel;
        };
        let ticks = seconds_to_ticks(seconds);
        let after = if add {
            time.accumulate(ticks)
        } else {
            time.remove(ticks)
        };
        store_in_slot(host, event.player, slot, after);
        let verb = if add { "Added" } else { "Removed" };
        event.replies.push(Text::literal(format!(
            "{verb} {} seconds",
            ticks / TICKS_CONST
        )));
        event.replies.push(Text::literal(if add {
            "Time added to bottle!"
        } else {
            "Time removed from bottle!"
        }));
        Flow::Cancel
    }

    /// `TiabItem.useOn`: with a bottle held on a furnace, spend stored ticks for
    /// a fresh accelerator or a doubled rate, and claim the click so the
    /// furnace's menu does not open.
    fn right_click_block(&self, host: HostSide, event: &mut RightClickBlockEvent<'_>) -> Flow {
        let HostSide::Server(host) = host else {
            return Flow::Continue;
        };
        let Some(bottle) = self.bottle() else {
            return Flow::Continue;
        };
        let Some(stack) = event.click.stack else {
            return Flow::Continue;
        };
        if stack.item() != bottle {
            return Flow::Continue;
        }
        let pos = event.click.pos;
        let Some(level) = host.level(event.click.dimension.key().id()) else {
            return Flow::Continue;
        };
        // Upstream answers FAIL: the block's own use runs (a furnace opens).
        if !is_furnace(host, level, pos) || un_acceleratable(host, level, pos) {
            return Flow::Continue;
        }
        let uuid = event.click.player;
        let creative = host
            .player(uuid)
            .and_then(|player| player.game_mode(&*host))
            == Some(mod_api::GameMode::Creative);
        let place = ItemPlace::PlayerHand {
            player: uuid,
            hand: event.click.hand,
        };
        let Some(held) = place.stack(host) else {
            return Flow::Continue;
        };
        let time = stored(&held);
        let (cost, upgraded) = match accelerator_at(level, pos) {
            Some(current) => {
                let Some(upgraded) = current.upgrade() else {
                    // At the 128x ceiling upstream answers SUCCESS and changes
                    // nothing.
                    *event.cancellation_result = InteractionResult::Success;
                    return Flow::Cancel;
                };
                (upgraded.upgrade_cost(), Some(upgraded))
            }
            None => (Accelerator::new().upgrade_cost(), None),
        };
        let paid = if creative {
            Some(time)
        } else {
            time.spend(cost)
        };
        let Some(paid) = paid else {
            // Not enough stored time: upstream answers SUCCESS and charges
            // nothing.
            *event.cancellation_result = InteractionResult::Success;
            return Flow::Cancel;
        };
        if !creative {
            host.transaction(|host, tx| {
                place.edit(host, tx, |stack| {
                    glue::set_stored(stack, paid);
                });
                Ok::<(), ()>(())
            })
            .ok();
        }
        set_accelerator(level, pos, upgraded.unwrap_or_else(Accelerator::new));
        *event.cancellation_result = InteractionResult::Success;
        Flow::Cancel
    }

    /// After the level's own tick pass: every accelerator in this level spends
    /// one game tick and advances its target by its extra block ticks.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        let level = event.level;
        glue::prune(&host.levels().list_levels());
        let mut kept = Vec::new();
        for (pos, accelerator) in take_for(level) {
            // Upstream removes the accelerator when the target stops being
            // tickable or joins the block tag.
            if un_acceleratable(host, level, pos) || !is_furnace(host, level, pos) {
                continue;
            }
            let (block_ticks, next) = accelerator.run_tick();
            glue::accelerate(host, level, pos, extra_block_ticks(block_ticks));
            if let Some(next) = next {
                kept.push((pos, next));
            }
        }
        for (pos, accelerator) in kept {
            set_accelerator(level, pos, accelerator);
        }
    }
}
