# The object list

Where a level's enemies and items come from, measured from the cartridge.

The column records in `docs/reference/level-1-1.md` carry terrain and nothing
else. Coins are the one exception, and only because the game draws them into
the background tilemap. Everything that moves comes from a second table.

## Finding it

Nothing here was read off a disassembly. The starting point was already in the
tree: while the column format was being pinned, `tools/find_level_pointer.py`
scanned work RAM for a 16-bit value inside the banked-ROM window that never
decreases while a level scrolls, and found exactly one, at `0xD010`. It holds
`0x6002` when World 1-1 opens, which is ROM offset `0x0A002`, and it advances
about 0.4 bytes per column. That is far too slow to be reading column records,
which average nine bytes each, so it was set aside as "some other table."

Hexdumping `0x0A002` shows three-byte groups whose first byte only ever
increases, running to a `0xFF`:

```
0C 0F 00   0F 0F 80   13 0C 84   21 0C 00   25 0C 84   28 04 00 ...
```

37 of them. `tools/trace_object_spawns.py` walks the real game through World
1-1 and prints the pointer's every move against the level's own column counter:

```
frame  column  pointer          record consumed
   40      29  0x6002->0x6005  0C 0F 00
   88      35  0x6005->0x6008  0F 0F 80
  152      43  0x6008->0x600B  13 0C 84
  ...
 2184     297  0x606E->0x6071  92 84 0B
```

36 moves for 37 records, since one move consumed two at once. The column
figure is the count of columns the game has written into the tilemap, which is
exact (it is the counting that captured all 300 columns of the level), and it
lands on `2x + 5` for every single record.

## The format

```
x      position, in units of 16 pixels (two columns)
y      low nibble is the row plus 2, high nibble is a pixel offset on x
kind   what to create, top bit marks an expert-mode-only object
```

A list is a run of these ending on `0xFF`. Records are sorted by `x`, which is
what lets the game use one forward read pointer and never look back.

### Position

`2x + 5` counts columns. Turning that into a camera position takes one more
number, since the game draws a column some way before it is visible. Pinning it needs
no scroll measurement, because the start of a level supplies it. The camera does
not move until Mario reaches the middle of the screen, so his screen X climbing
to its locked value of 81 marks a scroll of zero, and every pixel after that is
a pixel of scroll. `tools/measure_spawn_column.py` reads it off:

```
frame 34: mario reached screen x 81, camera unlocks
frame   43  scroll    9  columns 29
frame   51  scroll   17  columns 30
frame   59  scroll   25  columns 31
```

Eight frames, eight pixels, one column, with no drift. Column index `k` is
drawn at a scroll of `8k - 215`, so a record firing at count `2x + 5` fires at
a scroll of `16x - 183`.

The object then appears in its slot with an X of `0xBF`. That is an OAM
coordinate, which is offset by 8, so it is 183 pixels right of the camera. The
two cancel exactly:

```
world pixel x = (16x - 183) + 183 = 16x
```

No leftover offset, which is the sign the chain is right rather than fitted.

### The y byte is two fields

A filled slot gets a Y of `8 * (y & 0x0F) + 16`, an OAM coordinate, putting the
object at screen y `8 * (y & 0x0F)`. The playfield starts 16 pixels down, so
the object occupies playfield row `(y & 0x0F) - 2`.

Checked against the geometry decode, for all 37 of World 1-1's records: every
one lands on a cell that is not solid, and all 16 that sit at row 13 have solid
ground directly beneath them. Getting the row wrong by one would bury half the
list in the floor.

The high nibble is a horizontal offset, added to the X the slot gets. World 1-1
gave no way to see this, because it only ever uses nibbles 0 and 8 and the
whole nibble was small enough to read as part of the row. World 1-3 settles it
in one frame. Two of its records share an `x` byte and a row and differ only in
that nibble, and the game consumes both at once:

