
## Nominal Values

### Resolve a local nominal value

A value resolves to its nominal type declaration.

```tspp main.tspp
struct Point {
^ declaration:start
       ^^^^^ definition
    x: int32;
}
^ declaration:end

const point = Point { x: 1 };
      ^^^^^ reference
```

```query goto_type_definition main.tspp#reference
@goto_type_definition.target origin=main.tspp#reference location=main.tspp#declaration selection=main.tspp#definition symbol=main.tspp#Point@1
```

### Resolve a class-valued binding

A binding with a class type resolves to the class declaration.

```tspp main.tspp
class Widget {
^ declaration:widget:start
      ^^^^^^ definition:widget
}
^ declaration:widget:end

declare const widget: Widget;
              ^^^^^^ reference:widget
```

```query goto_type_definition main.tspp#reference:widget
@goto_type_definition.target origin=main.tspp#reference:widget location=main.tspp#declaration:widget selection=main.tspp#definition:widget symbol=main.tspp#Widget@1
```

### Resolve a typed parameter

A parameter resolves through its declared type.

```tspp main.tspp
struct Config {
^ declaration:config:start
       ^^^^^^ definition:config
    enabled: boolean;
}
^ declaration:config:end

function inspect(config: Config): void {}
                 ^^^^^^ reference:config
```

```query goto_type_definition main.tspp#reference:config
@goto_type_definition.target origin=main.tspp#reference:config location=main.tspp#declaration:config selection=main.tspp#definition:config symbol=main.tspp#Config@1
```

### Resolve an enum-valued binding

An enum value resolves to the enum declaration rather than one variant.

```tspp main.tspp
enum Color {
^ declaration:color:start
     ^^^^^ definition:color
    Red,
}
^ declaration:color:end

const color: Color = Color.Red;
      ^^^^^ reference:color
```

```query goto_type_definition main.tspp#reference:color
@goto_type_definition.target origin=main.tspp#reference:color location=main.tspp#declaration:color selection=main.tspp#definition:color symbol=main.tspp#Color@1
```

### Resolve the current value type

A value resolves to its nominal type after each edit.

```tspp main.tspp
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

```query goto_type_definition main.tspp#reference
@goto_type_definition.target origin=main.tspp#reference location=main.tspp#declaration:first selection=main.tspp#definition:first symbol=main.tspp#First@1
```

```tspp main.tspp change
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

```query goto_type_definition main.tspp#reference
@goto_type_definition.target origin=main.tspp#reference location=main.tspp#declaration:second selection=main.tspp#definition:second symbol=main.tspp#Second@2
```

## Type References

### Resolve primitive runtime declarations

Primitive types resolve to their runtime declarations.

```tspp main.tspp
type Text = string;
            ^^^^^^ reference:string

type Integer = bigint;
               ^^^^^^ reference:bigint
```

```query goto_type_definition main.tspp#reference:string
@goto_type_definition.target origin=main.tspp#reference:string location=tspp://string/string:16:1-19:2 selection=tspp://string/string:16:14-16:20 symbol=tspp://string/string#String@28
```

```query goto_type_definition main.tspp#reference:bigint
@goto_type_definition.target origin=main.tspp#reference:bigint location=tspp://math/bigint:27:1-36:2 selection=tspp://math/bigint:27:14-27:20 symbol=tspp://math/bigint#BigInt@26
```

### Resolve a direct type reference

A nominal name in type position resolves to its declaration.

```tspp main.tspp
class Animal {}
^ declaration:animal:start
      ^^^^^^ definition:animal
              ^ declaration:animal:end

declare const animal: Animal;
                      ^^^^^^ reference:animal
```

```query goto_type_definition main.tspp#reference:animal
@goto_type_definition.target origin=main.tspp#reference:animal location=main.tspp#declaration:animal selection=main.tspp#definition:animal symbol=main.tspp#Animal@1
```

### Resolve a local type alias

A type alias reference resolves to the alias declaration.

```tspp main.tspp
type UserId = int32;
^ declaration:user_id:start
     ^^^^^^ definition:user_id
                  ^ declaration:user_id:end

declare const userId: UserId;
                      ^^^^^^ reference:user_id
```

```query goto_type_definition main.tspp#reference:user_id
@goto_type_definition.target origin=main.tspp#reference:user_id location=main.tspp#declaration:user_id selection=main.tspp#definition:user_id symbol=main.tspp#UserId@1
```

## Imports

### Resolve an imported nominal value

