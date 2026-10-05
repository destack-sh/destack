# @destack/setting

Declare, place and resolve scoped settings.

## Declarations

`defineSetting` declares a typed setting with its scope and the overrides it permits.

```ts
import { defineSetting } from "@destack/setting/declare";
import { schema } from "@destack/schema";

export const editorMode = defineSetting({
    name: "editor.mode",
    title: "Editor mode",
    description: "Keyboard behavior in the note editor.",
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
    overrides: ["space", "installation", "device"],
    apply: "immediate",
});
```

### Releases

`convert` upgrades a setting's earlier values by the release narrowing its schema.

```ts
import { Expression } from "@destack/db";

export const keymap = defineSetting({
    ...editorMode.definition,
    schema: schema.object({ keymap: schema.enum(["standard", "vim"]) }),
    default: { keymap: "standard" },
    convert: { "2026.10.0": Expression.object({ keymap: Expression.column("value") }) }, // `value` is the stored value
});
```

### Stacks

`mode` sets a stack's value in its space, or recommends or requires it for the space's users.

```ts
import { defineSpace } from "@destack/space";

export const personal = defineSpace({
    settings: {
        editor: { setting: editorMode, value: "vim", mode: "recommend" },
    },
});
```

## Objects

A `setting` object holds one value of a setting at a placement, and readers resolve the effective value from the rows along a scope chain.

### Placements

A placement holds the override columns of a `setting` row, its device as `deviceId`, and spreads into a created value.

```ts
const placement = SettingPlacement.of({
    scope: userId,
    space: spaceId,
    installation: null,
    deviceId,
});
// { scope: userId, space: spaceId, deviceId }
```

### Reading

`resolve` reads a setting's value from the `setting` rows a client follows along the space's scope chain.

```ts
import { SettingSelection } from "@destack/setting";
import { setting } from "@destack/setting/object";

const selection = SettingSelection.parse({ scope: userId, space: spaceId });
const where = { OR: [editorMode.condition(selection), lineNumbers.condition(selection)] };
const reads = [personal, space].map((client) =>
    client.of({ setting }).query.setting.findMany({ where }),
);
const rows = (await Promise.all(reads)).flat();
const chain = await space.replica.chain(space.database);
const mode = editorMode.resolve(selection, rows, chain);
```

### Editing

The `setting` object's methods create, update and delete the row at a placement.

```ts
import { SettingPlacement, SettingResolution } from "@destack/setting";

const placement = SettingPlacement.of({ ...selection, scope: userId });
const observed = SettingResolution.observed(mode, placement);
const settings = personal.mutate(setting);
const release = editorMode.package.version; // the release the value is written against
const saved =
    observed === null
        ? settings.create({
              ...editorMode.reference,
              ...placement,
              mode: "set",
              value: "vim",
              release,
          })
        : settings.update({ ...observed, value: "vim", release });
await saved.confirmed;
```

## Service

`servedObjects` serves the `setting` objects, which apply the values stacks place and check them against the declaring release.

```ts
import { servedObjects } from "@destack/setting/server";

const { setting } = servedObjects(release); // release opens the package release a value names
const server = new ObjectServer({ objects: { setting, ...others }, database, callKey, origin });
```

## Tables

`settingTables` adds the setting values and their journal to the database of the service serving them.

```ts
import { settingTables } from "@destack/setting/stack";

export const spaceDatabase = defineDatabase({
    name: "main",
    tables: [...settingTables, ...others],
});
```

## Errors

A setting failure throws a `SettingError`, and `toServiceError` names the service error its caller receives.

```ts
import { SettingError } from "@destack/setting/error";

new SettingError("UNDECLARED", "no setting editor.mode").toServiceError(); // { code: "NOT_FOUND", message: "no setting editor.mode" }
```
