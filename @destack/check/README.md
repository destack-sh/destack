# @destack/check

`destack-check` is Oxlint with type-aware rules through `oxlint-tsgolint` and Oxfmt, configured once per workspace, with Markdown rules for one sentence per line and Destack's README shape.

```sh
bun run destack-check check src # oxlint --type-aware
bun run destack-check fix src # oxlint --fix
bun run destack-check format src # oxfmt
bun run destack-check configure # writes .oxlintrc.json, .oxfmtrc.json and each tsconfig.json
```

## Expectations

`check.expect` in a `destack.json` accepts the findings of the named rules in the named files, and `check` reports an entry that matched nothing.

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

`check` requires one title, consecutive heading levels, one sentence per line and a language on each code block, and each `README.md` section to be one prose line followed by listings.

```md
# @example/site

Serve a site from a space.

## Pages

`Page.publish` publishes a page at its path.
```

## API

`checkPackage`, `fixPackage`, `formatPackage` and `formatSource` run the commands from code, and `plugins` adds trusted lint plugins.

```ts
import { checkPackage, formatSource } from "@destack/check";

const result = await checkPackage({ directory: ".", files: ["src"], plugins: [{ name: "database", specifier, rules }] });
const source = await formatSource("generated.ts", generatedSource);
```
