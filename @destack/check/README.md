# @destack/check

Lint, format and check Destack packages and their READMEs with one command.

## Overview

`destack-check` runs Oxlint with type-aware rules and Oxfmt under one configuration per workspace, and checks Markdown beside them.

- **Command**: `check`, `fix`, `format`, `format-check`, `configure`, `readme` and `affected`, each run in a package directory.
- **Toolchain**: the Oxlint, Oxfmt and TypeScript installation a check runs; `workspace` for the checkout's, `at` for another.
- **Plugin**: a named set of lint rules a host trusts, every rule an error.
- **Diagnostic**: one finding with its rule, file and position; a `CheckResult` lists them all.

```ts
import { checkPackage, Toolchain } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";

const result = await checkPackage({
    directory: "/work/site",
    files: ["src"],
    toolchain: Toolchain.workspace(),
    environment: BunEnvironment.system,
});
result.diagnostics.length; // 0
```

## Examples

### Check and format a package

The commands run from a package directory, on its files or on the ones given.

```sh
bun run destack-check check src
bun run destack-check fix src
bun run destack-check format src
bun run destack-check configure
```

### Accept a finding

`check.expect` in a package's `destack.json` accepts the named rules in the named files, and `check` reports an entry that matched nothing.

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

### Type-check README listings

`readme` writes each `ts` listing of a package's README as a separate module, which the package's `typecheck` then compiles.

```sh
bun run destack-check readme && tsc --project tsconfig.json
```

### Add lint rules

A host passes further rules as a plugin, enabled as errors.

```ts
import { checkPackage, defineRule, Toolchain } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";

const noDebugger = defineRule({
    create: (context) => ({
        DebuggerStatement: (node) => context.report({ node, message: "debugger is forbidden" }),
    }),
});
await checkPackage({
    directory: "/work/site",
    toolchain: Toolchain.workspace(),
    environment: BunEnvironment.system,
    plugins: [{ name: "site", specifier: "/work/rules.ts", rules: { "no-debugger": noDebugger } }],
});
```

### Format generated source

`formatSource` formats one file's text as `format` would.

```ts
import { formatSource, Toolchain } from "@destack/check";

const source = await formatSource("generated.ts", "export const a=1", Toolchain.workspace());
```

## Comparison

### Oxlint

Rules, plugins and fixes follow Oxlint, which `check` runs; three things differ.

- Every rule is an error and the configuration is fixed, so packages have no rule overrides.
- `check.expect` accepts a finding with a reason, and an expectation that no longer matches fails.
- Markdown gets further rules: one sentence per line, consecutive headings, a language per code block, and a README's shape.
