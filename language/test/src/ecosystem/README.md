# Ecosystem Tests

Ecosystem tests validate Destack against real JS/TS repositories (pinned to reproducible refs).
The ecosystem suite is manifest driven and runs package and phase combinations as independent test cases.

## Fixtures

The ecosystem fixtures are organized as follows.

- `fixtures/ecosystem/packages/<package>.toml`: package manifests with repository metadata and workload controls.
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

Ignored cases include known failures and explicit ignore entries.
Package names may include patch markers.
`*` means the package passes with a local overlay patch from `fixtures/ecosystem/patches`.
`**` means the package depends on dependency patching or dependency replacement.
`Current` is computed from observed phase results.
`Target` is loaded from `package.tier` in each manifest.
<!-- (results are automatically updated by the ecosystem test runner) -->
<!-- begin:summary-results -->
| Package | parse | resolve | analyze | lower | Current | Target | Met |
|:--------|:--------:|:--------:|:--------:|:--------:|:-------:|:------:|:---:|
| affine  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| ajv     |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| angular |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ant-design |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| apollo-client |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| appsmith |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| appwrite |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| arktype |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| astro   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| authjs  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| aws-cdk |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| aws-sdk-js-v3 |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| axios   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| babel   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| backstage |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| budibase |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| cal-com |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| chakra-ui |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| chalk   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| changesets |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| citty   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| cloudflare-workers-sdk |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| code-server |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| commander |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| consola |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| cypress |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| d3      |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| date-fns |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| debug   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| definitelytyped |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T0   |  ✓  |
| dify    |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| directus |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| docusaurus |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| dotenv  |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| drizzle-orm |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| echarts |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| effect  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| electron |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| eslint  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| excalidraw |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| expo    |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| expo-router |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| express |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| fastify |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| firebase-js-sdk |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| floating-ui |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| fluentui |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| fp-ts   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| freecodecamp |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| grafana |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| graphql-codegen |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| graphql-js |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| h3      |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| hono    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| hoppscotch |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| immich  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| insomnia |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| io-ts   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| jest    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| jitsi-meet |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| joplin  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| jotai   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| kibana  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| koa     |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| kysely  |   -/-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| langchainjs |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| lodash  |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| material-ui |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| mattermost |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| medusa  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| mermaid |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| mobx    |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| ms      |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |   T5   |  x  |
| mysql2  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| n8n     |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| nest    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nextchat |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| nextjs  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nitro   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nocodb  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| node-postgres |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| novu    |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| nuqs    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nuxt    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nx      |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ofetch  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| openai-node |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| outline |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| payload |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| pglite  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| pinia   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| plane   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| playwright |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| playwright-test |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| pnpm    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| preact  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| prettier |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| prisma  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| puppeteer |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| qwik    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| radix-primitives |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-admin |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-hook-form |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-native |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| react-native-web |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| react-query |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-router |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| redux   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| redux-toolkit |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| remeda  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| remix   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| rocket-chat |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| rollup  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| runtypes |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| rxjs    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| semver  |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |   T5   |  x  |
| sentry-javascript |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| slate   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| socket-io |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| solid   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| storybook |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| strapi  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| stripe-node |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| styled-components |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| supabase |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| supabase-js |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| superjson |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| superset |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| svelte  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| sveltekit |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| swr     |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tabby   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| tailwindcss |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| tanstack-devtools |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-form |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-pacer |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-query |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-router |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-store |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-table |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-virtual |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| threejs |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tiptap  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| tldraw  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| trpc    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-morph |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-pattern |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-rest |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-toolbelt |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tslib   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| tsup    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tsx     |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| turbo   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| turborepo |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| twenty  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| type-fest |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typebox |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typegpu |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typeorm |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| typescript |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typescript-eslint |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typia   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| umami   |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| unbuild |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| undici  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| uuid    |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| valibot |    ✓     |    ✓     |   -/-    |   -/-    |   T1    |   T5   |  x  |
| vercel-ai |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| vite    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| vitepress |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| vitest  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| vscode  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| vue     |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| vueuse  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| webpack |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| ws      |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| xstate  |   -?-    |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| yarn-berry |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| zod     |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |   T5   |  x  |
| zustand |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
|---------|----------|----------|----------|----------|---------|--------|-----|
| total   | 109/109  |  18/18   |   3/3    |   3/3    |   -?-   |  -?-   | -?- |
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
tier = "T2"
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
Use `package.tier` with one of `T0`, `T1`, `T2`, `T3`, `T4`, or `T5` to track intended coverage.
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
