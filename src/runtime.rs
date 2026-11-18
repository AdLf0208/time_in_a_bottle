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
}
