# CLAUDE.md

Project guidelines for the Rust reproduction of Super Mario Land. Read this before every work session. Sub-agents inherit these rules.

## What this project is

A native Rust reproduction of the Game Boy title Super Mario Land. This is NOT an emulator. We reimplement the game's logic, physics, and rendering as clean, readable Rust that a person can modify to build custom levels or mechanics. The original assembly is our reference, not our template. We translate intent, not opcodes.

End goal: a faithful recreation of the cartridge that is easy to modify. The game's shipped content is the cartridge's own levels, enemies, and behavior. Moddability means a user can make their own levels; it does not mean the project ships invented levels as content. Any levels we author (the demo level, the example level, the demo campaign) are test fixtures, documentation, or placeholders until the real levels are extracted, never end-goal content.

Faithfulness is a working rule, not just an aim. Reproduce what is in Super Mario Land; do not invent mechanics, enemies, items, or levels. Before adding a mechanic, confirm it exists in the original (from play, an emulator, or the disassembly). If you build a stand-in before the exact original is pinned, or add something that is not in the game, label it in `docs/reference/faithfulness.md` and keep that audit current. When unsure whether something is canonical, ask rather than invent.

## Reference material

Secondary reference: the `kaspermeerts/supermarioland` disassembly. It maps some of the original assembly, physics constants, and memory layout. Lean on it as little as possible: prefer building clean Rust from observed behavior, tests, and screenshots against a real emulator. Reach for the disassembly only to settle a specific number or mechanic you cannot pin down otherwise, and cite what you take.

Preferred way to pin a ROM offset: observe, don't read assembly. Boot the verified ROM in an emulator, capture the bytes at the memory address that holds the data you want (VRAM, OAM, wherever the game put it), then search the ROM file for that exact byte sequence (`tools/find_rom_offset.py`). The offset it is found at is correct by construction, including bank switching, with no need to trace through disassembly addresses or reason about which ROM bank was switched in. This caught a real bug: a bank-1 assumption for the title screen tile offsets looked plausible and decoded without error, but rendered the wrong tiles (63% match instead of 99.82%) until it was checked this way. Reach for the disassembly text only when this technique cannot apply (there is nothing loaded into observable memory yet, or the logic itself, not an address, is what is unclear).

Shared boot sequences for these observation scripts live in `tools/sml_boot.py` (`boot_to_title`, `boot_to_gameplay`, `snapshot`/`restore` for save-state-based experiments). Use it instead of copy-pasting boot boilerplate into a new script. `restore` always ticks once after loading before sending input: a button pressed immediately after `load_state()` silently does not register, which cost a whole jump-timing sweep before it was caught.

When a single formula does not fit an observed trace (a curve fit leaves a systematic, not random, residual pattern), do not tune the formula further; check whether the game exposes its own internal state byte for the thing you are trying to derive (a phase, a mode, a counter) and split the fit along that boundary instead. This is what finally cracked gravity/jump physics after three earlier sessions failed to fit one accelerating curve to a jump arc: splitting the trace at the frame `0xC207` (an already-known "rising/falling" byte) flipped state, rather than at whatever frame a formula implied, is what made the pieces fit cleanly (see `docs/reference/physics.md`). The game's own state is a stronger source of truth than a fit's shape.

A format read off one instance can be underdetermined even when it fits that instance perfectly. Decode a second instance of the same thing before trusting the reading. The object record's `y` byte read cleanly as a row across all 37 of World 1-1's records and all 46 of World 1-2's, and World 1-3 then put nine records at rows 66 to 77 on a 16-row playfield: the byte is two nibbles, a row and a horizontal offset, and 1-1 could not show that because it only ever uses offsets 0 and 8. Where the data has siblings (other levels, other screens, other lists), the cheapest check on a format is the next sibling, and it is worth doing before building on the reading.

