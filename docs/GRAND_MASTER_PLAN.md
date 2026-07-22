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
- [ ] Window + rendering frontend that draws the framebuffer to screen
- [x] Headless screenshot command that renders a given state to PNG
- [x] Render the title screen from extracted assets
- [x] Golden-image test: render pipeline guarded by a committed golden of our own demo scene (CI-safe, non-infringing). Title-screen faithfulness is verified locally at 99.82% shade match vs the emulator reference, not committed (copyright).
- [x] Blog post: booting to the title screen

## Milestone 2: World 1-1 renders, Mario walks and falls

Goal: load level 1-1, see it on screen, move Mario left/right with gravity.

- [ ] Level data model and 1-1 tilemap extracted from the reference/ROM
- [ ] Scrolling camera that follows Mario
- [x] Mario entity: position, velocity, facing, sprite
- [x] Input mapping (keyboard to Game Boy buttons)
- [x] Walking physics: acceleration, max speed, friction (constants sourced from reference)
  - [ ] Verify walking constants against the emulator/disassembly (current values are provisional placeholders)
- [x] Gravity and ground collision against the tilemap
- [x] Jump physics (initial velocity, variable height)
- [x] Animation states: idle, walk, jump
- [ ] Tests: physics constants, collision cases, a scripted-input golden frame
- [ ] Blog post: World 1-1 and the physics of walking

## Milestone 3: Collision, enemies, and death

Goal: full solid-world collision, a Goomba-equivalent enemy that walks and can be stomped, Mario can die.

- [ ] Full tile collision (walls, floors, ceilings, slopes if present)
- [ ] Enemy framework (spawn, update, despawn offscreen)
- [ ] The 1-1 first enemy (Goomba equivalent): walk, turn at edges/walls
- [ ] Stomp interaction: kill enemy, bounce Mario
- [ ] Damage/death: Mario loses on contact, death animation, respawn/reset
- [ ] Tests: enemy movement, stomp vs. side-contact outcomes
- [ ] Blog post: enemies, stomps, and dying

## Milestone 4: Items, blocks, and scoring

Goal: question blocks, coins, the power-up flow, score and coin counters.

- [ ] Interactive blocks (question, brick): bump, spawn contents
- [ ] Coins: collect, counter, 100-coin life
- [ ] Power-up (mushroom equivalent): spawn, movement, pickup, size/state change
- [ ] HUD: score, coins, lives, timer
- [ ] Timer countdown and time-out death
- [ ] Tests: block bumping, coin counting, power-up state machine
- [ ] Blog post: blocks, coins, and getting big

## Milestone 5: Level completion and flow

Goal: reach the end of 1-1, complete it, advance. The core loop is playable end to end.

- [ ] Level-end trigger and completion sequence
- [ ] Level-to-level transition and world map or direct advance (per original)
- [ ] Lives and game-over flow
- [ ] Title -> play -> die/complete -> title loop closed
- [ ] Tests: completion trigger, game-over transition
- [ ] Blog post: closing the loop, a playable slice

## Milestone 6: Breadth (more levels and enemies)

Goal: expand from a vertical slice to coverage of the original game.

- [ ] Remaining World 1 levels
- [ ] Remaining enemy types for World 1
- [ ] Sound and music model (design first, then implement)
- [ ] Additional worlds, level by level
- [ ] Bosses and special stages
- [ ] Blog posts per major addition

## Milestone 7: Moddability

Goal: deliver on the promise that users can make custom levels and mechanics.

- [ ] Human-editable level format and loader
- [ ] Documentation and an example custom level
- [ ] Hooks or data-driven config for tuning mechanics
- [ ] Blog post: build your own level

---

## Backlog and notes

- Screenshots decision (made by the user, 2026-07-22): game screenshots are fine to use in blog posts as commentary. They are committed under `docs/blog/media/` via Git LFS. The ROM and raw extracted asset files stay gitignored; only curated images go in LFS. So the copyright concern that had parked the image tasks is resolved.
- What still gates the remaining Milestone 1 image tasks is technical, not legal: rendering the real title screen needs the title tile and tilemap data, which means finishing the extraction subtask by observing emulator VRAM (for example with a headless emulator run). That is the next real unblock for "render the title screen", the golden image, and the title-screen blog post.
- The ROM in the tree passes the hash check (verified 2026-07-22).
- Keep physics constants cited to the reference so behavior is defensible.
- Revisit module boundaries at the end of each milestone during self-improvement.
