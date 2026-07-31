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
| `J` | a jumper: still for 54 frames, then a 32 px hop 15 px high |
| `F` | a Fly enemy (walks and hops), see the note on non-cartridge content |
| `C` | a coin |
| `S` | a star (grants brief invincibility), see the note on non-cartridge content |
| `W` | a flower (makes Mario fire-powered) |
| `?` | question block, gives a coin when bumped (solid) |
| `P` | power block, gives a mushroom when bumped (solid) |
| `B` | brick block (solid) |
| `V` | a lift that runs up and down, 60 px each way |
| `H` | a lift that runs side to side, 53 px each way |
| `E` | the level-end trigger (walk into it to finish; not solid) |

Any other character is treated as empty. The block markers (`?`, `P`, `B`) are
part of the solid world, so Mario stands on them and bumps them from below. `E`
is passable.

## Content Super Mario Land does not have

Two markers spawn things the cartridge has no equivalent of: `S`, since Super
Mario Land has no invincibility star at all, and `F`, a generic hopping enemy
standing in for an SML enemy that has not been pinned yet. See
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
