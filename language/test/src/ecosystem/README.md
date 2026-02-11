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
| ajv     |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| angular |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| apollo-client |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| arktype |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| astro   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| chalk   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| changesets |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| cloudflare-workers-sdk |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| commander |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| dayjs   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| debug   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| definitelytyped |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| dotenv  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| drizzle-orm |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| effect  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| eslint  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| express |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| fastify |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| graphql-codegen |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| graphql-js |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| hono    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| io-ts   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| jest    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| jotai   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| koa     |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| lodash  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| ms      |    ✓     |    ✓     |   ---    |   ---    |     2  |     0  |       2  |     4 | 100.00% |    50.00% |
| mysql2  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| nest    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| nextjs  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| node-postgres |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| nuxt    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| nx      |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| playwright |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| pnpm    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| preact  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| prisma  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| qwik    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| radix-primitives |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| react   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| react-hook-form |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| react-query |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| react-router |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| redux-toolkit |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| remix   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| runtypes |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| rxjs    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| semver  |    ✓     |    ✓     |    ✓     |    ✓     |     4  |     0  |       0  |     4 | 100.00% |   100.00% |
| socket-io |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| solid   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| storybook |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| sveltekit |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| swr     |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-devtools |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-form |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-pacer |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-query |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-router |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-store |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-table |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tanstack-virtual |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| threejs |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| trpc    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| ts-toolbelt |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| tslib   |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| turborepo |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| type-fest |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| typebox |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| typegpu |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| typescript |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| typescript-eslint |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| undici  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| uuid    |    ✓     |    ✓     |   ---    |   ---    |     2  |     0  |       2  |     4 | 100.00% |    50.00% |
| valibot |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| vercel-ai |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| vite    |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| vitest  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| vscode  |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| vue     |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| ws      |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| yarn-berry |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| zod     |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
| zustand |    ✓     |   ---    |   ---    |   ---    |     1  |     0  |       3  |     4 | 100.00% |    25.00% |
|---------|----------|----------|----------|----------|--------|--------|---------|-------|---------|------------|
| total   |  83/83   |   3/3    |   1/1    |   1/1    |    88  |     0  |     244  |   332 | 100.00% |    26.51% |

Total Blended Pass Rate: **100.00%** (26.51% incl. ignored)
<!-- end:summary-results -->

## Manifest Schema

A manifest can specify base discovery and phase specific workload overrides.
A manifest can also specify compiler option overrides when a package uses uncommon file extension conventions.
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

[compiler_options]
js_as_jsx = true

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
