# Grand Master Plan

The living plan of record. Every task is a checkbox. Work happens one task at a time through the task execution skill. Completing every box here means the project is done.

Rules for this document:
- Organized into playable vertical slices (milestones). Each milestone ends in something you can run and watch.
- Task order is not binding. Milestones are grouped so earlier slices unlock later ones, but within that, pick work by dependency and value, not by list position.
- Mark tasks `[x]` only when done and tested.
- Split a task into subtasks when it turns out bigger than one sitting.
- Add new tasks as reality demands. Keep it honest.

Legend: `[ ]` todo, `[x]` done, `[~]` in progress.

---

## Milestone 0: Workspace and foundations

Goal: the repo builds, tests run, CI is green, the blog deploys.

- [x] Initialize repo, CLAUDE.md, README.md, .gitignore
- [x] Create this plan
- [x] Author the skills (git-github, grand-master-plan, task-execution, testing-validation, dev-blog, self-improvement)
- [x] CI workflow running `cargo test` on push
- [x] GitHub Pages workflow deploying the blog on push
- [x] Scaffold the Rust crate: `cargo` project that builds and has one passing test
- [x] Define the module layout (core logic, rendering, input, assets) as empty documented modules
- [x] First blog post: the workspace and how it is driven
- [x] Push to GitHub, confirm both workflows pass

## Milestone 1: Boot to title screen

Goal: run the app and see the Super Mario Land title screen, rendered by our code.

- [x] Replace the ROM with a hash-verified dump; add a `verify-rom` command that checks SHA-1, MD5, CRC32 and refuses to proceed on mismatch
- [x] Study the title screen logic and tile data (mainly by observing a real emulator; consult the `kaspermeerts/supermarioland` disassembly only where needed); write down the memory map notes in `docs/reference/`
- [x] Asset pipeline: extract the title screen tiles and palette from the verified ROM into our asset format (gitignored output, reproducible command)
  - [x] Tile decoder (2bpp), asset format (save/load + PGM preview), and a ROM-gated `extract-tiles` command that decodes a byte range reproducibly
  - [x] Pin the exact title-screen tile and tilemap source addresses (observe emulator VRAM) and extract them specifically, plus the BGP palette (tools/extract_title.py via PyBoy: signed addressing, map 0x9800, BGP 0xE4)
- [x] Game Boy display model: 160x144 framebuffer, 4-shade palette, tile and tilemap rendering
- [x] Window + rendering frontend that draws the framebuffer to screen (behind the gui feature; building blocks tested, window loop not run in this headless env)
- [x] Headless screenshot command that renders a given state to PNG
- [x] Render the title screen from extracted assets
- [x] Golden-image test: render pipeline guarded by a committed golden of our own demo scene (CI-safe, non-infringing). Title-screen faithfulness is verified locally at 99.82% shade match vs the emulator reference, not committed (copyright).
- [x] Blog post: booting to the title screen

## Milestone 2: World 1-1 renders, Mario walks and falls

Goal: load level 1-1, see it on screen, move Mario left/right with gravity.

