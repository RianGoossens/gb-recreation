# Super Mario Land, in Rust

A native, readable Rust reproduction of the Game Boy game Super Mario Land. Not an emulator: the game logic, physics, and rendering are reimplemented as clean Rust you can read and modify to build your own levels and mechanics.

Status: workspace bootstrap. Game development is tracked in [docs/GRAND_MASTER_PLAN.md](docs/GRAND_MASTER_PLAN.md) and driven one task at a time.

## How this repo is built

Most of the code, docs, plan, and blog posts here are written by Claude Code running as an agent, working through the skills in `.claude/skills/` one task at a time, with a human steering and reviewing. The repo is an experiment in whether that kind of agent workflow can carry a job as big as reproducing a full Game Boy game in clean Rust.

So expect some AI slop and rough edges. This is an honest attempt at a good agent workflow for a hard problem, nothing more.

## Requirements

| Tool | Purpose | Install |
|------|---------|---------|
| Rust (stable) + cargo | build and test the game | `shelly install rustup` then `rustup default stable` |
| git + gh CLI | version control and GitHub | `shelly install git github-cli` |
| uv | any Python scripting (optional) | `shelly install uv` |

Node.js is intentionally not used anywhere in this project.

The original ROM (`super_mario_land.gb`) is NOT included and is gitignored. You must supply your own legally obtained dump of Super Mario Land (World) v1.0. It is validated by hash before any tooling reads it (see below).

## Running the game

The windowed frontend is behind the `gui` feature, so the default build stays
headless and dependency-free. To play the current test level:

```sh
cargo run --release --features gui -- run
```

Controls: arrow keys move, X jumps (Z is the B button), Escape quits.

Without the feature, the binary still offers the headless commands below
(`verify-rom`, `extract-tiles`, `screenshot`, `render-title`). See `sml` with
no arguments for the list.

## Running the tests

```sh
cargo test
```

CI runs `cargo test` on every push (see [.github/workflows/ci.yml](.github/workflows/ci.yml)).

## Generating screenshots

The game exposes a headless screenshot path so any state can be rendered to a PNG without opening a window. This is used for visual testing and for the dev blog.

```sh
cargo run --release -- screenshot --state title --out shot.png
```

(The exact subcommands are added as the corresponding milestones land; see the plan.)

## Verifying the ROM

Before any tool consumes the ROM it must match Super Mario Land (World) v1.0:

- SHA-1 `418203621b887caa090215d97e3f509b79affd3e`
- MD5 `b259feb41811c7e4e1dc200167985c84`
- CRC32 `2c27ec70`

Check it:

```sh
sha1sum super_mario_land.gb
md5sum super_mario_land.gb
```

## The dev blog

Progress is logged as a live technical blog published to GitHub Pages from `docs/blog/`. Plain HTML, CSS, and JS, no frameworks. It deploys automatically on push (see [.github/workflows/pages.yml](.github/workflows/pages.yml)).

## How this repo is driven

This is an autonomous agent workspace. Guidelines live in [CLAUDE.md](CLAUDE.md). Work happens through skills in `.claude/skills/`:

- Plan of record: `docs/GRAND_MASTER_PLAN.md`, organized as playable vertical slices.
- Do the next task: run the task execution skill, chained with `/goal` (for example `/goal run the task execution skill 5 times`).
- Reference for the original game logic (used sparingly): the `kaspermeerts/supermarioland` disassembly.

## License

Code in this repository is the authors' original work. Super Mario Land is a trademark of Nintendo. No Nintendo ROM data or copyrighted assets are distributed here.
