# Basic Extensions

Tests for basic extension declarations on different types.

## Extension on Struct

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

const point = getPoint();
point.magnitude() satisfies number;
```

### extension method accesses struct fields

> Extension methods can access fields via `this`.

```ds
struct Point {
    x: number,
    y: number
}

extension Point {
    sum(): number {
        return this.x + this.y
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.sum() satisfies number;
```

### extension does not modify type shape

> Extension methods are resolved separately from the type's own members.

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

const point = getPoint();
point.x satisfies number;
point.y satisfies number;
```

### multiple extensions on same type

> A type can have multiple extension blocks.

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

declare function getVector(): Vector2;

const vector = getVector();
vector.add(vector) satisfies Vector2;
vector.scale(2) satisfies Vector2;
```

## Extension on Class

### extension on class

> Extensions can add methods to classes.

```ds
class Counter {
    count: number
}

extension Counter {
    increment(): void {}
    reset(): void {}
}

declare function getCounter(): Counter;

const counter = getCounter();
counter.increment();
counter.reset();
```

### extension method accesses class fields

> Extension methods can access class fields via `this`.

```ds
class Counter {
    count: number
}

extension Counter {
    doubled(): number {
        return this.count * 2
    }
}

declare function getCounter(): Counter;

const counter = getCounter();
counter.doubled() satisfies number;
```

## Extension on Interface

### extension on interface

> Extensions can add methods to interfaces.

```ds
interface Shape {
    area(): number
}

extension Shape {
    describe(): string {
        return ""
    }
}

declare function getShape(): Shape;

const shape = getShape();
shape.describe() satisfies string;
```

### extension method calls interface method

> Extension methods can call the interface's own methods.

```ds
interface Shape {
    area(): number
}

extension Shape {
    isLarge(): boolean {
        return this.area() > 100
    }
}

declare function getShape(): Shape;

const shape = getShape();
shape.isLarge() satisfies boolean;
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

extension Status {
    isActive(): boolean {
        return true
    }
}

declare function getStatus(): Status;

const status = getStatus();
status.isActive() satisfies boolean;
```

## Extension on Type Alias

### extension on type alias

> Extensions can extend type aliases (the alias resolves to its underlying type).

```ds
struct Point { x: number, y: number }
type Vector = Point;

extension Vector {
    length(): number {
        return 0
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
```
