# @destack/build

Compile Destack packages into one module output per runtime, and keep the builds by content.

## Overview

A workspace builds each Destack package it requires on the builds of its dependencies, compiling with Vite on Rolldown in a sandboxed compiler.

- **Compiler**: one sandboxed Vite process kept warm across builds; `build`, `check`, `inspect` and `index`.
- **CompilerPool**: the warm compilers of one store, leased one package at a time with `use`.
- **Workspace**: a checkout's packages read once, each after the packages it requires; `build` compiles its root.
- **BuildReader**: one build's manifest and files, addressed by the manifest's digest.
- **BuildStore**: builds kept by content in a bucket, as an OCI registry keeps images; `put`, `get`, `closure` and `fetch`.
- **BuildExtension**: the Vite plugins a dependency adds to the packages that use it, named by `build` in its `destack.json`.

```ts
import { CompilerPool, LocalBuildStore, outputsOf, Workspace } from "@destack/build";
import { Toolchain } from "@destack/check/toolchain";

async (store: LocalBuildStore, directory: string) => {
    await using compilers = await CompilerPool.open(store, async () => Toolchain.workspace());
    const workspace = await Workspace.open(directory, compilers);
    const outputs = outputsOf(workspace.root.packageJson);
    const build = await workspace.build({ outputs, timeout: 60_000 });
    const digest = await store.put(build);
};
```

## Examples

### Build named outputs

A build compiles the named module outputs, each for one runtime, and a browser output bundles its dependencies.

```ts
import type { Workspace } from "@destack/build";

async (workspace: Workspace) => {
    const build = await workspace.build({
        outputs: {
            library: { kind: "module", runtime: "bun" },
            browser: { kind: "module", runtime: "browser" },
        },
        versions: { "@example/notes": "2026.11.0-dev.3" },
    });
};
```

### Add a build extension

A package's `destack.json` points `build` at a `BuildExtension`, and every package that depends on it compiles with its plugins.

```ts
import type { BuildExtension } from "@destack/package/build";

export const markdownExtension: BuildExtension = {
    transform: () => [],
};
```

### Push a build to a registry

`PackageClient` pushes a build's blobs and then its manifest, as an OCI client does.

```ts
import { PackageClient } from "@destack/build/store";
import { BunEnvironment } from "@destack/environment/bun";
import type { BuildReader } from "@destack/package/manifest";

async (build: BuildReader) => {
    const url = "https://packages.example.com/builds/";
    const client = new PackageClient(url, fetch, BunEnvironment.system);
    const digest = await client.push(build);
};
```

### Create a package from a template

`Template.read` reads a template package, and `write` creates a new package from it with a fresh identity.

```ts
import { Template } from "@destack/build/template";
import { Toolchain } from "@destack/check/toolchain";
import { BunEnvironment } from "@destack/environment/bun";
import type { TemplateInput } from "@destack/package/template";

async (parameters: TemplateInput) => {
    const { files } = BunEnvironment.system;
    const template = await Template.read("/templates/blank", files);
    await template.write("/work/site", parameters, Toolchain.workspace(), files);
};
```

## Comparison

### Vite

Outputs, plugins and the module graph follow Vite on Rolldown; Destack's builds differ in three ways.

- Each Destack package builds once on its dependencies' builds, never as one bundle of the whole tree, and a browser output alone bundles.
- Builds are kept by content with a manifest of digests, so a machine fetches and checks them as an OCI registry serves images.
- Compilers run sandboxed and stay warm across a workspace's builds.
