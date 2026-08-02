---
name: testing-validation
description: Write, run, and manage tests for the Rust reproduction. Use whenever adding behavior, before checking a plan task done, or when validating physics, collision, and rendering. Covers unit tests, golden-image tests, and headless screenshot checks.
---

# Testing and validation

Everything we build is tested. A plan task with testable behavior is not done until it has tests and they pass.

## The window is not a testing surface

Rian does not run the GUI, so "open the window and look" is never a valid check. The game must be verifiable entirely headlessly. Two rules follow:

- Game logic lives in the headless `Game` object (`src/game.rs`), which steps a frame from a button snapshot and renders to a framebuffer. It never touches a window or the clock. The windowed frontend (behind the `gui` feature) is a thin shell over it and holds no logic worth testing.
- Every gameplay or visual feature must be reachable through headless paths: unit tests on `Game`, scripted-input assertions, golden-image tests, and the `sml play <out.png> [frames] [keys]` command that renders a scripted run to a PNG you can open and inspect. If a feature can only be seen by running the window, it is not done.

Golden images use our own levels and scenes (for example `Game::demo_level`), never extracted game data, so they are safe to commit and run in CI.

## Layers

1. Unit tests: pure logic, physics math, collision resolution, state machines. Fast, deterministic, in `#[cfg(test)]` modules next to the code or in `tests/`.
2. Golden-image tests: render a known game state headlessly to a framebuffer/PNG and compare against a committed reference image. Used for title screen, level rendering, sprites.
3. Scripted-input tests: feed a fixed sequence of inputs to the deterministic core, step N frames, assert on state or on a golden frame. Used for physics and gameplay.

## Rules

- The game core is deterministic: same inputs and same start state produce the same result. No wall-clock or RNG without a seedable source. This is what makes tests and screenshots reliable.
- Physics and collision constants get their own tests that pin their values (derived from observed behavior, or from the `kaspermeerts/supermarioland` disassembly when consulted), so a regression is caught immediately.
- Keep golden images small and committed under version control (they are our own renders, not ROM data). Store them under `tests/golden/`.
- When a golden image legitimately changes, regenerate it deliberately and review the diff before committing.

## Running

```sh
cargo test              # everything
cargo test physics      # filter by name
cargo test -- --nocapture   # see println output
```

## Golden-image workflow

1. Build the state in a test (or via the screenshot command).
2. Render headlessly to a PNG.
3. Compare to `tests/golden/<name>.png`. Fail on any pixel difference beyond a tiny tolerance.
4. To (re)establish a golden, run the test's regenerate path, eyeball the PNG, commit it.

Provide a helper so a missing golden fails loudly with instructions rather than silently passing.

## Screenshot-based visual checks

The game exposes a headless screenshot command (see README). Use it during development to render a state to PNG, then look at it. Compare against a real emulator when validating faithfulness. Curated comparison shots go into the blog.

## Before checking a plan task done

- `cargo test` is green.
- New behavior has at least one test that would fail if the behavior broke.
- For visual work, a golden-image or screenshot check exists and passed.

## Simulate a new physics/behavior model before committing it, not just after

Unit tests check what they were written to check; they will not catch a
model that is internally consistent but shaped wrong. When implementing a
constant or a model derived from observation (a physics regime, a state
machine), before committing: build a throwaway scripted scenario (an
`examples/*.rs`, or a `cargo test -- --nocapture` with prints), run it, and
compare the actual shape against the traced reference the constants came
from. Delete the scratch file once satisfied; do not commit it. This caught
two real bugs implementing the jump physics redesign that passing unit
tests alone did not: a regime routed through the wrong constant (produced a
40px jump instead of the traced 24px), and a fall-acceleration constant
whose per-trial fit looked fine in isolation but undershot once the full
arc was actually simulated (see `docs/reference/physics.md`). Checking
"does it compile and pass its own tests" is not the same question as "does
running it look like the thing it was supposed to reproduce."

## Delete the generated input and see if the tests still run

A test that reads a generated, gitignored artifact passes against whatever
was last generated, however old that is. `assets/extracted/level_*.txt` is
written by `sml extract-level`, and a change to what extraction produces
reached no test at all until somebody regenerated the files by hand: World
1-3 gained its boss and the spawn-count assertion that should have caught it
stayed green across two commits.

The control is to move the artifact away and run the suite. Doing that here
found the larger half of the problem. With every extracted level removed the
suite finished in 0.28 seconds instead of 28, because `session::world_levels`
read only those files, so a checkout with the ROM but no extraction run got
the placeholder campaign and every walkthrough test returned early without
reporting anything. A test that skips and a test that passes produce the same
summary line, so read the wall-clock time next to the count: a suite that got
much faster after a change to test inputs has stopped doing something.

Prefer deriving a fixture from the source inside the test
(`assets::level::level_from_rom`) over reading what a command wrote. Where a
test genuinely has to skip (no ROM in the tree), make the skip depend on the
source being absent rather than on a derived file being absent, so the only
thing that can silence the suite is the one condition a checkout is allowed
to be in.
