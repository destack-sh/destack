
## Locals

### Resolve a local variable declaration

A local reference resolves to its declaration.

```tspp main.tspp
const value = 1;
      ^^^^^ declaration:value

const other = value;
              ^^^^^ reference:value
```

```query goto_declaration main.tspp#reference:value
@goto_declaration.target origin=main.tspp#reference:value location=main.tspp#declaration:value symbol=main.tspp#value@1
```

### Resolve the current binding

A reference resolves to the declaration selected after each edit.

```tspp main.tspp
const first = 1;
      ^^^^^ declaration:first
const second = 2;
      ^^^^^^ declaration:second
const selected = first;
                 ^^^^^ reference
```

```query goto_declaration main.tspp#reference
@goto_declaration.target origin=main.tspp#reference location=main.tspp#declaration:first symbol=main.tspp#first@1
```

```tspp main.tspp change
const first = 1;
      ^^^^^ declaration:first
const second = 2;
      ^^^^^^ declaration:second
const selected = second;
                 ^^^^^^ reference
```

```query goto_declaration main.tspp#reference
@goto_declaration.target origin=main.tspp#reference location=main.tspp#declaration:second symbol=main.tspp#second@2
```

## Imports

### Resolve an imported symbol declaration

An imported reference resolves to its local import.

```tspp library.tspp
export function greet(name: string): string {
    return name;
}
```

```tspp main.tspp
import { greet } from "./library.tspp";
         ^^^^^ declaration:import_greet

const message = greet("World");
                ^^^^^ reference:greet
```

```query goto_declaration main.tspp#reference:greet
@goto_declaration.target origin=main.tspp#reference:greet location=main.tspp#declaration:import_greet symbol=main.tspp#greet@1
```

### Resolve an aliased import declaration

An aliased import resolves to its local alias.

```tspp alias_library.tspp
export function greet(name: string): string {
    return name;
}
```

```tspp alias_main.tspp
import { greet as localGreet } from "./alias_library.tspp";
         ^ target:import_local_greet:start
                  ^^^^^^^^^^ declaration:import_local_greet
                           ^ target:import_local_greet:end

const message = localGreet("World");
                ^^^^^^^^^^ reference:local_greet
```

```query goto_declaration alias_main.tspp#reference:local_greet
@goto_declaration.target origin=alias_main.tspp#reference:local_greet location=alias_main.tspp#target:import_local_greet selection=alias_main.tspp#declaration:import_local_greet symbol=alias_main.tspp#localGreet@1
```

### Keep the imported and local sides of an alias distinct

The imported name resolves to the exported declaration while the local name resolves to its alias.

```tspp library.tspp
export function greet(): void {}
^ target:exported_greet:start
                ^^^^^ declaration:exported_greet
                               ^ target:exported_greet:end
```

```tspp main.tspp
import { greet as welcome } from "./library.tspp";
         ^ target:local_welcome:start
         ^^^^^ imported_name
                  ^^^^^^^ declaration:local_welcome
                        ^ target:local_welcome:end

welcome();
^^^^^^^ reference:local_welcome
```

```query goto_declaration main.tspp#imported_name
@goto_declaration.target origin=main.tspp#imported_name location=library.tspp#target:exported_greet selection=library.tspp#declaration:exported_greet symbol=library.tspp#greet@1
```

```query goto_declaration main.tspp#reference:local_welcome
@goto_declaration.target origin=main.tspp#reference:local_welcome location=main.tspp#target:local_welcome selection=main.tspp#declaration:local_welcome symbol=main.tspp#welcome@1
```

### Resolve a re-exported alias declaration

A re-exported alias still resolves to its local import.

```tspp alias_base.tspp
export function greet(name: string): string {
    return name;
}
```

```tspp alias_barrel.tspp
export { greet as greetAlias } from "./alias_base.tspp";
```

```tspp alias_main.tspp
import { greetAlias } from "./alias_barrel.tspp";
         ^^^^^^^^^^ declaration:import_greet_alias

const message = greetAlias("World");
                ^^^^^^^^^^ reference:greet_alias
```

```query goto_declaration alias_main.tspp#reference:greet_alias
@goto_declaration.target origin=alias_main.tspp#reference:greet_alias location=alias_main.tspp#declaration:import_greet_alias symbol=alias_main.tspp#greetAlias@1
```

### Resolve an imported type declaration

An imported type resolves to its local import.

```tspp types.tspp
export struct Thing {
    value: int32;
}
```

```tspp main.tspp
import { Thing } from "./types.tspp";
         ^^^^^ declaration:import_thing

const item: Thing = Thing { value: 1 };
            ^^^^^ reference:thing
```

```query goto_declaration main.tspp#reference:thing
@goto_declaration.target origin=main.tspp#reference:thing location=main.tspp#declaration:import_thing symbol=main.tspp#Thing@1
```

### Resolve a namespace import declaration

A namespace reference resolves to its local alias.

```tspp utilities.tspp
export function ping(): void {}
```

