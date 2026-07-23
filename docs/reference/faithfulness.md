# Faithfulness audit

The end goal is a faithful recreation of Super Mario Land, easy to modify. This
file tracks how close each implemented piece is to the cartridge, so deviations
are visible and deliberate rather than accidental. Three labels:

- **canonical**: in the original game.
- **stand-in**: an equivalent we built before pinning the exact original, to be
  replaced or confirmed against the cartridge.
- **invented**: not in the original. Fine as an optional mod, but not end-goal
  content. Flagged for a decision.

## Power-ups and states

| Item / state | Label | Notes |
|--------------|-------|-------|
| Small Mario | canonical | |
| Super mushroom, big Mario | canonical | |
| Superball flower, fire Mario | canonical | SML's signature power-up |
| Superball projectile | canonical | thrown by fire Mario; bounces |
| Invincibility star | **invented** | Super Mario Land has NO star. Recommend removing it, or keeping it only as an opt-in mod, not in the faithful game. |

## Enemies

| Enemy | Label | Notes |
|-------|-------|-------|
| Goomba (walker) | stand-in | SML's ground walker is the Chibibo. Ours behaves like it (walk, turn at walls and ledges); confirm exact behavior against the cartridge. |
| Fly (hopper) | **invented / stand-in** | A generic hopping enemy, not a specific SML enemy. SML World 1 (Birabuto) has the Nokobon (a walking bomb). Recommend replacing the Fly with a real SML enemy. |

## Items, blocks, scoring

| Piece | Label | Notes |
|-------|-------|-------|
| Coins, 100-coin 1up | canonical | |
| Question block gives a coin | canonical | |
| Power block gives mushroom/flower by size | canonical | matches SML's size-based item |
| Brick block | stand-in | currently inert; SML lets big Mario break bricks (see below) |
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

1. Decide on the invented star (remove, or keep opt-in only).
2. Replace the Fly with a real SML enemy (Nokobon) once its behavior is confirmed.
3. Make brick blocks breakable by big/fire Mario (canonical).
4. Have the superball collect coins on contact (canonical).
5. Measure the movement constants against the cartridge and replace the placeholders.
6. Extract the real level geometry.
