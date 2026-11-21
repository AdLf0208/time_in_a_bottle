#!/usr/bin/env bash
# In-game check for Time in a Bottle: the bottle stores ticks and spends them
# accelerating a block entity by exactly the ticks spent.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200. Three furnaces stand on the stone floor with 64
# iron ore and 64 coal each, so all cook for the whole run:
#   A at z=0, the survival target;
#   B at z=4, the untouched furnace control;
#   C at z=8, the creative target.
# Two lit campfires with one potato each boil at z=10 (the accelerated one) and
# z=12 (the untouched control): the second accelerated family.
#
# `/tiab info <x> <y> <z>` prints one atomic line: the game clock, the first
# bottle's stored/total account and the target's family, timers, items and (for
# a furnace) fuel. One line with the clock makes every assertion exact — the
# bottle gains 20 stored ticks at the end of every second (`TiabMod`), so
#   spent = 20 * (floor(t1/20) - floor(t0/20)) - (stored1 - stored0)
# and a furnace's "total cooked" is `out * 200 + cook`, so against the control
# furnace its extra block ticks are exact to the tick. A campfire's `cook` is
# the first occupied slot's `CookingTimes` entry, which rises once a game tick
# and once more per accelerated tick, so against the control campfire the extra
# block ticks are exact to the tick there too.
#
# Default run:
#  A  the bottle registers; an unknown tiab id is refused; the three furnaces cook;
#  B  carrying the bottle stores exactly one tick a game tick (20 a second);
#  C  creative mode pays nothing (upstream's `canUse`): an unfilled bottle runs
#     its 600-tick budget (a completed cook can lose one accelerated tick,
#     notes.md §5), and the account does not change;
#  D  in survival an unfilled bottle pays for nothing (no account change, no
#     acceleration);
#  E  a bottle with 600+ stored ticks pays exactly 600 for one use, which runs
#     its 600-tick budget; a second click at the remainder does not upgrade;
#  F  two uses from 1,200 stored ticks cost exactly 1,200 and run at rate 2; a
#     third click cannot reach rate 4;
#  G  a campfire is a target too: one paid use makes its potato's timer run at
#     two game ticks a tick, the control campfire runs at one, the accelerated
#     potato finishes and drops as a baked potato while the control is still raw;
#  H  the carried shaped recipe crafts exactly one bottle in a real crafting
#     table and consumes every ingredient (controls: no clock, clock alone).
#
# The mod is one SDK package (`tiab.hmod`: the item, its assets/data and the
# code), so the game must be a launcher-built one that carries it — there is no
# `--dev-mod content` any more, and the old EMBEDDED/CONTROL legs (a separate
# content mod and a bundle-embedded copy) retired with the `.hnmod` format.
# Point `GAME=` at the launcher-built `horizon-client` of an instance with
# `tiab.hmod` installed; see notes.md, "Migration to the SDK format".
#
# SELFTEST=1 flips G's exhaustion assertion by one tick (it must FAIL), proving
# the new family's leg can fail; SELFTEST=2 flips E's 600 to 601.
set -Eeuo pipefail
export LC_ALL=C
[ -z "${TIAB_DEBUG:-}" ] || set -x

CLONE=${CLONE:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/tiab
# The control client and the game must be the same build: point both at the
# instance the launcher built (its game carries `tiab.hmod`, its ctl speaks the
# same protocol). Neither the SDK layout nor the retired `.hnmod` leg leaves one
# under the port, so the defaults are the shared pair of the *previous* pin only
# as a fallback for the ctl; the game has no default.
PIN_BIN=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$PIN_BIN/horizon-ctl}
GAME=${GAME:-}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/tiab-e2e.sock}
SELFTEST=${SELFTEST:-0}
KEEP=${KEEP:-0}
WORLD=TiabE2E
# Where the arena stands: the End's void, far from the central island.
X0=200
AX=$((X0 + 2))
BX=$((X0 + 2))
BZ=4
CX=$((X0 + 2))
CZ=8
# The two campfires: the accelerated one and its control.
GCZ=10
HCZ=12
ORE=64
COAL=64
USE_TICKS=600

