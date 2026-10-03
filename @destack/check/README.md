Check and format Destack TypeScript packages and their Markdown with Oxlint and Oxfmt.

## Commands

`destack-check` runs each command over the given files or directories of the current package.

```sh
bun run destack-check check src
bun run destack-check fix src
bun run destack-check format src
bun run destack-check format-check src
bun run destack-check configure
```

`check`, `fix` and `format` refuse a directory whose copies of the lint and format settings differ from the shared ones.

## Configuration

`configure` writes `.oxlintrc.json` and `.oxfmtrc.json` once per workspace at its root, in a template, or in a package outside any workspace, and `tsconfig.json` in each package.

| `tsconfig.json` sets                        | When the package                                                                                      |
| ------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| the shared compiler options and checks      | always, including `src`, `tests` and `*.config.ts` without nested packages                            |
| the `DOM` libraries                         | runs in a browser, has TSX views, uses `@destack/style` or `@opentui/solid`, or declares no Bun types |
| `bun` or `node` types                       | declares `@types/bun` or `@types/node`                                                                |
| `@destack/build/browser` types              | runs in a browser and declares `@destack/build`                                                       |
| `vite/client` types                         | declares `vite`                                                                                       |
| JSX for `@opentui/solid` or `@destack/view` | declares `@opentui/solid`, or has TSX modules using `@destack/view`                                   |

## Expectations

`check` accepts the findings that the `check.expect` entries of the covering workspace's `destack.json` files name, from whichever directory it runs in, and reports an entry over checked files that nothing matched.

```json
{
    "check": {
        "expect": [
            {
                "files": ["src/browser/devtools.ts"],
                "rules": ["eslint/no-console"],
                "reason": "the devtools exporter writes telemetry to the browser console"
            }
        ]
    }
}
```

## Markdown

`check` holds Markdown files to the Destack prose rules, such as one sentence per line and a single title.

## API

`checkPackage`, `fixPackage`, `formatPackage` and `formatSource` run the same checks from code.

```ts
import { checkPackage, formatPackage, formatSource } from "@destack/check";

const result = await checkPackage({ directory: ".", files: ["src"] });
await formatPackage({ directory: ".", files: ["src"] });
const source = await formatSource("generated.ts", generatedSource);
```

## Plugins

A host adds trusted lint plugins, with every rule enabled.

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
