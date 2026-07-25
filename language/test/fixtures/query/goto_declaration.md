# Goto Declaration

## Locals

### Resolve a local variable declaration

A local reference resolves to its declaration.

```ds main.ds
const value = 1;
      ^^^^^ declaration:value

const other = value;
              ^^^^^ reference:value
```

```query goto_declaration main.ds#reference:value
@goto_declaration.target origin=main.ds#reference:value location=main.ds#declaration:value symbol=main.ds#value@1
```

## Imports

### Resolve an imported symbol declaration

An imported reference resolves to its local import.

```ds library.ds
export function greet(name: string): string {
    return name;
}
```

```ds main.ds
import { greet } from "./library.ds";
         ^^^^^ declaration:import_greet

const message = greet("World");
                ^^^^^ reference:greet
```

```query goto_declaration main.ds#reference:greet
@goto_declaration.target origin=main.ds#reference:greet location=main.ds#declaration:import_greet symbol=main.ds#greet@1
```

### Resolve an aliased import declaration

An aliased import resolves to its local alias.

```ds alias_library.ds
export function greet(name: string): string {
    return name;
}
```

```ds alias_main.ds
import { greet as localGreet } from "./alias_library.ds";
                  ^^^^^^^^^^ declaration:import_local_greet

const message = localGreet("World");
                ^^^^^^^^^^ reference:local_greet
```

```query goto_declaration alias_main.ds#reference:local_greet
@goto_declaration.target origin=alias_main.ds#reference:local_greet location=alias_main.ds:1:10-1:29 selection=alias_main.ds#declaration:import_local_greet symbol=alias_main.ds#localGreet@1
```

### Keep the imported and local sides of an alias distinct

The imported name resolves to the exported declaration while the local name resolves to its alias.

```ds library.ds
export function greet(): void {}
                ^^^^^ declaration:exported_greet
```

```ds main.ds
import { greet as welcome } from "./library.ds";
         ^^^^^ imported_name
                  ^^^^^^^ declaration:local_welcome

welcome();
^^^^^^^ reference:local_welcome
```

```query goto_declaration main.ds#imported_name
@goto_declaration.target origin=main.ds#imported_name location=library.ds:1:1-1:33 selection=library.ds#declaration:exported_greet symbol=library.ds#greet@1
```

```query goto_declaration main.ds#reference:local_welcome
@goto_declaration.target origin=main.ds#reference:local_welcome location=main.ds:1:10-1:26 selection=main.ds#declaration:local_welcome symbol=main.ds#welcome@1
```

### Resolve a re-exported alias declaration

A re-exported alias still resolves to its local import.

```ds alias_base.ds
export function greet(name: string): string {
    return name;
}
```

```ds alias_barrel.ds
export { greet as greetAlias } from "./alias_base.ds";
```

```ds alias_main.ds
import { greetAlias } from "./alias_barrel.ds";
         ^^^^^^^^^^ declaration:import_greet_alias

const message = greetAlias("World");
                ^^^^^^^^^^ reference:greet_alias
```

```query goto_declaration alias_main.ds#reference:greet_alias
@goto_declaration.target origin=alias_main.ds#reference:greet_alias location=alias_main.ds#declaration:import_greet_alias symbol=alias_main.ds#greetAlias@1
```

### Resolve an imported type declaration

An imported type resolves to its local import.

```ds types.ds
export struct Thing {
    value: int32;
}
```

```ds main.ds
import { Thing } from "./types.ds";
         ^^^^^ declaration:import_thing

const item: Thing = Thing { value: 1 };
            ^^^^^ reference:thing
```

```query goto_declaration main.ds#reference:thing
@goto_declaration.target origin=main.ds#reference:thing location=main.ds#declaration:import_thing symbol=main.ds#Thing@1
```

### Resolve a namespace import declaration

A namespace reference resolves to its local alias.

```ds utilities.ds
export function ping(): void {}
```

```ds main.ds
import * as utilities from "./utilities.ds";
            ^^^^^^^^^ declaration:import_utilities

utilities.ping();
^^^^^^^^^ reference:utilities
```

