# Extension Methods

Tests for extension method signatures, parameters, and special members.

## Return Types

### extension method returns primitive

> Extension methods can return primitive types.

```ds
struct Point { x: number, y: number }

extension Point {
    length(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
```

### extension method returns void

> Extension methods can return void.

```ds
struct Logger { prefix: string }

extension Logger {
    log(message: string): void {}
}

declare function getLogger(): Logger;

const logger = getLogger();
logger.log("hello");
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

const vector = getVector();
vector.normalized() satisfies Vector2;
```

### extension method returns different type

> Extension methods can return a different type.

```ds
struct Point { x: number, y: number }

extension Point {
    toString(): string {
        return ""
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.toString() satisfies string;
```

## Parameters

### extension method with no parameters

> Extension methods can have no parameters (only implicit `this`).

```ds
struct Counter { value: number }

extension Counter {
    isZero(): boolean {
        return this.value == 0
    }
}

declare function getCounter(): Counter;

const counter = getCounter();
counter.isZero() satisfies boolean;
```

### extension method with single parameter

> Extension methods can have one parameter.

```ds
struct Point { x: number, y: number }

extension Point {
    scale(factor: number): Point {
        return Point { x: 0, y: 0 }
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.scale(2) satisfies Point;
```

### extension method with multiple parameters

> Extension methods can have multiple parameters.

```ds
struct Point { x: number, y: number }

extension Point {
    translate(dx: number, dy: number): Point {
        return Point { x: 0, y: 0 }
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.translate(10, 20) satisfies Point;
```

### extension method with typed parameter of same type

> Extension method parameters can be the same type as the extended type.

```ds
struct Vector2 { x: number, y: number }

extension Vector2 {
    dot(other: Vector2): number {
        return this.x * other.x + this.y * other.y
    }
}

declare function getVector(): Vector2;

const vector1 = getVector();
const vector2 = getVector();
vector1.dot(vector2) satisfies number;
```

### extension method with optional parameter

> Extension methods can have optional parameters.

```ds
struct Logger { prefix: string }

extension Logger {
    log(message: string, level?: number): void {}
}

declare function getLogger(): Logger;

const logger = getLogger();
logger.log("info");
logger.log("warn", 2);
```

## This Access

### extension method reads this fields

> Extension methods can read fields from `this`.

```ds
struct Rectangle { width: number, height: number }

extension Rectangle {
    area(): number {
        return this.width * this.height
    }
}

declare function getRectangle(): Rectangle;

const rectangle = getRectangle();
rectangle.area() satisfies number;
```

### extension method calls this methods

> Extension methods can call other methods on `this`.

```ds
struct Rectangle { width: number, height: number }

extension Rectangle {
    area(): number {
        return this.width * this.height
    }

    isLarge(): boolean {
        return this.area() > 100
    }
}

declare function getRectangle(): Rectangle;

const rectangle = getRectangle();
rectangle.isLarge() satisfies boolean;
```

### extension method calls type's own methods

> Extension methods can call methods defined on the type itself.

```ds
interface Measurable {
    measure(): number
}

extension Measurable {
    measureTwice(): number {
        return this.measure() * 2
    }
}

declare function getMeasurable(): Measurable;

const measurable = getMeasurable();
measurable.measureTwice() satisfies number;
```

## Method Chaining

### extension methods can be chained

> Extension methods returning the same type can be chained.

```ds
struct StringBuilder { value: string }

extension StringBuilder {
    append(text: string): StringBuilder {
        return StringBuilder { value: "" }
    }
}

declare function getStringBuilder(): StringBuilder;

const builder = getStringBuilder();
builder.append("a").append("b").append("c") satisfies StringBuilder;
```

### chained methods with different return types

> Method chains can include methods with different return types.

```ds
struct StringBuilder { value: string }

extension StringBuilder {
    append(text: string): StringBuilder {
        return StringBuilder { value: "" }
    }

    build(): string {
        return this.value
    }
}

declare function getStringBuilder(): StringBuilder;

const builder = getStringBuilder();
builder.append("a").append("b").build() satisfies string;
```

## Static Members

### extension with static method

> Extensions can define static methods.

```ds
struct Vector2 { x: number, y: number }

extension Vector2 {
    static zero(): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

Vector2.zero() satisfies Vector2;
```

### extension with static constant

> Extensions can define static constants.

```ds
struct Vector2 { x: number, y: number }

extension Vector2 {
    static Origin: Vector2 = Vector2 { x: 0, y: 0 }
}

Vector2.Origin satisfies Vector2;
```

### extension with both static and instance methods

> Extensions can have both static and instance methods.

```ds
struct Counter { value: number }

extension Counter {
    static create(): Counter {
        return Counter { value: 0 }
    }

    increment(): Counter {
        return Counter { value: this.value + 1 }
    }
}

Counter.create() satisfies Counter;

declare function getCounter(): Counter;
getCounter().increment() satisfies Counter;
```
