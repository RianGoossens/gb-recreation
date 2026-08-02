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
star and the Bouncer, and `run` and `play` apply it unless given
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
| Goomba (walker) | stand-in | SML's ground walker is the Chibibo. Ours behaves like it (walk, turn at walls). Its movement is now measured, on object kind `0x00`, the one World 1-1's list uses ten times: one pixel left every three frames without exception over 143 steps (`tools/measure_enemy_walk.py`), and it walks off a ledge rather than turning, falling straight down at a flat pixel per frame with its X frozen (`tools/probe_enemy_ledge.py`). The ledge-turn our engine used to do was carried over from another game and is gone. The cartridge calls kind `0x00` the Chibibo and draws it from the single atlas tile `0x90`, both now pinned, so what is left unverified is only its collision box (`docs/reference/sprites.md`). |
| Nokobon (ledge-turning walker) | canonical | Object kind `0x04`. Same walk speed as kind `0x00`, and it turns at a ledge where the other steps off. Both were settled by writing a wall and then a pit into the tilemap in front of them (`tools/probe_walker_turn.py`). World 1-1 has one, 1-2 has four. Named from the disassembly's equate list and drawn from atlas tiles `0x96` over `0x97`. |
| Fly (jumper) | canonical | Object kind `0x0E`. Traced with the camera held still (`tools/trace_jumper.py`): a 102-frame cycle, 54 frames standing perfectly still and then 16 position updates three frames apart, each 2 px sideways and a rise from a fixed table `[4, 4, 2, 2, 1, 1, 1, 0, 0, -1, -1, -1, -2, -2, -4, -4]`, giving a 32 px hop 15 px high. The table is not constant deceleration, so it is a table in the cartridge and a table in `src/core/enemy.rs`. It turns at a wall and hops off a ledge, both settled against a control run with no obstacle, which a hopper needs because it reverses and rises and falls on its own. World 1-1 has three, 1-2 has none, 1-3 has none. The cartridge calls it the Fly and draws it from atlas tiles `0xA0`, `0xA1`, `0xB0`, `0xB1`. |
| World 1-3's vertical mover | measured, **correctly absent** | Object kind `0x02`. X frozen; Y runs 16 px down at a pixel a frame, waits 62 frames, runs 16 px back up, waits 106, on a 200-frame period (`tools/measure_level_kind.py`). It never costs Mario a life at any overlap through two full cycles, and it does not hold him up either: with the tilemap carved away he drops straight past it. Neither an enemy nor a platform, so leaving it out of the level is right rather than a gap. What it is for is open. |
| Falling Slab (faller) | canonical | Object kind `0x0C`, the only kind World 1-3 puts in our level. X frozen; holds position for 175 frames from the frame the game creates it, then falls at exactly one pixel a frame. The 175 is the same with Mario directly under it, a screen away, and at the left edge (`tools/probe_faller_trigger.py`), so it is a timer. Its contact sweep matches a walker's exactly, so it hurts from every side but a stomp. `D` in the level format; 1-3 gets 6 of its 8 records, the other two starting inside solid tiles the text format cannot represent. Drawn from atlas tiles `0xDD` and `0xDE`. Unmeasured: whether it stops on a floor or falls through. Ours lands. |
| Drop block (kind `0x36`) | measured, canonical | The most-used kind in the game, 50 records across nine levels, 48 of which land in open space. It never moves until Mario stands on it, holds him at the same height a lift does, stays put for nine frames, and then descends a pixel a frame for good, carrying him. Its kind byte becomes `0x37` in the same slot on the frame he touches it: no removal and no hand-off to another slot. The surface is 8 wide, from a support window of 13 that reproduces the same 6 pixel foot the lift's 29 gave over a 24 pixel surface. Measured twice, in World 1-2 and 1-3 (`tools/probe_drop_block_support.py`). Drawn as one `0xEE` with the priority bit set. |
| Bunbun (flyer) | measured, canonical, with two gaps | Object kind `0x42`, and World 1-2 has more of it (19 records, 9 in normal play) than any other kind in any World 1 level. Traced with the camera held still (`tools/measure_flyer.py`): it flies left at a pixel a frame for 41 frames, holds still for 33, and repeats, never reversing and never changing height across four full cycles. Mario pinned high, pinned low, or moved to its far side gives the same trace frame for frame, and two later instances repeat the cadence. Its contact sweep is a walker's (`tools/probe_object_contact.py`): hurts from every side, a stomp kills it. `N` in the level format. Two gaps. Whether terrain stops it is unmeasured, because nothing solid sat on the rows it was traced along, and ours flies through. And whether terrain stops it is the only gap left on it: its tiles were measured later (`0xC2` `0xC3` / `0xD2` `0xD3`) and it draws from the cartridge. |
| Gao | measured movement, canonical, with two gaps | Object kind `0x3F`: nine records in World 1-3 (three in normal play) and three in World 4-2. Traced with the camera frozen (`tools/measure_level_kind.py`) it does not move a pixel on either axis in 700 frames, and ours stands still. Its tiles are the cartridge's, measured on the running game. Two gaps. It spits: a kind `0x23` fireball appears 4 px to its left every 137 frames, lives 117 of them, and flies up and left at a pixel a frame across and one every two frames up (`tools/watch_kind_neighbours.py`), and ours does that, but every fireball left the screen 20 frames before the next appeared, so a fixed 137 frame timer and a 20 frame wait after the last one is gone fit the trace equally and ours counts. Gao's own contact is measured now and is a walker's: the six-line sweep in World 1-3 matches the positive control exactly, hurt at every overlap and stomped clean with feet on top (`tools/probe_object_contact.py`), which is what ours already did. The fireball's contact is still unmeasured and uses the same shared path. |
| King Totomesu (kind `0x08`) | measured, canonical, with three gaps | World 1-3's boss, one record, eight columns from the level's right edge at column 292 row 11. Traced with the camera frozen (`tools/measure_level_kind.py`): its x never moves in 700 frames and its y runs 20 px up at a pixel every two frames, 20 back down, then holds still for the rest of a 162-frame cycle, which is what ours does. Its contact is the one thing separating it from every other kind measured: swept the same way, it costs Mario a life at +10, feet on top, where every other enemy is stomped harmlessly (`tools/probe_object_contact.py`), so ours cannot be stomped. `K` in the level format. Three gaps. What defeats one is not measured and ours cannot be defeated at all. What finishing the fight does to the level is not measured either. Its tiles are the cartridge's now, nine sprites 32 by 24 measured with Mario parked at the far side of the screen (`tools/measure_boss_sprite.py`), but only one of its two drawings is used: a second shares the head and replaces the other five tiles, on 32 frames of 200, and what selects between them is unmeasured, so ours never changes. Its contact box is the shared 5x5, which is measured on a Chibibo and not on this. It breathes fire: two shots a leap, one at the top and one while it stands, 56 and 106 frames apart (`tools/measure_boss_fire.py`, `tools/watch_kind_neighbours.py`), each leaving its mouth 4 px to its left on a line 7 px above its own position. |
| King Totomesu's fire (kind `0x1E`) | measured, canonical, with one gap | Not placed by any object list; only the boss makes one. A straight horizontal line, a pixel a frame left, from screen x 109 until it leaves at 0, identical to the frame across all four of that tool's controls (`tools/measure_flyer.py`). It hurts at every offset the contact sweep tries, +10 included, with both controls passing (`tools/probe_object_contact.py 0x1E 1-3`), so ours cannot be stomped either. Drawn from `0xC4` `0xC5`, two tiles side by side. The gap: the cartridge swaps that pair for `0xD4` `0xD5` every 8 frames and uses `0xFE` in the left half on the spawn frame, and ours draws the one pose. The 3-frame `0x1B` stage the slot holds before it becomes `0x1E` is not reproduced either, and what it is has not been looked at. |
| Honen (kind `0x10`) | measured, canonical, with two gaps | 24 records across World 2-1 and 2-3, more than any other kind in World 2, and dropped at extraction until now. Traced with the camera frozen (`tools/measure_level_kind.py 2-1 0x10 700`): x frozen, and 700 frames of y compress to 8 still, 8 down at a pixel a frame, 53 down at two, 31 still, 53 up at two, 8 up at one, four times with not a frame's variation. 161 frames, 114 px, and ours replays that run-length exactly. Its record is the top of the arc. Contact took a new mode: placed once and left it read harmless at all six offsets, because it leaves that spot in two frames, and `--follow` (Mario written onto it every frame) takes a life at 0 and -4. `O` in the level format. Two gaps: the cartridge draws it behind the level's own tiles (the OAM priority bit) and ours does not model priority at all, and whether a stomp kills it is not separable from its own dive below the screen. |
| Yurarin (kind `0x1D`) | measured, canonical, with three gaps | Nine records in World 2-3, the largest kind there, and the only kind measured whose movement depends on Mario. 191 px left in 163 frames, identical across all four of `measure_flyer.py`'s controls, and one pixel every three frames towards his height, up or down, stopping within a pixel of him (`tools/probe_vertical_chase.py`). Contact takes a life at all six offsets including +10, both controls passing, so ours cannot be stomped. `Y` in the level format, and 8 of the 9 records survive the text grid (one sits in a cell the format has already given to terrain). Two gaps: something stops it at screen y 127 with Mario below that and ours has no such bound; and the cartridge's own horizontal steps come in sizes of 0 to 3 pixels in a repeating pattern, staying within about 2 px of the straight line ours draws, which nothing explains. Its tiles are the cartridge's now, a 2x2 block measured with Yurarin Boo as the control in the same run, and it swaps with a second pose about every 45 frames which ours does not animate. |
| Bouncer (hopper) | **invented / stand-in** | A generic hopping enemy, not a specific SML enemy. It was called `Fly` until the cartridge's own kind `0x0E` turned out to be named that, which made one name mean two things. SML World 1 (Birabuto) has the Nokobon (a walking bomb). Gated the same way as the star. Still worth replacing with a real SML enemy rather than only hiding it. |

