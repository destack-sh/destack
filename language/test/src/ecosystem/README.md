# Ecosystem Tests

Ecosystem tests validate Destack against real TypeScript and JavaScript repositories pinned to reproducible refs.
The suite is manifest driven and runs package and phase combinations as independent test cases.

## Fixtures

The ecosystem fixtures are organized as follows.

- `fixtures/ecosystem/packages/*.toml`: package manifests with repository metadata and workload controls.
- `fixtures/ecosystem/checkouts/`: fetched package checkouts.
- `fixtures/ecosystem/patches/<package>/`: deterministic overlays applied on top of fetched checkouts.
- `fixtures/ecosystem/known-failures.txt`: temporary known failures.
- `fixtures/ecosystem/ignored.txt`: intentionally ignored cases.

## Case IDs

Ecosystem status files store case ids in the form `package-phase`.
Examples: `valibot-resolve`, `zod-analyze`, `ms-parse`.

## Phase Tiers

The current production phase tiers are as follows.

- `parse`
- `import`
- `resolve`
- `analyze`
- `elaborate`
- `execute`
- `lower`
- `optimize`

The default run is `parse`.
Use explicit phase selection to run larger slices.

## Status

The pass rate intentionally excludes ignored cases.
Ignored cases include known failures and explicit ignore entries.

<!-- (results are automatically updated by the ecosystem test runner) -->
<!-- begin:summary-results -->
| Package | parse | import | resolve | analyze | elaborate | execute | lower | optimize | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:--------|:------:|:------:|:------:|:------:|:------:|:------:|:------:|:------:|-------:|-------:|--------:|------:|--------:|-----------:|
| angular | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| apollo-client | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| arktype | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| astro   | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| changesets | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| definitelytyped | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| drizzle-orm | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| effect  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| eslint  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| fastify | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| graphql-codegen | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| graphql-js | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| hono    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| io-ts   | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| jest    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| jotai   | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| ms      | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| nest    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| nextjs  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| nuxt    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| nx      | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| playwright | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| pnpm    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| preact  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| prisma  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| qwik    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| react   | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| react-hook-form | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| react-query | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| redux-toolkit | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| remix   | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| runtypes | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| rxjs    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| solid   | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| storybook | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| sveltekit | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| tanstack-form | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| tanstack-query | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| tanstack-router | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| tanstack-store | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| tanstack-table | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| threejs | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| trpc    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| ts-toolbelt | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| turborepo | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| type-fest | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| typebox | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| typegpu | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| typescript | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| typescript-eslint | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| valibot | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| vite    | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| vitest  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| vscode  | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| vue     | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| yarn-berry | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| zod     | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
| zustand | ignored | ignored | ignored | ignored | ignored | ignored | ignored | ignored |     0  |     0  |       8  |     8 |       - |     0.00% |
|---------|--------|--------|--------|--------|--------|--------|--------|--------|--------|--------|---------|-------|---------|------------|
| total   |   -    |   -    |   -    |   -    |   -    |   -    |   -    |   -    |     0  |     0  |     464  |   464 |       - |     0.00% |

Total Blended Pass Rate: **-** (0.00% incl. ignored)
<!-- end:summary-results -->

## Manifest Schema

A manifest can specify base discovery and phase specific workload overrides.
Pass and skip expectations are tracked only in `known-failures.txt` and `ignored.txt`.

```toml
[package]
name = "example"
description = "Example package"
repo = "https://github.com/org/repo"
ref = "v1.2.3"
language = "ts"
tags = ["framework", "tsx"]

[discovery]
include = ["src/**/*.ts", "src/**/*.tsx"]
exclude = ["**/*.test.ts", "**/__tests__/**"]

[workloads.analyze]
max_files = 1000
```

Supported values for `package.language` are `js`, `ts`, and `ds`.

## Status Files

`known-failures.txt` and `ignored.txt` support `#` comments and one case id per line.

```txt
# temporary failures
ms-parse
valibot-resolve

# intentional skip
zod-elaborate # language difference
```

## Running

Use these commands to fetch and run the suite.
Missing package checkouts are auto-fetched during normal runs.
Use explicit fetch to prewarm local state.

```bash
just language/ecosystem-fetch
cargo test -p destack_test --test ecosystem
cargo test -p destack_test --test ecosystem -- --list
cargo test -p destack_test --test ecosystem -- --phase parse
cargo test -p destack_test --test ecosystem -- --phase import --phase resolve
cargo test -p destack_test --test ecosystem -- --all-phases
cargo test -p destack_test --test ecosystem -- --phase analyze --include-known-failures
cargo test -p destack_test --test ecosystem -- --update-known-failures
```

## Design Goals

The suite should stay deterministic, reproducible, and scalable across larger corpora.
Patch overlays are stamped and only re-applied when overlay content changes.
Compiler phase entrypoints prefer `package.json` entry fields (`exports`, `main`, `module`, `types`, `bin`) and fall back to deterministic source sampling.
Known failures are skipped by default, so use `--include-known-failures` when you want full diagnostics for those cases.
