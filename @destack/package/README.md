# @destack/package

Define Destack packages, transform their modules, and read their built manifests.

## Definitions

A package's `destack.json` holds its identity, runtimes and declaration constructors.

```json
{
    "id": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "language": "typescript",
    "runtimes": ["browser", "bun", "workerd"],
    "declarations": {
        "defineNotification": {
            "module": 1,
            "describes": [
                {
                    "kind": "notification",
                    "function": "./inspect#describeNotification",
                    "vocabulary": "./inspect#notificationVocabulary"
                },
                { "kind": "setting", "package": "@destack/setting", "function": "./inspect#describePreference" }
            ]
        }
    }
}
```

A kind's own entry declares up to three functions over its descriptions.

| Function | Answers | Used by |
|---|---|---|
| `function` | what a declaration is | the build, into the manifest |
| `compare` | what changed between two releases | the build's upgrade |
| `vocabulary` | which terms stored data holds, such as `object/note/relation/editor` | the build, the registry and spaces |

## Packages

`definePackage` declares the resources and secrets a stack binds when it installs the package.

```ts
import { definePackage } from "@destack/package/declare";

export default definePackage({ resources: { main: notesDatabase }, secrets: { "github-webhook": webhookSecret } });
```

## Modules

The module transform gives each module its package metadata and stamps declaration constructor calls with it.

```ts
const { id, name, version } = import.meta.destack.package;
```

## Transforms

Each entry point installs the module transform in one toolchain.

| Entry point | Installs |
|---|---|
| `@destack/package/transform/vite` | `modulePlugin()` for Vite and Vitest |
| `@destack/package/transform/bun` | `modulePlugin` for `Bun.build` and `Bun.plugin` |
| `@destack/package/transform/preload` | The Bun plugin for every module a process loads |

## Variants

A module named after a target replaces its base module in that target's builds.

```text
src/page/page.ts           shared by every target
src/page/page.server.ts    replaces page.ts in server builds, Bun processes and tests
src/page/page.browser.ts   replaces page.ts in browser builds
```

## Builds

A dependency's `BuildExtension` compiles and describes the outputs of the packages that use it, and `@destack/package/build` holds the runtime facts every build shares.

```ts
import { type BuildExtension, runtimeConditions } from "@destack/package/build";

export const spaceBuild: BuildExtension = { compile: (compilation) => [workloadPlugin(compilation)], describe: (compilation, compiled) => ({ workloads }) };
```

## Manifests

A `BuildReader` reads a built package's files and the descriptions its declarations produced.

```ts
import { openPackage } from "@destack/package/manifest";

const reader = await openPackage(location, { fetch });
const files = await reader.files();
const settings = await reader.declared(import.meta.destack.package.id, "setting", SettingDescription);
```
