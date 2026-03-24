# Goto Type Definition

Tests for LSP goto type definition functionality.

## Basic Types

### Variable with class type

Go to type definition should navigate from a variable to its type.

```ds
class MyClass {
//    ^^^^^^^ def:MyClass
    value: int32
}

const x: MyClass = MyClass { value: 42 };
//    ^ use:x
```

```query goto_type_definition use:x
def:MyClass
```

### Variable with struct type

Go to type definition should navigate from a variable to its struct type.

```ds
struct Point {
//     ^^^^^ def:Point
    x: float32
    y: float32
}

const p: Point = Point { x: 1.0, y: 2.0 };
//    ^ use:p
```

```query goto_type_definition use:p
def:Point
```

### Parameter with type

Go to type definition should work on function parameters.

```ds
struct Config {
//     ^^^^^^ def:Config
    enabled: bool
}

function process(cfg: Config): void {}
//               ^^^ use:cfg
```

```query goto_type_definition use:cfg
def:Config
```

## Type References

### Direct type reference

Go to type definition on a type should go to that type's definition.

```ds
class Animal {}
//    ^^^^^^ def:Animal

const x: Animal = Animal {};
//       ^^^^^^ use:Animal
```

```query goto_type_definition use:Animal
def:Animal
```

### Type alias reference

Go to type definition on a type alias should navigate to the alias declaration.

```ds
type UserId = int32;
//   ^^^^^^ def:UserId

const id: UserId = 1;
//        ^^^^^^ use:UserId
```

```query goto_type_definition use:UserId
def:UserId
```

### Re-exported type reference

Go to type definition should follow re-export chains to the original type.

```ds:types.ds
export type Thing = string;
//          ^^^^^ def:Thing_source
```

```ds:re_exports.ds
export type { Thing } from "./types.ds";
//            ^^^^^ def:Thing
```

```ds:main.ds
import type { Thing } from "./re_exports.ds";

const value: Thing = "ok";
//           ^^^^^ use:Thing
```

```query goto_type_definition use:Thing
def:Thing_source
```

### Enum type

Go to type definition should work with enum types.

```ds
enum Color {
//   ^^^^^ def:Color
    Red,
    Green,
    Blue,
}

const c: Color = Color.Red;
//    ^ use:c
```

```query goto_type_definition use:c
def:Color
```

## Primitive Types

### Primitive type variable

For variables with primitive types, goto type definition returns no result.

```ds
const x: int32 = 42;
//    ^ use:x
```

```query goto_type_definition use:x
<none>
```

## Type And Value Imports

### Resolve type definitions with mixed import shapes

Go to type definition should still resolve type imports when value imports are present in the same module.

```ds:types.ds
export struct Settings {
//            ^^^^^^^^ def:Settings
    enabled: bool
}
```

```ds:values.ds
export function settings(): int32 {
    return 1;
}
```

```ds:main.ds
import type { Settings } from "./types.ds";
import { settings } from "./values.ds";

const typed: Settings = Settings { enabled: true };
//           ^^^^^^^^ use:settings_type_use
const value = settings();
```

```query goto_type_definition use:settings_type_use
def:Settings
```

## Cross Module Import Shapes

### Resolve type definition through type-only import alias

Go to type definition should resolve aliased type-only imports to their source type.

```ds:types_alias.ds
export struct Settings {
//            ^^^^^^^^ def:Settings_alias_source
    enabled: bool
}
```

```ds:main_alias.ds
import type { Settings as AppSettings } from "./types_alias.ds";

const typed: AppSettings = AppSettings { enabled: true };
//           ^^^^^^^^^^^ use:app_settings_type
```

```query goto_type_definition use:app_settings_type
def:Settings_alias_source
```

### Resolve type definition through re-exported type alias chain

Go to type definition should follow multi-hop type-only re-export chains.

```ds:base_chain.ds
export interface ServiceOptions {
//               ^^^^^^^^^^^^^^ def:ServiceOptions
    enabled: boolean,
}
```

```ds:barrel_a.ds
export type { ServiceOptions as Options } from "./base_chain.ds";
```

```ds:barrel_b.ds
export type { Options } from "./barrel_a.ds";
```

