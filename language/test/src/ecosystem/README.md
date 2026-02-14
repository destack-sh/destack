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
Examples: `valibot-resolve`, `valibot-analyze`, `ms-parse`.

## Support Tiers

The ecosystem suite uses support tiers `T0` through `T5`:
- `T0` = "Syntax": we can parse.
- `T1` = "Import": we can parse and resolve everything.
- `T2` = "TSC": we can parse, resolve, and analyze it.
- `T3` = "AOT": we can parse, resolve, analyze, and lower it.
- `T4` = "Run": we can parse, resolve, analyze, lower, and run it.
- `T5` = "Test": we can parse, resolve, analyze, lower, run, and test it fully.

## Status

The pass rate intentionally excludes ignored cases.
Ignored cases include known failures and explicit ignore entries.
Package names may include patch markers.
`*` means the package passes with a local overlay patch from `fixtures/ecosystem/patches`.
`**` means the package depends on dependency patching or dependency replacement.
`Current` is computed from observed phase results.
`Target` is loaded from `package.target_tier` in each manifest.
<!-- (results are automatically updated by the ecosystem test runner) -->
<!-- begin:summary-results -->
| Package | parse | resolve | analyze | lower | Current | Target | Met | Total |  Rate   | Incl. Rate |
|:--------|:--------:|:--------:|:--------:|:--------:|:-------:|:------:|:---:|------:|--------:|-----------:|
| affine  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| ajv     |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| angular |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| ant-design |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| apollo-client |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| appsmith |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| appwrite |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| arktype |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| astro   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| authjs  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| aws-cdk |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| aws-sdk-js-v3 |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| axios   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| babel   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| backstage |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| budibase |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| cal-com |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| chakra-ui |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| chalk   |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| changesets |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| citty   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| cloudflare-workers-sdk |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| code-server |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| commander |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| consola |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| cypress |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| d3      |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| date-fns |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| debug   |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| definitelytyped |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| dify    |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| directus |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| docusaurus |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| dotenv  |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| drizzle-orm |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| echarts |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| effect  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| electron |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| eslint  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| excalidraw |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| expo    |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| expo-router |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| express |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| fastify |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| firebase-js-sdk |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| floating-ui |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| fluentui |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| fp-ts   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| freecodecamp |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| grafana |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| graphql-codegen |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| graphql-js |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| h3      |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| hono    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| hoppscotch |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| immich  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| insomnia |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| io-ts   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| jest    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| jitsi-meet |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| joplin  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| jotai   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| kibana  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| koa     |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| kysely  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| langchainjs |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| lodash  |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| material-ui |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| mattermost |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| medusa  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| mermaid |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| mobx    |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| ms      |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |  ---   | --- |     4 | 100.00% |   100.00% |
| mysql2  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| n8n     |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| nest    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| nextchat |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| nextjs  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| nitro   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| nocodb  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| node-postgres |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| novu    |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| nuqs    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| nuxt    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| nx      |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| ofetch  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| openai-node |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| outline |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| payload |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| pglite  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| pinia   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| plane   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| playwright |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| playwright-test |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| pnpm    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| preact  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| prettier |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| prisma  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| puppeteer |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| qwik    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| radix-primitives |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| react   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| react-admin |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| react-hook-form |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| react-native |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| react-native-web |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| react-query |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| react-router |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| redux   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| redux-toolkit |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| remeda  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| remix   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| rocket-chat |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| rollup  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| runtypes |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| rxjs    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| semver  |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |  ---   | --- |     4 | 100.00% |   100.00% |
| sentry-javascript |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| slate   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| socket-io |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| solid   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| storybook |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| strapi  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| stripe-node |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| styled-components |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| supabase |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| supabase-js |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| superjson |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| superset |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| svelte  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| sveltekit |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| swr     |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tabby   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| tailwindcss |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| tanstack-devtools |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-form |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-pacer |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-query |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-router |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-store |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-table |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tanstack-virtual |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| threejs |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tiptap  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| tldraw  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| trpc    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| ts-morph |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| ts-pattern |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| ts-rest |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| ts-toolbelt |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tslib   |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| tsup    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| tsx     |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| turbo   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| turborepo |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| twenty  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| type-fest |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| typebox |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| typegpu |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| typeorm |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| typescript |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| typescript-eslint |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| typia   |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| umami   |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| unbuild |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| undici  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| uuid    |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| valibot |    ✓     |    ✓     |   [--]   |   [--]   |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| vercel-ai |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| vite    |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| vitepress |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| vitest  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| vscode  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| vue     |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| vueuse  |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| webpack |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| ws      |    ✓     |    ✓     |   ---    |   ---    |   T1    |  ---   | --- |     4 | 100.00% |    50.00% |
| xstate  |   ---    |   ---    |   ---    |   ---    |   ---   |  ---   | --- |     4 |       - |     0.00% |
| yarn-berry |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
| zod     |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |  ---   | --- |     4 | 100.00% |   100.00% |
| zustand |    ✓     |   ---    |   ---    |   ---    |   T0    |  ---   | --- |     4 | 100.00% |    25.00% |
|---------|----------|----------|----------|----------|---------|--------|-----|-------|---------|------------|
| total   | 108/108  |  14/14   |   3/3    |   3/3    |   ---   |  ---   | --- |   724 | 100.00% |    17.68% |

