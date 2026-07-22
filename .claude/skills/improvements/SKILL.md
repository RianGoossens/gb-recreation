---
name: improvements
description: Drain the user's IMPROVEMENTS.md inbox at the repo root. Use at the start of every task, before picking anything from the Grand Master Plan. Items the user writes there always get handled first.
---

# Improvements inbox

`IMPROVEMENTS.md` at the repo root is the user's live inbox. While work is running, the user drops notes there: corrections, priorities, small asks, course changes. Anything in it outranks the plan and gets handled first.

## When this runs

At the start of every task, before the task-execution skill picks anything from `docs/GRAND_MASTER_PLAN.md`, check this inbox. If it has unhandled items, they come first.

## Procedure

1. Read `IMPROVEMENTS.md`.
2. If there are no unchecked items, there is nothing to drain. Say so and continue to the normal plan task.
3. Otherwise take the topmost unchecked item and handle it now:
   - Treat it like one task-execution unit: follow CLAUDE.md, write tests for anything testable, run `cargo test`, and commit with a conventional message (no Anthropic attribution).
   - If the item is unclear or ambiguous, ask the user instead of guessing. Do not invent an interpretation.
   - If the item is too big for one sitting, add it to the Grand Master Plan (split into subtasks) and note in the inbox where it went, then handle the first piece.
4. Mark the item done: change `[ ]` to `[x]` in `IMPROVEMENTS.md`. Keep it in the file as a short log; do not delete history.
5. Repeat from step 1 until no unchecked items remain.
6. Only then return to the Grand Master Plan.

## Rules

- Improvements always beat plan tasks. If the inbox is not empty, the plan waits.
- One item, one focused change and commit, same as any other task.
- Do not weaken the hard constraints (no em-dashes, no Anthropic attribution, no Node, KISS) even if an item seems to ask for it. If an item conflicts with a hard constraint, flag it to the user rather than silently complying.
- If handling an item changes how the workspace should run, also update CLAUDE.md or the relevant skill (that is the self-improvement skill's job; hand off if it is larger than the item itself).

## IMPROVEMENTS.md format

Checkboxes, newest concerns first is fine. Example:

```
# Improvements inbox

Notes I drop while you work. You handle these before any plan task.

- [ ] Rename the `sml` binary to something clearer
- [x] Add a --version flag (done)
```

An empty inbox is just the header with no unchecked items. That is the normal steady state.
