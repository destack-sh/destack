
## Interfaces

### Find implementations of an interface

An interface resolves to every nominal implementation.

```tspp main.tspp
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

```query goto_implementation main.tspp#target:drawable
@goto_implementation.target origin=main.tspp#target:drawable location=main.tspp#declaration:circle selection=main.tspp#implementation:circle symbol=main.tspp#Circle@4
@goto_implementation.target origin=main.tspp#target:drawable location=main.tspp#declaration:rectangle selection=main.tspp#implementation:rectangle symbol=main.tspp#Rectangle@7
```

### Return current implementations

Implementation lookup includes declarations added by later edits.

```tspp main.tspp
interface Drawable {
          ^^^^^^^^ target
    draw(): void;
}
```

```query goto_implementation main.tspp#target
@goto_implementation.none
```

```tspp main.tspp change
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

```query goto_implementation main.tspp#target
@goto_implementation.target origin=main.tspp#target location=main.tspp#declaration selection=main.tspp#implementation symbol=main.tspp#Circle@4
```

## Nominal Interfaces

### Find every direct nominal implementation kind

A nominal interface resolves to every class, struct, enum, and extension declaration that implements it.

```tspp main.tspp
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

```query goto_implementation main.tspp#target:display
@goto_implementation.target origin=main.tspp#target:display location=main.tspp#declaration:view selection=main.tspp#implementation:view symbol=main.tspp#View@2
@goto_implementation.target origin=main.tspp#target:display location=main.tspp#declaration:packet selection=main.tspp#implementation:packet symbol=main.tspp#Packet@3
@goto_implementation.target origin=main.tspp#target:display location=main.tspp#declaration:status selection=main.tspp#implementation:status symbol=main.tspp#Status@4
@goto_implementation.target origin=main.tspp#target:display location=main.tspp#declaration:user_id selection=main.tspp#implementation:user_id symbol=main.tspp#symbol@7
```

### Return no targets for an unimplemented interface

An interface without implementations has no implementation target.

```tspp main.tspp
newtype interface Display {}
                  ^^^^^^^ target
```

```query goto_implementation main.tspp#target
@goto_implementation.none
```

## Empty Results

### Return no implementations for functions

Functions are not implementation hierarchy targets.

```tspp main.tspp
function helper(): void {}
         ^^^^^^ target:helper
```

```query goto_implementation main.tspp#target:helper
@goto_implementation.none
```

### Return no implementations for structs

Structs do not have nominal subtypes.

```tspp main.tspp
struct Point {
       ^^^^^ target:point
    x: int32;
}
```

```query goto_implementation main.tspp#target:point
@goto_implementation.none
```

## Class Inheritance

### Find direct subclasses of a class

A class resolves to its direct subclasses.

```tspp main.tspp
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

```query goto_implementation main.tspp#target:base
@goto_implementation.target origin=main.tspp#target:base location=main.tspp#declaration:derived selection=main.tspp#implementation:derived symbol=main.tspp#Derived@4
```

## Methods

### Find implementations of an interface method

An interface method resolves to every member that implements it.

```tspp main.tspp
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

```query goto_implementation main.tspp#target
@goto_implementation.target origin=main.tspp#target location=main.tspp#declaration:view selection=main.tspp#implementation:view symbol=main.tspp#render@5
@goto_implementation.target origin=main.tspp#target location=main.tspp#declaration:document selection=main.tspp#implementation:document symbol=main.tspp#render@8
```

### Find implementations of an associated type

An interface associated type resolves to every implementing associated declaration.

```tspp main.tspp
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

```query goto_implementation main.tspp#target
@goto_implementation.target origin=main.tspp#target location=main.tspp#declaration selection=main.tspp#implementation symbol=main.tspp#Item@5
```

```diff main.tspp
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

```query goto_implementation main.tspp#target
@goto_implementation.target origin=main.tspp#target location=main.tspp#declaration selection=main.tspp#implementation symbol=main.tspp#Item@5
```

### Find overrides of a class method

An abstract class method resolves to every overriding member.

```tspp main.tspp
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

