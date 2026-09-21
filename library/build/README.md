Inspect, build, and preview Destack packages.

```ts
import { inspectPackage } from "@destack/build/inspect";

const inspection = await inspectPackage({
    directory,
    target: "server",
    runtime: "bun",
});

const { code, descriptions } = inspection;
```

```ts
import { buildPackage } from "@destack/build";

const build = await buildPackage({
    directory,
    dependencies,
    outputs: {
        library: {
            kind: "module",
            target: "server",
            runtime: "bun",
            bundle: false,
        },
        backend: {
            kind: "module",
            target: "server",
            runtime: "workerd",
            bundle: true,
        },
    },
});

const { manifest, files } = build;
await build.write(destination);
```

```ts
import { PackageBuilder } from "@destack/build";

await using builder = await PackageBuilder.start(directory);
const inspection = await builder.inspect({ target: "server", runtime: "bun" });
const first = await builder.build({ dependencies, outputs });
const edited = await builder.build({ dependencies, outputs });
```

```ts
const application = {
    kind: "web",
    app: "src/App.tsx",
    base: "/",
    minify: true,
    ssr: { runtime: "workerd" },
    prerender: {
        origin: "https://example.com",
        routes: ["/", "/about/"],
    },
} as const;

const build = await buildPackage({
    directory,
    dependencies,
    outputs: { website: application },
});
```

```ts
import { servePackage } from "@destack/build/local";

await using server = await servePackage({
    directory,
    dependencies,
    application,
    server: { host: "127.0.0.1", port: 3000 },
});

const { watcher, moduleGraph } = server.vite;
```

```ts
import { connect } from "@destack/build/client";

const client = connect({ url: "https://build.example.com", fetch: authenticatedFetch });
const inspection = await client.inspect({ source: revision, output: "library" });

const build = await client.build.start({ source: revision, outputs: ["library"] });
for await (const operation of await client.build.watch({ id: build.id })) {
    if (operation.state === "succeeded") {
        await download(operation.result.download);
    }
}

const preview = await client.preview.start({ source: checkout, application: "website" });
for await (const state of await client.preview.watch({ id: preview.id })) {
    if (state.state === "running") {
        open(state.url);
        break;
    }
}
await client.preview.stop({ id: preview.id });
```

```ts
import { BuildServer } from "@destack/build/server";
import { Server } from "@destack/service/server";

await using build = new BuildServer({
    health,
    authorize,
    audit,
    builds: {
        open: openImmutableSource,
        inspect: openInspectionSource,
        store: storePackageArchive,
    },
    previews: { open: openEditableSource },
    limits: {
        build: { concurrency: 4, capacity: 100, timeout: 60_000, retention: 3_600_000 },
        preview: { concurrency: 4, capacity: 100, retention: 3_600_000 },
    },
});

await using server = await Server.start({
    handler: build.handler,
    context: authenticate,
    drainTimeout: 10_000,
});

// host sandbox and listener
await listen(server.fetch.bind(server));
```
