# @destack/setting

Declare, place and resolve scoped settings.

## Overview

`@destack/setting` is VS Code's configuration contributions with Chrome's recommended and required policies: declared settings, scoped values, and resolutions with their sources.

Packages declare settings:

- `Setting`: a typed setting with a default and a scope, declared with `defineSetting`.
- `SettingScope`: where values live: a user, a space or a device.

People and managers place values:

- `setting`: a value placed in a scope.
- `SettingMode`: `"set"`, `"recommend"` or `"require"`.
- `deviceSetting`: a value one device stores.

Resolutions:

- `SettingResolution`: the effective value with its sources.
- `SettingError`: the reasons a value or resolution is refused, checked with `SettingError.is`.

Declare a setting, then read the value an installation runs with:

```ts
import { t } from "@destack/locale";
import { defineMount, definePackage } from "@destack/object";
import { implementPackage } from "@destack/object/server";
import { schema } from "@destack/schema";
import { defineSetting } from "@destack/setting/declare";

// declare how many notes a page lists, per space and installation
export const pageSize = defineSetting({
    name: "pageSize",
    title: t`Page size`,
    description: t`How many notes a page lists.`,
    schema: schema.number().int().min(10).max(100),
    default: 25,
    scope: "space",
    overrides: ["installation"],
});
const notes = defineMount({ name: "notes", path: "/notes" });
const notesPackage = definePackage({ mounts: [notes] });

export default implementPackage(notesPackage, (context) => {
    // read the resolved value
    const size = pageSize.read(context.settings);

    return { mounts: { notes: async () => Response.json({ size }) } };
});
```

## Reference

### defineSetting

`defineSetting(options)` declares a setting, and refuses a default its schema rejects.
A setting has a `default` or is `required`, so each scope must place a value.

| Option                 | What it declares                                                              |
| ---------------------- | ----------------------------------------------------------------------------- |
| `name`                 | the name within the package                                                   |
| `title`, `description` | the label and the behavior settings views show, as messages                   |
| `group`                | the group settings views show it in                                           |
| `schema`               | the value's schema                                                            |
| `default`, `required`  | the value where no scope placed one, or `true` to require a placed value      |
| `scope`, `overrides`   | where values live, and the overrides they may narrow by                       |
| `apply`                | `"immediate"`, the default, or `"restart"` for a consumer to apply a change   |
| `merge`                | `"value"`, the default, or `"key"` to resolve each key of a record on its own |
| `deprecated`           | the migration guidance of a deprecated setting                                |
| `convert`              | the conversions of earlier values, by the release introducing each            |

A `Setting` answers its value and its resolutions:

| Verb                                        | What it does                                                     |
| ------------------------------------------- | ---------------------------------------------------------------- |
| `read(settings)`                            | reads the value among an installation's resolved settings        |
| `resolve(selection, values, chain, device)` | resolves the value for a selection from placed and device values |
| `isUnset(selection, values, chain)`         | reports whether a required setting has no value for a selection  |
| `reference`                                 | the setting's `{ packageId, name }`, which its values refer to   |

### Scopes and overrides

Overrides narrow a value to part of the setting's scope.

| Scope      | Overrides it permits                                 |
| ---------- | ---------------------------------------------------- |
| `"user"`   | `"package"`, `"space"`, `"installation"`, `"device"` |
| `"space"`  | `"installation"`                                     |
| `"device"` | none, as each device stores its own value            |

### SettingResolution

A resolution ranks the default and every applying value, lowest precedence first.
Agreeing requirements win, and conflicting ones refuse the resolution.

| Precedence | Value                                                                                                                     |
| ---------- | ------------------------------------------------------------------------------------------------------------------------- |
| lowest     | the declaration's `default`                                                                                               |
|            | recommendations of enclosing scopes, a nearer scope above a farther one, and one for the installation above a broader one |
|            | values set in the selection's scope, an installation's above its space's above its package's above an unnarrowed one      |
| highest    | the device's own values                                                                                                   |

| Field         | What it is                                                                                                                            |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `value`       | the effective value                                                                                                                   |
| `sources`     | the winning values or agreeing requirements, then the invalid values skipped, each a `default`, `value`, `device` or `invalid` source |
| `overridden`  | the other values that apply, lowest precedence first                                                                                  |
| `enforcement` | `"required"` when a requirement fixes the value, else `"ordinary"`                                                                    |
| `keys`        | how each key resolved, for a setting that merges keys                                                                                 |

## Examples

### Require a value for a whole account

A requirement in an account applies to all its spaces.

```ts
import type { ClientOf } from "@destack/object/client";
import type settingPackage from "@destack/setting/package";
import type { Setting } from "@destack/setting";

/** Require every space in an account to list fifty notes a page. */
export async function requirePageSize(
    client: ClientOf<typeof settingPackage>,
    pageSize: Setting,
    accountId: string,
    requestId: string,
) {
    // place a requirement in the account, against the declaring release
    const { packageId, name } = pageSize.reference;
    await client.setting.create({
        scope: accountId,
        requestId,
        packageId,
        name,
        mode: "require",
        value: 50,
        release: pageSize.package.version,
    });
}
```

### Show where a value comes from

`resolve` answers the value with its sources.

```ts
import type { ClientOf } from "@destack/object/client";
import type { Identifier } from "@destack/schema";
import type settingPackage from "@destack/setting/package";
import type { Setting } from "@destack/setting";

/** Read a space's page size, and whether a requirement fixes it. */
export async function readPageSize(
    client: ClientOf<typeof settingPackage>,
    pageSize: Setting,
    space: Identifier<"space">,
) {
    // resolve the value for the space, without device values
    const { packageId, name } = pageSize.reference;
    const resolution = await client.setting.resolve({
        scope: space,
        packageId,
        name,
        selection: { scope: space },
        device: [],
    });

    return { value: resolution.value, isRequired: resolution.enforcement === "required" };
}
```

### Change a setting from the terminal

`destack setting` gets, sets or resets a value at the scope `--scope` selects: `account`, `space` or `device`.

```sh
destack setting pageSize set 50 --scope space
```

## Comparison

### VS Code settings

Declared settings with a schema, a default and a scope follow VS Code's configuration contributions.
Managers recommend or require values from enclosing scopes, as Chrome's policies do.

| VS Code                                 | `@destack/setting`         | Difference                                                               |
| --------------------------------------- | -------------------------- | ------------------------------------------------------------------------ |
| `contributes.configuration`             | `defineSetting`            | Declared in code, with a schema and `convert` for earlier values.        |
| `scope: "application"`, `"resource"`, … | `scope`, `overrides`       | A user, a space or a device, narrowed by package, space or installation. |
| user and workspace `settings.json`      | `setting` objects          | Rows in scopes, which enclosing scopes' values reach.                    |
| policies                                | `"recommend"`, `"require"` | Placed by an account or organization, with `require` above every value.  |
| `inspect(key)`                          | `resolve`                  | Answers the winning sources and the overridden ones.                     |
| `restartRequired`                       | `apply: "restart"`         | The same.                                                                |
