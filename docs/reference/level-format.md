# Level format

Levels are plain text. One character is one 8x8 tile, one line is one row, and
every row must be the same width (the level is a rectangle). Save a level as a
`.txt` file and load it with the `run` or `play` commands (see below), or in
code with `Level::from_file` / `Level::from_text`.

## Markers

| Char | Meaning |
|------|---------|
| `#` | solid tile (wall, floor, ceiling) |
| `^` | one-way platform: stands on it from above, walks and jumps through it otherwise |
| `.` | empty space |
| `M` | Mario's spawn (use one) |
| `G` | a ground walker: turns at walls, walks off ledges |
| `T` | a ground walker that turns at ledges too |
| `J` | the cartridge's Fly (kind `0x0E`): still for 54 frames, then a 32 px hop 15 px high |
| `D` | a faller: holds position for 175 frames, then drops a pixel a frame |
| `N` | a Bunbun (kind `0x42`): flies left through everything, 41 px in bursts of 41 frames with 33 still between them |
| `A` | a Gao (kind `0x3F`): stands still, and spits a fireball every 137 frames |
| `F` | a Bouncer (walks and hops), see the note on non-cartridge content |
| `C` | a coin |
| `S` | a star (grants brief invincibility), see the note on non-cartridge content |
| `W` | a flower (makes Mario fire-powered) |
| `?` | question block, gives a coin when bumped (solid) |
| `P` | power block, gives a mushroom when bumped (solid) |
| `B` | brick block (solid) |
| `V` | a lift that runs up and down, 60 px each way |
| `H` | a lift that runs side to side, 53 px each way |
| `X` | a drop block: holds still, and nine frames after Mario stands on it, gives way and carries him down |
| `E` | the level-end trigger (walk into it to finish; not solid) |

Any other character is treated as empty. The block markers (`?`, `P`, `B`) are
part of the solid world, so Mario stands on them and bumps them from below. `E`
is passable.

## Content Super Mario Land does not have

Two markers spawn things the cartridge has no equivalent of: `S`, since Super
Mario Land has no invincibility star at all, and `F`, a generic hopping enemy
of our own (the `Bouncer`) standing in for an SML enemy that has not been
pinned yet. It kept the letter `F` after being renamed, because the cartridge's
own kind `0x0E` turned out to be named Fly and a level file should not break
over what we call something. See
`docs/reference/faithfulness.md`.

The end goal is a faithful recreation, so **`run` and `play` drop both spawns
by default**. A level file using them still loads and still plays; it just
plays without them. Pass `--allow-non-canonical` (anywhere in the arguments)
to keep them:

```sh
cargo run -- play out.png 60 right mylevel.txt --allow-non-canonical
```

## Rules

- Every row must be the same width. A ragged level is reported as an error, not
  loaded.
- Put exactly one `M`. If there is none, Mario starts at the top-left.
- Leave the two bottom rows as floor (`#`) unless you want a pit; a gap in the
  floor is a pit Mario can fall into.

## Example

See `levels/example.txt`. It has a floor with a pit to jump, a floating
platform with coins above it, a question block and a power block, a couple of
Goombas, a brick, and an end trigger on the right.

```
........................................
...............C.C......................
......?..P....#####....B................
..M..C.C............G.........G.......E.
########################..##############
```

(That excerpt is trimmed for width; the real file is 40 tiles wide and 12 tall.)

## Tuning

Geometry is one half of a level; how Mario moves through it is the other,
and that is data too. A tuning file is a small `key = value` text block,
one assignment per line (`#` starts a comment, blank lines are ignored). An
unset key keeps its default; an unknown key or an unparseable value is a
reported error rather than being silently ignored.

| Key | Meaning | Default |
|-----|---------|---------|
| `walk_accel` | Horizontal acceleration while a direction is held | 43 |
| `friction` | Deceleration applied once no direction is held | 43 |
| `max_walk_speed` | Cap on horizontal speed | 256 |
| `jump_velocity` | Upward speed at takeoff, held roughly steady while rising with the button held | 602 |
| `rise_drift` | Tiny deceleration applied each frame of a held rise | 10 |
| `max_rise_frames` | How many frames a held rise lasts before it cuts to falling regardless of the button | 12 |
| `jump_cut` | Deceleration applied if the jump button is released before `max_rise_frames` runs out | 29 |
| `gravity` | Downward acceleration while falling | 76 |
| `max_fall_speed` | Cap on downward speed, so a long fall does not tunnel through a thin floor | 640 |
| `stomp_bounce` | Upward speed Mario gets from stomping an enemy | 360 |
| `timer_start` | The level timer's starting value | 400 |

All of the movement values are in subpixels (256 per pixel) per frame, or
per frame squared for the acceleration ones; see `docs/reference/physics.md`
for where each one comes from. A custom tuning file applies for the whole
run, including across level transitions and restarts within the same
session, not just the first level it is loaded with.

