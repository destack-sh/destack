# @destack/build

`@destack/build` compiles a package into one module output per runtime with Vite on Rolldown, and keeps builds by content as an OCI registry does.

```ts
// vite build
await using build = await buildPackage({
    directory,
    dependencies,
    outputs: await readOutputs(directory),
});
// a warm compiler, like vite build --watch
await using compiler = await Compiler.start(directory);
// Vite plugins
export const viewExtension: BuildExtension = {
    transform: ({ server, options }) => viewPlugins(server, options["@destack/view"]),
};
// OCI push: blobs, then the manifest
await new PackageClient("https://packages.example.com/builds/", fetch).push(reader);
await PackageBuild.read(destination); // every file checked against the manifest's digests
```

## Builds

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
    versions: { "@example/notes": "2026.10.0-dev.3" }, // listed sources at -dev.3
});
await build.write(destination);
```

## Builder

`Compiler.start` keeps one compiler warm across the builds and inspections of one checkout.

```ts
await using compiler = await Compiler.start(directory);
const inspection = await compiler.inspect({ runtime: "bun" });
// warm.reused: ["bun"]
await using warm = await compiler.build({ dependencies, outputs, store, timeout: 60_000 });
```

## Extensions

`build` in a package's `destack.json` names a `BuildExtension`, which every package depending on it applies once.

```json
{ "build": "./build#viewExtension" }
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

## Templates

`Template.read` reads a template package, and `write` creates a new package from it.

```ts
import { Template } from "@destack/build/template";

const template = await Template.read(directory);
await template.write(destination, { id, name: "@example/site", dependencies });
```
