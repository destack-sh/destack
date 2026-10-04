# @destack/web

Build and serve standalone Solid web applications with server rendering and prerendered pages.

## Outputs

`WebOptions` describes a `web` output, which a build compiles into a browser output and, with `ssr`, a server output.

```ts
import { buildPackage } from "@destack/build";
import type { WebOptions } from "@destack/web/build";

const website: WebOptions = {
    kind: "web",
    app: "src/app.tsx",
    ssr: { runtime: "bun", emit: false },
    prerender: { origin: "https://example.com", routes: ["/"], notFound: "/404" },
};
await using build = await buildPackage({ directory, dependencies, outputs: { website } });
```

## Entries

`entryServer` and `entryClient` name an authored pair of entries, and every rendered page loads the client entry before its head ends, as it loads a generated one.

```ts
const website: WebOptions = {
    kind: "web",
    app: "src/app.tsx",
    ssr: { runtime: "bun" },
    entryServer: "src/entry-server.tsx", // renders the document, without a script for the client entry
    entryClient: "src/entry-client.tsx", // hydrates the document
};
```

## Development server

`DevelopmentServer.start` serves a web application with the Vite watcher and hot module replacement, and restarts on the same port once its own or a dependency's `destack.json` changes.

```ts
import { DevelopmentServer } from "@destack/web/build";

await using server = await DevelopmentServer.start({
    directory, // resolves the releases its lockfile selects, as builds do
    application: website,
    server: { port: 3000 },
});
```
