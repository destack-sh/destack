# Ecosystem Tests

Ecosystem tests validate Destack against real TypeScript and JavaScript repositories pinned to reproducible refs.
The suite is manifest driven and runs package and phase combinations as independent test cases.

## Fixtures

The ecosystem fixtures are organized as follows.

- `fixtures/ecosystem/packages/*.toml`: package manifests with repository metadata and workload controls.
- `fixtures/ecosystem/checkouts/`: fetched package checkouts.
- `fixtures/ecosystem/patches/<package>/`: overlays applied on top of fetched checkouts.
- `fixtures/ecosystem/known-failures.txt`: temporary known failures.
- `fixtures/ecosystem/ignored.txt`: intentionally ignored cases.

## Case IDs

Ecosystem status files store case ids in the form `package-phase`.
Examples: `valibot-resolve`, `zod-analyze`, `ms-parse`.

## Phase Tiers

The current production phase tiers are as follows.

- `parse`
- `resolve`
- `analyze`
- `lower`

The default run is `parse`.
Use explicit phase selection to run larger slices.

## Status

The pass rate intentionally excludes ignored cases.
Ignored cases include known failures and explicit ignore entries.

<!-- (results are automatically updated by the ecosystem test runner) -->
<!-- begin:summary-results -->
| Package | parse | resolve | analyze | lower | Passed | Failed | Ignored | Total |  Rate   | Incl. Rate |
|:--------|:--------:|:--------:|:--------:|:--------:|-------:|-------:|--------:|------:|--------:|-----------:|
| angular |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| apollo-client |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| arktype |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| astro   |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| changesets |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| definitelytyped |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| drizzle-orm |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| effect  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| eslint  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| fastify |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| graphql-codegen |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| graphql-js |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| hono    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| io-ts   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| jest    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| jotai   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| ms      |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| nest    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| nextjs  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| nuxt    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| nx      |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| playwright |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| pnpm    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| preact  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| prisma  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| qwik    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| react   |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| react-hook-form |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| react-query |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| redux-toolkit |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| remix   |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| runtypes |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| rxjs    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| solid   |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| storybook |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| sveltekit |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| tanstack-form |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| tanstack-query |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| tanstack-router |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| tanstack-store |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| tanstack-table |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| threejs |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| trpc    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| ts-toolbelt |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| turborepo |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| type-fest |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| typebox |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| typegpu |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| typescript |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| typescript-eslint |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| valibot |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| vite    |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| vitest  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| vscode  |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| vue     |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| yarn-berry |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
| zod     |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| zustand |   ---    |   ---    |   ---    |   ---    |     0  |     0  |       4  |     4 |       - |     0.00% |
|---------|----------|----------|----------|----------|--------|--------|---------|-------|---------|------------|
| total   |    -     |    -     |    -     |    -     |     7  |     0  |     225  |   232 | 100.00% |     3.02% |

Total Blended Pass Rate: **100.00%** (3.02% incl. ignored)
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
zod-analyze # language difference
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
cargo test -p destack_test --test ecosystem -- --phase resolve
cargo test -p destack_test --test ecosystem -- --all-phases
cargo test -p destack_test --test ecosystem -- --phase analyze --include-known-failures
cargo test -p destack_test --test ecosystem -- --update-known-failures
```