```
column 241  record 76 0D 36
           -> slot 1: 36 00 78 BF 03 00 21 00
           -> slot 2: 36 00 78 C7 03 00 21 00
```

Same Y, X eight pixels apart. Across 1-3's records the slot X comes out as
`0xBF + (y >> 4)` every time: nibble 0 gives `0xBF`, 4 gives `0xC3`, 8 gives
`0xC7`, C gives `0xCB`. So the full position is

```
world pixel x = 16 * x + (y >> 4)
row           = (y & 0x0F) - 2
```

### The expert-mode bit

Only 16 of the 37 records ever fill an object slot on a first play through.
They are exactly the 16 whose `kind` byte has the top bit clear, which starts
out as a correlation.

`tools/probe_object_type_flag.py` settles it by asking the game. It copies the
cartridge to a temporary file, clears the top bit of one record that never
spawns (`13 0C 84`, the third), and runs both:

```
shipped: 16 objects spawned
patched: 17 objects spawned
only in the patched run: [(43, 4)]
```

Column 43 is `2 * 0x13 + 5`, the exact place the shipped run walked past it.
So the bit suppresses the record, and the low seven bits are the kind either
way.

### What the bit selects for

21 of 37 records carry it in World 1-1, too many to be leftovers, so something
has to turn them on. The game's own memory is the place to look, and the search
can be exhaustive rather than clever: walk until the read pointer is sitting on
a skipped record, snapshot there, then for every byte of work RAM and high RAM
restore the snapshot, poke that byte, and walk the same short distance
(`tools/find_skip_flag.py`). Two skipped records sit inside the window, so a
real flag shows up as two extra spawns; a poke that lands in the stack fills
every slot at once and is easy to throw out.

```
baseline: 0 slot fills in 140 frames
FF9A -> 2 fills
FFB3 -> 1 fills
CFFE, FFB8, FFBD, FFBE, FFBF -> 9 fills each
```

0xFF9A is the only address with the right signature, and it reads 0 through
every frame of ordinary play. Holding it at 1 across a whole run of World 1-1
(`tools/verify_skip_flag.py`) releases the records in order, at their predicted
columns:

```
plain run: 16 objects spawned
FF9A held at 0x01: every record up to column 137 spawned, 15 of 15
```

Expert mode adds to normal play rather than swapping sets: the 16 still spawn
and the other 21 join them, so World 1-1 carries 37 objects on a replay.

The disassembly names the byte and confirms the reading. `hram.asm` calls
0xFF9A `hWinCount`, bank 0 sets it from work RAM with the comment "Expert Mode
activated when non zero", and `Call_24EF` is the read that matters:

```
	ldh a, [hWinCount]
	and a
	jr nz, .jmp_24F7
	bit 7, [hl]		; 7th bit set means the enemy appears only in expert mode
	ret nz
```

So the cartridge ships one object list per level holding both passes, and
finishing the game is what unlocks the second half of it. `sml extract-level
<level> --expert` writes that version out: World 1-1 goes from 11 ground
walkers to 26.

## Where the objects go

Work RAM holds a table of ten 16-byte slots at `0xD100`. Byte 0 is the kind,
`0xFF` for a free slot; byte 2 is the Y above; byte 3 is the X. World 1-1 never
uses more than the first two slots.

`tools/find_object_slots.py` found the table without guessing at a base
address: it watches for each of a consumed record's three bytes turning up at an
address that did not hold it the frame before, and counts that over all 37
records. Field 2 goes to `0xD100` eleven times and `0xD110` four times, and a
stride of 16 falls out.

## All three of World 1's lists

`tools/find_object_lists.py` drives the walkthrough across the level
boundaries and reads `0xD010` each time a level opens, which is the same
measurement 1-1 got for free:

| level | list | records | first bytes |
|---|---|---|---|
| 1-1 | `0x0A002` | 37 | `0C 0F 00` |
| 1-2 | `0x0A073` | 46 | `0E 0C 84` |
| 1-3 | `0x0A0FE` | 48 | `0D 4D 02` |

