# @destack/check

Check and format Destack TypeScript packages and their Markdown with Oxlint and Oxfmt.

## Commands

`destack-check` runs a command over files or directories of the current package, and `check`, `fix` and `format` refuse configuration files that differ from what `configure` writes.

```sh
bun run destack-check check src
bun run destack-check fix src
bun run destack-check format src
bun run destack-check format-check src
bun run destack-check configure
```

## Configuration

`configure` writes `.oxlintrc.json` and `.oxfmtrc.json` at the workspace root, in a template or in a package outside any workspace, and writes `tsconfig.json` in each package.

```jsonc
{
    "compilerOptions": {
        // the shared compiler options and checks, in every package
        "strict": true,
        // the DOM libraries for a package that runs in a browser, has TSX views, uses @destack/style or @opentui/solid, or declares no Bun types
        "lib": ["ESNext", "DOM", "DOM.Iterable"],
        // bun or node by the declared type package, @destack/build/browser in a browser with @destack/build, and vite/client with vite
        "types": ["bun", "@destack/build/browser", "vite/client"],
        // JSX for @opentui/solid when declared, else for @destack/view in a package with TSX modules using it
        "jsx": "preserve",
        "jsxImportSource": "@destack/view",
    },
    // the sources and configuration files, without nested packages
    "include": ["src", "tests", "*.config.ts"],
}
```

## Expectations

`check.expect` in a `destack.json` accepts the findings of the named rules in the named files, and `check` reports an entry that matched nothing in the checked files.

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

`check` requires one sentence per line, one title, consecutive heading levels and a language on each code block in Markdown files.

```md
# @example/notes

Keep notes in notebooks.
Share each notebook with the people who need it.

## Sharing

A notebook's owner shares it with a person or a group.
```

## Package README

`check` requires each section of a `README.md` to be one prose line followed by listings, such as code blocks and lists, and reports tables.

```md
# @example/notes

Keep notes in notebooks.

## Sharing

`Notebook.share` shares a notebook in one of two roles.

- `reader` reads the notebook.
- `writer` edits the notebook.
```

## API

`checkPackage`, `fixPackage`, `formatPackage` and `formatSource` run the commands from code.

```ts
import { checkPackage, formatPackage, formatSource } from "@destack/check";

const result = await checkPackage({ directory: ".", files: ["src"] });
await formatPackage({ directory: ".", files: ["src"] });
const source = await formatSource("generated.ts", generatedSource);
```

## Plugins

`plugins` adds trusted lint plugins to `checkPackage` and enables every rule they define.

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

## Errors

A failed checker, a bad selection or an unsupported package throws a `CheckError` with the code `TOOL`, `CONFIGURATION` or `LANGUAGE`, and `toServiceError` names the service error its caller receives.

```ts
import { CheckError } from "@destack/check/error";

new CheckError("CONFIGURATION", "unknown command: tidy").toServiceError(); // { code: "BAD_REQUEST", … }
```
