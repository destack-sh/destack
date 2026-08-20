
## Declarations

### Outline top-level declarations

The outline follows declaration order and source ranges.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Point kind=struct range=main.ds#point_range selection=main.ds#point_selection
@outline.symbol depth=1 name=x kind=field detail=float32 range=main.ds#x_range selection=main.ds#x_selection
@outline.symbol depth=1 name=y kind=field detail=float32 range=main.ds#y_range selection=main.ds#y_selection
@outline.symbol depth=0 name=add kind=function detail="(left: int32, right: int32): int32" range=main.ds#add_range selection=main.ds#add_selection
@outline.symbol depth=0 name=Animal kind=class range=main.ds#animal_range selection=main.ds#animal_selection
@outline.symbol depth=1 name=name kind=field detail=string range=main.ds#name_range selection=main.ds#name_selection
@outline.symbol depth=0 name=answer kind=constant detail=42 range=main.ds#answer_range selection=main.ds#answer_selection
```

### Render unresolved class field types

An unresolved field type keeps its class and renders as `<error>`.

```ds main.ds
class Player {
^ player_range:start
      ^^^^^^ player_selection
    x: string;
    ^^^^^^^^^ field_range
    ^ field_selection
}
^ player_range:end
```

```query outline main.ds
@outline.symbol depth=0 name=Player kind=class range=main.ds#player_range selection=main.ds#player_selection
@outline.symbol depth=1 name=x kind=field detail=string range=main.ds#field_range selection=main.ds#field_selection
```

```diff main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Player kind=class range=main.ds#player_range selection=main.ds#player_selection
@outline.symbol depth=1 name=x kind=field detail="<error>" range=main.ds#field_range selection=main.ds#field_selection
```

### Render unresolved initializer types

An unresolved initializer keeps its declaration and renders as `<error>`.

```ds main.ds
const value = 1;
^^^^^^^^^^^^^^^ value_range
      ^^^^^ value_selection
```

```query outline main.ds
@outline.symbol depth=0 name=value kind=constant detail=1 range=main.ds#value_range selection=main.ds#value_selection
```

```diff main.ds
@@ -1,3 +1,3 @@
-const value = 1;
-^^^^^^^^^^^^^^^ value_range
+const value = missing;
+^^^^^^^^^^^^^^^^^^^^^ value_range
       ^^^^^ value_selection
```

```query outline main.ds
@outline.symbol depth=0 name=value kind=constant detail="<error>" range=main.ds#value_range selection=main.ds#value_selection
```

## Members

### Preserve hierarchical member order

Fields and methods immediately follow their owner.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Rectangle kind=struct range=main.ds#rectangle_range selection=main.ds#rectangle_selection
@outline.symbol depth=1 name=width kind=field detail=float32 range=main.ds#width_range selection=main.ds#width_selection
@outline.symbol depth=1 name=height kind=field detail=float32 range=main.ds#height_range selection=main.ds#height_selection
@outline.symbol depth=1 name=area kind=method detail="(): float32" range=main.ds#area_range selection=main.ds#area_selection
```

### Distinguish member roles

Constructors, accessors, and static members use their distinct roles and signatures.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Counter kind=class range=main.ds#counter_range selection=main.ds#counter_selection
@outline.symbol depth=1 name=zero kind=field detail="static readonly int32" range=main.ds#zero_range selection=main.ds#zero_selection
@outline.symbol depth=1 name=value kind=field detail=int32 range=main.ds#value_range selection=main.ds#value_selection
@outline.symbol depth=1 name=constructor kind=constructor detail="(value: int32)" range=main.ds#constructor_range selection=main.ds#constructor_selection
@outline.symbol depth=1 name=current kind=property detail="get (): int32" range=main.ds#current_get_range selection=main.ds#current_get_selection
@outline.symbol depth=1 name=current kind=property detail="set (next: int32): void" range=main.ds#current_set_range selection=main.ds#current_set_selection
@outline.symbol depth=1 name=create kind=method detail="static (): Counter" range=main.ds#create_range selection=main.ds#create_selection
```

### Outline a method with a compile-time default

Generic method details include their complete parameter header.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Access kind=type_alias detail="\"readonly\" | \"exclusive\"" range=main.ds:1:1-1:39 selection=main.ds:1:6-1:12
@outline.symbol depth=0 name=List kind=struct range=main.ds:3:1-5:2 selection=main.ds:3:8-3:12
@outline.symbol depth=1 name=value kind=field detail=T range=main.ds:4:5-4:13 selection=main.ds:4:5-4:10
@outline.symbol depth=0 name="extension of List<T>" kind=extension range=main.ds#extension_range selection=main.ds:7:17-7:24
@outline.symbol depth=1 name=read kind=method detail="<const A: Access = \"readonly\">(): T" range=main.ds#read_range selection=main.ds#read_selection
```

## Enum Members

### Outline enum members

Enum members remain children of their enum.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Color kind=enum range=main.ds#color_range selection=main.ds#color_selection
@outline.symbol depth=1 name=Red kind=enum_member range=main.ds#red selection=main.ds#red
@outline.symbol depth=1 name=Green kind=enum_member range=main.ds#green selection=main.ds#green
@outline.symbol depth=1 name=Blue kind=enum_member range=main.ds#blue selection=main.ds#blue
```

## Empty Modules

### Return an explicit empty outline

An empty module has no outline entries.

```ds main.ds
```

```query outline main.ds
@outline.none
```

## Declaration Blocks

### Preserve module and global declaration ownership

Module metadata and global declarations remain grouped under their declaration owners.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=module kind=module range=main.ds#module_range selection=main.ds#module_selection
@outline.symbol depth=1 name=role kind=constant detail="\"editor\"" range=main.ds#role_range selection=main.ds#role_selection
@outline.symbol depth=0 name=global kind=namespace range=main.ds#global_range selection=main.ds#global_selection
@outline.symbol depth=1 name=version kind=constant detail=int32 range=main.ds#version_range selection=main.ds#version_selection
```