```
# a floatier, higher-jumping feel
gravity = 40
jump_velocity = 900
```

## Running a level

```sh
# play it in a window (needs the gui feature); tuning.txt is optional
cargo run --features gui -- run levels/example.txt [tuning.txt]

# render a frame headlessly to a PNG (great for sharing a screenshot);
# tuning.txt is optional here too
cargo run -- play shot.png 1 "" levels/example.txt [tuning.txt]
```

Where a level file goes, the name of one of the cartridge's twelve levels
works too, and loads what `sml extract-level` wrote for it:

```sh
cargo run -- play shot.png 400 right 3-1
```

Only World 1 can be finished. See `docs/reference/faithfulness.md` for what
the other three worlds are missing.

## The table at the start of a bank

Every ROM bank that holds level data opens with a 0x32-byte header, and the
tile graphics start right after it (which is why World 1's tiles are at
`0x08032` and World 2's at `0x04032`). The header is two tables of 16-bit
pointers:

| offset | contents |
| --- | --- |
| `0x00` | thirteen screen list pointers |
| `0x1A` | twelve object list pointers |

Both are indexed by `world * 3 + level`, so World 1-1 is index 0 and World 4-3
is index 11. `level::level_list`, `level::level_objects` and
`level::level_list_head` read them.

A bank holds only some of the twelve levels, and the slots for the rest repeat
a triple rather than sitting empty, so a table read alone does not say which
world it belongs to. What pins it is the six screen lists and five object
lists already measured by playing to them: the tables reproduce every one of
them, at the index the world and level number give, and the level's own first
screen is six bytes past the entry every time. Two tests keep that control in
place.

That is enough to name the rest. Bank 1 is the only bank whose tables hold two
distinct triples, so it carries two worlds: World 2 at indices 3 to 5, which is
measured, and a second at 9 to 11, which is World 4. Bank 3's table is a single
triple repeated, and World 3 is the only world left for it.

| world | bank | screen lists | object lists |
| --- | --- | --- | --- |
| 1 | 2 | `0x0A192` `0x0A1B7` `0x0A1DA` | `0x0A002` `0x0A073` `0x0A0FE` |
| 2 | 1 | `0x055BB` `0x055E2` `0x05605` | `0x05179` `0x05222` `0x0529B` |
| 3 | 3 | `0x0D03F` `0x0D074` `0x0D09B` | `0x0CE74` `0x0CF1D` `0x0CFD8` |
| 4 | 1 | `0x05630` `0x05665` `0x05694` | `0x05311` `0x05405` `0x054D5` |

Worlds 3 and 4 have not been played to. Their entries decode, sort and
terminate like the measured ones, and the world numbering is an inference from
where the distinct triples sit.

The thirteenth screen pointer, at `0x18`, sits one past the twelve levels.
Bank 2's (`0x6190`) is the title screen, which is where the title screen's
tilemap is decoded from. Bank 1's (`0x55BB`) is its own first level's entry
over again, the same filler its unused level slots use, so it holds nothing
new. Bank 3's (`0x50C0`) is a list of four pointers to one screen: a corridor
walled across the top two rows and the bottom three and open for the eleven
between, with no coins in it, stored immediately after World 3-3's list. No
level list mentions that screen anywhere. What the game shows it for is still
open.

## The pointers before a level

Every screen list in the ROM starts three pointers before the level's own
first screen. What they were for was open from the moment the first list was
found. (World 1-1 looked like it had four. The scan finds lists by structure
and one extra pointer before 1-1 happens to decode; the bank header points at
the same three-pointer prefix every other level has.)

They are the level's bonus rooms, the coin chambers the raised exit door
leads to. Decoding the last two of them shows closed boxes with a solid floor
across their whole width, filled with coins, where a level's own screens have
gaps in the ground. That pair of properties separates all twelve bonus rooms
from all six opening screens, so `sml bonus-rooms <level>` draws them and
`level::is_bonus_room` tests for one.

A left wall is not part of the rule. Most of the rooms have one; World 2-3's
is an underwater chamber walled on both sides and open across the top two
rows, and requiring the wall threw it out.

A level with one bonus room stores the same pointer twice: World 2-3 has
`0x6327` in both places.

The pointer before those two is one of two things, and never a third room.
In worlds 1 and 2 it is the world's own opening screen: all three of World 1's
levels carry `0x62BE`, which is 1-1's first screen, and all three of World 2's
carry `0x56CD`, which is 2-1's. In worlds 3 and 4 it repeats one of the
level's own rooms instead: World 4's levels carry `0x6C81`, which is also
their second pointer, and World 3's carry `0x56A5` or `0x5CC2`, which are its
two rooms. Decoding it settles it either way, since the opening screens fail
the room test and the repeats pass it.

Worlds 3 and 4 also share one prefix across a whole world, so all three levels
of World 4 have the same two chambers. `level::prefix_rooms` reads all three
pointers, keeps the ones that decode to rooms and drops repeats, which gives
two rooms for every level except World 2-3's single one.

## A world loads its own tiles

A level's tile ids index whatever is in video RAM while it is playing, and
that is not one atlas for the whole cartridge. World 2's geometry decodes
exactly, every one of 2-1's 320 columns matching the running game, and renders
as garbage through World 1's tiles. Same failure as the title screen once had:
data that decodes without error and draws the wrong picture.

Reading video RAM at 2-1's opening (`tools/find_gameplay_tile_blocks.py 2-1`)
shows most of the atlas shared with World 1 and four spans replaced:

| rom | vram | size |
|---|---|---|
| `0x04032` | `0x8A00` | `0x03C0` |
| `0x04432` | `0x9340` | `0x0100` |
| `0x04572` | `0x9480` | `0x0280` |
| `0x09732` | `0x9700` | `0x0100` |

Three of the four come from `0x04032` onward, which is bank 1 plus `0x32`.
World 1's tiles are at `0x08032`, bank 2 plus `0x32`. So a world's own tiles
sit at the same offset into whichever bank holds its levels.

Measured on 2-1 and 2-2, which load byte-identical block layouts, so it is per
world rather than per level. 2-3 is not measured (the run to reach it kills the
machine), but it renders as a coherent underwater scene through the same
overlay.

### The cartridge's own copy tables

The search above cannot see worlds 3 and 4 without playing to them, and it
cannot see everything even where it does run: it works in `0x40`-byte chunks
and drops any chunk that is uniform or that turns up more than once in the ROM,
so its picture of the overlay has gaps.

The loader that does the copying is in bank 0 at `0x0D8B`. It takes the world
number, subtracts two, doubles it, and indexes two tables of pointers; each
copy then runs until the destination reaches a fixed address, which is where
the sizes come from.

| table | world 2 | world 3 | world 4 | destination | size |
|---|---|---|---|---|---|
| `0x0DED` | `0x4032` | `0x4032` | `0x47F2` | `0x8A00` | `0x03D0` |
| `0x0DF3` | `0x4402` | `0x4402` | `0x4BC2` | `0x9310` | `0x03F0` |

Two copies, not four blocks. The source is a bank-window address read with the
world's own bank switched in, which is why worlds 2 and 3 share an entry and
land on different data. World 1 is not in the tables at all: the shared atlas
is World 1's own.

Every measured World 2 span sits inside one of these two copies at the same
ROM-to-VRAM delta, and the fourth measured span (`0x09732` to `0x9700`) turns
out to be the shared atlas at that address rather than an overlay. So the two
readings agree wherever the search had anything to say, and the tables fill in
the rest. `level::tile_overlay` reads them and `level::tiles_for_level` applies
them.

That makes worlds 3 and 4 render, and what they draw is a check on the world
numbering the bank header gave: 3-1 opens on stone heads under clouds and 4-1
on bamboo stalks under a Chinese key pattern, which are Easton and Chai, the
third and fourth worlds of Super Mario Land. Reading the tables wrong, or
naming the worlds wrong, would not produce either picture.

### One tile per world is animated

Both tile loaders end with the same eight-byte copy the other way round, out
of the tile data they just wrote and into `0xC600`, reading every other byte.
The per-world loader takes them from its second source plus `0x2C1`; the
shared loader at `0x005F0` takes them from bank 2's `0x5603`. Both addresses
land on the same place in video RAM, `0x95D1`, which is the high bitplane of
background tile `0x5D`.

The routine at `0x02416` writes eight bytes back into `0x95D1`. It runs every
eight frames (`0xFFAC & 7`), and bit 3 of the same counter picks where the
bytes come from: `0xC600`, so the tile the world loaded with, or a table at
`0x3FC4` indexed by the world number out of the high nibble of `0xFFB4`. So
tile `0x5D` has two frames and holds each for eight frames.

What pins the table's base and its index is World 2's pair. Its second frame
is its first shifted down a row with every row rotated right two pixels, which
is a water surface flowing:

```
loaded            second frame
#.....##          ........
##...##.          ###.....
.##.##.#          #.##...#
########          .#.##.##
########          ########
```

That tile is World 2's water line: it fills the bottom row of 2-1 and 2-2 from
the first column to the last screen (which is dry ground with the exit door on
it) and the top row of 2-3 for all 360 columns, since 2-3 plays underwater end
to end. World 4 uses it along the bottom of 4-2 and Worlds 1 and 3 use it as
scenery, 25 cells in 1-3 and 1200 through the cave in 3-2.

`level::animation_frames` reads both frames for a world.

One trap worth keeping: background tiles use the signed addressing mode, so an
id below 128 reads from `0x9000` and the rest from `0x8800`. Computing
`0x8000 + id * 16` looks right and points at the wrong half of video RAM,
which made a test that should have caught an unused overlay pass by accident.
