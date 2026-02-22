# Goto Declaration

## Locals

### Local variable declaration

Goto declaration should return the local declaration.

```ds
const value = 1;
//    ^^^^^ decl:value

const other = value;
//            ^^^^ use:value
```

```query goto_declaration use:value
decl:value
```

## Imports

### Imported symbol declaration

Goto declaration should stop at the import declaration.

```ds:lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:main.ds
import { greet } from "./lib.ds";
//       ^^^^^ decl:import_greet

const message = greet("World");
//              ^^^^^ use:greet
```

```query goto_declaration use:greet
decl:import_greet
```

### Aliased import declaration

Goto declaration should stop at the local import alias when a symbol is imported with `as`.

```ds:alias_lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_main.ds
import { greet as localGreet } from "./alias_lib.ds";
//                ^^^^^^^^^^ decl:import_local_greet

const message = localGreet("World");
//              ^^^^^^^^^^ use:local_greet
```

```query goto_declaration use:local_greet
decl:import_local_greet
```

### Re-exported alias declaration

Goto declaration should stop at the local import alias even when the symbol is re-exported.

```ds:alias_base.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:alias_barrel.ds
export { greet as greetAlias } from "./alias_base.ds";
```

```ds:alias_main.ds
import { greetAlias } from "./alias_barrel.ds";
//       ^^^^^^^^^^ decl:import_greet_alias

const message = greetAlias("World");
//              ^^^^^^^^^^ use:greet_alias
```

```query goto_declaration use:greet_alias
decl:import_greet_alias
```

### Type-only import declaration

Goto declaration should stop at the type-only import specifier.

```ds:types.ds
export struct Thing {
    value: int32,
}
```

```ds:main.ds
import type { Thing } from "./types.ds";
//            ^^^^^ decl:import_thing

const item: Thing = Thing { value: 1 };
//          ^^^^^ use:thing
```

```query goto_declaration use:thing
decl:import_thing
```

### Namespace import declaration

Goto declaration should stop at the namespace import alias.

```ds:utils.ds
export function ping(): void {}
```

```ds:main.ds
import * as utils from "./utils.ds";
//          ^^^^^ decl:import_utils

utils.ping();
// ^^^^^ use:utils
```

```query goto_declaration use:utils
decl:import_utils
```

### Re-exported import declaration

Goto declaration should stop at the importing site even when the symbol is re-exported.

```ds:lib.ds
export function greet(name: string): string {
    return "Hello, " + name;
}
```

```ds:bar.ds
export { greet } from "./lib.ds";
```

```ds:main.ds
import { greet } from "./bar.ds";
//       ^^^^^ decl:import_greet

const message = greet("World");
//              ^^^^^ use:greet
```

```query goto_declaration use:greet
decl:import_greet
```

## Type And Value Imports

### Resolve declarations for mixed import shapes

Goto declaration should resolve type-only and value imports to their own local import specifiers.

```ds:types.ds
export type Settings = {
    enabled: boolean,
};
```

```ds:values.ds
export function settings(): int32 {
    return 1;
}
```

```ds:main.ds
import type { Settings } from "./types.ds";
//            ^^^^^^^^ decl:import_settings_type
import { settings } from "./values.ds";
//       ^^^^^^^^ decl:import_settings_value

const typed: Settings = { enabled: true };
//           ^^^^^^^^ use:settings_type_use
const value = settings();
//            ^^^^^^^^ use:settings_value_use
```

```query goto_declaration use:settings_type_use
decl:import_settings_type
```

```query goto_declaration use:settings_value_use
decl:import_settings_value
```

## Cross Module Import Shapes

### Type-only import alias declaration

Goto declaration should resolve aliased type-only imports to the local alias declaration.

```ds:types_alias.ds
export type Settings = {
    enabled: boolean,
};
```

```ds:main_alias.ds
import type { Settings as AppSettings } from "./types_alias.ds";
//                        ^^^^^^^^^^^ decl:import_app_settings

const typed: AppSettings = { enabled: true };
//           ^^^^^^^^^^^ use:app_settings_type
```

```query goto_declaration use:app_settings_type
decl:import_app_settings
```

### Re-exported type-only alias declaration

Goto declaration should still resolve to the local import declaration through re-exported type aliases.

```ds:base_type.ds
export interface ServiceConfig {
    enabled: boolean,
}
```

```ds:barrel_type.ds
export type { ServiceConfig as Config } from "./base_type.ds";
```

```ds:main_reexport_type.ds
import type { Config } from "./barrel_type.ds";
//            ^^^^^^ decl:import_config

const typed: Config = { enabled: true };
//           ^^^^^^ use:config_type
```

```query goto_declaration use:config_type
decl:import_config
```

### Default import declaration

Goto declaration should resolve default import usages to the local default import binding.

```ds:default_lib.ds
export default function createValue(): int32 {
    return 1;
}
```

```ds:default_main.ds
import buildValue from "./default_lib.ds";
//     ^^^^^^^^^^ decl:import_build_value

const value = buildValue();
//            ^^^^^^^^^^ use:build_value
```

```query goto_declaration use:build_value
decl:import_build_value
```

## Damaged Syntax

### Return no declaration for malformed unresolved member access

Goto declaration should return no result when the cursor is on malformed unresolved syntax.

```ds
function main(): void {
    missingValue.
//  ^^^^^^^^^^^ broken
}
```

```query goto_declaration broken
<none>
```
