Define Destack packages, transform their modules, and read their built manifests.

## Definition

A package's `destack.json` holds what its code cannot declare.

```json
{
    "id": "package-01996ab0-0000-7000-8000-000000000001",
    "language": "typescript",
    "targets": ["browser", "server"],
    "declarations": {
        "defineDatabase": { "module": 1, "inspect": { "kind": "resource", "describe": "./inspect#describeDatabase" } },
        "defineTable": { "module": 3 }
    }
}
```

## Modules

The transform gives each module its package metadata and stamps declaration constructor calls with it.

```ts
const { id, name, version } = import.meta.destack.package;
```

A module can have a variant per target, named after it, which replaces the module for that target.

```text
src/page/page.ts           shared by every target
src/page/page.server.ts    replaces page.ts in server builds, Bun processes and tests
src/page/page.browser.ts   replaces page.ts in browser builds
```

- A variant starts with `export * from "./page.ts"` and may add or replace exports.
- Importing another target's variant explicitly fails the build.
- Declarations belong in the base module, since inspection loads bases only.

## Manifests

A build describes a package's files, outputs and declarations in its manifest.

```ts
import { openPackage } from "@destack/package/manifest";

const reader = await openPackage(location, { fetch });
const files = await reader.files();
const services = await reader.domain("service", schema.array(DeclarationDescription));
```

Manifests describe the declarations the package owns, and refers to its dependencies' declarations by package and name.
