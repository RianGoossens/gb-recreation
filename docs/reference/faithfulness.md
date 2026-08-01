# Faithfulness audit

The end goal is a faithful recreation of Super Mario Land, easy to modify. This
file tracks how close each implemented piece is to the cartridge, so deviations
are visible and deliberate rather than accidental. Three labels:

- **canonical**: in the original game.
- **stand-in**: an equivalent we built before pinning the exact original, to be
  replaced or confirmed against the cartridge.
- **invented**: not in the original. Fine as an optional mod, but not end-goal
  content. Flagged for a decision.

Decision (Rian, 2026-07-23): invented pieces can stay in the codebase during
development. They must not ship in the final faithful build. Before release,
either remove them or gate them behind an explicit opt-in so the default game
matches the cartridge.

**Done, for the two spawnable ones.** `Level::without_non_canonical` drops the
star and the Fly, and `run` and `play` apply it unless given
`--allow-non-canonical`. The level format still parses both markers, since a
level file is data and refusing to load one would be a worse failure than
playing it faithfully.

## Power-ups and states

| Item / state | Label | Notes |
|--------------|-------|-------|
| Small Mario | canonical | |
| Super mushroom, big Mario | canonical | |
| Superball flower, fire Mario | canonical | SML's signature power-up |
| Superball projectile | canonical | thrown by fire Mario; bounces; collects coins |
| Invincibility star | **invented** | Super Mario Land has NO star. Gated: dropped from any level played by `run`/`play` unless `--allow-non-canonical` is given. |

## Enemies

| Enemy | Label | Notes |
|-------|-------|-------|
| Goomba (walker) | stand-in | SML's ground walker is the Chibibo. Ours behaves like it (walk, turn at walls). Its movement is now measured, on object kind `0x00`, the one World 1-1's list uses ten times: one pixel left every three frames without exception over 143 steps (`tools/measure_enemy_walk.py`), and it walks off a ledge rather than turning, falling straight down at a flat pixel per frame with its X frozen (`tools/probe_enemy_ledge.py`). The ledge-turn our engine used to do was carried over from another game and is gone. Still unverified: the mapping of kind `0x00` to this entity, and its sprite. |
| Ledge-turning walker | canonical behaviour, unnamed | Object kind `0x04`. Same walk speed as kind `0x00`, and it turns at a ledge where the other steps off. Both were settled by writing a wall and then a pit into the tilemap in front of them (`tools/probe_walker_turn.py`). World 1-1 has one, 1-2 has four. Which SML enemy it is remains unnamed. |
| Jumper | canonical behaviour, unnamed | Object kind `0x0E`. Traced with the camera held still (`tools/trace_jumper.py`): a 102-frame cycle, 54 frames standing perfectly still and then 16 position updates three frames apart, each 2 px sideways and a rise from a fixed table `[4, 4, 2, 2, 1, 1, 1, 0, 0, -1, -1, -1, -2, -2, -4, -4]`, giving a 32 px hop 15 px high. The table is not constant deceleration, so it is a table in the cartridge and a table in `src/core/enemy.rs`. It turns at a wall and hops off a ledge, both settled against a control run with no obstacle, which a hopper needs because it reverses and rises and falls on its own. World 1-1 has three, 1-2 has none, 1-3 has none. Which SML enemy it is remains unnamed. |
| World 1-3's vertical mover | measured, **correctly absent** | Object kind `0x02`. X frozen; Y runs 16 px down at a pixel a frame, waits 62 frames, runs 16 px back up, waits 106, on a 200-frame period (`tools/measure_level_kind.py`). It never costs Mario a life at any overlap through two full cycles, and it does not hold him up either: with the tilemap carved away he drops straight past it. Neither an enemy nor a platform, so leaving it out of the level is right rather than a gap. What it is for is open. |
| Faller | canonical behaviour, unnamed | Object kind `0x0C`, the only kind World 1-3 puts in our level. X frozen; holds position for 175 frames from the frame the game creates it, then falls at exactly one pixel a frame. The 175 is the same with Mario directly under it, a screen away, and at the left edge (`tools/probe_faller_trigger.py`), so it is a timer. Its contact sweep matches a walker's exactly, so it hurts from every side but a stomp. `D` in the level format; 1-3 gets 6 of its 8 records, the other two starting inside solid tiles the text format cannot represent. Unmeasured: whether it stops on a floor or falls through. Ours lands. |
| Fly (hopper) | **invented / stand-in** | A generic hopping enemy, not a specific SML enemy. SML World 1 (Birabuto) has the Nokobon (a walking bomb). Gated the same way as the star. Still worth replacing with a real SML enemy rather than only hiding it. |

## Open discrepancies

