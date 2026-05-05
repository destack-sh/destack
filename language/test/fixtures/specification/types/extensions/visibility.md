# Extension Visibility

Extension visibility depends on where the extension is declared relative to the target type.

| Kind | Definition | Visibility |
|------|------------|------------|
| **Inherent** | Same file as type | Automatic wherever type is used |
| **Local** | Different file from type, anonymous | Only in defining file |
| **Named** | Different file from type, named | Must be imported |

## inherent extensions

### inherent extension in same file

> Inherent extensions are visible next to the type they extend.

```ds
struct Point { x: number; y: number }

extension of Point {
    length(): number { return 0 }
}

declare function getPoint(): Point;

const point = getPoint();
point.length() satisfies number;
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

## local extensions

### local extension not visible in other files

> Local extensions are not visible outside the file where they are declared.

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

## named extensions

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
