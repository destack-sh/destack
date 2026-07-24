# Goto Implementation

## Interfaces

### Find implementations of an interface

An interface resolves to every nominal implementation.

```ds main.ds
interface Drawable {
          ^^^^^^^^ target:drawable
    draw(): void;
}

class Circle implements Drawable {
      ^^^^^^ implementation:circle
    draw(): void {}
}

struct Rectangle implements Drawable {
       ^^^^^^^^^ implementation:rectangle
    draw(): void {}
}
```

```query goto_implementation main.ds#target:drawable
@goto_implementation.target relation=implementation location=main.ds#implementation:circle symbol=main.ds#Circle@4
@goto_implementation.target relation=implementation location=main.ds#implementation:rectangle symbol=main.ds#Rectangle@7
```

## Nominal Interfaces

### Find every direct nominal implementation kind

A nominal interface resolves to classes, structs, enums, and extension targets that implement it.

```ds main.ds
newtype interface Display {}
                  ^^^^^^^ target:display

class View implements Display {}
      ^^^^ implementation:view

struct Packet implements Display {}
       ^^^^^^ implementation:packet

enum Status implements Display { Ready }
     ^^^^^^ implementation:status

newtype UserId = string;

extension of UserId implements Display {}
             ^^^^^^ implementation:user_id
```

```query goto_implementation main.ds#target:display
@goto_implementation.target relation=implementation location=main.ds#implementation:view symbol=main.ds#View@2
@goto_implementation.target relation=implementation location=main.ds#implementation:packet symbol=main.ds#Packet@3
@goto_implementation.target relation=implementation location=main.ds#implementation:status symbol=main.ds#Status@4
@goto_implementation.target relation=implementation location=main.ds#implementation:user_id symbol=main.ds#symbol@7
```

### Return no targets for an unimplemented interface

An interface without authored implementation edges has no implementation target.

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
      ^^^^^^^ implementation:derived
    value: int32;
}

class SubDerived extends Derived {}
```

```query goto_implementation main.ds#target:base
@goto_implementation.target relation=implementation location=main.ds#implementation:derived symbol=main.ds#Derived@4
```

## Methods

### Find implementations of an interface method

An interface method resolves to each exact member that implements it.

```ds main.ds
interface Renderable {
    render(): string;
    ^^^^^^ target
}

class View implements Renderable {
    render(): string {
    ^^^^^^ implementation:view
        return "";
    }
}

struct Document implements Renderable {
    render(): string {
    ^^^^^^ implementation:document
        return "";
    }
}
```

```query goto_implementation main.ds#target
@goto_implementation.target relation=implementation location=main.ds#implementation:view symbol=main.ds#render@5
@goto_implementation.target relation=implementation location=main.ds#implementation:document symbol=main.ds#render@8
```

### Find overrides of a class method

An abstract class method resolves to each exact overriding member.

```ds main.ds
abstract class Writer {
    abstract write(value: string): void;
             ^^^^^ target
}

class FileWriter extends Writer {
    override write(value: string): void {}
             ^^^^^ implementation
}
```

```query goto_implementation main.ds#target
@goto_implementation.target relation=implementation location=main.ds#implementation symbol=main.ds#write@6
```

## Cross-Module Interfaces

### Find implementations across modules

Implementations can live in another module.

```ds library.ds
export interface Drawable {
                 ^^^^^^^^ target:drawable
    draw(): void;
}
```

```ds implementation.ds
import { Drawable } from "./library.ds";

export class Circle implements Drawable {
             ^^^^^^ implementation:circle
    draw(): void {}
}

export struct Square implements Drawable {
              ^^^^^^ implementation:square
    draw(): void {}
}
```

```query goto_implementation library.ds#target:drawable
@goto_implementation.target relation=implementation location=implementation.ds#implementation:circle symbol=implementation.ds#Circle@2
@goto_implementation.target relation=implementation location=implementation.ds#implementation:square symbol=implementation.ds#Square@5
```

### Find implementations through a re-exported interface

A re-export alias retains the interface's implementation set.

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
             ^^^^^^ implementation:sprite
    render(): void {}
}

export struct Icon implements Renderable {
              ^^^^ implementation:icon
    render(): void {}
}
```

```query goto_implementation alias_library.ds#target:renderable
@goto_implementation.target relation=implementation location=alias_implementation.ds#implementation:sprite symbol=alias_implementation.ds#Sprite@2
@goto_implementation.target relation=implementation location=alias_implementation.ds#implementation:icon symbol=alias_implementation.ds#Icon@5
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
             ^^^^^^^ implementation:derived
```

```query goto_implementation library.ds#target:base
@goto_implementation.target relation=implementation location=implementation.ds#implementation:derived symbol=implementation.ds#Derived@2
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
             ^^^^^^ implementation:sprite
    render(): void {}
}
```

```query goto_implementation types.ds#target:renderable
@goto_implementation.target relation=implementation location=implementation.ds#implementation:sprite symbol=implementation.ds#Sprite@2
```