| Piece | Notes |
|-------|-------|
| The lift's own width | Small Mario measures 11 px wide, and a 16 px lift under an 11 px Mario should hold him across 26 px of his X. The measured support window is 29. The 3 px belongs to the lift, either because its surface is wider than its two sprites or because support has a pixel of slack each side. `src/core/lift.rs` still uses 16. |
| Big Mario's box | 11 x 16, of which only the width is measured. No run has reached a mushroom on the cartridge, so the height is still twice the small sprite's tile height rather than a number off the game. |

## Items, blocks, scoring

| Piece | Label | Notes |
|-------|-------|-------|
| Coins, 100-coin 1up | canonical | |
| Question block gives a coin | canonical | |
| Power block gives mushroom/flower by size | canonical | matches SML's size-based item |
| Brick block | canonical | big/fire Mario breaks it; small Mario bumps it |
| Score, lives, timer, time-out death | canonical | |
| Stomp worth 100 points | canonical | measured off the cartridge's status bar (`tools/measure_scores.py`) |
| Coin worth 100 points | canonical | measured the same way. The cartridge bumps the coin counter one frame before the score |
| Power-up worth 1000 points | **stand-in** | unmeasured; no run has reached a power-up on the cartridge yet |

## Physics and levels

- **Walking** (accel, friction, max walk speed): canonical, measured from the
  cartridge. See `docs/reference/physics.md` for the observation method
  (`tools/find_mario_speed.py`).
- **Gravity, jump**: canonical, measured and implemented. The cartridge runs
  a three-regime state machine (near-constant speed while rising with the
  jump button held, capped at a fixed frame count; real deceleration if
  released before that cap; real acceleration while falling), not one
  continuous `GRAVITY` acceleration, and `step_motion` in
  `src/core/physics.rs` now models it that way (`apply_vertical_accel`).
  Not pixel-perfect: the fitted constants were checked by simulating a
  full jump end to end and comparing against the traced arc (about 26px
  peak, landing around frame 25, versus the real 24-25px over ~24 frames),
  not derived and shipped untested. See `docs/reference/physics.md` for the
  fitted values, traces, and the correction made while implementing it.
- **Stomp bounce**: canonical, measured and implemented. 61 real stomps on
  the first World 1-1 Chibibo (`tools/measure_stomp_bounce.py`) give an 8px
  rise over about 12.4 frames, or 9px over 13.5 with the jump button held.
  A bounce decays like a released jump in both cases, not like a held rise,
  so `Mario` carries a `bouncing` flag. The shipped `STOMP_BOUNCE` (360) is
  the speed that reproduces that traced arc against the engine's `JUMP_CUT`;
  the raw `2d/t` reading (333) only reached 7px when simulated. See
  `docs/reference/physics.md`.
- **When enemies appear**: canonical. The cartridge streams objects in with one
  forward read pointer, and an object arrives with a slot X of `0xBF`, which is
  183 pixels right of the camera. `Game` does the same: spawns wait in a list
  and fire when the camera reaches them, and a fired record never comes back.
  Before this, every enemy was created at level start and the ones past the
  first screen were deleted on frame one, so World 1-1 played with 1 of its 14
  and 1-2 and 1-3 with none of theirs.
- **World 1-3's end trigger**: **stand-in**, and now at least a reachable one.
  1-3 has no exit door, since it ends the world rather than leading anywhere,
  so its trigger is placed by rule. The old rule (two columns from the right
  edge at the ground row) put it at column 298 row 13, a one-tile pocket walled
  in on all four sides, so the level could not be finished. It now goes on the
  rightmost cell with two free rows, solid underneath, and the same true of the
  column to its left, which is column 296. A flood fill from the spawn to the
  trigger is a test for all three levels. Where the cartridge actually ends 1-3
  is still unmeasured.
- **Levels**: World 1's three levels are extracted from the cartridge and are
  what `run` and `play` load by default when they have been generated
  (`sml extract-level 1-1|1-2|1-3`). The demo level, the example level and the
  demo campaign remain as test fixtures and as the fallback on a checkout
  without the ROM. Shipping invented levels is not a goal (see the end-goal
  note in CLAUDE.md).
- **Mario's collision box**: canonical for small Mario, 11 x 12, measured by
  walling him into corridors of seven widths and putting a ceiling over him at
  three heights (`tools/measure_mario_box.py`). Every trial agrees. The
  cartridge draws him 16 px across from two sprites, so the box sits inside
  what is on screen. This replaced an 8 x 8 box that was never measured.
- **World 1 geometry**: canonical, and verified end to end. Every column of
  all three levels (300, 280, 300) matches the running cartridge exactly.
