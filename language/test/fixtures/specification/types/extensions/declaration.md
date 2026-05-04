# Extension Declarations

Extension declarations on different nominal types.

## structs

### extension adds method to struct

> Extensions can add methods to a struct defined in the same file.

```ds
struct Point {
    x: number;
    y: number
}

extension of Point {
    magnitude(): number {
        return 0
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.magnitude() satisfies number;
```

### extension does not modify type shape

> Extension methods are resolved separately from the type's own members.

```ds
struct Point {
    x: number;
    y: number
}

extension of Point {
    magnitude(): number {
        return 0
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.x satisfies number;
point.y satisfies number;
```

### multiple extensions on same type

> A type can have multiple extension blocks.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 {
    add(other: Vector2): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

extension of Vector2 {
    scale(factor: number): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.add(vector) satisfies Vector2;
vector.scale(2) satisfies Vector2;
```

## classes

### extension on class

> Extensions can add methods to classes.

```ds
class Counter {
    count: number;

    constructor(count: number) {
        this.count = count;
    }
}

extension of Counter {
    increment(): void {}
    reset(): void {}
}

declare function getCounter(): Counter;

const counter = getCounter();
counter.increment();
counter.reset();
```

## interfaces

### extension on interface

> Extensions can add methods to interfaces.

```ds
interface Shape {
    area(): number
}

extension of Shape {
    describe(): string {
        return ""
    }
}

declare function getShape(): Shape;

const shape = getShape();
shape.describe() satisfies string;
```

## Extension on Enum

### extension on enum

> Extensions can add methods to enums.

```ds
enum Status {
    Active,
    Inactive,
    Pending
}

extension of Status {
    isActive(): boolean {
        return true
    }
}

declare function getStatus(): Status;

const status = getStatus();
status.isActive() satisfies boolean;
```
