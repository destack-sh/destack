# Extension Errors

Tests for error cases with extensions.

## Missing Methods

### calling nonexistent extension method

> Calling a method that doesn't exist on the type or extensions is an error.

```ds
struct Point { x: number, y: number }

extension Point {
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
struct Point { x: number, y: number }
struct Vector3 { x: number, y: number, z: number }

extension Point {
    magnitude(): number { return 0 }
}

declare function getVector(): Vector3;

const vector = getVector();
vector.magnitude();
```

- contains: does not exist

## Type Errors

### wrong return type used

> Return type of extension method is checked.

```ds
struct Point { x: number, y: number }

extension Point {
    magnitude(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.magnitude() satisfies string;
```

- contains: not assignable

### wrong argument type

> Passing wrong argument type to extension method is an error.

```ds
struct Calculator { value: number }

extension Calculator {
    add(a: number, b: number): number { return 0 }
}

declare function getCalculator(): Calculator;

const calculator = getCalculator();
calculator.add("one", 2);
```

- contains: not assignable

### wrong number of arguments

> Extension methods must be called with correct number of arguments.

```ds
struct Calculator { value: number }

extension Calculator {
    add(a: number, b: number): number { return 0 }
}

declare function getCalculator(): Calculator;

const calculator = getCalculator();
calculator.add(1);
```

- contains: expected 2 arguments

### too many arguments

> Extension methods reject extra arguments.

```ds
struct Calculator { value: number }

extension Calculator {
    add(a: number, b: number): number { return 0 }
}

declare function getCalculator(): Calculator;

const calculator = getCalculator();
calculator.add(1, 2, 3);
```

- contains: expected 2 arguments

## Visibility Errors

### local extension not visible from another file

> Local extension on foreign type is not visible from other files.

```ds:types.ds
export struct Vector2 { x: number, y: number }
```

```ds:extensions.ds
import { Vector2 } from "./types.ds"

extension Vector2 {
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

## This Errors

### accessing nonexistent field via this

> Extension methods cannot access fields that don't exist.

```ds
struct Point { x: number, y: number }

extension Point {
    getZ(): number {
        return this.z
    }
}
```

- contains: does not exist

## Extension Target Errors

### extension on undefined type

> Extension must target an existing type.

```ds
extension NonexistentType {
    process(): void {}
}
```

- contains: not defined

## Duplicate Method Handling

### shadowing between extensions

> When multiple extensions define the same method, the first one wins.
> This is not an error, but the second definition is ignored.

```ds
struct Vector2 { x: number, y: number }

extension Vector2 {
    process(): number { return 1 }
}

extension Vector2 {
    process(): string { return "" }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.process() satisfies number;
```

## Implementation Errors

### extension method body type mismatch

> Extension method body must match return type.

```ds
struct Point { x: number, y: number }

extension Point {
    magnitude(): number {
        return "not a number"
    }
}
```

- contains: not assignable

### extension method references undefined variable

> Extension methods cannot reference undefined variables.

```ds
struct Point { x: number, y: number }

extension Point {
    magnitude(): number {
        return undefinedVariable
    }
}
```

- contains: not defined
