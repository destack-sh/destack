# @destack/build

`@destack/build` compiles a package into one module output per runtime with Vite on Rolldown, and keeps builds by content as an OCI registry does.

```ts
// vite build
await using build = await buildPackage({
    directory,
    dependencies,
    outputs: await readOutputs(directory),
});
// warm compilers, like vite build --watch, building a workspace's units into one store
await using compilers = await CompilerPool.open(store, report);
await using build = await (await Workspace.open(directory, compilers)).build({ outputs });
// Vite plugins
export const viewExtension: BuildExtension = {
    transform: ({ server, options }) => viewPlugins(server, options["@destack/view"]),
};
// OCI push: blobs, then the manifest
await new PackageClient("https://packages.example.com/builds/", fetch).push(reader);
await PackageBuild.read(destination); // every file checked against the manifest's digests
```

## Builds

`buildPackage` compiles named module outputs, and `PackageBuild.write` writes a build with the manifest that describes it.

```ts
import { buildPackage } from "@destack/build";

await using build = await buildPackage({
    directory,
    dependencies,
    outputs: {
        library: { kind: "module", runtime: "bun" },
        browser: { kind: "module", runtime: "browser" }, // a browser output bundles its dependencies
    },
    versions: { "@example/notes": "2026.10.0-dev.3" }, // listed sources at -dev.3
});
await PackageBuild.write(build.reader, destination);
```

## Compilers

`Compiler.start` keeps one sandboxed compiler warm across a workspace's builds into a store, and `CompilerPool` leases warm compilers one package at a time.

```ts
await using compiler = await Compiler.start(directory, { store });
const inspection = await compiler.inspect({ directory, runtime: "bun", dependencies });
const code = await compiler.index({ directory, runtime: "bun", dependencies }); // graph.Document[]
// warm.reused: ["bun"]
await using warm = await compiler.build({ directory, dependencies, outputs, timeout: 60_000 });

await using compilers = await CompilerPool.open(store, report);
const built = await compilers.use(directory, (leased) => leased.build({ directory, dependencies, outputs }));
```

## Workspaces

`Workspace.open` reads a checkout once: the Destack members a package requires, the lockfile's npm pins, the toolchain and the git listing its keys read.

```ts
const workspace = await Workspace.open(directory, compilers);
workspace.units; // [{ name: "@example/notes", directory, requires: [] }, ..., the root last]
await using build = await workspace.build({ outputs, history }); // each unit stored once by its key
const inspection = await workspace.inspect({ runtime: "bun" });
const code = await workspace.index({ runtime: "bun" });
const dependencies = await workspace.storeDependencies({}); // the root's builds and npm units
```

## Extensions

`build` in a package's `destack.json` names a `BuildExtension`, which every package depending on it applies once.

```json
{ "build": "./build#viewExtension" }
```

## Listed extensions

`extensions` lists the packages whose extensions compile a package beyond its runtime dependencies, as Vite names its plugins.

```json
{ "extensions": ["@destack/view"] }
```

## npm units

`Workspace` builds each npm package a unit reaches as an npm unit for the unit's server runtimes: Bun runs the package as installed, workerd runs it converted to ESM, and a browser output bundles its packages.

```ts
// the workerd output follows Node's rules: a require of a missing host module or package throws when it runs,
// an import of one fails when workerd loads the module, and a native addon and its importers have no workerd output
const installation = new NpmInstallation(pins, store, toolchain);
const unit = await installation.open(installed);
const release = await installation.store(unit, convert); // { kind: "npm", ..., manifest }
```

## Store

`PackageStore` keeps builds in a bucket by content, and `fetch` serves their manifests, files and archives below a mount.

```ts
import { PackageStore, Retention } from "@destack/build/store";

const store = new PackageStore(bucket);
const digest = await store.put(build.reader);
const response = await store.fetch(request, new URL("https://packages.example.com/"), access);
await store.sweep(retained, new Date(Date.now() - 60 * 60 * 1000), Retention.none, new Date());
```

## Local store

`LocalPackageStore` caches one toolchain's builds on disk for a week, keyed by the files a build reads.

```ts
import { LocalPackageStore } from "@destack/build";

await using store = await LocalPackageStore.checkout(directory, "test"); // <checkout root>/node_modules/.cache/destack/test, or $DESTACK_CACHE_DIRECTORY/test
await store.rootDirectory(directory, manifests); // retained until the directory is gone
```

## Templates

`Template.read` reads a template package, and `write` creates a new package from it.

```ts
import { Template } from "@destack/build/template";

const template = await Template.read(directory);
await template.write(destination, { id, name: "@example/site", dependencies });
```
