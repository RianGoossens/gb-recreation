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

## Power-ups and states

| Item / state | Label | Notes |
|--------------|-------|-------|
| Small Mario | canonical | |
| Super mushroom, big Mario | canonical | |
| Superball flower, fire Mario | canonical | SML's signature power-up |
| Superball projectile | canonical | thrown by fire Mario; bounces; collects coins |
| Invincibility star | **invented** | Super Mario Land has NO star. Kept in the codebase for now (per Rian, 2026-07-23); must be removed or gated behind opt-in before the final faithful build. |

## Enemies

| Enemy | Label | Notes |
|-------|-------|-------|
| Goomba (walker) | stand-in | SML's ground walker is the Chibibo. Ours behaves like it (walk, turn at walls and ledges); confirm exact behavior against the cartridge. |
| Fly (hopper) | **invented / stand-in** | A generic hopping enemy, not a specific SML enemy. SML World 1 (Birabuto) has the Nokobon (a walking bomb). Kept in the codebase for now (per Rian, 2026-07-23); replace with a real SML enemy or gate behind opt-in before the final faithful build. |

## Items, blocks, scoring

| Piece | Label | Notes |
|-------|-------|-------|
| Coins, 100-coin 1up | canonical | |
| Question block gives a coin | canonical | |
| Power block gives mushroom/flower by size | canonical | matches SML's size-based item |
| Brick block | canonical | big/fire Mario breaks it; small Mario bumps it |
| Score, lives, timer, time-out death | canonical | point values not yet matched to the cartridge |

## Physics and levels

- **Movement constants** (walk accel, friction, gravity, jump, stomp bounce,
  speeds): PROVISIONAL placeholders, not measured from the cartridge. Pinning
  them to the original (by observing an emulator or the disassembly) is open work.
- **Levels**: the demo level, the example level, and the demo campaign are test
  fixtures, documentation, and placeholders. The real levels come from extracting
  the cartridge's geometry (ROM/emulator), which is open work. Shipping invented
  levels is not a goal (see the end-goal note in CLAUDE.md).

## Recommended next steps toward faithfulness

1. Measure the movement constants against the cartridge and replace the placeholders.
2. Extract the real level geometry.
3. Replace the Fly with a real SML enemy (Nokobon), or gate it behind opt-in, before the final faithful build.
4. Remove the invincibility star, or gate it behind opt-in, before the final faithful build.

(Brick breaking and superball coin collection are already canonical, done.)
