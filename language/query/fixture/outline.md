
## Declarations

### Outline top-level declarations

The outline follows declaration order and source ranges.

```tspp main.tspp
struct Point {
^ point_range:start
       ^^^^^ point_selection
    x: float32;
    ^^^^^^^^^^ x_range
    ^ x_selection
    y: float32;
    ^^^^^^^^^^ y_range
    ^ y_selection
}
^ point_range:end

function add(left: int32, right: int32): int32 {
^ add_range:start
         ^^^ add_selection
    return left + right;
}
^ add_range:end

class Animal {
^ animal_range:start
      ^^^^^^ animal_selection
    name: string;
    ^^^^^^^^^^^^ name_range
    ^^^^ name_selection
}
^ animal_range:end

const answer = 42;
^^^^^^^^^^^^^^^^^ answer_range
      ^^^^^^ answer_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=Point kind=struct range=main.tspp#point_range selection=main.tspp#point_selection
@outline.symbol depth=1 name=x kind=field detail=float32 range=main.tspp#x_range selection=main.tspp#x_selection
@outline.symbol depth=1 name=y kind=field detail=float32 range=main.tspp#y_range selection=main.tspp#y_selection
@outline.symbol depth=0 name=add kind=function detail="(left: int32, right: int32): int32" range=main.tspp#add_range selection=main.tspp#add_selection
@outline.symbol depth=0 name=Animal kind=class range=main.tspp#animal_range selection=main.tspp#animal_selection
@outline.symbol depth=1 name=name kind=field detail=string range=main.tspp#name_range selection=main.tspp#name_selection
@outline.symbol depth=0 name=answer kind=constant detail=42 range=main.tspp#answer_range selection=main.tspp#answer_selection
```

### Render unresolved class field types

An unresolved field type keeps its class and renders as `<error>`.

```tspp main.tspp
class Player {
^ player_range:start
      ^^^^^^ player_selection
    x: string;
    ^^^^^^^^^ field_range
    ^ field_selection
}
^ player_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=Player kind=class range=main.tspp#player_range selection=main.tspp#player_selection
@outline.symbol depth=1 name=x kind=field detail=string range=main.tspp#field_range selection=main.tspp#field_selection
```

```diff main.tspp
@@ -1,8 +1,8 @@
 class Player {
 ^ player_range:start
       ^^^^^^ player_selection
-    x: string;
-    ^^^^^^^^^ field_range
+    x: Missing;
+    ^^^^^^^^^^ field_range
     ^ field_selection
 }
 ^ player_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=Player kind=class range=main.tspp#player_range selection=main.tspp#player_selection
@outline.symbol depth=1 name=x kind=field detail="<error>" range=main.tspp#field_range selection=main.tspp#field_selection
```

### Render unresolved initializer types

An unresolved initializer keeps its declaration and renders as `<error>`.

```tspp main.tspp
const value = 1;
^^^^^^^^^^^^^^^ value_range
      ^^^^^ value_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=value kind=constant detail=1 range=main.tspp#value_range selection=main.tspp#value_selection
```

```diff main.tspp
@@ -1,3 +1,3 @@
-const value = 1;
-^^^^^^^^^^^^^^^ value_range
+const value = missing;
+^^^^^^^^^^^^^^^^^^^^^ value_range
       ^^^^^ value_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=value kind=constant detail="<error>" range=main.tspp#value_range selection=main.tspp#value_selection
```

### Outline an incomplete declaration

Show the outline after every inserted character.

```tspp main.tspp
// module
```

```tspp main.tspp type
// module

declare const x: Clone;
^^^^^^^^^^^^^^^^^^^^^^ declaration
              ^ binding
```

```query outline main.tspp
@outline.symbol depth=0 name=x kind=constant detail=Clone range=main.tspp#declaration selection=main.tspp#binding
```

### Outline current declarations

The outline follows declarations added by each edit.

```tspp main.tspp
function ping(): void {
^ ping_range:start
         ^^^^ ping_selection
}
^ ping_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=ping kind=function detail="(): void" range=main.tspp#ping_range selection=main.tspp#ping_selection
```

```tspp main.tspp change
function ping(): void {
^ ping_range:start
         ^^^^ ping_selection
}
^ ping_range:end

const answer = 42;
^^^^^^^^^^^^^^^^^ answer_range
      ^^^^^^ answer_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=ping kind=function detail="(): void" range=main.tspp#ping_range selection=main.tspp#ping_selection
@outline.symbol depth=0 name=answer kind=constant detail=42 range=main.tspp#answer_range selection=main.tspp#answer_selection
```

### Outline a statically absent declaration

A false static gate remains in the authored outline without type detail.