They sit back to back. 1-3 begins one byte past 1-2's terminator. 1-2 begins
two bytes past 1-1's, because there are two `0xFF` bytes at the end of 1-1's
list where every other list has one.

### The pointer does not carry its bank

`0xD010` is a bank-window address. `0x6002` is ROM `0x0A002` in bank 2 and
`0x0E002` in bank 3, and the pointer says nothing about which. That did not
matter while every level was in bank 2; World 2's are in bank 1.

Reading the bank off the mapped window at the moment a level opens gives the
wrong answer. It says bank 3 for World 1-1, whose list is pinned at `0x0A002`,
because the bank switched in at that instant is whatever the game last
touched, not the one the object list is read from.

`tools/find_object_bank.py` asks the spawns instead. It plays to the level,
pairs each step of the read pointer with the slot that fills on the same
frame, and checks the record's kind byte against all three candidate banks.
Only frames where exactly one record was consumed and exactly one slot filled
are scored: a slot can fill for reasons the list knows nothing about (World 2
gives Mario a torpedo, which takes a slot), and the pointer steps past
expert-only records without creating anything.

| level | pointer | bank 1 | bank 2 | bank 3 |
|---|---|---|---|---|
| 1-1 (control) | `0x6002` | 0/8 | **16/16** | 0/12 |
| 2-1 | `0x5179` | **37/37** | 2/37 | 0/30 |
| 2-2 | `0x5222` | **29/29** | 1/22 | 1/6 |

So World 2-1's list is at `0x05179` and 2-2's at `0x05222`, in bank 1
alongside World 2's terrain. 2-3's pointer reads `0x529B` when the level
opens, and its bank went unmeasured for a while: three attempts at the run
killed the machine before it started.

The cartridge answers it without the run. Every bank holding level data opens
with a table of object list pointers indexed by `world * 3 + level` (see
`level-format.md`), and bank 1's holds `0x529B` at index 5, World 2-3's slot.
The same table gives every measured list at its own index, so the pointer is
read from the bank the table sits in: `0x0529B`. It has 39 records, and bank
1's six lists then run back to back with no bytes left over, from World 2-1's
at `0x05179` through World 4-3's ending at `0x055B9`, two bytes before the bank's screen lists start.

### What the twelve levels actually use

Naming every level lets the whole roster be counted rather than guessed at.
Across all twelve object lists there are 41 distinct kind bytes, 40 of them in
normal play. Seven have been measured on the running cartridge, and those seven
account for 171 of the 481 normal-play records.

