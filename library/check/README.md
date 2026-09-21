Check and format Destack TypeScript packages with Oxlint and Oxfmt.

```sh
deno run -A @destack/check/command check src
deno run -A @destack/check/command fix src
deno run -A @destack/check/command format src
deno run -A @destack/check/command format-check src
deno run -A @destack/check/command configure
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
