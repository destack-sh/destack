# @destack/setting

`defineSetting` is a VS Code configuration contribution (`title`, `description`, a schema, `default`, a scope and the scopes that override it), stored as `setting` objects along a space's scope chain, and `mode` adds Chrome enterprise policy's recommended and mandatory values as `recommend` and `require`.

```ts
defineSetting({ name: "editor.mode", title, description, schema, default: "standard", scope: "user", overrides: ["space", "client"], apply: "immediate" }); // contributes.configuration
editor: { setting: editorMode, value: "vim", mode: "recommend" }; // a Chrome recommended policy, "require" a mandatory one
editorMode.resolve(selection, rows, chain, clientValues); // client, then user, account and default, as VS Code's workspace over user
merge: "key"; // each record key from its own nearest placement
```

## Declarations

`defineSetting` declares a typed setting with its scope and the overrides it permits, and `convert` upgrades its earlier values by release.

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

## Reading

`resolve` reads a setting's value from the `setting` rows a client follows along the space's scope chain.

```ts
const selection = SettingSelection.parse({ scope: userId, space: spaceId });
const rows = await personal
    .of({ setting })
    .query.setting.findMany({ where: editorMode.condition(selection) });
const mode = editorMode.resolve(selection, rows, await space.replica.chain(space.database));
SettingResolution.observed(mode, "client"); // { id, revision } of the client's value, or null
```

## Writing

The `setting` object's methods create, update and delete the row at a placement, and a `clientSetting` local object holds one client's value.

```ts
const placement = SettingPlacement.of({ ...selection, scope: userId });
await personal
    .mutate(setting)
    .create({ ...editorMode.reference, ...placement, mode: "set", value: "vim", release })
    .confirmed;
await client.mutate(clientSetting).create({ ...editorMode.reference, value: "vim", release })
    .confirmed;
```

## Service

`serveSettings` serves the `setting` objects, checking each written value against the catalog of its release, and `settingTables` lists their tables.

```ts
import { serveSettings } from "@destack/setting/server";
import { settingTables } from "@destack/setting/stack";

const { setting } = serveSettings(SettingCatalog.cached((written) => open(written)));
export const spaceDatabase = defineDatabase({
    name: "main",
    tables: [...settingTables, ...others],
});
```
