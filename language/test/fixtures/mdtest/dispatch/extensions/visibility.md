# Extension Visibility

Tests for extension visibility rules.
Extension visibility depends on where the extension is defined relative to the type:

| Kind | Definition | Visibility |
|------|------------|------------|
| **Inherent** | Same file as type | Automatic wherever type is used |
| **Local** | Different file from type, anonymous | Only in defining file |
| **Named** | Different file from type, named | Must be imported |

## Inherent Extensions

> Inherent extensions are defined in the same module as the type they extend.
> They are automatically visible wherever the type is used.

### inherent extension in same file

> Extension in same file as type is always visible.

```ds
struct Point { x: number, y: number }

extension for Point {
    length(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
```

### inherent extension on class

> Inherent extensions work on classes too.

```ds
class User { name: string }

extension for User {
    greet(): string { return "" }
}

declare function getUser(): User;

const user = getUser();
user.greet() satisfies string;
```

### inherent extension on interface

> Inherent extensions work on interfaces.

```ds
interface Shape {
    area(): number
}

extension for Shape {
    describe(): string { return "" }
}

declare function getShape(): Shape;

const shape = getShape();
shape.describe() satisfies string;
```

### inherent extension on enum

> Inherent extensions work on enums.

```ds
enum Color {
    Red,
    Green,
    Blue
}

extension for Color {
    isWarm(): boolean { return true }
}

declare function getColor(): Color;

const color = getColor();
color.isWarm() satisfies boolean;
```

### multiple inherent extensions

> A type can have multiple inherent extensions in the same file.

```ds
struct Vector2 { x: number, y: number }

extension for Vector2 {
    magnitude(): number { return 0 }
}

extension for Vector2 {
    normalized(): Vector2 { return Vector2 { x: 0, y: 0 } }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.magnitude() satisfies number;
vector.normalized() satisfies Vector2;
```

## Local Extensions

> Local extensions extend a foreign type (from another file).
> They are only visible in the file where they are declared.

### local extension not visible in other files

> Local extension is not visible when type is used in another file.

```ds:types.ds
export struct Vector2 { x: number, y: number }
```

```ds:extensions.ds
import { Vector2 } from "./types.ds"

extension for Vector2 {
    magnitude(): number { return 0 }
}
```

```ds:main.ds
import { Vector2 } from "./types.ds"

declare function getVector(): Vector2;

const vector = getVector();
const m = vector.magnitude();
```

- contains: does not exist

## Named Extensions

> Named extensions use the syntax `extension Name for Type { }`.
> They can be exported and must be imported to use (not yet fully implemented).

### named extension syntax

> Named extensions have a name before the `for` keyword.

```ds
struct Point { x: number, y: number }

extension PointHelpers for Point {
    distance(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.distance() satisfies number;
```

## Overlapping Extensions

### type's own members take priority over extensions

> When a type has a member and an extension adds a method with the same name,
> the type's own member is used.

```ds
struct Point {
    x: number,
    y: number,
    length(): number { return 0 }
}

extension for Point {
    length(): number { return 1 }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
```

### first extension wins for duplicates

> When multiple extensions define the same method, the first one wins.

```ds
struct Vector2 { x: number, y: number }

extension for Vector2 {
    process(): number { return 1 }
}

extension for Vector2 {
    process(): string { return "" }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.process() satisfies number;
```
