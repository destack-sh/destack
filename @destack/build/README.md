# @destack/build

Inspect and build Destack packages.

## Inspect

`inspectPackage` collects a package's modules, tests and declarations for one target.

```ts
import { inspectPackage } from "@destack/build/inspect";

const { code, descriptions } = await inspectPackage({ directory, runtime: "bun" });
```

## Build

`buildPackage` compiles named module outputs, and `write` writes them with a manifest that describes them.

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

## Outputs

`readOutputs` lists the module outputs a package ships: one bundled module per runtime its exports compile for, named after the runtime.

```ts
await using build = await buildPackage({
    directory,
    dependencies: await readDependencies(directory),
    outputs: await readOutputs(directory),
});
```

## Runtimes

`runtimes` in `destack.json` sets the runtimes each export compiles for, and the build refuses an export without them.

```json
{ "runtimes": ["browser", "bun", "workerd"], "exports": { "./view": { "runtimes": ["browser"] } } }
```

## Host modules

The build refuses a host module the runtime lacks, unless the importing package's `browser` field replaces it with `false`, which the build bundles as an empty module.

```json
{ "name": "cron-parser", "browser": { "fs": false, "fs/promises": false } }
```

## Upgrades

`history` passes what the package has published, and the build writes the `Upgrade` from it into the manifest.

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

## Prereleases

`version` releases the build at another version than its `package.json` names, such as a dev prerelease: its manifest, its `package.json` file, its modules' metadata and its declarations carry it.

```ts
await using build = await buildPackage({
    directory,
    dependencies,
    outputs,
    version: "2026.9.0-dev.3",
});
build.manifest.package.version; // "2026.9.0-dev.3"
```

## Builder

`PackageBuilder.start` keeps one compiler warm across the builds and inspections of one checkout.

```ts
import { PackageBuilder } from "@destack/build";

await using builder = await PackageBuilder.start(directory);
const inspection = await builder.inspect({ runtime: "bun" });
await using first = await builder.build({ dependencies, outputs });
await using edited = await builder.build({ dependencies, outputs });
```

## Sandbox

`PackageBuilder.start` runs the `Compiler` in a `@destack/sandbox` sandbox without network or host environment, because evaluating a package's declarations runs its code.

```ts
import { PackageBuilder } from "@destack/build";

// read the workspace and the compiler's tools, and write only the compiler's temporary files
await using builder = await PackageBuilder.start(directory);
```

## Toolchain

`Compiler.beside` locates a release's `destack-build` and the `toolchain` directory beside it, which holds the native compiler, the bundler binding, the checkers, the runtime declarations and the lint rules.

```ts
import { Compiler, PackageBuilder } from "@destack/build";

const compiler = Compiler.beside("/opt/destack/destack");
// { executable: "/opt/destack/destack-build", arguments: [], read: ["/opt/destack/destack-build", "/opt/destack/toolchain"] }
await using builder = await PackageBuilder.start(directory, compiler);
```

## Cancellation

`signal` or `timeout` cancels a build, and the builder compiles the next build with a fresh compiler.

```ts
await builder.build({ dependencies, outputs, signal, timeout: 60_000 }); // rejects with BUILD_FAILED when cancelled
await using next = await builder.build({ dependencies, outputs });
```

## Cache

`store` lets a build reuse the outputs its cache keys name in a `PackageStore`, and the build stores itself and its outputs there.

```ts
await using cold = await builder.build({ dependencies, outputs, store }); // cold.reused: []
await using warm = await builder.build({ dependencies, outputs, store }); // warm.reused: ["bun"], nothing compiled
```

## Cache keys

The compiler derives the cache keys without checking or compiling, and counts an installed package by its release and a source package by its manifests and `src` files.

```ts
// key each module by the program, its path and the digest of its import cycle
const module = await Digest.json({ context: { toolchain, options, packages }, path, cycle });

// key each output by the modules it imports and its request
const output = await Digest.json({
    ...{ toolchain, extensions, manifests, version, request, configuration },
    modules: [...imported].toSorted(),
});

// key the build by every output key and every input that changes it
const build = await Digest.json({
    ...{ toolchain, extensions, manifests, version, catalogs, outputs, modules },
    ...{ dependencies, history, commit, template },
});
```

## Cache hits

The builder looks up the build key in the store, then each output key, and skips the work a hit covers.

```ts
// restore the stored build without checking, evaluating or bundling
await store.cached(keys.build); // { kind: "build", manifest }

// write the output's files without bundling, still checking and describing every source
await store.cached(keys.outputs["bun"]); // { kind: "output", output: { outputs, sourceMaps, files } }
```

## Retention

`sweep` marks the manifests and files retention drops and deletes them in a later sweep, and `lease` keeps a build or file until a moment.

```ts
await store.lease(digest, new Date(Date.now() + 10 * 60 * 1000));
await store.sweep(retained, new Date(Date.now() - 60 * 60 * 1000));
```

## Extensions

`build` in a package's `destack.json` names a `BuildExtension`, and a package's build applies the extension of each package in its dependency closure once, in name order.

