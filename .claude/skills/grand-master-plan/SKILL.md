---
name: grand-master-plan
description: Maintain the living Grand Master Plan at docs/GRAND_MASTER_PLAN.md. Use when reviewing progress, marking tasks done, adding or splitting tasks, or reshaping milestones. The plan is organized into playable vertical slices.
---

# Grand Master Plan maintenance

The plan lives at `docs/GRAND_MASTER_PLAN.md`. It is the single source of truth for what is left to do. Completing every checkbox equals a finished project.

## Structure

- Milestones are playable vertical slices. Each one ends in something you can run and watch (boot to title, walk in 1-1, stomp a Goomba, and so on).
- Tasks are markdown checkboxes under a milestone.
- `[ ]` todo, `[~]` in progress, `[x]` done.

## When to touch the plan

- Starting a task: flip it to `[~]`.
- Finishing a task: flip it to `[x]`, but only after it is tested.
- A task is bigger than one sitting: split it into subtasks in place, leave the parent until all children are done.
- New work surfaces: add a task in the right milestone, or in Backlog if it does not fit yet.
- A milestone is complete: sanity-check the next milestone still makes sense before moving on.

## Rules

- Keep it honest. Do not mark something done to look productive. A checkbox is a claim that it works and is tested.
- "Tested" means reachable the way a real user would reach it, not just exercised through the Rust API in a unit test. Milestone 7's "Hooks or data-driven config for tuning mechanics" was checked off with `Tuning::from_text` fully implemented and unit-tested, but no CLI command ever called it, so a level author following the moddability blog post's own example could not actually use it without writing Rust. Worse, `Session` silently discarded any tuning applied after construction on every level transition or restart, a bug a unit test that only checked the first level would not catch. Before checking a user-facing capability done, trace it all the way to the interface a user actually touches (a CLI flag, a window, a file format), not just to a function that could theoretically be called.
- No em-dashes, no filler words (see CLAUDE.md).
- One concept per checkbox. If a box hides three things, split it.
- Every milestone should end with a "blog post" task so progress gets logged.

## How this pairs with other skills

- The task-execution skill reads this plan to pick the next task and writes back the result.
- The testing-validation skill gates whether a box can be checked.
- The self-improvement skill reviews whether the milestone shape still fits reality.

## Editing mechanics

Read the file, make the smallest edit that reflects reality, save. Do not rewrite the whole document to change one box. When adding subtasks, indent them under the parent:

```
- [~] Walking physics
  - [x] acceleration and max speed
  - [ ] friction and skid
```