```query goto_declaration main.ds#reference:utilities
@goto_declaration.target origin=main.ds#reference:utilities location=main.ds:1:8-1:22 selection=main.ds#declaration:import_utilities symbol=main.ds#utilities@1
```

### Resolve each segment of a namespace type path

The namespace root resolves to its local import, while the selected type resolves to its declaration.

```ds model.ds
export struct Settings {
              ^^^^^^^^ declaration:settings
    enabled: boolean;
}
```

```ds main.ds
import * as models from "./model.ds";
            ^^^^^^ declaration:models

type Selected = models.Settings;
                ^^^^^^ reference:models
                       ^^^^^^^^ reference:settings
```

```query goto_declaration main.ds#reference:models
@goto_declaration.target origin=main.ds#reference:models location=main.ds:1:8-1:19 selection=main.ds#declaration:models symbol=main.ds#models@1
```

```query goto_declaration main.ds#reference:settings
@goto_declaration.target origin=main.ds#reference:settings location=model.ds:1:1-3:2 selection=model.ds#declaration:settings symbol=model.ds#Settings@1
```

### Resolve a re-exported import declaration

A re-exported symbol resolves to its downstream import.

```ds library.ds
export function greet(name: string): string {
    return name;
}
```

```ds barrel.ds
export { greet } from "./library.ds";
```

```ds main.ds
import { greet } from "./barrel.ds";
         ^^^^^ declaration:import_greet

const message = greet("World");
                ^^^^^ reference:greet
```

```query goto_declaration main.ds#reference:greet
@goto_declaration.target origin=main.ds#reference:greet location=main.ds#declaration:import_greet symbol=main.ds#greet@1
```

### Resolve declarations for type and value imports

Type and value references resolve to their respective imports.

```ds types.ds
export type Settings = {
    enabled: boolean,
};
```

```ds values.ds
export function settings(): int32 {
    return 1;
}
```

```ds main.ds
import { Settings } from "./types.ds";
         ^^^^^^^^ declaration:import_settings_type
import { settings } from "./values.ds";
         ^^^^^^^^ declaration:import_settings_value

const typed: Settings = { enabled: true };
             ^^^^^^^^ reference:settings_type
const value = settings();
              ^^^^^^^^ reference:settings_value
```

```query goto_declaration main.ds#reference:settings_type
@goto_declaration.target origin=main.ds#reference:settings_type location=main.ds#declaration:import_settings_type symbol=main.ds#Settings@1
```

```query goto_declaration main.ds#reference:settings_value
@goto_declaration.target origin=main.ds#reference:settings_value location=main.ds#declaration:import_settings_value symbol=main.ds#settings@2
```

### Resolve an imported type alias declaration

An aliased type resolves to its local alias.

```ds types_alias.ds
export type Settings = {
    enabled: boolean,
};
```

```ds main_alias.ds
import { Settings as ApplicationSettings } from "./types_alias.ds";
                     ^^^^^^^^^^^^^^^^^^^ declaration:import_application_settings

const typed: ApplicationSettings = { enabled: true };
             ^^^^^^^^^^^^^^^^^^^ reference:application_settings
```

```query goto_declaration main_alias.ds#reference:application_settings
@goto_declaration.target origin=main_alias.ds#reference:application_settings location=main_alias.ds:1:10-1:41 selection=main_alias.ds#declaration:import_application_settings symbol=main_alias.ds#ApplicationSettings@1
```

### Resolve a re-exported type alias declaration

A re-exported type alias resolves to its downstream import.

```ds base_type.ds
export interface ServiceConfiguration {
    enabled: boolean;
}
```

```ds barrel_type.ds
export { ServiceConfiguration as Configuration } from "./base_type.ds";
```

```ds main_reexport_type.ds
import { Configuration } from "./barrel_type.ds";
         ^^^^^^^^^^^^^ declaration:import_configuration

const typed: Configuration = { enabled: true };
             ^^^^^^^^^^^^^ reference:configuration
```

