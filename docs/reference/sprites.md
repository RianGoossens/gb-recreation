# The object atlas

Where the cartridge keeps the pictures of everything that moves, and how to
read them.

## It was already in the tree

A level's tile copy puts ROM `0x08032` onward into video RAM `0x8000` through
`0x9800`, all of it, in one run (`docs/reference/level-format.md`). The
background reads from `0x8800` up. Everything below `0x9000` is the object
atlas, so the sprite graphics have been extracted for as long as the background
ones have. What was missing was which tiles are which object.

Part of it is per world. Comparing the four worlds' atlases tile for tile
leaves exactly ids `0xA0` through `0xDC` differing, 61 of the 256, which is
where a world's own enemies are drawn from. Everything else, Mario included, is
the same in all four. `sml sprites <world> <out.png>` writes a world's atlas
out as a picture.

## A frame is a block, not a run

The atlas is stored as a picture 16 tiles wide. A character frame is two tiles
across and two down, 16 by 16 pixels, so its four tile ids are `n`, `n + 1`,
`n + 16`, `n + 17`.

Reading the same bytes as four consecutive ids draws a scramble. That is how
the arrangement was settled: only one of the two readings produces figures.

Mario is at the start of the atlas. The first two rows are small Mario and the
next two are big Mario, eight blocks across each. Six of the eight hold Mario
at all, and five of those six are identified:

| pose | small | big |
|---|---|---|
| still, and walk 1 | 0, 1, 16, 17 | 32, 33, 48, 49 |
| walk 2 | 2, 3, 18, 19 | 34, 35, 50, 51 |
| walk 3 | 4, 5, 20, 21 | 36, 37, 52, 53 |
| jump | 8, 9, 24, 25 | 40, 41, 56, 57 |
| skid | 10, 11, 26, 27 | 42, 43, 58, 59 |

Which block plays when was traced on the running game
(`tools/trace_mario_frames.py`), by reading the tile of the sprite the game
drew at Mario's own position every frame alongside the state bytes that could
explain the choice:

- Standing still is the first block, held indefinitely.
- A walk cycles the first three blocks in order, about four frames each. The
  standing block is part of the cycle rather than separate from it.
- Every airborne frame is the fourth-across block, through both the rising and
  falling values of the cartridge's phase byte, so there is one jump pose and
  it does not change on the way down. Its ink is 14 wide against a walk
  frame's 10, which is his arms.
- Facing left draws the same blocks with the hardware's flip bit, which shows
  up in the trace as the top left sprite becoming the block's right tile.
- Pressing the opposite direction while still moving draws the sixth block,
  the skid. The trace caught it twice, at the two moments the input reversed
  during a run, both times for exactly seven frames: once as tile `0x0A`
  unflipped after pressing left while moving right, and once mirrored after
  the opposite. It is drawn facing the way he is still travelling rather than
  the way he is now pressing, which is the one pose whose flip does not follow
  the input.
- The rate does not change with his speed. Every stretch of the trace holds
  each block for exactly four frames, walking and with B held, over 28, 33 and
  9 changes of pose.

Two of the eight blocks in each row are not Mario. Rendering the atlas shows
the seventh holding a figure with dark hair in a robe, and the eighth a black
ball beside a partial shape in the small row and a side-view vehicle in the
big one. They also turn up in the trace as tiles `0x0F` and `0x1F` drawn near
Mario, which is what gave them away as separate objects sharing his rows.
Which characters they are is a reading of a picture and nothing has confirmed
it, so they are left unnamed.

The third block is the one Mario pose still unaccounted for. Nothing in the
trace draws it: standing, walking, running, jumping, skidding and dying all
land elsewhere. Swimming is the obvious candidate, since World 2 is the only
thing in the game that has never been played here, and that is a guess.

## Why these tiles are Mario

Not because they look like him. Mario's collision box was measured on the
cartridge by walling him into corridors of known width and subtracting
(`tools/measure_mario_box.py`), a route with nothing to do with graphics, and
came out 11 wide by 12 tall.

Small Mario's still frame draws ink over exactly 10 by 12 pixels of its block.
The heights agree, and a collision box a pixel wider than the drawing is
ordinary. Big Mario's is 16 tall, which is the difference between the sizes.

The ink also sits on the bottom edge of the block, `y` 4 through 15, so a
frame lines up with a position given as feet on the ground with no offset to
guess at. Horizontally it starts at `x` 3.

## Which tiles each object is drawn from

Measured on the running cartridge with `tools/measure_object_sprites.py`.
For every slot holding a kind, it collects the OAM entries the game placed
near the slot and reports them as offsets from the slot's own position.
Objects with a neighbour inside the window are skipped, and so are objects
close to Mario, who is drawn from this same atlas and was collected as if he
were kind `0x23`'s own tiles on the first attempt.

The game runs in 8x8 sprite mode, so an OAM tile id is an atlas id directly.

