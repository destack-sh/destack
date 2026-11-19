# @destack/build

Compile Destack packages into one module output per runtime, and keep the builds by content.

## Overview

`@destack/build` is Vite on Rolldown per package: sandboxed compilers, builds stored by digest, and images of the builds a package runs with.

Workspaces build packages:

- `Workspace`: a checkout's packages, each built once on its dependencies' builds.
- `CompilerPool`: warm, sandboxed compilers, leased one package at a time.
- `BuildExtension`: Vite plugins a dependency adds to the packages using it.
- `Template`: a template package, written out as a new package.

Stores keep builds and images:

- `BuildStore` and `LocalBuildStore`: builds and images by digest, as an OCI registry keeps images.
- `BuildImage`: a root build and the build of each package it runs with, from `@destack/package`.
- `RemoteBuildStore`: pushes builds and images to a forge, and reads them for `pull`.

Build a package, store it, and make an image of it with what it runs with:

```ts
import { CompilerPool, outputsOf, Workspace } from "@destack/build";
import type { LocalBuildStore } from "@destack/build";
import { Toolchain } from "@destack/check/toolchain";

/** Build a checkout's package, and make an image of it with its dependencies. */
export async function buildAndLock(store: LocalBuildStore, directory: string) {
    // build the root on its dependencies' builds
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspace = await Workspace.open(directory, compilers);
    const opened = await workspace.package();
    const build = await opened.build({ outputs: outputsOf(opened.root.packageJson) });

    // store it, and make an image of it with every build it runs with
    const root = await store.put(build);

    return await opened.image(root);
}
```

## Reference

### Workspace

`Workspace.open(directory, compilers, { resolver, selection, host })` reads a checkout once, and `workspace.package(directory)` takes a package with the units it requires.
Each unit compiles only on a key miss, once per workspace, and the host's packages resolve to its images.

| Verb                     | What it does                                                |
| ------------------------ | ----------------------------------------------------------- |
| `build(request)`         | builds the root with given outputs                          |
| `imageUnits(options)`    | builds and makes an image of every unit, by name            |
| `image(root, options)`   | makes an image of a root build with every build it requires |
| `inspect(…)`, `index(…)` | inspects or indexes the root's code                         |

`isPrerelease` stamps each build as a dev version, `<release>-dev.<time>.<key>`.

### BuildStore

| Verb                               | What it does                                                   |
| ---------------------------------- | -------------------------------------------------------------- |
| `put(build)`                       | stores a build's missing files, then its manifest              |
| `putImage(image)`, `image(digest)` | stores or reads an image                                       |
| `has(digests)`                     | tells which blobs the store has, one flag per digest           |
| `read(objects)`, `write(entries)`  | reads or writes files, manifests and images as tarball entries |
| `pull(source, root, runtimes)`     | pulls an image's or build's objects from another store         |
| `record(digest)`                   | records an image whose blobs arrived, as a moved space's       |
| `entriesOf(image)`                 | reads an image's builds by package name                        |
| `referencing(digest)`              | lists the manifests and images referencing a blob or build     |
| `fetch(request, mount)`            | serves manifests, files and the store's verbs below a mount    |

A `LocalBuildStore` also lays out an image's tree for a runtime with `runTree`, and writes distribution layers with `writeLayer`.

### RemoteBuildStore

`new RemoteBuildStore(url, fetch)` is a build store another host serves over HTTP, with the same `has`, `read` and `write`.
`push(store, images)` lists the images' objects, asks which the server has, and writes the rest in bounded uploads.
`RemoteBuildStore.open(builds, fetch, image, crypto)` opens the root build of an image another host serves, refusing an image whose bytes do not match its digest.
`attachments()` lists the files a build's server attached, as OCI's referrers API lists them.

## Examples

### Build named outputs

Each output is a module for one runtime, and a browser output bundles its dependencies.

```ts
import type { WorkspacePackage } from "@destack/build";

/** Build a library for Bun and a bundle for browsers, at a dev version. */
export async function buildBoth(opened: WorkspacePackage) {
    return await opened.build({
        outputs: {
            library: { kind: "module", runtime: "bun" },
            browser: { kind: "module", runtime: "browser" },
        },
    });
}
```

### Push and pull an image

`push` writes only the objects the forge lacks, and `pull` reads an image in one read per round.

```ts
import { RemoteBuildStore } from "@destack/build/store";
import type { LocalBuildStore } from "@destack/build";
import type { Digest } from "@destack/schema";

/** Push a stored image to a forge's build mount, then pull it into another store for Bun. */
export async function pushThenPull(
    store: LocalBuildStore,
    other: LocalBuildStore,
    digest: Digest,
    url: string,
) {
    // push the image's objects the forge lacks
    await new RemoteBuildStore(url, fetch).push(store, [digest]);

    // pull what Bun runs of it into the other store
    const source = new RemoteBuildStore(`${url}image/${digest}/`, fetch);
    await other.pull(source, { kind: "image", digest }, ["bun"]);
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

| Vite                 | `@destack/build`         | Difference                                               |
| -------------------- | ------------------------ | -------------------------------------------------------- |
| one bundle of an app | a build per package      | Only a browser output bundles its dependencies.          |
| a cache of a project | builds keyed by inputs   | A unit compiles only when its key misses, as Bazel does. |
| `dist/`              | builds by digest, images | Stored and fetched as an OCI registry serves images.     |
| plugins in a config  | `BuildExtension`         | A dependency adds plugins to the packages using it.      |