## Types

### Outline structural types

Interfaces contain their members, and type aliases remain top-level symbols.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=Drawable kind=interface range=main.ds#drawable_range selection=main.ds#drawable_selection
@outline.symbol depth=1 name=draw kind=method detail="(): void" range=main.ds#draw_range selection=main.ds#draw_selection
@outline.symbol depth=0 name=UserId kind=type_alias detail=string range=main.ds#type_range selection=main.ds#type_selection
```

### Outline an intrinsic newtype

The outline shows the authored intrinsic value.

```ds main.ds
newtype Buffer<T> = intrinsic;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ buffer_range
        ^^^^^^ buffer_selection
```

```query outline main.ds
@outline.symbol depth=0 name=Buffer kind=newtype detail=intrinsic range=main.ds#buffer_range selection=main.ds#buffer_selection
```

### Outline nominal types and extensions

Newtypes, nominal interfaces, and named extensions use distinct kinds.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=UserId kind=newtype detail=int64 range=main.ds#user_id_range selection=main.ds#user_id_selection
@outline.symbol depth=0 name=Measure kind=newtype_interface range=main.ds#measure_range selection=main.ds#measure_selection
@outline.symbol depth=1 name=Unit kind=associated_type range=main.ds#unit_range selection=main.ds#unit_selection
@outline.symbol depth=1 name=Scale kind=associated_const detail=uint64 range=main.ds#scale_range selection=main.ds#scale_selection
@outline.symbol depth=1 name=measure kind=method detail="(): float64" range=main.ds#measure_method_range selection=main.ds#measure_method_selection
@outline.symbol depth=0 name=Integer kind=extension range=main.ds#extension_range selection=main.ds#extension_selection
@outline.symbol depth=1 name=doubled kind=method detail="(): int32" range=main.ds#doubled_range selection=main.ds#doubled_selection
```

## Overloads

### Preserve overload declarations

Each overload remains a separate entry in source order.

```ds main.ds
declare function parse(value: string): int32;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ string_range
                 ^^^^^ string_selection
declare function parse(value: int32): int32;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ integer_range
                 ^^^^^ integer_selection
```

```query outline main.ds
@outline.symbol depth=0 name=parse kind=function detail="(value: string): int32" range=main.ds#string_range selection=main.ds#string_selection
@outline.symbol depth=0 name=parse kind=function detail="(value: int32): int32" range=main.ds#integer_range selection=main.ds#integer_selection
```

## Anonymous Owners

### Name an anonymous extension by its target

An anonymous extension remains visible without inventing a symbol identity.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name="extension of int32" kind=extension range=main.ds#extension_range selection=main.ds#target_selection
@outline.symbol depth=1 name=doubled kind=method detail="(): int32" range=main.ds#doubled_range selection=main.ds#doubled_selection
```

## Bindings

### Outline top-level bindings

Each top-level binding is an entry, including bindings introduced by one destructuring declaration.

```ds main.ds
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

```query outline main.ds
@outline.symbol depth=0 name=pair kind=constant detail="{ left: float64; right: float64 }" range=main.ds#pair_range selection=main.ds#pair_selection
@outline.symbol depth=0 name=left kind=constant detail=float64 range=main.ds#bindings_range selection=main.ds#left_selection
@outline.symbol depth=0 name=vertical kind=constant detail=float64 range=main.ds#bindings_range selection=main.ds#vertical_selection
@outline.symbol depth=0 name=count kind=variable detail=float64 range=main.ds#count_range selection=main.ds#count_selection
```

## Omitted Symbols

### Omit imports and local bindings

The outline includes document declarations but not dependencies, parameters, or local bindings.

```ds main.ds
import { source } from "./library.ds";

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

```ds library.ds
export const source = 1;
```

```query outline main.ds
@outline.symbol depth=0 name=read kind=function detail="(value: int32): int32" range=main.ds#read_range selection=main.ds#read_selection
@outline.symbol depth=0 name=exposed kind=constant detail=1 range=main.ds#exposed_range selection=main.ds#exposed_selection
```

## Source changes

### Add declarations to the outline

The outline follows declarations added to the selected revision.

```ds main.ds
function ping(): void {
^ ping_range:start
         ^^^^ ping_selection
}
^ ping_range:end
```

```query outline main.ds
@outline.symbol depth=0 name=ping kind=function detail="(): void" range=main.ds#ping_range selection=main.ds#ping_selection
```

```ds main.ds change
function ping(): void {
^ ping_range:start
         ^^^^ ping_selection
}
^ ping_range:end

const answer = 42;
^^^^^^^^^^^^^^^^^ answer_range
      ^^^^^^ answer_selection
```

```query outline main.ds
@outline.symbol depth=0 name=ping kind=function detail="(): void" range=main.ds#ping_range selection=main.ds#ping_selection
@outline.symbol depth=0 name=answer kind=constant detail=42 range=main.ds#answer_range selection=main.ds#answer_selection
```
