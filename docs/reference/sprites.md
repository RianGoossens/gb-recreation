# The object atlas

Where the cartridge keeps the pictures of everything that moves, and how to
read them.

## It was already in the tree

A level's tile copy puts ROM `0x08032` onward into video RAM `0x8000` through
`0x9800`, all of it, in one run (`docs/reference/level-format.md`). The
background reads from `0x8800` up. Everything below `0x9000` is the object
atlas, so the sprite graphics have been extracted for as long as the background
ones have. What was missing was which tiles are which object.

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