```ds:main_chain.ds
import type { Options } from "./barrel_b.ds";

const typed: Options = { enabled: true };
//           ^^^^^^^ use:options_type
```

```query goto_type_definition use:options_type
def:ServiceOptions
```

### Resolve type definition for default imported class

Go to type definition should resolve default imported class aliases to the source class declaration.

```ds:class_model.ds
export default class Widget {
//                   ^^^^^^ def:Widget_default
    value: int32;
}
```

```ds:main_default_class.ds
import WidgetModel from "./class_model.ds";

const model: WidgetModel = new WidgetModel();
//           ^^^^^^^^^^^ use:widget_model_type
```

```query goto_type_definition use:widget_model_type
def:Widget_default
```

### Resolve type definition through namespace import type member

Go to type definition should resolve namespaced type references from namespace imports.

```ds:ns_types.ds
export struct Settings {
//            ^^^^^^^^ def:NsSettings
    enabled: bool
}
```

```ds:ns_main.ds
import * as models from "./ns_types.ds";

const typed: models.Settings = models.Settings { enabled: true };
//                  ^^^^^^^^ use:namespace_settings_type
```

```query goto_type_definition use:namespace_settings_type
def:NsSettings
```

### Resolve type definition when value bindings shadow the type name

Go to type definition in a type annotation should still resolve the imported type when a local value has the same identifier.

```ds:shadow_types.ds
export struct Config {
//            ^^^^^^ def:ShadowConfig
    enabled: bool
}
```

```ds:shadow_main.ds
import type { Config as SharedConfig } from "./shadow_types.ds";

const SharedConfig = 1;

function consume(config: SharedConfig): void {
//                       ^^^^^^^^^^^ use:shared_config_type
    const local = SharedConfig;
    console.log(local);
}
```

```query goto_type_definition use:shared_config_type
def:ShadowConfig
```

## Damaged Syntax

### Keep type definitions working after malformed function declarations

Go to type definition should still work for later type uses after one malformed function head.

```ds
export function broken( {}

type Config = {
//   ^^^^^^ def:Config
    enabled: boolean,
};

const current: Config = { enabled: true };
//             ^^^^^^ use:Config
```

```query goto_type_definition use:Config
def:Config
```

### Keep type definitions working after malformed call statements

Go to type definition should still work for later type uses after one malformed call statement.

```ds
broken(,

type Config = {
//   ^^^^^^ def:Config
    enabled: boolean,
};

const current: Config = { enabled: true };
//             ^^^^^^ use:Config
```

```query goto_type_definition use:Config
def:Config
```

### Keep type definitions working after bare new recovery statements

Go to type definition should still work for later type uses after one bare `new` recovery statement.

```ds
new

type Config = {
//   ^^^^^^ def:Config
    enabled: boolean,
};

const current: Config = { enabled: true };
//             ^^^^^^ use:Config
```

```query goto_type_definition use:Config
def:Config
```

### Return no type definition for malformed unresolved member access

Go to type definition should return no result when the cursor is on malformed unresolved syntax.

```ds
function main(): void {
    missingValue.
//  ^^^^^^^^^^^ broken
}
```

```query goto_type_definition broken
<none>
```

## Ownership And This

### Goto type definition for ownership type forms

Goto type definition should resolve symbols in borrowed, owned, and pointer type forms.

```ds
struct Buffer {
//     ^^^^^^ def:buffer
    value: int32
}

declare const borrowed: &Buffer;
declare const owned: ^Buffer;
declare const pointer: *Buffer;

const a: &Buffer = borrowed;
//    ^ use:a
const b: ^Buffer = owned;
//    ^ use:b
const c: *Buffer = pointer;
//    ^ use:c
```

```query goto_type_definition use:a
def:buffer
```

```query goto_type_definition use:b
def:buffer
```

```query goto_type_definition use:c
def:buffer
```

### Goto type definition for explicit this parameters

Goto type definition should resolve symbols used in explicit this parameter types.

```ds
class Counter {}
//    ^^^^^^^ def:counter

function read(this: Counter): Counter {
//                  ^^^^^^^ use:counter
    return this;
}
```

```query goto_type_definition use:counter
def:counter
```
