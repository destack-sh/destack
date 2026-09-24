
## Nominal Values

### Resolve a local nominal value

A value resolves to its nominal type declaration.

```ds main.ds
struct Point {
^ declaration:start
       ^^^^^ definition
    x: int32;
}
^ declaration:end

const point = Point { x: 1 };
      ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target origin=main.ds#reference location=main.ds#declaration selection=main.ds#definition symbol=main.ds#Point@1
```

### Resolve a class-valued binding

A binding with a class type resolves to the class declaration.

```ds main.ds
class Widget {
^ declaration:widget:start
      ^^^^^^ definition:widget
}
^ declaration:widget:end

declare const widget: Widget;
              ^^^^^^ reference:widget
```

```query goto_type_definition main.ds#reference:widget
@goto_type_definition.target origin=main.ds#reference:widget location=main.ds#declaration:widget selection=main.ds#definition:widget symbol=main.ds#Widget@1
```

### Resolve a typed parameter

A parameter resolves through its declared type.

```ds main.ds
struct Config {
^ declaration:config:start
       ^^^^^^ definition:config
    enabled: boolean;
}
^ declaration:config:end

function inspect(config: Config): void {}
                 ^^^^^^ reference:config
```

```query goto_type_definition main.ds#reference:config
@goto_type_definition.target origin=main.ds#reference:config location=main.ds#declaration:config selection=main.ds#definition:config symbol=main.ds#Config@1
```

### Resolve an enum-valued binding

An enum value resolves to the enum declaration rather than one variant.

```ds main.ds
enum Color {
^ declaration:color:start
     ^^^^^ definition:color
    Red,
}
^ declaration:color:end

const color: Color = Color.Red;
      ^^^^^ reference:color
```

```query goto_type_definition main.ds#reference:color
@goto_type_definition.target origin=main.ds#reference:color location=main.ds#declaration:color selection=main.ds#definition:color symbol=main.ds#Color@1
```

### Resolve the current value type

A value resolves to its nominal type after each edit.

```ds main.ds
struct First {
^ declaration:first:start
       ^^^^^ definition:first
}
^ declaration:first:end

struct Second {
^ declaration:second:start
       ^^^^^^ definition:second
}
^ declaration:second:end

declare const value: First;
              ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target origin=main.ds#reference location=main.ds#declaration:first selection=main.ds#definition:first symbol=main.ds#First@1
```

```ds main.ds change
struct First {
^ declaration:first:start
       ^^^^^ definition:first
}
^ declaration:first:end

struct Second {
^ declaration:second:start
       ^^^^^^ definition:second
}
^ declaration:second:end

declare const value: Second;
              ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target origin=main.ds#reference location=main.ds#declaration:second selection=main.ds#definition:second symbol=main.ds#Second@2
```

## Type References

### Resolve primitive runtime declarations

Primitive types resolve to their runtime declarations.

```ds main.ds
type Text = string;
            ^^^^^^ reference:string

type Integer = bigint;
               ^^^^^^ reference:bigint
```

```query goto_type_definition main.ds#reference:string
@goto_type_definition.target origin=main.ds#reference:string location=destack://string/string:16:1-19:2 selection=destack://string/string:16:14-16:20 symbol=destack://string/string#String@28
```

```query goto_type_definition main.ds#reference:bigint
@goto_type_definition.target origin=main.ds#reference:bigint location=destack://math/bigint:27:1-36:2 selection=destack://math/bigint:27:14-27:20 symbol=destack://math/bigint#BigInt@26
```

### Resolve a direct type reference

A nominal name in type position resolves to its declaration.

```ds main.ds
class Animal {}
^ declaration:animal:start
      ^^^^^^ definition:animal
              ^ declaration:animal:end

declare const animal: Animal;
                      ^^^^^^ reference:animal
```

```query goto_type_definition main.ds#reference:animal
@goto_type_definition.target origin=main.ds#reference:animal location=main.ds#declaration:animal selection=main.ds#definition:animal symbol=main.ds#Animal@1
```

### Resolve a local type alias