## Open discrepancies

| Piece | Notes |
|-------|-------|
| Big Mario's box | 11 x 16, of which only the width is measured. No run has reached a mushroom on the cartridge, so the height is still twice the small sprite's tile height rather than a number off the game. Making the game think he already has one was tried and failed: `tools/find_power_byte.py` poked every byte of work RAM in turn and watched whether the sprite drawn at his position became one of big Mario's blocks, and none did. There is no positive control for that probe, since there is no other way to make him big, so a negative from it is weak. High RAM is unswept, because poking it hangs the emulator. |

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
  Every one of those traces was a standing jump, which
  `tools/measure_jump_height.py` later showed is the shortest of three: the
  cartridge gives 24px standing, 33 with a direction held, and 41 with B on
  top at speed. The engine reproduces the first two (26 and 35) through a
  glide that replaces the cap's abrupt stop while Mario is moving.
- **The extra 8px B adds to a jump**: **not reproduced**. B held during the
  flight takes a moving jump from 33px to 41 on the cartridge, and the engine
  gives 35 either way. `0xC20C` caps at 6 with or without B, so there is no
  measured run speed to hang it on, and what else B changes about a jump has
  not been measured. Nothing in World 1 has needed it so far.
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
- **What the terrain stops when he walks**: canonical, measured. Only the
  bottom 8 pixels of him, not the 12 he stands and jumps with. Writing a slab
  into World 1-1's own tilemap and walking him at it
  (`tools/probe_corridor_height.py`) puts the boundary exactly one tile up: in
  the 8 pixels above the floor it stops him at the column it starts in, and one
  row higher, where his head is, it does not slow him at all. A full-height
  wall in the same place is the control that says the writes landed. World
  1-3's corridor at columns 185 to 192 is one free row under a slab, which is
  why this matters: the walk through that level stopped dead at it.
