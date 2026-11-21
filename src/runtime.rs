// Time in a Bottle: the stored tick account and one accelerator's arithmetic.
//
// This is the pure half of the port (upstream `StoredTimeComponent`,
// `ITiabConfig` and the two numbers `TiabItem` and `TimeAcceleratorEntity`
// work with). It is also this crate's library: `cargo test` exercises it
// without a game, and horizon-mixins injects the same file as
// `crate::game::tiab`. No game type may appear here.
//
// Every constant is read from the 1.21.1 sources recorded in `FINDINGS.md`:
// `ITiabConfig` (the four defaults), `TiabItem.getEachUseDuration` /
// `getEnergyCost`, and `TimeAcceleratorEntity.tick`.

/// `ITiabConfig.TICKS_CONST()`: one stored tick per game tick.
pub const TICKS_CONST: i32 = 20;

/// `ITiabConfig.EACH_USE_DURATION()`: one use lasts thirty seconds.
pub const EACH_USE_DURATION_SECONDS: i32 = 30;

/// `TiabItem.getEachUseDuration()`: `TICKS_CONST * EACH_USE_DURATION`.
pub const EACH_USE_TICKS: i32 = TICKS_CONST * EACH_USE_DURATION_SECONDS;

/// `ITiabConfig.MAX_STORED_TIME()`: 360 days of ticks.
pub const MAX_STORED_TIME: i32 = 622_080_000;

/// `ITiabConfig.MAX_RATE_MULTI()`: the rate ceiling is `2^(n-1)`.
pub const MAX_RATE_MULTI: i32 = 8;

/// The fastest acceleration, one doubled step below `MAX_RATE_MULTI`.
pub const MAX_RATE: i32 = 1 << (MAX_RATE_MULTI - 1);

/// Upstream's `StoredTimeComponent`: the bottle's account.
///
/// `stored` is what can still be spent (capped); `total` is every tick the
/// bottle has ever stored (upstream never caps it).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoredTime {
    /// Spendable ticks, never above [`MAX_STORED_TIME`].
    pub stored: i32,
    /// Every tick stored so far.
    pub total: i32,
}

impl StoredTime {
    /// A bottle that has stored nothing.
    pub const EMPTY: Self = Self {
        stored: 0,
        total: 0,
    };

    /// `TiabItem.tickBottle`: add `ticks` to both counters.
    ///
    /// `stored` stops at [`MAX_STORED_TIME`]; `total` keeps growing. A negative
    /// or overflow would break upstream's `isPositive` guard, so such a step
    /// leaves the account unchanged.
    pub fn accumulate(self, ticks: i32) -> Self {
        if ticks < 0 {
            return self;
        }
        let (Some(stored), Some(total)) = (
            self.stored.checked_add(ticks),
            self.total.checked_add(ticks),
        ) else {
            // Upstream's `isPositive` guard reads a wrapped sum as negative;
            // refusing the step keeps a broken account rather than one that
            // claims more time than `MAX_STORED_TIME`.
            return self;
        };
        Self {
            stored: stored.min(MAX_STORED_TIME),
            total,
        }
    }

    /// `TiabItem.canUse` + the spend: take `cost`, or nothing when short.
    ///
    /// The caller checks the result with `is_some` (a bottle that cannot pay
    /// is upstream's `SUCCESS` with no change).
    pub fn spend(self, cost: i32) -> Option<Self> {
        if cost < 0 || self.stored < cost {
            return None;
        }
        Some(Self {
            stored: self.stored - cost,
            total: self.total,
        })
    }

    /// `/tiab time remove`: take `ticks`, never below zero.
    ///
    /// Upstream's command computes `-min(seconds, stored / TICKS_CONST) * TICKS_CONST`
    /// before the same guard, so a removal larger than the balance empties it.
    pub fn remove(self, ticks: i32) -> Self {
        if ticks <= 0 {
            return self;
        }
        Self {
            stored: self.stored.saturating_sub(ticks).max(0),
            total: self.total,
        }
    }

