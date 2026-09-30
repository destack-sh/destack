Declare, place and resolve scoped settings.

## Usage

A package declares a typed setting with its scope and the overrides it permits.

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

A client subscribes to the `setting` rows of the user's scope and the space the user acts in, and resolves each setting from them along the space's scope chain.

```ts
import { Condition } from "@destack/db/query";
import { SettingSelection } from "@destack/setting";
import { setting, type SettingRow } from "@destack/setting/object";

const selection = SettingSelection.parse({ scope: userId, space: spaceId });
const where = Condition.any(editorMode.condition(selection), lineNumbers.condition(selection));
const queries = [personal, space].map((client) => client.subscribe(setting, { where }));
const rows = (await Promise.all(queries.map((query) => query.read()))).flat() as SettingRow[];
const chain = await space.replica.chain(space.database);
const mode = editorMode.resolve(selection, rows, chain);
```

An editor finds the row at the placement it edits, and creates, updates or deletes it through the `setting` object's methods.

```ts
import { SettingPlacement, SettingResolution } from "@destack/setting";

const placement = SettingPlacement.of({ ...selection, scope: userId });
const observed = SettingResolution.observed(mode, placement);
const settings = personal.mutate(setting);
const release = editorMode.package.version; // the release the value is written against
const saved =
    observed === null
        ? settings.create({ ...editorMode.reference, ...placement, mode: "set", value: "vim", release })
        : settings.update({ ...observed, value: "vim", release });
await saved.confirmed;
```

The `setting` object has these methods.

| Method | Permission | Effect |
|---|---|---|
| `setting.get`, `setting.list` | `read` | read values placed in a scope |
| `setting.create` | `write` | places a value checked against its declaration |
| `setting.update` | `write` | changes a value's `mode`, `value` and `release` |
| `setting.delete` | `write` | removes a value and exposes the values it overrode |

A stack sets values in its space, or recommends or requires them for the space's users.

```ts
import { defineSpace } from "@destack/space";

export const personal = defineSpace({
    settings: {
        editor: { setting: editorMode, value: "vim", mode: "recommend" },
    },
});
```

## Resolution

A resolution takes the value of the highest applicable source, listed lowest first, and requirements must agree.

| Source | Placed in | Applies when | Precedence within |
|---|---|---|---|
| declaration default | the declaring release | always | |
| `recommend` | an enclosing scope | it carries no installation or the selected one | the nearer scope, then one for the selected installation |
| `set` | the selected scope | every override it carries matches | device, then installation, then space, then package |
| `require` | an enclosing scope | as `recommend` | none: equal values only |
| `invalid` | any scope | never: the declaration no longer accepts the stored value | |

## Releases

Values record the release they were written against, and a setting that narrows its schema converts earlier values by the release introducing the change.

```ts
export const keymap = defineSetting({
    ...editorMode.definition,
    schema: schema.object({ keymap: schema.enum(["standard", "vim"]) }),
    default: { keymap: "standard" },
    convert: { "2026.10.0": Expression.object({ keymap: Expression.column("value") }) }, // `value` is the stored value
});
```

## Hosting

A host serves setting values as objects and reconciles the values stacks place in their spaces.

```ts
import { settingReconcilers } from "@destack/setting/reconcile";
import { implementService } from "@destack/setting/server";

const service = implementService({ database, release, audit });
const reconcilers = settingReconcilers(release);
```