- **Lifts**: canonical, measured. World 1-1's kinds `0x0A` and `0x0B` carry
  Mario (`tools/probe_lift.py`) and run one pixel every two frames on a single
  axis, reversing every 120 frames over 60 pixels and every 106 over 53
  (`tools/measure_enemy_walk.py`). `src/core/lift.rs` implements that, and
  `sml extract-level` writes them into the level file as `V` and `H`: 2 in
  World 1-1, 7 in 1-2, none in 1-3. One inference is stated rather than
  measured: 1-2's `0x0A` and `0x0B` records are assumed to be the same objects
  as 1-1's, on the grounds that the kind byte selects the same code. The text
  level format is a tile grid, so an object placed at `16x + 8` lands on the
  column containing it and the half-tile offset is lost.
- **World 1 enemies**: canonical in placement, **incomplete**. The object
  lists decode (`docs/reference/objects.md`) and `sml extract-level` writes out
  the kinds whose movement has been measured: the two ground walkers and the
  jumper, giving 14 enemies in 1-1, 8 in 1-2, none in 1-3. World 1-3's own two
  kinds now have their motion measured but not what they do on contact, so they
  stay out; the level carrying no enemies is a known gap rather than an
  oversight. So a
  level is short of enemies rather than carrying invented ones. Which kind byte
  is which named SML enemy is also open; `tools/capture_object_sprites.py`
  draws each one, which tells them apart without naming them.
- **Level-end trigger placement**: canonical for 1-1 and 1-2, **stand-in** for
  1-3. The exit is a 2x2 door, `0x13 0x21` over `0x24 0x39`, and it appears
  exactly twice per level in the same column: a raised one leading to the
  bonus route and one at ground level. The extractor puts the trigger on the
  lower one. The rule it replaces (two columns from the right edge) landed on
  the door in both 1-1 and 1-2 by coincidence. World 1-3 has no door, because
  it ends the world instead of leading to another level, so its trigger still
  goes at the far end.

- **Tile solidity**: **faithful, measured from the cartridge.** The shipped
  rule is `tile >= 0x60`, with `0xF4` carved out as passable.

  Super Mario Land tests collision against the background tilemap in video
  RAM, so `tools/probe_solidity.py` writes each of the 256 tile ids into the
  tilemap in front of Mario and lets the game's own collision code answer.
  Every id is covered, not only the ones this level contains.

  | ids | verdict |
  |-----|---------|
  | `0x00`-`0x5F` | pass through, no support |
  | `0xF4` | passes through: it is a coin |
  | `0x68`, `0x69`, `0x6A`, `0x7C` | support from above, no sideways block |
  | everything else `>= 0x60` | solid |

  The four semi-solid ids are one-way platforms in the engine now, written
  `^` in the level format: Mario lands on one from above and walks or jumps
  straight through it otherwise. World 1-1 contains none of them, so its
  extracted file is unchanged. The second screen list in the ROM (the
  candidate World 1-2) contains 183 of them, laid out as horizontal runs like
  `104 105 105 105 106`, a left cap, a repeated middle and a right cap, with
  tile 55 hanging beneath as a support. That shape is what a platform looks
  like, and it agrees with what the probe measured. It is still untested
  against the running game in a real level, because 1-2 has not been reached
  in the emulator yet.

  `0xF4` is a coin, not decoration. Coins are drawn in the background tilemap
  rather than spawned from the object table, so they come straight out of the
  geometry decode: `tools/find_coin_tile.py` plays until the coin counter
  moves and reads which tilemap cell changed on that frame. World 1-1 has 18,
  including a tower of seven in one column, which an earlier note here called
  a bar of decoration floating in open sky. They are extracted as `C` markers
  and the level now has its real coins.

  Two earlier readings of this were wrong and shipped. The first was an
  allow-list of `{96}` plus two invented structural rules, which Rian caught
  by playing the cartridge: far too few pipes, far too few blocks, and none
  of the holes you can fall through (it produced a level with zero pits, on a
  level that has nine). The second, `0x60 <= tile <= 0xE8`, gives the right
  grid for this level by accident: `0xE8` is not a boundary, and 36 of the
  level's 43 ids were decided by a fit to the other 7. See
  `docs/reference/level-1-1.md`.

## Rendering

