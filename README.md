# Super Mario Land, in Rust

A native, readable Rust reproduction of the Game Boy game Super Mario Land. Not an emulator: the game logic, physics, and rendering are reimplemented as clean Rust you can read and modify to build your own levels and mechanics.

Status: workspace bootstrap. Game development is tracked in [docs/GRAND_MASTER_PLAN.md](docs/GRAND_MASTER_PLAN.md) and driven one task at a time.

## Requirements

| Tool | Purpose | Install |
|------|---------|---------|
| Rust (stable) + cargo | build and test the game | `shelly install rustup` then `rustup default stable` |
| git + gh CLI | version control and GitHub | `shelly install git github-cli` |
| uv | any Python scripting (optional) | `shelly install uv` |

Node.js is intentionally not used anywhere in this project.

The original ROM (`super_mario_land.gb`) is NOT included and is gitignored. You must supply your own legally obtained dump of Super Mario Land (World) v1.0. It is validated by hash before any tooling reads it (see below).

## Running the game

```sh
cargo run --release
```

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

- SHA-1 `b8449c25608d538e124707dc8e5d0b49cb376e19`
- MD5 `2e16d41584c205ba8fcd07fb3b22b644`
- CRC32 `B81DF11A`

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
- Reference for the original game logic: the `10yard/supermarioland` repository.

## License

Code in this repository is the authors' original work. Super Mario Land is a trademark of Nintendo. No Nintendo ROM data or copyrighted assets are distributed here.
