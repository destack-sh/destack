# Type-Only Imports

## Type Definitions

### Goto type definition with type-only import

Goto type definition should resolve type-only imports.

```ds:types.ds
export type Options = {
//            ^^^^^^^ def:Options
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";
//              ^^^^^^^ decl:Options_import

function configure(options: Options): void {
//                            ^^^^^^^ use:Options
    console.log(options.name);
}
```

```query goto_type_definition use:Options
def:Options
```

### Goto definition with type-only import

Goto definition should resolve type-only imports to the source type declaration.

```ds:types.ds
export type Options = {
//            ^^^^^^^ def:Options
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";

function configure(options: Options): void {
//                            ^^^^^^^ use:Options
    console.log(options.name);
}
```

```query goto_definition use:Options
def:Options
```

### Goto declaration with type-only import

Goto declaration should stop at the local type-only import specifier.

```ds:types.ds
export type Options = {
    name: string,
};
```

```ds:main.ds
import type { Options } from "./types.ds";
//              ^^^^^^^ decl:Options_import

function configure(options: Options): void {
//                            ^^^^^^^ use:Options
    console.log(options.name);
}
```

```query goto_declaration use:Options
decl:Options_import
```

## Re-Exported Type-Only Imports

### Type-only imports through re-export chains should still resolve correctly

Type-only imports re-exported through barrels should preserve the same definition and declaration behavior.

```ds:types.ds
export type Config = {
//            ^^^^^^ def:Config
    enabled: boolean,
};
```

```ds:barrel.ds
export type { Config as AppConfig } from "./types.ds";
```

```ds:main.ds
import type { AppConfig } from "./barrel.ds";
//              ^^^^^^^^^ decl:AppConfig_import

const config: AppConfig = { enabled: true };
//              ^^^^^^^^^ use:AppConfig
```

```query goto_type_definition use:AppConfig
def:Config
```

```query goto_definition use:AppConfig
def:Config
```

```query goto_declaration use:AppConfig
decl:AppConfig_import
```

### Type-only imports should stay in type space when value names shadow them

Type-only navigation should still resolve the type import when a value binding with the same name exists locally.

```ds:types.ds
export type Settings = {
//            ^^^^^^^^ def:Settings
    mode: string,
};
```

```ds:main.ds
import type { Settings } from "./types.ds";
//              ^^^^^^^^ decl:Settings_import

const Settings = 1;

function configure(options: Settings): void {
//                            ^^^^^^^^ use:Settings
    console.log(options.mode);
}
```

```query goto_type_definition use:Settings
def:Settings
```

```query goto_declaration use:Settings
decl:Settings_import
```

```query goto_definition use:Settings
def:Settings
```

## Aliases And Mixed Imports

### Aliased type-only imports should preserve declaration and definition targeting

Aliased type-only imports should stop at the local alias declaration while still resolving the source type definition.

```ds:types.ds
export type Options = {
//            ^^^^^^^ def:AliasedOptions
    name: string,
};
```

```ds:main.ds
import type { Options as AppOptions } from "./types.ds";
//                         ^^^^^^^^^^ decl:AppOptions_import

function configure(options: AppOptions): void {
//                            ^^^^^^^^^^ use:AppOptions
    console.log(options.name);
}
```

```query goto_declaration use:AppOptions
decl:AppOptions_import
```

```query goto_definition use:AppOptions
def:AliasedOptions
```

```query goto_type_definition use:AppOptions
def:AliasedOptions
```

### Type-only aliases should keep declaration and definition targeting beside value imports from the same module

Type-only and value imports from the same module should each keep their own declaration and definition behavior when the type import uses a local alias.

```ds:module.ds
export type Settings = {
//            ^^^^^^^^ def:MixedSettings
    mode: string,
};

export function createSettings(): Settings {
//                ^^^^^^^^^^^^^^ def:createSettings
    return { mode: "prod" };
}
```

```ds:main.ds
import type { Settings as ImportedSettings } from "./module.ds";
//                          ^^^^^^^^^^^^^^^^ decl:MixedSettings_import
import { createSettings } from "./module.ds";
//         ^^^^^^^^^^^^^^ decl:createSettings_import

function configure(settings: ImportedSettings): void {
//                             ^^^^^^^^^^^^^^^^ use:MixedSettings
    const created = createSettings();
//                    ^^^^^^^^^^^^^^ use:createSettings
    console.log(settings.mode, created.mode);
}
```

```query goto_declaration use:MixedSettings
decl:MixedSettings_import
```

```query goto_definition use:MixedSettings
def:MixedSettings
```

```query goto_type_definition use:MixedSettings
def:MixedSettings
```

```query goto_declaration use:createSettings
decl:createSettings_import
```

```query goto_definition use:createSettings
def:createSettings
```

### Type-only imports should keep declaration and definition targeting beside value imports from the same module

Type-only and value imports from the same module should each keep their own declaration and definition behavior without requiring a local alias.

```ds:module_plain.ds
export type Settings = {
//            ^^^^^^^^ def:MixedSettings_plain
    mode: string,
};

export function createSettings(): Settings {
//                ^^^^^^^^^^^^^^ def:createSettings_plain
    return { mode: "prod" };
}
```

```ds:main_plain.ds
import type { Settings } from "./module_plain.ds";
//              ^^^^^^^^ decl:MixedSettings_plain_import
import { createSettings } from "./module_plain.ds";
//         ^^^^^^^^^^^^^^ decl:createSettings_plain_import

const settings: Settings = createSettings();
//                ^^^^^^^^ use:MixedSettings_plain
//                           ^^^^^^^^^^^^^^ use:createSettings_plain
```

```query goto_declaration use:MixedSettings_plain
decl:MixedSettings_plain_import
```

```query goto_definition use:MixedSettings_plain
def:MixedSettings_plain
```

```query goto_type_definition use:MixedSettings_plain
def:MixedSettings_plain
```

```query goto_declaration use:createSettings_plain
decl:createSettings_plain_import
```

```query goto_definition use:createSettings_plain
def:createSettings_plain
```
