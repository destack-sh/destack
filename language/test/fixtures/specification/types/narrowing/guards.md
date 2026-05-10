# Guards

Guard expressions refine values in control flow.

## Familiar guards

### typeof narrows primitives

```ds
const value: string | number = 0;
if (typeof value == "string") {
    value satisfies string;
} else {
    value satisfies number;
}
```

### typeof preserves null objects

```ds
const value: { name: string } | null | string = "hello";
if (typeof value == "object") {
    value satisfies { name: string } | null;
} else {
    value satisfies string;
}
```

### typeof recognizes callable values

```ds
const value: (() => void) | string = "hello";
if (typeof value == "function") {
    value satisfies () => void;
} else {
    value satisfies string;
}
```

### instanceof narrows classes

```ds
class User {
    name: string = "";
}

const value: User | string = "hello";
if (value instanceof User) {
    value satisfies User;
} else {
    value satisfies string;
}
```

### instanceof requires classes

```ds
struct Point {
    x: int32;
    y: int32;
}

const value = Point { x: 1, y: 2 };
const ok = value instanceof Point;
```

- contains: instanceof requires a class type

### in narrows required keys

```ds
interface WithName {
    name: string;
}
interface WithId {
    id: number;
}

function narrow(value: WithName | WithId): void {
    if ("name" in value) {
        value satisfies WithName;
    } else {
        value satisfies WithId;
    }
}
```

## Runtime type guards

### is narrows primitive unions

```ds
const value: string | int32 = 1;
if (value is string) {
    value satisfies string;
} else {
    value satisfies int32;
}
```

### is narrows nominal unions

```ds
struct Rectangle {
    width: int32;
    height: int32;
}

struct Circle {
    radius: int32;
}

type Shape = Rectangle | Circle;

declare const shape: Shape;
if (shape is Rectangle) {
    shape.width satisfies int32;
} else {
    shape.radius satisfies int32;
}
```

### is narrows classes

```ds
class Admin {
    name: string = "";
}

const value: Admin | string = "root";
if (value is Admin) {
    value.name satisfies string;
} else {
    value satisfies string;
}
```

### newtype identity stays visible

```ds
newtype UserId = string;

const value: UserId | string = UserId("root");
if (value is UserId) {
    value satisfies UserId;
} else {
    value satisfies string;
}
```

### is expressions return boolean

```ds
class Admin {
    name: string = "";
}

const value: Admin | string = "root";
const ok = value is Admin;
ok satisfies boolean;
```

### is rejects structural types

```ds
const value: unknown = { name: "Ada" };
if (value is { name: string }) {
    value.name;
}
```

- contains: is cannot test structural object types
