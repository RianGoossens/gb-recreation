---
name: dev-blog
description: Write and publish posts to the live technical dev blog on GitHub Pages. Use when a major task or milestone lands. Produces self-contained HTML posts with dates, timestamps, and screenshots; no frameworks, no local line-number references.
---

# Live dev blog

A live technical logbook published to GitHub Pages from `docs/blog/`. Plain HTML, CSS, and JS. No frameworks, no build step, no Node. It deploys automatically on push via `.github/workflows/pages.yml`.

## Where things live

- `docs/blog/index.html` the post listing (the blog home).
- `docs/blog/posts/YYYY-MM-DD-slug.html` one file per post, self-contained.
- `docs/blog/style.css` shared styles.
- `docs/blog/posts.json` machine-readable index the home page reads to list posts.
- `docs/blog/media/` screenshots and images used by posts, committed via Git LFS (see below).

## Images and Git LFS

Screenshots of the running game are fine to include in blog posts as commentary, and get committed under `docs/blog/media/`. That directory is tracked by Git LFS (`.gitattributes`), so image files stay out of the main history. Two rules still hold: never commit the ROM itself or raw extracted asset files (those stay gitignored), and the Pages workflow checks out with `lfs: true` so images resolve on the live site.

## Publishing checklist

1. Confirm the work being written about is actually done and tested.
2. Capture screenshots with the game's headless screenshot command. Save curated ones under `docs/blog/media/` (committed via Git LFS).
3. Create `docs/blog/posts/YYYY-MM-DD-slug.html` from the template below.
4. Add an entry to `docs/blog/posts.json` (title, date, slug, one-line summary).
5. Verify the home page lists it and the post opens.
6. Commit (`docs(blog): ...`) and push. The Pages workflow deploys it.

## Post rules

- Voice: fun, but serious. Write with personality and a light touch, but stay technically honest and precise. No forced jokes, no fluff.
- Byline: every post is authored by "Rian's AI Assistant". Put it in the post header next to the date.
- Real date and timestamp in the post header. Use the actual current date/time.
- Self-contained: a reader learns how the task was solved from the post alone.
- In depth: explain the problem, the approach, the tricky parts, and how it was validated. Show code snippets and screenshots.
- No references to local line numbers ("see line 15"). Refer to files, functions, and concepts by name.
- No em-dashes. No filler words. Direct, technical writing.
- Not self-referential. Write about the game and the engineering, never about the blog's own construction (no notes on frameworks, HTML/CSS, "built by hand", and so on). The reader does not care how the page was made.
- Vary the look. Do not run every post through the same rigid template. Reach for more than one image when it helps (a before/after pair, a two-up comparison, an action shot next to a still), and change up the structure so posts do not feel identical. Keep it interesting to read.
- Use the shared components in `style.css` where they fit, so variety is easy and consistent:
  - `<p class="lead">` a stronger opening paragraph.
  - `<div class="callout">...</div>` an aside, warning, or highlight.
  - `<blockquote class="pull">...</blockquote>` a pull quote to break up long text.
  - `<div class="two-up"><figure>...</figure><figure>...</figure></div>` two images side by side.
- Multiple images are encouraged. Each goes under `docs/blog/media/` via Git LFS.

## Post template

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>POST TITLE: SML in Rust devlog</title>
  <link rel="stylesheet" href="../style.css">
</head>
<body>
  <main class="post">
    <p class="back"><a href="../index.html">&larr; all posts</a></p>
    <h1>POST TITLE</h1>
    <p class="meta">By Rian's AI Assistant, published YYYY-MM-DD HH:MM (local)</p>

    <p>Lead paragraph: what this task was and why it mattered.</p>

    <h2>The problem</h2>
    <p>...</p>

    <h2>The approach</h2>
    <p>...</p>
    <pre><code>// a small, relevant snippet</code></pre>

    <h2>Validation</h2>
    <p>How it was tested. Screenshots.</p>
    <figure>
      <img src="../media/screenshot.png" alt="describe the image">
      <figcaption>What this shows.</figcaption>
    </figure>

    <h2>What is next</h2>
    <p>...</p>
  </main>
</body>
</html>
```

Note: the post title uses a regular hyphen or colon in text, never an em-dash. The example title separator above should be a colon in real posts.

## posts.json entry

```json
{ "title": "Booting to the title screen", "date": "2026-07-22", "slug": "2026-07-22-boot-to-title", "summary": "One line on what landed." }
```

Keep `posts.json` newest-first. The home page renders the list from it.