An imported value follows its type into the defining module.

```tspp model.tspp
export struct Widget {
^ declaration:start
              ^^^^^^ definition
    value: int32;
}
^ declaration:end
```

```tspp main.tspp
import { Widget } from "./model.tspp";

const widget = Widget { value: 1 };
      ^^^^^^ reference
```

```query goto_type_definition main.tspp#reference
@goto_type_definition.target origin=main.tspp#reference location=model.tspp#declaration selection=model.tspp#definition symbol=model.tspp#Widget@1
```

## Scalar Values

### Return no nominal type for a scalar

A scalar value has no nominal type declaration.

```tspp main.tspp
const count = 1;
      ^^^^^ reference
```

```query goto_type_definition main.tspp#reference
@goto_type_definition.none
```

## Union Types

### Resolve every nominal member of a union

A binding with an unnamed union type resolves to each distinct nominal declaration.

```tspp main.tspp
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

```query goto_type_definition main.tspp#reference
@goto_type_definition.target origin=main.tspp#reference location=main.tspp#declaration:circle selection=main.tspp#definition:circle symbol=main.tspp#Circle@1
@goto_type_definition.target origin=main.tspp#reference location=main.tspp#declaration:square selection=main.tspp#definition:square symbol=main.tspp#Square@2
```

### Resolve the nominal member of a nullable union

Null has no definition target, while the nominal member resolves to its declaration.

```tspp main.tspp
class User {}
^ declaration:user:start
      ^^^^ definition:user
            ^ declaration:user:end

declare const user: User | null;
              ^^^^ reference
```

```query goto_type_definition main.tspp#reference
@goto_type_definition.target origin=main.tspp#reference location=main.tspp#declaration:user selection=main.tspp#definition:user symbol=main.tspp#User@1
```

## Imported Types

### Resolve an imported type

A plain import preserves the exported declaration's type symbol space.

```tspp model.tspp
export type Options = {
^ declaration:imported_type:start
            ^^^^^^^ definition:imported_type
    enabled: boolean;
};
^ declaration:imported_type:end
```

```tspp main.tspp
import { Options } from "./model.tspp";

const options: Options = { enabled: true };
               ^^^^^^^ reference:imported_type
```

```query goto_type_definition main.tspp#reference:imported_type
@goto_type_definition.target origin=main.tspp#reference:imported_type location=model.tspp#declaration:imported_type selection=model.tspp#definition:imported_type symbol=model.tspp#Options@1
```

### Resolve an imported type alias

An imported alias resolves to the defining type declaration.

```tspp model.tspp
export struct Settings {
^ declaration:settings:start
              ^^^^^^^^ definition:settings
    enabled: boolean;
}
^ declaration:settings:end
```

```tspp main.tspp
import { Settings as AppSettings } from "./model.tspp";

declare const settings: AppSettings;
                        ^^^^^^^^^^^ reference:settings
```

```query goto_type_definition main.tspp#reference:settings
@goto_type_definition.target origin=main.tspp#reference:settings location=model.tspp#declaration:settings selection=model.tspp#definition:settings symbol=model.tspp#Settings@1
```

### Resolve imported types and values together

One plain import preserves each declaration's original symbol space.

```tspp types.tspp
export struct Settings {
^ declaration:mixed:start
              ^^^^^^^^ definition:mixed
    enabled: boolean;
}
^ declaration:mixed:end
```

```tspp values.tspp
export function enabled(): boolean {
    return true;
}
```

```tspp main.tspp
import { Settings } from "./types.tspp";
import { enabled } from "./values.tspp";

declare const settings: Settings;
                        ^^^^^^^^ reference:mixed
const isEnabled = enabled();
```

```query goto_type_definition main.tspp#reference:mixed
@goto_type_definition.target origin=main.tspp#reference:mixed location=types.tspp#declaration:mixed selection=types.tspp#definition:mixed symbol=types.tspp#Settings@1
```

### Follow re-exports

A type reference follows every plain re-export to its declaration.

```tspp model.tspp
export interface ServiceOptions {
^ declaration:options:start
                 ^^^^^^^^^^^^^^ definition:options
    enabled: boolean;
}
^ declaration:options:end
```

```tspp first.tspp
export { ServiceOptions as Options } from "./model.tspp";
```

```tspp second.tspp
export { Options } from "./first.tspp";
```

```tspp main.tspp
import { Options } from "./second.tspp";

declare const options: Options;
                       ^^^^^^^ reference:options
