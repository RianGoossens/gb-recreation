---
name: task-execution
description: Execute exactly one concrete task from the Grand Master Plan, then update the plan. Use when the user runs /goal or asks to advance the project. Designed to be chained (for example "run the task execution skill 5 times").
---

# Task execution engine

One invocation completes exactly one concrete task from `docs/GRAND_MASTER_PLAN.md` and leaves the repo better and consistent. This skill is meant to be chained via `/goal` (for example `/goal run the task execution skill 5 times`), so each run must be self-contained and end in a clean state.

## Procedure

1. Read `docs/GRAND_MASTER_PLAN.md`. Pick the topmost unchecked task that is not blocked. Prefer finishing an in-progress `[~]` task over starting a new one.
2. If the chosen task is too big for one run, split it in the plan and take the first subtask instead.
3. Mark the task `[~]`.
4. Do the work:
   - Follow CLAUDE.md style and architecture rules.
   - Consult the `10yard/supermarioland` reference for any original-game logic, physics, or memory details. Cite what you use.
   - Write tests alongside the code (see the testing-validation skill). No task counts as done without tests when it has testable behavior.
5. Validate: run `cargo test` (and `cargo build`/`cargo run` for a screenshot when relevant). For visual work, generate a screenshot and inspect it.
6. Update the plan: mark the task `[x]`. Add any follow-up tasks that surfaced.
7. Commit with a conventional-commit message (see the git-github skill). One task, one focused commit (or a small set).
8. If the task was a major task or completed a milestone, trigger the dev-blog skill to publish a post.

## Picking a task well

- Respect milestone order. Do not start Milestone 3 work while Milestone 1 is unfinished, unless the earlier work is genuinely blocked (for example the ROM hash check).
- If everything in the current milestone is blocked, say so plainly and pick the smallest unblocking task, or surface the blocker to the user.

## Definition of done for one run

- The picked task is `[x]` in the plan, or split with its first subtask `[x]`.
- `cargo test` passes.
- Work is committed with a clean, conventional message and no Anthropic attribution.
- A blog post exists if the task warranted one.
- The working tree is clean.

## Chaining

When asked to run N times, repeat the whole procedure N times, each time re-reading the plan so choices reflect the latest state. Stop early and report if you hit a hard blocker (for example the missing verified ROM) rather than marking blocked work done.
