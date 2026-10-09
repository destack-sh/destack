# @destack/build

`buildPackage` is Vite's `build` on Rolldown, emitting one module output per runtime (`browser`, `bun`, `workerd`) with a manifest, a `BuildExtension` is a Vite plugin a dependency contributes, and `/store` keeps builds by content as an OCI registry keeps a manifest and its blobs.

```ts
await using build = await buildPackage({ directory, dependencies, outputs: await readOutputs(directory) }); // vite build
await using builder = await PackageBuilder.start(directory); // a warm compiler, like vite build --watch
export const viewExtension: BuildExtension = { transform: ({ server, options }) => viewPlugins(server, options["@destack/view"]) }; // Vite plugins
await new PackageClient("https://packages.example.com/builds/", fetch).push(reader); // OCI push: blobs, then the manifest
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
    version: "2026.9.0-dev.3", // a prerelease instead of package.json's version
});
await build.write(destination);
```

## Builder

`PackageBuilder.start` keeps one compiler warm across the builds and inspections of one checkout, and reuses outputs a `PackageStore` holds.

```ts
await using builder = await PackageBuilder.start(directory);
const inspection = await builder.inspect({ runtime: "bun" });
await using warm = await builder.build({ dependencies, outputs, store, timeout: 60_000 }); // warm.reused: ["bun"]
```

## Extensions

`build` in a package's `destack.json` names a `BuildExtension`, and a package's build applies the extension of each package in its dependency closure once.

```json
{ "build": "./build#viewExtension" }
```

## Store

`PackageStore` keeps builds in a bucket by content, and `PackageServer` serves their manifests, files and archives.

```ts
import { PackageServer, PackageStore } from "@destack/build/store";

const store = new PackageStore(bucket);
const digest = await store.put(build.reader);
const server = new PackageServer(new URL("https://packages.example.com/"), store);
await store.sweep(retained, new Date(Date.now() - 60 * 60 * 1000));
```

## Templates

`Template.read` reads a template package, and `write` creates a new package from it.

```ts
import { Template } from "@destack/build/template";

const template = await Template.read(directory);
await template.write(destination, { id, name: "@example/site", dependencies });
```
