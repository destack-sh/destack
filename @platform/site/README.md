# @platform/site

Read about Destack on its website, documentation and blog at destack.sh.

## Overview

`@platform/site` is destack.sh: the landing page, the documentation and the blog, prerendered from `content/` with islands that hydrate.

The site builds from:

- `src/`: the pages, the reader and the landing page's figures.
- `content/`: the blog posts and the documentation, as Markdown.
- `build/`: the site's own build program, which prerenders the pages.
- `scripts/`: the generators of the pricing tables and headwords, and the site's own tests.

## Reference

### Scripts

| Script           | What it does                                                       |
| ---------------- | ------------------------------------------------------------------ |
| `build`          | generates, then prerenders the site into `.output`                 |
| `generate`       | writes the pricing tables and headwords the pages read             |
| `generate-check` | refuses generated files that differ from what the generators write |
| `check`          | checks `build/`, `scripts/` and `content.ts`, tests and builds     |
| `clean`          | removes the generated files and the build outputs                  |