| kind | name | tiles | size |
|---|---|---|---|
| `0x00` | Chibibo | `0x90` | 8x8 |
| `0x02` | Pakkun Flower | `0x92` over `0x93` | 8x16 |
| `0x04` | Nokobon | `0x96` over `0x97` | 8x16 |
| `0x08` | King Totomesu | `0xCD` `0xCE` / `0xAB` `0xC6` `0xC7` / `0xBB` `0xD6` `0xD7` | 24x24 |
| `0x0A` `0x0B` | lifts | `0xEF` three times | 24x8 |
| `0x0C` | Falling Slab | `0xDD` `0xDE` | 16x8 |
| `0x0E` | Fly | `0xA0` `0xA1` / `0xB0` `0xB1` | 16x16 |
| `0x10` | Honen | `0xC1` over `0xD1` | 8x16 |
| `0x1E` | fire breath | `0xD4` `0xD5` | 16x8 |
| `0x23` | fireball | `0xE2` | 8x8 |
| `0x24` | Yurarin Boo | `0xA6` `0xA7` / `0xB6` `0xB7` | 16x16 |
| `0x27` | explosion | `0xCD` `0xCE` / `0xCA` `0xCB` `0xCC` / `0xDA` `0xDB` `0xDC` | 24x24 |
| `0x36` | drop block | `0xEE` | 8x8 |
| `0x3F` | Gao | `0xA4` `0xA5` / `0xB4` `0xB5` | 16x16 |
| `0x42` | Bunbun | `0xC2` `0xC3` / `0xD2` `0xD3` | 16x16 |
| `0x45` | arrow | `0xAC` over `0xBC` | 8x16 |

Every kind the table gives at 16x16 uses the same four ids relative to its
first: `n`, `n + 1`, `n + 16`, `n + 17`. The Fly is `0xA0`, Yurarin Boo `0xA6`,
Gao `0xA4`, Bunbun `0xC2`.

The Fly's four ids are `n`, `n + 1`, `n + 16`, `n + 17`, the same 2x2 block
Mario's frames are, which confirms the sheet's 16-tile width from the running
game rather than from looking at the picture. Not every object uses that
layout: the 8x16 kinds in the `0x9x` range stack consecutive ids instead.

The anchor is the same for every kind measured. The lowest row of the drawing
sits at the slot's y, and the left column one pixel right of its x. So an
object's position is the bottom left corner of what it draws.

### Two kinds are drawn behind the background

The OAM attribute byte came back as `0x80` for three of the sixteen kinds
measured, `0x02`, `0x10` and `0x36`, and clear for every other one. Bit 7 is the
hardware's priority bit: a sprite with it set is covered by any background
pixel that is not colour 0, so the object is hidden wherever the level's own
tiles are drawn and visible only where they are empty.

That is a partial answer to two questions the roster had left open. `0x02` is
named Pakkun Flower, a plant that lives in a pipe, and it moves 16 pixels down,
waits, and 16 back up on a 200 frame cycle: the priority bit is what lets it
disappear into the pipe rather than slide across the front of it. And `0x10`
is Honen, one of the two kinds that place themselves below the screen and swim
up in World 2.

It does not explain why `0x02` never cost Mario a life at any overlap through
two full cycles, which is still open. The other attribute bits set in the
capture (`0x02`, `0x04`) are the Game Boy Color palette and bank bits, which a
DMG ignores, so they carry nothing here.

Two things the measurement settled beyond the tiles. `OBP0`, the palette
register objects are drawn through, holds `0xE4`, which is the value the
renderer had been using as a default, so it stops being a guess.

And the lift's width, which had been 16 in the engine and disagreed with a
support window of 29 measured by dropping Mario onto one. The drawing is 24
wide, three tiles of `0xEF`, and re-sweeping that window a pixel at a time
(rather than two, which was the earlier resolution) puts it at offsets -2
through +26 of the lift's own position: exactly 29. A 24 pixel surface gives
29 for a foot of 6, and both edges of the window independently place that foot
at the same spot, centred on Mario's position. A 16 pixel surface gives 29 for
no foot width at all. So the lift is 24 and the game tests six pixels of
Mario, not his whole eleven (`src/core/lift.rs`).

## Drawing him

`Game` draws Mario from this atlas whenever the level brought cartridge
graphics, and keeps the placeholder block for a hand-written level, the same
split the background already uses.

The block is anchored to his feet: its bottom edge is the bottom of his
collision box, and it starts 3 pixels left of the box so the drawing lands
inside it. Facing left mirrors the frame, which is what the hardware's flip bit
does and why the atlas holds only right-facing Mario.

Which block plays when now comes from the trace above rather than from our own
animator, and the palette is the cartridge's `0xE4`. What is left open here is
narrow: the four unidentified blocks per row, and whether the walk's four-frame
cadence changes with his speed. Both are in
`docs/reference/faithfulness.md`.