| kind | records | levels | name | what it is |
|---|---|---|---|---|
| `0x00` | 32 | 1-1 1-2 2-1 2-2 4-1 4-2 | Chibibo | ground walker |
| `0x02` | 29 | 1-3 2-1 2-2 3-1 3-2 3-3 4-1 4-2 | Pakkun Flower | harmless oscillator (left out) |
| `0x03` | 8 | 3-1 3-2 | Ganchan, spawning |  |
| `0x04` | 54 | 1-1 1-2 1-3 2-1 2-2 3-1 3-2 3-3 4-1 4-2 | Nokobon | ledge turner |
| `0x06` | 2 | 4-3 | Genkotsu |  |
| `0x08` | 1 | 1-3 | King Totomesu |  |
| `0x09` | 3 | 4-2 | Pompon Flower |  |
| `0x0A` | 16 | 1-1 1-2 2-1 2-2 3-1 3-2 4-1 | platform | vertical lift |
| `0x0B` | 21 | 1-1 1-2 2-1 2-2 3-1 3-3 4-1 4-2 | platform | horizontal lift |
| `0x0C` | 9 | 1-3 4-1 | Falling Slab | faller |
| `0x0E` | 10 | 1-1 3-2 3-3 | Fly | jumper |
| `0x10` | 24 | 2-1 2-3 | Honen |  |
| `0x16` | 5 | 2-2 | Mekabon |  |
| `0x1A` | 1 | 2-3 | Dragonzamasu |  |
| `0x1D` | 9 | 2-3 | Yurarin |  |
| `0x20` | 6 | 2-3 | Gunion |  |
| `0x24` | 7 | 2-1 2-2 2-3 | Yurarin Boo |  |
| `0x25` | 11 | 3-2 | Suu |  |
| `0x2F` | 5 | 2-3 |  |  |
| `0x31` | 7 | 3-1 3-3 | Tokotoko |  |
| `0x32` | 1 | 3-3 | Hiyoihoi |  |
| `0x35` | 1 | 3-2 | falling spike |  |
| `0x36` | 50 | 1-2 1-3 2-1 2-2 3-1 3-2 3-3 4-1 4-2 | drop block |  |
| `0x38` | 5 | 3-3 4-1 | diagonal platform, north east |  |
| `0x39` | 4 | 3-3 4-1 | diagonal platform, north west |  |
| `0x3A` | 6 | 3-1 3-2 3-3 4-2 | small vertical platform |  |
| `0x3B` | 4 | 3-1 3-3 | small horizontal platform |  |
| `0x3C` | 8 | 3-1 3-3 | Batadon |  |
| `0x3F` | 6 | 1-3 4-2 | Gao |  |
| `0x42` | 9 | 1-2 | Bunbun |  |
| `0x47` | 2 | 3-3 | Ganchan |  |
| `0x48` | 1 | 2-3 | Tamao |  |
| `0x49` | 21 | 3-1 3-2 4-1 4-2 | pipe cannon |  |
| `0x52` | 18 | 4-3 | Roketon |  |
| `0x53` | 28 | 4-3 | chicken |  |
| `0x54` | 17 | 4-2 4-3 |  |  |
| `0x55` | 14 | 4-1 4-2 | Pakkun Flower, upside down |  |
| `0x56` | 13 | 4-1 | Pionpi |  |
| `0x59` | 12 | 4-3 | Chikako |  |
| `0x61` | 1 | 4-3 | Biokinton |  |

A blank in the last column means the byte has never been watched on the
cartridge. Two are worth measuring before the others on volume alone: `0x36`
turns up in nine of the twelve levels, and `0x04`, already measured, is the
most common thing in the game.

### Where the names come from

Nothing observable on the running game carries a name, so this is the one part
of the object system that cannot be measured. The names are the equate list in
`enemies.asm` of the `kaspermeerts/supermarioland` disassembly, and they cover
40 of the 41 kinds the levels place. The one left out is `0x2F`, which that
list guesses at with a question mark against it.

The list is checked rather than trusted, in two ways.

**The seven measured kinds.** Every id measured on the cartridge, with no
reference to any name, lands on a name that matches what it was seen doing:
`0x00` walks and falls off ledges (Chibibo), `0x04` walks and turns at them
(Nokobon), `0x0E` stands still and hops (Fly), `0x0C` waits and then drops
(Falling Slab), `0x0A` and `0x0B` carry Mario (platforms). They span `0x00` to
`0x0E`, so a table off by any amount breaks several at once.

**The four bosses.** Each of `0x08`, `0x1A`, `0x32` and `0x61` appears exactly
once in the whole cartridge, and each appears in the third level of the world
whose boss it is named for: King Totomesu in 1-3, Dragonzamasu in 2-3,
Hiyoihoi in 3-3, Biokinton in 4-3. That also checks something else. Worlds 3
and 4 were numbered from a table in the ROM and have never been played here, so
Hiyoihoi turning up in the level the header calls 3-3 is independent
corroboration of the numbering. Tamao (`0x48`) sits in 2-3 alongside
Dragonzamasu, and Tatanga (`0x60`) is in no list at all, so whatever puts him
on screen in 4-3 is Biokinton rather than the level.

