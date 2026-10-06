# @destack/package

Define Destack packages, transform their modules, and read their built manifests.

## Definitions

`destack.json` sets a package's id, language, runtimes, describers and stamped functions.

```json
{
    "id": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "language": "typescript",
    "runtimes": ["browser", "bun", "workerd"],
    "declarations": {
        "defineNotification": {
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
    },
    "stamps": {
        "defineNotification": { "module": 1 }
    }
}
```

## Description functions

An entry in `describes` names the kind's functions: `function` describes a declaration, `compare` what changed between releases, `vocabulary` the terms stored values hold, `symbols` the members it derives.

```json
{
    "kind": "note",
    "function": "./inspect#describeNote",
    "compare": "./inspect#compareNote",
    "vocabulary": "./inspect#noteVocabulary",
    "symbols": "./inspect#noteSymbols"
}
```

## Symbols

A `symbols` function returns a `graph.MemberSymbol` for a declaration and each member it derives.

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

`workspace` in the root `destack.json` holds the settings that members share.

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

`capabilities` in `destack.json` declares what the package uses beyond its sandbox.

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

## Optional capabilities

`optional` marks a capability that an installation must allow by name.

```json
{ "fs": { "access": "read", "reason": "indexes the folders you choose", "optional": true } }
```

## Workload capabilities

`capabilities` in `defineWorkload` limits a workload to a subset of its package's capabilities.

```ts
import { defineWorkload } from "@destack/service/workload";

export const indexer = defineWorkload({ name: "indexer", capabilities: ["network", "run"], start });
```

## Granted capabilities

`Capabilities.grant` gives an installation the required capabilities and the optional ones it allows.

```ts
import { Capabilities } from "@destack/package";

Capabilities.grant(workload.capabilities, ["fs"]); // required ones plus fs, never camera
```

## Described capabilities

The build writes each workload's capabilities into its `WorkloadDescription`.

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

## Examples

`defineExample` declares one declaration in a given state, which a host renders with the example's properties or with ones a control changed.

```tsx
import { defineExample } from "@destack/package/declare";

export const GhostButton = defineExample({
    of: Button,
    name: "ghost",
    properties: { variant: "ghost" },
    render: (properties) => <Button {...properties}>Cancel</Button>,
});

GhostButton.render({ variant: "outline" });
```

## Scenarios

`defineScenario` declares examples set in motion as data: the interaction its steps and observations speak, the examples it is given, the steps a driver takes on them, and the named observations it expects after each step or once at the end.

```ts
import { defineScenario } from "@destack/package/declare";
import { viewInteraction } from "@destack/view/scenario";

export const MoveCalendarDay = defineScenario({
    interaction: viewInteraction,
    name: "move the focused day with the arrow keys, crossing months",
    given: { examples: [LateOctoberExample], environment: { locale: "de-AT" } },
    when: [
        { action: "focus", target: { role: "button", name: "Freitag, 30. Oktober 2026" } },
        { action: "press", key: "ArrowRight" },
        { action: "press", key: "ArrowRight" },
    ],
    then: {
        observe: { month: { kind: "text", target: { role: "heading" } } },
        each: [{ month: "Oktober 2026" }, { month: "Oktober 2026" }, { month: "November 2026" }],
    },
});
```

## Interactions

An `Interaction` names the schemas of the steps, observations and environment its scenarios speak, and a runner picks a scenario's driver by its name.

```ts
import type { Interaction } from "@destack/package/declare";

export const viewInteraction: Interaction<Step, Observation, Environment> = {
    name: "ui",
    step: Step,
    observation: Observation,
    environment: Environment,
};
```

## Set steps

`SetStep` changes properties an example declares, as its owner or a control would, in every interaction.

```ts
{ action: "set", properties: { open: true }, example: "controlled" }
```

## Example graph

The build declares each example with a `shows` edge to the symbol its `of` names, and each scenario with `covers` edges to what its examples show.

```text
src/button/button.example.tsx#GhostButton:example   shows   src/button/button.tsx#Button
src/tabs/tabs.scenario.ts#MoveManualTabs:scenario    covers  src/tabs/tabs.tsx#Tabs
```

## Controls

An example's description holds a control for each property its package declares on the shown symbol: a select for a union of literals, a switch, a number or a text field, labelled by the property's documentation.