```query goto_implementation main.tspp#target
@goto_implementation.target origin=main.tspp#target location=main.tspp#declaration selection=main.tspp#implementation symbol=main.tspp#write@6
```

## Cross-Module Interfaces

### Find implementations across modules

Implementations can live in another module.

```tspp library.tspp
export interface Drawable {
                 ^^^^^^^^ target:drawable
    draw(): void;
    ^^^^ target:draw
}
```

```tspp implementation.tspp
import { Drawable } from "./library.tspp";

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

```query goto_implementation library.tspp#target:drawable
@goto_implementation.target origin=library.tspp#target:drawable location=implementation.tspp#declaration:circle selection=implementation.tspp#implementation:circle symbol=implementation.tspp#Circle@2
@goto_implementation.target origin=library.tspp#target:drawable location=implementation.tspp#declaration:square selection=implementation.tspp#implementation:square symbol=implementation.tspp#Square@5
```

```query goto_implementation library.tspp#target:draw
@goto_implementation.target origin=library.tspp#target:draw location=implementation.tspp#declaration:circle_draw selection=implementation.tspp#implementation:circle_draw symbol=implementation.tspp#draw@3
@goto_implementation.target origin=library.tspp#target:draw location=implementation.tspp#declaration:square_draw selection=implementation.tspp#implementation:square_draw symbol=implementation.tspp#draw@6
```

### Find implementations through a re-exported interface

A re-export alias preserves the interface's implementation set.

```tspp alias_library.tspp
export interface Renderable {
                 ^^^^^^^^^^ target:renderable
    render(): void;
}
```

```tspp alias_barrel.tspp
export { Renderable } from "./alias_library.tspp";
```

```tspp alias_implementation.tspp
import { Renderable } from "./alias_barrel.tspp";

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

```query goto_implementation alias_library.tspp#target:renderable
@goto_implementation.target origin=alias_library.tspp#target:renderable location=alias_implementation.tspp#declaration:sprite selection=alias_implementation.tspp#implementation:sprite symbol=alias_implementation.tspp#Sprite@2
@goto_implementation.target origin=alias_library.tspp#target:renderable location=alias_implementation.tspp#declaration:icon selection=alias_implementation.tspp#implementation:icon symbol=alias_implementation.tspp#Icon@5
```

## Cross-Module Classes

### Find subclasses across modules

A subclass can live in another module.

```tspp library.tspp
export class Base {}
             ^^^^ target:base
```

```tspp implementation.tspp
import { Base } from "./library.tspp";

export class Derived extends Base {}
^ declaration:derived:start
                                    ^ declaration:derived:end
             ^^^^^^^ implementation:derived
```

```query goto_implementation library.tspp#target:base
@goto_implementation.target origin=library.tspp#target:base location=implementation.tspp#declaration:derived selection=implementation.tspp#implementation:derived symbol=implementation.tspp#Derived@2
```

## Re-Export Chains

### Find implementations through multi-hop re-exports

Implementation lookup follows a chain of re-exports.

```tspp types.tspp
export interface Renderable {
                 ^^^^^^^^^^ target:renderable
    render(): void;
}
```

```tspp barrel_a.tspp
export { Renderable as Surface } from "./types.tspp";
```

```tspp barrel_b.tspp
export { Surface } from "./barrel_a.tspp";
```

```tspp implementation.tspp
import { Surface } from "./barrel_b.tspp";

export class Sprite implements Surface {
^ declaration:sprite:start
             ^^^^^^ implementation:sprite
    render(): void {}
}
^ declaration:sprite:end
```

```query goto_implementation types.tspp#target:renderable
@goto_implementation.target origin=types.tspp#target:renderable location=implementation.tspp#declaration:sprite selection=implementation.tspp#implementation:sprite symbol=implementation.tspp#Sprite@2
```
