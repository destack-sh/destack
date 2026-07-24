# Goto Type Definition

## Nominal Values

### Resolve a local nominal value

A value resolves to its nominal type declaration.

```ds main.ds
struct Point {
       ^^^^^ definition
    x: int32;
}

const point = Point { x: 1 };
      ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target relation=type_definition location=main.ds#definition symbol=main.ds#Point@1
```

### Resolve a class-valued binding

A binding with a class type resolves to the class declaration.

```ds main.ds
class Widget {
      ^^^^^^ definition:widget
}

declare const widget: Widget;
              ^^^^^^ reference:widget
```

```query goto_type_definition main.ds#reference:widget
@goto_type_definition.target relation=type_definition location=main.ds#definition:widget symbol=main.ds#Widget@1
```

### Resolve a typed parameter

A parameter resolves through its declared type.

```ds main.ds
struct Config {
       ^^^^^^ definition:config
    enabled: boolean;
}

function inspect(config: Config): void {}
                 ^^^^^^ reference:config
```

```query goto_type_definition main.ds#reference:config
@goto_type_definition.target relation=type_definition location=main.ds#definition:config symbol=main.ds#Config@1
```

### Resolve an enum-valued binding

An enum value resolves to the enum declaration rather than one variant.

```ds main.ds
enum Color {
     ^^^^^ definition:color
    Red,
}

const color: Color = Color.Red;
      ^^^^^ reference:color
```

```query goto_type_definition main.ds#reference:color
@goto_type_definition.target relation=type_definition location=main.ds#definition:color symbol=main.ds#Color@1
```

## Type References

### Resolve a direct type reference

A nominal name in type position resolves to its declaration.

```ds main.ds
class Animal {}
      ^^^^^^ definition:animal

declare const animal: Animal;
                      ^^^^^^ reference:animal
```

```query goto_type_definition main.ds#reference:animal
@goto_type_definition.target relation=type_definition location=main.ds#definition:animal symbol=main.ds#Animal@1
```

### Resolve a local type alias

A type alias reference resolves to the alias declaration.

```ds main.ds
type UserId = int32;
     ^^^^^^ definition:user_id

declare const userId: UserId;
                      ^^^^^^ reference:user_id
```

```query goto_type_definition main.ds#reference:user_id
@goto_type_definition.target relation=type_definition location=main.ds#definition:user_id symbol=main.ds#UserId@1
```

## Imports

### Resolve an imported nominal value

An imported value follows its type into the defining module.

```ds model.ds
export struct Widget {
              ^^^^^^ definition
    value: int32;
}
```

```ds main.ds
import { Widget } from "./model.ds";

const widget = Widget { value: 1 };
      ^^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target relation=type_definition location=model.ds#definition symbol=model.ds#Widget@1
```

## Scalar Values

### Return no nominal type for a scalar

A scalar value has no nominal type declaration.

```ds main.ds
const count = 1;
      ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.none
```

## Union Types

### Resolve every nominal member of a union

A binding with an unnamed union type resolves to each distinct nominal declaration.

```ds main.ds
class Circle {}
      ^^^^^^ definition:circle
class Square {}
      ^^^^^^ definition:square

declare const shape: Circle | Square;
              ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target relation=type_definition location=main.ds#definition:circle symbol=main.ds#Circle@1
@goto_type_definition.target relation=type_definition location=main.ds#definition:square symbol=main.ds#Square@2
```

### Resolve the nominal member of a nullable union

Null has no definition target, while the nominal member retains its declaration.

```ds main.ds
class User {}
      ^^^^ definition:user

declare const user: User | null;
              ^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target relation=type_definition location=main.ds#definition:user symbol=main.ds#User@1
```

## Imported Types

### Resolve an imported type

A plain import preserves the exported declaration's type symbol space.

```ds model.ds
export type Options = {
            ^^^^^^^ definition:imported_type
    enabled: boolean;
};
```

```ds main.ds
import { Options } from "./model.ds";

const options: Options = { enabled: true };
               ^^^^^^^ reference:imported_type
```

```query goto_type_definition main.ds#reference:imported_type
@goto_type_definition.target relation=type_definition location=model.ds#definition:imported_type symbol=model.ds#Options@1
```

### Resolve an imported type alias

An imported alias resolves to the defining type declaration.

```ds model.ds
export struct Settings {
              ^^^^^^^^ definition:settings
    enabled: boolean;
}
```

```ds main.ds
import { Settings as AppSettings } from "./model.ds";

declare const settings: AppSettings;
                        ^^^^^^^^^^^ reference:settings
```

```query goto_type_definition main.ds#reference:settings
@goto_type_definition.target relation=type_definition location=model.ds#definition:settings symbol=model.ds#Settings@1
```

### Resolve imported types and values together

One plain import preserves each declaration's original symbol space.

```ds types.ds
export struct Settings {
              ^^^^^^^^ definition:mixed
    enabled: boolean;
}
```

```ds values.ds
export function enabled(): boolean {
    return true;
}
```

