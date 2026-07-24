# Reference notes: the title screen

Working notes for Milestone 1. The goal is to draw the Super Mario Land title screen from our own code. This file records what we know about how the original produces it, so the extraction and rendering tasks have something concrete to build against. Values marked "observed" were read directly from the verified ROM or are fixed Game Boy hardware facts. Values marked "to pin" still need confirming during extraction, by watching a real emulator or, only where needed, checking the `kaspermeerts/supermarioland` disassembly.

## The cartridge (observed from the ROM header)

Read from the header region of the verified dump:

| Field | Address | Value | Meaning |
|-------|---------|-------|---------|
| Title | 0x0134 to 0x0143 | `SUPER MARIOLAND` | game title, ASCII, zero padded |
| CGB flag | 0x0143 | 0x00 | plain Game Boy, not Color |
| SGB flag | 0x0146 | 0x00 | no Super Game Boy features |
| Cartridge type | 0x0147 | 0x01 | MBC1, no RAM |
| ROM size | 0x0148 | 0x01 | 64 KiB, four 16 KiB banks |
| RAM size | 0x0149 | 0x00 | no cartridge RAM |
| Version | 0x014C | 0x01 | mask ROM revision |
| Header checksum | 0x014D | 0x9D | verified, matches computed |

So the game is a 64 KiB MBC1 cartridge. Bank 0 is fixed at 0x0000 to 0x3FFF, and banks 1 to 3 swap into 0x4000 to 0x7FFF. Any graphics we extract live somewhere in those four banks.

## Game Boy display model (hardware facts)

The pieces our renderer needs, independent of this game:

- Screen is 160x144 pixels, 2 bits per pixel, so four shades.
- Tiles are 8x8 pixels, stored as 16 bytes each, two bytes per row. For a row, one byte holds the low bit of each of the eight pixels and the next byte holds the high bit. Combining them gives a 0 to 3 color index per pixel.
- Tile data sits in VRAM at 0x8000 to 0x97FF, three overlapping blocks of 128 tiles.
- Two 32x32 background tile maps live at 0x9800 to 0x9BFF and 0x9C00 to 0x9FFF. Each entry is one byte, a tile index. Only 20x18 tiles are visible at once.
- The background palette register (BGP, 0xFF47) maps the four color indices to four shades. Sprites use OBP0 and OBP1.
- Scroll registers SCX (0xFF43) and SCY (0xFF42) shift the visible window over the 256x256 background.
- The LCD control register LCDC (0xFF40) selects which tile data block and which tile map are active, and toggles background, window, and sprites.

For a static screen, the background layer alone (one tile map plus the referenced tiles plus BGP) is enough. That is our first rendering target.

## How the title screen is composed (to pin during extraction)

Plan, to confirm by observation:

1. The title image is a background layer: a tile map filled with indices that point at the logo, the ground, and the text tiles, all decoded from tile data in one of the ROM banks.
2. There is at least one animated sprite on the title screen (Mario, plus a moving element). Sprites are separate from the background and come later; the first milestone target is the static background.
3. The palette is the standard four-shade DMG set through BGP.

## What the extraction task needs to produce

- The tile pixel data for the title screen tiles, decoded from the 2 bits per pixel format into our own tile representation.
- The background tile map for the title screen (which tile index sits in each cell).
- The palette (BGP value) so shades render correctly.

Output goes into our gitignored asset format by a reproducible command, never committed as ROM data. The exact source addresses for the title tiles and map are the first thing to nail down in the extraction task; record them here once observed.

## Observed at the title screen (from a headless PyBoy run)

Read live from VRAM after booting ~600 frames (see `tools/extract_title.py`):

| Thing | Value | Meaning |
|-------|-------|---------|
| LCDC | 0xC3 | background on, tile addressing signed, map at 0x9800 |
| Tile addressing | signed (0x8800 method) | map index is signed, tile 0 lives at 0x9000 |
| Background map base | 0x9800 | the title layout is here |
| BGP | 0xE4 | standard palette, index 0 lightest to 3 darkest |
| SCX, SCY | 0, 0 | no scroll, the visible 20x18 is the top-left of the map |
| Unique tiles | 110 of 360 cells | the 20x18 screen reuses 110 distinct tiles |

## ROM offsets for the title screen tile data

These four offsets are independently reproducible with no disassembly
reading: run `tools/find_rom_offset.py <vram_addr> <length>` for each VRAM
destination below (600 frames in, the title screen is up) and it reports the
same ROM offsets by searching the ROM file for the exact bytes the emulator
loaded into memory. That is how they were confirmed here.

They also happen to be documented in the kaspermeerts/supermarioland
disassembly (bank0.asm, `GameState_0E`), whose copy instructions use CPU
addresses like `ld hl, $791A`. Those addresses fall in the Game Boy's
bank-switched window ($4000-$7FFF), and `GameState_0E` runs with ROM bank 2
switched in, so a CPU address `A` there is at ROM file offset
`2*0x4000 + (A - 0x4000)`, not the raw value of `A`. A first pass at this
assumed bank 1 (file offset equals the CPU address): it looked plausible and
decoded without error, since it landed on other real, in-range tile data, but
it rendered the wrong tiles (63% match against an emulator reference instead
of 99.82%). Reading the disassembly text gave the wrong bank; the
observe-and-search method above gave the right one directly, which is why it
is the preferred technique (see CLAUDE.md).

| CPU addr | ROM file offset (bank 2) | VRAM destination | Size (bytes) | Tiles | Content |
|----------|--------------------------|-------------------|---------------|-------|---------|
| $5032 | 0x9032 | 0x9000 | 0x02C0 | 44 | Font, coins (signed indices 0x00-0x2B) |
| $791A | 0xB91A | 0x9300 | 0x0500 | 80 | Menu/logo background (signed indices 0x30-0x7F) |
| $7E1A | 0xBE1A | 0x8800 | 0x0170 | 23 | Additional tiles (signed indices 0x80-0x96) |
| $4862 | 0x8862 | 0x8AC0 | 0x0010 | 1 | Mushroom sprite |

The Rust extraction command (`sml extract-title`) reads these offsets directly
from the ROM binary. No emulator is involved in extracting the tile graphics.
The tilemap (20x18 VRAM tile indices) is embedded as a constant in
`src/assets/title.rs`: the level-loading routine that generates it is not yet
reimplemented, so the map was captured once from emulator VRAM
(`tools/dump_title_map.py`) and is documented as a fixed property of this ROM
rather than derived at runtime. Extracting it without an emulator, by
reimplementing the level loader, is open work.

## Earlier open questions (now answered)

- Exact VRAM tile block used by the title screen (LCDC bit 4: 0x8000 unsigned addressing or 0x8800 signed). Answer: signed.
- Which of the two tile maps holds the title layout. Answer: 0x9800.
- Where in the ROM banks the title tile graphics are stored. Answer: all in bank 2, see the ROM offsets table above. Extracted by `sml extract-title` directly from the ROM file. The tilemap layout is still a captured constant, not derived from the ROM at runtime (see above).
