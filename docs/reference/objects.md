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
| `0x36` | 50 | 1-2 1-3 2-1 2-2 3-1 3-2 3-3 4-1 4-2 | drop block | still until stood on, then gives way and carries him down (see below) |
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
cartridge.

### The drop block, kind `0x36`

The most-used kind in the game: nine of the twelve levels place it, 50 records
in all.

It never moves on its own. Traced for 600 frames in World 1-2 with the camera
frozen (`tools/measure_level_kind.py`), its slot x and y do not change by a
pixel. It is not an enemy: the whole vertical overlap was swept with Mario
written into the game's own position bytes (`tools/probe_object_contact.py`)
and it never cost him a life, while the positive control lost one at five
offsets out of six.

What it does when stood on took a different instrument. Dropping Mario onto it
puts the change on the frame his feet reach the surface, and he is falling
three pixels a frame there, so a block that catches him for one frame and one
with no surface at all look the same. `probe_drop_block_support.py` places him
at rest height instead, says he is on the ground, and then writes nothing
more, so what follows is the game's own collision:

- He stays there. Nine frames at a fixed gap of 10, the same height a lift
  rests him at.
- Then the block descends a pixel a frame and does not stop, slow down or come
  back, with the gap fixed at 10 the whole way: it carries him.
- The negative control is the same position with the slot's kind byte set to
  `0xFF` first. With the object gone he falls at three pixels a frame from the
  first frame, which is what says the nine frames are the block holding him.

The slot does not empty. Its kind byte becomes `0x37` on the frame he touches
it and stays there, and every other slot is unchanged for the following
frames, so the block is neither removed nor handed to a falling object
somewhere else. `0x37` is the same object in its second state.

Measured again in World 1-3 with the same numbers. That run adds one thing:
partway down, the level's own ground catches Mario and the block carries on
without him. Its negative control is not usable there, since 1-3's terrain
holds him at that spot anyway, so the 1-2 run is the one the reading rests on.

Its surface is 8 pixels, one tile. Sweeping Mario's x a pixel at a time holds
him over 13, and 8 + 6 - 1 is 13 for the same 6 pixel foot the lift's 29 pixel
window gave over a 24 pixel surface. Two different surface widths, one foot,
two runs sharing no number.

The sprite survey had skipped this kind, and the reason is in the data: the
blocks are placed in rows, so there is always another one eight pixels away,
inside the window that decides an object's sprites are its own. With Mario
pinned far above and the neighbour identified in another slot, it draws as a
single tile `0xEE`, next door to the lift's `0xEF`, with the OAM priority bit
set so the background covers it. That makes it the third kind measured with
that bit, after `0x02` and `0x10`.

`X` in the level format. 48 of the 50 records go into the extracted levels;
one of World 1-3's five starts inside a solid tile, which the text grid cannot
represent, the same as two of that level's fallers.

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

The width took a second pass to resolve, and the first pass had it backwards.
The lift was taken to be two sprites across, 16 pixels, which no foot width
turns into a window of 29, so the extra was blamed on Mario's box being wider
than the engine's.

Reading the sprites off the running game
(`tools/measure_object_sprites.py`, see `docs/reference/sprites.md`) says the
lift is three tiles of `0xEF`, 24 pixels. Re-running the sweep a pixel at a
time rather than two, which is what the original 29 was rounded from, puts the
window at offsets -2 through +26 of the lift's own position: 29 exactly. A 24
pixel surface gives 29 for a foot of 6 pixels, and both edges of the window
place that foot in the same spot, centred on Mario's position. So the surface
is 24 and the game tests six pixels of Mario rather than his whole eleven,
which is what a platform game does so you cannot stand on a ledge by a
fingernail. Nothing here is left over to blame on Mario.

The sweep needs one guard at single-pixel resolution that it did not need at
two: it runs long enough for the object to leave the screen and its slot to be
handed to something else, and anything measured after that is a different
object's surface.

One note on method, because it cost a while. Taking a save state once the lift
is on screen and restoring it per offset looks like the tidy way to run this
sweep, and it silently breaks the experiment: restoring and then placing Mario
drops him through the lift at every offset, including the ones a plain run
holds him at. Every offset has to run inside the same continuous approach.

These have names now, and drawings: see "Where the names come from" above for
the first and `docs/reference/sprites.md` for the second.

### Which way a lift sets off

The cadence and the half cycles came from watching a lift that had already
been running for a while, which cannot say which way it left the position its
record decodes to. The engine assumed +1 on both axes for a long time, and
half of that was wrong.