- **What a ceiling stops**: canonical, measured. 5 pixels of him, centred.
  Writing a ceiling of a known width into World 1-1's own tilemap and sweeping
  a jump under it (`tools/measure_head_width.py`) gives a capped window of 12
  for one tile and 28 for three, and `ceiling + head - 1` gives 5 from both.
  This replaced his full 11 pixel width, which was never measured. World 1-3's
  opening pocket, where he is under a block by one column, is escapable with
  the measured number and was not with the assumed one.
- **A ceiling that only caps some jumps**: **not reproduced**. Ours caps any
  rise whose 5 pixel head is under a solid tile. The cartridge does that
  standing still and pressing left, and pressing right it caps nothing at all,
  measured at two ceiling widths in World 1-1 and again against World 1-3's own
  block (`tools/measure_head_width.py`, `tools/probe_ceiling_cap.py`). It is
  not the instrument: Mario's position byte stays within a pixel of where the
  probe pins it, so he is under the ceiling for the whole jump. What the
  direction selects is unknown, and until it is, capping both ways is the
  conservative choice, since it makes fewer places reachable rather than more.
- **World 1 geometry**: canonical, and verified end to end. Every column of
  all three levels (300, 280, 300) matches the running cartridge exactly.
- **Lifts**: canonical, measured. World 1-1's kinds `0x0A` and `0x0B` carry
  Mario (`tools/probe_lift.py`) and run one pixel every two frames on a single
  axis, reversing every 120 frames over 60 pixels and every 106 over 53
  (`tools/measure_enemy_walk.py`). Which way each sets off from its record's
  position is measured too (`tools/measure_lift_phase.py`): the vertical one
  down, the horizontal one left, each a full half cycle before reversing, read
  in two levels. `src/core/lift.rs` implements that, and
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
| Mario's graphics | canonical | drawn from the cartridge's object atlas. A frame is a 2x2 block of the sheet, small Mario at its start and big Mario two rows down. That these are his tiles is checked against his collision box, measured on the cartridge by a route with nothing to do with graphics: the still frame's ink is 10 by 12 against a box of 11 by 12 (`docs/reference/sprites.md`) |
| Which of Mario's frames plays when | canonical | traced on the running game (`tools/trace_mario_frames.py`). Standing is the first block, a walk cycles through three blocks including that one, airborne is a single jump pose (block 4) that does not change between rising and falling, and reversing at speed draws block 5, facing the way he is still travelling. Each block is held exactly four frames, walking and running alike, so the rate does not change with his speed |
| The object palette | canonical | read off the running cartridge: `OBP0` holds `0xE4` and every object sprite measured selects it, so the default the renderer already used is the cartridge's value (`tools/measure_object_sprites.py`) |
| Enemy graphics | canonical | the four cartridge enemy kinds the engine implements draw from the atlas at the tiles the running game placed for them: Chibibo `0x90`, Nokobon `0x96`/`0x97`, Fly `0xA0`,`0xA1`,`0xB0`,`0xB1`, Falling Slab `0xDD`/`0xDE`. `EnemyKind::Bouncer` is ours, not the cartridge's, and keeps the placeholder block |
| The lift's drawing | canonical | tile `0xEF` three times over, the same for both axes |
| What of an enemy hurts Mario | canonical | a 5 by 5 box. Holding Mario at every offset from a Chibibo a pixel at a time and watching the life counter (`tools/measure_enemy_box.py`) gives a contact window of 15 across and 16 down; his own box is 11 by 12, and both axes independently give 5 |
| An enemy's body | **stand-in, one edge measured** | 8 by 8 for every kind, which the engine has always used. It is what walks into walls and stands on floors, a separate thing from the contact box above. Walking a Chibibo into a wall written into the cartridge's own tilemap stops it with its slot x four pixels inside the wall's column, reproduced at two placements sixteen pixels apart (`tools/measure_enemy_body.py`), so that edge is three pixels inside its eight pixel drawing. The other edge needs an object walking right into a wall: kind `0x00` never turns and kind `0x04` kept losing its slot mid-run, so it is not measured, and a point test at that spot fits the one reading as well as a box does |
| The lift's width | canonical | 24 pixels, which is what it draws and what its support window says. Mario is held over 29 pixels of his own position, swept a pixel at a time, and 24 plus a 6 pixel foot is 29; a 16 pixel surface cannot give 29 for any foot width |
| Items and blocks | **stand-in** | still drawn as flat blocks. Their tiles are in the same atlas, but which ones has not been pinned |