Two names disagree with what was measured, and the measurement wins.

- `0x0A` is named the horizontal platform and `0x0B` the vertical one. On the
  cartridge `0x0A` moves only on Y and `0x0B` only on X, watched twice
  (`measure_enemy_walk.py` and `probe_lift.py`). Both are recorded here as
  "platform" and the engine keeps the measured axes.
- `0x02` is named for a piranha plant, and a piranha plant hurts Mario. The
  contact sweep found it harmless at every vertical overlap through two full
  cycles. That is unexplained; it stays out of the extracted levels either way,
  and it is worth re-running the sweep on a level where the flower is not
  already extended when the probe starts.

`sml list-objects <level>` prints the name against each record.

### Every list runs to the end of its own level

The header pairs a screen list and an object list at the same index, and for
worlds 3 and 4 nothing has played there to confirm the pairing. The records
themselves do: across all twelve levels the last record sits between 96% and
99% of the way along the level, and none lands past the end.

| level | width | last record |
|---|---|---|
| 1-1 | 300 | 293 |
| 1-2 | 280 | 272 |
| 1-3 | 300 | 292 |
| 2-1 | 320 | 313 |
| 2-2 | 280 | 275 |
| 2-3 | 360 | 352 |
| 3-1 | 460 | 456 |
| 3-2 | 320 | 316 |
| 3-3 | 300 | 292 |
| 4-1 | 460 | 445 |
| 4-2 | 400 | 385 |
| 4-3 | 480 | 473 |

Of the 132 ways to pair a list with a level it does not belong to, 106 fall
outside that band, so the twelve agreeing is not something any pairing would
produce.

### World 2 puts objects on rows its records do not give

The kind byte matched on every one of those spawns. The row did not. 23 of
2-1's 37 spawns land somewhere its record's `y` byte does not predict: a `y`
of `0x13` reads as row 1 and the slot holds a Y of 166, the bottom of the
screen. 2-2 has 2 such spawns, and every one of World 1's agreed exactly.

That is not a bank question, so it did not change the reading above.

**What the disagreeing records are.** Naming the kinds answers most of it from
the ROM alone. Two kinds live only in World 2: Honen (`0x10`) and Yurarin Boo
(`0x24`), 38 records between them, and every single one has a row byte of 1.
Nothing else in the cartridge holds one row across a whole kind. A byte that
never varies is not carrying a position, and the observed Y agrees: 166 is not
`8 * row + 16` for any row, so it was never derived from the record. Those two
kinds place themselves, below the bottom of the screen, and swim up into it.

The counts line up on 2-2 exactly, where 2 records of those kinds spawn in
normal play and the trace found 2 disagreements. 2-1 has 22 and the trace found
23, so one spawn there is still unattributed. The nearest candidate is 2-1's
lone Chibibo at row 1, the only other row-1 record in the level, but nothing
here settles whether it is that one.

**World 2's lists are extracted now**, gated on placement rather than on the
trace. Every record of a kind the engine implements, across all twelve levels,
lands on a cell that is not solid, which is the same check World 1 passed and
which the row question would break. Neither Honen nor Yurarin Boo is among
them, so the records in doubt are left out of the level file regardless.

## What World 1-3's own kinds do

1-3 is the one level of World 1 whose extracted file carries no enemies at
all, because both kinds it introduces were unmeasured. `measure_level_kind.py`
fixes the reach: `trace_level_objects.reach_level` plays through 1-1 and 1-2
first, and from there the camera-freeze instrument works the same as it does in
1-1.

**Kind `0x02` oscillates vertically.** Its X never changes by a pixel in 600
frames. Its Y runs 16 pixels down at one pixel per frame, waits 62 frames, runs
16 pixels back up, waits 106, and repeats on a 200-frame period:

```
x: never moved
y: 96 moves of [-1, 1] px, 96 px travelled, +0 px net
   frames between moves: 1x90, 63x3, 107x2
   5 reversals, [78, 122] frames apart
```

