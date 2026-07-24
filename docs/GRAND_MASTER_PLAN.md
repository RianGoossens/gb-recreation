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
    - [~] Classify which tile IDs are solid by observing Mario's actual collisions against each one while walking through
      - [x] Ground (tile 96) confirmed solid and sky/background filler (tile 44) confirmed non-solid, by direct observation of the grounded flag and jump arcs (docs/reference/level-1-1.md)
      - [x] Resolved the "stuck at x=81" mystery: not a blockage, it is the standard mid-screen camera lock (confirmed via frame-by-frame screenshot diffs showing the background scrolling continuously while Mario's screen position stays pinned). No enemy, no wall; the earlier "SCX stays 0" finding was a sampling artifact from the status bar's mid-frame STAT split (docs/reference/level-1-1.md)
      - [ ] Pin the step/pyramid structure's solid tiles precisely (needs a sub-column-accurate probe; two captures of the same cell disagreed, see the doc)
    - [ ] Find a reliable per-frame read of the real scroll amount (naive once-per-frame SCX is aliased by the status-bar split; `0xC20B` is an unconfirmed lead, see the doc), then stitch the full scrolling width by walking through the whole level, recording tilemap and scroll per screen
    - [ ] Convert the extracted grid into `Level`/`Solids` and wire it in, ROM-gated
- [x] Scrolling camera that follows Mario
- [x] Mario entity: position, velocity, facing, sprite
- [x] Input mapping (keyboard to Game Boy buttons)
- [x] Walking physics: acceleration, max speed, friction (constants sourced from reference)
  - [x] Verify walking constants against the emulator/disassembly (accel, friction, max walk speed measured from WRAM via tools/find_mario_speed.py; gravity/jump/stomp still provisional)
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
- [x] Hooks or data-driven config for tuning mechanics
- [x] Blog post: build your own level

---

## Backlog and notes

- Screenshots decision (made by the user, 2026-07-22): game screenshots are fine to use in blog posts as commentary. They are committed under `docs/blog/media/` via Git LFS. The ROM and raw extracted asset files stay gitignored; only curated images go in LFS. So the copyright concern that had parked the image tasks is resolved.
- What still gates the remaining Milestone 1 image tasks is technical, not legal: rendering the real title screen needs the title tile and tilemap data, which means finishing the extraction subtask by observing emulator VRAM (for example with a headless emulator run). That is the next real unblock for "render the title screen", the golden image, and the title-screen blog post.
- The ROM in the tree passes the hash check (verified 2026-07-22).
- Keep physics constants cited to the reference so behavior is defensible.
- Revisit module boundaries at the end of each milestone during self-improvement.