    /// The tooltip's `Stored time HH:MM:SS` / `Total accumulated time HH:MM:SS`.
    pub fn clock_text(ticks: i32) -> String {
        let seconds = ticks.max(0) / TICKS_CONST;
        let (hours, minutes, seconds) = (seconds / 3_600, seconds % 3_600 / 60, seconds % 60);
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

/// One accelerator: upstream's `TimeAcceleratorEntity` state without the entity.
///
/// `rate` is how many block-entity ticks it runs each game tick; `remaining`
/// is how many game ticks it still runs for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accelerator {
    /// Block-entity ticks per game tick, 1..=[`MAX_RATE`].
    pub rate: i32,
    /// Game ticks left before it removes itself.
    pub remaining: i32,
}

impl Accelerator {
    /// `TiabItem.useOn`'s first use: rate 1 for one whole use.
    pub fn new() -> Self {
        Self {
            rate: 1,
            remaining: EACH_USE_TICKS,
        }
    }

    /// One game tick of acceleration: the block ticks to run now, and the
    /// accelerator after the tick (`None` when this was its last one).
    ///
    /// `TimeAcceleratorEntity.tick` runs its block ticks first, then
    /// `remaining -= 1`, then removes the entity at `<= 0`, so the last tick
    /// still accelerates and `remaining` counts the ticks left including the
    /// current one.
    pub fn run_tick(self) -> (i32, Option<Self>) {
        let remaining = self.remaining - 1;
        (
            self.rate,
            (remaining > 0).then_some(Self { remaining, ..self }),
        )
    }

    /// `TiabItem.getEnergyCost`: 600 at rate 1, else `rate / 2 * 600`.
    pub fn upgrade_cost(self) -> i32 {
        if self.rate <= 1 {
            EACH_USE_TICKS
        } else {
            self.rate / 2 * EACH_USE_TICKS
        }
    }

    /// The next rate, or `None` at [`MAX_RATE`] (upstream's `SUCCESS`).
    pub fn next_rate(self) -> Option<i32> {
        (self.rate < MAX_RATE).then(|| self.rate * 2)
    }

    /// `TiabItem.useOn`'s later use: double the rate and add half of the time
    /// already used up ([`EACH_USE_TICKS`] - `remaining`, halved).
    ///
    /// `None` at [`MAX_RATE`]; the caller has already charged
    /// [`Self::upgrade_cost`].
    pub fn upgrade(self) -> Option<Self> {
        let rate = self.next_rate()?;
        let used_up = EACH_USE_TICKS - self.remaining;
        Some(Self {
            rate,
            remaining: self.remaining + used_up / 2,
        })
    }
}

impl Default for Accelerator {
    fn default() -> Self {
        Self::new()
    }
}

/// `Commands.LEVEL_GAMEMASTERS`: an operator may read a furnace anywhere.
pub const DIAGNOSTIC_PERMISSION: u8 = 2;

/// The server's block-interaction reach, rounded up: 4.5 blocks standing, 5 in creative. A
/// readout within it is the ordinary interaction path — the same distance the player could
/// right-click the furnace from.
pub const INFO_REACH: f64 = 6.0;

/// Whether a source may read the diagnostic at `distance`: an operator anywhere, everyone else
/// only where they could open the block themselves. An unreadable distance is never close.
#[must_use]
pub fn info_permitted(permission: Option<u8>, distance: Option<f64>) -> bool {
    match permission {
        // No player behind the source: the console (and a command block) may read anywhere,
        // as the vanilla data commands do.
        None => true,
        Some(level) if level >= DIAGNOSTIC_PERMISSION => true,
        Some(_) => distance.is_some_and(|distance| distance.is_finite() && distance <= INFO_REACH),
    }
}

/// The block-entity families the accelerator can drive.
///
/// Upstream's `TimeAcceleratorEntity` runs the clicked block entity's own tick `rate` times a game
/// tick, whatever it is. The pin exposes no way to run a vanilla block entity's tick (`notes.md`
/// §5), so a family is accelerated by advancing the saved timers its own routine reads, and each
/// family the port can do that for is one entry here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    /// `minecraft:furnace`, `smoker`, `blast_furnace`: `cooking_time_spent`,
    /// `cooking_total_time` and `lit_time_remaining`.
    Furnace,
    /// `minecraft:campfire`, `soul_campfire`: the four `CookingTimes`/`CookingTotalTimes` slots.
    Campfire,
}

/// A campfire's fire state: its own tick cooks while it is lit and cools while it is not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fire {
    /// `lit=true`: `CampfireBlockEntity.cookTick`.
    Lit,
    /// `lit=false`: `CampfireBlockEntity.cooldownTick`.
    Unlit,
}