```query goto_declaration main_reexport_type.ds#reference:configuration
@goto_declaration.target origin=main_reexport_type.ds#reference:configuration location=main_reexport_type.ds#declaration:import_configuration symbol=main_reexport_type.ds#Configuration@1
```

### Resolve a default import declaration

A default import resolves to its local binding.

```ds default_library.ds
export default function createValue(): int32 {
    return 1;
}
```

```ds default_main.ds
import buildValue from "./default_library.ds";
       ^^^^^^^^^^ declaration:import_build_value

const value = buildValue();
              ^^^^^^^^^^ reference:build_value
```

```query goto_declaration default_main.ds#reference:build_value
@goto_declaration.target origin=default_main.ds#reference:build_value location=default_main.ds#declaration:import_build_value symbol=default_main.ds#buildValue@1
```

### Resolve declarations through default re-export alias chains

A default re-export alias resolves to its downstream import.

```ds library.ds
export default function buildWidget(): int32 {
    return 1;
}
```

```ds barrel.ds
export { default as buildWidget } from "./library.ds";
```

```ds main.ds
import { buildWidget } from "./barrel.ds";
         ^^^^^^^^^^^ declaration:buildWidget

const value = buildWidget();
              ^^^^^^^^^^^ reference:buildWidget
```

```query goto_declaration main.ds#reference:buildWidget
@goto_declaration.target origin=main.ds#reference:buildWidget location=main.ds#declaration:buildWidget symbol=main.ds#buildWidget@1
```

## Pattern Bindings

### Resolve a destructured binding declaration

A destructured reference resolves to the exact binding introduced by its pattern.

```ds main.ds
const pair = { left: 1, right: 2 };
const { left } = pair;
        ^^^^ declaration:left

const value = left;
              ^^^^ reference:left
```

```query goto_declaration main.ds#reference:left
@goto_declaration.target origin=main.ds#reference:left location=main.ds#declaration:left symbol=main.ds#left@2
```

## Generic Parameters

### Resolve a type parameter declaration

A type parameter reference resolves to its owning generic declaration.

```ds main.ds
function identity<T>(value: T): T {
                  ^ declaration:type_parameter
                                ^ reference:type_parameter
    return value;
}
```

```query goto_declaration main.ds#reference:type_parameter
@goto_declaration.target origin=main.ds#reference:type_parameter location=main.ds#declaration:type_parameter symbol=main.ds#T@2
```

## Members

### Resolve a member declaration

A selected field access resolves to the member declaration.

```ds main.ds
struct Point {
    x: int32;
    ^ declaration:field
}

function read(point: Point): int32 {
    return point.x;
                 ^ reference:field
}
```

```query goto_declaration main.ds#reference:field
@goto_declaration.target origin=main.ds#reference:field location=main.ds:2:5-2:13 selection=main.ds#declaration:field symbol=main.ds#x@2
```

### Resolve every exact member declaration selected through a union

A union receiver returns the finite member declaration set selected by checking.

```ds main.ds
class Alpha {
    run(): void {}
    ^^^ declaration:alpha_run
}

class Beta {
    run(): void {}
    ^^^ declaration:beta_run
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ reference
}
```

```query goto_declaration main.ds#reference
@goto_declaration.target origin=main.ds#reference location=main.ds:2:5-2:19 selection=main.ds#declaration:alpha_run symbol=main.ds#run@2
@goto_declaration.target origin=main.ds#reference location=main.ds:6:5-6:19 selection=main.ds#declaration:beta_run symbol=main.ds#run@5
```

## Labels

### [ignored] Resolve a control label declaration

A labeled break resolves to the exact enclosing label.

```ds main.ds
function choose(): int32 {
    outer: loop {
    ^^^^^ declaration:outer
        break outer: 1;
              ^^^^^ reference:outer
    }
}
```

```query goto_declaration main.ds#reference:outer
@goto_declaration.target origin=main.ds#reference:outer location=main.ds#declaration:outer symbol=main.ds#outer@2
```

## Missing Symbols

### Return no declaration for an unresolved name

An unresolved occurrence has no declaration identity.

```ds main.ds
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query goto_declaration main.ds#reference
@goto_declaration.none
```