```json
{
    "properties": { "variant": "ghost" },
    "controls": [
        {
            "property": "variant",
            "label": "The look.",
            "isOptional": true,
            "input": { "kind": "select", "options": ["default", "outline", "ghost"] }
        },
        { "property": "isPending", "isOptional": true, "input": { "kind": "boolean" } }
    ]
}
```

## Modules

The module transform sets `import.meta.destack.package` in each module.

```ts
const { id, name, version } = import.meta.destack.package;
```

### Stamps

`stamps` lists the functions that receive the calling module, by export path and parameter position.

```jsonc
{
    "stamps": {
        "t": { "module": 0 }, // t`Close` becomes t(__destackModule)`Close`
        "Message.context": { "module": 1 }, // Message.context("button")`Open` passes the module second
    },
}
```

## Transforms

`@destack/package/bun/preload` installs the module transform in Bun, and `modulePlugin` from `@destack/package/vite` in Vite and Vitest.

```toml
# bunfig.toml
preload = ["@destack/package/bun/preload"]
```

## Variants

A module with a `.server` or `.browser` suffix replaces its base module in builds for that target.

```text
src/page/page.ts           shared by every target
src/page/page.server.ts    replaces page.ts in server builds, Bun processes and tests
src/page/page.browser.ts   replaces page.ts in browser builds
```

## Builds

A `BuildExtension` transforms, compiles and describes the outputs of the packages that depend on it.

```ts
import type { BuildExtension } from "@destack/package/build";

export const spaceBuild: BuildExtension = {
    compile: (compilation) => [workloadPlugin(compilation)],
    describe: (compilation, compiled) => ({ workloads }),
};
```

## Manifests

`BuildReader.open` opens a built package by its manifest digest and reads its files and declarations, verifying every digest.

```ts
import { BuildReader } from "@destack/package/manifest";

const reader = await BuildReader.open(location, fetch, signal);
const settings = await reader.declared(import.meta.destack.package.id, "setting", SettingDescription);
```

## Build writers

`BuildWriter` writes a build in the format `BuildReader` reads: its files by digest, a graph file per module, the lists, and the manifest last.

```ts
import { BuildWriter } from "@destack/package/manifest";

const writer = new BuildWriter({
    write: async (path, bytes) => {
        await Bun.write(join(directory, path), bytes);
    },
});
await writer.write("src/index.js", bytes);
const manifest = await writer.finish({ package: notes, outputs, graph });
```

## Memory builds

`MemoryBuild` from `@destack/package/test` keeps a build in memory, written through a `BuildWriter`, for tests that install or run a release without compiling it.

```ts
import { MemoryBuild } from "@destack/package/test";

const empty = await MemoryBuild.write(new Map(), { package: notes });
const declaring = await MemoryBuild.declaring(notes, [
    { kind: "setting", package: setting.package.id, name: "theme", description: describeSetting(theme) },
]);
await declaring.reader.declarations(); // [{ kind: "setting", name: "theme", … }]
```

## Build caches

`BuildCache` keeps a value read from each build, keyed by its graph's digest.

```ts
import { BuildCache } from "@destack/package/manifest";

const catalogs = new BuildCache((reader) => SettingCatalog.read(reader));
const catalog = await catalogs.read(reader);
```

## Graph

`graph.Moniker.of` returns a moniker for a symbol that stays the same across builds.

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

A `graph.Module` holds a module's symbols, the declarations at them and its outgoing edges, and `graph.Declaration.at` declares a symbol as a kind.

```ts
const declaration = graph.Declaration.at(service, { kind: "service", package: packageId, name: "notes", description });
// { moniker: "package-…/src/server.ts#service:service", symbol: "package-…/src/server.ts#service", kind: "service", … }
```

## Graph reads

`graph` reads a build's root, and `module` reads one module's file by digest.

```ts
const root = await reader.graph();
const module = await reader.module(root.modules["src/server.ts"]);
```

## Errors

An invalid package throws a `PackageError`.

```ts
import { PackageError } from "@destack/package/error";

new PackageError("INVALID_FILE", "file digest mismatch: manifest.json").toServiceError(); // { code: "BAD_REQUEST", … }
```
