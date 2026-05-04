# Newtype Interfaces

`newtype interface` nominal conformance.

## assignment

### nominal interface requires explicit implements

> Structural matches do not satisfy nominal interfaces.

```ds
newtype interface Add<T> {
    add(other: T): T;
}

struct Vec2 {
    x: int32;
    y: int32;

    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
```

- contains: not assignable

### nominal interface satisfied via implements

> Explicit implements provides nominal conformance.

```ds
newtype interface Add<T> {
    add(other: T): T;
}

struct Vec2 {
    x: int32;
    y: int32;
}

extension of Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```

### nominal interface conformance is preserved through type aliases

> Nominal interface conformance remains available through type aliases.

```ds
newtype interface Add<T> {
    add(other: T): T;
}

type AddVec2 = Add<Vec2>;

struct Vec2 {
    x: int32;
    y: int32;
}

extension of Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: AddVec2 = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```

### nominal interfaces remain nominal across module boundaries

> Imported nominal interfaces are not satisfied by structural matches without explicit implements.

```ds:contract.ds
export newtype interface Add<T> {
    add(other: T): T;
}
```

```ds:main.ds
import { Add } from "./contract";

struct Vec2 {
    x: int32;
    y: int32;

    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
```

- contains: not assignable

### nominal interfaces can be satisfied across module boundaries via implements

> Imported nominal interfaces are satisfied when explicit implements is provided.

```ds:contract.ds
export newtype interface Add<T> {
    add(other: T): T;
}
```

```ds:main.ds
import { Add } from "./contract";

struct Vec2 {
    x: int32;
    y: int32;
}

extension of Vec2 implements Add<Vec2> {
    add(other: Vec2): Vec2 {
        Vec2 { x: this.x + other.x, y: this.y + other.y }
    }
}

const value: Add<Vec2> = Vec2 { x: 0, y: 0 };
value.add(Vec2 { x: 1, y: 1 }) satisfies Vec2;
```