`tools/measure_lift_phase.py` catches the slot on the frame the game fills it
rather than once the object has drifted somewhere readable, then releases
right so the camera stops: every pixel the slot's coordinates change after
that is the object's own. The vertical lift (`0x0A`) sets off **down** and the
horizontal one (`0x0B`) sets off **left**, in World 1-1 and again in World
1-2. Each ran 119 and 105 frames before reversing, against half cycles of 120
and 106, so a lift is created at one end of its travel rather than partway
along it: phase 0, and the one frame is the trace starting after a tick.

World 1-2's third gap is what made this worth measuring. The ledge ends at
column 222 and the level's one horizontal lift starts at column 232. Setting
off right it travels to 238 and never comes within 72 pixels of the ledge,
which no jump in the engine covers, so the gap read as uncrossable. Setting
off left it comes back to column 225 and the crossing is one jump, a ride to
the far end of the lift's travel, and a second jump. The walker in
`tests/extracted_level_1_1.rs` finishes World 1-2 now.

Two things about the instrument. The fly loop has to keep flattening the
terrain ahead: pinning Mario's Y byte carries him over the ground but not
through anything at that height, and World 1-2's block at column 212 stopped
him dead between the two lifts being compared, which is why the first run of
this reported the horizontal lift as never appearing. And the vertical lift is
the control for camera drift in the same run: its x moved 2 pixels in 400
frames after the release, so the horizontal one's 105 frames of leftward
travel are not the camera.

### Kind 0x42, Bunbun, flies left in bursts

World 1-2 carries 19 of them, more than any other kind in any World 1 level,
and every one was dropped at extraction. `tools/measure_flyer.py` traces it the
way `measure_lift_phase.py` traces a lift: catch the slot on the frame the game
fills it, release right so the camera stops, then read the slot every frame.

It flies **left, on a fixed height, in bursts**. One pixel a frame for 41
frames, then 33 frames holding still, and repeat: a 74 frame cycle covering 41
pixels, which averages a little over half a pixel a frame. It never reverses,
and it never gains or loses a pixel of height in 319 frames, which is four full
cycles. It leaves the screen on the left and its slot empties.

```
per frame  -1x41  +0x33  -1x41  +0x33  -1x41  +0x33 ...
```

Three controls, all in the same run. Mario pinned high (Y 24) and Mario pinned
low (Y 96) give traces that agree frame for frame, so the flight does not track
his height. Putting him back on the flyer's right once it has gone past does
not turn it. And two later instances, one of them created 24 pixels higher up,
repeat the same cadence, which is what says the 41 and the 33 belong to the
kind rather than to the phase one object happened to be created at.

Nothing solid sits on the rows it crossed, so whether terrain stops it is not
settled by this run.

On contact it is an ordinary enemy. `tools/probe_object_contact.py` sweeps
Mario through the whole overlap and its answer for `0x42` in World 1-2 is the
same six lines as its positive control, World 1-1's walker: unharmed with his
feet on top and the object gone, a life lost at every other offset. So it hurts
from the side and from below, and a stomp kills it, which is what the roster's
separate `0x43` "Bunbun, stomped" already suggested and did not establish.

### Kind 0x3F, Gao, stands still and spits