```tspp main.tspp
struct Position {
^ position_range:start
       ^^^^^^^^ position_selection
    x: float64;
    ^^^^^^^^^^ x_range
    ^ x_selection
    y: float64;
    ^^^^^^^^^^ y_range
    ^ y_selection
}
^ position_range:end

const position = Position { x: 1.0, y: 2.0 };
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ value_range
      ^^^^^^^^ value_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=Position kind=struct range=main.tspp#position_range selection=main.tspp#position_selection
@outline.symbol depth=1 name=x kind=field detail=float64 range=main.tspp#x_range selection=main.tspp#x_selection
@outline.symbol depth=1 name=y kind=field detail=float64 range=main.tspp#y_range selection=main.tspp#y_selection
@outline.symbol depth=0 name=position kind=constant detail=Position range=main.tspp#value_range selection=main.tspp#value_selection
```

```diff main.tspp
@@ -13,3 +13,8 @@
 const position = Position { x: 1.0, y: 2.0 };
 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ value_range
       ^^^^^^^^ value_selection
+
+@if(false)
+const position = 5;
+^^^^^^^^^^^^^^^^^^ absent_range
+      ^^^^^^^^ absent_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=Position kind=struct range=main.tspp#position_range selection=main.tspp#position_selection
@outline.symbol depth=1 name=x kind=field detail=float64 range=main.tspp#x_range selection=main.tspp#x_selection
@outline.symbol depth=1 name=y kind=field detail=float64 range=main.tspp#y_range selection=main.tspp#y_selection
@outline.symbol depth=0 name=position kind=constant detail=Position range=main.tspp#value_range selection=main.tspp#value_selection
@outline.symbol depth=0 name=position kind=constant range=main.tspp#absent_range selection=main.tspp#absent_selection
```

## Members

### Preserve hierarchical member order

Fields and methods immediately follow their owner.

```tspp main.tspp
struct Rectangle {
^ rectangle_range:start
       ^^^^^^^^^ rectangle_selection
    width: float32;
    ^^^^^^^^^^^^^^ width_range
    ^^^^^ width_selection
    height: float32;
    ^^^^^^^^^^^^^^^ height_range
    ^^^^^^ height_selection

    area(): float32 {
    ^ area_range:start
    ^^^^ area_selection
        return this.width * this.height;
    }
    ^ area_range:end
}
^ rectangle_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=Rectangle kind=struct range=main.tspp#rectangle_range selection=main.tspp#rectangle_selection
@outline.symbol depth=1 name=width kind=field detail=float32 range=main.tspp#width_range selection=main.tspp#width_selection
@outline.symbol depth=1 name=height kind=field detail=float32 range=main.tspp#height_range selection=main.tspp#height_selection
@outline.symbol depth=1 name=area kind=method detail="(): float32" range=main.tspp#area_range selection=main.tspp#area_selection
```

### Distinguish fields, constructors, accessors, and methods

Fields, constructors, accessors, and methods use their matching kinds and signatures.

```tspp main.tspp
class Counter {
^ counter_range:start
      ^^^^^^^ counter_selection
    static readonly zero: int32 = 0;
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ zero_range
                    ^^^^ zero_selection
    value: int32;
    ^^^^^^^^^^^^ value_range
    ^^^^^ value_selection

    constructor(value: int32) {
    ^ constructor_range:start
    ^^^^^^^^^^^ constructor_selection
        this.value = value;
    }
    ^ constructor_range:end

    get current(): int32 {
    ^ current_get_range:start
        ^^^^^^^ current_get_selection
        return this.value;
    }
    ^ current_get_range:end

    set current(next: int32) {
    ^ current_set_range:start
        ^^^^^^^ current_set_selection
        this.value = next;
    }
    ^ current_set_range:end

    static create(): Counter {
    ^ create_range:start
           ^^^^^^ create_selection
        return new Counter(0);
    }
    ^ create_range:end
}
^ counter_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=Counter kind=class range=main.tspp#counter_range selection=main.tspp#counter_selection
@outline.symbol depth=1 name=zero kind=field detail="static readonly int32" range=main.tspp#zero_range selection=main.tspp#zero_selection
@outline.symbol depth=1 name=value kind=field detail=int32 range=main.tspp#value_range selection=main.tspp#value_selection
@outline.symbol depth=1 name=constructor kind=constructor detail="(value: int32)" range=main.tspp#constructor_range selection=main.tspp#constructor_selection
@outline.symbol depth=1 name=current kind=property detail="get (): int32" range=main.tspp#current_get_range selection=main.tspp#current_get_selection
@outline.symbol depth=1 name=current kind=property detail="set (next: int32): void" range=main.tspp#current_set_range selection=main.tspp#current_set_selection
@outline.symbol depth=1 name=create kind=method detail="static (): Counter" range=main.tspp#create_range selection=main.tspp#create_selection
```

