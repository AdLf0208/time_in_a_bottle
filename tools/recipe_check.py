#!/usr/bin/env python3
"""Craft the Time in a Bottle recipe in a real crafting table.

Called by tools/e2e.sh with the open game's control socket. Uses the harness's
crafting grid (`click-slot`: 0-8 the hotbar, 100-108 the table's grid row-major,
200 the result), gives the ingredients in a known order, and asserts:

1. the jar's shaped recipe `GGG / DCD / LBL` (gold ingot, diamond, clock, lapis
   lazuli, glass bottle) crafts exactly one `tiab:time_in_a_bottle`: taking the
   result puts exactly one bottle in the inventory and every ingredient is gone;
2. control: the same grid without the clock crafts nothing (the recipe needs
   every cell);
3. control: the clock alone in its cell crafts nothing.

`ctl screen` is avoided on purpose: at this pin it can hang the whole leg (the
`e2e-guard` shim exists because of it). The hotbar slot of each ingredient comes
from `ctl inventory`, which answers. The table is replaced before every case, so
a failed/untaken result cannot leak into the next grid.
"""
import json
import subprocess
import sys

BOTTLE = "tiab:time_in_a_bottle"
GOLD = "minecraft:gold_ingot"
DIAMOND = "minecraft:diamond"
CLOCK = "minecraft:clock"
LAPIS = "minecraft:lapis_lazuli"
GLASS_BOTTLE = "minecraft:glass_bottle"

# The recipe's cells, row-major from slot 100.
RECIPE = {
    GOLD: [100, 101, 102],
    DIAMOND: [103, 105],
    CLOCK: [104],
    LAPIS: [106, 108],
    GLASS_BOTTLE: [107],
}

# What each case gives the player, in this order: an empty inventory takes them
# into hotbar slots 0, 1, 2, ...
INGREDIENTS = [
    (GOLD, 3),
    (DIAMOND, 2),
    (CLOCK, 1),
    (LAPIS, 2),
    (GLASS_BOTTLE, 1),
]
GRID = 100
RESULT = 200
# Where the taken result is put: the first main-inventory slot of the menu.
INVENTORY_FIRST = 9


def main():
    sock, ctl, x, y, z = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5]

    def run(*args):
        out = subprocess.run([ctl, "--socket", sock, *args], text=True, capture_output=True)
        try:
            reply = json.loads(out.stdout)
        except json.JSONDecodeError:
            raise SystemExit(f"ctl {' '.join(args)}: {out.stdout}{out.stderr}")
        # `click-slot` and friends answer an error object instead of failing the
        # process; a silent no-op must not pass as a craft.
        if isinstance(reply, dict) and "error" in reply:
            raise SystemExit(f"ctl {' '.join(args)}: {reply['error']}")
        return reply

    def wait(ticks=4):
        run("wait", str(ticks))

    def hotbar_slot(item):
        for slot in run("inventory")["hotbar"]:
            if slot.get("item") == item:
                return slot["slot"]
        raise SystemExit(f"missing ingredient {item} in the hotbar")

    def counts():
        return run("inventory")["counts"]

    def place(item, targets):
        """Hold `item`'s hotbar stack, right-click one into each target cell, put
        the rest back."""
        slot = str(hotbar_slot(item))
        run("click-slot", slot)
        wait(1)
        for target in targets:
            run("click-slot", str(target), "right")
            wait(1)
        run("click-slot", slot)
        wait(2)

    def setup(items):
        try:
            run("close")
        except SystemExit:
            pass
        wait(5)
        # Replace the table so its block entity, and therefore the crafting
        # grid, is fresh for this case.
        run("setblock", str(x), str(y), str(z), "minecraft:air")
        run("setblock", str(x), str(y), str(z), "minecraft:crafting_table")
        wait(2)
        # The caller's teleport can be swallowed while a screen is still
        # closing; a server `/tp` moves the player even then.
        run("command", f"tp @s {float(x) - 0.5} {y} {float(z) + 0.5}")
        wait(3)
        run("command", "clear @s")
        for item, count in items:
            run("give", item, str(count))
        wait(3)
        run("interact", str(x), str(y), str(z))
        wait(5)

    def craft(cells):
        """Place `cells` (item -> grid slots) and take whatever the result holds."""
        for item, targets in cells.items():
            place(item, targets)
        before = counts().get(BOTTLE, 0)
        run("click-slot", str(RESULT))
        wait(4)
        run("click-slot", str(INVENTORY_FIRST))
        wait(3)
        after = counts().get(BOTTLE, 0)
        return before, after

    failures = []

    # 1. The carried recipe itself.
    setup(INGREDIENTS)
    before, after = craft(RECIPE)
    left = counts()
    remaining = sum(left.get(item, 0) for item, _ in INGREDIENTS)
    print(f"inventory: bottle {before} -> {after}, {remaining} ingredients left")
    if after != before + 1:
        failures.append(f"the bottle recipe put {after - before} bottles in the inventory, expected 1")
    if remaining:
        failures.append(f"the recipe did not consume its ingredients ({remaining} left)")

    # 2. Control: the clock's cell empty, every other cell in place.
    setup([(item, count) for item, count in INGREDIENTS if item != CLOCK])
    without_clock = {item: targets for item, targets in RECIPE.items() if item != CLOCK}
    before, after = craft(without_clock)
    print(f"the same grid without the clock: bottle {before} -> {after}")
    if after != before:
        failures.append(f"a grid missing the clock crafted {after - before} bottles")

    # 3. Control: the clock alone.
    setup([(CLOCK, 1)])
    before, after = craft({CLOCK: [GRID + 4]})
    print(f"the clock alone: bottle {before} -> {after}")
    if after != before:
        failures.append(f"the clock alone crafted {after - before} bottles")

    run("close")
    wait(3)
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        raise SystemExit(1)
    print(
        "recipe PASS: the jar's shaped recipe crafts exactly 1 tiab:time_in_a_bottle and consumes "
        "every ingredient; a grid without the clock and the clock alone both stay empty"
    )


if __name__ == "__main__":
    main()