World 1-3 carries nine records of it, three of which spawn in normal play, and
World 4-2 three more. Traced with the camera frozen
(`uv run tools/measure_level_kind.py 1-3 0x3F 700`) it does not move a pixel
on either axis in 700 frames, which is longer than any cycle measured on the
cartridge (the Pakkun Flower's 200 frames is the slowest).

A kind that never moves is worth one more question, since the thing it does
may not be its own movement. `tools/watch_kind_neighbours.py` watches every
*other* slot while it sits there, and it is not idle:

```
frame  12: slot 1 filled with 0x23 at -4, +0 from it
frame 129: slot 1 emptied
frame 149: slot 1 filled with 0x23 at -4, +0 from it
frame 266: slot 1 emptied
...
```

Seven times in 900 frames, on the dot: a kind `0x23` fireball appears 4 pixels
to its left, lives 117 frames, and 20 frames later the next one appears, so
the cycle is 137 frames.

The fireball's own trace (`tools/measure_flyer.py 1-3 0x23`) is a straight
diagonal, up and to the left, until it leaves the screen: 103 pixels left and
50 up in 102 frames, so a pixel a frame across and a pixel every two frames
up. The position updates on alternate frames in steps of 2 and 3, which is
what a subpixel speed looks like read off a whole-pixel byte. The same shape
comes back from all three controls and from a second Gao on a different row.

The engine has the Gao standing still (`A` in the level format). The fireball
is measured and not implemented, and neither one's contact is: World 1-3
cannot host a contact probe, for reasons above.

## What is not decoded yet

- Which kind is which enemy. Five kinds spawn in World 1-1: `0x00` (nine
  times), `0x0E` (three), `0x04`, `0x0A`, `0x0B` (once each). `0x0A` and `0x0B`
  appear only in the last two records of the level, at columns 284 and 292.
  All five now have their movement measured; only the names are missing.
- What `0x02` is for, given it neither hurts Mario nor holds him up. Its
  drawing is now known and narrows the question rather than answering it: the
  game draws it with the hardware's priority bit set, so background pixels
  cover it, which is what a plant that hides in a pipe needs and matches its
  borrowed name. Only `0x02` and `0x10` carry that bit of the sixteen kinds
  measured (`docs/reference/sprites.md`). Why it is harmless is still open.
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
| `tools/measure_lift_phase.py` | which way a lift sets off, and from where in its cycle |
| `tools/measure_flyer.py` | how a flying kind moves, and whether it tracks Mario |
| `tools/probe_object_contact.py` | whether touching a kind costs Mario a life |
| `tools/probe_faller_trigger.py` | whether a fall is on a timer or on Mario |
| `tools/probe_death_state.py` | whether a state can kill Mario at all |
| `tools/find_skip_flag.py` | which byte of memory releases the skipped records |
| `tools/verify_skip_flag.py` | that byte across a whole level |

## The boss rooms

The four boss records place themselves the same way in the three worlds whose
boss is fought on foot. Read off the extracted levels rather than played:

| level | boss | column | right edge | row |
| --- | --- | --- | --- | --- |
| 1-3 | `0x08` King Totomesu | 292 | 300 | 11, on the floor |
| 2-3 | `0x1A` Dragonzamasu | 352 | 360 | 10, over open water |
| 3-3 | `0x32` Hiyoihoi | 292 | 300 | 12, on the floor |
| 4-3 | `0x61` Biokinton | 473 | 480 | 8 |

Each sits exactly eight columns from the right edge, and the last three
columns are a wall with a gap around rows 6 to 9 in the final one, which is
the passage out behind him. Three instances, so the shape is checked rather
than read off one (`each_world_builds_the_same_room_for_its_boss`).

Two things the third instance corrected. Distance to the end trigger looked
like the invariant from 1-3 and 3-3, which both put it four columns past the
boss, and 2-3 puts it one: a third level has no exit door, so its trigger
comes from `level::far_end` picking a standable cell, and where that lands is
a fact about that rule. And 1-3 and 3-3 both stand their boss on the room's
floor where 2-3's record is over open water, which is World 2 throughout.

4-3 is the exception in the way it should be. Its boss is eight columns from
the right edge like the others, but the end trigger is 155 columns behind it
at 318 and there is no wall anywhere in the level. That is the shape of a
flight rather than a room, and it is independent support for 4-3 being the
vehicle stage.

What none of this says is what a boss *does*: how it moves, what hurts it, or
what happens to the level when it dies. That needs the running cartridge.

### What World 1-3's boss does

King Totomesu (`0x08`) traced with the camera frozen
(`uv run tools/measure_level_kind.py 1-3 0x08 700`): **x never moves**, not a
pixel in 700 frames, and y runs 20 pixels up at one pixel every two frames,
20 back down at the same rate, then holds still for about 81 frames. That is
a 162-frame cycle of leaping straight up on the spot, 170 position updates
with 8 reversals, alternately 41 and 121 frames apart.

What happens on contact is **not measured**, and the reason is worth keeping.
`tools/probe_boss_contact.py` holds Mario at a fixed offset from the boss's
slot for 320 frames, longer than the 212 a death takes to register, and the
boss came back unharmed. That result is worthless: the same probe run against
`0x3F`, an ordinary enemy standing in the same room, also comes back
unharmed, so it cannot detect a death at that offset. One offset was the
mistake. `measure_enemy_box.py` sweeps a range precisely because the slot's
anchor and Mario's are not the same point, so dx=0 need not overlap anything.

Both kinds do leave their slot when Mario is placed 10 pixels above it, which
is a stomp for the ordinary enemy and an open question for the boss.

The second attempt swept all 41 offsets instead of testing one
(`tools/measure_boss_box.py`, which reaches the level once and restores a
snapshot per trial rather than replaying two levels 41 times). It moved the
failure rather than fixing it: the positive control now has no window
anywhere, so an ordinary enemy in World 1-3 does not cost Mario a life at any
offset. The offset was not the problem. Something about arriving here leaves
him in a state nothing collides with, and the suspect is the fly loop that
pins his Y byte every frame for thousands of frames to carry him across two
levels. The tie-breaker, if this is picked up again, is a control that uses
no enemy at all: walk him into a pit from the same restored state and see
whether *that* costs a life.

The third attempt ran that tie-breaker, and it answers cleanly. Take the floor
out of the tilemap under him, write nothing else at all so that what follows
is the game's own fall and the game's own death, and he loses no life. The
state is the problem, and no contact probe run from it can mean anything
whatever it points at. Note that the first version of this control pinned his
Y byte below the screen instead, which is not a fall and comes back
"unharmed" from any state at all.

That run also appeared to clear the fly loop, which was the standing suspect:
the tool lets go for 90 frames first so he falls to the floor, which is what
`tools/probe_ceiling_cap.py` does before walking him, and the pit still did
not kill him. That left the snapshot, since `tools/measure_lift.py` had
already recorded restoring a save state per trial silently breaking a
placement experiment. The named next step was the pit control with no restore
in front of it, inside one continuous approach.

### The state a death can happen in

That control has now been run (`tools/probe_death_state.py`), and the
snapshot is innocent. Two other things were wrong, and either one on its own
makes every contact result from World 1-3 meaningless.

**The fly loop leaves Mario unstepped.** It pins `0xC201` (his y) and
`0xC207` (his vertical phase, 0 for grounded) every frame, and when it lets
go he stays exactly where it left him: y 22, phase 0, for 240 frames standing
still and 240 more holding right, without a pixel of movement. Letting go for
90 frames does not undo it, so the earlier run's "the fly loop is cleared"
was reading a 90 frame window as a fall that never happened. Diffing his
bytes against an ordinary World 1-1 Mario shows exactly three that differ:

| address | World 1-1 | after the fly loop |
|---|---|---|
| `0xC200` | `0x80` | `0x00` |
| `0xC203` | `0x00` | `0x10` |
| `0xC210` | `0x80` | `0x01` |

Writing all three back restarts him. He falls and lands on the level's own
floor. `0xC200` alone does not do it, which is what makes the set rather than
the flag the thing to write.

**The pit dug the wrong columns.** It cleared the tilemap at
`screen x / 8`, and the background is a 32 column ring, so that is only a map
column while the level has not scrolled. World 1-1 from a cold boot is the one
place it holds, which is exactly where the control was run. `SCX`, the
register that shifts the background sideways on screen, reads 0 from work RAM
because the status bar is drawn at 0 and the playfield's value is set
mid-frame (`tools/sml_scroll.py`), so there is nothing to correct it with.
Clearing all 32 columns needs no scroll and cannot miss.

With both fixed, a pit costs Mario a life on frame 197 partway through World
1-3, reached by flying. The instrument works, and its positive control (the
same pit in World 1-1 from a cold boot, a life on frame 188) still passes.

### Contact still does not register, and the difference is one byte

The sweep was rebuilt on that state (`tools/measure_boss_box.py`) and the
pit control passes there too, from the same restored snapshot the trials run
from. The enemy control still finds nothing: kind `0x3F`, an ordinary enemy,
laid exactly on top of Mario for 300 frames, costs him nothing at any of the
41 offsets.

Two explanations are ruled out. The sweep now writes the object rather than
Mario, since his `0xC202` is a screen position and not a level position and
the two stop being the same number once a level has scrolled; that changed
nothing, and the same rig run in World 1-1 (`uv run tools/measure_boss_box.py
1-1 0x00 0x00 x`) hurts him at every offset it sweeps. So the rig registers
contact, and only in World 1-3 does it not.

What differs is in the bytes, and it is byte 1 of the slot record:

| | World 1-1 walker | World 1-3 objects |
|---|---|---|
| slot record | `00 01 86 3A ...` | `3F 00 16 3B ...` |
| Mario's y | `0x86`, on the floor | `0x16`, in mid air |

Byte 1 is 1 for the walker that is hurting him and 0 for everything reached
by flying, and it cannot be written: putting 1 there every frame leaves it 0
after the tick, so the game owns it. Mario's footing differs too, and
`land` cannot fix that without losing the object, since holding right for the
240 frames it takes to drop him from y 22 to y 38 scrolls the object out of
every slot, and holding left does not drop him at all.

Named next step: find what byte 1 of a slot record means.
`tools/watch_object_slot.py` follows one slot through a plain World 1-1 walk
with the terrain intact, so it can say when the byte turns 1 and what that
tracks. If it means the object has been woken by Mario actually arriving,
then no flatten-and-fly walkthrough can ever measure contact and the route
has to be a real playthrough.