/// The family of the block entity at `block`, when the accelerator can drive it.
///
/// `None` is upstream's `FAIL` and the readout's "not this mod's state": the click is not claimed
/// (the block's own `use` runs) and no timer is touched.
#[must_use]
pub fn accelerated_family(block: &str) -> Option<Family> {
    match block {
        "minecraft:furnace" | "minecraft:smoker" | "minecraft:blast_furnace" => {
            Some(Family::Furnace)
        }
        "minecraft:campfire" | "minecraft:soul_campfire" => Some(Family::Campfire),
        _ => None,
    }
}

/// Whether `block` is in the furnace family: the timing keys and output slot the readout reports.
#[must_use]
pub fn furnace_block(block: &str) -> bool {
    accelerated_family(block) == Some(Family::Furnace)
}

/// What `extra` accelerated block ticks do to one campfire slot's `CookingTimes` entry.
///
/// A lit campfire's own tick raises every occupied slot by one and completes the cook at
/// `progress >= total`; the accelerated steps therefore raise the timer, held at the slot's
/// cooking time so the campfire's own tick still finishes the job. An unlit campfire runs
/// `cooldownTick`, which lowers the progress by two a tick and never below zero.
#[must_use]
pub fn campfire_progress(progress: i32, total: i32, extra: i32, fire: Fire) -> i32 {
    if extra <= 0 {
        return progress.max(0);
    }
    match fire {
        Fire::Lit => progress.max(0).saturating_add(extra).min(total.max(0)),
        // `cooldownTick` runs `progress - 2` clamped at zero, once a tick.
        Fire::Unlit => progress
            .saturating_sub(extra.saturating_mul(2))
            .max(0)
            .min(total.max(0)),
    }
}

