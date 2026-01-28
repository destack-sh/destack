# Goto Implementation

## Basic interface implementation

### Find implementations of an interface

When cursor is on an interface name, find all types that implement it.

```ds
interface $0Drawable {
    draw(): void;
}

class Circle implements Drawable {
    draw(): void {}
}

struct Rectangle implements Drawable {
    draw(): void {}
}
```

```query implementation $0
main.ds:5:7-5:13
main.ds:9:8-9:17
```

## Non nominal symbols

### No implementations for functions

Functions are not implementation targets, so the result should be empty.

```ds
function $0helper(): void {}
```

```query implementation $0
<none>
```

## Class inheritance

### Find subclasses of a class

When cursor is on a class name, find direct subclasses.

```ds
class $0Base {
    value: int32;
}

class Derived extends Base {
    value: int32;
}

class SubDerived extends Derived {}
```

The snapshot lists the direct subclass locations.

```query implementation $0
main.ds:5:7-5:14
```

## Cross module interfaces

### Find implementations across modules

Implementations in other modules should be included in results.

```ds:lib.ds
export interface Drawable {
//              ^^^^^^^ def:Drawable
    draw(): void;
}
```

```ds:impl.ds
import type { Drawable } from "./lib.ds";

export class Circle implements Drawable {
    draw(): void {}
}

export struct Square implements Drawable {
    draw(): void {}
}
```

```ds:main.ds
import type { Drawable } from "./lib.ds";
import { Circle, Square } from "./impl.ds";

function paint(item: Drawable): void {
    print(item);
}

const _circle = Circle {};
const _square = Square {};
```

```query implementation def:Drawable
impl.ds:3:14-3:20
impl.ds:7:15-7:21
```
