# @destack/package

Describe Destack packages, their declarations, builds and source code.

## Overview

`@destack/package` is npm's package model extended for Destack: a `destack.json` beside `package.json`, declarations read back from modules, and builds addressed by digest.

You describe a package and what it may use:

- `PackageDefinition`: a package's `destack.json`, with its id, role, runtimes, sandbox and kinds.
- `Sandbox`: the abilities a package's workloads and views use, such as the network or the camera.

Modules declare things:

- `Kind`: a kind of declaration your package defines for others, declared with `defineKind`.
- `Example`, `Scenario` and `Benchmark`: a declaration in a state, steps through it, and operations measured against limits.

Builds are stored by digest:

- `PackageManifest`: one build's files, outputs, requirements and interface.
- `Lock`: a root build and the build of each package it runs with.
- `BuildReader`: a build opened by its manifest, every file checked against its digest.

Define a kind, and read the calling package's version inside a module:

```ts
import { defineKind } from "@destack/package";
import type {} from "@destack/package/import-meta";
import { schema } from "@destack/schema";

// define a kind of resource that hosts provision
export const QueueKind = defineKind({
    name: "queue",
    provisioned: { spec: schema.object({ size: schema.number() }) },
});

// read the package this module belongs to
const { id, name, version } = import.meta.destack.package;
```

## Reference

### PackageDefinition

`destack.json` sits beside `package.json`, which keeps the name, version and dependencies.

| Field                 | What it is                                                                                           |
| --------------------- | ---------------------------------------------------------------------------------------------------- |
| `id`                  | the package's identity, fixed when it is created                                                     |
| `role`                | `"app"`, `"library"` or `"builtin"`                                                                  |
| `language`            | the source language, such as `"typescript"`                                                          |
| `runtimes`            | the runtimes its exports compile for: `browser`, `bun`, `workerd`                                    |
| `sandbox`             | the abilities its workloads and views always use                                                     |
| `kinds`               | the kinds it defines for its dependents                                                              |
| `declarations`        | the declaration constructors the build describes, by name                                            |
| `entrypoints`         | the modules a host starts beside the bin, such as Worker scripts                                     |
| `shares`              | the packages its programs share with the code they load                                              |
| `build`, `extensions` | the build extension it gives dependents, such as `./build#viewExtension`, and the ones that build it |

### Sandbox

A workload gets only the abilities its sandbox lists.

| Ability                                                     | What it allows                                              |
| ----------------------------------------------------------- | ----------------------------------------------------------- |
| `network: { connect }`                                      | connections to the listed hosts                             |
| `server`                                                    | serving connections on loopback                             |
| `files: { access }`                                         | reading or writing the directories a person grants          |
| `env: { names }`                                            | reading the listed environment variables                    |
| `run: { commands }`                                         | spawning the listed commands                                |
| `process`                                                   | running in a Bun process instead of a workerd isolate       |
| `camera`, `microphone`, `geolocation`, `display-capture`, … | the browser features of the same name in Permissions Policy |

`Sandbox.merge` joins sandboxes, `Sandbox.select` keeps some abilities, and `Sandbox.policy` writes the Permissions Policy.

### Declarations

A module declares things by calling constructors, and the build lists them.

| Constructor                                                   | What it declares                                          |
| ------------------------------------------------------------- | --------------------------------------------------------- |
| `defineKind({ name, description })`                           | a kind whose declarations are described by a schema       |
| `defineKind({ name, provisioned: { spec, state } })`          | a kind of resource that providers provision               |
| `defineExample({ of, name, properties, render })`             | one declaration in a given state, after a Storybook story |
| `defineScenario({ of, interaction, given, when, then })`      | steps through examples and the observations they expect   |
| `defineBenchmark({ interaction, setup, operations, limits })` | operations measured per iteration against count limits    |

The module transform passes each constructor its package, as `import.meta.destack.package` has it.
`@destack/package/bun/preload` installs the transform in Bun, and `@destack/package/vite` in Vite and Vitest.

### PackageManifest

A manifest describes one build, and its digest is the build's address in every store.

