---
title: Legible
description: Software you can read, statically and while it runs.
---

# Legible

Code is both the most important and the least interesting part of software; we also want to see what it does, where it goes, and what changes when how where.
TS++ is built so that tools can read it as well as you could:
 1. Checking and linting see the whole program, not just one file.
 2. Queries search code by shape, not by text.
 3. Rewrites change code by shape, too.

## Check and Lint

`check` type checks and lints a package in one go, and every diagnostic has a longer explanation:

```sh
tspp check
tspp explain borrow-outlives-origin
```

## Query

Structural patterns find code by what it is, with `$NAME` holes that match any expression:

```sh
tspp query 'fetch($URL)' src
```

## Rewrite

The same patterns rewrite code, keeping whatever the holes matched:

```sh
tspp rewrite 'fetch($URL)' 'client.fetch($URL)' src
```