A type alias reference resolves to the alias declaration.

```ds main.ds
type UserId = int32;
^ declaration:user_id:start
     ^^^^^^ definition:user_id
                  ^ declaration:user_id:end

declare const userId: UserId;
                      ^^^^^^ reference:user_id
```

```query goto_type_definition main.ds#reference:user_id
@goto_type_definition.target origin=main.ds#reference:user_id location=main.ds#declaration:user_id selection=main.ds#definition:user_id symbol=main.ds#UserId@1
```

## Imports

### Resolve an imported nominal value

An imported value follows its type into the defining module.

```ds model.ds
export struct Widget {
^ declaration:start
              ^^^^^^ definition
    value: int32;
}
^ declaration:end
```

```ds main.ds
import { Widget } from "./model.ds";

const widget = Widget { value: 1 };
      ^^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target origin=main.ds#reference location=model.ds#declaration selection=model.ds#definition symbol=model.ds#Widget@1
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
^ declaration:circle:start
      ^^^^^^ definition:circle
              ^ declaration:circle:end
class Square {}
^ declaration:square:start
      ^^^^^^ definition:square
              ^ declaration:square:end

declare const shape: Circle | Square;
              ^^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target origin=main.ds#reference location=main.ds#declaration:circle selection=main.ds#definition:circle symbol=main.ds#Circle@1
@goto_type_definition.target origin=main.ds#reference location=main.ds#declaration:square selection=main.ds#definition:square symbol=main.ds#Square@2
```

### Resolve the nominal member of a nullable union

Null has no definition target, while the nominal member resolves to its declaration.

```ds main.ds
class User {}
^ declaration:user:start
      ^^^^ definition:user
            ^ declaration:user:end

declare const user: User | null;
              ^^^^ reference
```

```query goto_type_definition main.ds#reference
@goto_type_definition.target origin=main.ds#reference location=main.ds#declaration:user selection=main.ds#definition:user symbol=main.ds#User@1
```

## Imported Types

### Resolve an imported type

A plain import preserves the exported declaration's type symbol space.

```ds model.ds
export type Options = {
^ declaration:imported_type:start
            ^^^^^^^ definition:imported_type
    enabled: boolean;
};
^ declaration:imported_type:end
```

```ds main.ds
import { Options } from "./model.ds";

const options: Options = { enabled: true };
               ^^^^^^^ reference:imported_type
```

```query goto_type_definition main.ds#reference:imported_type
@goto_type_definition.target origin=main.ds#reference:imported_type location=model.ds#declaration:imported_type selection=model.ds#definition:imported_type symbol=model.ds#Options@1
```

### Resolve an imported type alias

An imported alias resolves to the defining type declaration.

```ds model.ds
export struct Settings {
^ declaration:settings:start
              ^^^^^^^^ definition:settings
    enabled: boolean;
}
^ declaration:settings:end
```

```ds main.ds
import { Settings as AppSettings } from "./model.ds";

declare const settings: AppSettings;
                        ^^^^^^^^^^^ reference:settings
```

```query goto_type_definition main.ds#reference:settings
@goto_type_definition.target origin=main.ds#reference:settings location=model.ds#declaration:settings selection=model.ds#definition:settings symbol=model.ds#Settings@1
```

### Resolve imported types and values together

One plain import preserves each declaration's original symbol space.

```ds types.ds
export struct Settings {
^ declaration:mixed:start
              ^^^^^^^^ definition:mixed
    enabled: boolean;
}
^ declaration:mixed:end
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
@goto_type_definition.target origin=main.ds#reference:mixed location=types.ds#declaration:mixed selection=types.ds#definition:mixed symbol=types.ds#Settings@1
```

### Follow re-exports

A type reference follows every plain re-export to its declaration.

```ds model.ds
export interface ServiceOptions {
^ declaration:options:start
                 ^^^^^^^^^^^^^^ definition:options
    enabled: boolean;
}
^ declaration:options:end
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
@goto_type_definition.target origin=main.ds#reference:options location=model.ds#declaration:options selection=model.ds#definition:options symbol=model.ds#ServiceOptions@1
```

