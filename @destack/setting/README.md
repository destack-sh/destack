# @destack/setting

Declare, place and resolve scoped settings.

## Overview

A package declares its settings with a schema and a default, values are placed along a scope chain, and a resolution picks the value that applies.

- **Setting**: a declared, typed setting with its default, the scope it belongs to and the overrides that narrow it; `resolve` picks the value for a selection.
- **Value**: a `setting` row at a placement with the mode `set`, `recommend` or `require`; `resolve` on the object type answers the value with its sources.
- **Placement**: a value's scope and the package, space or installation it applies to.

```ts
import type settingPackage from "@destack/setting/package";
import { defineSetting } from "@destack/setting/declare";
import { t } from "@destack/locale";
import type { ClientOf } from "@destack/object/client";
import { schema } from "@destack/schema";

export const editorMode = defineSetting({
    name: "editor.mode",
    title: t`Editor mode`,
    description: t`Keyboard behavior in the page editor.`,
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
    overrides: ["space", "installation", "device"],
    apply: "immediate",
});

async (client: ClientOf<typeof settingPackage>, userId: string, requestId: string) => {
    const { packageId, name } = editorMode.reference;
    await client.setting.create({
        scope: userId,
        requestId,
        packageId,
        name,
        mode: "set",
        value: "vim",
        release: "2026.11.0",
    });
};
```

## Examples

### Change a setting from the terminal

`destack setting` sets a value at a scope, such as this computer's.

```sh
destack setting editor.mode set vim --scope device
```

### Resolve a value

`resolve` ranks the default and every value placed along the selection's scope chain.

```ts
import { defineSetting } from "@destack/setting/declare";
import type { SettingValue } from "@destack/setting/object";
import { t } from "@destack/locale";
import { schema } from "@destack/schema";

const editorMode = defineSetting({
    name: "editor.mode",
    title: t`Editor mode`,
    description: t`Keyboard behavior in the page editor.`,
    schema: schema.enum(["standard", "vim"]),
    default: "standard",
    scope: "user",
});

(values: readonly SettingValue[], chain: readonly string[]) => {
    const resolution = editorMode.resolve({ scope: "user-1" }, values, chain);
    resolution.value; // "vim"
};
```

## Comparison

### VS Code settings

Declared settings with defaults, scopes and overrides follow VS Code's configuration contributions; three things differ.

- Values are rows along a scope chain of user, space, account and organization, and a manager can `recommend` or `require` one.
- A resolution reports the source of its value, as `git config --show-origin` does.
- A device keeps separate values, so one computer can differ from the person's others.
