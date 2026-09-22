Describe packages, source code, inspections, and built files.

```ts
import { PackageDeclaration, PackageId } from "@destack/package";

const id = PackageId.parse("package-01996ab0-0000-7000-8000-000000000001");

const declaration: PackageDeclaration = {
    package: { id, name: "@destack/schema", version: "2026.9.0" },
    definition: { id, language: "typescript", targets: ["browser", "server"] },
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

```json
{
    "id": "package-01996ab0-0000-7000-8000-000000000001",
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

```ts
import { BuildDescription } from "@destack/package/inspect";

const build = BuildDescription.parse(document.descriptions);
const { packages, inputs, outputs } = build;
```

```ts
import type {} from "@destack/package/import-meta";

const { id, name, version } = import.meta.destack.package;
```

```ts
import { describeFile } from "@destack/package/file";

const file = await describeFile("src/index.ts", "text/plain", bytes);
```

```ts
import { schema } from "@destack/schema";
import { describeTable } from "@destack/db";
import { TableDescription } from "@destack/db/inspect";
import { ModuleGraph, SymbolReference } from "@destack/package/code";
import { createPackageInspection } from "@destack/package/inspect";
import { tables } from "../db/index.ts";

const description = schema.object({
    tables: schema.array(
        schema.object({
            symbol: SymbolReference,
            description: TableDescription,
        }),
    ),
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
