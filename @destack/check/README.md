# @destack/check

Lint, format and check Destack packages and their READMEs with one command.

## Overview

`@destack/check` is Destack's toolchain: one fixed configuration of Oxlint, Oxfmt and TypeScript, Destack's lint rules, and the rule book, pinned per project as a development dependency.

Projects run its commands through `destack check`, `destack fix` and `destack test`:

- `Project`: a package the toolchain checks, fixes, configures and tests, its own `check`, `fix` scripts run after.
- `destack-check`: the same commands from the package it runs in, for the workspace's recipes.
- `TestStore`: the test files that passed over each package's inputs, here and in a shared bucket, with their durations.
- `Schedule`: the CI runners for packages, their checks and unpassed test files packed by duration.
- `checkPackage` and `fixPackage`: the lint checks as functions.
- `AGENTS.md`: the rule book `configure` writes into each workspace root, above the workspace's own `## Custom` section.

Hosts extend and run the tools:

- `Toolchain`: the Oxlint, Oxfmt and TypeScript installation a check runs.
- `defineRule` and `Plugin`: further lint rules a host trusts, every rule an error.
- `Diagnostic`: one finding with its rule, file and position.

Check a package's sources and READMEs from its directory:

```ts
import { checkPackage, Toolchain } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";

// check the package's sources and Markdown
const result = await checkPackage({
    directory: "/work/site",
    files: ["src", "README.md"],
    toolchain: Toolchain.workspace(),
    environment: BunEnvironment.system,
});
```

## Reference

### Project

`Project.open(directory, environment)` opens a package with the toolchain that the CLI resolved from it.

| Method          | What it does                                                                              |
| --------------- | ----------------------------------------------------------------------------------------- |
| `check(since?)` | formats, typechecks sources and tests, typechecks README listings and lints, as one list  |
| `fix()`         | writes the generated settings, formats and applies the lints' fixes                       |
| `test(options)` | runs the package's `TestPlan` files not yet passed over its inputs, Bun then Node targets |
| `inputs(options)` | digests what a test run reads besides its files: packages, lockfile, runtime, options |
| `configure()`   | writes `tsconfig.json`, `.oxlintrc.json`, `.oxfmtrc.json` and a workspace's `AGENTS.md`   |

### TestStore

`TestStore.open(environment)` keeps records in each package's `node_modules/.cache/destack/test`, and in the S3 bucket `DESTACK_CI_STORE` names when `DESTACK_CI_STORE_KEY_ID` and `DESTACK_CI_STORE_TOKEN` are set, as R2's token identifier and token.

| Method                                    | What it does                                                    |
| ----------------------------------------- | --------------------------------------------------------------- |
| `passed(directory, name, inputs)`        | lists the package's test files that passed over the inputs     |
| `pass(directory, name, inputs, durations)` | records a run's passed files and how long each took           |
| `durations(name)`                         | reads each file's duration from the package's newest records    |
| `check(name, milliseconds)`, `checked(name)` | records and reads how long the package's check took          |

### destack-check

`destack-check <command> [files]` runs in a package directory, on its files or the ones given.

| Command     | What it does                                               |
| ----------- | ---------------------------------------------------------- |
| `check`     | lints sources and checks Markdown                          |
| `fix`       | applies the safe fixes, then checks                        |
| `format`    | formats the files, `format-check` only reports             |
| `configure` | writes the tools' settings for the workspace               |
| `readme`    | typechecks the README's `ts` listings in their own program |
| `affected`  | lists the packages the changes since a base commit affect  |
| `schedule`  | packs packages' checks and unpassed test files onto CI runners |

`check.expect` in `destack.json` accepts named rules in named files with a reason, and an entry that matches nothing fails.

### Rules

Every rule is an error, beside Oxlint's own rules.

| Rule                      | What it requires                                                       |
| ------------------------- | ---------------------------------------------------------------------- |
| `boolean-prefix`          | question-shaped names for booleans, such as `isOpen`                   |
| `branch-comment-position` | each if and else case's comment before the case                        |
| `comment-style`           | short lowercase line comments                                          |
| `deterministic-handler`   | no clock, randomness or network in handlers and workflows              |
| `error-message-style`     | lowercase error messages without a final period                        |
| `exact-optional`          | schema properties that may be absent to refuse an explicit `undefined` |
| `jsdoc-sentence`          | documentation comments as sentences, right above their subject         |
| `method-signature-style`  | function members of interfaces as properties                           |
| `no-class-name`           | StyleX instead of class names                                          |
| `no-import-alias`         | no renamed imports                                                     |
| `no-index-logic`          | index modules with re-exports only                                     |
| `no-inline-config`        | no comments changing rules, except reasoned next-line suppressions     |
| `no-ambient-effect`       | host effects only in the environment's host modules                    |
| `no-manifest-import`      | no package manifests read from source                                  |
| `no-overload-cast`        | an implementation returning what its overloads return                  |
| `no-partial-assertions`   | tests asserting complete values                                        |
| `no-test-timeout`         | tests taking timeouts from their tier alone                            |
| `no-system-environment`   | system effects only in entrypoints and the environment package         |
| `padding-before-return`   | a blank line before a final return                                     |
| `package-layout`          | only imports the package layout allows                                 |
| `prevent-abbreviations`   | whole words in declared names                                          |
| `require-block-comment`   | a comment at the start of each block of a longer function              |
| `require-jsdoc`           | documentation on declarations, members and fields                      |
| `style-attribute`         | StyleX styles passed as `xstyle`, inline styles as `style`             |
| `style-focus`             | focus rings drawn with the theme's shared focus style                  |
| `style-hover`             | hover styles only where the pointer hovers                             |
| `style-shorthand`         | one value per longhand property                                        |
| `style-tokens`            | styles from the theme's tokens                                         |
| `style-xstyle-last`       | a caller's `xstyle` merged after a component's styles                  |
| `valid-package-handle`    | one package handle, default-exported from `src/package.ts`             |

| Markdown rule           | What it requires                                         |
| ----------------------- | -------------------------------------------------------- |
| `one-sentence-per-line` | one sentence per line                                    |
| `single-title`          | one title                                                |
| `heading-increment`     | headings one level apart                                 |
| `fenced-code-language`  | a language on every code block                           |
| `readme-description`    | a README's first line equal to its package's description |

## Examples

### Accept a finding

An expectation names its files, rules and reason.

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

### Add a lint rule

A host passes its own rules as a plugin.

```ts
import { checkPackage, defineRule, Toolchain } from "@destack/check";
import { BunEnvironment } from "@destack/environment/bun";

// refuse debugger statements
const noDebugger = defineRule({
    create: (context) => ({
        DebuggerStatement: (node) => context.report({ node, message: "debugger is forbidden" }),
    }),
});

// check a package with the rule
await checkPackage({
    directory: "/work/site",
    toolchain: Toolchain.workspace(),
    environment: BunEnvironment.system,
    plugins: [{ name: "site", specifier: "/work/rules.ts", rules: { "no-debugger": noDebugger } }],
});
```

## Comparison

### Oxlint

Rules, plugins and fixes follow Oxlint, which `check` runs.

| Oxlint                       | `@destack/check`  | Difference                                                   |
| ---------------------------- | ----------------- | ------------------------------------------------------------ |
| `.oxlintrc.json` per project | one configuration | Packages can't override rules.                               |
| `// oxlint-disable`          | `check.expect`    | Accepts a finding with a reason, and fails once it is stale. |
| JavaScript and TypeScript    | also Markdown     | READMEs get their own rules and typechecks.                  |