## Import Forms

### Resolve a namespace type member

A type member on a namespace import resolves to the exported declaration.

```ds model.ds
export struct Settings {
^ declaration:namespace:start
              ^^^^^^^^ definition:namespace
    enabled: boolean;
}
^ declaration:namespace:end
```

```ds main.ds
import * as models from "./model.ds";

declare const settings: models.Settings;
                               ^^^^^^^^ reference:namespace
```

```query goto_type_definition main.ds#reference:namespace
@goto_type_definition.target origin=main.ds#reference:namespace location=model.ds#declaration:namespace selection=model.ds#definition:namespace symbol=model.ds#Settings@1
```

### Resolve a default class import

A default import resolves to the defining class declaration.

```ds model.ds
export default class Widget {
^ declaration:default:start
                     ^^^^^^ definition:default
}
^ declaration:default:end
```

```ds main.ds
import WidgetModel from "./model.ds";

declare const widget: WidgetModel;
                      ^^^^^^^^^^^ reference:default
```

```query goto_type_definition main.ds#reference:default
@goto_type_definition.target origin=main.ds#reference:default location=model.ds#declaration:default selection=model.ds#definition:default symbol=model.ds#Widget@1
```

### Follow a default re-export alias

A named import follows a default re-export to the defining class.

```ds model.ds
export default class Widget {
^ declaration:reexport:start
                     ^^^^^^ definition:reexport
}
^ declaration:reexport:end
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
@goto_type_definition.target origin=main.ds#reference:reexport location=model.ds#declaration:reexport selection=model.ds#definition:reexport symbol=model.ds#Widget@1
```

## Ownership Forms

### Resolve borrowed, owned, and pointer values

Memory forms resolve to the nominal declaration of their contained value.

```ds main.ds
struct Buffer {
^ declaration:buffer:start
       ^^^^^^ definition:buffer
    value: int32;
}
^ declaration:buffer:end

declare const borrowed: &Buffer;
              ^^^^^^^^ reference:borrowed
declare const owned: ^Buffer;
              ^^^^^ reference:owned
declare const pointer: *Buffer;
              ^^^^^^^ reference:pointer
```

```query goto_type_definition main.ds#reference:borrowed
@goto_type_definition.target origin=main.ds#reference:borrowed location=main.ds#declaration:buffer selection=main.ds#definition:buffer symbol=main.ds#Buffer@1
```

```query goto_type_definition main.ds#reference:owned
@goto_type_definition.target origin=main.ds#reference:owned location=main.ds#declaration:buffer selection=main.ds#definition:buffer symbol=main.ds#Buffer@1
```

```query goto_type_definition main.ds#reference:pointer
@goto_type_definition.target origin=main.ds#reference:pointer location=main.ds#declaration:buffer selection=main.ds#definition:buffer symbol=main.ds#Buffer@1
```

## Explicit Receivers

### Resolve an explicit receiver type

An explicit receiver annotation resolves like any other nominal type reference.

```ds main.ds
class Counter {}
^ declaration:counter:start
      ^^^^^^^ definition:counter
               ^ declaration:counter:end

function read(this: Counter): Counter {
                    ^^^^^^^ reference:counter
    return this;
}
```

```query goto_type_definition main.ds#reference:counter
@goto_type_definition.target origin=main.ds#reference:counter location=main.ds#declaration:counter selection=main.ds#definition:counter symbol=main.ds#Counter@1
```

## Generic Applications

### Resolve an applied nominal type

A generic application resolves to its nominal declaration.

```ds main.ds
class Box<T> {
^ declaration:box:start
      ^^^ definition:box
    value: T;
}
^ declaration:box:end

declare const box: Box<int32>;
                   ^^^ reference:box
```

```query goto_type_definition main.ds#reference:box
@goto_type_definition.target origin=main.ds#reference:box location=main.ds#declaration:box selection=main.ds#definition:box symbol=main.ds#Box@1
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
