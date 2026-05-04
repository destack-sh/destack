# Extension Visibility

Extension visibility rules.
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
struct Point { x: number; y: number }

extension of Point {
    length(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
```

### inherent extension on class

> Inherent extensions apply to classes too.

```ds
class User {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

extension of User {
    greet(): string { return "" }
}

declare function getUser(): User;

const user = getUser();
user.greet() satisfies string;
```

### inherent extension on interface

> Inherent extensions apply to interfaces.

```ds
interface Shape {
    area(): number
}

extension of Shape {
    describe(): string { return "" }
}

declare function getShape(): Shape;

const shape = getShape();
shape.describe() satisfies string;
```

### inherent extension on enum

> Inherent extensions apply to enums.

```ds
enum Color {
    Red,
    Green,
    Blue
}

extension of Color {
    isWarm(): boolean { return true }
}

declare function getColor(): Color;

const color = getColor();
color.isWarm() satisfies boolean;
```

### multiple inherent extensions

> A type can have multiple inherent extensions in the same file.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 {
    magnitude(): number { return 0 }
}

extension of Vector2 {
    normalized(): Vector2 { return Vector2 { x: 0, y: 0 } }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.magnitude() satisfies number;
vector.normalized() satisfies Vector2;
```

### inherent extension across modules

> Inherent extensions are visible in other files that import the type.

```ds:types.ds
export struct Vector2 { x: number; y: number }

extension of Vector2 {
    magnitude(): number { return 0 }
}
```

```ds:main.ds
import { Vector2 } from "./types.ds"

declare function getVector(): Vector2;

const vector = getVector();
vector.magnitude() satisfies number;
```

## Local Extensions

> Local extensions extend a foreign type (from another file).
> They are only visible in the file where they are declared.

### local extension not visible in other files

> Local extension is not visible when type is used in another file.

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
const m = vector.magnitude();
```

- contains: does not exist

## Named Extensions

> Named extensions use the syntax `extension Name of Type { }`.
> They can be exported and must be imported to use (not yet fully implemented).

### named extension syntax

> Named extensions have a name before the `of` keyword.

```ds
struct Point { x: number; y: number }

extension PointHelpers of Point {
    distance(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.distance() satisfies number;
```

### named extension across modules

> Named extensions are visible when explicitly imported.

```ds:types.ds
export struct Point { x: number; y: number }
```

```ds:extensions.ds
import { Point } from "./types.ds"

export extension PointHelpers of Point {
    distance(): number { return 0 }
}
```

```ds:main.ds
import { Point } from "./types.ds"
import { PointHelpers } from "./extensions.ds"

declare function getPoint(): Point;

const point = getPoint();
point.distance() satisfies number;
```

### named extension requires import

> Named extensions are not visible without an explicit import.

```ds:types.ds
export struct Point { x: number; y: number }
```

```ds:extensions.ds
import { Point } from "./types.ds"

export extension PointHelpers of Point {
    distance(): number { return 0 }
}
```

```ds:main.ds
import { Point } from "./types.ds"

declare function getPoint(): Point;

const point = getPoint();
point.distance();
```

- contains: does not exist

## Overlapping Extensions

### first extension wins of duplicates

> When multiple extensions define the same method, the first one wins.

```ds
struct Vector2 { x: number; y: number }

extension of Vector2 {
    process(): number { return 1 }
}

extension of Vector2 {
    process(): string { return "" }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.process() satisfies number;
```
