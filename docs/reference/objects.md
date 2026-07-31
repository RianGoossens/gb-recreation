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
kind   what to create, top bit marks a record normal play skips
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

### The skip bit

Only 16 of the 37 records ever fill an object slot. They are exactly the 16
whose `kind` byte has the top bit clear, which is a correlation and not yet an
explanation.

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

What the flag selects *for* is open. 21 of 37 records carry it in World 1-1,
which is too many to be leftovers. A second placement set for the harder second
quest would fit the shape of the data, but nothing has been measured that says
so, and it is not claimed here.

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
the playfield. It carries the skip bit and never spawns.

## What the kinds do

Measured with `tools/measure_enemy_walk.py`, which freezes the camera by
letting go of right (Super Mario Land only scrolls while Mario moves) and then
reads the slot's X and Y directly, so what is left is the object's own motion.
Mario is held at the fly height throughout so a long measurement is not cut
short by his death.

All five kinds World 1-1 spawns:

| kind | count | horizontal | vertical |
|---|---|---|---|
| `0x00` | 10 | 1 px left every 3 frames, 143 steps, no reversal | none |
| `0x04` | 1 | 1 px every 3 frames, one reversal in 888 frames | none |
| `0x0E` | 3 | 1 to 4 px steps in bursts, net about zero | same, with 54-frame pauses |
| `0x0A` | 1 | none | 1 px every 2 frames, reversing every 120 frames (a lift) |
| `0x0B` | 1 | 1 px every 2 frames, reversing every 106 frames (a lift) | none |

`0x00` and `0x04` share a cadence, which is a reason to think they are the same
sort of thing. `0x00` never turned in 143 steps and `0x04` turned once, so what
makes it turn is not settled: one reversal cannot separate a wall from a timer.

`0x0E`'s bursts with long pauses and its steps of up to 4 pixels per frame in
both axes are the shape of something that jumps.

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

None of these has a name yet. Naming them means matching a 16-pixel sprite to
an SML enemy, and a wrong guess would put an invented enemy in the game through
the back door, so the ids stay as ids.

## What is not decoded yet

- Which kind is which enemy. Five kinds spawn in World 1-1: `0x00` (nine
  times), `0x0E` (three), `0x04`, `0x0A`, `0x0B` (once each). `0x0A` and `0x0B`
  appear only in the last two records of the level, at columns 284 and 292.
- What kinds `0x02` and `0x0C` are, and why 1-3 starts them inside solid tiles.
- What makes `0x04` turn around.
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
| `tools/find_object_lists.py` | where each World 1 level's list starts |
| `tools/trace_level_objects.py` | the same trace in any World 1 level |
