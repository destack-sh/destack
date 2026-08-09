
## Interfaces

### Find implementations of an interface

An interface resolves to every nominal implementation.

```ds main.ds
interface Drawable {
          ^^^^^^^^ target:drawable
    draw(): void;
}

class Circle implements Drawable {
^ declaration:circle:start
      ^^^^^^ implementation:circle
    draw(): void {}
}
^ declaration:circle:end

struct Rectangle implements Drawable {
^ declaration:rectangle:start
       ^^^^^^^^^ implementation:rectangle
    draw(): void {}
}
^ declaration:rectangle:end
```

```query goto_implementation main.ds#target:drawable
@goto_implementation.target origin=main.ds#target:drawable location=main.ds#declaration:circle selection=main.ds#implementation:circle symbol=main.ds#Circle@4
@goto_implementation.target origin=main.ds#target:drawable location=main.ds#declaration:rectangle selection=main.ds#implementation:rectangle symbol=main.ds#Rectangle@7
```

## Nominal Interfaces

### Find every direct nominal implementation kind

A nominal interface resolves to every class, struct, enum, and extension declaration that implements it.

```ds main.ds
newtype interface Display {}
                  ^^^^^^^ target:display

class View implements Display {}
^ declaration:view:start
                                ^ declaration:view:end
      ^^^^ implementation:view

struct Packet implements Display {}
^ declaration:packet:start
                                   ^ declaration:packet:end
       ^^^^^^ implementation:packet

enum Status implements Display { Ready }
^ declaration:status:start
                                        ^ declaration:status:end
     ^^^^^^ implementation:status

newtype UserId = string;

extension of UserId implements Display {}
^ declaration:user_id:start
                                         ^ declaration:user_id:end
             ^^^^^^ implementation:user_id
```

```query goto_implementation main.ds#target:display
@goto_implementation.target origin=main.ds#target:display location=main.ds#declaration:view selection=main.ds#implementation:view symbol=main.ds#View@2
@goto_implementation.target origin=main.ds#target:display location=main.ds#declaration:packet selection=main.ds#implementation:packet symbol=main.ds#Packet@3
@goto_implementation.target origin=main.ds#target:display location=main.ds#declaration:status selection=main.ds#implementation:status symbol=main.ds#Status@4
@goto_implementation.target origin=main.ds#target:display location=main.ds#declaration:user_id selection=main.ds#implementation:user_id symbol=main.ds#symbol@7
```

### Return no targets for an unimplemented interface

An interface without implementations has no implementation target.

```ds main.ds
newtype interface Display {}
                  ^^^^^^^ target
```

```query goto_implementation main.ds#target
@goto_implementation.none
```

## Empty Results

### Return no implementations for functions

Functions are not implementation hierarchy targets.

```ds main.ds
function helper(): void {}
         ^^^^^^ target:helper
```

```query goto_implementation main.ds#target:helper
@goto_implementation.none
```

### Return no implementations for structs

Structs do not have nominal subtypes.

```ds main.ds
struct Point {
       ^^^^^ target:point
    x: int32;
}
```

```query goto_implementation main.ds#target:point
@goto_implementation.none
```

## Class Inheritance

### Find direct subclasses of a class

A class resolves to its direct subclasses.

```ds main.ds
class Base {
      ^^^^ target:base
    value: int32;
}

class Derived extends Base {
^ declaration:derived:start
      ^^^^^^^ implementation:derived
    value: int32;
}
^ declaration:derived:end

class SubDerived extends Derived {}
```

```query goto_implementation main.ds#target:base
@goto_implementation.target origin=main.ds#target:base location=main.ds#declaration:derived selection=main.ds#implementation:derived symbol=main.ds#Derived@4
```

## Methods

### Find implementations of an interface method

An interface method resolves to every member that implements it.

```ds main.ds
interface Renderable {
    render(): string;
    ^^^^^^ target
}

class View implements Renderable {
    render(): string {
    ^ declaration:view:start
    ^^^^^^ implementation:view
        return "";
    }
    ^ declaration:view:end
}

struct Document implements Renderable {
    render(): string {
    ^ declaration:document:start
    ^^^^^^ implementation:document
        return "";
    }
    ^ declaration:document:end
}
```

```query goto_implementation main.ds#target
@goto_implementation.target origin=main.ds#target location=main.ds#declaration:view selection=main.ds#implementation:view symbol=main.ds#render@5
@goto_implementation.target origin=main.ds#target location=main.ds#declaration:document selection=main.ds#implementation:document symbol=main.ds#render@8
```

### Find implementations of an associated type

An interface associated type resolves to every implementing associated declaration.

```ds main.ds
interface Container {
    type Item;
         ^^^^ target
}

class StringContainer implements Container {
    type Item = string;
    ^ declaration:start
         ^^^^ implementation
                     ^ declaration:end
}
```

```query goto_implementation main.ds#target
@goto_implementation.target origin=main.ds#target location=main.ds#declaration selection=main.ds#implementation symbol=main.ds#Item@5
```

```diff main.ds
@@ -6,6 +6,6 @@
 class StringContainer implements Container {
-    type Item = string;
+    type Item = int32;
     ^ declaration:start
          ^^^^ implementation
-                     ^ declaration:end
+                    ^ declaration:end
 }
```

```query goto_implementation main.ds#target
@goto_implementation.target origin=main.ds#target location=main.ds#declaration selection=main.ds#implementation symbol=main.ds#Item@5
```

### [ignored] Find overrides of a class method

