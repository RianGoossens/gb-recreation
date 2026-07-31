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
| Goomba (walker) | stand-in | SML's ground walker is the Chibibo. Ours behaves like it (walk, turn at walls and ledges); confirm exact behavior against the cartridge. |
| Fly (hopper) | **invented / stand-in** | A generic hopping enemy, not a specific SML enemy. SML World 1 (Birabuto) has the Nokobon (a walking bomb). Gated the same way as the star. Still worth replacing with a real SML enemy rather than only hiding it. |

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
- **Levels**: World 1's three levels are extracted from the cartridge and are
  what `run` and `play` load by default when they have been generated
  (`sml extract-level 1-1|1-2|1-3`). The demo level, the example level and the
  demo campaign remain as test fixtures and as the fallback on a checkout
  without the ROM. Shipping invented levels is not a goal (see the end-goal
  note in CLAUDE.md).
- **World 1 geometry**: canonical. All 300 columns of 1-1 and all 280 of 1-2
  match the running cartridge exactly; 1-3's first 27 do so far.
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