**Kind `0x0C` falls.** X frozen again, then 127 moves of exactly +1, one per
frame, with no gap anywhere, until it left the level and its slot emptied. In
the one trace taken it stood still for 125 frames first.

### What they do on contact

Motion alone does not say what an object is. A 16 pixel oscillation fits a lift
and a crusher equally well. `probe_object_contact.py` writes Mario's position
straight into the two bytes the game reads, holds him there, and watches the
life counter, sweeping the whole vertical overlap rather than one offset.

Two controls run first, because an instrument that answers "hurt" to everything
is worth nothing. Both were needed. The positive control is World 1-1's kind
`0x00`, definitely an enemy:

```
positive control, World 1-1 kind 0x00 (a walker):
  offset +10: unharmed (and the object went away)
  offset  +6: lost a life
  ...
  offset  -8: lost a life
negative control, Mario 60 px to the side: unharmed
```

`+10` is feet on top, which is a stomp: Mario survives and the object goes. The
first version of this probe tested only that offset and reported the walker as
harmless. It also used a 180-frame window, and a forced death costs a life on
frame 212, after an animation the counter does not move during.

With the controls behaving, the two kinds separate cleanly.

**`0x02` is harmless.** Unharmed at every offset, through two full cycles of its
own motion. It does not hold Mario up either: carving the background tilemap
away so nothing else can catch him (`probe_lift.py 0x02 1-3 carve`) and dropping
him on it sends him straight past to the bottom of the screen. So it is neither
an enemy nor a platform.

**`0x0C` is an enemy.** Its sweep matches the walker's exactly: a stomp at `+10`
and a lost life at every offset from `+6` down. So it is a stompable hazard
that falls.

### What starts the faller

A hazard that drops when Mario walks under it and one that drops on a clock are
different obstacles, so `0x0C`'s wait had to be pinned before it could be
implemented. The two readings come apart if Mario is somewhere else:
`probe_faller_trigger.py` catches the slot the frame the game creates the
object, parks Mario at a chosen screen X, and counts frames to the first pixel
of fall.

```
  mario at screen x 191, object at 191: fell after 175 frames
  mario at screen x 60,  object at 191: fell after 175 frames
  mario at screen x 8,   object at 191: fell after 175 frames
```

Directly under it, a screen away, and at the left edge, all 175. A timer.

Catching the fill rather than waiting for the object to become visible matters
here: it is created at slot X `0xBF`, which is off the right of the screen, so
counting from when it scrolls into view spends part of the timer before the
measurement starts. That version reported 134.

### What World 1-3 added

1-3 is where the `y` byte's two fields came from, and it is worth saying how,
because the bytes alone pointed the wrong way.

Reading the whole low seven bits as a row works for 1-1 and 1-2 and puts nine
of 1-3's records at rows 66 to 77, off the bottom of a 16-row playfield. Those
nine are all kind `0x02`, all with a `y` high nibble of 4 or C. Switching to
the low nibble fixes the range and leaves 15 records sitting inside solid
tiles, which looks like a second error.

`tools/trace_level_objects.py` plays through to 1-3 and reads the slots, and
says the low nibble is right: record `0D 4D 02` gets a slot Y of `0x78`, which
is row 11, exactly what the low nibble predicts. The same trace showed the
high nibble driving the slot X, which is what completed the mapping.

The 15 records inside solid tiles are real. The game puts them there. All of
them are kinds `0x02` and `0x0C` that 1-3 introduces, and a trace confirms the
slot position matches the decode, so whatever those kinds are, they start
inside the terrain. `tests/rom_object_decode.rs` pins the count at 15 so a
later change to the position mapping cannot move it unnoticed.

One record, 1-3's `69 10 84`, has a row byte of 0, which puts it two rows above
the playfield. It is an expert-mode record, so normal play never reads it.

## What the kinds do

