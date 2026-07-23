# Level format

Levels are plain text. One character is one 8x8 tile, one line is one row, and
every row must be the same width (the level is a rectangle). Save a level as a
`.txt` file and load it with the `run` or `play` commands (see below), or in
code with `Level::from_file` / `Level::from_text`.

## Markers

| Char | Meaning |
|------|---------|
| `#` | solid tile (wall, floor, ceiling) |
| `.` | empty space |
| `M` | Mario's spawn (use one) |
| `G` | a Goomba enemy |
| `C` | a coin |
| `?` | question block, gives a coin when bumped (solid) |
| `P` | power block, gives a mushroom when bumped (solid) |
| `B` | brick block (solid) |
| `E` | the level-end trigger (walk into it to finish; not solid) |

Any other character is treated as empty. The block markers (`?`, `P`, `B`) are
part of the solid world, so Mario stands on them and bumps them from below. `E`
is passable.

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

## Running a level

```sh
# play it in a window (needs the gui feature)
cargo run --features gui -- run levels/example.txt

# render a frame headlessly to a PNG (great for sharing a screenshot)
cargo run -- play shot.png 1 "" levels/example.txt
```
