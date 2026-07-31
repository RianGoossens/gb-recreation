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
y      row, plus 2, top bit unexplained
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

### Row

A filled slot gets a Y of `8 * y + 16`, also an OAM coordinate, putting the
object at screen y `8 * y`. The playfield starts 16 pixels down, so the object
occupies playfield row `y - 2`.

Checked against the geometry decode, for all 37 records: every one lands on a
cell that is not solid, and all 16 that sit at row 13 have solid ground
directly beneath them. Getting the row wrong by one would bury half the list in
the floor.

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

## What is not decoded yet

- Which kind is which enemy. Five kinds spawn in World 1-1: `0x00` (nine
  times), `0x0E` (three), `0x04`, `0x0A`, `0x0B` (once each). `0x0A` and `0x0B`
  appear only in the last two records of the level, at columns 284 and 292.
- The top bit of the `y` byte. Seven records carry it, five of which also carry
  the skip bit, so it is independent of that.
- Where World 1-2's and 1-3's lists start. The same technique applies: read
  `0xD010` the moment each level opens.
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