- [~] Level data model and 1-1 tilemap extracted from the reference/ROM
  - [x] Level data model: solids plus Mario spawn, with a human-editable ASCII loader
  - [~] Extract the scrolling World 1-1 tilemap from the ROM/emulator into level data
    - [x] Pin World 1-1's tile graphics source and the opening screen's background tilemap by observing emulator VRAM (mirrors the title-screen technique; reuses the title screen's existing tile blocks, see docs/reference/level-1-1.md)
    - [ ] **Find the cartridge's own collision test in the ROM.** The shipped solidity rule (`0x60 <= tile <= 0xE8`) is a fit to 7 observed tiles plus "the level can be finished", and it decides the other 36 tile ids with no evidence. Rian caught the previous rule being wrong by playing it (too few pipes, too few blocks, no pits at all), so this needs a real source rather than a better fit. Locate the routine or table the game uses to decide whether a tile blocks Mario, the same way `find_level_pointer.py` located the level stream (docs/reference/level-1-1.md)
    - [~] Classify which tile IDs are solid by observing Mario's actual collisions against each one while walking through. `tools/classify_solid_tiles.py` settles 7 of the level's 43 tile ids cleanly (96 solid on 577 frames of support with zero sustained overlap; 44/49/50/51/54/94 non-solid). 36 remain unresolved: 27 the walker never reaches, and 9 it reaches with no evidence either way
      - [x] Ground (tile 96) confirmed solid and sky/background filler (tile 44) confirmed non-solid, by direct observation of the grounded flag and jump arcs (docs/reference/level-1-1.md)
      - [x] Resolved the "stuck at x=81" mystery: not a blockage, it is the standard mid-screen camera lock (confirmed via frame-by-frame screenshot diffs showing the background scrolling continuously while Mario's screen position stays pinned). No enemy, no wall; the earlier "SCX stays 0" finding was a sampling artifact from the status bar's mid-frame STAT split (docs/reference/level-1-1.md)
      - [x] Pin the step/pyramid structure's solid tiles: **not solid** (against a horizontal approach). Jump-timing sweeps couldn't settle it directly, but building the opening screen into a real level for our own engine (`tools/convert_level_1_1_to_level_format.py`) and walking it did: marking the pyramid solid per the earlier "level-design consistency" presumption made Mario get stuck oscillating at spawn in our engine, contradicting every one of the dozens of real-emulator traces this session showing him walk straight through. Marking it non-solid matches real behavior exactly. Solidity from above (falling onto its top) is untested and separate (docs/reference/level-1-1.md)
    - [x] Get a trustworthy world position: `SCX` and a full WRAM/OAM/HRAM scan (`tools/find_scroll_position.py`) both come up empty, so `tools/sml_scroll.py` measures the scroll by cross-correlating consecutive rendered frames instead. Validated at exactly 1 px/frame against the pinned walk speed. This overturned the stitching numbers below: the reactive walker was actually stuck against a pipe at world column 17, covering 137px in 5000 frames, and its reported 626/720/1880 figures came from integrating Mario's WRAM bytes, which go stale rather than blank through the ~150-frame death freeze. A walker that jumps when the measured scroll stalls now reaches world column 78 before its first death, and `stitch_level_1_1.py` stops there and writes a bounded 96-column map (docs/reference/level-1-1.md)
    - [x] Read the level out of the ROM instead of playing through it (Rian, issue 5). All 300 columns of World 1-1 now decode from the cartridge with no emulator involved. The format has two layers: a column record is a run list terminated by `0xFE` (`(row<<4)|count` plus tiles, where count 0 means a full 16 rows and `0xFD <tile>` repeats one tile for the run), and a level is a `0xFF`-terminated list of 16-bit pointers, one per 20-column screen. World 1-1's list is at 0x0A198: 15 screens, 6 of them reused, which is why world columns 0-19 and 40-59 are byte-identical. Verified 88/88 against every column the running game reveals, with the runner-up candidate at 45% (`tools/capture_columns.py`, `tools/decode_level.py --verify`). Found by watching RAM for the game's own banked-ROM read pointer (`tools/find_level_pointer.py`) rather than pattern-matching bytes; that also corrected two earlier wrong offsets and a wrong claim that a tile id could be `0xFE` (docs/reference/level-1-1.md)
    - [x] Stitch the full scrolling width by playing through it. Superseded twice over and now closed. The old "travelled past world column 1880" figures were artifacts of a broken position estimate (the walker was stuck against a pipe at world column 17), and the corrected play-through method still capped at world column 78. The ROM decode above replaces this path entirely. `tools/stitch_level_1_1.py` and the walker stay useful for capturing ground truth to check a decode against, which is what `tools/capture_columns.py` now does (docs/reference/level-1-1.md)
    - [ ] Port the level extraction from Python to Rust (Rian, 2026-07-25: Python is fine for prototyping, the finished thing is always Rust). The format is understood and verified now, so `tools/decode_level.py` and `tools/convert_level_1_1_to_level_format.py` have no reason to stay Python: neither touches the emulator (both still declare a stale `pyboy` dependency they never import). Move the column-record and screen-list parsing into `src/assets/level.rs` alongside the existing title-screen extraction, with the solidity rules as named, sourced constants, and add an `extract-level` command. The PyBoy observation tools (`sml_boot`, `sml_scroll`, `sml_walker`, `capture_columns`, `classify_solid_tiles`, `find_level_pointer`, `measure_*`) stay Python; they are measuring instruments, not product. Build-time versus runtime extraction is explicitly open, pick either
    - [x] Convert the confirmed grid into `Level`/`Solids` and wire it in, ROM-gated. `tools/convert_level_1_1_to_level_format.py` now reads all 300 columns from the ROM (no emulator) and writes a plain-text level `Level::from_file` loads, still gitignored and regenerated on demand. Solidity is observed where it can be (tile 96 solid; 44/49/50/51/54/94 non-solid) plus two stated inferences: fill propagates downward from a solid cell (without it the raised platform at columns 61-64 is a pit), and a column with no solid cell stands on its lowest non-sky tile (without it the level ends in a pit sixteen columns short of its own exit gate). Tile 99, the pipe, was settled by playing both readings: the cartridge's grounded feet row over that stretch reads `14 14 14 - 10 10 14 14`, our engine with the pipe passable runs flat across it, with it solid it climbs at the same place. `tests/extracted_level_1_1.rs` walks the level to its end trigger and is verified to fail when either inference is removed (docs/reference/level-1-1.md)
- [x] Scrolling camera that follows Mario
- [x] Mario entity: position, velocity, facing, sprite
- [x] Input mapping (keyboard to Game Boy buttons)
- [x] Walking physics: acceleration, max speed, friction (constants sourced from reference)
  - [x] Verify walking constants against the emulator/disassembly (accel, friction, max walk speed measured from WRAM via tools/find_mario_speed.py)
  - [x] Verify gravity/jump constants against the emulator (a three-regime state machine, not one acceleration; measured via WRAM plus the internal rise/fall phase byte, implemented in `step_motion`; see docs/reference/physics.md)
  - [x] Verify the stomp bounce against the emulator: measured from 61 landed stomps on the first World 1-1 Chibibo (`tools/measure_stomp_bounce.py`), 8px of rise over about 12.4 frames. A bounce decays like a released jump even with the button held, which is a different regime from a jump's held rise, so `Mario` gained a `bouncing` flag. `STOMP_BOUNCE` went 500 -> 360 (docs/reference/physics.md)
- [x] Gravity and ground collision against the tilemap
- [x] Jump physics (initial velocity, variable height)
- [x] Animation states: idle, walk, jump
- [x] Tests: physics constants, collision cases, a scripted-input golden frame (constants pinned; collision floor/wall/ceiling covered; game_walk_right golden)
- [x] Blog post: World 1-1 and the physics of walking

## Milestone 3: Collision, enemies, and death

Goal: full solid-world collision, a Goomba-equivalent enemy that walks and can be stomped, Mario can die.

- [x] Full tile collision (walls, floors, ceilings; one-tile pits). SML tile levels have no slopes, so none are modeled.
- [x] Enemy framework (spawn, update, despawn offscreen)
- [x] The 1-1 first enemy (Goomba equivalent): walk, turn at edges/walls
- [x] Stomp interaction: kill enemy, bounce Mario
- [x] Damage/death: Mario loses on contact, respawn/reset (death animation deferred until Mario has real sprites)
- [x] Tests: enemy movement, stomp vs. side-contact outcomes (walk/wall-turn/ledge-turn/fall; stomp bounce vs. side-contact death)
- [x] Blog post: enemies, stomps, and dying

## Milestone 4: Items, blocks, and scoring

Goal: question blocks, coins, the power-up flow, score and coin counters.

- [x] Interactive blocks (question, brick): bump, spawn contents
- [x] Coins: collect, counter, 100-coin life
- [x] Power-up (mushroom equivalent): spawn, movement, pickup, size/state change
- [x] HUD: score, coins, lives, timer
- [x] Timer countdown and time-out death
- [x] Tests: block bumping, coin counting, power-up state machine (bump gives coin; coin count + 100-coin life + score; power state grow/shrink/die)
- [x] Pin the cartridge's point values: a stomp and a coin are both 100, read off the real status bar (`tools/sml_hud.py` reads score/coins/lives/timer straight from the tilemap, since the digit tiles are the digits; `tools/measure_scores.py` logs every award during a real run). Both already matched what we award. The power-up's 1000 is still unmeasured (docs/reference/faithfulness.md)
- [x] Blog post: blocks, coins, and getting big

## Milestone 5: Level completion and flow

Goal: reach the end of 1-1, complete it, advance. The core loop is playable end to end.

- [x] Level-end trigger and completion sequence
- [x] Level-to-level transition and world map or direct advance (per original)
- [x] Lives and game-over flow
- [x] Title -> play -> die/complete -> title loop closed
- [x] Tests: completion trigger, game-over transition (game: end-completes-and-freezes, lives-out-ends; session: win, advance, game-over, title-return)
- [x] Blog post: closing the loop, a playable slice

## Milestone 6: Breadth (more levels and enemies)

Goal: expand from a vertical slice to coverage of the original game.

- [ ] Remaining World 1 levels (the original's real geometry; ROM/copyright gated)
- [x] A temporary demo campaign to exercise the multi-level flow (placeholder only, until the cartridge's real levels are extracted; shipping our own invented levels is NOT a project goal)
- [x] Remaining enemy types for World 1 (added a hopping Fly alongside the Goomba; more can follow)
- [x] Sound and music model: event model implemented (Game emits SoundEvents; frontend drains them)
- [x] Tone playback: the `gui` frontend plays each SoundEvent as a square-wave beep via `cpal` (src/audio.rs). Frequencies are invented placeholders, not read from the APU; flagged as a stand-in in docs/reference/faithfulness.md.
- [ ] Additional worlds, level by level
- [ ] Bosses and special stages
- [x] Blog posts per major addition (power-ups and polish post covers the star, superball, pause, one-way camera, sound; more per addition)

## Milestone 7: Moddability

Goal: deliver on the promise that users can make custom levels and mechanics.

- [x] Human-editable level format and loader
- [x] Documentation and an example custom level
- [x] Gate the non-cartridge content so the default build is faithful: `Level::without_non_canonical` drops the invincibility star (SML has none) and the Fly (a generic hopper, not an SML enemy), and `run`/`play` apply it unless given `--allow-non-canonical`. The level format still parses both markers, so a custom level using them loads and simply plays without them (docs/reference/faithfulness.md)
- [x] Hooks or data-driven config for tuning mechanics: `Tuning::from_text` existed and was tested but was never actually reachable from the CLI, and `Session` silently discarded custom tuning on every level transition or restart. Fixed: `run`/`play` both accept an optional tuning file, `Session` carries and reapplies it, and the key reference lives in `docs/reference/level-format.md` (previously nowhere but one blog post's two-line example)
- [x] Blog post: build your own level

---

## Backlog and notes

- Screenshots decision (made by the user, 2026-07-22): game screenshots are fine to use in blog posts as commentary. They are committed under `docs/blog/media/` via Git LFS. The ROM and raw extracted asset files stay gitignored; only curated images go in LFS. So the copyright concern that had parked the image tasks is resolved.
- What still gates the remaining Milestone 1 image tasks is technical, not legal: rendering the real title screen needs the title tile and tilemap data, which means finishing the extraction subtask by observing emulator VRAM (for example with a headless emulator run). That is the next real unblock for "render the title screen", the golden image, and the title-screen blog post.
- The ROM in the tree passes the hash check (verified 2026-07-22).
- Keep physics constants cited to the reference so behavior is defensible.
- Revisit module boundaries at the end of each milestone during self-improvement.
- Jump physics redesign: done. `step_motion` (`src/core/physics.rs`) now models the three measured regimes (near-constant held rise capped by a frame count, real deceleration on early release, real acceleration while falling) instead of one continuous `GRAVITY` acceleration. See `docs/reference/physics.md` for the traces, fits, and a correction made while implementing (the frame-cap-while-held case turned out to be a direct reset, not routed through the release-deceleration constant). Stomp bounce is measured too now, and no physics constant is provisional any more except `MAX_FALL_SPEED`, which is a tunneling guard rather than a modeled behavior.