| Piece | Label | Notes |
|-------|-------|-------|
| World 1 level graphics | canonical | drawn from the ROM's own tile data (one block, `0x08032` to VRAM `0x8000`), scored at 99.60% of playfield pixels against the emulator's first frame; the only difference is Mario's sprite, which the background renderer does not draw (`tools/compare_level_render.py`) |
| Level background in the game loop | canonical | the game itself now draws every one of the twelve levels with the cartridge's own tiles, its world's overlay included. A level written by hand carries no graphics and keeps the placeholder blocks |
| Where the playfield sits | canonical | the level is drawn in the 128 pixels below the status bar, not behind it. The camera's window is 128 tall for the same reason, so a level taller than the cartridge's sixteen rows scrolls vertically instead of losing its bottom |
| World 1-1's opening frame | canonical | our whole 160x144 frame matches the emulator capture of the same frame in every 8x8 cell except the two Mario stands in, where the cartridge draws his sprite and we draw a block (99.83% of pixels) |
| The animated background tile | canonical | tile `0x5D` alternates between two pictures every eight frames, the cadence the routine at `0x02416` runs on. In World 2 it is the water line |
| Status bar | canonical | the cartridge's own font, ids and layout, read off the World 1-1 capture and checked pixel for pixel against it (`src/hud.rs`). A level with no cartridge graphics falls back to the invented 3x5 font in `src/font.rs` |
| Coins and the exit door | canonical | a coin is the cartridge's tile `0xF4` from the world's own sheet; the door is part of the level's background and the engine draws no marker over it |
| Mario, enemies, items, blocks | **stand-in** | still drawn as flat blocks. The sprite tiles are in the ROM (`0x08032` to VRAM `0x8000`, visibly Mario's frames and the enemy set), but which tile ids each entity uses and in what order has not been pinned |

## Sound

| Piece | Label | Notes |
|-------|-------|-------|
| Sound event model (`SoundEvent`, emitted by `Game`) | canonical | the game marks the same moments the cartridge would play a sound; see `src/sound.rs`. |
| Tone playback (`sml::frontend::tone_for`, `src/audio.rs`, `gui` feature) | **stand-in** | each event plays an invented square-wave beep (frequency and duration picked for variety, not read from the APU). The cartridge's actual sound effect data (note sequences, duty cycles) has not been extracted. Replace with the real APU data once pinned. |

## Recommended next steps toward faithfulness

1. Extract the real level geometry.
2. Replace the Fly with a real SML enemy (Nokobon). Gating it keeps the default build faithful, but the enemy roster is still one short.
3. Pin the cartridge's real sound effect data (APU registers/note data per event) and replace the invented tones in `src/audio.rs`.
4. Measure the power-up's point value, the only scoring number still
   unchecked. Stomps and coins are pinned at 100 each.

(Brick breaking and superball coin collection are already canonical, done.)

## Worlds 2 to 4 are decoded but not playable

World 2's three levels decode from the cartridge and `sml extract-level 2-1`
writes them, but they cannot be finished:

- **No swimming.** 2-1 and 2-2 both open into water at world columns 60 to 79
  (both lists point at screen `0x5D32`, which has no floor), and the engine has
  no swimming, so a walker falls out of the level there. 2-3 is the same world's
  third level and gets to column 178.
- **Its enemies are in, minus the swimmers.** The row question turned out to be
  confined to Honen and Yurarin Boo, whose records all carry the same row byte
  and which place themselves below the screen, so neither is written to a level
  file. Every other World 2 record of a kind the engine implements lands on a
  cell that is not solid, the check World 1 passes, and those are extracted:
  2-1 gets 10 walkers and 4 lifts, 2-2 gets 14 and 5. 2-3 has none of the
  implemented kinds at all, so it stays empty.

World 2 cannot be finished without swimming, so it is not wired into the
default campaign, which stays World 1.

Worlds 3 and 4 arrived later, from the bank header rather than from playing,
and stand in the same place with more caveats:

- **Nobody has played there.** Their screen lists, object lists and tile
  overlays all come from tables in the ROM. The tables are checked against the
  eleven entries that were measured, and each list's records run to the end of
  the level they are paired with, but the levels themselves have never been
  compared column by column against the running cartridge the way World 1's
  and World 2's were.
- **World 3 and 4 numbering is an inference.** Bank 1 is the only bank whose
  tables hold two distinct triples, which makes its second triple World 4, and
  World 3 is what is left for bank 3. The graphics agree (3-1 opens on stone
  heads, 4-1 on bamboo, which are Easton and Chai) but that is corroboration,
  not a measurement.
- **Their enemies are extracted, unwatched.** The kinds the engine implements
  are written into the level files now that the row question is settled and
  every one of their records is checked to land in open space. Nothing has seen
  them move there, so what is being trusted is that a kind byte selects the same
  code in World 3 that it selects in World 1. The boss ids corroborate the
  pairing: King Totomesu, Dragonzamasu, Hiyoihoi and Biokinton each appear
  exactly once in the cartridge, in the third level of their own world.
- **World 4-3 is a vehicle stage.** It has no ground under the spawn, no exit
  door, and 4 of its 480 columns are solid. The engine has no aeroplane, so
  nothing in it can be played at all.

Only World 1 is wired into the default campaign.
