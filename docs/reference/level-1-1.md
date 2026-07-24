# Reference notes: World 1-1

Working notes for extracting World 1-1's real geometry. Values marked
"observed" were read directly from the verified ROM or a headless emulator
run. This file grows as the extraction subtasks land; see the plan for the
breakdown.

## Reaching gameplay (for any tool that needs it)

Boot the ROM, wait for the title screen (600 frames is enough, same as the
title extraction), press Start, then wait for the level to finish loading
(300 more frames is enough to reach a controllable Mario standing at the
spawn of 1-1). `tools/extract_level_1_1.py` does this.

## The opening screen (observed)

Read live from VRAM right after gameplay starts:

| Thing | Value | Meaning |
|-------|-------|---------|
| LCDC | 0xC3 | background on, signed tile addressing, map at 0x9800 |
| BGP | 0xE4 | same palette as the title screen |
| SCX, SCY | 0, 0 | no scroll yet, spawn is the left edge of the level |
| Tile addressing | signed (0x8800 method) | same scheme as the title screen |
| Unique tiles | 39 of 360 cells | the opening 20x18 screen |

## Tile graphics: no new ROM offsets needed

Every one of the 39 unique tiles used on the opening screen was checked
against the ROM (search each tile's live VRAM bytes for its file offset, the
same technique as `tools/find_rom_offset.py`). All of them land inside the
three tile blocks already pinned for the title screen (`title.rs`,
`title-screen.md`):

- Block 1 (`rom 0x9032`, `vram 0x9000`, size `0x2C0`)
- Block 2 (`rom 0xB91A`, `vram 0x9300`, size `0x500`)
- Block 3 (`rom 0xBE1A`, `vram 0x8800`, size `0x170`)

So the title screen and World 1-1 draw from the same shared bank-2 tile
atlas; the level just uses a different slice of it (ground, pyramid blocks,
palm trees, bushes, clouds) alongside the shared HUD/text tiles. No fourth
block was needed for this screen.

A caution from doing this search per tile: several tiles are simple or
repetitive enough (a solid color, a symmetric pattern) that the same 16 bytes
occur more than once in the ROM. Searching a single tile in isolation can
report a coincidental match in the wrong bank before the real one. The three
blocks above were cross-checked against every observed tile address falling
inside their already-verified ranges, not from a fresh single-tile search
taken at face value.

## The opening tilemap

`tools/extract_level_1_1.py` reads the background tile map the same way
`extract_title.py` does, and writes `assets/extracted/level_1_1_opening.tmap`
(our SMLM format) plus a tile sheet and reference screenshot, all gitignored.
This captures only the 20x18 view visible at spawn; the level scrolls well
past it.

## Tile solidity: what is confirmed so far

Method: force Mario to walk and jump through the opening screen (`SCX` stays
`0` here, so the raw tilemap index at `(feet_row, x // 8)` is directly
readable with no scroll math), and watch `0xC20A` (see `physics.md`) flip
between grounded and airborne. A tile a resting Mario stands on is solid; a
tile his jump arc passes through with no effect on his motion is not.

- Tile `96` (the ground surface row, `row 16` for the whole opening screen):
  **solid**. Directly observed: Mario's feet rest on it continuously
  whenever `0xC20A == 1`.
- Tile `44` (the blank sky/background filler, most of rows 0-15):
  **non-solid**. Directly observed: it is the tile at every cell Mario's
  jump arc passes through in open air with no collision effect.
- Tile `97` (directly beneath `96`, the underground fill) is presumed solid
  by level-design consistency (a ground block is solid all the way down),
  but this has not been independently confirmed by a direct collision, since
  Mario never touches it from below or the side on this screen. Flagging
  this rather than asserting it as observed.
- The mountain/pyramid background silhouette (tile `94` and neighbors) and
  the small step structure Mario partially climbed (tiles including `54`,
  `112`-`115`, `129`, `49`-`51`, `65`-`71`) are **not yet reliably
  classified**. Two captures of the same cell during a jump onto the
  structure read different tile IDs (`51` from a static pre-walk snapshot,
  `54` from a capture taken mid-jump), which most likely means Mario's exact
  pixel column did not match between the two reads rather than the tile
  itself changing; a single feet-position snapshot per event is not precise
  enough here. This needs a dedicated frame-by-frame probe that tracks
  Mario's exact sub-column at the moment of landing, not the broad sweep
  used for the ground/sky classification above.
- Separately, in this scripted run Mario could not walk past on-screen
  `x = 81` even with periodic jumps over ~1400 frames (the level never
  actually scrolls: `SCX` stays at `0` the entire time). An enemy blocking
  him was the first guess, but that does not hold up: dumping OAM at the
  stuck position shows only Mario's own sprite (four entries forming a
  16x16 sprite, `x` 66-81), no separate enemy nearby. The horizontal speed
  register (`0xC20C`, see `physics.md`) stays pinned at its maximum (`6`)
  the entire time he is stuck, meaning the game's own walk logic still
  thinks he is moving at full speed; only his displayed position refuses to
  advance. That rules out a plain "walked into a wall and stopped" story
  too (which would usually show speed decaying, not holding at max), and
  the background stays visually static (no scroll) rather than the camera
  picking up his motion instead. None of the three explanations tried so
  far account for all three observations at once. Left open rather than
  guessed at.

## Open work

- Work out what is actually happening at the `x = 81` blockage (see above);
  none of "enemy contact", "wall collision", or "camera lock, world still
  advancing" fully explain the combination of a maxed speed register, a
  frozen displayed position, and an unmoving background all at once.
- Pin the step/pyramid structure's solid tiles precisely (needs the
  sub-column-accurate probe described above).
- Get Mario past the early blockage so the rest of the screen, and
  eventually the scrolling sections beyond it, can be surveyed the same way.
- Stitch the full scrolling width: walk through the whole level while
  recording the tilemap and scroll position per screen.
- Convert the result into `Level`/`Solids` and wire it in behind the existing
  ROM gating, replacing the placeholder demo level.
