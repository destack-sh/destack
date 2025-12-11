# Extensions

Tests for extension declarations that add methods to existing types.

## Basic Extensions

### extension adds method to struct

> Extensions can add methods to a struct defined in the same file.

```ds
struct Point {
    x: number,
    y: number
}

extension Point {
    magnitude(): number {
        return 0
    }
}

declare function getPoint(): Point;

const p = getPoint();
const m: number = p.magnitude();
```

### extension method not on type directly

> Extension methods are not part of the type's shape directly.

```ds
struct Point {
    x: number,
    y: number
}

extension Point {
    magnitude(): number {
        return 0
    }
}

declare function getPoint(): Point;

const p = getPoint();
const x: number = p.x;
const y: number = p.y;
```

## Extension Visibility

### native extension visible in same file

> Extension defined in same file as type is visible.

```ds
struct Foo {}

extension Foo {
    bar(): number { return 42 }
}

declare function getFoo(): Foo;

const f = getFoo();
const b: number = f.bar();
```

### multiple extensions on same type

> A type can have multiple extensions.

```ds
struct Vector2 { x: number, y: number }

extension Vector2 {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

extension Vector2 {
    scale(factor: number): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVec(): Vector2;

const v = getVec();
const v2: Vector2 = v.add(v);
const v3: Vector2 = v.scale(2);
```

### extension on class

> Extensions can be added to classes.

```ds
class Counter {
    count: number
}

extension Counter {
    increment(): void {}
    reset(): void {}
}

declare function getCounter(): Counter;

const c = getCounter();
c.increment();
c.reset();
```
