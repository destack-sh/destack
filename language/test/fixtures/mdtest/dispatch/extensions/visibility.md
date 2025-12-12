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

extension Point {
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

extension User {
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

extension Shape {
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

extension Color {
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

extension Vector2 {
    magnitude(): number { return 0 }
}

extension Vector2 {
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

### local extension on imported type

> Local extension is visible in the file where it's defined.

```ds:types.ds
export struct Vector2 { x: number, y: number }
```

```ds:main.ds
import { Vector2 } from "./types.ds"

extension Vector2 {
    magnitude(): number { return 0 }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.magnitude() satisfies number;
```

### local extension not visible in other files

> Local extension is not visible when type is used in another file.

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
const m = vector.magnitude();
```

- contains: does not exist

### multiple local extensions on same type

> Multiple files can have their own local extensions on the same foreign type.

```ds:types.ds
export struct Vector2 { x: number, y: number }
```

```ds:main.ds
import { Vector2 } from "./types.ds"

extension Vector2 {
    doubled(): Vector2 { return Vector2 { x: this.x * 2, y: this.y * 2 } }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.doubled() satisfies Vector2;
```

## Named Extensions

> Named extensions use the syntax `extension Name: Type { }`.
> They can be exported and must be imported to use (not yet fully implemented).

### named extension syntax

> Named extensions have a name before the colon.

```ds
struct Point { x: number, y: number }

extension PointHelpers: Point {
    distance(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.distance() satisfies number;
```

### named extension on foreign type in same file

> Named extension is visible in the defining file.

```ds:types.ds
export struct Vector2 { x: number, y: number }
```

```ds:main.ds
import { Vector2 } from "./types.ds"

extension VectorHelpers: Vector2 {
    magnitude(): number { return 0 }
}

declare function getVector(): Vector2;

const vector = getVector();
vector.magnitude() satisfies number;
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

extension Point {
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
