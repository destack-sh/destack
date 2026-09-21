Check and format Destack TypeScript packages with Oxlint and Oxfmt.

```sh
bun run destack-check check src
bun run destack-check fix src
bun run destack-check format src
bun run destack-check format-check src
bun run destack-check configure
```

```ts
import { checkPackage, formatPackage } from "@destack/check";

const result = await checkPackage({ directory: ".", files: ["src"] });
await formatPackage({ directory: ".", files: ["src"] });
```

```ts
import { formatSource } from "@destack/check";

const source = await formatSource("generated.ts", generatedSource);
```

```ts
import { checkPackage } from "@destack/check";
import { fileURLToPath } from "node:url";
import databaseRules from "@example/database/lint";

await checkPackage({
    directory: ".",
    plugins: [
        {
            name: "database",
            specifier: fileURLToPath(import.meta.resolve("@example/database/lint")),
            rules: databaseRules.rules,
        },
    ],
});
```
