# @destack/package

Describe Destack packages, their declarations, builds and source code.

## Overview

Every Destack package has a `destack.json` beside its `package.json`, and its modules declare things the build reads back from the built package.

- **PackageDefinition**: a package's `destack.json`: its id, language, runtimes, the sandbox abilities its workloads use, and the kinds of its declarations.
- **PackageVersion**: a package's id, name and version, which the module transform passes as `import.meta.destack.package`.
- **Kind**: a kind of declaration, such as a resource a host provisions; `defineKind`.
- **Sandbox**: the abilities a workload uses, after Deno's permissions; `merge` and `select`.
- **BuildReader**: a built package opened by its manifest, every file checked against its digest; `declarations`, `files` and `load`.
- **Example**: one declaration in a given state, after a Storybook story; `defineExample`.

```ts
import { defineKind, ModuleMetadata, Sandbox } from "@destack/package";
import { schema } from "@destack/schema";

export const QueueKind = defineKind({
    name: "queue",
    provisioned: { spec: schema.object({ size: schema.number() }) },
});

export function defineQueue(options: { readonly name: string }, module?: ModuleMetadata) {
    const owner = ModuleMetadata.require(module, "defineQueue").package;

    return { owner, name: options.name };
}

const sandbox = Sandbox.merge(
    { network: { connect: ["api.github.com"] } },
    { run: { commands: ["git"] } },
);
const { id, name, version } = import.meta.destack.package;
```

## Examples

### Describe a package

`destack.json` sets a package's id, language, runtimes and the sandbox abilities it always uses.

```json
{
    "id": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "language": "typescript",
    "runtimes": ["browser", "bun", "workerd"],
    "sandbox": {
        "network": { "connect": ["api.github.com"] },
        "files": { "access": "read" }
    }
}
```

### Declare an example

`defineExample` shows one declaration with given properties, which a host renders and a control changes.

```ts
import { defineExample } from "@destack/package/declare";

const Badge = (properties: { readonly count?: number }) => `${properties.count ?? 0}`;

export const badgeMany = defineExample({
    of: Badge,
    name: "many",
    description: "A badge counting many unread notifications.",
    properties: { count: 120 },
    render: Badge,
});
```

### Read a built package

`BuildReader.open` opens a build at its location and reads its declarations and files.

```ts
import { BuildReader, type PackageLocation } from "@destack/package/manifest";
import { BunEnvironment } from "@destack/environment/bun";

async (location: PackageLocation) => {
    const { crypto } = BunEnvironment.system;
    const reader = await BuildReader.open(location, fetch, crypto);
    const declarations = await reader.declarations();
    const readme = await reader.load("README.md");
};
```

### Run the module transform

`@destack/package/bun/preload` installs the module transform in Bun, and `@destack/package/vite` in Vite and Vitest.

```toml
preload = ["@destack/package/bun/preload"]
```

## Comparison

### npm's package.json

`destack.json` sits beside `package.json` and adds what npm's manifest lacks; three things differ.

- A package declares the sandbox abilities its workloads use, after Deno's permissions, and a host grants nothing else.
- Declarations are values in modules, which the build reads back and stores in the package's manifest.
- A build is addressed by its manifest's digest, and every file is checked against it.
