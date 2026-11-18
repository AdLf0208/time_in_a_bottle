#!/usr/bin/env bash
# In-game check for Time in a Bottle slice 1: the bottle stores ticks and spends
# them accelerating a furnace by exactly the ticks spent.
#
# Every mod command is typed with `ctl chat "/cmd"` because `ctl command` runs
# the console path and bypasses the native `perform_command` hook.
#
# The End, in the void at x=200. Three furnaces stand on the stone floor with 64
# iron ore and 64 coal each, so all cook for the whole run:
#   A at z=0, the survival target;
#   B at z=4, the untouched control;
#   C at z=8, the creative target.
#
# `/tiab info <x> <y> <z>` prints one atomic line: the game clock, the first
# bottle's stored/total account and the furnace's `cooking_time_spent`, output
# count and fuel. One line with the clock makes every assertion exact — the
# bottle gains 20 stored ticks at the end of every second (`TiabMod`), so
#   spent = 20 * (floor(t1/20) - floor(t0/20)) - (stored1 - stored0)
# and a furnace's "total cooked" is `out * 200 + cook`, so against the control
# furnace its extra block ticks are exact to the tick.
#
# Default run:
#  A  the bottle registers; the three furnaces cook;
#  B  carrying the bottle stores exactly one tick a game tick (20 a second);
#  C  creative mode pays nothing (upstream's `canUse`): an unfilled bottle runs
#     exactly 600 block ticks, and the account does not change;
#  D  in survival an unfilled bottle pays for nothing (no account change, no
#     acceleration);
#  E  a bottle with 600+ stored ticks pays exactly 600 for one use, which runs
#     exactly 600 block ticks; a second click at the remainder does not upgrade;
#  F  two uses from 1,200 stored ticks cost exactly 1,200 and run at rate 2; a
#     third click cannot reach rate 4.
#
# The mod is one SDK package (`tiab.hmod`: the item, its assets/data and the
# code), so the game must be a launcher-built one that carries it — there is no
# `--dev-mod content` any more, and the old EMBEDDED/CONTROL legs (a separate
# content mod and a bundle-embedded copy) retired with the `.hnmod` format.
# Point `GAME=` at the launcher-built `horizon-client` of an instance with
# `tiab.hmod` installed; see notes.md, "Migration to the SDK format".
#
# SELFTEST=1 flips E's 600 to 601 (it must FAIL), proving the scenario can fail.
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
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --headless
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
# furnace. Fills "<P>_T", "<P>_STORED", "<P>_TOTAL", "<P>_COOK", "<P>_OUT".
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
    local stored total cook out acc
    stored=$(sed -n 's/.* stored=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    total=$(sed -n 's/.* total=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    cook=$(sed -n 's/.* cook=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    out=$(sed -n 's/.* out=\(-\{0,1\}[0-9][0-9]*\) .*/\1/p' <<<"$line")
    acc=$(sed -n 's/.* acc=\([a-z0-9x]*\).*/\1/p' <<<"$line")
    [ -n "$stored" ] && [ -n "$total" ] && [ -n "$cook" ] && [ -n "$out" ] && [ -n "$acc" ] \
        || fail "the info line did not parse: $line"
    printf -v "${prefix}_T" '%s' "$t"
    printf -v "${prefix}_STORED" '%s' "$stored"
    printf -v "${prefix}_TOTAL" '%s' "$total"
    printf -v "${prefix}_COOK" '%s' "$cook"
    printf -v "${prefix}_OUT" '%s' "$out"
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

# Hold the bottle and right-click the furnace's west face (the player teleports
# beside it). `ctl use-item` is a player right-click, so the native
# `right_click_block` hook hears it.
use_on() { # use_on X Y Z
    ctl command "execute in minecraft:the_end run tp @s $(($1 - 1)).5 $2 $(($3)).5" >/dev/null
    ctl wait 2 >/dev/null
    ctl select tiab:time_in_a_bottle >/dev/null
    ctl wait 2 >/dev/null
    ctl look-at-block "$1" "$2" "$3" west >/dev/null
    ctl wait 2 >/dev/null
    ctl use-item >/dev/null
    ctl wait 2 >/dev/null
}

echo "== launch (The End arena, the package carries the item) =="
launch
arena

echo "== A: the bottle registers, and three furnaces cook =="
ctl give tiab:time_in_a_bottle 1 >/dev/null
ctl wait 4 >/dev/null
ctl inventory | jq -e '[.hotbar[], .main[] | select(.item == "tiab:time_in_a_bottle")] | length >= 1' >/dev/null \
    || fail "the bottle item did not register: $(ctl inventory | jq -c '{hotbar,main}')"
place_furnace "$AX" 101 0
place_furnace "$BX" 101 "$BZ"
place_furnace "$CX" 101 "$CZ"
ctl wait 20 >/dev/null
info "$AX" 101 0 a
info "$BX" 101 "$BZ" b
info "$CX" 101 "$CZ" c
[ "$a_COOK" -ge 0 ] && [ "$b_COOK" -ge 0 ] && [ "$c_COOK" -ge 0 ] || fail "a furnace is missing"
[ "$(block_prop "$AX" 101 0 lit)" = "true" ] || fail "furnace A is not lit"
echo "  bottle in the inventory; A cook $a_COOK, B cook $b_COOK, C cook $c_COOK"

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
# must advance by exactly that many block ticks over the rest of its life.
[ "$c1_RATE" = 1 ] || fail "a creative use started at rate $c1_RATE, expected 1"
[ "$c1_REM" -gt 0 ] || fail "no accelerator after the creative use ($c1_LINE)"
[ "$CREATIVE_EXTRA" -eq "$c1_REM" ] \
    || fail "a creative use ran $CREATIVE_EXTRA block ticks, expected exactly its remaining $c1_REM"
echo "  creative use: spent 0, ran exactly $CREATIVE_EXTRA block ticks (its remaining budget)"

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
if [ "$SELFTEST" = 1 ]; then
    echo "  (selftest: asserting the use ran one block tick too many)"
    [ "$SPENT_EXTRA" -eq $((e2_REM + 1)) ] || fail "the use ran $SPENT_EXTRA block ticks, expected $((e2_REM + 1))"
else
    [ "$SPENT_EXTRA" -eq "$e2_REM" ] \
        || fail "the use ran $SPENT_EXTRA block ticks, expected exactly its remaining $e2_REM"
fi
echo "  one use: cost exactly $SPENT stored ticks, ran exactly $SPENT_EXTRA block ticks (its remaining budget)"

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
echo "  two uses: cost $SPENT, remaining ${f2_RATE}x${f2_REM}, ran $RATE_EXTRA block ticks"
# At rate R the furnace gets R block ticks a game tick from the accelerator, so
# beyond its own tick it gains R x its remaining game ticks.
[ "$RATE_EXTRA" -eq $((f2_RATE * f2_REM)) ] \
    || fail "a rate-2 use ran $RATE_EXTRA extra block ticks, expected exactly ${f2_RATE}x$f2_REM"

echo "PASS: the bottle registers and stores exactly one tick a game tick while carried, creative mode pays nothing and an unfilled bottle pays nothing in survival, 620 carried ticks pay exactly 600 for a use that runs exactly 600 block ticks, and 1,200 stored ticks buy two uses whose third cannot reach rate 4"