command -v jq >/dev/null || { echo "missing jq (every assertion parses a ctl reply)" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "set GAME= to the launcher-built horizon-client of an instance with tiab.hmod installed" >&2
    echo "EXIT:2"
    exit 2
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
# The control client has no per-port build in the SDK layout: use the shared one
# (`ports/tools/shared-bin.sh env`), or pass CTL= from the same build as GAME.
"$CTL" --help >/dev/null 2>&1 || { echo "missing horizon-ctl: $CTL (pass CTL=)" >&2; exit 1; }

mkdir -p /tmp/opencode
SCRATCH=$(mktemp -d /tmp/opencode/tiab-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 60 >&2 || true; exit 1; }
on_error() { echo "FAIL: line $1: '$2' exited $?" >&2; ctl logs --lines 40 >&2 || true; }
trap 'on_error "$LINENO" "$BASH_COMMAND"' ERR

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    cd "$SCRATCH"
    # `--offscreen`, not `--headless`: `click-slot` (the recipe phase) answers
    # "no renderer yet" on a headless game, because it clicks the drawn screen.
    # An offscreen window draws without showing one.
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --offscreen
                --wait --resources "$RES")
    "$CTL" --socket "$SOCK" launch "${args[@]}" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

block_name() { ctl block "$1" "$2" "$3" | jq -r '.block // empty'; }
block_prop() { # block_prop X Y Z prop
    ctl block "$1" "$2" "$3" | jq -r '.state' | grep -o "$4=[a-z0-9_]*" | cut -d= -f2 || true
}

# The End's void at X0: a stone floor and clear air above it.
arena() {
    ctl command "execute in minecraft:the_end run tp @s $X0.5 104 0.5" >/dev/null
    local loaded=false
    for _ in $(seq 1 120); do
        ctl wait 5 >/dev/null
        if [ "$(ctl block "$X0" 100 0 | jq -r '.loaded // false')" = true ]; then
            loaded=true
            break
        fi
    done
    [ "$loaded" = true ] || fail "the End arena chunk never loaded"
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 100 -14 $((X0 + 4)) 100 14 minecraft:stone" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run fill $((X0 - 2)) 101 -14 $((X0 + 4)) 108 14 minecraft:air" >/dev/null
    ctl wait 6 >/dev/null
    ctl command "execute in minecraft:the_end run kill @e[type=!player]" >/dev/null
    ctl command "execute in minecraft:the_end run tp @s $X0.5 101 0.5" >/dev/null
    ctl wait 20
    [ "$(block_name "$X0" 100 0)" = "minecraft:stone" ] || fail "the arena floor is not stone"
    [ "$(block_name "$X0" 101 0)" = "minecraft:air" ] || fail "the arena air above the floor is not air"
}

