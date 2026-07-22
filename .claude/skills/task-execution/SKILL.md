---
name: task-execution
description: Execute exactly one concrete task from the Grand Master Plan, then update the plan. Use when the user runs /goal or asks to advance the project. Designed to be chained (for example "run the task execution skill 5 times").
---

# Task execution engine

One invocation completes exactly one concrete task from `docs/GRAND_MASTER_PLAN.md` and leaves the repo better and consistent. This skill is meant to be chained via `/goal` (for example `/goal run the task execution skill 5 times`), so each run must be self-contained and end in a clean state.

## Procedure

0. Drain the improvements inbox first. Check `IMPROVEMENTS.md` at the repo root (see the improvements skill). If it has any unchecked item, handle the topmost one this run instead of a plan task, mark it `[x]`, and stop there. The plan waits until the inbox is empty.
0b. Then check GitHub issues authored by Rian (see the github-issues skill). Hard-check the author: only Rian's own issues count. An issue needs attention when it is fresh (no `awaiting-review` label) or Rian has replied on it. If one does, handle the oldest this run instead of a plan task: do the work, comment what you did, add the `awaiting-review` label, and stop. Never close issues yourself; Rian closes them when satisfied. Issues rank above the plan and below the inbox.
1. Read `docs/GRAND_MASTER_PLAN.md`. Pick whichever unblocked task makes the most sense to do next, judged by dependencies and value. Tasks are not strictly ordered, so list position does not decide this. Prefer finishing an in-progress `[~]` task over starting a new one.
2. If the chosen task is too big for one run, split it in the plan and take the first subtask instead.
3. Mark the task `[~]`.
4. Do the work:
   - Follow CLAUDE.md style and architecture rules.
   - Prefer building from observed behavior and emulator comparison. Consult the `kaspermeerts/supermarioland` disassembly sparingly, only to settle a specific number or mechanic you cannot pin down otherwise. Cite what you take.
   - Write tests alongside the code (see the testing-validation skill). No task counts as done without tests when it has testable behavior.
5. Validate: run `cargo test` (and `cargo build`/`cargo run` for a screenshot when relevant). For visual work, generate a screenshot and inspect it.
6. Update the plan: mark the task `[x]`. Add any follow-up tasks that surfaced.
7. Commit with a conventional-commit message (see the git-github skill). One task, one focused commit (or a small set).
8. If the task was a major task or completed a milestone, trigger the dev-blog skill to publish a post.

## Picking a task well

- Task order is a guide, not a rule. Milestones are grouped so earlier slices unlock later ones, which usually makes earlier work the higher-value pick, but nothing stops you taking a later task when it is unblocked and sensible.
- What actually gates a task is real dependencies (it needs another task's output) and blockers, not its position in the list. Skip blocked tasks and take the next workable one.
- If the sensible next work is genuinely blocked, say so plainly and either pick the smallest unblocking task or surface the blocker to the user.

## Definition of done for one run

- The picked task is `[x]` in the plan, or split with its first subtask `[x]`.
- `cargo test` passes.
- Work is committed with a clean, conventional message and no Anthropic attribution.
- A blog post exists if the task warranted one.
- The working tree is clean.

## Chaining

When asked to run N times, repeat the whole procedure N times, each time re-reading the plan so choices reflect the latest state. Stop early and report if you hit a hard blocker (for example the missing verified ROM) rather than marking blocked work done.