| Field       | What it is                                                                                |
| ----------- | ----------------------------------------------------------------------------------------- |
| `package`   | the package and version it builds, or an npm release                                      |
| `role`      | what the package is                                                                       |
| `destack`   | the Destack version whose toolchain built it                                              |
| `requires`  | the packages it requires, by name, at their declared ranges                               |
| `lists`     | the digests of its file, source map and declaration lists                                 |
| `interface` | the digest of what dependents' compilations read, so an unchanged one skips their rebuild |
| `outputs`   | its compiled outputs by name, each with its runtime and exports                           |

### Lock

A lock is `{ root, builds }`: a root build, and one build per package it runs with.
`Lock.digest` addresses it, `Lock.builds` lists every build, and `Lock.entries` reads them with their packages.

### BuildReader

`BuildReader.open(location, fetch, crypto)` opens a build a store serves, and `openDirectory` one on disk.

| Verb                         | What it does                                           |
| ---------------------------- | ------------------------------------------------------ |
| `digest()`                   | reads the manifest's digest                            |
| `files()`                    | lists the source, executable and asset files           |
| `load(path)`, `stream(path)` | reads a file whole or as a stream, checking its digest |
| `read(path, schema)`         | reads a JSON file with a schema                        |
| `declarations()`             | reads the declarations the build described             |
| `loaded(output)`             | lists the files an output's runtime loads              |
| `writeDirectory(directory)`  | writes the build to a directory, its manifest last     |

## Examples

### Describe a package

`destack.json` sets a package's id, role, runtimes and the abilities it always uses.

```json
{
    "id": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "role": "app",
    "language": "typescript",
    "runtimes": ["browser", "bun", "workerd"],
    "sandbox": {
        "network": { "connect": ["api.github.com"] },
        "files": { "access": "read" }
    }
}
```

### Declare an example

`defineExample` shows a declaration with given properties.

```ts
import { defineExample } from "@destack/package/declare";

/** A badge with an unread count. */
const Badge = (properties: { readonly count?: number }) => `${properties.count ?? 0}`;

export const badgeMany = defineExample({
    of: Badge,
    name: "many",
    description: "A badge counting many unread notifications.",
    properties: { count: 120 },
    render: Badge,
});
```

### Read a build's README

`BuildReader.open` checks every file it reads against the manifest.

```ts
import { BuildReader, type PackageLocation } from "@destack/package/manifest";
import { BunEnvironment } from "@destack/environment/bun";

/** Read the README a build ships. */
export async function readReadme(location: PackageLocation): Promise<string> {
    // open the build, then read the file against its digest
    const reader = await BuildReader.open(location, fetch, BunEnvironment.system.crypto);
    const bytes = await reader.load("README.md");

    return new TextDecoder().decode(bytes);
}
```

## Comparison

### npm

`destack.json` adds what npm's `package.json` lacks, and locks work like a flat `package-lock.json`.

| npm                  | `@destack/package`           | Difference                                                |
| -------------------- | ---------------------------- | --------------------------------------------------------- |
| `package.json`       | `destack.json` beside it     | Adds an id, runtimes, a sandbox and kinds.                |
| a tarball by version | `PackageManifest`            | Addressed by its digest, with every file checked.         |
| `package-lock.json`  | `Lock`                       | One build per name, each by digest.                       |
| a dev version        | `2026.11.0-dev.<time>.<key>` | Stamped from the build's key, as in Go's pseudo-versions. |

### Deno permissions

`Sandbox` follows Deno's permissions, declared in the package instead of given as flags.

| Deno                            | `Sandbox`              | Difference                                        |
| ------------------------------- | ---------------------- | ------------------------------------------------- |
| `--allow-net=host`              | `network: { connect }` | The same.                                         |
| `--allow-read`, `--allow-write` | `files: { access }`    | Only directories a person grants.                 |
| `--allow-env=NAME`              | `env: { names }`       | The same.                                         |
| `--allow-run=cmd`               | `run: { commands }`    | The same.                                         |
| none                            | `camera`, `usb`, …     | Browser features, as Permissions Policy has them. |
