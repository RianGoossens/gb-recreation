# CLAUDE.md

Project guidelines for the Rust reproduction of Super Mario Land. Read this before every work session. Sub-agents inherit these rules.

## What this project is

A native Rust reproduction of the Game Boy title Super Mario Land. This is NOT an emulator. We reimplement the game's logic, physics, and rendering as clean, readable Rust that a person can modify to build custom levels or mechanics. The original assembly is our reference, not our template. We translate intent, not opcodes.

## Reference material

Secondary reference: the `kaspermeerts/supermarioland` disassembly. It maps some of the original assembly, physics constants, and memory layout. Lean on it as little as possible: prefer building clean Rust from observed behavior, tests, and screenshots against a real emulator. Reach for the disassembly only to settle a specific number or mechanic you cannot pin down otherwise, and cite what you take.

## Communication and style rules (hard constraints)

These are not preferences. Treat a violation as a bug.

- No em-dashes anywhere: not in chat, comments, commit messages, markdown, blog posts, or code. Use commas, parentheses, or colons.
- No AI filler vocabulary. Banned words include: delve, robust, tapestry, navigate (as a metaphor), seamless, leverage (as a verb), realm, testament, boilerplate-speak. Say the plain thing.
- KISS. Prefer the simple, direct solution. Small functions, clear names.
- Minimal comments. Code should read on its own. Comment only when the "why" is not obvious from the code.
- Be direct in writing. State outcomes plainly. If something failed, say so with the evidence.

## Environment and tooling

- OS: Arch Linux.
- System packages: install with `shelly`. Do not use pacman, yay, or apt directly in scripts.
- Python (only if genuinely needed for scripting): use `uv` exclusively. Never invoke bare `python`/`pip`.
- Node.js is forbidden. No npm, no npx, no JS build tools. The blog is hand written HTML, CSS, and JS.
- Rust is the implementation language. Use stable `cargo`.

## Git and commits

- Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`, `refactor:`, `ci:`).
- Commit often. Branch per vertical slice or task. Merge back when a slice is playable.
- Never include Anthropic emails or Anthropic attribution in commits. No `Co-Authored-By` for the assistant. Author is always Rian Goossens <rian.goossens@gmail.com>.
- See `.claude/skills/git-github` for the full workflow.

## The ROM

- `super_mario_land.gb` and any extracted assets are gitignored. Never commit them.
- The ROM must pass a hash check before any tool consumes it. Expected Super Mario Land (World) v1.0:
  - SHA-1 `418203621b887caa090215d97e3f509b79affd3e`
  - MD5 `b259feb41811c7e4e1dc200167985c84`
  - CRC32 `2c27ec70`
- The file currently in the tree matches these hashes (verified 2026-07-22). The `verify-rom` command must still enforce them before any extraction, so a swapped file is caught.

## How work flows

1. `IMPROVEMENTS.md` at the repo root is the user's live inbox. Anything in it is handled first, before anything else. See `.claude/skills/improvements`.
2. Open GitHub issues authored by Rian come next, before any plan task. Hard-check the author so only Rian's own issues count. We advance them with a comment and an `awaiting-review` label and never close them; Rian closes when satisfied, and if he replies the issue comes back to us. See `.claude/skills/github-issues`.
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