Two techniques worth reaching for when a byte's meaning is ambiguous, both cheaper than reasoning about it. First, patch a scratch copy of the ROM in a temporary directory and see what changes: clearing one bit in one record proved that bit suppresses the record, at exactly the predicted column, where the byte pattern alone was only a correlation. Second, the camera only scrolls while Mario moves, so releasing right freezes it, and any screen coordinate that keeps changing after that is the object's own motion; that is what separated a walk speed, a fall, and two lifts from the scrolling they were mixed with. Both are in `docs/reference/objects.md`.

Measuring a constant from the cartridge and wiring it into the engine are two different jobs; finishing the first does not mean the second is safe to do quietly. Before committing a newly-derived physics/behavior model, simulate it end to end (a scripted scenario in a throwaway `examples/*.rs`, run and read, then deleted) and compare the shape against the traced reference, not just against the unit tests, which only check what they were written to check. Implementing the jump physics model this way caught two real bugs a fit alone could not show: one regime routed through the wrong deceleration constant (a 40px jump instead of 24px), and a constant whose per-trial fit was individually reasonable but undershot once actually simulated across a full arc (fixed by using total distance over total duration instead, sturdier than differentiating a noisy trace twice). Neither would have been caught by only checking that the numbers matched their own source measurements.

When a reference capture of the whole screen exists, compare the whole screen, not the part you changed. A test scoped to the change passes for the same reason the change looked right, and it will not see what has been wrong all along. Two bugs on the same day: the status bar's blank cell was set to a tile id that draws empty in World 1 and draws water in World 2, and a pixel-for-pixel test against a World 1 capture passed it; and the entire playfield had been drawn sixteen pixels too high, hidden behind the status bar with a white band along the bottom, ever since the game started rendering levels. Diffing all 23040 pixels of our frame against the emulator capture of the same frame found both, and reporting the mismatch per 8x8 cell said exactly where. That comparison is now a test (`the_opening_frame_matches_the_emulator_capture`): every cell has to match except the ones Mario stands in.

## Communication and style rules (hard constraints)

These are not preferences. Treat a violation as a bug.

- No em-dashes anywhere: not in chat, comments, commit messages, markdown, blog posts, or code. Use commas, parentheses, or colons.
- No AI filler vocabulary. Banned words include: delve, robust, tapestry, navigate (as a metaphor), seamless, leverage (as a verb), realm, testament, boilerplate-speak. Say the plain thing.
- No AI sentence tics. Do not use the "it is not X, it is Y" contrast (or "not a bug, a feature", and similar). Do not write two-part antithesis sentences for rhythm. Do not use "quietly", "honest", "the nice thing", "the whole point", "it turns out" as filler. State the point directly.
- Fancy language must be earned by the subject. Plain topics (basic physics, a counter, a loader) get plain writing. If a sentence would sound bizarre said aloud about a small technical task, rewrite it.
- KISS. Prefer the simple, direct solution. Small functions, clear names.
- Minimal comments. Code should read on its own. Comment only when the "why" is not obvious from the code.
- Be direct in writing. State outcomes plainly. If something failed, say so with the evidence.

## Environment and tooling

- OS: Arch Linux.
- System packages: install with `shelly`. Do not use pacman, yay, or apt directly in scripts.
- Python: use `uv` exclusively. Never invoke bare `python`/`pip`.
- Node.js is forbidden. No npm, no npx, no JS build tools. The blog is hand written HTML, CSS, and JS.
- Rust is the implementation language. Use stable `cargo`.
- Python is fine, and often the faster choice, for prototyping and trying things out: hunting a ROM format, driving PyBoy, measuring a constant, checking whether an idea holds. Reach for it freely there. The finished thing is always Rust. Anything that survives as part of the product (parsing the cartridge, decoding assets, game logic) gets ported once it is understood, and the Python that found it either goes away or stays as the observation tool it always was.
- The dividing line that keeps coming up: reading the cartridge's bytes belongs in Rust, observing a running emulator stays Python (there is no Game Boy emulator in this project, and those scripts are measuring instruments, not shipped code).
- Whether asset extraction happens at build time (writing a gitignored file) or at runtime (loading straight from the ROM) is deliberately undecided. Rian does not consider it important; pick whichever suits the task and do not treat either as settled policy.