```tspp main.tspp
import * as utilities from "./utilities.tspp";
       ^ target:import_utilities:start
            ^^^^^^^^^ declaration:import_utilities
                    ^ target:import_utilities:end

utilities.ping();
^^^^^^^^^ reference:utilities
```

```query goto_declaration main.tspp#reference:utilities
@goto_declaration.target origin=main.tspp#reference:utilities location=main.tspp#target:import_utilities selection=main.tspp#declaration:import_utilities symbol=main.tspp#utilities@1
```

### Resolve each segment of a namespace type path

The namespace root resolves to its local import, while the type segment resolves to its declaration.

```tspp model.tspp
export struct Settings {
^ target:settings:start
              ^^^^^^^^ declaration:settings
    enabled: boolean;
}
^ target:settings:end
```

```tspp main.tspp
import * as models from "./model.tspp";
       ^ target:models:start
            ^^^^^^ declaration:models
                 ^ target:models:end

type Selected = models.Settings;
                ^^^^^^ reference:models
                       ^^^^^^^^ reference:settings
```

```query goto_declaration main.tspp#reference:models
@goto_declaration.target origin=main.tspp#reference:models location=main.tspp#target:models selection=main.tspp#declaration:models symbol=main.tspp#models@1
```

```query goto_declaration main.tspp#reference:settings
@goto_declaration.target origin=main.tspp#reference:settings location=model.tspp#target:settings selection=model.tspp#declaration:settings symbol=model.tspp#Settings@1
```

### Resolve a nested namespace type path

Each namespace segment selects its authored import or re-export declaration.

```tspp model.tspp
export struct Packet {}
^^^^^^^^^^^^^^^^^^^^^^^ packet
              ^^^^^^ name
```

```tspp library.tspp
export * as models from "./model";
       ^ namespace:start
            ^^^^^^ name
                                ^ namespace:end
```

```tspp main.tspp
import * as library from "./library";
       ^ import:start
            ^^^^^^^ name
                  ^ import:end

declare const packet: library.models.Packet;
                      ^^^^^^^ root
                              ^^^^^^ namespace
                                     ^^^^^^ type
```

```query goto_declaration main.tspp#root
@goto_declaration.target origin=main.tspp#root location=main.tspp#import selection=main.tspp#name symbol=main.tspp#library@1
```

```query goto_declaration main.tspp#namespace
@goto_declaration.target origin=main.tspp#namespace location=library.tspp#namespace selection=library.tspp#name symbol=library.tspp#models@1
```

```query goto_declaration main.tspp#type
@goto_declaration.target origin=main.tspp#type location=model.tspp#packet selection=model.tspp#name symbol=model.tspp#Packet@1
```

### Resolve a re-exported import declaration

A re-exported symbol resolves to its downstream import.

```tspp library.tspp
export function greet(name: string): string {
    return name;
}
```

```tspp barrel.tspp
export { greet } from "./library.tspp";
```

```tspp main.tspp
import { greet } from "./barrel.tspp";
         ^^^^^ declaration:import_greet

const message = greet("World");
                ^^^^^ reference:greet
```

```query goto_declaration main.tspp#reference:greet
@goto_declaration.target origin=main.tspp#reference:greet location=main.tspp#declaration:import_greet symbol=main.tspp#greet@1
```

### Resolve declarations for type and value imports

Type and value references resolve to their respective imports.

```tspp types.tspp
export type Settings = {
    enabled: boolean,
};
```

```tspp values.tspp
export function settings(): int32 {
    return 1;
}
```

```tspp main.tspp
import { Settings } from "./types.tspp";
         ^^^^^^^^ declaration:import_settings_type
import { settings } from "./values.tspp";
         ^^^^^^^^ declaration:import_settings_value

const typed: Settings = { enabled: true };
             ^^^^^^^^ reference:settings_type
const value = settings();
              ^^^^^^^^ reference:settings_value
```

```query goto_declaration main.tspp#reference:settings_type
@goto_declaration.target origin=main.tspp#reference:settings_type location=main.tspp#declaration:import_settings_type symbol=main.tspp#Settings@1
```

```query goto_declaration main.tspp#reference:settings_value
@goto_declaration.target origin=main.tspp#reference:settings_value location=main.tspp#declaration:import_settings_value symbol=main.tspp#settings@2
```

### Resolve an imported type alias declaration

An aliased type resolves to its local alias.

```tspp types_alias.tspp
export type Settings = {
    enabled: boolean,
};
```

```tspp main_alias.tspp
import { Settings as ApplicationSettings } from "./types_alias.tspp";
         ^ target:import_application_settings:start
                     ^^^^^^^^^^^^^^^^^^^ declaration:import_application_settings
                                       ^ target:import_application_settings:end

const typed: ApplicationSettings = { enabled: true };
             ^^^^^^^^^^^^^^^^^^^ reference:application_settings
```

```query goto_declaration main_alias.tspp#reference:application_settings
@goto_declaration.target origin=main_alias.tspp#reference:application_settings location=main_alias.tspp#target:import_application_settings selection=main_alias.tspp#declaration:import_application_settings symbol=main_alias.tspp#ApplicationSettings@1
```

### Resolve a re-exported type alias declaration

