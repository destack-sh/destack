# @destack/build

Inspect and build Destack packages.

| Entry point | Holds |
|---|---|
| `@destack/build` | `buildPackage`, `PackageBuilder`, `PackageBuild`, `readOutputs`, `BuildError`. |
| `@destack/build/inspect` | `inspectPackage`. |
| `@destack/build/local` | `openSource`, `readDependencies`, `linkDependencies`, `resolutionPlugin` for tools compiling a checkout. |
| `@destack/build/store` | `PackageStore` and `PackageServer`. |
| `@destack/build/template` | `Template`. |
| `@destack/build/service`, `/client`, `/server`, `/access` | The build operation service, its client, its implementation and its policy. |
| `@destack/build/error` | `BuildError` and `BuildErrorCode`. |
| `@destack/build/browser` | The ambient types of browser modules. |

## Inspect

`inspectPackage` collects a package's modules, tests and declarations for one target.

```ts
import { inspectPackage } from "@destack/build/inspect";

const { code, descriptions } = await inspectPackage({ directory, runtime: "bun" });
```

## Build

`buildPackage` compiles named module outputs and writes a manifest describing them.

```ts
import { buildPackage } from "@destack/build";

await using build = await buildPackage({
    directory,
    dependencies,
    outputs: {
        library: { kind: "module", runtime: "bun", bundle: false },
        backend: { kind: "module", runtime: "workerd", bundle: true },
    },
});
await build.write(destination);
```

`readOutputs` lists one module output per runtime the package's exports declare.

```ts
await using build = await buildPackage({
    directory,
    dependencies: await readDependencies(directory),
    outputs: await readOutputs(directory),
});
```

An export compiles for the `runtimes` its destack.json declares, and the build refuses an export without them.

```json
{ "runtimes": ["browser", "bun", "workerd"], "exports": { "./view": { "runtimes": ["browser"] } } }
```

`history` holds what the package has published, and the build writes the upgrade from it into the manifest.

```ts
import { History, Upgrade } from "@destack/resource";

await using build = await buildPackage({
    directory,
    dependencies,
    outputs,
    history: await History.read(latest, vocabulary),
});
const upgrade =
    build.manifest.upgrade && (await build.reader.read(build.manifest.upgrade, Upgrade));
```

A `PackageBuilder` keeps one compiler process warm across builds of one checkout.

```ts
import { PackageBuilder } from "@destack/build";

await using builder = await PackageBuilder.start(directory);
const inspection = await builder.inspect({ runtime: "bun" });
await using first = await builder.build({ dependencies, outputs });
await using edited = await builder.build({ dependencies, outputs });
```

An abort or a deadline cancels the build, and the builder compiles the next one with a fresh compiler.

```ts
await builder.build({ dependencies, outputs, signal, timeout: 60_000 }); // rejects with BUILD_FAILED when cancelled
await using next = await builder.build({ dependencies, outputs });
```

## Extensions

A dependency declares a `BuildExtension` in its destack.json, and the build applies it to its dependents.

```json
{ "build": "./build#viewBuild" }
```

| Part | Role |
|---|---|
| `compile(compilation)` | Returns the plugins compiling one module output, and adds entries with `compilation.entry(entrypoint, module)`. |
| `describe(compilation, compiled)` | Describes the module output's workloads and views, reading what an entry reaches with `compiled.reach(entrypoint)`. |
| `outputs` | Compiles output kinds in a pass of its own, such as the `web` applications of `@destack/view`; the kind compiles and describes its outputs itself. |

The build knows only module outputs, and the extensions of a package's dependencies bring the rest.

| Extension | Adds |
|---|---|
| `@destack/view` | Solid and StyleX compilation, one `./view/<name>` entry per view, and `web` applications. |
| `@destack/space` | One `./workload/<name>` entry per workload on Bun, and one workload per objects-only service of a package without workloads. |

```ts
const { entrypoint, permissions } = build.manifest.outputs.browser.views.notes; // "output/browser/view-notes-….js"
const { entrypoint: workload } = build.manifest.outputs.bun.workloads.notes; // "./workload/notes"
```

## Manifests

A manifest describes the package's declarations, and refers to its dependencies' declarations by package and name.

```ts
import { PackageBuild } from "@destack/build";

const reader = await PackageBuild.open(destination);
const files = await reader.files();
const services = await reader.domain("service");
const description = files
    .find((file) => file.path === "src/index.ts")
    ?.descriptions?.find((description) => description.kind === "module");
const module = description && (await reader.module(description.file));
```

`PackageBuild.read` reads a written build back and verifies every file against the manifest.

```ts
await using build = await PackageBuild.read(destination);
```

## Store

A `PackageStore` keeps builds in a bucket by content, and a `PackageServer` serves their manifests, files and archives.

```ts
import { PackageServer, PackageStore } from "@destack/build/store";

const store = new PackageStore(bucket);
const digest = await store.put(build);
const { manifest, reader } = await store.contents(digest);
const server = new PackageServer(new URL("https://packages.example.com/"), store, { authorize });
const response = await server.fetch(request);
```

## Templates

A `Template` reads a template package and writes a new package from it.

```ts
import { Template } from "@destack/build/template";

const template = await Template.read(directory);
await template.write(destination, { id, name: "@example/notes", dependencies });
```

## Service

A build service builds and inspects revisions and stores their packages.

```ts
import { connect } from "@destack/build/client";
import { openPackage } from "@destack/package/manifest";

const client = connect({ url: "https://build.example.com", fetch: authenticatedFetch });
const build = await client.build.start({ source: revision, outputs: ["library"] });
for await (const operation of await client.build.watch({ id: build.id })) {
    if (operation.state === "succeeded") {
        const reader = await openPackage(operation.result.package, { fetch: authenticatedFetch });
    }
}
```

`implementService` serves builds with the host's sources, inspections and package store.

```ts
import { implementService } from "@destack/build/server";

const implementation = implementService(
    {
        builds: { open: openImmutableSource, inspect: openInspectionSource, store: storePackage },
        limits: { concurrency: 4, capacity: 100, timeout: 60_000, retention: 3_600_000 },
    },
    { access, audit },
    context,
);
```

Hosts register the `build` policy with their authorizer.

```ts
import { BUILD_POLICIES } from "@destack/build/access";

const objects = new ObjectServer({
    policies: [...BUILD_POLICIES],
    database,
    context,
    audit,
    journal,
});
```