/// `/tiab time add|remove <seconds>`: upstream clamps to
/// `MAX_STORED_TIME / TICKS_CONST` seconds, then multiplies by `TICKS_CONST`.
pub fn seconds_to_ticks(seconds: i32) -> i32 {
    if seconds <= 0 {
        return 0;
    }
    seconds.min(MAX_STORED_TIME / TICKS_CONST) * TICKS_CONST
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(hours: i32, minutes: i32, seconds: i32) -> String {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }

    #[test]
    fn accumulate_adds_the_ticks_and_the_total() {
        let after = StoredTime::EMPTY.accumulate(TICKS_CONST);
        assert_eq!(
            after,
            StoredTime {
                stored: 20,
                total: 20
            }
        );
        let again = after.accumulate(TICKS_CONST);
        assert_eq!(
            again,
            StoredTime {
                stored: 40,
                total: 40
            }
        );
    }

    #[test]
    fn accumulate_caps_only_the_stored_amount() {
        let nearly_full = StoredTime {
            stored: MAX_STORED_TIME - 10,
            total: 10,
        };
        let after = nearly_full.accumulate(TICKS_CONST);
        assert_eq!(
            after,
            StoredTime {
                stored: MAX_STORED_TIME,
                total: 30,
            }
        );
    }

    #[test]
    fn accumulate_ignores_a_step_that_would_go_negative() {
        let bottle = StoredTime {
            stored: 100,
            total: 200,
        };
        assert_eq!(bottle.accumulate(-1), bottle);
        assert_eq!(bottle.accumulate(i32::MAX), bottle);
    }

    #[test]
    fn spend_refuses_more_than_is_stored() {
        let short = StoredTime {
            stored: EACH_USE_TICKS - 1,
            total: 0,
        };
        assert_eq!(short.spend(EACH_USE_TICKS), None);
    }

    #[test]
    fn spend_leaves_exactly_the_remainder_of_the_total() {
        let full = StoredTime {
            stored: EACH_USE_TICKS,
            total: 5_000,
        };
        assert_eq!(
            full.spend(EACH_USE_TICKS),
            Some(StoredTime {
                stored: 0,
                total: 5_000
            })
        );
        let fat = StoredTime {
            stored: EACH_USE_TICKS + 100,
            total: 5_000,
        };
        assert_eq!(
            fat.spend(EACH_USE_TICKS),
            Some(StoredTime {
                stored: 100,
                total: 5_000
            })
        );
    }

    #[test]
    fn remove_takes_ticks_but_never_below_zero() {
        let bottle = StoredTime {
            stored: 100,
            total: 900,
        };
        assert_eq!(
            bottle.remove(40),
            StoredTime {
                stored: 60,
                total: 900
            }
        );
        assert_eq!(
            bottle.remove(1_000),
            StoredTime {
                stored: 0,
                total: 900
            }
        );
    }

    #[test]
    fn each_use_is_thirty_seconds_of_ticks() {
        assert_eq!(EACH_USE_TICKS, 600);
        assert_eq!(EACH_USE_TICKS, TICKS_CONST * EACH_USE_DURATION_SECONDS);
    }

    #[test]
    fn the_first_use_costs_one_use() {
        let fresh = Accelerator::new();
        assert_eq!(fresh.rate, 1);
        assert_eq!(fresh.remaining, EACH_USE_TICKS);
        assert_eq!(fresh.upgrade_cost(), EACH_USE_TICKS);
    }

    #[test]
    fn upgrade_cost_is_half_the_new_rate() {
        let at = |rate| Accelerator {
            rate,
            remaining: EACH_USE_TICKS,
        };
        assert_eq!(at(2).upgrade_cost(), 600);
        assert_eq!(at(4).upgrade_cost(), 1_200);
        assert_eq!(at(8).upgrade_cost(), 2_400);
        assert_eq!(at(64).upgrade_cost(), 19_200);
        assert_eq!(at(MAX_RATE).upgrade_cost(), 38_400);
    }

    #[test]
    fn the_rate_doubles_up_to_the_ceiling() {
        let at = |rate| Accelerator {
            rate,
            remaining: EACH_USE_TICKS,
        };
        assert_eq!(at(1).next_rate(), Some(2));
        assert_eq!(at(64).next_rate(), Some(MAX_RATE));
        assert_eq!(at(MAX_RATE).next_rate(), None);
        assert_eq!(MAX_RATE, 128);
    }

    #[test]
    fn an_upgrade_adds_half_the_used_up_time() {
        let running = Accelerator {
            rate: 1,
            remaining: 400,
        };
        // 200 ticks of the use are gone; half of them come back.
        assert_eq!(
            running.upgrade(),
            Some(Accelerator {
                rate: 2,
                remaining: 500
            })
        );
    }

    #[test]
    fn an_upgrade_at_the_ceiling_is_refused() {
        let fastest = Accelerator {
            rate: MAX_RATE,
            remaining: 300,
        };
        assert_eq!(fastest.upgrade(), None);
    }

    #[test]
    fn a_new_accelerator_runs_for_exactly_one_use() {
        let mut accelerator = Accelerator::new();
        let mut game_ticks = 0;
        let mut block_ticks = 0;
        loop {
            let (now, next) = accelerator.run_tick();
            game_ticks += 1;
            block_ticks += now;
            let Some(next) = next else { break };
            accelerator = next;
            assert!(game_ticks <= EACH_USE_TICKS, "still running after one use");
        }
        assert_eq!(game_ticks, EACH_USE_TICKS);
        assert_eq!(block_ticks, EACH_USE_TICKS);
    }

    #[test]
    fn a_game_tick_runs_the_rate_in_block_ticks() {
        let slow = Accelerator::new();
        assert_eq!(slow.run_tick().0, 1);
        let fast = Accelerator {
            rate: MAX_RATE,
            remaining: 1,
        };
        assert_eq!(fast.run_tick().0, MAX_RATE);
    }

    #[test]
    fn seconds_become_ticks_with_upstreams_clamp() {
        assert_eq!(seconds_to_ticks(0), 0);
        assert_eq!(seconds_to_ticks(1), TICKS_CONST);
        assert_eq!(seconds_to_ticks(30), EACH_USE_TICKS);
        assert_eq!(
            seconds_to_ticks(i32::MAX),
            MAX_STORED_TIME / TICKS_CONST * TICKS_CONST
        );
        assert_eq!(seconds_to_ticks(-5), 0);
    }

    #[test]
    fn clock_text_is_hours_minutes_seconds_of_stored_time() {
        assert_eq!(StoredTime::clock_text(0), clock(0, 0, 0));
        assert_eq!(StoredTime::clock_text(TICKS_CONST), clock(0, 0, 1));
        assert_eq!(StoredTime::clock_text(TICKS_CONST * 3_661), clock(1, 1, 1));
        assert_eq!(
            StoredTime::clock_text(MAX_STORED_TIME),
            clock(24 * 360, 0, 0)
        );
    }

    /// The review finding: `/tiab info` is a remote readout of a block's timers and of the
    /// output slot behind it. It is allowed where the player could open the furnace himself
    /// (within block reach) and, as an administrative read, anywhere for an operator; an
    /// unknown distance is never "close".
    #[test]
    fn a_diagnostic_read_needs_reach_or_permission() {
        assert!(
            !info_permitted(Some(0), Some(INFO_REACH + 1.0)),
            "far, no op"
        );
        assert!(info_permitted(Some(0), Some(INFO_REACH)), "at reach");
        assert!(info_permitted(None, Some(2.0)), "the console");
        assert!(
            info_permitted(Some(DIAGNOSTIC_PERMISSION), Some(500.0)),
            "op"
        );
        assert!(!info_permitted(Some(0), None), "unreadable is refused");
        assert!(!info_permitted(Some(0), Some(f64::NAN)), "not a distance");
    }

    /// Only the furnace family has the `cooking_time_spent`/`lit_time_remaining` payload and
    /// the output slot the readout reports; another container's slot 2 is not this mod's.
    #[test]
    fn only_the_furnace_family_is_read() {
        for block in [
            "minecraft:furnace",
            "minecraft:smoker",
            "minecraft:blast_furnace",
        ] {
            assert!(furnace_block(block), "{block}");
        }
        for block in [
            "minecraft:chest",
            "minecraft:hopper",
            "tiab:time_in_a_bottle",
        ] {
            assert!(!furnace_block(block), "{block}");
        }
    }

    /// Upstream accelerates any block entity, and a campfire is the second family whose saved
    /// timers this port can advance: its four cooking slots. A block with neither timers nor a
    /// known family is left to its own `use` (upstream's `FAIL`).
    #[test]
    fn the_campfire_family_is_the_second_accelerated_target() {
        assert_eq!(
            accelerated_family("minecraft:campfire"),
            Some(Family::Campfire)
        );
        assert_eq!(
            accelerated_family("minecraft:soul_campfire"),
            Some(Family::Campfire)
        );
        assert_eq!(
            accelerated_family("minecraft:furnace"),
            Some(Family::Furnace)
        );
        for block in [
            "minecraft:chest",
            "minecraft:hopper",
            "minecraft:grass_block",
        ] {
            assert_eq!(accelerated_family(block), None, "{block}");
        }
        assert!(!furnace_block("minecraft:campfire"));
    }

    /// A lit campfire cooks each occupied slot a tick at a time; `rate` extra block ticks raise
    /// the timer by `rate`, which is what makes the potato finish in half the game ticks.
    #[test]
    fn a_lit_campfire_advances_by_the_extra_ticks() {
        assert_eq!(campfire_progress(10, 600, 1, Fire::Lit), 11);
        assert_eq!(campfire_progress(10, 600, 63, Fire::Lit), 73);
        assert_eq!(campfire_progress(0, 600, 128, Fire::Lit), 128);
    }

    /// The timer stops at the slot's cooking time: the campfire's own tick completes the cook on
    /// `progress >= total`, so writing past it would only lose ticks (the furnace family's shape).
    #[test]
    fn a_campfire_timer_stops_at_its_cooking_time() {
        assert_eq!(campfire_progress(599, 600, 1, Fire::Lit), 600);
        assert_eq!(campfire_progress(590, 600, 63, Fire::Lit), 600);
        assert_eq!(campfire_progress(0, 600, i32::MAX, Fire::Lit), 600);
    }

    /// `cooldownTick`: an unlit campfire lowers every progress by two a tick, never below zero.
    /// The accelerated steps do the same, `rate` times.
    #[test]
    fn an_unlit_campfire_cools_two_ticks_a_tick() {
        assert_eq!(campfire_progress(100, 600, 1, Fire::Unlit), 98);
        assert_eq!(campfire_progress(100, 600, 63, Fire::Unlit), 0);
        assert_eq!(campfire_progress(3, 600, 2, Fire::Unlit), 0);
    }

    /// An empty slot holds no timer, and no accelerated step may invent one.
    #[test]
    fn an_empty_campfire_slot_stays_empty() {
        assert_eq!(campfire_progress(0, 0, 1, Fire::Lit), 0);
        assert_eq!(campfire_progress(0, 0, 128, Fire::Lit), 0);
        assert_eq!(campfire_progress(0, 0, 63, Fire::Unlit), 0);
        assert_eq!(campfire_progress(42, 600, 0, Fire::Lit), 42);
    }
}
