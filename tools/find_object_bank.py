# /// script
# requires-python = ">=3.10"
# dependencies = ["pyboy"]
# ///
"""Which ROM bank does a level's object list live in?

0xD010 holds the game's own read pointer into the object list, but it is a
bank-window address: 0x6002 is ROM 0x0A002 in bank 2 and 0x0E002 in bank 3,
and nothing in the pointer says which. Reading the bank off the mapped window
at the moment the level opens gives the wrong answer (it says bank 3 for
World 1-1, whose list is known to be at 0x0A002), because the bank switched in
at that instant is whatever the game last touched.

So ask the spawns instead. Play to the level, watch the pointer step, and each
time a slot fills compare the slot's position against what the record at that
pointer would predict, once per candidate bank:

    slot byte 0 = kind          slot Y = 8 * (y & 0x0F) + 16

The bank whose records predict the slots is the level's bank, by construction.
Every other bank is a different three bytes and predicts something else.

The bank is scored on the kind byte alone. Position is reported alongside it
but not scored: slot X is a screen coordinate and some kinds have already moved
by the frame the slot is read, and World 2-1 has records whose slot Y ignores
the row nibble outright (a `y` byte of 0x13 predicts 40 and the slot reads 166,
the bottom of the screen). Whatever that is, it is not a question about banks,
and folding it into the score would hide a clean answer behind an open one.

World 1-1 is the control: its list is already pinned at 0x0A002, so a run that
does not answer bank 2 for it is measuring itself.

Usage: uv run tools/find_object_bank.py [level]
"""

import sys

sys.path.insert(0, "tools")

from dump_object_slots import EMPTY, SLOTS, slot
from run_through_levels import (
    FLY_Y,
    LEVEL_GAP_FRAMES,
    LIVES,
    MARIO_Y,
    PHASE,
    SCREEN_X,
    SPAWN_X,
)
from trace_level_objects import NAMES, reach_level
from trace_object_spawns import BANK_BASES, pointer, rom_offset

FRAMES = 8000
SLOT_Y = 2


# An object resting on the ground reads a pixel below its record's row by the
# frame its slot is seen; one that starts in the air reads exactly.
SETTLE = 1


def same_kind(record, state):
    return (record[2] & 0x7F) == state[0]


def same_row(record, state):
    return abs(8 * (record[1] & 0x0F) + 16 - state[SLOT_Y]) <= SETTLE


def main():
    want = sys.argv[1] if len(sys.argv) > 1 else "2-1"
    rom = open("super_mario_land.gb", "rb").read()

    pb, capture = reach_level(NAMES.index(want))
    if pb is None:
        print(f"never reached World {want}")
        return 1

    first = pointer(pb)
    last = first
    types = [slot(pb, s)[0] for s in range(SLOTS)]
    scores = {bank: [0, 0] for bank in BANK_BASES}
    rows = {bank: 0 for bank in BANK_BASES}
    quiet = 0

    for frame in range(FRAMES):
        pb.memory[MARIO_Y] = FLY_Y
        if pb.memory[SCREEN_X] > SPAWN_X:
            pb.memory[PHASE] = 0
        pb.tick()
        if pb.memory[LIVES] == 0:
            break
        quiet = 0 if capture.step(pb, frame) else quiet + 1

        now = pointer(pb)
        stepped = []
        while last != now:
            stepped.append(last)
            last += 3

        filled = []
        for s in range(SLOTS):
            state = slot(pb, s)
            if types[s] == EMPTY and state[0] != EMPTY:
                filled.append(state)
            types[s] = state[0]

        # Only a fill on the same frame as a step counts. A slot can fill for
        # reasons the list knows nothing about (World 2 gives Mario a torpedo,
        # which takes a slot), and a step can pass an expert-only record, which
        # creates nothing. Scoring the frames where exactly one record was
        # consumed and exactly one slot filled leaves a pairing that is not in
        # question, and drops the rest rather than guessing at them.
        if len(filled) != 1:
            continue
        for bank in BANK_BASES:
            records = [rom[rom_offset(v, bank) : rom_offset(v, bank) + 3]
                       for v in stepped]
            records = [r for r in records if not r[2] & 0x80]
            if len(records) != 1:
                continue
            scores[bank][1] += 1
            if same_kind(records[0], filled[0]):
                scores[bank][0] += 1
            if same_row(records[0], filled[0]):
                rows[bank] += 1

        if capture.columns and quiet > LEVEL_GAP_FRAMES:
            break
    pb.stop()

    print(f"World {want}: 0xD010 opens at 0x{first:04X}\n")
    best = None
    for bank in BANK_BASES:
        hit, total = scores[bank]
        at = rom_offset(first, bank)
        share = hit / total if total else 0
        print(f"  bank {bank:#07X}, list at 0x{at:05X}: "
              f"{hit}/{total} kinds, {rows[bank]}/{total} rows")
        if best is None or share > best[1]:
            best = (bank, share)
    if best[1] < 1.0:
        print("\nno bank predicts every kind; the reading is not settled")
        return 1
    bank = best[0]
    print(f"\nWorld {want}'s object list is at 0x{rom_offset(first, bank):05X}")
    if rows[bank] < scores[bank][1]:
        missed = scores[bank][1] - rows[bank]
        print(f"  {missed} of its spawns land on a row its record does not give")
    return 0


if __name__ == "__main__":
    sys.exit(main())
