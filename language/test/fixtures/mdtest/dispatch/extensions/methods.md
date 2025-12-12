# Extension Methods

Tests for extension method signatures and parameters.

## Return Types

### extension method returns primitive

> Extension methods can return primitive types.

```ds
struct Point { x: number, y: number }

extension Point {
    length(): number { return 0 }
}

declare function getPoint(): Point;

const p = getPoint();
const len: number = p.length();
```

### extension method returns void

> Extension methods can return void.

```ds
struct Logger {}

extension Logger {
    log(message: string): void {}
}

declare function getLogger(): Logger;

const l = getLogger();
l.log("hello");
```

### extension method returns same type

> Extension methods can return the same type they extend.

```ds
struct Vector2 { x: number, y: number }

extension Vector2 {
    normalized(): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const v = getVector();
const n: Vector2 = v.normalized();
```

## Parameters

### extension method with parameters

> Extension methods can have parameters.

```ds
struct Point { x: number, y: number }

extension Point {
    translate(dx: number, dy: number): Point {
        return Point { x: 0, y: 0 }
    }
}

declare function getPoint(): Point;

const p = getPoint();
const moved: Point = p.translate(10, 20);
```

### extension method with typed parameter

> Extension method parameters are type checked.

```ds
struct Calculator {}

extension Calculator {
    add(a: number, b: number): number { return 0 }
}

declare function getCalc(): Calculator;

const calc = getCalc();
const sum: number = calc.add(1, 2);
```

### extension method wrong argument type

> Passing wrong argument type to extension method is an error.

```ds
struct Calculator {}

extension Calculator {
    add(a: number, b: number): number { return 0 }
}

declare function getCalc(): Calculator;

const calc = getCalc();
const sum: number = calc.add("one", 2);
```

- contains: not assignable

## Chaining

### extension methods can be chained

> Extension methods returning the same type can be chained.

```ds
struct Builder { value: string }

extension Builder {
    append(s: string): Builder {
        return Builder { value: "" }
    }
}

declare function getBuilder(): Builder;

const b = getBuilder();
const result: Builder = b.append("a").append("b").append("c");
```
