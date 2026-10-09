# @destack/package

`destack.json` is a package's `package.json` companion with Deno's permissions as `capabilities`, `defineExample` is a Storybook story with its `args` and controls, `defineScenario` is a Storybook play function written as Playwright-style steps, and `graph` holds a build's symbols under SCIP monikers and relationships.

```ts
const capabilities = { network: { connect: ["api.github.com"] }, env: { names: ["GITHUB_TOKEN"] }, run: { commands: ["git"] } }; // deno run --allow-net=api.github.com --allow-env=GITHUB_TOKEN --allow-run=git
defineExample({ of: Button, name: "ghost", description, properties: { variant: "ghost" }, render }); // a Storybook story's args
defineScenario({ of: Calendar, interaction: viewInteraction, name, description, given, when, then }); // a story's play function
graph.Moniker.of({ packageId, module: "src/page.ts", name: "Page" }); // "package-…/src/page.ts#Page", after SCIP's symbols
const { id, name, version } = import.meta.destack.package;
```

## Packages

`definePackage` declares the resources and secrets a stack binds when it installs the package.

```ts
import { definePackage } from "@destack/package/declare";

export default definePackage({ resources: { main: pagesDatabase } });
```

## Definitions

`destack.json` sets a package's id, language, runtimes, capabilities and the functions that describe its declarations.

```json
{
    "id": "package-01a0e95b-c8db-7258-a257-e7661dbc93c3",
    "language": "typescript",
    "runtimes": ["browser", "bun", "workerd"],
    "capabilities": {
        "network": { "connect": ["api.github.com"], "reason": "fetches linked repositories" },
        "fs": { "access": "read", "reason": "indexes the folders you choose", "optional": true }
    },
    "declarations": {
        "defineNotification": {
            "describes": [{ "kind": "notification", "function": "./inspect#describeNotification" }]
        }
    }
}
```

## Capabilities

`Capabilities.grant` gives an installation the required capabilities and the optional ones it allows, and `defineWorkload` limits a workload to a subset.

```ts
Capabilities.grant(workload.capabilities, ["fs"]); // required ones plus fs, never camera
export const indexer = defineWorkload({ name: "indexer", capabilities: ["network", "run"], start });
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

const reader = await BuildReader.open(location, fetch, signal);
const settings = await reader.declared(import.meta.destack.package.id, "setting", SettingDescription);
const module = await reader.module((await reader.graph()).modules["src/server.ts"]);
```

## Transforms

`@destack/package/bun/preload` installs the module transform in Bun, and `modulePlugin` from `@destack/package/vite` in Vite and Vitest.

```toml
preload = ["@destack/package/bun/preload"]
```
