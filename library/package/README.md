Describe packages, source code, inspections, and built files.

## Usage

```ts
import { parseDeclaration } from "@destack/package";

const declaration = parseDeclaration(
    {
        name: "@destack/schema",
        version: "2026.9.0",
        exports: { ".": "./src/index.ts" },
        dependencies: { zod: "4.6.5" },
    },
    { language: "typescript", targets: ["browser", "worker", "host"] },
);
```

```ts
import { parseManifest } from "@destack/package";

const manifest = parseManifest(document);
```

## Module metadata

```ts
import type {} from "@destack/package/import-meta";

const { name, version } = import.meta.destack.package;
```

## Files

```ts
import { describeFile, verifyFile } from "@destack/package/file";

const file = await describeFile("src/index.ts", "text/plain", bytes);
await verifyFile(file, bytes);
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
    return createPackageInspection("@destack/model", 1, code, description, {
        tables: Object.entries(tables).map(([name, table]) => ({
            symbol: code.resolveExport("src/index.ts", name),
            description: describeTable(table),
        })),
    });
}
```