Total Blended Pass Rate: **100.00%** (17.68% incl. ignored)
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
target_tier = "T2"
tags = ["framework", "tsx"]

[discovery]
include = ["src/**/*.ts", "src/**/*.tsx"]
exclude = ["**/*.test.ts", "**/__tests__/**"]
roots = ["packages/core", "packages/web"]

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
mode = "on-failure"
tool = "tsgo"
phases = ["resolve", "analyze"]

[[diagnostics]]
phase = "resolve"
code = "ER200"
file = "src/index.ts"
message = "unresolved module 'missing-package'"
```

Supported values for `package.language` are `js`, `ts`, and `ds`.
Use `package.target_tier` with one of `T0`, `T1`, `T2`, `T3`, `T4`, or `T5` to track intended coverage.
Set `patch.dependency_replacement = true` when a package requires dependency patching or replacement.
Use `discovery.roots` to constrain entrypoint ownership to specific package roots in monorepos.
Use `[prepare]` with tokenized `commands` to run package setup work like `pnpm build` before selected phases.
Use `[tsc]` to configure TypeScript parity checks with `tsgo` or `tsc` for selected phases.
`tsc.mode` supports `off`, `on-failure`, and `always`, and `tsc.tool` supports `auto`, `tsgo`, and `tsc`.

## Status Files

`known-failures.txt` and `ignored.txt` support `#` comments and one case id per line.

```txt
# temporary failures
valibot-analyze
valibot-lower
```

## Running

Use these commands to fetch and run the suite.
Missing package checkouts are auto-fetched during normal runs.
Explicit fetch runs also prune stale checkouts that no longer have manifests.
Resolve, analyze, and lower runs also install dependencies for freshly fetched packages.
Prepare commands run before selected compiler phases when configured in manifests.
Use explicit fetch to prewarm local state.

```bash
just language/ecosystem-fetch
cargo test -p destack_test --test ecosystem
cargo test -p destack_test --test ecosystem -- --list
cargo test -p destack_test --test ecosystem -- --phase parse
cargo test -p destack_test --test ecosystem -- --phase resolve
cargo test -p destack_test --test ecosystem -- --all-phases
cargo test -p destack_test --test ecosystem -- --phase analyze --include-known-failures
cargo test -p destack_test --test ecosystem -- --phase resolve --tsc-mode always --tsc-tool tsgo
cargo test -p destack_test --test ecosystem -- --update-known-failures
```
