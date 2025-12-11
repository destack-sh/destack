# Extension Implements

Tests for extensions that implement interfaces.

> NOTE: These tests are placeholders for when operator overloading is fully implemented.
> Currently, extensions can declare `implements` but operator dispatch is not yet wired up.

## Basic Implements

### extension implements interface

> Extensions can implement interfaces for a type.

```ds
interface Printable {
    toString(): string
}

struct Point { x: number, y: number }

extension Point implements Printable {
    toString(): string { return "" }
}

declare function getPoint(): Point;

const p = getPoint();
const s: string = p.toString();
```

## Operator Interfaces (Placeholder)

### extension implements Add

> When operator overloading is implemented, extensions can add operator support.

```ds
struct Vector2 { x: number, y: number }

// NOTE: Add interface and operator dispatch not yet implemented
// extension Vector2 implements Add<Vector2> {
//     add(other: Vector2): Vector2 {
//         return Vector2 { x: 0, y: 0 }
//     }
// }

extension Vector2 {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const a = getVector();
const b = getVector();
const c: Vector2 = a.add(b);
```
