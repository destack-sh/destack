# Extension Calls

Extension methods call like ordinary methods once the extension is visible.

## returns

### extension methods return primitives

Extension methods can return primitive types.

```ds
struct Point { x: number; y: number }

extension of Point {
    length(): number { 
        return 0;
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
```

### extension methods return void

Extension methods can return void.

```ds
struct Logger { prefix: string }

extension of Logger {
    log(message: string): void {}
}

declare function getLogger(): Logger;

const logger = getLogger();
logger.log("hello");
```

### extension methods return the receiver type

Extension methods can return the same type they extend.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 {
    normalized(): Vector2 {
        return Vector2 { x: 0, y: 0 }
    }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.normalized() satisfies Vector2;
```

### extension methods return other types

Extension methods can return a different type.

```ds
struct Point { x: number; y: number }

extension of Point {
    toString(): string {
        return ""
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.toString() satisfies string;
```

## parameters

### extension methods accept parameters

Extension methods can take ordinary parameters.

```ds
struct Point { x: number; y: number }

extension of Point {
    scale(factor: number): Point {
        return Point { x: 0, y: 0 }
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.scale(2) satisfies Point;
```

### extension methods accept multiple parameters

Extension methods can take multiple parameters.

```ds
struct Point { x: number; y: number }

extension of Point {
    translate(dx: number, dy: number): Point {
        return Point { x: 0, y: 0 }
    }
}

declare function getPoint(): Point;

const point = getPoint();
point.translate(10, 20) satisfies Point;
```

### extension methods accept optional parameters

Extension methods can take optional parameters.

```ds
struct Logger { prefix: string }

extension of Logger {
    log(message: string, level?: number): void {}
}

declare function getLogger(): Logger;

const logger = getLogger();
logger.log("info");
logger.log("warn", 2);
```

### extension methods can constrain this

Extension methods can declare an explicit this parameter to constrain the receiver.

```ds
struct Counter { value: number }

extension of Counter {
    increment(this: &Counter): void {
        this.value = this.value + 1
    }
}

let counter = Counter { value: 0 };
counter.increment();
```

## chaining

### extension methods chain through return types

Extension methods returning the receiver type can be chained.

```ds
struct StringBuilder { value: string }

extension of StringBuilder {
    append(text: string): StringBuilder {
        return StringBuilder { value: "" }
    }
}

declare function getStringBuilder(): StringBuilder;

const builder = getStringBuilder();
builder.append("a").append("b").append("c") satisfies StringBuilder;
```
