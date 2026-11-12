# @destack/build

Compile Destack packages into one module output per runtime, and keep the builds by content.

## Overview

`@destack/build` is Vite on Rolldown per package: sandboxed compilers, builds stored by digest, and locks of the builds a package runs with.

Workspaces build packages:

- `Workspace`: a checkout's packages, each built on its dependencies' builds.
- `CompilerPool`: warm, sandboxed compilers, leased one package at a time.
- `BuildExtension`: Vite plugins a dependency adds to the packages using it.
- `Template`: a template package, written out as a new package.

Stores keep builds and locks:

- `BuildStore` and `LocalBuildStore`: builds and locks by digest, as an OCI registry keeps images.
- `Lock`: a root build and the build of each package it runs with, from `@destack/package`.
- `PackageClient`: pushes builds and locks to a forge, and reads them for `pull`.

Build a package, store it, and lock it with what it runs with:

```ts
import { CompilerPool, outputsOf, Workspace } from "@destack/build";
import type { LocalBuildStore } from "@destack/build";
import { Toolchain } from "@destack/check/toolchain";

/** Build a checkout's package, and lock it with its dependencies. */
export async function buildAndLock(store: LocalBuildStore, directory: string) {
    // build the root on its dependencies' builds
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspace = await Workspace.open(directory, compilers);
    const build = await workspace.build({ outputs: outputsOf(workspace.root.packageJson) });

    // store it, and lock it with every build it runs with
    const root = await store.put(build);

    return await workspace.lock(root);
}
```

## Reference

### Workspace

`Workspace.open(directory, compilers, { resolver, selection, host })` reads the closure of packages a checkout's root requires, in build order.
Each unit is keyed by its toolchain, sources and its inputs' interfaces, and compiles only on a miss.
A package the host provides resolves to the host's lock as a builtin, and every other release to the resolver.

| Verb                     | What it does                                    |
| ------------------------ | ----------------------------------------------- |
| `build(request)`         | builds the root with given outputs              |
| `lockUnits(options)`     | builds and locks every unit, by name            |
| `lock(root, options)`    | locks a root build with every build it requires |
| `inspect(…)`, `index(…)` | inspects or indexes the root's code             |

`isPrerelease` stamps each build as a dev version, `<release>-dev.<time>.<key>`.

### BuildStore

| Verb                              | What it does                                                  |
| --------------------------------- | ------------------------------------------------------------- |
| `put(build)`                      | stores a build's missing files, then its manifest             |
| `putLock(lock)`, `lock(digest)`   | stores or reads a lock                                        |
| `has(digests)`                    | tells which blobs the store has, one flag per digest          |
| `read(objects)`, `write(entries)` | reads or writes files, manifests and locks as tarball entries |
| `pull(source, root, runtimes)`    | pulls a lock's or build's objects from another store          |
| `record(digest)`                  | records a lock whose blobs arrived, as a moved space's        |
| `entriesOf(lock)`                 | reads a lock's builds by package name                         |
| `referencing(digest)`             | lists the manifests and locks referencing a blob or build     |
| `fetch(request, mount)`           | serves manifests, files and the store's verbs below a mount   |

A `LocalBuildStore` also lays out a lock's tree for a runtime with `runTree`, and writes distribution layers with `writeLayer`.

### PackageClient

`new PackageClient(url, fetch)` is a build store over HTTP, with the same `has`, `read` and `write`.
`push(store, locks)` lists the locks' objects, asks which the server has, and writes the rest in bounded uploads.

## Examples

### Build named outputs

Each output is a module for one runtime, and a browser output bundles its dependencies.

```ts
import type { Workspace } from "@destack/build";

/** Build a library for Bun and a bundle for browsers, at a dev version. */
export async function buildBoth(workspace: Workspace) {
    return await workspace.build({
        outputs: {
            library: { kind: "module", runtime: "bun" },
            browser: { kind: "module", runtime: "browser" },
        },
    });
}
```

### Push and pull a lock

`push` writes only the objects the forge lacks, and `pull` reads a lock in one read per round.

```ts
import { PackageClient } from "@destack/build/store";
import type { LocalBuildStore } from "@destack/build";
import type { Digest } from "@destack/schema";

/** Push a stored lock to a forge's build mount, then pull it into another store for Bun. */
export async function pushThenPull(
    store: LocalBuildStore,
    other: LocalBuildStore,
    digest: Digest,
    url: string,
) {
    // push the lock's objects the forge lacks
    await new PackageClient(url, fetch).push(store, [digest]);

    // pull what Bun runs of it into the other store
    const source = new PackageClient(`${url}lock/${digest}/`, fetch);
    await other.pull(source, { kind: "lock", digest }, ["bun"]);
}
```

### Add a build extension

A package's `destack.json` names its extension in `build`.

```ts
import type { BuildExtension } from "@destack/package/build";

// add no plugins yet, to every dependent package's build
export const markdownExtension: BuildExtension = {
    transform: () => [],
};
```

### Create a package from a template

`write` gives the new package a fresh identity.

```ts
import { Template } from "@destack/build/template";
import { Toolchain } from "@destack/check/toolchain";
import { BunEnvironment } from "@destack/environment/bun";
import type { TemplateInput } from "@destack/package/template";

/** Create a site package from the blank template. */
export async function createSite(parameters: TemplateInput) {
    // read the template, and write a package with a fresh identity
    const { files } = BunEnvironment.system;
    const template = await Template.read("/templates/blank", files);
    await template.write("/work/site", parameters, Toolchain.workspace(), files);
}
```

## Comparison

### Vite

Outputs, plugins and the module graph follow Vite on Rolldown.

| Vite                 | `@destack/build`        | Difference                                               |
| -------------------- | ----------------------- | -------------------------------------------------------- |
| one bundle of an app | a build per package     | Only a browser output bundles its dependencies.          |
| a cache of a project | builds keyed by inputs  | A unit compiles only when its key misses, as Bazel does. |
| `dist/`              | builds by digest, locks | Stored and fetched as an OCI registry serves images.     |
| plugins in a config  | `BuildExtension`        | A dependency adds plugins to the packages using it.      |