```

```query goto_type_definition main.tspp#reference:options
@goto_type_definition.target origin=main.tspp#reference:options location=model.tspp#declaration:options selection=model.tspp#definition:options symbol=model.tspp#ServiceOptions@1
```

## Import Forms

### Resolve a namespace type member

A type member on a namespace import resolves to the exported declaration.

```tspp model.tspp
export struct Settings {
^ declaration:namespace:start
              ^^^^^^^^ definition:namespace
    enabled: boolean;
}
^ declaration:namespace:end
```

```tspp main.tspp
import * as models from "./model.tspp";

declare const settings: models.Settings;
                               ^^^^^^^^ reference:namespace
```

```query goto_type_definition main.tspp#reference:namespace
@goto_type_definition.target origin=main.tspp#reference:namespace location=model.tspp#declaration:namespace selection=model.tspp#definition:namespace symbol=model.tspp#Settings@1
```

### Resolve a default class import

A default import resolves to the defining class declaration.

```tspp model.tspp
export default class Widget {
^ declaration:default:start
                     ^^^^^^ definition:default
}
^ declaration:default:end
```

```tspp main.tspp
import WidgetModel from "./model.tspp";

declare const widget: WidgetModel;
                      ^^^^^^^^^^^ reference:default
```

```query goto_type_definition main.tspp#reference:default
@goto_type_definition.target origin=main.tspp#reference:default location=model.tspp#declaration:default selection=model.tspp#definition:default symbol=model.tspp#Widget@1
```

### Follow a default re-export alias

A named import follows a default re-export to the defining class.

```tspp model.tspp
export default class Widget {
^ declaration:reexport:start
                     ^^^^^^ definition:reexport
}
^ declaration:reexport:end
```

```tspp barrel.tspp
export { default as Widget } from "./model.tspp";
```

```tspp main.tspp
import { Widget } from "./barrel.tspp";

declare const widget: Widget;
                      ^^^^^^ reference:reexport
```

```query goto_type_definition main.tspp#reference:reexport
@goto_type_definition.target origin=main.tspp#reference:reexport location=model.tspp#declaration:reexport selection=model.tspp#definition:reexport symbol=model.tspp#Widget@1
```

## Ownership Forms

### Resolve borrowed, owned, and pointer values

Memory forms resolve to the nominal declaration of their contained value.

```tspp main.tspp
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

```query goto_type_definition main.tspp#reference:borrowed
@goto_type_definition.target origin=main.tspp#reference:borrowed location=main.tspp#declaration:buffer selection=main.tspp#definition:buffer symbol=main.tspp#Buffer@1
```

```query goto_type_definition main.tspp#reference:owned
@goto_type_definition.target origin=main.tspp#reference:owned location=main.tspp#declaration:buffer selection=main.tspp#definition:buffer symbol=main.tspp#Buffer@1
```

```query goto_type_definition main.tspp#reference:pointer
@goto_type_definition.target origin=main.tspp#reference:pointer location=main.tspp#declaration:buffer selection=main.tspp#definition:buffer symbol=main.tspp#Buffer@1
```

## Explicit Receivers

### Resolve an explicit receiver type

An explicit receiver annotation resolves like any other nominal type reference.

```tspp main.tspp
class Counter {}
^ declaration:counter:start
      ^^^^^^^ definition:counter
               ^ declaration:counter:end

function read(this: Counter): Counter {
                    ^^^^^^^ reference:counter
    return this;
}
```

```query goto_type_definition main.tspp#reference:counter
@goto_type_definition.target origin=main.tspp#reference:counter location=main.tspp#declaration:counter selection=main.tspp#definition:counter symbol=main.tspp#Counter@1
```

## Generic Applications

### Resolve an applied nominal type

A generic application resolves to its nominal declaration.

```tspp main.tspp
class Box<T> {
^ declaration:box:start
      ^^^ definition:box
    value: T;
}
^ declaration:box:end

declare const box: Box<int32>;
                   ^^^ reference:box
```

```query goto_type_definition main.tspp#reference:box
@goto_type_definition.target origin=main.tspp#reference:box location=main.tspp#declaration:box selection=main.tspp#definition:box symbol=main.tspp#Box@1
```

## Missing Symbols

### Return no type definition for an unresolved name

An unresolved occurrence has no type declaration.

```tspp main.tspp
function main(): void {
    missingValue;
    ^^^^^^^^^^^^ reference
}
```

```query goto_type_definition main.tspp#reference
@goto_type_definition.none
```
