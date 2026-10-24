# @destack/package

`destack.json` is a package's `package.json` companion whose `sandbox` lists the abilities its workloads use, after Deno's permissions, `defineExample` is a Storybook story with its `args` and controls, `defineScenario` is a Storybook play function written as Playwright-style steps, `graph` keeps a build's modules and declarations under SCIP monikers, and `graph.Document` keeps one source file of the code index with its symbols and edges, as SCIP's `Document` does.

```ts
// deno run --allow-net=api.github.com --allow-env=GITHUB_TOKEN --allow-run=git
const sandbox = {
    network: { connect: ["api.github.com"] },
    env: { names: ["GITHUB_TOKEN"] },
    run: { commands: ["git"] },
};
// a Storybook story's args
defineExample({ of: Button, name: "ghost", description, properties: { variant: "ghost" }, render });
// a story's play function
defineScenario({
    of: Calendar,
    interaction: viewInteraction,
    name,
    description,
    given,
    when,
    then,
});
// "package-…/src/page.ts#Page", after SCIP's symbols
graph.Moniker.of({ packageId, module: "src/page.ts", name: "Page" });
const { id, name, version } = import.meta.destack.package;
```

## Packages

`definePackage` declares the resources and secrets a stack binds when it installs the package.

```ts
import { definePackage } from "@destack/package/declare";

export default definePackage({ resources: { main: pagesDatabase } });
```

## Definitions

`destack.json` sets a package's id, language, runtimes, the sandbox abilities it always uses and the kinds of its declarations: its own with their functions, a dependency's with the value that kind's functions inspect.

```json
{
    "id": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "language": "typescript",
    "runtimes": ["browser", "bun", "workerd"],
    "sandbox": {
        "network": { "connect": ["api.github.com"] },
        "files": { "access": "read" }
    },
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
                    "value": "./inspect#notificationPreference"
                }
            ]
        }
    }
}
```

## Sandbox

`Sandbox.merge` joins the abilities a package always uses with those its enabled capabilities unlock, and `defineWorkload` limits a workload to a subset.

```ts
Sandbox.merge(definition.sandbox, { network: { connect: ["gmail.googleapis.com"] } }); // hosts united
export const indexer = defineWorkload({ name: "indexer", sandbox: ["network", "run"], start });
```

## Examples

`defineExample` declares one declaration in a given state, which a host renders with the example's properties or with ones a control changed.

```tsx
export const buttonGhost = defineExample({
    of: Button,
    name: "ghost",
    description: "a ghost button that cancels",
    properties: { variant: "ghost" },
    render: (properties) => <Button {...properties}>Cancel</Button>,
});
```

## Manifests

`BuildReader.open` opens a built package by its manifest digest and reads its files and declarations, verifying every digest, and `BuildWriter` writes one.

```ts
import { BuildReader } from "@destack/package/manifest";

const reader = await BuildReader.open(location, fetch, environment.crypto, signal);
const settings = await reader.declared(
    import.meta.destack.package.id,
    "setting",
    SettingDescription,
);
const module = await reader.module((await reader.graph()).modules["src/server.ts"]);
```

## Stack frames

`StackFrame.read` reads a stack trace's frames, a build's `Symbolicator` maps them through its source maps as an emitter does before export, and `StackFrame.attribute` resolves a frame to its symbol and declaration through the code index.

```ts
import { StackFrame, Symbolicator } from "@destack/package/stack";

const frames = await new Symbolicator(build).frames(StackFrame.read(error.stack ?? ""));
// [{ function: "rename", file: "…/workload.js", path: "src/note.ts", line: 3, isInApp: true }]
const attributed = StackFrame.attribute(frame, {
    code,
    module,
    lines: StackFrame.lineStarts(source),
});
// { …frame, moniker: "…/src/note.ts#Note.rename", declaration: { kind: "method", name: "rename", … } }
```

## Transforms

`@destack/package/bun/preload` installs the module transform in Bun, and `modulePlugin` from `@destack/package/vite` in Vite and Vitest.

```toml
preload = ["@destack/package/bun/preload"]
```