# `/tiab info X Y Z`: one line with the clock, the bottle's account and the
# target. Fills "<P>_T", "<P>_STORED", "<P>_TOTAL", "<P>_KIND", "<P>_COOK",
# "<P>_CT" (the cooking time), "<P>_OUT", "<P>_LIT", "<P>_ACC".
LAST_T=-1
INFO_SEQ=0
info() { # info X Y Z PREFIX
    local x=$1 y=$2 z=$3 prefix=$4 line t tries=0
    INFO_SEQ=$((INFO_SEQ + 1))
    while :; do
        # Every request carries its own sequence number, so a stale reply can
        # never be mistaken for this one; resend while it is missing.
        say "tiab info $x $y $z $INFO_SEQ"
        ctl wait 3 >/dev/null
        line=$(ctl messages 20 | jq -r '.chat[]' | grep "^tiab $x,$y,$z n=$INFO_SEQ " | tail -1 || true)
        if [ -z "$line" ]; then
            line=$(ctl logs --lines 200 2>/dev/null \
                | sed -n 's/.*\[CHAT\] \(tiab [0-9]*,[0-9]*,[0-9]* n=[0-9]* .*acc=[a-z0-9x]*\).*/\1/p' \
                | grep "^tiab $x,$y,$z n=$INFO_SEQ " | tail -1 || true)
        fi
        if [ -n "$line" ]; then
            t=$(sed -n 's/.* t=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
            if [ -n "$t" ] && [ "$t" -gt "$LAST_T" ]; then break; fi
        fi
        tries=$((tries + 1))
        [ "$tries" -lt 10 ] || fail "no /tiab info reply for $x,$y,$z (sequence $INFO_SEQ, last: ${line:-none})"
        ctl wait 3 >/dev/null
    done
    LAST_T=$t
    local stored total kind cook ct out lit acc
    stored=$(sed -n 's/.* stored=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    total=$(sed -n 's/.* total=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    kind=$(sed -n 's/.* kind=\([a-z]*\) .*/\1/p' <<<"$line")
    cook=$(sed -n 's/.* cook=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    ct=$(sed -n 's/.* ct=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    out=$(sed -n 's/.* out=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    lit=$(sed -n 's/.* lit=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    acc=$(sed -n 's/.* acc=\([a-z0-9x]*\).*/\1/p' <<<"$line")
    [ -n "$stored" ] && [ -n "$total" ] && [ -n "$kind" ] && [ -n "$cook" ] && [ -n "$ct" ] \
        && [ -n "$out" ] && [ -n "$lit" ] && [ -n "$acc" ] \
        || fail "the info line did not parse: $line"
    printf -v "${prefix}_T" '%s' "$t"
    printf -v "${prefix}_STORED" '%s' "$stored"
    printf -v "${prefix}_TOTAL" '%s' "$total"
    printf -v "${prefix}_KIND" '%s' "$kind"
    printf -v "${prefix}_COOK" '%s' "$cook"
    printf -v "${prefix}_CT" '%s' "$ct"
    printf -v "${prefix}_OUT" '%s' "$out"
    printf -v "${prefix}_LIT" '%s' "$lit"
    printf -v "${prefix}_LINE" '%s' "$line"
    # `acc=RESTxREMAINING`, or `none`: the accelerator at this position.
    if [ "$acc" = none ]; then
        printf -v "${prefix}_RATE" '%s' -1
        printf -v "${prefix}_REM" '%s' 0
    else
        printf -v "${prefix}_RATE" '%s' "${acc%x*}"
        printf -v "${prefix}_REM" '%s' "${acc#*x}"
    fi
    echo "  $line" >&2
}

# The stored ticks the bottle paid between two reads: the account gains 20 at
# the end of every second and loses only what a use costs.
spent_between() { # spent_between S0 T0 S1 T1
    # A command is handled before the tick's level pass, so a read at t sees the
    # state before that tick's end-of-second step: count the steps in [t0, t1).
    echo $(( 20 * ((($4 - 1) / 20) - (($2 - 1) / 20)) - ($3 - $1) ))
}

# A furnace's total cooked ticks, from an info snapshot.
cooked() { # cooked PREFIX
    local prefix=$1
    echo $(( ${prefix}_OUT * 200 + ${prefix}_COOK ))
}

# The extra block ticks a furnace got beyond the game clock, against B:
# extra PREFIX0 PREFIX1 B0 B1
extra_ticks() {
    local a0=$1 a1=$2 b0=$3 b1=$4
    echo $(( ($(cooked "$a1") - $(cooked "$a0") - (${a1}_T - ${a0}_T))
           - ($(cooked "$b1") - $(cooked "$b0") - (${b1}_T - ${b0}_T)) ))
}

place_furnace() { # place_furnace X Y Z
    ctl command "execute in minecraft:the_end run setblock $1 $2 $3 minecraft:furnace" >/dev/null
    ctl wait 4 >/dev/null
    [ "$(block_name "$1" "$2" "$3")" = "minecraft:furnace" ] || fail "no furnace at $1,$2,$3"
    ctl container "$1" "$2" "$3" minecraft:iron_ore "$ORE" >/dev/null
    ctl wait 2 >/dev/null
    ctl container "$1" "$2" "$3" minecraft:coal "$COAL" >/dev/null
    ctl wait 4 >/dev/null
}

# Put ITEM in the hotbar and select it: `select` only looks at the hotbar, and a
# `give` into a cleared inventory lands there. (The inventory screen is not used:
# `ctl screen` hangs at this pin, `ctl menu` lists an incomplete inventory.)
hold() { # hold ITEM
    ctl select "$1" >/dev/null 2>&1 || fail "$1 is not in the hotbar; give it after clear @s"
}

# Hold the bottle and right-click the furnace's west face (the player teleports
# beside it). `ctl use-item` is a player right-click, so the native
# `right_click_block` hook hears it.
use_on() { # use_on X Y Z [FACE]
    ctl command "execute in minecraft:the_end run tp @s $(($1 - 1)).5 $2 $(($3)).5" >/dev/null
    ctl wait 2 >/dev/null
    hold tiab:time_in_a_bottle
    ctl wait 2 >/dev/null
    ctl look-at-block "$1" "$2" "$3" "${4:-west}" >/dev/null
    ctl wait 2 >/dev/null
    ctl use-item >/dev/null
    ctl wait 2 >/dev/null
}

# Put one ITEM on the block at X Y Z with a player right-click (a campfire's own
# `use` places food). The aim is the block's top face: a campfire is 7/16 high, so
# a standing player's eye is above its side faces and `look-at-block <x y z> west`
# answers "no face of block … is visible from here".
put_food() { # put_food X Y Z ITEM
    ctl command "execute in minecraft:the_end run tp @s $(($1 - 1)).5 $2 $(($3)).5" >/dev/null
    ctl wait 2 >/dev/null
    hold "$4"
    ctl wait 2 >/dev/null
    ctl look-at-block "$1" "$2" "$3" up >/dev/null
    ctl wait 2 >/dev/null
    ctl use-item >/dev/null
    ctl wait 3 >/dev/null
}

echo "== launch (The End arena, the package carries the item) =="
launch
arena

echo "== A: the bottle registers, and three furnaces cook =="
ctl give tiab:time_in_a_bottle 1 >/dev/null
ctl wait 4 >/dev/null
ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "tiab:time_in_a_bottle")] | length >= 1' >/dev/null \
    || fail "the bottle item did not register: $(ctl inventory | jq -c '{hotbar,main}')"
if ctl give tiab:not_a_real_item 1 >/dev/null 2>&1; then
    fail "an unknown tiab id was accepted"
fi
place_furnace "$AX" 101 0
place_furnace "$BX" 101 "$BZ"
place_furnace "$CX" 101 "$CZ"
ctl wait 20 >/dev/null
info "$AX" 101 0 a
info "$BX" 101 "$BZ" b
info "$CX" 101 "$CZ" c
[ "$a_COOK" -ge 0 ] && [ "$b_COOK" -ge 0 ] && [ "$c_COOK" -ge 0 ] || fail "a furnace is missing"
[ "$a_KIND" = furnace ] || fail "the readout calls the furnace '$a_KIND'"
[ "$a_CT" = 200 ] || fail "the furnace's cooking time is $a_CT, expected 200"
[ "$(block_prop "$AX" 101 0 lit)" = "true" ] || fail "furnace A is not lit"
echo "  bottle in the inventory (an unknown id is refused); A cook $a_COOK, B cook $b_COOK, C cook $c_COOK"

echo "== B: carrying the bottle stores exactly one tick a game tick =="
say "tiab time remove @s 9999"
info "$AX" 101 0 b0
[ "$b0_STORED" = 0 ] || fail "the bottle still holds $b0_STORED stored ticks after the remove"
ctl wait 200 >/dev/null
info "$AX" 101 0 b1
GAIN=$((b1_STORED - b0_STORED))
EXPECTED=$(( 20 * (((b1_T - 1) / 20) - ((b0_T - 1) / 20)) ))
[ "$GAIN" -eq "$EXPECTED" ] \
    || fail "the bottle gained $GAIN ticks over $((b1_T - b0_T)) game ticks, expected $EXPECTED (20 a second)"
# `total` is every tick ever stored and the remove only empties `stored`
# (upstream's `StoredTimeComponent`), so total only grows.
[ "$b1_TOTAL" -ge "$b1_STORED" ] || fail "total ($b1_TOTAL) is below stored ($b1_STORED)"
echo "  $((b1_T - b0_T)) game ticks: stored $b0_STORED -> $b1_STORED (exactly $EXPECTED)"

echo "== C: creative mode pays nothing (upstream's canUse) =="
info "$CX" 101 "$CZ" c0
info "$BX" 101 "$BZ" cb0
use_on "$CX" 101 "$CZ"
info "$CX" 101 "$CZ" c1
SPENT=$(spent_between "$c0_STORED" "$c0_T" "$c1_STORED" "$c1_T")
[ "$SPENT" -eq 0 ] || fail "a creative use cost $SPENT stored ticks, expected 0"
ctl wait 1200 >/dev/null
info "$CX" 101 "$CZ" c2
info "$BX" 101 "$BZ" cb2
CREATIVE_EXTRA=$(extra_ticks c1 c2 cb0 cb2)
# `c1` read the accelerator's remaining budget in the same line: the furnace
# must advance by that many block ticks over the rest of its life. The
# fast-forward writes the cook timer at most at its cooking time, so a furnace
# tick that starts already there completes the cook but its own increment is
# lost — up to one accelerated tick per completed cook (`notes.md` §5; measured
# 1 in 9 cooks here, and 2 in ~100 probes with no campfire in the world).
[ "$c1_RATE" = 1 ] || fail "a creative use started at rate $c1_RATE, expected 1"
[ "$c1_REM" -gt 0 ] || fail "no accelerator after the creative use ($c1_LINE)"
C_COOKS=$((c2_OUT - c1_OUT))
C_LOSS=$((c1_RATE * C_COOKS))
[ "$CREATIVE_EXTRA" -le "$c1_REM" ] \
    || fail "a creative use ran $CREATIVE_EXTRA block ticks, more than its remaining $c1_REM"
[ "$CREATIVE_EXTRA" -ge $((c1_REM - C_LOSS)) ] \
    || fail "a creative use ran $CREATIVE_EXTRA block ticks, expected $c1_REM with at most $C_LOSS lost over $C_COOKS cooks"
echo "  creative use: spent 0, ran $CREATIVE_EXTRA of its $c1_REM remaining block ticks (at most $C_LOSS can be lost over $C_COOKS cooks)"

ctl command "gamemode survival" >/dev/null
ctl wait 10 >/dev/null

echo "== D: an unfilled bottle pays for nothing in survival =="
say "tiab time remove @s 9999"
info "$AX" 101 0 d0
info "$BX" 101 "$BZ" db0
use_on "$AX" 101 0
[ "$(ctl state | jq -r '.screen')" = "playing" ] || fail "the furnace menu opened on a refused use"
info "$AX" 101 0 d1
SPENT=$(spent_between "$d0_STORED" "$d0_T" "$d1_STORED" "$d1_T")
[ "$SPENT" -eq 0 ] || fail "an unfilled bottle paid $SPENT stored ticks"
ctl wait 300 >/dev/null
info "$AX" 101 0 d2
info "$BX" 101 "$BZ" db2
REFUSED_EXTRA=$(extra_ticks d1 d2 db0 db2)
[ "$REFUSED_EXTRA" -eq 0 ] || fail "an unfilled bottle ran $REFUSED_EXTRA block ticks"
echo "  survival, nothing stored: spent 0, ran $REFUSED_EXTRA block ticks, no menu"

echo "== E: a paid use costs exactly 600 and runs exactly 600 block ticks =="
# 580 carried ticks are not enough; the account is past 600 after this window.
info "$AX" 101 0 e0
[ "$e0_STORED" -lt "$USE_TICKS" ] || fail "the refusal check needs under 600 stored, has $e0_STORED"
ctl wait 620 >/dev/null
info "$AX" 101 0 e1
info "$BX" 101 "$BZ" eb1
[ "$e1_STORED" -ge "$USE_TICKS" ] || fail "620 carried ticks left only $e1_STORED stored"
use_on "$AX" 101 0
# A second click cannot pay the 600-tick upgrade with what is left.
use_on "$AX" 101 0
info "$AX" 101 0 e2
SPENT=$(spent_between "$e1_STORED" "$e1_T" "$e2_STORED" "$e2_T")
[ "$SPENT" -eq "$USE_TICKS" ] || fail "one paid use cost $SPENT stored ticks, expected $USE_TICKS"
ctl wait 5 >/dev/null
[ "$(block_prop "$AX" 101 0 lit)" = "true" ] || fail "the accelerated furnace's lit state is false"
ctl wait 1200 >/dev/null
info "$AX" 101 0 e3
info "$BX" 101 "$BZ" eb3
SPENT_EXTRA=$(extra_ticks e2 e3 eb1 eb3)
[ "$e2_RATE" = 1 ] || fail "the paid use started at rate $e2_RATE, expected 1"
E_COOKS=$((e3_OUT - e2_OUT))
E_LOSS=$((e2_RATE * E_COOKS))
if [ "$SELFTEST" = 2 ]; then
    echo "  (selftest: asserting the use ran one block tick too many)"
    [ "$SPENT_EXTRA" -eq $((e2_REM + 1)) ] || fail "the use ran $SPENT_EXTRA block ticks, expected $((e2_REM + 1))"
else
    [ "$SPENT_EXTRA" -le "$e2_REM" ] \
        || fail "the use ran $SPENT_EXTRA block ticks, more than its remaining $e2_REM"
    [ "$SPENT_EXTRA" -ge $((e2_REM - E_LOSS)) ] \
        || fail "the use ran $SPENT_EXTRA block ticks, expected $e2_REM with at most $E_LOSS lost over $E_COOKS cooks"
fi
echo "  one use: cost exactly $SPENT stored ticks, ran $SPENT_EXTRA of its $e2_REM remaining block ticks (at most $E_LOSS can be lost over $E_COOKS cooks)"

echo "== F: 1,200 stored ticks buy two uses; a third cannot reach rate 4 =="
say "tiab time remove @s 9999"
info "$AX" 101 0 f0
say "tiab time add @s 60"
info "$AX" 101 0 f1
info "$BX" 101 "$BZ" fb1
use_on "$AX" 101 0
use_on "$AX" 101 0
use_on "$AX" 101 0
info "$AX" 101 0 f2
SPENT=$(spent_between "$f1_STORED" "$f1_T" "$f2_STORED" "$f2_T")
[ "$SPENT" -eq 1200 ] || fail "two uses cost $SPENT stored ticks, expected 1200 (the third is refused)"
ctl wait 1200 >/dev/null
info "$AX" 101 0 f3
info "$BX" 101 "$BZ" fb3
RATE_EXTRA=$(extra_ticks f2 f3 fb1 fb3)
[ "$f2_RATE" = 2 ] || fail "two uses left rate $f2_RATE, expected 2"
# At rate R the furnace gets R block ticks a game tick from the accelerator, so
# beyond its own tick it gains R x its remaining game ticks. The fast-forward
# writes the cook timer at most at its cooking time, so the step that completes
# a cook loses its own increment (and up to R-1 of the write on the step that
# lands exactly on the cap): at most R ticks per completed cook (`notes.md` §5).
EXPECTED_EXTRA=$((f2_RATE * f2_REM))
COOKS=$((f3_OUT - f2_OUT))
MAX_LOSS=$((f2_RATE * COOKS))
echo "  two uses: cost $SPENT, remaining ${f2_RATE}x${f2_REM}, ran $RATE_EXTRA block ticks (at most $MAX_LOSS of $EXPECTED_EXTRA can be lost finishing a cook)"
[ "$RATE_EXTRA" -le "$EXPECTED_EXTRA" ] \
    || fail "a rate-2 use ran $RATE_EXTRA extra block ticks, more than ${f2_RATE}x$f2_REM"
[ "$RATE_EXTRA" -ge $((EXPECTED_EXTRA - MAX_LOSS)) ] \
    || fail "a rate-2 use ran $RATE_EXTRA extra block ticks, expected ${f2_RATE}x$f2_REM with at most $MAX_LOSS dropped over $COOKS cooks"

echo "== G: a campfire is a target too, at exactly the accelerator's rate =="
# A fresh inventory: one bottle (the account) and two potatoes.
ctl command "clear @s" >/dev/null
ctl give tiab:time_in_a_bottle 1 >/dev/null
ctl give minecraft:potato 2 >/dev/null
ctl wait 4 >/dev/null
for z in "$GCZ" "$HCZ"; do
    ctl command "execute in minecraft:the_end run setblock $AX 101 $z minecraft:campfire[lit=true]" >/dev/null
done
ctl wait 4 >/dev/null
[ "$(block_name "$AX" 101 "$GCZ")" = "minecraft:campfire" ] || fail "no campfire at $AX,101,$GCZ"
[ "$(block_prop "$AX" 101 "$GCZ" lit)" = "true" ] || fail "the accelerated campfire is not lit"
[ "$(block_name "$BX" 101 "$HCZ")" = "minecraft:campfire" ] || fail "no campfire at $BX,101,$HCZ"
put_food "$AX" 101 "$GCZ" minecraft:potato
put_food "$BX" 101 "$HCZ" minecraft:potato
info "$AX" 101 "$GCZ" g0
info "$BX" 101 "$HCZ" gb0
[ "$g0_KIND" = campfire ] || fail "the readout calls the campfire '$g0_KIND'"
[ "$g0_OUT" = 1 ] || fail "the accelerated campfire holds $g0_OUT potatoes, expected 1"
[ "$g0_CT" = 600 ] || fail "the campfire's cooking time is $g0_CT, expected 600"
[ "$g0_LIT" = 1 ] || fail "the readout says the campfire is unlit (lit=$g0_LIT)"
[ "$gb0_OUT" = 1 ] || fail "the control campfire holds $gb0_OUT potatoes, expected 1"
say "tiab time add @s 60"
ctl wait 3 >/dev/null
ctl messages 10 | jq -e '.chat | any(test("Added 60 seconds"))' >/dev/null \
    || fail "/tiab time add did not answer 'Added 60 seconds': $(ctl messages 10 | jq -c '.chat')"
# The campfire is 7/16 high: its top face is the visible one (see `put_food`).
use_on "$AX" 101 "$GCZ" up
info "$AX" 101 "$GCZ" g1
[ "$g1_RATE" = 1 ] || fail "the campfire use started at rate $g1_RATE, expected 1"
[ "$g1_REM" -gt 0 ] || fail "no accelerator on the campfire after the use ($g1_LINE)"
ctl wait 100 >/dev/null
info "$AX" 101 "$GCZ" g2
info "$BX" 101 "$HCZ" gb2
ELAPSED=$((g2_T - g1_T))
DELIVERED=$((g1_REM - g2_REM))
ACC_ADVANCE=$((g2_COOK - g1_COOK))
CTL_ADVANCE=$((gb2_COOK - gb0_COOK))
echo "  over $ELAPSED game ticks the campfire's timer advanced $ACC_ADVANCE ($DELIVERED of them accelerated); the control's advanced $CTL_ADVANCE"
[ "$DELIVERED" -gt 0 ] || fail "the accelerator delivered no campfire ticks"
if [ "$SELFTEST" = 1 ]; then
    echo "  (selftest: asserting the campfire took one tick too many)"
    [ "$ACC_ADVANCE" -eq $((ELAPSED + DELIVERED + 1)) ] \
        || fail "the campfire's timer advanced $ACC_ADVANCE, expected $((ELAPSED + DELIVERED + 1))"
else
    [ "$ACC_ADVANCE" -eq $((ELAPSED + DELIVERED)) ] \
        || fail "the campfire's timer advanced $ACC_ADVANCE, expected the $ELAPSED game ticks plus the $DELIVERED accelerated ticks"
fi
[ "$CTL_ADVANCE" -eq "$((gb2_T - gb0_T))" ] \
    || fail "the control campfire's timer advanced $CTL_ADVANCE, expected exactly its $((gb2_T - gb0_T)) game ticks"
[ "$g2_COOK" -lt "$g2_CT" ] || fail "the campfire finished before the assertion window ended"
# Watch the accelerated potato finish; the control one is still raw.
ctl command "execute in minecraft:the_end run tp @s $(($AX - 2)).5 101 $((GCZ + 2)).5" >/dev/null
ctl wait 250 >/dev/null
info "$AX" 101 "$GCZ" g3
info "$BX" 101 "$HCZ" gb3
[ "$g3_OUT" = 0 ] || fail "the accelerated campfire still holds $g3_OUT potatoes (cook $g3_COOK)"
[ "$gb3_OUT" = 1 ] || fail "the control campfire finished as well: out $gb3_OUT cook $gb3_COOK"
[ "$gb3_COOK" -gt 0 ] && [ "$gb3_COOK" -lt 600 ] \
    || fail "the control campfire's cook is $gb3_COOK, expected a half-cooked potato"
# A dropped stack's reply is `{kind: "item", item: {item: "<id>", count: n}}`.
DROPPED=$(ctl entities 6 minecraft:item | jq '[.entities[] | select(.item.item == "minecraft:baked_potato")] | length')
[ "$DROPPED" -ge 1 ] || fail "the accelerated campfire dropped no baked potato: $(ctl entities 6 | jq -c '[.entities[] | select(.kind == "item") | .item]')"
echo "  the accelerated potato finished and dropped ($DROPPED baked potato); the control is still cooking (cook $gb3_COOK)"

echo "== H: the carried recipe crafts exactly one bottle in a real table =="
CTX=$((AX + 2))
CTZ=$((GCZ - 12))
ctl command "execute in minecraft:the_end run setblock $CTX 101 $CTZ minecraft:crafting_table" >/dev/null
ctl wait 4 >/dev/null
[ "$(block_name "$CTX" 101 "$CTZ")" = "minecraft:crafting_table" ] || fail "no crafting table at $CTX,101,$CTZ"
python3 "$PORT/tools/recipe_check.py" "$SOCK" "$CTL" "$CTX" 101 "$CTZ" \
    || fail "the carried recipe check failed"

echo "PASS: the bottle registers (an unknown id is refused) and stores exactly one tick a game tick while carried, creative mode pays nothing and an unfilled bottle pays nothing in survival, 620 carried ticks pay exactly 600 for a use that runs its 600-tick budget, 1,200 stored ticks buy two uses whose third cannot reach rate 4, a paid use makes a campfire's potato cook at exactly the accelerator's rate and finish while the control is still raw, and the jar's shaped recipe crafts exactly one bottle from its five ingredients"
