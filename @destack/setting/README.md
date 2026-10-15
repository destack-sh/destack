# @destack/setting

`@destack/setting` declares typed settings, keeps their values as `setting` rows along a scope chain, and resolves the value that applies.

```ts
import { defineSetting } from "@destack/setting/declare";

export const editorMode = defineSetting({
    name: "editor.mode",
    title: "Editor mode",
    description: "Keyboard behavior in the page editor.",
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
    overrides: ["space", "installation", "client"],
    apply: "immediate",
});
```

## Values

A value is a `setting` row at a placement, with the mode `set`, `recommend` or `require`.

```ts
const placement = SettingPlacement.of({ scope: userId, space: spaceId });
await client
    .mutate(setting)
    .create({ ...editorMode.reference, ...placement, mode: "set", value: "vim", release });
```

## Resolving

`resolve` picks the value that applies to a selection from the values along its scope chain and a client's own values.

```ts
const selection = SettingSelection.parse({ scope: userId, space: spaceId });
const resolution = editorMode.resolve(selection, values, chain, clientValues);
resolution.value; // "vim"
const mode = await SettingValue.resolve(database, editorMode, { ...selection, scope: userId });
```

## Replicating

`SettingValue.replicate` replicates a scope's values from the store serving them and yields them after each change.

```ts
for await (const { chain, values } of SettingValue.replicate(
    { endpoint, scope: userId, caller, database, settings: [editorMode.reference], clock },
    signal,
)) {
    editorMode.resolve(selection, values, chain);
}
```

## Server

`settingServer` serves the setting package in a store, checking each written value against its declaration.

```ts
import { settingServer } from "@destack/setting/server";
import { settingTables } from "@destack/setting/stack";

const server = settingServer({ declared: SettingCatalog.cached(open), after: [] });
const database = defineDatabase({ name: "main", tables: settingTables });
```
