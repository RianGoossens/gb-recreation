---
description: Advance the project by running the task execution skill, optionally N times (e.g. /goal run the task execution skill 5 times).
---

Drive the Grand Master Plan forward using the task-execution skill.

User request: $ARGUMENTS

Instructions:
- Invoke the `task-execution` skill to complete work from `docs/GRAND_MASTER_PLAN.md`.
- If the request names a count (for example "5 times"), repeat the full task-execution procedure that many times, re-reading the plan before each run so choices reflect the latest state.
- If no count is given, run it once.
- Each run must end clean: tests green, plan updated, work committed with a conventional-commit message and no Anthropic attribution, and a blog post published if the task warranted one.
- Stop early and report if you hit a hard blocker (for example the ROM failing its hash check) instead of marking blocked work done.