### Outline a method with a compile-time default

Generic method details include their complete parameter header.

```tspp main.tspp
type Access = "readonly" | "exclusive";

struct List<T> {
    value: T;
}

extension<T> of List<T> {
^ extension_range:start
    read<const A: Access = "readonly">(): T {
    ^ read_range:start
    ^^^^ read_selection
        return this.value;
    }
    ^ read_range:end
}
^ extension_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=Access kind=type_alias detail="\"readonly\" | \"exclusive\"" range=main.tspp:1:1-1:39 selection=main.tspp:1:6-1:12
@outline.symbol depth=0 name=List kind=struct range=main.tspp:3:1-5:2 selection=main.tspp:3:8-3:12
@outline.symbol depth=1 name=value kind=field detail=T range=main.tspp:4:5-4:13 selection=main.tspp:4:5-4:10
@outline.symbol depth=0 name="extension of List<T>" kind=extension range=main.tspp#extension_range selection=main.tspp:7:17-7:24
@outline.symbol depth=1 name=read kind=method detail="<const A: Access = \"readonly\">(): T" range=main.tspp#read_range selection=main.tspp#read_selection
```

## Enum Members

### Outline enum members

Enum members remain children of their enum.

```tspp main.tspp
enum Color {
^ color_range:start
     ^^^^^ color_selection
    Red,
    ^^^ red
    Green,
    ^^^^^ green
    Blue,
    ^^^^ blue
}
^ color_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=Color kind=enum range=main.tspp#color_range selection=main.tspp#color_selection
@outline.symbol depth=1 name=Red kind=enum_member range=main.tspp#red selection=main.tspp#red
@outline.symbol depth=1 name=Green kind=enum_member range=main.tspp#green selection=main.tspp#green
@outline.symbol depth=1 name=Blue kind=enum_member range=main.tspp#blue selection=main.tspp#blue
```

## Empty Modules

### Return an explicit empty outline

An empty module has no outline entries.

```tspp main.tspp
```

```query outline main.tspp
@outline.none
```

## Declaration Blocks

### Preserve module and global declaration ownership

Module metadata and global declarations remain grouped under their declaration owners.

```tspp main.tspp
module {
^ module_range:start
^^^^^^ module_selection
    const role = "editor";
    ^^^^^^^^^^^^^^^^^^^^^ role_range
          ^^^^ role_selection
}
^ module_range:end

global {
^ global_range:start
^^^^^^ global_selection
    const version: int32 = 1;
    ^^^^^^^^^^^^^^^^^^^^^^^^ version_range
          ^^^^^^^ version_selection
}
^ global_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=module kind=module range=main.tspp#module_range selection=main.tspp#module_selection
@outline.symbol depth=1 name=role kind=constant detail="\"editor\"" range=main.tspp#role_range selection=main.tspp#role_selection
@outline.symbol depth=0 name=global kind=namespace range=main.tspp#global_range selection=main.tspp#global_selection
@outline.symbol depth=1 name=version kind=constant detail=int32 range=main.tspp#version_range selection=main.tspp#version_selection
```

## Types

### Outline structural types

Interfaces contain their members, and type aliases remain top-level symbols.

```tspp main.tspp
interface Drawable {
^ drawable_range:start
          ^^^^^^^^ drawable_selection
    draw(): void;
    ^^^^^^^^^^^^ draw_range
    ^^^^ draw_selection
}
^ drawable_range:end

type UserId = string;
^^^^^^^^^^^^^^^^^^^^ type_range
     ^^^^^^ type_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=Drawable kind=interface range=main.tspp#drawable_range selection=main.tspp#drawable_selection
@outline.symbol depth=1 name=draw kind=method detail="(): void" range=main.tspp#draw_range selection=main.tspp#draw_selection
@outline.symbol depth=0 name=UserId kind=type_alias detail=string range=main.tspp#type_range selection=main.tspp#type_selection
```

### Outline an intrinsic newtype

The outline shows the authored intrinsic value.

```tspp main.tspp
newtype Buffer<T> = intrinsic;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ buffer_range
        ^^^^^^ buffer_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=Buffer kind=newtype detail=intrinsic range=main.tspp#buffer_range selection=main.tspp#buffer_selection
```

### Outline nominal types and extensions

Newtypes, nominal interfaces, and named extensions use distinct kinds.

