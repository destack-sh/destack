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

`readOutputs` lists one module output per runtime the package's exports declare.

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
import { Compiler, PackageBuilder } from "@destack/build";

// read the workspace and the compiler's tools, and write only the compiler's temporary files
await using builder = await PackageBuilder.start(directory, Compiler.beside(process.execPath));
```

## Toolchain

A release ships `destack-build` next to a `toolchain` directory that holds its native tools, and a workspace runs the entry with Bun over its installed packages.

```text
destack-build                                     Bun with the compiler bundled
toolchain/
├── node_modules/@typescript/typescript-<os>-<cpu>/lib/tsc   the API's tsserverPath
├── node_modules/@rolldown/binding-<platform>/               NAPI_RS_NATIVE_LIBRARY_PATH
├── node_modules/vite/dist/client/                           read by the bundled vite
├── node_modules/{oxlint,oxfmt,@oxlint-tsgolint/<os>-<cpu>}  checks, run as Bun children
├── node_modules/{@types/bun,@cloudflare/workers-types}      runtime declarations
└── lint.js                                                  the Destack lint rules
```

## Cancellation

`signal` or `timeout` cancels a build, and the builder compiles the next build with a fresh compiler.

```ts
await builder.build({ dependencies, outputs, signal, timeout: 60_000 }); // rejects with BUILD_FAILED when cancelled
await using next = await builder.build({ dependencies, outputs });
```

## Workspaces

`Workspace.read` reads the members of the enclosing Bun workspace from its lockfile, and `dependencies` and `dependents` follow their workspace dependencies.

```ts
import { Workspace } from "@destack/build";

const workspace = await Workspace.read(directory); // undefined outside any workspace
workspace?.dependencies("packages/app"); // ["packages/app", "packages/library"]
workspace?.dependents("packages/library"); // ["packages/library", "packages/app"]
```

## Cache

`store` lets a build reuse the outputs its cache keys name in a `PackageStore`, and the build stores itself and its outputs there.

```ts
await using cold = await builder.build({ dependencies, outputs, store }); // cold.reused: []
await using warm = await builder.build({ dependencies, outputs, store }); // warm.reused: ["bun"], nothing compiled
```

## Cache keys

The compiler derives the cache keys without checking or compiling, and counts an installed package by its release and a source package by its manifests and `src` files.

```text
module   the module's bytes, the API digest of each import, the compiler options and packages, the toolchain
         feeds the output and build keys
output   the keys of the modules the output imports, its request and configuration, the package's manifests,
         the extensions and the toolchain
         names the output's files, description and source maps
build    every output key, every module key, the dependency resolutions, the published history,
         the template and the toolchain
         names the stored build's manifest
```

## Cache hits

The builder looks up each key and skips the work a hit covers.

```text
build key    skips the check, the evaluation and the bundle, and restores the stored build
output key   skips the output's bundle and writes its files, and still checks and describes every source
```

## Retention

`sweep` marks the manifests and files retention drops and deletes them in a later sweep, and `lease` keeps a build or file until a moment.

```ts
await store.lease(digest, new Date(Date.now() + 10 * 60 * 1000));
await store.sweep(retained, new Date(Date.now() - 60 * 60 * 1000));
```

## Extensions

`build` in a dependency's `destack.json` names a `BuildExtension`, and the build applies it to the dependents.

```json
{ "build": "./build#viewExtension" }
```

## Extension parts

`compile` returns the plugins for one module output, `describe` describes its views and workloads, and `outputs` adds further output kinds.

```ts
export const viewExtension: BuildExtension = {
    compile(compilation) {
        compilation.entry("./view/notes", "virtual:@destack/view/view/notes");
        return viewPlugins(compilation.directory, false);
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
export const webExtension: BuildExtension = { outputs: { web: webOutput } };
```

## Dependency extensions

The build compiles only module outputs, and the extensions of a package's dependencies add the rest.

```text
@destack/view    Solid and StyleX compilation, and one ./view/<name> entry per view
@destack/web     web outputs: Solid applications with server rendering and prerendered pages
@destack/space   one ./workload/<name> entry per workload on Bun, and one workload per objects-only service
                 of a package without workloads
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

## Graph files

A build writes its graph as one file per module named by its digest, which stays the same for an unchanged module, and a root file that names them.

```text
manifest.json             { …, "graph": { "path": "manifest/graph.json", "digest": … } }
manifest/graph.json       { "modules": { "src/note.ts": "e23ace0a…", "src/index.ts": "24b08be6…" } }
graph/e23ace0a….json      the symbols, declarations and edges of src/note.ts
graph/24b08be6….json      the symbols, declarations and edges of src/index.ts
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

`PackageServer` accepts uploads when given an `upload` function, and `push` sends each missing file and then the manifest.

```text
HEAD files/<digest>          200 when the store holds the file, else 404
PUT files/<digest>           201, or 400 for bytes of another digest and 413 past 64 MiB
PUT <digest>/manifest.json   201, or 409 while a file it lists is missing
```

## Templates

`Template.read` reads a template package, and `write` creates a new package from it.

```ts
import { Template } from "@destack/build/template";

const template = await Template.read(directory);
await template.write(destination, { id, name: "@example/notes", dependencies });
```