A re-exported type alias resolves to its downstream import.

```tspp base_type.tspp
export interface ServiceConfiguration {
    enabled: boolean;
}
```

```tspp barrel_type.tspp
export { ServiceConfiguration as Configuration } from "./base_type.tspp";
```

```tspp main_reexport_type.tspp
import { Configuration } from "./barrel_type.tspp";
         ^^^^^^^^^^^^^ declaration:import_configuration

const typed: Configuration = { enabled: true };
             ^^^^^^^^^^^^^ reference:configuration
```

```query goto_declaration main_reexport_type.tspp#reference:configuration
@goto_declaration.target origin=main_reexport_type.tspp#reference:configuration location=main_reexport_type.tspp#declaration:import_configuration symbol=main_reexport_type.tspp#Configuration@1
```

### Resolve a default import declaration

A default import resolves to its local binding.

```tspp default_library.tspp
export default function createValue(): int32 {
    return 1;
}
```

```tspp default_main.tspp
import buildValue from "./default_library.tspp";
       ^^^^^^^^^^ declaration:import_build_value

const value = buildValue();
              ^^^^^^^^^^ reference:build_value
```

```query goto_declaration default_main.tspp#reference:build_value
@goto_declaration.target origin=default_main.tspp#reference:build_value location=default_main.tspp#declaration:import_build_value symbol=default_main.tspp#buildValue@1
```

### Resolve declarations through default re-export alias chains

A default re-export alias resolves to its downstream import.

```tspp library.tspp
export default function buildWidget(): int32 {
    return 1;
}
```

```tspp barrel.tspp
export { default as buildWidget } from "./library.tspp";
```

```tspp main.tspp
import { buildWidget } from "./barrel.tspp";
         ^^^^^^^^^^^ declaration:buildWidget

const value = buildWidget();
              ^^^^^^^^^^^ reference:buildWidget
```

```query goto_declaration main.tspp#reference:buildWidget
@goto_declaration.target origin=main.tspp#reference:buildWidget location=main.tspp#declaration:buildWidget symbol=main.tspp#buildWidget@1
```

## Pattern Bindings

### Resolve a destructured binding declaration

A destructured reference resolves to the binding introduced by its pattern.

```tspp main.tspp
const pair = { left: 1, right: 2 };
const { left } = pair;
        ^^^^ declaration:left

const value = left;
              ^^^^ reference:left
```

```query goto_declaration main.tspp#reference:left
@goto_declaration.target origin=main.tspp#reference:left location=main.tspp#declaration:left symbol=main.tspp#left@2
```

## Generic Parameters

### Resolve a type parameter declaration

A type parameter reference resolves to its owning generic declaration.

```tspp main.tspp
function identity<T>(value: T): T {
                  ^ declaration:type_parameter
                                ^ reference:type_parameter
    return value;
}
```

```query goto_declaration main.tspp#reference:type_parameter
@goto_declaration.target origin=main.tspp#reference:type_parameter location=main.tspp#declaration:type_parameter symbol=main.tspp#T@2
```

## Members

### Resolve a member declaration

A field access resolves to the member declaration.

```tspp main.tspp
struct Point {
    x: int32;
    ^ declaration:field
    ^^^^^^^^ target:field
}

function read(point: Point): int32 {
    return point.x;
                 ^ reference:field
}
```

```query goto_declaration main.tspp#reference:field
@goto_declaration.target origin=main.tspp#reference:field location=main.tspp#target:field selection=main.tspp#declaration:field symbol=main.tspp#x@2
```

### Resolve every member declaration reached through a union

A union receiver returns every member declaration available at that access.

```tspp main.tspp
class Alpha {
    run(): void {}
    ^^^ declaration:alpha_run
    ^^^^^^^^^^^^^^ target:alpha_run
}

class Beta {
    run(): void {}
    ^^^ declaration:beta_run
    ^^^^^^^^^^^^^^ target:beta_run
}

function start(service: Alpha | Beta): void {
    service.run();
            ^^^ reference
}
```

```query goto_declaration main.tspp#reference
@goto_declaration.target origin=main.tspp#reference location=main.tspp#target:alpha_run selection=main.tspp#declaration:alpha_run symbol=main.tspp#run@2
@goto_declaration.target origin=main.tspp#reference location=main.tspp#target:beta_run selection=main.tspp#declaration:beta_run symbol=main.tspp#run@5
```

## Labels

### Resolve a control label declaration

A labeled break resolves to its enclosing label.

```tspp main.tspp
function choose(): int32 {
    outer: loop {
    ^ target:outer:start
    ^^^^^ declaration:outer
        break outer: 1;
              ^^^^^ reference:outer
    }
    ^ target:outer:end
}
```

```query goto_declaration main.tspp#reference:outer
@goto_declaration.target origin=main.tspp#reference:outer location=main.tspp#target:outer selection=main.tspp#declaration:outer symbol=main.tspp#outer@2
```

## Missing Symbols

### Return no declaration for an unresolved name

An unresolved occurrence has no declaration identity.

```tspp main.tspp
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query goto_declaration main.tspp#reference
@goto_declaration.none
```
