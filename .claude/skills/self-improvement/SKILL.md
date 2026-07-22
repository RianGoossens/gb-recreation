---
name: self-improvement
description: Periodically review and refine the workspace itself. Use every few milestones or when friction shows up: update CLAUDE.md, sharpen skills, adjust the plan's shape, or propose new sub-agents as the project scales.
---

# Self-improvement

The workspace is not fixed. As the project grows, the guidelines and skills should get sharper. Run this skill on purpose, not constantly: after a milestone, or when the same friction keeps recurring.

## What to review

1. CLAUDE.md
   - Are the rules still accurate? Anything the project now does that is not written down?
   - Are the style constraints being followed? If a violation slipped through, tighten the wording.
   - Is the architecture intent still what we are actually building? Update it to match reality.

2. Skills (`.claude/skills/*`)
   - Is each skill's `description` precise enough to trigger at the right time and not otherwise?
   - Did any skill's steps go stale (a command changed, a path moved)? Fix it.
   - Is there a repeated task with no skill? Write one. Is there a skill nobody uses? Cut or merge it.

3. The plan (`docs/GRAND_MASTER_PLAN.md`)
   - Do the milestones still map to playable slices? Reshape if the game taught us something.
   - Are tasks the right size? Split the ones that keep overflowing a single run.

4. Sub-agents
   - As work parallelizes, consider dedicated agents (for example: an asset-extraction agent, a physics-tuning agent, a blog-writer agent). Only add one when there is real, repeated, separable work for it. Keep each agent's brief tight and consistent with CLAUDE.md.

## How to change things

- Make focused edits, not rewrites. Explain the change in the commit (`chore(skills): ...` or `docs: ...`).
- Do not weaken the hard constraints (no em-dashes, no Anthropic attribution, no Node, KISS). Strengthen or clarify only.
- When you add or change a skill, update the skills index in CLAUDE.md.

## Trigger points

- End of a milestone.
- After a run where a skill's instructions were wrong or missing.
- When the same mistake happens twice. Encode the fix so it does not happen a third time.

## Output

A short summary of what changed and why, plus the commits. If nothing needs changing, say that plainly and stop. Do not invent busywork.