```json
{ "build": "./build#viewExtension" }
```

## Extension parts

`transform` returns the plugins transforming modules wherever they compile, given the runtime, server rendering and the options an output gives each extension; `compile` returns the plugins for one module output, `describe` describes its views and workloads, and `outputs` adds further output kinds.

```ts
export const viewExtension: BuildExtension = {
    transform: ({ server, options }) => viewPlugins(server, options["@destack/view"]), // a web output names its application
    compile(compilation) {
        compilation.entry("./view/notes", "virtual:@destack/view/view/notes");
        return [mountPlugin(views)];
    },
    describe(compilation, compiled) {
        return {
            views: {
                notes: {
                    entrypoint: compiled.exports["./view/notes"],
                    permissions,
                    capabilities,
                    presents,
                },
            },
        };
    },
};
export const styleExtension: BuildExtension = {
    transform: ({ directory }) => [
        styleX({
            ...styleOptions(directory),
            cssInjectionTarget: (path) => path.includes("entry-client"),
        }),
    ],
};
export const webExtension: BuildExtension = { outputs: { web: webOutput } };
```

## Extension packages

The build compiles only module outputs, and the extensions of the package's dependency closure add the rest.

```ts
import { iconExtension } from "@destack/icon/build"; // the body of each icon drawn by a literal name
import { styleExtension } from "@destack/style/build"; // StyleX compilation
import { viewExtension } from "@destack/view/build"; // Solid compilation and one ./view/<name> entry per view
import { webExtension } from "@destack/web/build"; // Solid applications with server rendering and prerendered pages
import { spaceBuild } from "@destack/space/build"; // one ./workload/<name> entry per workload on Bun and workerd
```

## Views and workloads

The manifest lists each view and workload with the entrypoint the extension emitted for it.

```ts
const { entrypoint, permissions } = build.manifest.outputs.browser.views.notes; // "output/browser/view-notes-….js"
const { entrypoint: workload } = build.manifest.outputs.bun.workloads.notes; // "./workload/notes"
```

## Manifests

`PackageBuild.open` returns a reader of a written build's files, declarations and graph.

```ts
import { PackageBuild } from "@destack/build";

const reader = await PackageBuild.open(destination);
const files = await reader.files();
const declarations = await reader.declarations();
const root = await reader.graph();
const module = await reader.module(root.modules["src/index.ts"]);
```

## Catalogs

A build ships the package's `locale/<tag>.json` catalogs beside its manifests, refusing another package's catalog or one whose locale its file name does not give.

```ts
import { Catalog } from "@destack/locale";

const catalogs = await Catalog.read(reader); // [{ package, locale: "de", messages, drafts }]
```

## Graph files

A build writes its graph as a root file listing one file per module by its digest, which stays the same for an unchanged module because symbols carry the names their declarations write, such as `#name` for a private member.

```ts
build.manifest.lists.graph; // { path: "manifest/graph.json", digest: "…", size, mediaType }
const root = await reader.graph(); // { modules: { "src/note.ts": "e23ace0a…", "src/index.ts": "24b08be6…" } }
const note = await reader.module(root.modules["src/note.ts"]); // graph/e23ace0a….json: the symbols, declarations and edges
```

## Verification

`PackageBuild.read` reads a written build and verifies every file against the manifest.

```ts
await using build = await PackageBuild.read(destination);
```

## Store

`PackageStore` keeps builds in a bucket by content and copies builds from another store, and `PackageServer` serves their manifests, files and archives.

```ts
import { PackageServer, PackageStore } from "@destack/build/store";

const store = new PackageStore(bucket);
const digest = await store.put(build);
const { manifest, reader } = await store.contents(digest);
await store.copy(
    { manifest: digest, url: `https://registry.example.com/builds/${digest}/` },
    fetch,
);
const server = new PackageServer(new URL("https://packages.example.com/"), store);
const response = await server.fetch(request, { authorize });
```

## Uploads

`PackageServer` accepts uploads when the access gives an `upload` function, and `pushBuild` sends each missing file and then the manifest.

```ts
import { pushBuild } from "@destack/build/store";

await pushBuild(await store.contents(digest), "https://packages.example.com/builds/", fetch);
// HEAD files/<digest>: 200 when the server holds the file, else 404
// PUT files/<digest>: 201, or 400 for bytes of another digest and 413 past 64 MiB
// PUT <digest>/manifest.json: 201, or 409 while a file it lists is missing
```

## Templates

`Template.read` reads a template package, and `write` creates a new package from it.

```ts
import { Template } from "@destack/build/template";

const template = await Template.read(directory);
await template.write(destination, { id, name: "@example/notes", dependencies });
```

## Errors

A failed inspection or build throws a `BuildError`, which a caller receives as `UNPROCESSABLE_CONTENT` with status 422.

```ts
import { BuildError } from "@destack/build/error";

new BuildError("BUILD_FAILED", "cannot resolve ./missing.ts").toServiceError(); // { code: "UNPROCESSABLE_CONTENT", … }
```