```tspp main.tspp
newtype UserId = int64;
^^^^^^^^^^^^^^^^^^^^^^ user_id_range
        ^^^^^^ user_id_selection

newtype interface Measure {
^ measure_range:start
                  ^^^^^^^ measure_selection
    abstract type Unit;
    ^^^^^^^^^^^^^^^^^^ unit_range
                  ^^^^ unit_selection
    abstract const Scale: uint;
    ^^^^^^^^^^^^^^^^^^^^^^^^^^ scale_range
                   ^^^^^ scale_selection
    measure(): float64;
    ^^^^^^^^^^^^^^^^^^ measure_method_range
    ^^^^^^^ measure_method_selection
}
^ measure_range:end

extension Integer of int32 {
^ extension_range:start
          ^^^^^^^ extension_selection
    doubled(): int32 {
    ^ doubled_range:start
    ^^^^^^^ doubled_selection
        return this + this;
    }
    ^ doubled_range:end
}
^ extension_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name=UserId kind=newtype detail=int64 range=main.tspp#user_id_range selection=main.tspp#user_id_selection
@outline.symbol depth=0 name=Measure kind=newtype_interface range=main.tspp#measure_range selection=main.tspp#measure_selection
@outline.symbol depth=1 name=Unit kind=associated_type range=main.tspp#unit_range selection=main.tspp#unit_selection
@outline.symbol depth=1 name=Scale kind=associated_const detail=uint64 range=main.tspp#scale_range selection=main.tspp#scale_selection
@outline.symbol depth=1 name=measure kind=method detail="(): float64" range=main.tspp#measure_method_range selection=main.tspp#measure_method_selection
@outline.symbol depth=0 name=Integer kind=extension range=main.tspp#extension_range selection=main.tspp#extension_selection
@outline.symbol depth=1 name=doubled kind=method detail="(): int32" range=main.tspp#doubled_range selection=main.tspp#doubled_selection
```

## Overloads

### Preserve overload declarations

Each overload remains a separate symbol in source order.

```tspp main.tspp
declare function parse(value: string): int32;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ string_range
                 ^^^^^ string_selection
declare function parse(value: int32): int32;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ integer_range
                 ^^^^^ integer_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=parse kind=function detail="(value: string): int32" range=main.tspp#string_range selection=main.tspp#string_selection
@outline.symbol depth=0 name=parse kind=function detail="(value: int32): int32" range=main.tspp#integer_range selection=main.tspp#integer_selection
```

## Anonymous Owners

### Name an anonymous extension by its target

An anonymous extension remains visible without inventing a symbol identity.

```tspp main.tspp
extension of int32 {
^ extension_range:start
             ^^^^^ target_selection
    doubled(): int32 {
    ^ doubled_range:start
    ^^^^^^^ doubled_selection
        return this + this;
    }
    ^ doubled_range:end
}
^ extension_range:end
```

```query outline main.tspp
@outline.symbol depth=0 name="extension of int32" kind=extension range=main.tspp#extension_range selection=main.tspp#target_selection
@outline.symbol depth=1 name=doubled kind=method detail="(): int32" range=main.tspp#doubled_range selection=main.tspp#doubled_selection
```

## Bindings

### Outline top-level bindings

Each top-level binding is a symbol, including bindings introduced by one destructuring declaration.

```tspp main.tspp
const pair = { left: 1, right: 2 };
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ pair_range
      ^^^^ pair_selection

const { left, right: vertical } = pair;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ bindings_range
        ^^^^ left_selection
                     ^^^^^^^^ vertical_selection

let count = 0;
^^^^^^^^^^^^^ count_range
    ^^^^^ count_selection
```

```query outline main.tspp
@outline.symbol depth=0 name=pair kind=constant detail="{ left: int64; right: int64 }" range=main.tspp#pair_range selection=main.tspp#pair_selection
@outline.symbol depth=0 name=left kind=constant detail=int64 range=main.tspp#bindings_range selection=main.tspp#left_selection
@outline.symbol depth=0 name=vertical kind=constant detail=int64 range=main.tspp#bindings_range selection=main.tspp#vertical_selection
@outline.symbol depth=0 name=count kind=variable detail=int64 range=main.tspp#count_range selection=main.tspp#count_selection
```

## Omitted Symbols

### Omit imports and local bindings

The outline includes document declarations but not dependencies, parameters, or local bindings.

```tspp main.tspp
import { source } from "./library.tspp";

function read(value: int32): int32 {
^ read_range:start
         ^^^^ read_selection
    const local = value;
    return local;
}
^ read_range:end

const exposed = 1;
^^^^^^^^^^^^^^^^^ exposed_range
      ^^^^^^^ exposed_selection
```

```tspp library.tspp
export const source = 1;
```

```query outline main.tspp
@outline.symbol depth=0 name=read kind=function detail="(value: int32): int32" range=main.tspp#read_range selection=main.tspp#read_selection
@outline.symbol depth=0 name=exposed kind=constant detail=1 range=main.tspp#exposed_range selection=main.tspp#exposed_selection
```
