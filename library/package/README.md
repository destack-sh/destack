Describe packages, source code, inspections, and built files.

## Usage

```ts
import { PackageDeclaration } from "@destack/package";

const declaration: PackageDeclaration = {
    package: { name: "@destack/schema", version: "2026.9.0" },
    definition: { language: "typescript", targets: ["browser", "server"] },
    exports: { ".": "./src/index.ts" },
    dependencies: { zod: "4.6.5" },
    peerDependencies: {},
    peerDependenciesMeta: {},
    optionalDependencies: {},
    devDependencies: {},
};
```

```ts
import { PackageManifest } from "@destack/package";

const manifest = PackageManifest.parse(document);
```

## Workloads

Workloads select code declarations and override package compute defaults.
The build collects resource and secret references from each workload's module dependencies.

```json
{
    "language": "typescript",
    "targets": ["browser", "server"],
    "compute": {
        "requests": { "cpu": 1, "memory": 512 },
        "limits": { "cpu": 2, "memory": 1024 },
        "scaling": { "minInstances": 0, "maxInstances": 4 }
    },
    "workloads": {
        "web": {
            "entrypoint": "./server",
            "services": ["web"],
            "compute": { "requests": { "memory": 768 } }
        }
    }
}
```

## Dependencies

```ts
import { BuildDescription } from "@destack/package/inspect";

const build = BuildDescription.parse(document.descriptions);
const { packages, inputs, outputs } = build;
```

Bundled dependencies appear in the build description; retained runtime imports appear in each output's `dependencies`.

## Globals

```ts
import type {} from "@destack/package/import-meta";

const { name, version } = import.meta.destack.package;
```

## Files

```ts
import { describeFile } from "@destack/package/file";

const file = await describeFile("src/index.ts", "text/plain", bytes);
```

## Inspection

```json
{
    "language": "typescript",
    "inspect": { "module": "src/inspect/index.ts", "export": "inspectPackage" }
}
```

```ts
import { schema } from "@destack/schema";
import { describeTable } from "@destack/db";
import { TableDescription } from "@destack/db/inspect";
import { ModuleGraph, SymbolReference } from "@destack/package/code";
import { createPackageInspection } from "@destack/package/inspect";
import { tables } from "../db/index.ts";

const description = schema.object({
    tables: schema.array(schema.object({
        symbol: SymbolReference,
        description: TableDescription,
    })),
});

export function inspectPackage(code: ModuleGraph) {
    return createPackageInspection(import.meta.destack.package.name, 1, code, description, {
        tables: Object.entries(tables).map(([name, table]) => ({
            symbol: code.resolveExport("src/index.ts", name),
            description: describeTable(table),
        })),
    });
}
```