```ds main.ds
import { Settings } from "./types.ds";
import { enabled } from "./values.ds";

declare const settings: Settings;
                        ^^^^^^^^ reference:mixed
const isEnabled = enabled();
```

```query goto_type_definition main.ds#reference:mixed
@goto_type_definition.target relation=type_definition location=types.ds#definition:mixed symbol=types.ds#Settings@1
```

### Follow re-exports

A type reference follows every plain re-export to its declaration.

```ds model.ds
export interface ServiceOptions {
                 ^^^^^^^^^^^^^^ definition:options
    enabled: boolean;
}
```

```ds first.ds
export { ServiceOptions as Options } from "./model.ds";
```

```ds second.ds
export { Options } from "./first.ds";
```

```ds main.ds
import { Options } from "./second.ds";

declare const options: Options;
                       ^^^^^^^ reference:options
```

```query goto_type_definition main.ds#reference:options
@goto_type_definition.target relation=type_definition location=model.ds#definition:options symbol=model.ds#ServiceOptions@1
```

## Import Forms

### Resolve a namespace type member

A type member on a namespace import resolves to the exported declaration.

```ds model.ds
export struct Settings {
              ^^^^^^^^ definition:namespace
    enabled: boolean;
}
```

```ds main.ds
import * as models from "./model.ds";

declare const settings: models.Settings;
                               ^^^^^^^^ reference:namespace
```

```query goto_type_definition main.ds#reference:namespace
@goto_type_definition.target relation=type_definition location=model.ds#definition:namespace symbol=model.ds#Settings@1
```

### Resolve a default class import

A default import resolves to the defining class declaration.

```ds model.ds
export default class Widget {
                     ^^^^^^ definition:default
}
```

```ds main.ds
import WidgetModel from "./model.ds";

declare const widget: WidgetModel;
                      ^^^^^^^^^^^ reference:default
```

```query goto_type_definition main.ds#reference:default
@goto_type_definition.target relation=type_definition location=model.ds#definition:default symbol=model.ds#Widget@1
```

### Follow a default re-export alias

A named import follows a default re-export to the defining class.

```ds model.ds
export default class Widget {
                     ^^^^^^ definition:reexport
}
```

```ds barrel.ds
export { default as Widget } from "./model.ds";
```

```ds main.ds
import { Widget } from "./barrel.ds";

declare const widget: Widget;
                      ^^^^^^ reference:reexport
```

```query goto_type_definition main.ds#reference:reexport
@goto_type_definition.target relation=type_definition location=model.ds#definition:reexport symbol=model.ds#Widget@1
```

## Ownership Forms

### Resolve borrowed, owned, and pointer values

Memory forms retain the nominal declaration of their contained value.

```ds main.ds
struct Buffer {
       ^^^^^^ definition:buffer
    value: int32;
}

declare const borrowed: &Buffer;
              ^^^^^^^^ reference:borrowed
declare const owned: ^Buffer;
              ^^^^^ reference:owned
declare const pointer: *Buffer;
              ^^^^^^^ reference:pointer
```

```query goto_type_definition main.ds#reference:borrowed
@goto_type_definition.target relation=type_definition location=main.ds#definition:buffer symbol=main.ds#Buffer@1
```

```query goto_type_definition main.ds#reference:owned
@goto_type_definition.target relation=type_definition location=main.ds#definition:buffer symbol=main.ds#Buffer@1
```

```query goto_type_definition main.ds#reference:pointer
@goto_type_definition.target relation=type_definition location=main.ds#definition:buffer symbol=main.ds#Buffer@1
```

## Explicit Receivers

### Resolve an explicit receiver type

An explicit receiver annotation resolves like any other nominal type reference.

```ds main.ds
class Counter {}
      ^^^^^^^ definition:counter

function read(this: Counter): Counter {
                    ^^^^^^^ reference:counter
    return this;
}
```

```query goto_type_definition main.ds#reference:counter
@goto_type_definition.target relation=type_definition location=main.ds#definition:counter symbol=main.ds#Counter@1
```

## Generic Applications

### Resolve an applied nominal type

A generic application retains its nominal declaration.

```ds main.ds
class Box<T> {
      ^^^ definition:box
    value: T;
}

declare const box: Box<int32>;
                   ^^^ reference:box
```

```query goto_type_definition main.ds#reference:box
@goto_type_definition.target relation=type_definition location=main.ds#definition:box symbol=main.ds#Box@1
```

## Type and Value Names

### Resolve a type hidden by a value binding

A value binding with the same name does not replace an imported type.

```ds model.ds
export struct Config {
              ^^^^^^ definition:config
    enabled: boolean;
}
```

```ds main.ds
import { Config as SharedConfig } from "./model.ds";

const SharedConfig = 1;

declare const config: SharedConfig;
                      ^^^^^^^^^^^^ reference:config
```

```query goto_type_definition main.ds#reference:config
@goto_type_definition.target relation=type_definition location=model.ds#definition:config symbol=model.ds#Config@1
```

## Missing Symbols

### Return no type definition for an unresolved name

An unresolved occurrence has no type declaration.

```ds main.ds
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query goto_type_definition main.ds#reference
@goto_type_definition.none
```