## Git and commits

- Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`, `refactor:`, `ci:`).
- Commit often. Branch per vertical slice or task. Merge back when a slice is playable.
- Never include Anthropic emails or Anthropic attribution in commits. No `Co-Authored-By` for the assistant. Author is always Rian Goossens <rian.goossens@gmail.com>.
- Pushing to `origin main` is a standing, pre-authorized part of this workflow, not a separate action to hold back on or ask about mid-task. A task or blog post is not done while it only exists as a local commit; push it. See `.claude/skills/git-github` for the full workflow.

## The ROM

- `super_mario_land.gb` and any extracted assets are gitignored. Never commit them.
- The ROM must pass a hash check before any tool consumes it. Expected Super Mario Land (World) v1.0:
  - SHA-1 `418203621b887caa090215d97e3f509b79affd3e`
  - MD5 `b259feb41811c7e4e1dc200167985c84`
  - CRC32 `2c27ec70`
- The file currently in the tree matches these hashes (verified 2026-07-22). The `verify-rom` command must still enforce them before any extraction, so a swapped file is caught.

## How work flows

1. `IMPROVEMENTS.md` at the repo root is the user's live inbox. Anything in it is handled first, before anything else. See `.claude/skills/improvements`.
2. Open GitHub issues authored by Rian come next, before any plan task. Hard-check the author so only Rian's own issues count. We advance them with a comment; by default we add an `awaiting-review` label and let Rian close, but if the issue body explicitly authorized closing we close it ourselves. Rian hands an issue back by removing the label (comment authorship is not a signal, since our comments post under his account). See `.claude/skills/github-issues`.
3. The plan of record is `docs/GRAND_MASTER_PLAN.md`. It is organized into playable vertical slices (milestones). Every task is a markdown checkbox.
4. Development happens one task at a time through the task execution skill. The user triggers it with `/goal`. Order: inbox, then Rian's issues, then the plan.
5. Everything is tested. See `.claude/skills/testing-validation`.
6. When a milestone or major task lands, publish a dev blog post. See `.claude/skills/dev-blog`.
7. Periodically run the self-improvement skill to keep this file and the skills current.

## Skills index

- `improvements` drain the user's `IMPROVEMENTS.md` inbox; handled before anything else.
- `github-issues` work Rian's own open GitHub issues (hard-checked author) before plan tasks; comment and label, never close.
- `git-github` version control workflow with the `gh` CLI.
- `grand-master-plan` maintain the living plan of vertical slices.
- `task-execution` pick one task, complete it, update the plan. Chained via `/goal`.
- `testing-validation` write, run, and manage tests.
- `dev-blog` write and publish posts to the GitHub Pages blog.
- `self-improvement` review and refine CLAUDE.md, skills, and sub-agents.

## Architecture intent (for the reproduction phase)

- Separate concerns: core game logic (no rendering), rendering/frontend, input, and asset loading are distinct modules.
- Deterministic core so the game state can be stepped and snapshotted for tests and screenshots.
- Provide a headless screenshot path so any game state can be rendered to a PNG for visual testing and the blog.
- The game loop is a headless, deterministic object (`src/game.rs`): it steps a frame from a button snapshot and renders to a framebuffer, with no window or clock. The windowed frontend (behind the `gui` feature) is a thin shell over it. The window is never a testing surface: Rian does not run it, so every feature must be verifiable headlessly through `Game` tests, scripted input, golden images, and `sml play`.
- Constants (gravity, jump velocity, speeds) live in named, documented places, sourced from the reference and cited.
