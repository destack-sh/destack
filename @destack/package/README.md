# @destack/package

Define Destack packages, transform their modules, and read their built manifests.

## Definitions

`destack.json` sets a package's id, language, runtimes and declaration constructors, and `describes` lists the functions that describe each kind a constructor declares.

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

## Description functions

An entry in `describes` names up to four functions for one kind: `function`, `compare`, `vocabulary` and `symbols`.

```jsonc
{
    "kind": "note",
    // what a declaration is, read by the build into the graph
    "function": "./inspect#describeNote",
    // what changed between two releases, read by the build's upgrade
    "compare": "./inspect#compareNote",
    // which terms stored values hold, such as object/note/relation/editor, read by the build, the registry and spaces
    "vocabulary": "./inspect#noteVocabulary",
    // which symbols a declaration derives and which declarations they name, read by the build into the graph
    "symbols": "./inspect#noteSymbols",
}
```

## Symbols

A `symbols` function returns a `graph.MemberSymbol` for the declaration and for each member it derives, and the build resolves each relationship target by kind and name in the declaring package unless the target names another package.

```ts
export function serviceSymbols(input: Record<string, JsonValue>): graph.MemberSymbol[] {
    const service = ServiceDescription.parse(input);
    const procedures = service.api.procedures.map((procedure) => ({
        kind: "procedure",
        name: procedure.name.join("."),
        description: procedure,
    }));

    return schema.array(graph.MemberSymbol).parse([
        {
            relationships: procedures.map((procedure) => ({
                kind: "serves",
                symbol: {
                    kind: "procedure",
                    name: procedure.name,
                    parent: { kind: "service", name: service.name },
                },
            })),
        },
        ...procedures.map((member) => ({ member, relationships: [] })),
    ]);
}
```

## Workspaces

`workspace` in the root `destack.json` holds the settings the members share, such as `check.expect` with files relative to the root, and `package.json` lists the members.

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

## Capabilities

`capabilities` in `destack.json` declares what the package uses beyond its sandbox, each with a `reason` that a person reads before consenting.

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

## Enforcement

The host's sandbox enforces the host capabilities on workloads, and the browser enforces the others as Permissions Policy features on the installation's origin.

```text
the host's sandbox  process, network, listen, fs, env, run
the browser         camera, microphone, geolocation, display-capture, clipboard-read, clipboard-write,
                     fullscreen, midi, usb, hid, serial, bluetooth, screen-wake-lock, idle-detection,
                     local-fonts, window-management, xr-spatial-tracking
no capability       the package's file, cache and temporary folders, WebGPU, Web Audio, gamepads, notifications
```

## Optional capabilities

`optional` marks a capability, other than `process`, that an installation declines until it allows the capability by name, and the person chooses the directories for `fs` during or after installation.

```json
{ "fs": { "access": "read", "reason": "indexes the folders you choose", "optional": true } }
```

## Workload capabilities

`capabilities` in `defineWorkload` limits a workload to a subset of its package's host capabilities, and `Capabilities.grant` keeps the required ones and the optional ones an installation allows.

```ts
import { Capabilities } from "@destack/package";
import { defineWorkload } from "@destack/service/workload";

export const indexer = defineWorkload({ name: "indexer", capabilities: ["network", "run"], start });

const running = Capabilities.grant(description.capabilities, ["fs", "camera"]);
```

## Described capabilities

The build writes each workload's capabilities into its `WorkloadDescription` and the package's browser capabilities into each `ViewDescription`.

```ts
const { capabilities } = build.manifest.outputs.bun.workloads.indexer; // { network: …, run: … }
const { capabilities: browser } = build.manifest.outputs.browser.views.notes; // { camera: … }
```

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

The module transform sets `import.meta.destack.package` in each module and passes it to each declaration constructor call.

```ts
const { id, name, version } = import.meta.destack.package;
```

## Transforms

`transform/vite` installs the module transform in Vite and Vitest, `transform/bun` in `Bun.build`, and `transform/preload` in every module a Bun process loads.

```toml
# bunfig.toml
preload = ["@destack/package/transform/preload"]
```

## Variants

A module with a `.server` or `.browser` suffix replaces its base module in builds for that target.

```text
src/page/page.ts           shared by every target
src/page/page.server.ts    replaces page.ts in server builds, Bun processes and tests
src/page/page.browser.ts   replaces page.ts in browser builds
```

## Builds

A dependency's `BuildExtension` compiles and describes the outputs of the packages that use it, and `@destack/package/build` exports values every build shares, such as the `TYPE_CHECKS` compiler checks.

```ts
import type { BuildExtension } from "@destack/package/build";

export const spaceBuild: BuildExtension = {
    compile: (compilation) => [workloadPlugin(compilation)],
    describe: (compilation, compiled) => ({ workloads }),
};
```

## Manifests

`BuildReader.open` opens a built package, and `declarations` and `declared` list its declarations without their derived members.

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

## Graph

`graph.Moniker.of` names a symbol, member or declaration in a build with a moniker that stays the same across builds.

```ts
import { graph } from "@destack/package";

const note = graph.Moniker.of({ packageId, module: "src/note.ts", name: "Note" });
// "package-…/src/note.ts#Note"
const title = graph.Moniker.of({ packageId, module: "src/note.ts", name: "Note", member: "title" });
// "package-…/src/note.ts#Note.title"
const object = graph.Moniker.of({ packageId, module: "src/note.ts", name: "Note", kind: "object" });
// "package-…/src/note.ts#Note:object"
```

## Graph modules

A `graph.Module` holds a module's symbols, declarations and outgoing edges, and a `graph.Root` names the file of each module by digest.

```json
{
    "path": "src/server.ts",
    "digest": "bfc1f6ca…",
    "imports": ["@destack/service"],
    "exports": [
        { "name": "service", "symbol": "package-…/src/server.ts#service", "isTypeOnly": false }
    ],
    "symbols": [
        {
            "moniker": "package-…/src/server.ts#service",
            "kind": "variable",
            "source": { "file": "src/server.ts", "start": 1686, "end": 1726 },
            "signature": "service: import(\"@destack/service\").Service<…>",
            "comment": "The public HTTP service.",
            "isExported": true
        }
    ],
    "declarations": [
        {
            "moniker": "package-…/src/server.ts#service:service",
            "symbol": "package-…/src/server.ts#service",
            "kind": "service",
            "package": "package-…",
            "name": "notes",
            "description": { "name": "notes" }
        },
        {
            "moniker": "package-…/src/server.ts#service.list:procedure",
            "symbol": "package-…/src/server.ts#service",
            "kind": "procedure",
            "package": "package-…",
            "name": "list",
            "description": { "method": "GET", "path": "/notes" }
        }
    ],
    "edges": [
        {
            "from": "package-…/src/server.ts#service:service",
            "to": "package-…/src/server.ts#service.list:procedure",
            "kind": "serves"
        }
    ]
}
```

## Graph reads

`graph` reads a build's root, and `module` reads one module's file by its digest and verifies the digest.

```ts
const root = await reader.graph();
const module = await reader.module(root.modules["src/server.ts"]);
```