Measured with `tools/measure_enemy_walk.py`, which freezes the camera by
letting go of right (Super Mario Land only scrolls while Mario moves) and then
reads the slot's X and Y directly, so what is left is the object's own motion.
Mario is held at the fly height throughout so a long measurement is not cut
short by his death.

All five kinds World 1-1 spawns:

| kind | count | horizontal | vertical |
|---|---|---|---|---|
| `0x00` | 10 | 1 px left every 3 frames, 143 steps, no reversal | Chibibo | none |
| `0x04` | 1 | 1 px every 3 frames, turns at walls and at ledges | Nokobon | none |
| `0x0E` | 3 | 1 to 4 px steps in bursts, net about zero | Fly | same, with 54-frame pauses |
| `0x0A` | 1 | none | platform | 1 px every 2 frames, reversing every 120 frames (a lift) |
| `0x0B` | 1 | 1 px every 2 frames, reversing every 106 frames (a lift) | platform | none |

`0x00` and `0x04` share a cadence exactly, and they part company at a ledge.
Writing a wall and then a pit into the tilemap in front of each
(`tools/probe_walker_turn.py`) separates them cleanly:

| kind | wall ahead | pit ahead |
|---|---|---|
| `0x00` | turns | walks off and falls |
| `0x04` | turns | turns |

So the cartridge has both a walker that ignores ledges and one that respects
them, which is why the engine carries two.

### Kind 0x0E, the jumper

`measure_enemy_walk.py` could only describe this one as noise: bursts of up to
4 px a frame on both axes with 54-frame pauses. Its summary suits something
that walks. `trace_jumper.py` prints the raw arc instead, with the camera held
still the same way, and the cycle comes out exact.

```
resting row at slot Y 136, lowest reached 121 (15 px above it)
9 excursions above the resting row, starts 102 frames apart, 45 frames each
the second excursion, as height above the resting row:
  0 4 4 4 8 8 8 10 10 10 12 12 12 13 13 13 14 14 14 15 15 15 15 15 15
  15 15 15 14 14 14 13 13 13 12 12 12 10 10 10 8 8 8 4 4 4 0
```

Every value is held for three frames, so the object updates its position 16
times per hop and then stands perfectly still. The updates are `-2` px
sideways every time, and the rises are:

```
+4 +4 +2 +2 +1 +1 +1 0 0 -1 -1 -1 -2 -2 -4 -4
```

That is not constant deceleration (which would be `+4 +3 +2 +1 ...`), so the
cartridge is reading a table and `src/core/enemy.rs` reads the same one. The
cycle is 16 x 3 = 48 frames of hop plus 54 of standing, which is the 102 the
excursions are apart.

Turning had to be measured differently from the walkers. A hopper reverses on
its own and rises and falls every hop, so "it turned" and "it fell" are true
whatever you put in front of it. `probe_walker_turn.py` now runs a control with
no obstacle and compares how far the object got:

```
control : reached  93 px
wall    : reached  36 px
pit     : reached  61 px, slot emptied at frame 180
```

So it turns at a wall and hops into a pit and out of the level.

### The two by the exit are lifts

`0x0A` and `0x0B` are the last two records in the level, at columns 284 and
293, either side of the exit door. Both move at a flat 1 pixel every 2 frames
and both reverse on a strict cycle, one straight up and down over 60 pixels,
the other straight left and right over 53.

That is the motion of a lift, and World 1-1's raised exit needs some way of
being reached, but Mario is the one who can answer it. `tools/probe_lift.py`
puts him directly above one and drops him:

```
frame  mario y   object x   object y
   15      102        136        112
   35      112        136        122
   55      122        136        132
   60      120        136        130
   65      117        136        127
```

He lands on it and rides it, 10 pixels above it, back down and up again. The
horizontal one holds him just as steadily: his Y sits at 38 against the
object's 48 and stays there for 195 frames.

So neither is an enemy. Both carry Mario, which is what makes the level's upper
exit reachable.

