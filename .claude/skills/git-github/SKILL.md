---
name: git-github
description: Version control workflow for this project using git and the gh CLI. Use when committing, branching, merging, pushing, opening PRs, or setting up the GitHub remote. Enforces conventional commits and the no-Anthropic-attribution rule.
---

# Git and GitHub workflow

## Non-negotiables

- Conventional Commits. Type prefix required: `feat`, `fix`, `docs`, `test`, `chore`, `refactor`, `ci`, `perf`, `style`.
- No em-dashes in commit messages or PR text.
- Never add Anthropic emails or assistant attribution. No `Co-Authored-By` for the assistant. The author is always the repo's configured user.
- Commit often. Small, focused commits beat one large one.

## One-time setup

Confirm identity is set on the repo (not Anthropic):

```sh
git config user.name "Rian Goossens"
git config user.email "rian.goossens@gmail.com"
git config commit.gpgsign false
```

Create the GitHub remote if it does not exist yet:

```sh
gh repo view >/dev/null 2>&1 || gh repo create gb-recreation --private --source=. --remote=origin
```

Use `--public` instead of `--private` only if the user asks. Note: GitHub Pages on a private repo needs a paid plan; if the blog must be live and the repo is private, tell the user.

## Commit message format

```
type(optional-scope): short imperative summary

Optional body explaining the why, wrapped at ~72 columns.
```

Examples:
- `feat(physics): add gravity and ground collision`
- `test(collision): cover ceiling-bump cases`
- `docs(blog): publish title-screen post`

## Branching

- One branch per vertical slice or discrete task: `slice/m2-walking-physics`, `task/verify-rom`.
- Work on the branch, commit as you go, merge back to `main` when the slice is playable and tests pass.

```sh
git switch -c slice/m2-walking-physics
# ... work, commit ...
git switch main
git merge --no-ff slice/m2-walking-physics
```

## Everyday loop

```sh
git status
git add -A
git commit -m "feat(scope): summary"
git push -u origin HEAD
```

The push is not optional or something to ask permission for mid-task: CLAUDE.md pre-authorizes it as a standing part of this workflow (see its "Git and commits" section). A run that ends with commits sitting only in the local tree, unpushed, is not finished. Push at the end of each task (or each self-contained batch of tasks in a chained run), same as the commit itself.

Before committing, run the tests (see the testing-validation skill). Do not commit a red tree unless the commit is explicitly a checkpoint and says so.

## Pull requests (when working on a shared branch)

```sh
gh pr create --title "feat(scope): summary" --body "What and why. No em-dashes."
gh pr checks   # watch CI
gh pr merge --squash --delete-branch
```

## Sanity check before pushing

- `git log --format='%an <%ae>' -1` shows the repo user, never an Anthropic address.
- Commit message has a conventional type prefix and no em-dash.
- The ROM and extracted assets are not staged (`git status` should never list `*.gb`).
