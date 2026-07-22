---
name: testing-validation
description: Write, run, and manage tests for the Rust reproduction. Use whenever adding behavior, before checking a plan task done, or when validating physics, collision, and rendering. Covers unit tests, golden-image tests, and headless screenshot checks.
---

# Testing and validation

Everything we build is tested. A plan task with testable behavior is not done until it has tests and they pass.

## Layers

1. Unit tests: pure logic, physics math, collision resolution, state machines. Fast, deterministic, in `#[cfg(test)]` modules next to the code or in `tests/`.
2. Golden-image tests: render a known game state headlessly to a framebuffer/PNG and compare against a committed reference image. Used for title screen, level rendering, sprites.
3. Scripted-input tests: feed a fixed sequence of inputs to the deterministic core, step N frames, assert on state or on a golden frame. Used for physics and gameplay.

## Rules

- The game core is deterministic: same inputs and same start state produce the same result. No wall-clock or RNG without a seedable source. This is what makes tests and screenshots reliable.
- Physics and collision constants get their own tests that pin the values sourced from the `10yard/supermarioland` reference, so a regression is caught immediately.
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
