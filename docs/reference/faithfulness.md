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
- **Levels**: the demo level, the example level, and the demo campaign are test
  fixtures, documentation, and placeholders. The real levels come from extracting
  the cartridge's geometry (ROM/emulator). Shipping invented
  levels is not a goal (see the end-goal note in CLAUDE.md).
- **World 1-1 geometry**: canonical. All 300 columns decode from the ROM and
  match the running game on every column there is ground truth for (88 of
  them). See `docs/reference/level-1-1.md`.
- **Tile solidity**: partly canonical, and the level format does not carry it,
  so it has to be observed. `tools/classify_solid_tiles.py` watches a real run
  and separates tiles Mario is supported by from tiles he rests inside. Clean
  results so far, over 68 columns of a real run:

  | tiles | verdict | evidence |
  |-------|---------|----------|
  | 96 | solid | held Mario up on 577 frames, never rested inside |
  | 44, 49, 50, 51, 54, 94 | non-solid | rested inside; never once supported |

  35 of the level's 43 tile ids are **unclassified**, because the walker only
  reaches world column 68 and never meets them. Any solidity the converter
  assigns beyond the table above is inference and is labelled as such where it
  is used.

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