An abstract class method resolves to every overriding member.

```ds main.ds
abstract class Writer {
    abstract write(value: string): void;
             ^^^^^ target
}

class FileWriter extends Writer {
    override write(value: string): void {}
    ^ declaration:start
                                         ^ declaration:end
             ^^^^^ implementation
}
```

```query goto_implementation main.ds#target
@goto_implementation.target origin=main.ds#target location=main.ds#declaration selection=main.ds#implementation symbol=main.ds#write@6
```

## Cross-Module Interfaces

### Find implementations across modules

Implementations can live in another module.

```ds library.ds
export interface Drawable {
                 ^^^^^^^^ target:drawable
    draw(): void;
    ^^^^ target:draw
}
```

```ds implementation.ds
import { Drawable } from "./library.ds";

export class Circle implements Drawable {
^ declaration:circle:start
             ^^^^^^ implementation:circle
    draw(): void {}
    ^ declaration:circle_draw:start
                   ^ declaration:circle_draw:end
    ^^^^ implementation:circle_draw
}
^ declaration:circle:end

export struct Square implements Drawable {
^ declaration:square:start
              ^^^^^^ implementation:square
    draw(): void {}
    ^ declaration:square_draw:start
                   ^ declaration:square_draw:end
    ^^^^ implementation:square_draw
}
^ declaration:square:end
```

```query goto_implementation library.ds#target:drawable
@goto_implementation.target origin=library.ds#target:drawable location=implementation.ds#declaration:circle selection=implementation.ds#implementation:circle symbol=implementation.ds#Circle@2
@goto_implementation.target origin=library.ds#target:drawable location=implementation.ds#declaration:square selection=implementation.ds#implementation:square symbol=implementation.ds#Square@5
```

```query goto_implementation library.ds#target:draw
@goto_implementation.target origin=library.ds#target:draw location=implementation.ds#declaration:circle_draw selection=implementation.ds#implementation:circle_draw symbol=implementation.ds#draw@3
@goto_implementation.target origin=library.ds#target:draw location=implementation.ds#declaration:square_draw selection=implementation.ds#implementation:square_draw symbol=implementation.ds#draw@6
```

### Find implementations through a re-exported interface

A re-export alias preserves the interface's implementation set.

```ds alias_library.ds
export interface Renderable {
                 ^^^^^^^^^^ target:renderable
    render(): void;
}
```

```ds alias_barrel.ds
export { Renderable } from "./alias_library.ds";
```

```ds alias_implementation.ds
import { Renderable } from "./alias_barrel.ds";

export class Sprite implements Renderable {
^ declaration:sprite:start
             ^^^^^^ implementation:sprite
    render(): void {}
}
^ declaration:sprite:end

export struct Icon implements Renderable {
^ declaration:icon:start
              ^^^^ implementation:icon
    render(): void {}
}
^ declaration:icon:end
```

```query goto_implementation alias_library.ds#target:renderable
@goto_implementation.target origin=alias_library.ds#target:renderable location=alias_implementation.ds#declaration:sprite selection=alias_implementation.ds#implementation:sprite symbol=alias_implementation.ds#Sprite@2
@goto_implementation.target origin=alias_library.ds#target:renderable location=alias_implementation.ds#declaration:icon selection=alias_implementation.ds#implementation:icon symbol=alias_implementation.ds#Icon@5
```

## Cross-Module Classes

### Find subclasses across modules

A subclass can live in another module.

```ds library.ds
export class Base {}
             ^^^^ target:base
```

```ds implementation.ds
import { Base } from "./library.ds";

export class Derived extends Base {}
^ declaration:derived:start
                                    ^ declaration:derived:end
             ^^^^^^^ implementation:derived
```

```query goto_implementation library.ds#target:base
@goto_implementation.target origin=library.ds#target:base location=implementation.ds#declaration:derived selection=implementation.ds#implementation:derived symbol=implementation.ds#Derived@2
```

## Re-Export Chains

### Find implementations through multi-hop re-exports

Implementation lookup follows a chain of re-exports.

```ds types.ds
export interface Renderable {
                 ^^^^^^^^^^ target:renderable
    render(): void;
}
```

```ds barrel_a.ds
export { Renderable as Surface } from "./types.ds";
```

```ds barrel_b.ds
export { Surface } from "./barrel_a.ds";
```

```ds implementation.ds
import { Surface } from "./barrel_b.ds";

export class Sprite implements Surface {
^ declaration:sprite:start
             ^^^^^^ implementation:sprite
    render(): void {}
}
^ declaration:sprite:end
```

```query goto_implementation types.ds#target:renderable
@goto_implementation.target origin=types.ds#target:renderable location=implementation.ds#declaration:sprite selection=implementation.ds#implementation:sprite symbol=implementation.ds#Sprite@2
```

## Source changes

### Add an implementation

Implementation lookup includes declarations added in later revisions.

```ds main.ds
interface Drawable {
          ^^^^^^^^ target
    draw(): void;
}
```

```query goto_implementation main.ds#target
@goto_implementation.none
```

```ds main.ds change
interface Drawable {
          ^^^^^^^^ target
    draw(): void;
}

class Circle implements Drawable {
^ declaration:start
      ^^^^^^ implementation
    draw(): void {}
}
^ declaration:end
```

```query goto_implementation main.ds#target
@goto_implementation.target origin=main.ds#target location=main.ds#declaration selection=main.ds#implementation symbol=main.ds#Circle@4
```
