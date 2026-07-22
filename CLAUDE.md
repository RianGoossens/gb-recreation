# CLAUDE.md

Project guidelines for the Rust reproduction of Super Mario Land. Read this before every work session. Sub-agents inherit these rules.

## What this project is

A native Rust reproduction of the Game Boy title Super Mario Land. This is NOT an emulator. We reimplement the game's logic, physics, and rendering as clean, readable Rust that a person can modify to build custom levels or mechanics. The original assembly is our reference, not our template. We translate intent, not opcodes.

## Reference material

Primary manual: the `10yard/supermarioland` GitHub repository. It maps the original assembly logic, physics constants, and memory layout. Reimplement that mapped logic piece by piece. Do not blindly disassemble the ROM from scratch when a mapping already exists.

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
  - SHA-1 `b8449c25608d538e124707dc8e5d0b49cb376e19`
  - MD5 `2e16d41584c205ba8fcd07fb3b22b644`
  - CRC32 `B81DF11A`
- The file currently in the tree does NOT match these hashes. Do not extract from it until it is replaced with a verified dump. The verification task lives in the Grand Master Plan.

## How work flows

1. The plan of record is `docs/GRAND_MASTER_PLAN.md`. It is organized into playable vertical slices (milestones). Every task is a markdown checkbox.
2. Development happens one task at a time through the task execution skill. The user triggers it with `/goal`.
3. Everything is tested. See `.claude/skills/testing-validation`.
4. When a milestone or major task lands, publish a dev blog post. See `.claude/skills/dev-blog`.
5. Periodically run the self-improvement skill to keep this file and the skills current.

## Skills index

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
- Constants (gravity, jump velocity, speeds) live in named, documented places, sourced from the reference and cited.
