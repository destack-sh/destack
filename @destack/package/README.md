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
                {
                    "kind": "setting",
                    "package": "@destack/setting",
                    "function": "./inspect#describePreference"
                }
            ]
        }
    }
}
```

A kind's own entry in `describes` declares up to three functions over its descriptions.

| Function     | Answers                                                              | Used by                            |
| ------------ | -------------------------------------------------------------------- | ---------------------------------- |
| `function`   | what a declaration is                                                | the build, into the manifest       |
| `compare`    | what changed between two releases                                    | the build's upgrade                |
| `vocabulary` | which terms stored data holds, such as `object/note/relation/editor` | the build, the registry and spaces |

## Workspaces

A workspace root's `destack.json` holds the settings its members share under `workspace`, and needs no identity unless the root is itself a package.

```json
{
    "$schema": "https://destack.app/schemas/2026.10.0/destack.json",
    "workspace": {
        "check": {
            "expect": [
                {
                    "files": ["scripts/release.ts"],
                    "rules": ["eslint/no-console"],
                    "reason": "the release script reports to its terminal"
                }
            ]
        }
    }
}
```

The root's `check.expect` names files relative to the root, as `@destack/check` reads it, and `package.json` keeps listing the members.

## Capabilities

`capabilities` in `destack.json` declares what the package reaches beyond its sandbox, each with the `reason` a person reads when consenting.

```json
{
    "runtimes": ["bun"],
    "capabilities": {
        "process": { "reason": "indexes checkouts with native tools" },
        "network": {
            "connect": ["api.github.com", "*.githubusercontent.com:443"],
            "reason": "fetches linked repositories"
        },
        "listen": { "reason": "serves its index to the editor" },
        "fs": { "access": "read", "reason": "indexes the folders you choose", "optional": true },
        "env": { "names": ["GITHUB_TOKEN"], "reason": "authenticates to GitHub" },
        "run": { "commands": ["git"], "reason": "reads history" },
        "camera": { "reason": "scans receipts", "optional": true },
        "clipboard-write": { "reason": "copies search results" }
    }
}
```

The host's sandbox enforces `process`, `network`, `listen`, `fs`, `env` and `run` on workloads.

Every capability except `process` may be `optional`, and installing declines it until the installation allows it by name.
The person chooses the directories `fs` reaches when installing or later, and the installation keeps them.
A package's own data, cache and temporary folders need no capability.
The browser enforces the other capabilities, `BROWSER_CAPABILITIES`, on the installation's origin: the Permissions Policy features `camera`, `microphone`, `geolocation`, `display-capture`, `clipboard-read`, `clipboard-write`, `fullscreen`, `midi`, `usb`, `hid`, `serial`, `bluetooth`, `screen-wake-lock`, `idle-detection`, `local-fonts`, `window-management` and `xr-spatial-tracking`.
WebGPU, Web Audio, gamepads and notifications need no capability.

## Workload capabilities

A workload uses the host capabilities its package declares unless its definition names a subset, and the browser capabilities belong to the views.

```ts
import { Capabilities } from "@destack/package";
import { defineWorkload } from "@destack/service/workload";

export const indexer = defineWorkload({ name: "indexer", capabilities: ["network", "run"], start });

const running = Capabilities.grant(description.capabilities, ["fs", "camera"]);
```

The build writes each workload's selection into its `WorkloadDescription` and the package's browser capabilities into each `ViewDescription`.
`Capabilities.grant` narrows them to the required ones and the optional ones an installation allows.
A host that cannot grant a capability refuses the workload's start with a `CapabilityError` naming it.

## Packages

`definePackage` declares the resources and secrets a stack binds when it installs the package.

```ts
import { definePackage } from "@destack/package/declare";

export default definePackage({
    resources: { main: notesDatabase },
    secrets: { "github-webhook": webhookSecret },
});
```

## Modules

The module transform gives each module its package metadata and stamps declaration constructor calls with it.

```ts
const { id, name, version } = import.meta.destack.package;
```

## Transforms

The `transform/vite`, `transform/bun` and `transform/preload` entry points install the module transform in Vite and Vitest, in `Bun.build`, and in every module a Bun process loads.

## Variants

A module named after a target replaces its base module in that target's builds.

```text
src/page/page.ts           shared by every target
src/page/page.server.ts    replaces page.ts in server builds, Bun processes and tests
src/page/page.browser.ts   replaces page.ts in browser builds
```

## Builds

A dependency's `BuildExtension` compiles and describes the outputs of the packages that use it, and `@destack/package/build` holds the runtime facts every build shares, such as `TYPE_CHECKS`, the compiler checks every package compiles under.

```ts
import type { BuildExtension } from "@destack/package/build";

export const spaceBuild: BuildExtension = {
    compile: (compilation) => [workloadPlugin(compilation)],
    describe: (compilation, compiled) => ({ workloads }),
};
```

## Manifests

A `BuildReader` reads a built package's files and the descriptions its declarations produced.

```ts
import { BuildReader } from "@destack/package/manifest";

const reader = await BuildReader.open(location, fetch, signal);
const files = await reader.files();
const settings = await reader.declared(
    import.meta.destack.package.id,
    "setting",
    SettingDescription,
);
```
