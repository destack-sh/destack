# Extension Rejections

Rejected extension method calls.

## missing methods

### calling nonexistent extension method

> Calling a method that doesn't exist on the type or extensions is an error.

```ds
struct Point { x: number; y: number }

extension of Point {
    magnitude(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.nonexistent();
```

- contains: does not exist

### method exists on different type

> Extension methods are scoped to their type.

```ds
struct Point { x: number; y: number }
struct Vector3 { x: number; y: number; z: number }

extension of Point {
    magnitude(): number { return 0 }
}

declare function getVector(): Vector3;

const vector = getVector();
vector.magnitude();
```

- contains: does not exist

## arguments

### wrong argument type

> Extension method arguments must satisfy their declared parameter types.

```ds
struct Calculator { value: number }

extension of Calculator {
    add(a: number, b: number): number { return 0 }
}

declare function getCalculator(): Calculator;

const calculator = getCalculator();
calculator.add("one", 2);
```

- contains: not assignable

## visibility

### local extension not visible from another file

> Local extension on foreign type is not visible from other files.

```ds:types.ds
export struct Vector2 { x: number; y: number }
```

```ds:extensions.ds
import { Vector2 } from "./types.ds"

extension of Vector2 {
    magnitude(): number { return 0 }
}
```

```ds:main.ds
import { Vector2 } from "./types.ds"

declare function getVector(): Vector2;

const vector = getVector();
vector.magnitude();
```

- contains: does not exist
