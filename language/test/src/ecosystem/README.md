# Ecosystem Tests

Ecosystem tests are Destack's curated external package suite.
They validate a small, curated set of pinned TS-first Node, backend, and tooling repositories.
They do not try to represent the whole JS or web ecosystem.

## Fixtures

The ecosystem fixtures are organized as follows.

- `fixtures/ecosystem/packages/<package>.toml`: package manifests with repository metadata and workload controls.
- `fixtures/ecosystem/checkouts/`: fetched package checkouts.
- `fixtures/ecosystem/patches/<package>/`: overlays applied on top of fetched checkouts when needed.
- `fixtures/ecosystem/known-failures.txt`: temporary known failures.
- `fixtures/ecosystem/ignored.txt`: intentionally ignored cases.

## Case IDs

Ecosystem status files store case ids in the form `package-phase`.
Examples: `valibot-resolve`, `valibot-analyze`, `ms-parse`.

## Support Tiers

The ecosystem suite uses support tiers `T0` through `T5`.
- `T0` = "Syntax": we can parse.
- `T1` = "Import": we can parse and resolve everything.
- `T2` = "TSC": we can parse, resolve, and analyze it.
- `T3` = "AOT": we can parse, resolve, analyze, and lower it.
- `T4` = "Run": we can parse, resolve, analyze, lower, and run it.
- `T5` = "Test": we can parse, resolve, analyze, lower, run, and test it fully.

## Current Scope

The current curated package set is:

- `arktype`
- `changesets`
- `citty`
- `consola`
- `drizzle-orm`
- `kysely`
- `ms`
- `openai-node`
- `trpc`
- `ts-morph`
- `ts-pattern`
- `ts-rest`
- `type-fest`
- `typebox`
- `typeorm`
- `typescript`
- `typia`
- `unbuild`
- `uuid`
- `valibot`
- `zod`

## Status

Ignored cases include known failures and explicit ignore entries.
Package names may include patch markers.
`*` means the package passes with a local overlay patch from `fixtures/ecosystem/patches`.
`**` means the package depends on dependency patching or dependency replacement.
`Current` is computed from observed phase results.
`Target` is loaded from `package.tier` in each manifest.
`Modules` and `Lines` use resolve phase loaded graph stats when available, otherwise `-?-`.
<!-- (results are automatically updated by the ecosystem test runner) -->
<!-- begin:summary-results -->
| Package | parse | resolve | analyze | lower | Modules | Lines | Current | Target | Met |
|:--------|:--------:|:--------:|:--------:|:--------:|:--------:|:-----:|:-------:|:------:|:---:|
| arktype |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| changesets |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| citty   |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| consola |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| drizzle-orm |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| kysely  |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| ms      |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| openai-node |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| trpc    |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| ts-morph |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| ts-pattern |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| ts-rest |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| type-fest |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| typebox |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| typeorm |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| typescript |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| typia   |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| unbuild |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| uuid    |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| valibot |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
| zod     |    ✓     |   -?-    |   -?-    |   -?-    |   -?-    |  -?-  |   T0    |   T3   |  x  |
|---------|----------|----------|----------|----------|----------|-------|---------|--------|-----|
| total   |  21/21   |   0/21   |   0/21   |   0/21   |   -?-    |  -?-  |   -?-   |  -?-   | -?- |
<!-- end:summary-results -->

## Manifest Schema

A manifest can specify base discovery and phase specific workload overrides.
A manifest can also specify compiler option overrides when a package uses uncommon file extension conventions.
Pass and skip expectations are tracked in `known-failures.txt`, `ignored.txt`, and per-manifest `[[diagnostics]]` entries.

```toml
[package]
name = "example"
description = "Example package"
repo = "https://github.com/org/repo"
ref = "v1.2.3"
language = "ts"
tier = "T3"
tags = ["backend", "tooling"]

[discovery]
include = ["src/**/*.ts", "packages/**/*.ts", "**/*.d.ts"]
exclude = ["**/*.test.ts", "**/__tests__/**"]
roots = ["packages/core"]

[compiler_options]
js_as_jsx = true

[patch]
dependency_replacement = true

[prepare]
phases = ["resolve", "analyze", "lower"]
commands = [
    ["pnpm", "build"],
]

[tsc]
enabled = true
```

## Running

Use the top level lanes for normal workflows.
Run `just full` for the full deep sweep.
Run the ecosystem suite directly when iterating on the ecosystem harness itself.

```bash
just language/fetch-ecosystem
cargo test -p destack_test --test ecosystem
cargo test -p destack_test --test ecosystem -- --list
cargo test -p destack_test --test ecosystem -- --phase parse
cargo test -p destack_test --test ecosystem -- --phase resolve
cargo test -p destack_test --test ecosystem -- --all-phases
cargo test -p destack_test --test ecosystem -- --phase analyze --include-known-failures
cargo test -p destack_test --test ecosystem -- --phase resolve --tsc-mode always --tsc-tool tsgo
cargo test -p destack_test --test ecosystem -- --phase parse --read-stats
cargo test -p destack_test --test ecosystem -- --phase parse --dump-read-paths
cargo test -p destack_test --test ecosystem -- --update-known-failures
```