## Sound

| Piece | Label | Notes |
|-------|-------|-------|
| Sound event model (`SoundEvent`, emitted by `Game`) | canonical | the game marks the same moments the cartridge would play a sound; see `src/sound.rs`. |
| Tone playback (`sml::frontend::tone_for`, `src/audio.rs`, `gui` feature) | **stand-in** | each event plays an invented square-wave beep (frequency and duration picked for variety, not read from the APU). The cartridge's actual sound effect data (note sequences, duty cycles) has not been extracted. Replace with the real APU data once pinned. |

## Recommended next steps toward faithfulness

1. Extract the real level geometry.
2. Replace the Bouncer with a real SML enemy (Nokobon). Gating it keeps the default build faithful, but the enemy roster is still one short.
3. Pin the cartridge's real sound effect data (APU registers/note data per event) and replace the invented tones in `src/audio.rs`.
4. Measure the power-up's point value, the only scoring number still
   unchecked. Stomps and coins are pinned at 100 each.

(Brick breaking and superball coin collection are already canonical, done.)

## Worlds 2 to 4 are decoded but not playable

World 2's three levels decode from the cartridge and `sml extract-level 2-1`
writes them, but they cannot be finished:

- **The water is crossed by lift, and there is no swimming to add.** 2-1 and
  2-2 both open into water at world columns 60 to 79 (both lists point at
  screen `0x5D32`, which has no floor), and a plain walker falls out of the
  level there. Swimming was the assumed missing piece for a while and the
  cartridge says otherwise. Its decoded tiles put the animated water line in
  the bottom row of the screen for 300 of 2-1's 320 columns, so the water sits
  below the playfield rather than being something he moves through, and 2-1's
  own object list puts two horizontal lifts inside the gap. Dropping him into
  it on the cartridge (`tools/probe_water.py`) leaves him alive on one of
  them, at the same slot y minus 10 a lift rests him at, carried back and
  forth by its cycle. 2-3 is the same world's third level and gets to column
  178. What crosses the gap is already in the engine; what is missing is a
  walker that knows to wait for a lift.
- **Its enemies are in, minus the swimmers.** The row question turned out to be
  confined to Honen and Yurarin Boo, whose records all carry the same row byte
  and which place themselves below the screen, so neither is written to a level
  file. Every other World 2 record of a kind the engine implements lands on a
  cell that is not solid, the check World 1 passes, and those are extracted:
  2-1 gets 10 walkers and 4 lifts, 2-2 gets 14 and 5. 2-3 has none of the
  implemented kinds at all, so it stays empty.

World 2 is not wired into the default campaign, which stays World 1, until
something plays it end to end. That is now a question about riding lifts and
about 2-3, whose whole 360 columns carry the water line in the *top* row, so
the level is under water throughout.

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