### How big a lift is

Building one needs its surface, and `tools/measure_lift.py` drops Mario at
every offset either side of it. He is held from an X byte of 134 to one of 162,
with the lift's slot X at 136: a window 29 pixels wide. Outside it he falls
straight past to the floor.

The height is exact and comes out on a tile boundary. Mario's Y byte reads 134
while he stands on World 1-1's ordinary ground, whose top edge is screen y 128,
so his Y byte is his feet plus 6. On the lift he rests at the slot's Y minus 10,
which puts his feet at `slotY - 16`, and that is the slot's Y as a screen
coordinate. In other words his feet sit exactly on the top of the row the
record decodes to. The walker agrees from the other direction: its slot Y of
136 puts its own top at screen y 120 and its bottom on the ground at 128.

The width does not resolve as cleanly. The lift is drawn from two 8-pixel
sprites, so 16 pixels across, and a 16-pixel platform under an 8-pixel Mario
would give a window of 23 rather than 29. The extra 6 says the cartridge's
small Mario is about 14 pixels wide for this test, against the 8 our engine
uses. The discrepancy sits in Mario's box. It is recorded in
`docs/reference/faithfulness.md` rather than papered over by widening the lift.

One note on method, because it cost a while. Taking a save state once the lift
is on screen and restoring it per offset looks like the tidy way to run this
sweep, and it silently breaks the experiment: restoring and then placing Mario
drops him through the lift at every offset, including the ones a plain run
holds him at. Every offset has to run inside the same continuous approach.

None of these has a name yet. Naming them means matching a 16-pixel sprite to
an SML enemy, and a wrong guess would put an invented enemy in the game through
the back door, so the ids stay as ids.

## What is not decoded yet

- Which kind is which enemy. Five kinds spawn in World 1-1: `0x00` (nine
  times), `0x0E` (three), `0x04`, `0x0A`, `0x0B` (once each). `0x0A` and `0x0B`
  appear only in the last two records of the level, at columns 284 and 292.
  All five now have their movement measured; only the names are missing.
- What `0x02` is for, given it neither hurts Mario nor holds him up.
- Whether the faller stops on a floor or passes through it. Ours lands, like
  every other enemy; the one traced fell out of the level, which does not
  distinguish the two.
- Why 1-3 starts them inside solid tiles.
- The rest of a slot's 16 bytes.

## Tools

| tool | what it answers |
|---|---|
| `tools/trace_object_spawns.py` | when the read pointer moves, and past what |
| `tools/find_object_slots.py` | where a record's bytes land in RAM |
| `tools/dump_object_slots.py` | what a slot holds as it fills |
| `tools/watch_object_slot.py` | one slot through a plain walk, terrain intact |
| `tools/measure_spawn_column.py` | column count against camera scroll |
| `tools/probe_object_type_flag.py` | what the kind byte's top bit does |
| `tools/measure_enemy_walk.py` | how a kind moves, with the camera frozen |
| `tools/probe_lift.py` | whether an object holds Mario up |
| `tools/measure_lift.py` | a lift's surface: how wide, and how high Mario rests |
| `tools/probe_walker_turn.py` | whether a walker turns at a wall or a ledge |
| `tools/find_object_lists.py` | where each level's list starts |
| `tools/trace_level_objects.py` | the same trace in any pinned level |
| `tools/find_object_bank.py` | which ROM bank a level's object list is in |
| `tools/trace_jumper.py` | the raw arc of a hopping kind, frame by frame |
| `tools/measure_level_kind.py` | how a kind moves, in 1-2 and 1-3 as well as 1-1 |
| `tools/probe_object_contact.py` | whether touching a kind costs Mario a life |
| `tools/probe_faller_trigger.py` | whether a fall is on a timer or on Mario |
| `tools/find_skip_flag.py` | which byte of memory releases the skipped records |
| `tools/verify_skip_flag.py` | that byte across a whole level |
