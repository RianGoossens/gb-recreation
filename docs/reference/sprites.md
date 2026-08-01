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
next two are big Mario, eight blocks across each, of which the first three are
a still pose and two walking poses:

| frame | small | big |
|---|---|---|
| still | 0, 1, 16, 17 | 32, 33, 48, 49 |
| walk 1 | 2, 3, 18, 19 | 34, 35, 50, 51 |
| walk 2 | 4, 5, 20, 21 | 36, 37, 52, 53 |

What the remaining five blocks in each row are has not been settled, and which
frame the game shows at which moment is a separate question that needs the
running cartridge.

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
| `0x24` | Yurarin Boo | `0xA6` `0xA7` / `0xB6` `0xB7` | 16x16 |

The Fly's four ids are `n`, `n + 1`, `n + 16`, `n + 17`, the same 2x2 block
Mario's frames are, which confirms the sheet's 16-tile width from the running
game rather than from looking at the picture. Not every object uses that
layout: the 8x16 kinds in the `0x9x` range stack consecutive ids instead.

The anchor is the same for every kind measured. The lowest row of the drawing
sits at the slot's y, and the left column one pixel right of its x. So an
object's position is the bottom left corner of what it draws.

Two things the measurement settled beyond the tiles. `OBP0`, the palette
register objects are drawn through, holds `0xE4`, which is the value the
renderer had been using as a default, so it stops being a guess. And a lift is
drawn 24 pixels wide, against the 16 the engine collides over; the support
window measured by dropping Mario onto one is 29 pixels, which matches
neither, so the lift's true width is still open and the collision box was left
alone.

## Drawing him

`Game` draws Mario from this atlas whenever the level brought cartridge
graphics, and keeps the placeholder block for a hand-written level, the same
split the background already uses.

The block is anchored to his feet: its bottom edge is the bottom of his
collision box, and it starts 3 pixels left of the box so the drawing lands
inside it. Facing left mirrors the frame, which is what the hardware's flip bit
does and why the atlas holds only right-facing Mario.

Two things here are ours rather than the cartridge's, and are logged in
`docs/reference/faithfulness.md`:

- **Which frame plays when.** The still pose is used standing and in the air,
  and the two walking poses alternate on the existing animator's cadence. The
  cartridge's own choice, and its jump pose, need the running game.
- **The object palette.** The hardware gives objects their own palette
  registers. Their values have not been read off the cartridge, so the default
  is used.
