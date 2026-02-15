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
| affine  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| ajv     |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| angular |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ant-design |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| apollo-client |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| appsmith |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| appwrite |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| arktype |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| astro   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| authjs  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| aws-cdk |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| aws-sdk-js-v3 |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| axios   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| babel   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| backstage |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| budibase |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| cal-com |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| chakra-ui |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| chalk   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| changesets |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| citty   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| cloudflare-workers-sdk |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| code-server |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| commander |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| consola |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| cypress |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| d3      |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| date-fns |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| debug   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| definitelytyped |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T0   |  ✓  |
| dify    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| directus |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| docusaurus |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| dotenv  |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| drizzle-orm |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| echarts |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| effect  |    ✓     |    x     |   -?-    |   -?-    |   T0    |   T5   |  x  |
| electron |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| eslint  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| excalidraw |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| expo    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| expo-router |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| express |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| fastify |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| firebase-js-sdk |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T5   | -?- |
| floating-ui |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| fluentui |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| fp-ts   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| freecodecamp |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| grafana |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| graphql-codegen |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| graphql-js |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| h3      |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| hono    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| hoppscotch |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| immich  |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| insomnia |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| io-ts   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| jest    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| jitsi-meet |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| joplin  |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| jotai   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| kibana  |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| koa     |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| kysely  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| langchainjs |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| lodash  |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| material-ui |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| mattermost |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| medusa  |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| mermaid |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| mobx    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ms      |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |   T5   |  x  |
| mysql2  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| n8n     |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| nest    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nextchat |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| nextjs  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nitro   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nocodb  |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| node-postgres |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| novu    |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| nuqs    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nuxt    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| nx      |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ofetch  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| openai-node |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| outline |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| payload |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| pglite  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| pinia   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| plane   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| playwright |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| playwright-test |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| pnpm    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| preact  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| prettier |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| prisma  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| puppeteer |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| qwik    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| radix-primitives |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-admin |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-hook-form |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-native |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| react-native-web |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| react-query |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| react-router |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| redux   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| redux-toolkit |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| remeda  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| remix   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| rocket-chat |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| rollup  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| runtypes |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| rxjs    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| semver  |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |   T5   |  x  |
| sentry-javascript |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| slate   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| socket-io |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| solid   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| storybook |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| strapi  |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| stripe-node |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| styled-components |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| supabase |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| supabase-js |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| superjson |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| superset |    x     |   -?-    |   -?-    |   -?-    |   -?-   |   T2   | -?- |
| svelte  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| sveltekit |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| swr     |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tabby   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| tailwindcss |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-devtools |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-form |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-pacer |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-query |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-router |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-store |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-table |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tanstack-virtual |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| threejs |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tiptap  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tldraw  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| trpc    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-morph |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-pattern |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-rest |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ts-toolbelt |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tslib   |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| tsup    |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| tsx     |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| turbo   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| turborepo |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| twenty  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
| type-fest |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typebox |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typegpu |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typeorm |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typescript |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typescript-eslint |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| typia   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| umami   |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T2   |  x  |
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
| webpack |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| ws      |    ✓     |    ✓     |   -?-    |   -?-    |   T1    |   T5   |  x  |
| xstate  |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| yarn-berry |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
| zod     |    ✓     |    ✓     |    ✓     |    ✓     |   T3    |   T5   |  x  |
| zustand |    ✓     |   -?-    |   -?-    |   -?-    |   T0    |   T5   |  x  |
|---------|----------|----------|----------|----------|---------|--------|-----|
| total   | 151/181  |  18/181  | 3/180 (+1) | 3/180 (+1) |   -?-   |  -?-   | -?- |
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
